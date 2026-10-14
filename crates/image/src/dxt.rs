//! The software DXT/S3TC decode band (`[0x5a57c0, 0x5a7920)`): DXT1 and DXT3 decoded to RGB565,
//! ARGB1555, ARGB4444 and ARGB8888 (the band has no software DXT5; fmt 7 is handed to the GPU as
//! native S3TC by the passthrough copier). Reached from the BLP decode path through the converter
//! dispatcher `0x5a4f60`.
//!
//! ## Shared interpolation LUT (`0x5a7910`, built at run time at `0xc0fa80`)
//! All endpoint interpolation goes through two precomputed tables, `la(n) = (n*256)/3 + 1` and
//! `lb(n) = (n*512)/3 + 1` (the exe stores both at 5-bit and 6-bit widths; they share the formula).
//! The combos `(lb[c0] + la[c1]) >> 8 = (2*c0 + c1)/3` and `(la[c0] + lb[c1]) >> 8 = (c0 + 2*c1)/3`
//! are the two interpolated DXT colors. [`build_interp_lut`] reproduces the table bytes; the
//! decoders compute through [`la`] and [`lb`] directly, which give identical values.
//!
//! ## DXT1 punch-through
//! `color0 > color1` gives the 4-color palette `{c0, c1, (2c0+c1)/3, (c0+2c1)/3}`; otherwise the
//! 3-color palette `{c0, c1, (c0+c1)/2, 0 = transparent black}`. DXT3 colour is always 4-color (alpha
//! is the explicit 4-bit plane).
//!
//! ## Per-block decoder ABI
//! `__fastcall(ecx=block, edx=&dstRowPtrs, [esp+4]=&bounds{x0,y0,x1,y1,adv}, [esp+8]=alphaCb)`.
//! The decoder writes pixel `(x,y)` at `rowptrs[y] + x*pixelsize` for `x in x0..=x1`, `y in
//! y0..=y1`, then advances `rowptrs[y]` by `adv*pixelsize` (so a driver reusing the row-pointer
//! array marches it across a block row). DXT1 decoders take no callback (`ret 4`); DXT3 decoders
//! take the alpha callback (`ret 8`), which receives the 4-bit nibble in `ecx` and returns the
//! alpha byte in `al` (`0x5a60b0` is the identity for 4444, `0x5a62b0` gives nibble<<4 for 8888).
//!
//! The transcription models `dstRowPtrs` as byte offsets into a `dst` slice (the exe's absolute
//! pointers), so a standalone decoder and the calls nested in a driver produce identical output
//! bytes.

// ============================================================================================
// Interpolation LUT  (0x5a7910)
// ============================================================================================

/// `la(n) = (n*256)/3 + 1` — the "×256" interpolation sub-table value (stored at `0xc0fa80` 5-bit /
/// `0xc0fb00` 6-bit). Truncating integer division, matching the binary's `(short)((n<<8)/3) + 1`
/// (all values < 0x8000, so the `short` cast is a no-op).
#[inline]
fn la(n: u32) -> u32 {
    ((n << 8) / 3) + 1
}

/// `lb(n) = (n*512)/3 + 1` — the "×512" interpolation sub-table value (stored at `0xc0fac0` 5-bit /
/// `0xc0fb80` 6-bit).
#[inline]
fn lb(n: u32) -> u32 {
    ((n * 512) / 3) + 1
}

/// `(2*c0 + c1)/3` for one channel: `(lb[c0] + la[c1]) >> 8`.
#[inline]
fn interp2(c0: u32, c1: u32) -> u32 {
    (lb(c0) + la(c1)) >> 8
}

/// `(c0 + 2*c1)/3` for one channel: `(la[c0] + lb[c1]) >> 8`.
#[inline]
fn interp1(c0: u32, c1: u32) -> u32 {
    (la(c0) + lb(c1)) >> 8
}

/// `(c0 + c1)/2` for one channel (the 3-color midpoint).
#[inline]
fn avg(c0: u32, c1: u32) -> u32 {
    (c0 + c1) / 2
}

