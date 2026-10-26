//! Text to a `double` (`systems/crt/strtod.md`): `strtod` (`0x741e39`) run whole, with `__fltin2`
//! (`0x747560`), `___strgtold12` (`0x4111f0`), `___mtold12` (`0x411129`), `___multtenpow12`
//! (`0x413111`), `_ld12tod` (`0x40fdcb`) and `__ld12cvt` (`0x40fc5f`) running as the client's,
//! against `benilla_re_crt::strtod`; `_errno` (`0x4104e6`) is stubbed with a slot the test reads.
//! `___strgtold12` and `__ld12cvt` also run alone, the second over the float format too.
//!
//! The result passes through `fld`/`fstp` of a double at `0x7475cf` and `0x741edc`; no case gives
//! a signalling NaN, which the emulator would keep signalling (`docs/METHOD.md#the-harness`).

mod common;

use benilla_re_crt::ld12::Ld12;
use benilla_re_crt::strtod::{ld12cvt, strgtold12, strtod, DOUBLE, FLOAT};
use benilla_re_difftest::{Emu, Wow};
use common::{ten_powers, Lcg};

const STRTOD: u64 = 0x741e39;
const STRGTOLD12: u64 = 0x4111f0;
const LD12CVT: u64 = 0x40fc5f;
const ERRNO: u64 = 0x4104e6;
const DOUBLE_FORMAT: u32 = 0x831928;
const FLOAT_FORMAT: u32 = 0x831940;

/// Numerals at every edge the scan and the rounding distinguish.
const EDGES: &[&str] = &[
    "",
    " ",
    "0",
    "-0",
    "+0",
    "00",
    "0.",
    ".0",
    ".",
    "-.",
    "+",
    "-",
    "1",
    "-1",
    " \t\n\x0b\x0c\r 42",
    "\x0b5",
    "1.5",
    "-1.5e3",
    "1e",
    "1e+",
    "1e-",
    "1ex",
    "1E5",
    "1d5",
    "1D-5",
    "1f5",
    "0x10",
    "0X1p3",
    "inf",
    "-INF",
    "nan",
    "1..2",
    "1.2.3",
    "1+2",
    "1-2",
    "0+1",
    "5e0",
    "5e00",
    "5e-0",
    "5e0007",
    "1e5200",
    "1e5201",
    "1e-5200",
    "1e-5201",
    "-1e-5201",
    "1e99999999999",
    "-1e99999999999",
    "1e-99999999999",
    "0e99999",
    "0.0000000000000000000000000000001",
    "000000000000000000000000000000000001",
    "123456789012345678901234",
    "1234567890123456789012345",
    "12345678901234567890123456789",
    "999999999999999999999999",
    "9999999999999999999999999",
    "999999999999999999999995",
    "0.123456789012345678901234567",
    "100000000000000000000000000000000",
    "1.7976931348623157e308",
    "1.7976931348623158e308",
    "1.797693134862315807e308",
    "1.7976931348623159e308",
    "-1.8e308",
    "1e308",
    "1e309",
    "2.2250738585072014e-308",
    "2.2250738585072011e-308",
    "2.2250738585072009e-308",
    "-2.2250738585072014e-308",
    "1e-308",
    "4.9e-324",
    "5e-324",
    "2.5e-324",
    "1e-330",
    "-1e-310",
    "0.1",
    "6e23",
    "0.2",
    "0.3",
    "3.14159265358979",
    "9007199254740993",
    "9007199254740992.5",
    "9007199254740993.0000000001",
    "4503599627370496.5",
    "4503599627370497.5",
    "2.675",
    "1.005",
    "123456.789e-3",
    "1x",
    "12abc",
    "1e5x",
    "1e+5+",
    "\u{80}1",
    "1\u{e9}",
];

struct Rig<'a> {
    e: Emu<'a>,
    buf: u64,
    end: u64,
    errno: u64,
}

