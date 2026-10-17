//! The stretches of `CGUnit_C::OnMonsterMove` (`0x6018f0`) run as slices, the packet's vectors and
//! the position record written in place: the near test `0x601982` to `0x6019cf`, `fsincos`
//! `0x601ab7` to `0x601ac0`, the delta `0x601aca` to `0x601b1a` and its copy `0x601b7e` to
//! `0x601be2`, the coincident tests `0x601c1d` to `0x601c50` and `0x601d41` to `0x601d79`, the
//! reflection `0x601e83` to `0x601eca`, the path length `0x601eda` to `0x601f7e`, and the length
//! through the speed and the duration, `0x601eda` to `0x601faf`. A gate is compared by `ah` and the
//! jump it decides.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::monster_move::*;

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
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
fn ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// The near gate, with points around 9.0 and random ones.
#[test]
fn monster_move_case1_near_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let ebx = e.alloc(0x200);
    let pos = e.alloc(0x40);
    let mut s = 0x6019_8200u32;
    let mut seen_proceed = false;
    let mut seen_skip = false;

    let mut case = |e: &mut Emu, base: [f32; 3], pkt: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x40, base[0].to_bits());
        e.write_u32(ebp - 0x3c, base[1].to_bits());
        e.write_u32(ebp - 0x38, base[2].to_bits());
        e.write_u32(pos + 0x10, pkt[0].to_bits());
        e.write_u32(pos + 0x14, pkt[1].to_bits());
        e.write_u32(pos + 0x18, pkt[2].to_bits());
        e.write_u32(ebx + 0x118, pos as u32);
        e.set_reg(RegisterX86::EBX, ebx);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x601982, 0x6019cf);

        let proceed = (ah(&*e) & 0x01) != 0;
        if proceed {
            seen_proceed = true;
        } else {
            seen_skip = true;
        }
        let (_sos, ours) = case1_near_gate(base, pkt);
        assert_eq!(
            proceed,
            ours,
            "{ctx}: bin_proceed={proceed} ah&1={:#04x} base={base:?} pkt={pkt:?}",
            ah(&*e) & 1
        );
    };

    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "coincident → proceed",
    );
    case(&mut e, [10.0, 0.0, 0.0], [0.0, 0.0, 0.0], "100 >= 9 → skip");
    case(
        &mut e,
        [3.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "9 == 9 exactly → skip (C0=0)",
    );
    case(
        &mut e,
        [2.9, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "8.41 < 9 → proceed",
    );
    case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "NaN delta → C0 set → proceed",
    );
    for i in 0..5000u32 {
        let pkt = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let base = match i % 4 {
            0 => {
                let r = frand(&mut s, 2.7, 3.3);
                [pkt[0] + r, pkt[1], pkt[2]]
            }
            1 => [
                pkt[0] + frand(&mut s, -1.0, 1.0),
                pkt[1] + frand(&mut s, -1.0, 1.0),
                pkt[2] + frand(&mut s, -1.0, 1.0),
            ],
            2 => [
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
            ],
            _ => [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
        };
        case(&mut e, base, pkt, &format!("fuzz{i}"));
    }
    assert!(seen_proceed, "proceed arm never seen");
    assert!(seen_skip, "skip arm never seen");
}

/// Fixed and random facings.
#[test]
fn monster_move_direction_sincos_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let cos_slot = e.alloc(0x10);
    let sin_slot = e.alloc(0x10);
    let mut s = 0x601a_b700u32;

    let case = |e: &mut Emu, angle: f32, ctx: &str| {
        e.write_u32(ebp - 0x24, angle.to_bits());
        e.set_reg(RegisterX86::EAX, cos_slot);
        e.set_reg(RegisterX86::EDX, sin_slot);

        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x601ab7, 0x601ac0);
        let bin_cos = e.read_f32(cos_slot);
        let bin_sin = e.read_f32(sin_slot);
        let (ours_cos, ours_sin) = direction_sincos(angle);
        assert_f32(bin_cos, ours_cos, &format!("{ctx} cos"));
        assert_f32(bin_sin, ours_sin, &format!("{ctx} sin"));
    };

    case(&mut e, 0.0, "angle 0");
    case(&mut e, std::f32::consts::FRAC_PI_2, "angle π/2");
    case(&mut e, std::f32::consts::PI, "angle π");
    case(&mut e, -std::f32::consts::FRAC_PI_2, "angle −π/2");
    case(&mut e, 6.2831855, "angle 2π");
    for i in 0..3000u32 {
        let angle = match i % 3 {
            0 => frand(&mut s, -6.2831855, 6.2831855),
            1 => frand(&mut s, -100.0, 100.0),
            _ => frand(&mut s, -1.0, 1.0),
        };
        case(&mut e, angle, &format!("fuzz{i}"));
    }
}

