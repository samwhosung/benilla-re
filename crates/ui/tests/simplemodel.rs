//! Differential tests of the arithmetic of the simple-model functions `SimpleModelSetModel`
//! (`0x76cd30`), the geometry getters (`0x76d080`, `0x76d0d0`), `SimpleModelSetAlpha`
//! (`0x76d120`), `SimpleModelUpdateTransform` (`0x76d1a0`), `SimpleModelDraw` (`0x76d240`) and
//! `0x76d7f0`, each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice of the widget-glue TU `[0x76cd30, 0x76d7f0]` is diffed by executing the
//! binary's own bytes — either as an in-place `Emu::run` instruction slice, or (where the owned op
//! is a tiny mid-block sequence whose value is left LIVE in st0 with no slot store, the
//! mid-block hazard) as a COPIED-BYTE ISLAND: the EXACT source bytes assembled in scratch with our
//! own register pointers, byte-verified against the disasm, plus an appended capture `fstp`. Every
//! in-body FP-arith opcode (the arithmetic opcode set: fmul/fadd/fsub/fabs/fmulp/…) lands in a
//! diffed slice. Fuzz reads the LCG's HIGH bits. Each owned-slice
//! test carries a deterministic `==threshold` boundary probe. The eps change-detect /
//! clamp branches are reconstructed from the EMITTED jcc, not the mask: `test ah,5` (C0|C2) read
//! by `jp`/`jnp` is a PARITY test.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::simplemodel::{
    byte_to_unit, d1a0_base, d1a0_components, d240_scaled, d240_settled, d680_channel, d680_pack,
    d7f0_unit_convert, field_diff, EPS_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}
fn ah_of(emu: &Emu) -> u8 {
    ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

struct Frame {
    this: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let this = emu.alloc(0x400);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { this, ebp, esp }
}
fn set_frame(emu: &mut Emu, f: &Frame) {
    emu.set_reg(RegisterX86::ESI, f.this);
    emu.set_reg(RegisterX86::ECX, f.this);
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESP, f.esp);
}

// ===========================================================================================
// 0x76cd30 / 0x76d120 — byte→[0,1] normalize: fild [int]; fmul [1/255]; fstp [esp]. fmul×1,
// byte-identical kernel (one slice covers both addresses; the int slot is [ebp+8]).
// ===========================================================================================
#[test]
fn glue_76cd30_byte_to_unit() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    // Both kernels: db 45 08 (fild [ebp+8]); d8 0d c8 26 80 00 (fmul [0x8026c8]); d9 1c 24 (fstp
    // [esp]). 0x76cd6b..0x76cd77 == 0x76d144..0x76d150 byte-for-byte (verified vs disasm).
    let run = |emu: &mut Emu, start: u64, end: u64, b: u8| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp + 0x8, u32::from(b)); // the movzx'd byte field, re-stored to [ebp+8]
        emu.run(start, end);
        emu.read_f32(f.esp) // fstp [esp]
    };
    let mut total = 0u32;
    for &(start, end) in &[(0x76cd6bu64, 0x76cd77u64), (0x76d144, 0x76d150)] {
        for b in 0u8..=255 {
            assert_eq!(
                run(&mut emu, start, end, b).to_bits(),
                byte_to_unit(b).to_bits(),
                "byte_to_unit @ {start:#x} b={b}"
            );
            total += 1;
        }
    }
    eprintln!("glue_76cd30_byte_to_unit: {total} bit-exact (0x76cd30/0x76d120 fild;fmul[1/255], all 256 bytes)");
}

