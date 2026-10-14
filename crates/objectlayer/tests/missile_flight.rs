//! The missile flight math against the exe. `CMissile::Advance` (`0x61e2a0`), `CMissile::Aim`
//! (`0x61d230`) and `DrawFishingLine` (`0x61f780`) run whole in the emulator on synthesized
//! missile, target and descriptor structs, with their object lookups, getters and drawing calls
//! stubbed, and the position, the aim point and the 65 vertices are compared bit for bit. The
//! sine and cosine are the host's on both sides.
//!
//! - Advance has three outcomes: a speed of 0 stalls, a step past the destination with bit 1 clear
//!   arrives, and otherwise the position moves; each is probed, then a random sweep counts them.
//! - The quadratic Aim returns the target point for a target that is not moving, for `a * a` under
//!   its floor and when no positive root exists, and otherwise the lead `T + V' * best`.
//! - The arc Aim seeds the generator, predicts its return and compares the rotated aim, and
//!   probes the length gate at a length whose `f32` equals the floor while the live value is under.
//! - The line runs in both device channel orders over random ends, heights and colours.
//!
//! Further tests probe every gate with NaN, infinities, zeros and the thresholds.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::missile_flight::{
    curved_arc_angle, effect_beam_trail, missile_advance, missile_target_intercept_curved,
    missile_target_intercept_quadratic, AdvanceStep, BeamVertex,
};
use common::{assert_f32, next, next_down, next_up, specials};

const ADVANCE_FN: u64 = 0x61e2a0;
const INTERCEPT_FN: u64 = 0x61d230;
const BEAM_FN: u64 = 0x61f780;
const PRNG_FN: u64 = 0x4531e0;
const PRNG_STATE: u64 = 0x88_2664;
const ARRIVAL_FN: u64 = 0x61e1d0;

/// Stubs the callees of `0x61e2a0` except the vector helpers `0x4549a0`, `0x4549c0` and `0x4549f0`,
/// which run for real. The object lookup returns null, so the homing recompute and the impact
/// block are skipped, and the arrival handler, the clock and the orientation tail do nothing.
/// `pop_args` is the callee's `ret N` over 4.
fn stub_advance(e: &mut Emu) {
    e.stub(0x468460, 0, 3); // `ClntObjMgrObjectPtr` returns null
    e.stub(INTERCEPT_FN, 0, 0); // the homing recompute, which is skipped
    e.stub(ARRIVAL_FN, 1, 2); // the arrival handler (ret 8), whose call is recorded
    e.stub(0x42c010, 0, 0); // the clock
                            // The orientation tail and the impact visuals do nothing and clean the stack.
    for (addr, pop) in [
        (0x513a40u64, 1u32),
        (0x4078e0, 1),
        (0x4b1df0, 2),
        (0x711a20, 3),
        (0x5fe2f0, 1),
        (0x61e9c0, 0),
        (0x710620, 1),
        (0x6717d0, 3),
        (0x710b90, 1),
        (0x710c50, 1),
        (0x710600, 1),
        (0x671010, 3),
        (0x7a5b10, 2),
        (0x7103d0, 2),
        (0x713640, 1),
        (0x4589a0, 0),
        (0x5f8cf0, 1),
    ] {
        e.stub(addr, 0, pop);
    }
}

#[allow(clippy::too_many_arguments)]
fn check_advance(
    e: &mut Emu,
    this_va: u64,
    cur: [f32; 3],
    dest: [f32; 3],
    speed: f32,
    dt: f32,
    flags: u8,
    ctx: &str,
) {
    e.write_u32(this_va + 0x1c, speed.to_bits());
    for (i, &c) in cur.iter().enumerate() {
        e.write_u32(this_va + 0x20 + 4 * i as u64, c.to_bits());
    }
    for (i, &d) in dest.iter().enumerate() {
        e.write_u32(this_va + 0x2c + 4 * i as u64, d.to_bits());
    }
    e.write_u32(this_va + 0x38, u32::from(flags)); // bit 1 suppresses the arrival
    e.write_u32(this_va + 0x74, 0); // no impact timer, so the impact block is skipped
    e.write_u32(this_va + 0x44, 0); // no trail
    e.write_u32(this_va + 0x7c, 0); // the effect callback slot
    e.clear_calls();
    e.call(ADVANCE_FN, this_va as u32, 0, &[dt.to_bits(), 0]);

    let bin = [
        e.read_f32(this_va + 0x20),
        e.read_f32(this_va + 0x24),
        e.read_f32(this_va + 0x28),
    ];
    let arrived = e.calls().iter().any(|c| c.addr == ARRIVAL_FN);
    match missile_advance(cur, dest, speed, dt, flags) {
        AdvanceStep::Stalled => {
            for i in 0..3 {
                assert_f32(bin[i], cur[i], &format!("{ctx}: stalled pos[{i}]"));
            }
            assert!(!arrived, "{ctx}: stalled but arrival handler fired");
        }
        AdvanceStep::Arrived => {
            for i in 0..3 {
                assert_f32(bin[i], cur[i], &format!("{ctx}: arrived pos[{i}]"));
            }
            assert!(arrived, "{ctx}: predicted arrival but 0x61e1d0 not called");
        }
        AdvanceStep::Moved(v) => {
            for i in 0..3 {
                assert_f32(bin[i], v[i], &format!("{ctx}: moved pos[{i}]"));
            }
            assert!(!arrived, "{ctx}: predicted move but arrival handler fired");
        }
    }
}