/// Runs a delta slice with the cosine and sine at `[ebp-0x58]` and `[ebp-0x54]` and the packet's
/// point at `+0x10` of the position record, and reads the three stores.
fn run_case_delta(
    e: &mut Emu,
    begin: u64,
    end: u64,
    pkt: [f32; 3],
    cos: f32,
    sin: f32,
) -> [f32; 3] {
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let ebx = e.alloc(0x200);
    let pos = e.alloc(0x40);
    let outbuf = e.alloc(0x40);
    e.write_u32(ebp - 0x58, cos.to_bits());
    e.write_u32(ebp - 0x54, sin.to_bits());
    e.write_u32(ebp - 0x50, 0u32);
    e.write_u32(pos + 0x10, pkt[0].to_bits());
    e.write_u32(pos + 0x14, pkt[1].to_bits());
    e.write_u32(pos + 0x18, pkt[2].to_bits());
    e.write_u32(ebx + 0x118, pos as u32);
    e.set_reg(RegisterX86::EBX, ebx);
    e.set_reg(RegisterX86::EDI, outbuf);
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.run(begin, end);
    [
        e.read_f32(ebp - 0x2c),
        e.read_f32(ebp - 0x28),
        e.read_f32(ebp - 0x24),
    ]
}

/// The first delta slice over random points and directions, any bits included.
#[test]
fn monster_move_case_delta_sub_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s = 0x601a_ca00u32;

    let case = |e: &mut Emu, pkt: [f32; 3], cos: f32, sin: f32, ctx: &str| {
        let bin = run_case_delta(e, 0x601aca, 0x601b1a, pkt, cos, sin);
        let ours = case_delta_sub(pkt, cos, sin);
        assert_f32(bin[0], ours[0], &format!("{ctx} x"));
        assert_f32(bin[1], ours[1], &format!("{ctx} y"));
        assert_f32(bin[2], ours[2], &format!("{ctx} z"));
    };

    case(&mut e, [1.0, 2.0, 3.0], 0.5, -0.5, "basic");
    case(&mut e, [0.0, 0.0, 0.0], 1.0, 0.0, "−unit");
    case(&mut e, [100.0, -100.0, 50.0], 0.6, 0.8, "diag");
    for i in 0..4000u32 {
        let pkt = [
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        let (cos, sin) = match i % 3 {
            0 => (frand(&mut s, -1.0, 1.0), frand(&mut s, -1.0, 1.0)),
            1 => (frand(&mut s, -1e6, 1e6), frand(&mut s, -1e6, 1e6)),
            _ => (frand_bits(&mut s), frand_bits(&mut s)),
        };
        case(&mut e, pkt, cos, sin, &format!("fuzz{i}"));
    }
}

