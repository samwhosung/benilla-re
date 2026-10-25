//! `CMovement::TurnPitchDisplacement` (`0x7c4fd0`), `CMovement::PitchArcDisplacement` (`0x7c5180`),
//! `CMovement::CalcBaseDirection` (`0x7c5880`) and `CMovementData::CalcDirection` (`0x7c5a20`) run
//! whole on a mover laid out at the client's offsets. The output fields start as sentinels, so a
//! field the function leaves alone is compared too. The arc functions divide by the turn and pitch
//! rates, so the random rates stay away from 0.
//!
//! Host double (`docs/METHOD.md#the-harness`): `velocity_basis_difftest`,
//! `velocity_vector_difftest` run `fsincos` at `0x7c58a1` and `0x7c58eb`. `pitch_arc_difftest` runs
//! `fsincos` at `0x7c521d`. `turn_pitch_arc_difftest` runs `fsincos` at `0x7c509c` and `0x7c50b7`.
//! The emulator computes each in host `double`, so where its result reaches a compared value the
//! host's math stands in for the x87's, and the test shows the code around it.
//!
//! `pitch_land_step_trace` runs the whole dispatcher `0x7c5360` on movers that do not swim, with
//! `fsincos` at `0x7c58a1` and `0x7c521d` and `fprem` at `0x73f91f`, the `fmod` of `0x7c4f30`.

use benilla_re_collision::move_arc::{
    pitch_arc, turn_pitch_arc, velocity_basis, velocity_vector, CMovement,
};
use benilla_re_collision::move_integrate::{pitch_integrate, CMovement as IntegrateMovement};
use benilla_re_collision::transport_facing::substep_dt;
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A random `f32` in `[-scale, scale)`.
fn fz(s: &mut u32, scale: f32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * (2.0 * scale) - scale
}

