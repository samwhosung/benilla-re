//! Differential tests of the arithmetic of the font-string functions from `0x770570` to
//! `CSimpleFontString::SetTextHeight` (`0x771600`), including `CSimpleFontString::LoadXML`
//! (`0x770f40`), each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice of the FP-bearing glue TU `[0x770570, 0x771600]` is diffed against the
//! binary's own bytes: registers/memory/stack synthesized, the stored `fstp [m32]` value or the
//! `fnstsw` AH read back. Every in-body FP-arith opcode (the arithmetic opcode set:
//! fmul/fsub/fsubr/fdivr/fabs/fmulp/fadd) lands in a diffed slice/island/whole-fn call:
//!   * 0x770570 (10): whole-fn `Emu::call` (two lerp blocks, body covered by construction).
//!   * 0x770720/0x770790/0x770f40 (2 each): one byte-identical-clone island (`fmul[1024];fdivr`).
//!   * 0x770910/0x770ad0 (4 each): two eps change-detect slices each (mem-`fsubr`, and the clone
//!     `fsubr st,st(1)` block — itself a clone across the two functions, one diff covers both).
//!   * 0x7714f0 (3): eps change-detect slice + the `fmul[esi+0x7c]` scale island.
//!   * 0x771600 (2): eps change-detect slice.
//!
//! Fuzz reads the LCG's HIGH bits. Every owned-slice predicate test
//! carries a deterministic `==eps` boundary probe. The eps change-detect branch is
//! reconstructed from the EMITTED jcc: `test ah,5` (C0|C2) read by `jp`/`jnp` is a PARITY test —
//! odd parity ⟺ ordered `|d| < eps`.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::fontstring_xml::{
    cvar_unit_convert, lerp_by_abs_delta, scale_mul, settled_eps, settled_eps_st, EPS_BITS,
    SCALE1024_BITS,
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
/// `test ah,5; jp` parity: odd set-bits in {C0,C2} ⟺ ordered `|d| < eps`. This is `settled`.
fn parity_odd(ah: u8) -> bool {
    (ah & 0x5).count_ones() % 2 == 1
}

const EPS_F32: f32 = f32::from_bits(0x3480_0000);

// ===========================================================================================
// 0x770570 — lerp-by-|delta|: two blocks, [eax+0xc] = |a-b|·(hi-lo)+lo and the +0x8/0x0 twin.
// Whole-fn `Emu::call` (__thiscall ecx=this, one stack arg = the `eax` ptr, ret 4): both blocks
// run by construction. fsub×2+fabs×2+fmulp×2+fadd×2 = 10 FP-arith, all covered.
// ===========================================================================================
#[test]
fn glue_770570_lerp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200); // ecx: holds [+0x104..+0x114]
    let blk = emu.alloc(0x40); // eax (the [ebp+8] arg): holds [+0x0..+0xc]

    // block1: a=[ecx+0x114] b=[ecx+0x104] hi=[eax+0xc] lo=[eax+0x4] -> store [eax+0xc].
    // block2: a=[ecx+0x110] b=[ecx+0x108] hi=[eax+0x8] lo=[eax]     -> store [eax+0x8].
    let run = |emu: &mut Emu,
               a1: f32,
               b1: f32,
               hi1: f32,
               lo1: f32,
               a2: f32,
               b2: f32,
               hi2: f32,
               lo2: f32|
     -> (f32, f32) {
        emu.write_u32(this + 0x114, a1.to_bits());
        emu.write_u32(this + 0x104, b1.to_bits());
        emu.write_u32(this + 0x110, a2.to_bits());
        emu.write_u32(this + 0x108, b2.to_bits());
        emu.write_u32(blk + 0xc, hi1.to_bits());
        emu.write_u32(blk + 0x4, lo1.to_bits());
        emu.write_u32(blk + 0x8, hi2.to_bits());
        emu.write_u32(blk, lo2.to_bits());
        emu.call(0x770570, this as u32, 0, &[blk as u32]);
        (emu.read_f32(blk + 0xc), emu.read_f32(blk + 0x8))
    };

    let mut total = 0u32;
    // deterministic cases incl. the |a-b|==0 boundary (frac exactly 0 ⇒ result == lo).
    for (a, b, hi, lo) in [
        (1.0f32, 1.0f32, 5.0f32, 2.0f32), // |a-b|=0 -> result = lo exactly
        (3.0, 1.0, 5.0, 2.0),             // frac=2.0
        (1.0, 3.0, 5.0, 2.0),             // |a-b|=2.0 (fabs)
        (1.5, 1.0, 10.0, -10.0),
        (0.0, 0.0, 0.0, 0.0),
    ] {
        let (r1, r2) = run(&mut emu, a, b, hi, lo, a, b, hi, lo);
        assert_eq!(
            r1.to_bits(),
            lerp_by_abs_delta(a, b, hi, lo).to_bits(),
            "blk1 a={a} b={b} hi={hi} lo={lo}"
        );
        assert_eq!(
            r2.to_bits(),
            lerp_by_abs_delta(a, b, hi, lo).to_bits(),
            "blk2 a={a} b={b} hi={hi} lo={lo}"
        );
        total += 2;
    }
    let mut s = 0x7705_7000u32;
    for _ in 0..3000 {
        let a1 = rng(&mut s, -100.0, 100.0);
        let b1 = rng(&mut s, -100.0, 100.0);
        let hi1 = rng(&mut s, -100.0, 100.0);
        let lo1 = rng(&mut s, -100.0, 100.0);
        let a2 = rng(&mut s, -100.0, 100.0);
        let b2 = rng(&mut s, -100.0, 100.0);
        let hi2 = rng(&mut s, -100.0, 100.0);
        let lo2 = rng(&mut s, -100.0, 100.0);
        let (r1, r2) = run(&mut emu, a1, b1, hi1, lo1, a2, b2, hi2, lo2);
        assert_eq!(
            r1.to_bits(),
            lerp_by_abs_delta(a1, b1, hi1, lo1).to_bits(),
            "blk1 fuzz"
        );
        assert_eq!(
            r2.to_bits(),
            lerp_by_abs_delta(a2, b2, hi2, lo2).to_bits(),
            "blk2 fuzz"
        );
        total += 2;
    }
    eprintln!("glue_770570_lerp: {total} bit-exact (two |a-b|·(hi-lo)+lo blocks, whole-fn call)");
}

