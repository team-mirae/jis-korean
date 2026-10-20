package me.sjun.jiskorean.engine;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;

import com.dylibso.chicory.runtime.ExportFunction;
import com.dylibso.chicory.runtime.Instance;
import com.dylibso.chicory.runtime.Memory;
import com.dylibso.chicory.wasm.Parser;

/**
 * The JIS Korean engine (../engine, compiled to wasm32) running on Chicory. Same engine and C ABI
 * as the macOS input method and the web playground; see engine/include/jis_korean.h.
 */
public final class WasmEngine {
	/** Result of one engine call. */
	public record Output(String commit, String preedit, String english, boolean consumed) {
		public static final Output EMPTY = new Output("", "", "", true);
	}

	private static final int SHIFT = 1;

	private final Memory memory;
	private final long handle;
	private final Map<String, Integer> ids = new HashMap<>();
	private final ExportFunction key;
	private final ExportFunction flush;
	private final ExportFunction clear;
	private final ExportFunction activate;
	private final ExportFunction setMode;
	private final ExportFunction isEscaped;
	private final ExportFunction commitPtr;
	private final ExportFunction commitLen;
	private final ExportFunction preeditPtr;
	private final ExportFunction preeditLen;
	private final ExportFunction englishPtr;
	private final ExportFunction englishLen;

	private WasmEngine(Instance instance) {
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

		ExportFunction codePtr = instance.export("jk_key_code_ptr");
		ExportFunction codeLen = instance.export("jk_key_code_len");
		long count = instance.export("jk_key_count").apply()[0];
		for (int id = 0; id < count; id++) {
			ids.put(string(codePtr.apply(id)[0], codeLen.apply(id)[0]), id);
		}
	}

	/** Loads the engine bundled with the mod. */
	public static WasmEngine load() {
		try (InputStream in = WasmEngine.class.getResourceAsStream("/assets/jiskorean/engine.wasm")) {
			if (in == null) {
				throw new IllegalStateException("engine.wasm missing from the mod jar");
			}
			return new WasmEngine(Instance.builder(Parser.parse(in)).build());
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
		boolean consumed = key.apply(handle, id, shift ? SHIFT : 0)[0] != 0;
		return output(consumed);
	}

	/** Commits everything unconfirmed, as the engine reads it (Korean). */
	public Output flush() {
		flush.apply(handle);
		return output(true);
	}

	/** Discards everything unconfirmed. */
	public void clear() {
		clear.apply(handle);
	}

	/** Leaves any F8 escape. */
	public void activate() {
		activate.apply(handle);
	}

	/**
	 * Switches the layout mode by its stable id (spec §3.6): 0 is Mode A (JIS), 1 is Mode B (US
	 * ANSI). Unknown ids are ignored by the engine.
	 */
	public void setMode(int mode) {
		setMode.apply(handle, mode);
	}

	public boolean escaped() {
		return isEscaped.apply(handle)[0] != 0;
	}

	private Output output(boolean consumed) {
		return new Output(
				string(commitPtr.apply(handle)[0], commitLen.apply(handle)[0]),
				string(preeditPtr.apply(handle)[0], preeditLen.apply(handle)[0]),
				string(englishPtr.apply(handle)[0], englishLen.apply(handle)[0]),
				consumed);
	}

	private String string(long ptr, long len) {
		return len == 0 ? "" : memory.readString((int) ptr, (int) len, StandardCharsets.UTF_8);
	}
}
