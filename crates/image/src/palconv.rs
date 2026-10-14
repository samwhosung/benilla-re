//! The BLP2 paletted-to-RGB and paletted-to-BGRA converters.
//!
//! These converters are reached from the paletted BLP decode path. The source is an 8-bit
//! palette-index plane (optionally followed by an alpha plane); the BLP palette is 256 x u32 BGRA at
//! `this+0x98` (entry byte `[0]` B, `[1]` G, `[2]` R, `[3]` A, so for index `i` the exe reads
//! B = pal[i*4+0], G = pal[i*4+1], R = pal[i*4+2] at offsets +0x98, +0x99 and +0x9a). `alpha_depth`
//! is the BLP `alphaDepth` field (`this+0xd`).
//!
//! Two families, tested by direct emulated invocation:
//!  - palette-expand to BGRA8888 (`0x5a8790`, `0x5a87d0`);
//!  - paletted to RGB444, RGB555, RGB565 and RGB565 with a 2-bit alpha plane by Floyd-Steinberg
//!    dither (`0x5a92d0`, `0x5a9580`, `0x5a9860`, `0x5a9a80`). Each call zeroes a two-row
//!    ping-pong error buffer on entry, so it is self-contained.
//!
//! The no-dither (`0x5a8910`, `0x5a8a40`, `0x5a8b80`) and ordered-dither (`0x5a8ea0`, `0x5a9030`,
//! `0x5a91d0`) siblings have no live caller and are not transcribed.
//!
//! Several alpha branches read fixed `.data` tables (the {0, 0xff} 1-bit table, the nibble-replicate
//! table, the 2-bit-alpha mask and shift tables), and the RGB444 `alphaDepth == 1` merge over-reads
//! a 2-entry table at `0x85c8c4` by the bit value, which is a degenerate combination: the read runs
//! on through the Bayer matrices that follow. All of these resolve against [`DATA`], the `.data`
//! slice `[0x85c8b0, 0x85c9d0)` of the exe (numeric constants only; the over-read stops before the
//! string at `0x85c9d0`).

/// The `.data` slice `[0x85c8b0, 0x85c9d0)` of `WoW.exe`, holding every fixed table these converters
/// index (including the degenerate over-read offsets). Read by [`d8`]/[`d16`]/[`di32`].
const DATA_BASE: usize = 0x85c8b0;
static DATA: [u8; 288] = [
    // 0x85c8b0  nibble-replicate table (n -> n*0x11) [16 bytes]
    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
    0xff, //
    // 0x85c8c0  {0x00,0xff} 1-bit alpha (bytes); then the 0x85c8c4 over-read region begins
    0x00, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, //
    0x05, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
    // 0x85c8f0  Bayer 4x4 for RGB444 ordered dither [16 i32] (reached only by the over-read)
    0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00,
    0x0c, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00,
    0x03, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
    0x0f, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00,
    0x00, //
    // 0x85c930  Bayer 4x4 for RGB555 (all channels) + RGB565 R/B [16 i32]
    0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
    0x06, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
    0x00, //
    // 0x85c970  Bayer 4x4 for RGB565 G [16 i32]
    0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
    0x00, //
    // 0x85c9b0  2-bit-alpha field masks (bytes) | 0x85c9b4 field shifts (i32) | 0x85c9c8 4-bit shifts
    0x03, 0x0c, 0x30, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x06, 0x00, 0x00, 0x00, 0x0f, 0xf0, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00,
];

#[inline]
fn d8(addr: usize) -> u8 {
    DATA[addr - DATA_BASE]
}
#[inline]
fn d16(addr: usize) -> u16 {
    let o = addr - DATA_BASE;
    u16::from_le_bytes([DATA[o], DATA[o + 1]])
}
#[inline]
fn di32(addr: usize) -> i32 {
    let o = addr - DATA_BASE;
    i32::from_le_bytes([DATA[o], DATA[o + 1], DATA[o + 2], DATA[o + 3]])
}

#[inline]
fn rd16(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}
#[inline]
fn wr16(b: &mut [u8], i: usize, v: u16) {
    b[i..i + 2].copy_from_slice(&v.to_le_bytes());
}

// ============================================================================================
// Palette-expand → BGRA8888
// ============================================================================================

/// `0x5a8790` — palette-expand → BGRA8888 with a trailing 8-bit alpha plane.
/// `__thiscall(ecx=this, [+8]=dst, [+0xc]=src, [+0x10]=count)`. `dst[i] = palette[idx[i]]` (BGRA dword)
/// then the alpha byte is overwritten from the alpha plane at `src[count + i]`.
pub fn expand_bgra8888_a8(pal: &[u8], dst: &mut [u8], src: &[u8], count: usize) {
    for i in 0..count {
        let idx = src[i] as usize;
        dst[i * 4] = pal[idx * 4]; // B
        dst[i * 4 + 1] = pal[idx * 4 + 1]; // G
        dst[i * 4 + 2] = pal[idx * 4 + 2]; // R
        dst[i * 4 + 3] = src[count + i]; // A (alpha plane), overwriting palette A
    }
}

