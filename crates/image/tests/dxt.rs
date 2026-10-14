//! Bit-exact tests of the DXT/S3TC decode band against `WoW.exe`: each function runs in the emulator
//! on synthesized and fuzzed inputs, and the transcription must match bit for bit.
//!
//! Setup notes for this band:
//!  - the decoders and drivers read the shared interpolation LUT at `0xc0fa80`, which is built at run
//!    time (zero in a fresh emulator). [`fresh`] calls `0x5a7910(ecx=0xc0fa80)` to build it before any
//!    decode.
//!  - the per-block decoders and drivers run a CRT lazy-once guard `0x409aef` (an atexit
//!    registration, unsafe to run in isolation because it calls into the CRT heap). The caller sets
//!    the guard flag before the call, so `0x409aef` is stubbed to a no-op (`pop_args=0`, cdecl).
//!  - the row-pointer array the per-block decoders take (`edx`) is built into a `dst` buffer; the test
//!    compares the written `dst` pixels (the decoder advances the row pointers as scratch).

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_image::dxt::{
    alpha_identity, alpha_nibble_shl4, build_interp_lut, dxt1_block_argb1555, dxt1_block_argb8888,
    dxt1_block_rgb565, dxt1_decode_argb1555, dxt1_decode_argb8888, dxt1_decode_rgb565,
    dxt1_pal_argb1555, dxt1_pal_argb8888, dxt1_pal_rgb565, dxt3_block_argb4444,
    dxt3_block_argb8888, dxt3_decode_argb4444, dxt3_decode_argb8888, pack_565comp_argb1555,
    pack_565comp_argb4444, pack_565comp_argb8888, pack_word_argb1555, pack_word_argb8888, pitch,
    Bounds, PITCH_MULT, PITCH_SHIFT,
};
use common::next;

const LUT_VA: u64 = 0x00c0_fa80;
const STUB_ATEXIT: u64 = 0x0040_9aef;
const CB_IDENTITY: u64 = 0x005a_60b0; // alpha callback: nibble -> nibble
const CB_NIBBLE_SHL4: u64 = 0x005a_62b0; // alpha callback: nibble -> nibble<<4

// LUT-gen / pitch / packer addresses.
const LUT_GEN: u64 = 0x005a_7910;
const PITCH: u64 = 0x005a_62c0;
const PK_8888: u64 = 0x005a_6590;
const PK_4444: u64 = 0x005a_6880;
const PK_1555: u64 = 0x005a_6e70;
const PK_WORD_1555: u64 = 0x005a_6ea0;
const PK_WORD_8888: u64 = 0x005a_7190;

/// A fresh emulator with `0x409aef` stubbed (atexit guard) and the interpolation LUT built at
/// `0xc0fa80`. Reused across cases within a test (the LUT and stub persist).
fn fresh(wow: &Wow) -> Emu<'_> {
    let mut e = Emu::new(wow);
    e.stub(STUB_ATEXIT, 0, 0);
    e.call(LUT_GEN, LUT_VA as u32, 0, &[]);
    e
}

fn rdu16(e: &Emu, va: u64) -> u16 {
    let b = e.read(va, 2);
    u16::from_le_bytes([b[0], b[1]])
}

// ============================================================================================
// Interpolation LUT generator (0x5a7910)
// ============================================================================================

#[test]
fn image_dxt_interp_lut() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let base = e.alloc(0x180);
    let sentinel = vec![0x5Au8; 0x180];
    e.write(base, &sentinel);
    e.call(LUT_GEN, base as u32, 0, &[]);
    let bin = e.read(base, 0x180);
    let ours = build_interp_lut();
    assert_eq!(bin, &ours[..], "interp LUT");
}

// ============================================================================================
// Pixel-format row pitch (0x5a62c0) + table byte-verify
// ============================================================================================