// ===========================================================================================
// 0x76d080 / 0x76d0d0 — field difference: fld [a]; fsub [b]; (value LEFT LIVE in st0). fsub×1,
// clone twins (disp32 only). Copied-byte island + appended capture fstp (byte-verified).
// ===========================================================================================
#[test]
fn glue_76d080_field_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let out = emu.alloc(8);

    // 0x76d080 island = 0x76d0b5..0x76d0c1 verbatim: d9 86 9c 03 00 00 (fld [esi+0x39c]);
    // d8 a6 90 03 00 00 (fsub [esi+0x390]). The binary leaves the difference LIVE in st0 (no slot
    // store — the caller consumes the x87 value), so the genuine observable is the un-narrowed
    // PC_53 f64; we append `dd 1d <out>` (fstp QWORD [out]) to capture it as f64 WITHOUT an f32
    // round. The fld/fsub bytes are byte-verified against the .text dump above.
    let isl0 = emu.alloc(0x20);
    let mut c0 = vec![0xd9u8, 0x86, 0x9c, 0x03, 0x00, 0x00]; // fld  [esi+0x39c]
    c0.extend_from_slice(&[0xd8, 0xa6, 0x90, 0x03, 0x00, 0x00]); // fsub [esi+0x390]
    c0.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out]  (capture live st0 as f64)
    c0.extend_from_slice(&(out as u32).to_le_bytes());
    let isl0_end = isl0 + c0.len() as u64;
    emu.write(isl0, &c0);

    // 0x76d0d0 island = 0x76d105..0x76d111 verbatim: fld [esi+0x3a0]; fsub [esi+0x394].
    let isl1 = emu.alloc(0x20);
    let mut c1 = vec![0xd9u8, 0x86, 0xa0, 0x03, 0x00, 0x00]; // fld  [esi+0x3a0]
    c1.extend_from_slice(&[0xd8, 0xa6, 0x94, 0x03, 0x00, 0x00]); // fsub [esi+0x394]
    c1.extend_from_slice(&[0xdd, 0x1d]);
    c1.extend_from_slice(&(out as u32).to_le_bytes());
    let isl1_end = isl1 + c1.len() as u64;
    emu.write(isl1, &c1);

    let read_f64 = |emu: &Emu, va: u64| -> f64 {
        f64::from_bits(u64::from(emu.read_u32(va)) | (u64::from(emu.read_u32(va + 4)) << 32))
    };
    let run = |emu: &mut Emu, isl: u64, end: u64, off_a: u64, off_b: u64, a: f32, b: f32| -> f64 {
        set_frame(emu, &f);
        emu.write_u32(f.this + off_a, a.to_bits());
        emu.write_u32(f.this + off_b, b.to_bits());
        emu.run(isl, end);
        read_f64(emu, out)
    };
    let mut total = 0u32;
    // deterministic boundaries: exact-cancel (a==b ⇒ 0), and a representable pair.
    for (a, b) in [
        (1.0f32, 1.0f32),
        (3.5, 1.25),
        (-2.0, 2.0),
        (0.0, 0.0),
        (16_777_216.0, 1.0), // f32 ulp boundary
    ] {
        let want = field_diff(a, b).to_bits();
        assert_eq!(
            run(&mut emu, isl0, isl0_end, 0x39c, 0x390, a, b).to_bits(),
            want,
            "field_diff 0x76d080 a={a} b={b}"
        );
        assert_eq!(
            run(&mut emu, isl1, isl1_end, 0x3a0, 0x394, a, b).to_bits(),
            want,
            "field_diff 0x76d0d0 a={a} b={b}"
        );
        total += 2;
    }
    let mut s = 0x76d0_8000u32;
    for _ in 0..3000 {
        let a = rng(&mut s, -1000.0, 1000.0);
        let b = rng(&mut s, -1000.0, 1000.0);
        let want = field_diff(a, b).to_bits();
        assert_eq!(
            run(&mut emu, isl0, isl0_end, 0x39c, 0x390, a, b).to_bits(),
            want,
            "field_diff 0x76d080 fuzz"
        );
        assert_eq!(
            run(&mut emu, isl1, isl1_end, 0x3a0, 0x394, a, b).to_bits(),
            want,
            "field_diff 0x76d0d0 fuzz"
        );
        total += 2;
    }
    eprintln!("glue_76d080_field_diff: {total} bit-exact (0x76d080/0x76d0d0 fld;fsub copied-byte islands)");
}

