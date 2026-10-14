//! The unit float leaves of `src/unit_fp.rs` against the exe: `GetDisplayAlpha` (`0x60d2d0`),
//! `IsWithinArrivalDistance` (`0x610d50`), `NamePlateSortKey` (`0x6089a0`), `WrapAngleToPi`
//! (`0x6084f0`), `DrunkSway` (`0x60ab20`), `GetNamePlateScreenPos` (`0x60f7f0`), `RestoreAlpha`
//! (`0x60d180`), `GetFootprintParams` (`0x607920`), `GetMerchantDiscount` (`0x612b80`) and the
//! colour read of `0x614550`. Each runs whole in the emulator, with the calls around it stubbed
//! where noted. The cosine of `DrunkSway` and the `fmod` of `WrapAngleToPi` are the host's. Each
//! test adds NaN, the infinities, both zeros and the thresholds to its random inputs.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::unit_fp::*;
use common::*;

const BASE_RENDER_ALPHA: u64 = 0x60d2d0;
const DIST2_WITHIN: u64 = 0x610d50;
const DIST2_ANCHOR: u64 = 0x6089a0;
const ANGLE_WRAP: u64 = 0x6084f0;
const COSINE_PULSE: u64 = 0x60ab20;
const FOLLOW_LIFT: u64 = 0x60f7f0;
const RECOMPUTE_RENDER_ALPHA: u64 = 0x60d180;
const WEAPON_OFFSET: u64 = 0x607920;
const VENDOR_PRICE_DISCOUNT: u64 = 0x612b80;
const DECAL_RGB: u64 = 0x614550;

// Callees stubbed or read around.
const ATEXIT: u64 = 0x409aef; // `_atexit`, called by the one-time init of `0x6089a0`
const START_ALPHA_FADE: u64 = 0x614f80; // the fade `0x60d180` calls (ret 8)
const MUZZLE_POINT: u64 = 0x608640; // fills the vector `0x60f7f0` lifts (ret 4)
const FOLLOW_CONSUME: u64 = 0x483ee0; // the consumer in `0x60f7f0`; its argument is read
const UNIT_REACTION: u64 = 0x6061e0; // `CGUnit_C::UnitReaction`, gating the 0.1 term (ret 4)
const DECAL_CFG_GLOBAL: u64 = 0xce9b60; // the fixed pointer `0x6d48b0` returns for `0x614550`

/// Plants a vtable island that returns an `f32` in `st0`, `fld dword [slot]; ret`. Returns the
/// island and the slot the test writes the value into before each call.
fn st0_island(e: &mut Emu) -> (u64, u64) {
    let island = e.alloc(0x10);
    let slot = e.alloc(0x10);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(slot as u32).to_le_bytes());
    code.push(0xc3);
    e.write(island, &code);
    (island, slot)
}

#[test]
fn base_render_alpha_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let row = e.alloc(0x20); // a display row; +0x14 is the integer alpha

    // No row (`this+0xb34 == 0`): 1.0.
    e.write_u32(this + 0xb34, 0);
    let bin = e.call_f32(BASE_RENDER_ALPHA, this as u32, 0, &[]);
    assert_f32(bin, base_render_alpha(None), "row absent → 1.0");

    // A row: the integer at +0x14 times 1/255, over edges and random values.
    e.write_u32(this + 0xb34, row as u32);
    let mut s = 0x60d2_d000u32;
    let mut probes: Vec<i32> = vec![
        0,
        1,
        -1,
        255,
        256,
        1000,
        -1000,
        i32::MAX,
        i32::MIN,
        0x007f_ffff,
    ];
    for _ in 0..4000 {
        probes.push(next(&mut s) as i32);
    }
    for raw in probes {
        e.write_u32(row + 0x14, raw as u32);
        let bin = e.call_f32(BASE_RENDER_ALPHA, this as u32, 0, &[]);
        assert_f32(bin, base_render_alpha(Some(raw)), &format!("raw={raw}"));
    }
}

