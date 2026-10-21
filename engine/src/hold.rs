//! JIS English's press and hold (spec §3.6.3): holding a letter key offers its accented and other
//! variants, as macOS does for English keyboard layouts.

use crate::key::Key;

/// A letter held down, with its list open.
#[derive(Clone, Debug)]
pub struct Held {
    /// The key held.
    pub key: Key,
    /// The letter as typed, taken back into the preedit.
    pub letter: char,
    pub variants: &'static [char],
    /// The variant chosen with the arrows, if any; the letter shows until one is.
    pub selected: Option<usize>,
}

impl Held {
    /// What the preedit shows, and what commits if nothing else is picked.
    pub fn shown(&self) -> char {
        self.selected.map_or(self.letter, |i| self.variants[i])
    }
}

/// The variants of a letter key, in list order (numbered 1, 2, … in the list), or `None` for keys
/// without any. macOS's English (US) press-and-hold set.
pub fn variants(key: Key, shift: bool) -> Option<&'static [char]> {
    use Key::*;
    let (lower, upper): (&'static [char], &'static [char]) = match key {
        KeyA => (&['à', 'á', 'â', 'ä', 'æ', 'ã', 'å', 'ā'], &['À', 'Á', 'Â', 'Ä', 'Æ', 'Ã', 'Å', 'Ā']),
        KeyC => (&['ç', 'ć', 'č'], &['Ç', 'Ć', 'Č']),
        KeyE => (&['è', 'é', 'ê', 'ë', 'ē', 'ė', 'ę'], &['È', 'É', 'Ê', 'Ë', 'Ē', 'Ė', 'Ę']),
        KeyI => (&['î', 'ï', 'í', 'ī', 'į', 'ì'], &['Î', 'Ï', 'Í', 'Ī', 'Į', 'Ì']),
        KeyL => (&['ł'], &['Ł']),
        KeyN => (&['ñ', 'ń'], &['Ñ', 'Ń']),
        KeyO => (&['ô', 'ö', 'ò', 'ó', 'œ', 'ø', 'ō', 'õ'], &['Ô', 'Ö', 'Ò', 'Ó', 'Œ', 'Ø', 'Ō', 'Õ']),
        KeyS => (&['ß', 'ś', 'š'], &['Ś', 'Š']),
        KeyU => (&['û', 'ü', 'ù', 'ú', 'ū'], &['Û', 'Ü', 'Ù', 'Ú', 'Ū']),
        KeyY => (&['ÿ'], &['Ÿ']),
        KeyZ => (&['ž', 'ź', 'ż'], &['Ž', 'Ź', 'Ż']),
        _ => return None,
    };
    Some(if shift { upper } else { lower })
}
