//! The setters of `systems/uicamera/transitions.md#arming-a-transition` run whole from the exe, one
//! test per setter, and the integrators `ShiftPitch` (`0x510120`) and `ShiftYaw` (`0x5102b0`) run
//! whole on the camera object's words.
//!
//! Each setter is a `__thiscall` on the camera object. A test seeds the channel's live word, the
//! mode word and the six descriptor dwords, runs the exe's code, and reads the descriptor and the
//! return value back. An outer setter reads its speed CVar through a pointer chain, which the test
//! builds: a small struct holds the speed at `+0x24` and the channel's global points to it.
//! Every branch is chosen from the high bits of an LCG, and each branch is tallied and required to
//! be hit: both early-outs, the full arm, the inner's within-0.001 return and both directions of
//! the wrap at the seam of π.
#![allow(
    clippy::float_cmp,
    clippy::too_many_arguments,
    clippy::needless_range_loop
)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::camera_transitions::{
    arm_transition, begin_transition, shift_pitch, shift_yaw, CamState, ARM_FOV, ARM_GROUND_PITCH,
    ARM_PITCH, ARM_PIVOT_BIAS, ARM_YAW,
};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// f32 in `[lo, hi)` from the LCG high bits.
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

fn eqf(bin: u32, mine: u32, ctx: &str) {
    assert_eq!(
        bin,
        mine,
        "{ctx}: binary {:#010x} ({}) vs rust {:#010x} ({})",
        bin,
        f32::from_bits(bin),
        mine,
        f32::from_bits(mine),
    );
}

/// A camera channel's fixed layout (`systems/uicamera/transitions.md#the-channels`).
#[derive(Clone, Copy)]
struct Chan {
    outer: u64,
    inner: u64,
    live_off: u64,
    arm: u32,
    base: u64,
    gptr: u64,
}

const CHANS: [Chan; 5] = [
    Chan {
        outer: 0x512270,
        inner: 0x5123c0,
        live_off: 0x10c,
        arm: ARM_FOV,
        base: 0x220,
        gptr: 0xbe1068,
    },
    Chan {
        outer: 0x512490,
        inner: 0x5125e0,
        live_off: 0x108,
        arm: ARM_GROUND_PITCH,
        base: 0x1a8,
        gptr: 0xbe0fc0,
    },
    Chan {
        outer: 0x512830,
        inner: 0x512980,
        live_off: 0x0f4,
        arm: ARM_PITCH,
        base: 0x1d8,
        gptr: 0xbe0ce4,
    },
    Chan {
        outer: 0x512a50,
        inner: 0x512ba0,
        live_off: 0x104,
        arm: ARM_PIVOT_BIAS,
        base: 0x1f0,
        gptr: 0xbe0fc8,
    },
    Chan {
        outer: 0x512c70,
        inner: 0x512dc0,
        live_off: 0x0f0,
        arm: ARM_YAW,
        base: 0x208,
        gptr: 0xbe1070,
    },
];

/// `this` must hold the live field (max `+0x10c`), flags (`+0x90`), and a descriptor whose top
/// offset is `base+0x14` (max `0x220+0x14 = 0x234`); 0x280 covers it with slack.
const THIS_SIZE: u64 = 0x280;

/// Write the channel's full pre-state into `this`.
fn put_state(emu: &mut Emu, this: u64, c: &Chan, st: &CamState) {
    emu.write_u32(this + c.live_off, st.live);
    emu.write_u32(this + 0x90, st.flags);
    emu.write_u32(this + c.base, st.start_ms);
    emu.write_u32(this + c.base + 0x04, st.duration);
    emu.write_u32(this + c.base + 0x08, st.target);
    emu.write_u32(this + c.base + 0x0c, st.from);
    emu.write_u32(this + c.base + 0x10, st.factor_memo);
    emu.write_u32(this + c.base + 0x14, st.delay_memo);
}
/// Read the channel's full post-state back out of `this`.
fn get_state(emu: &Emu, this: u64, c: &Chan) -> CamState {
    CamState {
        live: emu.read_u32(this + c.live_off),
        flags: emu.read_u32(this + 0x90),
        start_ms: emu.read_u32(this + c.base),
        duration: emu.read_u32(this + c.base + 0x04),
        target: emu.read_u32(this + c.base + 0x08),
        from: emu.read_u32(this + c.base + 0x0c),
        factor_memo: emu.read_u32(this + c.base + 0x10),
        delay_memo: emu.read_u32(this + c.base + 0x14),
    }
}

