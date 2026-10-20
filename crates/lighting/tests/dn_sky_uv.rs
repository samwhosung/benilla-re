//! `ProjectCloudLight` (`0x6cf9c0`) and `ProjectToCloudTexture` (`0x6cf870`) whole, against
//! `sky_dome_ray_point` and `sky_cloud_uv_scroll`. The quadratic solver `0x454f40` and the C
//! runtime's `acos` `0x741820` run in the emulator inside the calls. Inputs are finite: a NaN or
//! infinite direction sends `acos` into the C runtime's error path.
//!
//! Host double (`docs/METHOD.md#the-harness`): `sky_dome_ray_point_difftest` runs `fcos` at
//! `0x6cf9dc`. `sky_cloud_uv_scroll_difftest` runs `fcos` at `0x6cf8ab` and `fpatan` at `0x741866`.
//! The emulator computes each in host `double`, so where its result reaches a compared value the
//! host's math stands in for the x87's, and the test shows the code around it.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::dn_sky_uv::{
    sky_cloud_uv_scroll, sky_dome_ray_point, CloudUv, Vec3, ORIGIN_GLOBALS,
};
use common::{assert_f32, next};

/// A finite float across signs: a direction component, a world delta, a tiny value near the 1e-5
/// cutoff, or a mid value.
fn fz(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 4;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,
        1 => (m * 2000.0 - 1000.0) as f32,
        2 => (m * 0.1 - 0.05) as f32,
        _ => (m * 20.0 - 10.0) as f32,
    }
}

/// `0x6cf9c0` over directions with `v.z` of 0 (where `b` is 0), above 0 and below 0 (the two signs
/// of `b`, so both branches of the solver's sign split), axis rays, a non-zero origin and point,
/// large and small directions, and a fuzz. A direction of zero or non-finite has no root; the
/// transcription returns none and the case is skipped.
#[test]
fn sky_dome_ray_point_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let p_buf = emu.alloc(0x10);
    let v_buf = emu.alloc(0x10);
    let out_buf = emu.alloc(0x10);
    let mut s = 0x736b_7964_u32;

    let run = |emu: &mut Emu, p: Vec3, v: Vec3, origin: [f32; 3], ctx: &str| {
        let Some(ours) = sky_dome_ray_point(p, v, origin) else {
            return;
        };
        for (buf, vec) in [(p_buf, p), (v_buf, v)] {
            emu.write_u32(buf, vec.x.to_bits());
            emu.write_u32(buf + 4, vec.y.to_bits());
            emu.write_u32(buf + 8, vec.z.to_bits());
        }
        for (g, o) in ORIGIN_GLOBALS.iter().zip(origin.iter()) {
            emu.write_u32(*g, o.to_bits());
        }
        emu.write_u32(out_buf, 0xdead_beef);
        emu.write_u32(out_buf + 4, 0xdead_beef);
        emu.write_u32(out_buf + 8, 0xdead_beef);
        emu.call(
            0x6cf9c0,
            0,
            0,
            &[p_buf as u32, v_buf as u32, out_buf as u32],
        );
        assert_f32(emu.read_f32(out_buf), ours.x, &format!("{ctx} out.x"));
        assert_f32(emu.read_f32(out_buf + 4), ours.y, &format!("{ctx} out.y"));
        assert_f32(emu.read_f32(out_buf + 8), ours.z, &format!("{ctx} out.z"));
    };

    let zero = [0.0f32; 3];
    let pz = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let edges: [(Vec3, Vec3, [f32; 3]); 8] = [
        (
            pz,
            Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            zero,
        ),
        (
            pz,
            Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            zero,
        ),
        (
            pz,
            Vec3 {
                x: 0.3,
                y: 0.4,
                z: 0.866,
            },
            zero,
        ),
        (
            pz,
            Vec3 {
                x: 0.3,
                y: 0.4,
                z: -0.866,
            },
            zero,
        ),
        (
            Vec3 {
                x: 10.0,
                y: -5.0,
                z: 2.0,
            },
            Vec3 {
                x: 0.5,
                y: -0.5,
                z: 0.7072,
            },
            [100.0, 200.0, -50.0],
        ),
        (
            pz,
            Vec3 {
                x: 50.0,
                y: -30.0,
                z: 12.5,
            },
            zero,
        ),
        (
            pz,
            Vec3 {
                x: 1e-3,
                y: 1e-3,
                z: 1e-3,
            },
            zero,
        ),
        (
            pz,
            Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            [1.0, 2.0, 3.0],
        ),
    ];
    for (i, &(p, v, o)) in edges.iter().enumerate() {
        run(&mut emu, p, v, o, &format!("edge {i}"));
    }

    for case in 0..600 {
        let v = Vec3 {
            x: fz(&mut s),
            y: fz(&mut s),
            z: fz(&mut s),
        };
        let p = Vec3 {
            x: fz(&mut s),
            y: fz(&mut s),
            z: fz(&mut s),
        };
        let origin = [fz(&mut s), fz(&mut s), fz(&mut s)];
        run(&mut emu, p, v, origin, &format!("case {case}"));
    }
}

