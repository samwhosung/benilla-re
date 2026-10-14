//! The combat log range and blood kernels of `src/gobj1_fp.rs` against the exe. Each runs a slice
//! of its function, with the frame, registers and fields it reads set directly, reading the status
//! byte `fnstsw ax` leaves or a value stored by the code:
//! - `SpillBlood` (`0x625010`): the facing dot and its compare, `[0x6250b6, 0x6250d7)`;
//! - `ResolveCombatLogPair` (`0x626630`): the stored differences `[0x626768, 0x62678c)`, the
//!   squared ranges `[0x626720, 0x62673e)`, the source compare `[0x6267ac, 0x6267c7)` and the
//!   target compare `[0x62679c, 0x6267e3)`;
//! - `0x627de0`, `0x628ff0`, `0x62aac0` and `0x62b000`: the one-sided compare, `0x627de0` as the
//!   representative and each of the four again against the same inputs;
//! - `CombatLogSpellCast` (`0x62b1b0`): the stored differences `[0x62b243, 0x62b264)` and the
//!   compare `[0x62b269, 0x62b289)`.
//!
//! The sums are read as well: the instruction that loads the range or compares is replaced by
//! `fstp qword [ebp-0x40]`, so the slice ends with the sum itself, whose order a compare of one
//! bit rarely shows.
//!
//! Each test adds NaN, the infinities, both zeros and the equal-distance edge to its random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::gobj1_fp::*;
use common::*;

// ---- SpillBlood (0x625010) ----

/// Runs `[0x6250b6, 0x6250d7)` with the dword locals it reloads and returns whether `jnp` falls
/// through to `mov edi,0xf`, the front effect.
fn run_blood(e: &mut Emu, ebp: u64, dx: f32, dy: f32, cos: f32, sin: f32) -> bool {
    e.write_u32(ebp - 4, cos.to_bits());
    e.write_u32(ebp - 8, sin.to_bits());
    e.write_u32(ebp - 0x34, dy.to_bits());
    e.write_u32(ebp - 0x38, dx.to_bits());
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, ebp - 0x20);
    e.run(0x6250b6, 0x6250d7);
    parity_even(ah(e) & 0x05)
}

#[test]
fn diff_blood_spurt_front_back() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x6510_u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, dx: f32, dy: f32, cos: f32, sin: f32, ctx: &str| {
        let bin_front = run_blood(e, ebp, dx, dy, cos, sin);
        let ours = blood_spurt_front_back([dx, dy], [0.0, 0.0], cos, sin);
        assert_eq!(
            bin_front, ours,
            "{ctx}: binary front={bin_front} ours={ours} (dx={dx} dy={dy} cos={cos} sin={sin})"
        );
        seen.insert(bin_front);
    };

    // The dot below zero, above, exactly zero from two terms that cancel, and a step either side.
    case(&mut e, 1.0, 0.0, -1.0, 0.0, "below");
    case(&mut e, 1.0, 0.0, 1.0, 0.0, "above");
    case(&mut e, 0.0, 0.0, 1.0, 1.0, "zero");
    case(&mut e, 1.0, 1.0, 1.0, -1.0, "cancels to +0");
    case(
        &mut e,
        1.0,
        1.0,
        1.0,
        next_down(-1.0),
        "cancels, a step below",
    );
    case(
        &mut e,
        1.0,
        1.0,
        1.0,
        next_up(-1.0),
        "cancels, a step above",
    );
    case(&mut e, f32::NAN, 0.0, 1.0, 0.0, "nan");
    let sp = specials(&[]);
    for &dx in &sp {
        for &dy in &sp {
            for &cos in &sp {
                for &sin in &sp {
                    case(&mut e, dx, dy, cos, sin, "special");
                }
            }
        }
    }
    for i in 0..1200 {
        let (dx, dy, cos, sin) = if i % 3 == 0 {
            (
                frand_bits(&mut s),
                frand_bits(&mut s),
                frand_bits(&mut s),
                frand_bits(&mut s),
            )
        } else {
            (
                frand(&mut s, -200.0, 200.0),
                frand(&mut s, -200.0, 200.0),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
            )
        };
        case(&mut e, dx, dy, cos, sin, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&true), "the front effect never seen");
    assert!(seen.contains(&false), "the back effect never seen");
}

