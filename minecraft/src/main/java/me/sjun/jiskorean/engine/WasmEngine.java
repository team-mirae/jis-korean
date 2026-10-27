package me.sjun.jiskorean.engine;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
import java.util.function.Function;

import com.dylibso.chicory.runtime.ExportFunction;
import com.dylibso.chicory.runtime.Instance;
import com.dylibso.chicory.runtime.Memory;
import com.dylibso.chicory.wasm.ChicoryException;
import com.dylibso.chicory.wasm.Parser;
import com.dylibso.chicory.wasm.WasmModule;

/**
 * The JIS Korean engine (../engine, compiled to wasm32) running on Chicory. Same engine and C ABI
 * as the macOS input method and the web playground; see engine/include/jis_korean.h.
 *
 * <p>The engine never panics by design, but on wasm32 a Rust panic can't be caught (the C ABI's
 * {@code catch_unwind} is a no-op there) and becomes a trap. A trap must not take the game down: the
 * call returns a harmless fallback (a key passes through to the game), the instance is rebuilt from
 * the parsed module with the same mode, and whatever was staged is lost.
 */
public final class WasmEngine {
	/** Result of one engine call. */
	public record Output(String commit, String preedit, String english, boolean consumed) {
		public static final Output EMPTY = new Output("", "", "", true);
		/** A key the engine couldn't handle (a trap): typed nothing, let through to the game. */
		static final Output PASS = new Output("", "", "", false);
	}

	private static final int SHIFT = 1;
	private static final System.Logger LOG = System.getLogger("jiskorean");

	/** One instance of the engine and its exports; replaced after a trap. */
	private static final class Bound {
		final Memory memory;
		final long handle;
		final ExportFunction key;
		final ExportFunction flush;
		final ExportFunction clear;
		final ExportFunction activate;
		final ExportFunction setMode;
		final ExportFunction isEscaped;
		final ExportFunction commitPtr;
		final ExportFunction commitLen;
		final ExportFunction preeditPtr;
		final ExportFunction preeditLen;
		final ExportFunction englishPtr;
		final ExportFunction englishLen;
		final Instance instance;

		Bound(Instance instance) {
			this.instance = instance;
			memory = instance.memory();
			key = instance.export("jk_key");
			flush = instance.export("jk_flush");
			clear = instance.export("jk_clear");
			activate = instance.export("jk_activate");
			setMode = instance.export("jk_set_mode");
			isEscaped = instance.export("jk_is_escaped");
			commitPtr = instance.export("jk_commit_ptr");
			commitLen = instance.export("jk_commit_len");
			preeditPtr = instance.export("jk_preedit_ptr");
			preeditLen = instance.export("jk_preedit_len");
			englishPtr = instance.export("jk_english_ptr");
			englishLen = instance.export("jk_english_len");
			handle = instance.export("jk_new").apply()[0];
		}

		String string(long ptr, long len) {
			return len == 0 ? "" : memory.readString((int) ptr, (int) len, StandardCharsets.UTF_8);
		}
	}

	private final WasmModule module;
	private final Map<String, Integer> ids = new HashMap<>();
	private Bound bound;
	/** The mode last set, so a rebuilt instance types the same way. */
	private int mode;
	private boolean trapLogged;
	/** For tests: the next engine call traps, as a Rust panic would. */
	boolean trapNext;

	private WasmEngine(WasmModule module) {
		this.module = module;
		bound = new Bound(Instance.builder(module).build());
		Instance instance = bound.instance;
		ExportFunction codePtr = instance.export("jk_key_code_ptr");
		ExportFunction codeLen = instance.export("jk_key_code_len");
		long count = instance.export("jk_key_count").apply()[0];
		for (int id = 0; id < count; id++) {
			ids.put(bound.string(codePtr.apply(id)[0], codeLen.apply(id)[0]), id);
		}
	}

	/** Loads the engine bundled with the mod. */
	public static WasmEngine load() {
		try (InputStream in = WasmEngine.class.getResourceAsStream("/assets/jiskorean/engine.wasm")) {
			if (in == null) {
				throw new IllegalStateException("engine.wasm missing from the mod jar");
			}
			return new WasmEngine(Parser.parse(in));
		} catch (IOException e) {
			throw new UncheckedIOException(e);
		}
	}

	/** Whether the engine knows this {@code KeyboardEvent.code} name. */
	public boolean knows(String code) {
		return ids.containsKey(code);
	}

	/** One key press. Unknown codes count as "Other" (commit and pass through). */
	public Output key(String code, boolean shift) {
		int id = ids.getOrDefault(code, ids.get("Other"));
		return call(b -> {
			boolean consumed = b.key.apply(b.handle, id, shift ? SHIFT : 0)[0] != 0;
			return output(b, consumed);
		}, Output.PASS);
	}

	/** Commits everything unconfirmed, as the engine reads it (Korean). */
	public Output flush() {
		return call(b -> {
			b.flush.apply(b.handle);
			return output(b, true);
		}, Output.EMPTY);
	}

	/** Discards everything unconfirmed. */
	public void clear() {
		call(b -> b.clear.apply(b.handle), null);
	}

	/** Leaves any F8 escape. */
	public void activate() {
		call(b -> b.activate.apply(b.handle), null);
	}

	/**
	 * Switches the layout mode by its stable id (spec §3.6.1): 0 Mode A (JIS), 1 Mode B (US
	 * ANSI), 2 Mode C (UK ISO). An id the engine doesn't have (an English mode, which the mod's
	 * engine isn't built with) is refused, and the mode stays as it was.
	 *
	 * @return whether the engine took the mode
	 */
	public boolean setMode(int mode) {
		boolean set = call(b -> b.setMode.apply(b.handle, mode)[0] != 0, false);
		if (set) {
			this.mode = mode;
		}
		return set;
	}

	public boolean escaped() {
		return call(b -> b.isEscaped.apply(b.handle)[0] != 0, false);
	}

	private <T> T call(Function<Bound, T> body, T fallback) {
		try {
			if (trapNext) {
				trapNext = false;
				throw new ChicoryException("trap (test)");
			}
			return body.apply(bound);
		} catch (RuntimeException e) {
			// A trap (or a read out of bounds after one): the instance's state can't be trusted.
			if (!trapLogged) {
				trapLogged = true;
				LOG.log(System.Logger.Level.ERROR, "JIS Korean engine trapped; restarting it", e);
			}
			restart();
			return fallback;
		}
	}

	private void restart() {
		try {
			Bound fresh = new Bound(Instance.builder(module).build());
			if (mode != 0) {
				fresh.setMode.apply(fresh.handle, mode);
			}
			bound = fresh;
		} catch (RuntimeException e) {
			// Keep the old instance; the next call tries again.
			LOG.log(System.Logger.Level.ERROR, "JIS Korean engine failed to restart", e);
		}
	}

	private static Output output(Bound b, boolean consumed) {
		return new Output(
				b.string(b.commitPtr.apply(b.handle)[0], b.commitLen.apply(b.handle)[0]),
				b.string(b.preeditPtr.apply(b.handle)[0], b.preeditLen.apply(b.handle)[0]),
				b.string(b.englishPtr.apply(b.handle)[0], b.englishLen.apply(b.handle)[0]),
				consumed);
	}
}
