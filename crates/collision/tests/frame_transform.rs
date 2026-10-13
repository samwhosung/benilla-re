//! Stretches of `CMovement::TransformFrame` (`0x7c6620`) run as slices, none crossing a call:
//! `0x7c6645` to `0x7c6676`, `0x7c6676` to `0x7c66a7`, `0x7c66a7` to `0x7c66ca`, `0x7c66ca` to
//! `0x7c671c` and `0x7c672d` to `0x7c676c`.

use benilla_re_collision::frame_transform::*;
use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

/// `[0xcf5d00]`, zero in the image until its static initializer stores 2π.
const TWO_PI_GLOBAL: u64 = 0x00cf_5d00;
const TWO_PI_BITS: u32 = 0x40c9_0fdb;
const ZERO_GLOBAL: u64 = 0x007f_fd74;
const EPS_GLOBAL: u64 = 0x0080_29d4;
const EPS_BITS: u32 = 0x3480_0000;
const ONE_GLOBAL: u64 = 0x007f_f9d8;
const ONE_BITS: u32 = 0x3f80_0000;

const TWO_PI_F: f32 = 6.2831855;
const EPS_F: f32 = 2.384_185_8e-7;

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

/// Writes 2π to `[0xcf5d00]`, as its static initializer does.
fn install_wrap_globals(e: &mut Emu) {
    e.write_u32(TWO_PI_GLOBAL, TWO_PI_BITS);
    e.write_u32(ZERO_GLOBAL, 0);
}

/// Values at and one ulp around 0 and 2π, and NaN.
fn wrap_probe_values() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        1.0,
        std::f32::consts::PI,
        TWO_PI_F,
        TWO_PI_F - 1e-4,
        TWO_PI_F + 1e-4,
        6.2831855 * 1.5,
        -1e-4,
        -1.0,
        -std::f32::consts::PI,
        f32::from_bits(0x7fc0_0000),
    ]
}

/// `0x7c6645`: `+0x1c`.
#[test]
fn frame_transform_facing_advance_a_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    install_wrap_globals(&mut e);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x100);
    let mut s = 0x7c66_45a0u32;
    let mut seen_sub = false;
    let mut seen_add = false;
    let mut seen_keep = false;

    let mut case = |e: &mut Emu, arg: f32, facing: f32, ctx: &str| {
        e.write_u32(ebp + 0xc, arg.to_bits());
        e.write_u32(this + 0x1c, facing.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7c6645, 0x7c6676);
        let bin = e.read_f32(this + 0x1c);
        let ours = facing_advance_a(arg, facing);
        assert_f32(bin, ours, ctx);
        let v = f64::from(arg) + f64::from(facing);
        if v > f64::from(TWO_PI_F) {
            seen_sub = true;
        } else if v < 0.0 {
            seen_add = true;
        } else {
            seen_keep = true;
        }
    };

    for &facing in &[0.0f32, 1.0, 3.0, TWO_PI_F, -1.0] {
        for &arg in &wrap_probe_values() {
            case(&mut e, arg, facing, &format!("wrap a={arg} f={facing}"));
        }
    }

    case(&mut e, TWO_PI_F, 0.0, "arg+facing == 2π → keep");
    case(&mut e, 0.0, 0.0, "0 → keep");
    case(&mut e, -0.5, -0.5, "−1 < 0 → +2π");
    case(&mut e, 4.0, 4.0, "8 > 2π → −2π");
    for i in 0..4000u32 {
        let facing = frand(&mut s, 0.0, TWO_PI_F);
        let arg = match i % 4 {
            0 => frand(&mut s, -TWO_PI_F, TWO_PI_F),
            1 => TWO_PI_F - facing + frand(&mut s, -0.01, 0.01),
            2 => -facing + frand(&mut s, -0.01, 0.01),
            _ => frand_bits(&mut s),
        };
        case(&mut e, arg, facing, &format!("fuzz{i}"));
    }
    assert!(
        seen_sub && seen_add && seen_keep,
        "all wrap arms exercised (sub={seen_sub} add={seen_add} keep={seen_keep})"
    );
}