/// `0x5a87d0` — palette-expand → BGRA8888, then expand the alpha plane per `alpha_depth`:
/// `==1` → A = {0,0xff}[bit] (table `0x85c8c0`); `==4` → A = nibble-replicate (table `0x85c8b0`),
/// 2 px/byte; `==8` → A = alpha byte. Any other depth leaves A = 0xff.
pub fn expand_bgra8888_adepth(
    pal: &[u8],
    alpha_depth: u8,
    dst: &mut [u8],
    src: &[u8],
    count: usize,
) {
    for i in 0..count {
        let idx = src[i] as usize;
        dst[i * 4] = pal[idx * 4];
        dst[i * 4 + 1] = pal[idx * 4 + 1];
        dst[i * 4 + 2] = pal[idx * 4 + 2];
        dst[i * 4 + 3] = 0xff;
    }
    let alpha = &src[count..];
    match alpha_depth {
        1 => {
            let full = count >> 3;
            for (blk, &abyte) in alpha[..full].iter().enumerate() {
                let mut bits = abyte;
                for k in 0..8 {
                    let pix = blk * 8 + k;
                    dst[pix * 4 + 3] = d8(0x85c8c0 + (bits & 1) as usize);
                    bits >>= 1;
                }
            }
            let rem = count & 7;
            if rem != 0 {
                let mut bits = alpha[full];
                for k in 0..rem {
                    let pix = full * 8 + k;
                    dst[pix * 4 + 3] = d8(0x85c8c0 + (bits & 1) as usize);
                    bits >>= 1;
                }
            }
        }
        4 => {
            let half = count >> 1;
            for j in 0..half {
                let b = alpha[j];
                dst[(2 * j) * 4 + 3] = d8(0x85c8b0 + (b & 0xf) as usize);
                dst[(2 * j + 1) * 4 + 3] = d8(0x85c8b0 + (b >> 4) as usize);
            }
            if count & 1 != 0 {
                let b = alpha[half];
                dst[(2 * half) * 4 + 3] = d8(0x85c8b0 + (b & 0xf) as usize);
            }
        }
        8 => {
            for i in 0..count {
                dst[i * 4 + 3] = alpha[i];
            }
        }
        _ => {}
    }
}

// ============================================================================================
// Shared alpha-merge passes (paletted RGB444→ARGB4444 / RGB555→ARGB1555)
// ============================================================================================

/// RGB444 alpha-merge (shared by `0x5a8910`/`0x5a8ea0`/`0x5a92d0`): OR the alpha bits into the high
/// nibble of the already-written RGB444 word, producing ARGB4444. `alpha` is the alpha plane.
/// `alpha_depth==1` is the degenerate over-read of the 2-entry `0x85c8c4` table by the bit VALUE.
fn alpha_merge_444(dst: &mut [u8], alpha_depth: u8, alpha: &[u8], count: usize) {
    match alpha_depth {
        1 => {
            let mut mask = 1u32;
            let mut ai = 0usize;
            for i in 0..count {
                let sb = alpha[ai] as u32;
                let off = (sb & mask) as usize * 2;
                let v = d16(0x85c8c4 + off);
                let cur = rd16(dst, i * 2);
                wr16(dst, i * 2, cur | v);
                mask <<= 1;
                if mask >= 0x100 {
                    mask = 1;
                    ai += 1;
                }
            }
        }
        4 => {
            let mut ai = 0usize;
            for i in 0..count {
                let cur = rd16(dst, i * 2) as u32;
                if i & 1 == 0 {
                    let v = (alpha[ai] as u32) << 0xc;
                    wr16(dst, i * 2, (cur | v) as u16);
                } else {
                    let v = ((alpha[ai] & 0xf0) as u32) << 8;
                    wr16(dst, i * 2, (cur | v) as u16);
                    ai += 1;
                }
            }
        }
        8 => {
            for (i, &a) in alpha[..count].iter().enumerate() {
                let cur = rd16(dst, i * 2) as u32;
                let v = ((a & 0xf0) as u32) << 8;
                wr16(dst, i * 2, (cur | v) as u16);
            }
        }
        _ => {}
    }
}

