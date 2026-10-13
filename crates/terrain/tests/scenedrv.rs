//! The frame camera, scene and scroll arithmetic against `0x680bc0`, `0x681070` and `0x66fd50`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::spline_anchor::dot3;
use benilla_re_terrain::scenedrv::{
    render_fog_dist, render_leg_fog_offset, render_near_bias, render_pos_accumulate,
    scene_corner_negate, scene_dir_normalize, scene_dir_subtract, scene_dot_negate,
    scene_plane_len2, scene_plane_normalize, scene_plane_normalizes, scroll_accumulate,
    scroll_update, SCENE_EPS2, SCROLL_WRAP,
};
use common::{assert_f32, next};

/// A frame inside the harness stack (`0x70000000`, 0x100000 bytes).
const SLICE_ESP: u64 = 0x7008_0000;

/// A float in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// AH after `fnstsw ax`.
fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// Asserts bytes copied out of the client still equal the client's.
fn island_bytes_match(e: &Emu, island: u64, src: u64, len: usize) {
    let got = e.read(island, len);
    let want = e.read(src, len);
    assert_eq!(
        got, want,
        "island bytes at {island:#x} != WoW.exe[{src:#x}..] (len {len})"
    );
}

/// The direction `0x680bc0` normalizes in place.
const C7CFEC: u64 = 0xc7cfec;

const C7CFF0: u64 = 0xc7cff0;

const C7CFF4: u64 = 0xc7cff4;

const CORNER_X: u64 = 0xc7cf20;

const CORNER_Y: u64 = 0xc7cf24;

const CORNER_Z: u64 = 0xc7cf28;

const C7B498: u64 = 0xc7b498;

const C7D278: u64 = 0xc7d278;

const C62964: u64 = 0xc62964;

const C7D300: u64 = 0xc7d300;

const C62510: u64 = 0xc62510;

const RATE_U: u64 = 0x867918;

const RATE_V: u64 = 0x86791c;

/// `CaptureSceneCamera` (`0x680bc0`) from `0x680c01` to the call at `0x680c22`, edx the target and
/// esi the camera: the differences at `[esp]` up after `sub esp,0xc`.
#[test]
fn scene_dir_subtract_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let target = e.alloc(0x20);
    let cam = e.alloc(0x20);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, t: [f32; 3], c: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(target + 4 * k as u64, t[k].to_bits());
            e.write_u32(cam + 4 * k as u64, c[k].to_bits());
        }
        e.set_reg(RegisterX86::EDX, target);
        e.set_reg(RegisterX86::ESI, cam);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x680c01, 0x680c22);
        let base = SLICE_ESP - 0xc;
        let bin = [e.read_f32(base), e.read_f32(base + 4), e.read_f32(base + 8)];
        let ours = scene_dir_subtract(t, c);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} d[{k}]"));
        }
    };

    run_case(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "dirsub zero");
    run_case(&mut e, [1.0, 2.0, 3.0], [0.0, 0.0, 0.0], "dirsub 1-2-3");
    run_case(&mut e, [5.0, 5.0, 5.0], [5.0, 5.0, 5.0], "dirsub equal");
    let mut s = 0x680c_0101u32;
    for i in 0..3000u32 {
        let t = [
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
        ];
        let c = [
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
        ];
        run_case(&mut e, t, c, &format!("dirsub fuzz[{i}]"));
    }
}

