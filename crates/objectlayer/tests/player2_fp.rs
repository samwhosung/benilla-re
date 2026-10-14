//! The player interaction kernels of `src/player2_fp.rs` against the exe. Each runs a slice of its
//! function, with the registers and fields it reads set directly, reading the status byte `fnstsw
//! ax` leaves or a store:
//! - `LootCorpse` (`0x5df130`): the distance compare, `[0x5df1a0, 0x5df1d4)`;
//! - `LootUnit` (`0x5df2a0`): the reach and the compare through the `test` that precedes the `jp`,
//!   `[0x5df301, 0x5df389)`, with both `GetPosition` calls stubbed;
//! - `SpiritHealerDialog` (`0x5df730`), `TalentWipeDialog` (`0x5df980`) and `PetUnlearnDialog`
//!   (`0x5dfba0`): the distance compare against the runtime threshold, `[0x5df7a1, 0x5df7d2)`,
//!   `[0x5df9dc, 0x5dfa0d)` and `[0x5dfbfc, 0x5dfc2d)`, and the skill rank argument of the first,
//!   `[0x5df81e, 0x5df82c)`, with the real `__ftol`;
//! - `IsInsideAreaTrigger` (`0x5e22d0`): the sphere or box choice `[0x5e22ea, 0x5e22f5)` and the
//!   sphere compare `[0x5e22fa, 0x5e2331)`;
//! - `OnDrunkChanged` (`0x5e2a90`): the clamped level and its scaled fraction, `[0x5e2ac9,
//!   0x5e2aee)`.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either side,
//! to its random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::player2_fp::*;
use common::*;

/// Write a vec3 at `va`.
fn wvec(e: &mut Emu, va: u64, v: Vec3) {
    for (k, c) in v.iter().enumerate() {
        e.write_u32(va + 4 * k as u64, c.to_bits());
    }
}

/// A random vec3, mostly bounded and sometimes full range, sized so both arms fire.
fn fvec(s: &mut u32, scale: f32) -> Vec3 {
    let f = |s: &mut u32| {
        if (next(s) >> 8) & 7 == 0 {
            frand_bits(s)
        } else {
            frand(s, -scale, scale)
        }
    };
    [f(s), f(s), f(s)]
}

/// Positions whose squared distance is exactly `d2`'s square root along each axis in turn, and a
/// step either side of it.
fn axis_edges(root: f32) -> Vec<Vec3> {
    let mut v = Vec::new();
    for r in [root, next_up(root), next_down(root)] {
        v.push([r, 0.0, 0.0]);
        v.push([0.0, -r, 0.0]);
        v.push([0.0, 0.0, r]);
    }
    v
}

#[test]
fn loot_range_5df130_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let basevec = e.alloc(0x20);
    let subvec = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5df1_3000u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, base: Vec3, sub: Vec3, ctx: &str| {
        wvec(e, basevec, base);
        wvec(e, subvec, sub);
        e.write_u32(ebp - 4, subvec as u32);
        e.set_reg(RegisterX86::EAX, basevec);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x5df1a0, 0x5df1d4);
        let masked = ah(e) & 0x41;
        assert_eq!(
            masked != 0,
            loot_range_5df130(base, sub),
            "{ctx}: ah&0x41={masked:#04x} (this={base:?} target={sub:?})"
        );
        seen.insert(masked);
    };

    case(&mut e, [0.0; 3], [0.0; 3], "coincident");
    case(&mut e, [100.0, 0.0, 0.0], [0.0; 3], "far");
    case(&mut e, [5.0, 0.0, 0.0], [0.0; 3], "equal at 25");
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], "nan");
    for p in axis_edges(5.0) {
        case(&mut e, p, [0.0; 3], "edge");
        case(&mut e, [0.0; 3], p, "edge, swapped");
    }
    let sp = specials(&[5.0]);
    for &x in &sp {
        for &y in &sp {
            case(&mut e, [x, y, 0.0], [0.0; 3], "special");
            case(&mut e, [0.0, y, x], [-0.0; 3], "special, other axes");
        }
    }
    for i in 0..3000 {
        let scale = if (next(&mut s) >> 8).is_multiple_of(2) {
            3.5
        } else {
            20.0
        };
        let (base, sub) = (fvec(&mut s, scale), fvec(&mut s, scale));
        case(&mut e, base, sub, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&0x00), "out of range never seen");
    assert!(seen.contains(&0x01), "in range (C0) never seen");
    assert!(seen.contains(&0x40), "equal (C3) never seen");
    assert!(seen.contains(&0x41), "unordered never seen");
}

