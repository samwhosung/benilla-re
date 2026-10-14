//! The arithmetic of `0x674c50` against the exe: each stretch of the function between two addresses
//! runs as the exe's own instructions on a frame laid out in the emulator, and the stored values
//! are compared with the transcription bit for bit. The sine and cosine (`fsincos`, `fcos`, `fsin`)
//! are compared within one unit in the last place, since the host's functions stand in for the
//! x87's; the angle that feeds them and every other value is bit-exact.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::wx_rainspawn::{
    rainspawn_angle, rainspawn_cam_global, rainspawn_cell_delta, rainspawn_collide_pos,
    rainspawn_collide_scale, rainspawn_collide_wind, rainspawn_final_delta, rainspawn_hit_delta,
    rainspawn_inv_velmag, rainspawn_lifetime, rainspawn_pos_offset, rainspawn_pos_seed,
    rainspawn_residual, rainspawn_sincos, rainspawn_sky_time, rainspawn_vel_mag, rainspawn_vel_xy,
    rainspawn_vel_z, rainspawn_velmag, rainspawn_wind_apply, rainspawn_wind_delta,
    rainspawn_wind_factor, rainspawn_wind_factor_live, rainspawn_wind_scale2,
    rainspawn_world_scale, rng_unit_float,
};
use common::next;

/// Compare two f32 bit for bit, any two NaN equal.
fn assert_field(binary: f32, ours: f32, tag: &str, field: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{tag}.{field}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// Whether two values are within one unit in the last place; two NaN are.
fn within_1ulp(a: f32, b: f32) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    let to_ord = |x: f32| -> i64 {
        let bits = x.to_bits() as i32;
        if bits < 0 {
            i64::from(i32::MIN) - i64::from(bits)
        } else {
            i64::from(bits)
        }
    };
    (to_ord(a) - to_ord(b)).abs() <= 1
}

/// A fuzzed f32: random bits, and finite values of several scales and signs.
fn fuzz(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let bits = next(s);
    match pick {
        0 => f32::from_bits(bits),
        1 => (f64::from(bits >> 4) / f64::from(1u32 << 20) - 128.0) as f32,
        2 => (f64::from(bits >> 8) / f64::from(1u32 << 23) * 2000.0 - 1000.0) as f32,
        3 => (f64::from(bits >> 8) / f64::from(1u32 << 23) * 2.0 - 1.0) as f32,
        4 => f32::from_bits(0x3f80_0000 | (bits >> 9)),
        _ => -f32::from_bits(0x3f00_0000 | (bits >> 9)),
    }
}

/// A fuzzed draw: a float in [1, 2).
fn fuzz_rng(s: &mut u32) -> f32 {
    rng_unit_float(next(s))
}

/// A fuzzed f32 within `range`.
fn fuzz_bounded(s: &mut u32, range: f32) -> f32 {
    let u = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    ((u * 2.0 - 1.0) * f64::from(range)) as f32
}

const G_C6326C: u64 = 0xc6326c;
const G_C7CF20: u64 = 0xc7cf20;
const G_C7CF24: u64 = 0xc7cf24;
const G_C7CF28: u64 = 0xc7cf28;

/// `0x674c5c` to `0x674c7d`: the three cell spans.
#[test]
fn rainspawn_cell_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7261_696e_u32;

    let run = |emu: &mut Emu, hi: [f32; 3], lo: [f32; 3], tag: &str| {
        emu.write_u32(obj + 0x4, lo[0].to_bits());
        emu.write_u32(obj + 0x8, lo[1].to_bits());
        emu.write_u32(obj + 0xc, lo[2].to_bits());
        emu.write_u32(obj + 0x10, hi[0].to_bits());
        emu.write_u32(obj + 0x14, hi[1].to_bits());
        emu.write_u32(obj + 0x18, hi[2].to_bits());
        emu.write_u32(frame - 0x10, 0xdead_beef);
        emu.write_u32(frame - 0xc, 0xdead_beef);
        emu.write_u32(frame - 0x8, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x674c5c, 0x674c7d);
        for (i, off) in [0x10u64, 0xc, 0x8].into_iter().enumerate() {
            let binary = emu.read_f32(frame - off);
            assert_field(binary, rainspawn_cell_delta(hi[i], lo[i]), tag, "delta");
        }
    };

    run(&mut emu, [1.0, 2.0, 3.0], [0.0, 0.0, 0.0], "basic");
    run(&mut emu, [0.0; 3], [0.0; 3], "zero");
    run(
        &mut emu,
        [f32::NAN, f32::INFINITY, 1.0],
        [0.0, f32::INFINITY, 1.0],
        "nan-inf",
    );
    run(
        &mut emu,
        [0.333_333_34, 0.1, 1.0e20],
        [0.1, 0.333_333_34, -1.0e20],
        "subf32",
    );
    for _ in 0..300 {
        let hi = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let lo = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, hi, lo, "fuzz");
    }
    run(&mut emu, [1.5; 3], [1.5; 3], "eq");
    let one_ulp_up = f32::from_bits(1.0_f32.to_bits() + 1);
    run(&mut emu, [one_ulp_up; 3], [1.0; 3], "1ulp");
}