#[test]
fn missile_advance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    stub_advance(&mut e);
    let this_va = e.alloc(0x140);

    // One probe per shape.
    check_advance(
        &mut e,
        this_va,
        [0.0; 3],
        [10.0, 0.0, 0.0],
        0.0,
        1.0,
        0,
        "speed=0 stalled",
    );
    // A normal step: far from the destination, a small step.
    check_advance(
        &mut e,
        this_va,
        [0.0; 3],
        [100.0, 0.0, 0.0],
        5.0,
        1.0,
        0,
        "normal",
    );
    check_advance(
        &mut e,
        this_va,
        [1.0, 2.0, 3.0],
        [9.0, -4.0, 12.0],
        7.5,
        0.5,
        0,
        "normal diag",
    );
    // The length gate reads the live register, not the stored float: with d = (0x3413ccb5,
    // 0x3413ccb5, 0x3413ce44) the live length is 2.384185749e-7, under eps = 2^-22, while its `f32`
    // equals eps. The exe skips the normalize and carries d; a gate on the `f32` would normalize.
    // Bit 1 of the flags suppresses the arrival so the gate is reached.
    check_advance(
        &mut e,
        this_va,
        [0.0; 3],
        [
            f32::from_bits(0x3413_ccb5),
            f32::from_bits(0x3413_ccb5),
            f32::from_bits(0x3413_ce44),
        ],
        1.0,
        1.0,
        2,
        "fst-no-pop len<eps carry-not-normalize",
    );
    // Arrival: the step passes the destination and bit 1 is clear.
    check_advance(
        &mut e,
        this_va,
        [0.0; 3],
        [0.1, 0.0, 0.0],
        100.0,
        1.0,
        0,
        "arrival",
    );
    // The same overshoot with bit 1 set advances.
    check_advance(
        &mut e,
        this_va,
        [0.0; 3],
        [0.1, 0.0, 0.0],
        100.0,
        1.0,
        0x2,
        "overshoot bit1",
    );
    // At the destination.
    check_advance(
        &mut e,
        this_va,
        [5.0, 5.0, 5.0],
        [5.0, 5.0, 5.0],
        3.0,
        1.0,
        0,
        "degenerate",
    );
    // A negative speed passes the zero gate and integrates.
    check_advance(
        &mut e,
        this_va,
        [0.0; 3],
        [10.0, 0.0, 0.0],
        -4.0,
        1.0,
        0,
        "neg speed",
    );

    // Random draws, counted by outcome.
    let mut seed = 0x0061_e2a0u32;
    let frange = |s: &mut u32, span: f32| -> f32 {
        let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        (m as f32 * 2.0 - 1.0) * span
    };
    let mut hits = [0u32; 3]; // stalled, arrived, moved
    for i in 0..4000 {
        let cur = [
            frange(&mut seed, 200.0),
            frange(&mut seed, 200.0),
            frange(&mut seed, 200.0),
        ];
        let dest = [
            frange(&mut seed, 200.0),
            frange(&mut seed, 200.0),
            frange(&mut seed, 200.0),
        ];
        // Sometimes exactly 0, otherwise a wide range that includes overshoot.
        let speed = match (next(&mut seed) >> 8) % 5 {
            0 => 0.0,
            _ => (f64::from(next(&mut seed) >> 8) / f64::from(1u32 << 24)) as f32 * 400.0,
        };
        let dt = (f64::from(next(&mut seed) >> 8) / f64::from(1u32 << 24)) as f32 * 2.0;
        let flags = ((next(&mut seed) >> 8) & 0x2) as u8; // bit 1 of the flags
        match missile_advance(cur, dest, speed, dt, flags) {
            AdvanceStep::Stalled => hits[0] += 1,
            AdvanceStep::Arrived => hits[1] += 1,
            AdvanceStep::Moved(_) => hits[2] += 1,
        }
        check_advance(
            &mut e,
            this_va,
            cur,
            dest,
            speed,
            dt,
            flags,
            &format!("fuzz {i}"),
        );
    }
    assert!(hits[0] > 0, "advance: stalled never exercised");
    assert!(hits[1] > 0, "advance: arrival never exercised");
    assert!(hits[2] > 0, "advance: normal move never exercised");
}

/// The regions of one synthesized missile and target, allocated once.
#[derive(Clone, Copy)]
struct IcptRegions {
    this: u64,
    tgt: u64,
    vtable: u64,
    island14: u64,
    flagstruct: u64,
    velsub: u64,
    pos_buf: u64,
}

fn icpt_alloc(e: &mut Emu) -> IcptRegions {
    let r = IcptRegions {
        this: e.alloc(0x140),
        tgt: e.alloc(0x140),
        vtable: e.alloc(0x40),
        island14: e.alloc(0x10),
        flagstruct: e.alloc(0x10),
        velsub: e.alloc(0x90),
        pos_buf: e.alloc(0x10),
    };
    // The target's vtable at +0 and, at vtable +0x14, the position getter, whose stub returns
    // `pos_buf` for the three floats `0x61d2b0` reads.
    e.write_u32(r.tgt, r.vtable as u32);
    e.write_u32(r.vtable + 0x14, r.island14 as u32);
    e.write_u32(r.tgt + 8, r.flagstruct as u32); // the flag struct of the moving gate
    e.write_u32(r.tgt + 0x118, r.velsub as u32); // the velocity object
    r
}