impl Rig<'_> {
    fn new(wow: &Wow) -> Rig<'_> {
        let mut e = Emu::new(wow);
        let buf = e.alloc(0x2000);
        let end = e.alloc(0x10);
        let errno = e.alloc(0x10);
        e.stub(ERRNO, errno as u32, 0);
        Rig { e, buf, end, errno }
    }

    /// Runs `strtod(s, &end)`: the double's bits, the end index and the `errno` value.
    fn run(&mut self, s: &[u8]) -> (u64, usize, u32) {
        let mut z = s.to_vec();
        z.push(0);
        self.e.write(self.buf, &z);
        self.e.write_u32(self.end, 0xdead_beef);
        self.e.write_u32(self.errno, 0);
        let v = self
            .e
            .call_f64(STRTOD, 0, 0, &[self.buf as u32, self.end as u32]);
        let end = self.e.read_u32(self.end) as usize - self.buf as usize;
        (v.to_bits(), end, self.e.read_u32(self.errno))
    }
}

/// A random numeral: blanks, a sign, digit runs with zeros, a point, an exponent mark, a sign
/// and digits, each part present or not, and a tail.
fn numeral(g: &mut Lcg) -> Vec<u8> {
    let mut s = Vec::new();
    let pick = |g: &mut Lcg, set: &[u8]| set[g.below(set.len() as u64) as usize];
    for _ in 0..g.below(3) {
        s.push(pick(g, b" \t"));
    }
    if g.below(3) == 0 {
        s.push(pick(g, b"+-"));
    }
    let digits = |g: &mut Lcg, s: &mut Vec<u8>, n: u64| {
        let zeros = g.below(4) == 0;
        for _ in 0..n {
            s.push(if zeros && g.below(2) == 0 {
                b'0'
            } else {
                b'0' + g.below(10) as u8
            });
        }
    };
    let long = if g.below(4) == 0 { 40 } else { 20 };
    let n = g.below(long);
    digits(g, &mut s, n);
    if g.below(2) == 0 {
        s.push(b'.');
        let n = g.below(long);
        digits(g, &mut s, n);
    }
    if g.below(2) == 0 {
        s.push(pick(g, b"eEdD"));
        if g.below(2) == 0 {
            s.push(pick(g, b"+-"));
        }
        let e = match g.below(4) {
            0 => g.below(10),
            1 => g.below(400),
            2 => 290 + g.below(40),
            _ => g.below(6000),
        };
        s.extend(e.to_string().bytes());
    }
    if g.below(4) == 0 {
        s.push(pick(g, b"x.e+-0 "));
    }
    s
}

/// The numerals at the doubles' boundaries: each power of ten and each neighbour of a power of two
/// written with 17, 20 and 25 digits.
fn boundaries() -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    for k in -330..=310 {
        v.push(format!("1e{k}").into_bytes());
        v.push(format!("9.99999999999999999e{k}").into_bytes());
    }
    for e in [-1075, -1074, -1023, -1022, -1021, -1, 0, 52, 53, 1022, 1023] {
        let x = 2f64.powi(e);
        for bits in [x.to_bits().wrapping_sub(1), x.to_bits(), x.to_bits() + 1] {
            let y = f64::from_bits(bits);
            v.push(format!("{y:.16e}").into_bytes());
            v.push(format!("{y:.19e}").into_bytes());
            v.push(format!("{y:.24e}").into_bytes());
        }
    }
    v
}

#[test]
fn crt_strtod_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let t = ten_powers(&r.e);
    let mut g = Lcg(0x5717_0d00);
    let mut cases: Vec<Vec<u8>> = EDGES.iter().map(|s| s.as_bytes().to_vec()).collect();
    cases.extend(boundaries());
    for _ in 0..30_000 {
        cases.push(numeral(&mut g));
    }
    for s in &cases {
        let want = strtod(s, &t);
        let (bits, end, errno) = r.run(s);
        let shown = String::from_utf8_lossy(s);
        assert_eq!(bits, want.bits, "strtod({shown:?}) value");
        assert_eq!(end, want.end, "strtod({shown:?}) end");
        assert_eq!(
            errno,
            if want.erange { 0x22 } else { 0 },
            "strtod({shown:?}) errno"
        );
    }
}

fn get12(e: &Emu, at: u64) -> Ld12 {
    let b = e.read(at, 12);
    std::array::from_fn(|w| u16::from_le_bytes([b[2 * w], b[2 * w + 1]]))
}

