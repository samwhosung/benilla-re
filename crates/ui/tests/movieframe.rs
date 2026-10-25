//! Differential tests of the arithmetic of the movie and animated-texture functions `0x78c360`,
//! `0x78cf30` and `0x78d1a0`, each run against `WoW.exe` in the emulator.
//!
//! Coverage of the arithmetic opcode set (load/store/compare/convert/`fild`/`fxch` excluded),
//! counted in each `[addr, true_ret)`:
//!
//! * `0x78c360` [.,0x78c795 ret 4]: `fidiv` ×1 @0x78c5ba
//! * `0x78cf30` [.,0x78d19f ret]  : `fmul` ×4 (0x78cf9f/0x78cfa5/0x78cfdb/0x78cfe1),
//!   `fadd` ×2 (0x78cfab/0x78cfe7)  (two byte-identical blocks)
//! * `0x78d1a0` [.,0x78d53b ret]  : `fmul` ×2 (0x78d480 [field+0xc], 0x78d4ae [field+0x8])
//!
//! Diff strategy per op:
//! * `0x78c360` — own-address **slice** `0x78c5ae..0x78c5c9` over `fild qword; fidiv dword; fstp
//!   dword [eax+0x33c]`, reading back the stored f32. (1 fidiv.)
//! * `0x78cf30` — own-address **slices**, ONE PER BLOCK (`0x78cf9c..0x78cfb6` and
//!   `0x78cfd8..0x78cff2`), each running `fild qword; fmul; fmul; fadd; call __ftol` and reading EAX.
//!   The constants `[0x801360]`/`[0x7ffa24]` are read live from the mapped image; the slices cover
//!   ALL 4 fmul + 2 fadd. (Byte-verified the two blocks are identical FP.)
//! * `0x78d1a0` — a **copied-byte island** per field offset: `fld dword [scale]; <source fmul bytes>;
//!   fstp dword [out]`, the source `fmul [eax+0xc]` (`d8 48 0c`) / `fmul [eax+0x8]` (`d8 48 08`)
//!   byte-verified == 0x78d480 / 0x78d4ae respectively (else circular). eax points at a field cell.
//!
//! * `0x78c360`, whole — `movie_avi_read_whole`, below: the RIFF walk, the `hdrl` walk and both
//!   passes over `idx1`, over AVI files the test builds, against `read_movie_avi`.
//!
//! Fuzz reads the LCG's HIGH bits. Mandatory deterministic
//! `==`-boundary probes sit on the `__ftol` truncate pivots and exact divide/multiply boundaries.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::movieframe::{
    aspect_ratio, quad_scale, time_to_index, HALF_F32_BITS, MS_TO_S_F32_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0xc0; // room for [ebp-0xc0 .. ebp]
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { ebp, esp }
}