// ===========================================================================================
// 0x76d1a0 — base scale (fmul[5/3]; fmul[+0x7c]; fmulp) + 3 component scales (fmul×3). The
// in-place slice 0x76d1c1..0x76d235 runs the leaf 0x41ae70(1.0)=0.6 in-emu; covers all 6 fmul.
// ===========================================================================================
#[test]
fn glue_76d1a0_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    // 0x76d235 is `call 0x710650` (the next stage); halt before it. The slice pushes 0x3f800000,
    // calls the leaf, then the 6 fmuls store: base→[ebp-0xc]/[ebp-8]/[ebp-4]; components→
    // [ebp-0x24]/[ebp-0x20]/[ebp-0x1c]. ESI=this; ESP must have room for the push+call.
    let run =
        |emu: &mut Emu, f3a0: f32, f7c: f32, c390: f32, c394: f32, c398: f32| -> (f32, [f32; 3]) {
            set_frame(emu, &f);
            emu.write_u32(f.this + 0x3a0, f3a0.to_bits());
            emu.write_u32(f.this + 0x7c, f7c.to_bits());
            emu.write_u32(f.this + 0x390, c390.to_bits());
            emu.write_u32(f.this + 0x394, c394.to_bits());
            emu.write_u32(f.this + 0x398, c398.to_bits());
            emu.run(0x76d1c1, 0x76d235);
            let base = emu.read_f32(f.ebp - 0xc);
            let comps = [
                emu.read_f32(f.ebp - 0x24),
                emu.read_f32(f.ebp - 0x20),
                emu.read_f32(f.ebp - 0x1c),
            ];
            (base, comps)
        };
    let mut total = 0u32;
    // deterministic: f3a0=f7c=0 ⇒ base=0, comps=0; and a unit case.
    for (f3a0, f7c, c390, c394, c398) in [
        (0.0f32, 0.0f32, 1.0f32, 2.0f32, 3.0f32),
        (1.0, 1.0, 1.0, 1.0, 1.0),
        (2.0, 0.5, -1.0, 4.0, 0.0),
        (3.0, 0.0, 5.0, 6.0, 7.0), // f7c=0 ⇒ comps all 0
    ] {
        let (base, comps) = run(&mut emu, f3a0, f7c, c390, c394, c398);
        assert_eq!(
            base.to_bits(),
            d1a0_base(f3a0, f7c).to_bits(),
            "d1a0_base f3a0={f3a0} f7c={f7c}"
        );
        let want = d1a0_components(f7c, c390, c394, c398);
        for i in 0..3 {
            assert_eq!(comps[i].to_bits(), want[i].to_bits(), "d1a0_comp[{i}]");
        }
        total += 4;
    }
    let mut s = 0x76d1_a000u32;
    for _ in 0..2500 {
        let f3a0 = rng(&mut s, -100.0, 100.0);
        let f7c = rng(&mut s, -100.0, 100.0);
        let c390 = rng(&mut s, -100.0, 100.0);
        let c394 = rng(&mut s, -100.0, 100.0);
        let c398 = rng(&mut s, -100.0, 100.0);
        let (base, comps) = run(&mut emu, f3a0, f7c, c390, c394, c398);
        assert_eq!(
            base.to_bits(),
            d1a0_base(f3a0, f7c).to_bits(),
            "d1a0_base fuzz"
        );
        let want = d1a0_components(f7c, c390, c394, c398);
        for i in 0..3 {
            assert_eq!(comps[i].to_bits(), want[i].to_bits(), "d1a0_comp fuzz[{i}]");
        }
        total += 4;
    }
    eprintln!("glue_76d1a0_scale: {total} bit-exact (0x76d1a0 base fmul[5/3]+fmul+fmulp + 3 component fmul; leaf 0x41ae70 in-emu)");
}