/// RGB555 alpha-merge (shared by `0x5a8a40`/`0x5a9030`/`0x5a9580`): OR the alpha bit into bit 15 of
/// the already-written RGB555 word, producing ARGB1555. `alpha` is the alpha plane.
fn alpha_merge_555(dst: &mut [u8], alpha_depth: u8, alpha: &[u8], count: usize) {
    match alpha_depth {
        1 => {
            let mut bit = 0u32;
            let mut ai = 0usize;
            for i in 0..count {
                let sb = alpha[ai] as u32;
                let v = (((1u32 << (bit & 0x1f)) & sb) << ((0xf - bit) & 0x1f)) as u16;
                let cur = rd16(dst, i * 2);
                wr16(dst, i * 2, cur | v);
                bit += 1;
                if bit > 7 {
                    bit = 0;
                    ai += 1;
                }
            }
        }
        4 => {
            let mut state = 0u32;
            let mut ai = 0usize;
            for i in 0..count {
                let cur = rd16(dst, i * 2) as u32;
                if state == 0 {
                    let v = ((alpha[ai] as u32) & 0xfff8) << 0xc;
                    wr16(dst, i * 2, (cur | v) as u16);
                    state = alpha_depth as u32;
                } else {
                    state = 0;
                    let v = ((alpha[ai] as u32) & 0x80) << 8;
                    wr16(dst, i * 2, (cur | v) as u16);
                    ai += 1;
                }
            }
        }
        8 => {
            for (i, &a) in alpha[..count].iter().enumerate() {
                let cur = rd16(dst, i * 2) as u32;
                let v = ((a as u32) & 0x80) << 8;
                wr16(dst, i * 2, (cur | v) as u16);
            }
        }
        _ => {}
    }
}

// ============================================================================================
// Floyd-Steinberg converters
// ============================================================================================

/// Per-channel FS quantize parameters (R, G, B): additive bias, arithmetic-right-shift to depth,
/// the high-bits round mask, and the clamp max. The quant error is `accum - (accum+bias & mask)`.
struct FsParams {
    bias: [i32; 3],
    shift: [u32; 3],
    mask: [u32; 3],
    clamp: [i32; 3],
    /// pack: `(qR<<rsh) | (qG<<gsh) | qB`.
    rsh: u32,
    gsh: u32,
}

/// Dwords per error-buffer row in the binary's static BSS (`0x3018` bytes = `0xc06` dwords).
const FS_STRIDE: usize = 0xc06;

/// The shared Floyd-Steinberg color core: a two-row ping-pong error buffer (zeroed on entry, so each
/// call is self-contained), 3 ints (R,G,B) per pixel with 2 leading padding columns. Per pixel:
/// `accum = palChan<<16 + (incomingError>>4)`, +bias, arith-shift, clamp, pack; the quant error is
/// distributed by the binary's {7 right, 5 below-left, 3 below, 1 below-right}/16 kernel (the `/16`
/// is the `>>4` applied when the error is consumed). Writes one RGB word per pixel, row-major.
fn fs_core(pal: &[u8], dst: &mut [u8], src: &[u8], w: usize, h: usize, p: &FsParams) {
    let mut err = vec![0i32; 2 * FS_STRIDE];
    for row in 0..h {
        let cur = row & 1;
        let prev = row.wrapping_sub(1) & 1;
        for k in 0..6 {
            err[prev * FS_STRIDE + k] = 0;
        }
        let mut p7 = 6 + cur * FS_STRIDE; // current row, error read/forward base
        let mut p4 = 1 + prev * FS_STRIDE; // next row, error distribute base
        for col in 0..w {
            let idx = src[row * w + col] as usize;
            let chan = [
                pal[idx * 4 + 2] as i32, // R (+0x9a)
                pal[idx * 4 + 1] as i32, // G (+0x99)
                pal[idx * 4] as i32,     // B (+0x98)
            ];
            let mut e = [0i32; 3];
            let mut q = [0i32; 3];
            for c in 0..3 {
                let incoming = err[p7 - 3 + c] >> 4;
                let accum = (chan[c] << 16).wrapping_add(incoming);
                let biased = accum.wrapping_add(p.bias[c]);
                q[c] = (biased >> p.shift[c]).clamp(0, p.clamp[c]);
                e[c] = accum.wrapping_sub((biased as u32 & p.mask[c]) as i32);
            }
            let pixel = ((q[0] << p.rsh) | (q[1] << p.gsh) | q[2]) as u16;
            wr16(dst, (row * w + col) * 2, pixel);
            // distribute the quant error: 7 -> current row (next pixel), {5,3} -> next row adds,
            // {1} (the cross / below-right) -> next-row stores at p4+5..p4+7.
            for c in 0..3 {
                err[p7 + c] = err[p7 + c].wrapping_add(e[c].wrapping_mul(7));
            }
            for c in 0..3 {
                err[p4 - 1 + c] = err[p4 - 1 + c].wrapping_add(e[c].wrapping_mul(5));
            }
            for c in 0..3 {
                err[p4 + 2 + c] = err[p4 + 2 + c].wrapping_add(e[c].wrapping_mul(3));
            }
            err[p4 + 5] = e[0];
            err[p4 + 6] = e[1];
            err[p4 + 7] = e[2];
            p7 += 3;
            p4 += 3;
        }
    }
}

