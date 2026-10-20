//! `CMovement::GetTurnRate` (`0x7c5c50`) and `CMovement::GetPitchRate` (`0x7c5c90`) run whole
//! through `call_f64`; `CMovement::LinearDisplacement` (`0x7c4ed0`), `CMovement::IntegrateFacing`
//! (`0x7c4f30`), `CMovement::IntegratePitch` (`0x7c4f80`) and `CMovement::TurnArcDisplacement`
//! (`0x7c52c0`) run whole on a mover laid out at the client's offsets. 2π at `[0xcf5d00]` is zero
//! in the image until a static initializer stores it, so the test writes it. The trajectory tests
//! feed the facing and the pitch back step after step.
//!
//! Host double (`docs/METHOD.md#the-harness`): `strafe_turn_difftest`, `strafe_turn_trajectory` run
//! `fprem` at `0x73f91f`. `simple_turn_move_difftest` runs `fsincos` at `0x7c5307`. The emulator
//! computes each in host `double`, so where its result reaches a compared value the host's math
//! stands in for the x87's, and the test shows the code around it.

use benilla_re_collision::move_integrate::{
    get_pitch_rate, get_strafe_speed, linear_move, pitch_integrate, simple_turn_move, strafe_turn,
    CMovement,
};
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A random `f32` in `[-scale, scale)`.
fn fz(s: &mut u32, scale: f32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * (2.0 * scale) - scale
}

/// Random flags over the bits the functions read.
fn fz_flags(s: &mut u32) -> u32 {
    let bits = [0x10u32, 0x20, 0x40, 0x80, 0x01, 0x02, 0x04, 0x08, 0x2000];
    let mut f = 0u32;
    for (i, b) in bits.iter().enumerate() {
        if (next(s) >> (12 + i as u32)) & 1 != 0 {
            f |= *b;
        }
    }
    f
}

/// Asserts the same bits.
fn assert_bits(b: f32, o: f32, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        b.to_bits(),
        o.to_bits()
    );
}
/// Asserts the same bits.
fn assert_bits64(b: f64, o: f64, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#018x} ours={:#018x}",
        b.to_bits(),
        o.to_bits()
    );
}

/// One case: the mover's inputs, `dt` and the rate argument.
#[derive(Clone, Copy)]
struct Sc {
    flags: u32,
    speed9c: f32,
    speed84: f32,
    facing: f32,
    pitch: f32,
    dir: [f32; 3],
    basis: [f32; 3],
    z: f32,
    dt: f32,
    strafe_in: Option<f32>,
    turn_in: Option<f32>,
}

/// The mover the functions see.
fn cmov(sc: &Sc) -> CMovement {
    CMovement {
        flags: sc.flags,
        facing: sc.facing,
        pitch: sc.pitch,
        move_dir: sc.dir,
        turn_basis: sc.basis,
        turn_scale_z: sc.z,
        speed: sc.speed84,
        turn_speed: sc.speed9c,
    }
}

