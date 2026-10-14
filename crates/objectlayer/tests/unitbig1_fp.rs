//! The regions of `CGUnit_C::EmitWaterRipple` (`0x5fa760`) in `src/unitbig1_fp.rs` against the exe.
//! Each runs the slice of the function it covers, with the registers, frame and fields it reads set
//! directly, reading the store it ends in:
//! - R1 `[0x5fa7fc, 0x5fa81f)`, R2 `[0x5fa81f, 0x5fa833)`, R3 `[0x5fa836, 0x5fa847)`;
//! - R4a `[0x5fa89c, 0x5fa8d2)`, R4b `[0x5fa8d4, 0x5fa8e4)`, R5 `[0x5fa8ee, 0x5fa8ff)`;
//! - R6 `[0x5fa911, 0x5fa95f)`, R7 `[0x5fa976, 0x5fa999)`, R8 `[0x5fa9ab, 0x5fa9e6)`;
//! - R9 `[0x5fa9e6, 0x5faa2b)`, R10 `[0x5faa2f, 0x5faa5f)`;
//! - R11 `[0x5faa80, 0x5faaf8)` (random pan) and `[0x5faa91, 0x5faaf8)` (directional pan);
//! - R12 `[0x5fab50, 0x5fab67)`, through `__ftol`.
//!
//! The random float `u` in `[1, 2)` comes from the test's generator. Each test adds NaN, the
//! infinities, both zeros and its thresholds, equal and a step either side, to its random inputs;
//! the `__ftol` leg adds products past `i32` and `i64`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::unitbig1_fp::*;
use common::*;

// The RNG getter `0x4531e0` (`CRandom::uint32`-family) builds the [1,2) jitter floats — but every
// RNG-fed region's slice starts AFTER the `and 0x7fffff; or 0x3f800000` (we write `[ebp-0x1c]` /
// `[ebp+8]` directly), so the RNG call itself is never executed in any slice. No stub is needed.

// ---- shared helpers (the crate float idiom; per unith_fp.rs / unitm3_fp.rs) ----

/// A fuzzed [1,2) float exactly as the binary builds it: `(rng & 0x7fffff) | 0x3f800000`.
fn frand_unit(s: &mut u32) -> f32 {
    f32::from_bits((next(s) & 0x7f_ffff) | 0x3f80_0000)
}

#[test]
fn b1_clamped_reach_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x1000); // ESI; reach @+0xa5c
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5fa7_fc00u32;
    let mut seen_two = false;
    let mut seen_floor = false;

    let mut case = |e: &mut Emu, reach: f32, ctx: &str| {
        e.write_u32(this + 0xa5c, reach.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fa7fc, 0x5fa81f);
        let bin = e.read_f32(ebp - 8);
        if bin.to_bits() == ONE.to_bits() {
            seen_floor = true;
        } else {
            seen_two = true;
        }
        assert_f32(bin, clamped_reach(reach), ctx);
    };

    case(&mut e, 1.0, "2·1>1 → 2.0");
    case(&mut e, 0.5, "2·0.5==1 → floor 1.0");
    case(&mut e, 0.25, "2·0.25<1 → floor 1.0");
    case(&mut e, 0.5000001, "2·reach just >1 → 2·reach");
    case(&mut e, f32::NAN, "nan → floor 1.0");
    for i in 0..4000u32 {
        let reach = match i % 3 {
            0 => frand(&mut s, 0.0, 1.0), // straddle the 0.5 floor boundary
            1 => frand(&mut s, 0.0, 60.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, reach, &format!("fuzz{i}"));
    }
    // `2 * reach == 1.0` and a step either side, then the specials.
    for r in [0.5f32, next_up(0.5), next_down(0.5)] {
        case(&mut e, r, "floor edge");
    }
    for &v in &specials(&[0.5]) {
        case(&mut e, v, "special");
    }
    assert!(seen_two, "2·reach arm never seen");
    assert!(seen_floor, "floor-to-1.0 arm never seen");
}

