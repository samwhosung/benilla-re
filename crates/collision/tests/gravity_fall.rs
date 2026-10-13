//! The gravity functions run whole, the unrounded `st0` read through `call_f64` where the callers
//! use it so. The terminal speeds sit in the globals `[0x87d894]` (land) and `[0x87d898]` (swim),
//! which the test writes. The trajectory tests run `0x7c5d20` step after step, each step's speed
//! fed back as the next one's input, on the exe and on `gravity_integrate`, and check that the
//! trajectory compare catches a drift.

use benilla_re_collision::gravity_fall::{
    fall_complete, fall_displacement, fall_displacement_ms, fall_displacement_ms_plus_base,
    fall_displacement_ms_transport, fall_distance_quick, fallvel_scale, gravity_integrate,
    gravity_over_falltime, Cm,
};
use benilla_re_difftest::{Emu, Trajectory, Wow};

/// The land terminal speed.
const G_LAND: u64 = 0x87d894;
/// The swim terminal speed.
const G_SWIM: u64 = 0x87d898;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A random `f32` in `[-4, 4)`.
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}

fn assert32(b: f32, o: f32, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        b.to_bits(),
        o.to_bits()
    );
}
fn assert64(b: f64, o: f64, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#018x} ours={:#018x}",
        b.to_bits(),
        o.to_bits()
    );
}

/// Nine functions on random movers, flags and times.
#[test]
fn gravity_fall_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x120);
    let mut s = 0xfa11_9123u32;
    let mut checked = 0u32;

    for case in 0..900u32 {
        let mut flags = next(&mut s) & 0x00ff_00ff;
        if case & 1 != 0 {
            flags |= 0x0000_2000;
        }
        if case & 2 != 0 {
            flags |= 0x2000_0000;
        }

        let fall_vel = match case % 9 {
            0 => 0.0f32,
            1 => -0.0f32,
            2 => -1.0e-7f32,
            3 => -5.0e-7f32,
            4 => fz(&mut s) * 50.0,
            5 => fz(&mut s).abs() * 80.0,
            6 => -(fz(&mut s).abs() * 80.0),
            7 => fz(&mut s),
            _ => fz(&mut s) * 12.0,
        };

        let term_land = fz(&mut s).abs() * 30.0 + 1.0;
        let term_swim = fz(&mut s).abs() * 30.0 + 1.0;

        let field_18 = fz(&mut s);
        let field_7c = fz(&mut s);

        let fall_time_ms = match case % 5 {
            0 => 0u32,
            1 => 1,
            2 => (next(&mut s) >> 10) % 120,
            3 => (next(&mut s) >> 6) % 6000,
            _ => next(&mut s) % 50_000,
        };

        let dt = match case % 6 {
            0 => 0.0f32,
            1 => fz(&mut s).abs() * 0.01,
            2 => fz(&mut s).abs() * 6.0,
            3 => -(fz(&mut s).abs()),
            _ => fz(&mut s).abs(),
        };

        let ms_arg = match case % 4 {
            0 => 0u32,
            1 => (next(&mut s) >> 8) % 200,
            2 => (next(&mut s) >> 4) % 8000,
            _ => next(&mut s) % 45_000,
        };

        let arg2 = fz(&mut s);

        let cm = Cm {
            flags,
            fall_time_ms,
            fall_vel,
            field_18,
            field_7c,
            terminal_land: term_land,
            terminal_swim: term_swim,
        };

        emu.write_u32(this + 0x18, field_18.to_bits());
        emu.write_u32(this + 0x40, flags);
        emu.write_u32(this + 0x78, fall_time_ms);
        emu.write_u32(this + 0x7c, field_7c.to_bits());
        emu.write_u32(this + 0xa0, fall_vel.to_bits());
        emu.write_u32(G_LAND, term_land.to_bits());
        emu.write_u32(G_SWIM, term_swim.to_bits());

        let ecx = this as u32;

        let b = emu.call_f32(0x7c5d20, ecx, 0, &[dt.to_bits()]);
        assert32(
            b,
            gravity_integrate(&cm, dt),
            &format!("case {case}: 0x7c5d20 gravity_integrate"),
        );

        let b = emu.call_f32(0x7c5d70, ecx, 0, &[]);
        assert32(
            b,
            gravity_over_falltime(&cm),
            &format!("case {case}: 0x7c5d70 gravity_over_falltime"),
        );

        let b = emu.call_f64(0x7c5da0, ecx, 0, &[]);
        assert64(
            b,
            fallvel_scale(&cm),
            &format!("case {case}: 0x7c5da0 fallvel_scale"),
        );

        let b = emu.call(0x7c5de0, ecx, 0, &[]);
        assert_eq!(
            (b & 0xff) != 0,
            fall_complete(&cm),
            "case {case}: 0x7c5de0 fall_complete verdict (eax={b:#x})"
        );

        let b = emu.call_f64(0x7c5e70, ecx, 0, &[dt.to_bits()]);
        assert64(
            b,
            fall_displacement(&cm, dt),
            &format!("case {case}: 0x7c5e70 fall_displacement (dt={dt})"),
        );

        let b = emu.call_f64(0x7c5f00, ecx, 0, &[ms_arg]);
        assert64(
            b,
            fall_displacement_ms(&cm, ms_arg),
            &format!("case {case}: 0x7c5f00 fall_displacement_ms (ms={ms_arg})"),
        );

        let b = emu.call_f64(0x7c5f30, ecx, 0, &[ms_arg]);
        assert64(
            b,
            fall_displacement_ms_plus_base(&cm, ms_arg),
            &format!("case {case}: 0x7c5f30 fall_displacement_ms_plus_base"),
        );

        let b = emu.call_f64(0x7c60c0, ecx, 0, &[]);
        assert64(
            b,
            fall_distance_quick(&cm),
            &format!("case {case}: 0x7c60c0 fall_distance_quick"),
        );

        let b = emu.call_f64(0x7c6110, ecx, 0, &[ms_arg, arg2.to_bits()]);
        assert64(
            b,
            fall_displacement_ms_transport(&cm, ms_arg, arg2),
            &format!("case {case}: 0x7c6110 fall_displacement_ms_transport"),
        );

        checked += 1;
    }
    eprintln!("gravity_fall: {checked} rounds bit-exact, nine functions per round");
}