/// `0x674ca6` to `0x674ce8`: the position seeds from two draws.
#[test]
fn rainspawn_pos_seed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x706f_7372_u32;

    #[allow(clippy::too_many_arguments)]
    let run = |emu: &mut Emu, rng1: f32, rng2: f32, dx: f32, dy: f32, dz: f32, tag: &str| {
        emu.write_u32(frame - 0x4, rng2.to_bits());
        emu.write_u32(frame - 0x24, rng1.to_bits());
        emu.write_u32(frame - 0x10, dx.to_bits());
        emu.write_u32(frame - 0xc, dy.to_bits());
        emu.write_u32(frame - 0x8, dz.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x674ca6, 0x674ce8);
        let bx = emu.read_f32(sp);
        let by = emu.read_f32(sp + 4);
        let bz = emu.read_f32(sp + 8);
        let ours = rainspawn_pos_seed(rng1, rng2, dx, dy, dz);
        assert_field(bx, ours.out_x, tag, "out_x");
        assert_field(by, ours.out_y, tag, "out_y");
        assert_field(bz, ours.out_z, tag, "out_z");
    };

    run(&mut emu, 1.0, 1.0, 1.0, 1.0, 1.0, "unit");
    run(&mut emu, 1.5, 1.5, 2.0, 3.0, 4.0, "mid");
    run(&mut emu, 1.0, 2.0, 0.0, 0.0, 0.0, "zero-d");
    run(
        &mut emu,
        rng_unit_float(0x1234_5678),
        rng_unit_float(0x7777_7777),
        0.333_333_34,
        0.1,
        100.0,
        "subf32",
    );
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz_rng(&mut s),
            fuzz_rng(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            "fuzz",
        );
    }
}

/// `0x674d04` to `0x674d4b`: the angle bit-exact and its cosine and sine within one ulp.
#[test]
fn rainspawn_sincos_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7273_6e63_u32;
    let mut ulp_red = 0u32;

    let run = |emu: &mut Emu, rng: f32, w: f32, tag: &str, red: &mut u32| {
        emu.write_u32(frame - 0x24, rng.to_bits());
        emu.write_u32(obj + 0xd0, w.to_bits());
        emu.write_u32(frame - 0x40, 0xdead_beef);
        emu.write_u32(frame - 0x44, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x674d04, 0x674d4b);
        let bcos = emu.read_f32(frame - 0x40);
        let bsin = emu.read_f32(frame - 0x44);
        let bangle = emu.read_f32(frame - 0x24);
        let ours = rainspawn_sincos(rng, w);
        assert_field(bangle, ours.angle, tag, "angle");
        assert!(
            within_1ulp(bsin, ours.sin),
            "{tag}.sin beyond 1ulp: binary={:#010x} ours={:#010x}",
            bsin.to_bits(),
            ours.sin.to_bits()
        );
        assert!(
            within_1ulp(bcos, ours.cos),
            "{tag}.cos beyond 1ulp: binary={:#010x} ours={:#010x}",
            bcos.to_bits(),
            ours.cos.to_bits()
        );
        if bsin.to_bits() != ours.sin.to_bits() || bcos.to_bits() != ours.cos.to_bits() {
            *red += 1;
        }
    };

    run(&mut emu, 1.0, 1.0, "rng1", &mut ulp_red);
    run(&mut emu, 1.5, 0.5, "mid", &mut ulp_red);
    run(&mut emu, rng_unit_float(0), 2.0, "rng-lo", &mut ulp_red);
    run(
        &mut emu,
        rng_unit_float(0x7fff_ffff),
        10.0,
        "rng-hi",
        &mut ulp_red,
    );
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz_rng(&mut s),
            fuzz_bounded(&mut s, 100.0),
            "fuzz",
            &mut ulp_red,
        );
    }
}