/// The same, kept away from 0.
fn fz_nz(s: &mut u32, scale: f32) -> f32 {
    let x = fz(s, scale);
    if x.abs() < 0.5 {
        if x < 0.0 {
            x - 1.5
        } else {
            x + 1.5
        }
    } else {
        x
    }
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

/// One case: the mover's inputs, `dt`, the turn-rate argument and `0x7c5a20`'s argument.
#[derive(Clone, Copy)]
struct Sc {
    arc_flags: u32,
    vel_flags: u32,
    facing: f32,
    pitch: f32,
    speed: f32,
    turn_speed: f32,
    basis: [f32; 4],
    dt: f32,
    turn_in: Option<f32>,
    arg1: u32,
}

const SENT_DIR: [f32; 3] = [101.5, 202.5, 303.5];
const SENT_B: [f32; 4] = [404.5, 505.5, 606.5, 707.5];

/// The mover `0x7c4fd0` and `0x7c5180` see.
fn cmov_arc(sc: &Sc) -> CMovement {
    CMovement {
        flags: sc.arc_flags,
        facing: sc.facing,
        pitch: sc.pitch,
        dir: [0.0; 3],
        b68: sc.basis[0],
        b6c: sc.basis[1],
        b70: sc.basis[2],
        b74: sc.basis[3],
        speed: sc.speed,
        turn_speed: sc.turn_speed,
    }
}

/// The mover `0x7c5880` and `0x7c5a20` see.
fn cmov_vel(sc: &Sc) -> CMovement {
    CMovement {
        flags: sc.vel_flags,
        facing: sc.facing,
        pitch: sc.pitch,
        dir: SENT_DIR,
        b68: SENT_B[0],
        b6c: SENT_B[1],
        b70: SENT_B[2],
        b74: SENT_B[3],
        speed: sc.speed,
        turn_speed: sc.turn_speed,
    }
}

/// The cases: fixed flag combinations and pitches at and past the clamps, then random ones.
fn scenarios() -> Vec<Sc> {
    let base = Sc {
        arc_flags: 0x40,
        vel_flags: 0x1,
        facing: 0.5,
        pitch: 0.1,
        speed: 7.0,
        turn_speed: 3.0,
        basis: [0.8, 0.6, 0.9, 0.3],
        dt: 0.016_666_668,
        turn_in: Some(2.0),
        arg1: 1,
    };
    let mut v: Vec<Sc> = Vec::new();

    for &p in &[
        -1.6f32, -1.5707964, -1.5, -0.5, 0.0, 0.5, 1.5, 1.5707964, 1.6, -3.0, 3.0,
    ] {
        v.push(Sc {
            pitch: p,
            dt: 0.4,
            turn_speed: 6.0,
            arc_flags: 0x40,
            turn_in: Some(2.5),
            ..base
        });
        v.push(Sc {
            pitch: p,
            dt: 0.4,
            turn_speed: -6.0,
            arc_flags: 0x80 | 0x10,
            turn_in: None,
            ..base
        });
        v.push(Sc {
            pitch: p,
            dt: -0.3,
            turn_speed: 4.0,
            arc_flags: 0x40 | 0x2000,
            turn_in: Some(-1.75),
            ..base
        });
    }

    for &vf in &[0u32, 0x200000] {
        for &p in &[0.0f32, 1e-7, 9.536_743e-7, 1.0e-6, 0.7, -0.7, 1.2, -1.4] {
            for &fa in &[0.0f32, 0.7, -1.3, 2.5, 6.0] {
                v.push(Sc {
                    vel_flags: vf,
                    pitch: p,
                    facing: fa,
                    ..base
                });
            }
        }
    }

    for &mv in &[
        0u32, 0x1, 0x2, 0x4, 0x8, 0x3, 0x5, 0x6, 0x9, 0xa, 0xc, 0xd, 0xe,
    ] {
        for &extra in &[0u32, 0x200000, 0x2000, 0x2000 | 0x200000] {
            for &a in &[0u32, 1] {
                v.push(Sc {
                    vel_flags: mv | extra,
                    arg1: a,
                    facing: 0.9,
                    pitch: 0.6,
                    ..base
                });
            }
        }
    }

    let mut s = 0x6d5a_f00du32;
    for _ in 0..400 {
        let pitch_bit = if (next(&mut s) >> 20) & 1 != 0 {
            0x40u32
        } else {
            0x80
        };
        let mut af = pitch_bit;
        if (next(&mut s) >> 19) & 1 != 0 {
            af |= 0x2000;
        }
        if (next(&mut s) >> 18) & 1 != 0 {
            af |= 0x01;
        }
        let turn_in = if (next(&mut s) >> 17) & 1 != 0 {
            Some(fz_nz(&mut s, 6.0))
        } else {
            af |= if (next(&mut s) >> 16) & 1 != 0 {
                0x10
            } else {
                0x20
            };
            None
        };
        let mut vf = (next(&mut s) >> 8) & 0xf;
        if (next(&mut s) >> 12) & 1 != 0 {
            vf |= 0x2000;
        }
        if (next(&mut s) >> 13) & 1 != 0 {
            vf |= 0x200000;
        }
        v.push(Sc {
            arc_flags: af,
            vel_flags: vf,
            facing: fz(&mut s, 8.0),
            pitch: fz(&mut s, 2.2),
            speed: fz(&mut s, 10.0),
            turn_speed: fz_nz(&mut s, 6.0),
            basis: [
                fz(&mut s, 1.0),
                fz(&mut s, 1.0),
                fz(&mut s, 1.0),
                fz(&mut s, 1.0),
            ],
            dt: fz(&mut s, 0.5),
            turn_in,
            arg1: (next(&mut s) >> 14) & 1,
        });
    }
    v
}

const F_FLAGS: u64 = 0x40;
const F_LIVE_PITCH: u64 = 0x20;
const F_FACING: u64 = 0x50;
const F_PITCH: u64 = 0x54;
const F_DIR: u64 = 0x5c;
const F_B68: u64 = 0x68;
const F_SPEED: u64 = 0x84;
const F_TURN_SPEED: u64 = 0x9c;
const SENT: u32 = 0x7fc0_0001;

/// Writes an `f32` field.
fn wf(emu: &mut Emu, this: u64, off: u64, v: f32) {
    emu.write_u32(this + off, v.to_bits());
}

/// Writes the basis fields.
fn seed_basis(emu: &mut Emu, this: u64, basis: [f32; 4]) {
    for (i, &b) in basis.iter().enumerate() {
        wf(emu, this, F_B68 + (i as u64) * 4, b);
    }
}

/// `0x7c4fd0`: the output vector, with and without a turn-rate argument.
#[test]
fn turn_pitch_arc_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let arg = emu.alloc(0x10);
    let out = emu.alloc(0x20);
    let mut n = 0u32;
    for sc in scenarios() {
        emu.write_u32(this + F_FLAGS, sc.arc_flags);
        wf(&mut emu, this, F_PITCH, sc.pitch);
        wf(&mut emu, this, F_SPEED, sc.speed);
        wf(&mut emu, this, F_TURN_SPEED, sc.turn_speed);
        seed_basis(&mut emu, this, sc.basis);
        for k in 0..3 {
            emu.write_u32(out + (k as u64) * 4, SENT);
        }
        let arg2 = match sc.turn_in {
            Some(val) => {
                emu.write_u32(arg, val.to_bits());
                arg as u32
            }
            None => 0,
        };
        emu.call(
            0x7c4fd0,
            this as u32,
            0,
            &[sc.dt.to_bits(), arg2, out as u32],
        );
        let want = turn_pitch_arc(&cmov_arc(&sc), sc.dt, sc.turn_in);
        for (k, &w) in want.iter().enumerate() {
            let got = emu.read_f32(out + (k as u64) * 4);
            assert_bits(got, w, &format!("0x7c4fd0 turn_pitch_arc[{k}]"));
        }
        n += 1;
    }
    eprintln!("turn_pitch_arc: {n} cases bit-exact vs WoW.exe");
}

