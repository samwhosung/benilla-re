//! The unit frame update kernels of `src/unitm4_fp.rs` against the exe. Each runs a slice of the
//! function with the registers and frame it reads set directly, reading the store it ends in or the
//! status byte `fnstsw ax` leaves; the slices that call `WrapAngleToPi` (`0x6084f0`) run its code:
//! - `0x600cd0` (`CGUnit_C::UpdateDisplayFacing`): the delta `[0x600ea5, 0x600eb3)`, the dead
//!   band `[0x600ea5, 0x600ec0)`, the wrap to +-pi `[0x600eda, 0x600f0e)` and the box average with
//!   its select, apply and output wrap;
//! - `0x605650` (`IsInDeepLiquid`): the height against half the reach, `[0x6056ac, 0x6056ca)`;
//! - `0x607ed0` (`CGUnit_C::Update`): the spin-cast byte `[0x607f9b, 0x607fbe)`, the turn step cap
//!   `[0x6081dd, 0x608207)`, the lean `[0x60809a, 0x6080f6)`, the turn delta
//!   `[0x608119, 0x60813d)` and the apply `[0x608227, 0x60823f)`.
//!
//! Each test adds NaN, both zeros, its thresholds, equal and a step either side, and the extremes
//! of its integers to its random inputs. The angles leave out infinities and magnitudes past 1e15:
//! an infinity sends `fmod` to the C runtime's error path, which needs state the start-up sets and
//! the emulator does not run, and past that the emulator's partial `fprem` rounds in `f64`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::unitm4_fp::*;
use common::*;

// ---- shared helpers (the crate float idiom) -------------------------------

#[test]
fn u4_facing_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x2000); // ESI; prev at +0xc98
    let mut s = 0x600c_d000u32;

    let case = |e: &mut Emu, a: f32, prev: f32, ctx: &str| {
        e.write_u32(ebp - 4, a.to_bits());
        e.write_u32(this + 0xc98, prev.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x600ea5, 0x600eb3);
        let bin = e.read_f32(ebp - 4);
        assert_f32(bin, facing_delta(a, prev), ctx);
    };

    case(&mut e, 1.0, 0.5, "simple");
    case(&mut e, 0.0, 0.0, "zero");
    case(&mut e, -3.0, 3.0, "neg");
    for i in 0..3000u32 {
        let (a, prev) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (frand(&mut s, -10.0, 10.0), frand(&mut s, -10.0, 10.0))
        };
        case(&mut e, a, prev, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, 0.5, "special a");
        case(&mut e, 0.5, v, "special prev");
    }
}

#[test]
fn u4_facing_dead_band_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x2000);
    let mut s = 0x600c_d100u32;
    let mut seen_dead = false;
    let mut seen_live = false;

    let eps = f32::from_bits(0x3c23_d70a); // 0.01

    let mut case = |e: &mut Emu, a: f32, prev: f32, ctx: &str| {
        e.write_u32(ebp - 4, a.to_bits());
        e.write_u32(this + 0xc98, prev.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x600ea5, 0x600ec0);
        let m = ah(e) & 0x41;
        // test ah,0x41; jp 0x600ed8: jp taken (even parity of C0|C3) → smooth; not taken (C0 alone,
        // odd) → dead-band. So dead-band ⟺ ODD parity of (ah&0x41).
        let bits = (m & 0x01).count_ones() + (m & 0x40).count_ones();
        let bin_dead = bits % 2 == 1;
        if bin_dead {
            seen_dead = true;
        } else {
            seen_live = true;
        }
        assert_eq!(
            bin_dead,
            facing_dead_band(a, prev),
            "{ctx}: bin_dead={bin_dead} ah&0x41={m:#04x} a={a} prev={prev}"
        );
    };

    case(&mut e, 1.0, 1.0, "==  → dead");
    case(&mut e, 1.0, 1.005, "tiny → dead");
    case(&mut e, 1.0, 1.02, ">eps → live");
    case(&mut e, f32::NAN, 0.0, "nan → live");
    // Deterministic boundary probes: |Δ| == 0.01 EXACTLY (the fuzz cannot land on bit-exact eps).
    // The binary snaps here (C3 → odd parity → jp not taken → dead-band); a strict `<` would call it
    // live and diverge. Both signs of Δ, since `fabs` precedes the compare.
    case(&mut e, eps, 0.0, "+eps == 0.01 → dead");
    case(&mut e, 0.0, eps, "-eps == 0.01 → dead");
    case(
        &mut e,
        1.0 + f64::from(eps) as f32,
        1.0,
        "1+eps → boundary dead",
    );
    for i in 0..4000u32 {
        let prev = frand(&mut s, -6.0, 6.0);
        let a = match i % 3 {
            0 => prev + frand(&mut s, -0.02, 0.02) * (eps / 0.01), // straddle the 0.01 band
            1 => prev + frand(&mut s, -1.0, 1.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, a, prev, &format!("fuzz{i}"));
    }
    // `|d| == 0.01` and a step either side, both signs, then the specials.
    for x in [eps, next_up(eps), next_down(eps)] {
        case(&mut e, x, 0.0, "edge");
        case(&mut e, 0.0, x, "edge neg");
        case(&mut e, 1.0 + x, 1.0, "edge offset");
    }
    for &v in &specials(&[eps]) {
        case(&mut e, v, 0.5, "special a");
        case(&mut e, 0.5, v, "special prev");
    }
    assert!(seen_dead, "dead-band (|Δ|<=0.01) never seen");
    assert!(seen_live, "smooth (|Δ|>0.01 / NaN) never seen");
}