/// `0x680bc0` from `0x680c27` to the call at `0x680cb1`, the direction at `[ebp-0x18]`: the
/// normalized one at `0xc7cfec` up.
#[test]
fn scene_dir_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, d: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x18, d[0].to_bits());
        e.write_u32(ebp - 0x14, d[1].to_bits());
        e.write_u32(ebp - 0x10, d[2].to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x680c27, 0x680cb1);
        let bin = [e.read_f32(C7CFEC), e.read_f32(C7CFF0), e.read_f32(C7CFF4)];
        let ours = scene_dir_normalize(d);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} norm[{k}]"));
        }
    };

    run_case(&mut e, [1.0, 0.0, 0.0], "dirnorm unit-x");
    run_case(&mut e, [3.0, 4.0, 0.0], "dirnorm 3-4-0 (len 5)");
    run_case(&mut e, [1.0, 1.0, 1.0], "dirnorm 1-1-1");
    run_case(&mut e, [2.0, 2.0, 1.0], "dirnorm 2-2-1 (len 3)");
    let mut s = 0x680c_2727u32;
    for i in 0..3000u32 {
        let d = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
        ];
        run_case(&mut e, d, &format!("dirnorm fuzz[{i}]"));
    }
}

/// `0x680bc0`: the negated `DotVector3` (`0x602630`, run), from the calls at `0x680cb1` (to
/// `[0xc7bcbc]`) and `0x680d2e` (to `[0xc7cfc4]`).
#[test]
fn scene_dot_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let anchor = e.alloc(0x20);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;
    const OUT1: u64 = 0xc7bcbc;
    const OUT2: u64 = 0xc7cfc4;

    // `0x680cb1`: the direction at `0xc7cfec`, edx = the anchor
    let run_site1 = |e: &mut Emu, n: [f32; 3], a: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(C7CFEC + 4 * k as u64, n[k].to_bits());
            e.write_u32(anchor + 4 * k as u64, a[k].to_bits());
        }
        e.set_reg(RegisterX86::ECX, C7CFEC);
        e.set_reg(RegisterX86::EDX, anchor);
        e.set_reg(RegisterX86::ESI, anchor);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x680cb1, 0x680cbe);
        let bin = e.read_f32(OUT1);
        assert_f32(bin, scene_dot_negate(dot3(n, a)), &format!("{ctx} site1"));
    };

    // `0x680d2e` to the call at `0x680d4a`: the direction at `[ebp-0xc]`, esi = the anchor
    let run_site2 = |e: &mut Emu, n: [f32; 3], a: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0xc, n[0].to_bits());
        e.write_u32(ebp - 0x8, n[1].to_bits());
        e.write_u32(ebp - 0x4, n[2].to_bits());
        for (k, c) in a.iter().enumerate() {
            e.write_u32(anchor + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::ESI, anchor);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x680d2e, 0x680d4a);
        let bin = e.read_f32(OUT2);
        assert_f32(bin, scene_dot_negate(dot3(n, a)), &format!("{ctx} site2"));
    };

    let probes: [([f32; 3], [f32; 3]); 3] = [
        ([0.0, 0.0, 0.0], [1.0, 2.0, 3.0]),
        ([1.0, 0.0, 0.0], [5.0, 0.0, 0.0]),
        ([0.577, 0.577, 0.577], [10.0, 20.0, 30.0]),
    ];
    for (n, a) in probes {
        run_site1(&mut e, n, a, "dotneg probe");
        run_site2(&mut e, n, a, "dotneg probe");
    }
    let mut s = 0x680c_b6b6u32;
    for i in 0..2000u32 {
        let n = [
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
        ];
        let a = [
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        run_site1(&mut e, n, a, &format!("dotneg fuzz[{i}]"));
        run_site2(&mut e, n, a, &format!("dotneg fuzz[{i}]"));
    }
}

