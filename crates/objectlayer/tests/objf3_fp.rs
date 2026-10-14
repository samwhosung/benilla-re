//! The combat log and object base kernels of `src/objf3_fp.rs` against the exe. Each runs the slice
//! of the function it covers, with the frame, registers and fields it reads set directly, reading
//! the store it ends in or the status byte `fnstsw ax` leaves:
//! - `LogPowerDrain` (`0x627930`): the displayed amount, `[0x627a8f, 0x627ace)`;
//! - `LogAuraChange` (`0x62b480`): the distance and range gate, `[0x62b5ad, 0x62b5d9)`;
//! - `0x614cd0`: the gate `[0x614ce5, 0x614cf0)`, the deltas `[0x614cf5, 0x614d19)`, the lerp
//!   `[0x614cf5, 0x614d71)` with the ease factor captured from the exe's own `pow` `0x73f940`,
//!   and the amplitude product.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either
//! side, to its random inputs; the displayed amount adds the integers' extremes and products past
//! `i32`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::objf3_fp::*;
use common::*;

// ---- shared helpers (the crate float idiom, per dynobj_fp.rs) -------------

#[test]
fn f3_powerleech_display_amount_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x6279_3000u32;

    let case = |e: &mut Emu, count: i32, dmg: f32, ctx: &str| {
        e.write_u32(ebp + 0x10, count as u32);
        e.write_u32(ebp + 0x14, dmg.to_bits());
        e.write_u32(ebp + 0xc, 0); // ecx load only
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x627a8f, 0x627ace);
        let bin = e.read_u32(ebp + 0x14) as i32;
        let ours = powerleech_display_amount(count, dmg);
        assert_eq!(
            bin, ours,
            "{ctx}: count={count} dmg={dmg} binary={bin} ours={ours}"
        );
    };

    // deterministic probes: sign split, exact-half ties (the fistp round-to-nearest-even tie path),
    // and the 2p±0.5 boundary.
    case(&mut e, 10, 3.0, "10*3=30");
    case(&mut e, 10, -3.0, "neg product");
    case(&mut e, 0, 5.0, "0 product");
    case(&mut e, 1, 0.5, "p=0.5 → 2p-0.5=0.5 → tie");
    case(&mut e, 1, 1.5, "p=1.5 → 2p-0.5=2.5 → tie-even");
    case(&mut e, -1, 0.5, "p=-0.5 → 2p+0.5=-0.5 → tie");
    case(&mut e, 3, 0.25, "p=0.75");
    case(&mut e, 100_000, 10000.0, "large in-range");
    case(&mut e, -100_000, 10000.0, "large neg in-range");
    // Domain: the `fistp` of an out-of-i32-range `2·p` yields the x87 integer-indefinite
    // (0x80000000) — not modelled (the game's leech count is a small tick count, dmg a per-tick
    // amount, so `2·p` is always representable; same in-range discipline as `bag_fill_percent`/
    // `spin_cast_byte`). Keep `|2·p| < 2^30` so the round + `sar` stay in-range.
    for i in 0..6000u32 {
        // count up to ±5000 (tick counts), dmg up to ±1e5 → |2·p| < 1e9 < 2^30 bound is exceeded;
        //   bound the product instead: pick dmg so |count·dmg| < 5e8.
        let count = (next(&mut s) >> 8) as i32 % 10001 - 5000; // -5000..=5000
        let dmg = if i % 4 == 0 {
            // small magnitudes incl. fractional ties
            frand(&mut s, -4.0, 4.0)
        } else {
            frand(&mut s, -1e5, 1e5)
        };
        // guard the product into i32-safe range (the binary's domain); skip the rare overflow draw.
        if (f64::from(count) * f64::from(dmg)).abs() >= 5.0e8 {
            continue;
        }
        case(&mut e, count, dmg, &format!("fuzz{i}"));
    }
    // Counts and damages at the integers' extremes, products past `i32` (the `fistp` stores
    // 0x80000000), a negative zero product, ties at .5, and the specials.
    for &c in &[0, 1, -1, 2, i32::MAX, i32::MIN, 1000, -1000] {
        for &d in &[
            0.0f32, -0.0, 0.25, -0.25, 0.5, 1.5, -1.5, 1.0e9, -1.0e9, 3.0e9, 1.0e30,
        ] {
            case(&mut e, c, d, "extremes");
        }
    }
    for &v in &specials(&[0.5]) {
        case(&mut e, 3, v, "special damage");
    }
}

