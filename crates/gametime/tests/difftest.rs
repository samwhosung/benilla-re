//! The game clock against the exe. `0x643a70` runs whole through `call_f64`, which reads the
//! unrounded `st0`, with the tick source `0x42c010` stubbed and the clock's fields written: fixed
//! cases through zero, one and several day reductions, then random ones.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_gametime::{advance_offset, day_phase, minute_accum};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

#[test]
fn gametime_643a70_day_phase() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x80);
    emu.stub(0x42c010, 0, 0); // the tick source returns 0, so elapsed = 0 − base

    // `elapsed` is set through `base = −elapsed`.
    let run = |emu: &mut Emu, elapsed: u32, scale: f32, offset: f32| -> f64 {
        let base = 0u32.wrapping_sub(elapsed);
        emu.write_u32(this + 0x2c, scale.to_bits());
        emu.write_u32(this + 0x38, base);
        emu.write_u32(this + 0x3c, offset.to_bits());
        emu.call_f64(0x643a70, this as u32, 0, &[])
    };
    let want = |elapsed: u32, scale: f32, offset: f32| -> f64 {
        day_phase(0, 0u32.wrapping_sub(elapsed), scale, offset)
    };

    let mut total = 0u32;
    // (elapsed ms, scale, offset): under a day, one to several days, near the edge, negative.
    let cases: &[(u32, f32, f32)] = &[
        (1000, 1.0, 0.0),       // phase = 1.0 (no loop)
        (1_440_000, 1.0, 0.0),  // ≈ 1 day
        (5_000_000, 1.0, 0.0),  // multiple days → loop
        (0, 1.0, 720.0),        // offset = noon
        (3_600_000, 0.5, 0.0),  // scale 0.5
        (0, 1.0, -100.0),       // negative phase (no reduction)
        (864_000, 1.0, 1439.5), // near a boundary
        (2_880_000, 1.0, 0.0),  // 2 days
    ];
    for &(e, s, o) in cases {
        assert_eq!(
            run(&mut emu, e, s, o).to_bits(),
            want(e, s, o).to_bits(),
            "day_phase elapsed={e} scale={s} offset={o}"
        );
        total += 1;
    }
    let mut st = 0x643a_7000u32;
    for _ in 0..6000 {
        // At most a few days, so the reduction loop stays short.
        let elapsed = next(&mut st) % 50_000_000;
        let scale = rng(&mut st, 0.0, 2.0);
        let offset = rng(&mut st, -2000.0, 2000.0);
        assert_eq!(
            run(&mut emu, elapsed, scale, offset).to_bits(),
            want(elapsed, scale, offset).to_bits(),
            "day_phase fuzz elapsed={elapsed} scale={scale} offset={offset}"
        );
        total += 1;
    }
    eprintln!("gametime_643a70_day_phase: {total} bit-exact (mod-1440 day-phase)");
}