#[allow(clippy::too_many_arguments)]
fn check_intercept_quadratic(
    e: &mut Emu,
    r: &IcptRegions,
    p: [f32; 3],
    speed: f32,
    raw_pos: [f32; 3],
    tgt90: f32,
    tgt_bc: f32,
    vel_dir: [f32; 3],
    vel_scalar: f32,
    moving: bool,
    ctx: &str,
) {
    // The missile: speed, position, bit 3 clear (quadratic), and +0x3c = -1 to take the vtable
    // position path.
    e.write_u32(r.this + 0x1c, speed.to_bits());
    for (i, &v) in p.iter().enumerate() {
        e.write_u32(r.this + 0x20 + 4 * i as u64, v.to_bits());
    }
    e.write_u32(r.this + 0x38, 0); // bit 3 clear: the quadratic branch
    e.write_u32(r.this + 0x3c, 0xffff_ffff); // attachment id -1
                                             // Seed the aim slot with a sentinel; every quadratic outcome writes it.
    for i in 0..3 {
        e.write_u32(r.this + 0x2c + 4 * i as u64, 0xdead_0000 + i);
    }
    // The target position, read through the getter stub's return.
    for (i, &v) in raw_pos.iter().enumerate() {
        e.write_u32(r.pos_buf + 4 * i as u64, v.to_bits());
    }
    e.write_u32(r.tgt + 0x90, tgt90.to_bits());
    e.write_u32(r.tgt + 0xbc, tgt_bc.to_bits());
    e.write_u32(r.flagstruct + 8, if moving { 0x8 } else { 0 }); // bit 3 of [[tgt+8]+8]
                                                                 // The velocity object: the axis at +0x5c, +0x60, +0x64 and the scale at +0x84.
    e.write_u32(r.velsub + 0x5c, vel_dir[0].to_bits());
    e.write_u32(r.velsub + 0x60, vel_dir[1].to_bits());
    e.write_u32(r.velsub + 0x64, vel_dir[2].to_bits());
    e.write_u32(r.velsub + 0x84, vel_scalar.to_bits());

    e.call(INTERCEPT_FN, r.this as u32, 0, &[]);
    let bin = [
        e.read_f32(r.this + 0x2c),
        e.read_f32(r.this + 0x30),
        e.read_f32(r.this + 0x34),
    ];
    let ours = missile_target_intercept_quadratic(
        p, speed, raw_pos, tgt90, tgt_bc, vel_dir, vel_scalar, moving,
    );
    for i in 0..3 {
        assert_f32(bin[i], ours[i], &format!("{ctx}: aim[{i}]"));
    }
}

/// The outcome of a quadratic case, to count fuzz coverage.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum IcptOut {
    NotMoving,
    AFloor,
    ZeroDist,
    Lead,
    Other,
}

#[allow(clippy::too_many_arguments)]
fn classify_icpt(
    p: [f32; 3],
    speed: f32,
    raw_pos: [f32; 3],
    tgt90: f32,
    tgt_bc: f32,
    vel_dir: [f32; 3],
    vel_scalar: f32,
    moving: bool,
) -> IcptOut {
    if !moving {
        return IcptOut::NotMoving;
    }
    let aim = missile_target_intercept_quadratic(
        p, speed, raw_pos, tgt90, tgt_bc, vel_dir, vel_scalar, moving,
    );
    // The target point T with its height bias; the outcomes that write T return exactly this.
    let tz = (((tgt90 as f64 * tgt_bc as f64) * 0.75) + raw_pos[2] as f64) as f32;
    let t = [raw_pos[0], raw_pos[1], tz];
    if aim == t {
        // Tell a target on the missile from the gate fallbacks.
        let d = [
            (t[0] as f64 - p[0] as f64) as f32,
            (t[1] as f64 - p[1] as f64) as f32,
            (t[2] as f64 - p[2] as f64) as f32,
        ];
        if d == [0.0, 0.0, 0.0] {
            IcptOut::ZeroDist
        } else if speed.abs() < 0.07 {
            IcptOut::AFloor
        } else {
            IcptOut::Other
        }
    } else {
        IcptOut::Lead
    }
}

