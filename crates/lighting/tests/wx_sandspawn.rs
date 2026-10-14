//! The arithmetic of `0x6793f0` against the exe: each stretch of the function between two addresses
//! runs as the exe's own instructions on a frame laid out in the emulator, and the stored values
//! are compared with the transcription bit for bit. The sine and cosine (`fsincos`) are compared
//! within one unit in the last place, since the host's functions stand in for the x87's; the angle
//! that feeds them and every other value is bit-exact.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::wx_sandspawn::{
    rng_unit_float, sand_cam_global, sand_cell_delta, sand_lifetime, sand_negate, sand_pos_seed,
    sand_sincos, sand_speed, sand_vel_mag, sand_vel_offset, sand_vel_scale, sand_vel_xy,
    sand_vel_z, sand_world_scale,
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

/// `0x6793f6` to `0x679418`: the three cell spans.
#[test]
fn sand_cell_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7361_6e64_u32;

    let run = |emu: &mut Emu, hi: [f32; 3], lo: [f32; 3], tag: &str| {
        emu.write_u32(obj + 0x4, lo[0].to_bits());
        emu.write_u32(obj + 0x8, lo[1].to_bits());
        emu.write_u32(obj + 0xc, lo[2].to_bits());
        emu.write_u32(obj + 0x10, hi[0].to_bits());
        emu.write_u32(obj + 0x14, hi[1].to_bits());
        emu.write_u32(obj + 0x18, hi[2].to_bits());
        emu.write_u32(frame - 0x20, 0xdead_beef);
        emu.write_u32(frame - 0x1c, 0xdead_beef);
        emu.write_u32(frame - 0x18, 0xdead_beef);
        emu.set_reg(RegisterX86::ECX, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x6793f6, 0x679418);
        for (i, off) in [0x20u64, 0x1c, 0x18].into_iter().enumerate() {
            let binary = emu.read_f32(frame - off);
            assert_field(binary, sand_cell_delta(hi[i], lo[i]), tag, "delta");
        }
    };

    run(&mut emu, [1.0, 2.0, 3.0], [0.0, 0.0, 0.0], "basic");
    run(&mut emu, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "zero");
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
    run(&mut emu, [1.5, 1.5, 1.5], [1.5, 1.5, 1.5], "eq");
    let one_ulp_up = f32::from_bits(1.0_f32.to_bits() + 1);
    run(
        &mut emu,
        [one_ulp_up, one_ulp_up, one_ulp_up],
        [1.0, 1.0, 1.0],
        "1ulp",
    );
}

/// `0x679458` to `0x6794ad`: the position seeds from three draws.
#[test]
fn sand_pos_seed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x706f_7365_u32;

    #[allow(clippy::too_many_arguments)]
    let run = |emu: &mut Emu,
               rng_x: f32,
               rng_y: f32,
               rng_z: f32,
               dx: f32,
               dy: f32,
               dz: f32,
               tag: &str| {
        emu.write_u32(frame - 0x14, rng_x.to_bits());
        emu.write_u32(frame - 0x10, rng_y.to_bits());
        emu.write_u32(frame - 0xc, rng_z.to_bits());
        emu.write_u32(frame - 0x20, dx.to_bits());
        emu.write_u32(frame - 0x1c, dy.to_bits());
        emu.write_u32(frame - 0x18, dz.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x679458, 0x6794ad);
        let bx = emu.read_f32(sp);
        let by = emu.read_f32(sp + 4);
        let bz = emu.read_f32(sp + 8);
        let ours = sand_pos_seed(rng_x, rng_y, rng_z, dx, dy, dz);
        assert_field(bx, ours.out_x, tag, "out_x");
        assert_field(by, ours.out_y, tag, "out_y");
        assert_field(bz, ours.out_z, tag, "out_z");
    };

    run(&mut emu, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, "unit");
    run(&mut emu, 1.5, 1.5, 1.5, 2.0, 3.0, 4.0, "mid");
    run(&mut emu, 1.0, 2.0, 1.0, 0.0, 0.0, 0.0, "zero-d");
    run(
        &mut emu,
        rng_unit_float(0x1234_5678),
        rng_unit_float(0x7777_7777),
        rng_unit_float(0x0a0b_0c0d),
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
            fuzz_rng(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            "fuzz",
        );
    }
}