// ===========================================================================================
// Row 0x78c360 aspect_ratio — slice 0x78c5ae..0x78c5c9:
//   fild qword [ebp-0x38]; mov [ebp-0x40],edx; mov [ebp-0x3c],eax; mov eax,[ebp-0x10];
//   fidiv dword [ebp-0x40]; mov edx,[ebp-0x18]; mov [ebp-0x30],edx; fstp dword [eax+0x33c].
// fidiv @0x78c5ba (×1). Feed the u32 numerator (qword: low=num, high=0) at [ebp-0x38], the i32
// divisor at [ebp-0x40], the store base at [ebp-0x10], and read the f32 at [base+0x33c].
// ===========================================================================================
#[test]
fn glue_geom_78c360_aspect_ratio() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let base = emu.alloc(0x400); // the `this` store base; [base+0x33c] holds the result

    let run = |emu: &mut Emu, num: u32, den: i32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        // The slice's `mov [ebp-0x40], edx` (0x78c5b1) writes the divisor from EDX (loaded just
        // before the slice from [esi+ebx+0x14] in the full fn) — set EDX, not the slot directly.
        emu.set_reg(RegisterX86::EDX, den as u32 as u64); // i32 divisor (-> [ebp-0x40])
        emu.write_u32(f.ebp - 0x38, num); // low dword of the qword numerator (set before slice)
        emu.write_u32(f.ebp - 0x34, 0); // high dword forced 0 (the eax==0 path; set before slice)
        emu.write_u32(f.ebp - 0x10, base as u32); // store base for fstp [eax+0x33c]
        emu.write_u32(f.ebp - 0x18, 0); // scratch (mov [ebp-0x30],[ebp-0x18])
        emu.run(0x78c5ae, 0x78c5c9);
        emu.read_f32(base + 0x33c)
    };

    let mut total = 0u32;
    // deterministic boundaries: exact-divide cases, equal num/den (==1.0), 0 numerator, the i32 sign.
    let probes: [(u32, i32); 14] = [
        (0, 1),
        (1, 1),
        (2, 1),
        (1, 2),
        (1, 3),      // 0.333… — exact-ratio rounding pivot
        (4, 3),      // 1.333… (the typical 4:3 aspect)
        (16, 9),     // 1.777…
        (1024, 768), // a real pixel ratio
        (100, 7),    // 14.285…
        (0xffffffff, 1),
        (0xffffffff, 0xffff), // large num, large den
        (1, -1),              // negative divisor
        (7, -3),              // negative non-exact
        (0x80000000, 3),
    ];
    for &(num, den) in &probes {
        assert_eq!(
            run(&mut emu, num, den).to_bits(),
            aspect_ratio(num, den).to_bits(),
            "aspect_ratio num={num} den={den}"
        );
        total += 1;
    }
    // dense small grid
    for num in 0u32..=64 {
        for den in 1i32..=64 {
            assert_eq!(
                run(&mut emu, num, den).to_bits(),
                aspect_ratio(num, den).to_bits(),
                "aspect_ratio grid num={num} den={den}"
            );
            total += 1;
        }
    }
    // fuzz the u32 numerator x i32 divisor (high LCG bits); avoid den==0 (masked-#Z inf is exact
    // either way, but keep the diff over finite results dense; one explicit den==0 probe below).
    let mut s = 0x78c3_6000u32;
    for _ in 0..6000 {
        let num = next(&mut s);
        let mut den = next(&mut s) as i32;
        if den == 0 {
            den = 1;
        }
        assert_eq!(
            run(&mut emu, num, den).to_bits(),
            aspect_ratio(num, den).to_bits(),
            "aspect_ratio fuzz num={num} den={den}"
        );
        total += 1;
    }
    // explicit den==0: masked-#Z. fild qword(num) / 0 -> +inf for num>0, NaN for num==0 (0/0).
    for &num in &[0u32, 1, 1000, 0xffffffff] {
        assert_eq!(
            run(&mut emu, num, 0).to_bits(),
            aspect_ratio(num, 0).to_bits(),
            "aspect_ratio den=0 num={num}"
        );
        total += 1;
    }
    eprintln!(
        "glue_geom_78c360_aspect_ratio: {total} bit-exact (fild qword; fidiv dword; fstp dword)"
    );
}

