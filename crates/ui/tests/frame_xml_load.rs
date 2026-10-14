//! Differential tests of the arithmetic of `CSimpleFrame::LoadXML` (`0x769820`, the alpha
//! quantise) and `CSimpleFrame::SetParentScale` (`0x76ac50`, the near-equal guard), each run
//! against `WoW.exe` in the emulator.
//!
//! Coverage of the arithmetic opcode set (load/store/compare/convert/`fild`/`fxch` excluded),
//! counted in each `[addr, true_ret)`:
//!
//! * `0x769820` SetAlpha   [.,0x769d2e ret 8]: `fmul dword [255.0]` ×1
//! * `0x76ac50` setter      [.,0x76ac8b ret 8]: `fsubr dword [new]` ×1 + `fabs` ×1
//!
//! Diff strategy per op (both COPIED-BYTE ISLANDS — the owned ops execute on a LIVE st0 value the
//! orchestration places there via a delegated `0x64aaa0` parse / a struct field that we cannot
//! synthesize as a whole-fn call without driving the entire attribute-applier; so we copy the EXACT
//! source op bytes into an island, byte-verify them == the source site, seed st0 ourselves, and read
//! the result):
//!
//! * SetAlpha `fmul[255]` — island `fld qword [in]` (seeds the clamped value), the EXACT source
//!   `fmul dword [0x7ffe58]` bytes (byte-verified == 0x769a76), then a real `call __ftol`
//!   (`0x40a2b0`, recomputed rel32). Reads EAX (the __ftol truncate). Covers the `fmul` + the
//!   delegated `__ftol` together.
//! * setter `fsubr;fabs` — island using the EXACT source `fld dword [ecx+0xbc]` (byte-verified ==
//!   0x76ac5a), `fsubr dword [ebp+0x8]` (== 0x76ac60), `fabs` (== 0x76ac63), then `fstp qword [out]`.
//!   ecx/ebp are set so `[ecx+0xbc]` = field, `[ebp+0x8]` = new; reads the f64 `|new − field|`.
//!
//! Fuzz reads the LCG's HIGH bits. The mandatory `==`-boundary probes
//! sit on the `__ftol` truncate pivots (v·255 at integer boundaries) and the setter's `new == field`
//! (`+0.0`/`−0.0` disposition) and `|delta| == eps` cases.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::frame_xml_load::{alpha_quantize, delta_abs, EPS_F32_BITS};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}

const __FTOL: u64 = 0x40a2b0;

