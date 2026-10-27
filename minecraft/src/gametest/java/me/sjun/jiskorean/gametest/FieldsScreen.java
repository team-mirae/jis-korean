package me.sjun.jiskorean.gametest;

import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

/**
 * Two text fields and a Done button that reads the first field when it's clicked, as Add Server,
 * Create World or a command block's Done does. For {@link TypingGameTest}.
 */
public class FieldsScreen extends Screen {
	static final int X = 20;
	static final int WIDTH = 160;
	static final int HEIGHT = 20;
	static final int FIRST_Y = 40;
	static final int SECOND_Y = 80;
	static final int BUTTON_Y = 120;

	EditBox first;
	EditBox second;
	/** The first field's value when Done was clicked; null until it is. */
	String done;

	public FieldsScreen() {
		super(Component.literal("JIS Korean test fields"));
	}

	@Override
	protected void init() {
		first = addRenderableWidget(new EditBox(font, X, FIRST_Y, WIDTH, HEIGHT, Component.literal("first")));
		second = addRenderableWidget(new EditBox(font, X, SECOND_Y, WIDTH, HEIGHT, Component.literal("second")));
		addRenderableWidget(Button.builder(Component.literal("Done"), button -> done = first.getValue())
				.bounds(X, BUTTON_Y, WIDTH, HEIGHT).build());
		setInitialFocus(first);
	}
}
