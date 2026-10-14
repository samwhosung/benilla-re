//! The pixel-format conversion, copy and blit leaves of the conversion matrix, reached from the
//! uncompressed BLP decode path through the dispatcher `0x5a4f60`. The source pixel byte order is
//! BGRA (`[0]` B, `[1]` G, `[2]` R, `[3]` A).
//!
//! Converter ABI: `__fastcall(ecx=&dims{w,h}, edx=src, [esp+4]=srcStride, [esp+8]=dst,
//! [esp+0xc]=dstStride)`. The two block copies (`0x5a5740`, `0x5a5780`) ignore the strides and take
//! `dst` at `[esp+8]`; they are flat, size-driven byte copies.

/// Pixels written per row by the 16-bit packers. The `width < 2` simple path writes exactly `w`;
/// the `width >= 2` path is unrolled by 2 (one 32-bit dword, 2 pixels, per step, advancing the column
/// by 2 while it is below `w`), so an odd width rounds up to even: the extra pixel is read from the
/// next source column and written past `w`. The same as `convert::packed_width` (the RGB565 sibling).
fn packed_width(w: u32) -> u32 {
    if w < 2 {
        w
    } else {
        (w + 1) & !1
    }
}

/// BGRA8888 → ARGB4444. `0x5a5070`. Per pixel `(A>>4)<<12 | (R>>4)<<8 | (G>>4)<<4 | (B>>4)`.
/// The simple path and the unrolled-by-2 path compute identical per-pixel bytes (the unrolled path
/// just packs two pixels into one little-endian dword); writing `packed_width(w)` pixels per row
/// reproduces the full region of both. `src_stride`/`dst_stride` are byte row pitches.
pub fn bgra8888_to_argb4444(
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
            let a = u16::from(src[srow + x * 4 + 3]);
            let px = (a >> 4) << 12 | (r >> 4) << 8 | (g >> 4) << 4 | (b >> 4);
            let o = drow + x * 2;
            dst[o..o + 2].copy_from_slice(&px.to_le_bytes());
        }
    }
}

/// BGRA8888 → ARGB1555. `0x5a51d0`. Per pixel `(A>>7)<<15 | (R>>3)<<10 | (G>>3)<<5 | (B>>3)`.
/// Two-path / odd-width handling as [`bgra8888_to_argb4444`].
pub fn bgra8888_to_argb1555(
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
            let a = u16::from(src[srow + x * 4 + 3]);
            let px = (a >> 7) << 15 | (r >> 3) << 10 | (g >> 3) << 5 | (b >> 3);
            let o = drow + x * 2;
            dst[o..o + 2].copy_from_slice(&px.to_le_bytes());
        }
    }
}

/// BGRA8888 plain copy. `0x5a5480`. A stride-aware 32-bpp blit: when `srcStride==dstStride==w*4`
/// the binary copies `w*h` dwords contiguously (fast path), otherwise it copies `w` dwords per row
/// then advances each pointer by its stride. Both write exactly `w*4` bytes per row at the row base,
/// so a row-by-row byte copy reproduces them; bytes outside `[0, w*4)` of each dst row are untouched.
pub fn bgra8888_copy(
    w: u32,
    h: u32,
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    let row = w as usize * 4;
    for y in 0..h as usize {
        let s = y * src_stride;
        let d = y * dst_stride;
        dst[d..d + row].copy_from_slice(&src[s..s + row]);
    }
}

/// BGRA8888 alpha-keyed RGB copy. `0x5a5510`. Per pixel: if the source alpha byte is nonzero,
/// `dst = (dst & 0xff000000) | (src & 0x00ffffff)` (keep dst alpha, take src RGB); else dst is left
/// unchanged. Reads dst, so the caller must seed it.
pub fn bgra8888_alpha_key(
    w: u32,
    h: u32,
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    for y in 0..h as usize {
        let srow = y * src_stride;
        let drow = y * dst_stride;
        for x in 0..w as usize {
            let so = srow + x * 4;
            let s = u32::from_le_bytes([src[so], src[so + 1], src[so + 2], src[so + 3]]);
            if s & 0xff00_0000 != 0 {
                let dofs = drow + x * 4;
                let d =
                    u32::from_le_bytes([dst[dofs], dst[dofs + 1], dst[dofs + 2], dst[dofs + 3]]);
                let out = (d & 0xff00_0000) | (s & 0x00ff_ffff);
                dst[dofs..dofs + 4].copy_from_slice(&out.to_le_bytes());
            }
        }
    }
}