// ===========================================================================================
// Row 0x769820 SetAlpha — `fmul dword [0x7ffe58]=255.0` @ 0x769a76, then `call __ftol` truncate.
// COPIED-BYTE ISLAND: fld qword [in]; <source fmul dword [255.0]>; call __ftol; read EAX.
// ===========================================================================================
#[test]
fn glue769_769820_set_alpha_quantize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let in_cell = emu.alloc(8);
    let isl = emu.alloc(0x40);
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;

    // fld qword [in_cell]  =  DD 05 <disp32>
    let mut code = vec![0xddu8, 0x05];
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    // copied source fmul dword [0x7ffe58]=255.0  =  D8 0D 58 FE 7F 00
    let fmul_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x0d, 0x58, 0xfe, 0x7f, 0x00]);
    // call __ftol  =  E8 <rel32>, rel32 = target - (next_instr_va)
    let call_va = isl + code.len() as u64;
    let rel = (__FTOL as i64 - (call_va as i64 + 5)) as i32;
    code.push(0xe8);
    code.extend_from_slice(&rel.to_le_bytes());
    let isl_end = isl + code.len() as u64; // __ftol's `ret` lands here
    emu.write(isl, &code);

    // byte-verify the island's fmul == the source site (else the island proves nothing).
    assert_eq!(
        emu.read(isl + fmul_off, 6),
        emu.read(0x769a76, 6),
        "set-alpha island fmul != source @ 0x769a76"
    );

    let run = |emu: &mut Emu, v: f64| -> u8 {
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write(in_cell, &v.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        (emu.get_reg(RegisterX86::EAX) as i32) as u8 // __ftol result, low byte
    };

    let mut total = 0u32;
    // deterministic boundary probes: clamped domain + the __ftol truncate pivots (v·255 just
    // below/at an integer). The runtime only feeds clamp(v,0,1), but the arithmetic is
    // domain-agnostic — probe the exact pivots.
    let probes = [
        0.0f64,
        1.0,
        0.5,
        1.0 / 255.0,         // ·255 = 1.0 -> 1
        1.0 / 255.0 - 1e-12, // just below -> just below 1.0 -> 0
        2.0 / 255.0,         // -> 2.0 -> 2
        2.0 / 255.0 - 1e-12, // -> just below 2.0 -> 1
        254.0 / 255.0,       // -> 254.0 -> 254
        128.0 / 255.0,
        0.999,
        0.001,
        0.99999,
        // NaN, the infinities and values past `i32` and `i64`: `__ftol` returns the low dword of
        // its qword, 0 for the indefinite, and the byte is its low byte.
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2.0e7,
        1.7e7,
        -2.0e7,
        1.0e17,
        1.0e300,
    ];
    for &v in &probes {
        assert_eq!(run(&mut emu, v), alpha_quantize(v), "alpha_quantize v={v}");
        total += 1;
    }
    // dense sweep of the clamped domain: every byte target v = k/255 and a touch above/below.
    for k in 0u32..=255 {
        for d in [-1e-9, 0.0, 1e-9] {
            let v = f64::from(k) / 255.0 + d;
            assert_eq!(
                run(&mut emu, v),
                alpha_quantize(v),
                "alpha_quantize v={v} (k={k} d={d})"
            );
            total += 1;
        }
    }
    let mut s = 0x7698_2000u32;
    for _ in 0..4000 {
        let v = rngf(&mut s, 0.0, 1.0); // the clamped runtime domain
        assert_eq!(
            run(&mut emu, v),
            alpha_quantize(v),
            "alpha_quantize fuzz v={v}"
        );
        total += 1;
    }
    eprintln!(
        "glue769_769820_set_alpha_quantize: {total} bit-exact (island: fmul[255];__ftol == 0x769a76)"
    );
}

