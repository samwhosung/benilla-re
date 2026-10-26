//! The digits of a `double` (`systems/crt/decimal.md`): `___ld12mul` (`0x412ef1`),
//! `___multtenpow12` (`0x413111`), `___dtold` (`0x40ffcc`) and `$I10_OUTPUT` (`0x4116fd`), each run
//! whole against `benilla_re_crt`, with the helpers `___addl` (`0x41104f`), `0x411070`,
//! `0x4110ce` and `0x4110fc` and the copy `0x40d510` running as the client's.

mod common;

use benilla_re_crt::fltout::{dtold, i10_output, Fos, Ld10};
use benilla_re_crt::ld12::{ld12mul, multtenpow12, Ld12};
use benilla_re_difftest::{Emu, Wow};
use common::{doubles, ten_powers, Lcg};

const LD12MUL: u64 = 0x412ef1;
const MULTTENPOW12: u64 = 0x413111;
const DTOLD: u64 = 0x40ffcc;
const I10_OUTPUT: u64 = 0x4116fd;

fn put12(e: &mut Emu, at: u64, x: &Ld12) {
    let b: Vec<u8> = x.iter().flat_map(|w| w.to_le_bytes()).collect();
    e.write(at, &b);
}

fn get12(e: &Emu, at: u64) -> Ld12 {
    let b = e.read(at, 12);
    std::array::from_fn(|w| u16::from_le_bytes([b[2 * w], b[2 * w + 1]]))
}

/// A 12-byte real with an exponent from every regime: zero, the subnormal range, near the bias,
/// near the product's limits and 0x7fff, a mantissa normalized or not, and either sign.
fn ld12(g: &mut Lcg) -> Ld12 {
    let mut x: Ld12 = std::array::from_fn(|_| g.next() as u16);
    if g.below(2) == 0 {
        x[4] |= 0x8000;
    }
    let exp = match g.below(10) {
        0 => 0,
        1 => g.below(0x40) as u16,
        2 => 0x7fff,
        3 => 0x7ffe - g.below(0x40) as u16,
        4 => 0x3fbf + g.below(0x80) as u16 - 0x40,
        5 => 0x5fff + g.below(0x80) as u16 - 0x40,
        6 => 0x1fe0 + g.below(0x40) as u16,
        _ => 0x3fff + g.below(0x400) as u16 - 0x200,
    };
    if g.below(16) == 0 {
        x[..5].fill(0);
    }
    if g.below(16) == 0 {
        x[1..5].fill(0xffff);
    }
    x[5] = exp | (x[5] & 0x8000);
    x
}

/// Random pairs over every regime, and the factors `___multtenpow12` uses.
#[test]
fn crt_ld12mul_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let t = ten_powers(&e);
    let (xa, ya) = (e.alloc(16), e.alloc(16));
    let mut g = Lcg(0x1d12_0001);
    // A rounding that carries through the low 64 bits into the top word (`0x4130bf`), and one
    // that carries out of it.
    for (x, y) in [
        (
            [0xfffe, 0xffff, 0xffff, 0xffff, 0xc001, 0x3fff],
            [1, 0, 0, 0, 0x8000, 0x3fff],
        ),
        (
            [0xfffe, 0xffff, 0xffff, 0xffff, 0xffff, 0x3fff],
            [1, 0, 0, 0, 0x8000, 0x3fff],
        ),
    ] {
        put12(&mut e, xa, &x);
        put12(&mut e, ya, &y);
        e.call(LD12MUL, 0, 0, &[xa as u32, ya as u32]);
        let mut want = x;
        ld12mul(&mut want, &y);
        assert_eq!(get12(&e, xa), want, "{x:04x?} * {y:04x?}");
    }
    for n in 0..60_000 {
        let x = ld12(&mut g);
        let y = if n % 4 == 0 {
            let r = &t.pos[g.below(29) as usize];
            if n % 8 == 0 {
                *r
            } else {
                t.neg[g.below(29) as usize]
            }
        } else {
            ld12(&mut g)
        };
        put12(&mut e, xa, &x);
        put12(&mut e, ya, &y);
        e.call(LD12MUL, 0, 0, &[xa as u32, ya as u32]);
        let mut want = x;
        ld12mul(&mut want, &y);
        assert_eq!(get12(&e, xa), want, "{x:04x?} * {y:04x?}");
        assert_eq!(get12(&e, ya), y, "y kept");
    }
}