// ===========================================================================================
// 0x770720 / 0x770790 / 0x770f40 — cvar unit-convert kernel: fmul[1024.0]; fdivr [dividend].
// Byte-identical clone (verified: the only diff is the fdivr stack-disp, which here points at our
// own dividend cell). One COPIED-BYTE ISLAND covers the family: an `fld [scale]` prologue puts the
// scale getter's return on st0, then the verbatim kernel bytes run. fmul×1 + fdivr×1.
// ===========================================================================================
#[test]
fn glue_unit_convert_1024() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scale_cell = emu.alloc(4);
    let div_cell = emu.alloc(4);
    let res_cell = emu.alloc(4);

    // island = `fld [scale_cell]` + verbatim 0x77076b kernel (fmul[0x7ffd68]; fdivr [div_cell];
    // fstp [res_cell]). The fmul bytes are byte-verified vs source: d8 0d 68 fd 7f 00. The fdivr
    // opcode d8 7d ?? (memory form, mod=01) is reproduced with a disp32 to our cell via d8 3d.
    let region = emu.alloc(0x40);
    let mut c = vec![0xd9u8, 0x05]; // fld dword [scale_cell]
    c.extend_from_slice(&(scale_cell as u32).to_le_bytes());
    c.extend_from_slice(&[0xd8, 0x0d, 0x68, 0xfd, 0x7f, 0x00]); // fmul [0x7ffd68]=1024.0  (0x77076b, verbatim)
    c.extend_from_slice(&[0xd8, 0x3d]); // fdivr dword [disp32]  (the divr form: st0 = [m] / st0)
    c.extend_from_slice(&(div_cell as u32).to_le_bytes());
    c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [disp32]
    c.extend_from_slice(&(res_cell as u32).to_le_bytes());
    let end = region + c.len() as u64;
    emu.write(region, &c);

    let run = |emu: &mut Emu, dividend: f32, scale: f32| -> f32 {
        emu.write_u32(scale_cell, scale.to_bits());
        emu.write_u32(div_cell, dividend.to_bits());
        emu.set_reg(RegisterX86::ESP, 0x0020_0000); // any mapped stack (island has no push)
        emu.run(region, end);
        emu.read_f32(res_cell)
    };

    let mut total = 0u32;
    for (dividend, scale) in [
        (1024.0f32, 1.0f32), // -> 1.0 / 1.0 = 1.0 exactly
        (2048.0, 2.0),
        (-1024.0, 0.5),
        (100.0, 0.25),
        (0.0, 1.0),
        (1.0, 1.0), // 1 / 1024 (a clean exact-ish boundary)
    ] {
        assert_eq!(
            run(&mut emu, dividend, scale).to_bits(),
            cvar_unit_convert(dividend, scale).to_bits(),
            "uconv div={dividend} scale={scale}"
        );
        total += 1;
    }
    let mut s = 0x7707_2000u32;
    for _ in 0..3000 {
        let dividend = rng(&mut s, -5000.0, 5000.0);
        let scale = rng(&mut s, 0.01, 50.0);
        assert_eq!(
            run(&mut emu, dividend, scale).to_bits(),
            cvar_unit_convert(dividend, scale).to_bits(),
            "uconv fuzz div={dividend} scale={scale}"
        );
        total += 1;
    }
    let _ = SCALE1024_BITS;
    eprintln!("glue_unit_convert_1024: {total} bit-exact (0x770720/0x770790/0x770f40 fmul[1024]+fdivr clone)");
}