/// The copy at `0x601b7e` over the same kind of inputs.
#[test]
fn monster_move_case_delta_sub_clone_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s = 0x601b_7e00u32;

    let case = |e: &mut Emu, pkt: [f32; 3], cos: f32, sin: f32, ctx: &str| {
        let frame = e.alloc(0x100);
        let ebp = frame + 0x80;
        let ebx = e.alloc(0x200);
        let pos = e.alloc(0x40);
        let outbuf = e.alloc(0x40);
        e.write_u32(ebp - 0x58, cos.to_bits());
        e.write_u32(ebp - 0x54, sin.to_bits());
        e.write_u32(ebp - 0x50, 0u32);
        e.write_u32(pos + 0x10, pkt[0].to_bits());
        e.write_u32(pos + 0x14, pkt[1].to_bits());
        e.write_u32(pos + 0x18, pkt[2].to_bits());
        e.write_u32(ebx + 0x118, pos as u32);
        e.set_reg(RegisterX86::EBX, ebx);
        e.set_reg(RegisterX86::EDI, outbuf);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x601b7e, 0x601be2);
        let bin = [
            e.read_f32(outbuf),
            e.read_f32(outbuf + 4),
            e.read_f32(outbuf + 8),
        ];
        let ours = case_delta_sub(pkt, cos, sin);
        assert_f32(bin[0], ours[0], &format!("{ctx} x"));
        assert_f32(bin[1], ours[1], &format!("{ctx} y"));
        assert_f32(bin[2], ours[2], &format!("{ctx} z"));
    };

    case(&mut e, [1.0, 2.0, 3.0], 0.5, -0.5, "basic clone");
    case(&mut e, [-7.0, 7.0, 0.0], 0.6, 0.8, "diag clone");
    for i in 0..4000u32 {
        let pkt = [
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        let (cos, sin) = match i % 3 {
            0 => (frand(&mut s, -1.0, 1.0), frand(&mut s, -1.0, 1.0)),
            1 => (frand(&mut s, -1e6, 1e6), frand(&mut s, -1e6, 1e6)),
            _ => (frand_bits(&mut s), frand_bits(&mut s)),
        };
        case(&mut e, pkt, cos, sin, &format!("fuzz{i}"));
    }
}

/// The `≥ eps²` gate, with points around it.
#[test]
fn monster_move_case2_coincident_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x601c_1d00u32;
    let mut seen_store = false;
    let mut seen_skip = false;
    let eps2 = f32::from_bits(0x3a4a_4588);

    let mut case = |e: &mut Emu, a: [f32; 3], pkt: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x2c, a[0].to_bits());
        e.write_u32(ebp - 0x28, a[1].to_bits());
        e.write_u32(ebp - 0x24, a[2].to_bits());
        e.write_u32(ebp - 0x70, pkt[0].to_bits());
        e.write_u32(ebp - 0x6c, pkt[1].to_bits());
        e.write_u32(ebp - 0x68, pkt[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x601c1d, 0x601c50);

        let store = (ah(&*e) & 0x01) == 0;
        if store {
            seen_store = true;
        } else {
            seen_skip = true;
        }
        let (_sos, ours) = case2_coincident_gate(a, pkt);
        assert_eq!(store, ours, "{ctx}: bin_store={store} a={a:?} pkt={pkt:?}");
    };

    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "coincident → skip (sos 0 < eps²)",
    );
    case(
        &mut e,
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "1 >= eps² → store",
    );

    let r = (f64::from(eps2)).sqrt() as f32;
    case(
        &mut e,
        [r, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "sos == eps² (approx) boundary",
    );
    case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "NaN → C0 set → skip",
    );
    for i in 0..5000u32 {
        let pkt = [
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
        ];
        let a = match i % 4 {
            0 => {
                let d = frand(&mut s, 0.0, 0.05);
                [pkt[0] + d, pkt[1], pkt[2]]
            }
            1 => [pkt[0], pkt[1], pkt[2]],
            2 => [
                frand(&mut s, -10.0, 10.0),
                frand(&mut s, -10.0, 10.0),
                frand(&mut s, -10.0, 10.0),
            ],
            _ => [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
        };
        case(&mut e, a, pkt, &format!("fuzz{i}"));
    }
    assert!(seen_store, "store arm never seen");
    assert!(seen_skip, "skip arm never seen");
}