/// `0x7c5180`: the output vector.
#[test]
fn pitch_arc_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let out = emu.alloc(0x20);
    let mut n = 0u32;
    for sc in scenarios() {
        emu.write_u32(this + F_FLAGS, sc.arc_flags);
        wf(&mut emu, this, F_PITCH, sc.pitch);
        wf(&mut emu, this, F_SPEED, sc.speed);
        wf(&mut emu, this, F_TURN_SPEED, sc.turn_speed);
        seed_basis(&mut emu, this, sc.basis);
        for k in 0..3 {
            emu.write_u32(out + (k as u64) * 4, SENT);
        }
        emu.call(0x7c5180, this as u32, 0, &[sc.dt.to_bits(), out as u32]);
        let want = pitch_arc(&cmov_arc(&sc), sc.dt);
        for (k, &w) in want.iter().enumerate() {
            let got = emu.read_f32(out + (k as u64) * 4);
            assert_bits(got, w, &format!("0x7c5180 pitch_arc[{k}]"));
        }
        n += 1;
    }
    eprintln!("pitch_arc: {n} cases bit-exact vs WoW.exe");
}

/// Reads the direction and the basis as bits.
fn read_basis_out(emu: &Emu, this: u64) -> [u32; 7] {
    let mut o = [0u32; 7];
    for (k, slot) in o.iter_mut().enumerate() {
        *slot = emu.read_u32(this + F_DIR + (k as u64) * 4);
    }
    o
}

/// The same fields from the reimplementation.
fn want_basis_out(m: &CMovement) -> [u32; 7] {
    [
        m.dir[0].to_bits(),
        m.dir[1].to_bits(),
        m.dir[2].to_bits(),
        m.b68.to_bits(),
        m.b6c.to_bits(),
        m.b70.to_bits(),
        m.b74.to_bits(),
    ]
}

/// Writes sentinels to the direction and the basis.
fn seed_basis_out(emu: &mut Emu, this: u64) {
    for (k, &b) in SENT_DIR.iter().enumerate() {
        wf(emu, this, F_DIR + (k as u64) * 4, b);
    }
    for (k, &b) in SENT_B.iter().enumerate() {
        wf(emu, this, F_B68 + (k as u64) * 4, b);
    }
}

const OUT_NAMES: [&str; 7] = ["dir0", "dir1", "dir2", "b68", "b6c", "b70", "b74"];

/// `0x7c5880`: the direction and the basis.
#[test]
fn velocity_basis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let mut n = 0u32;
    for sc in scenarios() {
        emu.write_u32(this + F_FLAGS, sc.vel_flags);
        wf(&mut emu, this, F_FACING, sc.facing);
        wf(&mut emu, this, F_PITCH, sc.pitch);
        seed_basis_out(&mut emu, this);
        emu.call(0x7c5880, this as u32, 0, &[]);
        let mut m = cmov_vel(&sc);
        velocity_basis(&mut m);
        let got = read_basis_out(&emu, this);
        let want = want_basis_out(&m);
        for k in 0..7 {
            assert_bits(
                f32::from_bits(got[k]),
                f32::from_bits(want[k]),
                &format!("0x7c5880 velocity_basis[{}]", OUT_NAMES[k]),
            );
        }
        n += 1;
    }
    eprintln!("velocity_basis: {n} cases bit-exact vs WoW.exe");
}