#[test]
fn diff_blood_spurt_front_back_diff_rounding() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;

    // The code stores each difference to a dword before the dot, so the slice is fed the rounded
    // differences and the function takes the positions.
    let probe = |e: &mut Emu, ax: f32, vx: f32, ay: f32, vy: f32, cos: f32, sin: f32| {
        let dx = (f64::from(ax) - f64::from(vx)) as f32;
        let dy = (f64::from(ay) - f64::from(vy)) as f32;
        let bin_front = run_blood(e, ebp, dx, dy, cos, sin);
        let ours = blood_spurt_front_back([ax, ay], [vx, vy], cos, sin);
        assert_eq!(bin_front, ours, "diff rounding probe (ax={ax} vx={vx})");
    };
    probe(&mut e, 16_777_217.0, 1.0, 3.5, 0.25, 0.5, -0.5);
    probe(&mut e, 1e7, 3.0, -1e7, 7.0, -0.7, 0.7);
    probe(&mut e, 123.456, 78.9, -45.6, 12.3, 0.9, 0.1);
}

#[test]
fn diff_blood_spurt_dot() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    patch_fstp_qword(&mut e, 0x6250cf);
    let mut s = 0x6511_u32;

    let case = |e: &mut Emu, dx: f32, dy: f32, cos: f32, sin: f32| {
        e.write_u32(ebp - 4, cos.to_bits());
        e.write_u32(ebp - 8, sin.to_bits());
        e.write_u32(ebp - 0x34, dy.to_bits());
        e.write_u32(ebp - 0x38, dx.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x6250b6, 0x6250d2);
        let bin = read_f64(e, ebp - 0x40);
        let ours = blood_spurt_dot([dx, dy], [0.0, 0.0], cos, sin);
        assert!(
            same_f64(bin, ours),
            "dot: binary={bin:e} ours={ours:e} (dx={dx} dy={dy} cos={cos} sin={sin})"
        );
    };
    let sp = specials(&[]);
    for &dx in &sp {
        for &cos in &sp {
            case(&mut e, dx, 1.0, cos, 0.5);
            case(&mut e, dx, -3.0, cos, f32::NAN);
            case(&mut e, dx, f32::INFINITY, cos, -0.0);
        }
    }
    for _ in 0..3000 {
        case(
            &mut e,
            mixed(&mut s),
            mixed(&mut s),
            mixed(&mut s),
            mixed(&mut s),
        );
    }
}

// ---- ResolveCombatLogPair (0x626630) ----

#[test]
fn diff_two_endpoint_a() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x66a0_u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, d: [f32; 3], range2: f32, ctx: &str| {
        e.write_u32(ebp - 0x10, d[0].to_bits());
        e.write_u32(ebp - 0xc, d[1].to_bits());
        e.write_u32(ebp - 8, d[2].to_bits());
        e.write_u32(ebp + 0x24, range2.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6267ac, 0x6267c7);
        let bin_in = !parity_even(ah(e) & 0x05);
        let ours = source_in_range(d, [0.0; 3], range2);
        assert_eq!(
            bin_in, ours,
            "{ctx}: binary in={bin_in} ours={ours} (d={d:?} r2={range2})"
        );
        seen.insert(bin_in);
    };

    case(&mut e, [1.0, 1.0, 1.0], 100.0, "in");
    case(&mut e, [10.0, 10.0, 10.0], 100.0, "out");
    case(&mut e, [0.0, 0.0, 0.0], 0.0, "equal at zero");
    case(&mut e, [3.0, 4.0, 0.0], 25.0, "equal at 25");
    case(&mut e, [3.0, 4.0, 0.0], next_up(25.0), "a step above 25");
    case(&mut e, [3.0, 4.0, 0.0], next_down(25.0), "a step below 25");
    case(&mut e, [f32::NAN, 0.0, 0.0], 100.0, "nan");
    let sp = specials(&[5.0]);
    for &x in &sp {
        for &y in &sp {
            for &z in &[0.0, -0.0, 1.0, f32::NAN, f32::INFINITY] {
                for &r2 in &sp {
                    case(&mut e, [x, y, z], r2, "special");
                }
            }
        }
    }
    for i in 0..1500 {
        let d = point(&mut s, i);
        let r2 = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 2500.0)
        };
        case(&mut e, d, r2, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&true), "source in range never seen");
    assert!(seen.contains(&false), "source out of range never seen");
}

