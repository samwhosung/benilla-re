//! The `Wow.ini` profile's integer read (`systems/dbc.md`, Wow.ini values).

use benilla_re_storm::str_to_int;

/// `ProfileParseInt` (`0x445470`) of a non-null value: after a leading `'`, the up to four bytes
/// that follow, stopping at a NUL or a `'`, packed first byte highest, each byte unsigned; any
/// other value through `SStrToInt`.
pub fn parse_int(value: &[u8]) -> i32 {
    let byte = |i: usize| value.get(i).copied().unwrap_or(0);
    if byte(0) != b'\'' {
        return str_to_int(value);
    }
    let mut v: u32 = 0;
    for i in 1..=4 {
        let b = byte(i);
        if b == 0 || b == b'\'' {
            break;
        }
        v = (v << 8) | u32::from(b);
    }
    v as i32
}
