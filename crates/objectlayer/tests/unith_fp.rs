//! The unit class head kernels of `src/unith_fp.rs` against the exe, each read at the code it
//! covers with the registers, frame and fields it reads set directly:
//! - `CGUnit_C::CGUnit_C` (`0x5fad50`): the low-health ratio, `[0x5fb14b, 0x5fb15c)`;
//! - `CGUnit_C::PostInit` (`0x5fb880`): the ground-snap gate, `[0x5fb8ff, 0x5fb91a)`;
//! - `CGUnit_C::OnFootstep` (`0x5fbf70`): the distance to the camera `[0x5fbfdb, 0x5fc015)`, the
//!   inner range `[0x5fc0b3, 0x5fc0c1)`, the lifted z `[0x5fc157, 0x5fc197)` and the height gate
//!   `[0x5fc157, 0x5fc172)`;
//! - `CGUnit_C::ResolveMoveAnimation` (`0x5fd100`) whole, with `0x7c4c90` a float island;
//! - `CGUnit_C::PlayAnimation` (`0x5fe2f0`): the progress `[0x5fe536, 0x5fe550)`;
//! - `CGUnit_C::OnModelEvent` (`0x5ffbd0`): the level band, `[0x600030, 0x600044)`.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either
//! side, to its random inputs; the integer tests cover every level byte or the integers' extremes.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::unith_fp::*;
use common::*;

/// The movement-block time getter `0x7c4c90` — a direct `call rel32` (`__thiscall(ecx); ret 4`)
/// returning a float on st0. We overwrite its bytes with `fld dword[slot]; ret 4` so the real
/// `call` returns our chosen float. (`.text` is `Prot::ALL`.)
const TIMER_FN: u64 = 0x7c4c90;

/// Patch `0x7c4c90` to `fld dword[slot]; ret 4` and return the slot VA the test writes the desired
/// st0 float into before each `run`/`call`.
fn install_timer_stub(e: &mut Emu) -> u64 {
    let slot = e.alloc(0x10);
    // d9 05 <slot LE32> c2 04 00  =  fld dword ptr [slot] ; ret 4
    let mut bytes = vec![0xd9u8, 0x05];
    bytes.extend_from_slice(&(slot as u32).to_le_bytes());
    bytes.extend_from_slice(&[0xc2, 0x04, 0x00]);
    e.write(TIMER_FN, &bytes);
    slot
}

// ---- shared helpers (the crate float idiom; per unitm3_fp.rs) --------------