/// `0x6794df` to `0x679518`: the angle bit-exact and its cosine and sine within one ulp.
#[test]
fn sand_sincos_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7369_6e63_u32;
    let mut ulp_red = 0u32;

    let run = |emu: &mut Emu, rng: f32, tag: &str, red: &mut u32| {
        emu.write_u32(frame + 8, rng.to_bits());
        emu.write_u32(frame - 4, 0xdead_beef);
        emu.write_u32(frame - 8, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6794df, 0x679518);
        let bcos = emu.read_f32(frame - 8);
        let bsin = emu.read_f32(frame - 4);
        let ours = sand_sincos(rng);
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

    run(&mut emu, 1.0, "rng1", &mut ulp_red);
    run(&mut emu, 1.5, "rng1.5", &mut ulp_red);
    run(&mut emu, rng_unit_float(0), "rng-lo", &mut ulp_red);
    run(
        &mut emu,
        rng_unit_float(0x7fff_ffff),
        "rng-hi",
        &mut ulp_red,
    );
    for _ in 0..400 {
        run(&mut emu, fuzz_rng(&mut s), "fuzz", &mut ulp_red);
    }
}

/// `0x6794df` to `0x679509`: the angle alone, bit-exact.
#[test]
fn sand_angle_difftest() {
    use benilla_re_lighting::wx_sandspawn::sand_angle;
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x616e_676c_u32;

    let run = |emu: &mut Emu, rng: f32, tag: &str| {
        emu.write_u32(frame + 8, rng.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6794df, 0x679509);
        let binary = emu.read_f32(frame + 8);
        assert_field(binary, sand_angle(rng), tag, "angle");
    };

    run(&mut emu, 1.0, "rng1");
    run(&mut emu, 1.5, "mid");
    run(&mut emu, rng_unit_float(0x7fff_ffff), "hi");
    for _ in 0..300 {
        run(&mut emu, fuzz_rng(&mut s), "fuzz");
    }
}

/// `0x67952f` to `0x679580`: the speed and the horizontal velocity.
#[test]
fn sand_vel_xy_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let esi = emu.alloc(0x40);
    let mut s = 0x7665_6c78_u32;

    let run_mag = |emu: &mut Emu, rng: f32, tag: &str| {
        emu.write_u32(frame + 8, rng.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x67952f, 0x679552);
        let binary = emu.read_f32(frame + 8);
        assert_field(binary, sand_vel_mag(rng), tag, "mag");
    };

    let run_xy = |emu: &mut Emu, mag: f32, cos: f32, sin: f32, tag: &str| {
        emu.write_u32(frame + 8, mag.to_bits());
        emu.write_u32(frame - 4, cos.to_bits());
        emu.write_u32(frame - 8, sin.to_bits());
        emu.write_u32(frame - 0x20, 0xdead_beef);
        emu.write_u32(frame - 0x1c, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.run(0x679557, 0x679580);
        let bvx = emu.read_f32(frame - 0x20);
        let bvy = emu.read_f32(frame - 0x1c);
        let ours = sand_vel_xy(mag, cos, sin);
        assert_field(bvx, ours.vx, tag, "vx");
        assert_field(bvy, ours.vy, tag, "vy");
    };

    run_mag(&mut emu, 1.0, "rng1");
    run_mag(&mut emu, 1.5, "mid");
    run_mag(&mut emu, rng_unit_float(0x7fff_ffff), "hi");
    run_xy(&mut emu, 18.0, 1.0, 0.0, "xonly");
    run_xy(&mut emu, 18.666, 0.5, 0.866_025_4, "30deg");
    run_xy(&mut emu, 0.0, 0.5, 0.5, "zeromag");
    run_xy(&mut emu, f32::NAN, 1.0, 1.0, "nan");
    for _ in 0..300 {
        run_mag(&mut emu, fuzz_rng(&mut s), "fuzz");
        run_xy(&mut emu, fuzz(&mut s), fuzz(&mut s), fuzz(&mut s), "fuzz");
    }
}

