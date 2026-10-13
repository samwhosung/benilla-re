//! Slices of three movement handlers: `UnpackSplineOffset` (`0x602130`) from `0x602148` to
//! `0x6021b5`, reading the three stores; `0x6030c0` from `0x6030e0` to `0x6030f7`, reading
//! `[ebp-4]`, and from `0x60314a` to `0x60315d`, reading `ah` at the `test ah,0x41`; and
//! `ProcessLocalMoveEvent` (`0x60e0a0`) from `0x60e1f0` to `0x60e205`, reading `ah`.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::move_events::*;

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A random `f32` in `[lo, hi)` from the high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// Asserts the same bits, or NaN on both sides.
fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// `ah` after `fnstsw ax`: C0 is bit 0, C2 bit 2, C3 bit 6.
fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// Fixed and random packed words and bases.
#[test]
fn move_event_packed_delta() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x20);
    let base = e.alloc(0x20);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6021_3000u32;

    let case = |e: &mut Emu, packed: u32, b: [f32; 3], ctx: &str| {
        for (k, &bk) in b.iter().enumerate() {
            e.write_u32(base + 4 * k as u64, bk.to_bits());
        }
        e.write_u32(ebp + 8, packed);
        e.set_reg(RegisterX86::ESI, out);
        e.set_reg(RegisterX86::EDI, base);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x602148, 0x6021b5);
        let bin = [e.read_f32(out), e.read_f32(out + 4), e.read_f32(out + 8)];

        let ours = decode_packed_delta(packed, b);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    case(&mut e, 0x0000_0000, [0.0; 3], "all-zero fields");

    case(&mut e, 0x0000_07ff, [10.0, 20.0, 30.0], "x=-1");

    case(&mut e, 0x0000_0400, [0.0; 3], "x most-neg s11");
    case(&mut e, 0x0020_0000, [0.0; 3], "y most-neg s11");
    case(&mut e, 0x8000_0000, [0.0; 3], "z most-neg s10");

    case(&mut e, 0x7fdf_f3ff, [1.0, 2.0, 3.0], "all max-pos");

    case(
        &mut e,
        0x0100_0000,
        [5.0, 7.0, 1.0],
        "z·0.25==base.z (exact 0)",
    );

    for i in 0..6000u32 {
        let packed = next(&mut s);
        let b = if i % 4 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e3, 1e3),
                frand(&mut s, -1e3, 1e3),
                frand(&mut s, -1e3, 1e3),
            ]
        };
        case(&mut e, packed, b, &format!("fuzz{i}"));
    }
}

/// Fixed and random values.
#[test]
fn move_event_pitch_rate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x1000);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6030_e000u32;

    let case = |e: &mut Emu, rate: f32, ctx: &str| {
        e.write_u32(this + 0xa5c, rate.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6030e0, 0x6030f7);
        let bin = e.read_f32(ebp - 0x4);
        let ours = pitch_rate_value(rate);
        assert_f32(bin, ours, ctx);
    };

    case(&mut e, 0.0, "rate=0 → −1/36");

    case(
        &mut e,
        f64::from(PITCH_BIAS) as f32 / 0.75f32,
        "rate·0.75≈1/36 (==threshold)",
    );
    case(&mut e, 1.0, "rate=1");
    case(&mut e, -1.0, "rate=-1");
    case(&mut e, f32::NAN, "rate=NaN");
    case(&mut e, f32::INFINITY, "rate=inf");

    for i in 0..6000u32 {
        let rate = if i % 4 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1e3, 1e3)
        };
        case(&mut e, rate, &format!("fuzz{i}"));
    }
}

