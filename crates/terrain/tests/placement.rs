//! The placement arithmetic against `0x695650`, `0x698110` and `0x6a81d0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::placement::{
    basis_row_xform, bound_sphere, bounds_center, bounds_sum, chunk_fade_scale_twin,
    euler_deg_to_rad, euler_deg_to_rad_plus_pi, grid_index, grid_remainder, placement_origin_xform,
    recenter, world_add, BoundSphere,
};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes).
const SLICE_ESP: u64 = 0x7008_0000;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any float, NaN and the infinities included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// `PlaceMapObjDef` (`0x695650`): the two stretches without calls, from `0x6956e5` to `0x69578b` and
/// from `0x695854` to `0x695931`, ebx the entry, esi the def, `[ebp+8]` the origin.
#[test]
fn placement_setup_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let src = e.alloc(0x80);
    let dst = e.alloc(0x200);
    let base = e.alloc(0x10);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x6956_0000u32;

    // the origin and the three angles
    let block_a = |e: &mut Emu, src_f: &[(u64, f32)], base3: [f32; 3], ctx: &str| {
        for &(off, v) in src_f {
            e.write_u32(src + off, v.to_bits());
        }
        for (k, &v) in base3.iter().enumerate() {
            e.write_u32(base + 4 * k as u64, v.to_bits());
        }
        e.write_u32(ebp + 8, base as u32);
        e.set_reg(RegisterX86::EBX, src);
        e.set_reg(RegisterX86::ESI, dst);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6956e5, 0x69578b);
        let (v0, v1, v2) = (
            f32::from_bits(e.read_u32(src + 0x10)),
            f32::from_bits(e.read_u32(src + 0x8)),
            f32::from_bits(e.read_u32(src + 0xc)),
        );
        let ours = placement_origin_xform(v0, v1, v2, base3);
        assert_f32(e.read_f32(dst + 0x24), ours[0], &format!("{ctx} origin.x"));
        assert_f32(e.read_f32(dst + 0x28), ours[1], &format!("{ctx} origin.y"));
        assert_f32(e.read_f32(dst + 0x2c), ours[2], &format!("{ctx} origin.z"));
        let a1c = f32::from_bits(e.read_u32(src + 0x1c));
        let a14 = f32::from_bits(e.read_u32(src + 0x14));
        let a18 = f32::from_bits(e.read_u32(src + 0x18));
        assert_f32(
            e.read_f32(ebp - 0x10),
            euler_deg_to_rad(a1c),
            &format!("{ctx} euler[+0x1c]"),
        );
        assert_f32(
            e.read_f32(ebp - 0xc),
            euler_deg_to_rad(a14),
            &format!("{ctx} euler[+0x14]"),
        );
        assert_f32(
            e.read_f32(ebp - 0x8),
            euler_deg_to_rad_plus_pi(a18),
            &format!("{ctx} euler[+0x18]+pi"),
        );
    };

    // the two corners and the sphere
    let block_b = |e: &mut Emu, src_f: &[(u64, f32)], base3: [f32; 3], ctx: &str| {
        for &(off, v) in src_f {
            e.write_u32(src + off, v.to_bits());
        }
        for (k, &v) in base3.iter().enumerate() {
            e.write_u32(base + 4 * k as u64, v.to_bits());
        }
        e.write_u32(ebp + 8, base as u32);
        e.write_u32(ebp - 0x4, dst as u32);
        e.set_reg(RegisterX86::EBX, src);
        e.set_reg(RegisterX86::ESI, dst);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x695854, 0x695931);
        let p1_ours = basis_row_xform(
            f32::from_bits(e.read_u32(src + 0x34)),
            f32::from_bits(e.read_u32(src + 0x2c)),
            f32::from_bits(e.read_u32(src + 0x24)),
            base3,
        );
        assert_f32(e.read_f32(dst + 0x44), p1_ours[0], &format!("{ctx} p1.x"));
        assert_f32(e.read_f32(dst + 0x48), p1_ours[1], &format!("{ctx} p1.y"));
        assert_f32(e.read_f32(dst + 0x4c), p1_ours[2], &format!("{ctx} p1.z"));
        let p2_ours = basis_row_xform(
            f32::from_bits(e.read_u32(src + 0x28)),
            f32::from_bits(e.read_u32(src + 0x20)),
            f32::from_bits(e.read_u32(src + 0x30)),
            base3,
        );
        assert_f32(e.read_f32(dst + 0x50), p2_ours[0], &format!("{ctx} p2.x"));
        assert_f32(e.read_f32(dst + 0x54), p2_ours[1], &format!("{ctx} p2.y"));
        assert_f32(e.read_f32(dst + 0x58), p2_ours[2], &format!("{ctx} p2.z"));
        let BoundSphere { center, radius } = bound_sphere(p1_ours, p2_ours);
        assert_f32(
            e.read_f32(dst + 0x5c),
            center[0],
            &format!("{ctx} center.x"),
        );
        assert_f32(
            e.read_f32(dst + 0x60),
            center[1],
            &format!("{ctx} center.y"),
        );
        assert_f32(
            e.read_f32(dst + 0x64),
            center[2],
            &format!("{ctx} center.z"),
        );
        assert_f32(e.read_f32(dst + 0x68), radius, &format!("{ctx} radius"));
    };

    let off_a: [u64; 6] = [0x8, 0xc, 0x10, 0x14, 0x18, 0x1c];
    let off_b: [u64; 6] = [0x20, 0x24, 0x28, 0x2c, 0x30, 0x34];

    let det = |e: &mut Emu, fill: f32, base3: [f32; 3], ctx: &str| {
        let fa: Vec<(u64, f32)> = off_a.iter().map(|&o| (o, fill)).collect();
        let fb: Vec<(u64, f32)> = off_b.iter().map(|&o| (o, fill)).collect();
        block_a(e, &fa, base3, ctx);
        block_b(e, &fb, base3, ctx);
    };
    det(&mut e, 0.0, [0.0, 0.0, 0.0], "all-zero (radius 0 probe)");
    det(&mut e, 1.0, [0.0, 0.0, 0.0], "ones zero-base");
    det(&mut e, 90.0, [100.0, 200.0, 300.0], "90deg base");
    det(&mut e, 180.0, [-10.0, -20.0, -30.0], "180deg neg-base");
    // equal corners: radius 0
    {
        let fb: Vec<(u64, f32)> = vec![
            (0x34, 3.0),
            (0x28, 3.0),
            (0x2c, 4.0),
            (0x20, 4.0),
            (0x24, 5.0),
            (0x30, 5.0),
        ];
        block_b(&mut e, &fb, [7.0, 8.0, 9.0], "p1==p2 -> radius exactly 0");
    }

    let mut checked = 0u32;
    for i in 0..2500u32 {
        let base3: [f32; 3] = std::array::from_fn(|_| {
            if i % 6 == 0 {
                frand_bits(&mut s)
            } else {
                frand(&mut s, -2000.0, 2000.0)
            }
        });
        let fa: Vec<(u64, f32)> = off_a
            .iter()
            .map(|&o| {
                let v = if i % 6 == 0 {
                    frand_bits(&mut s)
                } else {
                    frand(&mut s, -360.0, 360.0)
                };
                (o, v)
            })
            .collect();
        let fb: Vec<(u64, f32)> = off_b
            .iter()
            .map(|&o| {
                let v = if i % 6 == 0 {
                    frand_bits(&mut s)
                } else {
                    frand(&mut s, -2000.0, 2000.0)
                };
                (o, v)
            })
            .collect();
        block_a(&mut e, &fa, base3, &format!("fuzzA{i}"));
        block_b(&mut e, &fb, base3, &format!("fuzzB{i}"));
        checked += 1;
    }
    eprintln!(
        "placement_setup: {checked} cases bit-exact vs WoW.exe @ 0x695650 \
         blocks [6956e5,69578b)/[695854,695931) — 34 FP opcodes covered"
    );
}