#[test]
fn b1_half_reach_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x80); // ESI; `mov eax,[esi]` (value unused)
    let mut s = 0x5fa8_1f00u32;

    let case = |e: &mut Emu, cr: f32, ctx: &str| {
        e.write_u32(ebp - 8, cr.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fa81f, 0x5fa833);
        assert_f32(e.read_f32(ebp - 0x28), half_reach(cr), ctx);
    };

    case(&mut e, 2.0, "2·0.5=1");
    case(&mut e, 1.0, "1·0.5=0.5");
    case(&mut e, 10.0, "10·0.5=5");
    for i in 0..3000u32 {
        let cr = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 1.0, 120.0)
        };
        case(&mut e, cr, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn b1_ground_height_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let pos = e.alloc(0x20); // EAX; self_z @+8
    let mut s = 0x5fa8_3600u32;
    let mut seen_proceed = false;
    let mut seen_bail = false;

    let mut case = |e: &mut Emu, ground_z: f32, self_z: f32, clamped: f32, ctx: &str| {
        e.write_u32(ebp - 0x24, ground_z.to_bits());
        e.write_u32(pos + 8, self_z.to_bits());
        e.write_u32(ebp - 8, clamped.to_bits());
        e.set_reg(RegisterX86::EAX, pos);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fa836, 0x5fa847);
        let bin_h = e.read_f32(ebp - 0x18);
        let (our_h, our_proceed) = ground_height_gate(ground_z, self_z, clamped);
        assert_f32(bin_h, our_h, &format!("{ctx} height"));
        let m = ah(e) & 0x1;
        // je 0x5fac59 (bail) ⟺ (ah&1)==0 ⟺ C0 clear. proceed (je not taken) ⟺ (ah&1)!=0.
        let bin_proceed = m != 0;
        if bin_proceed {
            seen_proceed = true;
        } else {
            seen_bail = true;
        }
        assert_eq!(
            bin_proceed, our_proceed,
            "{ctx}: proceed={bin_proceed} ah&1={m:#04x} g={ground_z} s={self_z} c={clamped}"
        );
    };

    case(&mut e, 0.5, 0.0, 2.0, "h0.5 < 2 → proceed");
    case(&mut e, 2.0, 0.0, 2.0, "h2 == clamped → bail (>=)");
    case(&mut e, 5.0, 0.0, 2.0, "h5 > 2 → bail");
    case(&mut e, f32::NAN, 0.0, 2.0, "nan height → proceed (C0 set)");
    // C0 set) but f32(height) rounds-half-to-even → 2.0 == clamped (a prior `f64(height_f32) >= clamped`
    // bailed). Lands the exact boundary `fcomp clamped` reads on the live f64, not the f32 store.
    case(
        &mut e,
        2.0,
        f32::from_bits(0x3380_0000),
        2.0,
        "fst-no-pop ½ulp: height_live<clamped but f32(height)==clamped → proceed, not bail",
    );
    for i in 0..5000u32 {
        let self_z = frand(&mut s, -50.0, 50.0);
        let clamped = frand(&mut s, 1.0, 30.0);
        let ground_z = match i % 3 {
            0 => self_z + clamped + frand(&mut s, -1.0, 1.0), // straddle height==clamped
            1 => self_z + frand(&mut s, 0.0, 30.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, ground_z, self_z, clamped, &format!("fuzz{i}"));
    }
    // `height == clamped` and a step either side, then the specials.
    for g in [3.0f32, next_up(3.0), next_down(3.0)] {
        case(&mut e, g, 0.0, 3.0, "edge");
    }
    for &v in &specials(&[3.0]) {
        case(&mut e, v, 0.0, 3.0, "special ground");
        case(&mut e, 3.0, v, 3.0, "special self");
        case(&mut e, 3.0, 0.0, v, "special clamped");
    }
    assert!(seen_proceed, "proceed arm never seen");
    assert!(seen_bail, "bail arm never seen");
}

