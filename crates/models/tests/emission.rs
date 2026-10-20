//! How an emitter spawns a particle (`systems/models/emission.md`): the spawn routines, the speed
//! and the setters called whole with the emitter's `CRandom` seeded through `0x44ea80`, the rest as
//! slices.
//!
//! Host double (`docs/METHOD.md#the-harness`): `emit_shape_sphere_difftest` runs `fsincos` at
//! `0x7b8e5b` and `0x7b8e76`. The emulator computes each in host `double`, so where its result
//! reaches a compared value the host's math stands in for the x87's, and the test shows the code
//! around it.

#![allow(clippy::needless_range_loop)] // index loops follow the `+off + 4·k` layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::emission::{
    burst_velocity, color_scale_apply_sum, emit_shape_plane, emit_shape_sphere, emit_shape_spline,
    emitter_speed, remap_fit, set_scalar_flush_denormal, spawn_age_offset, sphere_set_max,
    sphere_set_mid, spline_set_progress, spline_set_scale, CRandom, PlaneEmitter, RemapFit,
    SphereEmitter, SplineEmitter, FLUSH_EPS, PRNG_TABLE_LEN, PRNG_TABLE_VA,
};
use common::{assert_f32, assert_f64, frand, fuzz_f32, fuzz_scales, next, read_ah, SLICE_ESP};

/// The table `CRandom::uint32` reads, from the exe.
fn read_table(emu: &Emu) -> [u8; PRNG_TABLE_LEN] {
    let mut t = [0u8; PRNG_TABLE_LEN];
    for (i, b) in t.iter_mut().enumerate() {
        *b = emu
            .read_u32(u64::from(PRNG_TABLE_VA) + (i as u64 & !3))
            .to_le_bytes()[i & 3];
    }
    t
}

/// The LCG with a draw over four ranges up to ±50.
struct EmitRng(u32);

impl EmitRng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A "sane" mixed-magnitude finite f32 (emitter radii / pivots).
    fn f32_sane(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 4;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,
            1 => (m * 50.0) as f32,
            2 => (-(m * 50.0)) as f32,
            _ => (m * 5.0) as f32,
        }
    }
}

/// A `f32` over six ranges, one of them any bits.
fn fuzz_bits(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,       // [-1, 1)
        1 => (m * 2000.0 - 1000.0) as f32, // mid range
        2 => (m * 1.0e6) as f32,           // large
        3 => (-(m * 1.0e6)) as f32,        // large negative
        4 => (m * 1.0e-6) as f32,          // tiny
        _ => f32::from_bits(next(s)),      // raw bits (incl. NaN/Inf/denormal)
    }
}

/// 2.3841858e-7 (`0x8029d4`).
const REMAP_EPS_VA: u64 = 0x8029d4;

/// `0x7b8890` whole with a z source and flag 0x100, the emitter's state seeded through
/// `0x44ea80`: the age, start and direction read from the particle, the state stepped four times.
#[test]
fn emit_shape_plane_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    let this = emu.alloc(0x300);
    let part = emu.alloc(0x40);
    let mut lcg = EmitRng(0x10ca_1e55);

    for _ in 0..300u32 {
        // zero the emitter region, then set the plane fields + a nonzero z_pivot (radial branch).
        for i in 0..0xc0u64 {
            emu.write_u32(this + i * 4, 0);
        }
        let e = PlaneEmitter {
            half_x: lcg.f32_sane(),
            half_y: lcg.f32_sane(),
            spread_a: 0.0,
            spread_b: 0.0,
            // z_pivot must be nonzero AND make |temp| nonzero; pick a sane nonzero.
            z_pivot: {
                let z = lcg.f32_sane();
                if z == 0.0 {
                    1.0
                } else {
                    z
                }
            },
            flags: 0x100, // local-frame: param_2[4..6] = the computed direction; no 0x400 burst.
        };
        let base_speed = lcg.f32_sane().abs() + 0.5; // this+0x17c
        let variation = lcg.f32_sane() % 1.0; // this+0x184
        let p3 = (lcg.next() % 200) as f32 * 0.01;
        let seed = lcg.next();

        emu.write_u32(this + 0x290, e.half_x.to_bits());
        emu.write_u32(this + 0x294, e.half_y.to_bits());
        emu.write_u32(this + 0x188, e.z_pivot.to_bits());
        emu.write_u32(this + 0x1ac, e.flags);
        emu.write_u32(this + 0x17c, base_speed.to_bits());
        emu.write_u32(this + 0x184, variation.to_bits());
        // seed the embedded CRandom at +0x2c/+0x30 via 0x44ea80 (state lives there).
        emu.call(0x44ea80, (this + 0x2c) as u32, 0, &[seed]);
        let s0 = emu.read_u32(this + 0x2c);
        let s1 = emu.read_u32(this + 0x30);
        let mut rng = CRandom::from_state([s0, s1], table);

        // zero the particle record.
        for i in 0..0x10u64 {
            emu.write_u32(part + i * 4, 0);
        }
        // __thiscall: ecx=this; stack = param_2(record), param_3(p3), param_4(runtime; unused on
        // the 0x100 path). ret? — generator is `void`; we don't depend on stack cleanup.
        emu.call(0x7b8890, this as u32, 0, &[part as u32, p3.to_bits(), 0]);

        // our lockstep draws: draw0(age), draw1, draw2, then emitter_speed (1 step).
        let draw0 = rng.next_u32();
        let draw1 = rng.next_u32();
        let draw2 = rng.next_u32();
        let speed = emitter_speed(&mut rng, base_speed, variation) as f32;
        let age = spawn_age_offset(draw0, p3);
        let spawn = emit_shape_plane(&e, draw1, draw2, speed);

        // param_2[7] (offset +0x1c) = spawn-age field.
        assert_f32(emu.read_f32(part + 0x1c), age, "plane spawn-age");
        // param_2[0..2] = velocity.
        for k in 0..3 {
            assert_f32(
                emu.read_f32(part + k as u64 * 4),
                spawn.vel[k],
                &format!("plane vel[{k}]"),
            );
        }
        // param_2[4..6] = radial direction (flags & 0x100).
        for k in 0..3 {
            assert_f32(
                emu.read_f32(part + (4 + k) as u64 * 4),
                spawn.radial_dir[k],
                &format!("plane dir[{k}]"),
            );
        }
        // param_2[3] byte (offset +0xc) = 0.
        assert_eq!(emu.read_u32(part + 0xc) & 0xff, 0, "plane byte[3]");
        // The embedded PRNG state must have advanced exactly 4 steps (3 draws + 1 emitter_speed).
        assert_eq!(emu.read_u32(this + 0x2c), rng.state[0], "plane prng s0");
        assert_eq!(emu.read_u32(this + 0x30), rng.state[1], "plane prng s1");
    }
}

