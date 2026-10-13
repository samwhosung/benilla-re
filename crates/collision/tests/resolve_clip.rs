//! The sweep geometry run whole: the polygon clips `ClipPolygonByPlane` (`0x6318c0`),
//! `ClipPolygonByPlanes` (`0x631870`) and `ClipPolygonByPlanesExcept` (`0x632980`);
//! `CMovement::BuildVolume` (`0x631be0`); `PolygonContactDistance` (`0x632830`);
//! `IsPointOverListedTriangle` (`0x6335d0`); the crease helpers `IntersectThreePlanes`
//! (`0x634fc0`), `PointToLineDistanceSquared` (`0x634f40`) and `FindNearestTriangleEdge`
//! (`0x634da0`); `CMovement::ComputeWallNormal` (`0x634ae0`); `FindNearestFacet` (`0x632700`);
//! `CMovement::FindBevelContacts` (`0x632280`); `BuildTrianglePrism` (`0x632460`); and, with
//! `QueryWorldTriangles` (`0x6721b0`) and `IsUnitPlayerDriven` (`0x5fa550`) stubbed and the
//! triangle lists written by the test, `CMovement::GatherSweepFacets` (`0x631e70`),
//! `CMovement::MeasureHoverDrop` (`0x632a30`) and `CMovement::SweepVolume` (`0x632ba0`).

use benilla_re_collision::resolve_clip::{
    clip_confirm, poly_plane_clip, swept_prism_verts, ClipPoly,
};
use benilla_re_collision::resolve_toolkit::{capsule_kdop_planes, Plane};
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

/// Writes a clip polygon in the client's layout.
fn write_poly(emu: &mut Emu, va: u64, p: &ClipPoly) {
    for (i, v) in p.verts.iter().enumerate() {
        wv3(emu, va + (i as u64) * 0xc, v);
    }
    for (i, &a) in p.attrs.iter().enumerate() {
        emu.write_u32(va + 0xb4 + (i as u64) * 4, a as u32);
    }
    emu.write_u32(va + 0xf0, p.count);
}

/// Compares a clip polygon in memory with the reimplementation's.
fn diff_poly(emu: &Emu, va: u64, ours: &ClipPoly, ctx: &str) {
    let bc = emu.read_u32(va + 0xf0);
    assert_eq!(bc, ours.count, "{ctx}: count");
    for i in 0..ours.count as usize {
        for k in 0..3u64 {
            let b = emu.read_f32(va + (i as u64) * 0xc + k * 4);
            let o = ours.verts[i][k as usize];
            assert_eq!(
                b.to_bits(),
                o.to_bits(),
                "{ctx}: vert[{i}][{k}] binary={:#010x} ours={:#010x}",
                b.to_bits(),
                o.to_bits()
            );
        }
        let ba = emu.read_u32(va + 0xb4 + (i as u64) * 4) as i32;
        assert_eq!(ba, ours.attrs[i], "{ctx}: attr[{i}]");
    }
}

/// A random polygon of `n` vertices.
fn mk_poly(s: &mut u32, n: usize) -> ClipPoly {
    let mut p = ClipPoly::default();
    let c = [fz(s), fz(s), fz(s)];
    for i in 0..n {
        let a = (i as f32) * std::f32::consts::TAU / (n as f32);
        p.verts[i] = [
            c[0] + 2.0 * a.cos(),
            c[1] + 2.0 * a.sin(),
            c[2] + 0.3 * a.sin(),
        ];
        p.attrs[i] = (next(s) & 0x7f) as i32;
    }
    p.count = n as u32;
    p
}

/// `0x6318c0`, `0x632980` and `0x631be0`.
#[test]
fn resolve_clip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let poly_va = emu.alloc(0x100);
    let plane_va = emu.alloc(0xa0);
    let pos_va = emu.alloc(0x10);
    let verts_va = emu.alloc(0x70);
    let face_va = emu.alloc(0x20);
    let face_va2 = emu.alloc(0x120);
    let _ = face_va2;
    let mut s = 0xc11b_2026u32;
    let mut checked = 0u32;

    for case in 0..300u32 {
        let mut poly = mk_poly(&mut s, 3 + (case as usize % 6));
        let plane = Plane {
            n: [fz(&mut s) * 0.3, fz(&mut s) * 0.3, 1.0 + fz(&mut s) * 0.1],
            d: fz(&mut s),
        };
        let prov = (next(&mut s) & 0xff) as i32;
        write_poly(&mut emu, poly_va, &poly);
        wv3(&mut emu, plane_va, &plane.n);
        emu.write_u32(plane_va + 0xc, plane.d.to_bits());
        emu.call(0x6318c0, plane_va as u32, poly_va as u32, &[prov as u32]);
        poly_plane_clip(&mut poly, &plane, prov);
        diff_poly(&emu, poly_va, &poly, &format!("case {case}: clip"));

        let center = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let height = fz(&mut s).abs() + 0.5;
        let radius = fz(&mut s).abs() + 0.3;
        let planes = capsule_kdop_planes(&center, height, radius);
        let mut poly = mk_poly(&mut s, 4 + (case as usize % 4));
        if case % 3 == 0 {
            for v in poly.verts.iter_mut() {
                v[0] = center[0] + (v[0] - center[0]) * 0.2;
                v[1] = center[1] + (v[1] - center[1]) * 0.2;
                v[2] = center[2] - 0.2;
            }
        }
        let self_idx = (next(&mut s) % 9) as usize;
        for (i, pl) in planes.iter().enumerate() {
            wv3(&mut emu, plane_va + (i as u64) * 0x10, &pl.n);
            emu.write_u32(plane_va + (i as u64) * 0x10 + 0xc, pl.d.to_bits());
        }
        write_poly(&mut emu, poly_va, &poly);
        let eax = emu.call(
            0x632980,
            plane_va as u32,
            9,
            &[self_idx as u32, poly_va as u32],
        );
        let ours_ok = clip_confirm(&mut poly, &planes, self_idx);
        assert_eq!(eax & 0xff != 0, ours_ok, "case {case}: confirm verdict");
        if ours_ok {
            diff_poly(&emu, poly_va, &poly, &format!("case {case}: confirm poly"));
        }

        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let this_va = face_va + 0x20;
        emu.write_u32(this_va + 0xb0, radius.to_bits());
        wv3(&mut emu, pos_va, &pos);
        emu.call(
            0x631be0,
            this_va as u32,
            0,
            &[
                pos_va as u32,
                height.to_bits(),
                plane_va as u32,
                verts_va as u32,
                face_va as u32,
            ],
        );
        let our_planes = capsule_kdop_planes(&pos, height, radius);
        for (i, pl) in our_planes.iter().enumerate() {
            for k in 0..3u64 {
                let b = emu.read_f32(plane_va + (i as u64) * 0x10 + k * 4);
                assert_eq!(
                    b.to_bits(),
                    pl.n[k as usize].to_bits(),
                    "case {case}: prism plane{i}.n[{k}]"
                );
            }
            let b = emu.read_f32(plane_va + (i as u64) * 0x10 + 0xc);
            assert_eq!(b.to_bits(), pl.d.to_bits(), "case {case}: prism plane{i}.d");
        }
        let our_verts = swept_prism_verts(&pos, height, radius);
        for (i, v) in our_verts.iter().enumerate() {
            for k in 0..3u64 {
                let b = emu.read_f32(verts_va + (i as u64) * 0xc + k * 4);
                assert_eq!(
                    b.to_bits(),
                    v[k as usize].to_bits(),
                    "case {case}: prism vert[{i}][{k}] binary={:#010x} ours={:#010x}",
                    b.to_bits(),
                    v[k as usize].to_bits()
                );
            }
        }
        checked += 1;
    }
    eprintln!("resolve_clip: {checked} rounds bit-exact vs WoW.exe (clip + confirm)");
}

