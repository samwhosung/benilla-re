//! The arithmetic of `0x677750` against the exe: each stretch of the function between two addresses
//! runs as the exe's own instructions on a frame laid out in the emulator, and the stored values
//! are compared with the transcription bit for bit. The sines and cosines (`fsincos`, `fcos`,
//! `fsin`) are compared within one unit in the last place, since the host's functions stand in for
//! the x87's; the angles that feed them and every other value are bit-exact.
//!
//! Host double (`docs/METHOD.md#the-harness`): `snowfl_sincos_difftest` runs `fsincos` at
//! `0x677846`. `snowfl_sky_rot_difftest` runs `fcos` at `0x67797c` and `fsin` at `0x677981`. The
//! emulator computes each in host `double`, so where its result reaches a compared value the host's
//! math stands in for the x87's, and the test shows the code around it.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::wx_snowspawn::{
    snowfl_add_base, snowfl_add_offset, snowfl_angle1, snowfl_cam_global, snowfl_cell_delta,
    snowfl_drift, snowfl_drift_vel, snowfl_lifetime, snowfl_mtx_scale, snowfl_phase,
    snowfl_pos_seed, snowfl_rng_unit_float, snowfl_sincos, snowfl_sky_rot, snowfl_sub_drift,
    snowfl_vel_drift_scale, snowfl_vel_flag_scale, snowfl_vel_z_scale, snowfl_world_scale,
};
use common::next;