#[test]
fn dist2_within_threshold_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let a = e.alloc(0x10);
    let b = e.alloc(0x10);

    let check = |e: &mut Emu, av: [f32; 3], bv: [f32; 3], ctx: &str| {
        for (i, c) in av.iter().enumerate() {
            e.write_u32(a + 4 * i as u64, c.to_bits());
        }
        for (i, c) in bv.iter().enumerate() {
            e.write_u32(b + 4 * i as u64, c.to_bits());
        }
        let bin = e.call(DIST2_WITHIN, a as u32, b as u32, &[]) & 0xff;
        let ours = u32::from(dist2_within_threshold(av, bv));
        assert_eq!(bin, ours, "{ctx}: bin={bin} ours={ours}");
    };

    // The threshold is 1/360, its square about 7.7e-6; probes straddle it.
    let t = f64::from(DIST_THRESH);
    let just_in = (t * 0.9) as f32;
    let just_out = (t * 1.1) as f32;
    check(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "identical → in");
    check(&mut e, [just_in, 0.0, 0.0], [0.0, 0.0, 0.0], "dx just in");
    check(&mut e, [just_out, 0.0, 0.0], [0.0, 0.0, 0.0], "dx just out");
    check(&mut e, [0.0, just_in, 0.0], [0.0, 0.0, 0.0], "dy just in");
    check(&mut e, [0.0, 0.0, just_out], [0.0, 0.0, 0.0], "dz just out");
    check(
        &mut e,
        [t as f32, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "dx == thresh",
    );
    // The threshold square, equal and a step either side, and NaN, infinities and zeros.
    for x in [t as f32, next_up(t as f32), next_down(t as f32)] {
        check(&mut e, [0.0, x, 0.0], [0.0, 0.0, 0.0], "dy at thresh");
        check(&mut e, [0.0, 0.0, -x], [0.0, 0.0, 0.0], "dz at thresh");
    }
    for &v in &specials(&[t as f32]) {
        check(&mut e, [v, 0.0, 0.0], [0.0; 3], "special dx");
        check(&mut e, [0.0, v, 0.0], [0.0; 3], "special dy");
        check(&mut e, [0.0, 0.0, v], [0.0; 3], "special dz");
        check(&mut e, [0.0; 3], [v, v, v], "special b");
    }

    let mut s = 0x610d_5000u32;
    for i in 0..6000u32 {
        // Scales near the threshold, so both results are dense, and far from it.
        let scale = if (next(&mut s) >> 8).is_multiple_of(2) {
            0.005f32
        } else {
            50.0f32
        };
        let av = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let bv = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        check(&mut e, av, bv, &format!("fuzz[{i}]"));
    }
}

#[test]
fn dist2_to_anchor_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    // The first call runs the one-time init, which seeds the anchor and registers an `atexit`
    // handler; the register is stubbed to do nothing.
    e.stub(ATEXIT, 0, 0);
    let p = e.alloc(0x10);

    let check = |e: &mut Emu, px: f32, py: f32, ctx: &str| {
        e.write_u32(p, px.to_bits());
        e.write_u32(p + 4, py.to_bits());
        let bin = e.call_f32(DIST2_ANCHOR, p as u32, 0, &[]);
        assert_f32(bin, dist2_to_anchor(px, py), ctx);
    };
    // The anchor is (0.4, 0.3): probes at it, on each axis and away.
    check(&mut e, 0.4, 0.3, "at anchor → 0");
    check(&mut e, 0.0, 0.0, "origin");
    check(&mut e, 1.0, 1.0, "unit");
    check(&mut e, -1.0, -1.0, "neg");
    for &v in &specials(&[0.4, 0.3]) {
        check(&mut e, v, 0.3, "special x");
        check(&mut e, 0.4, v, "special y");
        check(&mut e, v, v, "special both");
    }

    let mut s = 0x6089_a000u32;
    for i in 0..5000u32 {
        check(
            &mut e,
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            &format!("fuzz[{i}]"),
        );
    }
}

