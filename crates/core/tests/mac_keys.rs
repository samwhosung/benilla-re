//! The Mac client's key tables, read from the reader's own i386 slice of the 1.12.1 (5875)
//! `World of Warcraft.app` and compared bit for bit with their transcription
//! (`systems/core/mackeys.md`). Each test skips without the slice (`MacSlice::load`).

use benilla_re_core::mac::{CHORD_PREFIXES, EDIT_COMMAND_KEYS, FIXED_KEY_CODES};
use benilla_re_difftest::MacSlice;

fn dwords(bytes: &[u8]) -> Vec<u32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_le_bytes(*c))
        .collect()
}

/// The 128 signed dwords at `mac:5bf320`, the fixed key table, against [`FIXED_KEY_CODES`].
#[test]
fn mac_fixed_key_codes() {
    let Some(mac) = MacSlice::load() else { return };
    let read: Vec<i32> = dwords(&mac.read(0x5bf320, 512))
        .into_iter()
        .map(|d| d as i32)
        .collect();
    assert_eq!(read, FIXED_KEY_CODES);
}

/// The five command and key pairs at `mac:693d60` against [`EDIT_COMMAND_KEYS`].
#[test]
fn mac_edit_command_keys() {
    let Some(mac) = MacSlice::load() else { return };
    let read = dwords(&mac.read(0x693d60, 40));
    let pairs: Vec<(u32, u32)> = read
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[a, b]| (a, b))
        .collect();
    assert_eq!(pairs, EDIT_COMMAND_KEYS);
}

/// The three code and string pairs at `mac:699fc8` against [`CHORD_PREFIXES`], each string read
/// through its pointer up to its NUL.
#[test]
fn mac_chord_prefixes() {
    let Some(mac) = MacSlice::load() else { return };
    let read = dwords(&mac.read(0x699fc8, 24));
    for (&[code, text], &(want_code, prefix)) in read.as_chunks::<2>().0.iter().zip(&CHORD_PREFIXES)
    {
        let bytes = mac.read(u64::from(text), prefix.len() + 1);
        assert_eq!(code, want_code, "the code of {prefix}");
        assert_eq!(
            bytes,
            [prefix.as_bytes(), &[0]].concat(),
            "the string of {prefix}"
        );
    }
}