/// `0x7c5a20`: the direction and the basis.
#[test]
fn velocity_vector_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let mut n = 0u32;
    for sc in scenarios() {
        emu.write_u32(this + F_FLAGS, sc.vel_flags);
        wf(&mut emu, this, F_FACING, sc.facing);
        wf(&mut emu, this, F_PITCH, sc.pitch);
        seed_basis_out(&mut emu, this);
        emu.call(0x7c5a20, this as u32, 0, &[sc.arg1]);
        let mut m = cmov_vel(&sc);
        velocity_vector(&mut m, sc.arg1);
        let got = read_basis_out(&emu, this);
        let want = want_basis_out(&m);
        for k in 0..7 {
            assert_bits(
                f32::from_bits(got[k]),
                f32::from_bits(want[k]),
                &format!("0x7c5a20 velocity_vector[{}]", OUT_NAMES[k]),
            );
        }
        n += 1;
    }
    eprintln!("velocity_vector: {n} cases bit-exact vs WoW.exe");
}

/// The mover `0x7c5360` sees when it does not swim: the basis `0x7c5880` builds, the anchor pitch
/// `+0x54` and the live pitch `+0x20` at `anchor`, a speed of 7 and a turn rate of 3.
fn land_mover(emu: &mut Emu, this: u64, flags: u32, anchor: f32) {
    emu.write_u32(this + F_FLAGS, flags);
    wf(emu, this, F_FACING, 0.5);
    wf(emu, this, F_PITCH, anchor);
    wf(emu, this, F_LIVE_PITCH, anchor);
    wf(emu, this, F_SPEED, 7.0);
    wf(emu, this, F_TURN_SPEED, 3.0);
    seed_basis_out(emu, this);
    emu.call(0x7c5880, this as u32, 0, &[]);
}

/// One dispatcher call: the return value, the output vector and the live pitch.
fn land_step(emu: &mut Emu, this: u64, out: u64, ms: u32) -> (u32, [f32; 3], f32) {
    for k in 0..3 {
        emu.write_u32(out + (k as u64) * 4, SENT);
    }
    let ret = emu.call(0x7c5360, this as u32, 0, &[ms, 0, out as u32]);
    let o = [
        emu.read_f32(out),
        emu.read_f32(out + 4),
        emu.read_f32(out + 8),
    ];
    (ret, o, emu.read_f32(this + F_LIVE_PITCH))
}

