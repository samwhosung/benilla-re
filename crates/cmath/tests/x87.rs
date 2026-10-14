//! The float-to-integer conversions against the exe: `__ftol` (`0x40a2b0`) run for real, and
//! `fistp` to a word, dword and qword run as one instruction, over NaN, the infinities, values at
//! and past each destination's range, ties and the signed zeros.

use benilla_re_cmath::x87::{fistp, fistp16, fistp64, fistp_trunc, ftol, ftol64};
use benilla_re_difftest::{Emu, RegisterX86, Wow};

const FTOL: u64 = 0x40a2b0;

fn probes() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        0.5,
        -0.5,
        1.5,
        2.5,
        -1.5,
        -2.5,
        0.999_999_9,
        -0.999_999_9,
        32_767.5,
        32_768.0,
        -32_768.0,
        -32_768.5,
        -32_769.0,
        2_147_483_647.0,
        2_147_483_647.5,
        2_147_483_648.0,
        -2_147_483_648.0,
        -2_147_483_648.5,
        -2_147_483_649.0,
        4_294_967_296.0,
        4_294_967_297.5,
        6.0e9,
        -3.0e9,
        9_223_372_036_854_775_807.0,
        9_223_372_036_854_775_808.0,
        -9_223_372_036_854_775_808.0,
        -9_223_372_036_854_777_856.0,
        1.0e19,
        -1.0e19,
        1.0e300,
        -1.0e300,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN,
    ];
    let mut s: u64 = 0x87f1_5700;
    for _ in 0..3000 {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // A random exponent and mantissa, so every magnitude from tiny to past `i64` appears.
        let bits = (s & 0x800f_ffff_ffff_ffff) | ((0x380 + (s >> 52) % 0x100) << 52);
        v.push(f64::from_bits(bits));
    }
    v
}

/// `fld qword [in]; call __ftol`: `edx:eax` is the `fistp qword` result, truncated.
#[test]
fn ftol_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let input = e.alloc(0x10);
    let code = e.alloc(0x20);
    let stack = e.alloc(0x100) + 0x80;
    let mut island = vec![0xdd, 0x05];
    island.extend_from_slice(&(input as u32).to_le_bytes());
    island.push(0xe8); // call __ftol
    let rel = (FTOL as u32).wrapping_sub((code + 6 + 5) as u32);
    island.extend_from_slice(&rel.to_le_bytes());
    let end = code + island.len() as u64;
    island.push(0x90);
    e.write(code, &island);
    for v in probes() {
        e.write(input, &v.to_bits().to_le_bytes());
        e.set_reg(RegisterX86::ESP, stack);
        e.run(code, end);
        let lo = e.get_reg(RegisterX86::EAX) as u32;
        let hi = e.get_reg(RegisterX86::EDX) as u32;
        let wide = (u64::from(hi) << 32 | u64::from(lo)) as i64;
        assert_eq!(wide, ftol64(v), "ftol64({v:e}) bits {:#018x}", v.to_bits());
        assert_eq!(lo as i32, ftol(v), "ftol({v:e}) bits {:#018x}", v.to_bits());
    }
    // The indefinite: eax 0 and edx 0x80000000 for NaN and past i64; a value past i32 keeps its
    // low dword.
    assert_eq!(ftol(f64::NAN), 0);
    assert_eq!(ftol(1.0e19), 0);
    assert_eq!(ftol64(f64::INFINITY), i64::MIN);
    assert_eq!(ftol(4_294_967_297.0), 1);
}

/// `fld qword [in]; fistp {word,dword,qword} [out]` under the client's round-to-nearest control
/// word, and the same dword store under the truncating control word `__ftol` sets.
#[test]
fn fistp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let input = e.alloc(0x10);
    let out = e.alloc(0x10);
    let cw = e.alloc(0x10);
    let code = e.alloc(0x80);
    let a = |x: u64| (x as u32).to_le_bytes();
    // fld qword [in]; fistp word [out]; fld; fistp dword [out+4]; fld; fistp qword [out+8]; then
    // fldcw [cw], fld, fistp dword [out+0x14], fldcw [cw+2].
    let mut island: Vec<u8> = vec![];
    for (op, modrm, off) in [(0xdfu8, 0x1du8, 0u64), (0xdb, 0x1d, 4), (0xdf, 0x3d, 8)] {
        island.extend([0xdd, 0x05]);
        island.extend(a(input));
        island.extend([op, modrm]);
        island.extend(a(out + off));
    }
    island.extend([0xd9, 0x2d]);
    island.extend(a(cw));
    island.extend([0xdd, 0x05]);
    island.extend(a(input));
    island.extend([0xdb, 0x1d]);
    island.extend(a(out + 0x14));
    island.extend([0xd9, 0x2d]);
    island.extend(a(cw + 2));
    let end = code + island.len() as u64;
    island.push(0x90);
    e.write(code, &island);
    for v in probes() {
        // The truncating control word, then the client's, as the words `fldcw` loads.
        e.write(cw, &0x0f7fu16.to_le_bytes());
        e.write(cw + 2, &0x037fu16.to_le_bytes());
        e.write(input, &v.to_bits().to_le_bytes());
        e.run(code, end);
        let b = e.read(out, 0x18);
        let w = i16::from_le_bytes([b[0], b[1]]);
        let d = i32::from_le_bytes([b[4], b[5], b[6], b[7]]);
        let q = i64::from_le_bytes(b[8..16].try_into().unwrap());
        let t = i32::from_le_bytes([b[0x14], b[0x15], b[0x16], b[0x17]]);
        assert_eq!(w, fistp16(v), "fistp16({v:e})");
        assert_eq!(d, fistp(v), "fistp({v:e})");
        assert_eq!(q, fistp64(v), "fistp64({v:e})");
        assert_eq!(t, fistp_trunc(v), "fistp_trunc({v:e})");
    }
}