// ===========================================================================================
// Row 0x78cf30 time_to_index — two byte-identical blocks, each:
//   fild qword [ebp-0x8]; fmul dword [esi+0x33c]; fmul dword [0x801360]=0.001; fadd dword
//   [0x7ffa24]=0.5; call __ftol.  Slice block1 = 0x78cf9c..0x78cfb6, block2 = 0x78cfd8..0x78cff2.
// fmul @0x78cf9f/0x78cfa5/0x78cfdb/0x78cfe1, fadd @0x78cfab/0x78cfe7 (the two blocks). Feed the u32
// time at [ebp-0x8] (qword, hi dword 0), the rate f32 at [esi+0x33c]; read the __ftol result in EAX.
// ===========================================================================================
#[test]
fn glue_geom_78cf30_time_to_index() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let esi = emu.alloc(0x400); // `this`; [esi+0x33c] holds the rate

    // confirm our transcribed constants match the live image bytes the slice reads.
    assert_eq!(
        u32::from_le_bytes(emu.read(0x801360, 4).try_into().unwrap()),
        MS_TO_S_F32_BITS,
        "MS_TO_S_F32 @ 0x801360"
    );
    assert_eq!(
        u32::from_le_bytes(emu.read(0x7ffa24, 4).try_into().unwrap()),
        HALF_F32_BITS,
        "HALF_F32 @ 0x7ffa24"
    );
    // byte-verify the two FP blocks are identical (the FP run, fild..fadd = 0x78cf9c..0x78cfb1).
    assert_eq!(
        emu.read(0x78cf9c, 0x78cfb1 - 0x78cf9c),
        emu.read(0x78cfd8, 0x78cfb1 - 0x78cf9c),
        "0x78cf30 block1 != block2 FP bytes"
    );

    let blocks = [(0x78cf9cu64, 0x78cfb6u64), (0x78cfd8, 0x78cff2)];
    let run = |emu: &mut Emu, start: u64, end: u64, t: u32, rate: f32| -> i32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(f.ebp - 0x8, t); // low dword of the qword time value
        emu.write_u32(f.ebp - 0x4, 0); // high dword forced 0
        emu.write_u32(esi + 0x33c, rate.to_bits());
        emu.run(start, end);
        emu.get_reg(RegisterX86::EAX) as u32 as i32
    };

    let mut total = 0u32;
    // deterministic: __ftol pivots and the exact-product boundaries. rate is the runtime f32 field.
    let rates = [
        0.0f32,
        1.0,
        0.5,
        30.0, // 30 fps-ish rate
        60.0,
        23.976,
        1.0 / 3.0,
        1000.0, // ·0.001 = 1.0
        0.001,
        12345.678,
    ];
    let times = [
        0u32, 1, 2, 499, 500, 501, 999, 1000, 1001, 1500, 33, 16, 0xffff, 0x10000, 1_000_000,
        0x7fffffff, 0x80000000, 0xffffffff,
    ];
    for &(start, end) in &blocks {
        for &rate in &rates {
            for &t in &times {
                assert_eq!(
                    run(&mut emu, start, end, t, rate),
                    time_to_index(t, rate),
                    "time_to_index @ {start:#x} t={t} rate={rate}"
                );
                total += 1;
            }
        }
        // a deterministic ==pivot: t·rate·0.001+0.5 exactly at an integer boundary.
        // rate=2000, t=1 -> 1·2000·0.001 = 2.0, +0.5 = 2.5 -> trunc 2.
        assert_eq!(
            run(&mut emu, start, end, 1, 2000.0),
            time_to_index(1, 2000.0),
            "time_to_index pivot @ {start:#x}"
        );
        // rate=500, t=1 -> 0.5, +0.5 = 1.0 -> trunc 1 (the ==1.0 boundary).
        assert_eq!(
            run(&mut emu, start, end, 1, 500.0),
            time_to_index(1, 500.0),
            "time_to_index ==1.0 pivot @ {start:#x}"
        );
        total += 2;
    }
    let mut s = 0x78cf_3000u32;
    for _ in 0..5000 {
        let start_end = if next(&mut s) & 0x100 != 0 {
            blocks[0]
        } else {
            blocks[1]
        };
        let t = next(&mut s);
        let rate = rngf(&mut s, -100.0, 100.0) as f32;
        assert_eq!(
            run(&mut emu, start_end.0, start_end.1, t, rate),
            time_to_index(t, rate),
            "time_to_index fuzz t={t} rate={rate}"
        );
        total += 1;
    }
    eprintln!(
        "glue_geom_78cf30_time_to_index: {total} bit-exact (2x: fild qword; fmul; fmul[0.001]; fadd[0.5]; __ftol)"
    );
}