/// `0x7b8d70` whole: the age read from the particle; the direction is `fsincos`.
#[test]
fn emit_shape_sphere_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    let this = emu.alloc(0x300);
    let part = emu.alloc(0x40);
    let mut lcg = EmitRng(0x5e7a_1100);
    for _ in 0..200u32 {
        for i in 0..0xc0u64 {
            emu.write_u32(this + i * 4, 0);
        }
        let e = SphereEmitter {
            radius_mid: lcg.f32_sane(),
            radius_span: lcg.f32_sane(),
            angle_a: lcg.f32_sane(),
            angle_b: lcg.f32_sane(),
            z_pivot: 1.0, // nonzero: normalize branch (still trig velocity); doesn't affect spawn-age
            flags: 0x100,
        };
        let p3 = (lcg.next() % 200) as f32 * 0.01;
        let seed = lcg.next();
        emu.write_u32(this + 0x290, e.radius_mid.to_bits());
        emu.write_u32(this + 0x298, e.radius_span.to_bits());
        emu.write_u32(this + 0x29c, e.angle_a.to_bits());
        emu.write_u32(this + 0x2a0, e.angle_b.to_bits());
        emu.write_u32(this + 0x188, e.z_pivot.to_bits());
        emu.write_u32(this + 0x1ac, e.flags);
        emu.write_u32(this + 0x17c, 1.0f32.to_bits());
        emu.write_u32(this + 0x184, 0.0f32.to_bits());
        emu.call(0x44ea80, (this + 0x2c) as u32, 0, &[seed]);
        let mut rng = CRandom::from_state(
            [emu.read_u32(this + 0x2c), emu.read_u32(this + 0x30)],
            table,
        );
        for i in 0..0x10u64 {
            emu.write_u32(part + i * 4, 0);
        }
        emu.call(0x7b8d70, this as u32, 0, &[part as u32, p3.to_bits(), 0]);

        let draw0 = rng.next_u32();
        let age = spawn_age_offset(draw0, p3);
        // The scalars before the sine and cosine take draw1, draw2 and draw3 (radius, theta, phi);
        // the velocity through `fsincos` is not compared.
        let draw1 = rng.next_u32();
        let draw2 = rng.next_u32();
        let draw3 = rng.next_u32();
        let _scalars = emit_shape_sphere(&e, draw1, draw2, draw3);

        assert_f32(emu.read_f32(part + 0x1c), age, "sphere spawn-age");
        assert_eq!(emu.read_u32(part + 0xc) & 0xff, 0, "sphere byte[3]");
    }
}

