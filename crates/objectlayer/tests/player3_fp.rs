//! The player range kernels of `src/player3_fp.rs` against the exe. Each runs a slice of its
//! function, with the registers and fields it reads set directly, reading the store it ends in or
//! the status byte `fnstsw ax` leaves; the slices that subtract call the real `0x4549a0`:
//! - `CanLootNow` (`0x5ec110`): the squared reach `[0x5ec142, 0x5ec17e)` and the squared distance
//!   with its compare `[0x5ec196, 0x5ec1c1)`;
//! - `InteractWithNpc` (`0x5f0130`): the squared distance `[0x5f01b9, 0x5f0200)` and the compare
//!   twice, `[0x5f0202, 0x5f020d)` and `[0x5f0238, 0x5f0243)`;
//! - `InteractWithSpell` (`0x5f05e0`): the squared distance and range compare `[0x5f0656,
//!   0x5f069c)`.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either side,
//! to its random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::player3_fp::*;
use common::*;

#[test]
fn p3_interact_reach_sq_5ec110() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let target = e.alloc(0x200);
    let sf = e.alloc(0x300);
    let tf = e.alloc(0x300);
    e.write_u32(this + 0x110, sf as u32);
    e.write_u32(target + 0x110, tf as u32);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5ec1_1000u32;

    let case = |e: &mut Emu, this_r: f32, target_r: f32, ctx: &str| {
        e.write_u32(sf + 0x1f0, this_r.to_bits());
        e.write_u32(tf + 0x1f0, target_r.to_bits());
        e.set_reg(RegisterX86::ECX, target);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5ec142, 0x5ec17e);
        let bin = e.read_f32(ebp - 4);
        assert_f32(
            bin,
            interact_reach_sq_5ec110(this_r, target_r),
            &format!("{ctx}: radii {this_r} {target_r}"),
        );
    };

    case(&mut e, 0.0, 0.0, "below 5.0, floored");
    case(&mut e, 1.0, 1.0, "below 5.0, floored");
    // The reach that is exactly 5.0 on the stack, and a step either side of the radius.
    let exact = (f64::from(REACH_FLOOR) - f64::from(REACH_SLOP)) as f32;
    case(&mut e, 0.0, exact, "reach exactly 5.0");
    case(&mut e, 0.0, next_up(exact), "a step above");
    case(&mut e, 0.0, next_down(exact), "a step below");
    let h = (5.0 - 3.0 - f64::from(REACH_SLOP)) as f32;
    case(&mut e, h, 3.0, "reach exactly 5.0 from two radii");
    case(&mut e, 10.0, 10.0, "above, kept");
    case(&mut e, 2.5, 2.5, "just above 5.0, kept");
    let sp = specials(&[exact]);
    for &a in &sp {
        for &b in &sp {
            case(&mut e, a, b, "special");
        }
    }
    for i in 0..4000u32 {
        let (this_r, target_r) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -2.0, 8.0), frand(&mut s, -2.0, 8.0))
        };
        case(&mut e, this_r, target_r, &format!("fuzz{i}"));
    }
}