/// `CaptureSceneCamera` (`0x680bc0`) from `0x680cc5` to the `jne` at `0x680cf3`: the stored
/// squared length at `[ebp-0x1c]` and the decision.
#[test]
fn scene_plane_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, c: f32, d: f32, ctx: &str| {
        e.write_u32(C7CFEC, c.to_bits());
        e.write_u32(C7CFF0, d.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x680cc5, 0x680cf3);
        let bin_len2 = e.read_f32(ebp - 0x1c);
        let ah = read_ah(e);
        let ours_len2 = scene_plane_len2(c, d);
        assert_f32(bin_len2, ours_len2 as f32, &format!("{ctx} len2"));
        // `jne` skips the scale when C0 or C3 is set
        assert_eq!(
            ah & 0x41 == 0,
            scene_plane_normalizes(ours_len2),
            "{ctx}: AH {:#04x}, len² {ours_len2}",
            ah & 0x41
        );
    };

    // about the gate
    let eps = f64::from(SCENE_EPS2);
    let c_eps = (eps.sqrt() as f32) * std::f32::consts::FRAC_1_SQRT_2;
    run_case(&mut e, c_eps, c_eps, "gate ~=eps");
    run_case(&mut e, 0.0, 0.0, "gate zero (len²=0<eps)");
    run_case(&mut e, 3.0, 4.0, "gate 3-4 (len²=25)");
    run_case(&mut e, 0.005, 0.005, "gate small (len²=5e-5<eps)");
    run_case(&mut e, 0.011, 0.0, "gate 0.011 (len²=1.21e-4>eps)");

    let mut s = 0x680c_c5c5u32;
    for i in 0..3000u32 {
        let (c, d) = if (next(&mut s) >> 8).is_multiple_of(4) {
            (frand(&mut s, -0.01, 0.01), frand(&mut s, -0.01, 0.01))
        } else {
            (frand(&mut s, -100.0, 100.0), frand(&mut s, -100.0, 100.0))
        };
        run_case(&mut e, c, d, &format!("gate fuzz[{i}]"));
    }
}

/// `0x680bc0` from `0x680cc5` to the join at `0x680d19`: `[ebp-0xc]` up, scaled or not.
#[test]
fn scene_plane_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, c: f32, d: f32, ctx: &str| {
        e.write_u32(C7CFEC, c.to_bits());
        e.write_u32(C7CFF0, d.to_bits());
        // `0x680cbe` clears it before the slice
        e.write_u32(ebp - 0x4, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x680cc5, 0x680d19);
        let bin = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let ours = scene_plane_normalize(c, d);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} pn[{k}]"));
        }
    };

    let eps = f64::from(SCENE_EPS2);
    let c_eps = (eps.sqrt() as f32) * std::f32::consts::FRAC_1_SQRT_2;
    run_case(&mut e, c_eps, c_eps, "pnorm ~=eps");
    run_case(&mut e, 0.0, 0.0, "pnorm zero (skip)");
    run_case(&mut e, 3.0, 4.0, "pnorm 3-4");
    run_case(&mut e, 1.0, 0.0, "pnorm unit-c");
    run_case(&mut e, 0.005, 0.005, "pnorm small (skip)");
    run_case(&mut e, 0.011, 0.0, "pnorm 0.011 (normalize)");

    let mut s = 0x680c_d1d1u32;
    for i in 0..3000u32 {
        let (c, d) = if (next(&mut s) >> 8).is_multiple_of(4) {
            (frand(&mut s, -0.01, 0.01), frand(&mut s, -0.01, 0.01))
        } else {
            (frand(&mut s, -100.0, 100.0), frand(&mut s, -100.0, 100.0))
        };
        run_case(&mut e, c, d, &format!("pnorm fuzz[{i}]"));
    }
}