#[test]
fn b1_speed_rate_recip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5fa8_9c00u32;
    let mut seen_cap = false;
    let mut seen_speed = false;

    let mut case = |e: &mut Emu, speed: f32, ctx: &str| {
        e.write_u32(ebp + 8, speed.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fa89c, 0x5fa8d2);
        let bin = e.read_f32(ebp - 0x20);
        // track which clamp leg fired (for coverage)
        if speed < 20.0 {
            seen_cap = true;
        } else {
            seen_speed = true;
        }
        assert_f32(bin, speed_rate_recip(speed), ctx);
    };

    case(&mut e, 5.0, "5<20 → 2.5/20");
    case(&mut e, 40.0, "40>20 → 2.5/40");
    case(&mut e, 20.0, "==20 → speed leg → 2.5/20");
    case(&mut e, 20.000002, "just >20 → 2.5/speed");
    case(&mut e, 19.999998, "just <20 → 2.5/20");
    for i in 0..4000u32 {
        let speed = match i % 3 {
            0 => frand(&mut s, 15.0, 25.0), // straddle 20.0
            1 => frand(&mut s, 0.0, 60.0),
            _ => frand(&mut s, 0.0, 200.0),
        };
        case(&mut e, speed, &format!("fuzz{i}"));
    }
    // `speed == 20` and a step either side, then the specials.
    for sp in [20.0f32, next_up(20.0), next_down(20.0)] {
        case(&mut e, sp, "cap edge");
    }
    // The slice is only entered for a speed above 1e-4: a gate before it skips the recip for 0,
    // negatives, NaN and tiny speeds.
    for &v in specials(&[20.0]).iter().filter(|v| **v > 1.0e-4) {
        case(&mut e, v, "special");
    }
    assert!(seen_cap, "speed<20 (cap) leg never seen");
    assert!(seen_speed, "speed>=20 leg never seen");
}

