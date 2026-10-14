//! A map object's bounds and vertex colours, run whole: `CMapObj::GetBoundingSphere` (`0x6a4310`),
//! `CMapObj::GetGroupBoundingSphere` (`0x6a44b0`), `PointInsidePlanes` (`0x6a4990`),
//! `CMapObjGroup::SampleVertexColor` (`0x6b9a50`) with `DominantAxis` (`0x7bf680`) inside it,
//! `CMapObjGroup::FixVertexAlpha` (`0x6c43d0`) with its distance call replaced by a chosen value,
//! and `MapObjPortalPointDistance` (`0x6c4240`) with its edge distances and polygon test replaced.

#![allow(clippy::needless_range_loop)] // the loops index the client's memory layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::wmo::{
    aabsp_nearest_dist, wmo_bounding_sphere, wmo_mocv_barycentric_color,
    wmo_point_in_convex_volume, wmo_vertex_fog_blend, ConvexPlane, FogResult, FogVertex,
    MocvSample, WmoGroup,
};
use common::assert_f32;

/// The LCG, with a coordinate drawn over four ranges from the high bits.
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    fn coord(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 4;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,
            1 => (m * 40.0 - 20.0) as f32,
            2 => (m * 4.0 - 2.0) as f32,
            _ => (m * 100.0 - 50.0) as f32,
        }
    }
}

/// A group in emulator memory, read through its pointers.
struct GroupEmu<'a, 'b> {
    emu: &'a Emu<'b>,
    this: u64,
}

impl GroupEmu<'_, '_> {
    fn ptr(&self, off: u64) -> u64 {
        u64::from(self.emu.read_u32(self.this + off))
    }

    fn byte(&self, va: u64) -> u32 {
        self.emu.read_u32(va & !3) >> ((va & 3) * 8)
    }
}

impl WmoGroup for GroupEmu<'_, '_> {
    fn movi(&self, byte_off: u32) -> u16 {
        self.byte(self.ptr(0xc8) + u64::from(byte_off)) as u16
    }

    fn movt(&self, vert: u16, axis: usize) -> f32 {
        self.emu
            .read_f32(self.ptr(0xd0) + u64::from(vert) * 0xc + axis as u64 * 4)
    }

    fn mocv(&self, vert: u16, chan: usize) -> u8 {
        self.byte(self.ptr(0xf4) + u64::from(vert) * 4 + chan as u64) as u8
    }

    fn mopy_flag(&self, tri: u16) -> u8 {
        self.byte(self.ptr(0xc4) + u64::from(tri) * 2) as u8
    }
}

/// Faces and points where storing `qb` as `f32` would move a weight by one.
const QB_PROBES: [([[u32; 3]; 3], [u32; 3]); 3] = [
    (
        [
            [0x3fe04e32, 0xbef7e828, 0x3e0c38f0],
            [0x3f0b8816, 0xbfaa33a3, 0x41abfa99],
            [0xbf687e21, 0xbdf391fc, 0x41bd10b3],
        ],
        [0xc229d0b8, 0x3f7fb1aa, 0x3f8d0da2],
    ),
    (
        [
            [0xc175f1a3, 0xbf0a3a09, 0xc12b20e7],
            [0xc17d5b15, 0x3ef966d5, 0xbea41bba],
            [0xc1411f05, 0x41294856, 0xc188a19e],
        ],
        [0xbf4f18b3, 0xbfd64eb0, 0x3ecffdd2],
    ),
    (
        [
            [0x3ec98130, 0xbdf8a6b5, 0xbf2a2b5e],
            [0xbe897ff0, 0xc0e38cd7, 0xbe9a305b],
            [0xbfa19546, 0xc1e17a98, 0xbf636e57],
        ],
        [0xc1930e90, 0xbf5bb581, 0x424480eb],
    ),
];

