//! The arithmetic of the map object queries, as slices: `SegmentDirection` (`0x6bc190`),
//! `CMapObjGroup::IntersectSegment` (`0x6b92b0`) from `0x6b9354` to `0x6b9382`,
//! `CMapObjGroup::SegmentAnyHit` (`0x6b95e0`) from `0x6b9685` to `0x6b96dd`, `CMapObj::RayHitGroup`
//! (`0x6a4cc0`) from `0x6a4d3c` to `0x6a4da2`, `CMapObj::IntersectSegment` (`0x6a37b0`) from
//! `0x6a3920` to `0x6a3a2d`, and `CMapObj::GatherBoxGroups` (`0x6a5020`) from `0x6a5140` to
//! `0x6a51a9`. `Vector3LengthSquared` (`0x4549f0`) and `SetVector3` (`0x4549a0`) run inside them.

#![allow(clippy::needless_range_loop)] // the loops index the client's memory layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::wmo_query::{
    box_overlap_deltas, cmapobj_seg_hit_point, seg_face_cross, seg_face_dot, seg_normal_scale,
    seg_normalize, segment_direction, segment_reach,
};
use common::{assert_f32, frand, frand3, next, SLICE_ESP};

/// The probes of a direction's length: zero (an infinite inverse), exact lengths, a zero or
/// negative reach, and a NaN.
const REACH_PROBES: [([f32; 3], f32); 6] = [
    ([0.0, 0.0, 0.0], 1.0),
    ([1.0, 0.0, 0.0], 2.0),
    ([3.0, 4.0, 0.0], 1.0),
    ([1.0, 2.0, 2.0], 0.0),
    ([-5.0, 0.0, 0.0], -2.5),
    ([f32::NAN, 0.0, 1.0], 1.0),
];

/// `0x6b92b0` from `0x6b9354` to `0x6b9382`: the direction at `[ebp-0x2c]` (`ecx`), the reach
/// through `edi`; `len·reach` is read from `[ebp-0x1c]` and `1/len` from `[ebp-0x20]`.
#[test]
fn bspdispatch_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let reach_p = e.alloc(0x10);
    let movsd_src = e.alloc(0x40);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run = |e: &mut Emu, dir: [f32; 3], reach: f32, ctx: &str| {
        for (k, d) in [0x2c, 0x28, 0x24].into_iter().enumerate() {
            e.write_u32(ebp - d, dir[k].to_bits());
        }
        e.write_u32(reach_p, reach.to_bits());
        e.write_u32(ebp + 0x10, reach_p as u32);
        e.write_u32(ebp - 0x1c, 0xdead_beef);
        e.write_u32(ebp - 0x20, 0xdead_beef);
        e.set_reg(RegisterX86::ECX, ebp - 0x2c);
        e.set_reg(RegisterX86::EDI, reach_p);
        e.set_reg(RegisterX86::ESI, movsd_src);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b9354, 0x6b9382);
        let (scaled, inv) = segment_reach(dir, reach);
        assert_f32(e.read_f32(ebp - 0x1c), scaled, &format!("{ctx}: reach"));
        assert_f32(e.read_f32(ebp - 0x20), inv, &format!("{ctx}: inverse"));
    };

    for (dir, reach) in REACH_PROBES {
        run(&mut e, dir, reach, &format!("{dir:?} {reach}"));
    }
    let mut s = 0x6b93_5454u32;
    for i in 0..5000u32 {
        let dir = frand3(&mut s, -1000.0, 1000.0);
        let reach = frand(&mut s, -100.0, 100.0);
        run(&mut e, dir, reach, &format!("fuzz[{i}]"));
    }
}