#[test]
fn u4_facing_wrap_pi_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x600c_d200u32;
    let mut seen_hi = false; // Δ>π
    let mut seen_lo = false; // Δ<−π
    let mut seen_mid = false; // unchanged

    let pi = f32::from_bits(0x4049_0fdb);

    let mut case = |e: &mut Emu, delta: f32, ctx: &str| {
        e.write_u32(ebp - 4, delta.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x600eda, 0x600f0e);
        let bin = e.read_f32(ebp - 4);
        let ours = facing_wrap_pi_delta(delta);
        if f64::from(delta) > f64::from(pi) {
            seen_hi = true;
        } else if f64::from(delta) < -f64::from(pi) {
            seen_lo = true;
        } else {
            seen_mid = true;
        }
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 0.0, "zero");
    case(&mut e, pi, "==π unchanged");
    case(&mut e, -pi, "==−π unchanged");
    case(&mut e, pi + 0.5, ">π → −2π");
    case(&mut e, -pi - 0.5, "<−π → +2π");
    case(&mut e, 6.0, "big +");
    case(&mut e, -6.0, "big −");
    case(&mut e, f32::NAN, "nan unchanged");
    for i in 0..5000u32 {
        let delta = match i % 3 {
            0 => frand(&mut s, -3.5, 3.5), // straddles both ±π edges
            1 => frand(&mut s, -8.0, 8.0), // full (−3π, 3π) range
            _ => frand_bits(&mut s),
        };
        case(&mut e, delta, &format!("fuzz{i}"));
    }
    for &v in &specials_wrap(&[pi, 2.0 * pi, 3.0 * pi]) {
        case(&mut e, v, "special");
    }
    assert!(seen_hi, "Δ>π arm never seen");
    assert!(seen_lo, "Δ<−π arm never seen");
    assert!(seen_mid, "unchanged arm never seen");
}

