//! The float kernels of `src/objtail_fp.rs` against the exe. Each runs a slice of its function,
//! with the registers and frame it reads set directly:
//! - `LogXpGainLine` (`0x637cd0`): the scaled XP, `[0x637cd8, 0x637ce9)` with the real `__ftol`,
//!   read from `eax`;
//! - the chair seats `0x47cf60`: the first seat and the step, `[0x47cf66, 0x47cfa2)`, and the whole
//!   body `[0x47cf66, 0x47cfe3)` with the transform `0x7bca80` stubbed, read from the offset slot.
//!
//! The XP test adds NaN, the infinities, both zeros and quotients at the edges of `i64` to its
//! random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::objtail_fp::*;
use common::*;

/// A fuzzed signed i32 from the LCG HIGH bits across a centered range.
fn irand(s: &mut u32, range: i64) -> i32 {
    let r = i64::from(next(s) >> 8) % (2 * range + 1) - range;
    r as i32
}

#[test]
fn ot_combatlog_xp_scaled_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x637c_d000u32;

    let case = |e: &mut Emu, amount: i32, rate: f32, ctx: &str| {
        e.set_reg(RegisterX86::ESI, u64::from(amount as u32));
        e.set_reg(RegisterX86::ECX, 0); // → ebx; unused before the 0x637ce9 halt
        e.write_u32(ebp + 8, rate.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x637cd8, 0x637ce9);
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        assert_eq!(
            bin,
            combatlog_xp_scaled(amount, rate),
            "{ctx}: binary={bin} ours={} amount={amount} rate={rate}",
            combatlog_xp_scaled(amount, rate)
        );
    };

    // Exact half quotients tell truncation from flooring and pin the sign.
    case(&mut e, 100, 2.0, "100/2 = 50");
    case(&mut e, 7, 2.0, "7/2 = 3.5 → trunc 3");
    case(
        &mut e,
        -7,
        2.0,
        "-7/2 = -3.5 → trunc -3 (toward zero, not floor)",
    );
    case(&mut e, -9, 2.0, "-9/2 = -4.5 → trunc -4");
    case(&mut e, 9, 2.0, "9/2 = 4.5 → trunc 4");
    case(&mut e, 1, 3.0, "1/3 = 0.333 → 0");
    case(&mut e, -1, 3.0, "-1/3 → 0");
    case(&mut e, 5, 1.0, "5/1 = 5");
    case(&mut e, 0, 1.0, "0/1 = 0");
    case(
        &mut e,
        1000,
        0.0,
        "1000/0 → +inf → __ftol indefinite low dword 0",
    );
    case(&mut e, -1000, 0.0, "-1000/0 → -inf → 0");
    case(&mut e, 0, 0.0, "0/0 → nan → 0");
    case(&mut e, i32::MAX, 1.0, "MAX/1 (in i64 range)");
    case(&mut e, i32::MIN, 1.0, "MIN/1");
    case(&mut e, 1, 1e-30, "1/tiny → huge → out of i64 range → 0");

    // A quotient of exactly -2^63 (the lowest `i64`) and one just inside the top, then the
    // quotients past `i64`, and the rate's specials against amounts either side of zero.
    case(
        &mut e,
        i32::MIN,
        f32::from_bits(0x2f80_0000),
        "-2^31 / 2^-32 = -2^63",
    );
    case(
        &mut e,
        i32::MAX,
        f32::from_bits(0x2f80_0000),
        "(2^31-1) / 2^-32",
    );
    case(
        &mut e,
        i32::MAX,
        f32::from_bits(0x2f7f_ffff),
        "just past 2^63 / 2^-32 below",
    );
    case(
        &mut e,
        i32::MIN,
        -f32::from_bits(0x2f80_0000),
        "-2^31 / -2^-32 = 2^63",
    );
    case(&mut e, i32::MIN, f32::from_bits(0x2f7f_ffff), "below -2^63");
    for &rate in &specials(&[1.0, 2.0]) {
        for amount in [0, 1, -1, 7, -7, 1000, i32::MAX, i32::MIN] {
            case(&mut e, amount, rate, &format!("rate {rate:e}"));
        }
    }

    for i in 0..6000u32 {
        let amount = if i % 4 == 0 {
            next(&mut s) as i32 // full i32 range
        } else {
            irand(&mut s, 5_000_000)
        };
        let rate = if i % 5 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -10.0, 10.0)
        };
        case(&mut e, amount, rate, &format!("fuzz{i}"));
    }
}