#[test]
fn diff_two_endpoint_b() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let posa = e.alloc(0x20);
    let posb = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x66b0_u32;

    // The slice starts at the target's subtraction and so passes the source's compare `fcomp
    // [ebp+0x24]` and its `jnp`, which leaves the slice when the source is in range. The source's
    // stored differences and squared range are zero, `0 < 0` fails and the run falls through.
    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, base: [f32; 3], sub: [f32; 3], range2: f32, ctx: &str| {
        for (i, v) in base.iter().enumerate() {
            e.write_u32(posa + 4 * i as u64, v.to_bits());
        }
        for (i, v) in sub.iter().enumerate() {
            e.write_u32(posb + 4 * i as u64, v.to_bits());
        }
        for off in [0x10u64, 0xc, 8] {
            e.write_u32(ebp - off, 0);
        }
        e.write_u32(ebp + 0x24, 0);
        e.write_u32(ebp + 0x1c, range2.to_bits());
        e.set_reg(RegisterX86::EAX, posa);
        e.set_reg(RegisterX86::ESI, posb);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x62679c, 0x6267e3);
        let bin_in = !parity_even(ah(e) & 0x05);
        let ours = target_in_range(base, sub, range2);
        assert_eq!(
            bin_in, ours,
            "{ctx}: binary in={bin_in} ours={ours} (r2={range2})"
        );
        seen.insert(bin_in);
    };

    case(&mut e, [1.0, 1.0, 1.0], [0.0; 3], 100.0, "in");
    case(&mut e, [10.0, 10.0, 10.0], [0.0; 3], 100.0, "out");
    case(&mut e, [0.0; 3], [0.0; 3], 0.0, "equal at zero");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 25.0, "equal at 25");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], next_up(25.0), "above");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], next_down(25.0), "below");
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], 100.0, "nan");
    let sp = specials(&[5.0]);
    for &x in &sp {
        for &y in &sp {
            for &r2 in &sp {
                case(&mut e, [x, y, 0.0], [0.0; 3], r2, "special");
                case(&mut e, [x, 0.0, y], [-0.0; 3], r2, "special, other axes");
            }
        }
    }
    for i in 0..1500 {
        let (base, sub) = (point(&mut s, i), point(&mut s, i));
        let r2 = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 2500.0)
        };
        case(&mut e, base, sub, r2, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&true), "target in range never seen");
    assert!(seen.contains(&false), "target out of range never seen");
}

