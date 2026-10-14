//! The six leaves of `sky_leaves` against the exe: the fog and cloud colour modifiers, the lerp,
//! the third, the storm weight and the cloud cover byte. Each runs the function's own instructions
//! in the emulator on edge values and a fuzz and compares bit for bit.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::sky_leaves::{
    cloud_color_modulate, cloud_density_clamp, fog_color_modulate, sky_ftol_field_setter,
    sky_scalar_lerp, sky_scale_third, FogColorModulate,
};
use common::{assert_f32, fuzz_f32, next};
const G_FOG_SCALE: u64 = 0xce99d0;
const G_FOG_COLOR: u64 = 0xce99f0;
const G_CLOUD_SCALE: u64 = 0xce99d4;
const G_DENSITY_IN: u64 = 0xce9ba0;
const G_DENSITY_OUT: u64 = 0xce9bcc;

/// A fog or cloud scale fuzzed mostly into [-0.1, 1.1], which straddles the blend 0 and 0xff
/// transitions, and occasionally wider.
fn fuzz_scale(s: &mut u32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match (next(s) >> 8) % 4 {
        0 | 1 => (m * 1.2 - 0.1) as f32,
        2 => (m * 5.0 - 2.0) as f32,
        _ => (m * 2.0) as f32,
    }
}

/// A packed colour: random, grey, and saturated.
fn fuzz_color(s: &mut u32) -> u32 {
    match (next(s) >> 8) % 5 {
        0 => 0xff00_0000,
        1 => 0xffff_ffff,
        2 => {
            let g = (next(s) >> 8) & 0xff;
            0xff00_0000 | (g << 16) | (g << 8) | g
        }
        3 => 0xffff_0000 | (next(s) & 0xffff),
        _ => next(s),
    }
}

#[test]
fn fog_color_modulate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec = emu.alloc(0x10);
    let mut s = 0x666f_675f_u32;

    let run = |emu: &mut Emu, color: u32, f4: f32, f8: f32, scale: f32, gcol: u32| {
        emu.write_u32(rec, color);
        emu.write_u32(rec + 4, f4.to_bits());
        emu.write_u32(rec + 8, f8.to_bits());
        emu.write_u32(G_FOG_SCALE, scale.to_bits());
        emu.write_u32(G_FOG_COLOR, gcol);
        emu.call(0x6cf1a0, rec as u32, 0, &[]);
        let ours = fog_color_modulate(FogColorModulate { color, f4, f8 }, scale, gcol);
        let ctx = format!("color={color:#010x} scale={scale} gcol={gcol:#010x}");
        assert_eq!(emu.read_u32(rec), ours.color, "fog color {ctx}");
        assert_f32(emu.read_f32(rec + 4), ours.f4, &format!("fog f4 {ctx}"));
        assert_f32(emu.read_f32(rec + 8), ours.f8, &format!("fog f8 {ctx}"));
    };
    let scale_edges = [
        0.0f32,
        1.0,
        0.5,
        0.25,
        0.75,
        -0.05,
        1.05,
        0.001,
        0.999,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &scale in &scale_edges {
        run(&mut emu, 0xff20_4080, 1.5, -3.0, scale, 0xff10_c0a0);
        run(&mut emu, 0xffff_ffff, 0.0, 0.0, scale, 0xff00_0000);
    }
    for _ in 0..700 {
        let color = fuzz_color(&mut s);
        let f4 = fuzz_f32(&mut s);
        let f8 = fuzz_f32(&mut s);
        let scale = fuzz_scale(&mut s);
        let gcol = fuzz_color(&mut s);
        run(&mut emu, color, f4, f8, scale, gcol);
    }
}

#[test]
fn cloud_color_modulate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec = emu.alloc(4);
    let mut s = 0x636c_6f75_u32;

    let run = |emu: &mut Emu, color: u32, scale: f32| {
        emu.write_u32(rec, color);
        emu.write_u32(G_CLOUD_SCALE, scale.to_bits());
        emu.call(0x6cf290, rec as u32, 0, &[]);
        let ours = cloud_color_modulate(color, scale);
        assert_eq!(
            emu.read_u32(rec),
            ours,
            "cloud color={color:#010x} scale={scale}"
        );
    };

    let scale_edges = [
        0.0f32,
        1.0,
        0.5,
        0.25,
        0.75,
        -0.05,
        1.05,
        0.001,
        0.999,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &scale in &scale_edges {
        run(&mut emu, 0xff20_4080, scale);
        run(&mut emu, 0xffff_ffff, scale);
        run(&mut emu, 0xff00_0000, scale);
    }
    for _ in 0..700 {
        let color = fuzz_color(&mut s);
        let scale = fuzz_scale(&mut s);
        run(&mut emu, color, scale);
    }
}

