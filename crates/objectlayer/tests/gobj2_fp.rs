//! The combat log range tests of `src/gobj2_fp.rs` against the exe. Each runs the compare of a
//! writer, with the registers and fields it reads set directly, reading the status byte `fnstsw
//! ax` leaves:
//! - the writers that subtract on the stack: `LogUnitDeath` (`0x62c160`) with the player in `edi`,
//!   and `LogInstakill` (`0x62cbe0`), `LogAuraDispel` (`0x62d480`) and `LogExtraAttacks`
//!   (`0x62d9f0`) with it in `esi`;
//! - the writers that store the differences: `LogAuraApplication` (`0x62b800`), from the reload of
//!   the stored differences, and `LogCastFailure` (`0x62c360`), from the subtraction through the
//!   vector constructor `0x4549a0`.
//!
//! The sums are read as well: the instruction that loads the range is replaced by `fstp qword
//! [ebp-0x40]`, so the slice ends with the sum itself, whose order a compare of one bit rarely
//! shows. Each test adds NaN, the infinities, both zeros and the equal-distance edge to its random
//! inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::gobj1_fp::copied_dist2;
use benilla_re_objectlayer::gobj1_fp::one_sided_dist2;
use benilla_re_objectlayer::gobj2_fp::*;
use common::*;

/// A subtracting compare: where the slice starts and ends, the range's slot relative to `ebp`, and
/// the register that holds the player's position.
struct Gate {
    start: u64,
    end: u64,
    range_at: i64,
    player: RegisterX86,
    range_load: u64,
}

const GATES: [Gate; 4] = [
    Gate {
        start: 0x62c1f9,
        end: 0x62c225,
        range_at: -4,
        player: RegisterX86::EDI,
        range_load: 0x62c219,
    },
    Gate {
        start: 0x62ccb7,
        end: 0x62cce3,
        range_at: -4,
        player: RegisterX86::ESI,
        range_load: 0x62ccd7,
    },
    Gate {
        start: 0x62d55b,
        end: 0x62d587,
        range_at: 8,
        player: RegisterX86::ESI,
        range_load: 0x62d57b,
    },
    Gate {
        start: 0x62dada,
        end: 0x62db06,
        range_at: -8,
        player: RegisterX86::ESI,
        range_load: 0x62dafa,
    },
];

fn run_gate(
    e: &mut Emu,
    g: &Gate,
    mem: (u64, u64, u64),
    t: [f32; 3],
    p: [f32; 3],
    range: f32,
    stop: u64,
) {
    let (target, player, ebp) = mem;
    for k in 0..3 {
        e.write_u32(target + 4 * k as u64, t[k].to_bits());
        e.write_u32(player + 4 * k as u64, p[k].to_bits());
    }
    e.write_u32((ebp as i64 + g.range_at) as u64, range.to_bits());
    e.set_reg(RegisterX86::EAX, target);
    e.set_reg(g.player, player);
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, ebp - 0x20);
    e.run(g.start, stop);
}

#[test]
fn diff_combat_gate_inreg() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let target = e.alloc(0x20);
    let player = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x0062_c160_u32;
    let g = &GATES[0];

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, t: [f32; 3], p: [f32; 3], range: f32, ctx: &str| {
        run_gate(e, g, (target, player, ebp), t, p, range, g.end);
        let bin = ah(e) & TEST_AH_41;
        let ours = combat_gate_inreg_ah(t, p, range) & TEST_AH_41;
        assert_eq!(
            bin, ours,
            "{ctx}: binary AH&41={bin:#04x} ours={ours:#04x} (t={t:?} p={p:?} range={range})"
        );
        seen.insert(bin);
    };

    // Each branch outcome: inside, outside, equal, and unordered by a position and by the range.
    case(&mut e, [0.0; 3], [0.0; 3], 5.0, "at the point");
    case(&mut e, [10.0, 0.0, 0.0], [0.0; 3], 5.0, "far");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 5.0, "equal at 5");
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
    case(&mut e, [1.0, 2.0, 2.0], [0.0; 3], 3.0, "equal at 3");
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], 5.0, "nan position");
    case(&mut e, [0.0; 3], [0.0; 3], f32::NAN, "nan range");
    let sp = specials(&[5.0]);
    for &x in &sp {
        for &y in &sp {
            for &z in &[0.0, 1.0, f32::NAN, f32::INFINITY] {
                for &r in &sp {
                    case(&mut e, [x, y, z], [0.0; 3], r, "special");
                }
            }
        }
    }
    for i in 0..3200 {
        let (t, p) = (point(&mut s, i), point(&mut s, i));
        let range = match i % 4 {
            0 => frand(&mut s, 0.0, 100.0),
            1 => frand(&mut s, 0.0, 5.0),
            2 => f32::from_bits(next(&mut s)).abs(),
            _ => frand_bits(&mut s),
        };
        case(&mut e, t, p, range, &format!("fuzz{i}"));
    }
    for want in [0x00, 0x01, 0x40, 0x41] {
        assert!(seen.contains(&want), "outcome {want:#04x} never seen");
    }
}

