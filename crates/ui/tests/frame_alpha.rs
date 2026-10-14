//! `Frame::SetAlpha` (`0x774e90`) against the exe: the slice `0x774f32` to `0x774f64` between
//! `lua_tonumber` and `__ftol` clamps the Lua number to `[0, 1]` and multiplies it by 255.0.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::frame_methods as frameapi_alpha;

const SLICE_BEG: u64 = 0x774f32;
const SLICE_END: u64 = 0x774f64;

/// Build the SetAlpha clamp+scale island once; return `(run, in_cell, out_cell)`.
fn build(emu: &mut Emu) -> (u64, u64, u64) {
    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x40);

    // `fld qword [in_cell]` = DD /0, modrm 0x05 (disp32 absolute) + disp32.
    let mut code: Vec<u8> = vec![0xdd, 0x05];
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    let body_off = code.len();
    // verbatim clamp+scale slice.
    let src = emu.read(SLICE_BEG, (SLICE_END - SLICE_BEG) as usize);
    code.extend_from_slice(&src);
    // `fstp qword [out_cell]` = DD /3, modrm 0x1d (disp32 absolute) + disp32.
    code.extend_from_slice(&[0xdd, 0x1d]);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());

    emu.write(isl, &code);
    // byte-verify the island's FP region == source (no circular self-verification).
    assert_eq!(
        &emu.read(isl + body_off as u64, src.len()),
        &emu.read(SLICE_BEG, src.len()),
        "SetAlpha island bytes != src [{SLICE_BEG:#x},{SLICE_END:#x})"
    );
    (isl, in_cell, out_cell)
}

/// Run the island on input double `x`, return the live `st0` product as `f64`.
fn run(emu: &mut Emu, isl: u64, isl_end: u64, in_cell: u64, out_cell: u64, x: f64) -> f64 {
    emu.write_u32(in_cell, (x.to_bits() & 0xffff_ffff) as u32);
    emu.write_u32(in_cell + 4, (x.to_bits() >> 32) as u32);
    emu.run(isl, isl_end);
    f64::from_bits((emu.read_u32(out_cell) as u64) | ((emu.read_u32(out_cell + 4) as u64) << 32))
}

#[test]
fn set_alpha_scaled() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (isl, in_cell, out_cell) = build(&mut emu);
    let isl_end = isl + 2 + 4 + (SLICE_END - SLICE_BEG) + 2 + 4;

    let chk = |emu: &mut Emu, x: f64| {
        let bin = run(emu, isl, isl_end, in_cell, out_cell, x);
        assert_eq!(
            bin.to_bits(),
            frameapi_alpha::set_alpha_scaled(x).to_bits(),
            "set_alpha_scaled x={x} ({:#018x}): binary {bin} vs rust {}",
            x.to_bits(),
            frameapi_alpha::set_alpha_scaled(x)
        );
    };

    // The clamp boundaries, where a strict and an inclusive comparison differ.
    let eps = f64::from_bits(1); // smallest positive subnormal step near the boundaries below
    let boundaries: [f64; 11] = [
        0.0,
        -0.0,
        f64::from_bits(0.0f64.to_bits() + 1), // +1 ulp above 0
        -eps,                                 // just below 0 -> clamps to 0
        1.0,
        f64::from_bits(1.0f64.to_bits() - 1), // just below 1 -> kept
        f64::from_bits(1.0f64.to_bits() + 1), // just above 1 -> clamps to 1
        0.5,
        f64::from_bits(0.5f64.to_bits() + 1),
        1.0 / 255.0, // a single alpha step
        254.0 / 255.0,
    ];
    for x in boundaries {
        chk(&mut emu, x);
    }

    // ---- spanning fuzz: below-0 / in-[0,1] / above-1, finite doubles ----
    let mut s = 0x774e_9000u32;
    let next = |s: &mut u32| {
        *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        *s
    };
    // f64 in [lo,hi) from 52 high-quality bits (two draws).
    let rngd = |s: &mut u32, lo: f64, hi: f64| -> f64 {
        let hi52 = ((next(s) >> 8) as u64) << 24 | (next(s) >> 8) as u64;
        lo + (hi52 as f64) / ((1u64 << 48) as f64) * (hi - lo)
    };
    for _ in 0..6000 {
        chk(&mut emu, rngd(&mut s, -2.0, 3.0)); // straddles both clamp bounds densely
    }
    for _ in 0..2000 {
        chk(&mut emu, rngd(&mut s, -1e6, 1e6)); // wide, clamp dominates
    }
    for _ in 0..2000 {
        chk(&mut emu, rngd(&mut s, 0.0, 1.0)); // dense interior (no clamp)
    }
}

#[test]
fn set_alpha_value() {
    // The full SetAlpha result = the clamped/scaled product truncated by `_ftol`. The clamp pins
    // the product in [0,255], so the truncate is an exact `as i64 as i32`; verify the composed
    // value against the island product for the boundary + a dense grid. This couples the diffed
    // arithmetic (set_alpha_scaled, oracle-checked above) to the integer the virtual setter stores.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (isl, in_cell, out_cell) = build(&mut emu);
    let isl_end = isl + 2 + 4 + (SLICE_END - SLICE_BEG) + 2 + 4;

    let mut s = 0x77_4f3200u32;
    let next = |s: &mut u32| {
        *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        *s
    };
    let rngd = |s: &mut u32, lo: f64, hi: f64| -> f64 {
        let hi52 = ((next(s) >> 8) as u64) << 24 | (next(s) >> 8) as u64;
        lo + (hi52 as f64) / ((1u64 << 48) as f64) * (hi - lo)
    };
    // include exact alpha steps k/255 so the truncate lands on every integer 0..255.
    for k in 0..=255u32 {
        let x = k as f64 / 255.0;
        let prod = run(&mut emu, isl, isl_end, in_cell, out_cell, x);
        assert_eq!(
            benilla_re_cmath::x87::ftol(prod),
            frameapi_alpha::set_alpha_value(x),
            "alpha step k={k} x={x}"
        );
    }
    for _ in 0..4000 {
        let x = rngd(&mut s, -2.0, 3.0);
        let prod = run(&mut emu, isl, isl_end, in_cell, out_cell, x);
        assert_eq!(
            benilla_re_cmath::x87::ftol(prod),
            frameapi_alpha::set_alpha_value(x),
            "alpha value x={x} ({:#018x})",
            x.to_bits()
        );
    }
    // NaN, the infinities and values past `i32` and `i64`: the clamp pins the product to 0 to 255
    // (a NaN goes through it), and `__ftol` returns the low dword of its qword, 0 for the
    // indefinite.
    for x in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0e10,
        -1.0e10,
        1.0e30,
        -1.0e30,
    ] {
        let prod = run(&mut emu, isl, isl_end, in_cell, out_cell, x);
        assert_eq!(
            benilla_re_cmath::x87::ftol(prod),
            frameapi_alpha::set_alpha_value(x),
            "alpha wide x={x}"
        );
    }
}
