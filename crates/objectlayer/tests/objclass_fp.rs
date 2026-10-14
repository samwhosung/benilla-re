//! The float leaves of `src/objclass_fp.rs` against the exe. `0x5f5c40`, `0x5f8cb0`, `0x5fc310`
//! and `0x5fc350` run whole. `0x5fc350` calls `0x7c4c90` with `[sub+0x40] & 0xf == 0`, which takes
//! its simple path and returns 0.0, so the timer is 0 and `2 * local` is the compared quantity.
//! Three are slices read at their `fnstsw ax`: the low-health ratio of `0x5fbd60`
//! (`[0x5fbd8f, 0x5fbda6)`), the z negate of `0x5fac70` (`[0x5facd4, 0x5facdd)`) and the chair
//! range of `0x5f5670` (`[0x5f56fa, 0x5f5724)`, with the threshold at `0xc4d808` set directly);
//! for the two compares the AH byte masked by the branch's `test ah,5` is compared. Each test adds
//! NaN, the infinities, both zeros and the thresholds to its random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::objclass_fp::{
    cast_time_elapsed, interact_range_ah, low_health_ratio_ah, negate_z, normalize_vec2,
    scale_vec3, transform_scale_copy,
};
use common::*;

/// The mask of the distance and ratio gates: `test ah,5` is C0 (0x01) or C2 (0x04). C3 (0x40) is
/// not tested, so equal and greater give the same branch.
const TEST_AH_5: u8 = 0x05;

#[test]
fn diff_scale_vec3() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x80);
    let out = e.alloc(0x20);
    let mut s = 0x5ca1e_u32;

    let case = |e: &mut Emu, scal: f32, v: [f32; 3], ctx: &str| {
        e.write_u32(this + 0x38, scal.to_bits());
        e.write_u32(this + 0x3c, v[0].to_bits());
        e.write_u32(this + 0x40, v[1].to_bits());
        e.write_u32(this + 0x44, v[2].to_bits());
        e.call(0x5f5c40, this as u32, 0, &[out as u32]);
        let bin = [e.read_f32(out), e.read_f32(out + 4), e.read_f32(out + 8)];
        let ours = scale_vec3(scal, v);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} comp{k}"));
        }
    };

    // targeted: zero scalar, unit scalar, negatives, the rounding edge.
    case(&mut e, 0.0, [1.0, -2.0, 3.5], "zero-s");
    case(&mut e, 1.0, [1.1, 2.2, 3.3], "unit-s");
    case(&mut e, -1.5, [10.0, -20.0, 0.0], "neg-s");
    case(&mut e, 0.1, [0.3, 0.3, 0.3], "round-edge");
    case(&mut e, 16_777_217.0, [0.5, 0.5, 0.5], "ulp-edge");
    for &v in &specials(&[]) {
        case(&mut e, v, [1.0, -2.0, 3.0], "special scalar");
        case(&mut e, 2.0, [v, v, v], "special vector");
    }
    // fuzz: broad magnitudes + full-bit-space (NaN/inf/subnormal) operands.
    for i in 0..400 {
        let scal = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1e6, 1e6)
        };
        let v = [
            frand(&mut s, -1e5, 1e5),
            frand_bits(&mut s),
            frand(&mut s, -1e3, 1e3),
        ];
        case(&mut e, scal, v, &format!("fuzz{i}"));
    }
}