/// Compare two f32 bit for bit, any two NaN equal.
fn snowfl_assert(binary: f32, ours: f32, tag: &str, field: &str) {
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
fn snowfl_within_1ulp(a: f32, b: f32) -> bool {
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
fn snowfl_fuzz(s: &mut u32) -> f32 {
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
fn snowfl_fuzz_rng(s: &mut u32) -> f32 {
    snowfl_rng_unit_float(next(s))
}

const G_C6326C: u64 = 0xc6326c;

/// `0x67775d` to `0x67777e`: the three cell spans.
#[test]
fn snowfl_cell_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x736e_6f77_u32;

    let run = |emu: &mut Emu, hi: [f32; 3], lo: [f32; 3], tag: &str| {
        emu.write_u32(obj + 0x4, lo[0].to_bits());
        emu.write_u32(obj + 0x8, lo[1].to_bits());
        emu.write_u32(obj + 0xc, lo[2].to_bits());
        emu.write_u32(obj + 0x10, hi[0].to_bits());
        emu.write_u32(obj + 0x14, hi[1].to_bits());
        emu.write_u32(obj + 0x18, hi[2].to_bits());
        emu.write_u32(frame - 0x18, 0xdead_beef);
        emu.write_u32(frame - 0x14, 0xdead_beef);
        emu.write_u32(frame - 0x10, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x67775d, 0x67777e);
        for (i, off) in [0x18u64, 0x14, 0x10].into_iter().enumerate() {
            snowfl_assert(
                emu.read_f32(frame - off),
                snowfl_cell_delta(hi[i], lo[i]),
                tag,
                "delta",
            );
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
        let hi = [
            snowfl_fuzz(&mut s),
            snowfl_fuzz(&mut s),
            snowfl_fuzz(&mut s),
        ];
        let lo = [
            snowfl_fuzz(&mut s),
            snowfl_fuzz(&mut s),
            snowfl_fuzz(&mut s),
        ];
        run(&mut emu, hi, lo, "fuzz");
    }
    run(&mut emu, [1.5; 3], [1.5; 3], "eq");
    let up = f32::from_bits(1.0_f32.to_bits() + 1);
    run(&mut emu, [up; 3], [1.0; 3], "1ulp");
}

/// `0x6777a7` to `0x6777e9`: the position seeds from two draws.
#[test]
fn snowfl_pos_seed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x706f_7365_u32;

    #[allow(clippy::too_many_arguments)]
    let run = |emu: &mut Emu, r1: f32, r2: f32, dx: f32, dy: f32, dz: f32, tag: &str| {
        emu.write_u32(frame - 0x20, r1.to_bits());
        emu.write_u32(frame - 0x1c, r2.to_bits());
        emu.write_u32(frame - 0x18, dx.to_bits());
        emu.write_u32(frame - 0x14, dy.to_bits());
        emu.write_u32(frame - 0x10, dz.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x6777a7, 0x6777e9);
        let ours = snowfl_pos_seed(r1, r2, dx, dy, dz);
        snowfl_assert(emu.read_f32(sp), ours.out_x, tag, "out_x");
        snowfl_assert(emu.read_f32(sp + 4), ours.out_y, tag, "out_y");
        snowfl_assert(emu.read_f32(sp + 8), ours.out_z, tag, "out_z");
    };

    run(&mut emu, 1.0, 1.0, 1.0, 1.0, 1.0, "unit");
    run(&mut emu, 1.5, 1.5, 2.0, 3.0, 4.0, "mid");
    run(&mut emu, 1.0, 2.0, 0.0, 0.0, 0.0, "zero-d");
    run(
        &mut emu,
        snowfl_rng_unit_float(0x1234_5678),
        snowfl_rng_unit_float(0x7777_7777),
        0.333_333_34,
        0.1,
        100.0,
        "subf32",
    );
    for _ in 0..400 {
        run(
            &mut emu,
            snowfl_fuzz_rng(&mut s),
            snowfl_fuzz_rng(&mut s),
            snowfl_fuzz(&mut s),
            snowfl_fuzz(&mut s),
            snowfl_fuzz(&mut s),
            "fuzz",
        );
    }
}

/// `0x6777f8` to `0x67784c`: the wind angle bit-exact and its cosine and sine within one ulp.
#[test]
fn snowfl_sincos_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7369_6e63_u32;
    let mut ulp_red = 0u32;

    let run = |emu: &mut Emu, m: f32, rng_raw: u32, tag: &str, red: &mut u32| {
        let rng3 = snowfl_rng_unit_float(rng_raw);
        emu.write_u32(obj + 0xa4, m.to_bits());
        emu.write_u32(frame - 0x44, 0xdead_beef);
        emu.write_u32(frame - 0x40, 0xdead_beef);
        emu.set_reg(RegisterX86::EAX, u64::from(rng_raw));
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x6777f8, 0x67784c);
        let bcos = emu.read_f32(frame - 0x44);
        let bsin = emu.read_f32(frame - 0x40);
        let ours = snowfl_sincos(m, rng3);
        assert!(
            snowfl_within_1ulp(bsin, ours.sin),
            "{tag}.sin beyond 1ulp: binary={:#010x} ours={:#010x}",
            bsin.to_bits(),
            ours.sin.to_bits()
        );
        assert!(
            snowfl_within_1ulp(bcos, ours.cos),
            "{tag}.cos beyond 1ulp: binary={:#010x} ours={:#010x}",
            bcos.to_bits(),
            ours.cos.to_bits()
        );
        if bsin.to_bits() != ours.sin.to_bits() || bcos.to_bits() != ours.cos.to_bits() {
            *red += 1;
        }
    };

    run(&mut emu, 1.0, 0x4000_0000, "m1", &mut ulp_red);
    run(&mut emu, 0.0, 0, "m0", &mut ulp_red);
    run(&mut emu, 0.5, 0, "rng-lo", &mut ulp_red);
    run(&mut emu, 0.5, 0x7fff_ffff, "rng-hi", &mut ulp_red);
    run(&mut emu, 4.0, 0x7fff_ffff, "m-hi", &mut ulp_red);
    for _ in 0..400 {
        let m = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 23) * 4.0) as f32;
        run(&mut emu, m, next(&mut s), "fuzz", &mut ulp_red);
    }
}