#[test]
fn diff_two_endpoint_dist2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let posa = e.alloc(0x20);
    let posb = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    // The compares become stores of the sums: the source's at `0x6267c2`, the target's at `0x6267dc`.
    patch_fstp_qword(&mut e, 0x6267dc);
    let mut s = 0x66b1_u32;

    let case = |e: &mut Emu, base: [f32; 3], sub: [f32; 3]| {
        for (i, v) in base.iter().enumerate() {
            e.write_u32(posa + 4 * i as u64, v.to_bits());
        }
        for (i, v) in sub.iter().enumerate() {
            e.write_u32(posb + 4 * i as u64, v.to_bits());
        }
        for off in [0x10u64, 0xc, 8] {
            e.write_u32(ebp - off, 0);
        }
        e.write_u32(ebp + 0x24, 0);
        e.set_reg(RegisterX86::EAX, posa);
        e.set_reg(RegisterX86::ESI, posb);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x62679c, 0x6267df);
        let bin = read_f64(e, ebp - 0x40);
        let ours = target_dist2(base, sub);
        assert!(
            same_f64(bin, ours),
            "target d2: binary={bin:e} ours={ours:e} (base={base:?} sub={sub:?})"
        );
    };
    for _ in 0..4000 {
        let (base, sub) = (point(&mut s, 2), point(&mut s, 2));
        case(&mut e, base, sub);
    }
    let sp = specials(&[]);
    for &x in &sp {
        for &y in &sp {
            case(&mut e, [x, y, 1.0], [0.0; 3]);
        }
    }

    // The source's sum is read with its own patch, from the stored differences.
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    patch_fstp_qword(&mut e, 0x6267c2);
    for _ in 0..4000 {
        let d = point(&mut s, 2);
        e.write_u32(ebp - 0x10, d[0].to_bits());
        e.write_u32(ebp - 0xc, d[1].to_bits());
        e.write_u32(ebp - 8, d[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6267ac, 0x6267c5);
        let bin = read_f64(&e, ebp - 0x40);
        let ours = source_dist2(d, [0.0; 3]);
        assert!(
            same_f64(bin, ours),
            "source d2: binary={bin:e} ours={ours:e} (d={d:?})"
        );
    }
}

#[test]
fn diff_two_endpoint_diffs() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let posa = e.alloc(0x20);
    let posb = e.alloc(0x20);
    let chain = e.alloc(0x40);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    // `mov ecx,[ebp+0x20]; mov ecx,[ecx]; mov edx,[ecx]` walks three pointers before the stores.
    e.write_u32(ebp + 0x20, chain as u32);
    e.write_u32(chain, (chain + 0x10) as u32);
    e.write_u32(chain + 0x10, (chain + 0x20) as u32);
    let mut s = 0x66d0_u32;

    let case = |e: &mut Emu, base: [f32; 3], sub: [f32; 3]| {
        for (i, v) in base.iter().enumerate() {
            e.write_u32(posa + 4 * i as u64, v.to_bits());
        }
        for (i, v) in sub.iter().enumerate() {
            e.write_u32(posb + 4 * i as u64, v.to_bits());
        }
        e.set_reg(RegisterX86::EAX, posa);
        e.set_reg(RegisterX86::ESI, posb);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x626768, 0x62678c);
        let got = [
            e.read_f32(ebp - 0x10),
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 8),
        ];
        for k in 0..3 {
            let want = (f64::from(base[k]) - f64::from(sub[k])) as f32;
            assert_f32(got[k], want, &format!("axis {k} of {base:?} less {sub:?}"));
        }
    };
    case(&mut e, [16_777_217.0, 1.0, 0.1], [1.0, 16_777_217.0, 0.3]);
    case(
        &mut e,
        [f32::INFINITY, -0.0, 0.0],
        [f32::INFINITY, 0.0, -0.0],
    );
    for i in 0..1500 {
        let (base, sub) = (point(&mut s, i), point(&mut s, i + 1));
        case(&mut e, base, sub);
    }
}

#[test]
fn two_endpoint_compose() {
    // The player is in range of the source, or else of the target.
    assert!(two_endpoint_visible(
        [1.0, 0.0, 0.0],
        [0.0; 3],
        100.0,
        [99.0, 0.0, 0.0],
        [0.0; 3],
        1.0
    ));
    assert!(two_endpoint_visible(
        [99.0, 0.0, 0.0],
        [0.0; 3],
        1.0,
        [1.0, 0.0, 0.0],
        [0.0; 3],
        100.0
    ));
    assert!(!two_endpoint_visible(
        [99.0, 0.0, 0.0],
        [0.0; 3],
        1.0,
        [99.0, 0.0, 0.0],
        [0.0; 3],
        1.0
    ));
    // The squared range is rounded to f32 where it is stored: with range 1 + 2^-23 the stored
    // square is 1 + 2^-22 and a distance of exactly that is out of range; the 53-bit square
    // 1 + 2^-22 + 2^-46 would put it in.
    let range = f32::from_bits(0x3f80_0001);
    let sub = [0.0; 3];
    let base = [1.0, 2.0f32.powi(-11), 0.0];
    assert!(!two_endpoint_visible(
        base,
        sub,
        range,
        [1e9, 0.0, 0.0],
        sub,
        1.0
    ));
    assert!(range_squared_stored(range) == 1.0 + 2.0f32.powi(-22));
}