/// `0x632830`.
#[test]
fn toi_difftest() {
    use benilla_re_collision::resolve_clip::polygon_toi;
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let poly_va = emu.alloc(0x100);
    let plane_va = emu.alloc(0xa0);
    let dir_va = emu.alloc(0x10);
    let outt_va = emu.alloc(8);
    let mut s = 0x701_2026u32;
    let mut checked = 0u32;
    for case in 0..300u32 {
        let center = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let height = fz(&mut s).abs() + 0.5;
        let radius = fz(&mut s).abs() + 0.3;
        let planes = capsule_kdop_planes(&center, height, radius);
        let mut poly = mk_poly(&mut s, 4 + (case as usize % 4));
        if case % 2 == 0 {
            for v in poly.verts.iter_mut() {
                v[0] = center[0] + (v[0] - center[0]) * 0.3;
                v[1] = center[1] + (v[1] - center[1]) * 0.3;
            }
        }
        let dir = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let self_idx = (next(&mut s) % 9) as usize;
        let t0 = fz(&mut s).abs() + 0.5;
        for (i, pl) in planes.iter().enumerate() {
            wv3(&mut emu, plane_va + (i as u64) * 0x10, &pl.n);
            emu.write_u32(plane_va + (i as u64) * 0x10 + 0xc, pl.d.to_bits());
        }
        write_poly(&mut emu, poly_va, &poly);
        wv3(&mut emu, dir_va, &dir);
        emu.write_u32(outt_va, t0.to_bits());
        let eax = emu.call(
            0x632830,
            poly_va as u32,
            dir_va as u32,
            &[plane_va as u32, self_idx as u32, outt_va as u32],
        );
        let mut our_t = t0;
        let hit = polygon_toi(&mut poly, &dir, &planes, self_idx, &mut our_t);
        assert_eq!(eax & 0xff != 0, hit, "case {case}: toi verdict");
        let bt = emu.read_f32(outt_va);
        assert_eq!(
            bt.to_bits(),
            our_t.to_bits(),
            "case {case}: toi t binary={:#010x} ours={:#010x}",
            bt.to_bits(),
            our_t.to_bits()
        );
        checked += 1;
    }
    eprintln!("toi: {checked} rounds bit-exact vs WoW.exe");
}

/// `0x6335d0`.
#[test]
fn point_in_tri_difftest() {
    use benilla_re_collision::resolve_clip::point_in_tri;
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let hits = emu.alloc(0x200);
    let this_va = emu.alloc(0x40);
    emu.write_u32(0xc4e534, hits as u32);
    let mut s = 0x917_2026u32;
    let mut checked = 0u32;
    for case in 0..300u32 {
        let tri = [
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
        ];
        let p = if case % 2 == 0 {
            let w = [0.4f32, 0.3, 0.3];
            core::array::from_fn(|k| tri[0][k] * w[0] + tri[1][k] * w[1] + tri[2][k] * w[2])
        } else {
            [fz(&mut s), fz(&mut s), fz(&mut s)]
        };
        let idx = (next(&mut s) % 4) as u64;
        for (r, v) in tri.iter().enumerate() {
            wv3(&mut emu, hits + idx * 0x34 + 0x10 + (r as u64) * 0xc, v);
        }
        wv3(&mut emu, this_va, &p);

        let eax = emu.call(0x6335d0, idx as u32, this_va as u32, &[]);
        let ours = point_in_tri(&tri, &p);
        assert_eq!(eax & 0xff != 0, ours, "case {case}: point-in-tri verdict");
        checked += 1;
    }
    eprintln!("point_in_tri: {checked} rounds bit-exact vs WoW.exe");
}

/// `0x631870`, `0x634fc0` and `0x634f40`.
#[test]
fn sibling_pure_difftest() {
    use benilla_re_collision::resolve_clip::{
        multi_plane_clip, ray_point_dist_sq, three_plane_point,
    };
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let pl_va = emu.alloc(0xa0);
    let out_va = emu.alloc(0x10);
    let p_va = emu.alloc(0x30);
    let poly_va = emu.alloc(0x100);
    let mut s = 0x51b1_2026u32;
    let mut checked = 0u32;
    for case in 0..300u32 {
        let mk = |s: &mut u32| Plane {
            n: [fz(s), fz(s), fz(s)],
            d: fz(s),
        };
        let (a, b, c) = (mk(&mut s), mk(&mut s), mk(&mut s));
        for (i, pl) in [&a, &b, &c].iter().enumerate() {
            wv3(&mut emu, pl_va + (i as u64) * 0x10, &pl.n);
            emu.write_u32(pl_va + (i as u64) * 0x10 + 0xc, pl.d.to_bits());
        }
        emu.call(
            0x634fc0,
            out_va as u32,
            pl_va as u32,
            &[(pl_va + 0x10) as u32, (pl_va + 0x20) as u32],
        );
        let ours = three_plane_point(&a, &b, &c);
        let bin = rv3(&emu, out_va);
        for k in 0..3 {
            assert_eq!(
                bin[k].to_bits(),
                ours[k].to_bits(),
                "case {case}: 3plane[{k}] binary={:#010x} ours={:#010x}",
                bin[k].to_bits(),
                ours[k].to_bits()
            );
        }

        let (p, q, dir) = (
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
        );
        wv3(&mut emu, p_va, &p);
        wv3(&mut emu, p_va + 0xc, &q);
        wv3(&mut emu, p_va + 0x18, &dir);
        let bd = emu.call_f32(
            0x634f40,
            p_va as u32,
            (p_va + 0xc) as u32,
            &[(p_va + 0x18) as u32],
        );
        let od = ray_point_dist_sq(&p, &q, &dir) as f32;
        assert_eq!(bd.to_bits(), od.to_bits(), "case {case}: raydist");

        let center = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let planes = capsule_kdop_planes(&center, fz(&mut s).abs() + 0.5, fz(&mut s).abs() + 0.3);
        let mut poly = mk_poly(&mut s, 4 + (case as usize % 4));
        if case % 2 == 0 {
            for v in poly.verts.iter_mut() {
                v[0] = center[0] + (v[0] - center[0]) * 0.2;
                v[1] = center[1] + (v[1] - center[1]) * 0.2;
            }
        }
        let n_planes = 3 + (case % 7) as usize;
        for (i, pl) in planes.iter().take(n_planes).enumerate() {
            wv3(&mut emu, pl_va + (i as u64) * 0x10, &pl.n);
            emu.write_u32(pl_va + (i as u64) * 0x10 + 0xc, pl.d.to_bits());
        }
        write_poly(&mut emu, poly_va, &poly);

        let eax = emu.call(0x631870, pl_va as u32, n_planes as u32, &[poly_va as u32]);
        let ours_ok = multi_plane_clip(&mut poly, &planes[..n_planes]);
        assert_eq!(eax & 0xff != 0, ours_ok, "case {case}: multiclip verdict");
        if ours_ok {
            diff_poly(&emu, poly_va, &poly, &format!("case {case}: multiclip"));
        }
        checked += 1;
    }
    eprintln!("sibling_pure: {checked} rounds bit-exact vs WoW.exe (3 functions)");
}