/// `0x7c6676`: `+0x50`.
#[test]
fn frame_transform_facing_advance_b_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    install_wrap_globals(&mut e);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x100);
    let mut s = 0x7c66_76a0u32;
    let mut seen_sub = false;
    let mut seen_add = false;
    let mut seen_keep = false;

    let mut case = |e: &mut Emu, arg: f32, facing: f32, ctx: &str| {
        e.write_u32(ebp + 0xc, arg.to_bits());
        e.write_u32(this + 0x50, facing.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7c6676, 0x7c66a7);
        let bin = e.read_f32(this + 0x50);
        let ours = facing_advance_b(arg, facing);
        assert_f32(bin, ours, ctx);
        let v = f64::from(arg) + f64::from(facing);
        if v > f64::from(TWO_PI_F) {
            seen_sub = true;
        } else if v < 0.0 {
            seen_add = true;
        } else {
            seen_keep = true;
        }
    };

    for &facing in &[0.0f32, 1.0, 3.0, TWO_PI_F, -1.0] {
        for &arg in &wrap_probe_values() {
            case(&mut e, arg, facing, &format!("wrap a={arg} f={facing}"));
        }
    }
    case(&mut e, TWO_PI_F, 0.0, "arg+facing == 2π → keep");
    case(&mut e, 0.0, 0.0, "0 → keep");
    case(&mut e, -0.5, -0.5, "−1 < 0 → +2π");
    case(&mut e, 4.0, 4.0, "8 > 2π → −2π");
    for i in 0..4000u32 {
        let facing = frand(&mut s, 0.0, TWO_PI_F);
        let arg = match i % 4 {
            0 => frand(&mut s, -TWO_PI_F, TWO_PI_F),
            1 => TWO_PI_F - facing + frand(&mut s, -0.01, 0.01),
            2 => -facing + frand(&mut s, -0.01, 0.01),
            _ => frand_bits(&mut s),
        };
        case(&mut e, arg, facing, &format!("fuzz{i}"));
    }
    assert!(
        seen_sub && seen_add && seen_keep,
        "all wrap arms exercised (sub={seen_sub} add={seen_add} keep={seen_keep})"
    );
}

/// `0x7c66a7`: both stores.
#[test]
fn frame_transform_transport_delta_affine_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x100);
    let edi = e.alloc(0x40);
    let mut s = 0x7c66_a7a0u32;

    let case = |e: &mut Emu, src: f32, scale: f32, scale2: f32, offset: f32, ctx: &str| {
        e.write_u32(edi + 0x28, src.to_bits());
        e.write_u32(edi + 0x38, offset.to_bits());
        e.write_u32(this + 0x7c, scale.to_bits());
        e.write_u32(this + 0x80, scale2.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7c66a7, 0x7c66ca);
        let bin0 = e.read_f32(this + 0x7c);
        let bin1 = e.read_f32(this + 0x80);
        assert_f32(
            bin0,
            transport_delta_affine(src, scale, offset),
            &format!("{ctx} store0"),
        );
        assert_f32(
            bin1,
            transport_delta_affine(src, scale2, offset),
            &format!("{ctx} store1"),
        );
    };

    case(&mut e, 2.0, 3.0, 4.0, 5.0, "2·3+5 / 2·4+5");
    case(&mut e, 0.0, 0.0, 0.0, 0.0, "all zero");
    case(&mut e, -1.5, 2.0, -3.0, 0.25, "neg mix");

    case(
        &mut e,
        163_207.33,
        638_521.25,
        -421_887.7,
        0.000_123_4,
        "large product, live-f64 carry",
    );
    case(
        &mut e,
        16_777_217.0,
        1.0000001,
        1.0,
        -8.0e7,
        "f32-gap product",
    );
    for i in 0..4000u32 {
        let src = match i % 3 {
            0 => frand(&mut s, -10.0, 10.0),
            1 => frand(&mut s, -1e6, 1e6),
            _ => frand_bits(&mut s),
        };
        let scale = match i % 3 {
            0 => frand(&mut s, -10.0, 10.0),
            1 => frand(&mut s, -1e6, 1e6),
            _ => frand_bits(&mut s),
        };
        let scale2 = frand(&mut s, -1e3, 1e3);
        let offset = match i % 2 {
            0 => frand(&mut s, -1e5, 1e5),
            _ => frand_bits(&mut s),
        };
        case(&mut e, src, scale, scale2, offset, &format!("fuzz{i}"));
    }
}