/// `0x6b9a50` whole on one face: `ecx` the group, then the point, the face 0 and the two outputs.
/// The projection tables are written as their initializer leaves them. Random faces and points,
/// half inside the face, cover the three projections and every weight correction; the probes are
/// faces whose second coordinate difference is not a `f32`, and colours of every byte.
#[test]
fn wmo_mocv_barycentric_color_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.write_u32(0xce2698, 1);
    for (va, v) in [
        (0xcde62c, 1),
        (0xcde630, 2),
        (0xcde634, 2),
        (0xcde638, 0),
        (0xcde63c, 0),
        (0xcde640, 1),
    ] {
        emu.write_u32(va, v);
    }
    let proj_axes = ([1usize, 2], [2usize, 0], [0usize, 1]);

    let this = emu.alloc(0x200);
    let movi = emu.alloc(0x40);
    let movt = emu.alloc(3 * 0xc + 0x10);
    let mocv = emu.alloc(3 * 4 + 0x10);
    let mopy = emu.alloc(0x40);
    emu.write_u32(this + 0xc8, movi as u32);
    emu.write_u32(this + 0xd0, movt as u32);
    emu.write_u32(this + 0xf4, mocv as u32);
    emu.write_u32(this + 0xc4, mopy as u32);
    for (k, idx) in [0u16, 1, 2].into_iter().enumerate() {
        emu.write(movi + 2 * k as u64, &idx.to_le_bytes());
    }
    let point = emu.alloc(0x10);
    let out = emu.alloc(0x10);
    let side = emu.alloc(0x10);

    let run =
        |emu: &mut Emu, verts: [[f32; 3]; 3], p: [f32; 3], cols: [u32; 3], flag: u8, ctx: &str| {
            for (i, v) in verts.iter().enumerate() {
                for (a, c) in v.iter().enumerate() {
                    emu.write_u32(movt + i as u64 * 0xc + a as u64 * 4, c.to_bits());
                }
            }
            for (i, c) in cols.iter().enumerate() {
                emu.write_u32(mocv + i as u64 * 4, *c);
            }
            for (a, c) in p.iter().enumerate() {
                emu.write_u32(point + a as u64 * 4, c.to_bits());
            }
            emu.write(mopy, &[flag, 0]);
            emu.write_u32(out, 0xdead_beef);
            emu.write(side, &[0xab]);
            let ours: MocvSample =
                wmo_mocv_barycentric_color(&GroupEmu { emu: &*emu, this }, p, 0, proj_axes);
            emu.call(
                0x6b9a50,
                this as u32,
                0,
                &[point as u32, 0, out as u32, side as u32],
            );
            let b_bgra = emu.read_u32(out).to_le_bytes();
            assert_eq!(b_bgra, ours.bgra, "{ctx}: colour (verts={verts:?} p={p:?})");
            assert_eq!(
                emu.read_u32(side).to_le_bytes()[0],
                ours.side_flag,
                "{ctx}: flag"
            );
        };

    for (k, (vb, pb)) in QB_PROBES.iter().enumerate() {
        let verts = vb.map(|v| v.map(f32::from_bits));
        let p = pb.map(f32::from_bits);
        for cols in [
            [0x00ff_0000, 0x0000_ff00, 0x0000_00ff],
            [0xffff_ffff, 0, 0x8080_8080],
            [0x0102_0304, 0xfffe_fdfc, 0x7f80_7f80],
        ] {
            run(&mut emu, verts, p, cols, 1, &format!("probe {k} {cols:x?}"));
        }
    }

    // Points whose weights are NaN, infinite, past `i32` or past `i64` once scaled by 256: the
    // `fistp` stores `0x80000000`.
    let tri = [[0.0f32, 0.0, 1.0], [4.0, 0.0, 2.0], [0.0, 4.0, 3.0]];
    for (k, p) in [
        [f32::NAN, 1.0, 1.0],
        [1.0, f32::NAN, 1.0],
        [f32::INFINITY, 1.0, 1.0],
        [1.0, f32::NEG_INFINITY, 1.0],
        [1.0e10, 1.0, 1.0],
        [1.0, -1.0e10, 1.0],
        [1.0e30, 1.0e30, 1.0],
        [-3.0e38, 3.0e38, 1.0],
    ]
    .into_iter()
    .enumerate()
    {
        run(
            &mut emu,
            tri,
            p,
            [0x00ff_0000, 0x0000_ff00, 0x0000_00ff],
            1,
            &format!("wide point {k}"),
        );
    }

    let mut rng = Lcg(0x6d6f_6376);
    let mut axis_hits = [0u32; 3];
    for i in 0..1200u32 {
        let verts = [0; 3].map(|_| [rng.coord(), rng.coord(), rng.coord()]);
        let cols = [rng.next(), rng.next(), rng.next()];
        let p = if rng.next() & 1 == 0 {
            let a = (rng.next() >> 8) as f32 / (1u32 << 24) as f32;
            let b = (1.0 - a) * ((rng.next() >> 8) as f32 / (1u32 << 24) as f32);
            let g = 1.0 - a - b;
            [0, 1, 2].map(|c| a * verts[0][c] + b * verts[1][c] + g * verts[2][c])
        } else {
            [rng.coord(), rng.coord(), rng.coord()]
        };
        let flag = (rng.next() >> 8) as u8;
        run(&mut emu, verts, p, cols, flag, &format!("fuzz[{i}]"));

        let e1 = [0, 1, 2].map(|c| verts[1][c] - verts[0][c]);
        let e2 = [0, 1, 2].map(|c| verts[2][c] - verts[0][c]);
        let cr = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        axis_hits[benilla_re_cmath::raymesh::dominant_axis(&cr)] += 1;
    }
    assert!(
        axis_hits.iter().all(|&h| h > 0),
        "projections {axis_hits:?}"
    );
}

