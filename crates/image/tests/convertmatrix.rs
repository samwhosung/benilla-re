//! Bit-exact tests of the conversion-matrix leaves against `WoW.exe`: each runs on synthesized and
//! fuzzed buffers in the emulator, and the transcription must be byte-equal over the full written
//! region, including the 16-bit packers' unrolled odd-width extra pixel and the untouched stride
//! padding and over-copy guard bytes. The in-place operations (alpha key, blend) read dst, so dst is
//! seeded with fuzzed bytes.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_image::convertmatrix::{
    bgra8888_alpha_key, bgra8888_blend, bgra8888_copy, bgra8888_to_argb1555, bgra8888_to_argb4444,
    copy16, dxt1_block_copy, dxt_block_copy_16,
};
use common::next;

const ARGB4444: u64 = 0x5a5070; // __fastcall(ecx=&dims{w,h}, edx=src, [esp+4]=srcStride, [esp+8]=dst, [esp+0xc]=dstStride)
const ARGB1555: u64 = 0x5a51d0;
const COPY32: u64 = 0x5a5480;
const ALPHA_KEY: u64 = 0x5a5510;
const BLEND: u64 = 0x5a5590;
const COPY16: u64 = 0x5a56b0;
const DXT1: u64 = 0x5a5740; // __fastcall(ecx=&dims, edx=src, [esp+4]=srcStride(unused), [esp+8]=dst)
const DXT35: u64 = 0x5a5780;

/// Deterministic alpha-probe set hit by every blend/key pixel before the fuzz: 0, 0xff, and a spread
/// of mid values (the source-over branch divides the range).
const ALPHAS: [u8; 8] = [0x00, 0xff, 0x80, 0x01, 0x7f, 0xfe, 0x40, 0xc0];

fn packed_width(w: u32) -> u32 {
    if w < 2 {
        w
    } else {
        (w + 1) & !1
    }
}

fn fuzz(buf: &mut [u8], s: &mut u32) {
    for b in buf.iter_mut() {
        *b = (next(s) >> 16) as u8;
    }
}

// ---- 16-bit packers (BGRA8888 -> ARGB4444 / ARGB1555) -------------------------------------------
// Same buffer layout as the RGB565 sibling: src is BGRA (4 B/px) over packed_width columns, dst is
// 16bpp (2 B/px); both the simple (w<2) and unrolled (w>=2, odd rounds up) regions are covered.

type Pack16 = fn(u32, u32, &[u8], usize, &mut [u8], usize);

fn check_pack16(wow: &Wow, addr: u64, ours_fn: Pack16, w: u32, h: u32, s: &mut u32) {
    let pw = packed_width(w) as usize;
    let src_stride = pw * 4;
    let dst_stride = pw * 2;
    let src_bytes = src_stride * h as usize;
    let dst_bytes = dst_stride * h as usize;

    let mut e = Emu::new(wow);
    let dims = e.alloc(8);
    e.write_u32(dims, w);
    e.write_u32(dims + 4, h);
    let src_va = e.alloc(src_bytes.max(4) as u64);
    let dst_va = e.alloc(dst_bytes.max(4) as u64);

    let mut src = vec![0u8; src_bytes];
    fuzz(&mut src, s);
    e.write(src_va, &src);
    let sentinel = vec![0xAAu8; dst_bytes];
    e.write(dst_va, &sentinel);

    e.call(
        addr,
        dims as u32,
        src_va as u32,
        &[src_stride as u32, dst_va as u32, dst_stride as u32],
    );
    let bin = e.read(dst_va, dst_bytes);

    let mut ours = sentinel.clone();
    ours_fn(w, h, &src, src_stride, &mut ours, dst_stride);
    assert_eq!(bin, ours, "pack16 {addr:#x} {w}x{h}");
}

const PACK16_DIMS: &[(u32, u32)] = &[
    (1, 1),
    (2, 1),
    (3, 1),
    (2, 2),
    (4, 4),
    (5, 3),
    (7, 2),
    (1, 4),
    (8, 8),
    (2, 4),
    (6, 1),
    (9, 5),
];

#[test]
fn image_bgra8888_to_argb4444() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x4444_0001u32;
    for &(w, h) in PACK16_DIMS {
        check_pack16(&wow, ARGB4444, bgra8888_to_argb4444, w, h, &mut s);
    }
}