/// `0x679580` to `0x6795a4`: the vertical velocity.
#[test]
fn sand_vel_z_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let scratch = emu.alloc(0x20);
    let mut s = 0x7665_6c7a_u32;

    let run = |emu: &mut Emu, rng: f32, tag: &str| {
        emu.write_u32(frame - 0x14, rng.to_bits());
        emu.write_u32(frame - 0x18, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ECX, scratch);
        emu.run(0x679580, 0x6795a4);
        let binary = emu.read_f32(frame - 0x18);
        assert_field(binary, sand_vel_z(rng), tag, "vz");
    };

    run(&mut emu, 1.0, "rng1");
    run(&mut emu, 1.5, "mid");
    run(&mut emu, rng_unit_float(0), "lo");
    run(&mut emu, rng_unit_float(0x7fff_ffff), "hi");
    for _ in 0..300 {
        run(&mut emu, fuzz_rng(&mut s), "fuzz");
    }
}

/// `0x6795af` to `0x6795bf`: the negated sky angle, including -0 and NaN.
#[test]
fn sand_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x100);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x6e65_6761_u32;

    let run = |emu: &mut Emu, angle: f32, tag: &str| {
        emu.write_u32(obj + 0x78, angle.to_bits());
        emu.write_u32(frame + 8, 0xdead_beef);
        emu.set_reg(RegisterX86::EAX, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x6795af, 0x6795bf);
        let binary = emu.read_f32(frame + 8);
        assert_field(binary, sand_negate(angle), tag, "neg");
    };

    run(&mut emu, 1.0, "one");
    run(&mut emu, -2.5, "neg");
    run(&mut emu, 0.0, "zero");
    run(&mut emu, -0.0, "negative zero");
    run(&mut emu, f32::NAN, "nan");
    run(&mut emu, f32::INFINITY, "inf");
    for _ in 0..200 {
        run(&mut emu, fuzz(&mut s), "fuzz");
    }
}

/// The weather manager pointer.
const G_C6326C: u64 = 0xc6326c;

/// `0x6796b7` to `0x6796fd`: the wind anchor times 1.75.
#[test]
fn sand_world_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cam = emu.alloc(0x80);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7773_636c_u32;

    let run = |emu: &mut Emu, v: [f32; 3], tag: &str| {
        emu.write_u32(G_C6326C, cam as u32);
        emu.write_u32(cam + 0x50, v[0].to_bits());
        emu.write_u32(cam + 0x54, v[1].to_bits());
        emu.write_u32(cam + 0x58, v[2].to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        let sp = espbase - 0xc;
        emu.write_u32(sp, 0xdead_beef);
        emu.write_u32(sp + 4, 0xdead_beef);
        emu.write_u32(sp + 8, 0xdead_beef);
        emu.run(0x6796b7, 0x6796fd);
        assert_field(emu.read_f32(sp + 8), sand_world_scale(v[2]), tag, "s2");
        assert_field(emu.read_f32(sp + 4), sand_world_scale(v[1]), tag, "s1");
        assert_field(emu.read_f32(sp), sand_world_scale(v[0]), tag, "s0");
    };

    run(&mut emu, [1.0, 2.0, 4.0], "basic");
    run(&mut emu, [0.0, -1.0, 1.0e20], "edge");
    run(&mut emu, [f32::NAN, f32::INFINITY, 0.333_333_34], "nan-sub");
    for _ in 0..300 {
        run(&mut emu, [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)], "fuzz");
    }
}

const G_C7CF20: u64 = 0xc7cf20;
const G_C7CF24: u64 = 0xc7cf24;
const G_C7CF28: u64 = 0xc7cf28;

/// `0x679702` to `0x679742`: the camera and global sums, whose z is stored before the global is
/// added.
#[test]
fn sand_cam_global_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x6361_6d67_u32;

    let run = |emu: &mut Emu, world: [f32; 3], cam: [f32; 3], g: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x2c, world[0].to_bits());
        emu.write_u32(frame - 0x28, world[1].to_bits());
        emu.write_u32(frame - 0x24, world[2].to_bits());
        emu.write_u32(esi, cam[0].to_bits());
        emu.write_u32(esi + 4, cam[1].to_bits());
        emu.write_u32(esi + 8, cam[2].to_bits());
        emu.write_u32(G_C7CF20, g[0].to_bits());
        emu.write_u32(G_C7CF24, g[1].to_bits());
        emu.write_u32(G_C7CF28, g[2].to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x679702, 0x679742);
        let ours = sand_cam_global(world, cam, g[0], g[1], g[2]);
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
        let ours = sand_cam_global([pw, pw, pw], [pc, pc, pc], pg, pg, pg);
        assert_ne!(
            ours.out_x.to_bits(),
            ours.out_z.to_bits(),
            "S8 split probe must diverge (live x vs f32-reload z): x={:#010x} z={:#010x}",
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

/// `0x679754` to `0x679772`: the grain speed.
#[test]
fn sand_speed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x7370_6565_u32;

    let run = |emu: &mut Emu, rng: f32, tag: &str| {
        emu.write_u32(frame + 8, rng.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x679754, 0x679772);
        let binary = emu.read_f32(frame + 8);
        assert_field(binary, sand_speed(rng), tag, "speed");
    };

    run(&mut emu, 1.0, "rng1");
    run(&mut emu, 1.5, "mid");
    run(&mut emu, rng_unit_float(0), "lo");
    run(&mut emu, rng_unit_float(0x7fff_ffff), "hi");
    for _ in 0..300 {
        run(&mut emu, fuzz_rng(&mut s), "fuzz");
    }
}