/// `0x6777f8` to `0x67783d`: the wind angle alone, bit-exact.
#[test]
fn snowfl_angle1_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x616e_6731_u32;

    let run = |emu: &mut Emu, m: f32, rng_raw: u32, tag: &str| {
        let rng3 = snowfl_rng_unit_float(rng_raw);
        emu.write_u32(obj + 0xa4, m.to_bits());
        emu.set_reg(RegisterX86::EAX, u64::from(rng_raw));
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x6777f8, 0x67783d);
        snowfl_assert(
            emu.read_f32(frame - 0x20),
            snowfl_angle1(m, rng3),
            tag,
            "angle",
        );
    };

    run(&mut emu, 1.0, 0x4000_0000, "m1");
    run(&mut emu, 0.0, 0, "m0");
    run(&mut emu, 0.5, 0x7fff_ffff, "hi");
    for _ in 0..300 {
        run(&mut emu, snowfl_fuzz(&mut s), next(&mut s), "fuzz");
    }
}

/// `0x677863` to `0x67788c`: the drift phase.
#[test]
fn snowfl_phase_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7068_6173_u32;

    let run = |emu: &mut Emu, m: f32, rng4: f32, tag: &str| {
        emu.write_u32(obj + 0xa4, m.to_bits());
        emu.write_u32(frame - 0x24, rng4.to_bits());
        emu.write_u32(frame - 0x1c, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x677863, 0x67788c);
        snowfl_assert(emu.read_f32(frame - 0x1c), snowfl_phase(m, rng4), tag, "t");
    };

    run(&mut emu, 1.0, 1.5, "m1");
    run(&mut emu, 0.0, 1.0, "m0");
    run(&mut emu, 2.5, snowfl_rng_unit_float(0), "lo");
    run(&mut emu, 2.5, snowfl_rng_unit_float(0x7fff_ffff), "hi");
    for _ in 0..300 {
        run(
            &mut emu,
            snowfl_fuzz(&mut s),
            snowfl_fuzz_rng(&mut s),
            "fuzz",
        );
    }
}

/// `0x67789b` to `0x6778f5`: the drift sine and cosine products and the vertical term.
#[test]
fn snowfl_drift_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x100) + 0x80;
    let param2 = emu.alloc(0x40);
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x6472_6966_u32;

    let run = |emu: &mut Emu, t: f32, sin: f32, cos: f32, m: f32, rng5: f32, tag: &str| {
        emu.write_u32(obj + 0xa4, m.to_bits());
        emu.write_u32(frame - 0x1c, t.to_bits());
        emu.write_u32(frame - 0x40, sin.to_bits());
        emu.write_u32(frame - 0x44, cos.to_bits());
        emu.write_u32(frame - 0xc, 0xdead_beef);
        emu.write_u32(frame - 0x8, 0xdead_beef);
        emu.write_u32(frame - 0x4, 0xdead_beef);
        emu.write_u32(frame + 8, param2 as u32);
        emu.set_reg(RegisterX86::EAX, u64::from(rng5.to_bits()));
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x67789b, 0x6778f5);
        let ours = snowfl_drift(t, sin, cos, m, rng5);
        snowfl_assert(emu.read_f32(frame - 0xc), ours.drift_sin, tag, "drift_sin");
        snowfl_assert(emu.read_f32(frame - 0x8), ours.drift_cos, tag, "drift_cos");
        snowfl_assert(emu.read_f32(frame - 0x4), ours.u, tag, "u");
    };

    run(&mut emu, 1.0, 0.5, 0.866_025_4, 1.0, 1.5, "basic");
    run(&mut emu, 0.0, 1.0, 1.0, 0.0, 1.0, "zero");
    run(&mut emu, f32::NAN, 1.0, 1.0, 1.0, 1.5, "nan");
    run(
        &mut emu,
        0.333_333_34,
        0.1,
        0.7,
        0.142_857_15,
        snowfl_rng_unit_float(0x55),
        "subf32",
    );
    for _ in 0..300 {
        run(
            &mut emu,
            snowfl_fuzz(&mut s),
            snowfl_fuzz(&mut s),
            snowfl_fuzz(&mut s),
            snowfl_fuzz(&mut s),
            snowfl_fuzz_rng(&mut s),
            "fuzz",
        );
    }
}