#[test]
fn u4_facing_box_apply_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x2000); // ESI; prev at +0xc98, history at +0xc9c..
    let hist = e.alloc(0x40); // EDI → 4-sample history (hist[0] set inside the slice)
    let mut s = 0x600c_d300u32;
    let mut seen_neg = false;
    let mut seen_pos = false;

    let tau = f32::from_bits(0x40c9_0fdb);

    let mut case = |e: &mut Emu, delta_in: f32, h1: f32, h2: f32, h3: f32, prev: f32, ctx: &str| {
        e.write_u32(ebp - 4, delta_in.to_bits());
        e.write_u32(hist + 4, h1.to_bits());
        e.write_u32(hist + 8, h2.to_bits());
        e.write_u32(hist + 0xc, h3.to_bits());
        e.write_u32(this + 0xc98, prev.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, hist);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // Reconstruct `new` to pick the reachable output store EIP (new<0 → 0x600ff4, else 0x601013).
        let ours = facing_box_apply(delta_in, h1, h2, h3, prev);
        let new_f = f64::from(ours);
        // `ours` is already post-wrap; the binary's output store happens on the pre-wrap sign. We
        // reconstruct the pre-wrap value to choose `until`.
        let sum = (((f64::from(0.0f32) + f64::from(delta_in)) + f64::from(h1)) + f64::from(h2))
            + f64::from(h3);
        let avg = sum * f64::from(f32::from_bits(0x3e80_0000));
        let d = f64::from(delta_in);
        let selected = if !d.is_nan() && d > 0.0 {
            // Δ>0 → min(Δ,avg): keep Δ iff Δ<avg(ordered), else avg.
            if !avg.is_nan() && d < avg {
                d
            } else {
                avg
            }
        } else if !d.is_nan() && !avg.is_nan() && d > avg {
            // Δ<=0 → max(Δ,avg): keep Δ iff Δ>avg(ordered), else avg.
            d
        } else {
            avg
        };
        // selected rounds to f32 at the [ebp-4] spill before the apply (mirrors the transcription).
        let prewrap =
            f64::from(selected as f32) * f64::from(f32::from_bits(0x3f00_0000)) + f64::from(prev);
        let new_lt0 = !prewrap.is_nan() && prewrap < 0.0;
        let until = if new_lt0 {
            seen_neg = true;
            0x600ff4
        } else {
            seen_pos = true;
            0x601013
        };
        let _ = new_f;
        e.run(0x600f7b, until);
        let bin = e.read_f32(this + 0xc98);
        assert_f32(bin, ours, ctx);
        let _ = tau;
    };

    case(&mut e, 0.1, 0.1, 0.1, 0.1, 1.0, "all +, prev 1");
    case(
        &mut e,
        -0.1,
        -0.1,
        -0.1,
        -0.1,
        0.05,
        "all −, prev tiny → +2π",
    );
    case(&mut e, 0.5, 0.0, 0.0, 0.0, 6.2, "near 2π edge");
    case(&mut e, 0.0, 0.0, 0.0, 0.0, 3.0, "zero delta");
    for i in 0..5000u32 {
        let delta_in = frand(&mut s, -3.2, 3.2);
        let h1 = frand(&mut s, -3.2, 3.2);
        let h2 = frand(&mut s, -3.2, 3.2);
        let h3 = frand(&mut s, -3.2, 3.2);
        let prev = match i % 3 {
            0 => frand(&mut s, 0.0, 6.3), // [0, 2π) plausible
            1 => frand(&mut s, -1.0, 1.0),
            _ => frand(&mut s, 6.0, 7.0), // push new>=2π
        };
        case(&mut e, delta_in, h1, h2, h3, prev, &format!("fuzz{i}"));
    }
    for &v in &specials(&[0.01]) {
        case(&mut e, v, 0.1, 0.2, 0.3, 1.0, "special delta");
        case(&mut e, 0.1, v, 0.2, 0.3, 1.0, "special h1");
        case(&mut e, 0.1, 0.2, v, 0.3, 1.0, "special h2");
        case(&mut e, 0.1, 0.2, 0.3, v, 1.0, "special h3");
        case(&mut e, 0.1, 0.2, 0.3, 0.4, v, "special prev");
    }
    // The output wrap at 0 and 2 pi, and the select with the average equal to the delta.
    for prev in [-0.5f32, 0.0, next_down(0.0), 6.2831855 - 0.1, 6.2831855] {
        case(&mut e, 0.2, 0.2, 0.2, 0.2, prev, "output wrap");
        case(&mut e, -0.2, -0.2, -0.2, -0.2, prev, "output wrap neg");
    }
    assert!(seen_neg, "new<0 output arm never seen");
    assert!(seen_pos, "new>=0 output arm never seen");
}

