//! The clippers, each run whole: `GetUnitCubeOutcode` (`0x686d20`), `ClipToUnitCube`
//! (`0x686db0`) and `ClipPortalPolygon` (`0x6b4a80`) with the planes at `0xc7d0dc` the test writes.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::assert_f32;

fn assert_poly(binary: &[[f32; 3]], ours: &[[f32; 3]], ctx: &str) {
    assert_eq!(
        binary.len(),
        ours.len(),
        "{ctx}: vertex count binary={} ours={}",
        binary.len(),
        ours.len()
    );
    for (i, (b, o)) in binary.iter().zip(ours.iter()).enumerate() {
        for c in 0..3 {
            assert_f32(b[c], o[c], &format!("{ctx}[{i}].{c}"));
        }
    }
}

/// Random finite floats: edge values about the cube's faces and the planes' bands, and values in
/// `[-4, 4)`, the size of the clip-space vertices and unit normals the callers pass. NaN is left
/// out. With magnitudes near `f32`'s limit the products overflow, and there the reimplementation
/// does not match the exe; no caller passes such values.
fn synth_coord(s: &mut u32) -> f32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    if *s & 0x10 == 0 {
        let edges = [
            0.0f32, -0.0, 1.0, -1.0, 0.5, 0.25, 0.75, 1.5, -0.5, 2.0, 0.01, -0.01, 0.009, -0.009,
            1.000001, -0.000001,
        ];
        edges[(*s >> 5) as usize % edges.len()]
    } else {
        // [-4, 4) from the high bits
        let u = (*s >> 8) as f32 / (1u32 << 24) as f32; // [0, 1)
        u * 8.0 - 4.0
    }
}

/// Reads `count` vertices 12 bytes apart, as `ClipPortalPolygon` writes them.
fn read_poly(emu: &Emu, base: u64, count: usize) -> Vec<[f32; 3]> {
    (0..count)
        .map(|i| {
            let b = base + i as u64 * 12;
            [emu.read_f32(b), emu.read_f32(b + 4), emu.read_f32(b + 8)]
        })
        .collect()
}

/// Reads `count` vertices through an array of pointers, as `ClipToUnitCube` writes them: its pools
/// at `0xc7b678` hold pointers to vertices.
fn read_poly_indirect(emu: &Emu, base: u64, count: usize) -> Vec<[f32; 3]> {
    (0..count)
        .map(|i| {
            let p = emu.read_u32(base + i as u64 * 4) as u64;
            [emu.read_f32(p), emu.read_f32(p + 4), emu.read_f32(p + 8)]
        })
        .collect()
}

#[test]
fn unitcell_outcode() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let out = emu.alloc(0x20); // 7 floats
    let p = emu.alloc(0x10);
    let mut s: u32 = 0x0123_4567;
    let mut checked = 0u32;
    for _ in 0..2000 {
        let v = [
            synth_coord(&mut s),
            synth_coord(&mut s),
            synth_coord(&mut s),
        ];
        for (i, &f) in v.iter().enumerate() {
            emu.write_u32(p + i as u64 * 4, f.to_bits());
        }
        // ecx = out, p on the stack, `ret 4`
        emu.call(0x686d20, out as u32, 0, &[p as u32]);
        let ours = benilla_re_terrain::unitcell_outcode(&v);
        for (i, &o) in ours[..6].iter().enumerate() {
            assert_f32(emu.read_f32(out + i as u64 * 4), o, "outcode dist");
        }
        // the sign word compared as bits
        assert_eq!(
            emu.read_u32(out + 6 * 4),
            ours[6].to_bits(),
            "outcode packed word"
        );
        checked += 1;
    }
    eprintln!("unitcell_outcode: {checked} points bit-exact vs WoW.exe @ 0x686d20");
}

