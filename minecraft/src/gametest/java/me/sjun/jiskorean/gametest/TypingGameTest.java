package me.sjun.jiskorean.gametest;

import java.util.Map;

import com.mojang.blaze3d.platform.InputConstants;

import net.fabricmc.fabric.api.client.gametest.v1.FabricClientGameTest;
import net.fabricmc.fabric.api.client.gametest.v1.context.ClientGameTestContext;
import net.fabricmc.fabric.api.client.gametest.v1.context.TestSingleplayerContext;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.screens.ChatScreen;

import me.sjun.jiskorean.client.JisKoreanClient;
import me.sjun.jiskorean.engine.Keys;

/**
 * The robot, in the real game: opens chat in a test world, presses keys (scancodes, through the
 * game's own keyboard handler) and checks what the chat field holds. Run with
 * {@code ./gradlew runClientGameTest}; screenshots land in build/run/clientGameTest/screenshots.
 *
 * <p>Key strings use the same notation as StagingTest: a–z, A–Z (with Shift), 0–9, a space for
 * Space, and {Name} for named keys.
 */
public class TypingGameTest implements FabricClientGameTest {
	private static final Map<String, Integer> NAMED = Map.of(
			"Tab", Keys.TAB, "Escape", Keys.ESCAPE, "Backspace", Keys.BACKSPACE,
			"F6", Keys.F6, "F7", Keys.F7, "F8", 65);

	@Override
	public void runTest(ClientGameTestContext context) {
		try (TestSingleplayerContext world = context.worldBuilder().create()) {
			world.getConnection().waitForChunksRender();
			context.runOnClient(client -> JisKoreanClient.pinMode(0));

			check(context, "gksrmf ", "한글 ");
			check(context, "hello ", "hello ");
			check(context, "dhsmf eklk zofj rkwk ", "오늘 다이아 캐러 가자 ");
			check(context, "let me craft a diamond sword ", "let me craft a diamond sword ");
			check(context, "Steve rkxll gkwk ", "Steve 같이 하자 ");
			check(context, "dkssud{F7} ", "dkssud ");
			check(context, "1k 7 ", "빠 워 "); // 7 is ㅝ; on its own it gets the implied ㅇ

			// Focus loss commits (spec §5.1): a click, another field, a resize keeps it.
			clickCommitsBeforeTheButtonReadsTheField(context);
			clickingAnotherFieldCommitsIntoTheFirst(context);
			resizeKeepsTheStagingArea(context);

			// The candidate window, mid-word.
			openChat(context);
			type(context, "gksrmf");
			context.waitTicks(2);
			context.takeScreenshot("jiskorean-candidates-korean");
			type(context, " minecraf");
			context.waitTicks(2);
			context.takeScreenshot("jiskorean-candidates-english");
			closeChat(context);
		}
	}

	private void clickCommitsBeforeTheButtonReadsTheField(ClientGameTestContext context) {
		FieldsScreen screen = openFields(context);
		type(context, "gkrry");
		click(context, FieldsScreen.BUTTON_Y);
		String done = context.computeOnClient(_ -> screen.done);
		closeFields(context);
		if (!"학교".equals(done)) {
			throw new AssertionError("Done read \"" + done + "\", expected \"학교\"");
		}
	}

	private void clickingAnotherFieldCommitsIntoTheFirst(ClientGameTestContext context) {
		FieldsScreen screen = openFields(context);
		// (Not dkssud: F7 made that English for the session, above.)
		type(context, "gksrmf");
		click(context, FieldsScreen.SECOND_Y);
		type(context, "hello ");
		String first = context.computeOnClient(_ -> screen.first.getValue());
		String second = context.computeOnClient(_ -> screen.second.getValue());
		closeFields(context);
		if (!"한글".equals(first) || !"hello ".equals(second)) {
			throw new AssertionError("fields \"" + first + "\", \"" + second + "\", expected \"한글\", \"hello \"");
		}
	}