/// `0x6778f5` to `0x677925`: the drift products, where the z product uses the wide scalar and the
/// others the stored one.
#[test]
fn snowfl_drift_vel_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let param2 = emu.alloc(0x80);
    let espbase = emu.alloc(0x100) + 0x40;
    let mut s = 0x6476_656c_u32;

    let run = |emu: &mut Emu, dz: f32, u: f32, vel0: f32, vel1: f32, tag: &str| {
        emu.write_u32(frame - 0x10, dz.to_bits());
        emu.write_u32(frame - 0x4, u.to_bits());
        emu.write_u32(param2 + 0xc, vel0.to_bits());
        emu.write_u32(param2 + 0x10, vel1.to_bits());
        emu.write_u32(param2 + 0x14, 0xdead_beef);
        emu.write_u32(frame + 8, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, param2);
        emu.set_reg(RegisterX86::EDI, param2 + 0xc);
        emu.set_reg(RegisterX86::EDX, param2 + 0xc);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x6778f5, 0x677925);
        let ours = snowfl_drift_vel(dz, u, vel0, vel1);
        snowfl_assert(emu.read_f32(sp), ours.out_x, tag, "out_x");
        snowfl_assert(emu.read_f32(sp + 4), ours.out_y, tag, "out_y");
        snowfl_assert(emu.read_f32(sp + 8), ours.out_z, tag, "out_z");
    };

    run(&mut emu, 4.0, 2.0, 1.0, 2.0, "basic");
    run(&mut emu, 0.0, 1.0, 1.0, 1.0, "zero-dz");
    run(&mut emu, 1.0, 0.0, 1.0, 1.0, "u0-div0");
    run(&mut emu, f32::NAN, 1.0, 1.0, 1.0, "nan");
    let pdz = f32::from_bits(0x4049_1bbc);
    let pu = f32::from_bits(0x3f80_0001);
    run(&mut emu, pdz, pu, 1.0, 1.0, "split");
    {
        let ours = snowfl_drift_vel(pdz, pu, 1.0, 1.0);
        let _ = ours;
    }
    for _ in 0..400 {
        let u = f32::from_bits(0x3f80_0000 | (next(&mut s) >> 9));
        run(
            &mut emu,
            snowfl_fuzz(&mut s),
            u,
            snowfl_fuzz(&mut s),
            snowfl_fuzz(&mut s),
            "fuzz",
        );
    }
}

/// `0x67792a` to `0x67794d`: the drift subtracted per axis.
#[test]
fn snowfl_sub_drift_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7375_6264_u32;

    let run = |emu: &mut Emu, base: [f32; 3], drift: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x34, base[0].to_bits());
        emu.write_u32(frame - 0x38, base[1].to_bits());
        emu.write_u32(frame - 0x3c, base[2].to_bits());
        emu.write_u32(frame - 0x4, drift[0].to_bits());
        emu.write_u32(frame - 0x8, drift[1].to_bits());
        emu.write_u32(frame - 0xc, drift[2].to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x67792a, 0x67794d);
        snowfl_assert(
            emu.read_f32(sp + 8),
            snowfl_sub_drift(base[0], drift[0]),
            tag,
            "d2",
        );
        snowfl_assert(
            emu.read_f32(sp + 4),
            snowfl_sub_drift(base[1], drift[1]),
            tag,
            "d1",
        );
        snowfl_assert(
            emu.read_f32(sp),
            snowfl_sub_drift(base[2], drift[2]),
            tag,
            "d0",
        );
    };

    run(&mut emu, [1.0, 2.0, 3.0], [0.5, 0.5, 0.5], "basic");
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
        run(
            &mut emu,
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            "fuzz",
        );
    }
    run(&mut emu, [1.5; 3], [1.5; 3], "eq");
}