// ===========================================================================================
// 0x76d240 — slice A: 2× clamp-pair eps change-detect (fsub;fabs per pair). slice B: 4× scaled
// product (fmul). Both diffed as in-place slices (the function's body is un-synthesizable: vtable
// call + matrix builders — but these two FP slices are self-contained on the clamped slots).
// ===========================================================================================
#[test]
fn glue_76d240_change_detect_and_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let eps = f32::from_bits(EPS_BITS);

    // --- slice A: change-detect. Pair 1 = ([ebp-0xc],[ebp-0x4]) @0x76d574..0x76d58d; pair 2 =
    // ([ebp-0x10],[ebp-0x8]) @0x76d58d..0x76d5a6. Each: fld [a]; fsub [b]; fabs; fcomp [eps];
    // fnstsw; test ah,5; jnp. We read AH after the fnstsw. (Both pairs share the exact shape.)
    let run_chg = |emu: &mut Emu, start: u64, oa: u64, ob: u64, a: f32, b: f32| -> u8 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - oa, a.to_bits());
        emu.write_u32(f.ebp - ob, b.to_bits());
        emu.run(start, start + 0x10); // fld;fsub;fabs;fcomp;fnstsw = exactly through the fnstsw
        ah_of(emu)
    };
    let mut total = 0u32;
    let (mut chg, mut settled) = (0u32, 0u32);
    // boundary: |a-b| == eps exactly.
    for &(start, oa, ob) in &[(0x76d574u64, 0xcu64, 0x4u64), (0x76d58d, 0x10, 0x8)] {
        for (a, b, _l) in [
            (eps, 0.0f32, "eq"),
            (eps * 0.5, 0.0, "below"),
            (eps * 2.0, 0.0, "above"),
            (f32::NAN, 0.0, "nan"),
        ] {
            let ah = run_chg(&mut emu, start, oa, ob, a, b);
            // test ah,5; jnp -> taken (settled) iff ODD parity (|d|<eps). settled()⇒odd.
            let bin = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(bin, d240_settled(a, b), "d240 chg @ {start:#x} a={a} b={b}");
            if bin {
                settled += 1;
            } else {
                chg += 1;
            }
            total += 1;
        }
    }
    let mut s = 0x76d2_4000u32;
    for &(start, oa, ob) in &[(0x76d574u64, 0xcu64, 0x4u64), (0x76d58d, 0x10, 0x8)] {
        for _ in 0..800 {
            let b = rng(&mut s, -2.0, 2.0);
            let a = b + rng(&mut s, -1.0e-6, 1.0e-6);
            let ah = run_chg(&mut emu, start, oa, ob, a, b);
            let bin = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(bin, d240_settled(a, b), "d240 chg fuzz @ {start:#x}");
            if bin {
                settled += 1;
            } else {
                chg += 1;
            }
            total += 1;
        }
    }
    assert!(
        chg > 0 && settled > 0,
        "d240 change-detect: need both outcomes"
    );

    // --- slice B: 4 scaled products. 0x76d5a6..0x76d5d7: push 1.0; push 0; sub esp,0x10; then
    // fld [ebp-8];fmul [ebp-0x18];fstp [esp+0xc]; fld [ebp-0x10];fmul[ebp-0x18];fstp[esp+8];
    // fld[ebp-4];fmul[ebp-0x14];fstp[esp+4]; fld[ebp-0xc];fmul[ebp-0x14];fstp[esp]. Read the four
    // f32 from the final ESP (after the push/sub): +0xc,+8,+4,+0.
    let run_scl =
        |emu: &mut Emu, f8: f32, f10: f32, f4: f32, fc: f32, s18: f32, s14: f32| -> [f32; 4] {
            set_frame(emu, &f);
            emu.write_u32(f.ebp - 0x8, f8.to_bits());
            emu.write_u32(f.ebp - 0x10, f10.to_bits());
            emu.write_u32(f.ebp - 0x4, f4.to_bits());
            emu.write_u32(f.ebp - 0xc, fc.to_bits());
            emu.write_u32(f.ebp - 0x18, s18.to_bits());
            emu.write_u32(f.ebp - 0x14, s14.to_bits());
            emu.run(0x76d5a6, 0x76d5d7);
            let esp = emu.get_reg(RegisterX86::ESP);
            [
                emu.read_f32(esp + 0xc),
                emu.read_f32(esp + 0x8),
                emu.read_f32(esp + 0x4),
                emu.read_f32(esp),
            ]
        };
    // deterministic: scale 0 ⇒ all 0; scale 1 ⇒ identity.
    for (f8, f10, f4, fc, s18, s14) in [
        (1.0f32, 1.0f32, 1.0f32, 1.0f32, 0.0f32, 0.0f32),
        (1.0, 2.0, 3.0, 4.0, 1.0, 1.0),
        (0.25, 0.5, 0.75, 1.0, 2.5, -3.5),
    ] {
        let got = run_scl(&mut emu, f8, f10, f4, fc, s18, s14);
        let want = d240_scaled(f8, f10, f4, fc, s18, s14);
        for i in 0..4 {
            assert_eq!(got[i].to_bits(), want[i].to_bits(), "d240_scaled[{i}]");
        }
        total += 4;
    }
    for _ in 0..2000 {
        let f8 = rng(&mut s, 0.0, 1.0);
        let f10 = rng(&mut s, 0.0, 1.0);
        let f4 = rng(&mut s, 0.0, 1.0);
        let fc = rng(&mut s, 0.0, 1.0);
        let s18 = rng(&mut s, -100.0, 100.0);
        let s14 = rng(&mut s, -100.0, 100.0);
        let got = run_scl(&mut emu, f8, f10, f4, fc, s18, s14);
        let want = d240_scaled(f8, f10, f4, fc, s18, s14);
        for i in 0..4 {
            assert_eq!(got[i].to_bits(), want[i].to_bits(), "d240_scaled fuzz[{i}]");
        }
        total += 4;
    }
    eprintln!("glue_76d240_change_detect_and_scale: {total} bit-exact (slice-A fsub;fabs ×2 changed={chg} settled={settled}; slice-B fmul ×4)");
}