/// `___strgtold12` alone, with its last four arguments set: the multiply flag, a scale and a
/// decimal point that apply only without an exponent or a point, and the implicit exponent that
/// takes a sign after the mantissa.
#[test]
fn crt_strgtold12_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let t = ten_powers(&e);
    let (buf, ld, end) = (e.alloc(0x200), e.alloc(0x10), e.alloc(0x10));
    let mut g = Lcg(0x5717_0d01);
    let mut cases: Vec<Vec<u8>> = EDGES.iter().map(|s| s.as_bytes().to_vec()).collect();
    for _ in 0..5_000 {
        cases.push(numeral(&mut g));
    }
    for s in &cases {
        for k in 0..4 {
            let mult12 = k & 1 != 0;
            let implicit = k & 2 != 0;
            let scale = g.below(41) as i32 - 20;
            let decpt = g.below(41) as i32 - 20;
            let mut z = s.clone();
            z.push(0);
            e.write(buf, &z);
            let flags = e.call(
                STRGTOLD12,
                0,
                0,
                &[
                    ld as u32,
                    end as u32,
                    buf as u32,
                    u32::from(mult12),
                    scale as u32,
                    decpt as u32,
                    u32::from(implicit),
                ],
            );
            let want = strgtold12(s, mult12, scale, decpt, implicit, &t);
            let shown = String::from_utf8_lossy(s);
            let what = format!("strgtold12({shown:?}, {mult12}, {scale}, {decpt}, {implicit})");
            assert_eq!(flags, want.flags, "{what} flags");
            assert_eq!(get12(&e, ld), want.ld, "{what} real");
            assert_eq!(
                e.read_u32(end) as usize - buf as usize,
                want.end,
                "{what} end"
            );
        }
    }
}

/// `__ld12cvt` alone over 12-byte reals of every exponent the two formats distinguish, with
/// mantissas whose bits after the kept ones are each pattern the rounding reads.
#[test]
fn crt_ld12cvt_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let (src, dst) = (e.alloc(0x10), e.alloc(0x10));
    let mut g = Lcg(0x5717_0d02);
    for i in 0..60_000u64 {
        let (fmt, format, prec) = if i % 2 == 0 {
            (DOUBLE_FORMAT, DOUBLE, 53)
        } else {
            (FLOAT_FORMAT, FLOAT, 24)
        };
        let mut man: u128 = u128::from(g.next()) << 64 | u128::from(g.next());
        man &= (1 << 80) - 1;
        // The bits past the kept ones: random, all clear, only the round bit, the round bit and
        // the next, the round bit and one far below, all set.
        let tail = 80 - prec;
        let keep = man >> tail << tail;
        man = match g.below(7) {
            0 => keep,
            1 => keep | 1 << (tail - 1),
            2 => keep | 3 << (tail - 2),
            3 => keep | 1 << (tail - 1) | 1,
            4 => keep | ((1 << tail) - 1),
            _ => man,
        };
        if g.below(8) != 0 {
            man |= 1 << 79;
        }
        if g.below(16) == 0 {
            man = (1 << 80) - 1;
        }
        let bias = 0x3fff;
        let exp: u16 = match g.below(8) {
            0 => 0,
            1 => g.below(0x7fff) as u16,
            2 => (bias + format.min_exp - format.precision as i32 - 2 + g.below(6) as i32) as u16,
            3 => (bias + format.min_exp - 2 + g.below(5) as i32) as u16,
            4 => (bias + format.max_exp - 2 + g.below(5) as i32) as u16,
            5 => 0x7fff,
            _ => {
                (bias + format.min_exp + g.below((format.max_exp - format.min_exp) as u64) as i32)
                    as u16
            }
        };
        let sign = if g.below(2) == 0 { 0x8000 } else { 0 };
        let x: Ld12 = [
            man as u16,
            (man >> 16) as u16,
            (man >> 32) as u16,
            (man >> 48) as u16,
            (man >> 64) as u16,
            exp | sign,
        ];
        let b: Vec<u8> = x.iter().flat_map(|w| w.to_le_bytes()).collect();
        e.write(src, &b);
        e.write(dst, &[0xa5; 8]);
        let ret = e.call(LD12CVT, 0, 0, &[src as u32, dst as u32, fmt]);
        let (bits, want_ret) = ld12cvt(&x, &format);
        let got = if prec == 53 {
            u64::from(e.read_u32(dst)) | u64::from(e.read_u32(dst + 4)) << 32
        } else {
            u64::from(e.read_u32(dst))
        };
        assert_eq!((got, ret), (bits, want_ret), "ld12cvt({x:04x?}, {prec})");
    }
}