#[test]
fn image_bgra8888_to_argb1555() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x1555_0001u32;
    for &(w, h) in PACK16_DIMS {
        check_pack16(&wow, ARGB1555, bgra8888_to_argb1555, w, h, &mut s);
    }
}

// ---- stride-aware byte copies (no dst read) -----------------------------------------------------
// `bpp` selects the 32bpp plain copy (4) or the 16bpp copy (2). `pad` of 0/0 exercises the contiguous
// fast path; nonzero pad forces the per-row slow path and leaves padding/tail bytes as sentinel.

type StrideCopy = fn(u32, u32, &[u8], usize, &mut [u8], usize);

fn check_copy(
    wow: &Wow,
    addr: u64,
    ours_fn: StrideCopy,
    bpp: usize,
    (w, h): (u32, u32),
    (src_pad, dst_pad): (usize, usize),
    s: &mut u32,
) {
    let src_stride = w as usize * bpp + src_pad;
    let dst_stride = w as usize * bpp + dst_pad;
    let src_bytes = src_stride * h as usize;
    let dst_bytes = dst_stride * h as usize;

    let mut e = Emu::new(wow);
    let dims = e.alloc(8);
    e.write_u32(dims, w);
    e.write_u32(dims + 4, h);
    let src_va = e.alloc(src_bytes.max(4) as u64);
    let dst_va = e.alloc(dst_bytes.max(4) as u64);

    let mut src = vec![0u8; src_bytes];
    fuzz(&mut src, s);
    e.write(src_va, &src);
    let sentinel = vec![0xAAu8; dst_bytes];
    e.write(dst_va, &sentinel);

    e.call(
        addr,
        dims as u32,
        src_va as u32,
        &[src_stride as u32, dst_va as u32, dst_stride as u32],
    );
    let bin = e.read(dst_va, dst_bytes);

    let mut ours = sentinel.clone();
    ours_fn(w, h, &src, src_stride, &mut ours, dst_stride);
    assert_eq!(
        bin, ours,
        "copy{bpp} {addr:#x} {w}x{h} pad {src_pad}/{dst_pad}"
    );
}

const COPY_DIMS: &[(u32, u32)] = &[
    (1, 1),
    (2, 1),
    (1, 3),
    (4, 4),
    (3, 5),
    (7, 2),
    (8, 8),
    (5, 1),
];
// (src_pad, dst_pad) in pixels-worth would vary; these are raw byte pads. 0/0 = fast path.
const PADS: &[(usize, usize)] = &[(0, 0), (8, 8), (8, 0), (0, 12), (4, 16)];

#[test]
fn image_bgra8888_copy() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x0032_0001u32;
    for &dim in COPY_DIMS {
        for &pad in PADS {
            check_copy(&wow, COPY32, bgra8888_copy, 4, dim, pad, &mut s);
        }
    }
}

#[test]
fn image_copy16() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x0016_0001u32;
    for &dim in COPY_DIMS {
        for &pad in PADS {
            check_copy(&wow, COPY16, copy16, 2, dim, pad, &mut s);
        }
    }
}

// ---- in-place 32bpp ops (read dst): alpha-key + source-over blend -------------------------------
// dst is seeded with fuzzed bytes. When `force_alpha`, every pixel's source alpha byte is overwritten
// from ALPHAS (guaranteeing the 0 / 0xff / mid branches); otherwise alpha stays fully fuzzed.

type InPlace = fn(u32, u32, &[u8], usize, &mut [u8], usize);