// ===========================================================================================
// eps change-detect predicate — fld[field]; fsubr[other]; fabs; fcomp[0x8029d4]; fnstsw.
// Diffed at every site (memory-fsubr at 0x770910/0x770ad0/0x7714f0/0x771600; fsubr st,st(1) at
// 0x770a84/0x770c1a). Each slice ends at the fnstsw; the AH parity is reconstructed and matched.
// ===========================================================================================
#[test]
fn glue_settled_eps() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200); // esi
    let stack = emu.alloc(0x1000);
    let ebp = stack + 0x800;

    // Each memory-fsubr site: (slice_start, slice_end, field_off, other_loc).
    // field is [esi+field_off]; other is either [ebp+disp] (arg) — we set both directly.
    // 0x770910: fld[esi+0x10]; fsubr[ebp+0xc]   -> end 0x77098c
    // 0x770ad0: fld[esi+0x10]; fsubr[ebp+0x8]   -> end 0x770b39
    // 0x7714f0: fld[esi+0xe4]; fsubr[ebp+0xc]   -> end 0x771532
    // 0x771600: fld[esi+0xe4]; fsubr[ebp+0x8]   -> end 0x771619
    struct Site {
        start: u64,
        end: u64,
        field_off: u64,
        other_disp: i64,
        name: &'static str,
    }
    let sites = [
        Site {
            start: 0x77097c,
            end: 0x77098c,
            field_off: 0x10,
            other_disp: 0xc,
            name: "0x770910",
        },
        Site {
            start: 0x770b29,
            end: 0x770b39,
            field_off: 0x10,
            other_disp: 0x8,
            name: "0x770ad0",
        },
        Site {
            start: 0x77151f,
            end: 0x771532,
            field_off: 0xe4,
            other_disp: 0xc,
            name: "0x7714f0",
        },
        Site {
            start: 0x771606,
            end: 0x771619,
            field_off: 0xe4,
            other_disp: 0x8,
            name: "0x771600",
        },
    ];

    let run_site = |emu: &mut Emu, s: &Site, field: f32, other: f32| -> u8 {
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::ECX, this);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x100);
        emu.write_u32(this + s.field_off, field.to_bits());
        emu.write_u32((ebp as i64 + s.other_disp) as u64, other.to_bits());
        emu.run(s.start, s.end);
        ah_of(emu)
    };

    let mut total = 0u32;
    let mut settled_n = 0u32;
    let mut changed_n = 0u32;
    let _ = EPS_BITS;
    for st in &sites {
        // deterministic: |other-field| == eps exactly (the inclusive/strict boundary).
        for (field, other, _l) in [
            (0.0f32, EPS_F32, "==eps"),
            (0.0, EPS_F32 * 0.5, "below"),
            (0.0, EPS_F32 * 2.0, "above"),
            (5.0, 5.0, "equal"),
            (0.0, f32::NAN, "nan"),
        ] {
            let ah = run_site(&mut emu, st, field, other);
            let bin = parity_odd(ah);
            assert_eq!(
                bin,
                settled_eps(field, other),
                "{} field={field} other={other}",
                st.name
            );
            if bin {
                settled_n += 1;
            } else {
                changed_n += 1;
            }
            total += 1;
        }
        let mut s = 0x7700_0000u32 ^ (st.start as u32);
        for _ in 0..800 {
            let field = rng(&mut s, -10.0, 10.0);
            let other = field + rng(&mut s, -1.0e-6, 1.0e-6);
            let ah = run_site(&mut emu, st, field, other);
            let bin = parity_odd(ah);
            assert_eq!(bin, settled_eps(field, other), "{} fuzz", st.name);
            if bin {
                settled_n += 1;
            } else {
                changed_n += 1;
            }
            total += 1;
        }
    }

    // The two-register `fsubr st,st(1)` block (0x770a84 in 0x770910 / 0x770c1a in 0x770ad0 —
    // byte-identical clone, one diff covers both): fld[edi+0x18]; fld[esi+0x18]; fsubr st,st(1);
    // fabs; fcomp[eps]; fnstsw. st1=[edi+0x18]=src, st0=[esi+0x18]=dst; fsubr st,st(1)=src-dst.
    let dst_obj = emu.alloc(0x40); // esi
    let src_obj = emu.alloc(0x40); // edi
    let run_st = |emu: &mut Emu, start: u64, end: u64, dst: f32, src: f32| -> u8 {
        emu.set_reg(RegisterX86::ESI, dst_obj);
        emu.set_reg(RegisterX86::EDI, src_obj);
        emu.set_reg(RegisterX86::ESP, stack + 0x100);
        emu.write_u32(dst_obj + 0x18, dst.to_bits());
        emu.write_u32(src_obj + 0x18, src.to_bits());
        emu.run(start, end);
        ah_of(emu)
    };
    for &(start, end, name) in &[
        (0x770a84u64, 0x770a96u64, "0x770910.st"),
        (0x770c1a, 0x770c2c, "0x770ad0.st"),
    ] {
        for (dst, src, _l) in [
            (0.0f32, EPS_F32, "==eps"),
            (0.0, EPS_F32 * 0.5, "below"),
            (0.0, EPS_F32 * 2.0, "above"),
            (7.0, 7.0, "equal"),
            (0.0, f32::NAN, "nan"),
        ] {
            let ah = run_st(&mut emu, start, end, dst, src);
            let bin = parity_odd(ah);
            assert_eq!(bin, settled_eps_st(dst, src), "{name} dst={dst} src={src}");
            if bin {
                settled_n += 1;
            } else {
                changed_n += 1;
            }
            total += 1;
        }
        let mut s = 0x7700_a840u32 ^ (start as u32);
        for _ in 0..800 {
            let dst = rng(&mut s, -10.0, 10.0);
            let src = dst + rng(&mut s, -1.0e-6, 1.0e-6);
            let ah = run_st(&mut emu, start, end, dst, src);
            let bin = parity_odd(ah);
            assert_eq!(bin, settled_eps_st(dst, src), "{name} fuzz");
            if bin {
                settled_n += 1;
            } else {
                changed_n += 1;
            }
            total += 1;
        }
    }

    assert!(
        settled_n > 0 && changed_n > 0,
        "need both settled/changed outcomes"
    );
    eprintln!("glue_settled_eps: {total} bit-exact (settled={settled_n} changed={changed_n}; 4 mem-fsubr + 2 st-fsubr sites)");
}

