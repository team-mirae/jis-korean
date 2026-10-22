/* JIS Korean engine C ABI. Matches engine/src/ffi.rs; the key ids are checked by
 * engine/tests/header.rs.
 *
 * Usage: create one engine per input context, call jk_key for each key press, then insert
 * jk_commit_* and show jk_preedit_* as marked text. If jk_key returns 0, pass the key on to
 * the app after that. Strings are UTF-8, not NUL-terminated, and valid until the next call on
 * the same handle. Not thread-safe: use a handle from one thread at a time.
 */
#ifndef JIS_KOREAN_H
#define JIS_KOREAN_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct JkEngine JkEngine;

/* Physical keys, by W3C KeyboardEvent.code. Map pure modifier presses to JK_KEY_MODIFIER and
 * any key not listed to JK_KEY_OTHER. */
enum {
    JK_KEY_DIGIT1 = 0,
    JK_KEY_DIGIT2 = 1,
    JK_KEY_DIGIT3 = 2,
    JK_KEY_DIGIT4 = 3,
    JK_KEY_DIGIT5 = 4,
    JK_KEY_DIGIT6 = 5,
    JK_KEY_DIGIT7 = 6,
    JK_KEY_DIGIT8 = 7,
    JK_KEY_DIGIT9 = 8,
    JK_KEY_DIGIT0 = 9,
    JK_KEY_MINUS = 10,
    JK_KEY_EQUAL = 11,
    JK_KEY_INTLYEN = 12,
    JK_KEY_KEYQ = 13,
    JK_KEY_KEYW = 14,
    JK_KEY_KEYE = 15,
    JK_KEY_KEYR = 16,
    JK_KEY_KEYT = 17,
    JK_KEY_KEYY = 18,
    JK_KEY_KEYU = 19,
    JK_KEY_KEYI = 20,
    JK_KEY_KEYO = 21,
    JK_KEY_KEYP = 22,
    JK_KEY_BRACKETLEFT = 23,
    JK_KEY_BRACKETRIGHT = 24,
    JK_KEY_KEYA = 25,
    JK_KEY_KEYS = 26,
    JK_KEY_KEYD = 27,
    JK_KEY_KEYF = 28,
    JK_KEY_KEYG = 29,
    JK_KEY_KEYH = 30,
    JK_KEY_KEYJ = 31,
    JK_KEY_KEYK = 32,
    JK_KEY_KEYL = 33,
    JK_KEY_SEMICOLON = 34,
    JK_KEY_QUOTE = 35,
    JK_KEY_BACKSLASH = 36,
    JK_KEY_KEYZ = 37,
    JK_KEY_KEYX = 38,
    JK_KEY_KEYC = 39,
    JK_KEY_KEYV = 40,
    JK_KEY_KEYB = 41,
    JK_KEY_KEYN = 42,
    JK_KEY_KEYM = 43,
    JK_KEY_COMMA = 44,
    JK_KEY_PERIOD = 45,
    JK_KEY_SLASH = 46,
    JK_KEY_INTLRO = 47,
    JK_KEY_F6 = 48,
    JK_KEY_F7 = 49,
    JK_KEY_F8 = 50,
    JK_KEY_F9 = 51,
    JK_KEY_F10 = 52,
    JK_KEY_SPACE = 53,
    JK_KEY_ENTER = 54,
    JK_KEY_BACKSPACE = 55,
    JK_KEY_ESCAPE = 56,
    JK_KEY_ARROWUP = 57,
    JK_KEY_ARROWDOWN = 58,
    JK_KEY_ARROWLEFT = 59,
    JK_KEY_ARROWRIGHT = 60,
    JK_KEY_BACKQUOTE = 61,
    JK_KEY_INTLBACKSLASH = 62,
    JK_KEY_MODIFIER = 63,
    JK_KEY_OTHER = 64,
};