#[test]
fn image_dxt_pitch() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // Byte-verify the embedded shift/mult tables against the `.data` slices in WoW.exe.
    let shift_bin = e.read(0x0085_c868, 36);
    let mult_bin = e.read(0x0085_c88c, 36);
    let mut shift_exp = Vec::new();
    let mut mult_exp = Vec::new();
    for &v in &PITCH_SHIFT {
        shift_exp.extend_from_slice(&v.to_le_bytes());
    }
    for &v in &PITCH_MULT {
        mult_exp.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(shift_bin, shift_exp, "pitch shift table 0x85c868");
    assert_eq!(mult_bin, mult_exp, "pitch mult table 0x85c88c");

    let dims = [
        0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 100, 128, 256, 1024,
    ];
    for fmt in 0..9u32 {
        for &dim in &dims {
            let bin = e.call(PITCH, fmt, dim, &[]);
            assert_eq!(bin, pitch(fmt, dim), "pitch fmt={fmt} dim={dim}");
        }
    }
}

// ============================================================================================
// Endpoint / component packers
// ============================================================================================

const RS: [u32; 7] = [0, 1, 2, 15, 16, 30, 31];
const GS: [u32; 8] = [0, 1, 2, 31, 32, 33, 62, 63];
const BS: [u32; 7] = [0, 1, 2, 15, 16, 30, 31];

#[test]
fn image_dxt_pack_565comp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let dst = e.alloc(4);

    let mut check = |addr: u64, r: u32, g: u32, b: u32, ours: u32, is16: bool| {
        e.write_u32(dst, 0xAAAA_AAAA);
        e.call(addr, dst as u32, 0, &[r, g, b]);
        let bin = if is16 {
            u32::from(rdu16(&e, dst))
        } else {
            e.read_u32(dst)
        };
        assert_eq!(bin, ours, "pack {addr:#x} r={r} g={g} b={b}");
    };

    for &r in &RS {
        for &g in &GS {
            for &b in &BS {
                check(PK_8888, r, g, b, pack_565comp_argb8888(r, g, b), false);
                check(
                    PK_4444,
                    r,
                    g,
                    b,
                    u32::from(pack_565comp_argb4444(r, g, b)),
                    true,
                );
                check(
                    PK_1555,
                    r,
                    g,
                    b,
                    u32::from(pack_565comp_argb1555(r, g, b)),
                    true,
                );
            }
        }
    }

    // Fuzz full byte ranges (the packers read byte args; the | 0xffe0 / >>1 masking is exercised).
    let mut s = 0x6590_0001u32;
    for _ in 0..400 {
        let r = (next(&mut s) >> 24) & 0xff;
        let g = (next(&mut s) >> 24) & 0xff;
        let b = (next(&mut s) >> 24) & 0xff;
        check(PK_8888, r, g, b, pack_565comp_argb8888(r, g, b), false);
        check(
            PK_4444,
            r,
            g,
            b,
            u32::from(pack_565comp_argb4444(r, g, b)),
            true,
        );
        check(
            PK_1555,
            r,
            g,
            b,
            u32::from(pack_565comp_argb1555(r, g, b)),
            true,
        );
    }
}

#[test]
fn image_dxt_pack_word() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let wslot = e.alloc(2);
    let dst = e.alloc(4);

    let mut check = |addr: u64, word: u16, ours: u32, is16: bool| {
        e.write(wslot, &word.to_le_bytes());
        e.write_u32(dst, 0xAAAA_AAAA);
        e.call(addr, dst as u32, 0, &[wslot as u32]);
        let bin = if is16 {
            u32::from(rdu16(&e, dst))
        } else {
            e.read_u32(dst)
        };
        assert_eq!(bin, ours, "packword {addr:#x} word={word:#06x}");
    };

    let words = [
        0x0000u16, 0xFFFF, 0x8000, 0x0001, 0xF800, 0x07E0, 0x001F, 0x1234, 0xABCD, 0x7BEF, 0x8410,
    ];
    for &w in &words {
        check(PK_WORD_1555, w, u32::from(pack_word_argb1555(w)), true);
        check(PK_WORD_8888, w, pack_word_argb8888(w), false);
    }
    let mut s = 0x6ea0_0001u32;
    for _ in 0..400 {
        let w = (next(&mut s) >> 8) as u16;
        check(PK_WORD_1555, w, u32::from(pack_word_argb1555(w)), true);
        check(PK_WORD_8888, w, pack_word_argb8888(w), false);
    }
}

// ============================================================================================
// DXT1 4-entry palette builders (0x5a7200 / 73b0 / 7640) — need the LUT
// ============================================================================================