	private void resizeKeepsTheStagingArea(ClientGameTestContext context) {
		// Chat rebuilds its field on a resize; what's staged moves on to the new one.
		openChat(context);
		type(context, "gksrmf");
		context.runOnClient(client -> client.gui.screen().resize(
				client.getWindow().getGuiScaledWidth(), client.getWindow().getGuiScaledHeight()));
		context.waitTick();
		type(context, " ");
		String actual = context.computeOnClient(client -> field(client.gui.screen()).getValue());
		closeChat(context);
		if (!"한글 ".equals(actual)) {
			throw new AssertionError("after a resize: expected \"한글 \", got \"" + actual + "\"");
		}
	}

	private FieldsScreen openFields(ClientGameTestContext context) {
		context.setScreen(FieldsScreen::new);
		context.waitForScreen(FieldsScreen.class);
		context.waitTick();
		return context.computeOnClient(client -> (FieldsScreen) client.gui.screen());
	}

	private void closeFields(ClientGameTestContext context) {
		context.setScreen(() -> null);
		context.waitForScreen(null);
	}

	/** Clicks the middle of the widget at GUI row {@code y} of {@link FieldsScreen}. */
	private void click(ClientGameTestContext context, int y) {
		double[] at = context.computeOnClient(client -> {
			double scale = (double) client.getWindow().getScreenWidth() / client.getWindow().getGuiScaledWidth();
			return new double[] {
				(FieldsScreen.X + FieldsScreen.WIDTH / 2.0) * scale,
				(y + FieldsScreen.HEIGHT / 2.0) * scale};
		});
		context.getInput().setCursorPos(at[0], at[1]);
		context.getInput().pressMouse(InputConstants.MOUSE_BUTTON_LEFT);
		context.waitTick();
	}

	private void check(ClientGameTestContext context, String keys, String expected) {
		openChat(context);
		type(context, keys);
		String actual = context.computeOnClient(client -> field(client.gui.screen()).getValue());
		closeChat(context);
		if (!expected.equals(actual)) {
			throw new AssertionError("keys " + keys + ": expected \"" + expected + "\", got \"" + actual + "\"");
		}
	}

	private static EditBox field(net.minecraft.client.gui.screens.Screen screen) {
		EditBox box = JisKoreanClient.focusedField(screen);
		if (box == null) {
			throw new AssertionError("no focused text field on " + screen);
		}
		return box;
	}

	private void openChat(ClientGameTestContext context) {
		context.getInput().pressKey(options -> options.keyChat);
		context.waitForScreen(ChatScreen.class);
		context.waitTick();
	}

	private void closeChat(ClientGameTestContext context) {
		// The first Escape drops anything staged, the second closes chat.
		context.getInput().pressKey(Keys.ESCAPE);
		context.waitTick();
		if (context.computeOnClient(client -> client.gui.screen() != null)) {
			context.getInput().pressKey(Keys.ESCAPE);
		}
		context.waitForScreen(null);
	}

	private void type(ClientGameTestContext context, String keys) {
		for (int i = 0; i < keys.length(); i++) {
			char c = keys.charAt(i);
			if (c == '{') {
				int end = keys.indexOf('}', i);
				context.getInput().pressKey(NAMED.get(keys.substring(i + 1, end)));
				i = end;
			} else if (c == ' ') {
				context.getInput().pressKey(Keys.SPACE);
			} else if (c >= 'a' && c <= 'z') {
				context.getInput().pressKey(4 + c - 'a');
			} else if (c >= 'A' && c <= 'Z') {
				context.getInput().holdShift();
				context.getInput().pressKey(4 + c - 'A');
				context.getInput().releaseShift();
			} else if (c >= '1' && c <= '9') {
				context.getInput().pressKey(30 + c - '1');
			} else if (c == '0') {
				context.getInput().pressKey(39);
			}
		}
		context.waitTick();
	}
}
