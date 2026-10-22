//! C ABI shared by the macOS, Windows and web (wasm32) shells. See `include/jis_korean.h`.
//!
//! An engine call stores its result in the handle; read it with `jk_commit_*` and
//! `jk_preedit_*` until the next call. Strings are UTF-8 and not NUL-terminated.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use crate::plan::{self, Counts};
use crate::resolve;
use crate::{Candidate, Dictionary, Engine, Key, Layout, Mode, Mods, Output, PAGE_SIZE, output_in};

pub struct JkEngine {
    engine: Engine,
    last: Output,
    counts: Counts,
    /// The last resolve call's answer (spec §8.3).
    resolved: String,
    #[cfg(feature = "predict")]
    predictions: Vec<crate::predict::Prediction>,
    /// The learner's last export (spec §10.11).
    #[cfg(feature = "predict")]
    learned: String,
}

/// A parsed hanja dictionary, shareable by many engines.
pub struct JkDict(Arc<Dictionary>);

/// Never unwind into the host: on Windows the engine runs inside every app's process.
fn guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(fallback)
}

fn with<T>(handle: *mut JkEngine, fallback: T, f: impl FnOnce(&mut JkEngine) -> T) -> T {
    // SAFETY: callers pass a handle from `jk_new` that hasn't been freed, from one thread.
    if unsafe { handle.as_mut() }.is_none() {
        return fallback;
    }
    match catch_unwind(AssertUnwindSafe(|| f(unsafe { &mut *handle }))) {
        Ok(value) => value,
        Err(_) => {
            // A panic: nothing from before it may be applied again (the shell reads `last` after
            // the call), and the unconfirmed state it left may be inconsistent.
            let h = unsafe { &mut *handle };
            h.last = Output::default();
            h.engine.clear();
            fallback
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_new() -> *mut JkEngine {
    Box::into_raw(Box::new(JkEngine {
        engine: Engine::new(),
        last: Output::default(),
        counts: Counts::default(),
        resolved: String::new(),
        #[cfg(feature = "predict")]
        predictions: Vec::new(),
        #[cfg(feature = "predict")]
        learned: String::new(),
    }))
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

/// The mode, by id (spec §3.6.1): 0 = Mode A (JIS), 1 = Mode B (US ANSI), 2 = Mode C (UK ISO).
/// Returns 1 if it switched, or 0 for an id this build doesn't have, keeping the current mode.
#[unsafe(no_mangle)]
pub extern "C" fn jk_set_mode(handle: *mut JkEngine, mode: u32) -> u32 {
    with(handle, 0, |h| match Mode::from_id(mode) {
        Some(mode) => {
            h.engine.set_mode(mode);
            1
        }
        None => 0,
    })
}

/// Setting: Dedicated symbol keys (spec §3.6), on (1) by default. Off (0), Mode C's [ ] and ISO
/// extra key type their UK ISO characters ([ { ] } \ |). Modes without optional symbol keys ignore it.
#[unsafe(no_mangle)]
pub extern "C" fn jk_set_dedicated_symbols(handle: *mut JkEngine, on: u32) {
    with(handle, (), |h| h.engine.set_dedicated_symbols(on != 0))
}

/// How many modes this build has. Ids have gaps: list them with [`jk_mode_id_at`].
#[unsafe(no_mangle)]
pub extern "C" fn jk_mode_count() -> u32 {
    Mode::ALL.len() as u32
}

/// The id of this build's `index`th mode (0 to count − 1, in menu order), or `u32::MAX` past the
/// end. The index is a position in the list, never an id.
#[unsafe(no_mangle)]
pub extern "C" fn jk_mode_id_at(index: u32) -> u32 {
    Mode::ALL.get(index as usize).map_or(u32::MAX, |m| m.id())
}

fn mode_text(mode: u32, text: fn(Mode) -> &'static str) -> &'static str {
    Mode::from_id(mode).map_or("", text)
}

/// A mode's display name (static UTF-8), for settings menus. Empty for an unknown id.
#[unsafe(no_mangle)]
pub extern "C" fn jk_mode_name_ptr(mode: u32) -> *const u8 {
    mode_text(mode, Mode::name).as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_mode_name_len(mode: u32) -> usize {
    mode_text(mode, Mode::name).len()
}

/// A mode's mode name (static UTF-8, `A`, `A_QWERTY`; spec §3.6.1). Empty for an unknown id.
#[unsafe(no_mangle)]
pub extern "C" fn jk_mode_code_ptr(mode: u32) -> *const u8 {
    mode_text(mode, Mode::code).as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_mode_code_len(mode: u32) -> usize {
    mode_text(mode, Mode::code).len()
}

/// Whether a mode is an English mode (spec §3.6.2): it types its letters and stock symbols, composes
/// nothing and consumes no key, so a shell types its keys from [`jk_legend_in`]. 0 for the Korean
/// modes and for an unknown id.
#[unsafe(no_mangle)]
pub extern "C" fn jk_mode_is_english(mode: u32) -> u32 {
    Mode::from_id(mode).is_some_and(|m| !m.is_korean()) as u32
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
    jk_legend_in(key, shift, escaped, 0)
}

/// The same in a given mode, by id.
#[unsafe(no_mangle)]
pub extern "C" fn jk_legend_in(key: u32, shift: u32, escaped: u32, mode: u32) -> u32 {
    let layout = if escaped != 0 { Layout::Escaped } else { Layout::JisKorean };
    let Some(mode) = Mode::from_id(mode) else { return 0 };
    output_in(mode, layout, Key::from_id(key), shift != 0).map_or(0, |o| o.char() as u32)
}

// --- Resolving keys and text (spec §8.3), on a copy of this engine: its mode and predictions

/// # Safety
/// `ptr` must point to `len` readable bytes (or be null).
unsafe fn utf8(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: per the contract above.
    String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(ptr, len) }).into_owned()
}

/// What a key press (`KeyG`, `S-Digit2`) types on its own in this engine's mode. Returns 1 and
/// stores the character (read it with `jk_resolved_*`), or 0 if it types none.
///
/// # Safety
/// `press` must point to `press_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jk_resolve_key_input(handle: *mut JkEngine, press: *const u8, press_len: usize) -> u32 {
    // SAFETY: per the contract above.
    let press = unsafe { utf8(press, press_len) };
    with(handle, 0, |h| store(h, resolve::key_input(&h.engine, &press)))
}

/// What the field shows after `context` (unconfirmed text typed so far) and then the key press.
/// Returns 1 and stores it, or 0 if the press isn't a key or no keys type the context.
///
/// # Safety
/// Each pointer must point to its length in readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jk_resolve_key_input_after(
    handle: *mut JkEngine,
    context: *const u8,
    context_len: usize,
    press: *const u8,
    press_len: usize,
) -> u32 {
    // SAFETY: per the contract above.
    let (context, press) = unsafe { (utf8(context, context_len), utf8(press, press_len)) };
    with(handle, 0, |h| store(h, resolve::key_input_after(&h.engine, &context, &press)))
}

/// Every combination of key presses that types `text`, with `min` to `max` presses. Returns how
/// many, and stores them as JSON (`[["KeyG","KeyK","KeyS"],…]`; `[]` for none, including a
/// nonsensical range).
///
/// # Safety
/// `text` must point to `text_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jk_resolve_keys_for_output(handle: *mut JkEngine, text: *const u8, text_len: usize, min: i64, max: i64) -> u32 {
    // SAFETY: per the contract above.
    let text = unsafe { utf8(text, text_len) };
    with(handle, 0, |h| {
        let found = resolve::keys_for_output(&h.engine, &text, min, max);
        let routes: Vec<String> = found
            .iter()
            .map(|route| format!("[{}]", route.iter().map(|p| format!("\"{p}\"")).collect::<Vec<_>>().join(",")))
            .collect();
        h.resolved = format!("[{}]", routes.join(","));
        found.len() as u32
    })
}

fn store(h: &mut JkEngine, answer: Option<String>) -> u32 {
    h.resolved = answer.clone().unwrap_or_default();
    answer.is_some() as u32
}

/// The last resolve call's answer (UTF-8).
#[unsafe(no_mangle)]
pub extern "C" fn jk_resolved_ptr(handle: *mut JkEngine) -> *const u8 {
    with(handle, std::ptr::null(), |h| h.resolved.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn jk_resolved_len(handle: *mut JkEngine) -> usize {
    with(handle, 0, |h| h.resolved.len())
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

// --- Prediction (spec §10, draft; only with the `predict` feature)

#[cfg(feature = "predict")]
pub use predict_ffi::*;

#[cfg(feature = "predict")]
mod predict_ffi {
    use std::sync::Arc;

    use super::{JkEngine, with};
    use crate::predict::Predictor;

    /// A loaded predictor, shareable by many engines.
    pub struct JkPredictor(Arc<Predictor>);

    unsafe fn text<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
        // SAFETY: callers pass `len` readable bytes at `ptr`.
        (!ptr.is_null()).then(|| unsafe { std::slice::from_raw_parts(ptr, len) })
    }

    /// Loads the predictor from `predict/ranker.bin`, the two word lists and the noun lexicon
    /// (`english-nouns.txt`; may be empty), all UTF-8. Null if anything is malformed.
    ///
    /// # Safety
    /// Each pointer must point to its length in readable bytes.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn jk_predictor_new(
        model: *const u8,
        model_len: usize,
        english: *const u8,
        english_len: usize,
        korean: *const u8,
        korean_len: usize,
        nouns: *const u8,
        nouns_len: usize,
    ) -> *mut JkPredictor {
        // SAFETY: per the contract above.
        let (Some(m), Some(e), Some(k)) =
            (unsafe { text(model, model_len) }, unsafe { text(english, english_len) }, unsafe { text(korean, korean_len) })
        else {
            return std::ptr::null_mut();
        };
        // SAFETY: per the contract above.
        let n = unsafe { text(nouns, nouns_len) }.unwrap_or(&[]);
        match (std::str::from_utf8(e), std::str::from_utf8(k), std::str::from_utf8(n)) {
            (Ok(e), Ok(k), Ok(n)) => match Predictor::parse(m, e, k, n) {
                Ok(p) => Box::into_raw(Box::new(JkPredictor(Arc::new(p)))),
                Err(_) => std::ptr::null_mut(),
            },
            _ => std::ptr::null_mut(),
        }
    }

    /// A new predictor: `base` with the user's own words added (training contract, spec §10.12),
    /// UTF-8 lines of `word` or `word<TAB>count` for Korean and English (either may be empty).
    /// `base` stays valid and unchanged. Null if `base` is null or a list isn't UTF-8.
    ///
    /// # Safety
    /// `base` must come from `jk_predictor_new` or this function; `korean` and `english` must
    /// point to their lengths in readable bytes (or be null with length 0).
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn jk_predictor_with_words(
        base: *const JkPredictor,
        korean: *const u8,
        korean_len: usize,
        english: *const u8,
        english_len: usize,
    ) -> *mut JkPredictor {
        // SAFETY: per the contract above.
        let Some(base) = (unsafe { base.as_ref() }) else { return std::ptr::null_mut() };
        // SAFETY: per the contract above.
        let (k, e) = (unsafe { text(korean, korean_len) }.unwrap_or(&[]), unsafe { text(english, english_len) }.unwrap_or(&[]));
        match (std::str::from_utf8(k), std::str::from_utf8(e)) {
            (Ok(k), Ok(e)) => Box::into_raw(Box::new(JkPredictor(Arc::new(base.0.with_user_words(k, e))))),
            _ => std::ptr::null_mut(),
        }
    }

    /// Learns from one example of what the user meant (training contract, spec §10.12): `keys` as
    /// `KeyboardEvent.code` names separated by spaces, `S-` for Shift (null or empty: the cheapest
    /// keys for `text`). Returns where `text` was among the predictions (0 = top), -1 if it wasn't
    /// listed, -2 if the input is malformed or there's no predictor.
    ///
    /// # Safety
    /// `text` and `keys` must point to their lengths in readable bytes (`keys` may be null with 0).
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn jk_learn_example(
        handle: *mut JkEngine,
        text_ptr: *const u8,
        text_len: usize,
        keys: *const u8,
        keys_len: usize,
    ) -> i32 {
        // SAFETY: per the contract above.
        let Some(Ok(intended)) = (unsafe { text(text_ptr, text_len) }).map(std::str::from_utf8) else { return -2 };
        // SAFETY: per the contract above.
        let keys = match unsafe { text(keys, keys_len) }.map(std::str::from_utf8) {
            None => None,
            Some(Ok(k)) if k.trim().is_empty() => None,
            Some(Ok(k)) => match crate::plan::parse_keys(k) {
                Some(presses) => Some(presses),
                None => return -2,
            },
            Some(Err(_)) => return -2,
        };
        with(handle, -2, |h| {
            if h.engine.predictions_available() {
                h.engine.learn_example(keys.as_deref(), intended).map_or(-1, |at| at as i32)
            } else {
                -2
            }
        })
    }

    /// # Safety
    /// `predictor` must come from `jk_predictor_new` and not be used afterwards.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn jk_predictor_free(predictor: *mut JkPredictor) {
        if !predictor.is_null() {
            // SAFETY: per the contract above.
            drop(unsafe { Box::from_raw(predictor) });
        }
    }

    /// Turns prediction on with `predictor`, or off (the lite engine) with null.
    ///
    /// # Safety
    /// `predictor` must be null or come from `jk_predictor_new` and not be freed yet.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn jk_set_predictor(handle: *mut JkEngine, predictor: *const JkPredictor) {
        // SAFETY: per the contract above.
        let predictor = unsafe { predictor.as_ref() }.map(|p| p.0.clone());
        with(handle, (), |h| h.engine.set_predictor(predictor))
    }

    /// Ranks the staged text's readings; returns how many (0 without a predictor).
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_predict(handle: *mut JkEngine) -> usize {
        with(handle, 0, |h| {
            h.predictions = h.engine.predictions();
            h.predictions.len()
        })
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn jk_prediction_ptr(handle: *mut JkEngine, index: usize) -> *const u8 {
        with(handle, std::ptr::null(), |h| h.predictions.get(index).map_or(std::ptr::null(), |p| p.text.as_ptr()))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn jk_prediction_len(handle: *mut JkEngine, index: usize) -> usize {
        with(handle, 0, |h| h.predictions.get(index).map_or(0, |p| p.text.len()))
    }

    /// The prediction's kind (`JK_PREDICTION_*`), or 255 if out of range.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_prediction_kind(handle: *mut JkEngine, index: usize) -> u32 {
        with(handle, 255, |h| h.predictions.get(index).map_or(255, |p| p.kind as u32))
    }

    /// Tunes one scenario (`JK_WEIGHT_*`): a multiplier on its learned probability, 1.0 as learned,
    /// clamped to 0.1–10. Unknown ids are ignored.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_set_prediction_weight(handle: *mut JkEngine, which: u32, value: f64) {
        if let Some(which) = crate::predict::Weight::from_id(which) {
            with(handle, (), |h| h.engine.set_prediction_weight(which, value))
        }
    }

    /// Hands a scenario's weight back to learning (or 1.0 when learning is off). Unknown ids are
    /// ignored.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_clear_prediction_weight(handle: *mut JkEngine, which: u32) {
        if let Some(which) = crate::predict::Weight::from_id(which) {
            with(handle, (), |h| h.engine.clear_prediction_weight(which))
        }
    }

    /// Learning from the user's choices (spec §10.11), off by default.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_set_learning(handle: *mut JkEngine, on: u32) {
        with(handle, (), |h| h.engine.set_learning(on != 0))
    }

    /// Whether choices are recorded while learning is on (on by default). Off for an app the user
    /// excluded from learning: what was learned still applies there.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_set_recording(handle: *mut JkEngine, on: u32) {
        with(handle, (), |h| h.engine.set_recording(on != 0))
    }

    /// The learned multiplier for a scenario (`JK_WEIGHT_*`); 1.0 for unknown ids.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_learned_weight(handle: *mut JkEngine, which: u32) -> f64 {
        match crate::predict::Weight::from_id(which) {
            Some(which) => with(handle, 1.0, |h| h.engine.learner().weight(which)),
            None => 1.0,
        }
    }

    /// 1 if what was learned changed since the last call: store `jk_learner_export` then.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_learner_changed(handle: *mut JkEngine) -> u32 {
        with(handle, 0, |h| h.engine.take_learned_changed() as u32)
    }

    /// What was learned, as one line of text (read it with `jk_learner_ptr`); returns its length.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_learner_export(handle: *mut JkEngine) -> usize {
        with(handle, 0, |h| {
            h.learned = h.engine.learner().export();
            h.learned.len()
        })
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn jk_learner_ptr(handle: *mut JkEngine) -> *const u8 {
        with(handle, std::ptr::null(), |h| h.learned.as_ptr())
    }

    /// Restores an export (UTF-8). Returns 1 if it was taken, 0 if it wasn't one (nothing changes).
    ///
    /// # Safety
    /// `ptr` must point to `len` readable bytes.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn jk_learner_import(handle: *mut JkEngine, ptr: *const u8, len: usize) -> u32 {
        // SAFETY: as documented.
        let Some(bytes) = (unsafe { text(ptr, len) }) else { return 0 };
        let Some(learner) = std::str::from_utf8(bytes).ok().and_then(crate::learn::Learner::import) else { return 0 };
        with(handle, 0, |h| {
            h.engine.set_learner(learner);
            1
        })
    }

    /// Forgets everything learned.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_learner_reset(handle: *mut JkEngine) {
        with(handle, (), |h| h.engine.set_learner(crate::learn::Learner::default()))
    }

    /// Chooses the ranker for the implied-ㅇ alternatives (`JK_RANKER_*`). Unknown ids are ignored.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_set_ranker(handle: *mut JkEngine, ranker: u32) {
        if let Some(ranker) = crate::predict::Ranker::from_id(ranker) {
            with(handle, (), |h| h.engine.set_ranker(ranker))
        }
    }

    /// A committing key while prediction `index` shows: the prediction stands in for the
    /// unconfirmed text, then the key acts on it. Returns 1 if the key was consumed; read the
    /// text with `jk_commit_*` and `jk_preedit_*`.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_key_after_prediction(handle: *mut JkEngine, index: usize, key: u32, mods: u32) -> u32 {
        with(handle, 0, |h| {
            h.last = h.engine.key_after_prediction(index, crate::Key::from_id(key), crate::Mods::from_bits(mods));
            h.predictions.clear();
            h.last.consumed as u32
        })
    }

    /// Commits prediction `index` because the host must (focus loss): learns nothing. Read the
    /// text with `jk_commit_*`.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_flush_prediction(handle: *mut JkEngine, index: usize) {
        with(handle, (), |h| {
            h.last = h.engine.flush_prediction(index);
            h.predictions.clear();
        })
    }

    /// Commits prediction `index`; read the text with `jk_commit_*`.
    #[unsafe(no_mangle)]
    pub extern "C" fn jk_commit_prediction(handle: *mut JkEngine, index: usize) {
        with(handle, (), |h| {
            h.last = h.engine.commit_prediction(index);
            h.predictions.clear();
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_leaves_no_output_to_apply_again() {
        let handle = jk_new();
        jk_key(handle, Key::from_code("KeyG") as u32, 0);
        jk_key(handle, Key::from_code("Space") as u32, 0);
        let consumed = with(handle, 7, |h| {
            assert_eq!(h.last.commit, "ㅎ ");
            panic!("simulated");
        });
        assert_eq!(consumed, 7);
        // SAFETY: the handle is live.
        let h = unsafe { &*handle };
        assert_eq!(h.last, Output::default());
        assert!(h.engine.preedit().is_empty());
        // SAFETY: from jk_new, freed once.
        unsafe { jk_free(handle) };
    }
}
