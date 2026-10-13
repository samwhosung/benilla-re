//! `0x7c22b0` and `0x7c23e0` run whole against `ray_plane` and `point_in_poly`, apart and
//! chained, and `DominantAxis` (`0x7bf680`) against `dominant_axis`. The axis table `0x7c23e0` reads is zero in the image until the static initializer
//! `0x7c18d0` runs, so each test writes the six values it stores. A NaN result matches any NaN:
//! the payload x87 carries through differs from the host's; the hit is compared exactly.

use benilla_re_cmath::raymesh::{dominant_axis, point_in_poly, ray_plane, Plane, Ray};
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 2.0 - 1.0
}

fn fz8(s: &mut u32) -> f32 {
    fz(s) * 8.0
}

fn assert_bits(b: f32, o: f32, what: &str) {
    if b.is_nan() && o.is_nan() {
        return;
    }
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        b.to_bits(),
        o.to_bits()
    );
}

fn plant_axis_table(emu: &mut Emu) {
    emu.write_u32(0xcf5c7c, 1);
    emu.write_u32(0xcf5c80, 2);
    emu.write_u32(0xcf5c84, 2);
    emu.write_u32(0xcf5c88, 0);
    emu.write_u32(0xcf5c8c, 0);
    emu.write_u32(0xcf5c90, 1);
}

fn write_ray(emu: &mut Emu, va: u64, r: &Ray) {
    for (i, c) in r.origin.iter().enumerate() {
        emu.write_u32(va + i as u64 * 4, c.to_bits());
    }
    for (i, c) in r.dir.iter().enumerate() {
        emu.write_u32(va + 0xc + i as u64 * 4, c.to_bits());
    }
}

fn write_plane(emu: &mut Emu, va: u64, p: &Plane) {
    for (i, c) in p.normal.iter().enumerate() {
        emu.write_u32(va + i as u64 * 4, c.to_bits());
    }
    emu.write_u32(va + 0xc, p.d.to_bits());
}

fn rand_ray(s: &mut u32) -> Ray {
    Ray {
        origin: [fz8(s), fz8(s), fz8(s)],
        dir: [fz(s), fz(s), fz(s)],
    }
}

fn rand_plane(s: &mut u32) -> Plane {
    let nx = fz(s);
    let ny = fz(s);
    let nz = fz(s);
    let len = ((nx as f64).powi(2) + (ny as f64).powi(2) + (nz as f64).powi(2)).sqrt();
    let (nx, ny, nz) = if len < 1.0e-9 {
        (1.0, 0.0, 0.0)
    } else {
        (
            (nx as f64 / len) as f32,
            (ny as f64 / len) as f32,
            (nz as f64 / len) as f32,
        )
    };
    Plane {
        normal: [nx, ny, nz],
        d: fz8(s),
    }
}

