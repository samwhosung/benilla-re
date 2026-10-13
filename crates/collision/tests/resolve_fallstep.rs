//! `CMovement::LimitStepByFall` (`0x635450`, through `call_f32`), `CMovement::IsRisingThroughStep`
//! (`0x635550`), `CMovement::SaveMotionState` (`0x635ea0`) and `CMovement::RestoreMotionState`
//! (`0x635f10`) run whole; the fall-time and apex functions they call run too, on the mover fields
//! the test writes.

#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::manual_is_multiple_of)]

use benilla_re_collision::resolve_fall::{fall_time_solve, swim_fall_vel};
use benilla_re_collision::resolve_fallstep::{
    fall_feasible, fall_step, state_restore, state_snapshot, SNAPSHOT_FIELDS,
};
use benilla_re_difftest::{Emu, RegisterX86, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// A random `f32` in `[-4, 4)`.
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}
/// Writes three `f32`.
fn wv3(emu: &mut Emu, va: u64, v: &[f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, c.to_bits());
    }
}
/// Reads three `f32`.
fn rv3(emu: &Emu, va: u64) -> [f32; 3] {
    core::array::from_fn(|i| emu.read_f32(va + (i as u64) * 4))
}
/// Widens an `f32`.
#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}
const SMALL_LEN: f64 = f32::from_bits(0x3580_0000) as f64;

#[derive(Clone, Copy)]
struct Case {
    fa0: f32,
    flags: u32,
    f7c: f32,
    pos_z: f32,
    f84: f32,
    rem: f32,
    t: f32,
    scratch: f32,
    len: f32,
    delta: [f32; 3],
}

const LEG0: usize = 0;
const PASS: usize = 1;
const NORESCALE: usize = 2;
const RESCALE: usize = 3;