/// Fixed cases, then random ones.
fn scenarios() -> Vec<Sc> {
    let mut v: Vec<Sc> = Vec::new();
    let base = Sc {
        flags: 0,
        speed9c: 3.0,
        speed84: 7.0,
        facing: 0.0,
        pitch: 0.0,
        dir: [1.0, 0.0, 0.0],
        basis: [1.0, 0.0, 0.0],
        z: 1.0,
        dt: 0.016_666_668,
        strafe_in: None,
        turn_in: Some(2.0),
    };

    for &p in &[
        -3.0f32, -1.5707964, -1.5, 0.0, 1.5, 1.5707964, 3.0, 2.5, -2.5,
    ] {
        v.push(Sc {
            pitch: p,
            flags: 0,
            ..base
        });
    }

    for &fa in &[
        -1.0f32, 1.0, -7.0, 7.0, 6.2831855, -6.2831855, 0.0, 12.0, -0.001,
    ] {
        v.push(Sc {
            facing: fa,
            flags: 0,
            strafe_in: None,
            ..base
        });
    }

    for &fl in &[
        0x00u32,
        0x10,
        0x20,
        0x40,
        0x80,
        0x2000,
        0x10 | 0x01,
        0x20 | 0x04,
        0x40 | 0x2000,
        0x80 | 0x08,
        0x10 | 0x20,
        0x40 | 0x80,
        0xff,
        0x200f,
    ] {
        v.push(Sc { flags: fl, ..base });
    }

    v.push(Sc {
        strafe_in: Some(4.5),
        turn_in: Some(-1.25),
        ..base
    });
    v.push(Sc {
        strafe_in: Some(-2.0),
        turn_in: Some(3.0),
        ..base
    });
    v.push(Sc {
        strafe_in: None,
        turn_in: None,
        flags: 0x10,
        ..base
    });

    let mut s = 0x1e3f_77a9u32;
    for _ in 0..360 {
        let tr = {
            let x = fz(&mut s, 6.0);
            if x.abs() < 0.25 {
                x + 1.0
            } else {
                x
            }
        };
        let turn_in = if next(&mut s) & 0x1000 != 0 {
            Some(tr)
        } else {
            None
        };

        let mut flags = fz_flags(&mut s);
        if turn_in.is_none() && flags & 0x30 == 0 {
            flags |= 0x10;
        }
        v.push(Sc {
            flags,
            speed9c: fz(&mut s, 12.0),
            speed84: fz(&mut s, 12.0),
            facing: fz(&mut s, 8.0),
            pitch: fz(&mut s, 4.0),
            dir: [fz(&mut s, 1.0), fz(&mut s, 1.0), fz(&mut s, 1.0)],
            basis: [fz(&mut s, 1.0), fz(&mut s, 1.0), fz(&mut s, 1.0)],
            z: fz(&mut s, 1.0),
            dt: fz(&mut s, 0.5),
            strafe_in: if next(&mut s) & 0x2000 != 0 {
                Some(fz(&mut s, 10.0))
            } else {
                None
            },
            turn_in,
        });
    }
    v
}

const F_FLAGS: u64 = 0x40;
const F_OUT_STRAFE: u64 = 0x1c;
const F_OUT_PITCH: u64 = 0x20;
const F_FACING: u64 = 0x50;
const F_PITCH: u64 = 0x54;
const F_DIR: u64 = 0x5c;
const F_BASIS: u64 = 0x68;
const F_SCALE_Z: u64 = 0x74;
const F_SPEED: u64 = 0x84;
const F_TURN_SPEED: u64 = 0x9c;
const SENT: u32 = 0x7fc0_0001;

/// `[0xcf5d00]`, which holds 2π once the static initializer runs.
const TWO_PI_VA: u64 = 0xcf5d00;
const TWO_PI_BITS: u32 = 0x40c9_0fdb;

/// Writes an `f32` field.
fn wf(emu: &mut Emu, this: u64, off: u64, v: f32) {
    emu.write_u32(this + off, v.to_bits());
}

/// `0x7c5c50`: the unrounded rate.
#[test]
fn get_strafe_speed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let mut n = 0u32;
    for sc in scenarios() {
        emu.write_u32(this + F_FLAGS, sc.flags);
        wf(&mut emu, this, F_TURN_SPEED, sc.speed9c);
        let got = emu.call_f64(0x7c5c50, this as u32, 0, &[]);
        let want = get_strafe_speed(sc.flags, sc.speed9c);
        assert_bits64(got, want, "0x7c5c50 get_strafe_speed");
        n += 1;
    }
    eprintln!("get_strafe_speed: {n} cases bit-exact vs WoW.exe");
}

/// `0x7c5c90`: the unrounded rate.
#[test]
fn get_pitch_rate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let mut n = 0u32;
    for sc in scenarios() {
        emu.write_u32(this + F_FLAGS, sc.flags);
        wf(&mut emu, this, F_TURN_SPEED, sc.speed9c);
        let got = emu.call_f64(0x7c5c90, this as u32, 0, &[]);
        let want = get_pitch_rate(sc.flags, sc.speed9c);
        assert_bits64(got, want, "0x7c5c90 get_pitch_rate");
        n += 1;
    }
    eprintln!("get_pitch_rate: {n} cases bit-exact vs WoW.exe");
}