/// `0x7b9500` whole, `0x453390` stubbed, with `+0x2a4` set and clear: the age and the cleared
/// flag.
#[test]
fn emit_shape_spline_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    let this = emu.alloc(0x400);
    let part = emu.alloc(0x40);
    let mut lcg = EmitRng(0x5b11_7e00);
    let (mut dirty_n, mut clean_n) = (0u32, 0u32);

    // The spline t-eval 0x453390 reads the knot array at this+0x2a8; stub it so it does not fault
    // and consumes its 3 stack args (__thiscall(ecx=this+0x2a8, t, out, 1) => ret? we stub it as a
    // no-op).
    emu.stub(0x453390, 0, 0);

    for iter in 0..200u32 {
        for i in 0..0x110u64 {
            emu.write_u32(this + i * 4, 0);
        }
        let dirty = iter % 2 == 0;
        let e = SplineEmitter {
            dirty: u32::from(dirty),
            forced: (lcg.next() % 100) as f32 * 0.01,
            range_lo: (lcg.next() % 100) as f32 * 0.01,
        };
        // hi = forced (+0x298); lo = range_lo (+0x294).
        let p3 = (lcg.next() % 200) as f32 * 0.01;
        let seed = lcg.next();
        emu.write_u32(this + 0x2a4, e.dirty);
        emu.write_u32(this + 0x298, e.forced.to_bits());
        emu.write_u32(this + 0x294, e.range_lo.to_bits());
        emu.write_u32(this + 0x188, 1.0f32.to_bits()); // nonzero z_pivot: the normalize branch
        emu.write_u32(this + 0x1ac, 0x100);
        emu.write_u32(this + 0x17c, 1.0f32.to_bits());
        emu.write_u32(this + 0x184, 0.0f32.to_bits());
        emu.call(0x44ea80, (this + 0x2c) as u32, 0, &[seed]);
        let mut rng = CRandom::from_state(
            [emu.read_u32(this + 0x2c), emu.read_u32(this + 0x30)],
            table,
        );
        for i in 0..0x10u64 {
            emu.write_u32(part + i * 4, 0);
        }
        emu.call(0x7b9500, this as u32, 0, &[part as u32, p3.to_bits(), 0]);

        let draw0 = rng.next_u32();
        let age = spawn_age_offset(draw0, p3);
        // dirty branch: NO second draw; clean branch: one draw for t.
        let (_t, dirty_cleared) = if dirty {
            emit_shape_spline(&e, 0)
        } else {
            let d1 = rng.next_u32();
            emit_shape_spline(&e, d1)
        };

        assert_f32(emu.read_f32(part + 0x1c), age, "spline spawn-age");
        assert_eq!(emu.read_u32(part + 0xc) & 0xff, 0, "spline byte[3]");
        // dirty flag cleared by the binary on the dirty branch.
        if dirty {
            assert!(dirty_cleared, "ours cleared the dirty flag");
            assert_eq!(emu.read_u32(this + 0x2a4), 0, "binary cleared dirty");
            dirty_n += 1;
        } else {
            assert_eq!(emu.read_u32(this + 0x2a4), 0, "dirty stays 0");
            clean_n += 1;
        }
    }
    assert!(dirty_n > 0 && clean_n > 0, "both spline branches fire");
}

/// `0x7b1cc0` whole after `0x44ea80`: the speed as the `f64` it leaves, the state stepped once.
#[test]
fn emitter_speed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    let this = emu.alloc(0x300);
    let mut lcg = EmitRng(0x7b1c_c001);
    for _ in 0..300u32 {
        for i in 0..0xc0u64 {
            emu.write_u32(this + i * 4, 0);
        }
        let base = lcg.f32_sane();
        let variation = lcg.f32_sane();
        let seed = lcg.next();
        emu.write_u32(this + 0x17c, base.to_bits());
        emu.write_u32(this + 0x184, variation.to_bits());
        emu.call(0x44ea80, (this + 0x2c) as u32, 0, &[seed]);
        let mut rng = CRandom::from_state(
            [emu.read_u32(this + 0x2c), emu.read_u32(this + 0x30)],
            table,
        );
        // __fastcall/__thiscall(ecx=this) returning a float in st0.
        let bspeed = emu.call_f64(0x7b1cc0, this as u32, 0, &[]);
        let ours = emitter_speed(&mut rng, base, variation);
        assert_f64(bspeed, ours, "emitter_speed");
        // state advanced exactly one step.
        assert_eq!(
            emu.read_u32(this + 0x2c),
            rng.state[0],
            "emitter_speed prng s0"
        );
        assert_eq!(
            emu.read_u32(this + 0x30),
            rng.state[1],
            "emitter_speed prng s1"
        );
    }
}

/// `0x7b8890` whole with flags 0x100 and 0x400: the direction with the burst added.
#[test]
fn burst_velocity_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    let this = emu.alloc(0x300);
    let part = emu.alloc(0x40);
    let mut lcg = EmitRng(0x4001_b115);
    for _ in 0..200u32 {
        for i in 0..0xc0u64 {
            emu.write_u32(this + i * 4, 0);
        }
        let e = PlaneEmitter {
            half_x: lcg.f32_sane(),
            half_y: lcg.f32_sane(),
            spread_a: 0.0,
            spread_b: 0.0,
            z_pivot: {
                let z = lcg.f32_sane();
                if z == 0.0 {
                    1.0
                } else {
                    z
                }
            },
            flags: 0x100 | 0x400,
        };
        let base_speed = lcg.f32_sane().abs() + 0.5;
        let variation = lcg.f32_sane() % 1.0;
        let burst_dir = [lcg.f32_sane(), lcg.f32_sane(), lcg.f32_sane()];
        let p3 = (lcg.next() % 200) as f32 * 0.01;
        let seed = lcg.next();

        emu.write_u32(this + 0x290, e.half_x.to_bits());
        emu.write_u32(this + 0x294, e.half_y.to_bits());
        emu.write_u32(this + 0x188, e.z_pivot.to_bits());
        emu.write_u32(this + 0x1ac, e.flags);
        emu.write_u32(this + 0x17c, base_speed.to_bits());
        emu.write_u32(this + 0x184, variation.to_bits());
        // burst direction at +0x258/+0x25c/+0x260.
        emu.write_u32(this + 0x258, burst_dir[0].to_bits());
        emu.write_u32(this + 0x25c, burst_dir[1].to_bits());
        emu.write_u32(this + 0x260, burst_dir[2].to_bits());
        emu.call(0x44ea80, (this + 0x2c) as u32, 0, &[seed]);
        let mut rng = CRandom::from_state(
            [emu.read_u32(this + 0x2c), emu.read_u32(this + 0x30)],
            table,
        );
        for i in 0..0x10u64 {
            emu.write_u32(part + i * 4, 0);
        }
        emu.call(0x7b8890, this as u32, 0, &[part as u32, p3.to_bits(), 0]);

        let draw0 = rng.next_u32();
        let draw1 = rng.next_u32();
        let draw2 = rng.next_u32();
        let speed = emitter_speed(&mut rng, base_speed, variation) as f32;
        let _age = spawn_age_offset(draw0, p3);
        let spawn = emit_shape_plane(&e, draw1, draw2, speed);
        // the 0x400 burst draw (after emitter_speed); applied in place to the radial dir.
        let dburst = rng.next_u32();
        let final_dir = burst_velocity(spawn.radial_dir, dburst, variation, burst_dir);
        for k in 0..3 {
            assert_f32(
                emu.read_f32(part + (4 + k) as u64 * 4),
                final_dir[k],
                &format!("burst dir[{k}]"),
            );
        }
    }
}