#[test]
fn missile_target_intercept_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let r = icpt_alloc(&mut e);
    e.stub(0x468460, r.tgt as u32, 3); // the object lookup returns the target
    e.stub(0x712cb0, 0, 1); // the attachment query, skipped by +0x3c = -1
    e.stub(0x712d50, 0, 2);
    // The position getter: thiscall(tgt, &out) returning `pos_buf`, whose floats are read.
    e.stub(r.island14, r.pos_buf as u32, 1);
    // `0x61eec0` and the `0x4549xx` vector helpers run for real.

    // A target that is not moving: T, with no intercept math.
    check_intercept_quadratic(
        &mut e,
        &r,
        [0.0; 3],
        30.0,
        [10.0, 20.0, 5.0],
        0.0,
        0.0,
        [1.0, 0.0, 0.0],
        2.0,
        false,
        "not-moving",
    );
    // A feasible lead: a fast missile, a crossing target, a positive root.
    check_intercept_quadratic(
        &mut e,
        &r,
        [0.0; 3],
        50.0,
        [100.0, 0.0, 0.0],
        0.0,
        0.0,
        [0.0, 1.0, 0.0],
        10.0,
        true,
        "lead crossing",
    );
    check_intercept_quadratic(
        &mut e,
        &r,
        [3.0, -2.0, 1.0],
        80.0,
        [40.0, 60.0, -10.0],
        0.0,
        0.0,
        [-1.0, -1.0, 0.5],
        6.0,
        true,
        "lead diag",
    );
    // The clamp: |V| > s, a target faster than the missile; still a positive root.
    check_intercept_quadratic(
        &mut e,
        &r,
        [0.0; 3],
        5.0,
        [50.0, 0.0, 0.0],
        0.0,
        0.0,
        [0.0, 1.0, 0.0],
        40.0,
        true,
        "clamp slow-missile",
    );
    // `a * a` under the floor: a tiny speed and no velocity write T.
    check_intercept_quadratic(
        &mut e,
        &r,
        [0.0; 3],
        0.01,
        [10.0, 0.0, 0.0],
        0.0,
        0.0,
        [0.0, 0.0, 0.0],
        0.0,
        true,
        "a-floor tiny speed",
    );
    // |d| = 0: the target on the missile leaves `best` at FLT_MAX, so T.
    check_intercept_quadratic(
        &mut e,
        &r,
        [7.0, 8.0, 9.0],
        30.0,
        [7.0, 8.0, 9.0],
        0.0,
        0.0,
        [1.0, 0.0, 0.0],
        3.0,
        true,
        "zero distance",
    );
    // The height bias `tgt90 * tgt_bc * 0.75` in the target point.
    check_intercept_quadratic(
        &mut e,
        &r,
        [0.0; 3],
        60.0,
        [20.0, 0.0, 4.0],
        2.0,
        3.0,
        [0.0, 1.0, 0.0],
        8.0,
        true,
        "z-bias lead",
    );

    // Random draws, counted by outcome.
    let mut seed = 0x0061_d230u32;
    let frange = |s: &mut u32, span: f32| -> f32 {
        let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        (m as f32 * 2.0 - 1.0) * span
    };
    let mut hits = std::collections::HashMap::<IcptOut, u32>::new();
    for i in 0..3000 {
        let p = [
            frange(&mut seed, 100.0),
            frange(&mut seed, 100.0),
            frange(&mut seed, 100.0),
        ];
        let raw = [
            frange(&mut seed, 100.0),
            frange(&mut seed, 100.0),
            frange(&mut seed, 100.0),
        ];
        let vel_dir = [
            frange(&mut seed, 1.0),
            frange(&mut seed, 1.0),
            frange(&mut seed, 1.0),
        ];
        let vel_scalar = (f64::from(next(&mut seed) >> 8) / f64::from(1u32 << 24)) as f32 * 30.0;
        let speed = match (next(&mut seed) >> 8) % 6 {
            0 => 0.01, // the zone of the floor on `a * a`
            _ => (f64::from(next(&mut seed) >> 8) / f64::from(1u32 << 24)) as f32 * 100.0 + 0.5,
        };
        let moving = (next(&mut seed) >> 8) & 1 == 1;
        *hits
            .entry(classify_icpt(
                p, speed, raw, 0.0, 0.0, vel_dir, vel_scalar, moving,
            ))
            .or_default() += 1;
        check_intercept_quadratic(
            &mut e,
            &r,
            p,
            speed,
            raw,
            0.0,
            0.0,
            vel_dir,
            vel_scalar,
            moving,
            &format!("fuzz {i}"),
        );
    }
    assert!(
        hits.get(&IcptOut::NotMoving).copied().unwrap_or(0) > 0,
        "icpt: not-moving never hit"
    );
    assert!(
        hits.get(&IcptOut::Lead).copied().unwrap_or(0) > 0,
        "icpt: lead never hit"
    );
    assert!(
        hits.get(&IcptOut::AFloor).copied().unwrap_or(0) > 0,
        "icpt: a-floor never hit"
    );
}