#[test]
fn ray_plane_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let ray_va = emu.alloc(0x20);
    let plane_va = emu.alloc(0x20);
    let t_va = emu.alloc(0x10);
    let pt_va = emu.alloc(0x10);
    let mut s = 0x51a3_77c9u32;

    let nan = f32::from_bits(0x7fc0_0000);

    let mut cases: Vec<(Ray, Plane, f32)> = vec![
        (
            Ray {
                origin: [0.0, 0.0, 5.0],
                dir: [0.0, 0.0, -1.0],
            },
            Plane {
                normal: [0.0, 0.0, 1.0],
                d: 0.0,
            },
            1.0e-3,
        ),
        (
            Ray {
                origin: [3.0, -2.0, 0.0],
                dir: [1.0, 0.0, 0.0],
            },
            Plane {
                normal: [0.0, 0.0, 1.0],
                d: 0.0,
            },
            1.0e-3,
        ),
        (
            Ray {
                origin: [3.0, -2.0, 5.0],
                dir: [1.0, 0.0, 0.0],
            },
            Plane {
                normal: [0.0, 0.0, 1.0],
                d: 0.0,
            },
            1.0e-3,
        ),
        (
            Ray {
                origin: [0.0, 0.0, 0.0],
                dir: [0.1, 0.2, 0.9],
            },
            Plane {
                normal: [0.0, 0.0, 1.0],
                d: 0.0,
            },
            1.0e-3,
        ),
        (
            Ray {
                origin: [0.0, 0.0, 4.0],
                dir: [1.0, 0.0, 1.5e-4],
            },
            Plane {
                normal: [0.0, 0.0, 1.0],
                d: 0.0,
            },
            1.0e-3,
        ),
        (
            Ray {
                origin: [0.0, 0.0, 4.0],
                dir: [1.0, 0.0, 5.0e-5],
            },
            Plane {
                normal: [0.0, 0.0, 1.0],
                d: 0.0,
            },
            1.0e-3,
        ),
        (
            Ray {
                origin: [0.0, 0.0, 1.0],
                dir: [nan, 0.0, -1.0],
            },
            Plane {
                normal: [0.0, 0.0, 1.0],
                d: 0.0,
            },
            1.0e-3,
        ),
        (
            Ray {
                origin: [0.0, 0.0, 1.0],
                dir: [0.0, 0.0, -1.0],
            },
            Plane {
                normal: [0.0, 0.0, nan],
                d: 0.0,
            },
            1.0e-3,
        ),
    ];
    for _ in 0..2000 {
        cases.push((rand_ray(&mut s), rand_plane(&mut s), 1.0e-3));

        let p = rand_plane(&mut s);
        let dir = [fz(&mut s), fz(&mut s), fz(&mut s)];

        let p0 = [fz8(&mut s), fz8(&mut s), fz8(&mut s)];
        let nn = p.normal;
        let dist = (p0[0] as f64 * nn[0] as f64
            + p0[1] as f64 * nn[1] as f64
            + p0[2] as f64 * nn[2] as f64
            + p.d as f64) as f32;
        let origin = [
            (p0[0] as f64 - dist as f64 * nn[0] as f64) as f32,
            (p0[1] as f64 - dist as f64 * nn[1] as f64) as f32,
            (p0[2] as f64 - dist as f64 * nn[2] as f64) as f32,
        ];
        cases.push((Ray { origin, dir }, p, 1.0e-3));
    }

    let mut asserts = 0usize;

    let ptr_modes = [(true, true), (true, false), (false, true), (false, false)];

    for (ci, (ray, plane, eps)) in cases.iter().enumerate() {
        for (mi, &(want_t, want_pt)) in ptr_modes.iter().enumerate() {
            write_ray(&mut emu, ray_va, ray);
            write_plane(&mut emu, plane_va, plane);

            emu.write_u32(t_va, 0xdead_beef);
            for k in 0..3 {
                emu.write_u32(pt_va + k * 4, 0xdead_beef);
            }
            let t_arg = if want_t { t_va as u32 } else { 0 };
            let pt_arg = if want_pt { pt_va as u32 } else { 0 };
            let ret = emu.call(
                0x7c22b0,
                ray_va as u32,
                plane_va as u32,
                &[t_arg, pt_arg, eps.to_bits()],
            ) & 0xff;

            let (hit, our_t, our_pt) = ray_plane(ray, plane, *eps);
            assert_eq!(
                ret != 0,
                hit,
                "0x7c22b0 case {ci} mode {mi}: binary hit={} ours hit={hit}",
                ret != 0
            );
            asserts += 1;

            if hit && want_t {
                assert_bits(emu.read_f32(t_va), our_t, &format!("0x7c22b0[{ci}.{mi}].t"));
                asserts += 1;
            }
            if hit && want_pt {
                for (k, want) in our_pt.iter().enumerate() {
                    assert_bits(
                        emu.read_f32(pt_va + k as u64 * 4),
                        *want,
                        &format!("0x7c22b0[{ci}.{mi}].point[{k}]"),
                    );
                    asserts += 1;
                }
            }
        }
    }
    eprintln!(
        "ray_plane: {asserts} bit-exact asserts over {} cases",
        cases.len()
    );
}

#[allow(clippy::too_many_arguments)]
fn run_pip(
    emu: &mut Emu,
    q_va: u64,
    v_va: u64,
    query: &[f32; 3],
    verts: &[f32],
    n: usize,
    axis_idx: usize,
) -> bool {
    for (i, c) in query.iter().enumerate() {
        emu.write_u32(q_va + i as u64 * 4, c.to_bits());
    }
    for (i, c) in verts.iter().enumerate() {
        emu.write_u32(v_va + i as u64 * 4, c.to_bits());
    }
    let ret = emu.call(
        0x7c23e0,
        q_va as u32,
        v_va as u32,
        &[n as u32, axis_idx as u32],
    ) & 0xff;
    ret != 0
}

