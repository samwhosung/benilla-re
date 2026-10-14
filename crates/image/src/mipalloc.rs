//! BLP and mip allocation and layout: the absolute-pointer mip table population, the alpha-weighted
//! box-filter mip downsample, the BLP format-translate LUT, and the per-mip byte-size arithmetic.
//! These build on the format-agnostic [`crate::mipgeo`] size formulas; `fmt` is the gx device-format
//! id (its meaning belongs to the graphics layer; this layer consumes it).

use crate::mipgeo::{level_count, level_size};

/// Populate a mip chain's absolute-pointer table for a `w x h` image in gx format `fmt`, given the
/// table's base address `base`. The chain is one contiguous block: a leading array of `count` 32-bit
/// pointers, then the concatenated mip pixel data, so `table[i] = base + count*4 + Σ_{j<i} level_size(j)`
/// (i.e. each entry points at the start of that level's pixels, just past the pointer array). This is
/// the offset math shared by `0x5a4af0` (which `SMemAlloc`s `base` for `mip_sum + count*4` bytes,
/// then populates) and `0x5a4bb0` (populates a caller-supplied `base`). Bit-exact transcription.
pub fn populate_mip_table(fmt: u32, w: u32, h: u32, base: u32) -> Vec<u32> {
    let count = level_count(w, h);
    let data_base = base.wrapping_add(count.wrapping_mul(4));
    let mut table = Vec::with_capacity(count as usize);
    let mut acc = 0u32;
    let mut i = 0u32;
    while i < count {
        table.push(data_base.wrapping_add(acc));
        acc = acc.wrapping_add(level_size(i, w, h, fmt));
        i += 1;
    }
    table
}

/// Box-filter mip downsample of a 32-bit **BGRA** image (`0x5a4c30`). For each destination
/// pixel, averages the `bh x bw` source block (`bw = src_w/dst_w`, `bh = src_h/dst_h`, integer
/// division), weighting each channel by its source alpha. When the block's alpha sum is non-zero
/// the RGB channels are `Σ(channel·A) / ΣA` and the alpha is `ΣA / (bw·bh)`; when it is zero the
/// channels fall back to the plain `Σchannel / count` average with an alpha of 0. All divisions are
/// signed (`__alldiv`); only the low byte of each quotient is kept. Source pixel byte order is
/// `[0]=B,[1]=G,[2]=R,[3]=A`; the output dword is written in the same BGRA order. `dst` holds
/// `dst_w·dst_h` dwords; `src` holds `src_w·src_h` dwords (caller must size the block ratios so the
/// per-block reads stay in bounds — the binary does no clamping). Bit-exact transcription.
pub fn box_downsample(dst: &mut [u8], dst_w: u32, dst_h: u32, src: &[u8], src_w: u32, src_h: u32) {
    let bw = (src_w / dst_w) as usize; // block width
    let bh = (src_h / dst_h) as usize; // block height
    let src_stride = src_w as usize * 4; // source row pitch in bytes
    let mut src_off = 0usize; // byte offset into src
    let mut dst_idx = 0usize; // destination dword index
    for _row in 0..dst_h {
        for _col in 0..dst_w {
            // 64-bit accumulators (the binary keeps explicit lo/hi dword pairs with CARRY4).
            let (mut sum_a, mut sum_ra, mut sum_ga, mut sum_ba): (i64, i64, i64, i64) =
                (0, 0, 0, 0);
            let (mut sum_r, mut sum_g, mut sum_b): (i64, i64, i64) = (0, 0, 0);
            let mut count: i64 = 0;
            let mut row_off = src_off; // the block's current row
            for _by in 0..bh {
                let mut p = row_off + 3; // the alpha byte of the row's first pixel
                for _bx in 0..bw {
                    let a = i64::from(src[p]); // [+3] A
                    let b = i64::from(src[p - 3]); // [+0] B
                    let g = i64::from(src[p - 2]); // [+1] G
                    let r = i64::from(src[p - 1]); // [+2] R
                    sum_a += a;
                    sum_ba += b * a;
                    sum_ga += g * a;
                    sum_ra += r * a;
                    count += 1;
                    sum_r += r;
                    sum_g += g;
                    sum_b += b;
                    p += 4;
                }
                row_off += src_stride;
            }
            let out: u32 = if sum_a != 0 {
                let b = ((sum_ba / sum_a) as u8) as u32;
                let g = ((sum_ga / sum_a) as u8) as u32;
                let r = ((sum_ra / sum_a) as u8) as u32;
                let a = ((sum_a / (bw as i64 * bh as i64)) as u8) as u32;
                (a << 24) | (r << 16) | (g << 8) | b
            } else {
                let b = ((sum_b / count) as u8) as u32;
                let g = ((sum_g / count) as u8) as u32;
                let r = ((sum_r / count) as u8) as u32;
                (r << 16) | (g << 8) | b
            };
            let o = dst_idx * 4;
            dst[o..o + 4].copy_from_slice(&out.to_le_bytes());
            dst_idx += 1;
            src_off += bw * 4; // next block along the row
        }
        src_off += (bh - 1) * src_stride; // the rest of the block rows
    }
}

/// The BLP format-translate LUT at `0x85c8c8` (`0x5a80d0` returns `*(u32*)(0x85c8c8 + idx*4)`).
/// Maps a gx device-format id / BLP alphaType (the two small format enums, range `0..=9`) to a
/// conversion-matrix class (1=BGRA8888, 2=ARGB4444, 3=ARGB1555, 4=RGB565, 5=DXT1, 6=DXT3, 7=DXT5,
/// 8=other16, 0=none). The exe's `.data` bytes at file offset `0x45c8c8` are
/// `05 06 01 03 02 04 00 07 00 00` (idx 0..=9).
pub const FORMAT_LUT: [u32; 10] = [5, 6, 1, 3, 2, 4, 0, 7, 0, 0];

/// `0x5a80d0` — index the [`FORMAT_LUT`] (the binary does no bounds check).
pub fn format_translate(idx: u32) -> u32 {
    FORMAT_LUT[idx as usize]
}

/// BLP `ComputeMipByteSize` (`0x5a8140`): for a BLP whose full dims are `width`/`height`, the
/// byte size and row pitch of mip `level` in gx format `fmt`. Returns `(out_size, out_row, ret)`
/// where `ret` is the binary's nonzero-on-success value. Level dims clamp to `max(1, dim>>level)`;
/// `area = w_l·h_l`, `q = max(area>>2, 1)`; fmt2 → `(area*4, w_l*4)`; fmt{3,4,5} → `(area*2, w_l*2)`;
/// fmt9 → `(area*2 + q, w_l*2)`; any other fmt → `(0, 0)` and `ret = 0`. Bit-exact transcription.
pub fn compute_mip_byte_size(width: u32, height: u32, fmt: u32, level: u32) -> (u32, u32, u32) {
    let mut wl = width >> (level & 0x1f);
    if wl < 2 {
        wl = 1;
    }
    let mut hl = height >> (level & 0x1f);
    if hl < 2 {
        hl = 1;
    }
    let area = hl.wrapping_mul(wl);
    let mut q = area >> 2;
    if q == 0 {
        q = 1;
    }
    match fmt {
        2 => (area.wrapping_mul(4), wl.wrapping_mul(4), 1),
        3..=5 => (area.wrapping_mul(2), wl.wrapping_mul(2), 1),
        9 => (q.wrapping_add(area.wrapping_mul(2)), wl.wrapping_mul(2), 1),
        _ => (0, 0, 0),
    }
}