/// Powers from -5000 to 5000, every single octal digit and 8191, with and without `mult12`.
#[test]
fn crt_multtenpow12_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let t = ten_powers(&e);
    let xa = e.alloc(16);
    let mut g = Lcg(0x10e5_0002);
    let mut pows: Vec<i32> = (-64..=64).collect();
    // Every octal digit of the four full groups, and the one row of the fifth: a power of 8192
    // or more reads past the table.
    for k in 0..5 {
        for d in 1..if k == 4 { 2 } else { 8 } {
            pows.push(d << (3 * k));
            pows.push(-(d << (3 * k)));
        }
    }
    pows.extend([8191, -8191, 4095 + 4096, -(4095 + 4096)]);
    for _ in 0..4000 {
        pows.push(g.below(10_001) as i32 - 5000);
    }
    for &p in &pows {
        for mult12 in [0u32, 1] {
            let mut x = ld12(&mut g);
            if g.below(2) == 0 {
                x[5] = 0x3fff + (g.below(64) as u16) - 32;
                x[4] |= 0x8000;
            }
            put12(&mut e, xa, &x);
            e.call(MULTTENPOW12, 0, 0, &[xa as u32, p as u32, mult12]);
            let mut want = x;
            multtenpow12(&mut want, p, mult12 != 0, &t);
            assert_eq!(get12(&e, xa), want, "{x:04x?} * 10^{p} mult12 {mult12}");
        }
    }
}

#[test]
fn crt_dtold_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let (out, inp) = (e.alloc(16), e.alloc(16));
    let mut g = Lcg(0xd701_0003);
    for bits in doubles(&mut g, 20_000) {
        e.write(out, &[0xa5; 12]);
        e.write(inp, &bits.to_le_bytes());
        e.call(DTOLD, 0, 0, &[out as u32, inp as u32]);
        let b = e.read(out, 12);
        let want = dtold(bits);
        let got = Ld10 {
            man: u64::from_le_bytes(b[..8].try_into().unwrap()),
            exp: u16::from_le_bytes([b[8], b[9]]),
        };
        assert_eq!(got, want, "{bits:016x}");
        assert_eq!(b[10..], [0xa5, 0xa5], "two bytes past the real kept");
    }
}

fn fos_bytes(f: &Fos) -> Vec<u8> {
    let mut b = f.exp.to_le_bytes().to_vec();
    b.extend([f.sign, f.len]);
    b.extend_from_slice(&f.man);
    b
}

/// The doubles as `___dtold` widens them and random 80-bit reals, for 0 to 24 digits and both
/// flags; the `FOS` starts with the same bytes on both sides and is compared whole.
#[test]
fn crt_i10_output_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let t = ten_powers(&e);
    let fos = e.alloc(32);
    let mut g = Lcg(0x110_0004);
    let mut lds: Vec<Ld10> = doubles(&mut g, 3000).into_iter().map(dtold).collect();
    for _ in 0..3000 {
        let mut man = g.next() << 1 | g.below(2);
        if g.below(4) != 0 {
            man |= 1 << 63;
        }
        let exp = match g.below(4) {
            0 => g.below(0x8000) as u16,
            1 => 0x7fff,
            _ => 0x3fff + g.below(0x200) as u16 - 0x100,
        } | (g.below(2) as u16) << 15;
        lds.push(Ld10 { man, exp });
    }
    let mut args = vec![(17, 0u32)];
    for n in 0..=24 {
        args.push((n, 0));
        args.push((n, 1));
        args.push((-n, 1));
    }
    for (k, ld) in lds.iter().enumerate() {
        let (ndigits, flags) = if k % 3 == 0 {
            args[0]
        } else {
            args[g.below(args.len() as u64) as usize]
        };
        let mut want = Fos {
            exp: g.next() as i16,
            sign: g.next() as u8,
            len: g.next() as u8,
            man: std::array::from_fn(|_| g.next() as u8),
        };
        e.write(fos, &fos_bytes(&want));
        let lo = ld.man as u32;
        let hi = (ld.man >> 32) as u32;
        let ret = e.call(
            I10_OUTPUT,
            0,
            0,
            &[lo, hi, u32::from(ld.exp), ndigits as u32, flags, fos as u32],
        );
        let want_ret = i10_output(*ld, ndigits, flags, &mut want, &t);
        assert_eq!(
            (ret as i32, e.read(fos, 26)),
            (want_ret, fos_bytes(&want)),
            "{ld:x?} ndigits {ndigits} flags {flags}"
        );
    }
}