/// `0x6c43d0` whole over one vertex (the group's batch table makes the loop run once), with
/// `0x6c4340` replaced by `fld dword [slot]; ret 8` so the distance is chosen. Distances: 0, −0,
/// below, at and above the reach 6.6666665, negative and NaN; colours with and without alpha.
#[test]
fn wmo_vertex_fog_blend_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let this = emu.alloc(0x200);
    let batches = emu.alloc(0x40);
    emu.write(this + 0x3c, &1u16.to_le_bytes());
    emu.write_u32(this + 0xdc, batches as u32);
    emu.write_u32(batches + 0x14, 0);
    let movt = emu.alloc(0x40);
    emu.write_u32(this + 0xd0, movt as u32);
    let mocv = emu.alloc(0x40);
    emu.write_u32(this + 0xf4, mocv as u32);
    let dist_slot = emu.alloc(0x10);
    let mut stub = vec![0xd9u8, 0x05];
    stub.extend_from_slice(&(dist_slot as u32).to_le_bytes());
    stub.extend_from_slice(&[0xc2, 0x08, 0x00]);
    emu.write(0x6c4340, &stub);

    let mut rng = Lcg(0xf067_b1d0);
    let dists: [f32; 14] = [
        0.0,
        0.5,
        1.0,
        3.3333,
        6.0,
        6.6666,
        6.666_666_5,
        6.7,
        10.0,
        -1.0,
        100.0,
        -0.0,
        f32::NAN,
        f32::from_bits(0x40d5_5554),
    ];
    let mut cases = Vec::new();
    for &d in &dists {
        for _ in 0..60 {
            let mut s = rng.next();
            if rng.next() & 1 == 0 {
                s &= 0x00ff_ffff;
            }
            cases.push((d, s));
        }
    }
    let (mut white, mut blended, mut untouched) = (0u32, 0u32, 0u32);
    for (d, slot0) in cases {
        emu.write_u32(dist_slot, d.to_bits());
        emu.write_u32(mocv, slot0);
        emu.call(0x6c43d0, 0, 0, &[this as u32]);
        let expect = match wmo_vertex_fog_blend(
            FogVertex {
                bgra: slot0.to_le_bytes(),
            },
            d,
        ) {
            FogResult::White => {
                white += 1;
                0xffff_ffff
            }
            FogResult::Blended(v) => {
                blended += 1;
                u32::from_le_bytes(v.bgra)
            }
            FogResult::Untouched => {
                untouched += 1;
                slot0
            }
        };
        assert_eq!(emu.read_u32(mocv), expect, "dist={d} colour={slot0:#010x}");
        assert_eq!(emu.read_u32(this + 0xbc) & 0xff, 1, "the group is marked");
    }
    assert!(white > 0 && blended > 0 && untouched > 0);
}

