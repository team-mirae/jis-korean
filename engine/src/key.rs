//! Physical keys, named by their W3C `KeyboardEvent.code` (spec §2).

macro_rules! keys {
    ($($name:ident = $code:literal),* $(,)?) => {
        /// A physical key position. The discriminant is the stable id used over the C ABI
        /// (`JK_KEY_*` in `include/jis_korean.h`), so only ever append before `Modifier`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Key { $($name),* }

        impl Key {
            pub const ALL: &'static [Key] = &[$(Key::$name),*];

            /// The canonical `KeyboardEvent.code` name.
            pub fn code(self) -> &'static str {
                match self { $(Key::$name => $code),* }
            }

            fn from_canonical(code: &str) -> Option<Key> {
                match code { $($code => Some(Key::$name),)* _ => None }
            }
        }
    };
}

keys! {
    Digit1 = "Digit1", Digit2 = "Digit2", Digit3 = "Digit3", Digit4 = "Digit4", Digit5 = "Digit5",
    Digit6 = "Digit6", Digit7 = "Digit7", Digit8 = "Digit8", Digit9 = "Digit9", Digit0 = "Digit0",
    Minus = "Minus", Equal = "Equal", IntlYen = "IntlYen",
    KeyQ = "KeyQ", KeyW = "KeyW", KeyE = "KeyE", KeyR = "KeyR", KeyT = "KeyT",
    KeyY = "KeyY", KeyU = "KeyU", KeyI = "KeyI", KeyO = "KeyO", KeyP = "KeyP",
    BracketLeft = "BracketLeft", BracketRight = "BracketRight",
    KeyA = "KeyA", KeyS = "KeyS", KeyD = "KeyD", KeyF = "KeyF", KeyG = "KeyG",
    KeyH = "KeyH", KeyJ = "KeyJ", KeyK = "KeyK", KeyL = "KeyL",
    Semicolon = "Semicolon", Quote = "Quote", Backslash = "Backslash",
    KeyZ = "KeyZ", KeyX = "KeyX", KeyC = "KeyC", KeyV = "KeyV",
    KeyB = "KeyB", KeyN = "KeyN", KeyM = "KeyM",
    Comma = "Comma", Period = "Period", Slash = "Slash", IntlRo = "IntlRo",
    F6 = "F6", F7 = "F7", F8 = "F8", F9 = "F9", F10 = "F10",
    Space = "Space", Enter = "Enter", Backspace = "Backspace",
    // Used by the hanja candidate list; elsewhere they commit and pass through like any other key.
    Escape = "Escape", ArrowUp = "ArrowUp", ArrowDown = "ArrowDown",
    ArrowLeft = "ArrowLeft", ArrowRight = "ArrowRight",
    // Left of 1 on US ANSI (right end of the number row on an HHKB). Mode B only (spec §3.6).
    Backquote = "Backquote",
    // Between left Shift and Z on ISO keyboards. Mode C only (spec §3.6).
    IntlBackslash = "IntlBackslash",
    // A modifier pressed on its own: ignored, never commits.
    Modifier = "Modifier",
    // Any other key: commits, then passes through to the app.
    Other = "Other",
}

const MODIFIER_CODES: &[&str] = &[
    "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight", "AltLeft", "AltRight",
    "MetaLeft", "MetaRight", "CapsLock", "Fn",
];

impl Key {
    /// Maps any `KeyboardEvent.code` to a key. Unknown codes become [`Key::Other`].
    pub fn from_code(code: &str) -> Key {
        if MODIFIER_CODES.contains(&code) {
            return Key::Modifier;
        }
        Key::from_canonical(code).unwrap_or(Key::Other)
    }

    /// The key with this C ABI id. Out-of-range ids become [`Key::Other`].
    pub fn from_id(id: u32) -> Key {
        Key::ALL.get(id as usize).copied().unwrap_or(Key::Other)
    }

    /// The lowercase Latin letter on a letter key.
    pub fn letter(self) -> Option<char> {
        let rest = self.code().strip_prefix("Key")?;
        let mut chars = rest.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Some(c.to_ascii_lowercase()),
            _ => None,
        }
    }

    /// The digit on a number-row key.
    pub fn digit(self) -> Option<char> {
        let rest = self.code().strip_prefix("Digit")?;
        rest.chars().next()
    }
}

/// Modifier state for one key event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Mods {
    pub const SHIFT: u32 = 1;
    pub const CTRL: u32 = 2;
    pub const ALT: u32 = 4;
    pub const META: u32 = 8;

    pub const NONE: Mods = Mods { shift: false, ctrl: false, alt: false, meta: false };
    pub const SHIFTED: Mods = Mods { shift: true, ctrl: false, alt: false, meta: false };

    pub fn from_bits(bits: u32) -> Mods {
        Mods {
            shift: bits & Self::SHIFT != 0,
            ctrl: bits & Self::CTRL != 0,
            alt: bits & Self::ALT != 0,
            meta: bits & Self::META != 0,
        }
    }

    /// Ctrl, Alt/Option or Cmd/Win held: the keystroke is a shortcut for the app (spec §3.5).
    pub fn is_shortcut(self) -> bool {
        self.ctrl || self.alt || self.meta
    }
}