enum {
    JK_MOD_SHIFT = 1,
    JK_MOD_CTRL = 2,
    JK_MOD_ALT = 4,  /* Alt / Option */
    JK_MOD_META = 8, /* Cmd / Win */
    JK_MOD_REPEAT = 16, /* the key's auto-repeat while held (an English mode's variants list ignores it) */
    JK_MOD_CAPS = 32,   /* Caps Lock on (the English modes' variants lists offer capitals) */
};

JkEngine *jk_new(void);
void jk_free(JkEngine *engine);

/* Returns 1 if the key was consumed, 0 to pass it on to the app. */
uint32_t jk_key(JkEngine *engine, uint32_t key, uint32_t mods);
/* Commits everything unconfirmed: focus loss, app switch, IME switch. */
void jk_flush(JkEngine *engine);
/* The IME was switched back to: leaves any F8 escape. */
void jk_activate(JkEngine *engine);
void jk_set_enter_commits(JkEngine *engine, uint32_t on);
/* The mode, by id (spec §3.6.1): 0 = Mode A (JIS, the default), 1 = Mode B (US ANSI),
 * 2 = Mode C (UK ISO). Returns 1 if it switched, 0 for an id this build doesn't have (the mode
 * stays as it was). */
uint32_t jk_set_mode(JkEngine *engine, uint32_t mode);
/* Dedicated symbol keys (spec §3.6), on (1) by default. Off (0), Mode C's [ ] and ISO extra key
 * type their UK ISO characters ([ { ] } \ |). Modes without optional symbol keys ignore it. */
void jk_set_dedicated_symbols(JkEngine *engine, uint32_t on);
/* How many modes this build has. Ids have gaps: list them with jk_mode_id_at, never 0..count. */
uint32_t jk_mode_count(void);
/* The id of the index-th mode (0 to count - 1, menu order), or UINT32_MAX past the end. */
uint32_t jk_mode_id_at(uint32_t index);
/* A mode's display name (static UTF-8), for settings menus. Empty for an unknown id. */
const uint8_t *jk_mode_name_ptr(uint32_t mode);
size_t jk_mode_name_len(uint32_t mode);
/* A mode's mode name ("A", "A_QWERTY"; spec §3.6.1). Empty for an unknown id. */
const uint8_t *jk_mode_code_ptr(uint32_t mode);
size_t jk_mode_code_len(uint32_t mode);
uint32_t jk_is_escaped(JkEngine *engine);

const uint8_t *jk_commit_ptr(JkEngine *engine);
size_t jk_commit_len(JkEngine *engine);
const uint8_t *jk_preedit_ptr(JkEngine *engine);
size_t jk_preedit_len(JkEngine *engine);
uint32_t jk_key_count(void);
const uint8_t *jk_key_code_ptr(uint32_t key);
size_t jk_key_code_len(uint32_t key);
/* The character a key types (Unicode scalar), or 0 if none. For drawing key legends. */
uint32_t jk_legend(uint32_t key, uint32_t shift, uint32_t escaped);
/* The same in a given mode, by id; 0 for an unknown id. */
uint32_t jk_legend_in(uint32_t key, uint32_t shift, uint32_t escaped, uint32_t mode);

/* Resolving keys and text (spec §8.3), on a copy of this engine: its mode and, with predictions
 * on, its predictor and weights. A key press is a KeyboardEvent.code, with "S-" for Shift held
 * ("KeyG", "S-Digit2"). Read each answer with jk_resolved_* until the next call. */
/* What the press types on its own: 1 and the character, or 0. */
uint32_t jk_resolve_key_input(JkEngine *engine, const uint8_t *press, size_t press_len);
/* What the field shows after context (unconfirmed text) and the press: 1 and the text, or 0. */
uint32_t jk_resolve_key_input_after(JkEngine *engine, const uint8_t *context, size_t context_len,
                                    const uint8_t *press, size_t press_len);
/* Every combination of presses that types text, min to max presses: the count, and JSON
 * [["KeyG","KeyK","KeyS"],...]. A nonsensical range gives 0 and []. */