/// `0x634ae0` and `0x634da0`.
#[test]
fn slide_normal_difftest() {
    use benilla_re_collision::resolve_clip::{closest_tri_edge, slide_normal_select};
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let hits = emu.alloc(0x200);
    emu.write_u32(0xc4e534, hits as u32);
    let this_va = emu.alloc(0x100);
    let feat_va = emu.alloc(0x40);
    let out_va = emu.alloc(0x10);
    let p_va = emu.alloc(0x20);
    let mut s = 0x511d_e001u32;
    let mut checked = 0u32;
    for case in 0..300u32 {
        let idx = (next(&mut s) % 4) as u64;
        let basis = [
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
        ];
        let mut hn = {
            let v = [fz(&mut s), fz(&mut s), fz(&mut s) + 0.5];
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            [v[0] / l, v[1] / l, v[2] / l]
        };
        if case % 9 == 0 {
            hn = [0.0, 0.0, 1.0];
        }
        let hd = fz(&mut s);
        for (k, c) in hn.iter().enumerate() {
            emu.write_u32(hits + idx * 0x34 + (k as u64) * 4, c.to_bits());
        }
        emu.write_u32(hits + idx * 0x34 + 0xc, hd.to_bits());
        for (r, v) in basis.iter().enumerate() {
            wv3(&mut emu, hits + idx * 0x34 + 0x10 + (r as u64) * 0xc, v);
        }

        let point = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let n = {
            let v = [fz(&mut s), fz(&mut s), fz(&mut s) + 0.2];
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            [v[0] / l, v[1] / l, v[2] / l]
        };
        wv3(&mut emu, p_va, &point);
        wv3(&mut emu, p_va + 0xc, &n);
        emu.call(
            0x634da0,
            out_va as u32,
            idx as u32,
            &[p_va as u32, (p_va + 0xc) as u32],
        );
        let ours = closest_tri_edge(&basis, &point, &n);
        let bin = rv3(&emu, out_va);
        for k in 0..3 {
            assert_eq!(
                bin[k].to_bits(),
                ours[k].to_bits(),
                "case {case}: tri_edge[{k}] binary={:#010x} ours={:#010x}",
                bin[k].to_bits(),
                ours[k].to_bits()
            );
        }

        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = fz(&mut s).abs() + 0.3;
        for (k, c) in pos.iter().enumerate() {
            emu.write_u32(this_va + 0x10 + (k as u64) * 4, c.to_bits());
        }
        emu.write_u32(this_va + 0xb0, radius.to_bits());
        let f0 = Plane {
            n: [fz(&mut s), fz(&mut s), fz(&mut s)],
            d: fz(&mut s),
        };
        let f1 = Plane {
            n: [fz(&mut s), fz(&mut s), fz(&mut s)],
            d: fz(&mut s),
        };
        wv3(&mut emu, feat_va, &f0.n);
        emu.write_u32(feat_va + 0xc, f0.d.to_bits());
        wv3(&mut emu, feat_va + 0x10, &f1.n);
        emu.write_u32(feat_va + 0x1c, f1.d.to_bits());
        let ftype = [0u32, 2, 2, 3, 4, 2, 5][(case % 7) as usize];
        emu.call(
            0x634ae0,
            this_va as u32,
            0,
            &[out_va as u32, idx as u32, feat_va as u32, ftype],
        );
        let ours = slide_normal_select(&pos, radius, &hn, hd, &basis, &[f0, f1], ftype);
        let bin = rv3(&emu, out_va);
        for k in 0..3 {
            assert_eq!(
                bin[k].to_bits(),
                ours[k].to_bits(),
                "case {case}: slide[{k}] (type {ftype}) binary={:#010x} ours={:#010x}",
                bin[k].to_bits(),
                ours[k].to_bits()
            );
        }
        checked += 1;
    }

    {
        let e0 = [1.0f32, 0.0, 0.0];
        let e1 = [1.0f32, f32::from_bits(0x3500_0000), 0.5];
        let c0 = [
            (f64::from(e0[1]) * f64::from(e1[2]) - f64::from(e0[2]) * f64::from(e1[1])) as f32,
            (f64::from(e0[2]) * f64::from(e1[0]) - f64::from(e0[0]) * f64::from(e1[2])) as f32,
            (f64::from(e0[0]) * f64::from(e1[1]) - f64::from(e0[1]) * f64::from(e1[0])) as f32,
        ];
        let len = ((f64::from(c0[0]) * f64::from(c0[0]) + f64::from(c0[1]) * f64::from(c0[1]))
            + f64::from(c0[2]) * f64::from(c0[2]))
        .sqrt();
        let cz_live = (f64::from(c0[2]) / len).abs();
        let par = f64::from(f32::from_bits(0x3580_0000));
        assert!(
            cz_live < par && f64::from(cz_live as f32) >= par,
            "slide_normal straddle must bracket PAR_GUARD: cz_live={cz_live:.13e}"
        );
        let idx = 0u64;
        let hn = [0.0f32, 0.0, 1.0];
        let hd = 1000.0f32;
        let basis = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let pos = [0.0f32, 0.0, 0.0];
        let radius = 1.0f32;
        for (k, c) in hn.iter().enumerate() {
            emu.write_u32(hits + idx * 0x34 + (k as u64) * 4, c.to_bits());
        }
        emu.write_u32(hits + idx * 0x34 + 0xc, hd.to_bits());
        for (r, v) in basis.iter().enumerate() {
            wv3(&mut emu, hits + idx * 0x34 + 0x10 + (r as u64) * 0xc, v);
        }
        for (k, c) in pos.iter().enumerate() {
            emu.write_u32(this_va + 0x10 + (k as u64) * 4, c.to_bits());
        }
        emu.write_u32(this_va + 0xb0, radius.to_bits());
        let f0 = Plane { n: e0, d: 0.0 };
        let f1 = Plane { n: e1, d: 0.0 };
        wv3(&mut emu, feat_va, &f0.n);
        emu.write_u32(feat_va + 0xc, f0.d.to_bits());
        wv3(&mut emu, feat_va + 0x10, &f1.n);
        emu.write_u32(feat_va + 0x1c, f1.d.to_bits());
        emu.call(
            0x634ae0,
            this_va as u32,
            0,
            &[out_va as u32, idx as u32, feat_va as u32, 2],
        );
        let ours = slide_normal_select(&pos, radius, &hn, hd, &basis, &[f0, f1], 2);
        let bin = rv3(&emu, out_va);
        for k in 0..3 {
            assert_eq!(
                bin[k].to_bits(),
                ours[k].to_bits(),
                "fst-no-pop straddle slide[{k}]: binary={:#010x} ours={:#010x}",
                bin[k].to_bits(),
                ours[k].to_bits()
            );
        }
    }
    eprintln!("slide_normal: {checked} rounds bit-exact vs WoW.exe (2 functions)");
}