/// Endpoint pairs exercising both punch-through modes (`c0>c1` 4-color, `c0<=c1` 3-color, `c0==c1`).
const ENDPOINTS: [(u16, u16); 14] = [
    (0xFFFF, 0x0000),
    (0x0000, 0xFFFF),
    (0x8000, 0x8000),
    (0xF800, 0x07E0),
    (0x07E0, 0xF800),
    (0x001F, 0x8410),
    (0x8410, 0x001F),
    (0xFFFF, 0xFFFE),
    (0xFFFE, 0xFFFF),
    (0x1234, 0xABCD),
    (0xABCD, 0x1234),
    (0x7BEF, 0x7BEF),
    (0x0001, 0x0000),
    (0x0000, 0x0001),
];

#[test]
fn image_dxt1_palette_builders() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = fresh(&wow);
    let blk = e.alloc(4);
    let pal = e.alloc(16);

    let mut probe = |c0: u16, c1: u16| {
        let mut b = [0u8; 4];
        b[0..2].copy_from_slice(&c0.to_le_bytes());
        b[2..4].copy_from_slice(&c1.to_le_bytes());
        e.write(blk, &b);

        // 565
        e.write(pal, &[0xAAu8; 16]);
        e.call(0x005a_7200, blk as u32, pal as u32, &[]);
        let bin: Vec<u16> = (0..4).map(|i| rdu16(&e, pal + i * 2)).collect();
        assert_eq!(
            bin[..],
            dxt1_pal_rgb565(c0, c1)[..],
            "pal565 {c0:#06x},{c1:#06x}"
        );

        // 1555
        e.write(pal, &[0xAAu8; 16]);
        e.call(0x005a_73b0, blk as u32, pal as u32, &[]);
        let bin: Vec<u16> = (0..4).map(|i| rdu16(&e, pal + i * 2)).collect();
        assert_eq!(
            bin[..],
            dxt1_pal_argb1555(c0, c1)[..],
            "pal1555 {c0:#06x},{c1:#06x}"
        );

        // 8888
        e.write(pal, &[0xAAu8; 16]);
        e.call(0x005a_7640, blk as u32, pal as u32, &[]);
        let bin: Vec<u32> = (0..4).map(|i| e.read_u32(pal + i * 4)).collect();
        assert_eq!(
            bin[..],
            dxt1_pal_argb8888(c0, c1)[..],
            "pal8888 {c0:#06x},{c1:#06x}"
        );
    };

    for &(c0, c1) in &ENDPOINTS {
        probe(c0, c1);
    }
    let mut s = 0x7200_0001u32;
    for _ in 0..300 {
        probe((next(&mut s) >> 8) as u16, (next(&mut s) >> 8) as u16);
    }
}

// ============================================================================================
// Per-block decoders (0x5a6990 / 6be0 / 6ef0 DXT1; 0x5a62f0 / 65c0 DXT3) — need LUT + atexit stub
// ============================================================================================

/// Bounds probe set: full 4×4, partial-block clamps, and a couple with nonzero x0/y0 (exercises the
/// index pre-shift the binary supports, even though the drivers always pass x0=y0=0).
const BOUNDSES: [Bounds; 9] = [
    Bounds {
        x0: 0,
        y0: 0,
        x1: 3,
        y1: 3,
        adv: 4,
    },
    Bounds {
        x0: 0,
        y0: 0,
        x1: 0,
        y1: 0,
        adv: 1,
    },
    Bounds {
        x0: 0,
        y0: 0,
        x1: 1,
        y1: 1,
        adv: 2,
    },
    Bounds {
        x0: 0,
        y0: 0,
        x1: 2,
        y1: 3,
        adv: 3,
    },
    Bounds {
        x0: 0,
        y0: 0,
        x1: 3,
        y1: 1,
        adv: 4,
    },
    Bounds {
        x0: 0,
        y0: 0,
        x1: 1,
        y1: 3,
        adv: 2,
    },
    Bounds {
        x0: 1,
        y0: 0,
        x1: 3,
        y1: 3,
        adv: 4,
    },
    Bounds {
        x0: 0,
        y0: 1,
        x1: 3,
        y1: 2,
        adv: 4,
    },
    Bounds {
        x0: 1,
        y0: 1,
        x1: 3,
        y1: 3,
        adv: 4,
    },
];