/// BGRA8888 source-over alpha blend. `0x5a5590`. With `a = src>>24`: `a==0` leaves dst untouched;
/// `a==0xff` copies src RGB keeping dst alpha (the alpha-key fast branch); otherwise per channel
/// `out_c = ((src_c - dst_c) * a >> 8) + dst_c` (unsigned 32-bit wrap, low byte kept), dst alpha
/// preserved. Reads dst, so the caller must seed it.
pub fn bgra8888_blend(
    w: u32,
    h: u32,
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    for y in 0..h as usize {
        let srow = y * src_stride;
        let drow = y * dst_stride;
        for x in 0..w as usize {
            let so = srow + x * 4;
            let s = u32::from_le_bytes([src[so], src[so + 1], src[so + 2], src[so + 3]]);
            let a = s >> 24;
            if a == 0 {
                continue;
            }
            let dofs = drow + x * 4;
            let d = u32::from_le_bytes([dst[dofs], dst[dofs + 1], dst[dofs + 2], dst[dofs + 3]]);
            let out = if a == 0xff {
                (d & 0xff00_0000) | (s & 0x00ff_ffff)
            } else {
                let sb = s & 0xff;
                let sg = (s >> 8) & 0xff;
                let sr = (s >> 16) & 0xff;
                let db = d & 0xff;
                let dg = (d >> 8) & 0xff;
                let dr = (d >> 16) & 0xff;
                let ob = (sb.wrapping_sub(db).wrapping_mul(a) >> 8).wrapping_add(db) & 0xff;
                let og = (sg.wrapping_sub(dg).wrapping_mul(a) >> 8).wrapping_add(dg) & 0xff;
                let or = (sr.wrapping_sub(dr).wrapping_mul(a) >> 8).wrapping_add(dr) & 0xff;
                (d & 0xff00_0000) | (or << 16) | (og << 8) | ob
            };
            dst[dofs..dofs + 4].copy_from_slice(&out.to_le_bytes());
        }
    }
}

/// 16bpp → 16bpp copy. `0x5a56b0`. Stride-aware (fast path when `srcStride==dstStride==w*2`),
/// copies exactly `w*2` bytes per row at the row base; row-by-row byte copy reproduces it.
pub fn copy16(w: u32, h: u32, src: &[u8], src_stride: usize, dst: &mut [u8], dst_stride: usize) {
    let row = w as usize * 2;
    for y in 0..h as usize {
        let s = y * src_stride;
        let d = y * dst_stride;
        dst[d..d + row].copy_from_slice(&src[s..s + row]);
    }
}

/// DXT1 block copy. `0x5a5740`. 8 bytes per 4×4 block: dims are clamped to ≥4 (`<5 → 4`) and
/// `floor(cw*ch / 2)` bytes are copied flat (strides ignored). The binary derives that count as
/// `(n>>3)` dwords + `((n>>1)&3)` tail bytes with `n = cw*ch & 0x3fffffff`.
pub fn dxt1_block_copy(w: u32, h: u32, src: &[u8], dst: &mut [u8]) {
    let cw = if w < 5 { 4 } else { w };
    let ch = if h < 5 { 4 } else { h };
    let n = cw.wrapping_mul(ch) & 0x3fff_ffff;
    let dwords = n >> 3;
    let tail = (n >> 1) & 3;
    let total = (dwords * 4 + tail) as usize;
    dst[..total].copy_from_slice(&src[..total]);
}

/// DXT3/DXT5 block copy. `0x5a5780`. 16 bytes per 4×4 block: dims clamped to ≥4, `cw*ch` bytes
/// copied flat (strides ignored). The binary derives the count as `((p & 0x1fffffff) >> 2)` dwords
/// + `(p & 3)` tail bytes with `p = cw*ch`.
pub fn dxt_block_copy_16(w: u32, h: u32, src: &[u8], dst: &mut [u8]) {
    let cw = if w < 5 { 4 } else { w };
    let ch = if h < 5 { 4 } else { h };
    let p = cw.wrapping_mul(ch);
    let dwords = (p & 0x1fff_ffff) >> 2;
    let tail = p & 3;
    let total = (dwords * 4 + tail) as usize;
    dst[..total].copy_from_slice(&src[..total]);
}