/// `0x680bc0`: the camera negated, at `0x680d82` and at `0x680ee9`.
#[test]
fn scene_corner_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    // `0x680d7c` to the call at `0x680da5`: to `[esp]` up after `sub esp,0xc`
    let run_group1 = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(CORNER_X, g[0].to_bits());
        e.write_u32(CORNER_Y, g[1].to_bits());
        e.write_u32(CORNER_Z, g[2].to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x680d7c, 0x680da5);
        let base = SLICE_ESP - 0xc;
        let bin = [e.read_f32(base), e.read_f32(base + 4), e.read_f32(base + 8)];
        for k in 0..3 {
            assert_f32(bin[k], scene_corner_negate(g[k]), &format!("{ctx} g1[{k}]"));
        }
    };
    // `0x680ee3` to the call at `0x680f0b`: to `[ebp-0x28]` up
    let run_group2 = |e: &mut Emu, g: [f32; 3], ctx: &str| {
        e.write_u32(CORNER_X, g[0].to_bits());
        e.write_u32(CORNER_Y, g[1].to_bits());
        e.write_u32(CORNER_Z, g[2].to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x680ee3, 0x680f0b);
        let bin = [
            e.read_f32(ebp - 0x28),
            e.read_f32(ebp - 0x24),
            e.read_f32(ebp - 0x20),
        ];
        for k in 0..3 {
            assert_f32(bin[k], scene_corner_negate(g[k]), &format!("{ctx} g2[{k}]"));
        }
    };

    run_group1(&mut e, [0.0, 0.0, 0.0], "corner zero");
    run_group1(&mut e, [1.0, -1.0, 1.0], "corner ±1");
    run_group2(&mut e, [0.0, 0.0, 0.0], "corner zero");
    run_group2(&mut e, [1.0, -1.0, 1.0], "corner ±1");
    let mut s = 0x680d_7c7cu32;
    for i in 0..2000u32 {
        let g = [
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
        ];
        run_group1(&mut e, g, &format!("corner fuzz[{i}]"));
        run_group2(&mut e, g, &format!("corner fuzz[{i}]"));
    }
}

/// `DrawWorldScene` (`0x681070`) from `0x6810d8` to `0x6810ea`: `[0xc7d278]`.
#[test]
fn render_near_bias_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let run_case = |e: &mut Emu, near: f32, ctx: &str| {
        e.write_u32(C7B498, near.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(0x6810d8, 0x6810ea);
        let bin = e.read_f32(C7D278);
        assert_f32(bin, render_near_bias(near), ctx);
    };
    run_case(&mut e, 0.0, "near 0");
    run_case(&mut e, f32::from_bits(0x4205_5555), "near ==33.333 (→0)");
    run_case(&mut e, 1000.0, "near 1000");
    let mut s = 0x6810_d8d8u32;
    for i in 0..3000u32 {
        let near = frand(&mut s, -5000.0, 5000.0);
        run_case(&mut e, near, &format!("near fuzz[{i}]"));
    }
}

/// `0x681070` from `0x6810f5` to `0x68111a`, the field of view loaded first: `[0xc7d300]`.
#[test]
fn render_fog_dist_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // `fld dword [in]` and then the client's bytes from `0x6810f5` through the store at `0x681114`
    const SRC: u64 = 0x6810f5;
    const LEN: usize = 0x68111a - 0x6810f5;
    let body = e.read(SRC, LEN);
    assert_eq!(
        &body[0..6],
        &[0xd8, 0x0d, 0x24, 0xfa, 0x7f, 0x00],
        "0x6810f5 fmul [0x7ffa24] drifted"
    );
    let inp = e.alloc(0x10);
    let isl = e.alloc(0x80);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(inp as u32).to_le_bytes());
    let prefix_len = code.len();
    code.extend_from_slice(&body);
    e.write(isl, &code);
    island_bytes_match(&e, isl + prefix_len as u64, SRC, LEN);
    let end = isl + code.len() as u64;

    let run_case = |e: &mut Emu, fov: f32, fog: f32, ctx: &str| {
        e.write_u32(inp, fov.to_bits());
        e.write_u32(C62964, fog.to_bits());
        e.set_reg(RegisterX86::EBX, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(isl, end);
        let bin = e.read_f32(C7D300);
        assert_f32(bin, render_fog_dist(fov, fog), ctx);
    };

    // a field of view of 0 and a fog of 0 give 0
    run_case(&mut e, 0.0, 100.0, "fog fov=0 (cos1→0)");
    run_case(&mut e, 1.0, 0.0, "fog fog=0");
    run_case(&mut e, std::f32::consts::FRAC_PI_2, 5000.0, "fog fov=π/2");
    run_case(&mut e, std::f32::consts::FRAC_PI_4, 5000.0, "fog fov=π/4");
    let mut s = 0x6810_f5f5u32;
    for i in 0..3000u32 {
        let fov = frand(&mut s, 0.05, 2.8);
        let fog = frand(&mut s, 10.0, 8000.0);
        run_case(&mut e, fov, fog, &format!("fog fuzz[{i}] fov={fov}"));
    }
}

