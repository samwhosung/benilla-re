//! The resolution layer's helpers run whole: the vector, box and plane helpers `0x6372d0` to
//! `0x637480`, `PlaneDistanceAlongRay` (`0x6329e0`, through `call_f32`) and `BuildVolumePlanes`
//! (`0x631440`).

use benilla_re_collision::resolve_toolkit::{
    aabb_union, plane_from_points, point_in_aabb, ray_plane_t, vec3_add_scalar, vec3_negate,
    vec3_sub_scalar, Aabb, Plane,
};
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// A random `f32` in `[-4, 4)`.
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}

/// Writes three `f32`.
fn wv3(emu: &mut Emu, va: u64, v: &[f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, c.to_bits());
    }
}
/// Reads three `f32`.
fn rv3(emu: &Emu, va: u64) -> [f32; 3] {
    core::array::from_fn(|i| emu.read_f32(va + (i as u64) * 4))
}
/// Asserts the same bits.
fn af(b: f32, o: f32, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        b.to_bits(),
        o.to_bits()
    );
}

/// The seven helpers on random vectors, boxes and planes.
#[test]
fn resolve_toolkit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let v_va = emu.alloc(0x20);
    let b_va = emu.alloc(0x20);
    let b2_va = emu.alloc(0x20);
    let out_va = emu.alloc(0x20);
    let p_va = emu.alloc(0x30);
    let mut s = 0x7001_cafe_u32;
    let mut checked = 0u32;

    for case in 0..250u32 {
        let v = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let sc = fz(&mut s);

        wv3(&mut emu, v_va, &v);
        emu.call(0x6372d0, v_va as u32, 0, &[sc.to_bits()]);
        let mut ours = v;
        vec3_add_scalar(&mut ours, sc);
        let bin = rv3(&emu, v_va);
        for k in 0..3 {
            af(bin[k], ours[k], &format!("case {case}: add_scalar[{k}]"));
        }

        wv3(&mut emu, v_va, &v);
        emu.call(0x637300, v_va as u32, 0, &[sc.to_bits()]);
        let mut ours = v;
        vec3_sub_scalar(&mut ours, sc);
        let bin = rv3(&emu, v_va);
        for k in 0..3 {
            af(bin[k], ours[k], &format!("case {case}: sub_scalar[{k}]"));
        }

        wv3(&mut emu, v_va, &v);
        emu.call(0x637330, v_va as u32, 0, &[out_va as u32]);
        let bin = rv3(&emu, out_va);
        let ours = vec3_negate(&v);
        for k in 0..3 {
            af(bin[k], ours[k], &format!("case {case}: negate[{k}]"));
        }

        let mut bb = Aabb {
            min: [
                fz(&mut s).min(0.0) - 1.0,
                fz(&mut s).min(0.0) - 1.0,
                fz(&mut s).min(0.0) - 1.0,
            ],
            max: [
                fz(&mut s).max(0.0) + 1.0,
                fz(&mut s).max(0.0) + 1.0,
                fz(&mut s).max(0.0) + 1.0,
            ],
        };
        let mut p = [fz(&mut s), fz(&mut s), fz(&mut s)];
        if case % 5 == 0 {
            p[(case % 3) as usize] = bb.min[(case % 3) as usize];
        }
        wv3(&mut emu, b_va, &bb.min);
        wv3(&mut emu, b_va + 0xc, &bb.max);
        wv3(&mut emu, p_va, &p);
        let eax = emu.call(0x637350, b_va as u32, 0, &[p_va as u32]);
        assert_eq!(
            eax & 1 != 0,
            point_in_aabb(&bb, &p),
            "case {case}: point_in_aabb"
        );

        let bb2 = Aabb {
            min: [fz(&mut s), fz(&mut s), fz(&mut s)],
            max: [fz(&mut s), fz(&mut s), fz(&mut s)],
        };
        if case % 4 == 0 {
            bb.min = bb2.min;
        }
        wv3(&mut emu, b_va, &bb.min);
        wv3(&mut emu, b_va + 0xc, &bb.max);
        wv3(&mut emu, b2_va, &bb2.min);
        wv3(&mut emu, b2_va + 0xc, &bb2.max);
        emu.call(0x6373b0, out_va as u32, b_va as u32, &[b2_va as u32]);
        let ours = aabb_union(&bb, &bb2);
        let bmin = rv3(&emu, out_va);
        let bmax = rv3(&emu, out_va + 0xc);
        for k in 0..3 {
            af(
                bmin[k],
                ours.min[k],
                &format!("case {case}: union.min[{k}]"),
            );
            af(
                bmax[k],
                ours.max[k],
                &format!("case {case}: union.max[{k}]"),
            );
        }

        let (p0, p1, p2) = (
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
        );
        wv3(&mut emu, p_va, &p0);
        wv3(&mut emu, p_va + 0xc, &p1);
        wv3(&mut emu, p_va + 0x18, &p2);
        emu.call(
            0x637480,
            out_va as u32,
            0,
            &[p_va as u32, (p_va + 0xc) as u32, (p_va + 0x18) as u32],
        );
        let ours = plane_from_points(&p0, &p1, &p2);
        let bn = rv3(&emu, out_va);
        for (k, &c) in bn.iter().enumerate() {
            af(c, ours.n[k], &format!("case {case}: plane.n[{k}]"));
        }
        af(
            emu.read_f32(out_va + 0xc),
            ours.d,
            &format!("case {case}: plane.d"),
        );

        let mut plane = Plane {
            n: [fz(&mut s), fz(&mut s), fz(&mut s)],
            d: fz(&mut s),
        };
        let mut dir = [fz(&mut s), fz(&mut s), fz(&mut s)];
        if case % 7 == 0 {
            plane.n = [1.0, 0.0, 0.0];
            dir = [
                f32::from_bits(0x3400_0000) * if case % 2 == 0 { 1.0 } else { -1.0 },
                1.0,
                0.5,
            ];
        }
        let pt = [fz(&mut s), fz(&mut s), fz(&mut s)];
        wv3(&mut emu, p_va, &pt);
        wv3(&mut emu, out_va, &plane.n);
        emu.write_u32(out_va + 0xc, plane.d.to_bits());
        wv3(&mut emu, v_va, &dir);
        let bt = emu.call_f32(0x6329e0, p_va as u32, out_va as u32, &[v_va as u32]);
        let ot = ray_plane_t(&pt, &plane, &dir) as f32;
        af(bt, ot, &format!("case {case}: ray_plane_t"));

        checked += 1;
    }
    eprintln!("resolve_toolkit: {checked} rounds bit-exact vs WoW.exe (7 functions per round)");
}