fn assert_state(bin: &CamState, mine: &CamState, eax: u32, r: u32, ctx: &str) {
    assert_eq!(eax, r, "{ctx}: return code binary {eax} vs rust {r}");
    eqf(bin.live, mine.live, &format!("{ctx} live"));
    assert_eq!(bin.flags, mine.flags, "{ctx} flags");
    assert_eq!(bin.start_ms, mine.start_ms, "{ctx} start_ms");
    eqf(bin.duration, mine.duration, &format!("{ctx} duration"));
    eqf(bin.target, mine.target, &format!("{ctx} target"));
    eqf(bin.from, mine.from, &format!("{ctx} start"));
    eqf(
        bin.factor_memo,
        mine.factor_memo,
        &format!("{ctx} factor_memo"),
    );
    eqf(
        bin.delay_memo,
        mine.delay_memo,
        &format!("{ctx} delay_memo"),
    );
}

/// How many wrap iterations (lo, hi) the request triggers — for the ±π seam tally.
fn wrap_kind(live: f32, target: f32) -> (bool, bool) {
    let pi = f32::from_bits(0x4049_0fdb) as f64;
    let two_pi = f32::from_bits(0x40c9_0fdb) as f64;
    let mut l = f64::from(live);
    let t = f64::from(target);
    let mut lo = false;
    let mut hi = false;
    while t - l < -pi {
        l -= two_pi;
        lo = true;
    }
    while t - l > pi {
        l += two_pi;
        hi = true;
    }
    (lo, hi)
}

// ---------- the outer setters ----------

struct OuterCase {
    pre: CamState,
    speed: f32,
    target: f32,
    delay: f32,
    factor: f32,
    now: i32,
}

