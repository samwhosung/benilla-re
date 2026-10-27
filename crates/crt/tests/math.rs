//! `_frexp` (`0x7fd8db`) with `__decomp` (`0x40c6e6`), `__sptype`, `__set_exp`, `__handle_qnan1`,
//! `__except1`, `__handle_exc` and `__ctrlfp`; `__CIfmod` (`0x73f90a`) through the dispatcher
//! `0x7436c0`, `0x743997` and the exit `0x74378a` with `__87except` (`0x747aa8`); and `__CIpow`
//! (`0x73f940`) with `__powhlp` (`0x744c8c`) and `__d_inttype` (`0x744dbc`), all run whole against
//! `benilla_re_crt::math` (`systems/crt/math.md`). `_errno` (`0x4104e6`) is a stub that answers a
//! slot the test reads.
//!
//! `__CIfmod` and `__CIpow` take their operands on the x87 stack, so a code island loads them
//! (`fld qword`) and stores the result (`fstp qword`). `pow` is run where an operand is a NaN or an
//! infinity, which never reaches `fyl2x`. The one `fprem` the fmod cases run, at `0x73f91f`, is
//! over operands of at most 24 significant bits within 20 binary orders of each other, whose
//! remainder the emulator's double arithmetic computes exactly (`docs/METHOD.md#the-harness`);
//! the comparison there rests on that, except for the sign of a zero remainder: the exe's `fprem`
//! gives the dividend's sign, the emulator's gives `+0`, so a zero answer is compared by value.
//! No `fmod` or `pow` operand is a signalling NaN, which the emulator would keep signalling where a
//! real load quiets it; `frexp` takes them, since it quiets by adding.

mod common;

use benilla_re_crt::math::{fmod, frexp, pow_nonfinite, Answer};
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::Lcg;

const FREXP: u64 = 0x7fd8db;
const CIFMOD: u64 = 0x73f90a;
const CIPOW: u64 = 0x73f940;
const ERRNO: u64 = 0x4104e6;
const EDOM: u32 = 33;
const STACK: u64 = 0x7008_0000;

struct Rig<'a> {
    e: Emu<'a>,
    errno: u64,
    data: u64,
    island: u64,
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let errno = e.alloc(8);
        e.stub(ERRNO, errno as u32, 0);
        let data = e.alloc(0x40);
        let island = e.alloc(0x40);
        Rig {
            e,
            errno,
            data,
            island,
        }
    }

    fn answer(&mut self, bits: u64) -> Answer {
        Answer {
            bits,
            edom: self.e.read_u32(self.errno) == EDOM,
        }
    }

    fn frexp(&mut self, x: u64) -> (Answer, i32) {
        self.e.write_u32(self.errno, 0);
        let slot = self.data;
        self.e.write_u32(slot, 0xdead_beef);
        let r = self
            .e
            .call_f64(FREXP, 0, 0, &[x as u32, (x >> 32) as u32, slot as u32]);
        let exp = self.e.read_u32(slot) as i32;
        (self.answer(r.to_bits()), exp)
    }

    /// `target` with `lower` the register below the top and `top` the top.
    fn two(&mut self, target: u64, lower: u64, top: u64) -> Answer {
        self.e.write_u32(self.errno, 0);
        let (a, b, res) = (self.data, self.data + 8, self.data + 16);
        self.e.write(a, &lower.to_le_bytes());
        self.e.write(b, &top.to_le_bytes());
        self.e.write(res, &[0; 8]);
        let at = self.island;
        let mut code = vec![0xdd, 0x05];
        code.extend((a as u32).to_le_bytes());
        code.extend([0xdd, 0x05]);
        code.extend((b as u32).to_le_bytes());
        code.push(0xe8);
        let rel = (target as i64 - (at as i64 + code.len() as i64 + 4)) as i32;
        code.extend(rel.to_le_bytes());
        code.extend([0xdd, 0x1d]);
        code.extend((res as u32).to_le_bytes());
        self.e.write(at, &code);
        self.e.set_reg(RegisterX86::ESP, STACK);
        self.e.run(at, at + code.len() as u64);
        let bits = u64::from_le_bytes(self.e.read(res, 8).try_into().unwrap());
        self.answer(bits)
    }
}

/// The answers match; after an `fprem`, a zero remainder by value alone.
fn same(got: Answer, want: Answer, a: u64, b: u64) -> bool {
    let fprem = [a, b]
        .iter()
        .all(|&x| x >> 52 & 0x7ff != 0x7ff && x << 1 != 0);
    got == want || (fprem && got.bits << 1 == 0 && want.bits << 1 == 0 && got.edom == want.edom)
}

