//! `Snow::SetIntensity` (`0x6776c0`), `Sand::SetIntensity` (`0x679380`) and `GetRainTailLength`
//! (`0x674bf0`) whole, against `snow_density_param_set`, `sand_density_param_set` and
//! `wind_speed_gate`; and the box the rain, snow and sand constructors (`0x674620`, `0x677420`,
//! `0x679140`) stamp, as a slice of each, against `weather_effect_halfbox`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::weather_scalars::{
    sand_density_param_set, snow_density_param_set, weather_effect_halfbox, wind_speed_gate,
    DENSITY_K_TABLE,
};
use common::{assert_f32, fuzz_f32, next};

/// The weather scale cell `OnWeatherDensityChanged` (`0x67b870`) writes. The test writes it and
/// passes the same K to the transcription, so every entry of `DENSITY_K_TABLE` runs.
const K_CELL: u64 = 0x8680ec;

/// `0x6776c0` whole, including `Mists::SetRate` (`0x67a980`). Both shader bytes, intensities on and
/// either side of 0.5 (the `x − 0.5` gate), both zeros, NaN and the infinities run among the fuzz;
/// the stored intensity is compared as bits.
#[test]
fn snow_density_param_set_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x100);
    let mut s = 0x736e_6f77_u32;

    let run = |emu: &mut Emu, shader_leg: bool, k: f32, intensity: f32, ctx: &str| {
        emu.write_u32(K_CELL, k.to_bits());
        emu.write_u32(this, u32::from(shader_leg)); // low byte is the `test al,al` key
        emu.write_u32(this + 0x3c, 0xdead_beef);
        emu.write_u32(this + 0x94, 0xdead_beef);
        emu.write_u32(this + 0xa0, 0xdead_beef);
        emu.write_u32(this + 0xa4, 0xdead_beef);
        emu.call(0x6776c0, this as u32, 0, &[intensity.to_bits()]);
        let ours = snow_density_param_set(shader_leg, k, intensity);
        assert_eq!(
            emu.read_u32(this + 0xa4),
            ours.intensity_raw,
            "{ctx} +0xa4 (intensity raw)"
        );
        assert_f32(
            emu.read_f32(this + 0xa0),
            ours.rate_a0,
            &format!("{ctx} +0xa0"),
        );
        assert_f32(
            emu.read_f32(this + 0x94),
            ours.rate_94,
            &format!("{ctx} +0x94"),
        );
        assert_f32(
            emu.read_f32(this + 0x3c),
            ours.ramp_arg,
            &format!("{ctx} +0x3c (ramp arg)"),
        );
    };

    let edges: [f32; 15] = [
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
            0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32,
            1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 0.5) as f32,
            2 => fuzz_f32(&mut s),
            _ => f32::from_bits(next(&mut s)),
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

/// `0x679380` whole, including `Mists::SetRate`. Both shader bytes and the same intensities run;
/// its mist rate has no gate at 0.5.
#[test]
fn sand_density_param_set_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x100);
    let mut s = 0x7361_6e64_u32;

    let run = |emu: &mut Emu, shader_leg: bool, k: f32, intensity: f32, ctx: &str| {
        emu.write_u32(K_CELL, k.to_bits());
        emu.write_u32(this, u32::from(shader_leg)); // low byte is the `test al,al` key
        emu.write_u32(this + 0x3c, 0xdead_beef);
        emu.write_u32(this + 0x94, 0xdead_beef);
        emu.write_u32(this + 0x9c, 0xdead_beef);
        emu.call(0x679380, this as u32, 0, &[intensity.to_bits()]);
        let ours = sand_density_param_set(shader_leg, k, intensity);
        assert_f32(
            emu.read_f32(this + 0x94),
            ours.rate_94,
            &format!("{ctx} +0x94"),
        );
        assert_f32(
            emu.read_f32(this + 0x9c),
            ours.rate_9c,
            &format!("{ctx} +0x9c"),
        );
        assert_f32(
            emu.read_f32(this + 0x3c),
            ours.ramp_arg,
            &format!("{ctx} +0x3c (ramp arg)"),
        );
    };

    let edges: [f32; 15] = [
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
            0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32,
            1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 0.5) as f32,
            2 => fuzz_f32(&mut s),
            _ => f32::from_bits(next(&mut s)),
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