/// `0x7b9180` whole: `+0x290` and `+0x298` read back.
#[test]
fn sphere_set_mid_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x300);

    let run_case = |e: &mut Emu, max: f32, mid: f32, ctx: &str| {
        e.write_u32(this + 0x294, max.to_bits());
        e.write_u32(this + 0x290, 0xdead_beef); // junk so a non-write diverges
        e.write_u32(this + 0x298, 0xdead_beef);
        let _ = e.call(0x7b9180, this as u32, 0, &[mid.to_bits()]);
        let bin_mid = e.read_f32(this + 0x290);
        let bin_span = e.read_f32(this + 0x298);
        let ours = sphere_set_mid(max, mid);
        assert_f32(bin_mid, ours.mid, &format!("{ctx} mid"));
        assert_f32(bin_span, ours.half_span, &format!("{ctx} half_span"));
    };

    // ==threshold / edge probes: equal (span 0), ±0, exact subtractions, negatives, big spread.
    run_case(&mut e, 5.0, 5.0, "sphere_mid eq (span 0)");
    run_case(&mut e, 0.0, 0.0, "sphere_mid zero");
    run_case(&mut e, 1.0, -1.0, "sphere_mid 1,-1 (span 2)");
    run_case(&mut e, -1.0, 1.0, "sphere_mid -1,1 (span -2)");
    run_case(&mut e, 1.0e30, 1.0e-30, "sphere_mid huge,tiny");
    run_case(
        &mut e,
        f32::from_bits(0x0000_0001),
        0.0,
        "sphere_mid subnormal max",
    );

    let mut s = 0x7b91_8080u32;
    for i in 0..3000u32 {
        let max = fuzz_f32(&mut s);
        let mid = fuzz_f32(&mut s);
        run_case(&mut e, max, mid, &format!("sphere_mid fuzz[{i}]"));
    }
}

/// `0x7b91a0` whole: `+0x294` and `+0x298` read back.
#[test]
fn sphere_set_max_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x300);

    let run_case = |e: &mut Emu, mid: f32, max: f32, ctx: &str| {
        e.write_u32(this + 0x290, mid.to_bits());
        e.write_u32(this + 0x294, 0xdead_beef);
        e.write_u32(this + 0x298, 0xdead_beef);
        let _ = e.call(0x7b91a0, this as u32, 0, &[max.to_bits()]);
        let bin_max = e.read_f32(this + 0x294);
        let bin_span = e.read_f32(this + 0x298);
        let ours = sphere_set_max(mid, max);
        assert_f32(bin_max, ours.max, &format!("{ctx} max"));
        assert_f32(bin_span, ours.half_span, &format!("{ctx} half_span"));
    };

    run_case(&mut e, 5.0, 5.0, "sphere_max eq (span 0)");
    run_case(&mut e, 0.0, 0.0, "sphere_max zero");
    run_case(&mut e, -1.0, 1.0, "sphere_max -1,1 (span 2)");
    run_case(&mut e, 1.0, -1.0, "sphere_max 1,-1 (span -2)");
    run_case(&mut e, 1.0e-30, 1.0e30, "sphere_max tiny,huge");
    run_case(
        &mut e,
        0.0,
        f32::from_bits(0x0000_0001),
        "sphere_max subnormal max",
    );

    let mut s = 0x7b91_a0a0u32;
    for i in 0..3000u32 {
        let mid = fuzz_f32(&mut s);
        let max = fuzz_f32(&mut s);
        run_case(&mut e, mid, max, &format!("sphere_max fuzz[{i}]"));
    }
}

