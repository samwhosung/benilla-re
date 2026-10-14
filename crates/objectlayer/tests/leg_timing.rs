//! The span-time solve of `0x5f9120` against the exe, in two slices. The per-stop slice
//! `[0x5f91af, 0x5f924f)` runs the length fetch `0x453300` into `st0`, the constant-acceleration
//! solve, the rounding to ms and `__ftol`, and reads `eax`. The end slice `[0x5f92b8, 0x5f9335)`
//! runs the same with the length read from memory. Each runs with `edi` at 0 (the first span) and
//! not.
//!
//! `0x453300` returns the length from a synthesized segment: with two points it is the first
//! float of the segment's length array.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::leg_timing::{arc_time_ms, arc_time_ms_final};
use common::{next, next_down, next_up, specials};

struct Rig {
    esi: u64,
    seg: u64,
    lenarr: u64,
    buf: u64,
    stack: u64,
    ebp: u64,
}

fn rig(emu: &mut Emu) -> Rig {
    let esi = emu.alloc(0x40); // the path object: +0x10 base, +0x18 speed, +0x20 accel
    let seg = emu.alloc(0x40); // the segment `0x453300` reads: +0x20 is the length array
    let lenarr = emu.alloc(0x40);
    let buf = emu.alloc(0x40); // the leg buffer the end slice reads its length from, at +8
    let stack = emu.alloc(0x2000);
    let frame = emu.alloc(0x100);
    Rig {
        esi,
        seg,
        lenarr,
        buf,
        stack,
        ebp: frame + 0x60,
    }
}

/// The per-stop slice: `eax` is the span's ms.
fn run_stop(
    emu: &mut Emu,
    r: &Rig,
    length: f32,
    last: f32,
    speed: f32,
    accel: f32,
    edi: u32,
) -> i32 {
    emu.write_u32(r.esi + 0x18, speed.to_bits());
    emu.write_u32(r.esi + 0x20, accel.to_bits());
    emu.write_u32(r.seg + 0x20, r.lenarr as u32);
    emu.write_u32(r.lenarr, length.to_bits());
    let esp = r.stack + 0x1000;
    emu.write_u32(esp, 2);
    emu.set_reg(RegisterX86::ESP, esp);
    emu.set_reg(RegisterX86::EBP, r.ebp);
    emu.set_reg(RegisterX86::ESI, r.esi);
    emu.set_reg(RegisterX86::ECX, r.seg);
    emu.set_reg(RegisterX86::EDI, u64::from(edi));
    emu.write_u32(r.ebp + 0x10, last.to_bits());
    emu.run(0x5f91af, 0x5f924f);
    emu.get_reg(RegisterX86::EAX) as i32
}

/// The end slice, started at `test edi,edi` so the `jne` reads `edi`.
fn run_end(
    emu: &mut Emu,
    r: &Rig,
    length: f32,
    last: f32,
    speed: f32,
    accel: f32,
    edi: u32,
) -> i32 {
    emu.write_u32(r.esi + 0x10, 0);
    emu.write_u32(r.esi + 0x18, speed.to_bits());
    emu.write_u32(r.esi + 0x20, accel.to_bits());
    emu.write_u32(r.buf + 8, length.to_bits());
    emu.set_reg(RegisterX86::ESP, r.stack + 0x1000);
    emu.set_reg(RegisterX86::EBP, r.ebp);
    emu.set_reg(RegisterX86::ESI, r.esi);
    emu.set_reg(RegisterX86::EBX, r.buf);
    emu.set_reg(RegisterX86::EDI, u64::from(edi));
    emu.write_u32(r.ebp + 0x10, last.to_bits());
    emu.run(0x5f92b8, 0x5f9335);
    emu.get_reg(RegisterX86::EAX) as i32
}