/// `0x632700`.
#[test]
fn per_face_toi_difftest() {
    use benilla_re_collision::resolve_clip::{per_face_toi, HitRecord};
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let block_va = emu.alloc(0x20);
    let recs_va = emu.alloc(0x200);
    let dir_va = emu.alloc(0x10);
    let clip_va = emu.alloc(0x40);
    let prism_va = emu.alloc(0xa0);
    let best_va = emu.alloc(8);
    let idx_va = emu.alloc(8);
    emu.write_u32(block_va + 8, recs_va as u32);
    let mut s = 0x70f_d001u32;
    let mut checked = 0u32;
    for case in 0..200u32 {
        let n_hits = 1 + (next(&mut s) % 5) as usize;
        let mut hits = vec![HitRecord::default(); n_hits];
        let dir = [fz(&mut s), fz(&mut s), fz(&mut s)];
        for h in hits.iter_mut() {
            let v = [fz(&mut s), fz(&mut s), fz(&mut s) + 0.4];
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            h.normal = [v[0] / l, v[1] / l, v[2] / l];
            if case % 3 == 0 {
                let dl = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2])
                    .sqrt()
                    .max(0.01);
                h.normal = [-dir[0] / dl, -dir[1] / dl, -dir[2] / dl];
            }
            h.d = fz(&mut s);
            h.basis = [
                [fz(&mut s) * 2.0, fz(&mut s) * 2.0, fz(&mut s) * 2.0],
                [fz(&mut s) * 2.0, fz(&mut s) * 2.0, fz(&mut s) * 2.0],
                [fz(&mut s) * 2.0, fz(&mut s) * 2.0, fz(&mut s) * 2.0],
            ];
        }
        emu.write_u32(block_va + 4, n_hits as u32);
        for (i, h) in hits.iter().enumerate() {
            let rv = recs_va + (i as u64) * 0x34;
            wv3(&mut emu, rv, &h.normal);
            emu.write_u32(rv + 0xc, h.d.to_bits());
            for (r, v) in h.basis.iter().enumerate() {
                wv3(&mut emu, rv + 0x10 + (r as u64) * 0xc, v);
            }
        }
        wv3(&mut emu, dir_va, &dir);

        let clip: Vec<Plane> = (0..3)
            .map(|_| Plane {
                n: [fz(&mut s) * 0.2, fz(&mut s) * 0.2, 1.0],
                d: fz(&mut s).abs() + 1.0,
            })
            .collect();
        for (i, pl) in clip.iter().enumerate() {
            wv3(&mut emu, clip_va + (i as u64) * 0x10, &pl.n);
            emu.write_u32(clip_va + (i as u64) * 0x10 + 0xc, pl.d.to_bits());
        }
        let center = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let prism = capsule_kdop_planes(&center, fz(&mut s).abs() + 0.5, fz(&mut s).abs() + 0.3);
        for (i, pl) in prism.iter().enumerate() {
            wv3(&mut emu, prism_va + (i as u64) * 0x10, &pl.n);
            emu.write_u32(prism_va + (i as u64) * 0x10 + 0xc, pl.d.to_bits());
        }
        let self_idx = 5 + (next(&mut s) % 4) as usize;
        let best0 = fz(&mut s).abs() + 0.5;
        emu.write_u32(best_va, best0.to_bits());
        emu.write_u32(idx_va, 0xffff_ffff);
        let eax = emu.call(
            0x632700,
            block_va as u32,
            dir_va as u32,
            &[
                clip_va as u32,
                3,
                prism_va as u32,
                self_idx as u32,
                best_va as u32,
                idx_va as u32,
            ],
        );
        let mut best = best0;
        let mut ridx = 0xffff_ffffu32;
        let ours = per_face_toi(&hits, &dir, &clip, &prism, self_idx, &mut best, &mut ridx);
        assert_eq!(eax & 0xff != 0, ours, "case {case}: pftoi verdict");
        let bb = emu.read_f32(best_va);
        assert_eq!(
            bb.to_bits(),
            best.to_bits(),
            "case {case}: pftoi best binary={:#010x} ours={:#010x}",
            bb.to_bits(),
            best.to_bits()
        );
        if ours {
            assert_eq!(emu.read_u32(idx_va), ridx, "case {case}: pftoi idx");
        }
        checked += 1;
    }
    eprintln!("per_face_toi: {checked} rounds bit-exact vs WoW.exe");
}

/// `0x632280`.
#[test]
fn candidate_collect_difftest() {
    use benilla_re_collision::resolve_clip::{candidate_collect, swept_prism_verts, HitRecord};
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this_va = emu.alloc(0x100);
    let block_va = emu.alloc(0x10);
    let recs_va = emu.alloc(0x200);
    let pos_va = emu.alloc(0x10);
    let prism_pl = emu.alloc(0xa0);
    let prism_v = emu.alloc(0x80);
    let ftab_va = emu.alloc(0x20);
    let mv_va = emu.alloc(0x10);
    let mani_va = emu.alloc(0x60);
    let cnt_va = emu.alloc(8);
    let best_va = emu.alloc(8);
    emu.write_u32(block_va + 8, recs_va as u32);

    let aux_va = emu.alloc(0x10);
    let idxout_va = emu.alloc(8);
    emu.write_u32(this_va + 0x30, aux_va as u32);
    emu.write_u32(aux_va, 0);
    emu.write_u32(aux_va + 4, 0);
    let mut s = 0xc011_ec70u32;
    let mut checked = 0u32;
    for case in 0..200u32 {
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = fz(&mut s).abs() + 0.3;
        let height = fz(&mut s).abs() + 0.6;
        emu.write_u32(this_va + 0xb0, radius.to_bits());
        emu.write_u32(this_va + 0xb4, height.to_bits());
        wv3(&mut emu, pos_va, &pos);

        emu.call(
            0x631be0,
            this_va as u32,
            0,
            &[
                pos_va as u32,
                height.to_bits(),
                prism_pl as u32,
                prism_v as u32,
                ftab_va as u32,
            ],
        );
        let mut ftab = [0u8; 0x20];
        for (i, b) in ftab.iter_mut().enumerate() {
            *b = (emu.read_u32(ftab_va + (i as u64 & !3)) >> ((i % 4) * 8)) as u8;
        }
        let planes_arr = capsule_kdop_planes(&pos, height, radius);
        let verts = swept_prism_verts(&pos, height, radius);

        let n_hits = 1 + (next(&mut s) % 4) as usize;
        let mut hits = vec![HitRecord::default(); n_hits];
        let mv = [fz(&mut s), fz(&mut s), fz(&mut s)];
        for h in hits.iter_mut() {
            let v = [fz(&mut s), fz(&mut s), fz(&mut s) + 0.4];
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            h.normal = core::array::from_fn(|k| {
                -mv[k]
                    / (mv[0] * mv[0] + mv[1] * mv[1] + mv[2] * mv[2])
                        .sqrt()
                        .max(0.01)
            });
            if case % 2 == 0 {
                h.normal = [v[0] / l, v[1] / l, v[2] / l];
            }
            h.d = fz(&mut s);
            h.basis = core::array::from_fn(|_| {
                [
                    pos[0] + fz(&mut s),
                    pos[1] + fz(&mut s),
                    pos[2] + fz(&mut s),
                ]
            });
        }
        emu.write_u32(block_va + 4, n_hits as u32);
        for (i, h) in hits.iter().enumerate() {
            let rv = recs_va + (i as u64) * 0x34;
            wv3(&mut emu, rv, &h.normal);
            emu.write_u32(rv + 0xc, h.d.to_bits());
            for (r, v) in h.basis.iter().enumerate() {
                wv3(&mut emu, rv + 0x10 + (r as u64) * 0xc, v);
            }
        }
        wv3(&mut emu, mv_va, &mv);
        let best0 = fz(&mut s).abs() + 1.0;
        emu.write_u32(best_va, best0.to_bits());
        emu.write_u32(idxout_va, 0xfeed_face);
        emu.write_u32(cnt_va, 0);
        for i in 0..0x60u64 {
            emu.write(mani_va + i, &[0u8]);
        }
        emu.call(
            0x632280,
            this_va as u32,
            0,
            &[
                block_va as u32,
                mv_va as u32,
                1.0f32.to_bits(),
                mv_va as u32,
                prism_pl as u32,
                prism_v as u32,
                ftab_va as u32,
                idxout_va as u32,
                mani_va as u32,
                cnt_va as u32,
                best_va as u32,
            ],
        );
        let mut manifold = [Plane::default(); 5];
        let mut count = 0u32;
        let mut best = best0;
        let mut rec_idx = 0xfeed_faceu32;
        candidate_collect(
            &hits,
            &mv,
            &mv,
            1.0,
            &planes_arr,
            &verts,
            &ftab,
            &mut manifold,
            &mut count,
            &mut best,
            &mut rec_idx,
        );
        assert_eq!(emu.read_u32(cnt_va), count, "case {case}: collect count");
        let bi = emu.read_u32(idxout_va);
        assert_eq!(
            bi, rec_idx,
            "case {case}: collect rec_idx binary={bi:#x} ours={rec_idx:#x}"
        );
        let bb = emu.read_f32(best_va);
        assert_eq!(
            bb.to_bits(),
            best.to_bits(),
            "case {case}: collect best binary={:#010x} ours={:#010x}",
            bb.to_bits(),
            best.to_bits()
        );
        for (i, m) in manifold.iter().enumerate().take(count as usize) {
            let pv = mani_va + (i as u64) * 0x10;
            let bn = rv3(&emu, pv);
            for (k, &c) in bn.iter().enumerate() {
                assert_eq!(
                    c.to_bits(),
                    m.n[k].to_bits(),
                    "case {case}: mani[{i}].n[{k}]"
                );
            }
            assert_eq!(
                emu.read_f32(pv + 0xc).to_bits(),
                m.d.to_bits(),
                "case {case}: mani[{i}].d"
            );
        }
        checked += 1;
    }
    eprintln!("candidate_collect: {checked} rounds bit-exact vs WoW.exe");
}