/// `0x7b99c0` whole: the clamp at 0 and 1, the change at 2.3841858e-7, and NaN; `+0x298`,
/// `+0x2a4` and `+0xa8` read back.
#[test]
fn spline_set_progress_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x300);

    // junk sentinels for the dirty-write slots (so the skip leg is observable).
    const J298: u32 = 0x1234_5678;
    const J2A4: u32 = 0x9abc_def0;
    const JA8: u32 = 0x0f1e_2d3c;

    let run_case = |e: &mut Emu, arg: f32, old: f32, mul: f32, ctx: &str| {
        e.write_u32(this + 0x298, J298);
        e.write_u32(this + 0x290, mul.to_bits());
        e.write_u32(this + 0x2a4, J2A4);
        e.write_u32(this + 0xa8, JA8);
        // `old` is read from +0x298 — but the setter reads it, then (on dirty) overwrites it. Seed
        // the genuine old value into +0x298 (overriding the junk above) so the eps gate sees it.
        e.write_u32(this + 0x298, old.to_bits());
        let _ = e.call(0x7b99c0, this as u32, 0, &[arg.to_bits()]);
        let bin_flag = e.read_u32(this + 0x2a4);
        let ours = spline_set_progress(arg, old, mul);
        if ours.dirty {
            assert_eq!(bin_flag, 1, "{ctx} dirty flag (+0x2a4)");
            assert_f32(
                e.read_f32(this + 0x298),
                ours.recompute,
                &format!("{ctx} recompute (+0x298)"),
            );
            assert_f32(
                e.read_f32(this + 0xa8),
                ours.derived,
                &format!("{ctx} derived (+0xa8)"),
            );
        } else {
            // skip leg: the flag and the +0xa8 slot were never written (still junk); +0x298 holds
            // the old value we seeded (also never rewritten).
            assert_eq!(bin_flag, J2A4, "{ctx} clean flag untouched (+0x2a4)");
            assert_eq!(e.read_u32(this + 0xa8), JA8, "{ctx} clean +0xa8 untouched");
            assert_eq!(
                e.read_u32(this + 0x298),
                old.to_bits(),
                "{ctx} clean +0x298 == seeded old"
            );
        }
    };

    let eps = f32::from_bits(0x3480_0000); // 2.384185791015625e-07

    // ==threshold probes around the clamp bounds and the eps gate.
    run_case(&mut e, 0.5, 0.0, 2.0, "spline mid arg");
    run_case(&mut e, -0.3, 0.0, 1.0, "spline arg<0 → 0");
    run_case(&mut e, 1.7, 0.0, 1.0, "spline arg>1 → 1");
    run_case(
        &mut e,
        0.0,
        0.0,
        1.0,
        "spline arg=0 old=0 (|diff|=0 < ε, clean)",
    );
    run_case(&mut e, 1.0, 1.0, 1.0, "spline arg=1 old=1 (clean)");
    run_case(&mut e, 0.5, 0.5, 1.0, "spline arg=old (|diff|=0, clean)");
    // |clamped − old| exactly ε (boundary: ≥ ε → dirty).
    run_case(&mut e, eps, 0.0, 1.0, "spline |diff|==ε (dirty)");
    // |clamped − old| just under ε.
    run_case(
        &mut e,
        f32::from_bits(eps.to_bits() - 1),
        0.0,
        1.0,
        "spline |diff|<ε (clean)",
    );
    // NaN arg → survives low clamp, becomes 1.0 at high clamp.
    run_case(&mut e, f32::NAN, 0.0, 3.0, "spline NaN arg → 1.0");
    run_case(
        &mut e,
        f32::NAN,
        1.0,
        3.0,
        "spline NaN arg, old=1 (|diff|=0 clean)",
    );
    // negative zero arg (−0.0 is NOT < 0.0).
    run_case(&mut e, -0.0, 0.5, 1.0, "spline -0.0 arg");
    // old is NaN → |clamped − NaN| = NaN → unordered → dirty.
    run_case(
        &mut e,
        0.5,
        f32::NAN,
        2.0,
        "spline old=NaN (unordered → dirty)",
    );
    // mul edge: mul=0 → derived 0; mul huge.
    run_case(&mut e, 0.5, 0.0, 0.0, "spline mul=0 → derived 0");
    run_case(&mut e, 0.5, 0.0, 1.0e30, "spline mul huge");

    let mut s = 0x7b99_c0c0u32;
    for i in 0..4000u32 {
        // bias args toward [0,1] (so the clamp branches and the eps gate both exercise), but also
        // let out-of-range and wild values through.
        let arg = match next(&mut s) >> 30 {
            0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32, // [0,1)
            1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 1.2 - 0.1) as f32, // [-0.1,1.1)
            _ => fuzz_f32(&mut s),
        };
        // old near the clamped band so |diff| straddles ε often; plus wide fuzz.
        let old = if next(&mut s) & 1 == 0 {
            arg + (fuzz_f32(&mut s) * 1.0e-6)
        } else {
            fuzz_f32(&mut s)
        };
        let mul = fuzz_f32(&mut s);
        run_case(&mut e, arg, old, mul, &format!("spline fuzz[{i}]"));
    }
}