/// `0x5a7910` — fill the shared interpolation LUT (`__fastcall(ecx=base)`, here returns the
/// 0x180-byte image). Four sub-tables: `la` 5-bit (base+0x00, 32 entries), `lb` 5-bit (base+0x40),
/// `la` 6-bit (base+0x80, 64 entries), `lb` 6-bit (base+0x100). Each entry is a little-endian `u16`.
pub fn build_interp_lut() -> [u8; 0x180] {
    let mut buf = [0u8; 0x180];
    // First loop: 5-bit tables, n = 0..31 (acc = n*512 < 0x4000).
    let mut n: u32 = 0;
    let mut acc: u32 = 0;
    loop {
        let a = (((n << 8) / 3) + 1) as u16;
        let b = ((acc / 3) + 1) as u16;
        let o = (n * 2) as usize;
        buf[o..o + 2].copy_from_slice(&a.to_le_bytes());
        buf[0x40 + o..0x40 + o + 2].copy_from_slice(&b.to_le_bytes());
        acc += 0x200;
        n += 1;
        if acc >= 0x4000 {
            break;
        }
    }
    // Second loop: 6-bit tables, n = 0..63 (acc = n*512 < 0x8000).
    let mut n: u32 = 0;
    let mut acc: u32 = 0;
    loop {
        let a = (((n << 8) / 3) + 1) as u16;
        let b = ((acc / 3) + 1) as u16;
        let o = (n * 2) as usize;
        buf[0x80 + o..0x80 + o + 2].copy_from_slice(&a.to_le_bytes());
        buf[0x100 + o..0x100 + o + 2].copy_from_slice(&b.to_le_bytes());
        acc += 0x200;
        n += 1;
        if acc >= 0x8000 {
            break;
        }
    }
    buf
}

// ============================================================================================
// Pixel-format row pitch  (0x5a62c0)
// ============================================================================================

/// `.data` shift table `0x85c868` (9 x i32), compared with the exe's bytes by the test.
/// (Note: index 8 is `0`, not `2` — fmt 8 "other16" is a flat 16bpp pitch.)
pub const PITCH_SHIFT: [u32; 9] = [0, 0, 0, 0, 0, 2, 2, 2, 0];
/// `.data` mult table `0x85c88c` (9 x i32), compared with the exe's bytes by the test.
pub const PITCH_MULT: [u32; 9] = [0, 4, 2, 2, 2, 8, 16, 16, 2];

/// `0x5a62c0` — `__fastcall(ecx=fmt, edx=dim)` → `(dim >> shift[fmt]) * mult[fmt]`, with the DXT
/// 4-texel-minimum clamp: for `fmt in {5,6,7}` (the block-compressed formats) a `dim < 5` is forced
/// to 4 before the shift. `fmt` indexes the gx device-format enum `0..=8`.
pub fn pitch(fmt: u32, dim: u32) -> u32 {
    let mut d = dim;
    if (4 < fmt) && (fmt < 8) && (dim < 5) {
        d = 4;
    }
    let f = fmt as usize;
    (d >> (PITCH_SHIFT[f] & 0x1f)).wrapping_mul(PITCH_MULT[f])
}

// ============================================================================================
// Endpoint / component packers
// ============================================================================================