#[test]
fn uh_low_hp_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let view = e.alloc(0x80); // EAX; health @+0x40, maxhealth @+0x58
    let mut s = 0x5fad_5000u32;
    let mut seen_low = false;
    let mut seen_full = false;

    let mut case = |e: &mut Emu, health: i32, max_health: i32, ctx: &str| {
        e.write_u32(view + 0x40, health as u32);
        e.write_u32(view + 0x58, max_health as u32);
        e.set_reg(RegisterX86::EAX, view);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fb14b, 0x5fb15c);
        let m = ah(e) & 0x5;
        // jp 0x5fb170: jp taken (EVEN parity of C0|C2) → skip low-HP; not taken (ODD) → fire. So
        // low-HP ⟺ ODD parity of (ah&5).
        let bits = (m & 0x01).count_ones() + (m & 0x04).count_ones();
        let bin_low = bits % 2 == 1;
        if bin_low {
            seen_low = true;
        } else {
            seen_full = true;
        }
        assert_eq!(
            bin_low,
            low_hp_gate(health, max_health),
            "{ctx}: bin_low={bin_low} ah&5={m:#04x} h={health} mh={max_health}"
        );
    };

    case(&mut e, 10, 100, "0.10 < 0.2 → low");
    case(&mut e, 50, 100, "0.50 > 0.2 → full");
    case(&mut e, 100, 100, "1.0 → full");
    // Deterministic boundary probe: ratio == 0.2 EXACTLY (health/maxhealth = 1/5). The binary's
    // `==0.2` boundary SKIPS (C3 → not in mask → even parity → jp taken → full); a `<=` reading
    // would call it low and diverge.
    case(&mut e, 1, 5, "1/5 == 0.2 exactly → full (skip)");
    case(&mut e, 200, 1000, "200/1000 == 0.2 → full");
    case(&mut e, 199, 1000, "0.199 < 0.2 → low");
    case(&mut e, 201, 1000, "0.201 > 0.2 → full");
    for i in 0..4000u32 {
        let max_health = match i % 3 {
            0 => 100,
            1 => 1 + ((next(&mut s) >> 8) % 100000) as i32,
            _ => 5 * (1 + ((next(&mut s) >> 8) % 20000) as i32), // exact 1/5-multiples straddle 0.2
        };
        let health = match i % 4 {
            0 => (next(&mut s) >> 8) as i32 % (max_health.max(1)), // <maxhealth
            1 => (f64::from(max_health) * 0.2) as i32 + (((next(&mut s) >> 8) % 3) as i32 - 1),
            _ => (next(&mut s) >> 8) as i32 % (2 * max_health.max(1)),
        };
        case(&mut e, health, max_health.max(1), &format!("fuzz{i}"));
    }
    // The signs, zeros and extremes of the integers, and a ratio of exactly 0.2 as f64.
    let ints = [0, 1, -1, 2, 5, 10, 100, i32::MAX, i32::MIN, 0x4000_0000];
    for &h in &ints {
        for &m in &ints {
            case(&mut e, h, m, "ints");
        }
    }
    assert!(seen_low, "low-HP arm never seen");
    assert!(seen_full, "full-HP arm never seen");
}

#[test]
fn uh_ground_snap_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x1000); // ESI; reach @+0xa5c
    let pos = e.alloc(0x20); // EAX; self_z @+8
    let mut s = 0x5fb8_8000u32;
    let mut seen_snap = false;
    let mut seen_skip = false;

    let mut case = |e: &mut Emu, ground_z: f32, self_z: f32, reach: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, ground_z.to_bits());
        e.write_u32(pos + 8, self_z.to_bits());
        e.write_u32(this + 0xa5c, reach.to_bits());
        e.set_reg(RegisterX86::EAX, pos);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fb8ff, 0x5fb91a);
        let m = ah(e) & 0x41;
        // test ah,0x41; jne 0x5fb948 (skip). snap ⟺ (ah&0x41)==0 ⟺ height>reach·0.75 ordered.
        let bin_snap = m == 0;
        if bin_snap {
            seen_snap = true;
        } else {
            seen_skip = true;
        }
        assert_eq!(
            bin_snap,
            ground_snap_gate(ground_z, self_z, reach),
            "{ctx}: bin_snap={bin_snap} ah&0x41={m:#04x} g={ground_z} s={self_z} r={reach}"
        );
    };

    case(&mut e, 10.0, 0.0, 1.0, "h10 > 0.75 → snap");
    case(&mut e, 0.75, 0.0, 1.0, "h0.75 == reach·0.75 → skip");
    case(&mut e, 0.5, 0.0, 1.0, "h0.5 < 0.75 → skip");
    case(&mut e, 0.0, 0.0, f32::NAN, "nan reach → skip");
    for i in 0..5000u32 {
        let self_z = frand(&mut s, -50.0, 50.0);
        let ground_z = match i % 3 {
            0 => self_z + frand(&mut s, -2.0, 2.0), // straddle the reach·0.75 boundary
            1 => self_z + frand(&mut s, 0.0, 30.0),
            _ => frand_bits(&mut s),
        };
        let reach = match i % 4 {
            0 => frand(&mut s, 0.0, 4.0),
            _ => frand(&mut s, 0.0, 60.0),
        };
        case(&mut e, ground_z, self_z, reach, &format!("fuzz{i}"));
    }
    // `height == reach * 0.75` (3 against 4) and a step either side, then the specials.
    for g in [3.0f32, next_up(3.0), next_down(3.0)] {
        case(&mut e, g, 0.0, 4.0, "edge");
    }
    for r in [next_up(4.0), next_down(4.0)] {
        case(&mut e, 3.0, 0.0, r, "reach step");
    }
    for &v in &specials(&[3.0, 4.0]) {
        case(&mut e, v, 0.0, 4.0, "special ground");
        case(&mut e, 3.0, v, 4.0, "special self");
        case(&mut e, 3.0, 0.0, v, "special reach");
    }
    assert!(seen_snap, "snap (height>reach·0.75) never seen");
    assert!(seen_skip, "skip never seen");
}