fn dxt1_block(c0: u16, c1: u16, idx: [u8; 4]) -> [u8; 8] {
    let mut b = [0u8; 8];
    b[0..2].copy_from_slice(&c0.to_le_bytes());
    b[2..4].copy_from_slice(&c1.to_le_bytes());
    b[4..8].copy_from_slice(&idx);
    b
}

fn dxt3_block(alpha: [u16; 4], c0: u16, c1: u16, idx: [u8; 4]) -> [u8; 16] {
    let mut b = [0u8; 16];
    for (i, a) in alpha.iter().enumerate() {
        b[i * 2..i * 2 + 2].copy_from_slice(&a.to_le_bytes());
    }
    b[8..10].copy_from_slice(&c0.to_le_bytes());
    b[10..12].copy_from_slice(&c1.to_le_bytes());
    b[12..16].copy_from_slice(&idx);
    b
}

/// Our DXT3 block-decoder transcription signature (block, dst, row offsets, bounds, alpha callback).
type Dxt3BlockFn = fn(&[u8], &mut [u8], &mut [usize], Bounds, fn(u32) -> u32);

/// DXT1 block-decoder check. `ours` decodes into a Rust `dst`; `addr` is the binary fn (no callback).
#[allow(clippy::too_many_arguments)]
fn check_dxt1_block(
    e: &mut Emu,
    addr: u64,
    ours: fn(&[u8], &mut [u8], &mut [usize], Bounds),
    block: &[u8; 8],
    b: Bounds,
    pixsize: usize,
    label: &str,
) {
    let row_stride = 4 * pixsize + 8;
    let dst_bytes = 4 * row_stride;
    let blk = e.alloc(8);
    e.write(blk, block);
    let dst = e.alloc(dst_bytes as u64);
    let sentinel = vec![0xAAu8; dst_bytes];
    e.write(dst, &sentinel);
    let rp = e.alloc(16);
    for k in 0..4 {
        e.write_u32(rp + k as u64 * 4, (dst as usize + k * row_stride) as u32);
    }
    let bnd = e.alloc(24);
    for (i, v) in [b.x0, b.y0, b.x1, b.y1, b.adv, b.adv].iter().enumerate() {
        e.write_u32(bnd + i as u64 * 4, *v);
    }
    e.call(addr, blk as u32, rp as u32, &[bnd as u32]);
    let bin = e.read(dst, dst_bytes);

    let mut od = sentinel.clone();
    let mut ro = [0usize, row_stride, 2 * row_stride, 3 * row_stride];
    ours(block, &mut od, &mut ro, b);
    assert_eq!(bin, od, "{label} {addr:#x}");
}

/// DXT3 block-decoder check (callback-driven alpha).
#[allow(clippy::too_many_arguments)]
fn check_dxt3_block(
    e: &mut Emu,
    addr: u64,
    cb: u64,
    ours: Dxt3BlockFn,
    alpha_fn: fn(u32) -> u32,
    block: &[u8; 16],
    b: Bounds,
    pixsize: usize,
    label: &str,
) {
    let row_stride = 4 * pixsize + 8;
    let dst_bytes = 4 * row_stride;
    let blk = e.alloc(16);
    e.write(blk, block);
    let dst = e.alloc(dst_bytes as u64);
    let sentinel = vec![0xAAu8; dst_bytes];
    e.write(dst, &sentinel);
    let rp = e.alloc(16);
    for k in 0..4 {
        e.write_u32(rp + k as u64 * 4, (dst as usize + k * row_stride) as u32);
    }
    let bnd = e.alloc(24);
    for (i, v) in [b.x0, b.y0, b.x1, b.y1, b.adv, b.adv].iter().enumerate() {
        e.write_u32(bnd + i as u64 * 4, *v);
    }
    e.call(addr, blk as u32, rp as u32, &[bnd as u32, cb as u32]);
    let bin = e.read(dst, dst_bytes);

    let mut od = sentinel.clone();
    let mut ro = [0usize, row_stride, 2 * row_stride, 3 * row_stride];
    ours(block, &mut od, &mut ro, b, alpha_fn);
    assert_eq!(bin, od, "{label} {addr:#x}");
}

