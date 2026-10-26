//! Character classes and case in the C locale (`systems/crt/ctype.md`): the type table `_ctype`
//! (`0x831724`) and `toupper`'s fold.

/// The bits `_ctype` holds for EOF and the bytes 0 to 0xff: `0x20` control, `0x28` the controls
/// 9 to 13, `0x48` the space, `0x10` punctuation, `0x84` digits, `0x81` and `0x82` the hex
/// letters, `0x01` and `0x02` the other letters, and nothing for EOF or 0x80 to 0xff.
pub fn ctype(c: i32) -> u16 {
    match c {
        0x09..=0x0d => 0x28,
        0x00..=0x1f | 0x7f => 0x20,
        0x20 => 0x48,
        0x30..=0x39 => 0x84,
        0x41..=0x46 => 0x81,
        0x47..=0x5a => 0x01,
        0x61..=0x66 => 0x82,
        0x67..=0x7a => 0x02,
        0x21..=0x7e => 0x10,
        _ => 0,
    }
}

/// `isalpha` (`0x73fb91`) for EOF and the bytes: the table's bits `0x103`.
pub fn isalpha(c: i32) -> i32 {
    i32::from(ctype(c) & 0x103)
}

/// `toupper` (`0x41089b`) with the locale word `0x883b08` at 0: `a` to `z` less 0x20, every
/// other value as it is.
pub fn toupper(c: i32) -> i32 {
    if (0x61..=0x7a).contains(&c) {
        c - 0x20
    } else {
        c
    }
}