/// `0x7c4ed0`: the output vector.
#[test]
fn linear_move_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let out = emu.alloc(0x20);
    let mut n = 0u32;
    for sc in scenarios() {
        for (i, &d) in sc.dir.iter().enumerate() {
            wf(&mut emu, this, F_DIR + (i as u64) * 4, d);
        }
        wf(&mut emu, this, F_SPEED, sc.speed84);
        for k in 0..3 {
            emu.write_u32(out + (k as u64) * 4, SENT);
        }
        emu.call(0x7c4ed0, this as u32, 0, &[sc.dt.to_bits(), out as u32]);
        let want = linear_move(&cmov(&sc), sc.dt);
        for (k, &w) in want.iter().enumerate() {
            let got = emu.read_f32(out + (k as u64) * 4);
            assert_bits(got, w, &format!("0x7c4ed0 linear_move[{k}]"));
        }
        n += 1;
    }
    eprintln!("linear_move: {n} cases bit-exact vs WoW.exe");
}

/// `0x7c4f30`: `+0x1c`, with and without the rate argument.
#[test]
fn strafe_turn_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let arg = emu.alloc(0x10);
    emu.write_u32(TWO_PI_VA, TWO_PI_BITS);
    let mut n = 0u32;
    for sc in scenarios() {
        emu.write_u32(this + F_FLAGS, sc.flags);
        wf(&mut emu, this, F_TURN_SPEED, sc.speed9c);
        wf(&mut emu, this, F_FACING, sc.facing);
        emu.write_u32(this + F_OUT_STRAFE, SENT);
        let arg2 = match sc.strafe_in {
            Some(v) => {
                emu.write_u32(arg, v.to_bits());
                arg as u32
            }
            None => 0,
        };
        emu.call(0x7c4f30, this as u32, 0, &[sc.dt.to_bits(), arg2]);
        let got = emu.read_f32(this + F_OUT_STRAFE);
        let want = strafe_turn(&cmov(&sc), sc.dt, sc.strafe_in);
        assert_bits(got, want, "0x7c4f30 strafe_turn");
        n += 1;
    }
    eprintln!("strafe_turn: {n} cases bit-exact vs WoW.exe");
}

/// `0x7c4f80`: `+0x20`.
#[test]
fn pitch_integrate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let mut n = 0u32;
    for sc in scenarios() {
        emu.write_u32(this + F_FLAGS, sc.flags);
        wf(&mut emu, this, F_TURN_SPEED, sc.speed9c);
        wf(&mut emu, this, F_PITCH, sc.pitch);
        emu.write_u32(this + F_OUT_PITCH, SENT);
        emu.call(0x7c4f80, this as u32, 0, &[sc.dt.to_bits()]);
        let got = emu.read_f32(this + F_OUT_PITCH);
        let want = pitch_integrate(&cmov(&sc), sc.dt);
        assert_bits(got, want, "0x7c4f80 pitch_integrate");
        n += 1;
    }
    eprintln!("pitch_integrate: {n} cases bit-exact vs WoW.exe");
}

/// `0x7c52c0`: the output vector, with and without the rate argument.
#[test]
fn simple_turn_move_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let arg = emu.alloc(0x10);
    let out = emu.alloc(0x20);
    let mut n = 0u32;
    for sc in scenarios() {
        emu.write_u32(this + F_FLAGS, sc.flags);
        wf(&mut emu, this, F_TURN_SPEED, sc.speed9c);
        wf(&mut emu, this, F_SPEED, sc.speed84);
        for (i, &b) in sc.basis.iter().enumerate() {
            wf(&mut emu, this, F_BASIS + (i as u64) * 4, b);
        }
        wf(&mut emu, this, F_SCALE_Z, sc.z);
        for k in 0..3 {
            emu.write_u32(out + (k as u64) * 4, SENT);
        }
        let arg2 = match sc.turn_in {
            Some(v) => {
                emu.write_u32(arg, v.to_bits());
                arg as u32
            }
            None => 0,
        };
        emu.call(
            0x7c52c0,
            this as u32,
            0,
            &[sc.dt.to_bits(), arg2, out as u32],
        );
        let want = simple_turn_move(&cmov(&sc), sc.dt, sc.turn_in);
        for (k, &w) in want.iter().enumerate() {
            let got = emu.read_f32(out + (k as u64) * 4);
            assert_bits(got, w, &format!("0x7c52c0 simple_turn_move[{k}]"));
        }
        n += 1;
    }
    eprintln!("simple_turn_move: {n} cases bit-exact vs WoW.exe");
}