#[allow(clippy::too_many_arguments)]
fn check(
    emu: &mut Emu,
    r: &Rig,
    length: f32,
    last: f32,
    speed: f32,
    accel: f32,
    edi: u32,
    ctx: &str,
) {
    let stop = run_stop(emu, r, length, last, speed, accel, edi);
    assert_eq!(
        stop,
        arc_time_ms(length, last, speed, accel, edi == 0),
        "stop {ctx} length={length:e} last={last:e} speed={speed:e} accel={accel:e} edi={edi}"
    );
    let end = run_end(emu, r, length, last, speed, accel, edi);
    assert_eq!(
        end,
        arc_time_ms_final(length, last, speed, accel, edi == 0),
        "end {ctx} length={length:e} last={last:e} speed={speed:e} accel={accel:e} edi={edi}"
    );
}

#[test]
fn leg_time_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let r = rig(&mut emu);

    let cases: &[(f32, f32, f32, f32)] = &[
        (10.0, 0.0, 2.0, 4.0),
        (5.0, 1.0, 1.0, 8.0),
        (100.0, 0.0, 3.0, 2.0),
        (0.5, 0.0, 0.1, 10.0),
        (1.0, 1.0, 2.0, 3.0), // seg = 0
        (50.0, 10.0, 5.0, 5.0),
        (3.0, 0.0, 10.0, 0.5), // da large against seg
        (1000.0, 0.0, 1.0, 1.0),
    ];
    for &(length, last, speed, accel) in cases {
        for edi in [0u32, 1] {
            check(&mut emu, &r, length, last, speed, accel, edi, "case");
        }
    }

    let mut s = 0x5f91_2000u32;
    let rng = |s: &mut u32, lo: f32, hi: f32| {
        lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
    };
    for _ in 0..5000 {
        let last = rng(&mut s, 0.0, 1000.0);
        let length = last + rng(&mut s, 0.0, 5000.0);
        let speed = rng(&mut s, 0.1, 50.0);
        let accel = rng(&mut s, 0.1, 50.0);
        let edi = (next(&mut s) >> 8) & 1;
        check(&mut emu, &r, length, last, speed, accel, edi, "fuzz");
    }
}

/// The compares and the conversion at their edges. `da < seg` and `da < seg / 2` run at equality
/// and a step either side, the span lengths and the rates take NaN, both infinities and both
/// zeros, and spans long enough that `t * 1000` leaves `i64` and `__ftol` stores 0.
#[test]
fn leg_time_special_values_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let r = rig(&mut emu);

    // speed 2 and accel 4 give ta = 0.5 and da = 0.5, so seg = 0.5 is `da == seg` and seg = 1
    // is `da == seg / 2`.
    let (speed, accel) = (2.0f32, 4.0f32);
    for seg in [0.5f32, 1.0] {
        for length in [seg, next_up(seg), next_down(seg)] {
            for edi in [0u32, 1] {
                check(&mut emu, &r, length, 0.0, speed, accel, edi, "threshold");
            }
        }
    }

    let bases: [(f32, f32, f32, f32); 4] = [
        (10.0, 0.0, 2.0, 4.0),
        (100.0, 3.0, 3.0, 2.0),
        (0.75, 0.25, 0.1, 10.0),
        (1.0e6, 0.0, 0.5, 0.25),
    ];
    let sp = specials(&[0.5, 1.0]);
    for &(length, last, speed, accel) in &bases {
        for &v in &sp {
            for edi in [0u32, 1] {
                check(&mut emu, &r, v, last, speed, accel, edi, "length");
                check(&mut emu, &r, length, v, speed, accel, edi, "last");
                check(&mut emu, &r, length, last, v, accel, edi, "speed");
                check(&mut emu, &r, length, last, speed, v, edi, "accel");
            }
        }
    }
    // t * 1000 past i64: a huge span at a tiny speed.
    for edi in [0u32, 1] {
        check(&mut emu, &r, 1.0e30, 0.0, 1.0e-3, 1.0, edi, "huge");
        check(&mut emu, &r, 3.0e38, 0.0, 1.0e-30, 1.0e-30, edi, "huge");
        check(&mut emu, &r, 1.0e12, 0.0, 1.0, 1.0e-3, edi, "huge");
    }
}