#[test]
fn b1_volume_recip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let this = e.alloc(0x80); // ESI; `mov edx,[esi]` (value unused)
    let mut s = 0x5fa8_d400u32;

    let case = |e: &mut Emu, recip: f32, ctx: &str| {
        e.write_u32(ebp - 0x20, recip.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fa8d4, 0x5fa8e4);
        assert_f32(e.read_f32(ebp - 0x14), volume_recip(recip), ctx);
    };

    case(&mut e, 0.5, "1/0.5=2");
    case(&mut e, 0.125, "1/0.125=8");
    case(&mut e, 1.0, "1/1=1");
    case(&mut e, 0.0, "1/0=+inf");
    case(&mut e, -0.0, "1/-0=-inf");
    for i in 0..3000u32 {
        let recip = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.01, 2.6) // the realistic 2.5/max(speed,20) range
        };
        case(&mut e, recip, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn b1_pitch_base_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x80); // ESI; [esi]=vtbl; EAX=[esi]
    let vtbl = e.alloc(0x80);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let slot = e.alloc(0x10); // the float the stub loads onto st0
    let stub = e.alloc(0x10); // `fld dword[slot]; ret` (d9 05 <slot LE32> c3)
    let mut s = 0x5fa8_ee00u32;

    let mut stub_bytes = vec![0xd9u8, 0x05];
    stub_bytes.extend_from_slice(&(slot as u32).to_le_bytes());
    stub_bytes.push(0xc3);
    e.write(stub, &stub_bytes);
    e.write_u32(this, vtbl as u32);
    e.write_u32(vtbl + 0x1c, stub as u32);

    let case = |e: &mut Emu, orient: f32, ctx: &str| {
        e.write_u32(slot, orient.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        // The slice STARTS at 0x5fa8ee (`call [eax+0x1c]`) — the `mov eax,[esi]` at 0x5fa8ea is
        // BEFORE it — so EAX must already be the vtbl ptr. The `fstp [ebp-0x4]` is at 0x5fa8fc, so
        // `until` = 0x5fa8ff to execute it.
        e.set_reg(RegisterX86::EAX, vtbl);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fa8ee, 0x5fa8ff);
        assert_f32(e.read_f32(ebp - 0x4), pitch_base(orient), ctx);
    };

    case(&mut e, 3.0, "3·(1/3)≈1");
    case(&mut e, 1.0, "1·(1/3)=1/3");
    case(&mut e, 0.0, "0");
    case(&mut e, -6.2831855, "neg orient");
    for i in 0..3000u32 {
        let orient = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -7.0, 7.0)
        };
        case(&mut e, orient, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn b1_pitch_jitter_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5fa9_1100u32;
    let mut seen_lo = false; // snapped to 1/3
    let mut seen_hi = false; // snapped to 5/3

    let mut case = |e: &mut Emu, u: f32, base: f32, ctx: &str| {
        e.write_u32(ebp - 0x1c, u.to_bits());
        e.write_u32(ebp - 0x4, base.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fa911, 0x5fa95f);
        let bin = e.read_f32(ebp - 0x4);
        if bin.to_bits() == ONE_THIRD.to_bits() {
            seen_lo = true;
        }
        if bin.to_bits() == FIVE_THIRD.to_bits() {
            seen_hi = true;
        }
        assert_f32(bin, pitch_jitter(u, base), ctx);
    };

    // v = base·(0.9+0.2·(u−1)); for u∈[1,2) the factor ∈[0.9,1.1). Clamp v into [1/3,5/3]:
    //   v<1/3 → 1/3 ; v>=5/3 → 5/3 ; else keep v.
    case(&mut e, 1.0, 1.0, "v≈0.9 in-band → keep");
    case(&mut e, 1.5, 3.0, "v≈3 >= 5/3 → snap 5/3");
    case(&mut e, 1.0, 0.3, "v≈0.27 < 1/3 → snap 1/3");
    case(&mut e, 1.5, 0.1, "v tiny < 1/3 → snap 1/3");
    case(&mut e, 1.0, 0.37, "≈1/3 boundary probe");
    case(&mut e, 1.0, 1.85, "≈5/3 boundary probe");
    case(
        &mut e,
        1.0,
        f32::NAN,
        "nan base → clamp1 jp taken → clamp2 → snap 5/3",
    );
    for i in 0..6000u32 {
        let u = frand_unit(&mut s);
        let base = match i % 4 {
            0 => frand(&mut s, 0.3, 0.4),   // straddle the 1/3 clamp boundary
            1 => frand(&mut s, 1.45, 1.95), // straddle the 5/3 clamp boundary
            2 => frand(&mut s, -2.0, 2.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, u, base, &format!("fuzz{i}"));
    }
    for &v in &specials(&[1.0, ONE_THIRD, FIVE_THIRD]) {
        case(&mut e, v, 1.0, "special u");
        case(&mut e, 1.5, v, "special base");
    }
    assert!(seen_lo, "snap-to-1/3 arm never seen");
    assert!(seen_hi, "snap-to-5/3 arm never seen");
}

#[test]
fn b1_jitter_pc_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5fa9_7600u32;

    let case = |e: &mut Emu, u: f32, ctx: &str| {
        e.write_u32(ebp - 0x1c, u.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fa976, 0x5fa999);
        assert_f32(e.read_f32(ebp - 0xc), jitter_pc(u), ctx);
    };

    case(&mut e, 1.0, "u1 → 0.6");
    case(&mut e, 2.0, "u2 → 0.7 (u−1=1)");
    case(&mut e, 1.5, "u1.5 → 0.65");
    for i in 0..3000u32 {
        // exercise the real [1,2) domain plus full-range for robustness
        let u = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand_unit(&mut s)
        };
        case(&mut e, u, &format!("fuzz{i}"));
    }
    for &v in &specials(&[1.0]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn b1_jitter_p1c_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5fa9_ab00u32;

    let case = |e: &mut Emu, u: f32, vol: f32, ctx: &str| {
        e.write_u32(ebp - 0x1c, u.to_bits());
        e.write_u32(ebp - 0x14, vol.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fa9ab, 0x5fa9e6);
        assert_f32(e.read_f32(ebp - 0x1c), jitter_p1c(u, vol), ctx);
    };

    case(&mut e, 1.0, 1.0, "u1 vol1 → (3.0)·12/36 = 1.0");
    case(&mut e, 2.0, 1.0, "u2 vol1 → (4.5)·12/36 = 1.5");
    case(&mut e, 1.5, 2.0, "u1.5 vol2");
    for i in 0..3000u32 {
        let u = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand_unit(&mut s)
        };
        let vol = match i % 3 {
            0 => frand(&mut s, 0.3, 8.0),
            1 => frand_bits(&mut s),
            _ => frand(&mut s, -4.0, 4.0),
        };
        case(&mut e, u, vol, &format!("fuzz{i}"));
    }
    for &v in &specials(&[1.0]) {
        case(&mut e, v, 1.0, "special u");
        case(&mut e, 1.5, v, "special vol");
    }
}