/// `0x6cf870` with `n` the integer at `this+0x1c`, over the horizontal length at the 1e-5 cutoff
/// and the float either side of it, directions straight up and down and horizontal (the `theta`
/// clamp at a quarter of π on both sides), a non-zero origin, `n` of 0 and negative, and a fuzz.
#[test]
fn sky_cloud_uv_scroll_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40);
    let vin_buf = emu.alloc(0x10);
    let out_buf = emu.alloc(0x10);
    let mut s = 0x636c_7576_u32;

    let run = |emu: &mut Emu, vin: Vec3, origin: [f32; 3], n: i32, ctx: &str| {
        emu.write_u32(this + 0x1c, n as u32);
        emu.write_u32(vin_buf, vin.x.to_bits());
        emu.write_u32(vin_buf + 4, vin.y.to_bits());
        emu.write_u32(vin_buf + 8, vin.z.to_bits());
        for (g, o) in ORIGIN_GLOBALS.iter().zip(origin.iter()) {
            emu.write_u32(*g, o.to_bits());
        }
        emu.write_u32(out_buf, 0xdead_beef);
        emu.write_u32(out_buf + 4, 0xdead_beef);
        emu.call(0x6cf870, this as u32, 0, &[vin_buf as u32, out_buf as u32]);
        let ours: CloudUv = sky_cloud_uv_scroll(vin, origin, n);
        assert_f32(emu.read_f32(out_buf), ours.u, &format!("{ctx} out.u"));
        assert_f32(emu.read_f32(out_buf + 4), ours.v, &format!("{ctx} out.v"));
    };

    let zero = [0.0f32; 3];
    let edges: [(Vec3, [f32; 3], i32); 12] = [
        (
            Vec3 {
                x: 1e-5,
                y: 0.0,
                z: 0.5,
            },
            zero,
            8,
        ),
        (
            Vec3 {
                x: 1.001e-5,
                y: 0.0,
                z: 0.5,
            },
            zero,
            8,
        ),
        (
            Vec3 {
                x: f32::from_bits(0x3727_c5ad),
                y: 0.0,
                z: 0.5,
            },
            zero,
            8,
        ),
        (
            Vec3 {
                x: 0.0,
                y: f32::from_bits(0x3727_c5ac),
                z: -0.5,
            },
            zero,
            8,
        ),
        (
            Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            zero,
            8,
        ),
        (
            Vec3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            zero,
            8,
        ),
        (
            Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            zero,
            8,
        ),
        (
            Vec3 {
                x: 0.5,
                y: 0.5,
                z: 0.7072,
            },
            zero,
            8,
        ),
        (
            Vec3 {
                x: 0.3,
                y: 0.4,
                z: 5.0,
            },
            zero,
            16,
        ),
        (
            Vec3 {
                x: 10.0,
                y: -10.0,
                z: 0.0,
            },
            [5.0, -5.0, 0.0],
            4,
        ),
        (
            Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            zero,
            0,
        ),
        (
            Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            zero,
            -8,
        ),
    ];
    for (i, &(vin, o, n)) in edges.iter().enumerate() {
        run(&mut emu, vin, o, n, &format!("edge {i}"));
    }

    for case in 0..600 {
        let vin = Vec3 {
            x: fz(&mut s),
            y: fz(&mut s),
            z: fz(&mut s),
        };
        let origin = if (next(&mut s) >> 8) & 1 == 0 {
            zero
        } else {
            [fz(&mut s), fz(&mut s), fz(&mut s)]
        };
        let n = match (next(&mut s) >> 8) % 4 {
            0 => 8,
            1 => 16,
            2 => (next(&mut s) >> 16) as i32 & 0xff,
            _ => -((next(&mut s) >> 16) as i32 & 0x3f),
        };
        run(&mut emu, vin, origin, n, &format!("case {case}"));
    }
}