fn check_inplace(
    wow: &Wow,
    addr: u64,
    ours_fn: InPlace,
    (w, h): (u32, u32),
    (src_pad, dst_pad): (usize, usize),
    force_alpha: bool,
    s: &mut u32,
) {
    let src_stride = w as usize * 4 + src_pad;
    let dst_stride = w as usize * 4 + dst_pad;
    let src_bytes = src_stride * h as usize;
    let dst_bytes = dst_stride * h as usize;

    let mut e = Emu::new(wow);
    let dims = e.alloc(8);
    e.write_u32(dims, w);
    e.write_u32(dims + 4, h);
    let src_va = e.alloc(src_bytes.max(4) as u64);
    let dst_va = e.alloc(dst_bytes.max(4) as u64);

    let mut src = vec![0u8; src_bytes];
    fuzz(&mut src, s);
    if force_alpha {
        let mut k = 0usize;
        for y in 0..h as usize {
            for x in 0..w as usize {
                src[y * src_stride + x * 4 + 3] = ALPHAS[k % ALPHAS.len()];
                k += 1;
            }
        }
    }
    let mut dst0 = vec![0u8; dst_bytes];
    fuzz(&mut dst0, s);
    e.write(src_va, &src);
    e.write(dst_va, &dst0);

    e.call(
        addr,
        dims as u32,
        src_va as u32,
        &[src_stride as u32, dst_va as u32, dst_stride as u32],
    );
    let bin = e.read(dst_va, dst_bytes);

    let mut ours = dst0.clone();
    ours_fn(w, h, &src, src_stride, &mut ours, dst_stride);
    assert_eq!(
        bin, ours,
        "inplace {addr:#x} {w}x{h} force_alpha={force_alpha}"
    );
}

#[test]
fn image_bgra8888_alpha_key() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x0051_0001u32;
    for &dim in COPY_DIMS {
        check_inplace(
            &wow,
            ALPHA_KEY,
            bgra8888_alpha_key,
            dim,
            (0, 0),
            true,
            &mut s,
        );
        check_inplace(
            &wow,
            ALPHA_KEY,
            bgra8888_alpha_key,
            dim,
            (8, 12),
            false,
            &mut s,
        );
    }
}

#[test]
fn image_bgra8888_blend() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x0059_0001u32;
    for &dim in COPY_DIMS {
        check_inplace(&wow, BLEND, bgra8888_blend, dim, (0, 0), true, &mut s);
        check_inplace(&wow, BLEND, bgra8888_blend, dim, (8, 12), false, &mut s);
    }
}

// ---- DXT block copies (flat, size-driven; dims clamped >=4; strides ignored) --------------------
// Allocate src/dst to the upper bound `clamp(w)*clamp(h)` bytes plus a dst guard, so an over-copy
// past the per-format byte count (DXT1 = bound/2, DXT3/5 = bound) shows as a corrupted guard.

type Block = fn(u32, u32, &[u8], &mut [u8]);

fn check_block(wow: &Wow, addr: u64, ours_fn: Block, w: u32, h: u32, s: &mut u32) {
    let cw = if w < 5 { 4 } else { w };
    let ch = if h < 5 { 4 } else { h };
    let cap = (cw * ch) as usize; // upper bound on bytes copied (DXT3/5); DXT1 copies cap/2
    let src_bytes = cap.max(4);
    let dst_bytes = cap + 8; // + over-copy guard

    let mut e = Emu::new(wow);
    let dims = e.alloc(8);
    e.write_u32(dims, w);
    e.write_u32(dims + 4, h);
    let src_va = e.alloc(src_bytes as u64);
    let dst_va = e.alloc(dst_bytes as u64);

    let mut src = vec![0u8; src_bytes];
    fuzz(&mut src, s);
    e.write(src_va, &src);
    let sentinel = vec![0xAAu8; dst_bytes];
    e.write(dst_va, &sentinel);

    // [esp+4]=srcStride (unused by the binary), [esp+8]=dst.
    e.call(addr, dims as u32, src_va as u32, &[0, dst_va as u32]);
    let bin = e.read(dst_va, dst_bytes);

    let mut ours = sentinel.clone();
    ours_fn(w, h, &src, &mut ours);
    assert_eq!(bin, ours, "block {addr:#x} {w}x{h}");
}

const BLOCK_DIMS: &[(u32, u32)] = &[
    (1, 1),
    (4, 4),
    (2, 3),
    (8, 8),
    (12, 4),
    (3, 9),
    (5, 5),
    (16, 16),
    (7, 5),
];

#[test]
fn image_dxt1_block_copy() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x0074_0001u32;
    for &(w, h) in BLOCK_DIMS {
        check_block(&wow, DXT1, dxt1_block_copy, w, h, &mut s);
    }
}

#[test]
fn image_dxt35_block_copy() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x0078_0001u32;
    for &(w, h) in BLOCK_DIMS {
        check_block(&wow, DXT35, dxt_block_copy_16, w, h, &mut s);
    }
}