#[test]
fn sky_scalar_lerp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x6c65_7270_u32;

    let edges: [(f32, f32, f32); 9] = [
        (1.0, 5.0, 0.5),
        (5.0, 1.0, 0.5),
        (3.0, 3.0, 0.5),
        (1.0, 5.0, 0.0),
        (1.0, 5.0, 1.0),
        (1.0, 5.0, -0.5),
        (1.0, 5.0, 2.0),
        (f32::NAN, 5.0, 0.5),
        (1.0, f32::NAN, 0.5),
    ];
    for (i, &(a, b, t)) in edges.iter().enumerate() {
        let binary = emu.call_f32(0x6d15f0, 0, 0, &[a.to_bits(), b.to_bits(), t.to_bits()]);
        assert_f32(
            binary,
            sky_scalar_lerp(a, b, t),
            &format!("edge {i} a={a} b={b} t={t}"),
        );
    }
    for case in 0..600 {
        let a = fuzz_f32(&mut s);
        let b = fuzz_f32(&mut s);
        let t = match (next(&mut s) >> 8) % 3 {
            0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32,
            1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 3.0 - 1.0) as f32,
            _ => fuzz_f32(&mut s),
        };
        let binary = emu.call_f32(0x6d15f0, 0, 0, &[a.to_bits(), b.to_bits(), t.to_bits()]);
        assert_f32(
            binary,
            sky_scalar_lerp(a, b, t),
            &format!("case {case} a={a} b={b} t={t}"),
        );
    }
}

#[test]
fn sky_scale_third_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x7468_7264_u32;
    let edges = [
        0.0f32,
        -0.0,
        1.0,
        3.0,
        -3.0,
        90.0,
        180.0,
        360.0,
        f32::NAN,
        f32::INFINITY,
    ];
    for (i, &x) in edges.iter().enumerate() {
        let binary = emu.call_f32(0x6d6e50, 0, 0, &[x.to_bits()]);
        assert_f32(binary, sky_scale_third(x), &format!("edge {i} x={x}"));
    }
    for case in 0..400 {
        let x = fuzz_f32(&mut s);
        let binary = emu.call_f32(0x6d6e50, 0, 0, &[x.to_bits()]);
        assert_f32(binary, sky_scale_third(x), &format!("case {case} x={x}"));
    }
}

#[test]
fn cloud_density_clamp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x6473_7479_u32;

    let run = |emu: &mut Emu, density: f32| {
        emu.write_u32(G_DENSITY_IN, density.to_bits());
        emu.write_u32(G_DENSITY_OUT, 0xdead_beef);
        emu.call(0x6d4500, 0, 0, &[]);
        assert_f32(
            emu.read_f32(G_DENSITY_OUT),
            cloud_density_clamp(density),
            &format!("density={density}"),
        );
    };
    let edges = [
        0.0f32,
        0.1,
        0.24,
        0.25,
        0.26,
        0.5,
        1.0,
        -0.5,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &d in &edges {
        run(&mut emu, d);
    }
    for _ in 0..400 {
        let d = match (next(&mut s) >> 8) % 3 {
            0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 0.6) as f32,
            1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 1.0 - 0.5) as f32,
            _ => fuzz_f32(&mut s),
        };
        run(&mut emu, d);
    }
}

#[test]
fn sky_ftol_field_setter_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec = emu.alloc(0x10);
    let mut s = 0x6674_6f6c_u32;

    let run = |emu: &mut Emu, field: f32, arg: f32| {
        emu.write_u32(rec + 0xc, field.to_bits());
        emu.write_u32(rec + 8, 0);
        emu.call(0x6d0970, rec as u32, 0, &[arg.to_bits()]);
        let binary = (emu.read_u32(rec + 8) & 0xff) as u8;
        assert_eq!(
            binary,
            sky_ftol_field_setter(field, arg),
            "field={field} arg={arg}"
        );
    };

    let field_edges = [0.0f32, -0.0, 0.5, -0.5, 1.0, 2.0, f32::NAN, 100.0];
    let arg_edges = [0.0f32, 0.5, -0.5, 1.0, 2.0, -3.0, 1.0e30, -1.0e30, f32::NAN];
    for &field in &field_edges {
        for &arg in &arg_edges {
            run(&mut emu, field, arg);
        }
    }
    for _ in 0..500 {
        let field = match (next(&mut s) >> 8) % 3 {
            0 => 0.0,
            1 => -0.0,
            _ => {
                let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
                (m * 100.0 - 50.0) as f32
            }
        };
        let arg = match (next(&mut s) >> 8) % 2 {
            0 => {
                let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
                (m * 100.0 - 50.0) as f32
            }
            _ => fuzz_f32(&mut s),
        };
        run(&mut emu, field, arg);
    }
}