#[test]
fn diff_two_endpoint_range_sq() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let ptrs = e.alloc(0x20);
    let dummy = e.alloc(0x10);
    // `mov edx,[ebp+0x1c]; mov ecx,[edx]` reads the pointer the squared target range replaces.
    e.write_u32(ptrs, ptrs as u32 + 8);
    e.write_u32(ptrs + 8, 0);
    // The call `0x626810` returns the target's range in `st0`; the source's is stored at `[ebp+0x24]`.
    let slot = install_st0_stub(&mut e, 0x626810, 0);
    let mut s = 0x66c0_u32;
    let case = |e: &mut Emu, ra: f32, rb: f32, ctx: &str| {
        e.write_u32(ebp + 0x24, ra.to_bits());
        e.write_u32(ebp + 0x1c, ptrs as u32);
        e.write_u32(slot, rb.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, dummy);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x626720, 0x62673e);
        assert_f32(
            e.read_f32(ebp + 0x24),
            range_squared_stored(ra),
            &format!("{ctx}: source"),
        );
        assert_f32(
            e.read_f32(ebp + 0x1c),
            range_squared_stored(rb),
            &format!("{ctx}: target"),
        );
    };
    case(&mut e, 5.0, 12.0, "plain");
    case(
        &mut e,
        f32::from_bits(0x3f80_0001),
        f32::from_bits(0x3f80_0001),
        "1 + 2^-23",
    );
    for &r in &specials(&[1.0, 5.0]) {
        case(&mut e, r, 3.0, "special source");
        case(&mut e, 3.0, r, "special target");
    }
    for i in 0..1000 {
        let r = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -150.0, 150.0)
        };
        let q = if i % 3 == 1 {
            frand_bits(&mut s)
        } else {
            mixed(&mut s)
        };
        case(&mut e, r, q, &format!("fuzz{i}"));
    }
}

// ---- 0x627de0, 0x628ff0, 0x62aac0, 0x62b000 ----

/// A one-sided compare: where the slice starts, where `fcompp` leaves `fnstsw`, the stack slot of
/// the range, and the register holding the player's position.
struct Twin {
    start: u64,
    end: u64,
    range_at: i64,
    sum_end: u64,
    ebx_sub: bool,
}

const TWINS: [Twin; 4] = [
    Twin {
        start: 0x627f0c,
        end: 0x627f38,
        range_at: -8,
        sum_end: 0x627f2f,
        ebx_sub: false,
    },
    Twin {
        start: 0x629083,
        end: 0x6290af,
        range_at: -4,
        sum_end: 0x6290a6,
        ebx_sub: false,
    },
    Twin {
        start: 0x62ab71,
        end: 0x62ab9d,
        range_at: -8,
        sum_end: 0x62ab94,
        ebx_sub: true,
    },
    Twin {
        start: 0x62b090,
        end: 0x62b0bc,
        range_at: -8,
        sum_end: 0x62b0b3,
        ebx_sub: false,
    },
];