#[test]
fn f3_combat_log_in_range_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pos_a = e.alloc(0x20);
    let pos_b = e.alloc(0x20);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x62b4_8000u32;
    let mut seen_in = false;
    let mut seen_out = false;

    let mut case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], range: f32, ctx: &str| {
        for k in 0..3 {
            e.write_u32(pos_a + 4 * k as u64, a[k].to_bits());
            e.write_u32(pos_b + 4 * k as u64, b[k].to_bits());
        }
        e.write_u32(ebp - 0x14, range.to_bits());
        e.set_reg(RegisterX86::EAX, pos_a);
        e.set_reg(RegisterX86::ESI, pos_b);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x62b5ad, 0x62b5d9);
        let m = ah(e) & 0x41;
        let bin_in = m == 0; // jne 0x62b7c3 not taken (continue) ⟺ ah&0x41==0 ⟺ dist²<range²
        if bin_in {
            seen_in = true;
        } else {
            seen_out = true;
        }
        let ours = combat_log_in_range(a, b, range);
        assert_eq!(
            bin_in,
            ours,
            "{ctx}: ah&0x41={m:#04x} bin_in={bin_in} ours={ours} dist²={} range²={}",
            combat_log_dist2(a, b),
            f64::from(range) * f64::from(range)
        );
    };

    // deterministic probes: clearly in, clearly out, and the EXACT dist²==range² C3-equal boundary
    // (a 3-4-5 triangle → dist=5, choose range=5 exactly so range²==dist² → the binary's `==` arm =
    // OUT, which the strict-`<` reading must reproduce).
    case(&mut e, [0.0; 3], [1.0, 0.0, 0.0], 5.0, "in");
    case(&mut e, [0.0; 3], [100.0, 0.0, 0.0], 5.0, "out");
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [3.0, 0.0, 4.0],
        5.0,
        "dist=5 range=5 → ==boundary OUT",
    );
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [3.0, 0.0, 4.0],
        5.000_01,
        "dist=5 range>5 → IN",
    );
    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [3.0, 0.0, 4.0],
        4.999_99,
        "dist=5 range<5 → OUT",
    );
    case(&mut e, [0.0; 3], [0.0; 3], 0.0, "0 vs 0 → ==boundary OUT");
    case(&mut e, [f32::NAN, 0.0, 0.0], [0.0; 3], 5.0, "nan pos → OUT");
    case(
        &mut e,
        [0.0; 3],
        [1.0, 0.0, 0.0],
        f32::NAN,
        "nan range → OUT",
    );
    for i in 0..6000u32 {
        let scale = match i % 3 {
            0 => 1.0,
            1 => 50.0,
            _ => 1000.0,
        };
        let mk = |s: &mut u32| -> [f32; 3] {
            if next(s) >> 31 == 1 {
                [frand_bits(s), frand_bits(s), frand_bits(s)]
            } else {
                [
                    frand(s, -scale, scale),
                    frand(s, -scale, scale),
                    frand(s, -scale, scale),
                ]
            }
        };
        let a = mk(&mut s);
        let b = mk(&mut s);
        let range = if i % 7 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, scale * 2.0)
        };
        case(&mut e, a, b, range, &format!("fuzz{i}"));
    }
    // `dist^2 == range^2` (3-4-5 against 5) and a step either side, then the specials.
    for x in [3.0f32, next_up(3.0), next_down(3.0)] {
        case(&mut e, [x, 4.0, 0.0], [0.0; 3], 5.0, "equal");
    }
    for r in [next_up(5.0), next_down(5.0)] {
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], r, "range step");
    }
    for &v in &specials(&[5.0]) {
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], v, "special range");
        for k in 0..3 {
            let mut a = [3.0f32, 4.0, 0.0];
            a[k] = v;
            case(&mut e, a, [0.0; 3], 5.0, "special a");
            let mut b = [0.0f32; 3];
            b[k] = v;
            case(&mut e, [3.0, 4.0, 0.0], b, 5.0, "special b");
        }
    }
    assert!(seen_in, "in-range (dist²<range²) never seen");
    assert!(seen_out, "out-of-range (dist²>=range² / NaN) never seen");
}