#[test]
fn angle_wrap_pi_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);

    let check = |e: &mut Emu, arg: f32, ctx: &str| {
        let bin = e.call_f32(ANGLE_WRAP, 0, 0, &[arg.to_bits()]);
        assert_f32(bin, angle_wrap_pi(arg), &format!("{ctx} arg={arg}"));
    };
    let pi = std::f32::consts::PI;
    let tau = std::f32::consts::TAU; // 2 pi, across the wrap in both directions
                                     // Each branch (above pi, within, below -pi) and the exact boundaries at +-pi.
    for &a in &[
        0.0f32,
        0.1,
        -0.1,
        pi,
        -pi,
        pi - 0.001,
        pi + 0.001,
        tau,
        -tau,
        tau - 0.0001,
        -(tau - 0.0001),
        3.0 * pi,
        -3.0 * pi,
        100.0,
        -100.0,
    ] {
        check(&mut e, a, "edge");
    }
    // Exactly +-pi and a step either side, +-2 pi, zeros and NaN. An infinity sends `fmod` to the
    // C runtime's error path, which needs state the start-up sets and the emulator does not run,
    // and past about 1e16 the emulator's partial `fprem` rounds in `f64` where the CPU is exact.
    for &a in specials(&[pi, tau, 3.0 * pi, 1.0e9])
        .iter()
        .filter(|a| a.is_nan() || a.abs() < 1.0e15)
    {
        check(&mut e, a, "special");
    }
    let mut s = 0x6084_f000u32;
    for i in 0..6000u32 {
        // Well past +-pi, so the `fmod` reduction runs in both directions.
        check(&mut e, frand(&mut s, -50.0, 50.0), &format!("fuzz[{i}]"));
    }
    // The band around pi, where the branches meet.
    for i in 0..3000u32 {
        check(&mut e, frand(&mut s, -7.0, 7.0), &format!("band[{i}]"));
    }
}

#[test]
fn cosine_pulse_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);

    let check = |e: &mut Emu, n: i32, t: f32, ctx: &str| {
        // `ecx` is `n` and the stack argument is `t`.
        let bin = e.call_f32(COSINE_PULSE, n as u32, 0, &[t.to_bits()]);
        assert_f32(bin, cosine_pulse(n, t), &format!("{ctx} n={n} t={t}"));
    };
    // Only `t == 0` returns 0.0; a negative `t` and NaN compute.
    check(&mut e, 0, 0.0, "t==0 → 0");
    check(&mut e, 5, -1.0, "t<0 → compute");
    check(&mut e, 0, 1.0, "n==0");
    check(&mut e, 1, 1.0, "n==1");
    check(&mut e, 360, 1.0, "n==360");
    // The emulator leaves an infinite operand of `fcos` unchanged where a CPU gives NaN, and a
    // phase that overflows `f32` in the store becomes one; so `t` stays under 1e30. Phases past
    // 2^63 are left unchanged in both.
    for &t in specials(&[1.0])
        .iter()
        .filter(|t| t.is_nan() || t.abs() < 1.0e30)
    {
        for n in [0, 1, 360, -1, i32::MAX, i32::MIN] {
            check(&mut e, n, t, "special");
        }
    }

    let mut s = 0x60ab_2000u32;
    for i in 0..6000u32 {
        // A small `n`, as a clock gives, and a positive `t`.
        let n = ((next(&mut s) >> 8) % 2000) as i32;
        let t = frand(&mut s, 0.0001, 5.0);
        check(&mut e, n, t, &format!("fuzz[{i}]"));
    }
    // Negative `t` too.
    for i in 0..1000u32 {
        let n = ((next(&mut s) >> 8) % 2000) as i32;
        let t = frand(&mut s, -5.0, 0.0);
        check(&mut e, n, t, &format!("neg[{i}]"));
    }
}