#[test]
fn p3_interact_in_range_5ec110() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this_pos = e.alloc(0x20);
    let target_pos = e.alloc(0x20);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5ec1_9600u32;
    let mut seen = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, tp: [f32; 3], gp: [f32; 3], reach_sq: f32, ctx: &str| {
        for k in 0..3 {
            e.write_u32(this_pos + 4 * k as u64, tp[k].to_bits());
            e.write_u32(target_pos + 4 * k as u64, gp[k].to_bits());
        }
        e.write_u32(ebp - 4, reach_sq.to_bits());
        e.set_reg(RegisterX86::EAX, this_pos);
        e.set_reg(RegisterX86::EDI, target_pos);
        e.set_reg(RegisterX86::EBX, 1);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5ec196, 0x5ec1c1);
        let m = ah(e) & 0x41;
        let bin_in = !parity_even(m);
        seen.insert(m);
        let dist2 = dist2_5ec110(tp, gp);
        assert_eq!(
            bin_in,
            interact_in_range_5ec110(dist2, reach_sq),
            "{ctx}: ah&0x41={m:#04x} dist2={dist2} reach_sq={reach_sq}"
        );
    };

    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 25.0, "d² == r²");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 24.0, "d² > r²");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 26.0, "d² < r²");
    case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        next_up(25.0),
        "r² a step above",
    );
    case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        next_down(25.0),
        "r² a step below",
    );
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], 25.0, "nan");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], f32::NAN, "nan reach");
    let sp = specials(&[25.0]);
    for &x in &sp {
        for &r in &sp {
            case(&mut e, [x, 0.0, 0.0], [0.0; 3], r, "special");
            case(&mut e, [0.0, -0.0, x], [-0.0; 3], r, "special, other axes");
        }
    }
    for i in 0..5000u32 {
        let scale = match i % 3 {
            0 => 6.0,
            1 => 50.0,
            _ => 1e3,
        };
        let tp = if i % 7 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -scale, scale),
                frand(&mut s, -scale, scale),
                frand(&mut s, -scale, scale),
            ]
        };
        let gp = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let reach_sq = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            let r = frand(&mut s, 5.0, 30.0);
            (f64::from(r) * f64::from(r)) as f32
        };
        case(&mut e, tp, gp, reach_sq, &format!("fuzz{i}"));
    }
    for want in [0x00, 0x01, 0x40, 0x41] {
        assert!(seen.contains(&want), "outcome {want:#04x} never seen");
    }
}

#[test]
fn p3_dist2_f32deltas_5f0130() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this_pos = e.alloc(0x20);
    let target_pos = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let ebx = e.alloc(0x40);
    let mut s = 0x5f01_3000u32;

    // `ecx` is reloaded from `[ebp-4]` as the target's position; `[ebx+0xc]` is read and unused.
    let case = |e: &mut Emu, tp: [f32; 3], gp: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(this_pos + 4 * k as u64, tp[k].to_bits());
            e.write_u32(target_pos + 4 * k as u64, gp[k].to_bits());
        }
        e.write_u32(ebp - 4, target_pos as u32);
        e.set_reg(RegisterX86::EAX, this_pos);
        e.set_reg(RegisterX86::EBX, ebx);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5f01b9, 0x5f0200);
        let bin = e.read_f32(ebp - 4);
        assert_f32(
            bin,
            dist2_f32deltas_5f0130(tp, gp),
            &format!("{ctx}: {tp:?} less {gp:?}"),
        );
    };

    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], "3-4-5");
    case(&mut e, [0.0; 3], [0.0; 3], "zero");
    case(&mut e, [0.1, 0.1, 0.1], [0.0; 3], "deltas not exact");
    case(&mut e, [1e7, 1.0, -1e7], [0.0; 3], "wide magnitudes");
    let sp = specials(&[]);
    for &x in &sp {
        for &y in &sp {
            case(&mut e, [x, y, 1.0], [0.0; 3], "special");
            case(&mut e, [y, 1.0, x], [-0.0; 3], "special, other axes");
        }
    }
    for i in 0..4000u32 {
        let scale = match i % 3 {
            0 => 6.0,
            1 => 50.0,
            _ => 1e3,
        };
        let tp = if i % 6 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -scale, scale),
                frand(&mut s, -scale, scale),
                frand(&mut s, -scale, scale),
            ]
        };
        let gp = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        case(&mut e, tp, gp, &format!("fuzz{i}"));
    }
}