/// The arc branch (bit 3 set): the generator is seeded, and `0x4531e0`, `0x7be490` and the `fsin`
/// run for real. `rv` is predicted by calling `0x4531e0` with the same seed, then the state is
/// rewound so the call inside the function reproduces it. The sine and cosine are the host's on
/// both sides.
#[test]
fn missile_target_intercept_curved_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let r = icpt_alloc(&mut e);
    e.stub(0x468460, r.tgt as u32, 3);
    e.stub(0x712cb0, 0, 1);
    e.stub(0x712d50, 0, 2);
    e.stub(r.island14, r.pos_buf as u32, 1);
    // `0x4531e0`, `0x7be490` and `0x4549xx` run for real.

    let mut seed = 0x0c12_3457u32;
    let frange = |s: &mut u32, span: f32| -> f32 {
        let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        (m as f32 * 2.0 - 1.0) * span
    };
    let mut writes = 0u32;
    for i in 0..500 {
        let p = [
            frange(&mut seed, 50.0),
            frange(&mut seed, 50.0),
            frange(&mut seed, 50.0),
        ];
        let raw = [
            frange(&mut seed, 50.0),
            frange(&mut seed, 50.0),
            frange(&mut seed, 50.0),
        ];
        let speed = (f64::from(next(&mut seed) >> 8) / f64::from(1u32 << 24)) as f32 * 40.0 + 1.0;
        // Seed the generator, predict `rv`, then rewind the state.
        let acc = next(&mut seed);
        let idx = next(&mut seed);
        e.write_u32(PRNG_STATE, acc);
        e.write_u32(PRNG_STATE + 4, idx);
        let rv = e.call(PRNG_FN, PRNG_STATE as u32, 0, &[]);
        e.write_u32(PRNG_STATE, acc); // rewind
        e.write_u32(PRNG_STATE + 4, idx);

        // The missile: bit 3 set (the arc), +0x3c = -1, speed and position, no height bias.
        e.write_u32(r.this + 0x1c, speed.to_bits());
        for (k, &v) in p.iter().enumerate() {
            e.write_u32(r.this + 0x20 + 4 * k as u64, v.to_bits());
        }
        e.write_u32(r.this + 0x38, 0x8); // bit 3 set: the arc
        e.write_u32(r.this + 0x3c, 0xffff_ffff);
        for k in 0..3 {
            e.write_u32(r.this + 0x2c + 4 * k as u64, 0xbeef_0000 + k);
        }
        for (k, &v) in raw.iter().enumerate() {
            e.write_u32(r.pos_buf + 4 * k as u64, v.to_bits());
        }
        e.write_u32(r.tgt + 0x90, 0);
        e.write_u32(r.tgt + 0xbc, 0);

        e.call(INTERCEPT_FN, r.this as u32, 0, &[]);
        let bin = [
            e.read_f32(r.this + 0x2c),
            e.read_f32(r.this + 0x30),
            e.read_f32(r.this + 0x34),
        ];
        match missile_target_intercept_curved(p, speed, raw, 0.0, 0.0, rv) {
            Some(aim) => {
                writes += 1;
                for (k, (&b, &a)) in bin.iter().zip(aim.iter()).enumerate() {
                    assert_f32(b, a, &format!("curved {i}: aim[{k}]"));
                    assert!(
                        b.is_finite(),
                        "curved {i}: aim[{k}] non-finite {:#010x}",
                        b.to_bits()
                    );
                }
            }
            None => {
                // An early return leaves the aim slot at its sentinel.
                for (k, &b) in bin.iter().enumerate() {
                    assert_eq!(
                        b.to_bits(),
                        0xbeef_0000 + k as u32,
                        "curved {i}: expected no-write but aim[{k}] changed"
                    );
                }
            }
        }
    }
    assert!(
        writes > 0,
        "curved: every case took a degenerate early-out (no write exercised)"
    );

    // The length gate at `0x61d321` reads the live register: with d = (a, a, b) =
    // (0x3413ccb5, 0x3413ccb5, 0x3413ce44) the live length is 2.384185749e-7, under eps = 2^-22,
    // while its `f32` equals eps. The exe returns without writing; a gate on the `f32` would
    // go on into the arc. P = 0, so d is the target.
    {
        let a = f32::from_bits(0x3413_ccb5);
        let b = f32::from_bits(0x3413_ce44);
        let lensq = (f64::from(a) * f64::from(a) + f64::from(a) * f64::from(a))
            + f64::from(b) * f64::from(b);
        let len_live = lensq.sqrt();
        let eps = f32::from_bits(0x3480_0000);
        assert!(
            len_live < f64::from(eps) && len_live as f32 >= eps,
            "curved straddle must bracket EPS: len_live={len_live:.13e}"
        );
        let p = [0.0f32, 0.0, 0.0];
        let raw = [a, a, b];
        let speed = 10.0f32;
        e.write_u32(r.this + 0x1c, speed.to_bits());
        for (k, &v) in p.iter().enumerate() {
            e.write_u32(r.this + 0x20 + 4 * k as u64, v.to_bits());
        }
        e.write_u32(r.this + 0x38, 0x8); // bit 3 set: the arc
        e.write_u32(r.this + 0x3c, 0xffff_ffff);
        for k in 0..3 {
            e.write_u32(r.this + 0x2c + 4 * k as u64, 0xbeef_0000 + k);
        }
        for (k, &v) in raw.iter().enumerate() {
            e.write_u32(r.pos_buf + 4 * k as u64, v.to_bits());
        }
        e.write_u32(r.tgt + 0x90, 0);
        e.write_u32(r.tgt + 0xbc, 0);
        e.call(INTERCEPT_FN, r.this as u32, 0, &[]);
        let bin = [
            e.read_f32(r.this + 0x2c),
            e.read_f32(r.this + 0x30),
            e.read_f32(r.this + 0x34),
        ];
        match missile_target_intercept_curved(p, speed, raw, 0.0, 0.0, 0x1234_5678) {
            None => {
                for (k, &bv) in bin.iter().enumerate() {
                    assert_eq!(
                        bv.to_bits(),
                        0xbeef_0000 + k as u32,
                        "curved straddle: the exe returns early and leaves aim[{k}]"
                    );
                }
            }
            Some(aim) => {
                for (k, (&bv, &av)) in bin.iter().zip(aim.iter()).enumerate() {
                    assert_f32(bv, av, &format!("curved straddle: aim[{k}]"));
                }
            }
        }
    }

    // The angle helper stays finite for returns across the sign and mask edges.
    for raw_rv in [0u32, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0x1234_5678] {
        let a = curved_arc_angle(raw_rv);
        assert!(a.is_finite(), "curved_arc_angle({raw_rv:#x}) = {a}");
    }
}

#[derive(Clone, Copy)]
struct BeamRegions {
    this: u64,
    colorvec: u64,
    desc: u64,
    tgt: u64,
    vtable: u64,
    island_p1: u64,
    island_scale: u64,
    scale_slot: u64,
    ctx_obj: u64,
    vbuf: u64,
    caps: u64,
    p0_buf: u64,
    p1_buf: u64,
}