#[test]
fn unitcube_clip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.stub(0x409aef, 0, 1); // `_atexit` of the one-time pool setup does nothing
    let inbuf = emu.alloc(0x2000); // up to about 170 vertices, 12 bytes apart
    let out_ptr = emu.alloc(8);
    let out_cnt = emu.alloc(8);
    let mut s: u32 = 0xfeed_1234;
    let mut checked = 0u32;
    let mut nonempty = 0u32;
    // Polygons wholly inside [0, 1]³, every code 0, which random inputs never give: the input is
    // returned as it is. Vertices on a face, 0.0 or 1.0, have code 0 too.
    let inside_probes: [&[[f32; 3]]; 2] = [
        &[[0.2, 0.2, 0.2], [0.8, 0.3, 0.4], [0.5, 0.7, 0.6]],
        &[
            [0.0, 0.5, 0.5],
            [0.5, 0.0, 0.999],
            [1.0, 0.5, 0.5],
            [0.5, 1.0, 0.001],
        ],
    ];
    for case in 0..4002usize {
        // The projector (0x50de90) clips small convex polygons of 3 to 8 vertices near the cube.
        // More vertices or larger coordinates overflow the function's fixed stack pools, which no
        // caller does.
        let verts: Vec<[f32; 3]> = if let Some(probe) = inside_probes.get(case) {
            probe.to_vec()
        } else {
            let count = 3 + (case % 6);
            let mut v: Vec<[f32; 3]> = Vec::with_capacity(count);
            for _ in 0..count {
                v.push([
                    synth_coord(&mut s),
                    synth_coord(&mut s),
                    synth_coord(&mut s),
                ]);
            }
            v
        };
        let count = verts.len();
        for (i, v) in verts.iter().enumerate() {
            for (c, &f) in v.iter().enumerate() {
                emu.write_u32(inbuf + (i * 3 + c) as u64 * 4, f.to_bits());
            }
        }
        emu.write_u32(out_ptr, 0xdead_beef);
        emu.write_u32(out_cnt, 0xdead_beef);
        // ecx = inbuf, edx = count, &out_ptr and &out_cnt on the stack, `ret 8`
        let ret = emu.call(
            0x686db0,
            inbuf as u32,
            count as u32,
            &[out_ptr as u32, out_cnt as u32],
        );
        let ours = benilla_re_terrain::unitcube_clip(&verts);
        match ours {
            None => assert_eq!(ret, 0, "case {case}: empty clip must return 0"),
            Some(poly) => {
                assert_eq!(ret, 1, "case {case}: nonempty clip must return 1");
                let pool = emu.read_u32(out_ptr) as u64;
                let n = emu.read_u32(out_cnt) as usize;
                let bin = read_poly_indirect(&emu, pool, n);
                assert_poly(&bin, &poly, &format!("unitcube_clip case {case}"));
                nonempty += 1;
            }
        }
        checked += 1;
    }
    eprintln!(
        "unitcube_clip: {checked} polygons bit-exact vs WoW.exe @ 0x686db0 ({nonempty} nonempty)"
    );
}