// ===========================================================================================
// 0x76d680 — colour pack: 3× (clamp[0,1]; fmul[255]; fadd[0.5]; __ftol) packed into EDI.
// In-place slice 0x76d6ac..0x76d79a covers all 6 fmul/fadd + the 3 __ftol; read EDI.
// ===========================================================================================
#[test]
fn glue_76d680_color_pack() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    // The unpacked channels are at [ebp-8],[ebp-0xc],[ebp-0x10] (0x7bbf20 wrote them). Slice runs
    // the three clamp+scale blocks then the three __ftol packs; halt at 0x76d79a (mov [ebp+8],edi).
    let run = |emu: &mut Emu, a8: f32, ac: f32, a10: f32| -> u32 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0x8, a8.to_bits());
        emu.write_u32(f.ebp - 0xc, ac.to_bits());
        emu.write_u32(f.ebp - 0x10, a10.to_bits());
        emu.run(0x76d6ac, 0x76d79a);
        emu.get_reg(RegisterX86::EDI) as u32
    };
    let mut total = 0u32;
    // deterministic boundaries: 0.0, 1.0 (exact clamp edges), and a mid value whose ·255+0.5 lands
    // on an integer boundary (e.g. c such that c·255+0.5 == 128.0).
    for (a8, ac, a10, _l) in [
        (0.0f32, 1.0f32, 0.5f32, "edges"),
        (-1.0, 2.0, 0.0, "out-of-range"),
        (1.0, 1.0, 1.0, "all-white"),
        (0.0, 0.0, 0.0, "all-black"),
        (
            (127.5 / 255.0),
            (0.5 / 255.0),
            (254.5 / 255.0),
            "ftol-boundaries",
        ),
        (f32::NAN, 0.5, 0.5, "nan-ch"),
    ] {
        let got = run(&mut emu, a8, ac, a10);
        assert_eq!(
            got,
            d680_pack(a8, ac, a10),
            "d680_pack a8={a8} ac={ac} a10={a10}"
        );
        // also exercise the single-channel kernel against the byte the pack places.
        assert_eq!(u32::from(d680_channel(a8)), got & 0xff, "d680_channel a8");
        total += 1;
    }
    let mut s = 0x76d6_8000u32;
    for _ in 0..3000 {
        // span below 0, in [0,1], and above 1 to hit every clamp branch.
        let a8 = rng(&mut s, -0.5, 1.5);
        let ac = rng(&mut s, -0.5, 1.5);
        let a10 = rng(&mut s, -0.5, 1.5);
        let got = run(&mut emu, a8, ac, a10);
        assert_eq!(got, d680_pack(a8, ac, a10), "d680_pack fuzz");
        total += 1;
    }
    eprintln!(
        "glue_76d680_color_pack: {total} bit-exact (0x76d680 3×(clamp;fmul[255];fadd[0.5];__ftol))"
    );
}

// ===========================================================================================
// 0x76d7f0 — unit-convert via __ftol: fld [ebp+8]; fmul [1000]; __ftol (truncate). fmul×1.
// In-place slice 0x76d843..0x76d859 covers fmul + __ftol; read EAX.
// ===========================================================================================
#[test]
fn glue_76d7f0_unit_convert() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    // 0x76d843: fld [ebp+8]; fmul [0x8022cc]=1000.0; fcom 0.0; fnstsw; call 0x40a2b0=__ftol. Halt
    // at 0x76d859 (push eax) — EAX holds the truncated i32.
    let run = |emu: &mut Emu, arg: f32| -> i32 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(0x76d843, 0x76d859);
        emu.get_reg(RegisterX86::EAX) as i32
    };
    let mut total = 0u32;
    // deterministic: exact-integer products and a fractional truncate boundary.
    for arg in [0.0f32, 1.0, 1.5, -1.5, 0.0019999, -0.0019999, 2.147] {
        assert_eq!(run(&mut emu, arg), d7f0_unit_convert(arg), "d7f0 arg={arg}");
        total += 1;
    }
    let mut s = 0x76d7_f000u32;
    for _ in 0..4000 {
        let arg = rng(&mut s, -2_000_000.0, 2_000_000.0);
        assert_eq!(
            run(&mut emu, arg),
            d7f0_unit_convert(arg),
            "d7f0 fuzz arg={arg}"
        );
        total += 1;
    }
    eprintln!("glue_76d7f0_unit_convert: {total} bit-exact (0x76d7f0 fmul[1000];__ftol truncate)");
}