/// `0x632460`.
#[test]
fn edge_planes_difftest() {
    use benilla_re_collision::resolve_clip::test_tri_edge_planes;
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let verts_va = emu.alloc(0x40);
    let idx_va = emu.alloc(8);
    let ref_va = emu.alloc(0x10);
    let ext_va = emu.alloc(0x10);
    let out_va = emu.alloc(0x60);
    let mut s = 0xed6e_0001u32;
    for case in 0..300u32 {
        let verts = [
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
        ];
        let refn = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let ext = [fz(&mut s), fz(&mut s), fz(&mut s)];
        for (i, v) in verts.iter().enumerate() {
            wv3(&mut emu, verts_va + (i as u64) * 0xc, v);
        }
        emu.write(idx_va, &[0u8, 1, 2, 0]);
        wv3(&mut emu, ref_va, &refn);
        wv3(&mut emu, ext_va, &ext);

        for i in 0..5u64 {
            emu.write_u32(out_va + i * 0x10, 0);
            emu.write_u32(out_va + i * 0x10 + 4, 0);
            emu.write_u32(out_va + i * 0x10 + 8, 1.0f32.to_bits());
            emu.write_u32(out_va + i * 0x10 + 0xc, 0);
        }
        let eax = emu.call(
            0x632460,
            verts_va as u32,
            idx_va as u32,
            &[ref_va as u32, ext_va as u32, out_va as u32],
        );
        let (ours, ok) = test_tri_edge_planes(&verts, &ext, &refn);
        assert_eq!(eax & 0xff != 0, ok, "case {case}: ep verdict");
        if ok {
            for (i, o) in ours.iter().enumerate() {
                let pv = out_va + (i as u64) * 0x10;
                let bn = rv3(&emu, pv);
                for (k, &c) in bn.iter().enumerate() {
                    assert_eq!(
                        c.to_bits(),
                        o.n[k].to_bits(),
                        "case {case}: ep[{i}].n[{k}] binary={:#010x} ours={:#010x}",
                        c.to_bits(),
                        o.n[k].to_bits()
                    );
                }
                assert_eq!(
                    emu.read_f32(pv + 0xc).to_bits(),
                    o.d.to_bits(),
                    "case {case}: ep[{i}].d"
                );
            }
        }
    }
    eprintln!("edge_planes: 300 rounds bit-exact");
}

/// `0x631e70` off a transport.
#[test]
fn static_overlap_difftest() {
    use benilla_re_collision::resolve_clip::{static_overlap, HitRecord, TraceWorld};
    use benilla_re_collision::resolve_state::MaskSelectInput;
    use benilla_re_collision::resolve_toolkit::Aabb;
    for (pred, trace_ok) in [(false, true), (true, true), (true, false)] {
        let Some(wow) = Wow::load() else { return };
        let mut emu = Emu::new(&wow);
        emu.stub(0x5fa550, pred as u32, 0);
        emu.stub(0x6721b0, trace_ok as u32, 2);
        let this_va = emu.alloc(0x200);
        let unit_va = emu.alloc(0x1000);
        let desc_va = emu.alloc(0x20);
        let ptr2_va = emu.alloc(0x20);
        let solid_va = emu.alloc(0x200);
        let liq_va = emu.alloc(0x200);
        let disp_va = emu.alloc(0x10);
        emu.write_u32(this_va + 0x15c, unit_va as u32);
        emu.write_u32(unit_va + 8, desc_va as u32);
        emu.write_u32(unit_va + 0xe68, ptr2_va as u32);
        emu.write_u32(0xc4e534, solid_va as u32);
        emu.write_u32(0xc4e56c, liq_va as u32);
        let mut s = 0x57a7_0c01u32 ^ (pred as u32) ^ ((trace_ok as u32) << 4);
        for case in 0..150u32 {
            let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
            let radius = fz(&mut s).abs() + 0.3;
            let height = fz(&mut s).abs() + 0.6;
            let disp = [fz(&mut s) * 0.5, fz(&mut s) * 0.5, fz(&mut s) * 0.5];
            let flags = (next(&mut s) & 0x000f_0000) | if case % 3 == 0 { 0x0020_0000 } else { 0 };
            let cache = if case % 4 == 0 {
                Aabb {
                    min: [-20.0, -20.0, -20.0],
                    max: [20.0, 20.0, 20.0],
                }
            } else {
                Aabb {
                    min: [fz(&mut s) - 1.0, fz(&mut s) - 1.0, fz(&mut s) - 1.0],
                    max: [fz(&mut s) + 1.0, fz(&mut s) + 1.0, fz(&mut s) + 1.0],
                }
            };
            let n_liq = 1 + (next(&mut s) % 3) as usize;
            let liquid: Vec<HitRecord> = (0..n_liq)
                .map(|_| HitRecord {
                    normal: [fz(&mut s), fz(&mut s), fz(&mut s)],
                    d: fz(&mut s),
                    basis: Default::default(),
                })
                .collect();

            for (k, c) in pos.iter().enumerate() {
                emu.write_u32(this_va + 0x10 + (k as u64) * 4, c.to_bits());
            }
            emu.write_u32(this_va + 0xb0, radius.to_bits());
            emu.write_u32(this_va + 0xb4, height.to_bits());
            emu.write_u32(this_va + 0x40, flags);
            emu.write_u32(this_va + 0x38, 0);
            emu.write_u32(this_va + 0x3c, 0);
            emu.write_u32(this_va + 0x20, 0);
            emu.write_u32(desc_va + 8, 0x10);
            emu.write_u32(ptr2_va + 8, 0);
            for (k, c) in cache.min.iter().enumerate() {
                emu.write_u32(0xc4e5a0 + (k as u64) * 4, c.to_bits());
            }
            for (k, c) in cache.max.iter().enumerate() {
                emu.write_u32(0xc4e5ac + (k as u64) * 4, c.to_bits());
            }
            emu.write_u32(0xc4e568, n_liq as u32);
            for (i, h) in liquid.iter().enumerate() {
                let rv = liq_va + (i as u64) * 0x34;
                wv3(&mut emu, rv, &h.normal);
                emu.write_u32(rv + 0xc, h.d.to_bits());
            }
            emu.write_u32(0xc4e530, 0);
            wv3(&mut emu, disp_va, &disp);
            let eax = emu.call(
                0x631e70,
                this_va as u32,
                0,
                &[
                    emu.read_u32(disp_va),
                    emu.read_u32(disp_va + 4),
                    emu.read_u32(disp_va + 8),
                ],
            );
            let mut world = TraceWorld {
                cache_box: cache,
                solid: vec![],
                liquid: liquid.clone(),
                trace_ok,
            };
            let inp = MaskSelectInput {
                flags,
                pitch: 0.0,
                pred,
                unit_flag1: true,
                unit_flag2: false,
            };
            let ours = static_overlap(&pos, radius, height, flags, &disp, None, &inp, &mut world);
            assert_eq!(
                eax & 0xff != 0,
                ours,
                "p{pred}t{trace_ok} case {case}: verdict"
            );

            let mut bin_cache = Aabb::default();
            for k in 0..3usize {
                bin_cache.min[k] = emu.read_f32(0xc4e5a0 + (k as u64) * 4);
                bin_cache.max[k] = emu.read_f32(0xc4e5ac + (k as u64) * 4);
            }
            for k in 0..3 {
                assert_eq!(
                    bin_cache.min[k].to_bits(),
                    world.cache_box.min[k].to_bits(),
                    "p{pred}t{trace_ok} case {case}: cache.min[{k}]"
                );
                assert_eq!(
                    bin_cache.max[k].to_bits(),
                    world.cache_box.max[k].to_bits(),
                    "p{pred}t{trace_ok} case {case}: cache.max[{k}]"
                );
            }

            if ours && flags & 0x0020_0000 != 0 {
                for (i, l) in world.liquid.iter().enumerate() {
                    let rv = liq_va + (i as u64) * 0x34;
                    let bn = rv3(&emu, rv);
                    for (k, &c) in bn.iter().enumerate() {
                        assert_eq!(
                            c.to_bits(),
                            l.normal[k].to_bits(),
                            "p{pred}t{trace_ok} case {case}: liq[{i}].n[{k}]"
                        );
                    }
                    assert_eq!(
                        emu.read_f32(rv + 0xc).to_bits(),
                        l.d.to_bits(),
                        "p{pred}t{trace_ok} case {case}: liq[{i}].d"
                    );
                }
            }
        }
    }
    eprintln!("static_overlap: 450 rounds bit-exact vs WoW.exe");
}