#[test]
fn diff_range_gate_visible() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let posa = e.alloc(0x20);
    let posb = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x27d0_u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, base: [f32; 3], sub: [f32; 3], range: f32, ctx: &str| {
        for (i, v) in base.iter().enumerate() {
            e.write_u32(posa + 4 * i as u64, v.to_bits());
        }
        for (i, v) in sub.iter().enumerate() {
            e.write_u32(posb + 4 * i as u64, v.to_bits());
        }
        e.write_u32(ebp - 8, range.to_bits());
        e.set_reg(RegisterX86::EAX, posa);
        e.set_reg(RegisterX86::ESI, posb);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x627f0c, 0x627f38);
        let bin_visible = ah(e) & 0x41 == 0;
        let ours = range_gate_visible(base, sub, range);
        assert_eq!(
            bin_visible, ours,
            "{ctx}: binary visible={bin_visible} ours={ours} (base={base:?} sub={sub:?} range={range})"
        );
        seen.insert(bin_visible);
    };

    case(&mut e, [1.0, 1.0, 1.0], [0.0; 3], 100.0, "in");
    case(&mut e, [100.0, 0.0, 0.0], [0.0; 3], 1.0, "out");
    case(&mut e, [1.0, 0.0, 0.0], [0.0; 3], 1.0, "equal");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 5.0, "equal at 5");
    case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        next_up(5.0),
        "range a step above",
    );
    case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        next_down(5.0),
        "range a step below",
    );
    case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        -5.0,
        "negative range, same square",
    );
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], 10.0, "nan");
    let sp = specials(&[5.0]);
    for &x in &sp {
        for &y in &sp {
            for &z in &[0.0, 1.0, f32::NAN, f32::INFINITY] {
                for &r in &sp {
                    case(&mut e, [x, y, z], [0.0; 3], r, "special");
                }
            }
        }
    }
    for i in 0..1600 {
        let (base, sub) = (point(&mut s, i), point(&mut s, i));
        let range = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 150.0)
        };
        case(&mut e, base, sub, range, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&true), "the line written never seen");
    assert!(seen.contains(&false), "the line skipped never seen");
}

#[test]
fn range_gate_clone_twins() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let posa = e.alloc(0x20);
    let posb = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0xc10e_u32;

    for i in 0..600 {
        let (base, sub) = (point(&mut s, i), point(&mut s, i));
        let range = frand(&mut s, 0.0, 120.0);
        let ours = range_gate_visible(base, sub, range);
        for (ti, t) in TWINS.iter().enumerate() {
            for (k, v) in base.iter().enumerate() {
                e.write_u32(posa + 4 * k as u64, v.to_bits());
            }
            for (k, v) in sub.iter().enumerate() {
                e.write_u32(posb + 4 * k as u64, v.to_bits());
            }
            e.write_u32((ebp as i64 + t.range_at) as u64, range.to_bits());
            e.set_reg(RegisterX86::EAX, posa);
            e.set_reg(
                if t.ebx_sub {
                    RegisterX86::EBX
                } else {
                    RegisterX86::ESI
                },
                posb,
            );
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, ebp - 0x20);
            e.run(t.start, t.end);
            let bin_visible = ah(&e) & 0x41 == 0;
            assert_eq!(
                bin_visible, ours,
                "twin#{ti} iter{i}: binary visible={bin_visible} ours={ours} (range={range})"
            );
        }
    }
}

#[test]
fn diff_range_gate_dist2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let posa = e.alloc(0x20);
    let posb = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    // The instruction that loads the range becomes a store of the sum on the stack.
    for t in &TWINS {
        patch_fstp_qword(&mut e, t.sum_end - 3);
    }
    let mut s = 0x27d1_u32;

    for _ in 0..3000 {
        let (base, sub) = (point(&mut s, 2), point(&mut s, 2));
        let ours = one_sided_dist2(base, sub);
        for (ti, t) in TWINS.iter().enumerate() {
            for (k, v) in base.iter().enumerate() {
                e.write_u32(posa + 4 * k as u64, v.to_bits());
            }
            for (k, v) in sub.iter().enumerate() {
                e.write_u32(posb + 4 * k as u64, v.to_bits());
            }
            e.set_reg(RegisterX86::EAX, posa);
            e.set_reg(
                if t.ebx_sub {
                    RegisterX86::EBX
                } else {
                    RegisterX86::ESI
                },
                posb,
            );
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, ebp - 0x20);
            e.run(t.start, t.sum_end);
            let bin = read_f64(&e, ebp - 0x40);
            assert!(
                same_f64(bin, ours),
                "twin#{ti} d2: binary={bin:e} ours={ours:e} (base={base:?} sub={sub:?})"
            );
        }
    }
}