#[test]
fn diff_normalize_vec2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vec = e.alloc(0x10);
    let mut s = 0xbeef_u32;
    let mut below = 0u32;
    let mut above = 0u32;

    let case = |e: &mut Emu, x: f32, y: f32, ctx: &str| {
        e.write_u32(vec, x.to_bits());
        e.write_u32(vec + 4, y.to_bits());
        e.call(0x5f8cb0, vec as u32, 0, &[]);
        let bin = [e.read_f32(vec), e.read_f32(vec + 4)];
        let ours = normalize_vec2(x, y);
        assert_f32(bin[0], ours[0], &format!("{ctx} x"));
        assert_f32(bin[1], ours[1], &format!("{ctx} y"));
    };

    // targeted: the eps dead-band (unchanged) and the normalize path.
    case(&mut e, 0.0, 0.0, "zero"); // len 0 < eps -> unchanged
    case(&mut e, 1e-6, 0.0, "tiny"); // len 1e-6 -> normalize (eps=2.4e-7)
    case(&mut e, 1e-9, 1e-9, "subeps"); // below eps -> unchanged
    case(&mut e, 3.0, 4.0, "345"); // classic -> (0.6, 0.8)
    case(&mut e, -3.0, 4.0, "neg");
    // The floor: a length of exactly `eps` and a step either side of it, along an axis, then
    // zeros of both signs, NaN and the infinities.
    let eps = benilla_re_objectlayer::objclass_fp::NORM2_EPS;
    for l in [eps, next_up(eps), next_down(eps)] {
        case(&mut e, l, 0.0, "eps along x");
        case(&mut e, 0.0, -l, "eps along y");
    }
    for &v in &specials(&[eps]) {
        case(&mut e, v, 1.0, "special x");
        case(&mut e, 1.0, v, "special y");
        case(&mut e, v, v, "special both");
    }

    for i in 0..600 {
        // mix tiny (straddles eps), normal, and full-bit-space inputs.
        let (x, y) = match i % 4 {
            0 => (frand(&mut s, -1e-6, 1e-6), frand(&mut s, -1e-6, 1e-6)),
            1 => (frand(&mut s, -100.0, 100.0), frand(&mut s, -100.0, 100.0)),
            2 => (frand_bits(&mut s), frand_bits(&mut s)),
            _ => (frand(&mut s, -1e8, 1e8), frand(&mut s, -1e8, 1e8)),
        };
        // Count which branch the binary takes (via our transcription's gate) for coverage.
        let len = (f64::from(x) * f64::from(x) + f64::from(y) * f64::from(y)).sqrt();
        if len.abs() < f64::from(benilla_re_objectlayer::objclass_fp::NORM2_EPS) {
            below += 1;
        } else if len.is_finite() {
            above += 1;
        }
        case(&mut e, x, y, &format!("fuzz{i}"));
    }
    assert!(below > 0, "eps-dead-band branch never exercised");
    assert!(above > 0, "normalize branch never exercised");
}

#[test]
fn diff_transform_scale_copy() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x1000); // need up to +0xc6c
    let src = e.alloc(0x40);
    e.write_u32(this + 0xb3c, src as u32); // [this+0xb3c] = sub-object ptr
    let mut s = 0xc0ffee_u32;

    let case = |e: &mut Emu, d18: u32, d1c: f32, d20: f32, d24: u32, ctx: &str| {
        e.write_u32(src + 0x18, d18);
        e.write_u32(src + 0x1c, d1c.to_bits());
        e.write_u32(src + 0x20, d20.to_bits());
        e.write_u32(src + 0x24, d24);
        e.call(0x5fc310, this as u32, 0, &[]);
        let (c5c, c64, c68, c6c) = transform_scale_copy(d18, d1c, d20, d24);
        assert_eq!(e.read_u32(this + 0xc5c), c5c, "{ctx} c5c copy");
        assert_f32(e.read_f32(this + 0xc64), c64, &format!("{ctx} c64 scale"));
        assert_f32(e.read_f32(this + 0xc68), c68, &format!("{ctx} c68 scale"));
        assert_eq!(e.read_u32(this + 0xc6c), c6c, "{ctx} c6c copy");
    };

    case(&mut e, 0x1234_5678, 36.0, 72.0, 0xdead_beef, "basic"); // 72/36=2, 36/36=1
    case(&mut e, 0, 0.0, 0.0, 0, "zeros");
    case(&mut e, 0xffff_ffff, -360.0, 360.0, 0x4000_0000, "neg");
    for &v in &specials(&[]) {
        case(&mut e, 1, v, 36.0, 2, "special 1c");
        case(&mut e, 1, 36.0, v, 2, "special 20");
    }
    for i in 0..400 {
        case(
            &mut e,
            next(&mut s),
            frand(&mut s, -1e5, 1e5),
            frand_bits(&mut s),
            next(&mut s),
            &format!("fuzz{i}"),
        );
    }
}

