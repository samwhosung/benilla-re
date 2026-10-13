//! The object layer's facing arithmetic against the exe. `CGUnit_C::RotateFacings` (`0x601050`)
//! runs whole on both fields. `CGUnit_C::OnAutoMoveTransportChange` (`0x6115e0`) runs whole with
//! `0x6103a0` stubbed to 1 and the mode `[0xc4d888]` at 2, which reaches the wrap. `0x615070`
//! runs from `0x6150c7` to `0x6150da` with `HeadingBetweenPoints` (`0x47f220`) replaced by a
//! load of the chosen heading, and `0x615150` from `0x615185` to `0x615191`.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::facing::*;

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// Asserts the same bits, or NaN on both sides.
fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// Replaces the call target `fn_va` with `fld dword [slot]; ret n` and returns the slot, so a
/// slice's call leaves a chosen value in `st0`.
fn install_st0_stub(e: &mut Emu, fn_va: u64, n: u16) -> u64 {
    let slot = e.alloc(0x10);

    let mut bytes = vec![0xd9u8, 0x05];
    bytes.extend_from_slice(&(slot as u32).to_le_bytes());
    bytes.push(0xc2);
    bytes.extend_from_slice(&n.to_le_bytes());
    e.write(fn_va, &bytes);
    slot
}

/// Both wrap arms and none, 0 and 2π exactly, one ulp under 2π, NaN, and random sums.
#[test]
fn wrap_angle_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x1000);
    let mut s = 0x6010_5000u32;
    let mut seen_add = false;
    let mut seen_sub = false;
    let mut seen_none = false;

    let mut case = |e: &mut Emu, acc: f32, delta: f32, ctx: &str| {
        e.write_u32(this + 0xc98, acc.to_bits());
        e.write_u32(this + 0xc94, acc.to_bits());
        e.call(0x601050, this as u32, 0, &[delta.to_bits()]);
        let yaw = e.read_f32(this + 0xc98);
        let pitch = e.read_f32(this + 0xc94);
        let want = wrap_angle(acc, delta);
        assert_f32(yaw, want, &format!("{ctx} yaw"));
        assert_f32(pitch, want, &format!("{ctx} pitch"));
        let sum = f64::from(acc) + f64::from(delta);
        if !sum.is_nan() {
            if sum < 0.0 {
                seen_add = true;
            } else if sum >= f64::from(TWO_PI) {
                seen_sub = true;
            } else {
                seen_none = true;
            }
        }
    };

    case(&mut e, 0.0, 0.0, "0+0 (==0 lower bound → no wrap)");
    case(&mut e, TWO_PI, 0.0, "2π+0 (==2π → subtract)");
    case(&mut e, TWO_PI, -0.0, "2π+(-0) (==2π)");
    let just_under = f32::from_bits(TWO_PI.to_bits() - 1);
    case(
        &mut e,
        just_under,
        0.0,
        "(2π−1ulp)+0 (just under → no wrap)",
    );
    case(&mut e, 1.0, -1.0, "1+(-1)=0 (==0 → no wrap)");
    case(&mut e, 1.0, -2.0, "1-2=-1 (<0 → add 2π)");
    case(&mut e, 4.0, 4.0, "8 (>2π → subtract)");
    case(&mut e, f32::NAN, 0.0, "NaN acc → passthrough");
    case(&mut e, 0.0, f32::NAN, "NaN delta → passthrough");

    for i in 0..6000u32 {
        let (acc, delta) = if i % 5 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            let acc = frand(&mut s, -7.0, 13.0);
            let delta = match i % 3 {
                0 => frand(&mut s, -1.0, 1.0),
                1 => frand(&mut s, -7.0, 7.0),
                _ => frand(&mut s, -0.01, 0.01),
            };
            (acc, delta)
        };
        case(&mut e, acc, delta, &format!("fuzz{i}"));
    }
    assert!(
        seen_add && seen_sub && seen_none,
        "all three wrap arms exercised (add={seen_add} sub={seen_sub} none={seen_none})"
    );
}