/// `0x7c5360` with a pitch bit and no swimming (flags without 0x200000): the pitch is integrated
/// and the pitch arc runs on the land basis (1, 0), whose `z` is the vertical term. The horizontal
/// reach is `|Rv|·sin(p·t)` below the clamp and fixed at `|Rv|·cos a` past it, so from a tilt
/// against the bit it peaks and falls back; it is 0 from a saturated anchor. A mover whose mode is
/// only pitch or only turn returns 0, falling or not, with or without `override`; one that falls
/// with a turn bit returns 1.
#[test]
fn pitch_land_step_trace() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0xb0);
    let out = emu.alloc(0x20);
    // 2π at `0xcf5d00`, which a static initializer stores and `0x7c4f30`'s `fmod` reads.
    emu.write_u32(0xcf5d00, 0x40c9_0fdb);
    let half_pi = f32::from_bits(0x3fc9_0fdb);
    let reach = |o: [f32; 3]| (f64::from(o[0]).powi(2) + f64::from(o[1]).powi(2)).sqrt();
    let run = |emu: &mut Emu, flags: u32, anchor: f32, ms: u32| {
        land_mover(emu, this, flags, anchor);
        let basis = [
            emu.read_f32(this + F_B68),
            emu.read_f32(this + F_B68 + 4),
            emu.read_f32(this + F_B68 + 8),
            emu.read_f32(this + F_B68 + 12),
        ];
        let (ret, o, live) = land_step(emu, this, out, ms);
        (basis, ret, o, live)
    };

    // A walker pitching up and one pitching down, from level and from a tilt: the exe's output is
    // the pitch arc on the basis `0x7c5880` leaves, with the pitch integrated.
    let mut n = 0u32;
    for &(flags, anchor) in &[
        (0x41u32, 0.0f32),
        (0x81, 0.0),
        (0x41, 0.4),
        (0x81, -0.4),
        (0x2041, 0.0),
        (0x45, 0.0),
    ] {
        for &ms in &[16u32, 100, 400, 700, 900, 1000, 2000, 5000] {
            let (basis, ret, o, live) = run(&mut emu, flags, anchor, ms);
            assert_eq!(&basis[2..], &[1.0, 0.0], "land basis, flags {flags:#x}");
            assert_eq!(ret, 1, "mode 9 or 13 returns 1, flags {flags:#x}");
            let dt = substep_dt(ms);
            let m = CMovement {
                flags,
                facing: 0.5,
                pitch: anchor,
                b68: basis[0],
                b6c: basis[1],
                b70: basis[2],
                b74: basis[3],
                speed: 7.0,
                turn_speed: 3.0,
                ..Default::default()
            };
            for (k, &w) in pitch_arc(&m, dt).iter().enumerate() {
                assert_bits(o[k], w, &format!("flags {flags:#x} ms {ms} out[{k}]"));
            }
            let im = IntegrateMovement {
                flags,
                pitch: anchor,
                turn_speed: 3.0,
                ..Default::default()
            };
            assert_bits(live, pitch_integrate(&im, dt), "live pitch");
            n += 1;
        }
    }

    // Past the clamp the horizontal part is fixed and only z grows: up from level the reach is
    // speed / pitch rate = 7 / 2.25, and the commit drops z on land.
    let (_, _, early, _) = run(&mut emu, 0x41, 0.0, 2000);
    let (_, _, late, live) = run(&mut emu, 0x41, 0.0, 5000);
    assert_bits(early[0], late[0], "x stops growing");
    assert_bits(early[1], late[1], "y stops growing");
    assert!(
        late[2] > early[2],
        "z keeps growing: {} {}",
        early[2],
        late[2]
    );
    assert!(
        (reach(late) - 7.0 / 2.25).abs() < 1e-3,
        "reach {}",
        reach(late)
    );
    assert_eq!(live, half_pi, "the live pitch is held at the clamp");

    // From a tilt against the bit the reach peaks at |Rv| (p·t = π/2, 0.70 s) and falls back to
    // |Rv|·cos a once the clamp holds (0.88 s): the walker steps back toward the anchor.
    let (_, _, peak, _) = run(&mut emu, 0x41, -0.4, 700);
    let (_, _, held, live) = run(&mut emu, 0x41, -0.4, 900);
    assert!(reach(peak) > reach(held), "{} {}", reach(peak), reach(held));
    assert!(
        (reach(peak) - 7.0 / 2.25).abs() < 1e-3,
        "peak {}",
        reach(peak)
    );
    assert!(
        (reach(held) - 7.0 / 2.25 * 0.4f64.cos()).abs() < 1e-3,
        "held {}",
        reach(held)
    );
    assert_eq!(live, half_pi);

    // From a saturated anchor the horizontal part is 0, up or down.
    for &(flags, anchor) in &[(0x41u32, half_pi), (0x81, -half_pi)] {
        let (_, ret, o, live) = run(&mut emu, flags, anchor, 100);
        assert_eq!(ret, 1);
        assert!(o[0] == 0.0 && o[1] == 0.0, "no horizontal step: {o:?}");
        assert_eq!(live, anchor);
    }

    // Only pitch bits for the mode: the dispatcher returns 0 and leaves the output alone, falling
    // or not. A turn bit while falling is mode 0, which returns 1 with no handler.
    for &(flags, want) in &[
        (0x40u32, 0u32),
        (0x80, 0),
        (0x2040, 0),
        (0x2080, 0),
        (0x2050, 0),
        (0x10, 0),
        (0x50, 0),
        (0x2010, 1),
        (0x2000, 1),
    ] {
        let (_, ret, o, _) = run(&mut emu, flags, 0.0, 100);
        assert_eq!(ret, want, "return for flags {flags:#x}");
        assert!(
            o.iter().all(|v| v.to_bits() == SENT),
            "flags {flags:#x} leaves the output alone: {o:?}"
        );
    }
    // The non-null `override` branch (`0x7c539b`) sets bit 2 unless falling: a pitch bit alone is
    // mode 10, falling it stays mode 8, and both return 0.
    let ovr = emu.alloc(0x10);
    emu.write_u32(ovr, 1.0f32.to_bits());
    for &flags in &[0x40u32, 0x2040] {
        land_mover(&mut emu, this, flags, 0.0);
        for k in 0..3 {
            emu.write_u32(out + (k as u64) * 4, SENT);
        }
        let ret = emu.call(0x7c5360, this as u32, 0, &[100, ovr as u32, out as u32]);
        assert_eq!(ret, 0, "override, flags {flags:#x}");
    }
    let (_, ret, o, _) = run(&mut emu, 0x2041, 0.0, 100);
    assert_eq!(ret, 1, "falling with forward and pitch is mode 9");
    assert!(o.iter().any(|v| v.to_bits() != SENT));
    eprintln!("pitch_land_step: {n} arc cases bit-exact vs WoW.exe");
}