#[test]
fn follow_anchor_lift_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    // `0x60f7f0(ecx = objA, edx = objB)` saves objB in `esi`, calls `0x608640` with objA to fill
    // the vector, lifts its z by 2/3 and calls `0x483ee0` with objB. Both callees are stubbed, and
    // z is read back through the pointer `0x483ee0` receives.
    let obj = e.alloc(0x2000);
    let vfill = e.alloc(0x10); // the vector `0x608640` writes through its first argument
    e.stub_write_through(MUZZLE_POINT, 0, vfill, 12, 0, 1);
    // `0x483ee0(ecx = objB, &v, arg)`: the call is recorded so the pointer can be read back.
    e.stub(FOLLOW_CONSUME, 0, 0);

    let check = |e: &mut Emu, v: [f32; 3], arg: u32, ctx: &str| {
        for (i, c) in v.iter().enumerate() {
            e.write_u32(vfill + 4 * i as u64, c.to_bits());
        }
        e.clear_calls();
        let _ = e.call(FOLLOW_LIFT, obj as u32, obj as u32, &[arg]);
        let consume = e
            .calls()
            .iter()
            .find(|c| c.addr == FOLLOW_CONSUME)
            .copied()
            .expect("consumer called");
        let v_ptr = u64::from(consume.args[0]); // `&v` is the first stack argument
        let bin_z = e.read_f32(v_ptr + 8);
        assert_f32(bin_z, follow_anchor_lift(v[2]), ctx);
    };

    let mut s = 0x60f7_f000u32;
    check(&mut e, [1.0, 2.0, 3.0], 0, "z=3");
    check(&mut e, [0.0, 0.0, 0.0], 0, "z=0");
    for &z in &specials(&[]) {
        check(&mut e, [1.0, 2.0, z], 0, "special z");
    }
    for i in 0..3000u32 {
        let v = [
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
        ];
        check(&mut e, v, next(&mut s), &format!("fuzz[{i}]"));
    }
}

#[test]
fn recompute_render_alpha_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let vt = e.alloc(0x100); // this+0 is the vtable; +0x6c the base alpha island
    let inst = e.alloc(0x100); // this+0xb50 is the effect list head; +0x78 its factor
    let (island, base_slot) = st0_island(&mut e);
    e.write_u32(this, vt as u32);
    e.write_u32(vt + 0x6c, island as u32);
    // `0x614f80(this, alpha, 1000)` is stubbed and its alpha argument captured.
    e.stub(START_ALPHA_FADE, 0, 2);

    let check = |e: &mut Emu, base: f32, rate: Option<f32>, ctx: &str| {
        e.write_u32(base_slot, base.to_bits());
        match rate {
            Some(r) => {
                e.write_u32(this + 0xb50, inst as u32);
                e.write_u32(inst + 0x78, r.to_bits());
            }
            None => e.write_u32(this + 0xb50, 0),
        }
        e.clear_calls();
        let _ = e.call(RECOMPUTE_RENDER_ALPHA, this as u32, 0, &[]);
        let call = e
            .calls()
            .iter()
            .find(|c| c.addr == START_ALPHA_FADE)
            .expect("0x614f80 called");
        let bin_alpha = f32::from_bits(call.args[0]);
        assert_f32(bin_alpha, recompute_render_alpha(base, rate), ctx);
    };

    check(&mut e, 1.0, None, "no effect, passed through");
    check(&mut e, 2.5, Some(0.5), "scaled");
    check(&mut e, 0.0, Some(3.0), "base 0");
    for &v in &specials(&[]) {
        check(&mut e, v, Some(0.5), "special base");
        check(&mut e, 2.0, Some(v), "special factor");
    }
    let mut s = 0x60d1_8000u32;
    for i in 0..3000u32 {
        let base = frand(&mut s, -10.0, 10.0);
        let rate = if (next(&mut s) >> 8).is_multiple_of(2) {
            None
        } else {
            Some(frand(&mut s, -4.0, 4.0))
        };
        check(&mut e, base, rate, &format!("fuzz[{i}]"));
    }
}