/// `0x67796a` to `0x677986`: the sky rotation cosine and sine within one ulp.
#[test]
fn snowfl_sky_rot_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cam = emu.alloc(0x100);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x736b_7972_u32;
    let mut ulp_red = 0u32;

    let run = |emu: &mut Emu, ang: f32, tag: &str, red: &mut u32| {
        emu.write_u32(G_C6326C, cam as u32);
        emu.write_u32(cam + 0x78, ang.to_bits());
        emu.write_u32(frame - 0x50, 0xdead_beef);
        emu.write_u32(frame - 0x4c, 0xdead_beef);
        emu.set_reg(RegisterX86::EAX, cam);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x67796a, 0x677986);
        let bcos = emu.read_f32(frame - 0x50);
        let bsin = emu.read_f32(frame - 0x4c);
        let ours = snowfl_sky_rot(ang);
        assert!(
            snowfl_within_1ulp(bcos, ours.cos),
            "{tag}.cos beyond 1ulp: binary={:#010x} ours={:#010x}",
            bcos.to_bits(),
            ours.cos.to_bits()
        );
        assert!(
            snowfl_within_1ulp(bsin, ours.sin),
            "{tag}.sin beyond 1ulp: binary={:#010x} ours={:#010x}",
            bsin.to_bits(),
            ours.sin.to_bits()
        );
        if bcos.to_bits() != ours.cos.to_bits() || bsin.to_bits() != ours.sin.to_bits() {
            *red += 1;
        }
    };

    run(&mut emu, 0.0, "zero", &mut ulp_red);
    run(
        &mut emu,
        std::f32::consts::FRAC_PI_2,
        "halfpi",
        &mut ulp_red,
    );
    run(&mut emu, std::f32::consts::PI, "pi", &mut ulp_red);
    run(&mut emu, -2.5, "neg", &mut ulp_red);
    for _ in 0..400 {
        let ang = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 23) * 200.0 - 100.0) as f32;
        run(&mut emu, ang, "fuzz", &mut ulp_red);
    }
}

/// `0x677986` to `0x6779af`: the matrix scale term.
#[test]
fn snowfl_mtx_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cam = emu.alloc(0x100);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x6d74_7873_u32;

    let run = |emu: &mut Emu, v: f32, tag: &str| {
        emu.write_u32(cam + 0x7c, v.to_bits());
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.set_reg(RegisterX86::EAX, cam);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x677986, 0x6779af);
        snowfl_assert(emu.read_f32(sp), snowfl_mtx_scale(v), tag, "scale");
    };

    run(&mut emu, 18.0, "basic");
    run(&mut emu, 0.0, "zero");
    run(&mut emu, f32::NAN, "nan");
    run(&mut emu, -1.0e20, "big-neg");
    run(&mut emu, 0.333_333_34, "subf32");
    for _ in 0..300 {
        run(&mut emu, snowfl_fuzz(&mut s), "fuzz");
    }
}

/// `0x677a58` to `0x677a86`: the wind anchor times 1.75.
#[test]
fn snowfl_world_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let anchor = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7773_636c_u32;

    let run = |emu: &mut Emu, v: [f32; 3], tag: &str| {
        emu.write_u32(frame + 0x14, anchor as u32);
        emu.write_u32(anchor, v[0].to_bits());
        emu.write_u32(anchor + 4, v[1].to_bits());
        emu.write_u32(anchor + 8, v[2].to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x677a58, 0x677a86);
        snowfl_assert(emu.read_f32(sp + 8), snowfl_world_scale(v[2]), tag, "s2");
        snowfl_assert(emu.read_f32(sp + 4), snowfl_world_scale(v[1]), tag, "s1");
        snowfl_assert(emu.read_f32(sp), snowfl_world_scale(v[0]), tag, "s0");
    };

    run(&mut emu, [1.0, 2.0, 4.0], "basic");
    run(&mut emu, [0.0, -1.0, 1.0e20], "edge");
    run(&mut emu, [f32::NAN, f32::INFINITY, 0.333_333_34], "nan-sub");
    for _ in 0..300 {
        run(
            &mut emu,
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            "fuzz",
        );
    }
}

