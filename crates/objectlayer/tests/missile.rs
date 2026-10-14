//! The missile's float math against the exe. `0x61eec0` (the axis scale) runs whole on edge
//! values and random vectors. `CMissile::Launch` (`0x61ceb0`) runs whole with its callees stubbed,
//! so the flight-speed leg is the only code that changes `+0x1c`. The leg has four outcomes, each
//! driven: a speed of 0 or less skips it; a positive remaining time gives `dist / remaining`,
//! capped at 100 or not; and none left gives 100. A random sweep counts the branches it takes and
//! requires the last three. A second test probes the gates with NaN, infinities and zeros, and the
//! cap at equality.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::missile::{missile_scale_vec, missile_traveltime};
use common::{assert_f32, next, specials};

const SCALE_FN: u64 = 0x61eec0;
const TRAVELTIME_FN: u64 = 0x61ceb0;

/// One scale case: writes the scale (`+0x84`) and the axis (`+0x5c`, `+0x60`, `+0x64`), runs
/// `0x61eec0(missile, out)` and compares each component.
fn check_scale(e: &mut Emu, this_va: u64, out_va: u64, scale: f32, axis: [f32; 3], ctx: &str) {
    e.write_u32(this_va + 0x84, scale.to_bits());
    e.write_u32(this_va + 0x5c, axis[0].to_bits());
    e.write_u32(this_va + 0x60, axis[1].to_bits());
    e.write_u32(this_va + 0x64, axis[2].to_bits());
    e.write(out_va, &[0u8; 12]);
    e.call(SCALE_FN, this_va as u32, 0, &[out_va as u32]);

    let bin = [
        e.read_f32(out_va),
        e.read_f32(out_va + 4),
        e.read_f32(out_va + 8),
    ];
    let ours = missile_scale_vec(scale, axis);
    for i in 0..3 {
        assert_f32(bin[i], ours[i], &format!("{ctx}: out[{i}]"));
    }
}

#[test]
fn missile_scale_vec_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this_va = e.alloc(0x90);
    let out_va = e.alloc(0x10);

    // Edge values: each scale against each axis.
    let scalars = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        2.5,
        -3.75,
        1.0e20,
        1.0e-20,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let axes: [[f32; 3]; 6] = [
        [0.0, 0.0, 0.0],
        [1.0, -1.0, 0.5],
        [-2.0, 3.0, -4.0],
        [1.0e18, -1.0e-18, 7.0],
        [f32::INFINITY, 1.0, f32::NAN],
        [123.456, -789.0, 0.001],
    ];
    for (si, &s) in scalars.iter().enumerate() {
        for (ai, &a) in axes.iter().enumerate() {
            check_scale(&mut e, this_va, out_va, s, a, &format!("edge s{si} a{ai}"));
        }
    }

    // Random values over mixed magnitudes.
    let mut seed = 0x5ca1_e000u32;
    let draw = |s: &mut u32| -> f32 {
        let pick = (next(s) >> 8) % 4;
        let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,
            1 => (m * 200.0 - 100.0) as f32,
            2 => (m * 1.0e9) as f32,
            _ => (m * 1.0e-9) as f32,
        }
    };
    for i in 0..2000 {
        let s = draw(&mut seed);
        let a = [draw(&mut seed), draw(&mut seed), draw(&mut seed)];
        check_scale(&mut e, this_va, out_va, s, a, &format!("fuzz {i}"));
    }
}

/// The millisecond clock the stubbed `OsGetAsyncTimeMs` (`0x42c010`) returns.
const TICK: u32 = 0x1000_0000;

/// Stubs every callee of `0x61ceb0` so the function reaches the speed leg and returns: the object
/// lookup returns null (the animation path is skipped), the advance returns nonzero, the clock
/// returns `TICK`, and the model, list and animation calls do nothing. Each `pop_args` is the
/// callee's `ret N` over 4.
fn stub_traveltime(e: &mut Emu) {
    e.stub(0x61d720, 0, 0); // effect type to flag switch (thiscall, no stack arguments)
    e.stub(0x61ce50, 0, 1); // list insert (ret 4)
    e.stub(0x468460, 0, 3); // `ClntObjMgrObjectPtr` returns null (ret 0xc)
    e.stub(0x61d230, 0, 0); // `CMissile::Aim` (fastcall, no stack arguments)
    e.stub(0x61e2a0, 1, 2); // `CMissile::Advance` returns nonzero (ret 8)
    e.stub(0x712ed0, 0, 0); // model query returns 0
    e.stub(0x42c010, TICK, 0); // the clock returns `TICK`
    e.stub(0x711960, 0, 1); // model check (ret 4)
    e.stub(0x7121a0, 0, 7); // model submit (ret 0x1c)
                            // Off the path, stubbed so any path still returns:
    e.stub(0x712cb0, 0, 1);
    e.stub(0x5fe2f0, 0, 1);
    e.stub(0x4078e0, 0, 1);
    e.stub(0x711a20, 0, 3);
    e.stub(0x713020, 0, 0);
    e.stub(0x40a2b0, 0, 0); // `__ftol`
}