#[test]
fn f3_render_smooth_active_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x614c_d000u32;
    let mut seen_on = false;
    let mut seen_off = false;

    // the exact threshold (read from the live image) for the == probe.
    let thr = e.read_f32(0x80c6a8);

    let mut case = |e: &mut Emu, cur_z: f32, ctx: &str| {
        e.write_u32(ebp - 4, cur_z.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x614ce5, 0x614cf0);
        let m = ah(e) & 1;
        let bin_active = m == 0; // jne 0x614d71 not taken (run smoothing) ⟺ C0 clear ⟺ cur_z>=thr
        if bin_active {
            seen_on = true;
        } else {
            seen_off = true;
        }
        let ours = render_smooth_active(cur_z);
        assert_eq!(
            bin_active, ours,
            "{ctx}: cur_z={cur_z} thr={thr} ah&1={m} bin_active={bin_active} ours={ours}"
        );
    };

    // deterministic probes: the EXACT threshold (cur_z==thr → C0 clear → ACTIVE), just below, just
    // above, and NaN.
    case(&mut e, thr, "cur_z==thr → ACTIVE (>=)");
    case(
        &mut e,
        f32::from_bits(thr.to_bits() - 1),
        "cur_z just below thr → OFF",
    );
    case(
        &mut e,
        f32::from_bits(thr.to_bits() + 1),
        "cur_z just above thr → ACTIVE",
    );
    case(&mut e, 0.0, "0 < thr → OFF");
    case(&mut e, 1.0, "1 > thr → ACTIVE");
    case(&mut e, f32::NAN, "nan → OFF");
    for i in 0..5000u32 {
        let cur_z = if i % 5 == 0 {
            frand_bits(&mut s)
        } else if i % 3 == 0 {
            // cluster near the threshold to exercise the boundary densely
            frand(&mut s, thr - 0.05, thr + 0.05)
        } else {
            frand(&mut s, -10.0, 10.0)
        };
        case(&mut e, cur_z, &format!("fuzz{i}"));
    }
    for z in [
        SMOOTH_Z_THRESHOLD,
        next_up(SMOOTH_Z_THRESHOLD),
        next_down(SMOOTH_Z_THRESHOLD),
    ] {
        case(&mut e, z, "threshold");
    }
    for &v in &specials(&[SMOOTH_Z_THRESHOLD]) {
        case(&mut e, v, "special");
    }
    assert!(seen_on, "active (cur_z>=thr) never seen");
    assert!(seen_off, "inactive (cur_z<thr / NaN) never seen");
}

#[test]
fn f3_render_smooth_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x614c_d100u32;

    let case = |e: &mut Emu, target: [f32; 3], cur: [f32; 3], ctx: &str| {
        e.write_u32(this + 0x104, target[0].to_bits());
        e.write_u32(this + 0x108, target[1].to_bits());
        e.write_u32(this + 0x10c, target[2].to_bits());
        e.write_u32(ebp - 0xc, cur[0].to_bits());
        e.write_u32(ebp - 8, cur[1].to_bits());
        e.write_u32(ebp - 4, cur[2].to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x614cf5, 0x614d19);
        let bin = [
            e.read_f32(ebp - 0x18),
            e.read_f32(ebp - 0x14),
            e.read_f32(ebp - 0x10),
        ];
        let ours = render_smooth_delta(target, cur);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} Δ[{k}]"));
        }
    };

    case(&mut e, [10.0, 20.0, 30.0], [1.0, 2.0, 3.0], "simple");
    case(&mut e, [0.0; 3], [0.0; 3], "zeros");
    case(&mut e, [1e7, 1.0, -1e7], [1.0, 1e7, 1.0], "cancel");
    for i in 0..4000u32 {
        let (target, cur) = if i % 4 == 0 {
            (
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
                [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
            )
        } else {
            (
                [
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                ],
                [
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                    frand(&mut s, -1e4, 1e4),
                ],
            )
        };
        case(&mut e, target, cur, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut a = [1.0f32, 2.0, 3.0];
            a[k] = v;
            case(&mut e, a, [0.5, 0.25, 0.125], "special target");
            case(&mut e, [0.5, 0.25, 0.125], a, "special cur");
        }
    }
}