/// Builds the line's regions and stubs its callees; `__ftol` and the colour pack run for real.
fn beam_rig(e: &mut Emu) -> BeamRegions {
    let b = BeamRegions {
        this: e.alloc(0x140),
        colorvec: e.alloc(0x80),
        desc: e.alloc(0x40),
        tgt: e.alloc(0x140),
        vtable: e.alloc(0x40),
        island_p1: e.alloc(0x10),
        island_scale: e.alloc(0x10),
        scale_slot: e.alloc(0x10),
        ctx_obj: e.alloc(0x10),
        vbuf: e.alloc(65 * 16 + 16),
        caps: e.alloc(0x40),
        p0_buf: e.alloc(0x10),
        p1_buf: e.alloc(0x10),
    };
    // The target's vtable: +0x14 is the end-point getter and +0x1c the scale getter.
    e.write_u32(b.tgt, b.vtable as u32);
    e.write_u32(b.vtable + 0x14, b.island_p1 as u32);
    e.write_u32(b.vtable + 0x1c, b.island_scale as u32);
    // The scale getter `fld dword [scale_slot]; ret` returns the scale in `st0`.
    let mut island = vec![0xd9u8, 0x05];
    island.extend_from_slice(&(b.scale_slot as u32).to_le_bytes());
    island.push(0xc3);
    e.write(b.island_scale, &island);

    e.stub(0x7130e0, 1, 1); // the model event check, enabled
    e.stub(0x468460, b.tgt as u32, 3); // the guid lookup returns the target
                                       // The start read `0x7131b0`, thiscall(this, &out, fourcc), writes three floats through its
                                       // first argument.
    e.stub_write_through(0x7131b0, 0, b.p0_buf, 12, 0, 2);
    // The end read through vtable +0x14, thiscall(tgt, &out), writes three floats likewise.
    e.stub_write_through(b.island_p1, 0, b.p1_buf, 12, 0, 1);
    e.stub(0x58a140, b.ctx_obj as u32, 1); // begins the primitive and returns a draw context
    e.stub(0x58a080, b.vbuf as u32, 0); // locks the vertex buffer and returns our base
    e.stub(0x58a230, b.caps as u32, 0); // the device caps; the swap flag is read at +0x1c
    for (addr, pop) in [
        (0x58a0a0u64, 0u32),
        (0x58a7c0, 0),
        (0x589f40, 0),
        (0x589e60, 0),
        (0x58a810, 1),
        (0x589f50, 0),
    ] {
        e.stub(addr, 0, pop);
    }
    b
}

/// One line: runs `0x61f780` and compares all 65 vertices, the position bit for bit and the colour
/// word.
#[allow(clippy::too_many_arguments)]
fn check_beam(
    e: &mut Emu,
    b: &BeamRegions,
    p0: [f32; 3],
    p1: [f32; 3],
    scale: f32,
    tgt_bc: f32,
    chan: [f32; 3],
    swap: bool,
    ctx: &str,
) {
    e.write(b.p0_buf, &bytemuck_3(p0));
    e.write(b.p1_buf, &bytemuck_3(p1));
    e.write_u32(b.scale_slot, scale.to_bits());
    e.write_u32(b.tgt + 0xbc, tgt_bc.to_bits());
    e.write_u32(b.colorvec + 0x54, chan[0].to_bits());
    e.write_u32(b.colorvec + 0x58, chan[1].to_bits());
    e.write_u32(b.colorvec + 0x5c, chan[2].to_bits());
    e.write_u32(b.caps + 0x1c, u32::from(swap));
    e.write(b.vbuf, &[0u8; 65 * 16]);
    e.write_u32(b.desc + 0x10, 0);
    e.write_u32(b.desc + 0x14, 0);

    e.call(BEAM_FN, b.this as u32, b.colorvec as u32, &[b.desc as u32]);

    let ours = effect_beam_trail(p0, p1, scale, tgt_bc, chan, swap);
    for (vi, ov) in ours.iter().enumerate() {
        let base = b.vbuf + (vi * 16) as u64;
        let bv = BeamVertex {
            pos: [e.read_f32(base), e.read_f32(base + 4), e.read_f32(base + 8)],
            color: e.read_u32(base + 0xc),
        };
        for (k, (&bp, &op)) in bv.pos.iter().zip(ov.pos.iter()).enumerate() {
            assert_f32(bp, op, &format!("{ctx} swap={swap} v{vi} pos[{k}]"));
        }
        assert_eq!(
            bv.color, ov.color,
            "{ctx} swap={swap} v{vi} color: binary={:#010x} ours={:#010x}",
            bv.color, ov.color
        );
    }
}

#[test]
fn effect_beam_trail_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let b = beam_rig(&mut e);

    let mut seed = 0x0061_f780u32;
    let frange = |s: &mut u32, span: f32| -> f32 {
        let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        (m as f32 * 2.0 - 1.0) * span
    };
    let chan_draw = |s: &mut u32| -> f32 {
        // In-range and out-of-range values, for the clamp legs.
        match (next(s) >> 8) % 3 {
            0 => (f64::from(next(s) >> 8) / f64::from(1u32 << 24)) as f32, // [0,1)
            1 => (f64::from(next(s) >> 8) / f64::from(1u32 << 24)) as f32 * 4.0 - 2.0, // [-2,2)
            _ => {
                let m = (next(s) >> 8) % 5;
                [0.0f32, 1.0, -0.5, 1.5, 0.5][m as usize]
            }
        }
    };
    let mut swap_hits = [0u32; 2];
    for i in 0..600 {
        let p0 = [
            frange(&mut seed, 100.0),
            frange(&mut seed, 100.0),
            frange(&mut seed, 100.0),
        ];
        let p1 = [
            frange(&mut seed, 100.0),
            frange(&mut seed, 100.0),
            frange(&mut seed, 100.0),
        ];
        let scale = frange(&mut seed, 5.0);
        let tgt_bc = frange(&mut seed, 3.0);
        let chan = [
            chan_draw(&mut seed),
            chan_draw(&mut seed),
            chan_draw(&mut seed),
        ];
        let swap = (next(&mut seed) >> 8) & 1 == 1;
        swap_hits[swap as usize] += 1;
        check_beam(
            &mut e,
            &b,
            p0,
            p1,
            scale,
            tgt_bc,
            chan,
            swap,
            &format!("beam {i}"),
        );
    }
    assert!(
        swap_hits[0] > 0 && swap_hits[1] > 0,
        "beam: both swap-gate states must be exercised"
    );
}