#[test]
fn diff_combat_gate_twins() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let target = e.alloc(0x20);
    let player = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x0062_cbe0_u32;
    let mut seen = std::collections::BTreeSet::new();

    for g in &GATES[1..] {
        let mut case = |e: &mut Emu, t: [f32; 3], p: [f32; 3], range: f32| {
            run_gate(e, g, (target, player, ebp), t, p, range, g.end);
            let bin = ah(e) & TEST_AH_41;
            let ours = combat_gate_inreg_ah(t, p, range) & TEST_AH_41;
            assert_eq!(
                bin, ours,
                "gate {:#x}: binary AH&41={bin:#04x} ours={ours:#04x} (t={t:?} p={p:?} range={range})",
                g.start
            );
            seen.insert(bin);
        };
        case(&mut e, [0.0; 3], [0.0; 3], 5.0);
        case(&mut e, [10.0, 0.0, 0.0], [0.0; 3], 5.0);
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 5.0);
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], next_up(5.0));
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], next_down(5.0));
        case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], 5.0);
        let sp = specials(&[5.0]);
        for &x in &sp {
            for &r in &sp {
                case(&mut e, [x, 4.0, 0.0], [0.0; 3], r);
                case(&mut e, [0.0, x, 3.0], [-0.0; 3], r);
            }
        }
        for i in 0..500 {
            let (t, p) = (point(&mut s, i), point(&mut s, i));
            let range = frand(&mut s, 0.0, 30.0);
            case(&mut e, t, p, range);
        }
    }
    for want in [0x00, 0x01, 0x40, 0x41] {
        assert!(seen.contains(&want), "outcome {want:#04x} never seen");
    }
}

#[test]
fn diff_combat_gate_copied_twin() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let target = e.alloc(0x20);
    let player = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x0062_c360_u32;
    let mut seen = std::collections::BTreeSet::new();

    // The slice subtracts, stores each difference below `esp` and calls the vector constructor
    // `0x4549a0`, which copies them to `[ebp-0x24]`; `[esp]` is that slot's neighbour.
    let mut case = |e: &mut Emu, t: [f32; 3], p: [f32; 3], range: f32| {
        for k in 0..3 {
            e.write_u32(target + 4 * k as u64, t[k].to_bits());
            e.write_u32(player + 4 * k as u64, p[k].to_bits());
        }
        e.write_u32(ebp - 0x10, range.to_bits());
        e.set_reg(RegisterX86::EAX, target);
        e.set_reg(RegisterX86::ESI, player);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x24);
        e.run(0x62c434, 0x62c47a);
        let bin = ah(e) & TEST_AH_41;
        let dx = (f64::from(t[0]) - f64::from(p[0])) as f32;
        let dy = (f64::from(t[1]) - f64::from(p[1])) as f32;
        let dz = (f64::from(t[2]) - f64::from(p[2])) as f32;
        let ours = combat_gate_copied_ah(dx, dy, dz, range) & TEST_AH_41;
        assert_eq!(
            bin, ours,
            "0x62c360: binary AH&41={bin:#04x} ours={ours:#04x} (t={t:?} p={p:?} range={range})"
        );
        seen.insert(bin);
    };
    case(&mut e, [0.0; 3], [0.0; 3], 5.0);
    case(&mut e, [10.0, 0.0, 0.0], [0.0; 3], 5.0);
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 5.0);
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], next_up(5.0));
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], next_down(5.0));
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], 5.0);
    let sp = specials(&[5.0]);
    for &x in &sp {
        for &r in &sp {
            case(&mut e, [x, 4.0, 0.0], [0.0; 3], r);
            case(&mut e, [0.0, x, 3.0], [-0.0; 3], r);
        }
    }
    for i in 0..800 {
        let (t, p) = (point(&mut s, i), point(&mut s, i));
        case(&mut e, t, p, frand(&mut s, 0.0, 30.0));
    }
    for want in [0x00, 0x01, 0x40, 0x41] {
        assert!(seen.contains(&want), "outcome {want:#04x} never seen");
    }
}