/// `AnchorPlacement` (`0x698110`): its stretches between calls, ebx the def (the position at `[ebx]`).
#[test]
fn ground_anchor_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let def = e.alloc(0x20);
    let frame = e.alloc(0x100);
    let ebp = frame + 0xb0;
    let mut s = 0x6981_0000u32;
    let slot = |disp: u64| ebp - disp;

    // `0x69821f` to `0x698242`: the corners at `[ebp-0x18]` and `[ebp-0xc]`, the sums to `[esp]` up
    // after `sub esp,0xc`
    let sum_case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], ctx: &str| -> [f32; 3] {
        e.write_u32(slot(0x18), a[0].to_bits());
        e.write_u32(slot(0x14), a[1].to_bits());
        e.write_u32(slot(0x10), a[2].to_bits());
        e.write_u32(slot(0xc), b[0].to_bits());
        e.write_u32(slot(0x8), b[1].to_bits());
        e.write_u32(slot(0x4), b[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x69821f, 0x698242);
        let ours = bounds_sum(a, b);
        let got = [
            e.read_f32(SLICE_ESP - 0xc),
            e.read_f32(SLICE_ESP - 0x8),
            e.read_f32(SLICE_ESP - 0x4),
        ];
        for k in 0..3 {
            assert_f32(got[k], ours[k], &format!("{ctx} sum[{k}]"));
        }
        ours
    };

    // `0x698247` to `0x698273`: the sums at `[ebp-0x5c]`, the centre to `[esp]` up
    let center_case = |e: &mut Emu, sum: [f32; 3], ctx: &str| -> [f32; 3] {
        e.write_u32(slot(0x5c), sum[0].to_bits());
        e.write_u32(slot(0x58), sum[1].to_bits());
        e.write_u32(slot(0x54), sum[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x698247, 0x698273);
        let ours = bounds_center(sum);
        let got = [
            e.read_f32(SLICE_ESP - 0xc),
            e.read_f32(SLICE_ESP - 0x8),
            e.read_f32(SLICE_ESP - 0x4),
        ];
        for k in 0..3 {
            assert_f32(got[k], ours[k], &format!("{ctx} center[{k}]"));
        }
        ours
    };

    // `0x698278` to `0x6982b3`: both corners less the centre at `[ebp-0x24]`, in place
    let recenter_case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], c: [f32; 3], ctx: &str| {
        e.write_u32(slot(0x18), a[0].to_bits());
        e.write_u32(slot(0x14), a[1].to_bits());
        e.write_u32(slot(0x10), a[2].to_bits());
        e.write_u32(slot(0xc), b[0].to_bits());
        e.write_u32(slot(0x8), b[1].to_bits());
        e.write_u32(slot(0x4), b[2].to_bits());
        e.write_u32(slot(0x24), c[0].to_bits());
        e.write_u32(slot(0x20), c[1].to_bits());
        e.write_u32(slot(0x1c), c[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x698278, 0x6982b3);
        let ra = recenter(a, c);
        let rb = recenter(b, c);
        assert_f32(e.read_f32(slot(0x18)), ra[0], &format!("{ctx} a-c.x"));
        assert_f32(e.read_f32(slot(0x14)), ra[1], &format!("{ctx} a-c.y"));
        assert_f32(e.read_f32(slot(0x10)), ra[2], &format!("{ctx} a-c.z"));
        assert_f32(e.read_f32(slot(0xc)), rb[0], &format!("{ctx} b-c.x"));
        assert_f32(e.read_f32(slot(0x8)), rb[1], &format!("{ctx} b-c.y"));
        assert_f32(e.read_f32(slot(0x4)), rb[2], &format!("{ctx} b-c.z"));
    };

    // `0x6982d4` to `0x6982f6`: `[ebp-0x44]` plus the position, to `[esp]` up
    let world_case = |e: &mut Emu, r: [f32; 3], pos: [f32; 3], ctx: &str| -> [f32; 3] {
        e.write_u32(slot(0x44), r[0].to_bits());
        e.write_u32(slot(0x40), r[1].to_bits());
        e.write_u32(slot(0x3c), r[2].to_bits());
        e.write_u32(def, pos[0].to_bits());
        e.write_u32(def + 4, pos[1].to_bits());
        e.write_u32(def + 8, pos[2].to_bits());
        e.set_reg(RegisterX86::EBX, def);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6982d4, 0x6982f6);
        let ours = world_add(r, pos);
        let got = [
            e.read_f32(SLICE_ESP - 0xc),
            e.read_f32(SLICE_ESP - 0x8),
            e.read_f32(SLICE_ESP - 0x4),
        ];
        for k in 0..3 {
            assert_f32(got[k], ours[k], &format!("{ctx} world[{k}]"));
        }
        ours
    };

    // `0x6982fb` to `0x698319`: x from `[ebp-0x30]`, the cell to `[ebp-0x38]`
    let gidx_x = |e: &mut Emu, anchor_x: f32, ctx: &str| -> i32 {
        e.write_u32(slot(0x30), anchor_x.to_bits());
        e.write_u32(slot(0x38), 0xdead_beef);
        e.write_u32(ebp + 8, 0xdead_beef);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6982fb, 0x698319);
        let bin = e.read_u32(slot(0x38)) as i32;
        assert_eq!(
            bin,
            grid_index(anchor_x),
            "{ctx} grid-index-X (anchor={anchor_x})"
        );
        bin
    };
    // `0x698319` to `0x698337`: y from `[ebp-0x2c]`, the cell to `[ebp-0x34]`
    let gidx_y = |e: &mut Emu, anchor_z: f32, ctx: &str| -> i32 {
        e.write_u32(slot(0x2c), anchor_z.to_bits());
        e.write_u32(slot(0x34), 0xdead_beef);
        e.write_u32(ebp + 8, 0xdead_beef);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x698319, 0x698337);
        let bin = e.read_u32(slot(0x34)) as i32;
        assert_eq!(
            bin,
            grid_index(anchor_z),
            "{ctx} grid-index-Z (anchor={anchor_z})"
        );
        bin
    };

    // `0x698337` to `0x69835a`: the remainders over `[ebx]` and `[ebx+4]`
    let rem_case =
        |e: &mut Emu, ix: i32, iy: i32, ax: f32, az: f32, posx: f32, posy: f32, ctx: &str| {
            e.write_u32(slot(0x38), ix as u32);
            e.write_u32(slot(0x34), iy as u32);
            e.write_u32(slot(0x30), ax.to_bits());
            e.write_u32(slot(0x2c), az.to_bits());
            e.write_u32(def, posx.to_bits());
            e.write_u32(def + 4, posy.to_bits());
            e.set_reg(RegisterX86::EBX, def);
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.run(0x698337, 0x69835a);
            assert_f32(
                e.read_f32(def),
                grid_remainder(ix, ax, posx),
                &format!("{ctx} rem.x"),
            );
            assert_f32(
                e.read_f32(def + 4),
                grid_remainder(iy, az, posy),
                &format!("{ctx} rem.y"),
            );
        };

    let _ = sum_case(&mut e, [1.0, 2.0, 3.0], [4.0, 5.0, 6.0], "edge sum");
    let _ = center_case(&mut e, [3.0, 5.0, 7.0], "edge center");
    recenter_case(
        &mut e,
        [10.0, 20.0, 30.0],
        [1.0, 2.0, 3.0],
        [5.0, 5.0, 5.0],
        "edge recenter",
    );
    let _ = world_case(&mut e, [0.5, 1.5, 2.5], [100.0, 200.0, 300.0], "edge world");

    // cell edges and values near halves
    for &anchor in &[
        0.0f32,
        4.166_666_5,
        -4.166_666_5,
        8.333_333,
        100.0,
        -100.0,
        0.5,
        -0.5,
        1.875,
        6.041_666_5,
    ] {
        gidx_x(&mut e, anchor, "edge");
        gidx_y(&mut e, anchor, "edge");
    }
    // a product near 1.0, which less 0.5 is a half
    {
        let anchor = 1.0_f32 / 0.24_f32 - 4.166_666_5;
        gidx_x(&mut e, anchor, "tie-probe ~0.5");
        gidx_y(&mut e, anchor, "tie-probe ~0.5");
    }
    // cell 0 leaves the position
    rem_case(&mut e, 0, 0, 0.0, 0.0, 50.0, 60.0, "rem idx0 -> pos");
    rem_case(&mut e, 3, -2, 12.5, -8.33, 100.0, -50.0, "rem nonzero");
    rem_case(
        &mut e,
        i32::MIN,
        7,
        0.0,
        9.0,
        1.0,
        2.0,
        "rem indefinite-idx",
    );

    let mut checked = 0u32;
    for i in 0..2500u32 {
        let rnd3 = |s: &mut u32, wide: bool| -> [f32; 3] {
            std::array::from_fn(|_| {
                if wide {
                    frand_bits(s)
                } else {
                    frand(s, -1000.0, 1000.0)
                }
            })
        };
        let wide = i % 6 == 0;
        let a = rnd3(&mut s, wide);
        let b = rnd3(&mut s, wide);
        let sum = sum_case(&mut e, a, b, &format!("fuzz{i}"));
        let center = center_case(&mut e, sum, &format!("fuzz{i}"));
        recenter_case(&mut e, a, b, center, &format!("fuzz{i}"));
        let r = rnd3(&mut s, wide);
        let pos = rnd3(&mut s, wide);
        let _ = world_case(&mut e, r, pos, &format!("fuzz{i}"));

        let ax = if wide {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -500.0, 17000.0)
        };
        let az = if i % 6 == 1 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -500.0, 17000.0)
        };
        let ix = gidx_x(&mut e, ax, &format!("fuzz{i}"));
        let iz = gidx_y(&mut e, az, &format!("fuzz{i}"));
        let posx = frand(&mut s, -2000.0, 2000.0);
        let posy = frand(&mut s, -2000.0, 2000.0);
        rem_case(&mut e, ix, iz, ax, az, posx, posy, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "ground_anchor: {checked} cases bit-exact vs WoW.exe @ 0x698110 \
         slices [21f,242)/[247,273)/[278,2b3)/[2d4,2f6)/[2fb,319)/[319,337)/[337,35a) — 27 FP opcodes"
    );
}