const FS_444: FsParams = FsParams {
    bias: [0x80000; 3],
    shift: [0x14; 3],
    mask: [0xfff00000; 3],
    clamp: [0xf; 3],
    rsh: 8,
    gsh: 4,
};
const FS_555: FsParams = FsParams {
    bias: [0x40000; 3],
    shift: [0x13; 3],
    mask: [0xfff80000; 3],
    clamp: [0x1f; 3],
    rsh: 10,
    gsh: 5,
};
const FS_565: FsParams = FsParams {
    bias: [0x40000, 0x20000, 0x40000],
    shift: [0x13, 0x12, 0x13],
    mask: [0xfff80000, 0xfffc0000, 0xfff80000],
    clamp: [0x1f, 0x3f, 0x1f],
    rsh: 11,
    gsh: 5,
};

/// `0x5a92d0` — paletted → RGB444 (then ARGB4444), Floyd-Steinberg dither.
/// `__thiscall(ecx=this, [+8]=dst, [+0xc]=src, [+0x10]=w, [+0x14]=h)`.
pub fn fs_rgb444(pal: &[u8], alpha_depth: u8, dst: &mut [u8], src: &[u8], w: usize, h: usize) {
    fs_core(pal, dst, src, w, h, &FS_444);
    let count = w * h;
    alpha_merge_444(dst, alpha_depth, &src[count..], count);
}

/// `0x5a9580` — paletted → RGB555 (then ARGB1555), Floyd-Steinberg dither.
pub fn fs_rgb555(pal: &[u8], alpha_depth: u8, dst: &mut [u8], src: &[u8], w: usize, h: usize) {
    fs_core(pal, dst, src, w, h, &FS_555);
    let count = w * h;
    alpha_merge_555(dst, alpha_depth, &src[count..], count);
}

/// `0x5a9860` — paletted → RGB565, Floyd-Steinberg dither (opaque, no alpha plane).
pub fn fs_rgb565(pal: &[u8], dst: &mut [u8], src: &[u8], w: usize, h: usize) {
    fs_core(pal, dst, src, w, h, &FS_565);
}

/// `0x5a9a80` — paletted → RGB565 (FS) + a packed 2-bit alpha plane written after the color plane
/// at `dst + count*2`. `alpha_depth==1` → bit-walk; `==4` → nibble (shift table `0x85c9c8` = {2,6});
/// `==8` → top 2 bits. Field mask/shift tables at `0x85c9b0`/`0x85c9b4`.
pub fn fs_rgb565_a2(pal: &[u8], alpha_depth: u8, dst: &mut [u8], src: &[u8], w: usize, h: usize) {
    fs_core(pal, dst, src, w, h, &FS_565);
    let count = w * h;
    let alpha = &src[count..];
    let base = count * 2; // alpha plane starts right after the color plane
    match alpha_depth {
        1 => {
            for i in 0..count {
                let db = base + (i >> 2);
                let bits = (alpha[i >> 3] >> (i & 7)) as u32;
                let shift2 = (di32(0x85c9b4 + (i & 3) * 4) as u32) & 0x1f;
                let mask = d8(0x85c9b0 + (i & 3));
                let val = ((bits << 1) << shift2) as u8;
                dst[db] = val | (!mask & dst[db]);
            }
        }
        4 => {
            for i in 0..count {
                let db = base + (i >> 2);
                let shift1 = d8(0x85c9c8 + (i & 1) * 4);
                let bits = (alpha[i >> 1] >> shift1) as u32;
                let shift2 = (di32(0x85c9b4 + (i & 3) * 4) as u32) & 0x1f;
                let mask = d8(0x85c9b0 + (i & 3));
                let val = (bits << shift2) as u8;
                dst[db] = val | (!mask & dst[db]);
            }
        }
        8 => {
            for (i, &a) in alpha[..count].iter().enumerate() {
                let db = base + (i >> 2);
                let bits = (a >> 6) as u32;
                let shift2 = (di32(0x85c9b4 + (i & 3) * 4) as u32) & 0x1f;
                let mask = d8(0x85c9b0 + (i & 3));
                let val = (bits << shift2) as u8;
                dst[db] = val | (!mask & dst[db]);
            }
        }
        _ => {}
    }
}