#[test]
fn diff_cast_time_elapsed() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x1000); // need up to +0xa30 and the +0x9a8 sub-object
    let mut s = 0x71_1e_u32;
    let timer = 0.0_f32; // 0x7c4c90's _DAT_007ffd74 simple-path return

    let mut both = [0u32; 2]; // [#false, #true]
    let mut case = |e: &mut Emu, local: f32, ctx: &str| {
        // CMovement sub-object at this+0x9a8: [sub+0x40] flags with low nibble 0 -> simple path.
        e.write_u32(this + 0x9a8 + 0x40, 0x0000_0010); // dl & 0xf == 0
        e.write_u32(this + 0xa30, local.to_bits());
        let bin = e.call(0x5fc350, this as u32, 0, &[]) & 1;
        let ours = cast_time_elapsed(local, timer);
        assert_eq!(
            bin == 1,
            ours,
            "{ctx}: binary={bin} ours={ours} (local={local}, 2*local vs timer=0)"
        );
        both[bin as usize] += 1;
    };

    // local <= 0 -> 2*local <= 0 <= timer(0) -> true; local > 0 -> 2*local > 0 > ... -> depends.
    case(&mut e, 0.0, "zero"); // 2*0=0, 0>=0 -> true
    case(&mut e, -1.0, "neg"); // 2*-1=-2, 0>=-2 -> true
    case(&mut e, 1.0, "pos"); // 2*1=2, 0>=2 false
    case(&mut e, -0.0, "negzero");
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
    for i in 0..400 {
        let local = match i % 3 {
            0 => frand(&mut s, -1e3, 1e3),
            1 => frand(&mut s, -1e-3, 1e-3),
            _ => frand_bits(&mut s),
        };
        case(&mut e, local, &format!("fuzz{i}"));
    }
    assert!(
        both[0] > 0 && both[1] > 0,
        "one branch never fired: {both:?}"
    );
}

#[test]
fn diff_low_health_ratio() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let fields = e.alloc(0x100);
    e.write_u32(this + 0x110, fields as u32);
    let mut s = 0x10_5ea1_u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, hp: i32, max: i32, ctx: &str| {
        e.write_u32(fields + 0x40, hp as u32);
        e.write_u32(fields + 0x58, max as u32);
        e.set_reg(RegisterX86::ESI, this);
        e.run(0x5fbd8f, 0x5fbda6); // stop just after `fnstsw ax`
        let bin = ah(e) & TEST_AH_5;
        let ours = low_health_ratio_ah(hp, max) & TEST_AH_5;
        assert_eq!(
            bin, ours,
            "{ctx}: binary AH&5={bin:#04x} ours={ours:#04x} (hp={hp} max={max})"
        );
        seen.insert(bin);
    };

    // Ratios under and over 0.2, a zero divisor (+inf) and 0/0 (unordered, C0|C2).
    case(&mut e, 10, 100, "low"); // 0.1 < 0.2 -> C0
    case(&mut e, 50, 100, "high"); // 0.5 > 0.2 -> 0
    case(&mut e, 1, 5, "one fifth"); // 0.2 as f64 is below the f32 threshold 0.2, so C0
    case(&mut e, 100, 0, "div0"); // health/0 -> +inf > 0.2 -> 0
    case(&mut e, 0, 0, "zerozero"); // 0/0 -> NaN unordered -> C0|C2
    case(&mut e, -10, 100, "neghp"); // -0.1 < 0.2 -> C0
    for &h in &[0, 1, -1, 5, 20, 21, 100, i32::MAX, i32::MIN] {
        for &m in &[0, 1, -1, 5, 100, 105, i32::MAX, i32::MIN] {
            case(&mut e, h, m, "ints");
        }
    }
    for i in 0..500 {
        let hp = (next(&mut s) >> 8) as i32 % 50_000 - 5_000;
        let max = match i % 4 {
            0 => 0,
            _ => (next(&mut s) >> 8) as i32 % 50_000,
        };
        case(&mut e, hp, max, &format!("fuzz{i}"));
    }
    // coverage over the branch-visible outcomes: C0 (ratio<0.2), 0 (ratio>=0.2), C0|C2 (unordered).
    assert!(seen.contains(&0x01), "ratio<0.2 (C0) never seen");
    assert!(seen.contains(&0x00), "ratio>=0.2 never seen");
    assert!(seen.contains(&0x05), "unordered (C0|C2) never seen");
}

