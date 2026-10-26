//! Bit-exact tests of the FreeType arithmetic the font layer reaches, each the exe's own function
//! run whole in the emulator beside `benilla_re_font::freetype`:
//!
//! - **0x7cd0e0 `FT_MulFix`**: both paths, their thresholds `|a| = 0x800` and `|b| = 0x100000` and a
//!   step either side, both signs, `i32::MIN`, `b = 0x10000`, `a = 0`, and fuzz.
//! - **0x7d0340 `FT_Vector_Transform`**: a vector and a matrix in scratch, the identity, a swap, a
//!   rotation-like matrix and fuzz.
//! - **0x7d8e00, the encoding lookup**: every platform 0 to 4 against encodings 0 to 11 and
//!   `u32::MAX`.
//! - **0x7ce830 `FT_Get_Kerning`**: a face in scratch whose driver's kerning slot is a stub that
//!   writes a raw vector; modes 0, 1, 2 and 3, both signs, around the rounding edge.
//! - **0x7dabe0 `Set_High_Precision`**: both settings, the seven fields.
//! - **0x7dac90 `Vertical_Sweep_Span`**: a raster in scratch at both precisions, crossings around
//!   the jitter edge, spans off either end of the row, within one byte and across bytes.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_font::freetype::{
    encoding_tag, kerning, mul_fix, set_high_precision, vector_transform, vertical_span, Precision,
};
use common::next;

const FT_MUL_FIX: u64 = 0x7cd0e0;
const FT_VECTOR_TRANSFORM: u64 = 0x7d0340;
const ENCODING_TAG: u64 = 0x7d8e00;
const FT_GET_KERNING: u64 = 0x7ce830;
const SET_HIGH_PRECISION: u64 = 0x7dabe0;
const VERTICAL_SWEEP_SPAN: u64 = 0x7dac90;

#[test]
fn mul_fix_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let edges: &[i32] = &[
        0,
        1,
        -1,
        0x7ff,
        0x800,
        0x801,
        -0x800,
        -0x801,
        0xffff,
        0x10000,
        0x10001,
        0xf_ffff,
        0x10_0000,
        0x10_0001,
        -0x10_0000,
        -0x10_0001,
        0x7fff_ffff,
        i32::MIN,
        i32::MIN + 1,
        12345,
        -98765,
        24576,
        0x8000,
    ];
    let check = |e: &mut Emu, a: i32, b: i32| {
        let bin = e.call(FT_MUL_FIX, a as u32, b as u32, &[]) as i32;
        assert_eq!(bin, mul_fix(a, b), "FT_MulFix({a:#x}, {b:#x})");
    };
    for &a in edges {
        for &b in edges {
            check(&mut e, a, b);
        }
    }
    let mut s = 0x7cd0_e0e0u32;
    for _ in 0..4000 {
        let a = next(&mut s) as i32 >> (next(&mut s) >> 27);
        let b = next(&mut s) as i32 >> (next(&mut s) >> 27);
        check(&mut e, a, b);
    }
}

#[test]
fn vector_transform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let v = e.alloc(8);
    let m = e.alloc(16);
    let check = |e: &mut Emu, vec: (i32, i32), mat: [i32; 4]| {
        e.write_u32(v, vec.0 as u32);
        e.write_u32(v + 4, vec.1 as u32);
        for (i, &x) in mat.iter().enumerate() {
            e.write_u32(m + 4 * i as u64, x as u32);
        }
        e.call(FT_VECTOR_TRANSFORM, v as u32, m as u32, &[]);
        let bin = (e.read_u32(v) as i32, e.read_u32(v + 4) as i32);
        assert_eq!(bin, vector_transform(vec, mat), "{vec:?} by {mat:?}");
    };
    let mats: &[[i32; 4]] = &[
        [0x10000, 0, 0, 0x10000],
        [0, 0x10000, 0x10000, 0],
        [0xb505, -0xb505, 0xb505, 0xb505],
        [0x8000, 0x4000, -0x4000, 0x18000],
        [-0x10000, 0, 0, -0x10000],
    ];
    let vecs: &[(i32, i32)] = &[(0, 0), (64, 0), (0, 64), (-640, 1280), (0x900, -0x900)];
    for &mat in mats {
        for &vec in vecs {
            check(&mut e, vec, mat);
        }
    }
    let mut s = 0x7d03_4034u32;
    for _ in 0..1000 {
        let f = |s: &mut u32| (next(s) as i32) >> (8 + (next(s) >> 29));
        let vec = (f(&mut s), f(&mut s));
        let mat = [f(&mut s), f(&mut s), f(&mut s), f(&mut s)];
        check(&mut e, vec, mat);
    }
}

#[test]
fn encoding_tag_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for platform in 0..=4u32 {
        for encoding in (0..=11u32).chain([0xffff, u32::MAX]) {
            let bin = e.call(ENCODING_TAG, platform, encoding, &[]);
            assert_eq!(
                bin,
                encoding_tag(platform, encoding),
                "platform {platform}, encoding {encoding}"
            );
        }
    }
}