/// `0x674d04` to `0x674d3c`: the angle alone, bit-exact.
#[test]
fn rainspawn_angle_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7261_6e67_u32;

    let run = |emu: &mut Emu, rng: f32, w: f32, tag: &str| {
        emu.write_u32(frame - 0x24, rng.to_bits());
        emu.write_u32(obj + 0xd0, w.to_bits());
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x674d04, 0x674d3c);
        let binary = emu.read_f32(frame - 0x24);
        assert_field(binary, rainspawn_angle(rng, w), tag, "angle");
    };

    run(&mut emu, 1.0, 1.0, "rng1");
    run(&mut emu, 1.5, 0.5, "mid");
    run(&mut emu, rng_unit_float(0x7fff_ffff), 3.0, "hi");
    for _ in 0..300 {
        run(&mut emu, fuzz_rng(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x674d62` to `0x674d8d`: the speed.
#[test]
fn rainspawn_vel_mag_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7276_6d67_u32;

    let run = |emu: &mut Emu, rng: f32, w: f32, tag: &str| {
        emu.write_u32(frame - 0x20, rng.to_bits());
        emu.write_u32(obj + 0xd0, w.to_bits());
        emu.write_u32(frame - 0x4, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x674d62, 0x674d8d);
        let binary = emu.read_f32(frame - 0x4);
        assert_field(binary, rainspawn_vel_mag(rng, w), tag, "mag");
    };

    run(&mut emu, 1.0, 1.0, "rng1");
    run(&mut emu, 1.5, 0.5, "mid");
    run(&mut emu, rng_unit_float(0), 2.0, "lo");
    run(&mut emu, rng_unit_float(0x7fff_ffff), 3.0, "hi");
    for _ in 0..300 {
        run(&mut emu, fuzz_rng(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x674d9c` to `0x674db7`: the horizontal velocity.
#[test]
fn rainspawn_vel_xy_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7278_795f_u32;

    let run = |emu: &mut Emu, mag: f32, sin: f32, cos: f32, tag: &str| {
        emu.write_u32(frame - 0x4, mag.to_bits());
        emu.write_u32(frame - 0x44, sin.to_bits());
        emu.write_u32(frame - 0x40, cos.to_bits());
        emu.write_u32(frame - 0x1c, 0xdead_beef);
        emu.write_u32(frame - 0x18, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x674d9c, 0x674db7);
        let bvx = emu.read_f32(frame - 0x1c);
        let bvy = emu.read_f32(frame - 0x18);
        let ours = rainspawn_vel_xy(mag, sin, cos);
        assert_field(bvx, ours.vx, tag, "vx");
        assert_field(bvy, ours.vy, tag, "vy");
    };

    run(&mut emu, 18.0, 0.0, 1.0, "xonly");
    run(&mut emu, 18.666, 0.866_025_4, 0.5, "30deg");
    run(&mut emu, 0.0, 0.5, 0.5, "zeromag");
    run(&mut emu, f32::NAN, 1.0, 1.0, "nan");
    for _ in 0..300 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x674db7` to `0x674df6`: the vertical velocity.
#[test]
fn rainspawn_vel_z_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let arg = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7276_7a5f_u32;

    let run = |emu: &mut Emu, rng: f32, w: f32, tag: &str| {
        emu.write_u32(frame - 0x20, rng.to_bits());
        emu.write_u32(obj + 0xd0, w.to_bits());
        emu.write_u32(frame + 8, arg as u32);
        emu.write_u32(frame - 0x14, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x674db7, 0x674df6);
        let binary = emu.read_f32(frame - 0x14);
        assert_field(binary, rainspawn_vel_z(rng, w), tag, "vz");
    };

    run(&mut emu, 1.0, 1.0, "rng1");
    run(&mut emu, 1.5, 0.5, "mid");
    run(&mut emu, rng_unit_float(0), 2.0, "lo");
    run(&mut emu, rng_unit_float(0x7fff_ffff), 3.0, "hi");
    for _ in 0..300 {
        run(&mut emu, fuzz_rng(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x674df6` to `0x674e0d`: the tilt factor, as stored and as left wide.
#[test]
fn rainspawn_wind_factor_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x40);
    let scratch = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7277_665f_u32;

    let run = |emu: &mut Emu, dz_cell: f32, span: f32, tag: &str| {
        emu.write_u32(frame - 0x8, dz_cell.to_bits());
        emu.write_u32(obj + 0x14, span.to_bits());
        emu.write_u32(frame + 8, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EDI, scratch);
        emu.set_reg(RegisterX86::EDX, scratch);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x674df6, 0x674e0d);
        let binary = emu.read_f32(frame + 8);
        assert_field(binary, rainspawn_wind_factor(dz_cell, span), tag, "factor");
    };

    run(&mut emu, 4.0, 2.0, "basic");
    run(&mut emu, 0.0, 1.0, "zero");
    run(&mut emu, 1.0, 0.0, "div0");
    run(&mut emu, f32::NAN, 1.0, "nan");
    run(&mut emu, 0.333_333_34, 0.7, "subf32");
    for _ in 0..300 {
        run(&mut emu, fuzz(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x674df6` to `0x674e26`: the wind products, the first from the wide factor and the other two
/// from the stored one.
#[test]
fn rainspawn_wind_apply_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x40);
    let edi = emu.alloc(0x40);
    let edx = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7277_615f_u32;

    let run = |emu: &mut Emu, dz_cell: f32, span: f32, basis: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x8, dz_cell.to_bits());
        emu.write_u32(obj + 0x14, span.to_bits());
        emu.write_u32(edi, basis[0].to_bits());
        emu.write_u32(edi + 4, basis[1].to_bits());
        emu.write_u32(edi + 8, basis[2].to_bits());
        emu.write_u32(frame + 8, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EDX, edx);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.write_u32(espbase, 0xdead_beef);
        emu.write_u32(espbase + 4, 0xdead_beef);
        emu.write_u32(espbase + 8, 0xdead_beef);
        emu.run(0x674df6, 0x674e26);
        let factor_f32 = rainspawn_wind_factor(dz_cell, span);
        let factor_live = rainspawn_wind_factor_live(dz_cell, span);
        let ours = rainspawn_wind_apply(factor_live, factor_f32, basis);
        assert_field(emu.read_f32(espbase + 8), ours.out2, tag, "out2");
        assert_field(emu.read_f32(espbase + 4), ours.out1, tag, "out1");
        assert_field(emu.read_f32(espbase), ours.out0, tag, "out0");
    };

    run(&mut emu, 4.0, 2.0, [1.0, 2.0, 3.0], "basic");
    run(&mut emu, 0.0, 1.0, [1.0, 1.0, 1.0], "zero");
    run(&mut emu, f32::NAN, 1.0, [1.0, 1.0, 1.0], "nan");
    let pdz = f32::from_bits(0xc116_edc6);
    let pspan = f32::from_bits(0x4105_fc36);
    let pbasis = f32::from_bits(0x4b98_3bf5);
    run(&mut emu, pdz, pspan, [pbasis, pbasis, pbasis], "split");
    {
        let factor_f32 = rainspawn_wind_factor(pdz, pspan);
        let factor_live = rainspawn_wind_factor_live(pdz, pspan);
        let ours = rainspawn_wind_apply(factor_live, factor_f32, [pbasis, pbasis, pbasis]);
        assert_ne!(
            ours.out2.to_bits(),
            ours.out0.to_bits(),
            "R8 split probe must diverge (live out2 vs f32-reload out0): out2={:#010x} out0={:#010x}",
            ours.out2.to_bits(),
            ours.out0.to_bits()
        );
    }
    for _ in 0..400 {
        let big = f32::from_bits(0x4b80_0000 | (next(&mut s) >> 9));
        run(
            &mut emu,
            fuzz(&mut s),
            fuzz(&mut s),
            [big, big, big],
            "fuzz",
        );
    }
}

/// `0x674e2b` to `0x674e4e`: the position less the wind offset.
#[test]
fn rainspawn_wind_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7277_645f_u32;

    let run = |emu: &mut Emu, pos: [f32; 3], woff: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x34, pos[2].to_bits());
        emu.write_u32(frame - 0x38, pos[1].to_bits());
        emu.write_u32(frame - 0x3c, pos[0].to_bits());
        emu.write_u32(frame - 0x14, woff[2].to_bits());
        emu.write_u32(frame - 0x18, woff[1].to_bits());
        emu.write_u32(frame - 0x1c, woff[0].to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x674e2b, 0x674e4e);
        assert_field(
            emu.read_f32(sp + 8),
            rainspawn_wind_delta(pos[2], woff[2]),
            tag,
            "d2",
        );
        assert_field(
            emu.read_f32(sp + 4),
            rainspawn_wind_delta(pos[1], woff[1]),
            tag,
            "d1",
        );
        assert_field(
            emu.read_f32(sp),
            rainspawn_wind_delta(pos[0], woff[0]),
            tag,
            "d0",
        );
    };

    run(&mut emu, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0], "basic");
    run(&mut emu, [0.0; 3], [0.0; 3], "zero");
    run(
        &mut emu,
        [f32::NAN, f32::INFINITY, 1.0],
        [0.0, f32::INFINITY, 1.0],
        "nan-inf",
    );
    for _ in 0..300 {
        run(
            &mut emu,
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            "fuzz",
        );
    }
}

/// `0x674e66` to `0x674eb0`: the sky cosine and sine (within one ulp) and the rotation angle.
#[test]
fn rainspawn_sky_time_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mgr = emu.alloc(0x100);
    let frame = emu.alloc(0x200) + 0x100;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7273_6b79_u32;
    let mut ulp_red = 0u32;

    let run = |emu: &mut Emu, g78: f32, g7c: f32, tag: &str, red: &mut u32| {
        emu.write_u32(G_C6326C, mgr as u32);
        emu.write_u32(mgr + 0x78, g78.to_bits());
        emu.write_u32(mgr + 0x7c, g7c.to_bits());
        emu.write_u32(frame - 0x5c, 0xdead_beef);
        emu.write_u32(frame - 0x58, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let ang_slot = espbase - 0xc;
        emu.write_u32(ang_slot, 0xdead_beef);
        emu.run(0x674e66, 0x674eb0);
        let bcos = emu.read_f32(frame - 0x5c);
        let bsin = emu.read_f32(frame - 0x58);
        let bangle = emu.read_f32(ang_slot);
        let ours = rainspawn_sky_time(g78, g7c);
        assert_field(bangle, ours.angle, tag, "angle");
        assert!(
            within_1ulp(bcos, ours.cos),
            "{tag}.cos beyond 1ulp: binary={:#010x} ours={:#010x}",
            bcos.to_bits(),
            ours.cos.to_bits()
        );
        assert!(
            within_1ulp(bsin, ours.sin),
            "{tag}.sin beyond 1ulp: binary={:#010x} ours={:#010x}",
            bsin.to_bits(),
            ours.sin.to_bits()
        );
        if bcos.to_bits() != ours.cos.to_bits() || bsin.to_bits() != ours.sin.to_bits() {
            *red += 1;
        }
    };

    run(&mut emu, 0.0, 1.0, "zero", &mut ulp_red);
    run(&mut emu, 1.5707964, 2.0, "halfpi", &mut ulp_red);
    run(&mut emu, 3.25, 18.0, "near-pi", &mut ulp_red);
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz_bounded(&mut s, 100.0),
            fuzz(&mut s),
            "fuzz",
            &mut ulp_red,
        );
    }
}

/// `0x674f6b` to `0x674fb1`: the wind anchor times 1.75.
#[test]
fn rainspawn_world_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mgr = emu.alloc(0x100);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7277_7363_u32;

    let run = |emu: &mut Emu, v: [f32; 3], tag: &str| {
        emu.write_u32(G_C6326C, mgr as u32);
        emu.write_u32(mgr + 0x50, v[0].to_bits());
        emu.write_u32(mgr + 0x54, v[1].to_bits());
        emu.write_u32(mgr + 0x58, v[2].to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x674f6b, 0x674fb1);
        assert_field(emu.read_f32(sp + 8), rainspawn_world_scale(v[2]), tag, "s2");
        assert_field(emu.read_f32(sp + 4), rainspawn_world_scale(v[1]), tag, "s1");
        assert_field(emu.read_f32(sp), rainspawn_world_scale(v[0]), tag, "s0");
    };

    run(&mut emu, [1.0, 2.0, 4.0], "basic");
    run(&mut emu, [0.0, -1.0, 1.0e20], "edge");
    run(&mut emu, [f32::NAN, f32::INFINITY, 0.333_333_34], "nan-sub");
    for _ in 0..300 {
        run(&mut emu, [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)], "fuzz");
    }
}

/// `0x674fb6` to `0x674ff1`: the camera and global sums, whose z is stored before the global is
/// added.
#[test]
fn rainspawn_cam_global_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7263_6d67_u32;

    let run = |emu: &mut Emu, world: [f32; 3], cam: [f32; 3], g: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x50, world[0].to_bits());
        emu.write_u32(frame - 0x4c, world[1].to_bits());
        emu.write_u32(frame - 0x48, world[2].to_bits());
        emu.write_u32(esi, cam[0].to_bits());
        emu.write_u32(esi + 4, cam[1].to_bits());
        emu.write_u32(esi + 8, cam[2].to_bits());
        emu.write_u32(G_C7CF20, g[0].to_bits());
        emu.write_u32(G_C7CF24, g[1].to_bits());
        emu.write_u32(G_C7CF28, g[2].to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x674fb6, 0x674ff1);
        let ours = rainspawn_cam_global(world, cam, g[0], g[1], g[2]);
        assert_field(emu.read_f32(esi), ours.out_x, tag, "out_x");
        assert_field(emu.read_f32(esi + 4), ours.out_y, tag, "out_y");
        assert_field(emu.read_f32(esi + 8), ours.out_z, tag, "out_z");
    };

    run(
        &mut emu,
        [1.0, 2.0, 3.0],
        [10.0, 20.0, 30.0],
        [100.0, 200.0, 300.0],
        "basic",
    );
    run(&mut emu, [0.0; 3], [0.0; 3], [0.0; 3], "zero");
    run(
        &mut emu,
        [f32::NAN, f32::INFINITY, 1.0],
        [0.0, 0.0, f32::INFINITY],
        [1.0, 1.0, 1.0],
        "nan-inf",
    );
    let pw = f32::from_bits(0x3f4f_1bbc);
    let pc = f32::from_bits(0x4b00_0002);
    let pg = f32::from_bits(0x3f00_0027);
    run(&mut emu, [pw; 3], [pc; 3], [pg; 3], "split");
    {
        let ours = rainspawn_cam_global([pw; 3], [pc; 3], pg, pg, pg);
        assert_ne!(
            ours.out_x.to_bits(),
            ours.out_z.to_bits(),
            "R12 split probe must diverge (live x vs f32-reload z): x={:#010x} z={:#010x}",
            ours.out_x.to_bits(),
            ours.out_z.to_bits()
        );
    }
    for _ in 0..400 {
        let big = f32::from_bits(0x4b80_0000 | (next(&mut s) >> 9));
        run(
            &mut emu,
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [big, big, big],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            "fuzz",
        );
    }
}

/// `0x674ff1` to `0x675013`: the second wind apply.
#[test]
fn rainspawn_wind_scale2_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7277_7332_u32;

    let run = |emu: &mut Emu, factor: f32, basis: [f32; 3], tag: &str| {
        emu.write_u32(frame + 8, factor.to_bits());
        emu.write_u32(edi, basis[0].to_bits());
        emu.write_u32(edi + 4, basis[1].to_bits());
        emu.write_u32(edi + 8, basis[2].to_bits());
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x674ff1, 0x675013);
        assert_field(
            emu.read_f32(sp + 8),
            rainspawn_wind_scale2(factor, basis[2]),
            tag,
            "v2",
        );
        assert_field(
            emu.read_f32(sp + 4),
            rainspawn_wind_scale2(factor, basis[1]),
            tag,
            "v1",
        );
        assert_field(
            emu.read_f32(sp),
            rainspawn_wind_scale2(factor, basis[0]),
            tag,
            "v0",
        );
    };

    run(&mut emu, 3.2, [1.0, 2.0, 3.0], "basic");
    run(&mut emu, 0.0, [1.0, 1.0, 1.0], "zero");
    run(&mut emu, f32::NAN, [1.0, 0.0, f32::INFINITY], "nan");
    run(&mut emu, 0.333_333_34, [0.1, 0.7, 1.0e20], "subf32");
    for _ in 0..300 {
        run(
            &mut emu,
            fuzz(&mut s),
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            "fuzz",
        );
    }
}

/// `0x675018` to `0x67503a`: the position plus the wind offset.
#[test]
fn rainspawn_pos_offset_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7270_6f66_u32;

    let run = |emu: &mut Emu, woff: [f32; 3], pos: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x14, woff[2].to_bits());
        emu.write_u32(frame - 0x18, woff[1].to_bits());
        emu.write_u32(frame - 0x1c, woff[0].to_bits());
        emu.write_u32(esi, pos[0].to_bits());
        emu.write_u32(esi + 4, pos[1].to_bits());
        emu.write_u32(esi + 8, pos[2].to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x675018, 0x67503a);
        assert_field(
            emu.read_f32(sp + 8),
            rainspawn_pos_offset(woff[2], pos[2]),
            tag,
            "o2",
        );
        assert_field(
            emu.read_f32(sp + 4),
            rainspawn_pos_offset(woff[1], pos[1]),
            tag,
            "o1",
        );
        assert_field(
            emu.read_f32(sp),
            rainspawn_pos_offset(woff[0], pos[0]),
            tag,
            "o0",
        );
    };

    run(&mut emu, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0], "basic");
    run(&mut emu, [0.0; 3], [0.0; 3], "zero");
    run(
        &mut emu,
        [f32::NAN, 1.0, f32::INFINITY],
        [0.0, f32::INFINITY, 0.0],
        "nan-inf",
    );
    run(
        &mut emu,
        [0.1, 0.7, 1.0e20],
        [0.333_333_34, 0.3, -1.0e20],
        "subf32",
    );
    for _ in 0..300 {
        run(
            &mut emu,
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            "fuzz",
        );
    }
}

/// `0x6750ad` to `0x6750cf`: the wind apply on the collision leg.
#[test]
fn rainspawn_collide_wind_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7263_7764_u32;

    let run = |emu: &mut Emu, factor: f32, basis: [f32; 3], tag: &str| {
        emu.write_u32(frame + 8, factor.to_bits());
        emu.write_u32(edi, basis[0].to_bits());
        emu.write_u32(edi + 4, basis[1].to_bits());
        emu.write_u32(edi + 8, basis[2].to_bits());
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x6750ad, 0x6750cf);
        assert_field(
            emu.read_f32(sp + 8),
            rainspawn_collide_wind(factor, basis[2]),
            tag,
            "v2",
        );
        assert_field(
            emu.read_f32(sp + 4),
            rainspawn_collide_wind(factor, basis[1]),
            tag,
            "v1",
        );
        assert_field(
            emu.read_f32(sp),
            rainspawn_collide_wind(factor, basis[0]),
            tag,
            "v0",
        );
    };

    run(&mut emu, 3.2, [1.0, 2.0, 3.0], "basic");
    run(&mut emu, 0.0, [1.0, 1.0, 1.0], "zero");
    run(&mut emu, f32::NAN, [1.0, 0.0, f32::INFINITY], "nan");
    for _ in 0..300 {
        run(
            &mut emu,
            fuzz(&mut s),
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            "fuzz",
        );
    }
}

/// `0x6750d4` to `0x6750f7`: the collision leg products times the selector.
#[test]
fn rainspawn_collide_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7263_7363_u32;

    let run = |emu: &mut Emu, v: [f32; 3], scale: f32, tag: &str| {
        emu.write_u32(frame - 0x60, v[2].to_bits());
        emu.write_u32(frame - 0x64, v[1].to_bits());
        emu.write_u32(frame - 0x68, v[0].to_bits());
        emu.write_u32(frame - 0x4, scale.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x6750d4, 0x6750f7);
        assert_field(
            emu.read_f32(sp + 8),
            rainspawn_collide_scale(v[2], scale),
            tag,
            "v2",
        );
        assert_field(
            emu.read_f32(sp + 4),
            rainspawn_collide_scale(v[1], scale),
            tag,
            "v1",
        );
        assert_field(
            emu.read_f32(sp),
            rainspawn_collide_scale(v[0], scale),
            tag,
            "v0",
        );
    };

    run(&mut emu, [1.0, 2.0, 3.0], 1.0, "scale1");
    run(&mut emu, [1.0, 2.0, 3.0], 2.0, "scale2");
    run(&mut emu, [0.0; 3], 2.0, "zero");
    run(&mut emu, [f32::NAN, 0.0, f32::INFINITY], 1.0, "nan");
    for _ in 0..300 {
        run(
            &mut emu,
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            if next(&mut s) >> 8 & 1 == 0 { 1.0 } else { 2.0 },
            "fuzz",
        );
    }
}

/// `0x6750fc` to `0x67511f`: the collision leg position.
#[test]
fn rainspawn_collide_pos_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7263_7073_u32;

    let run = |emu: &mut Emu, v: [f32; 3], pos: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x28, v[2].to_bits());
        emu.write_u32(frame - 0x2c, v[1].to_bits());
        emu.write_u32(frame - 0x30, v[0].to_bits());
        emu.write_u32(frame - 0x34, pos[2].to_bits());
        emu.write_u32(frame - 0x38, pos[1].to_bits());
        emu.write_u32(frame - 0x3c, pos[0].to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x6750fc, 0x67511f);
        assert_field(
            emu.read_f32(sp + 8),
            rainspawn_collide_pos(v[2], pos[2]),
            tag,
            "o2",
        );
        assert_field(
            emu.read_f32(sp + 4),
            rainspawn_collide_pos(v[1], pos[1]),
            tag,
            "o1",
        );
        assert_field(
            emu.read_f32(sp),
            rainspawn_collide_pos(v[0], pos[0]),
            tag,
            "o0",
        );
    };

    run(&mut emu, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0], "basic");
    run(&mut emu, [0.0; 3], [0.0; 3], "zero");
    run(
        &mut emu,
        [f32::NAN, 1.0, f32::INFINITY],
        [0.0, f32::INFINITY, 0.0],
        "nan-inf",
    );
    for _ in 0..300 {
        run(
            &mut emu,
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            "fuzz",
        );
    }
}

/// `0x675181` to `0x6751c3`: the speed, its reciprocal and the hit delta, with `0x4549f0` run for
/// real.
#[test]
fn rainspawn_velmag_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x40);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x400) + 0x200;
    let mut s = 0x7276_6d61_u32;

    let run = |emu: &mut Emu, vel: [f32; 3], hit: [f32; 3], target: [f32; 3], tag: &str| {
        emu.write_u32(edi, vel[0].to_bits());
        emu.write_u32(edi + 4, vel[1].to_bits());
        emu.write_u32(edi + 8, vel[2].to_bits());
        emu.write_u32(esi, hit[0].to_bits());
        emu.write_u32(esi + 4, hit[1].to_bits());
        emu.write_u32(esi + 8, hit[2].to_bits());
        emu.write_u32(frame - 0x10, target[0].to_bits());
        emu.write_u32(frame - 0xc, target[1].to_bits());
        emu.write_u32(frame - 0x8, target[2].to_bits());
        emu.write_u32(frame + 8, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x675181, 0x6751c3);
        let velmag = rainspawn_velmag(vel);
        let inv = rainspawn_inv_velmag(velmag);
        assert_field(emu.read_f32(frame + 8), inv, tag, "inv_velmag");
        assert_field(
            emu.read_f32(frame - 0x30),
            rainspawn_hit_delta(hit[0], target[0]),
            tag,
            "dx",
        );
        assert_field(
            emu.read_f32(frame - 0x2c),
            rainspawn_hit_delta(hit[1], target[1]),
            tag,
            "dy",
        );
        assert_field(
            emu.read_f32(frame - 0x28),
            rainspawn_hit_delta(hit[2], target[2]),
            tag,
            "dz",
        );
    };

    run(
        &mut emu,
        [3.0, 4.0, 0.0],
        [3.0, 4.0, 5.0],
        [0.0, 0.0, 0.0],
        "3-4-5",
    );
    run(
        &mut emu,
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        "unit",
    );
    run(
        &mut emu,
        [0.0, 0.0, 0.0],
        [1.0, 2.0, 3.0],
        [0.0, 0.0, 0.0],
        "vel0",
    );
    run(
        &mut emu,
        [0.5, 0.25, 0.125],
        [0.333_333_34, 0.1, 0.7],
        [0.1, 0.333_333_34, 0.2],
        "subf32",
    );
    for _ in 0..400 {
        run(
            &mut emu,
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            "fuzz",
        );
    }
}

/// `0x6751c3` to `0x675201`: the residual along the velocity, with the path fraction kept wide.
#[test]
fn rainspawn_residual_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x400) + 0x200;
    let mut s = 0x7272_6573_u32;

    let run = |emu: &mut Emu, delta: [f32; 3], vel: [f32; 3], inv: f32, tag: &str| {
        emu.write_u32(frame - 0x30, delta[0].to_bits());
        emu.write_u32(frame - 0x2c, delta[1].to_bits());
        emu.write_u32(frame - 0x28, delta[2].to_bits());
        emu.write_u32(frame - 0x10, delta[0].to_bits());
        emu.write_u32(frame - 0xc, delta[1].to_bits());
        emu.write_u32(frame - 0x8, delta[2].to_bits());
        emu.write_u32(edi, vel[0].to_bits());
        emu.write_u32(edi + 4, vel[1].to_bits());
        emu.write_u32(edi + 8, vel[2].to_bits());
        emu.write_u32(frame + 8, inv.to_bits());
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::ECX, frame - 0x30);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x6751c3, 0x675201);
        let sp = espbase - 0xc;
        let ours = rainspawn_residual(delta, vel, inv);
        assert_field(emu.read_f32(sp + 8), ours.res2, tag, "res2");
        assert_field(emu.read_f32(sp + 4), ours.res1, tag, "res1");
        assert_field(emu.read_f32(sp), ours.res0, tag, "res0");
    };

    run(&mut emu, [3.0, 4.0, 0.0], [1.0, 0.0, 0.0], 0.2, "basic");
    run(&mut emu, [0.0; 3], [1.0, 1.0, 1.0], 1.0, "zero-delta");
    run(&mut emu, [1.0, 2.0, 3.0], [0.0, 0.0, 0.0], 0.5, "zero-vel");
    run(
        &mut emu,
        [0.333_333_34, 0.1, 0.7],
        [0.5, 0.25, 0.125],
        0.142_857_15,
        "subf32",
    );
    run(
        &mut emu,
        [f32::NAN, 1.0, f32::INFINITY],
        [1.0, 1.0, 1.0],
        0.5,
        "nan-inf",
    );
    for _ in 0..400 {
        run(
            &mut emu,
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            fuzz(&mut s),
            "fuzz",
        );
    }
}

/// `0x67521c` to `0x67523c`: the final hit delta.
#[test]
fn rainspawn_final_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7266_645f_u32;

    let run = |emu: &mut Emu, hit: [f32; 3], target: [f32; 3], tag: &str| {
        emu.write_u32(esi, hit[0].to_bits());
        emu.write_u32(esi + 4, hit[1].to_bits());
        emu.write_u32(esi + 8, hit[2].to_bits());
        emu.write_u32(frame - 0x10, target[0].to_bits());
        emu.write_u32(frame - 0xc, target[1].to_bits());
        emu.write_u32(frame - 0x8, target[2].to_bits());
        emu.write_u32(frame - 0x30, 0xdead_beef);
        emu.write_u32(frame - 0x2c, 0xdead_beef);
        emu.write_u32(frame - 0x28, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x67521c, 0x67523c);
        assert_field(
            emu.read_f32(frame - 0x30),
            rainspawn_final_delta(hit[0], target[0]),
            tag,
            "dx",
        );
        assert_field(
            emu.read_f32(frame - 0x2c),
            rainspawn_final_delta(hit[1], target[1]),
            tag,
            "dy",
        );
        assert_field(
            emu.read_f32(frame - 0x28),
            rainspawn_final_delta(hit[2], target[2]),
            tag,
            "dz",
        );
    };

    run(&mut emu, [3.0, 4.0, 5.0], [0.0, 0.0, 0.0], "basic");
    run(&mut emu, [1.0; 3], [1.0; 3], "zero");
    run(
        &mut emu,
        [f32::NAN, f32::INFINITY, 1.0],
        [0.0, f32::INFINITY, 1.0],
        "nan-inf",
    );
    run(
        &mut emu,
        [0.333_333_34, 0.1, 1.0e20],
        [0.1, 0.333_333_34, -1.0e20],
        "subf32",
    );
    for _ in 0..300 {
        run(
            &mut emu,
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            "fuzz",
        );
    }
}

/// `0x67523c` to `0x675251`: the lifetime.
#[test]
fn rainspawn_lifetime_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x400) + 0x200;
    let mut s = 0x726c_6966_u32;

    let run = |emu: &mut Emu, delta: [f32; 3], inv: f32, bias: f32, tag: &str| {
        emu.write_u32(frame - 0x30, delta[0].to_bits());
        emu.write_u32(frame - 0x2c, delta[1].to_bits());
        emu.write_u32(frame - 0x28, delta[2].to_bits());
        emu.write_u32(frame + 8, inv.to_bits());
        emu.write_u32(esi + 0x1c, bias.to_bits());
        emu.write_u32(esi + 0x20, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::ECX, frame - 0x30);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x67523c, 0x675251);
        let binary = emu.read_f32(esi + 0x20);
        assert_field(
            binary,
            rainspawn_lifetime(delta, inv, bias),
            tag,
            "lifetime",
        );
    };

    run(&mut emu, [3.0, 4.0, 0.0], 0.2, 1.0, "3-4-5");
    run(&mut emu, [0.0; 3], 1.0, 0.0, "zero-dist");
    run(&mut emu, [1.0, 0.0, 0.0], f32::INFINITY, 0.0, "inf-inv");
    run(&mut emu, [f32::NAN, 0.0, 0.0], 0.5, 0.0, "nan");
    run(
        &mut emu,
        [0.333_333_34, 0.1, 0.7],
        0.142_857_15,
        0.05,
        "subf32",
    );
    for _ in 0..400 {
        run(
            &mut emu,
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            fuzz(&mut s),
            fuzz(&mut s),
            "fuzz",
        );
    }
}