#[test]
fn poly_clip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.stub(0x409aef, 0, 1); // `_atexit` of the one-time pool setup does nothing

    // The planes at 0xc7d0d8 + 0x10·k, (nx, ny, nz, unused): the function walks [esi] from 0xc7d0dc
    // (ny) with nx at [esi-4] and nz at [esi+4]. First the 45° pyramid about +z that 0x680610 builds,
    // then other sets.
    let plane_base = 0xc7d0d8u64; // plane 0's nx
    let write_planes = |emu: &mut Emu, planes: &[[f32; 3]; 4]| {
        for (k, pl) in planes.iter().enumerate() {
            let b = plane_base + k as u64 * 0x10;
            emu.write_u32(b, pl[0].to_bits()); // nx
            emu.write_u32(b + 4, pl[1].to_bits()); // ny
            emu.write_u32(b + 8, pl[2].to_bits()); // nz
            emu.write_u32(b + 0xc, 0); // unused
        }
    };

    // The only planes a caller (0x6b46f0) clips against are that pyramid, ±1/√2 about +z; two
    // rotations of it are run too. All are unit normals through the origin: with normals of any
    // length, crossings that nearly coincide come out differently from the exe, and no caller
    // passes such planes.
    let inv = 1.0f32 / 2.0f32.sqrt();
    let pyramid: [[f32; 3]; 4] = [
        [inv, 0.0, inv],  // x >= -z
        [-inv, 0.0, inv], // x <=  z
        [0.0, inv, inv],  // y >= -z
        [0.0, -inv, inv], // y <=  z
    ];
    let inv3 = 1.0f32 / 3.0f32.sqrt();
    let steep: [[f32; 3]; 4] = [
        [inv3, inv3, inv3],
        [-inv3, -inv3, inv3],
        [inv3, -inv3, inv3],
        [-inv3, inv3, inv3],
    ];
    let half = 1.0f32 / 5.0f32.sqrt();
    let wide: [[f32; 3]; 4] = [
        [2.0 * half, 0.0, half],
        [-2.0 * half, 0.0, half],
        [0.0, 2.0 * half, half],
        [0.0, -2.0 * half, half],
    ];
    let plane_sets = [pyramid, steep, wide];

    let inbuf = emu.alloc(0x800);
    let out_ptr = emu.alloc(8);
    let out_cnt = emu.alloc(8);
    let mut s: u32 = 0x2bad_c0de;
    let mut checked = 0u32;
    let mut nonempty = 0u32;
    for case in 0..3000 {
        let planes = plane_sets[(case % 3) as usize];
        write_planes(&mut emu, &planes);

        // 3 to 8 vertices near 1 with z > 0, so the pyramid clips
        let count = 3 + (case % 6) as usize;
        let mut verts: Vec<[f32; 3]> = Vec::with_capacity(count);
        for _ in 0..count {
            verts.push([
                synth_coord(&mut s),
                synth_coord(&mut s),
                synth_coord(&mut s).abs() + 0.3,
            ]);
        }
        // The function reads vertices 16 bytes apart and takes +0x0, +0x4 and +0xc, dropping +0x8:
        // the caller passes (x, y, z, w) and the clip runs on (x, y, w). The triple is written to
        // +0x0, +0x4 and +0xc, with a NaN at +0x8 that must not be read.
        for (i, v) in verts.iter().enumerate() {
            let b = inbuf + i as u64 * 16;
            emu.write_u32(b, v[0].to_bits());
            emu.write_u32(b + 4, v[1].to_bits());
            emu.write_u32(b + 8, 0x7fc0_0000); // +8 is never read
            emu.write_u32(b + 0xc, v[2].to_bits());
        }
        emu.write_u32(out_ptr, 0xdead_beef);
        emu.write_u32(out_cnt, 0xdead_beef);
        // `__cdecl`: verts, count, &out_ptr, &out_cnt; `ret 0x10`
        emu.call(
            0x6b4a80,
            0,
            0,
            &[inbuf as u32, count as u32, out_ptr as u32, out_cnt as u32],
        );
        let bin_ptr = emu.read_u32(out_ptr);
        let bin_cnt = emu.read_u32(out_cnt) as usize;
        let ours = benilla_re_terrain::poly_clip(&verts, &planes);
        match ours {
            None => {
                assert_eq!(bin_ptr, 0, "case {case}: empty clip writes *out_ptr=0");
                assert_eq!(bin_cnt, 0, "case {case}: empty clip writes *out_count=0");
            }
            Some(poly) => {
                let bin = read_poly(&emu, bin_ptr as u64, bin_cnt);
                assert_poly(&bin, &poly, &format!("poly_clip case {case}"));
                nonempty += 1;
            }
        }
        checked += 1;
    }
    eprintln!(
        "poly_clip: {checked} polygons bit-exact vs WoW.exe @ 0x6b4a80 ({nonempty} nonempty)"
    );
}