/// `0x631440`: the nine planes.
#[test]
fn kdop_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let c_va = emu.alloc(0x10);
    let out_va = emu.alloc(0xa0);
    let mut s = 0x6b_d0_70_01_u32;
    let mut checked = 0u32;
    for case in 0..200u32 {
        let center = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let height = fz(&mut s).abs() + 0.5;
        let radius = fz(&mut s).abs() + 0.2;
        wv3(&mut emu, c_va, &center);
        for i in 0..0xa0u64 {
            emu.write(out_va + i, &[0xcc]);
        }
        emu.call(
            0x631440,
            0,
            0,
            &[
                c_va as u32,
                height.to_bits(),
                radius.to_bits(),
                out_va as u32,
            ],
        );
        let ours =
            benilla_re_collision::resolve_toolkit::capsule_kdop_planes(&center, height, radius);
        for (i, pl) in ours.iter().enumerate() {
            let base = out_va + (i as u64) * 0x10;
            let bn = rv3(&emu, base);
            for (k, &c) in bn.iter().enumerate() {
                af(c, pl.n[k], &format!("case {case}: P{i}.n[{k}]"));
            }
            af(
                emu.read_f32(base + 0xc),
                pl.d,
                &format!("case {case}: P{i}.d"),
            );
        }
        checked += 1;
    }
    eprintln!("kdop: {checked} rounds bit-exact (9 planes per round)");
}