/// `0x6a4310` and `0x6a44b0` whole on the same random boxes, some inverted, and with the map
/// object unloaded.
#[test]
fn wmo_bounding_sphere_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let this = emu.alloc(0x400);
    let records = emu.alloc(0x40 * 0x20 + 0x10);
    emu.write_u32(this + 0x130, records as u32);
    let out = emu.alloc(0x10);

    let check = |emu: &Emu, c: [f32; 3], r: f32, ctx: &str| {
        for k in 0..3 {
            assert_f32(
                emu.read_f32(out + 4 * k as u64),
                c[k],
                &format!("{ctx}: c[{k}]"),
            );
        }
        assert_f32(emu.read_f32(out + 0xc), r, &format!("{ctx}: radius"));
    };
    let clear = |emu: &mut Emu| {
        for k in 0..4 {
            emu.write_u32(out + 4 * k, 0xdead_beef);
        }
    };

    let mut rng = Lcg(0xb05e_d05e);
    for i in 0..1200u32 {
        let loaded = i % 100 != 7;
        emu.write(this + 0x1d4, &[u8::from(loaded)]);
        let min = [rng.coord(), rng.coord(), rng.coord()];
        let max = if rng.next() & 1 == 0 {
            [0, 1, 2].map(|k| min[k] + rng.coord().abs())
        } else {
            [rng.coord(), rng.coord(), rng.coord()]
        };
        let (c, r) = wmo_bounding_sphere(loaded, min, max);

        for k in 0..3u64 {
            emu.write_u32(this + 0x1a0 + k * 4, min[k as usize].to_bits());
            emu.write_u32(this + 0x1ac + k * 4, max[k as usize].to_bits());
        }
        clear(&mut emu);
        emu.call(0x6a4310, this as u32, 0, &[out as u32]);
        check(&emu, c, r, &format!("map object {min:?} {max:?}"));

        let idx = (rng.next() >> 8) % 0x10;
        let rec = records + u64::from(idx) * 0x20;
        for k in 0..3u64 {
            emu.write_u32(rec + 4 + k * 4, min[k as usize].to_bits());
            emu.write_u32(rec + 0x10 + k * 4, max[k as usize].to_bits());
        }
        clear(&mut emu);
        emu.call(0x6a44b0, this as u32, 0, &[out as u32, idx]);
        check(&emu, c, r, &format!("group {min:?} {max:?}"));
    }
}

