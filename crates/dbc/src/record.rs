//! A record type's `Read` (`systems/dbc.md`, Record reads): one `SFile::Read` (`0x648460`) per
//! column in order, every column read whether or not an earlier one failed, then the string
//! columns rebased once all succeeded.
//!
//! A layout is the reader's column schedule, `(size in bytes, is a string)` in call order, the
//! record offsets running on from 0 without gaps.

/// The address `Read` stores for every string column when the string block base is null: the
/// client's empty string (`0x882748`).
pub const EMPTY_STRING: u32 = 0x0088_2748;

/// One column: its size in bytes, and whether it is a string-block offset read into a temporary.
pub type Column = (usize, bool);

/// `CharHairGeosetsRec::Read` (`0x5753b0`): six dwords.
pub const CHAR_HAIR_GEOSETS: &[Column] = &[(4, false); 6];

/// `CharSectionsRec::Read` (`0x575540`): six dwords, three texture names, a dword.
pub const CHAR_SECTIONS: &[Column] = &[
    (4, false),
    (4, false),
    (4, false),
    (4, false),
    (4, false),
    (4, false),
    (4, true),
    (4, true),
    (4, true),
    (4, false),
];

/// `CharacterFacialHairStylesRec::Read` (`0x575a80`): six dwords, then three as one 12-byte read.
pub const CHARACTER_FACIAL_HAIR_STYLES: &[Column] = &[
    (4, false),
    (4, false),
    (4, false),
    (4, false),
    (4, false),
    (4, false),
    (12, false),
];

/// `WeaponImpactSoundsRec::Read` (`0x587450`): three dwords, then ten and ten as two 40-byte reads.
pub const WEAPON_IMPACT_SOUNDS: &[Column] =
    &[(4, false), (4, false), (4, false), (40, false), (40, false)];

/// `LightParamsRec::Read` (`0x589030`): nine dwords.
pub const LIGHT_PARAMS: &[Column] = &[(4, false); 9];

/// `SpellRec::Read` (`0x583750`): 118 reads into 692 bytes; the four localized strings at `+0x1e0`,
/// `+0x204`, `+0x228` and `+0x24c`, each eight offsets and a dword.
pub const SPELL: &[Column] = &{
    let head: [usize; 68] = [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 8, 32, 32, 4, 4, 4, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
        12, 12, 12, 12, 12, 12, 12, 12, 8, 4, 4, 4,
    ];
    let tail: [usize; 14] = [4, 4, 4, 4, 4, 8, 4, 4, 4, 4, 12, 4, 4, 4];
    let mut out = [(0usize, false); 118];
    let mut i = 0;
    while i < head.len() {
        out[i] = (head[i], false);
        i += 1;
    }
    // Four localized strings: eight string offsets and a flags dword each.
    let mut s = 0;
    while s < 4 {
        let mut k = 0;
        while k < 9 {
            out[head.len() + s * 9 + k] = (4, k < 8);
            k += 1;
        }
        s += 1;
    }
    let mut t = 0;
    while t < tail.len() {
        out[head.len() + 36 + t] = (tail[t], false);
        t += 1;
    }
    out
};

/// The record size a layout fills.
pub fn record_size(layout: &[Column]) -> usize {
    layout.iter().map(|&(size, _)| size).sum()
}

/// Reads one record as `Read` does. `reads[i]` is what column `i`'s `SFile::Read` gives: its
/// bytes, or `None` for a read that returns 0 and writes nothing. A scalar column is copied into
/// the record as it is read; a string column's offset waits in a temporary. When every read
/// succeeded each string column becomes `string_base + offset` (32-bit wrapping), or
/// [`EMPTY_STRING`] when `string_base` is 0, and the result is true. Otherwise the string columns
/// are left as they were and the result is false (the caller logs `Error reading <Type>Rec`).
pub fn read_record(
    layout: &[Column],
    reads: &[Option<&[u8]>],
    string_base: u32,
    record: &mut [u8],
) -> bool {
    assert_eq!(layout.len(), reads.len());
    let mut offsets = Vec::new();
    let mut pos = 0;
    let mut ok = true;
    for (&(size, string), read) in layout.iter().zip(reads) {
        match read {
            Some(bytes) => {
                assert_eq!(bytes.len(), size);
                if string {
                    let v = u32::from_le_bytes(bytes[..4].try_into().unwrap());
                    offsets.push((pos, v));
                } else {
                    record[pos..pos + size].copy_from_slice(bytes);
                }
            }
            None => ok = false,
        }
        pos += size;
    }
    if !ok {
        return false;
    }
    for (at, offset) in offsets {
        let v = if string_base == 0 {
            EMPTY_STRING
        } else {
            string_base.wrapping_add(offset)
        };
        record[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    true
}