#[test]
fn interact_reach_5df2a0_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let target = e.alloc(0x200);
    let this_radius_struct = e.alloc(0x200);
    let tgt_radius_struct = e.alloc(0x200);
    let this_vt = e.alloc(0x40);
    let tgt_vt = e.alloc(0x40);
    let this_pos_va = e.alloc(0x20);
    let tgt_pos_va = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;

    // The two `GetPosition` calls return the planted vectors.
    let this_getpos = e.alloc(0x10);
    let tgt_getpos = e.alloc(0x10);
    e.write(this_getpos, &[0xc3]);
    e.write(tgt_getpos, &[0xc3]);
    e.write_u32(this + 0x110, this_radius_struct as u32);
    e.write_u32(target + 0x110, tgt_radius_struct as u32);
    e.write_u32(this, this_vt as u32);
    e.write_u32(target, tgt_vt as u32);
    e.write_u32(this_vt + 0x14, this_getpos as u32);
    e.write_u32(tgt_vt + 0x14, tgt_getpos as u32);
    e.stub(this_getpos, this_pos_va as u32, 1);
    e.stub(tgt_getpos, tgt_pos_va as u32, 1);

    let mut s = 0x5df2_a000u32;
    let mut seen = std::collections::BTreeSet::new();
    let mut case =
        |e: &mut Emu, this_r: f32, tgt_r: f32, this_pos: Vec3, tgt_pos: Vec3, ctx: &str| {
            e.write_u32(this_radius_struct + 0x1f0, this_r.to_bits());
            e.write_u32(tgt_radius_struct + 0x1f0, tgt_r.to_bits());
            wvec(e, this_pos_va, this_pos);
            wvec(e, tgt_pos_va, tgt_pos);
            e.set_reg(RegisterX86::ESI, this);
            e.set_reg(RegisterX86::EDI, target);
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, ebp - 0x20);
            e.clear_calls();
            e.run(0x5df301, 0x5df389);
            let masked = ah(e) & 0x5;
            let bin_near = masked.count_ones().is_multiple_of(2);
            assert_eq!(
            bin_near,
            interact_reach_5df2a0(this_r, tgt_r, this_pos, tgt_pos),
            "{ctx}: ah&5={masked:#04x} radii {this_r} {tgt_r} this={this_pos:?} target={tgt_pos:?}"
        );
            assert_f32(
                e.read_f32(ebp - 4),
                interact_reach_value(this_r, tgt_r),
                &format!("{ctx}: reach of {this_r} {tgt_r}"),
            );
            seen.insert(masked);
        };

    case(&mut e, 1.0, 1.0, [0.0; 3], [0.0; 3], "coincident");
    case(&mut e, 1.0, 1.0, [0.0; 3], [100.0, 0.0, 0.0], "far");
    case(
        &mut e,
        0.0,
        0.0,
        [0.0; 3],
        [5.0, 0.0, 0.0],
        "reach floored, distance 5",
    );
    case(
        &mut e,
        1.0,
        1.0,
        [f32::NAN, 0.0, 0.0],
        [0.0; 3],
        "nan distance",
    );
    // The reach that is exactly 5.0 on the stack: 3 + h + 1.3333334 with h the f32 that completes it.
    let h = (5.0 - 3.0 - f64::from(REACH_5DF2A0_SLOP)) as f32;
    assert_eq!(3.0 + f64::from(h) + f64::from(REACH_5DF2A0_SLOP), 5.0);
    for r in [h, next_up(h), next_down(h)] {
        case(&mut e, r, 3.0, [0.0; 3], [5.0, 0.0, 0.0], "reach at 5.0");
        case(
            &mut e,
            r,
            3.0,
            [0.0; 3],
            [next_up(5.0), 0.0, 0.0],
            "reach at 5.0, farther",
        );
    }
    // The reach squared against the squared distance: a distance of reach exactly, and a step over.
    let reach = interact_reach_value(2.0, 4.0);
    for d in [reach, next_up(reach), next_down(reach)] {
        case(
            &mut e,
            2.0,
            4.0,
            [0.0; 3],
            [d, 0.0, 0.0],
            "distance at the reach",
        );
    }
    let sp = specials(&[2.0]);
    for &a in &sp {
        for &b in &sp {
            case(&mut e, a, b, [0.0; 3], [7.0, 0.0, 0.0], "special radii");
        }
    }
    for &x in &specials(&[5.0]) {
        case(
            &mut e,
            0.0,
            0.0,
            [x, 0.0, 0.0],
            [0.0; 3],
            "special distance",
        );
        case(
            &mut e,
            0.0,
            0.0,
            [0.0, 0.0, -0.0],
            [0.0, x, 0.0],
            "special distance, y",
        );
    }
    for i in 0..3000 {
        let this_r = frand(&mut s, 0.0, 6.0);
        let tgt_r = frand(&mut s, 0.0, 6.0);
        let scale = if (next(&mut s) >> 8).is_multiple_of(2) {
            9.0
        } else {
            25.0
        };
        let this_pos = fvec(&mut s, scale);
        let tgt_pos = fvec(&mut s, scale);
        case(
            &mut e,
            this_r,
            tgt_r,
            this_pos,
            tgt_pos,
            &format!("fuzz{i}"),
        );
    }
    assert!(
        seen.iter().any(|m| m.count_ones().is_multiple_of(2)),
        "near never seen"
    );
    assert!(
        seen.iter().any(|m| !m.count_ones().is_multiple_of(2)),
        "far never seen"
    );
}