#[test]
fn f3_render_smooth_lerp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;

    //   fstp qword [t_slot]. `call rel32` = e8 (target - (blob+5_to_call_end)). Build once.
    let arg_slot = e.alloc(8);
    let t_slot = e.alloc(8);
    let blob = e.alloc(0x20);
    {
        let mut code: Vec<u8> = Vec::new();
        // fld qword ptr [0x80c6a0]  = DD 05 <disp32>
        code.extend_from_slice(&[0xdd, 0x05]);
        code.extend_from_slice(&0x0080_c6a0u32.to_le_bytes());
        // fld dword ptr [arg_slot]  = D9 05 <disp32>
        code.extend_from_slice(&[0xd9, 0x05]);
        code.extend_from_slice(&(arg_slot as u32).to_le_bytes());
        // call 0x73f940  = E8 <rel32>; rel32 = 0x73f940 - (addr_of_next_insn)
        let call_site = blob + code.len() as u64; // address of the E8 byte
        let next_ip = call_site + 5;
        let rel = (0x0073_f940i64 - next_ip as i64) as i32;
        code.push(0xe8);
        code.extend_from_slice(&rel.to_le_bytes());
        // fstp qword ptr [t_slot]  = DD 1D <disp32>
        code.extend_from_slice(&[0xdd, 0x1d]);
        code.extend_from_slice(&(t_slot as u32).to_le_bytes());
        e.write(blob, &code);
    }
    // blob length: fld qword (DD 05 + disp32 = 6) + fld dword (D9 05 + disp32 = 6) + call (E8 + rel32
    //   = 5) + fstp qword (DD 1D + disp32 = 6) = 23 bytes.
    let blob_end = blob + 23;

    let mut s = 0x614c_d200u32;

    let case = |e: &mut Emu, target: [f32; 3], cur: [f32; 3], arg: f32, ctx: &str| {
        //   32-bit load (matching the region's `fld dword [ebp+8]`), so write `arg` as an f32 dword.
        e.write_u32(arg_slot, arg.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(blob, blob_end);
        let t = f64::from_le_bytes(e.read(t_slot, 8).try_into().unwrap());

        // --- run the full smoothing region (real pow), read the binary stores ---
        e.write_u32(this + 0x104, target[0].to_bits());
        e.write_u32(this + 0x108, target[1].to_bits());
        e.write_u32(this + 0x10c, target[2].to_bits());
        e.write_u32(ebp - 0xc, cur[0].to_bits());
        e.write_u32(ebp - 8, cur[1].to_bits());
        e.write_u32(ebp - 4, cur[2].to_bits());
        e.write_u32(ebp + 8, arg.to_bits()); // [ebp+8]=arg → pow exponent
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x614cf5, 0x614d71);
        let bin = [
            e.read_f32(this + 0x104),
            e.read_f32(this + 0x108),
            e.read_f32(this + 0x10c),
        ];

        let delta = render_smooth_delta(target, cur);
        let ours = render_smooth_lerp(delta, cur, t);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} new[{k}] (t={t})"));
        }
    };

    // deterministic probes: identity (arg makes t≈1 region), the live-vs-spilled divergence regime
    // (deltas whose ·t products are commonly not f32-representable), and zero delta.
    case(&mut e, [10.0, 20.0, 30.0], [1.0, 2.0, 3.0], 0.5, "simple");
    case(
        &mut e,
        [0.2, 0.2, 0.2],
        [0.1, 0.1, 0.1],
        0.333,
        "tiny deltas (spill regime)",
    );
    case(&mut e, [5.0, 5.0, 5.0], [5.0, 5.0, 5.0], 0.7, "zero delta");
    case(
        &mut e,
        [100.0, -100.0, 0.5],
        [0.5, 100.0, -100.0],
        0.9,
        "mixed",
    );
    for i in 0..3000u32 {
        let target = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
        ];
        let cur = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
        ];
        // The argument stays in a range where `pow` is finite: the interpolation is under test, not
        // the edges of `pow`.
        let arg = frand(&mut s, 0.0, 4.0);
        case(&mut e, target, cur, arg, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut a = [1.0f32, 2.0, 3.0];
            a[k] = v;
            case(&mut e, a, [0.5, 0.25, 0.125], 0.5, "special target");
            case(&mut e, [0.5, 0.25, 0.125], a, 0.5, "special cur");
        }
    }
}

#[test]
fn f3_render_amp_product_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x614c_d300u32;

    // arm A: amp from [esi+0x9c]; arm B: amp from [esi+0x98]. b=[esi+0x94], c=[esi+0x90].
    let case = |e: &mut Emu, amp: f32, b: f32, c: f32, arm_a: bool, ctx: &str| {
        let amp_off = if arm_a { 0x9c } else { 0x98 };
        e.write_u32(this + amp_off, amp.to_bits());
        e.write_u32(this + 0x94, b.to_bits());
        e.write_u32(this + 0x90, c.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        let start = if arm_a { 0x614d8b } else { 0x614d93 };
        e.run(start, 0x614dc0);
        let bin = e.read_f32(ebp - 0x24);
        let ours = render_amp_product(amp, b, c);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 2.0, 3.0, 4.0, true, "arm A 2*(3*4)");
    case(&mut e, 2.0, 3.0, 4.0, false, "arm B 2*(3*4)");
    case(&mut e, 0.0, 5.0, 6.0, true, "arm A zero amp");
    case(&mut e, 1.0, 0.1, 0.3, false, "arm B fractional");
    for i in 0..4000u32 {
        let (amp, b, c) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
            )
        };
        let arm_a = !(next(&mut s) >> 8).is_multiple_of(2);
        case(&mut e, amp, b, c, arm_a, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for arm in [true, false] {
            case(&mut e, v, 2.0, 3.0, arm, "special amp");
            case(&mut e, 2.0, v, 3.0, arm, "special b");
            case(&mut e, 2.0, 3.0, v, arm, "special c");
        }
    }
}