#[test]
fn uh_placement_dist2_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let goal = e.alloc(0x20); // EDI
    let posbase = e.alloc(0x40); // EAX (slice adds 8); camera eye xyz at +8/+0xc/+0x10
    let mut s = 0x5fbf_7000u32;
    let mut seen_in = false;
    let mut seen_out = false;

    let mut case = |e: &mut Emu, gp: [f32; 3], pp: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(goal + 4 * k as u64, gp[k].to_bits());
            e.write_u32(posbase + 8 + 4 * k as u64, pp[k].to_bits());
        }
        e.set_reg(RegisterX86::EDI, goal);
        e.set_reg(RegisterX86::EAX, posbase);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fbfdb, 0x5fc015);
        let bin_d2 = e.read_f32(ebp - 4);
        assert_f32(bin_d2, placement_dist2(gp, pp), &format!("{ctx} dist2"));
        let m = ah(e) & 0x41;
        // je 0x5fc305 (out) ⟺ (ah&0x41)==0 ⟺ dist²>2500 ordered. proceed ⟺ (ah&0x41)!=0.
        let bin_in = m != 0;
        if bin_in {
            seen_in = true;
        } else {
            seen_out = true;
        }
        assert_eq!(
            bin_in,
            placement_in_outer_range(bin_d2),
            "{ctx}: bin_in={bin_in} ah&0x41={m:#04x} d2={bin_d2}"
        );
    };

    case(&mut e, [0.0; 3], [0.0; 3], "d0 → in");
    case(&mut e, [60.0, 0.0, 0.0], [0.0; 3], "d3600 > 2500 → out");
    // Deterministic boundary: dist² == 2500 exactly (dx=50, dy=dz=0). The `==2500` proceeds (C3 in
    // the mask → ah&0x41!=0 → je not taken → in); a strict `<` reading would call it out.
    case(
        &mut e,
        [50.0, 0.0, 0.0],
        [0.0; 3],
        "dx50 d²==2500 → in (<=)",
    );
    case(
        &mut e,
        [25.0, 0.0, 0.0],
        [0.0; 3],
        "d²==625 (inner edge), outer in",
    );
    for i in 0..3000u32 {
        let pp = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let gp = match i % 2 {
            0 => [
                pp[0] + frand(&mut s, -55.0, 55.0), // straddle the 50-yd outer radius
                pp[1] + frand(&mut s, -55.0, 55.0),
                pp[2] + frand(&mut s, -55.0, 55.0),
            ],
            _ => [
                frand(&mut s, -80.0, 80.0),
                frand(&mut s, -80.0, 80.0),
                frand(&mut s, -80.0, 80.0),
            ],
        };
        case(&mut e, gp, pp, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut g = [1.0f32, 2.0, 3.0];
            g[k] = v;
            case(&mut e, g, [0.5, 0.25, 0.125], "special goal");
            let mut q = [0.5f32, 0.25, 0.125];
            q[k] = v;
            case(&mut e, [1.0, 2.0, 3.0], q, "special pos");
        }
    }
    assert!(seen_in, "outer-range in arm never seen");
    assert!(seen_out, "outer-range out arm never seen");
}