fn specials() -> Vec<u64> {
    let mut v = vec![
        0,
        1 << 63,
        1,
        (1 << 63) | 1,
        0x000f_ffff_ffff_ffff,
        0x800f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        0x7fef_ffff_ffff_ffff,
        0xffef_ffff_ffff_ffff,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
        0x7ff8_0000_0000_0001,
        0xfff8_0000_0000_0001,
        0x7ffc_0000_0000_0000,
        0xfffc_0000_0000_0000,
        0x7fff_ffff_ffff_ffff,
        0xffff_ffff_ffff_ffff,
    ];
    for x in [
        1.0, 2.0, 3.0, 4.0, 0.5, 0.25, 1.5, 2.5, 5.5, 7.0, 10.0, 100.0, 1e-5, 1e300, 12345.678,
    ] {
        let b = f64::to_bits(x);
        v.extend([b, b | 1 << 63]);
    }
    v
}

#[test]
fn crt_frexp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut g = Lcg(0x7fd8_db00);
    let mut xs = specials();
    for k in 0..52 {
        xs.push(1 << k);
        xs.push(1 << 63 | 1 << k);
        xs.push(0x0010_0000_0000_0000 | 1 << k);
    }
    for e in 0..0x7ffu64 {
        xs.push(e << 52 | 0x000a_bcde_f012_3456);
        xs.push(e << 52 | 1 << 63);
    }
    for _ in 0..4000 {
        xs.push(g.next());
    }
    for x in xs {
        let want = frexp(x);
        let got = rig.frexp(x);
        assert_eq!(got, want, "frexp({x:#018x})");
    }
}

#[test]
fn crt_fmod_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let s = specials();
    // Pairs with a non-finite or zero operand never reach `fprem`.
    for &a in &s {
        for &b in &s {
            let finite_pair = |x: u64| x >> 52 & 0x7ff != 0x7ff && x << 1 != 0;
            if finite_pair(a) && finite_pair(b) {
                continue;
            }
            let got = rig.two(CIFMOD, a, b);
            assert!(same(got, fmod(a, b), a, b), "fmod({a:#x}, {b:#x}): {got:?}");
        }
    }
    // A double subnormal is a normal number on the x87 stack: its remainder is itself.
    for &(a, b) in &[
        (1u64, 3.0f64),
        (1 << 63 | 1, 2.0),
        (0x000f_ffff_ffff_ffff, -4.0),
    ] {
        let b = f64::to_bits(b);
        let got = rig.two(CIFMOD, a, b);
        assert!(same(got, fmod(a, b), a, b), "fmod({a:#x}, {b:#x}): {got:?}");
    }
    // Operands of at most 24 significant bits within 20 orders: `fprem` is exact in the emulator.
    let mut g = Lcg(0xf0d0_0001);
    for _ in 0..6000 {
        let mant = |g: &mut Lcg| (1 + g.below(1 << 24)) as f64;
        let e = g.below(20) as i32;
        let mut a = mant(&mut g) * 2f64.powi(e);
        let mut b = mant(&mut g);
        if g.below(2) == 0 {
            a = -a;
        }
        if g.below(2) == 0 {
            b = -b;
        }
        let (a, b) = (a.to_bits(), b.to_bits());
        let got = rig.two(CIFMOD, a, b);
        assert!(same(got, fmod(a, b), a, b), "fmod({a:#x}, {b:#x}): {got:?}");
    }
    for &(a, b) in &[
        (5.0, 3.0),
        (-5.0, 3.0),
        (5.0, -3.0),
        (6.0, 3.0),
        (-6.0, 3.0),
        (0.5, 2.0),
    ] {
        let (a, b) = (f64::to_bits(a), f64::to_bits(b));
        let got = rig.two(CIFMOD, a, b);
        assert!(same(got, fmod(a, b), a, b), "fmod({a:#x}, {b:#x}): {got:?}");
    }
}

#[test]
fn crt_pow_nonfinite_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut rig = Rig::new(&wow);
    let mut s = specials();
    for x in [
        -2.0, -3.0, 6.0, -4.0, 0.75, -0.75, 1e-310, -1e-310, 1e19, -1e19,
    ] {
        s.push(f64::to_bits(x));
    }
    let mut cases = 0;
    for &x in &s {
        for &y in &s {
            if let Some(want) = pow_nonfinite(x, y) {
                assert_eq!(rig.two(CIPOW, x, y), want, "pow({x:#x}, {y:#x})");
                cases += 1;
            }
        }
    }
    assert!(cases > 500, "{cases}");
}