/// The colour channels and the endpoints at the values a compare or a conversion distinguishes:
/// each channel in turn takes NaN, the infinities, both zeros, 1 and a step either side, and a
/// value whose product with 255 plus 0.5 sits at a byte boundary; the ends and heights take the
/// special values too.
#[test]
fn effect_beam_trail_special_values_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let b = beam_rig(&mut e);

    let base = ([1.0f32, 2.0, 3.0], [11.0f32, -7.0, 5.0], 2.0f32, 0.5f32);
    let mut chans = specials(&[1.0, 0.5]);
    // A value whose product with 255 plus 0.5 is near an integer, then a step either side.
    for k in [1.0f32, 100.0, 254.0] {
        let c = (k - 0.5) / 255.0;
        chans.extend([c, next_up(c), next_down(c)]);
    }
    for swap in [false, true] {
        for &v in &chans {
            for ch in 0..3 {
                let mut chan = [0.25f32, 0.5, 0.75];
                chan[ch] = v;
                check_beam(
                    &mut e,
                    &b,
                    base.0,
                    base.1,
                    base.2,
                    base.3,
                    chan,
                    swap,
                    &format!("chan[{ch}]={v:e}"),
                );
            }
        }
        for &v in &specials(&[]) {
            for k in 0..3 {
                let mut p0 = base.0;
                p0[k] = v;
                check_beam(&mut e, &b, p0, base.1, base.2, base.3, [0.5; 3], swap, "p0");
                let mut p1 = base.1;
                p1[k] = v;
                check_beam(&mut e, &b, base.0, p1, base.2, base.3, [0.5; 3], swap, "p1");
            }
            check_beam(
                &mut e, &b, base.0, base.1, v, base.3, [0.5; 3], swap, "scale",
            );
            check_beam(
                &mut e, &b, base.0, base.1, base.2, v, [0.5; 3], swap, "tgt_bc",
            );
        }
    }
}

/// Packs a `[f32; 3]` into 12 little-endian bytes, the write-through payload.
fn bytemuck_3(v: [f32; 3]) -> [u8; 12] {
    let mut out = [0u8; 12];
    for (i, &c) in v.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&c.to_bits().to_le_bytes());
    }
    out
}

/// `CMissile::Advance` at its gates: `step^2` equal to `|d|^2` and a step either side, a length at
/// the floor `2^-22` and a step either side, both flag states, and NaN, infinities and zeros in
/// each input.
#[test]
fn missile_advance_special_values_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    stub_advance(&mut e);
    let this_va = e.alloc(0x140);

    // A step of 5 over d = (3, 4, 0): `step^2 == |d|^2`, the not-arrived edge.
    for x in [3.0f32, next_up(3.0), next_down(3.0)] {
        for flags in [0u8, 2] {
            check_advance(
                &mut e,
                this_va,
                [0.0; 3],
                [x, 4.0, 0.0],
                5.0,
                1.0,
                flags,
                "arrival edge",
            );
        }
    }
    // The normalize floor: d = (eps, 0, 0) and a step either side, with arrival suppressed.
    let eps = f32::from_bits(0x3480_0000);
    for x in [eps, next_up(eps), next_down(eps)] {
        check_advance(
            &mut e,
            this_va,
            [0.0; 3],
            [x, 0.0, 0.0],
            1.0,
            1.0,
            2,
            "floor",
        );
    }

    let base_cur = [1.0f32, -2.0, 3.0];
    let base_dest = [20.0f32, 15.0, -9.0];
    for flags in [0u8, 2] {
        for &v in &specials(&[eps]) {
            check_advance(&mut e, this_va, base_cur, base_dest, v, 0.5, flags, "speed");
            check_advance(&mut e, this_va, base_cur, base_dest, 4.0, v, flags, "dt");
            for k in 0..3 {
                let mut c = base_cur;
                c[k] = v;
                check_advance(&mut e, this_va, c, base_dest, 4.0, 0.5, flags, "cur");
                let mut d = base_dest;
                d[k] = v;
                check_advance(&mut e, this_va, base_cur, d, 4.0, 0.5, flags, "dest");
            }
        }
    }
}

/// The inputs of one quadratic case.
struct QuadCase {
    p: [f32; 3],
    speed: f32,
    raw_pos: [f32; 3],
    tgt90: f32,
    tgt_bc: f32,
    vel_dir: [f32; 3],
    vel_scalar: f32,
}

/// `CMissile::Aim` in its quadratic branch with NaN, infinities, zeros and the thresholds in each
/// input, for a moving target. A speed over a velocity of equal size runs `s^2` equal to `|V|^2`;
/// zero velocity and distance run the root and floor edges.
#[test]
fn missile_target_intercept_special_values_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let r = icpt_alloc(&mut e);
    e.stub(0x468460, r.tgt as u32, 3);
    e.stub(0x712cb0, 0, 1);
    e.stub(0x712d50, 0, 2);
    e.stub(r.island14, r.pos_buf as u32, 1);

    let base = QuadCase {
        p: [1.0, 2.0, 3.0],
        speed: 30.0,
        raw_pos: [40.0, -20.0, 10.0],
        tgt90: 1.0,
        tgt_bc: 2.0,
        vel_dir: [0.0, 1.0, 0.0],
        vel_scalar: 8.0,
    };
    let run = |e: &mut Emu, c: &QuadCase, ctx: &str| {
        check_intercept_quadratic(
            e,
            &r,
            c.p,
            c.speed,
            c.raw_pos,
            c.tgt90,
            c.tgt_bc,
            c.vel_dir,
            c.vel_scalar,
            true,
            ctx,
        );
    };
    let eps = f32::from_bits(0x3480_0000);
    let sp = specials(&[30.0, eps]);
    for &v in &sp {
        run(&mut e, &QuadCase { speed: v, ..base }, "speed");
        run(
            &mut e,
            &QuadCase {
                vel_scalar: v,
                ..base
            },
            "vel_scalar",
        );
        run(&mut e, &QuadCase { tgt90: v, ..base }, "tgt90");
        run(&mut e, &QuadCase { tgt_bc: v, ..base }, "tgt_bc");
        for k in 0..3 {
            let mut p = base.p;
            p[k] = v;
            run(&mut e, &QuadCase { p, ..base }, "p");
            let mut raw_pos = base.raw_pos;
            raw_pos[k] = v;
            run(&mut e, &QuadCase { raw_pos, ..base }, "raw_pos");
            let mut vel_dir = base.vel_dir;
            vel_dir[k] = v;
            run(&mut e, &QuadCase { vel_dir, ..base }, "vel_dir");
        }
    }
    // `s^2 == |V|^2`: a speed equal to the velocity, and a step either side.
    for s in [8.0f32, next_up(8.0), next_down(8.0)] {
        run(&mut e, &QuadCase { speed: s, ..base }, "s equals |V|");
    }
    // A velocity of length near the normalize floor under the clamp: s tiny, V tiny.
    for len in [eps, next_up(eps), next_down(eps)] {
        run(
            &mut e,
            &QuadCase {
                speed: 1.0e-9,
                vel_dir: [len, 0.0, 0.0],
                vel_scalar: 1.0,
                ..base
            },
            "velocity floor",
        );
    }
}