#[test]
fn uh_placement_inner_range_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5fc0_b300u32;
    let mut seen_in = false;
    let mut seen_out = false;

    let inner = f32::from_bits(0x441c_4000); // 625.0

    let mut case = |e: &mut Emu, dist2: f32, ctx: &str| {
        e.write_u32(ebp - 4, dist2.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fc0b3, 0x5fc0c1);
        let m = ah(e) & 0x41;
        let bin_in = m != 0;
        if bin_in {
            seen_in = true;
        } else {
            seen_out = true;
        }
        assert_eq!(
            bin_in,
            placement_in_inner_range(dist2),
            "{ctx}: bin_in={bin_in} ah&0x41={m:#04x} d2={dist2}"
        );
    };

    case(&mut e, 100.0, "100 < 625 → in");
    case(&mut e, 1000.0, "1000 > 625 → out");
    case(&mut e, inner, "==625 exactly → in (<=)");
    case(&mut e, f32::NAN, "nan → in (C0|C2|C3 in mask)");
    for i in 0..3000u32 {
        let dist2 = match i % 3 {
            0 => frand(&mut s, 500.0, 750.0), // straddle 625
            1 => frand(&mut s, 0.0, 3000.0),
            _ => frand_bits(&mut s).abs(),
        };
        case(&mut e, dist2, &format!("fuzz{i}"));
    }
    // 625 and 2500 exactly and a step either side, then the specials.
    for d in [
        625.0f32,
        next_up(625.0),
        next_down(625.0),
        2500.0,
        next_up(2500.0),
        next_down(2500.0),
    ] {
        case(&mut e, d, "edge");
    }
    for &v in &specials(&[625.0, 2500.0]) {
        case(&mut e, v, "special");
    }
    assert!(seen_in, "inner-range in arm never seen");
    assert!(seen_out, "inner-range out arm never seen");
}

#[test]
fn uh_placement_ground_z_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let this = e.alloc(0x1000); // ESI; reach @+0xa5c
    let pos = e.alloc(0x20); // EAX; self_z @+8
    let goal = e.alloc(0x20); // EDI; goal.z @+8
    let mut s = 0x5fbf_7200u32;

    let case = |e: &mut Emu, goal_z: f32, ground_z: f32, self_z: f32, reach: f32, ctx: &str| {
        e.write_u32(ebp - 0x10, ground_z.to_bits()); // [ebp-0x10] = raycast ground z
        e.write_u32(pos + 8, self_z.to_bits()); // [eax+8] = self z
        e.write_u32(this + 0xa5c, reach.to_bits());
        e.write_u32(goal + 8, goal_z.to_bits()); // [edi+8] = goal z (→ [ebp-0x1c])
        e.set_reg(RegisterX86::EAX, pos);
        e.set_reg(RegisterX86::EDI, goal);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fc157, 0x5fc197);
        let bin = e.read_f32(ebp - 0x1c);
        assert_f32(bin, placement_ground_z(goal_z, ground_z, self_z), ctx);
    };

    // reach large so height < reach·0.5 (the `jp` no-spawn gate does not fire → the fadd runs).
    case(&mut e, 100.0, 0.5, 0.0, 1000.0, "simple +0.5 lift");
    case(&mut e, -10.0, -2.0, 1.0, 1000.0, "neg lift");
    case(&mut e, 0.0, 0.0, 0.0, 1000.0, "zero");
    // (f64) through `+ goal.z` — rounding it to f32 first diverges by an f32 ulp. These inputs make
    // |height| large and NON-f32-representable so an f32-spill mistranscription is caught (the
    // small-height fuzz below leaves every height f32-exact and cannot discriminate). reach huge so
    // the height gate proceeds. Witness: live 1434000.25 vs spilled 1434000.375.
    case(
        &mut e,
        163200.33,
        632_252.7,
        -638547.25,
        1.0e7,
        "live-height divergence (witness)",
    );
    case(
        &mut e,
        0.0,
        632_252.7,
        -638547.25,
        1.0e7,
        "live-height divergence, goal=0",
    );
    case(
        &mut e,
        8388609.0,
        16777216.0,
        0.5,
        1.0e8,
        "large height f32-gap regime",
    );
    case(
        &mut e,
        -7654321.0,
        16777216.0,
        0.5,
        1.0e8,
        "large height, neg goal",
    );
    for i in 0..4000u32 {
        let self_z = frand(&mut s, -100.0, 100.0);
        let ground_z = self_z + frand(&mut s, -5.0, 5.0); // keep |height| small vs the big reach
        let goal_z = frand(&mut s, -200.0, 200.0);
        let reach = frand(&mut s, 200.0, 1000.0); // large → gate proceeds
        case(&mut e, goal_z, ground_z, self_z, reach, &format!("fuzz{i}"));
    }
    // The slice includes the height gate, which leaves the function when it skips, so only
    // inputs that pass it run.
    for &v in &specials(&[]) {
        case(&mut e, v, 2.0, 1.0, 100.0, "special goal z");
        if placement_height_gate(v, 1.0, 100.0) {
            case(&mut e, 5.0, v, 1.0, 100.0, "special ground");
        }
        if placement_height_gate(2.0, v, 100.0) {
            case(&mut e, 5.0, 2.0, v, 100.0, "special self");
        }
        if placement_height_gate(2.0, 1.0, v) {
            case(&mut e, 5.0, 2.0, 1.0, v, "special reach");
        }
    }
}