/// `0x677a8b` to `0x677ac5`: the camera and global sums, whose z is stored before the global is
/// added.
#[test]
fn snowfl_cam_global_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x40);
    let gptr = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x6361_6d67_u32;

    let run = |emu: &mut Emu, world: [f32; 3], cam: [f32; 3], g: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x30, world[0].to_bits());
        emu.write_u32(frame - 0x2c, world[1].to_bits());
        emu.write_u32(frame - 0x28, world[2].to_bits());
        emu.write_u32(esi, cam[0].to_bits());
        emu.write_u32(esi + 4, cam[1].to_bits());
        emu.write_u32(esi + 8, cam[2].to_bits());
        emu.write_u32(frame + 0x10, gptr as u32);
        emu.write_u32(gptr, g[0].to_bits());
        emu.write_u32(gptr + 4, g[1].to_bits());
        emu.write_u32(gptr + 8, g[2].to_bits());
        emu.write_u32(frame + 8, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x677a8b, 0x677ac5);
        let ours = snowfl_cam_global(world, cam, g[0], g[1], g[2]);
        snowfl_assert(emu.read_f32(esi), ours.out_x, tag, "out_x");
        snowfl_assert(emu.read_f32(esi + 4), ours.out_y, tag, "out_y");
        snowfl_assert(emu.read_f32(esi + 8), ours.out_z, tag, "out_z");
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
        [0.333_333_34, 0.142_857_15, 0.111_111_1],
        [16_777_217.0, 16_777_217.0, 16_777_217.0],
        [0.5, 0.5, 0.5],
        "narrow",
    );
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
    run(&mut emu, [pw, pw, pw], [pc, pc, pc], [pg, pg, pg], "split");
    {
        let ours = snowfl_cam_global([pw, pw, pw], [pc, pc, pc], pg, pg, pg);
        assert_ne!(
            ours.out_x.to_bits(),
            ours.out_z.to_bits(),
            "R9 split probe must diverge (live x vs f32-reload z): x={:#010x} z={:#010x}",
            ours.out_x.to_bits(),
            ours.out_z.to_bits()
        );
    }
    for _ in 0..400 {
        let big = f32::from_bits(0x4b80_0000 | (next(&mut s) >> 9));
        run(
            &mut emu,
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            [big, big, big],
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            "fuzz",
        );
    }
}

/// `0x677ac5` to `0x677ae4`: the stored scalar times the velocity.
#[test]
fn snowfl_vel_z_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x40;
    let mut s = 0x767a_7363_u32;

    let run = |emu: &mut Emu, sc: f32, vel: [f32; 3], tag: &str| {
        emu.write_u32(frame + 8, sc.to_bits());
        emu.write_u32(edi, vel[0].to_bits());
        emu.write_u32(edi + 4, vel[1].to_bits());
        emu.write_u32(edi + 8, vel[2].to_bits());
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x677ac5, 0x677ae4);
        snowfl_assert(
            emu.read_f32(sp + 8),
            snowfl_vel_z_scale(sc, vel[2]),
            tag,
            "v2",
        );
        snowfl_assert(
            emu.read_f32(sp + 4),
            snowfl_vel_z_scale(sc, vel[1]),
            tag,
            "v1",
        );
        snowfl_assert(emu.read_f32(sp), snowfl_vel_z_scale(sc, vel[0]), tag, "v0");
    };

    run(&mut emu, 2.0, [1.0, 2.0, 3.0], "basic");
    run(&mut emu, 0.0, [1.0, 1.0, 1.0], "zero");
    run(&mut emu, f32::NAN, [1.0, 0.0, f32::INFINITY], "nan");
    run(&mut emu, 0.333_333_34, [0.1, 0.7, 1.0e20], "subf32");
    for _ in 0..300 {
        run(
            &mut emu,
            snowfl_fuzz(&mut s),
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            "fuzz",
        );
    }
}