/// The `> eps²` gate, with points around it.
#[test]
fn monster_move_case34_coincident_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let mut s = 0x601d_4100u32;
    let mut seen_store = false;
    let mut seen_skip = false;
    let eps2 = f32::from_bits(0x3a4a_4588);

    let mut case = |e: &mut Emu, a: [f32; 3], base: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x4c, a[0].to_bits());
        e.write_u32(ebp - 0x48, a[1].to_bits());
        e.write_u32(ebp - 0x44, a[2].to_bits());
        e.write_u32(ebp - 0x40, base[0].to_bits());
        e.write_u32(ebp - 0x3c, base[1].to_bits());
        e.write_u32(ebp - 0x38, base[2].to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x601d41, 0x601d79);

        let store = (ah(&*e) & 0x41) == 0;
        if store {
            seen_store = true;
        } else {
            seen_skip = true;
        }
        let (_sos, ours) = case34_coincident_gate(a, base);
        assert_eq!(
            store,
            ours,
            "{ctx}: bin_store={store} ah={:#04x} a={a:?} base={base:?}",
            ah(&*e)
        );
    };

    case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "coincident sos=0 <= eps² → skip",
    );
    case(&mut e, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], "1 > eps² → store");
    let r = (f64::from(eps2)).sqrt() as f32;
    case(
        &mut e,
        [r, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "sos ≈ eps² boundary (strict)",
    );
    case(
        &mut e,
        [f32::from_bits(0x7fc0_0000), 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "NaN → C0|C3 set → skip",
    );
    for i in 0..5000u32 {
        let base = [
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
        ];
        let a = match i % 4 {
            0 => {
                let d = frand(&mut s, 0.0, 0.05);
                [base[0] + d, base[1], base[2]]
            }
            1 => [base[0], base[1], base[2]],
            2 => [
                frand(&mut s, -10.0, 10.0),
                frand(&mut s, -10.0, 10.0),
                frand(&mut s, -10.0, 10.0),
            ],
            _ => [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
        };
        case(&mut e, a, base, &format!("fuzz{i}"));
    }
    assert!(seen_store, "store arm never seen");
    assert!(seen_skip, "skip arm never seen");
}

/// Fixed and random point pairs.
#[test]
fn monster_move_reflect_through_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let buf = e.alloc(0x40);
    let mut s = 0x601e_8300u32;

    let case = |e: &mut Emu, p: [f32; 3], q: [f32; 3], ctx: &str| {
        e.write_u32(buf + 0xc, p[0].to_bits());
        e.write_u32(buf + 0x10, p[1].to_bits());
        e.write_u32(buf + 0x14, p[2].to_bits());
        e.write_u32(buf + 0x18, q[0].to_bits());
        e.write_u32(buf + 0x1c, q[1].to_bits());
        e.write_u32(buf + 0x20, q[2].to_bits());
        e.set_reg(RegisterX86::EAX, buf);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x601e83, 0x601eca);
        let bin = [e.read_f32(buf), e.read_f32(buf + 4), e.read_f32(buf + 8)];
        let ours = reflect_through(p, q);
        assert_f32(bin[0], ours[0], &format!("{ctx} x"));
        assert_f32(bin[1], ours[1], &format!("{ctx} y"));
        assert_f32(bin[2], ours[2], &format!("{ctx} z (live-vs-f32 dz)"));
    };

    case(&mut e, [0.0, 0.0, 0.0], [1.0, 2.0, 3.0], "out = −(q)");
    case(&mut e, [5.0, 5.0, 5.0], [5.0, 5.0, 5.0], "q==p → out=p");

    case(
        &mut e,
        [163_200.33, -638_547.25, 632_252.7],
        [-638_547.25, 632_252.7, 163_200.33],
        "live-vs-f32 z divergence witness",
    );
    case(
        &mut e,
        [0.5, 0.5, 16_777_216.0],
        [0.0, 0.0, 0.5],
        "z f32-gap",
    );
    for i in 0..5000u32 {
        let p = match i % 3 {
            0 => [
                frand(&mut s, -1000.0, 1000.0),
                frand(&mut s, -1000.0, 1000.0),
                frand(&mut s, -1e6, 1e6),
            ],
            1 => [
                frand(&mut s, -1e7, 1e7),
                frand(&mut s, -1e7, 1e7),
                frand(&mut s, -1e7, 1e7),
            ],
            _ => [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
        };
        let q = match i % 3 {
            0 => [
                frand(&mut s, -1000.0, 1000.0),
                frand(&mut s, -1000.0, 1000.0),
                frand(&mut s, -1e6, 1e6),
            ],
            1 => [
                frand(&mut s, -1e7, 1e7),
                frand(&mut s, -1e7, 1e7),
                frand(&mut s, -1e7, 1e7),
            ],
            _ => [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
        };
        case(&mut e, p, q, &format!("fuzz{i}"));
    }
}

/// Paths of several points; the length is read where the speed divide takes it.
#[test]
fn monster_move_arc_length_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let ebx = e.alloc(0x200);
    let pos = e.alloc(0x100);
    let buf = e.alloc(0x400);
    let mut s = 0x601e_da00u32;

    let case = |e: &mut Emu, pts: &[[f32; 3]], move_ms: u32, ctx: &str| {
        let n = pts.len();
        assert!(n >= 4, "need N>=4 points");
        for (k, p) in pts.iter().enumerate() {
            e.write_u32(buf + (12 * k) as u64, p[0].to_bits());
            e.write_u32(buf + (12 * k + 4) as u64, p[1].to_bits());
            e.write_u32(buf + (12 * k + 8) as u64, p[2].to_bits());
        }
        e.write_u32(ebp - 0xc, move_ms);
        e.write_u32(ebx + 0x118, pos as u32);
        e.write_u32(pos + 0x8c, 1.0f32.to_bits());
        e.write_u32(ebp - 0x8, 0);
        e.set_reg(RegisterX86::EDX, n as u64);
        e.set_reg(RegisterX86::EDI, buf);
        e.set_reg(RegisterX86::EBX, ebx);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x601eda, 0x601f7e);

        let bin = e.read_f32(ebp - 0x24);
        let acc = arc_length(&pts[1..n - 1]);
        let ours = (acc / (f64::from(move_ms) * f64::from(f32::from_bits(0x3a83_126f)))) as f32;
        assert_f32(bin, ours, &format!("{ctx} (acc={acc})"));
    };

    let line: Vec<[f32; 3]> = (0..6).map(|i| [i as f32 * 0.5, 0.0, 0.0]).collect();
    case(&mut e, &line, 1000, "straight line 6pt");
    let zig: Vec<[f32; 3]> = (0..8)
        .map(|i| [i as f32 * 0.3, (i % 2) as f32 * 0.4, i as f32 * 0.1])
        .collect();
    case(&mut e, &zig, 500, "zigzag 8pt");
    for it in 0..1500u32 {
        let n = 4 + (next(&mut s) >> 8) as usize % 20;
        let pts: Vec<[f32; 3]> = (0..n)
            .map(|_| {
                [
                    frand(&mut s, -20.0, 20.0),
                    frand(&mut s, -20.0, 20.0),
                    frand(&mut s, -20.0, 20.0),
                ]
            })
            .collect();

        let move_ms = 1 + (next(&mut s) >> 8) % 10000;
        case(&mut e, &pts, move_ms, &format!("fuzz{it}"));
    }
}