#[test]
fn u4_airborne_reach_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x1000); // ESI; reach at +0xa5c
    let hit = e.alloc(0x20); // EAX; hit_z at +8
    let mut s = 0x6056_5000u32;
    let mut seen_air = false;
    let mut seen_ground = false;

    let mut case = |e: &mut Emu, self_z: f32, hit_z: f32, reach: f32, ctx: &str| {
        e.write_u32(ebp - 4, self_z.to_bits());
        e.write_u32(hit + 8, hit_z.to_bits());
        e.write_u32(this + 0xa5c, reach.to_bits());
        e.set_reg(RegisterX86::EAX, hit);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6056ac, 0x6056ca);
        let m = ah(e) & 0x41;
        // fcompp height vs reach·0.5; C0=height<reach·0.5, C3=eq. test ah,0x41; jne 0x6056d8 → ret 0.
        // return 1 (airborne) ⟺ (ah&0x41)==0 ⟺ height>reach·0.5 ordered.
        let bin_air = m == 0;
        if bin_air {
            seen_air = true;
        } else {
            seen_ground = true;
        }
        assert_eq!(
            bin_air,
            airborne_reach_gate(self_z, hit_z, reach),
            "{ctx}: bin_air={bin_air} ah&0x41={m:#04x} self_z={self_z} hit_z={hit_z} reach={reach}"
        );
    };

    case(&mut e, 10.0, 0.0, 1.0, "h10 > 0.5 → air");
    case(&mut e, 0.5, 0.0, 1.0, "h0.5 == reach·0.5 → ground");
    case(&mut e, 0.1, 0.0, 1.0, "h0.1 < 0.5 → ground");
    case(&mut e, 0.0, 0.0, f32::NAN, "nan reach → ground");
    for i in 0..5000u32 {
        let hit_z = frand(&mut s, -50.0, 50.0);
        let self_z = match i % 3 {
            0 => hit_z + frand(&mut s, -2.0, 2.0), // straddle the reach·0.5 boundary
            1 => hit_z + frand(&mut s, 0.0, 30.0),
            _ => frand_bits(&mut s),
        };
        let reach = match i % 4 {
            0 => frand(&mut s, 0.0, 4.0),
            _ => frand(&mut s, 0.0, 60.0),
        };
        case(&mut e, self_z, hit_z, reach, &format!("fuzz{i}"));
    }
    // `height == reach / 2` exactly (reach 2, height 1) and a step either side, then the specials.
    for z in [1.0f32, next_up(1.0), next_down(1.0)] {
        case(&mut e, z, 0.0, 2.0, "edge");
    }
    for r in [next_up(2.0), next_down(2.0)] {
        case(&mut e, 1.0, 0.0, r, "reach step");
    }
    for &v in &specials(&[1.0, 2.0]) {
        case(&mut e, v, 0.0, 2.0, "special self");
        case(&mut e, 1.0, v, 2.0, "special hit");
        case(&mut e, 1.0, 0.0, v, "special reach");
    }
    assert!(seen_air, "airborne (height>reach·0.5) never seen");
    assert!(seen_ground, "grounded never seen");
}

const INVERT_FLAG: u64 = 0xc4daa4;

/// The count is an `i32` and the factors 0.002 and 128.0, so the `__ftol` input is at most about
/// 5.5e8 in magnitude and never NaN or out of range.
#[test]
fn u4_spin_cast_byte_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x607f_9b00u32;
    let mut seen_inv = false;
    let mut seen_dir = false;

    let mut case = |e: &mut Emu, n: i32, invert: bool, ctx: &str| {
        e.write_u32(ebp - 0xc, n as u32);
        e.write_u32(INVERT_FLAG, u32::from(invert));
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x607f9b, 0x607fbe);
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        if invert {
            seen_inv = true;
        } else {
            seen_dir = true;
        }
        assert_eq!(
            bin,
            spin_cast_byte(n, invert),
            "{ctx}: bin={bin} n={n} invert={invert}"
        );
    };

    case(&mut e, 0, false, "0 dir");
    case(&mut e, 0, true, "0 inv → 128");
    case(&mut e, 250, false, "250 dir → 64000? no, ×0.002×128");
    case(&mut e, 100, true, "100 inv");
    case(&mut e, -50, false, "neg dir");
    for i in 0..4000u32 {
        // n stays in a sane range so frac·128 fits an i32 cleanly (the spin period is ~500 ticks).
        let n = ((next(&mut s) >> 8) % 2000) as i32 - 500;
        let invert = (next(&mut s) >> 8).is_multiple_of(2);
        case(&mut e, n, invert, &format!("fuzz{i}"));
    }
    for n in [
        0,
        1,
        -1,
        500,
        1000,
        i32::MAX,
        i32::MIN,
        0x4000_0000,
        -0x4000_0000,
    ] {
        for invert in [false, true] {
            case(&mut e, n, invert, "extremes");
        }
    }
    assert!(seen_inv, "invert arm never seen");
    assert!(seen_dir, "direct arm never seen");
}