/// `0x6a4990` whole: random planes, half biased so the point is inside, one forced outside plane
/// otherwise, no planes, a plane through the point (distance 0), a NaN plane, and an unloaded map
/// object.
#[test]
fn wmo_point_in_convex_volume_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let this = emu.alloc(0x400);
    let planes = emu.alloc(0x40 * 0x10 + 0x10);
    emu.write_u32(this + 0x15c, planes as u32);
    let point = emu.alloc(0x10);

    let run = |emu: &mut Emu, loaded: bool, pl: &[ConvexPlane], p: [f32; 3], ctx: &str| {
        emu.write(this + 0x1d4, &[u8::from(loaded)]);
        emu.write_u32(this + 0x194, pl.len() as u32);
        for (k, plane) in pl.iter().enumerate() {
            let base = planes + k as u64 * 0x10;
            for a in 0..3 {
                emu.write_u32(base + 4 * a as u64, plane.n[a].to_bits());
            }
            emu.write_u32(base + 0xc, plane.d.to_bits());
        }
        for (a, c) in p.iter().enumerate() {
            emu.write_u32(point + a as u64 * 4, c.to_bits());
        }
        let binary = emu.call(0x6a4990, this as u32, 0, &[point as u32]) & 0xff;
        let ours = wmo_point_in_convex_volume(loaded, pl, p);
        assert_eq!(binary, ours, "{ctx}: p={p:?} planes={pl:?}");
        ours
    };

    let up = |d: f32| ConvexPlane {
        n: [0.0, 0.0, 1.0],
        d,
    };
    run(
        &mut emu,
        true,
        &[up(-1.0), up(0.0)],
        [0.0, 0.0, 0.0],
        "distance 0",
    );
    run(&mut emu, true, &[up(-0.0)], [0.0, 0.0, 0.0], "distance -0");
    run(
        &mut emu,
        true,
        &[up(f32::NAN), up(-1.0)],
        [0.0, 0.0, 0.0],
        "nan plane",
    );
    run(
        &mut emu,
        true,
        &[up(f32::MIN_POSITIVE)],
        [0.0, 0.0, 0.0],
        "just outside",
    );
    run(&mut emu, false, &[up(-1.0)], [0.0, 0.0, 0.0], "unloaded");
    run(&mut emu, false, &[], [0.0, 0.0, 0.0], "unloaded, no planes");

    let mut rng = Lcg(0x636f_6e76);
    let (mut inside, mut outside, mut none) = (0u32, 0u32, 0u32);
    for i in 0..1200u32 {
        let count = (rng.next() >> 8) % 0x10;
        let p = [rng.coord(), rng.coord(), rng.coord()];
        let bias_inside = (rng.next() >> 8) & 1 == 0;
        let mut pl = Vec::new();
        for k in 0..count {
            let force_outside = !bias_inside && k == count / 2;
            let n = if force_outside {
                [0.0f32, 0.0, 0.0]
            } else {
                [rng.coord(), rng.coord(), rng.coord()]
            };
            let np = f64::from(n[1]) * f64::from(p[1])
                + f64::from(n[2]) * f64::from(p[2])
                + f64::from(n[0]) * f64::from(p[0]);
            let d = if force_outside {
                rng.coord().abs() + 1.0
            } else if bias_inside {
                (-np - f64::from(rng.coord().abs())) as f32
            } else {
                rng.coord()
            };
            pl.push(ConvexPlane { n, d });
        }
        let ours = run(&mut emu, true, &pl, p, &format!("fuzz[{i}]"));
        match (count, ours) {
            (0, _) => none += 1,
            (_, 1) => inside += 1,
            _ => outside += 1,
        }
    }
    assert!(inside > 0 && outside > 0 && none > 0);
}