/// Runs one channel's outer setter: seeds the state and the speed, calls the exe, and compares the
/// descriptor and the return value with `arm_transition`.
fn run_outer(c: &Chan) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(THIS_SIZE);
    // The speed CVar's chain: *(gptr) = s_struct, speed = *(s_struct + 0x24).
    let s_struct = emu.alloc(0x40);
    emu.write_u32(c.gptr, s_struct as u32);

    let mut cases: Vec<OuterCase> = Vec::new();
    let blank = CamState::default();

    // one case per branch, then both wrap seams
    // already arming: the bit is set and the memo words match the request.
    cases.push(OuterCase {
        pre: CamState {
            live: 1.0f32.to_bits(),
            flags: c.arm,
            target: 0.5f32.to_bits(),
            delay_memo: 0.25f32.to_bits(),
            factor_memo: 2.0f32.to_bits(),
            ..blank
        },
        speed: 0.016,
        target: 0.5,
        delay: 0.25,
        factor: 2.0,
        now: 1000,
    });
    // already-there: target == live, no flag.
    cases.push(OuterCase {
        pre: CamState {
            live: 0.75f32.to_bits(),
            ..blank
        },
        speed: 0.016,
        target: 0.75,
        delay: 0.25,
        factor: 2.0,
        now: 1000,
    });
    // full re-arm, no wrap.
    cases.push(OuterCase {
        pre: CamState {
            live: 0.0f32.to_bits(),
            ..blank
        },
        speed: 0.016,
        target: 1.5,
        delay: 0.3,
        factor: 1.25,
        now: 500,
    });
    // full re-arm, arming flag set but caches mismatch (falls through early-out 1).
    cases.push(OuterCase {
        pre: CamState {
            live: 0.0f32.to_bits(),
            flags: c.arm,
            target: 0.5f32.to_bits(),
            delay_memo: 9.0f32.to_bits(),
            factor_memo: 9.0f32.to_bits(),
            ..blank
        },
        speed: 0.02,
        target: 1.5,
        delay: 0.3,
        factor: 1.25,
        now: 500,
    });
    // upper wrap seam: target - live > π.
    cases.push(OuterCase {
        pre: CamState {
            live: 0.0f32.to_bits(),
            ..blank
        },
        speed: 0.016,
        target: 4.0,
        delay: 0.2,
        factor: 1.0,
        now: 250,
    });
    // lower wrap seam: target - live < -π.
    cases.push(OuterCase {
        pre: CamState {
            live: 0.0f32.to_bits(),
            ..blank
        },
        speed: 0.016,
        target: -4.0,
        delay: 0.2,
        factor: 1.0,
        now: 250,
    });
    // negative delay: the start time is now - __ftol(delay * -1000), now minus a positive count.
    cases.push(OuterCase {
        pre: CamState {
            live: 2.0f32.to_bits(),
            flags: 0xdead_0000 & !c.arm,
            ..blank
        },
        speed: 0.033,
        target: -2.0,
        delay: -0.4,
        factor: 3.0,
        now: 5000,
    });

    // --- fuzz grid straddling each branch + wrap seam ---
    let mut s = 0x5e77_0000u32 ^ (c.outer as u32);
    for _ in 0..4000 {
        let decide = (next(&mut s) >> 8) % 4;
        let live = rng(&mut s, -7.0, 7.0);
        // for the "already there" path push target onto live; for "already arming" reuse the request
        // AND match the descriptor's stored target / caches (all three must be within dt to fire).
        let (flags, target, pre_target, delay_memo, factor_memo);
        let delay = rng(&mut s, -1.5, 1.5);
        let factor = rng(&mut s, 0.1, 4.0);
        match decide {
            0 => {
                // candidate already-arming: arm bit set, desc.target + both caches = the request.
                flags = c.arm | (next(&mut s) & !c.arm);
                target = rng(&mut s, -7.0, 7.0);
                pre_target = target.to_bits();
                delay_memo = delay.to_bits();
                factor_memo = factor.to_bits();
            }
            1 => {
                // candidate already-there: target within a hair of live.
                flags = next(&mut s) & !c.arm;
                target = live + rng(&mut s, -0.0005, 0.0005);
                pre_target = rng(&mut s, -7.0, 7.0).to_bits();
                delay_memo = rng(&mut s, -2.0, 2.0).to_bits();
                factor_memo = rng(&mut s, 0.1, 4.0).to_bits();
            }
            _ => {
                flags = next(&mut s);
                target = rng(&mut s, -7.0, 7.0);
                pre_target = rng(&mut s, -7.0, 7.0).to_bits();
                delay_memo = rng(&mut s, -2.0, 2.0).to_bits();
                factor_memo = rng(&mut s, 0.1, 4.0).to_bits();
            }
        }
        cases.push(OuterCase {
            pre: CamState {
                live: live.to_bits(),
                flags,
                start_ms: next(&mut s),
                duration: rng(&mut s, -5.0, 5.0).to_bits(),
                target: pre_target,
                from: rng(&mut s, -7.0, 7.0).to_bits(),
                factor_memo,
                delay_memo,
            },
            speed: rng(&mut s, 0.004, 0.06),
            target,
            delay,
            factor,
            now: (next(&mut s) >> 8) as i32,
        });
    }

    let (mut arming, mut there, mut full, mut wlo, mut whi, mut inner0) =
        (0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
    for (i, cc) in cases.iter().enumerate() {
        put_state(&mut emu, this, c, &cc.pre);
        emu.write_u32(s_struct + 0x24, cc.speed.to_bits());
        emu.clear_calls();
        let eax = emu.call(
            c.outer,
            this as u32,
            0,
            &[
                cc.target.to_bits(),
                cc.delay.to_bits(),
                cc.factor.to_bits(),
                cc.now as u32,
            ],
        );
        let bin = get_state(&emu, this, c);

        let mut mine = cc.pre;
        let r = arm_transition(
            &mut mine, c.arm, cc.speed, cc.target, cc.delay, cc.factor, cc.now,
        );
        assert_state(
            &bin,
            &mine,
            eax,
            r,
            &format!("outer {:#x} case {i}", c.outer),
        );

        // tally (from the model's classification of this case)
        let live_f = f32::from_bits(cc.pre.live);
        let (lo, hi) = wrap_kind(live_f, cc.target);
        if lo {
            wlo += 1;
        }
        if hi {
            whi += 1;
        }
        // re-derive which branch fired, matching arm_transition's order.
        let wrapped = {
            let mut l = f64::from(live_f);
            let t = f64::from(cc.target);
            let pi = f32::from_bits(0x4049_0fdb) as f64;
            let two_pi = f32::from_bits(0x40c9_0fdb) as f64;
            while t - l < -pi {
                l -= two_pi;
            }
            while t - l > pi {
                l += two_pi;
            }
            l
        };
        let dt = f32::from_bits(0x3a83_126f) as f64;
        let arm_hit = cc.pre.flags & c.arm != 0
            && (f64::from(f32::from_bits(cc.pre.target)) - f64::from(cc.target)).abs() < dt
            && (f64::from(f32::from_bits(cc.pre.delay_memo)) - f64::from(cc.delay)).abs() < dt
            && (f64::from(f32::from_bits(cc.pre.factor_memo)) - f64::from(cc.factor)).abs() < dt;
        if arm_hit {
            arming += 1;
        } else if (wrapped - f64::from(cc.target)).abs() < dt {
            there += 1;
        } else {
            full += 1;
            if r == 0 {
                inner0 += 1; // the inner's dt early-out fired after the f32 round-trip
            }
        }
    }
    eprintln!(
        "outer {:#x}: {} bit-exact; arming={arming} there={there} full={full} \
         inner_dt0={inner0} wrap_lo={wlo} wrap_hi={whi}",
        c.outer,
        cases.len()
    );
    assert!(
        arming > 0,
        "outer {:#x}: already-arming early-out never hit",
        c.outer
    );
    assert!(
        there > 0,
        "outer {:#x}: already-there early-out never hit",
        c.outer
    );
    assert!(full > 0, "outer {:#x}: full re-arm path never hit", c.outer);
    assert!(
        wlo > 0 && whi > 0,
        "outer {:#x}: both wrap seams not covered",
        c.outer
    );
}