#[test]
fn uh_placement_height_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let this = e.alloc(0x1000);
    let pos = e.alloc(0x20);
    let mut s = 0x5fc1_5700u32;
    let mut seen_proceed = false;
    let mut seen_skip = false;

    let mut case = |e: &mut Emu, ground_z: f32, self_z: f32, reach: f32, ctx: &str| {
        e.write_u32(ebp - 0x10, ground_z.to_bits());
        e.write_u32(pos + 8, self_z.to_bits());
        e.write_u32(this + 0xa5c, reach.to_bits());
        e.set_reg(RegisterX86::EAX, pos);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fc157, 0x5fc172);
        let m = ah(e) & 0x5;
        // jp 0x5fc303 (skip): jp taken (EVEN parity of C0|C2) → skip; not taken (ODD, C0 alone) →
        // proceed. So proceed ⟺ ODD parity ⟺ height<reach·0.5 ordered.
        let bits = (m & 0x01).count_ones() + (m & 0x04).count_ones();
        let bin_proceed = bits % 2 == 1;
        if bin_proceed {
            seen_proceed = true;
        } else {
            seen_skip = true;
        }
        assert_eq!(
            bin_proceed,
            placement_height_gate(ground_z, self_z, reach),
            "{ctx}: proceed={bin_proceed} ah&5={m:#04x} g={ground_z} s={self_z} r={reach}"
        );
    };

    case(&mut e, 0.1, 0.0, 1.0, "h0.1 < 0.5 → proceed");
    case(&mut e, 0.5, 0.0, 1.0, "h0.5 == reach·0.5 → skip");
    case(&mut e, 10.0, 0.0, 1.0, "h10 > 0.5 → skip");
    case(&mut e, 0.0, 0.0, f32::NAN, "nan reach → skip");
    for i in 0..5000u32 {
        let self_z = frand(&mut s, -50.0, 50.0);
        let ground_z = match i % 3 {
            0 => self_z + frand(&mut s, -2.0, 2.0), // straddle reach·0.5
            1 => self_z + frand(&mut s, 0.0, 30.0),
            _ => frand_bits(&mut s),
        };
        let reach = frand(&mut s, 0.0, 60.0);
        case(&mut e, ground_z, self_z, reach, &format!("fuzz{i}"));
    }
    // `height == reach * 0.5` (1 against 2) and a step either side, then the specials.
    for g in [1.0f32, next_up(1.0), next_down(1.0)] {
        case(&mut e, g, 0.0, 2.0, "edge");
    }
    for r in [next_up(2.0), next_down(2.0)] {
        case(&mut e, 1.0, 0.0, r, "reach step");
    }
    for &v in &specials(&[1.0, 2.0]) {
        case(&mut e, v, 0.0, 2.0, "special ground");
        case(&mut e, 1.0, v, 2.0, "special self");
        case(&mut e, 1.0, 0.0, v, "special reach");
    }
    assert!(seen_proceed, "height-gate proceed arm never seen");
    assert!(seen_skip, "height-gate skip arm never seen");
}