/// `0x7c66ca`: the three products.
#[test]
fn frame_transform_matvec3_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let this = e.alloc(0x80);
    let edi = e.alloc(0x40);
    let vec = this + 0x5c;
    let mut s = 0x7c66_caa0u32;

    let case = |e: &mut Emu, m: [f32; 16], v: [f32; 3], ctx: &str| {
        for (i, &mi) in m.iter().enumerate() {
            e.write_u32(edi + 4 * i as u64, mi.to_bits());
        }
        e.write_u32(vec, v[0].to_bits());
        e.write_u32(vec + 4, v[1].to_bits());
        e.write_u32(vec + 8, v[2].to_bits());

        e.set_reg(RegisterX86::EAX, vec);
        e.set_reg(RegisterX86::ECX, vec);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7c66ca, 0x7c671c);
        let bin = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let ours = matvec3(m, v);
        assert_f32(bin[0], ours[0], &format!("{ctx} out0"));
        assert_f32(bin[1], ours[1], &format!("{ctx} out1"));
        assert_f32(bin[2], ours[2], &format!("{ctx} out2"));
    };

    let mut ident = [0.0f32; 16];
    ident[0] = 1.0;
    ident[5] = 1.0;
    ident[10] = 1.0;
    case(&mut e, ident, [1.0, 2.0, 3.0], "identity");
    case(&mut e, ident, [-4.0, 5.5, -6.25], "identity neg");
    let dense: [f32; 16] = [
        0.36, -0.8, 0.0, 0.0, 0.48, 0.6, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    case(&mut e, dense, [1.0, 2.0, 3.0], "dense");

    let big: [f32; 16] = [
        1e7, 1e-3, 4.7, 0.0, -3.3e6, 9.1, 2.2e5, 0.0, 8.8, -1.1e7, 6.6e-4, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    case(
        &mut e,
        big,
        [1.7e3, -2.9e-2, 5.5e4],
        "big mixed accumulation",
    );
    for i in 0..3000u32 {
        let mut m = [0.0f32; 16];
        for mi in m.iter_mut() {
            *mi = match i % 4 {
                0 => frand(&mut s, -1.0, 1.0),
                1 => frand(&mut s, -100.0, 100.0),
                2 => frand(&mut s, -1e6, 1e6),
                _ => frand_bits(&mut s),
            };
        }
        let v = match i % 2 {
            0 => [
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
                frand(&mut s, -50.0, 50.0),
            ],
            _ => [
                frand(&mut s, -1e5, 1e5),
                frand(&mut s, -1e5, 1e5),
                frand(&mut s, -1e5, 1e5),
            ],
        };
        case(&mut e, m, v, &format!("fuzz{i}"));
    }
}

/// `0x7c672d`: `+0x68` and `+0x6c`, with lengths around 2^-22.
#[test]
fn frame_transform_normalize2_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    e.write_u32(EPS_GLOBAL, EPS_BITS);
    e.write_u32(ONE_GLOBAL, ONE_BITS);
    let this = e.alloc(0x100);
    let mut s = 0x7c67_2da0u32;
    let mut seen_norm = false;
    let mut seen_skip = false;

    let mut case = |e: &mut Emu, x: f32, y: f32, ctx: &str| {
        e.write_u32(this + 0x68, x.to_bits());
        e.write_u32(this + 0x6c, y.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7c672d, 0x7c676c);
        let bin = [e.read_f32(this + 0x68), e.read_f32(this + 0x6c)];
        let ours = normalize2(x, y);
        assert_f32(bin[0], ours[0], &format!("{ctx} x"));
        assert_f32(bin[1], ours[1], &format!("{ctx} y"));
        let len = normalize2_len(x, y);
        if normalize2_fires(len) {
            seen_norm = true;
        } else {
            seen_skip = true;
        }
    };

    case(&mut e, 3.0, 4.0, "3,4 → /5");
    case(&mut e, 1.0, 0.0, "1,0 → 1,0");
    case(&mut e, -1.0, -1.0, "−1,−1");
    case(&mut e, 0.0, 0.0, "0,0 → len 0 < eps → skip (raw 0,0)");

    case(&mut e, EPS_F, 0.0, "len == eps exactly → normalize");

    case(&mut e, EPS_F * 0.5, 0.0, "len < eps → skip raw");

    case(&mut e, EPS_F * 1.5, 0.0, "len > eps → normalize");
    case(
        &mut e,
        f32::from_bits(0x7fc0_0000),
        1.0,
        "NaN x → len NaN → normalize (NaN out)",
    );
    case(&mut e, 1e20, 1e20, "huge → len finite/inf path");
    for i in 0..5000u32 {
        let (x, y) = match i % 5 {
            0 => (frand(&mut s, -10.0, 10.0), frand(&mut s, -10.0, 10.0)),
            1 => (frand(&mut s, -1e6, 1e6), frand(&mut s, -1e6, 1e6)),

            2 => (frand(&mut s, -3.0 * EPS_F, 3.0 * EPS_F), 0.0),
            3 => (
                frand(&mut s, -2.0 * EPS_F, 2.0 * EPS_F),
                frand(&mut s, -2.0 * EPS_F, 2.0 * EPS_F),
            ),
            _ => (frand_bits(&mut s), frand_bits(&mut s)),
        };
        case(&mut e, x, y, &format!("fuzz{i}"));
    }
    assert!(
        seen_norm && seen_skip,
        "both normalize arms exercised (norm={seen_norm} skip={seen_skip})"
    );
}