/// `0x6c4240` whole: `ecx` the map object (its `+0x134` the portal vertices), then the point and
/// the portal record (`+0` first vertex, `+2` count, `+4` the plane). `0x7c46d0` is replaced by
/// code that loads the next of a list of squared distances; `0x7bf680` returns 0 and `0x7c23e0`
/// returns the chosen verdict. Cases: the plane distance around the 1/6 threshold, the verdict both
/// ways, and lists with ties, zeros, negatives (a NaN root) and NaN.
#[test]
fn aabsp_nearest_dist_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let run_all = |verdict: u32| {
        let mut emu = Emu::new(&wow);
        let list = emu.alloc(0x100);
        let cursor = emu.alloc(0x10);
        // mov eax,[cursor]; fld dword [eax]; add dword [cursor],4; ret 4
        let mut stub = vec![0xa1u8];
        stub.extend_from_slice(&(cursor as u32).to_le_bytes());
        stub.extend_from_slice(&[0xd9, 0x00, 0x83, 0x05]);
        stub.extend_from_slice(&(cursor as u32).to_le_bytes());
        stub.extend_from_slice(&[0x04, 0xc2, 0x04, 0x00]);
        emu.write(0x7c46d0, &stub);
        emu.stub(0x7bf680, 0, 0);
        emu.stub(0x7c23e0, verdict, 2);
        let this = emu.alloc(0x200);
        emu.write_u32(this + 0x134, (this + 0x140) as u32);
        let point_va = emu.alloc(0x10);
        let face_va = emu.alloc(0x20);

        let run = |emu: &mut Emu, plane: [f32; 4], point: [f32; 3], dsq: &[f32], ctx: &str| {
            for (i, d) in dsq.iter().enumerate() {
                emu.write_u32(list + 4 * i as u64, d.to_bits());
            }
            emu.write_u32(cursor, list as u32);
            for (i, c) in point.iter().enumerate() {
                emu.write_u32(point_va + 4 * i as u64, c.to_bits());
            }
            emu.write(face_va, &0u16.to_le_bytes());
            emu.write(face_va + 2, &(dsq.len() as u16).to_le_bytes());
            for (i, c) in plane.iter().enumerate() {
                emu.write_u32(face_va + 4 + 4 * i as u64, c.to_bits());
            }
            let binary = emu.call_f32(0x6c4240, this as u32, 0, &[point_va as u32, face_va as u32]);
            let ours =
                aabsp_nearest_dist(point, plane, dsq.len() as u16, dsq, &mut || verdict != 0);
            assert_f32(
                binary,
                ours,
                &format!("verdict {verdict}, {ctx}: plane={plane:?}"),
            );
        };

        let x = [1.0f32, 0.0, 0.0, 0.0];
        let sixth = (1.0f64 / 36.0).sqrt() as f32;
        for pd in [
            0.0f32,
            0.05,
            0.1,
            -0.1,
            0.16,
            sixth,
            f32::from_bits(sixth.to_bits() - 1),
            f32::from_bits(sixth.to_bits() + 1),
            3.0,
            f32::NAN,
        ] {
            run(
                &mut emu,
                x,
                [pd, 0.0, 0.0],
                &[4.0, 9.0],
                &format!("pd={pd}"),
            );
        }
        let far = [3.0f32, 0.0, 0.0];
        run(&mut emu, x, far, &[], "no edges");
        run(&mut emu, x, far, &[9.0, 4.0, 16.0], "least in the middle");
        run(&mut emu, x, far, &[4.0, 4.0], "tie");
        run(&mut emu, x, far, &[0.0, 1.0], "zero");
        run(&mut emu, x, far, &[-1.0, 2.0], "negative");
        run(&mut emu, x, far, &[f32::NAN, 2.0, f32::NAN], "nan");
        run(&mut emu, x, far, &[f32::INFINITY, f32::MAX], "infinite");
        let mut s = 0x6e65_6172u32;
        let fz = |s: &mut u32| -> f32 {
            let pick = (common::next(s) >> 8) % 4;
            let m = f64::from(common::next(s) >> 8) / f64::from(1u32 << 24);
            match pick {
                0 => (m * 2.0 - 1.0) as f32,
                1 => (m * 20.0 - 10.0) as f32,
                2 => (m * 200.0 - 100.0) as f32,
                _ => (m * 0.02 - 0.01) as f32,
            }
        };
        for i in 0..300 {
            let plane = [fz(&mut s), fz(&mut s), fz(&mut s), fz(&mut s)];
            let point = [fz(&mut s), fz(&mut s), fz(&mut s)];
            let n = ((common::next(&mut s) >> 8) % 4) as usize + 1;
            let dsq: Vec<f32> = (0..n).map(|_| fz(&mut s).abs()).collect();
            run(&mut emu, plane, point, &dsq, &format!("fuzz[{i}]"));
        }
    };
    run_all(0);
    run_all(1);
}
