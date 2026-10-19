//! Finger travel on a JIS keyboard: how far the fingers move to press a sequence of keys.
//!
//! Positions are key centers in key units (1 = one key width) on a JIS 109 board: the number row
//! at y = 0, the letter rows below it, the function row 1.5 above. Each key belongs to one finger
//! (standard touch typing; F6–F10 to the finger under them). A press moves its finger from where
//! it is to the key; every other finger that's away from home returns there, which counts too.
//! Modifiers move nothing (Shift is held, not travelled to), and thumbs rest on Space.

use crate::Key;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Finger {
    LeftPinky,
    LeftRing,
    LeftMiddle,
    LeftIndex,
    RightIndex,
    RightMiddle,
    RightRing,
    RightPinky,
}

const FINGERS: usize = 8;

/// Home row keys, per finger.
const HOME: [Key; FINGERS] = [Key::KeyA, Key::KeyS, Key::KeyD, Key::KeyF, Key::KeyJ, Key::KeyK, Key::KeyL, Key::Semicolon];

/// Where a key is, and which finger presses it. None: a thumb key, a modifier, or not on the board.
fn place(key: Key) -> Option<(f64, f64, Finger)> {
    use Finger::*;
    use Key::*;
    let (x, y, finger) = match key {
        F6 => (8.0, -1.5, RightIndex),
        F7 => (9.0, -1.5, RightMiddle),
        F8 => (10.0, -1.5, RightRing),
        F9 => (11.5, -1.5, RightPinky),
        F10 => (12.5, -1.5, RightPinky),
        Digit1 => (1.5, 0.0, LeftPinky),
        Digit2 => (2.5, 0.0, LeftRing),
        Digit3 => (3.5, 0.0, LeftMiddle),
        Digit4 => (4.5, 0.0, LeftIndex),
        Digit5 => (5.5, 0.0, LeftIndex),
        Digit6 => (6.5, 0.0, RightIndex),
        Digit7 => (7.5, 0.0, RightIndex),
        Digit8 => (8.5, 0.0, RightMiddle),
        Digit9 => (9.5, 0.0, RightRing),
        Digit0 => (10.5, 0.0, RightPinky),
        Minus => (11.5, 0.0, RightPinky),
        Equal => (12.5, 0.0, RightPinky),
        IntlYen => (13.5, 0.0, RightPinky),
        Backspace => (14.5, 0.0, RightPinky),
        KeyQ => (2.0, 1.0, LeftPinky),
        KeyW => (3.0, 1.0, LeftRing),
        KeyE => (4.0, 1.0, LeftMiddle),
        KeyR => (5.0, 1.0, LeftIndex),
        KeyT => (6.0, 1.0, LeftIndex),
        KeyY => (7.0, 1.0, RightIndex),
        KeyU => (8.0, 1.0, RightIndex),
        KeyI => (9.0, 1.0, RightMiddle),
        KeyO => (10.0, 1.0, RightRing),
        KeyP => (11.0, 1.0, RightPinky),
        BracketLeft => (12.0, 1.0, RightPinky),
        BracketRight => (13.0, 1.0, RightPinky),
        Enter => (14.25, 1.5, RightPinky),
        KeyA => (2.25, 2.0, LeftPinky),
        KeyS => (3.25, 2.0, LeftRing),
        KeyD => (4.25, 2.0, LeftMiddle),
        KeyF => (5.25, 2.0, LeftIndex),
        KeyG => (6.25, 2.0, LeftIndex),
        KeyH => (7.25, 2.0, RightIndex),
        KeyJ => (8.25, 2.0, RightIndex),
        KeyK => (9.25, 2.0, RightMiddle),
        KeyL => (10.25, 2.0, RightRing),
        Semicolon => (11.25, 2.0, RightPinky),
        Quote => (12.25, 2.0, RightPinky),
        Backslash => (13.25, 2.0, RightPinky),
        KeyZ => (2.75, 3.0, LeftPinky),
        KeyX => (3.75, 3.0, LeftRing),
        KeyC => (4.75, 3.0, LeftMiddle),
        KeyV => (5.75, 3.0, LeftIndex),
        KeyB => (6.75, 3.0, LeftIndex),
        KeyN => (7.75, 3.0, RightIndex),
        KeyM => (8.75, 3.0, RightIndex),
        Comma => (9.75, 3.0, RightMiddle),
        Period => (10.75, 3.0, RightRing),
        Slash => (11.75, 3.0, RightPinky),
        IntlRo => (12.75, 3.0, RightPinky),
        _ => return None,
    };
    Some((x, y, finger))
}

fn position(key: Key) -> (f64, f64) {
    place(key).map_or((0.0, 0.0), |(x, y, _)| (x, y))
}

/// Where each finger is. Starts with every finger on its home key.
#[derive(Clone, Debug, PartialEq)]
pub struct Hand {
    at: [(f64, f64); FINGERS],
    /// Total travel so far, in key units.
    pub travel: f64,
}

impl Default for Hand {
    fn default() -> Self {
        Hand { at: HOME.map(position), travel: 0.0 }
    }
}

impl Hand {
    /// Presses `key`, returning how far the fingers moved for it.
    pub fn press(&mut self, key: Key) -> f64 {
        let Some((x, y, finger)) = place(key) else { return 0.0 };
        let mut moved = 0.0;
        for (f, (at, home)) in self.at.iter_mut().zip(HOME).enumerate() {
            let target = if f == finger as usize { (x, y) } else { position(home) };
            moved += distance(*at, target);
            *at = target;
        }
        self.travel += moved;
        moved
    }
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// The travel for pressing `keys` in order from the home row.
pub fn travel(keys: impl IntoIterator<Item = Key>) -> f64 {
    let mut hand = Hand::default();
    keys.into_iter().for_each(|k| {
        hand.press(k);
    });
    hand.travel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_keys_cost_nothing_and_reaches_cost_their_distance() {
        assert_eq!(travel([Key::KeyA, Key::KeyS, Key::KeyD, Key::KeyF, Key::KeyJ]), 0.0);
        // J → U is one row up, a quarter key left: then back home on the next press.
        let up = distance(position(Key::KeyJ), position(Key::KeyU));
        assert!((travel([Key::KeyU]) - up).abs() < 1e-9);
        assert!((travel([Key::KeyU, Key::KeyA]) - 2.0 * up).abs() < 1e-9);
        // The same finger again moves from where it is, not from home.
        let across = distance(position(Key::KeyU), position(Key::KeyN));
        assert!((travel([Key::KeyU, Key::KeyN]) - up - across).abs() < 1e-9);
        // F6 is far.
        assert!(travel([Key::F6]) > 3.0);
    }

    #[test]
    fn modifiers_and_thumbs_move_nothing() {
        assert_eq!(travel([Key::Modifier, Key::Space, Key::Other]), 0.0);
    }
}