/// `0x631e70` on a transport.
#[test]
fn static_overlap_transport_difftest() {
    use benilla_re_collision::resolve_clip::{static_overlap, HitRecord, TraceWorld};
    use benilla_re_collision::resolve_state::MaskSelectInput;
    use benilla_re_collision::resolve_toolkit::Aabb;
    for (pred, trace_ok) in [(false, true), (true, true), (true, false)] {
        let Some(wow) = Wow::load() else { return };
        let mut emu = Emu::new(&wow);
        emu.stub(0x5fa550, pred as u32, 0);
        emu.stub(0x6721b0, trace_ok as u32, 2);
        let this_va = emu.alloc(0x200);
        let unit_va = emu.alloc(0x1000);
        let desc_va = emu.alloc(0x20);
        let ptr2_va = emu.alloc(0x20);
        let solid_va = emu.alloc(0x200);
        let liq_va = emu.alloc(0x200);
        let disp_va = emu.alloc(0x10);
        let mbuf = emu.alloc(0x40);

        let mut stub = vec![0x8bu8, 0xf9, 0xbe];
        stub.extend_from_slice(&(mbuf as u32).to_le_bytes());
        stub.extend_from_slice(&[0xb9, 0x10, 0x00, 0x00, 0x00, 0xf3, 0xa5, 0xc2, 0x08, 0x00]);
        emu.write(0x630ac0, &stub);
        emu.write_u32(this_va + 0x15c, unit_va as u32);
        emu.write_u32(unit_va + 8, desc_va as u32);
        emu.write_u32(unit_va + 0xe68, ptr2_va as u32);
        emu.write_u32(0xc4e534, solid_va as u32);
        emu.write_u32(0xc4e56c, liq_va as u32);
        let mut s = 0x7a3c_1d05u32 ^ (pred as u32) ^ ((trace_ok as u32) << 4);
        for case in 0..150u32 {
            let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
            let radius = fz(&mut s).abs() + 0.3;
            let height = fz(&mut s).abs() + 0.6;
            let disp = [fz(&mut s) * 0.5, fz(&mut s) * 0.5, fz(&mut s) * 0.5];

            let m: [[f32; 3]; 4] = core::array::from_fn(|c| {
                core::array::from_fn(|_| {
                    if c == 3 {
                        fz(&mut s) * 0.25
                    } else {
                        fz(&mut s) * 0.3
                    }
                })
            });
            let flags = (next(&mut s) & 0x000f_0000) | if case % 3 == 0 { 0x0020_0000 } else { 0 };
            let cache = if case % 4 == 0 {
                Aabb {
                    min: [-20.0, -20.0, -20.0],
                    max: [20.0, 20.0, 20.0],
                }
            } else {
                Aabb {
                    min: [fz(&mut s) - 1.0, fz(&mut s) - 1.0, fz(&mut s) - 1.0],
                    max: [fz(&mut s) + 1.0, fz(&mut s) + 1.0, fz(&mut s) + 1.0],
                }
            };
            let n_liq = 1 + (next(&mut s) % 3) as usize;
            let liquid: Vec<HitRecord> = (0..n_liq)
                .map(|_| HitRecord {
                    normal: [fz(&mut s), fz(&mut s), fz(&mut s)],
                    d: fz(&mut s),
                    basis: Default::default(),
                })
                .collect();

            for (k, c) in pos.iter().enumerate() {
                emu.write_u32(this_va + 0x10 + (k as u64) * 4, c.to_bits());
            }
            for c in 0..4u64 {
                for r in 0..3u64 {
                    emu.write_u32(mbuf + c * 0x10 + r * 4, m[c as usize][r as usize].to_bits());
                }
                emu.write_u32(mbuf + c * 0x10 + 0xc, 0);
            }
            emu.write_u32(this_va + 0xb0, radius.to_bits());
            emu.write_u32(this_va + 0xb4, height.to_bits());
            emu.write_u32(this_va + 0x40, flags);
            emu.write_u32(this_va + 0x38, 0x1234_5678);
            emu.write_u32(this_va + 0x3c, 0);
            emu.write_u32(this_va + 0x20, 0);
            emu.write_u32(desc_va + 8, 0x10);
            emu.write_u32(ptr2_va + 8, 0);
            for (k, c) in cache.min.iter().enumerate() {
                emu.write_u32(0xc4e5a0 + (k as u64) * 4, c.to_bits());
            }
            for (k, c) in cache.max.iter().enumerate() {
                emu.write_u32(0xc4e5ac + (k as u64) * 4, c.to_bits());
            }
            emu.write_u32(0xc4e568, n_liq as u32);
            for (i, h) in liquid.iter().enumerate() {
                let rv = liq_va + (i as u64) * 0x34;
                wv3(&mut emu, rv, &h.normal);
                emu.write_u32(rv + 0xc, h.d.to_bits());
            }
            emu.write_u32(0xc4e530, 0);
            wv3(&mut emu, disp_va, &disp);
            let eax = emu.call(
                0x631e70,
                this_va as u32,
                0,
                &[
                    emu.read_u32(disp_va),
                    emu.read_u32(disp_va + 4),
                    emu.read_u32(disp_va + 8),
                ],
            );
            let mut world = TraceWorld {
                cache_box: cache,
                solid: vec![],
                liquid: liquid.clone(),
                trace_ok,
            };
            let inp = MaskSelectInput {
                flags,
                pitch: 0.0,
                pred,
                unit_flag1: true,
                unit_flag2: false,
            };
            let ours = static_overlap(
                &pos,
                radius,
                height,
                flags,
                &disp,
                Some(&m),
                &inp,
                &mut world,
            );
            assert_eq!(
                eax & 0xff != 0,
                ours,
                "p{pred}t{trace_ok} case {case}: verdict"
            );
            let mut bin_cache = Aabb::default();
            for k in 0..3usize {
                bin_cache.min[k] = emu.read_f32(0xc4e5a0 + (k as u64) * 4);
                bin_cache.max[k] = emu.read_f32(0xc4e5ac + (k as u64) * 4);
            }
            for k in 0..3 {
                assert_eq!(
                    bin_cache.min[k].to_bits(),
                    world.cache_box.min[k].to_bits(),
                    "p{pred}t{trace_ok} case {case}: cache.min[{k}]"
                );
                assert_eq!(
                    bin_cache.max[k].to_bits(),
                    world.cache_box.max[k].to_bits(),
                    "p{pred}t{trace_ok} case {case}: cache.max[{k}]"
                );
            }
            if ours && flags & 0x0020_0000 != 0 {
                for (i, l) in world.liquid.iter().enumerate() {
                    let rv = liq_va + (i as u64) * 0x34;
                    let bn = rv3(&emu, rv);
                    for (k, &c) in bn.iter().enumerate() {
                        assert_eq!(
                            c.to_bits(),
                            l.normal[k].to_bits(),
                            "p{pred}t{trace_ok} case {case}: liq[{i}].n[{k}]"
                        );
                    }
                    assert_eq!(
                        emu.read_f32(rv + 0xc).to_bits(),
                        l.d.to_bits(),
                        "p{pred}t{trace_ok} case {case}: liq[{i}].d"
                    );
                }
            }
        }
    }
    eprintln!("static_overlap (on-transport): 450 rounds bit-exact vs WoW.exe @ 0x631e70");
}