/// `0x7c4f30` over many steps, the facing fed back.
#[test]
fn strafe_turn_trajectory() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let arg = emu.alloc(0x10);
    emu.write_u32(TWO_PI_VA, TWO_PI_BITS);

    let configs: [(&str, f32, f32, f32, usize); 4] = [
        ("forward sweep", 2.0, 1.0 / 60.0, 0.0, 4000),
        ("reverse sweep (+2π leg)", -2.0, 1.0 / 60.0, 0.0, 4000),
        ("fast spin", 50.0, 1.0 / 60.0, 1.0, 4000),
        (
            "slow creep at 2π boundary",
            0.01,
            1.0 / 60.0,
            6.2831855,
            4000,
        ),
    ];

    for (label, omega, dt, facing0, steps) in configs {
        emu.write_u32(this + F_FLAGS, 0);
        emu.write_u32(arg, omega.to_bits());
        let dt_bits = dt.to_bits();
        let ecx = this as u32;
        let arg_ptr = arg as u32;
        let cm0 = CMovement::default();

        let r = emu.trajectory(
            steps,
            vec![facing0.to_bits()],
            |emu, st| {
                emu.write_u32(this + F_FACING, st[0]);
                emu.write_u32(this + F_OUT_STRAFE, SENT);
                emu.call(0x7c4f30, ecx, 0, &[dt_bits, arg_ptr]);
                vec![emu.read_u32(this + F_OUT_STRAFE)]
            },
            |st| {
                let cm = CMovement {
                    facing: f32::from_bits(st[0]),
                    ..cm0
                };
                vec![strafe_turn(&cm, dt, Some(omega)).to_bits()]
            },
        );
        r.assert_match(&format!("strafe_turn trajectory [{label}]"));
    }
    eprintln!(
        "strafe_turn: closed-loop heading trajectory bit-exact over {} configs",
        configs.len()
    );
}

/// `0x7c4f80` over many steps, the pitch fed back.
#[test]
fn pitch_integrate_trajectory() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);

    let configs: [(&str, u32, f32, f32, f32, usize); 4] = [
        ("ramp up to +π/2", 0x40, 3.0, 1.0 / 60.0, 0.0, 4000),
        ("ramp down to -π/2", 0x80, 3.0, 1.0 / 60.0, 0.0, 4000),
        (
            "scaled rate (0x200f) up",
            0x40 | 0x2000,
            3.0,
            1.0 / 60.0,
            -1.0,
            4000,
        ),
        (
            "start past +π/2 (saturate)",
            0x40,
            3.0,
            1.0 / 60.0,
            2.0,
            2000,
        ),
    ];

    for (label, flags, turn_speed, dt, pitch0, steps) in configs {
        emu.write_u32(this + F_FLAGS, flags);
        wf(&mut emu, this, F_TURN_SPEED, turn_speed);
        let dt_bits = dt.to_bits();
        let ecx = this as u32;
        let cm0 = CMovement {
            flags,
            turn_speed,
            ..CMovement::default()
        };

        let r = emu.trajectory(
            steps,
            vec![pitch0.to_bits()],
            |emu, st| {
                emu.write_u32(this + F_PITCH, st[0]);
                emu.write_u32(this + F_OUT_PITCH, SENT);
                emu.call(0x7c4f80, ecx, 0, &[dt_bits]);
                vec![emu.read_u32(this + F_OUT_PITCH)]
            },
            |st| {
                let cm = CMovement {
                    pitch: f32::from_bits(st[0]),
                    ..cm0
                };
                vec![pitch_integrate(&cm, dt).to_bits()]
            },
        );
        r.assert_match(&format!("pitch_integrate trajectory [{label}]"));
    }
    eprintln!(
        "pitch_integrate: closed-loop pitch trajectory bit-exact over {} configs",
        configs.len()
    );
}