// ===========================================================================================
// Row 0x78d1a0 quad_scale — copied-byte ISLAND per field offset:
//   fld dword [scale_cell]; <source fmul [eax+off] bytes>; fstp dword [out_cell].
// The source `fmul [eax+0xc]` (d8 48 0c @0x78d480) and `fmul [eax+0x8]` (d8 48 08 @0x78d4ae) are
// byte-verified against the island's fmul; eax points at a field cell so [eax+off] = field.
// fmul ×2. Feed scale at [scale_cell] and field at [eax+off]; read the f32 product at [out_cell].
// ===========================================================================================
#[test]
fn glue_geom_78d1a0_quad_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let scale_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let field_base = emu.alloc(0x40); // eax; [eax+0xc]/[eax+0x8] = field

    // build one island per offset: fld [scale] (D9 05 disp32); <fmul bytes>; fstp [out] (D9 1D disp32)
    let build = |emu: &mut Emu, fmul_bytes: &[u8]| -> (u64, u64, u64) {
        let isl = emu.alloc(0x20);
        let mut code = vec![0xd9u8, 0x05];
        code.extend_from_slice(&(scale_cell as u32).to_le_bytes());
        let fmul_off = code.len() as u64;
        code.extend_from_slice(fmul_bytes);
        code.push(0xd9);
        code.push(0x1d);
        code.extend_from_slice(&(out_cell as u32).to_le_bytes());
        let isl_end = isl + code.len() as u64;
        emu.write(isl, &code);
        (isl, isl + fmul_off, isl_end)
    };
    // offset 0xc (0x78d480) and offset 0x8 (0x78d4ae)
    let (isl_c, fmul_c, end_c) = build(&mut emu, &[0xd8, 0x48, 0x0c]);
    let (isl_8, fmul_8, end_8) = build(&mut emu, &[0xd8, 0x48, 0x08]);
    // byte-verify each island's fmul == its source site (else the island proves nothing).
    assert_eq!(
        emu.read(fmul_c, 3),
        emu.read(0x78d480, 3),
        "quad_scale island fmul[+0xc] != source @ 0x78d480"
    );
    assert_eq!(
        emu.read(fmul_8, 3),
        emu.read(0x78d4ae, 3),
        "quad_scale island fmul[+0x8] != source @ 0x78d4ae"
    );

    let run = |emu: &mut Emu, isl: u64, end: u64, foff: u64, scale: f32, field: f32| -> f32 {
        emu.set_reg(RegisterX86::EAX, field_base);
        emu.write_u32(scale_cell, scale.to_bits());
        emu.write_u32(field_base + foff, field.to_bits());
        emu.run(isl, end);
        emu.read_f32(out_cell)
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, scale: f32, field: f32| {
        let want = quad_scale(scale, field);
        assert_eq!(
            run(emu, isl_c, end_c, 0xc, scale, field).to_bits(),
            want.to_bits(),
            "quad_scale[+0xc] scale={scale} field={field}"
        );
        assert_eq!(
            run(emu, isl_8, end_8, 0x8, scale, field).to_bits(),
            want.to_bits(),
            "quad_scale[+0x8] scale={scale} field={field}"
        );
        *total += 2;
    };

    // deterministic boundaries: 0, 1, exact powers, signs, and a ==product pivot.
    let scales = [
        0.0f32, -0.0, 1.0, -1.0, 0.5, 2.0, 0.75, 12.0, 1e-7, 1e7, 0.999_999,
    ];
    let fields = [
        0.0f32, -0.0, 1.0, -1.0, 0.5, 256.0, 0.125, 3.5, -0.001, 65535.0,
    ];
    for &scale in &scales {
        for &field in &fields {
            check(&mut emu, &mut total, scale, field);
        }
    }
    // ==pivot: scale·field is exactly representable f32 at one product but rounds at the f32 store.
    check(&mut emu, &mut total, 1.0 / 3.0, 3.0); // 0.333… · 3 -> ~1.0 (rounding boundary)
    check(&mut emu, &mut total, 0.1, 10.0); // 0.1f32 · 10 -> the classic non-exact pivot

    let mut s = 0x78d1_a000u32;
    for _ in 0..4000 {
        let scale = rngf(&mut s, -10.0, 10.0) as f32;
        let field = rngf(&mut s, -1000.0, 1000.0) as f32;
        check(&mut emu, &mut total, scale, field);
    }
    eprintln!(
        "glue_geom_78d1a0_quad_scale: {total} bit-exact (2x island: fld; fmul [eax+off]; fstp)"
    );
}

// ===========================================================================================
// Row 0x78c360 read_movie_avi — the whole function, over synthetic AVI files the test builds.
// The Storm calls are stubs over an in-memory file: the open (0x6477c0), the read (0x648460),
// the seek (0x648a60), the close (0x648730), the buffer allocator (0x6462e0) and the path
// formatter (0x64a7f0). `SStrCmpI` (0x64a4c0) and `__alloca_probe` (0x40a280) run as the exe has
// them. The reads and seeks are the stubs' only effect on the result: the open writes a handle,
// the read copies the bytes that exist and fails when fewer than asked remain, and the seek
// returns the new position (whence 0 from the start, 1 from the current position).
// ===========================================================================================

use benilla_re_ui::movieframe::{read_movie_avi, MovieAvi};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const READ_MOVIE_AVI: u64 = 0x78c360;
const SENTINEL: u32 = 0xa5a5_a5a5;

fn le(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

/// A chunk: id, size, payload, and a pad byte after an odd payload (not counted in the size).
fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend(le(payload.len() as u32));
    v.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        v.push(0);
    }
    v
}

fn list(kind: &[u8; 4], ty: &[u8; 4], kids: &[Vec<u8>]) -> Vec<u8> {
    let body: Vec<u8> = kids.concat();
    let mut v = kind.to_vec();
    v.extend(le(4 + body.len() as u32));
    v.extend_from_slice(ty);
    v.extend(body);
    v
}

/// A 56-byte stream header: type, handler, flags, priority and language, initial frames, scale,
/// rate, start, length, then the buffer size, quality, sample size and frame rectangle.
fn strh(kind: &[u8; 4], scale: u32, rate: u32, length: u32) -> Vec<u8> {
    let mut p = kind.to_vec();
    for w in [0, 0, 0, 0, scale, rate, 0, length, 0, 0, 0, 0, 0] {
        p.extend(le(w));
    }
    chunk(b"strh", &p)
}

