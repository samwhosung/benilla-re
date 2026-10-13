//! The map object pieces against `0x69a2a0`, `0x6c0bb0` and `0x68bda0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::mapobj::{
    falloff_scale, light_bounds_refresh, liquid_tex_matrix, AspectBranch,
};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes).
const SLICE_ESP: u64 = 0x7008_0000;

/// `test ah,5` mask (C0|C2).
const TEST_AH_5: u8 = 0x5;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any float, NaN and the infinities included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// AH after `fnstsw ax`.
fn ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// `BuildFootprintMatrix` (`0x69a2a0`) called whole with `RotateMatrix` (`0x7bdd60`) stubbed: the
/// matrix and the angle passed on; `ScaleMatrixRows` (`0x7bdca0`) runs.
#[test]
fn liquid_tex_matrix_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // `RotateMatrix`, which takes the negated angle first
    e.stub(0x7bdd60, 0, 3);
    let out = e.alloc(0x40);
    let aspect = e.alloc(0x10);
    let mut s = 0x69a2_a000u32;
    let mut branches = std::collections::BTreeSet::new();
    let mut checked = 0u32;

    let case = |e: &mut Emu, mirror: bool, angle: f32, asp: [f32; 2], ctx: &str| {
        e.clear_calls();
        e.write_u32(aspect, asp[0].to_bits());
        e.write_u32(aspect + 4, asp[1].to_bits());
        for i in 0..16u64 {
            e.write_u32(out + i * 4, 0xdead_beef);
        }
        let edx = u32::from(mirror);
        e.call(0x69a2a0, out as u32, edx, &[angle.to_bits(), aspect as u32]);
        let bin_mat: [f32; 16] = std::array::from_fn(|i| e.read_f32(out + i as u64 * 4));
        let calls = e.calls();
        assert_eq!(calls.len(), 1, "{ctx}: expected exactly one rotZ stub call");
        let bin_neg_angle = f32::from_bits(calls[0].args[0]);

        let ours = liquid_tex_matrix(mirror, angle, asp);
        for (k, (&b, &o)) in bin_mat.iter().zip(ours.matrix.iter()).enumerate() {
            assert_f32(b, o, &format!("{ctx} mat[{k}]"));
        }
        assert_f32(bin_neg_angle, ours.neg_angle, &format!("{ctx} neg_angle"));
        ours.branch
    };

    // each aspect branch, mirrored and not
    branches.insert(case(
        &mut e,
        false,
        0.5,
        [3.0, 4.0],
        "no-mirror a<b (ScaleY=b/a)",
    ));
    branches.insert(case(
        &mut e,
        true,
        1.25,
        [4.0, 3.0],
        "mirror a>b (ScaleX=a/b)",
    ));
    branches.insert(case(&mut e, false, -2.0, [2.0, 2.0], "a==b (None)"));
    branches.insert(case(&mut e, true, 0.0, [5.0, 5.0], "mirror a==b (None)"));
    branches.insert(case(&mut e, false, 2.5, [1.0, 1000.0], "a<<b (ScaleY)"));
    branches.insert(case(&mut e, true, 1.0, [1000.0, 1.0], "a>>b (ScaleX)"));

    for i in 0..3000u32 {
        let mirror = (next(&mut s) >> 8).is_multiple_of(2);
        let angle = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -10.0, 10.0)
        };
        let asp = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [frand(&mut s, 0.1, 100.0), frand(&mut s, 0.1, 100.0)]
        };
        branches.insert(case(&mut e, mirror, angle, asp, &format!("fuzz{i}")));
        checked += 1;
    }
    assert!(
        branches.contains(&AspectBranch::ScaleY),
        "ScaleY (a<b) branch never exercised"
    );
    assert!(
        branches.contains(&AspectBranch::ScaleX),
        "ScaleX (a>b) branch never exercised"
    );
    assert!(
        branches.contains(&AspectBranch::None),
        "None (a==b/NaN) branch never exercised"
    );
    eprintln!(
        "liquid_tex_matrix: {checked} cases + 6 directed bit-exact vs WoW.exe @ 0x69a2a0 (rotZ 0x7bdd60 stubbed)"
    );
}

