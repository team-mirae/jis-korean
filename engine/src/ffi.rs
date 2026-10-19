//! C ABI shared by the macOS, Windows and web (wasm32) shells. See `include/jis_korean.h`.
//!
//! An engine call stores its result in the handle; read it with `jk_commit_*` and
//! `jk_preedit_*` until the next call. Strings are UTF-8 and not NUL-terminated.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use crate::plan::{self, Counts};
use crate::{Candidate, Dictionary, Engine, Key, Layout, Mods, Output, PAGE_SIZE, output};

pub struct JkEngine {
    engine: Engine,
    last: Output,
    counts: Counts,
}

/// A parsed hanja dictionary, shareable by many engines.
pub struct JkDict(Arc<Dictionary>);

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
    Box::into_raw(Box::new(JkEngine { engine: Engine::new(), last: Output::default(), counts: Counts::default() }))
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

// --- Hanja (spec §7)

/// Allocates `len` bytes in the engine's memory, for passing data in from wasm hosts.
#[unsafe(no_mangle)]
pub extern "C" fn jk_alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// # Safety
/// `ptr` and `len` must come from one `jk_alloc` call, and `ptr` not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jk_dealloc(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        // SAFETY: per the contract above; length 0 so no element is dropped.
        drop(unsafe { Vec::from_raw_parts(ptr, 0, len) });
    }
}

/// Parses a dictionary (`dict/hanja.tsv` contents, UTF-8). Null if it isn't UTF-8.
///
/// # Safety
/// `ptr` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jk_dict_new(ptr: *const u8, len: usize) -> *mut JkDict {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: per the contract above.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    guard(std::ptr::null_mut(), || match std::str::from_utf8(bytes) {
        Ok(text) => Box::into_raw(Box::new(JkDict(Arc::new(Dictionary::parse(text))))),
        Err(_) => std::ptr::null_mut(),
    })
}

/// Engines given this dictionary keep their own reference, so it can be freed any time.
///
/// # Safety
/// `dict` must come from `jk_dict_new` and not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jk_dict_free(dict: *mut JkDict) {
    if !dict.is_null() {
        // SAFETY: per the contract above.
        drop(unsafe { Box::from_raw(dict) });
    }
}

/// # Safety
/// `dict` must be null or come from `jk_dict_new` and not be freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jk_set_dictionary(handle: *mut JkEngine, dict: *const JkDict) {
    // SAFETY: per the contract above.
    let Some(dict) = (unsafe { dict.as_ref() }) else { return };
    with(handle, (), |h| h.engine.set_dictionary(dict.0.clone()))
}

/// Candidates on the last result: 0 when no list is open.
#[unsafe(no_mangle)]
pub extern "C" fn jk_candidate_count(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.last.candidates.as_ref().map_or(0, |c| c.items.len()))
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_candidate_selected(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.last.candidates.as_ref().map_or(0, |c| c.selected))
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_candidate_page_size() -> usize {
    PAGE_SIZE
}

fn candidate(h: &JkEngine, index: usize) -> Option<&Candidate> {
    h.last.candidates.as_ref()?.items.get(index)
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_candidate_ptr(handle: *mut JkEngine, index: usize) -> *const u8 {
    with(handle, std::ptr::null(), |h| candidate(h, index).map_or(std::ptr::null(), |c| c.hanja.as_ptr()))
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_candidate_len(handle: *mut JkEngine, index: usize) -> usize {
    with(handle, 0, |h| candidate(h, index).map_or(0, |c| c.hanja.len()))
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_candidate_comment_ptr(handle: *mut JkEngine, index: usize) -> *const u8 {
    with(handle, std::ptr::null(), |h| candidate(h, index).map_or(std::ptr::null(), |c| c.comment.as_ptr()))
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_candidate_comment_len(handle: *mut JkEngine, index: usize) -> usize {
    with(handle, 0, |h| candidate(h, index).map_or(0, |c| c.comment.len()))
}

// --- English reading (for hosts that rank Korean against English, like the Minecraft mod)

/// The last result's keystrokes read as English on stock JIS.
#[unsafe(no_mangle)]
pub extern "C" fn jk_english_ptr(handle: *mut JkEngine) -> *const u8 {
    with(handle, std::ptr::null(), |h| h.last.english.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_english_len(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.last.english.len())
}

/// Discards everything unconfirmed without committing it.
#[unsafe(no_mangle)]
pub extern "C" fn jk_clear(handle: *mut JkEngine) {
    with(handle, (), |h| {
        h.engine.clear();
        h.last = Output::default();
    })
}

// --- Key-press counts (the web playground's comparison)

/// Counts the key presses `text` (UTF-8) takes on JIS Korean and on conventional 2-set, using this
/// engine's hanja dictionary if it has one. Read the results with `jk_plan_*`.
///
/// # Safety
/// `ptr` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jk_plan(handle: *mut JkEngine, ptr: *const u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: per the contract above.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let text = String::from_utf8_lossy(bytes).into_owned();
    with(handle, (), |h| h.counts = plan::count(&text, h.engine.dictionary()))
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_plan_jis(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.counts.jis)
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_plan_conventional(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.counts.conventional)
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_plan_chars(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.counts.chars)
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_plan_off_keyboard(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.counts.off_keyboard)
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_plan_dropped(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.counts.dropped)
}