#[test]
fn uh_fall_timer_anim_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let slot = install_timer_stub(&mut e);
    let this = e.alloc(0x2000); // ECX
    let view = e.alloc(0x400); // [esi+0x110]
    let mov = e.alloc(0x400); // [esi+0x118]
    let out = e.alloc(0x10); // [esp+8]
    let mut s = 0x5fd1_0000u32;
    let mut seen_8f = false;
    let mut seen_5 = false;
    let mut seen_4 = false;

    // Static state routing into the fall leaf (set once; the per-case writes only timer + swing).
    e.write_u32(this + 0x9e8, 0x1); // move flags: &0xf!=0, &2==0, &0x200000==0
    e.write_u32(this + 0xd58, 0x10); // &0x70 != 0
    e.write_u32(this + 0x110, view as u32);
    e.write_u32(this + 0x118, mov as u32);
    e.write_u32(mov + 0xa4, 0); // aura ptr null → 0x5fd1bc
    e.write(view + 0x213, &[0]); // [+0x213]&2 == 0

    let mut case = |e: &mut Emu, timer: f32, swing: f32, ctx: &str| {
        e.write_u32(slot, timer.to_bits()); // the 0x7c4c90 stub return
        e.write_u32(this + 0xa30, swing.to_bits()); // [esi+0xa30] swing time
        e.write_u32(out, 0xdead_beef); // poison: must be overwritten
                                       // arg flags = 4 (bit2 set → 0x5fd12d passes), *out = out.
        e.call(0x5fd100, this as u32, 0, &[4, out as u32]);
        let bin = e.read_u32(out);
        let ours = fall_timer_anim(timer, swing);
        assert_eq!(
            bin, ours,
            "{ctx}: bin={bin:#x} ours={ours:#x} t={timer} sw={swing}"
        );
        match bin {
            0x8f => seen_8f = true,
            0x5 => seen_5 = true,
            0x4 => seen_4 = true,
            _ => panic!("{ctx}: unexpected *out {bin:#x}"),
        }
    };

    case(&mut e, 20.0, 1.0, "t20 >= 11 → 0x8f");
    case(&mut e, 5.0, 1.0, "t5 < 11, 5>2·1 → 0x5");
    case(&mut e, 5.0, 10.0, "t5 < 11, 5<2·10 → 0x4");
    // Deterministic boundary probes.
    case(&mut e, 11.0, 1.0, "t==11 exactly → 0x8f (C0 clear)");
    case(&mut e, 4.0, 2.0, "t==2·swing exactly → 0x4 (== arm)");
    case(&mut e, f32::NAN, 1.0, "nan t → second gate → 0x4 (jne)");
    for i in 0..3000u32 {
        let swing = frand(&mut s, 0.0, 8.0);
        let timer = match i % 3 {
            0 => frand(&mut s, 9.0, 13.0), // straddle 11.0
            1 => f64::from(2.0 * swing) as f32 + frand(&mut s, -1.0, 1.0), // straddle 2·swing
            _ => frand(&mut s, 0.0, 20.0),
        };
        case(&mut e, timer, swing, &format!("fuzz{i}"));
    }
    // The speed at 11 and a step either side, at twice the swing time and a step either side, and
    // the specials.
    for t in [11.0f32, next_up(11.0), next_down(11.0)] {
        case(&mut e, t, 20.0, "11 edge");
    }
    for t in [4.0f32, next_up(4.0), next_down(4.0)] {
        case(&mut e, t, 2.0, "twice edge");
    }
    for &v in &specials(&[11.0, 4.0]) {
        case(&mut e, v, 2.0, "special timer");
        case(&mut e, 4.0, v, "special swing");
    }
    assert!(seen_8f, "0x8f (timer>=11) arm never seen");
    assert!(seen_5, "0x5 (timer>2·swing) arm never seen");
    assert!(seen_4, "0x4 (timer<=2·swing) arm never seen");
}