#[test]
fn point_in_poly_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    plant_axis_table(&mut emu);
    let q_va = emu.alloc(0x10);
    let v_va = emu.alloc(0x200);
    let mut s = 0x2c6e_91afu32;

    let mut asserts = 0usize;

    let square: Vec<f32> = vec![0.0, 0.0, 7.0, 2.0, 0.0, 7.0, 2.0, 2.0, 7.0, 0.0, 2.0, 7.0];

    let tri: Vec<f32> = vec![-1.0, -1.0, 3.0, 3.0, 0.0, 3.0, 0.0, 4.0, 3.0];

    for poly_pick in 0..2 {
        let (verts, n) = if poly_pick == 0 {
            (&square, 4usize)
        } else {
            (&tri, 3usize)
        };
        for gy in -8..=16 {
            for gx in -8..=16 {
                let qx = gx as f32 * 0.25;
                let qy = gy as f32 * 0.25;
                let query = [qx, qy, fz8(&mut s)];
                let got = run_pip(&mut emu, q_va, v_va, &query, verts, n, 2);
                let our = point_in_poly(&query, verts, n, 2);
                assert_eq!(got, our, "pip grid poly {poly_pick} q=({qx},{qy})");
                asserts += 1;
            }
        }

        let mut probes: Vec<[f32; 3]> = Vec::new();
        for i in 0..n {
            probes.push([verts[3 * i], verts[3 * i + 1], 0.0]);
            let j = (i + 1) % n;
            probes.push([
                (verts[3 * i] + verts[3 * j]) * 0.5,
                (verts[3 * i + 1] + verts[3 * j + 1]) * 0.5,
                0.0,
            ]);
        }
        for query in &probes {
            let got = run_pip(&mut emu, q_va, v_va, query, verts, n, 2);
            let our = point_in_poly(query, verts, n, 2);
            assert_eq!(got, our, "pip boundary poly {poly_pick} q={query:?}");
            asserts += 1;
        }
    }

    let proj_a = [2usize, 0, 1];
    let proj_b = [1usize, 2, 0];
    for axis_idx in 0..3usize {
        for _ in 0..400 {
            let n = 3 + (next(&mut s) as usize % 5);
            let mut verts = vec![0.0f32; 3 * n];

            for (i, vchunk) in verts.chunks_mut(3).enumerate() {
                let ang = (i as f32 / n as f32) * std::f32::consts::TAU;
                let r = 1.0 + fz(&mut s) * 0.2;
                vchunk[proj_a[axis_idx]] = ang.cos() * r;
                vchunk[proj_b[axis_idx]] = ang.sin() * r;
                vchunk[axis_idx] = fz8(&mut s);
            }
            let query = [fz(&mut s) * 1.5, fz(&mut s) * 1.5, fz(&mut s) * 1.5];
            let got = run_pip(&mut emu, q_va, v_va, &query, &verts, n, axis_idx);
            let our = point_in_poly(&query, &verts, n, axis_idx);
            assert_eq!(got, our, "pip axis {axis_idx} n={n} q={query:?}");
            asserts += 1;
        }
    }

    let sq_cw: Vec<f32> = vec![0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 0.0];
    for gy in -2..=10 {
        for gx in -2..=10 {
            let query = [gx as f32 * 0.3, gy as f32 * 0.3, fz8(&mut s)];
            let got = run_pip(&mut emu, q_va, v_va, &query, &sq_cw, 4, 2);
            let our = point_in_poly(&query, &sq_cw, 4, 2);
            assert_eq!(got, our, "pip cw q={query:?}");
            asserts += 1;
        }
    }

    let degen: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 2.0, 2.0, 0.0];
    for gy in -2..=8 {
        for gx in -2..=8 {
            let query = [gx as f32 * 0.5, gy as f32 * 0.5, 0.0];
            let got = run_pip(&mut emu, q_va, v_va, &query, &degen, 3, 2);
            let our = point_in_poly(&query, &degen, 3, 2);
            assert_eq!(got, our, "pip degenerate q={query:?}");
            asserts += 1;
        }
    }

    eprintln!("point_in_poly: {asserts} bit-exact asserts");
}