// ---------- the inner setters ----------

struct InnerCase {
    pre: CamState,
    target: f32,
    duration_bits: u32,
    start_bits: u32,
}

/// Runs one channel's inner setter directly and compares the descriptor and the return value with
/// `begin_transition`.
fn run_inner(c: &Chan) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(THIS_SIZE);

    let mut cases: Vec<InnerCase> = Vec::new();
    let blank = CamState::default();

    // dt early-out: target == live.
    cases.push(InnerCase {
        pre: CamState {
            live: 0.5f32.to_bits(),
            ..blank
        },
        target: 0.5,
        duration_bits: 0x1111_2222,
        start_bits: 0x3333_4444,
    });
    // full store, no wrap.
    cases.push(InnerCase {
        pre: CamState {
            live: 0.0f32.to_bits(),
            flags: 0x00ab_cd00,
            ..blank
        },
        target: 1.0,
        duration_bits: 0xdead_beef,
        start_bits: 0x0000_007b,
    });
    // upper wrap seam.
    cases.push(InnerCase {
        pre: CamState {
            live: 0.0f32.to_bits(),
            ..blank
        },
        target: 4.0,
        duration_bits: 0xa5a5_a5a5,
        start_bits: 0x0000_0064,
    });
    // lower wrap seam.
    cases.push(InnerCase {
        pre: CamState {
            live: 0.0f32.to_bits(),
            ..blank
        },
        target: -4.0,
        duration_bits: 0x5a5a_5a5a,
        start_bits: 0xffff_ff9c,
    });

    let mut s = 0x0001_0000_u32.wrapping_add(c.inner as u32);
    for _ in 0..4000 {
        let live = rng(&mut s, -7.0, 7.0);
        let target = if (next(&mut s) >> 8).is_multiple_of(4) {
            // straddle the dt early-out.
            live + rng(&mut s, -0.0005, 0.0005)
        } else {
            rng(&mut s, -7.0, 7.0)
        };
        cases.push(InnerCase {
            pre: CamState {
                live: live.to_bits(),
                flags: next(&mut s),
                start_ms: next(&mut s),
                duration: next(&mut s),
                target: rng(&mut s, -7.0, 7.0).to_bits(),
                from: rng(&mut s, -7.0, 7.0).to_bits(),
                factor_memo: next(&mut s),
                delay_memo: next(&mut s),
            },
            target,
            duration_bits: next(&mut s),
            start_bits: next(&mut s),
        });
    }

    let (mut ret0, mut ret1, mut wlo, mut whi) = (0u32, 0u32, 0u32, 0u32);
    for (i, cc) in cases.iter().enumerate() {
        put_state(&mut emu, this, c, &cc.pre);
        emu.clear_calls();
        let eax = emu.call(
            c.inner,
            this as u32,
            0,
            &[cc.target.to_bits(), cc.duration_bits, cc.start_bits],
        );
        let bin = get_state(&emu, this, c);

        let mut mine = cc.pre;
        let r = begin_transition(&mut mine, c.arm, cc.target, cc.duration_bits, cc.start_bits);
        assert_state(
            &bin,
            &mine,
            eax,
            r,
            &format!("inner {:#x} case {i}", c.inner),
        );

        if r == 0 {
            ret0 += 1;
        } else {
            ret1 += 1;
        }
        let (lo, hi) = wrap_kind(f32::from_bits(cc.pre.live), cc.target);
        if lo {
            wlo += 1;
        }
        if hi {
            whi += 1;
        }
    }
    eprintln!(
        "inner {:#x}: {} bit-exact; dt_early(ret0)={ret0} stored(ret1)={ret1} \
         wrap_lo={wlo} wrap_hi={whi}",
        c.inner,
        cases.len()
    );
    assert!(ret0 > 0, "inner {:#x}: dt early-out never hit", c.inner);
    assert!(ret1 > 0, "inner {:#x}: full-store path never hit", c.inner);
    assert!(
        wlo > 0 && whi > 0,
        "inner {:#x}: both wrap seams not covered",
        c.inner
    );
}

