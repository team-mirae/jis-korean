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
    JK_KEY_MODIFIER = 56,
    JK_KEY_OTHER = 57,
};

enum {
    JK_MOD_SHIFT = 1,
    JK_MOD_CTRL = 2,
    JK_MOD_ALT = 4,  /* Alt / Option */
    JK_MOD_META = 8, /* Cmd / Win */
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

#ifdef __cplusplus
}
#endif

#endif