/// `0x67979f` to `0x6797c1`: the speed times the velocity.
#[test]
fn sand_vel_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x7673_636c_u32;

    let run = |emu: &mut Emu, speed: f32, vel: [f32; 3], tag: &str| {
        emu.write_u32(frame + 8, speed.to_bits());
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
        emu.run(0x67979f, 0x6797c1);
        assert_field(
            emu.read_f32(sp + 8),
            sand_vel_scale(speed, vel[2]),
            tag,
            "v2",
        );
        assert_field(
            emu.read_f32(sp + 4),
            sand_vel_scale(speed, vel[1]),
            tag,
            "v1",
        );
        assert_field(emu.read_f32(sp), sand_vel_scale(speed, vel[0]), tag, "v0");
    };

    run(&mut emu, 3.2, [1.0, 2.0, 3.0], "basic");
    run(&mut emu, 0.0, [1.0, 1.0, 1.0], "zero-speed");
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

/// `0x6797c6` to `0x6797e8`: the scaled velocity plus the position.
#[test]
fn sand_vel_offset_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x100) + 0x80;
    let mut s = 0x766f_6666_u32;

    let run = |emu: &mut Emu, sv: [f32; 3], pos: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x30, sv[0].to_bits());
        emu.write_u32(frame - 0x34, sv[1].to_bits());
        emu.write_u32(frame - 0x38, sv[2].to_bits());
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
        emu.run(0x6797c6, 0x6797e8);
        assert_field(
            emu.read_f32(sp + 8),
            sand_vel_offset(sv[0], pos[2]),
            tag,
            "o2",
        );
        assert_field(
            emu.read_f32(sp + 4),
            sand_vel_offset(sv[1], pos[1]),
            tag,
            "o1",
        );
        assert_field(emu.read_f32(sp), sand_vel_offset(sv[2], pos[0]), tag, "o0");
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

/// `0x679817` to `0x679864`: the lifetime, with `0x4549f0` run for real.
#[test]
fn sand_lifetime_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x40);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x400) + 0x200;
    let mut s = 0x6c69_6665_u32;

    let run =
        |emu: &mut Emu, pos: [f32; 3], hit: [f32; 3], vel: [f32; 3], param3: f32, tag: &str| {
            emu.write_u32(esi, pos[0].to_bits());
            emu.write_u32(esi + 4, pos[1].to_bits());
            emu.write_u32(esi + 8, pos[2].to_bits());
            emu.write_u32(frame - 0x20, hit[0].to_bits());
            emu.write_u32(frame - 0x1c, hit[1].to_bits());
            emu.write_u32(frame - 0x18, hit[2].to_bits());
            emu.write_u32(edi, vel[0].to_bits());
            emu.write_u32(edi + 4, vel[1].to_bits());
            emu.write_u32(edi + 8, vel[2].to_bits());
            emu.write_u32(frame + 0xc, param3.to_bits());
            emu.write_u32(esi + 0x1c, 0xdead_beef);
            emu.set_reg(RegisterX86::ESI, esi);
            emu.set_reg(RegisterX86::EDI, edi);
            emu.set_reg(RegisterX86::EBP, frame);
            emu.set_reg(RegisterX86::ESP, espbase);
            emu.run(0x679817, 0x679864);
            let binary = emu.read_f32(esi + 0x1c);
            let ours = sand_lifetime(pos, hit, vel, param3);
            assert_field(binary, ours, tag, "lifetime");
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
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            fuzz(&mut s),
            "fuzz",
        );
    }
}
