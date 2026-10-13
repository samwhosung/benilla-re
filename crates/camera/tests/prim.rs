//! `SetParticleDensity` (`0x7adfb0`) run whole and read from `0x87d5fc`, and the `fsincos` of
//! `StoreAngleProperty` (`0x7acc40`) run as the slice `0x7acc87` to `0x7acc90`.

mod common;

use benilla_re_camera::prim::{angle_cossin, clamp01};
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::{assert_f32, fuzz};

const CLAMP01: u64 = 0x7ad_fb0;
const G_CLAMP: u64 = 0x0087_d5fc; // the density
const COSSIN_BEGIN: u64 = 0x7acc87; // fld [ebp+8]; fsincos; fstp [eax]; fstp [edx]
const COSSIN_END: u64 = 0x7acc90;

#[test]
fn camera_clamp01() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // Each edge exactly and a step either side, then random values.
    let probes = [
        0.0f32,
        -0.0,
        1.0,
        f32::from_bits(0x3f7f_ffff), // just below 1.0
        f32::from_bits(0x3f80_0001), // just above 1.0
        f32::from_bits(0x8000_0001), // smallest negative
        f32::from_bits(0x0000_0001), // smallest positive
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.5,
        2.0,
        -2.0,
    ];
    let mut s: u32 = 0x7adf_0001;
    for case in 0..3000u32 {
        let x = probes
            .get(case as usize)
            .copied()
            .unwrap_or_else(|| fuzz(&mut s, 3.0));
        e.call(CLAMP01, 0, 0, &[x.to_bits()]);
        let bin = f32::from_bits(e.read_u32(G_CLAMP));
        assert_f32(bin, clamp01(x), &format!("clamp01 case={case} x={x:?}"));
    }
}

#[test]
fn camera_angle_cossin() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let cosout = e.alloc(4);
    let sinout = e.alloc(4);
    let ebp = frame + 0x80;
    let mut s: u32 = 0x7acc_0001;
    for case in 0..3000u32 {
        let angle = fuzz(&mut s, 7.0); // about ±2π
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.set_reg(RegisterX86::EAX, cosout);
        e.set_reg(RegisterX86::EDX, sinout);
        e.write_u32(ebp + 8, angle.to_bits());
        e.run(COSSIN_BEGIN, COSSIN_END);
        let (ocos, osin) = angle_cossin(angle);
        assert_f32(
            e.read_f32(cosout),
            ocos,
            &format!("angle_cos case={case} a={angle}"),
        );
        assert_f32(
            e.read_f32(sinout),
            osin,
            &format!("angle_sin case={case} a={angle}"),
        );
    }
}
