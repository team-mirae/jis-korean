package me.sjun.jiskorean.client;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.EditBox;

import me.sjun.jiskorean.Staging;
import me.sjun.jiskorean.rank.Ranker;

/**
 * The staging area's own window, drawn just above the caret of the focused field. The game's
 * suggestion line ({@code EditBox#setSuggestion}) isn't used: chat commands need it.
 *
 * <pre>
 *   [ 한글 | gksrmf ]        ← ranked reading highlighted; Space/Enter/Tab commits it
 *   ─────────────────
 *   /chat field▏
 * </pre>
 * With nothing staged, a small 한 badge shows the mod is on.
 */
final class CandidateWindow {
	// The project's palette (#433e3f #7a7265 #8e6e53 #c69c72 #c0b7b1).
	private static final int BACKGROUND = 0xE6433E3F;
	private static final int CHOSEN_BACKGROUND = 0xFF8E6E53;
	private static final int CHOSEN = 0xFFFFFFFF;
	private static final int OTHER = 0xFFC0B7B1;
	private static final int HINT = 0xFF7A7265;
	private static final int ACCENT = 0xFFC69C72;
	private static final int PAD = 3;
	private static final int GAP = 6;

	private CandidateWindow() {
	}

	static void draw(GuiGraphicsExtractor graphics, EditBox field, Staging staging) {
		Font font = Minecraft.getInstance().font;
		int caret = field.getScreenX(field.getCursorPosition());
		int height = font.lineHeight + PAD * 2;
		int bottom = field.getY() - 2;
		int top = bottom - height;
		graphics.nextStratum();

		if (staging.isEmpty()) {
			String badge = staging.escaped() ? "한 두벌식" : "한";
			int width = font.width(badge) + PAD * 2;
			int x = field.getRight() - width;
			graphics.fill(x, top, x + width, bottom, BACKGROUND);
			graphics.text(font, badge, x + PAD, top + PAD, ACCENT, false);
			return;
		}

		String value = field.getValue();
		String before = value.substring(0, Math.min(field.getCursorPosition(), value.length()));
		Ranker.Choice choice = staging.ranking(before);
		boolean korean = choice.language() == Ranker.Language.KOREAN;
		String first = staging.korean();
		String second = staging.english();
		String hint = "F6 한 · F7 En";

		int firstWidth = font.width(first);
		int secondWidth = font.width(second);
		int width = PAD + firstWidth + GAP + secondWidth + GAP + font.width(hint) + PAD;
		int x = Math.max(2, Math.min(caret - PAD, graphics.guiWidth() - width - 2));

		graphics.fill(x, top, x + width, bottom, BACKGROUND);
		int firstX = x + PAD;
		int secondX = firstX + firstWidth + GAP;
		int chosenX = korean ? firstX : secondX;
		int chosenWidth = korean ? firstWidth : secondWidth;
		graphics.fill(chosenX - 2, top + 1, chosenX + chosenWidth + 2, bottom - 1, CHOSEN_BACKGROUND);
		graphics.text(font, first, firstX, top + PAD, korean ? CHOSEN : OTHER, false);
		graphics.text(font, second, secondX, top + PAD, korean ? OTHER : CHOSEN, false);
		graphics.text(font, hint, secondX + secondWidth + GAP, top + PAD, HINT, false);
	}
}