/// `0x632a30`.
#[test]
fn step_up_difftest() {
    use benilla_re_collision::resolve_clip::{
        step_up_probe, swept_prism_verts, HitRecord, TraceWorld,
    };
    use benilla_re_collision::resolve_state::MaskSelectInput;
    use benilla_re_collision::resolve_toolkit::Aabb;
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 1, 2);
    let this_va = emu.alloc(0x200);
    let unit_va = emu.alloc(0x1000);
    let desc_va = emu.alloc(0x20);
    let ptr2_va = emu.alloc(0x20);
    let aux_va = emu.alloc(0x10);
    let block_va = emu.alloc(0x10);
    let recs_va = emu.alloc(0x200);
    let best_va = emu.alloc(8);
    let pos2_va = emu.alloc(0x10);
    let ftabsrc_va = emu.alloc(0x80);
    emu.write_u32(this_va + 0x15c, unit_va as u32);
    emu.write_u32(unit_va + 8, desc_va as u32);
    emu.write_u32(unit_va + 0xe68, ptr2_va as u32);
    emu.write_u32(this_va + 0x30, aux_va as u32);
    emu.write_u32(block_va + 8, recs_va as u32);
    emu.write_u32(desc_va + 8, 0x10);
    let mut s = 0x57e9_0001u32;
    let mut checked = 0u32;
    for case in 0..150u32 {
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = fz(&mut s).abs() + 0.3;
        let height = fz(&mut s).abs() + 0.6;
        let flags = next(&mut s) & 0xf_0000;
        let over = [
            pos[0] + fz(&mut s) * 0.2,
            pos[1] + fz(&mut s) * 0.2,
            pos[2] + fz(&mut s) * 0.2,
        ];
        let use_over = case % 2 == 0;

        {
            let pl = emu.alloc(0xa0);
            let vv = emu.alloc(0x80);
            let pp = emu.alloc(0x10);
            wv3(&mut emu, pp, if use_over { &over } else { &pos });
            emu.call(
                0x631be0,
                this_va as u32,
                0,
                &[
                    pp as u32,
                    height.to_bits(),
                    pl as u32,
                    vv as u32,
                    ftabsrc_va as u32,
                ],
            );
        }
        let mut ftab = [0u8; 0x20];
        for (i, b) in ftab.iter_mut().enumerate() {
            *b = (emu.read_u32(ftabsrc_va + (i as u64 & !3)) >> ((i % 4) * 8)) as u8;
        }
        let n_hits = 1 + (next(&mut s) % 3) as usize;
        let hits: Vec<HitRecord> = (0..n_hits)
            .map(|_| {
                let v = [fz(&mut s) * 0.3, fz(&mut s) * 0.3, 1.0f32];
                let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                HitRecord {
                    normal: [v[0] / l, v[1] / l, v[2] / l],
                    d: fz(&mut s),
                    basis: core::array::from_fn(|_| {
                        [
                            pos[0] + fz(&mut s),
                            pos[1] + fz(&mut s),
                            pos[2] - 1.0 + fz(&mut s) * 0.3,
                        ]
                    }),
                }
            })
            .collect();
        for (k, c) in pos.iter().enumerate() {
            emu.write_u32(this_va + 0x10 + (k as u64) * 4, c.to_bits());
        }
        emu.write_u32(this_va + 0xb0, radius.to_bits());
        emu.write_u32(this_va + 0xb4, height.to_bits());
        emu.write_u32(this_va + 0x40, flags);
        emu.write_u32(this_va + 0x38, 0);
        emu.write_u32(this_va + 0x3c, 0);
        emu.write_u32(this_va + 0x20, 0);
        emu.write_u32(ptr2_va + 8, 0);
        emu.write_u32(block_va + 4, n_hits as u32);
        for (i, h) in hits.iter().enumerate() {
            let rv = recs_va + (i as u64) * 0x34;
            wv3(&mut emu, rv, &h.normal);
            emu.write_u32(rv + 0xc, h.d.to_bits());
            for (r, v) in h.basis.iter().enumerate() {
                wv3(&mut emu, rv + 0x10 + (r as u64) * 0xc, v);
            }
        }

        let cache = Aabb {
            min: [-0.1, -0.1, -0.1],
            max: [0.1, 0.1, 0.1],
        };
        for k in 0..3usize {
            emu.write_u32(0xc4e5a0 + (k as u64) * 4, cache.min[k].to_bits());
            emu.write_u32(0xc4e5ac + (k as u64) * 4, cache.max[k].to_bits());
        }
        emu.write_u32(0xc4e530, 0);
        emu.write_u32(0xc4e568, 0);
        let best0 = fz(&mut s).abs() + 0.5;
        emu.write_u32(best_va, best0.to_bits());
        wv3(&mut emu, pos2_va, &over);
        let eax = emu.call(
            0x632a30,
            this_va as u32,
            0,
            &[
                block_va as u32,
                best_va as u32,
                if use_over { pos2_va as u32 } else { 0 },
            ],
        );
        let mut world = TraceWorld {
            cache_box: cache,
            solid: vec![],
            liquid: vec![],
            trace_ok: true,
        };
        let inp = MaskSelectInput {
            flags,
            pitch: 0.0,
            pred: true,
            unit_flag1: true,
            unit_flag2: false,
        };
        let mut best = best0;
        let ours = step_up_probe(
            &pos,
            radius,
            height,
            flags,
            &inp,
            use_over.then_some(&over),
            &hits,
            &mut world,
            &ftab,
            &mut best,
        );
        assert_eq!(eax & 0xff != 0, ours, "case {case}: stepup verdict");
        let bb = emu.read_f32(best_va);
        assert_eq!(
            bb.to_bits(),
            best.to_bits(),
            "case {case}: stepup best binary={:#010x} ours={:#010x}",
            bb.to_bits(),
            best.to_bits()
        );
        let _ = swept_prism_verts;
        checked += 1;
    }
    eprintln!("step_up: {checked} rounds bit-exact vs WoW.exe");
}