#[test]
fn ot_transport_segment_offset0_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x60);
    let ebp = frame + 0x40;
    let mut s = 0x47cf_6000u32;

    let case = |e: &mut Emu, n: u32, ctx: &str| {
        e.set_reg(RegisterX86::EDX, u64::from(n));
        e.set_reg(RegisterX86::ECX, 0);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x47cf66, 0x47cfa2);
        let bin_off = e.read_f32(ebp - 0x10);
        let bin_step = e.read_f32(ebp - 4);
        let step_base = e.read_f32(0x803fac);
        assert_f32(
            bin_off,
            transport_segment_offset0(n),
            &format!("{ctx} offset0"),
        );
        assert_f32(
            bin_step,
            transport_segment_step(step_base),
            &format!("{ctx} step"),
        );
    };

    // The count at the edge where offset0 changes sign (0 gives +0.5, 1 gives -0.0, 2 gives -0.5),
    // counts whose `(n - 1) * 0.5` is not exact in f32, and the high bit, which is magnitude.
    case(&mut e, 0, "N=0 (offset0 = +0.5)");
    case(&mut e, 1, "N=1 (offset0 = -0.0)");
    case(&mut e, 2, "N=2");
    case(&mut e, 3, "N=3");
    case(&mut e, 10, "N=10");
    case(&mut e, 100, "N=100");
    case(&mut e, 0x100_0001, "N large, (N-1)*0.5 not f32-exact");
    case(
        &mut e,
        0x8000_0000,
        "N high-bit set → +2^31 (zero-extended, NOT signed)",
    );
    case(&mut e, u32::MAX, "N=u32::MAX");

    for i in 0..5000u32 {
        let n = if i % 3 == 0 {
            next(&mut s) >> 8 // 0..2^24 plausible segment counts
        } else {
            next(&mut s) // full u32 range (incl. high bit — the binary zero-extends)
        };
        case(&mut e, n, &format!("fuzz{i}"));
    }
}

#[test]
fn ot_transport_segment_local_y_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // The transform `0x7bca80` is stubbed to return a readable vec3: the loop copies it to `dst`
    // and never uses it in the offset arithmetic. The call pops the one argument `push ebx` left.
    let scratch = e.alloc(0x20);
    e.stub(0x7bca80, scratch as u32, 4);
    let dst = e.alloc(0x4000); // N vec3 outputs (N capped below to stay in budget)
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let mut s = 0x47cf_6001u32;

    let case = |e: &mut Emu, n: i32, ctx: &str| {
        e.set_reg(RegisterX86::ECX, 0x1000); // this (matrix) — stubbed, unused
        e.set_reg(RegisterX86::EDX, u64::from(n as u32));
        e.write_u32(ebp + 8, dst as u32);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x47cf66, 0x47cfe3);
        let bin = e.read_f32(ebp - 0x10);
        // model: offset0/step are the f32-rounded slice values; advance N times.
        let step_base = e.read_f32(0x803fac);
        let offset0 = transport_segment_offset0(n.max(0) as u32);
        let step = transport_segment_step(step_base);
        let ours = transport_segment_local_y(offset0, step, n.max(0) as u32);
        assert_f32(bin, ours, ctx);
    };

    // Small counts, where each advance shows, and counts where the f32 rounding at every spill
    // differs from one wide add.
    case(&mut e, 1, "N=1 (one advance)");
    case(&mut e, 2, "N=2");
    case(&mut e, 3, "N=3");
    case(&mut e, 5, "N=5");
    case(&mut e, 7, "N=7 odd centering");
    case(&mut e, 33, "N=33 (sum not f32-exact each step)");
    case(&mut e, 64, "N=64");
    case(&mut e, 300, "N=300");

    for i in 0..1500u32 {
        let n = 1 + ((next(&mut s) >> 8) % 96) as i32; // 1..=96 segments (instruction budget)
        case(&mut e, n, &format!("fuzz{i}"));
    }
}