/// Both speed scales and path speeds either side of the scaled speed.
#[test]
fn monster_move_speed_duration_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let ebx = e.alloc(0x200);
    let pos = e.alloc(0x100);
    let buf = e.alloc(0x400);
    let mut s = 0x601f_3d00u32;
    let mut seen_alt = false;
    let mut seen_default = false;
    let mut seen_arc_win = false;
    let mut seen_speed_win = false;

    let mut case =
        |e: &mut Emu, pts: &[[f32; 3]], speed: f32, alt_scale: bool, move_ms: u32, ctx: &str| {
            let n = pts.len();
            for (k, p) in pts.iter().enumerate() {
                e.write_u32(buf + (12 * k) as u64, p[0].to_bits());
                e.write_u32(buf + (12 * k + 4) as u64, p[1].to_bits());
                e.write_u32(buf + (12 * k + 8) as u64, p[2].to_bits());
            }
            e.write_u32(pos + 0x8c, speed.to_bits());
            e.write_u32(ebx + 0x118, pos as u32);

            e.write_u32(ebp - 0x8, if alt_scale { 0x200 } else { 0 });
            e.write_u32(ebp - 0xc, move_ms);
            e.set_reg(RegisterX86::EDX, n as u64);
            e.set_reg(RegisterX86::EDI, buf);
            e.set_reg(RegisterX86::EBX, ebx);
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);

            e.run(0x601eda, 0x601faf);
            let bin_1c = e.read_f32(ebp - 0x1c);
            let bin_24 = e.read_f32(ebp - 0x24);
            let acc = arc_length(&pts[1..n - 1]);
            let ours = arc_speed_duration(acc, speed, alt_scale, move_ms);
            assert_f32(bin_1c, ours, &format!("{ctx} [ebp-0x1c]"));
            assert_f32(bin_24, ours, &format!("{ctx} [ebp-0x24]"));

            if alt_scale {
                seen_alt = true;
            } else {
                seen_default = true;
            }
            let scale = if alt_scale { 10.0 } else { 4.0 };
            let speed_scaled = f64::from(speed) * scale;
            let arc_speed = acc / (f64::from(move_ms) * f64::from(f32::from_bits(0x3a83_126f)));
            if move_ms != 0 && arc_speed < speed_scaled {
                seen_arc_win = true;
            } else {
                seen_speed_win = true;
            }
        };

    let pts: Vec<[f32; 3]> = (0..7).map(|i| [i as f32 * 0.6, 0.0, 0.0]).collect();

    case(&mut e, &pts, 0.001, false, 500, "arc-win default scale");
    case(&mut e, &pts, 0.001, true, 500, "arc-win alt scale");

    case(&mut e, &pts, 100.0, false, 1, "speed-win (move_ms=1)");
    case(&mut e, &pts, 50.0, true, 2, "speed-win alt");

    case(&mut e, &pts, 1.0, false, 0, "move_ms==0 → speed_scaled");
    for it in 0..2500u32 {
        let n = 4 + (next(&mut s) >> 8) as usize % 16;
        let pts: Vec<[f32; 3]> = (0..n)
            .map(|_| {
                [
                    frand(&mut s, -15.0, 15.0),
                    frand(&mut s, -15.0, 15.0),
                    frand(&mut s, -15.0, 15.0),
                ]
            })
            .collect();
        let speed = match it % 3 {
            0 => frand(&mut s, 0.0005, 0.5),
            1 => frand(&mut s, 1.0, 100.0),
            _ => frand(&mut s, 0.001, 10.0),
        };
        let alt = (next(&mut s) >> 8).is_multiple_of(2);
        let move_ms = match it % 3 {
            0 => 1 + (next(&mut s) >> 8) % 20,
            _ => 1 + (next(&mut s) >> 8) % 20000,
        };
        case(&mut e, &pts, speed, alt, move_ms, &format!("fuzz{it}"));
    }
    assert!(seen_alt && seen_default, "both speed scales exercised");
    assert!(
        seen_arc_win && seen_speed_win,
        "both speed-select arms exercised"
    );
}