/// `0x7b9a60` whole: `+0x290` and `+0xa8` read back.
#[test]
fn spline_set_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x400); // ECX → object; +0x298 read, +0x290/+0xa8 written
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, scale: f32, field_298: f32, ctx: &str| {
        e.write_u32(obj + 0x298, field_298.to_bits()); // [ecx+0x298] = the fmul operand
        e.write_u32(obj + 0x290, 0xdead_beef); // junk-seed (the raw-bits store target)
        e.write_u32(obj + 0xa8, 0xdead_beef); // junk-seed (the product target)
        e.write_u32(ebp + 0x8, scale.to_bits()); // [ebp+8] = scale (fld + mov source)
        e.set_reg(RegisterX86::ECX, obj);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // run(0x7b9a63,0x7b9a7b): fld [ebp+8]; mov eax; fmul [ecx+0x298]; mov [ecx+0x290]; fstp
        // [ecx+0xa8].
        e.run(0x7b9a63, 0x7b9a7b);
        let bin = e.read_f32(obj + 0xa8); // the product +0xa8
        let scale_stored = e.read_u32(obj + 0x290); // +0x290 := scale raw bits (sanity)
        assert_eq!(
            scale_stored,
            scale.to_bits(),
            "{ctx}: +0x290 raw-bits store binary={scale_stored:#010x} expected={:#010x}",
            scale.to_bits()
        );
        let ours = spline_set_scale(scale, field_298);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 1.0, 1.0, "1·1");
    run_case(&mut e, 2.0, 0.5, "2·0.5");
    run_case(&mut e, -3.0, 4.0, "neg");
    run_case(&mut e, 1.0e20, 1.0e20, "overflow → inf");
    run_case(&mut e, f32::NAN, 1.0, "NaN");
    run_case(&mut e, 0.0, 1.0e30, "0·huge");

    let mut s = 0x7b9a_6060u32;
    for i in 0..5000u32 {
        let scale = fuzz_bits(&mut s);
        let field_298 = fuzz_bits(&mut s);
        run_case(&mut e, scale, field_298, &format!("setscale fuzz[{i}]"));
    }
}

/// `0x7b2140` from `0x7b219c` to `0x7b21a5`: the size at `[ebp-0x4]`, the ground at `[ebp-0x8]`;
/// read from `esi+0x8`.
#[test]
fn color_scale_apply_sum_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let particle = e.alloc(0x40); // ESI → the particle; result f32 at +8
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, local_c: f32, local_8: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, local_c.to_bits()); // [ebp-4] = local_c (the fld source)
        e.write_u32(ebp - 0x8, local_8.to_bits()); // [ebp-8] = local_8 (the fadd source)
        e.write_u32(particle + 0x8, 0xdead_beef); // junk-seed so a non-write would diverge
        e.set_reg(RegisterX86::ESI, particle);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // run(0x7b219c,0x7b21a5): fld [ebp-4]; fadd [ebp-8]; fstp [esi+8]. PAST the store.
        e.run(0x7b219c, 0x7b21a5);
        let bin = e.read_f32(particle + 0x8);
        let ours = color_scale_apply_sum(local_c, local_8);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0.0, 0.0, "zero+zero");
    run_case(&mut e, 1.0, 2.0, "1+2");
    run_case(&mut e, -3.5, 0.25, "mixed sign");
    run_case(&mut e, 1.0e30, 1.0e30, "overflow → inf");
    run_case(&mut e, f32::NAN, 1.0, "NaN + 1");

    let mut s = 0x7b21_4040u32;
    for i in 0..5000u32 {
        let local_c = fuzz_bits(&mut s);
        let local_8 = fuzz_bits(&mut s);
        run_case(&mut e, local_c, local_8, &format!("colorsum fuzz[{i}]"));
    }
}

/// `0x7b5d30` whole: `+0x1f8` and `+0x1f4` read back.
#[test]
fn remap_fit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);

    // The threshold read from the exe.
    let eps = e.read_f32(REMAP_EPS_VA);

    let this = e.alloc(0x200); // CRibbonEmitter (slope @ +0x1f8, intercept @ +0x1f4)

    let run_case = |e: &mut Emu, x0: f32, y0: f32, x1: f32, y1: f32, ctx: &str| {
        e.write_u32(this + 0x1f8, 0xdead_beef);
        e.write_u32(this + 0x1f4, 0xdead_beef);
        // __thiscall: ecx = this, stack args = (x0, y0, x1, y1). ret 0x10.
        e.call(
            0x7b5d30,
            this as u32,
            0,
            &[x0.to_bits(), y0.to_bits(), x1.to_bits(), y1.to_bits()],
        );
        let bin = RemapFit {
            slope: e.read_f32(this + 0x1f8),
            intercept: e.read_f32(this + 0x1f4),
        };
        let ours = remap_fit(x0, y0, x1, y1, eps);
        assert_f32(bin.slope, ours.slope, &format!("{ctx} slope (+0x1f8)"));
        assert_f32(
            bin.intercept,
            ours.intercept,
            &format!("{ctx} intercept (+0x1f4)"),
        );
    };

    // ==threshold / edge probes: dx==0 (|dx|<eps → zero path), dx exactly eps (slope path — `>=`),
    // dx just below eps (zero path), unit slope, NaN dx (slope path → NaN out).
    run_case(&mut e, 5.0, 1.0, 5.0, 9.0, "remap dx==0 (zero path)");
    run_case(&mut e, 0.0, 0.0, eps, 1.0, "remap dx==eps (slope path)");
    run_case(
        &mut e,
        0.0,
        0.0,
        f32::from_bits(eps.to_bits() - 1),
        1.0,
        "remap dx just-below-eps (zero path)",
    );
    run_case(&mut e, 0.0, 0.0, 1.0, 1.0, "remap unit (slope=1)");
    run_case(&mut e, 2.0, 3.0, 6.0, 11.0, "remap slope=2 int=-1");
    run_case(&mut e, 1.0, 2.0, f32::NAN, 4.0, "remap dx=NaN (slope path)");
    run_case(&mut e, -10.0, 5.0, 10.0, -5.0, "remap neg slope");

    let mut s = 0x7b5d_3030u32;
    for i in 0..6000u32 {
        // mix tiny dx (straddle eps → both branches), and ordinary spreads.
        let x0 = frand(&mut s, -100.0, 100.0);
        let x1 = match (next(&mut s) >> 8) % 4 {
            0 => x0 + frand(&mut s, -1.0e-4, 1.0e-4), // tiny dx — straddles eps
            1 => x0,                                  // dx==0 exactly (zero path)
            _ => frand(&mut s, -100.0, 100.0),        // ordinary
        };
        let y0 = frand(&mut s, -100.0, 100.0);
        let y1 = frand(&mut s, -100.0, 100.0);
        run_case(&mut e, x0, y0, x1, y1, &format!("remap fuzz[{i}]"));
    }
}