/// The arc branch's gates: a length at the floor and a step either side, the near-vertical guard
/// `|nz - 1| < eps`, and NaN, infinities and zeros in the positions and the speed, with the
/// generator's return across its sign and mask edges.
#[test]
fn missile_target_intercept_curved_special_values_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let r = icpt_alloc(&mut e);
    e.stub(0x468460, r.tgt as u32, 3);
    e.stub(0x712cb0, 0, 1);
    e.stub(0x712d50, 0, 2);
    e.stub(r.island14, r.pos_buf as u32, 1);

    let run = |e: &mut Emu, p: [f32; 3], raw: [f32; 3], speed: f32, rv: u32, ctx: &str| {
        // Seed the generator from `rv`, read the return it makes, then rewind it so the call
        // inside the function makes the same return.
        e.write_u32(PRNG_STATE, rv);
        e.write_u32(PRNG_STATE + 4, rv.rotate_left(7));
        let predicted = e.call(PRNG_FN, PRNG_STATE as u32, 0, &[]);
        e.write_u32(PRNG_STATE, rv);
        e.write_u32(PRNG_STATE + 4, rv.rotate_left(7));

        e.write_u32(r.this + 0x1c, speed.to_bits());
        for (k, &v) in p.iter().enumerate() {
            e.write_u32(r.this + 0x20 + 4 * k as u64, v.to_bits());
        }
        e.write_u32(r.this + 0x38, 0x8);
        e.write_u32(r.this + 0x3c, 0xffff_ffff);
        for k in 0..3 {
            e.write_u32(r.this + 0x2c + 4 * k as u64, 0xbeef_0000 + k);
        }
        for (k, &v) in raw.iter().enumerate() {
            e.write_u32(r.pos_buf + 4 * k as u64, v.to_bits());
        }
        e.write_u32(r.tgt + 0x90, 0);
        e.write_u32(r.tgt + 0xbc, 0);
        e.call(INTERCEPT_FN, r.this as u32, 0, &[]);
        let bin = [
            e.read_f32(r.this + 0x2c),
            e.read_f32(r.this + 0x30),
            e.read_f32(r.this + 0x34),
        ];
        match missile_target_intercept_curved(p, speed, raw, 0.0, 0.0, predicted) {
            Some(aim) => {
                for k in 0..3 {
                    assert_f32(bin[k], aim[k], &format!("{ctx}: aim[{k}]"));
                }
            }
            None => {
                for (k, &b) in bin.iter().enumerate() {
                    assert_eq!(
                        b.to_bits(),
                        0xbeef_0000 + k as u32,
                        "{ctx}: the exe writes aim[{k}] where the transcription returns early"
                    );
                }
            }
        }
    };

    let eps = f32::from_bits(0x3480_0000);
    for len in [eps, next_up(eps), next_down(eps), 0.0, 1.0e-3] {
        run(
            &mut e,
            [0.0; 3],
            [len, 0.0, 0.0],
            10.0,
            0x1234_5678,
            "length floor",
        );
    }
    // The near-vertical guard: d is straight up, so the normalized z is 1 within eps.
    for dz in [10.0f32, 1.0e6, 1.0, 1.0e-3] {
        run(
            &mut e,
            [0.0; 3],
            [0.0, 0.0, dz],
            10.0,
            0x1234_5678,
            "vertical",
        );
        run(
            &mut e,
            [0.0; 3],
            [dz * 1.0e-8, 0.0, dz],
            10.0,
            0x1234_5678,
            "near vertical",
        );
    }
    let p = [1.0f32, 2.0, 3.0];
    let raw = [30.0f32, -20.0, 8.0];
    for &v in &specials(&[]) {
        run(&mut e, p, raw, v, 0x1234_5678, "speed");
        for k in 0..3 {
            let mut pp = p;
            pp[k] = v;
            run(&mut e, pp, raw, 10.0, 0x1234_5678, "p");
            let mut rr = raw;
            rr[k] = v;
            run(&mut e, p, rr, 10.0, 0x1234_5678, "raw");
        }
    }
    for rv in [
        0u32,
        1,
        0x007f_ffff,
        0x0080_0000,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
    ] {
        run(&mut e, p, raw, 10.0, rv, "rv");
    }
}
