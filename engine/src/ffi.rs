//! C ABI shared by the macOS, Windows and web (wasm32) shells. See `include/jis_korean.h`.
//!
//! An engine call stores its result in the handle; read it with `jk_commit_*` and
//! `jk_preedit_*` until the next call. Strings are UTF-8 and not NUL-terminated.

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::{Engine, Key, Layout, Mods, Output, output};

pub struct JkEngine {
    engine: Engine,
    last: Output,
}

/// Never unwind into the host: on Windows the engine runs inside every app's process.
fn guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(fallback)
}

fn with<T>(handle: *mut JkEngine, fallback: T, f: impl FnOnce(&mut JkEngine) -> T) -> T {
    // SAFETY: callers pass a handle from `jk_new` that hasn't been freed, from one thread.
    match unsafe { handle.as_mut() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_new() -> *mut JkEngine {
    Box::into_raw(Box::new(JkEngine { engine: Engine::new(), last: Output::default() }))
}

/// # Safety
/// `handle` must come from `jk_new` and not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jk_free(handle: *mut JkEngine) {
    if !handle.is_null() {
        // SAFETY: per the contract above.
        drop(unsafe { Box::from_raw(handle) });
    }
}

/// Handles one key press. Returns 1 if consumed, 0 if the shell should pass the key on to the
/// app after applying the commit and preedit.
#[unsafe(no_mangle)]
pub extern "C" fn jk_key(handle: *mut JkEngine, key: u32, mods: u32) -> u32 {
    with(handle, 0, |h| {
        h.last = h.engine.key(Key::from_id(key), Mods::from_bits(mods));
        h.last.consumed as u32
    })
}

/// Commits everything unconfirmed (focus loss, app or IME switch).
#[unsafe(no_mangle)]
pub extern "C" fn jk_flush(handle: *mut JkEngine) {
    with(handle, (), |h| h.last = h.engine.flush())
}

/// The IME was switched back to: leaves any F8 escape.
#[unsafe(no_mangle)]
pub extern "C" fn jk_activate(handle: *mut JkEngine) {
    with(handle, (), |h| h.engine.activate())
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_set_enter_commits(handle: *mut JkEngine, on: u32) {
    with(handle, (), |h| h.engine.set_enter_commits(on != 0))
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_is_escaped(handle: *mut JkEngine) -> u32 {
    with(handle, 0, |h| h.engine.is_escaped() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_commit_ptr(handle: *mut JkEngine) -> *const u8 {
    with(handle, std::ptr::null(), |h| h.last.commit.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_commit_len(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.last.commit.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_preedit_ptr(handle: *mut JkEngine) -> *const u8 {
    with(handle, std::ptr::null(), |h| h.last.preedit.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_preedit_len(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.last.preedit.len())
}

/// Number of key ids; ids are `0..jk_key_count()`.
#[unsafe(no_mangle)]
pub extern "C" fn jk_key_count() -> u32 {
    Key::ALL.len() as u32
}

/// The `KeyboardEvent.code` name of a key id (static string).
#[unsafe(no_mangle)]
pub extern "C" fn jk_key_code_ptr(key: u32) -> *const u8 {
    Key::from_id(key).code().as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_key_code_len(key: u32) -> usize {
    Key::from_id(key).code().len()
}

/// The character a key types, for drawing key legends: a Unicode scalar value, or 0 if the key
/// types nothing by itself.
#[unsafe(no_mangle)]
pub extern "C" fn jk_legend(key: u32, shift: u32, escaped: u32) -> u32 {
    let layout = if escaped != 0 { Layout::Escaped } else { Layout::JisKorean };
    output(layout, Key::from_id(key), shift != 0).map_or(0, |o| o.char() as u32)
}