/// One flight-speed case: `origin` is the second argument, which the function copies to `+0x20`,
/// `cur` is set at `+0x2c`, `speed` at `+0x1c` and `launch_tick` at `+0x78`. It runs `0x61ceb0`
/// and compares the speed left at `+0x1c`.
#[allow(clippy::too_many_arguments)]
fn check_traveltime(
    e: &mut Emu,
    this_va: u64,
    p2_va: u64,
    origin: [f32; 3],
    cur: [f32; 3],
    speed: f32,
    launch_tick: i32,
    ctx: &str,
) {
    e.write_u32(this_va, 1); // a nonzero model pointer at +0x00, past the null check
    e.write_u32(this_va + 0x38, 0); // a clear flag word keeps the animation path off
    e.write_u32(this_va + 0x1c, speed.to_bits());
    e.write_u32(this_va + 0x2c, cur[0].to_bits());
    e.write_u32(this_va + 0x30, cur[1].to_bits());
    e.write_u32(this_va + 0x34, cur[2].to_bits());
    e.write_u32(this_va + 0x78, launch_tick as u32);
    e.write_u32(p2_va, origin[0].to_bits());
    e.write_u32(p2_va + 4, origin[1].to_bits());
    e.write_u32(p2_va + 8, origin[2].to_bits());
    e.clear_calls();
    e.call(TRAVELTIME_FN, this_va as u32, 0, &[p2_va as u32]);

    let bin = e.read_f32(this_va + 0x1c);
    let ours = missile_traveltime(origin, cur, speed, TICK as i32, launch_tick);
    assert_f32(bin, ours, ctx);
}

/// The branch the leg takes, to count fuzz coverage: 0 skip, 1 no clamp, 2 clamp, 3 fallback.
fn branch_of(origin: [f32; 3], cur: [f32; 3], speed: f32, now: i32, launch_tick: i32) -> u8 {
    if speed > 0.0 {
        let dx = cur[0] as f64 - origin[0] as f64;
        let dy = cur[1] as f64 - origin[1] as f64;
        let dz = cur[2] as f64 - origin[2] as f64;
        let dist = ((dz * dz + dy * dy) + dx * dx).sqrt();
        let t0 = (dist / speed as f64) as f32;
        let elapsed = now.wrapping_sub(launch_tick);
        let remaining = t0 as f64 - elapsed as f64 * f32::from_bits(0x3a83_126f) as f64;
        if remaining > 0.0 {
            let q = dist as f32 as f64 / remaining as f32 as f64;
            if 100.0f64 < q {
                2
            } else {
                1
            }
        } else {
            3
        }
    } else {
        0
    }
}