#[test]
fn diff_negate_z() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40); // ebp = frame (so [ebp-4],[ebp-8] are valid)
    let ebp = frame + 0x20;
    let posvec = e.alloc(0x20); // eax = posvec; pos.z at [eax+8]
    let mut s = 0x2_e60_u32;

    let case = |e: &mut Emu, z: f32, ctx: &str| {
        e.write_u32(ebp - 4, 0.0_f32.to_bits()); // [ebp-4] = 0.0 (the mov imm the body did earlier)
        e.write_u32(posvec + 8, z.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::EAX, posvec);
        e.run(0x5facd4, 0x5facdd); // fld;fsub;fstp -> [ebp-8]
        let bin = e.read_f32(ebp - 8);
        let ours = negate_z(z);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 0.0, "zero");
    case(&mut e, 5.5, "pos");
    case(&mut e, -42.25, "neg");
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
    for i in 0..400 {
        let z = if i % 2 == 0 {
            frand(&mut s, -1e6, 1e6)
        } else {
            frand_bits(&mut s)
        };
        case(&mut e, z, &format!("fuzz{i}"));
    }
}

const THRESH_GLOBAL: u64 = 0xc4d808;

#[test]
fn diff_interact_range() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let posvec = e.alloc(0x20);
    let database = e.alloc(0x20);
    let esi = database + 0x8; // so [esi-8]=data.x, [esi-4]=data.y, [esi]=data.z
    let mut s = 0x5_eed0_u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, pos: [f32; 3], data: [f32; 3], thr: f32, ctx: &str| {
        for k in 0..3 {
            e.write_u32(posvec + 4 * k as u64, pos[k].to_bits());
            e.write_u32(database + 4 * k as u64, data[k].to_bits());
        }
        e.write_u32(THRESH_GLOBAL, thr.to_bits());
        e.set_reg(RegisterX86::EAX, posvec);
        e.set_reg(RegisterX86::ESI, esi);
        e.run(0x5f56fa, 0x5f5724); // distance compute + fcomp + fnstsw ax
        let bin = ah(e) & TEST_AH_5;
        let ours = interact_range_ah(pos, data, thr) & TEST_AH_5;
        assert_eq!(bin, ours, "{ctx}: binary AH&5={bin:#04x} ours={ours:#04x}");
        seen.insert(bin);
    };

    // dist² < thr (C0 in-range), > thr (0 out), == thr (C3, invisible to test ah,5 -> 0).
    case(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 25.0, "atpoint"); // 0 < 25 -> C0
    case(&mut e, [10.0, 0.0, 0.0], [0.0, 0.0, 0.0], 25.0, "far"); // 100 > 25 -> 0
    case(&mut e, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], 25.0, "edge"); // 25 == 25 -> C3 -> 0
    case(&mut e, [1.0, 2.0, 2.0], [0.0, 0.0, 0.0], 9.0, "exact9"); // 1+4+4=9 == 9 -> C3 -> 0
                                                                   // The threshold equal to the squared distance and a step either side, then NaN, infinities
                                                                   // and zeros in the threshold and each coordinate.
    for t in [9.0f32, next_up(9.0), next_down(9.0)] {
        case(
            &mut e,
            [1.0, 2.0, 2.0],
            [0.0, 0.0, 0.0],
            t,
            "threshold step",
        );
    }
    for &v in &specials(&[9.0]) {
        case(
            &mut e,
            [1.0, 2.0, 2.0],
            [0.0, 0.0, 0.0],
            v,
            "special threshold",
        );
        for k in 0..3 {
            let mut p = [1.0f32, 2.0, 2.0];
            p[k] = v;
            case(&mut e, p, [0.0, 0.0, 0.0], 9.0, "special pos");
            let mut d = [0.0f32; 3];
            d[k] = v;
            case(&mut e, [1.0, 2.0, 2.0], d, 9.0, "special data");
        }
    }
    for i in 0..600 {
        let pos = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let data = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        // thresholds spanning below/at/above the typical squared distances, plus full-bit edges.
        let thr = match i % 3 {
            0 => frand(&mut s, 0.0, 10_000.0),
            1 => frand(&mut s, 0.0, 50.0),
            _ => f32::from_bits(next(&mut s)).abs(),
        };
        case(&mut e, pos, data, thr, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&0x01), "in-range (C0) never seen");
    assert!(seen.contains(&0x00), "out-of-range never seen");
}