#[test]
fn weapon_attach_offset_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let desc = e.alloc(0x400); // this+0x110 is the descriptor; +0x1fc the path selector
    let vt = e.alloc(0x100); // this+0 is the vtable; +0x1c the scalar island
    let (island, r_slot) = st0_island(&mut e);
    let out_id = e.alloc(0x10);
    let out_vec = e.alloc(0x10);
    e.write_u32(this, vt as u32);
    e.write_u32(vt + 0x1c, island as u32);
    e.write_u32(this + 0x110, desc as u32);

    // Two paths choose the scale pair: the display path when `desc+0x1fc > 0` and `this+0xdc` is
    // nonzero reads `+0xcc4` and `+0xcc8`; otherwise `+0xc64` and `+0xc68`.
    let check = |e: &mut Emu, r: f32, sel: i32, model_dc: u32, sa: f32, sb: f32, ctx: &str| {
        e.write_u32(r_slot, r.to_bits());
        e.write_u32(desc + 0x1fc, sel as u32);
        e.write_u32(this + 0xdc, model_dc);
        let path_a = sel > 0 && model_dc != 0;
        if path_a {
            e.write_u32(this + 0xcc4, sa.to_bits());
            e.write_u32(this + 0xcc8, sb.to_bits());
        } else {
            e.write_u32(this + 0xc64, sa.to_bits());
            e.write_u32(this + 0xc68, sb.to_bits());
        }
        e.write(out_vec, &[0u8; 8]);
        let _ = e.call(
            WEAPON_OFFSET,
            this as u32,
            0,
            &[out_id as u32, out_vec as u32],
        );
        let bin = [e.read_f32(out_vec), e.read_f32(out_vec + 4)];
        let ours = weapon_attach_offset(r, sa, sb);
        assert_f32(bin[0], ours[0], &format!("{ctx} .x"));
        assert_f32(bin[1], ours[1], &format!("{ctx} .y"));
    };

    check(&mut e, 2.0, 1, 0x1234, 0.5, 0.25, "path A");
    check(&mut e, 2.0, 0, 0x1234, 0.5, 0.25, "path B (sel<=0)");
    check(&mut e, 2.0, 1, 0, 0.5, 0.25, "path B (model_dc==0)");
    for &v in &specials(&[]) {
        check(&mut e, v, 1, 0x1234, 0.5, 0.25, "special r");
        check(&mut e, 2.0, 1, 0x1234, v, 0.25, "special a");
        check(&mut e, 2.0, 0, 0x1234, 0.5, v, "special b");
    }
    let mut s = 0x6079_2000u32;
    for i in 0..3000u32 {
        let r = frand(&mut s, -50.0, 50.0);
        let sel = if (next(&mut s) >> 8).is_multiple_of(2) {
            1
        } else {
            -1
        };
        let dc = if (next(&mut s) >> 8).is_multiple_of(2) {
            0
        } else {
            0xabcd
        };
        let sa = frand(&mut s, -4.0, 4.0);
        let sb = frand(&mut s, -4.0, 4.0);
        check(&mut e, r, sel, dc, sa, sb, &format!("fuzz[{i}]"));
    }
}

