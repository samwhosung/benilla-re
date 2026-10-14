//! `Rain::SetIntensity` (`0x6749e0`) and `SetPatter` (`0x675280`) whole, against
//! `rain_density_param_set` and `patter_record_init`: a record is poisoned, the function runs, and
//! every field it writes is read back and compared bit for bit.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::weather_init::{patter_record_init, rain_density_param_set};
use benilla_re_lighting::weather_scalars::DENSITY_K_TABLE;
use common::{assert_f32, fuzz_f32, next};

/// The weather scale cell `OnWeatherDensityChanged` (`0x67b870`) writes, 0.1, 0.33, 0.66 or 1.0 for
/// `weatherDensity` 0 to 3. The test writes the cell and passes the same K, so every entry runs.
const K_CELL: u64 = 0x8680ec;

/// `0x6749e0` whole, including `Mists::SetRate` (`0x67a980`), which stores the mist rate at
/// `+0x3c`. Both values of the shader byte, intensities on and either side of 0.5 (the `x − 0.5`
/// gate), both zeros, NaN (quiet and signalling) and the infinities run among the fuzz.
#[test]
fn rain_density_param_set_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x100);
    let mut s = 0x7261_696e_u32;

    let run = |emu: &mut Emu, shader_leg: bool, k: f32, intensity: f32, ctx: &str| {
        emu.write_u32(K_CELL, k.to_bits());
        // Byte 0 is the shader key; the four outputs are poisoned.
        emu.write_u32(this, u32::from(shader_leg));
        emu.write_u32(this + 0x3c, 0xdead_beef);
        emu.write_u32(this + 0xb4, 0xdead_beef);
        emu.write_u32(this + 0xcc, 0xdead_beef);
        emu.write_u32(this + 0xd0, 0xdead_beef);
        emu.call(0x6749e0, this as u32, 0, &[intensity.to_bits()]);
        let ours = rain_density_param_set(shader_leg, k, intensity);
        assert_eq!(
            emu.read_u32(this + 0xd0),
            ours.intensity_raw,
            "{ctx} +0xd0 (intensity raw)"
        );
        assert_f32(
            emu.read_f32(this + 0xcc),
            ours.rate_cc,
            &format!("{ctx} +0xcc"),
        );
        assert_f32(
            emu.read_f32(this + 0xb4),
            ours.rate_b4,
            &format!("{ctx} +0xb4"),
        );
        assert_f32(
            emu.read_f32(this + 0x3c),
            ours.ramp_arg,
            &format!("{ctx} +0x3c (ramp arg)"),
        );
    };

    let edges: [f32; 17] = [
        0.0,
        0.5,
        1.0,
        -1.0,
        0.25,
        0.75,
        100.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -0.0,
        f32::from_bits(0x3eff_ffff),
        f32::from_bits(0x3f00_0001),
        f32::NAN,
        f32::from_bits(0x7fa0_0000),
        f32::from_bits(0xffc0_0000),
        f32::MIN_POSITIVE / 2.0,
        -0.5,
    ];
    for (i, &x) in edges.iter().enumerate() {
        run(
            &mut emu,
            false,
            DENSITY_K_TABLE[i % 4],
            x,
            &format!("edge-ff {i} x={x}"),
        );
        run(
            &mut emu,
            true,
            DENSITY_K_TABLE[i % 4],
            x,
            &format!("edge-shader {i} x={x}"),
        );
    }

    for case in 0..600 {
        let shader_leg = (next(&mut s) >> 8) & 1 == 1;
        let k = DENSITY_K_TABLE[((next(&mut s) >> 8) % 4) as usize];
        let intensity = match (next(&mut s) >> 8) % 4 {
            0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32, // [0,1)
            1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 0.5) as f32, // [-0.5,1.5)
            2 => fuzz_f32(&mut s),
            _ => f32::from_bits(next(&mut s)), // anything incl. NaN/inf
        };
        run(
            &mut emu,
            shader_leg,
            k,
            intensity,
            &format!("case {case} shader_leg={shader_leg} x={intensity}"),
        );
    }
}

/// `0x675280` whole. The point is copied as integers, so its bits are arbitrary; the time runs
/// through the x87, so both zeros, a subnormal, the infinities and a quiet NaN of each sign run
/// among the fuzz.
#[test]
fn patter_record_init_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let dst = emu.alloc(0x20);
    let src = emu.alloc(0x10);
    let mut s = 0x7061_7474_u32;

    let run = |emu: &mut Emu, val: f32, pos: [u32; 3], ctx: &str| {
        emu.write_u32(src, pos[0]);
        emu.write_u32(src + 4, pos[1]);
        emu.write_u32(src + 8, pos[2]);
        // The record is poisoned, including `+0xc`, which the function leaves alone.
        for off in [0u64, 4, 8, 0xc, 0x10, 0x14] {
            emu.write_u32(dst + off, 0xdead_beef);
        }
        emu.call(0x675280, 0, 0, &[dst as u32, val.to_bits(), src as u32]);
        let ours = patter_record_init(val, pos);
        assert_eq!(emu.read_u32(dst), ours.pos[0], "{ctx} dst+0");
        assert_eq!(emu.read_u32(dst + 4), ours.pos[1], "{ctx} dst+4");
        assert_eq!(emu.read_u32(dst + 8), ours.pos[2], "{ctx} dst+8");
        assert_eq!(
            emu.read_u32(dst + 0x10),
            ours.val_raw,
            "{ctx} dst+0x10 (val raw)"
        );
        assert_f32(
            emu.read_f32(dst + 0x14),
            ours.lifetime,
            &format!("{ctx} dst+0x14 (lifetime)"),
        );
    };

    let edges: [f32; 10] = [
        0.0,
        -0.0,
        0.25,
        -0.25,
        1.0,
        f32::MIN_POSITIVE / 2.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xffc0_0000),
    ];
    for (i, &v) in edges.iter().enumerate() {
        run(
            &mut emu,
            v,
            [next(&mut s), next(&mut s), next(&mut s)],
            &format!("edge {i} val={v}"),
        );
    }

    for case in 0..600 {
        let val = fuzz_f32(&mut s);
        let pos = [next(&mut s), next(&mut s), next(&mut s)];
        run(&mut emu, val, pos, &format!("case {case} val={val}"));
    }
}
