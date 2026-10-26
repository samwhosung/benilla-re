//! What the tests share: the tables of powers of ten read from the exe, a generator, and the
//! doubles every float test runs.
#![allow(dead_code)]

use benilla_re_crt::ld12::{Ld12, TenPowers};
use benilla_re_difftest::Emu;

pub const TEN_POS: u64 = 0x832490;
pub const TEN_NEG: u64 = 0x8325f0;

fn rows(e: &Emu, at: u64) -> [Ld12; 29] {
    let b = e.read(at, 29 * 12);
    std::array::from_fn(|r| {
        std::array::from_fn(|w| u16::from_le_bytes([b[r * 12 + 2 * w], b[r * 12 + 2 * w + 1]]))
    })
}

/// The two tables at `0x832490` and `0x8325f0`.
pub fn ten_powers(e: &Emu) -> TenPowers {
    TenPowers {
        pos: rows(e, TEN_POS),
        neg: rows(e, TEN_NEG),
    }
}

pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11 ^ self.0 << 31
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Doubles across every regime the digit code distinguishes: both zeros, subnormals, the
/// infinities and NaNs of each kind, powers of ten and their neighbours, values whose 18th digit
/// is a 5, and random bit patterns.
// 3.14159265358979 is pi to 15 digits, as a Lua source writes it, not the constant.
#[allow(clippy::approx_constant)]
pub fn doubles(g: &mut Lcg, random: usize) -> Vec<u64> {
    let mut v: Vec<u64> = vec![
        0,
        1 << 63,
        1,
        (1 << 63) | 1,
        0x000f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        0x7fef_ffff_ffff_ffff,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
        0xfff8_0000_0000_0001,
        0x7ff8_0000_0000_0001,
        0x7ff4_0000_0000_0000,
        0xfff0_0000_0000_0001,
        0x7ff0_0000_0000_0001,
    ];
    for x in [
        0.1,
        0.5,
        1.0,
        1.5,
        2.0,
        9.5,
        10.0,
        99.5,
        100.0,
        0.001,
        0.0001,
        0.00001,
        1e-5,
        9.999_995e-5,
        123456.789,
        1e15,
        1e16,
        1e17,
        1e20,
        1e21,
        1e22,
        1e100,
        1e300,
        1e308,
        1e-300,
        5e-324,
        2.2250738585072014e-308,
        0.3,
        1.0 / 3.0,
        2.0 / 3.0,
        3.14159265358979,
        9.999999999999999,
        99999.95,
        999999.5,
        0.05,
        0.15,
        0.25,
        0.35,
        1.005,
        2.675,
        4.35,
        9007199254740993.0,
        1.234_567_890_123_456_8e17,
        0.000123456,
        4503599627370496.5,
    ] {
        let b = f64::to_bits(x);
        v.extend([b, b ^ (1 << 63), b + 1, b - 1]);
    }
    for k in -330..=310 {
        let b = format!("1e{k}").parse::<f64>().unwrap().to_bits();
        v.extend([b, b + 1, b.wrapping_sub(1)]);
    }
    for _ in 0..random {
        v.push(g.next());
        // Short decimals, which round on a digit near the 17th.
        let digits = g.below(1_000_000_000_000_000_000) as f64;
        let scale = g.below(40) as i32 - 20;
        v.push((digits * 10f64.powi(scale)).to_bits());
    }
    v
}
