//! Storm's option setter (`0x64ec80`, `systems/storm/printf.md#storm-options`).

/// The cells the options live in, `0xc53ee8` to `0xc53f1c`, by option id; option 6 has two and
/// option 8 none.
pub const OPTION_CELLS: [(u32, &[u32]); 12] = [
    (1, &[0xc53ef8]),
    (2, &[0xc53efc]),
    (3, &[0xc53ef4]),
    (4, &[0xc53f00]),
    (5, &[0xc53f04]),
    (6, &[0xc53ef0, 0xc53ee8]),
    (7, &[0xc53f08]),
    (9, &[0xc53f0c]),
    (10, &[0xc53f10]),
    (11, &[0xc53f14]),
    (12, &[0xc53f18]),
    (13, &[0xc53f1c]),
];

/// `0x64ec80(id, data, size)`, `__stdcall`: sets option `id`. It calls `SErrSetLastError(0x57)`
/// first, and on success `SErrSetLastError(0)`; the codes are returned in that order with the
/// result. A null `data`, an id outside 1 to 13 or 8, or a size other than 4 (8 for option 6)
/// fails. Option 2 takes a nonzero `v` with `(v − 1) ^ v == 2v − 1` on 32 bits: a power of two, or
/// `2^31 + 2^k` for `k` below 31, since `2v` wraps; options 1 to 7
/// store the value, 6 its two dwords at `0xc53ef0` and `0xc53ee8`, and 9 to 13 store `v != 0`.
/// `write(cell, value)` receives each store.
pub fn set_option(
    id: u32,
    data: Option<[u32; 2]>,
    size: u32,
    write: &mut dyn FnMut(u32, u32),
) -> (bool, Vec<u32>) {
    let mut codes = vec![0x57];
    let Some(value) = data else {
        return (false, codes);
    };
    let Some(&(_, cells)) = OPTION_CELLS.iter().find(|(o, _)| *o == id) else {
        return (false, codes);
    };
    let want = if id == 6 { 8 } else { 4 };
    if size != want {
        return (false, codes);
    }
    let v = value[0];
    match id {
        2 => {
            if v == 0 || (v.wrapping_sub(1) ^ v) != v.wrapping_add(v).wrapping_sub(1) {
                return (false, codes);
            }
            write(cells[0], v);
        }
        6 => {
            write(cells[0], value[0]);
            write(cells[1], value[1]);
        }
        9..=13 => write(cells[0], u32::from(v != 0)),
        _ => write(cells[0], v),
    }
    codes.push(0);
    (true, codes)
}