#[test]
fn image_dxt1_block_decoders() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = fresh(&wow);

    // Deterministic punch-through probes: 4-color (c0>c1) and 3-color (c0<=c1), indices 0..3.
    let blocks = [
        dxt1_block(0xFFFF, 0x0000, [0xE4, 0xE4, 0xE4, 0xE4]), // 4-color, idx 0,1,2,3
        dxt1_block(0x0000, 0xFFFF, [0xE4, 0x1B, 0x4E, 0xB1]), // 3-color (idx3=transparent)
        dxt1_block(0x8000, 0x8000, [0x1B, 0x1B, 0x1B, 0x1B]), // equal -> 3-color
        dxt1_block(0xF800, 0x07E0, [0x39, 0x6C, 0x93, 0xC6]), // red>green -> 4-color
        dxt1_block(0x07E0, 0xF800, [0x12, 0x34, 0x56, 0x78]), // green<red endpoints -> 3-color
    ];
    for blk in &blocks {
        for &b in &BOUNDSES {
            check_dxt1_block(
                &mut e,
                0x005a_6990,
                dxt1_block_rgb565,
                blk,
                b,
                2,
                "dxt1_565",
            );
            check_dxt1_block(
                &mut e,
                0x005a_6be0,
                dxt1_block_argb1555,
                blk,
                b,
                2,
                "dxt1_1555",
            );
            check_dxt1_block(
                &mut e,
                0x005a_6ef0,
                dxt1_block_argb8888,
                blk,
                b,
                4,
                "dxt1_8888",
            );
        }
    }

    // Fuzz.
    let mut s = 0x6990_0001u32;
    for _ in 0..120 {
        let blk = dxt1_block(
            (next(&mut s) >> 8) as u16,
            (next(&mut s) >> 8) as u16,
            [
                (next(&mut s) >> 16) as u8,
                (next(&mut s) >> 16) as u8,
                (next(&mut s) >> 16) as u8,
                (next(&mut s) >> 16) as u8,
            ],
        );
        let b = BOUNDSES[(next(&mut s) >> 8) as usize % BOUNDSES.len()];
        check_dxt1_block(
            &mut e,
            0x005a_6990,
            dxt1_block_rgb565,
            &blk,
            b,
            2,
            "dxt1_565_fz",
        );
        check_dxt1_block(
            &mut e,
            0x005a_6be0,
            dxt1_block_argb1555,
            &blk,
            b,
            2,
            "dxt1_1555_fz",
        );
        check_dxt1_block(
            &mut e,
            0x005a_6ef0,
            dxt1_block_argb8888,
            &blk,
            b,
            4,
            "dxt1_8888_fz",
        );
    }
}

#[test]
fn image_dxt3_block_decoders() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = fresh(&wow);

    // Alpha words covering nibbles 0..15; colour endpoints + indices varied.
    let blocks = [
        dxt3_block(
            [0x3210, 0x7654, 0xBA98, 0xFEDC],
            0xFFFF,
            0x0000,
            [0xE4, 0xE4, 0xE4, 0xE4],
        ),
        dxt3_block(
            [0x0000, 0xFFFF, 0x1248, 0x8421],
            0x07E0,
            0xF800,
            [0x1B, 0x4E, 0xB1, 0xE4],
        ),
        dxt3_block(
            [0xABCD, 0x1234, 0x5678, 0x9ABC],
            0xF800,
            0x001F,
            [0x39, 0x6C, 0x93, 0xC6],
        ),
    ];
    for blk in &blocks {
        for &b in &BOUNDSES {
            check_dxt3_block(
                &mut e,
                0x005a_62f0,
                CB_NIBBLE_SHL4,
                dxt3_block_argb8888,
                alpha_nibble_shl4,
                blk,
                b,
                4,
                "dxt3_8888",
            );
            check_dxt3_block(
                &mut e,
                0x005a_65c0,
                CB_IDENTITY,
                dxt3_block_argb4444,
                alpha_identity,
                blk,
                b,
                2,
                "dxt3_4444",
            );
        }
    }

    let mut s = 0x62f0_0001u32;
    for _ in 0..120 {
        let mut raw = [0u8; 16];
        for byte in raw.iter_mut() {
            *byte = (next(&mut s) >> 16) as u8;
        }
        let b = BOUNDSES[(next(&mut s) >> 8) as usize % BOUNDSES.len()];
        check_dxt3_block(
            &mut e,
            0x005a_62f0,
            CB_NIBBLE_SHL4,
            dxt3_block_argb8888,
            alpha_nibble_shl4,
            &raw,
            b,
            4,
            "dxt3_8888_fz",
        );
        check_dxt3_block(
            &mut e,
            0x005a_65c0,
            CB_IDENTITY,
            dxt3_block_argb4444,
            alpha_identity,
            &raw,
            b,
            2,
            "dxt3_4444_fz",
        );
    }
}

