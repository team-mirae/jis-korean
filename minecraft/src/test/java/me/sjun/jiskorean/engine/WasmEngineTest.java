package me.sjun.jiskorean.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/** A trap in the engine (a Rust panic, which wasm32 can't catch) must not take the game down. */
class WasmEngineTest {
	@Test
	void aTrapPassesTheKeyAndRestartsTheEngine() {
		WasmEngine engine = WasmEngine.load();
		assertTrue(engine.setMode(1));
		engine.key("KeyG", false);
		engine.trapNext = true;
		WasmEngine.Output out = engine.key("KeyK", false);
		assertFalse(out.consumed(), "the key goes to the game");
		assertEquals("", out.commit());
		// A fresh instance, still in Mode B, which has no ¥ key (Mode A has).
		assertFalse(engine.key("IntlYen", false).consumed());
		WasmEngine modeA = WasmEngine.load();
		assertTrue(modeA.key("IntlYen", false).consumed());
		out = engine.key("KeyG", false);
		assertTrue(out.consumed());
		assertEquals("ㅎ", out.preedit());
		out = engine.key("KeyK", false);
		assertEquals("하", out.preedit());
	}

	@Test
	void unknownModeIsRefused() {
		WasmEngine engine = WasmEngine.load();
		assertFalse(engine.setMode(99));
		assertTrue(engine.setMode(0));
	}
}
