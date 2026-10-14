//! Differential tests of the arithmetic of `CLayoutFrame::SetPoint` (`0x767c70`),
//! `SetLayoutScale` (`0x768360`), `SetFrameScale` (`0x76ac10`), `PackColorFromFloats`
//! (`0x76f900`) and `0x783380`, each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed by executing the binary's own bytes. The whole-function rows
//! (`0x76f900` colour pack, `0x783380` guarded fraction) are diffed via `Emu::call`/`Emu::call_f64`
//! — the body is self-contained (only arg/`this` fields + the native `__ftol`), so the call covers
//! every FP-arith op by construction. The change-detect rows (`0x767c70`/`0x768360`/`0x76ac10`)
//! interleave object-graph derefs and vtable dispatch around their owned `fld; fsub(r); fabs;
//! fcomp [eps]` kernel, so each is diffed two ways: (a) a COPIED-BYTE ISLAND of the exact source
//! `fld; fsub(r); fabs` bytes (byte-verified == source) with an appended `fstp qword [cell]` to
//! capture the un-narrowed PC_53 difference (the genuine observable — the binary never stores it),
//! and (b) an IN-PLACE slice run through the `fnstsw` to read AH and reconstruct the EMITTED-jcc
//! branch by PARITY. Every in-body FP-arith opcode (the arithmetic opcode set) lands in
//! a diffed slice. Fuzz reads the LCG's HIGH bits. Each test carries
//! deterministic `==threshold` boundary probes: the eps pivot (`|diff| == eps`), the reject-zero
//! pivot (`arg == 0.0`), and the fraction's gate-sign pivot (`gate == 0` / `gate > 0`).
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::layout_scale::{
    ac10_diff, ac10_unchanged, accept_nonzero, anchor_diff, anchor_unchanged, channel, fraction,
    pack4, rsub_diff, rsub_unchanged, EPS_BITS,
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
fn read_f64(emu: &Emu, va: u64) -> f64 {
    f64::from_bits(u64::from(emu.read_u32(va)) | (u64::from(emu.read_u32(va + 4)) << 32))
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

// ===========================================================================================
// ROW 1 — 0x767c70 [true_ret 0x767d62 ret 0x18] : 2× epsilon change-detect (X, Y).
//   fsub ×2 (0x767cb9, 0x767cce) + fabs ×2 (0x767cbc, 0x767cd1) = 4 arithmetic.
//   COVERAGE: the X island (fld [edi+4]; fsub [ebp+0x14]; fabs) covers fsub@0x767cb9 + fabs@0x767cbc;
//   the Y island (fld [edi+8]; fsub [ebp+0x18]; fabs) covers fsub@0x767cce + fabs@0x767cd1. The two
//   in-place fnstsw slices re-execute the same ops and supply the branch parity ==eps probe.
// ===========================================================================================
#[test]
fn glue_767_767c70_anchor_change_detect() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let eps = f32::from_bits(EPS_BITS);

    // --- copied-byte islands: fld [edi+disp]; fsub [ebp+disp]; fabs; fstp qword [cell] ---
    let cell = emu.alloc(8);
    // X island = source 0x767cb6..0x767cbe verbatim (fld [edi+4]; fsub [ebp+0x14]; fabs).
    let islx = emu.alloc(0x20);
    let mut cx = vec![0xd9u8, 0x47, 0x04]; // fld  [edi+4]
    cx.extend_from_slice(&[0xd8, 0x65, 0x14]); // fsub [ebp+0x14]
    cx.extend_from_slice(&[0xd9, 0xe1]); // fabs
    cx.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [cell]
    cx.extend_from_slice(&(cell as u32).to_le_bytes());
    let islx_end = islx + cx.len() as u64;
    emu.write(islx, &cx);
    // Y island = source 0x767ccb..0x767cd3 verbatim (fld [edi+8]; fsub [ebp+0x18]; fabs).
    let isly = emu.alloc(0x20);
    let mut cy = vec![0xd9u8, 0x47, 0x08]; // fld  [edi+8]
    cy.extend_from_slice(&[0xd8, 0x65, 0x18]); // fsub [ebp+0x18]
    cy.extend_from_slice(&[0xd9, 0xe1]); // fabs
    cy.extend_from_slice(&[0xdd, 0x1d]);
    cy.extend_from_slice(&(cell as u32).to_le_bytes());
    let isly_end = isly + cy.len() as u64;
    emu.write(isly, &cy);
    // islands credited only when the bytes match source (fld+fsub+fabs = 8 bytes).
    assert_eq!(&emu.read(islx, 8), &emu.read(0x767cb6, 8), "islx != src");
    assert_eq!(&emu.read(isly, 8), &emu.read(0x767ccb, 8), "isly != src");

    // value diff: write field at [edi+disp_f], arg at [ebp+disp_a], run island, read f64 cell.
    let run_val =
        |emu: &mut Emu, isl: u64, end: u64, df: u64, da: u64, field: f32, arg: f32| -> f64 {
            emu.set_reg(RegisterX86::EDI, f.this);
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.write_u32(f.this + df, field.to_bits());
            emu.write_u32(f.ebp + da, arg.to_bits());
            emu.run(isl, end);
            read_f64(emu, cell)
        };
    // branch parity: in-place slice through the fnstsw. X-test halt 0x767cc6, Y-test halt 0x767cdb.
    let run_ah =
        |emu: &mut Emu, start: u64, end: u64, df: u64, da: u64, field: f32, arg: f32| -> u8 {
            emu.set_reg(RegisterX86::EDI, f.this);
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.write_u32(f.this + df, field.to_bits());
            emu.write_u32(f.ebp + da, arg.to_bits());
            emu.run(start, end);
            ah_of(emu)
        };

    let mut total = 0u32;
    let (mut within, mut outside) = (0u32, 0u32);
    // (island, island_end, field_disp, arg_disp, fnstsw_start, fnstsw_end)
    let cases: [(u64, u64, u64, u64, u64, u64); 2] = [
        (islx, islx_end, 0x4, 0x14, 0x767cb6, 0x767cc6),
        (isly, isly_end, 0x8, 0x18, 0x767ccb, 0x767cdb),
    ];
    // deterministic boundaries: |field-arg| just-below / exactly / just-above eps, equal, NaN.
    let det: [(f32, f32); 7] = [
        (5.0, 5.0),             // exact match → diff 0 < eps → within
        (5.0, 5.0 + eps),       // diff == eps → NOT < eps → outside
        (5.0, 5.0 + eps * 0.5), // below eps → within
        (5.0, 5.0 + eps * 2.0), // above eps → outside
        (0.0, 0.0),
        (1.0e6, -1.0e6), // large diff → outside
        (f32::NAN, 0.0), // NaN → outside
    ];
    for &(isl, iend, df, da, fs, fe) in &cases {
        for &(field, arg) in &det {
            // value (un-narrowed f64 |field-arg|)
            assert_eq!(
                run_val(&mut emu, isl, iend, df, da, field, arg).to_bits(),
                anchor_diff(field, arg).to_bits(),
                "anchor_diff isl={isl:#x} field={field} arg={arg}"
            );
            // branch parity. X test: jp taken (PF=1) iff EVEN ⇒ NOT within. Y test: jnp taken
            // (PF=0) iff ODD ⇒ within. We reduce both to "within" via parity of (AH & 5).
            let ah = run_ah(&mut emu, fs, fe, df, da, field, arg);
            let odd = (ah & 0x5).count_ones() % 2 == 1; // ODD ⇔ diff < eps ⇔ within
            assert_eq!(
                odd,
                anchor_unchanged(field, arg),
                "anchor_unchanged isl={isl:#x}"
            );
            if odd {
                within += 1;
            } else {
                outside += 1;
            }
            total += 2;
        }
        // fuzz: tight band around eps to hit both sides of the threshold + wide spread.
        let mut s = (fs as u32).wrapping_mul(0x9e37);
        for _ in 0..2500 {
            let base = rng(&mut s, -1000.0, 1000.0);
            let delta = rng(&mut s, -4.0 * eps, 4.0 * eps);
            let (field, arg) = (base, base + delta);
            assert_eq!(
                run_val(&mut emu, isl, iend, df, da, field, arg).to_bits(),
                anchor_diff(field, arg).to_bits(),
                "anchor_diff fuzz isl={isl:#x}"
            );
            let ah = run_ah(&mut emu, fs, fe, df, da, field, arg);
            let odd = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(
                odd,
                anchor_unchanged(field, arg),
                "anchor_unchanged fuzz isl={isl:#x}"
            );
            if odd {
                within += 1;
            } else {
                outside += 1;
            }
            total += 2;
        }
    }
    assert!(
        within > 0 && outside > 0,
        "767c70: need both branch outcomes"
    );
    eprintln!(
        "glue_767_767c70_anchor_change_detect: {total} bit-exact \
         (fsub×2+fabs×2; within={within} outside={outside})"
    );
}

// ===========================================================================================
// ROW 2 — 0x768360 [true_ret 0x7683cb ret 8] : reject-zero guard + 1× epsilon change-detect.
//   fsubr ×1 (0x76838a) + fabs ×1 (0x76838d) = 2 arithmetic (the 0x768363 fcomp 0.0 is compare-only).
//   COVERAGE: the island (fld [ecx+0x58]; fsubr [ebp+8]; fabs) covers both. Two ==probes: the
//   reject-zero guard (fcomp 0.0, halt at fnstsw 0x76836e) and the change-detect (halt 0x768397).
// ===========================================================================================
#[test]
fn glue_767_768360_rsub_change_detect() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let eps = f32::from_bits(EPS_BITS);

    // island = source 0x768387..0x76838f verbatim (fld [ecx+0x58]; fsubr [ebp+8]; fabs).
    let cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut c = vec![0xd9u8, 0x41, 0x58]; // fld   [ecx+0x58]
    c.extend_from_slice(&[0xd8, 0x6d, 0x08]); // fsubr [ebp+8]
    c.extend_from_slice(&[0xd9, 0xe1]); // fabs
    c.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [cell]
    c.extend_from_slice(&(cell as u32).to_le_bytes());
    let isl_end = isl + c.len() as u64;
    emu.write(isl, &c);
    assert_eq!(&emu.read(isl, 8), &emu.read(0x768387, 8), "isl != src");

    let run_val = |emu: &mut Emu, stored: f32, arg: f32| -> f64 {
        emu.set_reg(RegisterX86::ECX, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.this + 0x58, stored.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(isl, isl_end);
        read_f64(emu, cell)
    };
    // change-detect branch: fld [ecx+0x58]; fsubr [ebp+8]; fabs; fcomp [eps]; fnstsw → AH.
    let run_chg_ah = |emu: &mut Emu, stored: f32, arg: f32| -> u8 {
        emu.set_reg(RegisterX86::ECX, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.this + 0x58, stored.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(0x768387, 0x768397); // through the fnstsw at 0x768395
        ah_of(emu)
    };
    // reject-zero guard: fld [ebp+8]; fcomp [0.0]; fnstsw → AH (halt at test ah,0x44 = 0x76836e).
    let run_guard_ah = |emu: &mut Emu, arg: f32| -> u8 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(0x768363, 0x76836e);
        ah_of(emu)
    };

    let mut total = 0u32;
    let (mut within, mut outside) = (0u32, 0u32);
    let (mut accept, mut reject) = (0u32, 0u32);

    // reject-zero ==probe: jp (PF=1) taken ⇒ accept iff EVEN parity of (AH & 0x44); reject iff ODD.
    for arg in [0.0f32, -0.0, 1.0, -1.0, 1.0e-30, f32::NAN, f32::INFINITY] {
        let ah = run_guard_ah(&mut emu, arg);
        let even = (ah & 0x44).count_ones() % 2 == 0; // EVEN ⇔ jp taken ⇔ accept
        assert_eq!(even, accept_nonzero(arg), "accept_nonzero arg={arg}");
        if even {
            accept += 1;
        } else {
            reject += 1;
        }
        total += 1;
    }
    assert!(reject > 0 && accept > 0, "768360 guard: need both outcomes");

    // change-detect value + branch over the eps boundary.
    let det: [(f32, f32); 6] = [
        (3.0, 3.0),
        (3.0, 3.0 + eps),
        (3.0, 3.0 + eps * 0.5),
        (3.0, 3.0 + eps * 2.0),
        (-2.0, 2.0),
        (1.0, f32::NAN),
    ];
    for &(stored, arg) in &det {
        assert_eq!(
            run_val(&mut emu, stored, arg).to_bits(),
            rsub_diff(stored, arg).to_bits(),
            "rsub_diff stored={stored} arg={arg}"
        );
        let ah = run_chg_ah(&mut emu, stored, arg);
        let odd = (ah & 0x5).count_ones() % 2 == 1; // ODD ⇔ diff < eps ⇔ within
        assert_eq!(
            odd,
            rsub_unchanged(stored, arg),
            "rsub_unchanged stored={stored} arg={arg}"
        );
        if odd {
            within += 1;
        } else {
            outside += 1;
        }
        total += 2;
    }
    let mut s = 0x7683_6000u32;
    for _ in 0..3000 {
        let base = rng(&mut s, -500.0, 500.0);
        let delta = rng(&mut s, -4.0 * eps, 4.0 * eps);
        let (stored, arg) = (base, base + delta);
        assert_eq!(
            run_val(&mut emu, stored, arg).to_bits(),
            rsub_diff(stored, arg).to_bits(),
            "rsub_diff fuzz"
        );
        let ah = run_chg_ah(&mut emu, stored, arg);
        let odd = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(odd, rsub_unchanged(stored, arg), "rsub_unchanged fuzz");
        if odd {
            within += 1;
        } else {
            outside += 1;
        }
        total += 2;
    }
    assert!(
        within > 0 && outside > 0,
        "768360 change-detect: need both outcomes"
    );
    eprintln!(
        "glue_767_768360_rsub_change_detect: {total} bit-exact \
         (fsubr×1+fabs×1; accept={accept} reject={reject} within={within} outside={outside})"
    );
}

// ===========================================================================================
// ROW 3 — 0x76ac10 [true_ret 0x76ac4b ret 8] : 1× epsilon change-detect (reverse-subtract).
//   fsubr ×1 (0x76ac20) + fabs ×1 (0x76ac23) = 2 arithmetic.
//   COVERAGE: the island (fld [ecx+0xb8]; fsubr [ebp+8]; fabs) covers both; the in-place fnstsw
//   slice supplies the change-detect ==eps probe.
// ===========================================================================================
#[test]
fn glue_767_76ac10_rsub_change_detect() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let eps = f32::from_bits(EPS_BITS);

    // island = source 0x76ac1a..0x76ac25 verbatim (fld [ecx+0xb8]; fsubr [ebp+8]; fabs).
    // fld [ecx+0xb8] = D9 81 b8 00 00 00 (6 bytes); fsubr [ebp+8] = D8 6D 08 (3); fabs = D9 E1 (2).
    let cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut c = vec![0xd9u8, 0x81, 0xb8, 0x00, 0x00, 0x00]; // fld   [ecx+0xb8]
    c.extend_from_slice(&[0xd8, 0x6d, 0x08]); // fsubr [ebp+8]
    c.extend_from_slice(&[0xd9, 0xe1]); // fabs
    c.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [cell]
    c.extend_from_slice(&(cell as u32).to_le_bytes());
    let isl_end = isl + c.len() as u64;
    emu.write(isl, &c);
    assert_eq!(&emu.read(isl, 11), &emu.read(0x76ac1a, 11), "isl != src");

    let run_val = |emu: &mut Emu, stored: f32, arg: f32| -> f64 {
        emu.set_reg(RegisterX86::ECX, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.this + 0xb8, stored.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(isl, isl_end);
        read_f64(emu, cell)
    };
    let run_ah = |emu: &mut Emu, stored: f32, arg: f32| -> u8 {
        emu.set_reg(RegisterX86::ECX, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.this + 0xb8, stored.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(0x76ac1a, 0x76ac2d); // through the fnstsw at 0x76ac2b
        ah_of(emu)
    };

    let mut total = 0u32;
    let (mut within, mut outside) = (0u32, 0u32);
    let det: [(f32, f32); 6] = [
        (7.0, 7.0),
        (7.0, 7.0 + eps),
        (7.0, 7.0 + eps * 0.5),
        (7.0, 7.0 + eps * 2.0),
        (-3.0, 3.0),
        (1.0, f32::NAN),
    ];
    for &(stored, arg) in &det {
        assert_eq!(
            run_val(&mut emu, stored, arg).to_bits(),
            ac10_diff(stored, arg).to_bits(),
            "ac10_diff stored={stored} arg={arg}"
        );
        let ah = run_ah(&mut emu, stored, arg);
        let odd = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(
            odd,
            ac10_unchanged(stored, arg),
            "ac10_unchanged stored={stored} arg={arg}"
        );
        if odd {
            within += 1;
        } else {
            outside += 1;
        }
        total += 2;
    }
    let mut s = 0x76ac_1000u32;
    for _ in 0..3000 {
        let base = rng(&mut s, -800.0, 800.0);
        let delta = rng(&mut s, -4.0 * eps, 4.0 * eps);
        let (stored, arg) = (base, base + delta);
        assert_eq!(
            run_val(&mut emu, stored, arg).to_bits(),
            ac10_diff(stored, arg).to_bits(),
            "ac10_diff fuzz"
        );
        let ah = run_ah(&mut emu, stored, arg);
        let odd = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(odd, ac10_unchanged(stored, arg), "ac10_unchanged fuzz");
        if odd {
            within += 1;
        } else {
            outside += 1;
        }
        total += 2;
    }
    assert!(
        within > 0 && outside > 0,
        "76ac10 change-detect: need both outcomes"
    );
    eprintln!(
        "glue_767_76ac10_rsub_change_detect: {total} bit-exact \
         (fsubr×1+fabs×1; within={within} outside={outside})"
    );
}

// ===========================================================================================
// ROW 4 — 0x76f900 [true_ret 0x76f973 ret 0x10] : 4-channel colour pack.
//   fmul ×4 (0x76f907/91e/936/94f) + fadd ×4 (0x76f910/928/93e/95d) = 8 arithmetic.
//   COVERAGE: whole-fn Emu::call (ecx=out scratch, 4 f32 args) — body touches only [ebp+args],
//   [esi]=[ecx], and the native __ftol; covers all 8 ops by construction. Read [ecx] = packed u32.
// ===========================================================================================
#[test]
fn glue_767_76f900_color_pack() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out = emu.alloc(0x10);

    let run = |emu: &mut Emu, a0: f32, a1: f32, a2: f32, a3: f32| -> u32 {
        emu.write_u32(out, 0); // clear destination
                               // __thiscall: ecx = out ptr (mov esi,ecx; mov [esi],ebx). 4 f32 stack args; ret 0x10.
        emu.call(
            0x76f900,
            out as u32,
            0,
            &[a0.to_bits(), a1.to_bits(), a2.to_bits(), a3.to_bits()],
        );
        emu.read_u32(out)
    };

    let mut total = 0u32;
    // deterministic: clamp edges (0/1), the .5 round pivots, out-of-range, NaN low byte 0x00.
    let det: [(f32, f32, f32, f32); 7] = [
        (0.0, 1.0, 0.5, 0.25),
        (1.0, 1.0, 1.0, 1.0),
        (0.0, 0.0, 0.0, 0.0),
        // ·255+0.5 lands exactly on an integer boundary for each channel:
        (0.5 / 255.0, 127.5 / 255.0, 254.5 / 255.0, 1.5 / 255.0),
        (-1.0, 2.0, -0.5, 1.5), // out of [0,1] — no clamp gates here, so __ftol of the raw product
        (f32::NAN, 0.5, f32::NAN, 0.5),
        (100.0, -100.0, 1.0e9, -1.0e9), // wide — exercises full i32 __ftol, low byte kept
    ];
    for &(a0, a1, a2, a3) in &det {
        assert_eq!(
            run(&mut emu, a0, a1, a2, a3),
            pack4(a0, a1, a2, a3),
            "pack4 ({a0},{a1},{a2},{a3})"
        );
        // single-channel kernel against the packed byte positions.
        let got = run(&mut emu, a0, a1, a2, a3);
        assert_eq!(u32::from(channel(a0)), got >> 24, "ch0");
        assert_eq!(u32::from(channel(a1)), (got >> 16) & 0xff, "ch1");
        assert_eq!(u32::from(channel(a2)), (got >> 8) & 0xff, "ch2");
        assert_eq!(u32::from(channel(a3)), got & 0xff, "ch3");
        total += 1;
    }
    let mut s = 0x76f9_0000u32;
    for _ in 0..5000 {
        // span below 0, in [0,1], and above 1 to cover the full __ftol domain (no clamp gates).
        let a0 = rng(&mut s, -0.5, 1.5);
        let a1 = rng(&mut s, -0.5, 1.5);
        let a2 = rng(&mut s, -0.5, 1.5);
        let a3 = rng(&mut s, -0.5, 1.5);
        assert_eq!(
            run(&mut emu, a0, a1, a2, a3),
            pack4(a0, a1, a2, a3),
            "pack4 fuzz"
        );
        total += 1;
    }
    eprintln!("glue_767_76f900_color_pack: {total} bit-exact (fmul[255]×4 + fadd[0.5]×4 + __ftol×4 → packed u32)");
}

// ===========================================================================================
// ROW 5 — 0x783380 [true_ret 0x7833b0 ret] : guarded fraction (num>0 ? num/den : 0.0).
//   fsub ×2 (0x783386 num, 0x78339f den) + fdivrp ×1 (0x7833a5) = 3 arithmetic.
//   COVERAGE: whole-fn Emu::call_f64 (ecx=struct ptr, fields at +0x318/0x31c/0x320) — no-prologue
//   leaf, returns the quotient LIVE in st0 read back un-narrowed as f64; covers all 3 ops by
//   construction (the num<=0/NaN branch returns 0.0). ==probe: the numerator-sign pivot num>0.
// ===========================================================================================
#[test]
fn glue_767_783380_guarded_fraction() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x400);

    let run = |emu: &mut Emu, c318: f32, c31c: f32, c320: f32| -> f64 {
        emu.write_u32(this + 0x318, c318.to_bits());
        emu.write_u32(this + 0x31c, c31c.to_bits());
        emu.write_u32(this + 0x320, c320.to_bits());
        // __thiscall leaf returning float in st0 → call_f64 reads the un-narrowed PC_53 st0.
        emu.call_f64(0x783380, this as u32, 0, &[])
    };

    let mut total = 0u32;
    let (mut ratio, mut zero) = (0u32, 0u32);
    // result = top/gate where gate = c31c-c318 (sign-gated divisor), top = c320-c318 (dividend).
    // gate>0 → ratio; gate==0 (C3 pivot) / gate<0 (C0) / NaN → 0.0.
    let det: [(f32, f32, f32); 9] = [
        (0.0, 1.0, 2.0),      // gate=1>0, top=2 → 2.0
        (1.0, 3.0, 5.0),      // gate=2>0, top=4 → 2.0
        (0.0, 0.0, 1.0),      // gate=0 → 0.0 (pivot: jne taken on C3)
        (2.0, 1.0, 5.0),      // gate=-1<0 → 0.0
        (0.0, 1.0, 1.0),      // gate=1>0, top=1 → 1.0
        (0.0, 2.0, 1.0),      // gate=2>0, top=1 → 0.5
        (0.0, 1.0, 0.0),      // gate=1>0, top=0 → 0.0 (top zero, gate>0)
        (0.0, f32::NAN, 1.0), // gate=NaN → 0.0
        (10.0, 7.0, 3.0),     // gate=-3<0 → 0.0
    ];
    for &(c318, c31c, c320) in &det {
        let got = run(&mut emu, c318, c31c, c320).to_bits();
        let want = fraction(c318, c31c, c320).to_bits();
        assert_eq!(got, want, "fraction c318={c318} c31c={c31c} c320={c320}");
        if (f64::from(c31c) - f64::from(c318)) > 0.0 {
            ratio += 1;
        } else {
            zero += 1;
        }
        total += 1;
    }
    assert!(
        ratio > 0 && zero > 0,
        "783380: need both gate>0 and gate<=0 outcomes"
    );
    let mut s = 0x7833_8000u32;
    for _ in 0..5000 {
        let c318 = rng(&mut s, -1000.0, 1000.0);
        // bias c31c around c318 so num straddles 0 (hit both branches of the sign pivot).
        let c31c = c318 + rng(&mut s, -50.0, 50.0);
        let c320 = rng(&mut s, -1000.0, 1000.0);
        let got = run(&mut emu, c318, c31c, c320).to_bits();
        let want = fraction(c318, c31c, c320).to_bits();
        assert_eq!(
            got, want,
            "fraction fuzz c318={c318} c31c={c31c} c320={c320}"
        );
        if (f64::from(c31c) - f64::from(c318)) > 0.0 {
            ratio += 1;
        } else {
            zero += 1;
        }
        total += 1;
    }
    eprintln!(
        "glue_767_783380_guarded_fraction: {total} bit-exact \
         (fsub×2+fdivrp×1; ratio={ratio} zero={zero})"
    );
}