/// The runtime threshold `_DAT_00c4c28c` the three dialogs compare against.
const C4C28C: u64 = 0xc4c28c;

/// Runs a dialog's compare slice with `eax` the target's position and `esi` this player's, and
/// returns the status byte masked with `0x41`.
fn range_slice(
    e: &mut Emu,
    span: (u64, u64),
    vas: (u64, u64),
    base: Vec3,
    sub: Vec3,
    ebp: u64,
) -> u8 {
    wvec(e, vas.0, base);
    wvec(e, vas.1, sub);
    e.set_reg(RegisterX86::EAX, vas.0);
    e.set_reg(RegisterX86::ESI, vas.1);
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, ebp - 0x20);
    e.run(span.0, span.1);
    ah(e) & 0x41
}

type GateFn = fn(Vec3, Vec3, f32) -> bool;

/// The three dialogs: where the compare slice runs and the function that transcribes it.
const DIALOGS: [(u64, u64, GateFn, &str); 3] = [
    (0x5df7a1, 0x5df7d2, range_gate_5df730, "0x5df730"),
    (0x5df9dc, 0x5dfa0d, range_gate_5df980, "0x5df980"),
    (0x5dfbfc, 0x5dfc2d, range_gate_5dfba0, "0x5dfba0"),
];

#[test]
fn range_gate_5df730_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let basevec = e.alloc(0x20);
    let subvec = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5df7_3000u32;
    let (start, end, ourfn, label) = DIALOGS[0];
    let span = (start, end);
    let vas = (basevec, subvec);

    let mut seen = std::collections::BTreeSet::new();
    for &thr in &[25.0f32, 36.0, 100.0, 0.0] {
        e.write_u32(C4C28C, thr.to_bits());
        for i in 0..900 {
            let scale = if (next(&mut s) >> 8).is_multiple_of(2) {
                thr.sqrt() * 1.3 + 1.0
            } else {
                thr.sqrt() * 0.3 + 0.5
            };
            let base = fvec(&mut s, scale);
            let sub = fvec(&mut s, scale);
            let masked = range_slice(&mut e, span, vas, base, sub, ebp);
            assert_eq!(
                masked != 0,
                ourfn(sub, base, thr),
                "{label} thr={thr} fuzz{i}: ah={masked:#04x}"
            );
            seen.insert(masked);
        }
    }
    assert!(seen.contains(&0x00), "out never seen");
    assert!(seen.contains(&0x01), "in (C0) never seen");
}