#[test]
fn b1_height_blend_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5fa9_e600u32;
    let mut seen_run = false;
    let mut seen_skip = false;

    // prior [ebp-0x14] (p14) value the skip-path must leave untouched.
    let prior_p14 = f32::from_bits(0x3e2a_aaab); // the binary's 1/6 const-init

    let mut case = |e: &mut Emu, height: f32, half: f32, r: f32, pc: f32, p4: f32, ctx: &str| {
        e.write_u32(ebp - 0x18, height.to_bits());
        e.write_u32(ebp - 0x28, half.to_bits());
        e.write_u32(ebp - 8, r.to_bits());
        e.write_u32(ebp - 0xc, pc.to_bits());
        e.write_u32(ebp - 0x4, p4.to_bits());
        e.write_u32(ebp - 0x14, prior_p14.to_bits()); // R8's const-init [ebp-0x14]=1/6
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fa9e6, 0x5faa2b);
        let (op14, opc, op4, run) = height_blend(height, half, r, pc, p4, prior_p14);
        if run {
            seen_run = true;
        } else {
            seen_skip = true;
        }
        assert_f32(e.read_f32(ebp - 0x14), op14, &format!("{ctx} p14"));
        assert_f32(e.read_f32(ebp - 0xc), opc, &format!("{ctx} pc"));
        assert_f32(e.read_f32(ebp - 0x4), op4, &format!("{ctx} p4"));
    };

    // run path needs height>half ordered; r is the clamped reach (>= height to keep t sane, but the
    // transcription is bit-exact regardless of magnitude).
    case(&mut e, 1.5, 1.0, 2.0, 0.6, 1.0, "height>half → blend");
    case(&mut e, 1.0, 1.0, 2.0, 0.6, 1.0, "height==half → skip");
    case(&mut e, 0.5, 1.0, 2.0, 0.6, 1.0, "height<half → skip");
    case(&mut e, f32::NAN, 1.0, 2.0, 0.6, 1.0, "nan height → skip");
    // a premature f32-round of t/b would diverge.
    case(
        &mut e,
        163200.33,
        1.0,
        632_252.7,
        0.6123457,
        1.3123457,
        "live-t large magnitude",
    );
    case(
        &mut e,
        8388609.0,
        1.0,
        16777216.0,
        0.7,
        1.3,
        "live-t f32-gap regime",
    );
    for i in 0..5000u32 {
        let half = frand(&mut s, 0.5, 30.0);
        let height = match i % 3 {
            0 => half + frand(&mut s, -1.0, 1.0), // straddle height==half
            1 => half + frand(&mut s, 0.0, 20.0),
            _ => frand(&mut s, -50.0, 50.0),
        };
        let r = frand(&mut s, 1.0, 80.0);
        let pc = frand(&mut s, 0.5, 0.8);
        let p4 = frand(&mut s, 0.3, 1.7);
        case(&mut e, height, half, r, pc, p4, &format!("fuzz{i}"));
    }
    // `height == half` and a step either side, then the specials in each input.
    for h in [3.0f32, next_up(3.0), next_down(3.0)] {
        case(&mut e, h, 3.0, 10.0, 0.5, 0.5, "edge");
    }
    for &v in &specials(&[3.0]) {
        case(&mut e, v, 3.0, 10.0, 0.5, 0.5, "special height");
        case(&mut e, 5.0, v, 10.0, 0.5, 0.5, "special half");
        case(&mut e, 5.0, 3.0, v, 0.5, 0.5, "special r");
        case(&mut e, 5.0, 3.0, 10.0, v, 0.5, "special pc");
        case(&mut e, 5.0, 3.0, 10.0, 0.5, v, "special p4");
    }
    assert!(seen_run, "blend run arm never seen");
    assert!(seen_skip, "blend skip arm never seen");
}