/// `CMapLight::UpdateBounds` (`0x6c0bb0`) called whole.
#[test]
fn light_bounds_refresh_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let light = e.alloc(0x100);
    let mut s = 0x6c0b_b000u32;
    let (mut kept, mut replaced) = (0u32, 0u32);
    let mut checked = 0u32;

    let mut case = |e: &mut Emu, radius_in: f32, falloff: f32, pos: [f32; 3], ctx: &str| {
        for off in (0x20u64..0x70).step_by(4) {
            e.write_u32(light + off, 0xdead_beef);
        }
        e.write_u32(light + 0xec, radius_in.to_bits());
        e.write_u32(light + 0xf0, falloff.to_bits());
        e.write_u32(light + 0x88, pos[0].to_bits());
        e.write_u32(light + 0x8c, pos[1].to_bits());
        e.write_u32(light + 0x90, pos[2].to_bits());
        e.call(0x6c0bb0, light as u32, 0, &[]);

        let bin = benilla_re_terrain::mapobj::LightBounds {
            radius: e.read_f32(light + 0xec),
            center: [
                e.read_f32(light + 0x24),
                e.read_f32(light + 0x28),
                e.read_f32(light + 0x2c),
            ],
            aabb_min: [
                e.read_f32(light + 0x44),
                e.read_f32(light + 0x48),
                e.read_f32(light + 0x4c),
            ],
            aabb_max: [
                e.read_f32(light + 0x50),
                e.read_f32(light + 0x54),
                e.read_f32(light + 0x58),
            ],
            sphere_center: [
                e.read_f32(light + 0x5c),
                e.read_f32(light + 0x60),
                e.read_f32(light + 0x64),
            ],
            sphere_radius: e.read_f32(light + 0x68),
        };
        let ours = light_bounds_refresh(radius_in, falloff, pos);
        assert_f32(bin.radius, ours.radius, &format!("{ctx} radius"));
        assert_f32(
            bin.sphere_radius,
            ours.sphere_radius,
            &format!("{ctx} sphere_radius"),
        );
        for k in 0..3 {
            assert_f32(bin.center[k], ours.center[k], &format!("{ctx} center[{k}]"));
            assert_f32(
                bin.aabb_min[k],
                ours.aabb_min[k],
                &format!("{ctx} min[{k}]"),
            );
            assert_f32(
                bin.aabb_max[k],
                ours.aabb_max[k],
                &format!("{ctx} max[{k}]"),
            );
            assert_f32(
                bin.sphere_center[k],
                ours.sphere_center[k],
                &format!("{ctx} sphere_center[{k}]"),
            );
        }
        // 5.0 taken in place of the radius
        if ours.radius == 5.0 && radius_in != 5.0 {
            replaced += 1;
        } else if ours.radius == radius_in {
            kept += 1;
        }
    };

    // a falloff equal to 0.0, either sign, takes 5.0
    case(&mut e, 12.5, 0.0, [1.0, 2.0, 3.0], "falloff 0.0");
    case(&mut e, 12.5, -0.0, [1.0, 2.0, 3.0], "falloff -0.0");
    // any other falloff, NaN included, keeps the radius
    case(
        &mut e,
        12.5,
        0.25,
        [10.0, -20.0, 30.0],
        "falloff=0.25 (keep r=12.5)",
    );
    case(
        &mut e,
        12.5,
        -1.0,
        [0.0, 0.0, 0.0],
        "falloff<0 (keep r=12.5)",
    );
    case(
        &mut e,
        12.5,
        f32::NAN,
        [1.0, 1.0, 1.0],
        "falloff NaN (keep r=12.5)",
    );

    for i in 0..3000u32 {
        let radius_in = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1e3, 1e3)
        };
        // a third of the falloffs exactly 0
        let falloff = if (next(&mut s) >> 8).is_multiple_of(3) {
            0.0
        } else if i % 7 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -10.0, 10.0)
        };
        let pos = if i % 6 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
            ]
        };
        case(&mut e, radius_in, falloff, pos, &format!("fuzz{i}"));
        checked += 1;
    }
    assert!(
        kept > 0 && replaced > 0,
        "both radius dispositions exercised (kept={kept} replaced={replaced})"
    );
    eprintln!(
        "light_bounds_refresh: {checked} cases + probes bit-exact vs WoW.exe @ 0x6c0bb0 (kept={kept} replaced={replaced})"
    );
}

/// `AddSplash` (`0x68bda0`) from `0x68be62` to `0x68be85`, the size at `[ebp+0x14]`.
#[test]
fn falloff_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s = 0x68bd_a000u32;
    let mut seen = std::collections::BTreeSet::new();
    let mut checked = 0u32;

    let ebp = SLICE_ESP;

    let mut case = |e: &mut Emu, param: f32, ctx: &str| {
        e.write_u32(ebp + 0x14, param.to_bits());
        e.write_u32(ebp + 0x1c, 0xdead_beef);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP - 0x40);
        e.run(0x68be62, 0x68be85);
        let bin_ah = ah(e) & TEST_AH_5;
        let bin_out = e.read_f32(ebp + 0x1c);
        let ours = falloff_scale(param);
        // `jp` skips the product on an even count of C0 and C2
        let bin_jp_taken = bin_ah.count_ones().is_multiple_of(2);
        assert_eq!(
            !bin_jp_taken, ours.scaled,
            "{ctx}: binary AH&5={bin_ah:#04x} (jp_taken={bin_jp_taken}) ours_scaled={}",
            ours.scaled
        );
        assert_f32(bin_out, ours.scale, &format!("{ctx} scale"));
        seen.insert(bin_ah);
    };

    // equal to 1/6 keeps 1.0
    let one_sixth = f32::from_bits(0x3e2a_aaab);
    case(&mut e, one_sixth, "equal (keep 1.0)");
    // just below scales
    case(
        &mut e,
        f32::from_bits(0x3e2a_aaaa),
        "param just<1/6 (scale)",
    );
    case(&mut e, 0.1, "param=0.1 (<1/6, scale)");
    case(&mut e, 0.0, "0 (scale)");
    case(&mut e, 0.5, "param=0.5 (>1/6, 1.0)");
    case(&mut e, 100.0, "param=100 (>1/6, 1.0)");
    case(&mut e, f32::NAN, "param NaN (unordered, 1.0)");
    case(&mut e, -3.0, "param=-3 (<1/6, scale=-18)");

    for i in 0..4000u32 {
        let param = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -0.5, 0.5)
        };
        case(&mut e, param, &format!("fuzz{i}"));
        checked += 1;
    }
    assert!(
        seen.contains(&0x00),
        "even parity (1.0: param>=1/6 or NaN) never seen"
    );
    assert!(seen.contains(&0x01), "C0 (param<1/6, scale) never seen");
    assert!(seen.contains(&0x05), "C0|C2 (NaN, 1.0) never seen");
    eprintln!(
        "falloff_scale: {checked} cases + probes bit-exact vs WoW.exe @ 0x68bda0 slice [0x68be62,0x68be85)"
    );
}
