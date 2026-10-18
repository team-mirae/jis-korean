//! Keeps `include/jis_korean.h` in step with the key ids.

use jis_korean::Key;

#[test]
fn header_key_ids_match() {
    let header = include_str!("../include/jis_korean.h");
    for (id, key) in Key::ALL.iter().enumerate() {
        let line = format!("JK_KEY_{} = {id},", key.code().to_uppercase());
        assert!(header.contains(&line), "include/jis_korean.h is missing `{line}`");
    }
    assert_eq!(header.matches("JK_KEY_").count(), Key::ALL.len() + 2, "stray JK_KEY_ ids in the header");
}