/// `0x7b5d30` from `0x7b5d33` to `0x7b5d45`: `ah` after the compare of `|s1 − s0|` with
/// 2.3841858e-7, at, either side of it, and NaN.
#[test]
fn remap_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);
    let eps = e.read_f32(REMAP_EPS_VA);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, x0: f32, x1: f32, ctx: &str| {
        e.write_u32(ebp + 8, x0.to_bits()); // [ebp+8] = x0
        e.write_u32(ebp + 0x10, x1.to_bits()); // [ebp+0x10] = x1
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b5d33, 0x7b5d45); // fld[ebp+0x10]; fsub[ebp+8]; fld st; fabs; fcomp[0x8029d4]; fnstsw
        let ah = read_ah(e);
        // `test ah,0x5; jnp` taken (ZERO path) ⇔ parity of (ah & 5) ODD.
        let bin_zero = !(ah & 0x5).count_ones().is_multiple_of(2);
        // value model: zero path iff ordered AND |dx| < eps. NaN → slope path (not zero).
        let dx = f64::from(x1) - f64::from(x0);
        let ours_zero = dx.abs() < f64::from(eps); // false for NaN → slope path (matches)
        assert_eq!(
            bin_zero, ours_zero,
            "{ctx}: binary zero-path={bin_zero} ours={ours_zero} (x0={x0} x1={x1} dx={dx} ah={ah:#04x})"
        );
    };

    // ==eps boundary: |dx|==eps → SLOPE path (>=); just-below → ZERO; just-above → SLOPE.
    run_case(&mut e, 0.0, eps, "gate |dx|==eps (slope)");
    run_case(
        &mut e,
        0.0,
        f32::from_bits(eps.to_bits() - 1),
        "gate |dx| just-below-eps (zero)",
    );
    run_case(
        &mut e,
        0.0,
        f32::from_bits(eps.to_bits() + 1),
        "gate |dx| just-above-eps (slope)",
    );
    run_case(&mut e, 5.0, 5.0, "gate dx==0 (zero)");
    run_case(&mut e, 0.0, -eps, "gate dx==-eps (slope, fabs)");
    run_case(&mut e, 1.0, 100.0, "gate large dx (slope)");
    run_case(&mut e, 1.0, f32::NAN, "gate dx=NaN (slope)");

    let mut s = 0x7b5d_3333u32;
    for i in 0..4000u32 {
        let x0 = frand(&mut s, -100.0, 100.0);
        let x1 = match (next(&mut s) >> 8) % 3 {
            0 => x0 + frand(&mut s, -1.0e-4, 1.0e-4), // tiny dx — straddles eps
            1 => x0,                                  // dx==0
            _ => frand(&mut s, -100.0, 100.0),
        };
        run_case(&mut e, x0, x1, &format!("gate fuzz[{i}]"));
    }
}