#[test]
fn get_kerning_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // face +0x58 size, +0x60 driver; driver +0x14 class; class +0x58 get_kerning, here a stub that
    // writes the raw vector through its second stack argument and returns 0 (`ret 8`).
    let face = e.alloc(0x80);
    let size = e.alloc(0x40);
    let driver = e.alloc(0x40);
    let class = e.alloc(0x80);
    let stub = e.alloc(0x10);
    let raw = e.alloc(8);
    let out = e.alloc(8);
    e.write_u32(face + 0x58, size as u32);
    e.write_u32(face + 0x60, driver as u32);
    e.write_u32(driver + 0x14, class as u32);
    e.write_u32(class + 0x58, stub as u32);
    e.stub_write_through(stub, 1, raw, 8, 0, 2);
    let scales: &[(i32, i32)] = &[(0x10000, 0x10000), (24576, 24576), (0x6000, 0x9000)];
    let raws: &[(i32, i32)] = &[
        (0, 0),
        (-150, 0),
        (150, -150),
        (85, 86),
        (-85, -86),
        (2049, -3000),
    ];
    for &(xs, ys) in scales {
        e.write_u32(size + 0x10, xs as u32);
        e.write_u32(size + 0x14, ys as u32);
        for &r in raws {
            e.write_u32(raw, r.0 as u32);
            e.write_u32(raw + 4, r.1 as u32);
            for mode in 0..=3u32 {
                e.write_u32(out, 0xdead_beef);
                e.write_u32(out + 4, 0xdead_beef);
                let err = e.call(FT_GET_KERNING, face as u32, 7, &[9, mode, out as u32]);
                assert_eq!(err, 0);
                let bin = (e.read_u32(out) as i32, e.read_u32(out + 4) as i32);
                assert_eq!(bin, kerning(r, mode, xs, ys), "raw {r:?} mode {mode}");
            }
        }
    }
}

fn read_precision(e: &Emu, ras: u64) -> Precision {
    let f = |o: u64| e.read_u32(ras + o) as i32;
    Precision {
        bits: f(0),
        precision: f(4),
        half: f(8),
        mask: f(0xc),
        shift: f(0x10),
        step: f(0x14),
        jitter: f(0x18),
    }
}

#[test]
fn set_high_precision_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ras = e.alloc(0x40);
    for (edx, high) in [(0u32, false), (0x100, true), (1, true)] {
        e.call(SET_HIGH_PRECISION, ras as u32, edx, &[]);
        assert_eq!(
            read_precision(&e, ras),
            set_high_precision(high),
            "edx {edx:#x}"
        );
    }
}

#[test]
fn vertical_span_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // The raster's fields the span reads and writes: +0x3c width (u16), +0x40 target, +0x98 the
    // row's offset, +0xa2 and +0xa4 the byte bounds (i16).
    let ras = e.alloc(0x100);
    const ROW: u64 = 0x40;
    let target = e.alloc(0x200);
    let base = target + ROW;
    let mut s = 0x7dac_9090u32;
    for high in [false, true] {
        e.call(SET_HIGH_PRECISION, ras as u32, u32::from(high) << 8, &[]);
        let p = read_precision(&e, ras);
        let one = p.precision;
        let mut cases: Vec<(i32, i32, u16)> = Vec::new();
        for &w in &[1u16, 7, 8, 9, 37] {
            for x1 in [
                -2 * one,
                -one - 1,
                -1,
                0,
                1,
                one / 2,
                one - 1,
                one,
                one + 1,
                5 * one + 3,
            ] {
                for d in [
                    0,
                    1,
                    p.jitter,
                    p.jitter + 1,
                    one,
                    one + p.jitter,
                    one + p.jitter + 1,
                ] {
                    for extra in [0, 2 * one, 9 * one, 40 * one] {
                        cases.push((x1, x1 + d + extra, w));
                    }
                }
            }
        }
        for _ in 0..2000 {
            let w = 1 + (next(&mut s) >> 26) as u16;
            let x1 = ((next(&mut s) >> 8) as i32 % (i32::from(w) + 4) - 2) * one
                + (next(&mut s) >> 8) as i32 % one;
            let x2 = x1 + (next(&mut s) >> 8) as i32 % (3 * one * i32::from(w));
            cases.push((x1, x2, w));
        }
        for (x1, x2, w) in cases {
            let (min_x, max_x) = (
                ((next(&mut s) >> 24) as i16) - 64,
                ((next(&mut s) >> 24) as i16) - 64,
            );
            let canvas: Vec<u8> = (0..0x200)
                .map(|_| (next(&mut s) >> 24) as u8 & 0x11)
                .collect();
            e.write(target, &canvas);
            e.write(ras + 0x3c, &w.to_le_bytes());
            e.write_u32(ras + 0x40, base as u32);
            e.write_u32(ras + 0x98, 0);
            e.write(ras + 0xa2, &min_x.to_le_bytes());
            e.write(ras + 0xa4, &max_x.to_le_bytes());
            e.call(
                VERTICAL_SWEEP_SPAN,
                ras as u32,
                0,
                &[x1 as u32, x2 as u32, 0, 0],
            );
            let bin = e.read(target, 0x200);
            let mut want = canvas.clone();
            let (mut wmin, mut wmax) = (min_x, max_x);
            if let Some(span) = vertical_span(&p, x1, x2, w, min_x, max_x) {
                for (i, b) in span.bytes.iter().enumerate() {
                    want[(ROW as i32 + span.first) as usize + i] |= b;
                }
                (wmin, wmax) = (span.min_x, span.max_x);
            }
            let ctx = format!("high {high} x1 {x1} x2 {x2} width {w}");
            assert_eq!(bin, want, "{ctx}: row");
            let rd16 = |o: u64| i16::from_le_bytes(e.read(ras + o, 2).try_into().unwrap());
            assert_eq!((rd16(0xa2), rd16(0xa4)), (wmin, wmax), "{ctx}: bounds");
        }
    }
}