#[test]
fn range_gate_5df980_clone_family_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let basevec = e.alloc(0x20);
    let subvec = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5df9_8000u32;
    let vas = (basevec, subvec);

    let mut seen = std::collections::BTreeSet::new();
    for &(start, end, ourfn, label) in &DIALOGS {
        let span = (start, end);
        let mut case = |e: &mut Emu, thr: f32, base: Vec3, sub: Vec3, ctx: &str| {
            e.write_u32(C4C28C, thr.to_bits());
            let masked = range_slice(e, span, vas, base, sub, ebp);
            assert_eq!(
                masked != 0,
                ourfn(sub, base, thr),
                "{label} {ctx}: thr={thr} ah={masked:#04x} target={base:?} this={sub:?}"
            );
            seen.insert((label, masked));
        };
        // The threshold equal to the squared distance and a step either side; NaN in either.
        for thr in [
            25.0f32,
            next_up(25.0),
            next_down(25.0),
            0.0,
            -0.0,
            f32::NAN,
            f32::INFINITY,
        ] {
            for p in axis_edges(5.0) {
                case(&mut e, thr, p, [0.0; 3], "edge");
                case(&mut e, thr, [0.0; 3], p, "edge, swapped");
            }
            case(&mut e, thr, [f32::NAN, 0.0, 0.0], [0.0; 3], "nan position");
            case(&mut e, thr, [0.0; 3], [0.0; 3], "coincident");
        }
        for &thr in &specials(&[25.0]) {
            for &x in &specials(&[]) {
                case(&mut e, thr, [x, 3.0, 4.0], [0.0; 3], "special");
            }
        }
        for &thr in &[25.0f32, 36.0, 100.0, 0.0] {
            for i in 0..400 {
                let scale = if (next(&mut s) >> 8).is_multiple_of(2) {
                    thr.sqrt() * 1.3 + 1.0
                } else {
                    thr.sqrt() * 0.3 + 0.5
                };
                let base = fvec(&mut s, scale);
                let sub = fvec(&mut s, scale);
                case(&mut e, thr, base, sub, &format!("fuzz{i}"));
            }
        }
    }
    for (_, _, _, label) in DIALOGS {
        assert!(seen.contains(&(label, 0x00)), "{label}: out never seen");
        assert!(seen.contains(&(label, 0x01)), "{label}: in (C0) never seen");
        assert!(seen.contains(&(label, 0x40)), "{label}: equal never seen");
        assert!(
            seen.contains(&(label, 0x41)),
            "{label}: unordered never seen"
        );
    }
}

/// The skill is an `i32` and the scale 0.05, so the `__ftol` input is at most about 1.1e8 in
/// magnitude and never NaN or out of range; the `i32` extremes are among the cases.
#[test]
fn skill_rank_scale_5df730_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x400);
    let ebp = frame + 0x300;
    let mut s = 0x5df8_1e00u32;

    let case = |e: &mut Emu, skill: i32, ctx: &str| {
        e.write_u32(ebp + 8, skill as u32);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.run(0x5df81e, 0x5df82c);
        let bin = e.get_reg(RegisterX86::EAX) as i32;
        assert_eq!(
            bin,
            skill_rank_scale_5df730(skill),
            "{ctx}: skill={skill} bin={bin}"
        );
    };

    for (skill, ctx) in [
        (0, "zero"),
        (100, "100 gives 5"),
        (300, "300 gives 15"),
        (1, "1 truncates to 0"),
        (19, "19 * 0.05 is 0.95"),
        (20, "20 * 0.05 is just over 1"),
        (-1, "-1 truncates toward zero"),
        (-19, "-19"),
        (-20, "-20"),
        (-50, "negative"),
        (i32::MAX, "i32::MAX"),
        (i32::MIN, "i32::MIN"),
    ] {
        case(&mut e, skill, ctx);
    }
    for k in 1..60 {
        case(&mut e, 20 * k, "multiples of 20");
        case(&mut e, 20 * k - 1, "one below a multiple of 20");
        case(&mut e, -20 * k, "negative multiples of 20");
    }
    for i in 0..3000 {
        let skill = (next(&mut s) >> 8) as i32 % 30000 - 5000;
        case(&mut e, skill, &format!("fuzz{i}"));
    }
    for i in 0..500 {
        case(&mut e, next(&mut s) as i32, &format!("full range {i}"));
    }
}