// ---------- one test per setter, named by its address ----------

#[test]
fn cam_512270() {
    run_outer(&CHANS[0]);
}
#[test]
fn cam_5123c0() {
    run_inner(&CHANS[0]);
}
#[test]
fn cam_512490() {
    run_outer(&CHANS[1]);
}
#[test]
fn cam_5125e0() {
    run_inner(&CHANS[1]);
}
#[test]
fn cam_512830() {
    run_outer(&CHANS[2]);
}
#[test]
fn cam_512980() {
    run_inner(&CHANS[2]);
}
#[test]
fn cam_512a50() {
    run_outer(&CHANS[3]);
}
#[test]
fn cam_512ba0() {
    run_inner(&CHANS[3]);
}
#[test]
fn cam_512c70() {
    run_outer(&CHANS[4]);
}
#[test]
fn cam_512dc0() {
    run_inner(&CHANS[4]);
}
fn eqf_f32(bin: u32, mine: f32, ctx: &str) {
    assert_eq!(
        bin,
        mine.to_bits(),
        "{ctx}: binary {:#010x} ({}) vs rust {:#010x} ({mine})",
        bin,
        f32::from_bits(bin),
        mine.to_bits()
    );
}

// ---------- the integrators ----------

/// Seeds the three words at `off` on the camera object, runs the integrator on `delta`, and reads
/// the three words back as (target, from, live).
fn tri_case(
    emu: &mut Emu,
    fnva: u64,
    this: u64,
    off: [u64; 3],
    delta: f32,
    init: [f32; 3],
) -> [f32; 3] {
    for k in 0..3 {
        emu.write_u32(this + off[k], init[k].to_bits());
    }
    emu.call(fnva, this as u32, 0, &[delta.to_bits()]);
    [
        emu.read_f32(this + off[0]),
        emu.read_f32(this + off[1]),
        emu.read_f32(this + off[2]),
    ]
}

