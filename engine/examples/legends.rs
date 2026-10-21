//! Prints each key's JIS Korean legends in one mode, unshifted and with Shift, by C ABI key id, for
//! shells that draw or bundle the layout (macOS builds its Keyboard Viewer layouts from this; spec
//! §3, §3.6).
//! Usage: cargo run --example legends [mode id]   (default 0, Mode A; lines of
//! `id<TAB>unshifted<TAB>shifted`; F10 where Shift+Z is F10)

use jis_korean::{Key, Layout, Mode, output_in};

fn main() {
    let id = std::env::args().nth(1).and_then(|id| id.parse().ok()).unwrap_or(0);
    let Some(mode) = Mode::from_id(id) else {
        eprintln!("no mode {id} in this build");
        std::process::exit(1);
    };
    for (id, &key) in Key::ALL.iter().enumerate() {
        let legend = |shift: bool| {
            if key == Key::KeyZ && shift && mode.is_korean() {
                // Shift+Z is F10 (spec §6), not a character.
                return "F10".to_string();
            }
            output_in(mode, Layout::JisKorean, key, shift).map_or(String::new(), |out| out.char().to_string())
        };
        let (base, shifted) = (legend(false), legend(true));
        if !base.is_empty() || !shifted.is_empty() {
            println!("{id}\t{base}\t{shifted}");
        }
    }
}