/// `0x677ae9` to `0x677b0b`: the offsets added per axis.
#[test]
fn snowfl_add_offset_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x616f_6666_u32;

    let run = |emu: &mut Emu, a: [f32; 3], b: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x4, a[0].to_bits());
        emu.write_u32(frame - 0x8, a[1].to_bits());
        emu.write_u32(frame - 0xc, a[2].to_bits());
        emu.write_u32(frame + 0x14, b[2].to_bits());
        emu.write_u32(esi + 4, b[1].to_bits());
        emu.write_u32(esi, b[0].to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x677ae9, 0x677b0b);
        snowfl_assert(
            emu.read_f32(sp + 8),
            snowfl_add_offset(a[0], b[2]),
            tag,
            "c2",
        );
        snowfl_assert(
            emu.read_f32(sp + 4),
            snowfl_add_offset(a[1], b[1]),
            tag,
            "c1",
        );
        snowfl_assert(emu.read_f32(sp), snowfl_add_offset(a[2], b[0]), tag, "c0");
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
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            "fuzz",
        );
    }
}

/// `0x677b6d` to `0x677b8f`: the drift scalar times the velocity.
#[test]
fn snowfl_vel_drift_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7664_7363_u32;

    let run = |emu: &mut Emu, sc: f32, vel: [f32; 3], tag: &str| {
        emu.write_u32(frame + 8, sc.to_bits());
        emu.write_u32(edi, vel[0].to_bits());
        emu.write_u32(edi + 4, vel[1].to_bits());
        emu.write_u32(edi + 8, vel[2].to_bits());
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x677b6d, 0x677b8f);
        snowfl_assert(
            emu.read_f32(sp + 8),
            snowfl_vel_drift_scale(sc, vel[2]),
            tag,
            "v2",
        );
        snowfl_assert(
            emu.read_f32(sp + 4),
            snowfl_vel_drift_scale(sc, vel[1]),
            tag,
            "v1",
        );
        snowfl_assert(
            emu.read_f32(sp),
            snowfl_vel_drift_scale(sc, vel[0]),
            tag,
            "v0",
        );
    };

    run(&mut emu, 1.5, [1.0, 2.0, 3.0], "basic");
    run(&mut emu, 0.0, [1.0, 1.0, 1.0], "zero");
    run(&mut emu, f32::NAN, [1.0, 0.0, f32::INFINITY], "nan");
    run(&mut emu, 0.333_333_34, [0.1, 0.7, 1.0e20], "subf32");
    for _ in 0..300 {
        run(
            &mut emu,
            snowfl_fuzz(&mut s),
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            "fuzz",
        );
    }
}

/// `0x677b94` to `0x677bb7`: the products times the collision flag.
#[test]
fn snowfl_vel_flag_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7666_6c67_u32;

    let run = |emu: &mut Emu, vec: [f32; 3], flag: f32, tag: &str| {
        emu.write_u32(frame - 0x4, vec[0].to_bits());
        emu.write_u32(frame - 0x8, vec[1].to_bits());
        emu.write_u32(frame - 0xc, vec[2].to_bits());
        emu.write_u32(frame + 0x14, flag.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x677b94, 0x677bb7);
        snowfl_assert(
            emu.read_f32(sp + 8),
            snowfl_vel_flag_scale(vec[0], flag),
            tag,
            "v2",
        );
        snowfl_assert(
            emu.read_f32(sp + 4),
            snowfl_vel_flag_scale(vec[1], flag),
            tag,
            "v1",
        );
        snowfl_assert(
            emu.read_f32(sp),
            snowfl_vel_flag_scale(vec[2], flag),
            tag,
            "v0",
        );
    };

    run(&mut emu, [1.0, 2.0, 3.0], 2.0, "flag2");
    run(&mut emu, [1.0, 2.0, 3.0], 1.0, "flag1");
    run(&mut emu, [0.0; 3], 2.0, "zero");
    run(&mut emu, [f32::NAN, 0.0, f32::INFINITY], 2.0, "nan");
    run(&mut emu, [0.333_333_34, 0.1, 1.0e20], 2.0, "subf32");
    for _ in 0..300 {
        let flag = if next(&mut s) & 0x100 != 0 {
            2.0
        } else {
            snowfl_fuzz(&mut s)
        };
        run(
            &mut emu,
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            flag,
            "fuzz",
        );
    }
}