/// `0x7b1cc0` whole from states written directly, sign bits and spreads at their extremes, and
/// fuzz: the speed read as the `f64` it leaves, the state read back.
#[test]
fn emitter_speed_states_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    let emitter = emu.alloc(0x300);
    let structured: &[(u32, u32, f32, f32)] = &[
        (0x0000_0000, 0x0000_0000, 1.0, 1.0),
        (0xffff_ffff, 0x1234_5678, 1.0, 1.0),
        (0x8000_0000, 0x89ab_cdef, 0.5, 2.0),
        (0x7fff_ffff, 0xdead_beef, 0.25, 4.0),
        (0x1357_9bdf, 0x0246_8ace, 1.0, 0.0),
        (0xcafe_babe, 0xfeed_face, -1.0, -1.0),
        (0x0000_0001, 0x0000_0001, f32::NAN, 1.0),
    ];
    let mut s = 0x7b1c_c001u32;
    let fuzz: Vec<(u32, u32, f32, f32)> = (0..400)
        .map(|_| {
            (
                next(&mut s),
                next(&mut s),
                fuzz_f32(&mut s),
                fuzz_f32(&mut s),
            )
        })
        .collect();
    for (acc, state, spread, scale) in structured.iter().copied().chain(fuzz) {
        emu.write_u32(emitter + 0x2c, acc);
        emu.write_u32(emitter + 0x30, state);
        emu.write_u32(emitter + 0x184, spread.to_bits());
        emu.write_u32(emitter + 0x17c, scale.to_bits());
        let bin = emu.call_f64(0x7b1cc0, emitter as u32, 0, &[]);
        let mut rng = CRandom::from_state([acc, state], table);
        let ours = emitter_speed(&mut rng, scale, spread);
        let ctx = format!("acc={acc:#x} state={state:#x}");
        assert_f64(bin, ours, &ctx);
        assert_eq!(emu.read_u32(emitter + 0x2c), rng.state[0], "{ctx} acc");
        assert_eq!(emu.read_u32(emitter + 0x30), rng.state[1], "{ctx} state");
    }
}

/// `0x7b5cd3..0x7b5cf8`: the z source stored, flushed under 0.001; the threshold, its neighbours,
/// zeros, NaN and infinities, then a fuzz about the threshold.
#[test]
fn set_scalar_flush_denormal_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x200); // ECX → the object; scalar at +0x188
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, x: f32, ctx: &str| {
        e.write_u32(ebp + 8, x.to_bits()); // [ebp+8] = x (the fld/mov source)
        e.write_u32(obj + 0x188, 0xdead_beef); // junk-seed so a non-write would diverge
        e.set_reg(RegisterX86::ECX, obj);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // run(0x7b5cd3,0x7b5cf8): fld; fabs; store x; fcomp eps; fnstsw; test ah,5; jp; (flush 0).
        // jp@7b5cec is internal to the slice. Halt at 0x7b5cf8 (pop ebp), PAST both stores.
        e.run(0x7b5cd3, 0x7b5cf8);
        let bin = e.read_f32(obj + 0x188);
        let ours = set_scalar_flush_denormal(x);
        assert_f32(bin, ours, ctx);
    };

    // ==threshold / boundary probes around the STRICT `|x| < eps` flush.
    let eps = FLUSH_EPS;
    run_case(&mut e, eps, "x == +eps (KEEP — not <)"); // |x|==eps → keep x
    run_case(&mut e, -eps, "x == -eps (KEEP)");
    run_case(
        &mut e,
        f32::from_bits(eps.to_bits() - 1),
        "x just below +eps (FLUSH → 0)",
    ); // |x|<eps → 0
    run_case(
        &mut e,
        f32::from_bits(eps.to_bits() + 1),
        "x just above +eps (KEEP)",
    ); // |x|>eps → keep
    run_case(&mut e, 0.0, "x == +0 (FLUSH → 0, already 0)");
    run_case(&mut e, -0.0, "x == -0 (FLUSH → +0)"); // |−0|<eps → store 0.0 (positive)
    run_case(&mut e, 1.0, "x == 1 (KEEP)");
    run_case(&mut e, -5.0, "x == -5 (KEEP)");
    run_case(&mut e, 1.0e-4, "x 1e-4 (< eps, FLUSH)");
    run_case(&mut e, 1.0e-3, "x 1e-3 ≈ eps region");
    run_case(&mut e, 2.0e-3, "x 2e-3 (> eps, KEEP)");
    run_case(&mut e, f32::from_bits(1), "x smallest denormal (FLUSH)");
    run_case(
        &mut e,
        f32::from_bits(0x0080_0000),
        "x smallest normal (FLUSH)",
    );
    run_case(&mut e, f32::NAN, "x NaN (KEEP — jp taken)"); // unordered → keep x (bit-exact NaN)
    run_case(&mut e, f32::INFINITY, "x +Inf (KEEP)");
    run_case(&mut e, f32::NEG_INFINITY, "x -Inf (KEEP)");

    let mut s = 0x7b5c_d000u32;
    for i in 0..6000u32 {
        // bias toward the gate band so both legs are well-exercised.
        let x = match (next(&mut s) >> 8) % 4 {
            0 => {
                // straddle eps tightly: eps ± a small f32 step, both signs.
                let sgn = if next(&mut s) & 0x100 == 0 { 1.0 } else { -1.0 };
                let step = (next(&mut s) % 32) as i32 - 16;
                sgn * f32::from_bits((eps.to_bits() as i32 + step) as u32)
            }
            1 => fuzz_scales(&mut s),
            2 => {
                // sub-eps magnitudes (flush leg) + denormals.
                let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
                let sgn = if next(&mut s) & 0x100 == 0 { 1.0 } else { -1.0 };
                (sgn * (m * 2.0e-3)) as f32
            }
            _ => f32::from_bits(next(&mut s)), // raw bits (incl. NaN/Inf/denormal)
        };
        run_case(&mut e, x, &format!("flush fuzz[{i}] x={x}"));
    }
}