/// `0x632ba0`.
#[test]
fn earliest_contact_difftest() {
    use benilla_re_collision::resolve_clip::{earliest_contact, HitRecord, TraceWorld};
    use benilla_re_collision::resolve_state::MaskSelectInput;
    use benilla_re_collision::resolve_toolkit::Aabb;
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 1, 2);
    let this_va = emu.alloc(0x200);
    let unit_va = emu.alloc(0x1000);
    let desc_va = emu.alloc(0x20);
    let ptr2_va = emu.alloc(0x20);
    let aux_va = emu.alloc(0x10);
    let block_va = emu.alloc(0x10);
    let recs_va = emu.alloc(0x200);
    let dir_va = emu.alloc(0x10);
    let contact_va = emu.alloc(0x20);
    let mani_va = emu.alloc(0x60);
    let cnt_va = emu.alloc(8);
    let outt_va = emu.alloc(8);
    let ftabsrc_va = emu.alloc(0x80);
    emu.write_u32(this_va + 0x15c, unit_va as u32);
    emu.write_u32(unit_va + 8, desc_va as u32);
    emu.write_u32(unit_va + 0xe68, ptr2_va as u32);
    emu.write_u32(this_va + 0x30, aux_va as u32);
    emu.write_u32(block_va + 8, recs_va as u32);
    emu.write_u32(desc_va + 8, 0x10);
    let mut s = 0xea51_e001u32;
    let mut checked = 0u32;
    for case in 0..150u32 {
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = fz(&mut s).abs() + 0.3;
        let height = fz(&mut s).abs() + 0.6;
        let flags = next(&mut s) & 0xf_0000;
        let dir = {
            let v = [fz(&mut s), fz(&mut s), fz(&mut s) - 0.2];
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(0.01);
            [v[0] / l, v[1] / l, v[2] / l]
        };
        let dist = if case % 9 == 0 {
            1e-8
        } else {
            fz(&mut s).abs() + 0.05
        };

        {
            let pl = emu.alloc(0xa0);
            let vv = emu.alloc(0x80);
            let pp = emu.alloc(0x10);
            wv3(&mut emu, pp, &pos);
            emu.call(
                0x631be0,
                this_va as u32,
                0,
                &[
                    pp as u32,
                    height.to_bits(),
                    pl as u32,
                    vv as u32,
                    ftabsrc_va as u32,
                ],
            );
        }
        let mut ftab = [0u8; 0x20];
        for (i, b) in ftab.iter_mut().enumerate() {
            *b = (emu.read_u32(ftabsrc_va + (i as u64 & !3)) >> ((i % 4) * 8)) as u8;
        }
        let n_hits = 1 + (next(&mut s) % 3) as usize;
        let hits: Vec<HitRecord> = (0..n_hits)
            .map(|_| {
                let v = [
                    -dir[0] + fz(&mut s) * 0.2,
                    -dir[1] + fz(&mut s) * 0.2,
                    -dir[2] + fz(&mut s) * 0.2,
                ];
                let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(0.01);
                HitRecord {
                    normal: [v[0] / l, v[1] / l, v[2] / l],
                    d: fz(&mut s),
                    basis: core::array::from_fn(|_| {
                        [
                            pos[0] + dir[0] + fz(&mut s) * 0.5,
                            pos[1] + dir[1] + fz(&mut s) * 0.5,
                            pos[2] + dir[2] + fz(&mut s) * 0.5,
                        ]
                    }),
                }
            })
            .collect();
        for (k, c) in pos.iter().enumerate() {
            emu.write_u32(this_va + 0x10 + (k as u64) * 4, c.to_bits());
        }
        emu.write_u32(this_va + 0xb0, radius.to_bits());
        emu.write_u32(this_va + 0xb4, height.to_bits());
        emu.write_u32(this_va + 0x40, flags);
        emu.write_u32(this_va + 0x38, 0);
        emu.write_u32(this_va + 0x3c, 0);
        emu.write_u32(this_va + 0x20, 0);
        emu.write_u32(ptr2_va + 8, 0);
        emu.write_u32(block_va + 4, n_hits as u32);
        for (i, h) in hits.iter().enumerate() {
            let rv = recs_va + (i as u64) * 0x34;
            wv3(&mut emu, rv, &h.normal);
            emu.write_u32(rv + 0xc, h.d.to_bits());
            for (r, v) in h.basis.iter().enumerate() {
                wv3(&mut emu, rv + 0x10 + (r as u64) * 0xc, v);
            }
        }
        let cache = Aabb {
            min: [-0.1, -0.1, -0.1],
            max: [0.1, 0.1, 0.1],
        };
        for k in 0..3usize {
            emu.write_u32(0xc4e5a0 + (k as u64) * 4, cache.min[k].to_bits());
            emu.write_u32(0xc4e5ac + (k as u64) * 4, cache.max[k].to_bits());
        }
        emu.write_u32(0xc4e530, 0);
        emu.write_u32(0xc4e568, 0);
        wv3(&mut emu, dir_va, &dir);
        emu.write_u32(outt_va, 0x7fc0_0001);
        emu.write_u32(contact_va, 0xfeed_face);
        emu.write_u32(cnt_va, 0);
        for i in 0..0x60u64 {
            emu.write(mani_va + i, &[0u8]);
        }

        let eax = emu.call(
            0x632ba0,
            this_va as u32,
            0,
            &[
                block_va as u32,
                dir_va as u32,
                dist.to_bits(),
                contact_va as u32,
                mani_va as u32,
                cnt_va as u32,
                outt_va as u32,
                0,
            ],
        );
        let mut world = TraceWorld {
            cache_box: cache,
            solid: vec![],
            liquid: vec![],
            trace_ok: true,
        };
        let inp = MaskSelectInput {
            flags,
            pitch: 0.0,
            pred: true,
            unit_flag1: true,
            unit_flag2: false,
        };
        let mut manifold = [Plane::default(); 5];
        let mut count = 0u32;
        let mut out_t = f32::from_bits(0x7fc0_0001);
        let mut rec_idx = 0xfeed_faceu32;
        let ours = earliest_contact(
            &pos,
            radius,
            height,
            flags,
            false,
            &dir,
            dist,
            &inp,
            &hits,
            &mut world,
            &ftab,
            &mut manifold,
            &mut count,
            &mut out_t,
            &mut rec_idx,
        );
        assert_eq!(eax & 0xff != 0, ours, "case {case}: ec verdict");
        let bidx = emu.read_u32(contact_va);
        assert_eq!(
            bidx, rec_idx,
            "case {case}: ec rec_idx binary={bidx:#x} ours={rec_idx:#x}"
        );
        if ours {
            let bt = emu.read_f32(outt_va);
            assert_eq!(
                bt.to_bits(),
                out_t.to_bits(),
                "case {case}: ec t binary={:#010x} ours={:#010x}",
                bt.to_bits(),
                out_t.to_bits()
            );
        }
        checked += 1;
    }
    eprintln!("earliest_contact: {checked} rounds bit-exact vs WoW.exe");
}