uint32_t jk_resolve_keys_for_output(JkEngine *engine, const uint8_t *text, size_t text_len,
                                    int64_t min, int64_t max);
const uint8_t *jk_resolved_ptr(JkEngine *engine);
size_t jk_resolved_len(JkEngine *engine);

/* Hanja (F7). Parse the dictionary once and share it; engines keep their own reference. */
typedef struct JkDict JkDict;
JkDict *jk_dict_new(const uint8_t *tsv, size_t len); /* NULL if not UTF-8 */
void jk_dict_free(JkDict *dict);
void jk_set_dictionary(JkEngine *engine, const JkDict *dict);

/* The open candidate list on the last result. Count 0 = no list. Show a page of
 * jk_candidate_page_size() around the selected one; keys 1-9 pick on that page. A selected index
 * equal to the count means none is selected (JIS English's press-and-hold list, spec §3.6.3). */
size_t jk_candidate_count(JkEngine *engine);
size_t jk_candidate_selected(JkEngine *engine);
size_t jk_candidate_page_size(void);
const uint8_t *jk_candidate_ptr(JkEngine *engine, size_t index);
size_t jk_candidate_len(JkEngine *engine, size_t index);
const uint8_t *jk_candidate_comment_ptr(JkEngine *engine, size_t index);
size_t jk_candidate_comment_len(JkEngine *engine, size_t index);

/* The last result's keystrokes read as English on the mode's stock keyboard (letters with case, digits,
 * symbols), for hosts that choose between Korean and English themselves. */
const uint8_t *jk_english_ptr(JkEngine *engine);
size_t jk_english_len(JkEngine *engine);
/* Discards everything unconfirmed without committing it. */
void jk_clear(JkEngine *engine);

/* Key presses for a text on JIS Korean and on conventional 2-set (plan.rs; Shift counts as a
 * press, symbols off a layout's legends count 3). Uses the engine's hanja dictionary if set. */
void jk_plan(JkEngine *engine, const uint8_t *text, size_t len);
size_t jk_plan_jis(JkEngine *engine);
size_t jk_plan_conventional(JkEngine *engine);
size_t jk_plan_chars(JkEngine *engine);
size_t jk_plan_off_keyboard(JkEngine *engine); /* characters no key types, counted 3 each */
size_t jk_plan_dropped(JkEngine *engine);

/* Prediction (spec §10, draft). Only in builds with the `predict` feature (the default); the lite
 * build has none of these. Without a predictor set, jk_predict returns 0: the deterministic engine. */
typedef struct JkPredictor JkPredictor;
enum {
    JK_PREDICTION_KOREAN = 0,
    JK_PREDICTION_ENGLISH = 1,
    JK_PREDICTION_ENGLISH_CAPITALIZED = 2,
    JK_PREDICTION_ENGLISH_UPPER = 3,
    JK_PREDICTION_TWO_SET = 4,
    JK_PREDICTION_KOREAN_KEEP_FINALS = 5,
    JK_PREDICTION_HANJA = 6,
    JK_PREDICTION_BARE_VOWELS = 7,
    JK_PREDICTION_MIXED = 8,
    JK_PREDICTION_IMPLIED_L = 9, /* an implied ㄹ after a ㄴ final: 전력 for 저녁 */
};
JkPredictor *jk_predictor_new(const uint8_t *model, size_t model_len, const uint8_t *english_words,
                              size_t english_len, const uint8_t *korean_words, size_t korean_len,
                              const uint8_t *english_nouns, size_t nouns_len); /* nouns may be NULL */