#[test]
fn area_trigger_kind_5e22d0_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x40);
    let mut s = 0x5e22_d100u32;
    let mut seen = std::collections::BTreeSet::new();

    // `fcomp radius, 0.0; fnstsw; test ah,0x44; jnp` goes to the box on an odd number of C2 and C3.
    let mut case = |e: &mut Emu, radius: f32| {
        e.write_u32(rec + 0x14, radius.to_bits());
        e.set_reg(RegisterX86::ESI, rec);
        e.run(0x5e22ea, 0x5e22f5);
        let box_path = !(ah(e) & 0x44).count_ones().is_multiple_of(2);
        assert_eq!(
            box_path,
            area_trigger_is_box(radius),
            "radius {radius}: ah={:#04x}",
            ah(e)
        );
        seen.insert(box_path);
    };
    for &r in &specials(&[]) {
        case(&mut e, r);
    }
    for _ in 0..1000 {
        case(&mut e, frand_bits(&mut s));
    }
    assert!(seen.contains(&true) && seen.contains(&false));
}

#[test]
fn area_trigger_sphere_5e22d0_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x40);
    let point = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5e22_d000u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, center: Vec3, pt: Vec3, radius: f32, ctx: &str| {
        e.write_u32(rec + 0x8, center[0].to_bits());
        e.write_u32(rec + 0xc, center[1].to_bits());
        e.write_u32(rec + 0x10, center[2].to_bits());
        e.write_u32(rec + 0x14, radius.to_bits());
        wvec(e, point, pt);
        e.set_reg(RegisterX86::ESI, rec);
        e.set_reg(RegisterX86::EDI, point);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x5e22fa, 0x5e2331);
        let masked = ah(e) & 0x1;
        assert_eq!(
            masked == 0,
            area_trigger_sphere_5e22d0(center, pt, radius),
            "{ctx}: ah&1={masked:#04x} centre={center:?} point={pt:?} radius={radius}"
        );
        seen.insert(masked);
    };

    case(&mut e, [0.0; 3], [0.0; 3], 5.0, "coincident");
    case(&mut e, [0.0; 3], [100.0, 0.0, 0.0], 5.0, "far");
    case(&mut e, [0.0; 3], [5.0, 0.0, 0.0], 5.0, "on the surface");
    case(&mut e, [0.0; 3], [f32::NAN, 0.0, 0.0], 5.0, "nan");
    for p in axis_edges(5.0) {
        case(&mut e, [0.0; 3], p, 5.0, "edge");
    }
    for r in [next_up(5.0), next_down(5.0), -5.0] {
        case(&mut e, [0.0; 3], [5.0, 0.0, 0.0], r, "radius edge");
    }
    let sp = specials(&[5.0]);
    for &r in &sp {
        for &x in &sp {
            case(&mut e, [0.0; 3], [x, 0.0, 0.0], r, "special");
            case(
                &mut e,
                [-0.0, x, 0.0],
                [0.0, 0.0, 3.0],
                r,
                "special, other axes",
            );
        }
    }
    for i in 0..3000 {
        let radius = frand(&mut s, 0.5, 30.0);
        let scale = if (next(&mut s) >> 8).is_multiple_of(2) {
            radius * 1.2
        } else {
            radius * 0.5
        };
        let center = fvec(&mut s, scale);
        let pt = fvec(&mut s, scale);
        case(&mut e, center, pt, radius, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&0x00), "inside never seen");
    assert!(seen.contains(&0x01), "outside never seen");
}

#[test]
fn drunk_fraction_5e2a90_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let fields = e.alloc(0x40);
    let obj = e.alloc(0x40);
    let scratch = e.alloc(0x20);
    // `mov al,[edx+0x1d]` reads the player's level byte; `mov ecx,[edi+0x8]; mov esi,[ecx]` read
    // a GUID the later check uses.
    e.write_u32(obj + 8, scratch as u32);
    let mut s = 0x5e2a_9000u32;

    let case = |e: &mut Emu, level: u8, ctx: &str| {
        e.write(fields + 0x1d, &[level]);
        e.set_reg(RegisterX86::EDX, fields);
        e.set_reg(RegisterX86::EDI, obj);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x5e2ac9, 0x5e2aee);
        let bin = e.read_f32(ebp + 0xc);
        assert_f32(
            bin,
            drunk_fraction_5e2a90(level),
            &format!("{ctx}: level {level}"),
        );
    };

    for level in 0..=255u8 {
        case(&mut e, level, "every byte");
    }
    for i in 0..500u32 {
        let level = ((next(&mut s) >> 8) & 0xff) as u8;
        case(&mut e, level, &format!("fuzz{i}"));
    }
}
