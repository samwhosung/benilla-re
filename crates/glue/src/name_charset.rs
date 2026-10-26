//! The character name charsets (`systems/glue/enterworld.md`, Name charsets): the pair a client's
//! region or locale picks, and the unit tests of each charset.

/// The primary and secondary charset of regions 1 to 5, the table at `0x86c9e4`.
pub const REGION_PAIRS: [(u32, u32); 5] = [(1, 1), (2, 4), (1, 1), (3, 1), (3, 1)];
/// The primary charset of each locale index, the table at `0x86ca0c`.
pub const LOCALE_PRIMARY: [u32; 8] = [1, 2, 1, 1, 3, 3, 1, 0];

/// `GetNameCharsets` (`0x6c9a70`): the region's pair for a region of 1 to 5, else the locale's
/// primary, entry 0 standing in for an index outside 0 to 7 or a zero entry, and the secondary 4
/// for index 1, else 1.
pub fn name_charsets(region: i32, locale: i32) -> (u32, u32) {
    if (1..=5).contains(&region) {
        return REGION_PAIRS[(region - 1) as usize];
    }
    let index = if (0..8).contains(&locale) && LOCALE_PRIMARY[locale as usize] != 0 {
        locale as usize
    } else {
        0
    };
    (LOCALE_PRIMARY[index], if index == 1 { 4 } else { 1 })
}

/// `IsLatin1Letter` (`0x6c9c60`): A to Z, a to z, U+00C0 to U+00DD but U+00D7, U+00DF, and U+00E0
/// to U+00FF but U+00F7.
pub fn is_latin1_letter(u: u16) -> bool {
    matches!(u, 0x41..=0x5a | 0x61..=0x7a | 0xdf)
        || ((0xc0..=0xdd).contains(&u) && u != 0xd7)
        || ((0xe0..=0xff).contains(&u) && u != 0xf7)
}

/// `IsHangulNameSyllable` (`0x6c9d50`): a unit in U+AC00 to U+D7A3 found in `syllables`, the 2,350
/// words at `0x86ca30`.
pub fn is_hangul_name_syllable(u: u16, syllables: &[u16]) -> bool {
    (0xac00..=0xd7a3).contains(&u) && syllables.contains(&u)
}

/// `IsCjkIdeograph` (`0x6c9d80`): U+4E00 to U+9FFF, U+3400 to U+4DFF or U+F900 to U+FAFF.
pub fn is_cjk_ideograph(u: u16) -> bool {
    matches!(u, 0x4e00..=0x9fff | 0x3400..=0x4dff | 0xf900..=0xfaff)
}

/// `IsAsciiLetter` (`0x6c9ce0`): A to Z or a to z.
pub fn is_ascii_letter(u: u16) -> bool {
    matches!(u, 0x41..=0x5a | 0x61..=0x7a)
}

/// `IsInNameCharset` (`0x6c9dd0`): charset 1 to 4; any other takes no unit.
pub fn in_name_charset(u: u16, set: u32, syllables: &[u16]) -> bool {
    match set {
        1 => is_latin1_letter(u),
        2 => is_hangul_name_syllable(u, syllables),
        3 => is_cjk_ideograph(u),
        4 => is_ascii_letter(u),
        _ => false,
    }
}