/// Arguments around the threshold, and random pairs.
#[test]
fn move_event_pitch_thresh() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x1000);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x6031_4a00u32;
    let mut seen_gt = false;
    let mut seen_le = false;

    let mut case = |e: &mut Emu, rate: f32, arg: f32, ctx: &str| {
        e.write_u32(this + 0xa5c, rate.to_bits());
        e.write_u32(ebp + 0x10, arg.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60314a, 0x60315d);
        let ah = read_ah(e);

        let bin_exceeds = (ah & 0x41) == 0;
        let ours = pitch_arg_exceeds(arg, rate);
        assert_eq!(
            bin_exceeds, ours,
            "{ctx}: AH={ah:#04x} rate={rate} arg={arg} (binary ecx!=ours)"
        );
        if ours {
            seen_gt = true;
        } else {
            seen_le = true;
        }
    };

    case(&mut e, 2.5, 1.0, "arg==thresh (==threshold, not exceeds)");
    let just_over = f32::from_bits(1.0f32.to_bits() + 1);
    case(&mut e, 2.5, just_over, "arg just over thresh (exceeds)");
    let just_under = f32::from_bits(1.0f32.to_bits() - 1);
    case(
        &mut e,
        2.5,
        just_under,
        "arg just under thresh (not exceeds)",
    );
    case(&mut e, 10.0, 0.0, "arg<thresh");
    case(&mut e, 1.0, 100.0, "arg>thresh");
    case(&mut e, 1.0, f32::NAN, "arg=NaN (not exceeds)");
    case(&mut e, f32::NAN, 1.0, "rate=NaN (not exceeds)");

    for i in 0..5000u32 {
        let rate = frand(&mut s, -50.0, 50.0);

        let t = rate * 0.4;
        let arg = match i % 3 {
            0 => t + frand(&mut s, -0.5, 0.5),
            1 => frand(&mut s, -50.0, 50.0),
            _ => t + frand(&mut s, -0.01, 0.01),
        };
        case(&mut e, rate, arg, &format!("fuzz{i}"));
    }
    assert!(
        seen_gt && seen_le,
        "both threshold arms exercised (exceeds={seen_gt} not={seen_le})"
    );
}

/// Differences of 0.1 and one ulp either side, and random pairs.
#[test]
fn move_event_move_deadband() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let new = e.alloc(0x40);
    let this = e.alloc(0x1000);
    let mut s = 0x60e1_f000u32;
    let mut seen_moved = false;
    let mut seen_still = false;

    let mut case = |e: &mut Emu, nx: f32, cx: f32, ctx: &str| {
        e.write_u32(new + 0x1c, nx.to_bits());
        e.write_u32(this + 0xc70, cx.to_bits());
        e.set_reg(RegisterX86::ECX, new);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60e1f0, 0x60e205);
        let ah = read_ah(e);

        let bin_moved = (ah & 0x41) != 0;
        let ours = move_axis_changed(nx, cx);
        assert_eq!(
            bin_moved, ours,
            "{ctx}: AH={ah:#04x} new={nx} cur={cx} (binary jne-disposition != ours)"
        );
        if ours {
            seen_moved = true;
        } else {
            seen_still = true;
        }
    };

    case(
        &mut e,
        MOVE_DEADBAND,
        0.0,
        "|delta|==0.1 (==threshold → moved)",
    );
    let one_over = f32::from_bits(MOVE_DEADBAND.to_bits() + 1);
    case(&mut e, one_over, 0.0, "|delta| just over 0.1 (moved)");
    let one_under = f32::from_bits(MOVE_DEADBAND.to_bits() - 1);
    case(&mut e, one_under, 0.0, "|delta| just under 0.1 (still)");
    case(&mut e, 0.0, 0.0, "|delta|=0 (still)");
    case(&mut e, 5.0, 0.0, "|delta|=5 (moved)");
    case(&mut e, -0.5, 0.0, "|delta|=0.5 neg (moved)");
    case(&mut e, f32::NAN, 0.0, "new=NaN (moved — unordered)");
    case(&mut e, 0.0, f32::NAN, "cur=NaN (moved — unordered)");

    for i in 0..5000u32 {
        let cur = frand(&mut s, -100.0, 100.0);

        let d = match i % 3 {
            0 => frand(&mut s, -0.2, 0.2),
            1 => frand(&mut s, -10.0, 10.0),
            _ => frand(&mut s, -0.11, 0.11),
        };
        let nv = if i % 7 == 0 {
            frand_bits(&mut s)
        } else {
            cur + d
        };
        case(&mut e, nv, cur, &format!("fuzz{i}"));
    }
    assert!(
        seen_moved && seen_still,
        "both dead-band arms exercised (moved={seen_moved} still={seen_still})"
    );
}