#[test]
fn u4_turn_step_cap_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x1000); // ESI; turn-rate at +0xa44
    let mut s = 0x6081_dd00u32;
    let mut seen_step = false;
    let mut seen_cap = false;

    let mut case = |e: &mut Emu, tick_delta: i64, rate: f32, abs_delta: f32, ctx: &str| {
        e.write_u32(ebp - 0x18, (tick_delta as u64 & 0xffff_ffff) as u32);
        e.write_u32(ebp - 0x14, ((tick_delta as u64 >> 32) & 0xffff_ffff) as u32);
        e.write_u32(this + 0xa44, rate.to_bits());
        e.write_u32(ebp - 4, abs_delta.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6081dd, 0x608207);
        let bin = e.read_f32(ebp - 0xc);
        let ours = turn_step_cap(tick_delta, rate, abs_delta);
        let step = ((tick_delta as f64) * f64::from(f32::from_bits(0x3a83_126f)) * f64::from(rate))
            * f64::from(8.0f32);
        if !step.is_nan() && step > f64::from(abs_delta) {
            seen_cap = true;
        } else {
            seen_step = true;
        }
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 100, 0.01, 1.0, "step < |Δ| → step");
    case(&mut e, 100, 0.01, 0.001, "step > |Δ| → cap");
    case(&mut e, 0, 0.01, 0.5, "zero tick");
    case(&mut e, 33, 0.0349, 6.2, "typical frame");
    for i in 0..4000u32 {
        let tick_delta = i64::from((next(&mut s) >> 8) % 200); // ms per frame, 0..199
        let rate = frand(&mut s, 0.0, 0.2);
        let abs_delta = match i % 3 {
            0 => frand(&mut s, 0.0, 0.5), // straddle small steps
            1 => frand(&mut s, 0.0, 3.2),
            _ => frand_bits(&mut s).abs(),
        };
        case(&mut e, tick_delta, rate, abs_delta, &format!("fuzz{i}"));
    }
    // `step == abs_delta` exactly (a step of 8 * 0.001f32 * 1000 against its own value) and a
    // step either side, the tick delta at its extremes, then the specials.
    let step = 1000.0 * f64::from(MS_TO_S) * 1.0 * f64::from(TURN_GAIN);
    for a in [step as f32, next_up(step as f32), next_down(step as f32)] {
        case(&mut e, 1000, 1.0, a, "edge");
    }
    for td in [0i64, 1, -1, i64::MAX, i64::MIN, 1 << 40] {
        case(&mut e, td, 0.5, 1.0, "tick extremes");
    }
    for &v in &specials(&[]) {
        case(&mut e, 1000, v, 1.0, "special rate");
        case(&mut e, 1000, 1.0, v, "special abs");
    }
    assert!(seen_step, "keep-step arm never seen");
    assert!(seen_cap, "cap-to-|Δ| arm never seen");
}