/// `0x681070` from `0x6811a0` to `0x6811af`: `[0xc7b66c]`.
#[test]
fn render_leg_fog_offset_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let esi = e.alloc(0x40);
    const C7B66C: u64 = 0xc7b66c;
    let run_case = |e: &mut Emu, leg_z: f32, ctx: &str| {
        e.write_u32(esi + 0x10, leg_z.to_bits());
        e.set_reg(RegisterX86::ESI, esi);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(0x6811a0, 0x6811af);
        let bin = e.read_f32(C7B66C);
        assert_f32(bin, render_leg_fog_offset(leg_z), ctx);
    };
    run_case(&mut e, 0.0, "leg 0");
    run_case(&mut e, -f32::from_bits(0x4205_5555), "leg ==-33.333 (→0)");
    run_case(&mut e, 1000.0, "leg 1000");
    let mut s = 0x6811_a0a0u32;
    for i in 0..3000u32 {
        let leg_z = frand(&mut s, -5000.0, 5000.0);
        run_case(&mut e, leg_z, &format!("leg fuzz[{i}]"));
    }
}

/// `0x681070` from `0x681310` to `0x68131c`, the first of six alike: `[ebp-0x2c]`.
#[test]
fn render_pos_accumulate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    // the six sites are `fld [abs]; fadd [ebp-N]; fstp [ebp-N]`, differing in the addresses only
    const FADD_SITES: [u64; 6] = [0x681316, 0x681322, 0x68132e, 0x68133a, 0x681346, 0x681352];
    for site in FADD_SITES {
        assert_eq!(
            e.read(site, 2),
            vec![0xd8, 0x45],
            "0x681070 fadd [ebp-N] site {site:#x} drifted (not d8 45)"
        );
    }

    let run_case = |e: &mut Emu, corner: f32, acc: f32, ctx: &str| {
        e.write_u32(CORNER_X, corner.to_bits());
        e.write_u32(ebp - 0x2c, acc.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // to the next instruction, past the store
        e.run(0x681310, 0x68131c);
        let bin = e.read_f32(ebp - 0x2c);
        assert_f32(bin, render_pos_accumulate(corner, acc), ctx);
    };
    run_case(&mut e, 0.0, 0.0, "acc zero");
    run_case(&mut e, 1.0, 2.0, "acc 1+2");
    run_case(&mut e, -100.0, 100.0, "acc -100+100");
    let mut s = 0x6813_1010u32;
    for i in 0..3000u32 {
        let corner = frand(&mut s, -5000.0, 5000.0);
        let acc = frand(&mut s, -5000.0, 5000.0);
        run_case(&mut e, corner, acc, &format!("acc fuzz[{i}]"));
    }
}