/// `0x7c5d20` over long runs from rest, from below and above the terminal speed, and with one step
/// past it.
#[test]
fn gravity_integrate_trajectory() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x120);

    let configs: [(&str, u32, f32, f32, f32, f32, usize); 5] = [
        ("land ramp from 0", 0, 52.0, 40.0, 0.0, 1.0 / 60.0, 4000),
        (
            "swim ramp from 0",
            0x2000_0000,
            52.0,
            40.0,
            0.0,
            1.0 / 60.0,
            4000,
        ),
        ("ramp up from -200", 0, 52.0, 40.0, -200.0, 1.0 / 30.0, 4000),
        (
            "single-step overshoot then clamp",
            0,
            52.0,
            40.0,
            0.0,
            3.0,
            2000,
        ),
        (
            "start above terminal",
            0,
            52.0,
            40.0,
            500.0,
            1.0 / 60.0,
            2000,
        ),
    ];

    for (label, flags, tl, tsw, v0, dt, steps) in configs {
        emu.write_u32(this + 0x40, flags);
        emu.write_u32(G_LAND, tl.to_bits());
        emu.write_u32(G_SWIM, tsw.to_bits());
        let cm = Cm {
            flags,
            terminal_land: tl,
            terminal_swim: tsw,
            ..Cm::default()
        };
        let dt_bits = dt.to_bits();
        let ecx = this as u32;

        let r = emu.trajectory(
            steps,
            vec![v0.to_bits()],
            |emu, st| {
                emu.write_u32(this + 0xa0, st[0]);
                vec![emu.call_f32(0x7c5d20, ecx, 0, &[dt_bits]).to_bits()]
            },
            |st| {
                let mut c = cm;
                c.fall_vel = f32::from_bits(st[0]);
                vec![gravity_integrate(&c, dt).to_bits()]
            },
        );
        r.assert_match(&format!("gravity_integrate trajectory [{label}]"));
    }
    eprintln!(
        "gravity_integrate: closed-loop trajectory bit-exact over {} configs",
        configs.len()
    );
}

/// A drift put into the reimplementation's sixth step is reported at that step.
#[test]
fn trajectory_detects_injected_drift() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let mut kstep = 0usize;
    let r = emu.trajectory(
        20,
        vec![0],
        |_emu, st| vec![st[0] + 1],
        |st| {
            kstep += 1;
            vec![st[0] + if kstep == 6 { 2 } else { 1 }]
        },
    );
    match r {
        Trajectory::Diverged { step, bin, kern } => {
            assert_eq!(step, 5, "drift injected at step 5 must be localized there");
            assert_eq!((bin[0], kern[0]), (6, 7), "diverged states");
        }
        Trajectory::Match { .. } => panic!("trajectory FAILED to detect injected drift (vacuous)"),
    }
    eprintln!("trajectory: injected drift correctly localized to step 5 (helper is non-vacuous)");
}
