package me.sjun.jiskorean.client;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

/**
 * The layout mode the JIS Korean input method is in, so the mod types the way the OS input method
 * would. On macOS it's read from the input method's public API (spec §8.2):
 * {@code defaults read me.sjun.inputmethod.JISKorean CurrentModeID}, 0 for Mode A (JIS) and 1 for
 * Mode B (US ANSI). Elsewhere, or if JIS Korean has never run, it's Mode A.
 *
 * <p>Reading takes a process, so {@link #refresh()} reads in the background and {@link #get()}
 * returns the last value read.
 */
final class SystemMode {
	static final int MODE_A = 0;
	private static final String DOMAIN = "me.sjun.inputmethod.JISKorean";
	private static final boolean MAC = System.getProperty("os.name", "").toLowerCase().contains("mac");

	private static final ExecutorService READER = Executors.newSingleThreadExecutor(runnable -> {
		Thread thread = new Thread(runnable, "JIS Korean mode reader");
		thread.setDaemon(true);
		return thread;
	});
	private static final AtomicBoolean pending = new AtomicBoolean();
	private static volatile int mode = MODE_A;

	private SystemMode() {
	}

	/** The last mode read (Mode A until a read finishes). */
	static int get() {
		return mode;
	}

	/** Reads the mode again in the background. Calls while a read is under way are dropped. */
	static void refresh() {
		if (MAC && pending.compareAndSet(false, true)) {
			READER.execute(() -> {
				try {
					mode = read();
				} finally {
					pending.set(false);
				}
			});
		}
	}

	private static int read() {
		try {
			Process process = new ProcessBuilder("/usr/bin/defaults", "read", DOMAIN, "CurrentModeID")
					.redirectErrorStream(true)
					.start();
			String out;
			try (InputStream in = process.getInputStream()) {
				out = new String(in.readAllBytes(), StandardCharsets.UTF_8).trim();
			}
			if (!process.waitFor(2, TimeUnit.SECONDS) || process.exitValue() != 0) {
				return MODE_A;
			}
			return Integer.parseInt(out);
		} catch (IOException | NumberFormatException e) {
			return MODE_A;
		} catch (InterruptedException e) {
			Thread.currentThread().interrupt();
			return MODE_A;
		}
	}
}
