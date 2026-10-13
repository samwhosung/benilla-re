//! `0x7c29f0` and `IntersectRayTriangle` (`0x7c2c40`) run whole against `segment_tri_moller` and
//! `segment_tri_moller_liquid`, over random rays and triangles, degenerate triangles, rays aimed
//! at the centroid, four slacks, reordered indices and null outputs, then the determinant gate on
//! its thresholds. The outputs start as a NaN
//! sentinel and are compared only on a hit.

use benilla_re_cmath::intersect::{segment_tri_moller, segment_tri_moller_liquid, Ray};
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}

fn write_vec3(emu: &mut Emu, va: u64, v: &[f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, c.to_bits());
    }
}

fn assert_f32(b: f32, o: f32, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        b.to_bits(),
        o.to_bits()
    );
}

#[test]
fn intersect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let ray_va = emu.alloc(0x20);
    let verts_va = emu.alloc(0x100);
    let idx_va = emu.alloc(0x10);
    let out_t = emu.alloc(8);
    let out_uv = emu.alloc(8);
    let mut s = 0xc011_1de5u32;
    let mut checked = 0u32;

    // The triangle (0,0,0), (1,0,0), (0,1,0) and a ray along z by d give det = −d exactly: d = ∓1e-6
    // puts det on each threshold (`0x7c2d37`, `0x7c2d44`), half of it inside, and NaN goes on.
    let e = 1e-6_f32;
    let probes = [-e, e, -0.5 * e, 0.5 * e, f32::NAN];
    for case in 0..400u32 + probes.len() as u32 {
        let mut ray = Ray {
            origin: [fz(&mut s), fz(&mut s), fz(&mut s)],
            dir: [fz(&mut s), fz(&mut s), fz(&mut s)],
        };
        let mut tri = [
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
        ];
        if case % 11 == 0 {
            tri[2] = tri[1]; // degenerate: det near 0
        }
        if case % 13 == 0 {
            // aimed at the centroid, so hits are common
            let c: [f32; 3] = core::array::from_fn(|i| (tri[0][i] + tri[1][i] + tri[2][i]) / 3.0);
            ray.dir = core::array::from_fn(|i| c[i] - ray.origin[i]);
        }
        let probe = case >= 400;
        if probe {
            ray = Ray {
                origin: [0.25, 0.25, 1.0],
                dir: [0.0, 0.0, probes[(case - 400) as usize]],
            };
            tri = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        }
        let eps = [0.002f32, 0.01, 0.0, 0.5][(case % 4) as usize];
        let want_t = case % 3 != 1;
        let want_uv = case % 5 != 2;

        write_vec3(&mut emu, ray_va, &ray.origin);
        write_vec3(&mut emu, ray_va + 0xc, &ray.dir);
        for (i, v) in tri.iter().enumerate() {
            write_vec3(&mut emu, verts_va + (i as u64) * 12, v);
        }
        let ord: [u16; 3] = if case % 7 == 0 && !probe {
            [2, 0, 1]
        } else {
            [0, 1, 2]
        };
        for (i, &ix) in ord.iter().enumerate() {
            emu.write(idx_va + (i as u64) * 2, &ix.to_le_bytes());
        }
        let sentinel = 0x7fc0_0001u32;
        for off in [0u64, 4] {
            emu.write_u32(out_t + off, sentinel);
            emu.write_u32(out_uv + off, sentinel);
        }

        let t_arg = if want_t { out_t as u32 } else { 0 };
        let uv_arg = if want_uv { out_uv as u32 } else { 0 };

        // 0x7c29f0, u16 indices
        let eax = emu.call(
            0x7c29f0,
            ray_va as u32,
            verts_va as u32,
            &[idx_va as u32, t_arg, uv_arg, eps.to_bits()],
        );
        let mut t = f32::from_bits(sentinel);
        let mut uv = [f32::from_bits(sentinel); 2];
        let hit = segment_tri_moller(
            &ray,
            &tri,
            &[ord[0], ord[1], ord[2]],
            want_t.then_some(&mut t),
            want_uv.then_some(&mut uv),
            eps,
        );
        assert_eq!(eax & 0xff != 0, hit, "case {case}: 0x7c29f0 verdict");
        if hit && want_t {
            assert_f32(emu.read_f32(out_t), t, &format!("case {case}: 0x7c29f0 t"));
        }
        if hit && want_uv {
            assert_f32(
                emu.read_f32(out_uv),
                uv[0],
                &format!("case {case}: 0x7c29f0 u"),
            );
            assert_f32(
                emu.read_f32(out_uv + 4),
                uv[1],
                &format!("case {case}: 0x7c29f0 v"),
            );
        }

        // 0x7c2c40, u32 indices
        let idx32: [u32; 3] = [ord[0] as u32, ord[1] as u32, ord[2] as u32];
        for (i, &ix) in idx32.iter().enumerate() {
            emu.write_u32(idx_va + (i as u64) * 4, ix);
        }
        for off in [0u64, 4] {
            emu.write_u32(out_t + off, sentinel);
            emu.write_u32(out_uv + off, sentinel);
        }
        let eax = emu.call(
            0x7c2c40,
            ray_va as u32,
            verts_va as u32,
            &[idx_va as u32, t_arg, uv_arg, eps.to_bits()],
        );
        let mut t = f32::from_bits(sentinel);
        let mut uv = [f32::from_bits(sentinel); 2];
        let hit = segment_tri_moller_liquid(
            &ray,
            &tri,
            &idx32,
            want_t.then_some(&mut t),
            want_uv.then_some(&mut uv),
            eps,
        );
        assert_eq!(eax & 0xff != 0, hit, "case {case}: 0x7c2c40 verdict");
        if hit && want_t {
            assert_f32(emu.read_f32(out_t), t, &format!("case {case}: 0x7c2c40 t"));
        }
        if hit && want_uv {
            assert_f32(
                emu.read_f32(out_uv),
                uv[0],
                &format!("case {case}: 0x7c2c40 u"),
            );
            assert_f32(
                emu.read_f32(out_uv + 4),
                uv[1],
                &format!("case {case}: 0x7c2c40 v"),
            );
        }
        checked += 1;
    }
    eprintln!("intersect: {checked} rounds bit-exact, two functions per round");
}