#[test]
fn diff_combat_gate_copied() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x0062_b800_u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, dx: f32, dy: f32, dz: f32, range: f32, ctx: &str| {
        e.write_u32(ebp - 0x2c, dx.to_bits());
        e.write_u32(ebp - 0x28, dy.to_bits());
        e.write_u32(ebp - 0x24, dz.to_bits());
        e.write_u32(ebp - 0x10, range.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x62b941, 0x62b961);
        let bin = ah(e) & TEST_AH_41;
        let ours = combat_gate_copied_ah(dx, dy, dz, range) & TEST_AH_41;
        assert_eq!(
            bin, ours,
            "{ctx}: binary AH&41={bin:#04x} ours={ours:#04x} (dx={dx} dy={dy} dz={dz} range={range})"
        );
        seen.insert(bin);
    };

    case(&mut e, 0.0, 0.0, 0.0, 5.0, "at the point");
    case(&mut e, 10.0, 0.0, 0.0, 5.0, "far");
    case(&mut e, 3.0, 4.0, 0.0, 5.0, "equal at 5");
    case(&mut e, 3.0, 4.0, 0.0, next_up(5.0), "range a step above");
    case(&mut e, 3.0, 4.0, 0.0, next_down(5.0), "range a step below");
    case(&mut e, 1.0, 2.0, 2.0, 3.0, "equal at 3");
    case(&mut e, f32::NAN, 0.0, 0.0, 5.0, "nan difference");
    case(&mut e, 0.0, 0.0, 0.0, f32::NAN, "nan range");
    let sp = specials(&[5.0]);
    for &x in &sp {
        for &y in &sp {
            for &z in &[0.0, 1.0, f32::NAN, f32::INFINITY] {
                for &r in &sp {
                    case(&mut e, x, y, z, r, "special");
                }
            }
        }
    }
    for i in 0..3200 {
        let pick = |s: &mut u32| {
            if i % 5 == 0 {
                frand_bits(s)
            } else {
                frand(s, -50.0, 50.0)
            }
        };
        let (dx, dy, dz) = (pick(&mut s), pick(&mut s), pick(&mut s));
        let range = match i % 4 {
            0 => frand(&mut s, 0.0, 100.0),
            1 => frand(&mut s, 0.0, 5.0),
            2 => f32::from_bits(next(&mut s)).abs(),
            _ => frand_bits(&mut s),
        };
        case(&mut e, dx, dy, dz, range, &format!("fuzz{i}"));
    }
    for want in [0x00, 0x01, 0x40, 0x41] {
        assert!(seen.contains(&want), "outcome {want:#04x} never seen");
    }
}

#[test]
fn diff_combat_gate_dist2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let target = e.alloc(0x20);
    let player = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    for g in &GATES {
        patch_fstp_qword(&mut e, g.range_load);
    }
    patch_fstp_qword(&mut e, 0x62b957);
    patch_fstp_qword(&mut e, 0x62c470);
    let mut s = 0x0062_c161_u32;

    for _ in 0..2000 {
        let (t, p) = (point(&mut s, 2), point(&mut s, 2));
        let ours = one_sided_dist2(t, p);
        for g in &GATES {
            run_gate(
                &mut e,
                g,
                (target, player, ebp),
                t,
                p,
                1.0,
                g.range_load + 3,
            );
            let bin = read_f64(&e, ebp - 0x40);
            assert!(
                same_f64(bin, ours),
                "gate {:#x} d2: binary={bin:e} ours={ours:e} (t={t:?} p={p:?})",
                g.start
            );
        }

        // The reload of stored differences in `0x62b800`.
        let d = point(&mut s, 2);
        e.write_u32(ebp - 0x2c, d[0].to_bits());
        e.write_u32(ebp - 0x28, d[1].to_bits());
        e.write_u32(ebp - 0x24, d[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x62b941, 0x62b95a);
        let bin = read_f64(&e, ebp - 0x40);
        let ours = copied_dist2(d[0], d[1], d[2]);
        assert!(
            same_f64(bin, ours),
            "0x62b800 d2: binary={bin:e} ours={ours:e} (d={d:?})"
        );

        // The subtraction, stores and constructor call of `0x62c360`.
        for k in 0..3 {
            e.write_u32(target + 4 * k as u64, t[k].to_bits());
            e.write_u32(player + 4 * k as u64, p[k].to_bits());
        }
        e.set_reg(RegisterX86::EAX, target);
        e.set_reg(RegisterX86::ESI, player);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x24);
        e.run(0x62c434, 0x62c473);
        let bin = read_f64(&e, ebp - 0x40);
        let dx = (f64::from(t[0]) - f64::from(p[0])) as f32;
        let dy = (f64::from(t[1]) - f64::from(p[1])) as f32;
        let dz = (f64::from(t[2]) - f64::from(p[2])) as f32;
        let ours = copied_dist2(dx, dy, dz);
        assert!(
            same_f64(bin, ours),
            "0x62c360 d2: binary={bin:e} ours={ours:e} (t={t:?} p={p:?})"
        );
    }
}