// ===========================================================================================
// 0x7714f0 — scale-multiply: fld[ebp+0xc]; fmul[esi+0x7c]; fstp[esp]. fmul×1.
// Copied-byte island: `fld [arg_cell]` + verbatim fmul[esi+0x7c] + fstp [res_cell].
// ===========================================================================================
#[test]
fn glue_7714f0_scale_mul() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x100); // esi (field at +0x7c)
    let arg_cell = emu.alloc(4);
    let res_cell = emu.alloc(4);

    // island: fld [arg_cell]; fmul [esi+0x7c] (verbatim d8 4e 7c); fstp [res_cell].
    let region = emu.alloc(0x40);
    let mut c = vec![0xd9u8, 0x05]; // fld dword [arg_cell]
    c.extend_from_slice(&(arg_cell as u32).to_le_bytes());
    c.extend_from_slice(&[0xd8, 0x4e, 0x7c]); // fmul dword [esi+0x7c]  (0x771563, verbatim)
    c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [disp32]
    c.extend_from_slice(&(res_cell as u32).to_le_bytes());
    let end = region + c.len() as u64;
    emu.write(region, &c);

    let run = |emu: &mut Emu, arg: f32, field: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::ESP, 0x0020_0000);
        emu.write_u32(arg_cell, arg.to_bits());
        emu.write_u32(this + 0x7c, field.to_bits());
        emu.run(region, end);
        emu.read_f32(res_cell)
    };

    let mut total = 0u32;
    for (arg, field) in [
        (1.0f32, 1.0f32),
        (2.0, 3.0),
        (-1.5, 4.0),
        (0.0, 100.0),
        (16_777_216.0, 1.0000001),
    ] {
        assert_eq!(
            run(&mut emu, arg, field).to_bits(),
            scale_mul(arg, field).to_bits(),
            "scale arg={arg} field={field}"
        );
        total += 1;
    }
    let mut s = 0x7714_f000u32;
    for _ in 0..3000 {
        let arg = rng(&mut s, -1000.0, 1000.0);
        let field = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run(&mut emu, arg, field).to_bits(),
            scale_mul(arg, field).to_bits(),
            "scale fuzz"
        );
        total += 1;
    }
    eprintln!("glue_7714f0_scale_mul: {total} bit-exact (0x7714f0 fmul[esi+0x7c])");
}