// ---- CombatLogSpellCast (0x62b1b0) ----

#[test]
fn diff_quest_range_gate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0xb1b0_u32;

    let mut seen = std::collections::BTreeSet::new();
    let mut case = |e: &mut Emu, base: [f32; 3], sub: [f32; 3], range: f32, ctx: &str| {
        // The vector constructor copies the stored differences to `[ebp-0x18]`.
        let d: Vec<f32> = (0..3)
            .map(|k| (f64::from(base[k]) - f64::from(sub[k])) as f32)
            .collect();
        e.write_u32(ebp - 0x18, d[0].to_bits());
        e.write_u32(ebp - 0x14, d[1].to_bits());
        e.write_u32(ebp - 0x10, d[2].to_bits());
        e.write_u32(ebp - 4, range.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x62b269, 0x62b289);
        let bin_visible = ah(e) & 0x41 == 0;
        let ours = quest_range_gate_visible(base, sub, range);
        assert_eq!(
            bin_visible, ours,
            "{ctx}: binary visible={bin_visible} ours={ours} (base={base:?} sub={sub:?} range={range})"
        );
        seen.insert(bin_visible);
    };

    case(&mut e, [1.0, 1.0, 1.0], [0.0; 3], 100.0, "in");
    case(&mut e, [100.0, 0.0, 0.0], [0.0; 3], 1.0, "out");
    case(&mut e, [1.0, 0.0, 0.0], [0.0; 3], 1.0, "equal");
    case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], 5.0, "equal at 5");
    case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        next_up(5.0),
        "range a step above",
    );
    case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        next_down(5.0),
        "range a step below",
    );
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], 10.0, "nan");
    let sp = specials(&[5.0]);
    for &x in &sp {
        for &y in &sp {
            for &r in &sp {
                case(&mut e, [x, y, 0.0], [0.0; 3], r, "special");
                case(&mut e, [x, 0.0, y], [-0.0; 3], r, "special, other axes");
            }
        }
    }
    for i in 0..1600 {
        let (base, sub) = (point(&mut s, i), point(&mut s, i));
        let range = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 150.0)
        };
        case(&mut e, base, sub, range, &format!("fuzz{i}"));
    }
    assert!(seen.contains(&true), "the line written never seen");
    assert!(seen.contains(&false), "the line skipped never seen");
}

#[test]
fn diff_quest_dist2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let posa = e.alloc(0x20);
    let posb = e.alloc(0x20);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    // The sum, from the differences the constructor copies; then the dword stores before it.
    patch_fstp_qword(&mut e, 0x62b27f);
    let mut s = 0xb1b1_u32;

    for _ in 0..3000 {
        let d = point(&mut s, 2);
        e.write_u32(ebp - 0x18, d[0].to_bits());
        e.write_u32(ebp - 0x14, d[1].to_bits());
        e.write_u32(ebp - 0x10, d[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x20);
        e.run(0x62b269, 0x62b282);
        let bin = read_f64(&e, ebp - 0x40);
        let ours = quest_dist2(d, [0.0; 3]);
        assert!(
            same_f64(bin, ours),
            "quest d2: binary={bin:e} ours={ours:e} (d={d:?})"
        );
    }

    for i in 0..1500 {
        let (base, sub) = (point(&mut s, i), point(&mut s, i + 1));
        for (i, v) in base.iter().enumerate() {
            e.write_u32(posa + 4 * i as u64, v.to_bits());
        }
        for (i, v) in sub.iter().enumerate() {
            e.write_u32(posb + 4 * i as u64, v.to_bits());
        }
        e.set_reg(RegisterX86::EAX, posa);
        e.set_reg(RegisterX86::ESI, posb);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x62b243, 0x62b264);
        let esp = e.get_reg(RegisterX86::ESP);
        for k in 0..3 {
            let got = e.read_f32(esp + 4 * k as u64);
            let want = (f64::from(base[k]) - f64::from(sub[k])) as f32;
            assert_f32(got, want, &format!("argument {k} of {base:?} less {sub:?}"));
        }
    }
}