#[test]
fn cam_510120() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x300);
    let off = [0x1e0u64, 0x1e4, 0xf4];
    let mut s = 0x0510_1200u32;
    let (mut clamp_hi, mut clamp_lo, mut mid, mut wrapped) = (0u32, 0u32, 0u32, 0u32);
    let mut n = 0u32;
    for _ in 0..6000 {
        let delta = rng(&mut s, -4.0, 4.0);
        let init = [
            rng(&mut s, -2.0, 2.0),
            rng(&mut s, -8.0, 8.0),
            rng(&mut s, -8.0, 8.0),
        ];
        let got = tri_case(&mut emu, 0x510120, this, off, delta, init);
        let mine = shift_pitch(delta, init[0], init[1], init[2]);
        eqf_f32(
            got[0].to_bits(),
            mine.target,
            &format!("510120 target d={delta} i={init:?}"),
        );
        eqf_f32(
            got[1].to_bits(),
            mine.from,
            &format!("510120 from d={delta} i={init:?}"),
        );
        eqf_f32(
            got[2].to_bits(),
            mine.live,
            &format!("510120 live d={delta} i={init:?}"),
        );
        let v = delta as f64 + init[0] as f64;
        if v >= 1.5533431 {
            clamp_hi += 1;
        } else if v < -1.5533431 {
            clamp_lo += 1;
        } else {
            mid += 1;
        }
        // The wrap to within π of the target never fires here: all three words are clamped to
        // ±89 degrees (±1.5533), so they differ by at most 3.107 < π. The run still checks the
        // wrap chain's no-op.
        if mine.from != mine.live {
            wrapped += 1;
        }
        n += 1;
    }
    eprintln!("cam_510120: {n} bit-exact (clamp_hi={clamp_hi} clamp_lo={clamp_lo} mid={mid} slaves-differ={wrapped}); ±π wrap unreachable post-clamp");
    assert!(
        clamp_hi > 0 && clamp_lo > 0 && mid > 0,
        "510120 coverage gap"
    );
}

#[test]
fn cam_5102b0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x300);
    let off = [0x210u64, 0x214, 0xf0];
    let mut s = 0x0510_2b00u32;
    let mut n = 0u32;
    let mut wrapped = 0u32;
    for _ in 0..6000 {
        let delta = rng(&mut s, -10.0, 10.0);
        let init = [
            rng(&mut s, -10.0, 10.0),
            rng(&mut s, -10.0, 10.0),
            rng(&mut s, -10.0, 10.0),
        ];
        let got = tri_case(&mut emu, 0x5102b0, this, off, delta, init);
        let mine = shift_yaw(delta, init[0], init[1], init[2]);
        eqf_f32(
            got[0].to_bits(),
            mine.target,
            &format!("5102b0 target d={delta} i={init:?}"),
        );
        eqf_f32(
            got[1].to_bits(),
            mine.from,
            &format!("5102b0 from d={delta} i={init:?}"),
        );
        eqf_f32(
            got[2].to_bits(),
            mine.live,
            &format!("5102b0 live d={delta} i={init:?}"),
        );
        // The target is normalized into [0, 2π).
        assert!(
            (0.0..6.2831856).contains(&mine.target),
            "5102b0 target out of range: {}",
            mine.target
        );
        // A wrap that fires puts the start or live word outside [0, 2π): it was normalized into
        // it, then shifted by 2π to sit within π of the target.
        let outside = |w: f32| !(0.0..=6.2831856).contains(&w);
        if outside(mine.from) || outside(mine.live) {
            wrapped += 1;
        }
        n += 1;
    }
    eprintln!("cam_5102b0: {n} bit-exact (norm+wrap; wrapped={wrapped})");
    assert!(wrapped > 0, "5102b0 wrap branch never hit");
}