#[test]
fn u4_lean_smooth_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let mut s = 0x6080_9a00u32;
    let mut seen_narrow = false; // π/4
    let mut seen_wide = false; // π/2
    let mut seen_neg = false; // fchs applied

    let mut case = |e: &mut Emu, flags: u32, facing: f32, displayed: f32, ctx: &str| {
        e.write_u32(this + 0x9e8, flags);
        e.write_u32(this + 0xc98, facing.to_bits());
        e.write_u32(this + 0xc94, displayed.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60809a, 0x6080f6);
        let bin = e.read_f32(this + 0xc94);
        if flags & 3 != 0 {
            seen_narrow = true;
        } else {
            seen_wide = true;
        }
        if flags & 2 == (flags >> 1) & 2 {
            seen_neg = true;
        }
        assert_f32(bin, lean_smooth(flags, facing, displayed), ctx);
    };

    case(&mut e, 0, 1.0, 0.0, "wide, fchs");
    case(&mut e, 1, 1.0, 0.0, "narrow");
    case(&mut e, 3, 2.0, 1.0, "narrow, fchs");
    case(&mut e, 2, 0.5, 0.5, "wide");
    for i in 0..4000u32 {
        let flags = (next(&mut s) >> 8) & 0x7; // exercise bits 0..2 (the threshold + fchs gate)
        let facing = frand(&mut s, 0.0, 6.3);
        let displayed = frand(&mut s, 0.0, 6.3);
        case(&mut e, flags, facing, displayed, &format!("fuzz{i}"));
    }
    for flags in 0u32..8 {
        for &v in &specials_wrap(&[]) {
            case(&mut e, flags, v, 0.5, "special facing");
            case(&mut e, flags, 0.5, v, "special displayed");
        }
    }
    assert!(seen_narrow, "π/4 threshold never seen");
    assert!(seen_wide, "π/2 threshold never seen");
    assert!(seen_neg, "fchs (negated threshold) never seen");
}

#[test]
fn u4_turn_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x2000);
    let mut s = 0x6081_1900u32;

    let case = |e: &mut Emu, facing: f32, displayed: f32, ctx: &str| {
        e.write_u32(this + 0xc98, facing.to_bits());
        e.write_u32(this + 0xc94, displayed.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x608119, 0x60813d);
        let bin_signed = e.read_f32(ebp - 8);
        let bin_abs = e.read_f32(ebp - 4);
        let (signed, abs) = turn_delta(facing, displayed);
        assert_f32(bin_signed, signed, &format!("{ctx} signed"));
        assert_f32(bin_abs, abs, &format!("{ctx} abs"));
    };

    case(&mut e, 1.0, 0.0, "simple");
    case(&mut e, 0.0, 5.0, "wraps");
    case(&mut e, 3.1, 0.0, "near π");
    for i in 0..4000u32 {
        let facing = frand(&mut s, 0.0, 6.3);
        let displayed = match i % 2 {
            0 => frand(&mut s, 0.0, 6.3),
            _ => frand(&mut s, -8.0, 8.0), // push the wrap arms
        };
        case(&mut e, facing, displayed, &format!("fuzz{i}"));
    }
    for &v in &specials_wrap(&[]) {
        case(&mut e, v, 0.5, "special facing");
        case(&mut e, 0.5, v, "special displayed");
    }
}

#[test]
fn u4_turn_apply_step_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x2000);
    let mut s = 0x6082_2700u32;

    let case = |e: &mut Emu, accum: f32, displayed: f32, ctx: &str| {
        e.write_u32(ebp + 8, accum.to_bits());
        e.write_u32(this + 0xc94, displayed.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x608227, 0x60823f);
        let bin = e.read_f32(this + 0xc94);
        assert_f32(bin, turn_apply_step(accum, displayed), ctx);
    };

    case(&mut e, 0.1, 1.0, "small step");
    case(&mut e, 3.0, 4.0, "wraps");
    case(&mut e, -0.2, 0.0, "neg step");
    for i in 0..4000u32 {
        let accum = frand(&mut s, -3.5, 3.5);
        let displayed = match i % 2 {
            0 => frand(&mut s, 0.0, 6.3),
            _ => frand(&mut s, -8.0, 8.0),
        };
        case(&mut e, accum, displayed, &format!("fuzz{i}"));
    }
    for &v in &specials_wrap(&[]) {
        case(&mut e, v, 0.5, "special accum");
        case(&mut e, 0.5, v, "special displayed");
    }
}