/// `0x6b95e0` from `0x6b9685` to `0x6b96dd`: the direction at `[ebp-0x28]` (`ecx`), the reach
/// through `edi` and `[ebp+0xc]`; read back from `[ebp-0x18]`, `[ebp-0x1c]` and the direction's
/// slots.
#[test]
fn seg_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let reach_p = e.alloc(0x10);
    let movsd_src = e.alloc(0x40);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run = |e: &mut Emu, dir: [f32; 3], reach: f32, ctx: &str| {
        for (k, d) in [0x28, 0x24, 0x20].into_iter().enumerate() {
            e.write_u32(ebp - d, dir[k].to_bits());
        }
        e.write_u32(reach_p, reach.to_bits());
        e.write_u32(ebp + 0xc, reach_p as u32);
        e.write_u32(ebp + 8, movsd_src as u32);
        e.set_reg(RegisterX86::ECX, ebp - 0x28);
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::EDI, reach_p);
        e.set_reg(RegisterX86::ESI, movsd_src);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b9685, 0x6b96dd);
        let (scaled, inv, unit) = seg_normalize(dir, reach);
        assert_f32(e.read_f32(ebp - 0x18), scaled, &format!("{ctx}: reach"));
        assert_f32(e.read_f32(ebp - 0x1c), inv, &format!("{ctx}: inverse"));
        for (k, d) in [0x28, 0x24, 0x20].into_iter().enumerate() {
            assert_f32(e.read_f32(ebp - d), unit[k], &format!("{ctx}: unit[{k}]"));
        }
    };

    for (dir, reach) in REACH_PROBES {
        run(&mut e, dir, reach, &format!("{dir:?} {reach}"));
    }
    let mut s = 0x6b96_8585u32;
    for i in 0..4000u32 {
        let dir = frand3(&mut s, -1000.0, 1000.0);
        let reach = frand(&mut s, -100.0, 100.0);
        run(&mut e, dir, reach, &format!("fuzz[{i}]"));
    }
}

/// `0x6a4cc0` from `0x6a4d3c` to `0x6a4da2`: the ray at `esi`, `t` at `[ebp-4]`; the hit is read
/// from `[ebp-0x1c]`. The stretch reads a vertex index through `0xc62560`, which is pointed at
/// memory.
#[test]
fn cmapobj_seg_hit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ray = e.alloc(0x40);
    let mocv = e.alloc(0x10);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    e.write_u32(0x00c6_2560, mocv as u32);

    let run = |e: &mut Emu, p0: [f32; 3], p1: [f32; 3], t: f32, ctx: &str| {
        for k in 0..3 {
            e.write_u32(ray + 4 * k as u64, p0[k].to_bits());
            e.write_u32(ray + 0xc + 4 * k as u64, p1[k].to_bits());
            e.write_u32(ebp - 0x1c + 4 * k as u64, 0xdead_beef);
        }
        e.write_u32(ebp - 0x4, t.to_bits());
        e.set_reg(RegisterX86::ESI, ray);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a4d3c, 0x6a4da2);
        let ours = cmapobj_seg_hit_point(p0, p1, t);
        for k in 0..3 {
            let got = e.read_f32(ebp - 0x1c + 4 * k as u64);
            assert_f32(got, ours[k], &format!("{ctx}: hit[{k}]"));
        }
    };

    run(&mut e, [10.0, 20.0, 30.0], [11.0, 22.0, 33.0], 0.0, "t=0");
    run(&mut e, [10.0, 20.0, 30.0], [11.0, 22.0, 33.0], 1.0, "t=1");
    run(&mut e, [5.0, 5.0, 5.0], [5.0, 5.0, 5.0], 0.5, "coincident");
    run(&mut e, [0.0, 0.0, 0.0], [-4.0, 5.0, -6.0], 0.5, "middle");
    run(&mut e, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0], f32::NAN, "nan t");
    let mut s = 0x6a4d_3c3cu32;
    for i in 0..5000u32 {
        let p0 = frand3(&mut s, -2000.0, 2000.0);
        let p1 = frand3(&mut s, -2000.0, 2000.0);
        let t = frand(&mut s, 0.0, 1.0);
        run(&mut e, p0, p1, t, &format!("fuzz[{i}]"));
    }
}

/// `0x6a37b0` from `0x6a3920` to `0x6a39ab`: the face's indices through `0xc6255c` and the
/// vertices through `0xc62554`; the normal is read from `[ebp-0x30]`.
#[test]
fn seg_face_cross_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let verts = e.alloc(0x40);
    let face = e.alloc(0x10);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    for (k, idx) in [0u16, 1, 2].into_iter().enumerate() {
        e.write(face + 2 * k as u64, &idx.to_le_bytes());
    }
    e.write_u32(0x00c6_255c, face as u32);
    e.write_u32(0x00c6_2554, verts as u32);

    // the face's first vertex is `vc`, its second `vb`, its third `va`
    let run = |e: &mut Emu, va: [f32; 3], vb: [f32; 3], vc: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(verts + 4 * k as u64, vc[k].to_bits());
            e.write_u32(verts + 12 + 4 * k as u64, vb[k].to_bits());
            e.write_u32(verts + 24 + 4 * k as u64, va[k].to_bits());
        }
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a3920, 0x6a39ab);
        let ours = seg_face_cross(va, vb, vc);
        for k in 0..3 {
            let got = e.read_f32(ebp - 0x30 + 4 * k as u64);
            assert_f32(got, ours[k], &format!("{ctx}: n[{k}]"));
        }
    };

    run(&mut e, [1.0; 3], [1.0; 3], [1.0; 3], "degenerate");
    run(
        &mut e,
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0],
        "unit xy",
    );
    run(
        &mut e,
        [2.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "collinear",
    );
    run(
        &mut e,
        [f32::NAN, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0],
        "nan",
    );
    let mut s = 0x6a39_2020u32;
    for i in 0..4000u32 {
        let va = frand3(&mut s, -500.0, 500.0);
        let vb = frand3(&mut s, -500.0, 500.0);
        let vc = frand3(&mut s, -500.0, 500.0);
        run(&mut e, va, vb, vc, &format!("fuzz[{i}]"));
    }
}