#[test]
fn uh_anim_progress_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let slot = install_timer_stub(&mut e);
    let frame = e.alloc(0x200); // ebp mid-frame so [ebp-0x15c]/[ebp-0xc] are in range
    let ebp = frame + 0x180;
    let ebx = e.alloc(0x1000); // ECX = [ebx+0x9a8] (lea; unused by the stub)
    let mut s = 0x5fe2_f000u32;

    let case = |e: &mut Emu, timer: f32, divisor: f32, ctx: &str| {
        e.write_u32(slot, timer.to_bits());
        e.write_u32(ebp - 0x15c, divisor.to_bits());
        e.set_reg(RegisterX86::EBX, ebx);
        e.set_reg(RegisterX86::EDI, 0); // cmp [ebp-0x160],edi — value irrelevant to the FP
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x5fe536, 0x5fe550);
        let bin = e.read_f32(ebp - 0xc);
        assert_f32(bin, anim_progress(timer, divisor), ctx);
    };

    case(&mut e, 1.0, 2.0, "1/2 = 0.5");
    case(&mut e, 3.0, 4.0, "3/4 = 0.75");
    case(&mut e, 0.0, 1.0, "0/1 = 0");
    case(&mut e, 10.0, 3.0, "10/3 round");
    for i in 0..3000u32 {
        let (timer, divisor) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s))
        } else {
            // keep divisor away from 0 (a 0 divisor → ±inf, faithfully matched, but exercise finite)
            let d = frand(&mut s, -100.0, 100.0);
            let d = if d.abs() < 1e-3 { 1.0 } else { d };
            (frand(&mut s, -1000.0, 1000.0), d)
        };
        case(&mut e, timer, divisor, &format!("fuzz{i}"));
    }
    // Deterministic ±inf probe (divisor exactly 0): the binary's fdiv yields ±inf; our f64 / 0.0
    // matches bit-for-bit through the f32 spill.
    case(&mut e, 1.0, 0.0, "1/0 → +inf");
    case(&mut e, -1.0, 0.0, "-1/0 → -inf");
    for &v in &specials(&[]) {
        case(&mut e, v, 2.0, "special timer");
        case(&mut e, 3.0, v, "special divisor");
    }
}

#[test]
fn uh_anim_event_level_band_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x5ffb_d000u32;
    let mut seen_high = false;
    let mut seen_low = false;

    let mut case = |e: &mut Emu, level: u8, ctx: &str| {
        let clamped = u32::from(level.min(0x64)); // the 0x60001a-0x60002d byte clamp
        e.write_u32(ebp - 4, clamped);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x600030, 0x600044);
        let m = ah(e) & 0x1;
        // jne 0x60004d (low/bit-test band) ⟺ C0 set ⟺ frac<0.5. high band ⟺ (ah&1)==0 ⟺ frac>=0.5.
        let bin_high = m == 0;
        if bin_high {
            seen_high = true;
        } else {
            seen_low = true;
        }
        assert_eq!(
            bin_high,
            anim_event_level_band(level),
            "{ctx}: bin_high={bin_high} ah&1={m:#04x} level={level}"
        );
    };

    case(&mut e, 0, "level 0 → low");
    case(&mut e, 49, "level 49 (0.49<0.5) → low");
    case(&mut e, 50, "level 50 (0.5==0.5) → high (>= arm)");
    case(&mut e, 51, "level 51 → high");
    case(&mut e, 100, "level 100 (clamp) → high");
    case(&mut e, 200, "level 200 → 100 → high");
    for level in 0u8..=255 {
        case(&mut e, level, &format!("level{level}"));
    }
    for i in 0..1000u32 {
        let level = ((next(&mut s) >> 8) & 0xff) as u8;
        case(&mut e, level, &format!("fuzz{i}"));
    }
    // Every level byte, across the clamp at 100 and the band edge at 51.
    for level in 0..=255u8 {
        case(&mut e, level, "every level");
    }
    assert!(seen_high, "level>=50 high-band arm never seen");
    assert!(seen_low, "level<50 low-band arm never seen");
}
