//! The plane tests, each run whole: `GetPlaneOutcode` (`0x686c20`), `TestSpherePlanes`
//! (`0x686b80`), `TestBoxPlanes` (`0x686940`) and `TranslateConvexVolume` (`0x686820`).

#![allow(clippy::needless_range_loop)] // the loops index the client's memory layout

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_terrain::frustum::{volume_translate, Plane, QueryVolume};
use common::assert_f32;

/// Random finite floats after a set of edge values; NaN is left out of the values.
fn synth_f32(n: usize, seed: u32, count: usize) -> Vec<Vec<f32>> {
    let edges = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        33.5,
        -33.5,
        0.019_444_443,
        -0.019_444_443,
        0.02,
        -0.02,
        533.0,
        -17066.0,
        1e30,
        -1e30,
        123.456,
    ];
    let mut v: Vec<Vec<f32>> = edges.iter().map(|&e| vec![e; n]).collect();
    let mut s = seed;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..count {
        let mut m = vec![0f32; n];
        for x in m.iter_mut() {
            let mut f = f32::from_bits(next());
            while f.is_nan() {
                f = f32::from_bits(next());
            }
            *x = f;
        }
        v.push(m);
    }
    v
}

/// Writes floats to the emulator's memory.
fn put(emu: &mut Emu, base: u64, v: &[f32]) {
    for (i, &f) in v.iter().enumerate() {
        emu.write_u32(base + i as u64 * 4, f.to_bits());
    }
}

/// Six planes from 24 floats.
fn planes6(flat: &[f32]) -> [Plane; 6] {
    let mut p = [[0f32; 4]; 6];
    for (i, plane) in p.iter_mut().enumerate() {
        plane.copy_from_slice(&flat[i * 4..i * 4 + 4]);
    }
    p
}

#[test]
fn outcode6() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let planes_mem = emu.alloc(6 * 0x10);
    let point_mem = emu.alloc(0x10);
    let code_mem = emu.alloc(8);

    // 24 floats a plane set, 3 a point
    let plane_sets = synth_f32(24, 0x1357_9bdf, 400);
    let points = synth_f32(3, 0x2468_ace0, 400);
    let mut checked = 0u32;
    for (n, ps) in plane_sets.iter().enumerate() {
        let pt: &[f32; 3] = points[n % points.len()].as_slice().try_into().unwrap();
        let planes = planes6(ps);
        put(&mut emu, planes_mem, ps);
        put(&mut emu, point_mem, pt);

        // ecx = planes, the point and the code on the stack
        emu.write_u32(code_mem, 0xdead_beef);
        emu.call(
            0x686c20,
            planes_mem as u32,
            0,
            &[point_mem as u32, code_mem as u32],
        );
        assert_eq!(
            emu.read_u32(code_mem),
            benilla_re_terrain::frustum::outcode6(&planes, pt),
            "outcode6 set#{n}"
        );
        checked += 1;
    }
    eprintln!("outcode6: {checked} cases bit-exact vs WoW.exe @ 0x686c20");
}

#[test]
fn sphere_frustum() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // the six planes at +0x0
    let vol_mem = emu.alloc(0x100);
    let sphere_mem = emu.alloc(0x10);

    let vol_sets = synth_f32(6 * 4 + 8 * 3, 0x5a5a_a5a5, 400); // 24 plane floats, then 24 more
    let spheres = synth_f32(4, 0x7777_1111, 400);
    let mut checked = 0u32;
    for (n, vs) in vol_sets.iter().enumerate() {
        let sphere: &[f32; 4] = spheres[n % spheres.len()].as_slice().try_into().unwrap();
        let planes = planes6(&vs[..24]);
        // the planes at +0x0, 16 bytes apart
        for (i, plane) in planes.iter().enumerate() {
            put(&mut emu, vol_mem + i as u64 * 0x10, plane);
        }
        put(&mut emu, sphere_mem, sphere);

        // ecx = planes, the sphere on the stack; eax
        let ret = emu.call(0x686b80, vol_mem as u32, 0, &[sphere_mem as u32]);
        assert_eq!(
            ret,
            benilla_re_terrain::frustum::sphere_frustum(&planes, sphere),
            "sphere_frustum set#{n}"
        );
        checked += 1;
    }
    eprintln!("sphere_frustum: {checked} cases bit-exact vs WoW.exe @ 0x686b80");
}

