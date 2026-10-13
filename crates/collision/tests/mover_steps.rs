//! Slices of the mover's functions: `0x618a4c` to `0x618a5f` of `CMovement::SwitchTransport`
//! (`0x618970`), with `GetTransportFacing` (`0x630b70`) replaced by a load of the chosen facing;
//! `0x616e74` to `0x616ea1` and `0x616eff` to `0x616f2c` of `CMovement::IntegrateStep`
//! (`0x616de0`); `0x617ebe` to `0x617f09` of `CMovement::Teleport` (`0x617e90`), reading `ebx`; and
//! `0x619e4a` to `0x619e5d` of `CMovement::CompleteSpline` (`0x619de0`).

use benilla_re_collision::mover_steps::*;
use benilla_re_difftest::{Emu, RegisterX86, Wow};

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

/// The negated facing at `[ebp+0xc]`.
#[test]
fn negate_facing_618970_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let slot = install_st0_stub(&mut e, 0x630b70, 8);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let esi = e.alloc(0x80);

    let case = |e: &mut Emu, facing: f32, ctx: &str| {
        e.write_u32(slot, facing.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, esi);
        e.set_reg(RegisterX86::EDI, 0x1234);
        e.set_reg(RegisterX86::EBX, 0x5678);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x618a4c, 0x618a5f);
        let bin = e.read_f32(ebp + 0xc);
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
    let mut s = 0x6189_7000u32;
    for i in 0..2000u32 {
        case(&mut e, frand(&mut s, -8.0, 8.0), &format!("fuzz{i}"));
    }
    for i in 0..2000u32 {
        case(&mut e, frand_bits(&mut s), &format!("fuzz-bits{i}"));
    }
}

/// The three sums.
#[test]
fn integrate_apply_add_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let esi = e.alloc(0x80);

    let case = |e: &mut Emu, delta: [f32; 3], vel: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0xc, delta[0].to_bits());
        e.write_u32(ebp - 0x8, delta[1].to_bits());
        e.write_u32(ebp - 0x4, delta[2].to_bits());
        e.write_u32(esi + 0x44, vel[0].to_bits());
        e.write_u32(esi + 0x48, vel[1].to_bits());
        e.write_u32(esi + 0x4c, vel[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, esi);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x616e74, 0x616ea1);
        let bin = [
            e.read_f32(ebp - 0x30),
            e.read_f32(ebp - 0x2c),
            e.read_f32(ebp - 0x28),
        ];
        let ours = integrate_apply_add(delta, vel);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} [{k}]"));
        }
    };

    case(&mut e, [1.0, 2.0, 3.0], [0.5, -0.5, 10.0], "simple");
    case(&mut e, [0.0; 3], [0.0; 3], "zero");
    case(&mut e, [-1.0, -2.0, -3.0], [1.0, 2.0, 3.0], "cancel");
    case(&mut e, [f32::INFINITY, 1.0, 1.0], [1.0; 3], "+inf + 1");
    case(
        &mut e,
        [f32::INFINITY, 1.0, 1.0],
        [f32::NEG_INFINITY, 1.0, 1.0],
        "inf + −inf → NaN",
    );
    case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [1.0; 3],
        "NaN + 1",
    );

    case(&mut e, [16777216.0, 0.0, 0.0], [1.0, 0.0, 0.0], "2^24 + 1");
    let mut s = 0x616d_e000u32;
    for i in 0..2000u32 {
        let d = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
        ];
        let v = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
        ];
        case(&mut e, d, v, &format!("fuzz{i}"));
    }
    for i in 0..1000u32 {
        let d = [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)];
        let v = [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)];
        case(&mut e, d, v, &format!("fuzz-bits{i}"));
    }
}

/// The three differences.
#[test]
fn integrate_unapply_sub_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let esi = e.alloc(0x80);

    let case = |e: &mut Emu, inv: [f32; 3], vel: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x18, inv[0].to_bits());
        e.write_u32(ebp - 0x14, inv[1].to_bits());
        e.write_u32(ebp - 0x10, inv[2].to_bits());
        e.write_u32(esi + 0x44, vel[0].to_bits());
        e.write_u32(esi + 0x48, vel[1].to_bits());
        e.write_u32(esi + 0x4c, vel[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, esi);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x616eff, 0x616f2c);
        let bin = [
            e.read_f32(ebp - 0x3c),
            e.read_f32(ebp - 0x38),
            e.read_f32(ebp - 0x34),
        ];
        let ours = integrate_unapply_sub(inv, vel);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} [{k}]"));
        }
    };

    case(&mut e, [1.0, 2.0, 3.0], [0.5, -0.5, 10.0], "simple");
    case(&mut e, [0.0; 3], [0.0; 3], "zero");
    case(&mut e, [1.0, 2.0, 3.0], [1.0, 2.0, 3.0], "self → 0");
    case(&mut e, [f32::INFINITY, 1.0, 1.0], [1.0; 3], "+inf − 1");
    case(
        &mut e,
        [f32::INFINITY, 1.0, 1.0],
        [f32::INFINITY, 1.0, 1.0],
        "inf − inf → NaN",
    );
    case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [1.0; 3],
        "NaN − 1",
    );
    case(&mut e, [16777216.0, 0.0, 0.0], [1.0, 0.0, 0.0], "2^24 − 1");
    let mut s = 0x6f16_de00u32;
    for i in 0..2000u32 {
        let inv = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
        ];
        let v = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
        ];
        case(&mut e, inv, v, &format!("fuzz{i}"));
    }
    for i in 0..1000u32 {
        let inv = [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)];
        let v = [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)];
        case(&mut e, inv, v, &format!("fuzz-bits{i}"));
    }
}