/// `0x677bbc` to `0x677bdf`: the base added per axis.
#[test]
fn snowfl_add_base_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x6162_6173_u32;

    let run = |emu: &mut Emu, v: [f32; 3], base: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x4, v[0].to_bits());
        emu.write_u32(frame - 0x8, v[1].to_bits());
        emu.write_u32(frame - 0xc, v[2].to_bits());
        emu.write_u32(frame - 0x34, base[0].to_bits());
        emu.write_u32(frame - 0x38, base[1].to_bits());
        emu.write_u32(frame - 0x3c, base[2].to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x677bbc, 0x677bdf);
        snowfl_assert(
            emu.read_f32(sp + 8),
            snowfl_add_base(v[0], base[0]),
            tag,
            "b2",
        );
        snowfl_assert(
            emu.read_f32(sp + 4),
            snowfl_add_base(v[1], base[1]),
            tag,
            "b1",
        );
        snowfl_assert(emu.read_f32(sp), snowfl_add_base(v[2], base[2]), tag, "b0");
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
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            "fuzz",
        );
    }
}

/// `0x677c3a` to `0x677d05`: the lifetime from the path fraction, with `0x4549f0` run for real.
#[test]
fn snowfl_lifetime_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x80);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x200) + 0x100;
    let espbase = emu.alloc(0x400) + 0x200;
    let mut s = 0x6c69_6665_u32;

    let run =
        |emu: &mut Emu, pos: [f32; 3], hit: [f32; 3], vel: [f32; 3], param: f32, tag: &str| {
            emu.write_u32(esi, pos[0].to_bits());
            emu.write_u32(esi + 4, pos[1].to_bits());
            emu.write_u32(esi + 8, pos[2].to_bits());
            emu.write_u32(frame - 0x18, hit[0].to_bits());
            emu.write_u32(frame - 0x14, hit[1].to_bits());
            emu.write_u32(frame - 0x10, hit[2].to_bits());
            emu.write_u32(edi, vel[0].to_bits());
            emu.write_u32(edi + 4, vel[1].to_bits());
            emu.write_u32(edi + 8, vel[2].to_bits());
            emu.write_u32(frame + 0xc, param.to_bits());
            emu.write_u32(esi + 0x1c, 0xdead_beef);
            emu.set_reg(RegisterX86::ESI, esi);
            emu.set_reg(RegisterX86::EDI, edi);
            emu.set_reg(RegisterX86::EBP, frame);
            emu.set_reg(RegisterX86::ESP, espbase);
            emu.run(0x677c3a, 0x677d05);
            snowfl_assert(
                emu.read_f32(esi + 0x1c),
                snowfl_lifetime(pos, hit, vel, param),
                tag,
                "lifetime",
            );
        };

    run(
        &mut emu,
        [3.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        1.0,
        "3-4-5",
    );
    run(
        &mut emu,
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        0.0,
        "zero-dist",
    );
    run(
        &mut emu,
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        0.0,
        "vel0-div0",
    );
    run(
        &mut emu,
        [f32::NAN, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        0.0,
        "nan",
    );
    run(
        &mut emu,
        [0.333_333_34, 0.1, 0.7],
        [0.1, 0.333_333_34, 0.2],
        [0.5, 0.25, 0.125],
        0.05,
        "subf32",
    );
    for _ in 0..400 {
        run(
            &mut emu,
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            [
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
                snowfl_fuzz(&mut s),
            ],
            snowfl_fuzz(&mut s),
            "fuzz",
        );
    }
}
