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
 * would, by its engine id (spec §3.6.1): 0 Mode A (JIS), 1 Mode B (US ANSI), 2 Mode C (UK ISO), or an
 * English mode's id, which the mod types as its board's Korean mode ({@code Staging.koreanMode}).
 * On macOS it's read from the input method's public API (spec §8.2):
 * {@code defaults read me.sjun.inputmethod.JISKorean CurrentModeID}. On Windows it's the input
 * method's {@code Mode} setting (spec §8.4), read as JIS Korean reads it:
 * {@code HKCU\Software\JIS Korean}, else {@code HKLM\Software\JIS Korean}. Elsewhere, or if JIS
 * Korean has never been set up, it's Mode A.
 *
 * <p>Reading takes a process, so {@link #refresh()} reads in the background and {@link #get()}
 * returns the last value read.
 */
final class SystemMode {
	static final int MODE_A = 0;
	private static final String DOMAIN = "me.sjun.inputmethod.JISKorean";
	private static final String OS = System.getProperty("os.name", "").toLowerCase();
	private static final boolean MAC = OS.contains("mac");
	private static final boolean WINDOWS = OS.contains("windows");
	private static final String[] WINDOWS_KEYS = {"HKCU\\Software\\JIS Korean", "HKLM\\Software\\JIS Korean"};

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
		if ((MAC || WINDOWS) && pending.compareAndSet(false, true)) {
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
		if (WINDOWS) {
			for (String key : WINDOWS_KEYS) {
				String out = run("reg", "query", key, "/v", "Mode");
				int mode = out == null ? -1 : parseRegQuery(out);
				if (mode >= 0) {
					return mode;
				}
			}
			return MODE_A;
		}
		String out = run("/usr/bin/defaults", "read", DOMAIN, "CurrentModeID");
		try {
			return out == null ? MODE_A : Integer.parseInt(out.trim());
		} catch (NumberFormatException e) {
			return MODE_A;
		}
	}

	/**
	 * The mode in {@code reg query <key> /v Mode} output, a line such as
	 * {@code "    Mode    REG_DWORD    0x1"}: the engine mode id as stored, as the macOS
	 * path passes {@code CurrentModeID} on; -1 if there's none.
	 */
	static int parseRegQuery(String out) {
		for (String line : out.split("\\R")) {
			String[] fields = line.trim().split("\\s+");
			if (fields.length == 3 && fields[0].equals("Mode") && fields[1].equals("REG_DWORD")
					&& fields[2].startsWith("0x")) {
				try {
					return Integer.parseInt(fields[2].substring(2), 16);
				} catch (NumberFormatException e) {
					return -1;
				}
			}
		}
		return -1;
	}

	/** The command's output, or null if it failed or took more than 2 seconds. */
	private static String run(String... command) {
		try {
			Process process = new ProcessBuilder(command).redirectErrorStream(true).start();
			// Wait first: reading first would block for as long as a hung command keeps its output
			// open, and the reader (and every later refresh) with it. The output is a line or two, so
			// it can't fill the pipe while we wait.
			if (!process.waitFor(2, TimeUnit.SECONDS)) {
				process.destroyForcibly();
				return null;
			}
			if (process.exitValue() != 0) {
				return null;
			}
			try (InputStream in = process.getInputStream()) {
				return new String(in.readAllBytes(), StandardCharsets.UTF_8);
			}
		} catch (IOException e) {
			return null;
		} catch (InterruptedException e) {
			Thread.currentThread().interrupt();
			return null;
		}
	}
}