#[test]
fn b1_material0_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5faa_2f00u32;

    let case = |e: &mut Emu, p14: f32, pc: f32, p1c: f32, p4: f32, ctx: &str| {
        e.write_u32(ebp - 0x14, p14.to_bits());
        e.write_u32(ebp - 0xc, pc.to_bits());
        e.write_u32(ebp - 0x1c, p1c.to_bits());
        e.write_u32(ebp - 0x4, p4.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5faa2f, 0x5faa5f);
        let (o14, opc, o1c, op4) = material0_scale(p14, pc, p1c, p4);
        assert_f32(e.read_f32(ebp - 0x14), o14, &format!("{ctx} p14"));
        assert_f32(e.read_f32(ebp - 0xc), opc, &format!("{ctx} pc"));
        assert_f32(e.read_f32(ebp - 0x1c), o1c, &format!("{ctx} p1c"));
        assert_f32(e.read_f32(ebp - 0x4), op4, &format!("{ctx} p4"));
    };

    case(&mut e, 1.0, 1.0, 1.0, 1.0, "unit → 0.8/1/0.25/0.6");
    case(&mut e, 0.5, 0.65, 1.2, 1.0, "typical params");
    for i in 0..3000u32 {
        let f = |s: &mut u32| {
            if next(s).is_multiple_of(4) {
                frand_bits(s)
            } else {
                frand(s, -3.0, 3.0)
            }
        };
        case(
            &mut e,
            f(&mut s),
            f(&mut s),
            f(&mut s),
            f(&mut s),
            &format!("fuzz{i}"),
        );
    }
    for &v in &specials(&[]) {
        case(&mut e, v, v, v, v, "special all");
        case(&mut e, v, 1.0, 2.0, 3.0, "special p14");
    }
}

