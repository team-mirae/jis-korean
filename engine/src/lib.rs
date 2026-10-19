//! Core engine of JIS Korean. All behavior is defined in `docs/spec.md`.
//!
//! Rust callers use [`Engine`]. The macOS, Windows and web shells use the C ABI in [`ffi`]
//! (`include/jis_korean.h`).

mod engine;
pub mod ffi;
mod hangul;
mod hanja;
mod key;
mod layout;
pub mod plan;

pub use engine::{Candidates, Engine, Output, PAGE_SIZE, full_width};
pub use hanja::{Candidate, Dictionary};
pub use key::{Key, Mods};
pub use layout::{Layout, Out, output};