/// `CMapDoodadDef::Repose` (`0x6a81d0`) from `0x6a83b4` to `0x6a83cd`: ebx = this, the fade at
/// `[ebp-0x1c]`, eax a buffer the slice copies from; the product at `[ebx+0x68]`.
#[test]
fn chunk_fade_scale_twin_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x80);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x30;
    let mut s = 0x6a81_0000u32;

    let case = |e: &mut Emu, fade: f32, scale: f32, ctx: &str| {
        e.write_u32(ebp - 0x1c, fade.to_bits());
        e.write_u32(this + 0x30, scale.to_bits());
        e.write_u32(this + 0x68, 0xdead_beef);
        e.set_reg(RegisterX86::EAX, this);
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6a83b4, 0x6a83cd);
        assert_f32(
            e.read_f32(this + 0x68),
            chunk_fade_scale_twin(fade, scale),
            ctx,
        );
    };

    case(&mut e, 0.0, 0.0, "0*0");
    case(&mut e, 1.0, 1.0, "1*1");
    case(&mut e, 0.5, 2.0, "0.5*2 = 1 exact");
    case(&mut e, -1.0, 3.0, "-1*3");
    case(&mut e, 0.1, 0.1, "0.1*0.1 (round at store)");
    case(&mut e, f32::INFINITY, 0.0, "inf*0 -> NaN");

    let mut checked = 0u32;
    for i in 0..3000u32 {
        let (fade, scale) = if i % 5 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -10.0, 10.0), frand(&mut s, -10.0, 10.0))
        };
        case(&mut e, fade, scale, &format!("fuzz{i}"));
        checked += 1;
    }
    eprintln!(
        "chunk_fade_scale_twin: {checked} cases bit-exact vs WoW.exe @ 0x6a81d0 \
         slice [6a83b4,6a83cd) fmul@6a83b9 (twin of 0x6952a0)"
    );
}