/// `0x635450`: the time, the move and its length.
#[test]
fn fall_step_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let vmax_land = emu.read_f32(0x87d894);
    let vmax_swim = emu.read_f32(0x87d898);
    let this_va = emu.alloc(0x200);
    let delta_va = emu.alloc(0x10);
    let outlen_va = emu.alloc(0x10);
    let sent = 0x7fc0_0001u32;

    let mut cases: Vec<Case> = vec![
        Case {
            fa0: 0.0,
            flags: 0,
            f7c: 10.0,
            pos_z: 0.0,
            f84: 1.0,
            rem: 2.0,
            t: 2.0,
            scratch: 0.0,
            len: 1.0,
            delta: [0.0, 0.0, 0.0],
        },
        Case {
            fa0: 0.0,
            flags: 0,
            f7c: 10.0,
            pos_z: 0.0,
            f84: 1.0,
            rem: 2.0,
            t: 2.0,
            scratch: 1.0,
            len: 1.414,
            delta: [1.0, 1.0, 0.0],
        },
        Case {
            fa0: 0.0,
            flags: 0,
            f7c: 10.0,
            pos_z: 0.0,
            f84: 1.0,
            rem: 0.05,
            t: 0.5,
            scratch: 0.0,
            len: 1.414,
            delta: [1.0, 1.0, 0.0],
        },
        Case {
            fa0: 0.0,
            flags: 0,
            f7c: 10.0,
            pos_z: 0.0,
            f84: 1.0,
            rem: 2.0,
            t: 0.5,
            scratch: 0.0,
            len: 1.414,
            delta: [1.0, 1.0, 0.0],
        },
        Case {
            fa0: 0.0,
            flags: 0,
            f7c: 10.0,
            pos_z: 0.0,
            f84: 1.0,
            rem: 2.0,
            t: 0.5,
            scratch: 0.0,
            len: 1.0,
            delta: [0.0, 0.0, 0.0],
        },
        Case {
            fa0: 0.0,
            flags: 0,
            f7c: 10.0,
            pos_z: 0.0,
            f84: 1.0,
            rem: 2.0,
            t: 0.5,
            scratch: 0.0,
            len: 1e-9,
            delta: [1.0, 1.0, 0.0],
        },
        Case {
            fa0: 0.0,
            flags: 0x2000_0000,
            f7c: 4.0,
            pos_z: 0.0,
            f84: 1.0,
            rem: 2.0,
            t: 0.2,
            scratch: 0.0,
            len: 1.414,
            delta: [1.0, 1.0, 0.0],
        },
        Case {
            fa0: 5.0,
            flags: 0x2000,
            f7c: 10.0,
            pos_z: 0.0,
            f84: 1.0,
            rem: 2.0,
            t: 0.1,
            scratch: 0.0,
            len: 1.414,
            delta: [1.0, 1.0, 0.0],
        },
        Case {
            fa0: 7.0,
            flags: 0x2000,
            f7c: 10.0,
            pos_z: 0.0,
            f84: 1.0,
            rem: 2.0,
            t: 0.1,
            scratch: 2.5,
            len: 1.414,
            delta: [1.0, 1.0, 0.0],
        },
    ];

    let mut s = 0xfa11_5e70u32;
    for case in 0..600u32 {
        let fa0 = match case % 4 {
            0 => 0.0,
            1 => 1e-9,
            2 => fz(&mut s).abs() * 8.0,
            _ => -(fz(&mut s).abs() * 8.0),
        };
        let flags = (if (next(&mut s) >> 8) % 3 == 0 {
            0x2000_0000
        } else {
            0
        }) | (if (next(&mut s) >> 8) % 2 == 0 {
            0x2000
        } else {
            0
        });
        let f7c = fz(&mut s) * 20.0;
        let pos_z = fz(&mut s) * 5.0;
        let f84 = fz(&mut s).abs() + 0.1;
        let delta = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let rem = match case % 5 {
            0 => 1e-9,
            1 => fz(&mut s).abs() * 0.01,
            _ => fz(&mut s).abs() * 3.0,
        };
        let t = match case % 6 {
            0 => fz(&mut s).abs() * 100.0,
            1 => 0.0,
            _ => fz(&mut s).abs(),
        };
        let scratch = if (next(&mut s) >> 8) % 7 == 0 {
            fz(&mut s)
        } else {
            0.0
        };
        let len = match case % 8 {
            0 => 1e-9,
            _ => fz(&mut s).abs() * 2.0 + 0.001,
        };
        cases.push(Case {
            fa0,
            flags,
            f7c,
            pos_z,
            f84,
            rem,
            t,
            scratch,
            len,
            delta,
        });
    }

    let mut leg_hits = [0u32; 4];
    let mut len_skip = 0u32;
    let mut len_compute = 0u32;
    let mut feas_hits = [0u32; 2];
    let mut checked = 0u32;

    for (ci, c) in cases.iter().enumerate() {
        emu.write_u32(this_va + 0x18, c.pos_z.to_bits());
        emu.write_u32(this_va + 0x40, c.flags);
        emu.write_u32(this_va + 0x7c, c.f7c.to_bits());
        emu.write_u32(this_va + 0x84, c.f84.to_bits());
        emu.write_u32(this_va + 0xa0, c.fa0.to_bits());
        wv3(&mut emu, delta_va, &c.delta);
        emu.write_u32(outlen_va, sent);

        let st0 = emu.call_f32(
            0x635450,
            this_va as u32,
            0,
            &[
                c.rem.to_bits(),
                c.t.to_bits(),
                delta_va as u32,
                outlen_va as u32,
                c.scratch.to_bits(),
                c.len.to_bits(),
            ],
        );
        let eax_bin = emu.get_reg(RegisterX86::EAX) as u32;
        let delta_bin = rv3(&emu, delta_va);
        let outlen_bin = emu.read_f32(outlen_va);

        let mut delta_ours = c.delta;
        let mut outlen_ours = f32::from_bits(sent);
        let out = fall_step(
            c.fa0,
            c.flags,
            vmax_land,
            vmax_swim,
            c.f7c,
            c.pos_z,
            c.f84,
            c.rem,
            c.t,
            c.scratch,
            c.len,
            &mut delta_ours,
            &mut outlen_ours,
        );

        assert_eq!(
            st0.to_bits(),
            (out.consumed as f32).to_bits(),
            "case {ci}: consumed binary={:#010x} ours={:#010x}",
            st0.to_bits(),
            (out.consumed as f32).to_bits()
        );
        for k in 0..3 {
            assert_eq!(
                delta_bin[k].to_bits(),
                delta_ours[k].to_bits(),
                "case {ci}: delta[{k}] binary={:#010x} ours={:#010x}",
                delta_bin[k].to_bits(),
                delta_ours[k].to_bits()
            );
        }
        assert_eq!(
            outlen_bin.to_bits(),
            outlen_ours.to_bits(),
            "case {ci}: out_len binary={:#010x} ours={:#010x}",
            outlen_bin.to_bits(),
            outlen_ours.to_bits()
        );

        let feas = fall_feasible(
            c.fa0,
            c.flags,
            c.f84,
            c.t,
            c.rem,
            [c.delta[0], c.delta[1]],
            c.scratch,
        );
        feas_hits[feas.eax as usize] += 1;
        let frac = if f(c.len) > SMALL_LEN {
            len_compute += 1;
            let sq = f(c.delta[1]) * f(c.delta[1]) + f(c.delta[0]) * f(c.delta[0]);
            ((sq.sqrt() / f(c.len)) * f(c.rem)) as f32
        } else {
            len_skip += 1;
            0.0
        };
        let dd = (f(c.f7c) - f(c.pos_z) - f(c.delta[2])) as f32;
        let solved = fall_time_solve(c.fa0, c.flags, vmax_land, vmax_swim, dd, feas.eax);
        let leg = if solved <= f(c.t) {
            LEG0
        } else {
            let r = solved - f(c.t);
            if r > f(c.rem) {
                PASS
            } else if r < f(frac) {
                RESCALE
            } else {
                NORESCALE
            }
        };
        leg_hits[leg] += 1;
        if leg == LEG0 {
            assert_eq!(
                eax_bin, 0,
                "case {ci}: leg0 eax should be 0, got {eax_bin:#x}"
            );
        }
        checked += 1;
    }
    eprintln!(
        "fall_step: {checked} cases bit-exact; legs [leg0={} pass={} norescale={} rescale={}] \
         len[skip={len_skip} compute={len_compute}] feas[0={} 1={}]",
        leg_hits[0], leg_hits[1], leg_hits[2], leg_hits[3], feas_hits[0], feas_hits[1]
    );
    for (i, h) in leg_hits.iter().enumerate() {
        assert!(*h > 0, "fall-step leg {i} never hit");
    }
    assert!(len_skip > 0 && len_compute > 0, "len gate not both-sided");
    assert!(
        feas_hits[0] > 0 && feas_hits[1] > 0,
        "feasibility not both-sided"
    );
}