void jk_predictor_free(JkPredictor *predictor);
void jk_set_predictor(JkEngine *engine, const JkPredictor *predictor); /* NULL: lite */
size_t jk_predict(JkEngine *engine); /* ranks the staged text; returns the count */
const uint8_t *jk_prediction_ptr(JkEngine *engine, size_t index);
size_t jk_prediction_len(JkEngine *engine, size_t index);
uint32_t jk_prediction_kind(JkEngine *engine, size_t index);
void jk_commit_prediction(JkEngine *engine, size_t index); /* then read jk_commit_* */
/* Commits prediction `index` because the host must (focus loss): learns nothing. */
void jk_flush_prediction(JkEngine *engine, size_t index);
/* A committing key (Space, Enter, a shortcut, ...) while prediction `index` shows: the prediction
 * stands in for the unconfirmed text, then the key acts on it. Returns 1 if consumed. */
uint32_t jk_key_after_prediction(JkEngine *engine, size_t index, uint32_t key, uint32_t mods);
/* User tuning (spec §10.5): a multiplier on each scenario's learned probability, 1.0 = as learned,
 * clamped to 0.1-10; 0 turns an alternative off (not Korean or English). */
enum {
    JK_WEIGHT_KOREAN = 0,
    JK_WEIGHT_ENGLISH = 1,
    JK_WEIGHT_IMPLIED = 2,
    JK_WEIGHT_MIXED = 3,
    JK_WEIGHT_TWO_SET = 4,
    JK_WEIGHT_KEEP_FINALS = 5,
    JK_WEIGHT_HANJA = 6,
    JK_WEIGHT_IMPLIED_L = 7, /* 0 turns it off */
};
void jk_set_prediction_weight(JkEngine *engine, uint32_t which, double value);
/* How the implied-ㅇ alternatives (ㅠㅠ vs 유유, 말이 vs 마리) are ranked (spec §10.7): the rules, or
 * the effort a typist who meant them skipped (experimental). Unknown ids are ignored. */
enum {
    JK_RANKER_RULES = 0,
    JK_RANKER_TRAVEL = 1,
    JK_RANKER_ROUTE = 2, /* route preference (spec §10.8, experimental) */
};
void jk_set_ranker(JkEngine *engine, uint32_t ranker);
/* Learning from the user's choices (spec §10.11): opt-in, on the device only. The engine records
 * choices itself when predictions are committed; the host only stores the export and turns it on.
 * A weight set with jk_set_prediction_weight wins over the learned one until cleared. */
void jk_clear_prediction_weight(JkEngine *engine, uint32_t which);
void jk_set_learning(JkEngine *engine, uint32_t on);
void jk_set_recording(JkEngine *engine, uint32_t on);       /* 0: learned weights apply, nothing recorded */
double jk_learned_weight(JkEngine *engine, uint32_t which); /* 0.5-2, 1.0 = no change */
uint32_t jk_learner_changed(JkEngine *engine);              /* 1 once after each change */
size_t jk_learner_export(JkEngine *engine);                 /* then read jk_learner_ptr */
const uint8_t *jk_learner_ptr(JkEngine *engine);
uint32_t jk_learner_import(JkEngine *engine, const uint8_t *text, size_t len); /* 1 if taken */
void jk_learner_reset(JkEngine *engine);
/* The training contract (docs/training-contract.md, spec §10.12). A predictor with the user's own
 * words added (lines of `word` or `word<TAB>count`; the base stays as it is): */
JkPredictor *jk_predictor_with_words(const JkPredictor *base, const uint8_t *korean, size_t korean_len,
                                     const uint8_t *english, size_t english_len);
/* Learns from an example: keys as KeyboardEvent.code names, "S-" for Shift (null/empty: the cheapest
 * keys for the text). Returns its place among the predictions (0 = top), -1 not listed, -2 bad input. */
int32_t jk_learn_example(JkEngine *engine, const uint8_t *text, size_t text_len, const uint8_t *keys,
                         size_t keys_len);

/* Memory in the engine's address space, for wasm hosts passing the dictionary in. */
uint8_t *jk_alloc(size_t len);
void jk_dealloc(uint8_t *ptr, size_t len);

#ifdef __cplusplus
}
#endif

#endif