/// `0x674bf0` whole: the wind vector is written at `+0x50` of the object `[0xc6326c]` points to.
/// Squared speeds on, a step either side of and far from 0.001, a zero, NaN and the infinities run;
/// the result is 2.0 below the threshold and 0 at it, above it and for NaN.
#[test]
fn wind_speed_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    const MGR_GLOBAL: u64 = 0xc6326c;
    let mgr = emu.alloc(0x90);
    emu.write_u32(MGR_GLOBAL, mgr as u32);
    let mut s = 0x7769_6e64_u32;

    let run = |emu: &mut Emu, x: f32, y: f32, z: f32, ctx: &str| {
        emu.write_u32(mgr + 0x50, x.to_bits());
        emu.write_u32(mgr + 0x54, y.to_bits());
        emu.write_u32(mgr + 0x58, z.to_bits());
        let binary = emu.call_f32(0x674bf0, 0, 0, &[]);
        let ours = wind_speed_gate(x, y, z);
        assert_f32(binary, ours, ctx);
    };

    let edges: [(f32, f32, f32); 12] = [
        (0.0, 0.0, 0.0),
        (-0.0, -0.0, -0.0),
        (0.0316, 0.0, 0.0),
        (0.0317, 0.0, 0.0),
        (1.0, 0.0, 0.0),
        (0.02, 0.02, 0.02),
        (-0.01, 0.01, 0.01),
        (f32::NAN, 1.0, 0.0),
        (0.0, f32::NAN, 0.0),
        (0.0, 0.0, f32::NAN),
        (f32::INFINITY, 0.0, 0.0),
        (0.0, f32::NEG_INFINITY, 0.0),
    ];
    for (i, &(x, y, z)) in edges.iter().enumerate() {
        run(&mut emu, x, y, z, &format!("edge {i} ({x},{y},{z})"));
    }

    // The squared speed of one component steps through the f32 neighbours of the square root of
    // 0.001, on each axis, so the compare at the threshold sees values on both sides of it.
    let root = 0.001f32.sqrt().to_bits();
    for step in 0..=8u32 {
        let c = f32::from_bits(root - 4 + step);
        run(&mut emu, c, 0.0, 0.0, &format!("threshold x {step}"));
        run(&mut emu, 0.0, c, 0.0, &format!("threshold y {step}"));
        run(&mut emu, 0.0, 0.0, -c, &format!("threshold z {step}"));
    }

    for case in 0..600 {
        let (x, y, z) = match (next(&mut s) >> 8) % 4 {
            // Components within 0.05, so the square straddles 0.001 densely.
            0 => {
                let c = |s: &mut u32| {
                    (f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 0.1 - 0.05) as f32
                };
                (c(&mut s), c(&mut s), c(&mut s))
            }
            1 => (fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)),
            2 => (
                fuzz_f32(&mut s) * 50.0,
                fuzz_f32(&mut s) * 50.0,
                fuzz_f32(&mut s) * 50.0,
            ),
            _ => (
                f32::from_bits(next(&mut s)),
                f32::from_bits(next(&mut s)),
                f32::from_bits(next(&mut s)),
            ),
        };
        run(&mut emu, x, y, z, &format!("case {case} ({x},{y},{z})"));
    }
}

/// Runs `[begin, until)` of a constructor on a frame holding the two arguments at `[ebp+8]` and
/// `[ebp+0xc]`, with ESI, EBX and EDI set to cover the base register of all three constructors and
/// room on the stack for the push inside one slice; the six outputs are poisoned and compared.
#[allow(clippy::too_many_arguments)]
fn diff_halfbox_slice(
    emu: &mut Emu,
    begin: u64,
    until: u64,
    this: u64,
    frame: u64,
    stack: u64,
    arg0: f32,
    arg1: f32,
    ctx: &str,
) {
    let ebp = frame + 0x40;
    emu.write_u32(ebp + 8, arg0.to_bits());
    emu.write_u32(ebp + 0xc, arg1.to_bits());
    for off in [0x4u64, 0x8, 0xc, 0x10, 0x14, 0x18] {
        emu.write_u32(this + off, 0xdead_beef);
    }
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESP, stack + 0x80); // room below for the in-slice `push`
    emu.set_reg(RegisterX86::ESI, this);
    emu.set_reg(RegisterX86::EBX, this + 0x4);
    emu.set_reg(RegisterX86::EDI, this + 0x4);
    emu.run(begin, until);

    let ours = weather_effect_halfbox(arg0, arg1);
    for (i, &off) in [0x4u64, 0x8, 0xc, 0x10, 0x14, 0x18].iter().enumerate() {
        assert_f32(
            emu.read_f32(this + off),
            ours[i],
            &format!("{ctx} box[{i}] (+{off:#x})"),
        );
    }
}

/// Runs one constructor's slice over edge arguments and a fuzz.
fn run_halfbox_case(addr_label: &str, begin: u64, until: u64) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40);
    let frame = emu.alloc(0x80);
    let stack = emu.alloc(0x100);
    let mut s = 0x6862_6f78_u32 ^ (begin as u32);

    let edges: [(f32, f32); 8] = [
        (0.0, 0.0),
        (1.0, 2.0),
        (-1.0, -2.0),
        (3.5, 0.25),
        (-0.0, 100.0),
        (f32::INFINITY, 1.0),
        (f32::NEG_INFINITY, 0.5),
        (f32::NAN, -3.0),
    ];
    for (i, &(a, b)) in edges.iter().enumerate() {
        diff_halfbox_slice(
            &mut emu,
            begin,
            until,
            this,
            frame,
            stack,
            a,
            b,
            &format!("{addr_label} edge {i} ({a},{b})"),
        );
    }
    for case in 0..400 {
        let a = fuzz_f32(&mut s);
        let b = fuzz_f32(&mut s);
        diff_halfbox_slice(
            &mut emu,
            begin,
            until,
            this,
            frame,
            stack,
            a,
            b,
            &format!("{addr_label} case {case} ({a},{b})"),
        );
    }
}

/// The rain constructor from `fld [ebp+8]` at `0x6746fa` to the push at `0x674757`.
#[test]
fn weather_effect_ctor_674620_difftest() {
    run_halfbox_case("0x674620", 0x6746fa, 0x674757);
}

/// The snow constructor from `0x6774ce` to the push at `0x677528`.
#[test]
fn weather_effect_helper_677420_difftest() {
    run_halfbox_case("0x677420", 0x6774ce, 0x677528);
}

/// The sand constructor from `0x6791ef` to `mov edx,ebx` at `0x67924e`; the slice holds a push that
/// the scratch stack absorbs.
#[test]
fn weather_effect_helper_679140_difftest() {
    run_halfbox_case("0x679140", 0x6791ef, 0x67924e);
}