const E_CNZ: usize = 0;
const E_FA0: usize = 1;
const E_AB: usize = 2;
const E_AGTC: usize = 3;
const E_M1: usize = 4;
const E_M0: usize = 5;

/// `0x635550`.
#[test]
fn fall_feasible_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    let this_va = emu.alloc(0x200);
    let vec_va = emu.alloc(0x10);

    #[derive(Clone, Copy)]
    struct FCase {
        fa0: f32,
        flags: u32,
        f84: f32,
        a: f32,
        b: f32,
        vec: [f32; 2],
        c: f32,
    }

    let mut cases: Vec<FCase> = vec![
        FCase {
            fa0: -1.0,
            flags: 0x2000,
            f84: 1.0,
            a: 0.0,
            b: 0.0,
            vec: [0.0, 0.0],
            c: 3.0,
        },
        FCase {
            fa0: -1.0,
            flags: 0x2000,
            f84: 1.0,
            a: 0.0,
            b: 0.0,
            vec: [0.0, 0.0],
            c: -0.0,
        },
        FCase {
            fa0: 1.0,
            flags: 0x2000,
            f84: 1.0,
            a: 0.0,
            b: 0.0,
            vec: [0.0, 0.0],
            c: 0.0,
        },
        FCase {
            fa0: -10.0,
            flags: 0x2000,
            f84: 1.0,
            a: 0.0,
            b: 0.0,
            vec: [0.0, 0.0],
            c: 0.0,
        },
        FCase {
            fa0: -10.0,
            flags: 0x2000,
            f84: 1.0,
            a: 5.0,
            b: 0.0,
            vec: [0.0, 0.0],
            c: 0.0,
        },
        FCase {
            fa0: -10.0,
            flags: 0x2000,
            f84: 4.0,
            a: 0.5,
            b: 1.0,
            vec: [0.01, 0.01],
            c: 0.0,
        },
        FCase {
            fa0: -10.0,
            flags: 0x2000,
            f84: 0.01,
            a: 0.5,
            b: 1.0,
            vec: [3.0, 3.0],
            c: 0.0,
        },
        FCase {
            fa0: -10.0,
            flags: 0,
            f84: 1.0,
            a: -1.0,
            b: -1.0,
            vec: [0.0, 0.0],
            c: 0.0,
        },
    ];
    let mut s = 0x5ea1_b00cu32;
    for _ in 0..600u32 {
        let fa0 = match (next(&mut s) >> 8) % 3 {
            0 => 0.0,
            1 => fz(&mut s).abs() * 5.0,
            _ => -(fz(&mut s).abs() * 20.0),
        };
        let flags = if (next(&mut s) >> 8) % 2 == 0 {
            0x2000
        } else {
            0
        };
        let f84 = fz(&mut s) * 3.0;
        let a = fz(&mut s) * 2.0;
        let b = fz(&mut s) * 2.0;
        let vec = [fz(&mut s), fz(&mut s)];
        let c = if (next(&mut s) >> 8) % 4 == 0 {
            fz(&mut s)
        } else {
            0.0
        };
        cases.push(FCase {
            fa0,
            flags,
            f84,
            a,
            b,
            vec,
            c,
        });
    }

    let mut exits = [0u32; 6];
    let mut checked = 0u32;
    for (ci, fc) in cases.iter().enumerate() {
        emu.write_u32(this_va + 0xa0, fc.fa0.to_bits());
        emu.write_u32(this_va + 0x40, fc.flags);
        emu.write_u32(this_va + 0x84, fc.f84.to_bits());
        emu.write_u32(vec_va, fc.vec[0].to_bits());
        emu.write_u32(vec_va + 4, fc.vec[1].to_bits());

        let eax_bin = emu.call(
            0x635550,
            this_va as u32,
            0,
            &[
                fc.a.to_bits(),
                fc.b.to_bits(),
                vec_va as u32,
                fc.c.to_bits(),
            ],
        );

        let c_cell = emu.get_reg(RegisterX86::ESP) - 4;
        let c_bin = emu.read_f32(c_cell);

        let ours = fall_feasible(fc.fa0, fc.flags, fc.f84, fc.a, fc.b, fc.vec, fc.c);
        assert_eq!(
            eax_bin, ours.eax,
            "case {ci}: eax binary={eax_bin} ours={}",
            ours.eax
        );
        assert_eq!(
            c_bin.to_bits(),
            ours.c.to_bits(),
            "case {ci}: c-cell binary={:#010x} ours={:#010x}",
            c_bin.to_bits(),
            ours.c.to_bits()
        );

        let exit = if fc.c.to_bits() != 0 {
            E_CNZ
        } else if !(f(fc.fa0) < 0.0) {
            E_FA0
        } else {
            let cv = swim_fall_vel(fc.fa0, fc.flags) as f32;
            if f(fc.a) + f(fc.b) < f(cv) {
                E_AB
            } else if f(fc.a) > f(cv) {
                E_AGTC
            } else {
                let m = (f(cv) - f(fc.a)) * f(fc.f84);
                let vs = f(fc.vec[1]) * f(fc.vec[1]) + f(fc.vec[0]) * f(fc.vec[0]);
                if m * m > vs {
                    E_M1
                } else {
                    E_M0
                }
            }
        };
        exits[exit] += 1;
        checked += 1;
    }
    eprintln!(
        "fall_feasible: {checked} cases bit-exact; exits [c!=0={} fa0>=0={} a+b<c={} a>c={} \
         M2>v={} M2<=v={}]",
        exits[0], exits[1], exits[2], exits[3], exits[4], exits[5]
    );
    for (i, h) in exits.iter().enumerate() {
        assert!(*h > 0, "feasibility exit {i} never hit");
    }
}