/// The same cases for the global.
#[test]
fn broadcast_wrap_angle_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x400);

    e.stub(0x6103a0, 1, 0);
    let mut s = 0x6115_e000u32;
    let mut seen_add = false;
    let mut seen_sub = false;
    let mut seen_none = false;

    let mut case = |e: &mut Emu, acc: f32, delta: f32, ctx: &str| {
        e.write_u32(0xc4d888, 2);
        e.write_u32(0xc4d9a8, acc.to_bits());
        e.call(0x6115e0, this as u32, 0, &[0u32, delta.to_bits()]);
        let got = e.read_f32(0xc4d9a8);
        let want = broadcast_wrap_angle(acc, delta);
        assert_f32(got, want, ctx);
        let sum = f64::from(acc) + f64::from(delta);
        if !sum.is_nan() {
            if sum < 0.0 {
                seen_add = true;
            } else if sum >= f64::from(TWO_PI) {
                seen_sub = true;
            } else {
                seen_none = true;
            }
        }
    };

    case(&mut e, 0.0, 0.0, "0+0 (==0 → no wrap)");
    case(&mut e, TWO_PI, 0.0, "2π+0 (==2π → subtract)");
    let just_under = f32::from_bits(TWO_PI.to_bits() - 1);
    case(&mut e, just_under, 0.0, "(2π−1ulp)+0 (no wrap)");
    case(&mut e, 1.0, -2.0, "1-2=-1 (<0 → add 2π)");
    case(&mut e, 5.0, 5.0, "10 (>2π → subtract)");
    case(&mut e, f32::NAN, 0.0, "NaN acc → passthrough");

    for i in 0..6000u32 {
        let (acc, delta) = if i % 5 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -7.0, 13.0), frand(&mut s, -7.0, 7.0))
        };
        case(&mut e, acc, delta, &format!("fuzz{i}"));
    }
    assert!(
        seen_add && seen_sub && seen_none,
        "all three wrap arms exercised (add={seen_add} sub={seen_sub} none={seen_none})"
    );
}

/// Signs, zeros, the infinities, NaN and random headings; the result is the `f32` at `[esp]` after
/// the three pushes.
#[test]
fn negate_facing_615070_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let slot = install_st0_stub(&mut e, 0x47f220, 0);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let esi = e.alloc(0x80);

    let case = |e: &mut Emu, facing: f32, ctx: &str| {
        e.write_u32(slot, facing.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, esi);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6150c7, 0x6150da);
        let bin = e.read_f32(SLICE_ESP - 0xc);
        assert_f32(bin, negate_facing(facing), ctx);
    };

    for &v in &[
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        std::f32::consts::PI,
        -std::f32::consts::PI,
        std::f32::consts::TAU,
        -3.5,
        100.25,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
    ] {
        case(&mut e, v, &format!("explicit {v}"));
    }
    let mut s = 0x6150_7000u32;
    for i in 0..2000u32 {
        case(&mut e, frand(&mut s, -8.0, 8.0), &format!("fuzz{i}"));
    }
    for i in 0..2000u32 {
        case(&mut e, frand_bits(&mut s), &format!("fuzz-bits{i}"));
    }
}

/// Ordered and reversed bounds, equal, the infinities, NaN, a difference that rounds, and random
/// bounds.
#[test]
fn bounds_z_extent_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let edx = e.alloc(0x40);
    let ebx = e.alloc(0x200);

    let case = |e: &mut Emu, max_z: f32, min_z: f32, ctx: &str| {
        e.write_u32(edx + 0x14, max_z.to_bits());
        e.write_u32(edx + 0x8, min_z.to_bits());
        e.set_reg(RegisterX86::EDX, edx);
        e.set_reg(RegisterX86::EBX, ebx);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x615185, 0x615191);
        let bin = e.read_f32(ebx + 0xbc);
        assert_f32(bin, bounds_z_extent(max_z, min_z), ctx);
    };

    case(&mut e, 10.0, 2.0, "10−2");
    case(&mut e, 2.0, 10.0, "2−10 (neg)");
    case(&mut e, 0.0, 0.0, "0−0");
    case(&mut e, 5.0, 5.0, "equal → 0");
    case(&mut e, 1.0e20, -1.0e20, "huge");
    case(&mut e, f32::INFINITY, 1.0, "+inf − 1");
    case(&mut e, f32::INFINITY, f32::INFINITY, "inf−inf → NaN");
    case(&mut e, f32::from_bits(0x7fc0_0000), 1.0, "NaN max");

    case(&mut e, 16777216.0, 1.0, "2^24 − 1 (rounding regime)");
    case(&mut e, 16777217.0, 0.5, "2^24+1 unrep, +0.5");
    let mut s = 0x6151_5000u32;
    for i in 0..2000u32 {
        case(
            &mut e,
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            &format!("fuzz{i}"),
        );
    }
    for i in 0..2000u32 {
        case(
            &mut e,
            frand_bits(&mut s),
            frand_bits(&mut s),
            &format!("fuzz-bits{i}"),
        );
    }
}