/// `0x6435a0` runs whole (`ret 4`) and the result is read from `[+0x30]`. The one-minute advance
/// `0x643ad0` is stubbed, since it changes the clock but not the accumulator; `__ftol` runs. The
/// cases take each path: accumulate only, the pending correction, and the reduction loop.
#[test]
fn gametime_6435a0_minute_accum() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x643ad0, 0, 0); // the one-minute advance (__thiscall, ret)
    let this = emu.alloc(0x80);

    let run = |emu: &mut Emu, dt: f32, scale: f32, accum: f32, pending: i32| -> f32 {
        emu.write_u32(this + 0x2c, scale.to_bits());
        emu.write_u32(this + 0x30, accum.to_bits());
        emu.write_u32(this + 0x34, pending as u32);
        emu.call(0x6435a0, this as u32, 0, &[dt.to_bits()]);
        emu.read_f32(this + 0x30)
    };

    let mut total = 0u32;
    let cases: &[(f32, f32, f32, i32)] = &[
        (0.5, 1.0, 0.0, 0),   // accumulate only, result < 1
        (2.5, 1.0, 0.0, 0),   // the loop takes 2.5 to 0.5
        (0.3, 1.0, 0.4, 0),   // accumulate to 0.7
        (3.0, 2.0, 0.5, 5),   // the correction (pending 5, result 6.5), then the loop
        (10.0, 1.0, 0.0, 3),  // correction clamps to 3, then loop the rest
        (1.0, 1.0, 0.0, 1),   // result exactly 1.0, pending 1
        (0.9, 0.5, 0.2, 2),   // result 0.65 < 1, no correction
        (5.5, 1.0, 0.0, 100), // pending above trunc(result), clamped to it
    ];
    for &(dt, s, a, p) in cases {
        assert_eq!(
            run(&mut emu, dt, s, a, p).to_bits(),
            minute_accum(dt, s, a, p).to_bits(),
            "minute_accum dt={dt} scale={s} accum={a} pending={p}"
        );
        total += 1;
    }
    // The conversion runs for a result of at least 1 with a nonzero pending count, and the loop after
    // it takes 1.0 off the result per pass. A result past `i32` takes billions of passes (or never
    // ends, once the `f32` is too large for the subtraction to change it), so no probe reaches
    // the conversion with such an input; NaN skips it, as `result >= 1` fails.
    let mut st = 0x6435_a000u32;
    for _ in 0..5000 {
        // A small result, so the reduction loop stays short.
        let dt = rng(&mut st, 0.0, 20.0);
        let scale = rng(&mut st, 0.0, 3.0);
        let accum = rng(&mut st, 0.0, 1.0);
        let pending = (next(&mut st) % 20) as i32;
        assert_eq!(
            run(&mut emu, dt, scale, accum, pending).to_bits(),
            minute_accum(dt, scale, accum, pending).to_bits(),
            "minute_accum fuzz dt={dt} scale={scale} accum={accum} pending={pending}"
        );
        total += 1;
    }
    eprintln!("gametime_6435a0_minute_accum: {total} bit-exact (accumulate + __ftol correction + reduce loop)");
}

/// The slice `0x643b11` to `0x643b1e` of `0x643ad0`: `fild dword [ebp-4]` (the new minute),
/// `mov [esi+0x38],eax` (the tick), `fadd [esi+0x30]`, `fstp [esi+0x3c]`, read from `[esi+0x3c]`.
#[test]
fn gametime_643ad0_advance_offset() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x80);
    let frame = emu.alloc(0x80);
    let ebp = frame + 0x40;
    let stack = emu.alloc(0x400);

    let run = |emu: &mut Emu, new_minute: i32, accum: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x300);
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::EAX, 0); // the tick stored to [esi+0x38]
        emu.write_u32(ebp - 0x4, new_minute as u32);
        emu.write_u32(this + 0x30, accum.to_bits());
        emu.run(0x643b11, 0x643b1e);
        emu.read_f32(this + 0x3c)
    };

    let mut total = 0u32;
    for nm in [0i32, 1, 719, 720, 1439] {
        for &a in &[0.0f32, 0.25, 0.5, 0.999, -0.3, 1.0e-7] {
            assert_eq!(
                run(&mut emu, nm, a).to_bits(),
                advance_offset(nm, a).to_bits(),
                "advance_offset new_minute={nm} accum={a}"
            );
            total += 1;
        }
    }
    let mut st = 0x643a_d000u32;
    for _ in 0..3000 {
        let nm = (next(&mut st) % 1440) as i32;
        let a = rng(&mut st, -1.0, 1.0);
        assert_eq!(
            run(&mut emu, nm, a).to_bits(),
            advance_offset(nm, a).to_bits(),
            "advance_offset fuzz new_minute={nm} accum={a}"
        );
        total += 1;
    }
    eprintln!("gametime_643ad0_advance_offset: {total} bit-exact (fild + fadd offset)");
}