/// `0x6a37b0` from `0x6a39bd` to `0x6a39d5`: the normal at `[ebp-0x30]`, the direction at
/// `[ebp-0x40]`, `[ebp-0x44]` and `[ebp-0x48]`; the dot is read through `edi`.
#[test]
fn seg_face_dot_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let out = e.alloc(0x10);

    let run = |e: &mut Emu, cross: [f32; 3], tdir: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(ebp - 0x30 + 4 * k as u64, cross[k].to_bits());
        }
        for (k, d) in [0x40, 0x44, 0x48].into_iter().enumerate() {
            e.write_u32(ebp - d, tdir[k].to_bits());
        }
        e.set_reg(RegisterX86::EDI, out);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a39bd, 0x6a39d5);
        assert_f32(e.read_f32(out), seg_face_dot(cross, tdir), ctx);
    };

    run(&mut e, [0.0, 0.0, 0.0], [1.0, 2.0, 3.0], "zero normal");
    run(&mut e, [1.0, 0.0, 0.0], [0.0, 0.0, 5.0], "one term");
    run(&mut e, [1.0, 1.0, 1.0], [1.0, 1.0, 1.0], "ones");
    run(
        &mut e,
        [1.0e30, -1.0e30, 1.0],
        [1.0e10, 1.0e10, 1.0],
        "cancelling",
    );
    let mut s = 0x6a39_bdbdu32;
    for i in 0..4000u32 {
        let cross = frand3(&mut s, -1e4, 1e4);
        let tdir = frand3(&mut s, -10.0, 10.0);
        run(&mut e, cross, tdir, &format!("fuzz[{i}]"));
    }
}

/// `0x6a37b0` from `0x6a3a11` to `0x6a3a2d`: the normal at `[ebp-0x24]`, `[ebp-0x20]` and
/// `[ebp-0x1c]`, the dot through `edi`, where the result is read.
#[test]
fn seg_normal_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let out = e.alloc(0x10);

    let run = |e: &mut Emu, cross: [f32; 3], dot: f32, ctx: &str| {
        for k in 0..3 {
            e.write_u32(ebp - 0x24 + 4 * k as u64, cross[k].to_bits());
        }
        e.write_u32(out, dot.to_bits());
        e.set_reg(RegisterX86::EDI, out);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a3a11, 0x6a3a2d);
        assert_f32(e.read_f32(out), seg_normal_scale(cross, dot), ctx);
    };

    run(&mut e, [0.0, 0.0, 0.0], 1.0, "zero normal");
    run(&mut e, [0.0, 0.0, 0.0], 0.0, "0/0");
    run(&mut e, [1.0, 0.0, 0.0], 7.0, "unit normal");
    run(&mut e, [3.0, 4.0, 0.0], 10.0, "length 5");
    let mut s = 0x6a3a_1111u32;
    for i in 0..4000u32 {
        let cross = frand3(&mut s, -1e3, 1e3);
        let dot = frand(&mut s, -1e5, 1e5);
        run(&mut e, cross, dot, &format!("fuzz[{i}]"));
    }
}