#[test]
fn aabb_frustum_pvertex() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let planes_mem = emu.alloc(6 * 0x10);
    let aabb_mem = emu.alloc(0x20); // min at +0x0, max at +0xc

    let plane_sets = synth_f32(24, 0x0f1e_2d3c, 400);
    let mins = synth_f32(3, 0xc505_5a11, 400);
    let extents = synth_f32(3, 0x9e11_a7e5, 400);
    let mut checked = 0u32;
    for (n, ps) in plane_sets.iter().enumerate() {
        let mn: &[f32; 3] = mins[n % mins.len()].as_slice().try_into().unwrap();
        // a box with max at least min where the values are finite, and raw pairs too, so the
        // corner is picked either way
        let ext = &extents[n % extents.len()];
        let mx: [f32; 3] = [
            (f64::from(mn[0]) + f64::from(ext[0].abs())) as f32,
            (f64::from(mn[1]) + f64::from(ext[1].abs())) as f32,
            (f64::from(mn[2]) + f64::from(ext[2].abs())) as f32,
        ];
        let planes = planes6(ps);
        put(&mut emu, planes_mem, ps);
        put(&mut emu, aabb_mem, mn); // min at +0x0
        put(&mut emu, aabb_mem + 0xc, &mx); // max at +0xc

        let ret = emu.call(0x686940, planes_mem as u32, 0, &[aabb_mem as u32]);
        assert_eq!(
            ret,
            benilla_re_terrain::frustum::aabb_frustum_pvertex(&planes, &mx, mn),
            "aabb_pvertex set#{n}"
        );
        checked += 1;
    }
    eprintln!("aabb_frustum_pvertex: {checked} cases bit-exact vs WoW.exe @ 0x686940");
}

#[test]
fn frustum_volume_translate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let vol_mem = emu.alloc(0x100); // planes at +0x0, corners at +0x60, the points at +0xc0 and +0xcc
    let delta_mem = emu.alloc(0x10);

    // 54 floats: 24 for the planes, 24 for the corners, 6 for the points
    let vol_sets = synth_f32(54, 0x600d_cafe, 400);
    let deltas = synth_f32(3, 0xfeed_f00d, 400);
    let mut checked = 0u32;
    for (n, vs) in vol_sets.iter().enumerate() {
        let delta: &[f32; 3] = deltas[n % deltas.len()].as_slice().try_into().unwrap();
        let planes = planes6(&vs[..24]);
        let mut corners = [[0f32; 3]; 8];
        for (i, c) in corners.iter_mut().enumerate() {
            c.copy_from_slice(&vs[24 + i * 3..24 + i * 3 + 3]);
        }
        let anchor_a: [f32; 3] = vs[48..51].try_into().unwrap();
        let anchor_b: [f32; 3] = vs[51..54].try_into().unwrap();

        // the volume as the function reads it
        for (i, plane) in planes.iter().enumerate() {
            put(&mut emu, vol_mem + i as u64 * 0x10, plane);
        }
        for (i, c) in corners.iter().enumerate() {
            put(&mut emu, vol_mem + 0x60 + i as u64 * 0xc, c);
        }
        put(&mut emu, vol_mem + 0xc0, &anchor_a);
        put(&mut emu, vol_mem + 0xcc, &anchor_b);
        put(&mut emu, delta_mem, delta);

        emu.call(0x686820, vol_mem as u32, 0, &[delta_mem as u32]);

        let mut ours = QueryVolume {
            planes,
            corners,
            anchor_a,
            anchor_b,
        };
        volume_translate(&mut ours, delta);

        // every field read back
        for (i, plane) in ours.planes.iter().enumerate() {
            // only d (+0xc) changes; the normals are checked too
            for c in 0..4 {
                assert_f32(
                    emu.read_f32(vol_mem + i as u64 * 0x10 + c as u64 * 4),
                    plane[c],
                    &format!("plane[{i}][{c}] set#{n}"),
                );
            }
        }
        for (i, corner) in ours.corners.iter().enumerate() {
            for c in 0..3 {
                assert_f32(
                    emu.read_f32(vol_mem + 0x60 + i as u64 * 0xc + c as u64 * 4),
                    corner[c],
                    &format!("corner[{i}][{c}] set#{n}"),
                );
            }
        }
        for c in 0..3 {
            assert_f32(
                emu.read_f32(vol_mem + 0xc0 + c as u64 * 4),
                ours.anchor_a[c],
                &format!("anchor_a[{c}] set#{n}"),
            );
            assert_f32(
                emu.read_f32(vol_mem + 0xcc + c as u64 * 4),
                ours.anchor_b[c],
                &format!("anchor_b[{c}] set#{n}"),
            );
        }
        checked += 1;
    }
    eprintln!("frustum_volume_translate: {checked} volumes bit-exact vs WoW.exe @ 0x686820");
}
