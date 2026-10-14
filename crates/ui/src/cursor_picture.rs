//! The cursor's dragging picture (`systems/ui/cursor.md#dragging-picture`): the 2 by 2 box filter
//! that `0x523840` runs over the 64 by 64 icon.

/// The box filter of `BuildCursorPicture` (`0x523840`): a 64 by 64 ARGB texture reduced to 32 by 32.
/// The enclosing function takes the texture handle `0xbe2c60` (with no picture state at `0xbe2c48`
/// it only frees the texture), reads its pixels through `0x44ae00` and its size through `0x44b3b0`,
/// refuses any size but 64 by 64, writes the 32 by 32 result at `0xbe1c28`, frees the texture
/// (`0x41aed0`), zeroes the handle and calls `0x523790`; only the filter is computed here.
///
/// Output texel `(row, col)` averages the quad `src[128 * row + 2 * col]` (top left), `+1`, `+64`
/// and `+65`. `src` is 4096 `u32` row-major and the result is 1024. See [`box_filter_quad`].
#[must_use]
pub fn build_cursor_picture(src: &[u32; 64 * 64]) -> [u32; 32 * 32] {
    let mut out = [0u32; 32 * 32];
    for row in 0..32 {
        for col in 0..32 {
            let base = 128 * row + 2 * col;
            let a = src[base];
            let b = src[base + 1];
            let c = src[base + 64];
            let d = src[base + 65];
            out[row * 32 + col] = box_filter_quad(a, b, c, d);
        }
    }
    out
}

/// One texel of the filter: the four texels `a` (top left), `b` (top right), `c` (bottom left) and
/// `d` (bottom right) averaged by channel, with alpha forced to `0xff`. The binary splits each
/// texel into two lanes so that two additions average all four channels, each lane carrying its
/// rounding term before the shift by 2:
///
/// ```text
/// G  = ((a>>2 & 0x3fc0) + (c>>2 & 0x3fc0) + (b>>2 & 0x3fc0) + 0x80 + (d>>2 & 0x3fc0)) & 0xff00
/// RB = ((a & 0xff00ff) + (c & 0xff00ff) + (b & 0xff00ff) + 0x20002 + (d & 0xff00ff)) >> 2 & 0xffff00ff
/// out = G | RB | 0xff000000
/// ```
///
/// The green lane shifts each source by 2 first and masks to `0x3fc0`, so the four greens sum
/// without carrying into red, adds `0x80` and masks to `0xff00`. The red and blue lane keeps the two
/// channels interleaved in `0xff00ff`, adds `0x20002` (2 in each channel), shifts by 2 and masks to
/// `0xffff00ff`. The terms are added in the order the binary does, `a, c, b, bias, d`, with
/// wrapping 32-bit adds; no term overflows under these masks.
#[must_use]
pub fn box_filter_quad(a: u32, b: u32, c: u32, d: u32) -> u32 {
    let g = ((a >> 2) & 0x3fc0)
        .wrapping_add((c >> 2) & 0x3fc0)
        .wrapping_add((b >> 2) & 0x3fc0)
        .wrapping_add(0x80)
        .wrapping_add((d >> 2) & 0x3fc0)
        & 0xff00;
    let rb = ((a & 0xff_00ff)
        .wrapping_add(c & 0xff_00ff)
        .wrapping_add(b & 0xff_00ff)
        .wrapping_add(0x2_0002)
        .wrapping_add(d & 0xff_00ff)
        >> 2)
        & 0xffff_00ff;
    g | rb | 0xff00_0000
}