fn strf_video(width: u32, height: u32) -> Vec<u8> {
    let mut p = Vec::new();
    for w in [40, width, height, 0x0018_0001, 0x3556_4944, 0, 0, 0, 0, 0] {
        p.extend(le(w));
    }
    chunk(b"strf", &p)
}

fn strf_audio() -> Vec<u8> {
    chunk(
        b"strf",
        &[1, 0, 2, 0, 0x44, 0xac, 0, 0, 0, 0, 0, 0, 4, 0, 16, 0, 0, 0],
    )
}

#[derive(Clone, Copy)]
enum Stream {
    /// scale, rate, length, width, height
    Video(u32, u32, u32, u32, u32),
    Audio,
}

#[derive(Clone)]
struct Spec {
    /// The tag spellings of the file: lower-case or upper-case.
    lower: bool,
    streams: Vec<Stream>,
    /// Chunks of the `movi` list in file order: id and payload.
    movi: Vec<([u8; 4], Vec<u8>)>,
    /// The `idx1` entries as positions in `movi`, in index order; `None` omits the index.
    index: Option<Vec<usize>>,
    /// A chunk before `hdrl`, between it and `movi`, and after the index.
    junk_before: bool,
    junk_between: bool,
    junk_after: bool,
}

fn spell(spec: &Spec, tag: &[u8; 4]) -> [u8; 4] {
    if spec.lower {
        tag.map(|b| b.to_ascii_lowercase())
    } else {
        *tag
    }
}

fn build(spec: &Spec) -> Vec<u8> {
    let mut kids = vec![chunk(&spell(spec, b"avih"), &[0; 56])];
    for s in &spec.streams {
        let strl = match *s {
            Stream::Video(scale, rate, len, w, h) => {
                vec![
                    strh(&spell(spec, b"vids"), scale, rate, len),
                    strf_video(w, h),
                ]
            }
            Stream::Audio => vec![strh(&spell(spec, b"auds"), 1, 44100, 0), strf_audio()],
        };
        kids.push(list(&spell(spec, b"LIST"), &spell(spec, b"strl"), &strl));
    }
    let hdrl = list(&spell(spec, b"LIST"), &spell(spec, b"hdrl"), &kids);

    let mut movi_kids = Vec::new();
    let mut offsets = Vec::new();
    let mut at = 4u32; // from the `movi` type field
    for (id, payload) in &spec.movi {
        let c = chunk(id, payload);
        offsets.push(at);
        at += c.len() as u32;
        movi_kids.push(c);
    }
    let movi = list(&spell(spec, b"LIST"), &spell(spec, b"movi"), &movi_kids);

    let mut body = spell(spec, b"AVI ").to_vec();
    if spec.junk_before {
        body.extend(chunk(b"JUNK", &[7; 12]));
    }
    body.extend(hdrl);
    if spec.junk_between {
        body.extend(list(b"LIST", b"INFO", &[chunk(b"ISFT", b"benilla\0")]));
    }
    body.extend(movi);
    if let Some(order) = &spec.index {
        let mut p = Vec::new();
        for &i in order {
            p.extend(spec.movi[i].0);
            p.extend(le(0x10)); // flags
            p.extend(le(offsets[i]));
            p.extend(le(spec.movi[i].1.len() as u32));
        }
        body.extend(chunk(&spell(spec, b"idx1"), &p));
    }
    if spec.junk_after {
        body.extend(chunk(b"JUNK", &[9; 6]));
    }
    let mut file = spell(spec, b"RIFF").to_vec();
    file.extend(le(body.len() as u32));
    file.extend(body);
    file
}

/// What an AVI reader that follows the format would pull from the file: the `d` chunks as a size
/// and data, the `w` chunks, each in index order, with the video record sizes counted in.
fn by_the_format(spec: &Spec) -> (Vec<u8>, Vec<u8>) {
    let (mut video, mut audio) = (Vec::new(), Vec::new());
    for &i in spec.index.iter().flatten() {
        let (id, payload) = &spec.movi[i];
        if id[2] == b'd' {
            video.extend(le(payload.len() as u32));
            video.extend_from_slice(payload);
        } else if id[2] == b'w' {
            audio.extend_from_slice(payload);
        }
    }
    (video, audio)
}