#[test]
fn vendor_price_discount_difftest() {
    // `UnitReaction` (`0x6061e0`) gates the 0.1 term and is stubbed to a fixed return. A stub is a
    // permanent hook, so each reaction value runs on its own emulator. `0x612b80(ecx = vendor,
    // player)`: the honor rank is `PLAYER_BYTES_3` byte 3 at `[player+0xe68]+0x1f` and the PvP
    // flag is bit 12 of the vendor's `UNIT_FIELD_FLAGS` at `[vendor+0x110]+0xa0`. The result is
    // read as the full `st0` (`call_f64`): both callers consume it unrounded (`0x4faf8f`,
    // `0x4dc462`), and 0.1f + 0.05f is not an `f32`.
    for &reaction in &[0i32, 1, 3, 4, 5, 6, 7] {
        let Some(wow) = Wow::load() else { return };
        let mut e = Emu::new(&wow);
        let vendor = e.alloc(0x2000);
        let player = e.alloc(0x2000);
        let unit_block = e.alloc(0x400);
        let player_block = e.alloc(0x40);
        e.write_u32(vendor + 0x110, unit_block as u32);
        e.write_u32(player + 0xe68, player_block as u32);
        e.stub(UNIT_REACTION, reaction as u32, 1);

        let run = |e: &mut Emu, flags: u32, rank: u8, ctx: &str| {
            e.write_u32(unit_block + 0xa0, flags);
            e.write(player_block + 0x1f, &[rank]);
            let bin = e.call_f64(VENDOR_PRICE_DISCOUNT, vendor as u32, 0, &[player as u32]);
            let ours = vendor_price_discount(reaction, flags & 0x1000 != 0, rank);
            assert_eq!(
                bin.to_bits(),
                ours.to_bits(),
                "{ctx}: bin={bin:e} ours={ours:e}"
            );
        };
        // Bit 12 alone, every other bit without it, and all bits: only bit 12 gates the honor
        // terms. The ranks straddle both thresholds (5 and 6, 7 and 8), rank 0, the top ranks (18,
        // 19) and the unsigned boundary (127 and 128).
        for &flags in &[0u32, 0x1000, !0x1000u32, u32::MAX] {
            for &rank in &[0u8, 1, 4, 5, 6, 7, 8, 9, 18, 19, 60, 127, 128, 255] {
                run(
                    &mut e,
                    flags,
                    rank,
                    &format!("reaction={reaction} flags={flags:#x} rank={rank}"),
                );
            }
        }
    }
    // The five values it can produce: 0, 0.05f, 0.1f, 0.1f + 0.05f and 0.1f + 2 * 0.05f, as `f64`.
    let r = |x: f64| x.to_bits();
    assert_eq!(r(vendor_price_discount(4, true, 6)), 0x3fa9_9999_a000_0000);
    assert_eq!(r(vendor_price_discount(4, true, 8)), 0x3fb9_9999_a000_0000);
    assert_eq!(r(vendor_price_discount(5, false, 8)), 0x3fb9_9999_a000_0000);
    assert_eq!(r(vendor_price_discount(5, true, 6)), 0x3fc3_3333_3800_0000);
    assert_eq!(r(vendor_price_discount(5, true, 8)), 0x3fc9_9999_a000_0000);
}

#[test]
fn decal_rgb_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let model = e.alloc(0x400); // this+0xd8 and +0xdc are the model; +0x190 receives the triple
                                // `0x6d48b0` returns the fixed global `0xce9b60`, in the exe's mapped data; its bytes at
                                // +0x178 to +0x17a are the channels. `0x710d40(model, &rgb)` copies the triple to +0x190.
    e.write_u32(this + 0xdc, model as u32);
    e.write_u32(this + 0xd8, model as u32);

    let check = |e: &mut Emu, bytes: [u8; 3], bit: u8, ctx: &str| {
        e.write(DECAL_CFG_GLOBAL + 0x178, &bytes);
        e.write(model + 0x190, &[0u8; 12]);
        let _ = e.call(DECAL_RGB, this as u32, 0, &[u32::from(bit)]);
        let bin = [
            e.read_f32(model + 0x190),
            e.read_f32(model + 0x194),
            e.read_f32(model + 0x198),
        ];
        let ours = decal_rgb(bytes);
        for i in 0..3 {
            assert_f32(bin[i], ours[i], &format!("{ctx} chan[{i}]"));
        }
    };

    check(&mut e, [0, 0, 0], 0, "all 0");
    check(&mut e, [255, 255, 255], 1, "all 255");
    check(&mut e, [255, 0, 0], 2, "R only");
    check(&mut e, [0, 255, 0], 3, "G only");
    check(&mut e, [0, 0, 255], 4, "B only");
    check(&mut e, [1, 2, 3], 5, "1,2,3");
    let mut s = 0x6145_5000u32;
    for i in 0..4000u32 {
        let bytes = [
            (next(&mut s) >> 8) as u8,
            (next(&mut s) >> 8) as u8,
            (next(&mut s) >> 8) as u8,
        ];
        let bit = ((next(&mut s) >> 8) % 32) as u8;
        check(&mut e, bytes, bit, &format!("fuzz[{i}]"));
    }
}