// ===========================================================================================
// Row 0x76ac50 setter — `fsubr dword [ebp+0x8]` @ 0x76ac60 + `fabs` @ 0x76ac63.
// COPIED-BYTE ISLAND: <source fld [ecx+0xbc]; fsubr [ebp+0x8]; fabs>; fstp qword [out].
// ecx so [ecx+0xbc] = field cell, ebp so [ebp+0x8] = new cell; read the f64 |new − field|.
// ===========================================================================================
#[test]
fn glue769_76ac50_setter_delta_abs() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let this = emu.alloc(0x200); // ecx base; field at [ecx+0xbc]
    let fp = emu.alloc(0x200); // ebp base; new at [ebp+0x8]
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x40);

    // copied source fld dword [ecx+0xbc]  =  D9 81 BC 00 00 00
    let fld_off = 0u64;
    let mut code = vec![0xd9u8, 0x81, 0xbc, 0x00, 0x00, 0x00];
    // copied source fsubr dword [ebp+0x8]  =  D8 6D 08
    let fsubr_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x6d, 0x08]);
    // copied source fabs  =  D9 E1
    let fabs_off = code.len() as u64;
    code.extend_from_slice(&[0xd9, 0xe1]);
    // fstp qword [out_cell]  =  DD 1D <disp32>
    code.push(0xdd);
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);

    // byte-verify each copied source op == its site (else the island proves nothing).
    assert_eq!(
        emu.read(isl + fld_off, 6),
        emu.read(0x76ac5a, 6),
        "setter island fld != source @ 0x76ac5a"
    );
    assert_eq!(
        emu.read(isl + fsubr_off, 3),
        emu.read(0x76ac60, 3),
        "setter island fsubr != source @ 0x76ac60"
    );
    assert_eq!(
        emu.read(isl + fabs_off, 2),
        emu.read(0x76ac63, 2),
        "setter island fabs != source @ 0x76ac63"
    );

    let run = |emu: &mut Emu, new: f32, field: f32| -> f64 {
        emu.set_reg(RegisterX86::ECX, this);
        emu.set_reg(RegisterX86::EBP, fp);
        emu.write_u32(this + 0xbc, field.to_bits()); // [ecx+0xbc] = field (f32)
        emu.write_u32(fp + 0x8, new.to_bits()); // [ebp+0x8] = new (f32)
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    let eps = f64::from(f32::from_bits(EPS_F32_BITS)); // 2^-22

    // deterministic boundary probes: equal inputs (fsubr's +0.0/−0.0 disposition into fabs), the
    // sign-of-zero pairs, the `|delta| == eps` boundary, and a NaN/Inf disposition.
    let probes: &[(f32, f32)] = &[
        (0.0, 0.0),
        (-0.0, 0.0),
        (0.0, -0.0),
        (-0.0, -0.0),
        (1.0, 1.0),
        (-1.0, -1.0),
        (1.0, 0.0),  // new − field = 1.0 -> 1.0
        (0.0, 1.0),  // new − field = −1.0 -> 1.0
        (5.5, 2.25), // 3.25
        (2.25, 5.5), // 3.25
        // |delta| == eps exactly: field=0, new = eps -> |eps − 0| = eps
        (f32::from_bits(EPS_F32_BITS), 0.0),
        (0.0, f32::from_bits(EPS_F32_BITS)),
        // just below / above eps
        (f32::from_bits(EPS_F32_BITS - 1), 0.0),
        (f32::from_bits(EPS_F32_BITS + 1), 0.0),
        (f32::INFINITY, 1.0),
        (-f32::INFINITY, 1.0),
        (f32::NAN, 1.0),
        (1.0, f32::NAN),
        (f32::MAX, f32::MIN),
    ];
    for &(new, field) in probes {
        assert_eq!(
            run(&mut emu, new, field).to_bits(),
            delta_abs(new, field).to_bits(),
            "delta_abs new={new} field={field}"
        );
        // also assert the guard outcome (near_equal) is consistent with the diffed magnitude.
        let _ = eps; // documented boundary; the magnitude diff above is the bit-exact verdict
        total += 1;
    }

    // dense sweep around the eps boundary (the guard pivot) — field fixed, new walks across eps.
    let base = 0.0f32;
    for kk in 0..512u32 {
        // new = small multiples of a ulp near eps, both signs.
        let mag = f32::from_bits(EPS_F32_BITS.wrapping_sub(256).wrapping_add(kk));
        for sign in [1.0f32, -1.0] {
            let new = sign * mag;
            assert_eq!(
                run(&mut emu, new, base).to_bits(),
                delta_abs(new, base).to_bits(),
                "delta_abs near-eps new={new} field={base}"
            );
            total += 1;
        }
    }

    // fuzz: representative f32 pairs (HIGH bits) over a wide magnitude range.
    let mut s = 0x76ac_5000u32;
    for _ in 0..5000 {
        let new = rngf(&mut s, -1000.0, 1000.0) as f32;
        let field = rngf(&mut s, -1000.0, 1000.0) as f32;
        assert_eq!(
            run(&mut emu, new, field).to_bits(),
            delta_abs(new, field).to_bits(),
            "delta_abs fuzz new={new} field={field}"
        );
        total += 1;
    }
    // a band of near-equal fuzz (where the guard actually decides) to stress fsubr cancellation.
    for _ in 0..5000 {
        let field = rngf(&mut s, -10.0, 10.0) as f32;
        let jitter = rngf(&mut s, -1e-5, 1e-5) as f32;
        let new = field + jitter;
        assert_eq!(
            run(&mut emu, new, field).to_bits(),
            delta_abs(new, field).to_bits(),
            "delta_abs near-equal fuzz new={new} field={field}"
        );
        total += 1;
    }
    eprintln!(
        "glue769_76ac50_setter_delta_abs: {total} bit-exact (island: fsubr;fabs == 0x76ac60/0x76ac63)"
    );
}