#[test]
fn missile_traveltime_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    stub_traveltime(&mut e);
    let this_va = e.alloc(0x90);
    let p2_va = e.alloc(0x10);
    let now = TICK as i32;

    // One probe per branch. Skip: a speed of 0 or less leaves `+0x1c` as it is.
    check_traveltime(
        &mut e,
        this_va,
        p2_va,
        [0.0; 3],
        [10.0, 0.0, 0.0],
        0.0,
        now,
        "A speed=0",
    );
    check_traveltime(
        &mut e,
        this_va,
        p2_va,
        [0.0; 3],
        [10.0, 0.0, 0.0],
        -5.0,
        now,
        "A speed<0",
    );
    // No clamp: with no time elapsed, `remaining = dist / speed` and the quotient is the speed.
    check_traveltime(
        &mut e,
        this_va,
        p2_va,
        [0.0; 3],
        [30.0, 40.0, 0.0],
        25.0,
        now,
        "B",
    );
    // Clamp: with no time elapsed and a speed above 100, the result is 100.
    check_traveltime(
        &mut e,
        this_va,
        p2_va,
        [0.0; 3],
        [30.0, 40.0, 0.0],
        150.0,
        now,
        "C speed>100",
    );
    // Clamp: a tiny positive remaining gives a huge quotient.
    check_traveltime(
        &mut e,
        this_va,
        p2_va,
        [0.0; 3],
        [600.0, 0.0, 0.0],
        10.0,
        now - 59,
        "C tiny-rem",
    );
    // Fallback: a launch far in the past leaves no time, so 100.
    check_traveltime(
        &mut e,
        this_va,
        p2_va,
        [0.0; 3],
        [5.0, 0.0, 0.0],
        10.0,
        now - 10_000_000,
        "D",
    );

    // Random draws, counted by branch.
    let mut seed = 0x4d15_51e7u32;
    let frange = |s: &mut u32, span: f32| -> f32 {
        let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        (m as f32 * 2.0 - 1.0) * span
    };
    let mut hits = [0u32; 4];
    for i in 0..4000 {
        let origin = [
            frange(&mut seed, 500.0),
            frange(&mut seed, 500.0),
            frange(&mut seed, 500.0),
        ];
        let cur = [
            frange(&mut seed, 500.0),
            frange(&mut seed, 500.0),
            frange(&mut seed, 500.0),
        ];
        // A speed in (0, 200], on both sides of the 100 clamp.
        let speed =
            (f64::from(next(&mut seed) >> 8) / f64::from(1u32 << 24)) as f32 * 200.0 + 1.0e-3;
        // Elapsed time on three scales: within 10^7 ms either way (mostly the fallback), within
        // 10^4 ms (around the clamp), and exactly 0 (the quotient is the speed).
        let elapsed = match (next(&mut seed) >> 8) % 3 {
            0 => ((next(&mut seed) >> 8) % 20_000_001) as i32 - 10_000_000,
            1 => ((next(&mut seed) >> 8) % 20_001) as i32 - 10_000,
            _ => 0,
        };
        let launch_tick = now.wrapping_sub(elapsed);
        hits[branch_of(origin, cur, speed, now, launch_tick) as usize] += 1;
        check_traveltime(
            &mut e,
            this_va,
            p2_va,
            origin,
            cur,
            speed,
            launch_tick,
            &format!("fuzz {i}"),
        );
    }
    assert!(hits[1] > 0, "branch B (no clamp) never exercised");
    assert!(hits[2] > 0, "branch C (clamp) never exercised");
    assert!(hits[3] > 0, "branch D (fallback) never exercised");
}

/// The speed leg's gates at their edges. A zero remaining time is built exactly: with `speed` 1 and
/// the destination `t0` away, `remaining = t0 - 1024 * 0.001f32` is 0 when `t0` is that product, and
/// a step either side of `t0` is just over and under. With no time elapsed the quotient is the
/// speed, so a speed of 100 and a step either side run the cap at equality. NaN, infinities and
/// zeros in the speed, the points and the clock follow.
#[test]
fn missile_traveltime_special_values_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    stub_traveltime(&mut e);
    let this_va = e.alloc(0x90);
    let p2_va = e.alloc(0x10);
    let now = TICK as i32;

    let t0 = (1024.0f64 * f64::from(f32::from_bits(0x3a83_126f))) as f32;
    for x in [t0, common::next_up(t0), common::next_down(t0)] {
        check_traveltime(
            &mut e,
            this_va,
            p2_va,
            [0.0; 3],
            [x, 0.0, 0.0],
            1.0,
            now - 1024,
            "remaining zero",
        );
    }
    for speed in [100.0f32, common::next_up(100.0), common::next_down(100.0)] {
        check_traveltime(
            &mut e,
            this_va,
            p2_va,
            [0.0; 3],
            [100.0, 0.0, 0.0],
            speed,
            now,
            "cap equality",
        );
    }

    let sp = specials(&[100.0]);
    let base = ([1.0f32, 2.0, 3.0], [30.0f32, 40.0, 0.0], 25.0f32);
    for &v in &sp {
        for elapsed in [0, 1000, -1000, 10_000_000] {
            let lt = now.wrapping_sub(elapsed);
            check_traveltime(&mut e, this_va, p2_va, base.0, base.1, v, lt, "speed");
            for k in 0..3 {
                let mut o = base.0;
                o[k] = v;
                check_traveltime(&mut e, this_va, p2_va, o, base.1, base.2, lt, "origin");
                let mut c = base.1;
                c[k] = v;
                check_traveltime(&mut e, this_va, p2_va, base.0, c, base.2, lt, "cur");
            }
        }
    }
    for lt in [i32::MIN, i32::MAX, 0, -1] {
        check_traveltime(&mut e, this_va, p2_va, base.0, base.1, base.2, lt, "tick");
    }
}
