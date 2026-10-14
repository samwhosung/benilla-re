//! Pixel-format converters (paletted or truecolor to the device format): the BGRA8888 intermediate
//! to 16-bit quantize-and-pack family. The source pixel byte order is BGRA (`[0]` B, `[1]` G, `[2]` R,
//! `[3]` A).

/// Pixels written per row. `0x5a5330`'s `width < 2` path writes exactly `w`; its `width >= 2`
/// path is unrolled by 2 (it writes a 32-bit dword, 2 pixels, per step, advancing the column by 2
/// while it is below `w`), so an odd width rounds up to even: the extra pixel is read from the next
/// source column and written past `w`. The transcription mirrors that to match the output buffer
/// bit for bit.
fn packed_width(w: u32) -> u32 {
    if w < 2 {
        w
    } else {
        (w + 1) & !1
    }
}

/// BGRA8888 → RGB565 (alpha dropped). `0x5a5330`. Per pixel
/// `(R>>3)<<11 | (G>>2)<<5 | (B>>3)`; `src_stride`/`dst_stride` are byte row pitches. Writes
/// `packed_width(w) * h` pixels (see [`packed_width`]).
pub fn bgra8888_to_rgb565(
    w: u32,
    h: u32,
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    let n = packed_width(w) as usize;
    for y in 0..h as usize {
        let srow = y * src_stride;
        let drow = y * dst_stride;
        for x in 0..n {
            let b = u16::from(src[srow + x * 4]);
            let g = u16::from(src[srow + x * 4 + 1]);
            let r = u16::from(src[srow + x * 4 + 2]);
            let px = (r >> 3) << 11 | (g >> 2) << 5 | (b >> 3);
            let o = drow + x * 2;
            dst[o..o + 2].copy_from_slice(&px.to_le_bytes());
        }
    }
}