#[inline]
fn rd16(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

/// Split a 565 colour word into `[r5, g6, b5]`.
#[inline]
fn chan(c: u16) -> [u32; 3] {
    let c = c as u32;
    [c >> 11, (c >> 5) & 0x3f, c & 0x1f]
}

/// `0x5a6590` — RGB565-component → ARGB8888 endpoint pack (`thiscall ecx=dst, [r,g,b] on stack`).
/// `(((r|0xffffffe0)<<9 | g)<<7 | b)<<3` — channels to MSB, alpha 0xFF, no low-bit replication.
pub fn pack_565comp_argb8888(r: u32, g: u32, b: u32) -> u32 {
    let (r, g, b) = (r & 0xff, g & 0xff, b & 0xff);
    ((((r | 0xffff_ffe0) << 9) | g) << 7 | b) << 3
}

/// `0x5a6880` — RGB565-component → ARGB4444 endpoint pack.
/// `((r&0xfffe | 0xffe0)<<5 | g&0xfffc)<<2 | (b>>1)`.
pub fn pack_565comp_argb4444(r: u32, g: u32, b: u32) -> u16 {
    let (r, g, b) = (r & 0xff, g & 0xff, b & 0xff);
    (((((r & 0xfffe) | 0xffe0) << 5) | (g & 0xfffc)) << 2 | (b >> 1)) as u16
}

/// `0x5a6e70` — RGB565-component → ARGB1555 endpoint pack.
/// `((r|0xffe0)<<6 | g&0xfffe)<<4 | b`.
pub fn pack_565comp_argb1555(r: u32, g: u32, b: u32) -> u16 {
    let (r, g, b) = (r & 0xff, g & 0xff, b & 0xff);
    ((((r | 0xffe0) << 6) | (g & 0xfffe)) << 4 | b) as u16
}

/// `0x5a6ea0` — packed 565 word → ARGB1555 (`thiscall ecx=dst, [esp+4]=&word`). Extracts the
/// 565 channels and packs identically to [`pack_565comp_argb1555`].
pub fn pack_word_argb1555(word: u16) -> u16 {
    let c = chan(word);
    pack_565comp_argb1555(c[0], c[1], c[2])
}

/// `0x5a7190` — packed 565 word → ARGB8888 (`thiscall ecx=dst, [esp+4]=&word`). Extracts the
/// 565 channels and packs identically to [`pack_565comp_argb8888`].
pub fn pack_word_argb8888(word: u16) -> u32 {
    let c = chan(word);
    pack_565comp_argb8888(c[0], c[1], c[2])
}

/// RGB565 pack from already-extracted channels: `((g | r<<6)<<5) | b = (r<<11)|(g<<5)|b`.
#[inline]
fn pack565(r: u32, g: u32, b: u32) -> u16 {
    let (r, g, b) = (r & 0xff, g & 0xff, b & 0xff);
    (((g | (r << 6)) << 5) | b) as u16
}

// ============================================================================================
// DXT1 4-entry palette builders  (0x5a7200 / 73b0 / 7640)
// ============================================================================================
//
// These are the edge-path palette builders, but the per-block decoders compute the identical
// 4-entry palette inline (the exe's inline expansion and the packer-based form are equal, so the
// builders and the decoders, both tested against the exe, share this code).

/// `0x5a7200` — DXT1 4-entry palette → RGB565 (`__fastcall ecx=&block, edx=&pal[4]`).
/// Endpoints copied raw; punch-through on `color0 > color1`.
pub fn dxt1_pal_rgb565(color0: u16, color1: u16) -> [u16; 4] {
    let a = chan(color0);
    let b = chan(color1);
    let mut p = [0u16; 4];
    p[0] = color0;
    p[1] = color1;
    if color1 < color0 {
        p[2] = pack565(
            interp2(a[0], b[0]),
            interp2(a[1], b[1]),
            interp2(a[2], b[2]),
        );
        p[3] = pack565(
            interp1(a[0], b[0]),
            interp1(a[1], b[1]),
            interp1(a[2], b[2]),
        );
    } else {
        p[2] = pack565(avg(a[0], b[0]), avg(a[1], b[1]), avg(a[2], b[2]));
        p[3] = 0;
    }
    p
}

/// `0x5a73b0` — DXT1 4-entry palette → ARGB1555.
pub fn dxt1_pal_argb1555(color0: u16, color1: u16) -> [u16; 4] {
    let a = chan(color0);
    let b = chan(color1);
    let mut p = [0u16; 4];
    p[0] = pack_word_argb1555(color0);
    p[1] = pack_word_argb1555(color1);
    if color1 < color0 {
        p[2] = pack_565comp_argb1555(
            interp2(a[0], b[0]),
            interp2(a[1], b[1]),
            interp2(a[2], b[2]),
        );
        p[3] = pack_565comp_argb1555(
            interp1(a[0], b[0]),
            interp1(a[1], b[1]),
            interp1(a[2], b[2]),
        );
    } else {
        p[2] = pack_565comp_argb1555(avg(a[0], b[0]), avg(a[1], b[1]), avg(a[2], b[2]));
        p[3] = 0;
    }
    p
}

/// `0x5a7640` — DXT1 4-entry palette → ARGB8888.
pub fn dxt1_pal_argb8888(color0: u16, color1: u16) -> [u32; 4] {
    let a = chan(color0);
    let b = chan(color1);
    let mut p = [0u32; 4];
    p[0] = pack_word_argb8888(color0);
    p[1] = pack_word_argb8888(color1);
    if color1 < color0 {
        p[2] = pack_565comp_argb8888(
            interp2(a[0], b[0]),
            interp2(a[1], b[1]),
            interp2(a[2], b[2]),
        );
        p[3] = pack_565comp_argb8888(
            interp1(a[0], b[0]),
            interp1(a[1], b[1]),
            interp1(a[2], b[2]),
        );
    } else {
        p[2] = pack_565comp_argb8888(avg(a[0], b[0]), avg(a[1], b[1]), avg(a[2], b[2]));
        p[3] = 0;
    }
    p
}

/// DXT3 4-entry colour palette → ARGB8888 (always 4-color; no punch-through).
fn dxt3_pal_argb8888(color0: u16, color1: u16) -> [u32; 4] {
    let a = chan(color0);
    let b = chan(color1);
    [
        pack_565comp_argb8888(a[0], a[1], a[2]),
        pack_565comp_argb8888(b[0], b[1], b[2]),
        pack_565comp_argb8888(
            interp2(a[0], b[0]),
            interp2(a[1], b[1]),
            interp2(a[2], b[2]),
        ),
        pack_565comp_argb8888(
            interp1(a[0], b[0]),
            interp1(a[1], b[1]),
            interp1(a[2], b[2]),
        ),
    ]
}

/// DXT3 4-entry colour palette → ARGB4444 (always 4-color). Entries keep the packer's full 16-bit
/// output; the decoder masks `& 0xfff` and OR's the explicit alpha nibble at bits 12..15.
fn dxt3_pal_argb4444(color0: u16, color1: u16) -> [u16; 4] {
    let a = chan(color0);
    let b = chan(color1);
    [
        pack_565comp_argb4444(a[0], a[1], a[2]),
        pack_565comp_argb4444(b[0], b[1], b[2]),
        pack_565comp_argb4444(
            interp2(a[0], b[0]),
            interp2(a[1], b[1]),
            interp2(a[2], b[2]),
        ),
        pack_565comp_argb4444(
            interp1(a[0], b[0]),
            interp1(a[1], b[1]),
            interp1(a[2], b[2]),
        ),
    ]
}

// ============================================================================================
// Per-block decoders
// ============================================================================================

/// Block decode bounds: inclusive pixel rectangle `[x0,x1] x [y0,y1]` within the 4×4 block and the
/// per-row pointer advance `adv` (in pixels). `row_off[y]` is a byte offset into `dst`.
#[derive(Clone, Copy)]
pub struct Bounds {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
    pub adv: u32,
}

/// Alpha callback `0x5a60b0` (identity): returns the 4-bit nibble. Used by DXT3→ARGB4444.
pub fn alpha_identity(nibble: u32) -> u32 {
    nibble & 0xf
}

/// Alpha callback `0x5a62b0` (nibble<<4): expands a 4-bit nibble to an 8-bit alpha (no replication).
/// Used by DXT3→ARGB8888.
pub fn alpha_nibble_shl4(nibble: u32) -> u32 {
    (nibble & 0xf) << 4
}

/// `0x5a6990` — decode one DXT1 block → RGB565 into `dst` via `row_off`.
pub fn dxt1_block_rgb565(block: &[u8], dst: &mut [u8], row_off: &mut [usize], b: Bounds) {
    let pal = dxt1_pal_rgb565(rd16(block, 0), rd16(block, 2));
    for y in b.y0..=b.y1 {
        let yi = y as usize;
        let mut bits = (block[4 + yi] as u32) >> (b.x0 * 2);
        let base = row_off[yi];
        for x in b.x0..=b.x1 {
            let o = base + (x as usize) * 2;
            dst[o..o + 2].copy_from_slice(&pal[(bits & 3) as usize].to_le_bytes());
            bits >>= 2;
        }
        row_off[yi] = base + (b.adv as usize) * 2;
    }
}

/// `0x5a6be0` — decode one DXT1 block → ARGB1555.
pub fn dxt1_block_argb1555(block: &[u8], dst: &mut [u8], row_off: &mut [usize], b: Bounds) {
    let pal = dxt1_pal_argb1555(rd16(block, 0), rd16(block, 2));
    for y in b.y0..=b.y1 {
        let yi = y as usize;
        let mut bits = (block[4 + yi] as u32) >> (b.x0 * 2);
        let base = row_off[yi];
        for x in b.x0..=b.x1 {
            let o = base + (x as usize) * 2;
            dst[o..o + 2].copy_from_slice(&pal[(bits & 3) as usize].to_le_bytes());
            bits >>= 2;
        }
        row_off[yi] = base + (b.adv as usize) * 2;
    }
}

/// `0x5a6ef0` — decode one DXT1 block → ARGB8888.
pub fn dxt1_block_argb8888(block: &[u8], dst: &mut [u8], row_off: &mut [usize], b: Bounds) {
    let pal = dxt1_pal_argb8888(rd16(block, 0), rd16(block, 2));
    for y in b.y0..=b.y1 {
        let yi = y as usize;
        let mut bits = (block[4 + yi] as u32) >> (b.x0 * 2);
        let base = row_off[yi];
        for x in b.x0..=b.x1 {
            let o = base + (x as usize) * 4;
            dst[o..o + 4].copy_from_slice(&pal[(bits & 3) as usize].to_le_bytes());
            bits >>= 2;
        }
        row_off[yi] = base + (b.adv as usize) * 4;
    }
}

/// `0x5a62f0` — decode one DXT3 block → ARGB8888. The DXT3 block is `[8B explicit alpha][8B
/// DXT1-style colour]`: alpha word for row `y` at `block[y*2]` (4 bits/pixel), colour endpoints at
/// `block+8`/`block+10`, colour index byte for row `y` at `block[12+y]`. `alpha_fn` expands the
/// 4-bit nibble (the binary's callback).
pub fn dxt3_block_argb8888(
    block: &[u8],
    dst: &mut [u8],
    row_off: &mut [usize],
    b: Bounds,
    alpha_fn: fn(u32) -> u32,
) {
    let pal = dxt3_pal_argb8888(rd16(block, 8), rd16(block, 10));
    for y in b.y0..=b.y1 {
        let yi = y as usize;
        let mut cbits = (block[12 + yi] as u32) >> (b.x0 * 2);
        let mut abits = (rd16(block, yi * 2) as u32) >> (b.x0 * 4);
        let base = row_off[yi];
        for x in b.x0..=b.x1 {
            let a = alpha_fn(abits & 0xf) & 0xff;
            let color = pal[(cbits & 3) as usize] & 0x00ff_ffff;
            let px = color | (a << 24);
            let o = base + (x as usize) * 4;
            dst[o..o + 4].copy_from_slice(&px.to_le_bytes());
            cbits >>= 2;
            abits >>= 4;
        }
        row_off[yi] = base + (b.adv as usize) * 4;
    }
}

/// `0x5a65c0` — decode one DXT3 block → ARGB4444. As [`dxt3_block_argb8888`] but 16bpp output:
/// `(pal[idx] & 0xfff) | (alpha_nibble << 12)`.
pub fn dxt3_block_argb4444(
    block: &[u8],
    dst: &mut [u8],
    row_off: &mut [usize],
    b: Bounds,
    alpha_fn: fn(u32) -> u32,
) {
    let pal = dxt3_pal_argb4444(rd16(block, 8), rd16(block, 10));
    for y in b.y0..=b.y1 {
        let yi = y as usize;
        let mut cbits = (block[12 + yi] as u32) >> (b.x0 * 2);
        let mut abits = (rd16(block, yi * 2) as u32) >> (b.x0 * 4);
        let base = row_off[yi];
        for x in b.x0..=b.x1 {
            let a = alpha_fn(abits & 0xf) & 0xff;
            let color = (pal[(cbits & 3) as usize] as u32) & 0xfff;
            let px = (color | (a << 12)) as u16;
            let o = base + (x as usize) * 2;
            dst[o..o + 2].copy_from_slice(&px.to_le_bytes());
            cbits >>= 2;
            abits >>= 4;
        }
        row_off[yi] = base + (b.adv as usize) * 2;
    }
}

// ============================================================================================
// Block-grid drivers  (0x5a57c0 / 5a20 / 5c80 / 5ef0 / 60c0)
// ============================================================================================
//
// Converter ABI: `__fastcall(ecx=&dims{w,h}, edx=src, [esp+4]=srcStride, [esp+8]=dst,
// [esp+0xc]=dstStride)`. The surface is tiled into 4×4 blocks (`ceil(w/4) x ceil(h/4)`), the dst
// row-pointer array is built per block-row (`row_off[k] = (by*4+k)*dstStride`) and marched across
// the row by the per-block decoder's `adv`. Aligned surfaces (w,h >= 4 and multiples of 4) take the
// full path (bounds `{0,0,3,3,4}`); otherwise the edge path clamps the bounds to `min(dim-1,3)`
// (the binary clamps against the surface dim, not the per-block remainder — matched faithfully).

#[allow(clippy::too_many_arguments)]
fn drive(
    w: u32,
    h: u32,
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
    block_size: usize,
    mut decode: impl FnMut(&[u8], &mut [u8], &mut [usize], Bounds),
) {
    if w == 0 || h == 0 {
        return;
    }
    let bounds = if w < 4 || h < 4 || (w & 3) != 0 || (h & 3) != 0 {
        let cx = (w - 1).min(3);
        let cy = (h - 1).min(3);
        Bounds {
            x0: 0,
            y0: 0,
            x1: cx,
            y1: cy,
            adv: cx + 1,
        }
    } else {
        Bounds {
            x0: 0,
            y0: 0,
            x1: 3,
            y1: 3,
            adv: 4,
        }
    };
    let nbx = w.div_ceil(4);
    let nby = h.div_ceil(4);
    for by in 0..nby {
        let mut row_off = [0usize; 4];
        for (k, ro) in row_off.iter_mut().enumerate() {
            *ro = (by as usize * 4 + k) * dst_stride;
        }
        for bx in 0..nbx {
            let blk = by as usize * src_stride + bx as usize * block_size;
            decode(&src[blk..], dst, &mut row_off, bounds);
        }
    }
}

/// `0x5a57c0` — DXT1 surface → RGB565.
pub fn dxt1_decode_rgb565(
    w: u32,
    h: u32,
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    drive(w, h, src, src_stride, dst, dst_stride, 8, dxt1_block_rgb565);
}

/// `0x5a5a20` — DXT1 surface → ARGB1555.
pub fn dxt1_decode_argb1555(
    w: u32,
    h: u32,
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    drive(
        w,
        h,
        src,
        src_stride,
        dst,
        dst_stride,
        8,
        dxt1_block_argb1555,
    );
}

/// `0x5a5c80` — DXT1 surface → ARGB8888 (BGRA8888 in memory order).
pub fn dxt1_decode_argb8888(
    w: u32,
    h: u32,
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    drive(
        w,
        h,
        src,
        src_stride,
        dst,
        dst_stride,
        8,
        dxt1_block_argb8888,
    );
}

/// `0x5a5ef0` — DXT3 surface → ARGB4444.
pub fn dxt3_decode_argb4444(
    w: u32,
    h: u32,
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    drive(
        w,
        h,
        src,
        src_stride,
        dst,
        dst_stride,
        16,
        |blk, d, ro, b| dxt3_block_argb4444(blk, d, ro, b, alpha_identity),
    );
}

/// `0x5a60c0` — DXT3 surface → ARGB8888.
pub fn dxt3_decode_argb8888(
    w: u32,
    h: u32,
    src: &[u8],
    src_stride: usize,
    dst: &mut [u8],
    dst_stride: usize,
) {
    drive(
        w,
        h,
        src,
        src_stride,
        dst,
        dst_stride,
        16,
        |blk, d, ro, b| dxt3_block_argb8888(blk, d, ro, b, alpha_nibble_shl4),
    );
}