/// `ebx`, with squared distances around 900.
#[test]
fn facing_pos_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let esi = e.alloc(0x80);
    let target = e.alloc(0x20);

    let mut seen_jump = false;
    let mut seen_nojump = false;

    let case = |e: &mut Emu,
                seen_jump: &mut bool,
                seen_nojump: &mut bool,
                sp: [f32; 3],
                tp: [f32; 3],
                ctx: &str| {
        for k in 0..3 {
            e.write_u32(esi + 0x10 + 4 * k as u64, sp[k].to_bits());
            e.write_u32(target + 4 * k as u64, tp[k].to_bits());
        }
        e.write_u32(ebp + 0x10, target as u32);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, esi);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x617ebe, 0x617f09);
        let bin_jumped = (e.get_reg(RegisterX86::EBX) & 0xffff_ffff) == 1;
        if bin_jumped {
            *seen_jump = true;
        } else {
            *seen_nojump = true;
        }
        let d2 = facing_pos_delta2(sp, tp);
        assert_eq!(
            bin_jumped,
            facing_pos_jumped(d2),
            "{ctx}: bin_jumped={bin_jumped} d2={d2} ebx={:#x}",
            e.get_reg(RegisterX86::EBX)
        );
    };

    case(
        &mut e,
        &mut seen_jump,
        &mut seen_nojump,
        [0.0; 3],
        [0.0; 3],
        "d0 → no-jump",
    );
    case(
        &mut e,
        &mut seen_jump,
        &mut seen_nojump,
        [100.0, 0.0, 0.0],
        [0.0; 3],
        "d10000 > 900 → jump",
    );

    case(
        &mut e,
        &mut seen_jump,
        &mut seen_nojump,
        [30.0, 0.0, 0.0],
        [0.0; 3],
        "dx30 d²==900 → no-jump (strict >)",
    );
    case(
        &mut e,
        &mut seen_jump,
        &mut seen_nojump,
        [0.0, 30.0, 0.0],
        [0.0; 3],
        "dy30 d²==900 → no-jump",
    );
    case(
        &mut e,
        &mut seen_jump,
        &mut seen_nojump,
        [0.0, 0.0, 30.0],
        [0.0; 3],
        "dz30 d²==900 → no-jump",
    );

    case(
        &mut e,
        &mut seen_jump,
        &mut seen_nojump,
        [f32::from_bits(0x41f0_0001), 0.0, 0.0],
        [0.0; 3],
        "dx just>30 → jump",
    );

    case(
        &mut e,
        &mut seen_jump,
        &mut seen_nojump,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [0.0; 3],
        "NaN self → no-jump",
    );

    let mut s = 0x617e_9000u32;
    for i in 0..3000u32 {
        let sp = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];

        let tp = match i % 2 {
            0 => [
                sp[0] + frand(&mut s, -35.0, 35.0),
                sp[1] + frand(&mut s, -35.0, 35.0),
                sp[2] + frand(&mut s, -35.0, 35.0),
            ],
            _ => [
                frand(&mut s, -80.0, 80.0),
                frand(&mut s, -80.0, 80.0),
                frand(&mut s, -80.0, 80.0),
            ],
        };
        case(
            &mut e,
            &mut seen_jump,
            &mut seen_nojump,
            sp,
            tp,
            &format!("fuzz{i}"),
        );
    }
    assert!(seen_jump, "jump (ebx=1) arm never seen");
    assert!(seen_nojump, "no-jump (ebx=0) arm never seen");
}

/// The fraction.
#[test]
fn fall_progress_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;

    let case = |e: &mut Emu, num: i32, denom: i32, ctx: &str| {
        e.write_u32(ebp - 4, num as u32);
        e.set_reg(RegisterX86::EDX, denom as u32 as u64);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x619e4a, 0x619e5d);
        let bin = e.read_f32(ebp - 4);
        assert_f32(bin, fall_progress(num, denom), ctx);
    };

    case(&mut e, 0, 1, "0/1");
    case(&mut e, 1, 2, "1/2");
    case(&mut e, 1, 3, "1/3 (rounding)");
    case(&mut e, 5, 5, "5/5 = 1");
    case(&mut e, 100, 7, "100/7");
    case(&mut e, 1, 1000, "1/1000");
    case(&mut e, 16_777_217, 1, "2^24+1 / 1 (int→f32 rounds)");
    case(&mut e, 33_554_433, 1, "2^25+1 / 1");
    case(&mut e, 2_000_000_001, 3, "big num / 3");
    case(&mut e, i32::MAX, 7, "INT_MAX / 7");
    let mut s = 0x619d_e000u32;
    for i in 0..3000u32 {
        let num = (next(&mut s) >> 1) as i32;
        let denom = ((next(&mut s) % 1_000_000) + 1) as i32;
        case(&mut e, num, denom, &format!("fuzz{i}"));
    }
    for i in 0..1000u32 {
        let num = (next(&mut s) % 4096) as i32;
        let denom = ((next(&mut s) % 4096) + 1) as i32;
        case(&mut e, num, denom, &format!("fuzz-small{i}"));
    }
}