// ============================================================================================
// Block-grid drivers (0x5a57c0 / 5a20 / 5c80 DXT1; 0x5a5ef0 / 60c0 DXT3) — need LUT + atexit stub
// ============================================================================================

type Driver = fn(u32, u32, &[u8], usize, &mut [u8], usize);

#[allow(clippy::too_many_arguments)]
fn check_driver(
    e: &mut Emu,
    addr: u64,
    ours: Driver,
    w: u32,
    h: u32,
    block_size: usize,
    pixsize: usize,
    src_pad: usize,
    dst_pad: usize,
    s: &mut u32,
) {
    let nbx = w.div_ceil(4) as usize;
    let nby = h.div_ceil(4) as usize;
    let src_stride = nbx * block_size + src_pad;
    let dst_stride = nbx * 4 * pixsize + dst_pad;
    let src_bytes = (nby * src_stride).max(block_size);
    let dst_bytes = (nby * 4 * dst_stride).max(pixsize);

    let dims = e.alloc(8);
    e.write_u32(dims, w);
    e.write_u32(dims + 4, h);
    let src_va = e.alloc(src_bytes as u64);
    let dst_va = e.alloc(dst_bytes as u64);

    let mut src = vec![0u8; src_bytes];
    for byte in src.iter_mut() {
        *byte = (next(s) >> 16) as u8;
    }
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

    let mut od = sentinel.clone();
    ours(w, h, &src, src_stride, &mut od, dst_stride);
    assert_eq!(bin, od, "driver {addr:#x} {w}x{h} pad {src_pad}/{dst_pad}");
}

/// Aligned (full path) + small/misaligned (edge path) surface dims.
const SURF_DIMS: [(u32, u32); 20] = [
    (4, 4),
    (8, 8),
    (8, 4),
    (4, 8),
    (16, 16),
    (12, 8),
    (1, 1),
    (2, 2),
    (3, 3),
    (1, 4),
    (4, 1),
    (2, 4),
    (3, 1),
    (1, 2),
    (6, 4),
    (4, 6),
    (6, 6),
    (5, 8),
    (8, 5),
    (12, 6),
];
const DRV_PADS: [(usize, usize); 3] = [(0, 0), (0, 16), (16, 32)];

#[test]
fn image_dxt1_drivers() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = fresh(&wow);
    let mut s = 0x57c0_0001u32;
    let drivers: [(u64, Driver, usize); 3] = [
        (0x005a_57c0, dxt1_decode_rgb565, 2),
        (0x005a_5a20, dxt1_decode_argb1555, 2),
        (0x005a_5c80, dxt1_decode_argb8888, 4),
    ];
    for (addr, ours, pixsize) in drivers {
        for &(w, h) in &SURF_DIMS {
            for &(sp, dp) in &DRV_PADS {
                check_driver(&mut e, addr, ours, w, h, 8, pixsize, sp, dp, &mut s);
            }
        }
    }
}

#[test]
fn image_dxt3_drivers() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = fresh(&wow);
    let mut s = 0x5ef0_0001u32;
    let drivers: [(u64, Driver, usize); 2] = [
        (0x005a_5ef0, dxt3_decode_argb4444, 2),
        (0x005a_60c0, dxt3_decode_argb8888, 4),
    ];
    for (addr, ours, pixsize) in drivers {
        for &(w, h) in &SURF_DIMS {
            for &(sp, dp) in &DRV_PADS {
                check_driver(&mut e, addr, ours, w, h, 16, pixsize, sp, dp, &mut s);
            }
        }
    }
}