#[test]
fn p3_in_range_gate_5f0130() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5f02_0200u32;
    let mut seen = std::collections::BTreeSet::new();

    // The compare is made twice. The first `jne` at 0x5f0210 and the second `je` at 0x5f0246 read
    // the same status bits, so both slices are held to `in_range_5f0130`.
    let mut case = |e: &mut Emu, dist2: f32, threshold: f32, ctx: &str| {
        for (start, end) in [(0x5f0202u64, 0x5f020du64), (0x5f0238, 0x5f0243)] {
            e.write_u32(ebp - 4, dist2.to_bits());
            e.write_u32(0xc4c28c, threshold.to_bits());
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.run(start, end);
            let m = ah(e) & 0x41;
            assert_eq!(
                m != 0,
                in_range_5f0130(dist2, threshold),
                "{ctx} at {start:#x}: ah&0x41={m:#04x} dist2={dist2} thr={threshold}"
            );
            seen.insert(m);
        }
    };

    case(&mut e, 25.0, 25.0, "d² == thr");
    case(&mut e, 26.0, 25.0, "d² > thr");
    case(&mut e, 24.0, 25.0, "d² < thr");
    case(&mut e, next_up(25.0), 25.0, "a step above");
    case(&mut e, next_down(25.0), 25.0, "a step below");
    case(&mut e, f32::NAN, 25.0, "nan d²");
    case(&mut e, 25.0, f32::NAN, "nan threshold");
    let sp = specials(&[25.0]);
    for &d in &sp {
        for &t in &sp {
            case(&mut e, d, t, "special");
        }
    }
    for i in 0..4000u32 {
        let dist2 = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 1e4)
        };
        let threshold = if i % 7 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 1e4)
        };
        case(&mut e, dist2, threshold, &format!("fuzz{i}"));
    }
    for want in [0x00, 0x01, 0x40, 0x41] {
        assert!(seen.contains(&want), "outcome {want:#04x} never seen");
    }
}

#[test]
fn p3_attack_range_gate_5f05e0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this_pos = e.alloc(0x20);
    let target_pos = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5f05_e000u32;
    let mut seen = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, tp: [f32; 3], gp: [f32; 3], range: f32, ctx: &str| {
        for k in 0..3 {
            e.write_u32(this_pos + 4 * k as u64, tp[k].to_bits());
            e.write_u32(target_pos + 4 * k as u64, gp[k].to_bits());
        }
        e.write_u32(ebp - 8, range.to_bits());
        e.set_reg(RegisterX86::EAX, this_pos);
        e.set_reg(RegisterX86::ESI, target_pos);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5f0656, 0x5f069c);
        let m = ah(e) & 0x41;
        let dist2 = dist2_f32deltas_5f05e0(tp, gp);
        assert_eq!(
            m == 0,
            attack_range_gate_5f05e0(range, dist2),
            "{ctx}: ah&0x41={m:#04x} range={range} dist2={dist2}"
        );
        seen.insert(m);
    };

    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 5.0, "r² == d²");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 6.0, "r² > d²");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 4.0, "r² < d²");
    case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        next_up(5.0),
        "range a step above",
    );
    case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        next_down(5.0),
        "range a step below",
    );
    case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        -5.0,
        "negative range, same square",
    );
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], 5.0, "nan");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], f32::NAN, "nan range");
    let sp = specials(&[5.0]);
    for &x in &sp {
        for &r in &sp {
            case(&mut e, [x, 4.0, 0.0], [0.0; 3], r, "special");
            case(&mut e, [0.0, x, 3.0], [-0.0; 3], r, "special, other axes");
        }
    }
    for i in 0..5000u32 {
        let scale = match i % 3 {
            0 => 6.0,
            1 => 50.0,
            _ => 1e3,
        };
        let tp = if i % 7 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -scale, scale),
                frand(&mut s, -scale, scale),
                frand(&mut s, -scale, scale),
            ]
        };
        let gp = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let range = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 40.0)
        };
        case(&mut e, tp, gp, range, &format!("fuzz{i}"));
    }
    for want in [0x00, 0x01, 0x40, 0x41] {
        assert!(seen.contains(&want), "outcome {want:#04x} never seen");
    }
}