/// The emulator with the Storm stubs in place, and the in-memory file they serve.
struct Rig<'a> {
    emu: Emu<'a>,
    file: Rc<RefCell<Vec<u8>>>,
    opens: Rc<Cell<bool>>,
    closes: Rc<Cell<u32>>,
    arena: u64,
    name: u64,
    this: u64,
}

impl<'a> Rig<'a> {
    fn new(wow: &Wow) -> Rig<'a> {
        let mut emu = Emu::new(wow);
        let file = Rc::new(RefCell::new(Vec::new()));
        let pos = Rc::new(Cell::new(0i64));
        let opens = Rc::new(Cell::new(true));
        let closes = Rc::new(Cell::new(0u32));
        // 0x64a7f0 SStrPrintf (cdecl): the name is not read back.
        emu.stub(0x64a7f0, 0, 0);
        // 0x6477c0 OpenEx(archive, name, flags, &handle): flags 1, a handle on success.
        {
            let (opens, pos) = (opens.clone(), pos.clone());
            emu.stub_fn(0x6477c0, 4, move |io| {
                assert_eq!(io.arg(2), 1, "open flags");
                if !opens.get() {
                    return 0;
                }
                pos.set(0);
                io.write_u32(u64::from(io.arg(3)), 0x1234);
                1
            });
        }
        // 0x648460 Read(file, buffer, size, &read, 0, 0).
        {
            let (file, pos) = (file.clone(), pos.clone());
            emu.stub_fn(0x648460, 6, move |io| {
                assert_eq!(io.arg(0), 0x1234, "read handle");
                let file = file.borrow();
                let start = usize::try_from(pos.get()).unwrap().min(file.len());
                let want = io.arg(2) as usize;
                let end = (start + want).min(file.len());
                io.write(u64::from(io.arg(1)), &file[start..end]);
                pos.set(pos.get() + (end - start) as i64);
                if io.arg(3) != 0 {
                    io.write_u32(u64::from(io.arg(3)), (end - start) as u32);
                }
                u32::from(end - start == want)
            });
        }
        // 0x648a60 SetFilePointer(file, distance, &high, whence): high is null here.
        {
            let pos = pos.clone();
            emu.stub_fn(0x648a60, 4, move |io| {
                assert_eq!(io.arg(0), 0x1234, "seek handle");
                assert_eq!(io.arg(2), 0, "high distance");
                match io.arg(3) {
                    0 => pos.set(i64::from(io.arg(1))),
                    1 => pos.set(pos.get() + i64::from(io.arg(1) as i32)),
                    w => panic!("whence {w}"),
                }
                pos.get() as u32
            });
        }
        {
            let closes = closes.clone();
            emu.stub_fn(0x648730, 1, move |_| {
                closes.set(closes.get() + 1);
                1
            });
        }
        let arena = emu.alloc(0x20000);
        emu.stub_alloc_sized(0x6462e0, arena, arena + 0x20000, 0, 4);
        let name = emu.alloc(0x10);
        let this = emu.alloc(0x400);
        Rig {
            emu,
            file,
            opens,
            closes,
            arena,
            name,
            this,
        }
    }

    /// Runs the reader over `file`: `None` when it returns 0, else the fields it stored.
    fn run(&mut self, file: Vec<u8>, opens: bool) -> Option<MovieAvi> {
        self.opens.set(opens);
        *self.file.borrow_mut() = file;
        let closes = self.closes.get();
        self.emu.reset_bump(self.arena);
        self.emu.write(self.arena, &vec![0u8; 0x20000]);
        self.emu.write(self.this, &[0u8; 0x400]);
        for off in [0x33c, 0x340, 0x344, 0x348] {
            self.emu.write_u32(self.this + off, SENTINEL);
        }
        let r = self
            .emu
            .call(READ_MOVIE_AVI, self.this as u32, 0, &[self.name as u32]);
        // the handle is closed on every path that opened it
        assert_eq!(self.closes.get() - closes, u32::from(opens), "closes");
        let (emu, this) = (&self.emu, self.this);
        let field = |off: u64| {
            let v = emu.read_u32(this + off);
            (v != SENTINEL).then_some(v)
        };
        if r == 0 {
            for off in [0x33c, 0x340, 0x344, 0x348] {
                assert!(field(off).is_none(), "field {off:#x} stored on failure");
            }
            return None;
        }
        assert_eq!(r, 1, "return value");
        assert_eq!(
            emu.read_u32(this + 0x3a0),
            emu.read_u32(this + 0x39c),
            "the read pointer starts at the video buffer"
        );
        let buf = |ptr: u64, len: u64| {
            let p = u64::from(emu.read_u32(this + ptr));
            emu.read(p, emu.read_u32(this + len) as usize)
        };
        Some(MovieAvi {
            frame_rate: field(0x33c).map(f32::from_bits),
            frame_count: field(0x348),
            width: field(0x340),
            height: field(0x344),
            video: buf(0x39c, 0x3a4),
            audio: buf(0x3a8, 0x3ac),
        })
    }

    /// Builds the file, runs the exe and the model over it and requires them to agree; returns
    /// what they stored.
    fn check(&mut self, what: &str, spec: &Spec) -> MovieAvi {
        let file = build(spec);
        let want = read_movie_avi(&file);
        let got = self.run(file, true);
        assert_eq!(got, want, "{what}");
        got.expect("a well-formed file reads")
    }
}