#[test]
fn raymesh_composed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    plant_axis_table(&mut emu);
    let ray_va = emu.alloc(0x20);
    let plane_va = emu.alloc(0x20);
    let t_va = emu.alloc(0x10);
    let pt_va = emu.alloc(0x10);
    let q_va = emu.alloc(0x10);
    let v_va = emu.alloc(0x40);
    let mut s = 0x77a1_0b3du32;

    let tri: [f32; 9] = [-1.0, -1.0, 0.0, 2.0, -1.0, 0.0, 0.0, 2.0, 0.0];
    let plane = Plane {
        normal: [0.0, 0.0, 1.0],
        d: 0.0,
    };

    let mut asserts = 0usize;
    let mut hits = 0usize;
    for _ in 0..3000 {
        let ox = fz(&mut s) * 3.0;
        let oy = fz(&mut s) * 3.0;
        let ray = Ray {
            origin: [ox, oy, 2.0 + fz(&mut s).abs() * 3.0],
            dir: [fz(&mut s) * 0.3, fz(&mut s) * 0.3, -1.0],
        };

        write_ray(&mut emu, ray_va, &ray);
        write_plane(&mut emu, plane_va, &plane);
        emu.write_u32(t_va, 0xdead_beef);
        for k in 0..3 {
            emu.write_u32(pt_va + k * 4, 0xdead_beef);
        }
        let hit_plane = emu.call(
            0x7c22b0,
            ray_va as u32,
            plane_va as u32,
            &[t_va as u32, pt_va as u32, 0.001f32.to_bits()],
        ) & 0xff;
        let (our_hit, our_t, our_pt) = ray_plane(&ray, &plane, 0.001);
        assert_eq!(hit_plane != 0, our_hit, "composed: plane-hit mismatch");
        asserts += 1;
        if hit_plane == 0 {
            continue;
        }
        assert_bits(emu.read_f32(t_va), our_t, "composed t");
        for (k, want) in our_pt.iter().enumerate() {
            assert_bits(emu.read_f32(pt_va + k as u64 * 4), *want, "composed pt");
        }
        asserts += 4;

        for (i, c) in our_pt.iter().enumerate() {
            emu.write_u32(q_va + i as u64 * 4, c.to_bits());
        }
        for (i, c) in tri.iter().enumerate() {
            emu.write_u32(v_va + i as u64 * 4, c.to_bits());
        }
        let inside = emu.call(0x7c23e0, q_va as u32, v_va as u32, &[3, 2]) & 0xff;
        let our_in = point_in_poly(&our_pt, &tri, 3, 2);
        assert_eq!(
            inside != 0,
            our_in,
            "composed: point-in-tri mismatch at {our_pt:?}"
        );
        asserts += 1;
        if inside != 0 {
            hits += 1;
        }
    }
    eprintln!("raymesh_composed: {asserts} bit-exact asserts, {hits} ray↔triangle hits");
}

/// `DominantAxis` (`0x7bf680`, ecx = the vector) whole: ties and NaN go to the later axis, since
/// each compare is `test ah,0x41; jne`.
#[test]
fn dominant_axis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let v_va = emu.alloc(0x10);
    let mut s = 0x7bf6_8000u32;
    let run = |emu: &mut Emu, v: [f32; 3], what: &str| {
        for (i, c) in v.iter().enumerate() {
            emu.write_u32(v_va + 4 * i as u64, c.to_bits());
        }
        let got = emu.call(0x7bf680, v_va as u32, 0, &[]) as usize;
        assert_eq!(got, dominant_axis(&v), "{what}: {v:?}");
    };
    let nan = f32::NAN;
    for (v, what) in [
        ([2.0, -2.0, 1.0], "x ties y"),
        ([2.0, 1.0, -2.0], "x ties z"),
        ([1.0, 2.0, 2.0], "y ties z"),
        ([3.0, 3.0, 3.0], "all equal"),
        ([0.0, -0.0, 0.0], "signed zeros"),
        ([nan, 1.0, 0.5], "NaN x"),
        ([2.0, nan, 1.0], "NaN y"),
        ([2.0, 1.0, nan], "NaN z"),
        ([3.0, 1.0, 2.0], "x"),
        ([1.0, 3.0, 2.0], "y"),
        ([1.0, 2.0, 3.0], "z"),
    ] {
        run(&mut emu, v, what);
    }
    for i in 0..4000 {
        // small integers, so ties come often
        let v = [0, 1, 2].map(|_| ((next(&mut s) >> 24) % 7) as f32 - 3.0);
        run(&mut emu, v, &format!("fuzz{i}"));
        let w = [fz8(&mut s), fz8(&mut s), fz8(&mut s)];
        run(&mut emu, w, &format!("fuzz{i}"));
    }
}