/// `0x6bc190` from `0x6bc193` to `0x6bc1af`: the segment at `ecx`, the output through `[ebp+8]`.
#[test]
fn segment_direction_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let seg = e.alloc(0x40);
    let out = e.alloc(0x40);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run = |e: &mut Emu, s: &[f32; 6], ctx: &str| {
        for (k, &f) in s.iter().enumerate() {
            e.write_u32(seg + 4 * k as u64, f.to_bits());
        }
        for k in 0..3u64 {
            e.write_u32(out + 4 * k, 0xdead_beef);
        }
        e.write_u32(ebp + 8, out as u32);
        e.set_reg(RegisterX86::ECX, seg);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bc193, 0x6bc1af);
        for (k, &o) in segment_direction(s).iter().enumerate() {
            assert_f32(e.read_f32(out + 4 * k as u64), o, &format!("{ctx}: [{k}]"));
        }
    };

    run(&mut e, &[0.0, 0.0, 0.0, 1.0, 2.0, 3.0], "from the origin");
    run(&mut e, &[1.0, 2.0, 3.0, 1.0, 2.0, 3.0], "zero");
    run(&mut e, &[5.0, 5.0, 5.0, 0.0, 0.0, 0.0], "negative");
    run(
        &mut e,
        &[-1.0, -2.0, -3.0, 4.0, 5.0, 6.0],
        "across the origin",
    );
    run(&mut e, &[1.0e20, 0.0, 0.0, -1.0e20, 0.0, 0.0], "large");
    run(
        &mut e,
        &[f32::NAN, 0.0, -0.0, 1.0, f32::INFINITY, 0.0],
        "nan, infinity, -0",
    );
    let fuzz = |s: &mut u32| -> f32 {
        let pick = (next(s) >> 8) % 5;
        let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,
            1 => (m * 20.0 - 10.0) as f32,
            2 => (m * 2000.0 - 1000.0) as f32,
            3 => (m * 1.0e6) as f32,
            _ => (m * 1.0e-6) as f32,
        }
    };
    let mut s = 0x7365_6764u32;
    for i in 0..6000u32 {
        let rec = [0; 6].map(|_| fuzz(&mut s));
        run(&mut e, &rec, &format!("fuzz[{i}]"));
    }
}

/// `0x6a5020` from `0x6a5140` to each delta's store: the box at `esi`, the corner at `ecx`; each
/// delta is read from `[ebp+0x10]` after its own `fstp`. Corners on a face, at −0 and at NaN
/// probe the signs the outcode reads.
#[test]
fn box_overlap_deltas_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let boxm = e.alloc(0x20);
    let qm = e.alloc(0x20);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    const HALTS: [u64; 6] = [0x6a514a, 0x6a5159, 0x6a516d, 0x6a5181, 0x6a5195, 0x6a51a9];

    let run = |e: &mut Emu, b: [f32; 6], q: [f32; 3], ctx: &str| {
        for (k, &c) in b.iter().enumerate() {
            e.write_u32(boxm + 4 * k as u64, c.to_bits());
        }
        for (k, &c) in q.iter().enumerate() {
            e.write_u32(qm + 4 * k as u64, c.to_bits());
        }
        let ours = box_overlap_deltas(b, q);
        for (k, &halt) in HALTS.iter().enumerate() {
            e.set_reg(RegisterX86::ESI, boxm);
            e.set_reg(RegisterX86::ECX, qm);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(0x6a5140, halt);
            assert_f32(e.read_f32(ebp + 0x10), ours[k], &format!("{ctx}: d[{k}]"));
        }
    };

    let unit = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
    run(&mut e, unit, [0.0, 0.0, 0.0], "inside");
    run(&mut e, unit, [-1.0, -1.0, -1.0], "on the minimum");
    run(&mut e, unit, [1.0, 1.0, 1.0], "on the maximum");
    run(&mut e, unit, [5.0, -5.0, 0.0], "outside");
    run(&mut e, [2.0; 6], [2.0, 2.0, 2.0], "point box");
    run(&mut e, [0.0; 6], [-0.0, -0.0, -0.0], "-0 corner");
    run(&mut e, [-0.0; 6], [0.0, 0.0, 0.0], "-0 box");
    run(&mut e, unit, [f32::NAN, 0.0, 0.0], "nan");
    let mut s = 0x6a50_2020u32;
    for i in 0..4000u32 {
        let lo = frand3(&mut s, -1000.0, 0.0);
        let ext = frand3(&mut s, 0.0, 1000.0);
        let b = [
            lo[0],
            lo[1],
            lo[2],
            lo[0] + ext[0],
            lo[1] + ext[1],
            lo[2] + ext[2],
        ];
        let q = frand3(&mut s, -1200.0, 1200.0);
        run(&mut e, b, q, &format!("fuzz[{i}]"));
    }
}