/// A change to the plain file's spec.
type Mutation = fn(&mut Spec);

fn payload(n: usize, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| seed.wrapping_add((i as u8).wrapping_mul(7)))
        .collect()
}

fn base() -> Spec {
    Spec {
        lower: false,
        streams: vec![Stream::Video(1001, 30000, 5, 1024, 768), Stream::Audio],
        movi: vec![
            (*b"00dc", payload(10, 1)),
            (*b"01wb", payload(6, 2)),
            (*b"00dc", payload(4, 3)),
            (*b"01wb", payload(8, 4)),
            (*b"00dc", payload(0, 5)),
        ],
        index: Some(vec![0, 1, 2, 3, 4]),
        junk_before: false,
        junk_between: false,
        junk_after: false,
    }
}

#[test]
fn movie_avi_read_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut cases = 0u32;

    // A plain file: the rate is dwRate / dwScale as a float, the size is the first video strf's,
    // and the data are the format's own, so the offsets count from the `movi` type field.
    let spec = base();
    let got = rig.check("plain", &spec);
    assert_eq!(got.frame_rate, Some((30000.0f64 / 1001.0) as f32));
    assert_eq!(
        (got.frame_count, got.width, got.height),
        (Some(5), Some(1024), Some(768))
    );
    assert_eq!(
        (got.video, got.audio),
        by_the_format(&spec),
        "plain: the format's data"
    );
    cases += 1;

    // Chunks the walks skip, and the case of every tag.
    let variants: [(&str, Mutation); 4] = [
        ("junk before", |s| s.junk_before = true),
        ("junk between", |s| s.junk_between = true),
        ("junk after", |s| s.junk_after = true),
        ("lower-case tags", |s| s.lower = true),
    ];
    for (what, mutate) in variants {
        let mut spec = base();
        mutate(&mut spec);
        let got = rig.check(what, &spec);
        assert_eq!(
            (got.video, got.audio),
            by_the_format(&spec),
            "{what}: the format's data"
        );
        cases += 1;
    }

    // The id test reads the third letter: ids that end in `d` or `w` without it, and ids whose
    // third letter is `d` or `w` without the usual last letter, in every combination. The extra
    // chunk adds its record to the video buffer, its data to the audio buffer, or nothing.
    let ids: [&[u8; 4]; 14] = [
        b"00dc", b"00db", b"01dc", b"00wb", b"01wb", b"02wb", b"00dw", b"00wd", b"00xd", b"00xw",
        b"00Dc", b"00Wb", b"ix00", b"00pc",
    ];
    let (base_video, base_audio) = by_the_format(&base());
    for (n, id) in ids.iter().enumerate() {
        let len = 6 + n;
        let mut spec = base();
        spec.movi.push((**id, payload(len, 9)));
        spec.movi.push((*b"00dc", payload(2, 11)));
        spec.index = Some((0..spec.movi.len()).collect());
        let got = rig.check(&format!("id {}", String::from_utf8_lossy(*id)), &spec);
        let (video, audio) = match id[2] {
            b'd' => (4 + len, 0),
            b'w' => (0, len),
            _ => (0, 0),
        };
        assert_eq!(
            got.video.len(),
            base_video.len() + video + 6,
            "video, id {id:?}"
        );
        assert_eq!(
            got.audio.len(),
            base_audio.len() + audio,
            "audio, id {id:?}"
        );
        cases += 1;
    }

    // The rate and the frame count come from every video header, the last one winning; the size
    // from a strf that follows a video header with no audio header before it.
    let mut spec = base();
    spec.streams = vec![Stream::Audio, Stream::Video(1, 25, 7, 800, 600)];
    let got = rig.check("audio first", &spec);
    assert_eq!(
        (got.frame_rate, got.frame_count, got.width, got.height),
        (Some(25.0), Some(7), None, None)
    );
    let mut spec = base();
    spec.streams = vec![
        Stream::Video(1, 24, 3, 800, 600),
        Stream::Audio,
        Stream::Video(1, 25, 7, 1024, 768),
    ];
    let got = rig.check("two video headers", &spec);
    assert_eq!((got.frame_rate, got.frame_count), (Some(25.0), Some(7)));
    assert_eq!(
        (got.width, got.height),
        (Some(800), Some(600)),
        "the strf after an audio header is not read"
    );
    let mut spec = base();
    spec.streams = vec![
        Stream::Video(1, 24, 3, 800, 600),
        Stream::Video(2, 50, 9, 1024, 768),
        Stream::Audio,
    ];
    let got = rig.check("two video headers, no audio before", &spec);
    assert_eq!(
        (got.frame_rate, got.frame_count, got.width, got.height),
        (Some(25.0), Some(9), Some(1024), Some(768))
    );
    let mut spec = base();
    spec.streams = vec![Stream::Video(0, 30, 5, 1024, 768)];
    assert_eq!(
        rig.check("zero scale", &spec).frame_rate,
        Some(f32::INFINITY)
    );
    let mut spec = base();
    spec.index = None;
    let got = rig.check("no index", &spec);
    assert_eq!((got.video.len(), got.audio.len()), (0, 0));
    let mut spec = base();
    spec.streams = Vec::new();
    spec.index = Some(vec![0, 1]);
    let got = rig.check("no streams", &spec);
    assert_eq!(got.frame_rate, None);
    cases += 6;

    // A read that fails ends the walk and the function still returns 1, and a file that does not
    // open or carry the two tags returns 0 with the handle closed.
    let plain = build(&base());
    for cut in [12, 13, 19, 20] {
        let got = rig.run(plain[..cut].to_vec(), true);
        assert_eq!(got, read_movie_avi(&plain[..cut]), "cut at {cut}");
        assert!(got.is_some(), "cut at {cut}");
        cases += 1;
    }
    for cut in [0, 5, 11] {
        assert_eq!(rig.run(plain[..cut].to_vec(), true), None, "cut at {cut}");
        cases += 1;
    }
    assert_eq!(rig.run(plain.clone(), false), None, "open fails");
    cases += 1;
    for (at, bytes) in [(0, *b"RIFX"), (8, *b"AVIX"), (8, *b"WAVE")] {
        let mut bad = plain.clone();
        bad[at..at + 4].copy_from_slice(&bytes);
        assert_eq!(rig.run(bad.clone(), true), None, "tag {bytes:?} at {at}");
        assert_eq!(read_movie_avi(&bad), None);
        cases += 1;
    }

    // Fuzz: stream order and count, ids from the pool above, chunk sizes, index order and repeats.
    let mut seed = 0x78c3_6001u32;
    for n in 0..250 {
        let mut spec = base();
        spec.lower = next(&mut seed).is_multiple_of(4);
        spec.junk_before = next(&mut seed).is_multiple_of(3);
        spec.junk_between = next(&mut seed).is_multiple_of(3);
        spec.junk_after = next(&mut seed).is_multiple_of(3);
        spec.streams = (0..1 + next(&mut seed) % 4)
            .map(|_| match next(&mut seed) % 3 {
                0 => Stream::Audio,
                _ => Stream::Video(
                    1 + next(&mut seed) % 1001,
                    next(&mut seed) % 60000,
                    next(&mut seed) % 99,
                    16 * (1 + next(&mut seed) % 64),
                    next(&mut seed) % 800,
                ),
            })
            .collect();
        spec.movi = (0..next(&mut seed) % 24)
            .map(|k| {
                (
                    *ids[next(&mut seed) as usize % ids.len()],
                    payload((next(&mut seed) % 40) as usize, k as u8),
                )
            })
            .collect();
        spec.index = if next(&mut seed).is_multiple_of(8) || spec.movi.is_empty() {
            None
        } else {
            let count = next(&mut seed) % 30;
            Some(
                (0..count)
                    .map(|_| next(&mut seed) as usize % spec.movi.len())
                    .collect(),
            )
        };
        rig.check(&format!("fuzz {n}"), &spec);
        cases += 1;
    }
    eprintln!("movie_avi_read_whole: {cases} files, the exe and the model agree");
}