/// `0x635ea0` and `0x635f10`: the record and the restored fields.
#[test]
fn state_copies_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    let this_va = emu.alloc(0x200);
    let this2_va = emu.alloc(0x200);
    let buf_va = emu.alloc(0x40);
    let mut s = 0x57a7_e000u32;
    let mut checked = 0u32;

    for _case in 0..300u32 {
        let src: [u32; 14] = core::array::from_fn(|_| next(&mut s));
        for (i, off) in SNAPSHOT_FIELDS.iter().enumerate() {
            emu.write_u32(this_va + u64::from(*off), src[i]);
        }
        for k in 0..14u64 {
            emu.write_u32(buf_va + k * 4, 0x7fc0_0001);
        }
        emu.call(0x635ea0, this_va as u32, 0, &[buf_va as u32]);
        let buf_bin: [u32; 14] = core::array::from_fn(|i| emu.read_u32(buf_va + (i as u64) * 4));
        let buf_ours = state_snapshot(&src);
        for i in 0..14 {
            assert_eq!(
                buf_bin[i], buf_ours[i],
                "snapshot buf[{i}] (this+{:#x}) binary={:#010x} ours={:#010x}",
                SNAPSHOT_FIELDS[i], buf_bin[i], buf_ours[i]
            );
        }

        let rbuf: [u32; 14] = core::array::from_fn(|_| next(&mut s));
        for k in 0..14u64 {
            emu.write_u32(buf_va + k * 4, rbuf[k as usize]);
        }
        for off in SNAPSHOT_FIELDS.iter() {
            emu.write_u32(this2_va + u64::from(*off), 0x7fc0_0001);
        }
        emu.call(0x635f10, this2_va as u32, 0, &[buf_va as u32]);
        let this_bin: [u32; 14] =
            core::array::from_fn(|i| emu.read_u32(this2_va + u64::from(SNAPSHOT_FIELDS[i])));
        let this_ours = state_restore(&rbuf);
        for i in 0..14 {
            assert_eq!(
                this_bin[i], this_ours[i],
                "restore this+{:#x} binary={:#010x} ours={:#010x}",
                SNAPSHOT_FIELDS[i], this_bin[i], this_ours[i]
            );
        }

        emu.call(0x635ea0, this_va as u32, 0, &[buf_va as u32]);
        for off in SNAPSHOT_FIELDS.iter() {
            emu.write_u32(this2_va + u64::from(*off), 0xdead_beef);
        }
        emu.call(0x635f10, this2_va as u32, 0, &[buf_va as u32]);
        for (i, off) in SNAPSHOT_FIELDS.iter().enumerate() {
            let v = emu.read_u32(this2_va + u64::from(*off));
            assert_eq!(
                v, src[i],
                "inverse this+{off:#x} = {v:#010x}, expected {:#010x}",
                src[i]
            );
        }
        checked += 1;
    }
    eprintln!("state_copies: {checked} rounds bit-exact (snapshot + restore + inverse, 2 kernels)");
}