#[test]
fn b1_pan_angle_random_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5faa_8000u32;

    let case = |e: &mut Emu, u: f32, ctx: &str| {
        e.write_u32(ebp + 8, u.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5faa80, 0x5faaf8);
        assert_f32(e.read_f32(ebp + 8), pan_angle_random(u), ctx);
    };

    case(&mut e, 1.0, "u1 → 0");
    case(&mut e, 2.0, "u2 → 2π (u−1=1)");
    case(&mut e, 1.5, "u1.5 → π");
    for i in 0..3000u32 {
        let u = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand_unit(&mut s)
        };
        case(&mut e, u, &format!("fuzz{i}"));
    }
    for &v in &specials(&[1.0]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn b1_pan_angle_directional_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200); // ESI; [esi+0x118] = move block ptr
    let mblk = e.alloc(0x80); // move block; flags @+0x40
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5faa_9100u32;
    e.write_u32(this + 0x118, mblk as u32);

    let mut seen = [false; 9];
    let mut case = |e: &mut Emu, orient: f32, mf: u8, arm: usize, ctx: &str| {
        e.write_u32(mblk + 0x40, u32::from(mf));
        e.write_u32(ebp + 8, orient.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5faa91, 0x5faaf8);
        seen[arm] = true;
        assert_f32(e.read_f32(ebp + 8), pan_angle_directional(orient, mf), ctx);
    };

    // exercise every move-flag arm explicitly (arm index = which offset).
    case(&mut e, 1.0, 4 | 1, 0, "mf&4,&1 → +π/4");
    case(&mut e, 1.0, 4 | 2, 1, "mf&4,&2 → +3π/4");
    case(&mut e, 1.0, 4, 2, "mf&4 only → +π/2");
    case(&mut e, 1.0, 8 | 1, 3, "mf&8,&1 → −π/4");
    case(&mut e, 1.0, 8 | 2, 4, "mf&8,&2 → −3π/4");
    case(&mut e, 1.0, 8, 5, "mf&8 only → −π/2");
    case(&mut e, 1.0, 2, 6, "mf&2 (no 4/8) → +π");
    case(&mut e, 1.0, 0, 7, "mf none → orient unchanged");
    case(&mut e, 1.0, 1, 8, "mf&1 only (no 4/8/2) → orient unchanged");
    for i in 0..4000u32 {
        let orient = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -7.0, 7.0)
        };
        let mf = ((next(&mut s) >> 8) & 0x0f) as u8; // exercise the 4/8/2/1 bits
        let arm = if mf & 4 != 0 {
            if mf & 1 != 0 {
                0
            } else if mf & 2 != 0 {
                1
            } else {
                2
            }
        } else if mf & 8 != 0 {
            if mf & 1 != 0 {
                3
            } else if mf & 2 != 0 {
                4
            } else {
                5
            }
        } else if mf & 2 != 0 {
            6
        } else {
            7
        };
        case(&mut e, orient, mf, arm, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for mf in 0u8..16 {
            let arm = if mf & 4 != 0 {
                if mf & 1 != 0 {
                    0
                } else if mf & 2 != 0 {
                    1
                } else {
                    2
                }
            } else if mf & 8 != 0 {
                if mf & 1 != 0 {
                    3
                } else if mf & 2 != 0 {
                    4
                } else {
                    5
                }
            } else if mf & 2 != 0 {
                6
            } else if mf & 1 != 0 {
                8
            } else {
                7
            };
            case(&mut e, v, mf, arm, "special orient");
        }
    }
    for (i, hit) in seen.iter().enumerate() {
        assert!(*hit, "directional arm {i} never seen");
    }
}

#[test]
fn b1_dist_atten_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x5fab_5000u32;

    let case = |e: &mut Emu, b: f32, recip: f32, ctx: &str| {
        e.write_u32(ebp - 0x10, b.to_bits());
        e.write_u32(ebp - 0x20, recip.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fab50, 0x5fab67);
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        assert_eq!(bin, dist_atten(b, recip), "{ctx}: b={b} recip={recip}");
    };

    case(&mut e, 1.0, 0.5, "1·0.5·0.25·-1000 = -125");
    case(&mut e, 0.75, 2.5, "0.75·2.5·0.25·-1000");
    case(&mut e, 1.0, 0.125, "1·0.125·0.25·-1000 = -31.25 → -31");
    case(&mut e, 0.0, 1.0, "0 → 0");
    case(&mut e, -1.0, 0.5, "neg b → +125");
    for i in 0..3000u32 {
        // realistic ranges: b = 0.5·t+0.5 ∈ roughly (0,1.5], recip = 2.5/max(speed,20) ∈ (0,2.5].
        let b = frand(&mut s, -1.5, 1.5);
        let recip = frand(&mut s, 0.0, 2.6);
        case(&mut e, b, recip, &format!("fuzz{i}"));
    }
    // Products past `i32` keep the low dword of the 64-bit result and products past `i64` give 0.
    for &(b, r) in &[
        (1.0e8f32, 1.0f32),
        (1.0e15, 1.0),
        (1.0e20, 1.0),
        (1.0e30, 1.0e30),
        (-1.0e15, 1.0),
    ] {
        case(&mut e, b, r, "wide");
    }
    for &v in &specials(&[]) {
        case(&mut e, v, 2.0, "special b");
        case(&mut e, 2.0, v, "special recip");
    }
}