/// `CWorld::BeginFrame` (`0x66fd50`) from `0x66fe10` to `0x66fe35`, one channel: both offsets
/// before the wraps.
#[test]
fn scroll_accumulate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let buf = e.alloc(0x40);
    let ecx = buf + 0x10;
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, dt: f32, ru: f32, rv: f32, off_u: f32, off_v: f32, ctx: &str| {
        e.write_u32(C62510, dt.to_bits());
        e.write_u32(RATE_U, ru.to_bits());
        e.write_u32(RATE_V, rv.to_bits());
        e.write_u32(ecx - 0x4, off_u.to_bits());
        e.write_u32(ecx, off_v.to_bits());
        e.set_reg(RegisterX86::ECX, ecx);
        e.set_reg(RegisterX86::EDX, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x66fe10, 0x66fe35);
        let bin_u = e.read_f32(ecx - 0x4);
        let bin_v = e.read_f32(ecx);
        assert_f32(bin_u, scroll_accumulate(dt, ru, off_u), &format!("{ctx} u"));
        assert_f32(bin_v, scroll_accumulate(dt, rv, off_v), &format!("{ctx} v"));
    };

    run_case(&mut e, 0.0, -1.0, 0.5, 10.0, 20.0, "scroll dt=0");
    run_case(&mut e, 0.5, 0.0, 0.0, 1.0, 2.0, "scroll rate=0");
    run_case(&mut e, 1.0, 1.0, 2.0, 0.0, 0.0, "scroll unit");
    run_case(&mut e, -1.0, -1.0, -2.0, 0.0, 0.0, "scroll neg");
    run_case(&mut e, 1.0, 64.0, 64.0, 0.0, 0.0, "scroll acc==64");

    let mut s = 0x66fe_1010u32;
    for i in 0..3000u32 {
        let dt = frand(&mut s, 0.0, 0.1);
        let ru = frand(&mut s, -32.0, 32.0);
        let rv = frand(&mut s, -32.0, 32.0);
        let off_u = frand(&mut s, 0.0, 64.0);
        let off_v = frand(&mut s, 0.0, 64.0);
        run_case(
            &mut e,
            dt,
            ru,
            rv,
            off_u,
            off_v,
            &format!("scroll fuzz[{i}]"),
        );
    }
}

/// `0x66fd50` from `0x66fe10` to `0x66fe57`, one channel through both wraps.
#[test]
fn scroll_update_wrap_probe() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let buf = e.alloc(0x40);
    let ecx = buf + 0x10;
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;

    let run_case = |e: &mut Emu, dt: f32, rate: [f32; 2], off: [f32; 2], ctx: &str| {
        e.write_u32(C62510, dt.to_bits());
        e.write_u32(RATE_U, rate[0].to_bits());
        e.write_u32(RATE_V, rate[1].to_bits());
        e.write_u32(ecx - 0x4, off[0].to_bits());
        e.write_u32(ecx, off[1].to_bits());
        e.set_reg(RegisterX86::ECX, ecx);
        e.set_reg(RegisterX86::EDX, 0);
        e.set_reg(RegisterX86::EBX, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x66fe10, 0x66fe57);
        let ours = scroll_update(dt, rate, off);
        assert_f32(e.read_f32(ecx - 0x4), ours[0], &format!("{ctx} u"));
        assert_f32(e.read_f32(ecx), ours[1], &format!("{ctx} v"));
    };

    run_case(&mut e, 1.0, [64.0, 64.0], [0.0, 0.0], "64 (wraps)");
    let below = f32::from_bits(SCROLL_WRAP.to_bits() - 1);
    run_case(
        &mut e,
        1.0,
        [below, below],
        [0.0, 0.0],
        "just below 64 (kept)",
    );
    run_case(&mut e, 1.0, [70.0, 70.0], [0.0, 0.0], "70 (wraps)");
    run_case(&mut e, 0.5, [10.0, 10.0], [1.0, 1.0], "6 (kept)");
    // a sum just below 64 that stores as 64.0: u compares the unrounded sum and keeps it, v the
    // stored one and wraps
    let step = f32::from_bits(0x3620_0000);
    run_case(&mut e, 1.0, [step, step], [below, below], "rounds to 64");
    run_case(&mut e, 1.0, [f32::NAN, f32::NAN], [0.0, 0.0], "NaN (kept)");

    let mut s = 0x66fe_4545u32;
    for i in 0..2000u32 {
        let dt = frand(&mut s, 0.0, 2.0);
        let rate = [frand(&mut s, -40.0, 40.0), frand(&mut s, -40.0, 40.0)];
        let off = [frand(&mut s, 0.0, 64.0), frand(&mut s, 0.0, 64.0)];
        run_case(&mut e, dt, rate, off, &format!("wrap fuzz[{i}]"));
    }
}
