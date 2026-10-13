//! `NormalizeAngle` (`0x7abbe0`) run whole through `call_f32`, whose `fstp dword` is the caller's
//! store. `0xcf5730` and `0xcf5734` are zero in the image until their static initializers run, so
//! the test runs `0x7ac5b0` and then `0x7ac5d0`, which divides by the first's result, and checks
//! the bits they store.

mod common;

use benilla_re_camera::normalize_fp::normalize_2pi;
use benilla_re_difftest::{Emu, Wow};
use common::{assert_f32, fuzz, next};

const NORMALIZE: u64 = 0x7ab_be0;
const INIT_TWO_PI: u64 = 0x7ac_5b0; // InitCameraTwoPi: [0xcf5734] = π + π
const INIT_RECIP: u64 = 0x7ac_5d0; // InitCameraInvTwoPi: [0xcf5730] = 1.0 / [0xcf5734]
const G_TWO_PI: u64 = 0x00cf_5734;
const G_RECIP: u64 = 0x00cf_5730;

/// Runs the two static initializers and checks the bits they store.
fn init_globals(e: &mut Emu) {
    e.call(INIT_TWO_PI, 0, 0, &[]); // first: the second divides by it
    e.call(INIT_RECIP, 0, 0, &[]);
    assert_eq!(
        e.read_u32(G_TWO_PI),
        0x40c9_0fdb,
        "init produced [0xcf5734]=2π bits"
    );
    assert_eq!(
        e.read_u32(G_RECIP),
        0x3e22_f983,
        "init produced [0xcf5730]=1/(2π) bits"
    );
}

#[test]
fn camera_normalize_2pi() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    init_globals(&mut e);

    let two_pi = f32::from_bits(0x40c9_0fdb);

    // The sign branch, whole turns where the truncation flips, NaN, the infinities, and the
    // smallest values either side of 0.
    let probes: &[f32] = &[
        0.0,
        -0.0,
        two_pi,
        -two_pi,
        2.0 * two_pi,
        -2.0 * two_pi,
        3.0 * two_pi,
        -3.0 * two_pi,
        std::f32::consts::PI,
        -std::f32::consts::PI,
        std::f32::consts::FRAC_PI_2,
        -std::f32::consts::FRAC_PI_2,
        f32::from_bits(0x0000_0001), // smallest positive subnormal
        f32::from_bits(0x8000_0001), // smallest negative subnormal
        f32::from_bits(0x3f80_0000), // 1.0
        f32::from_bits(0xbf80_0000), // -1.0
        100.0,
        -100.0,
        1.0e6,
        -1.0e6,
        // either side of a whole turn
        f32::from_bits(two_pi.to_bits() - 1),
        f32::from_bits(two_pi.to_bits() + 1),
        f32::from_bits((-two_pi).to_bits() - 1),
        f32::from_bits((-two_pi).to_bits() + 1),
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        // x/2π near and past ±2^31
        2.0e9,
        -2.0e9,
        1.0e10,
        -1.0e10,
    ];

    let mut count = 0u64;
    let mut s: u32 = 0x7abb_0001;
    for case in 0..40_000u32 {
        let x = if (case as usize) < probes.len() {
            probes[case as usize]
        } else {
            // One of four scales, chosen by the high bits.
            let band = (next(&mut s) >> 8) % 4;
            match band {
                0 => fuzz(&mut s, 7.0),   // about one turn
                1 => fuzz(&mut s, 100.0), // a few turns
                2 => fuzz(&mut s, 1.0e5), // many turns
                _ => fuzz(&mut s, 1.0e8), // very many, inside i32
            }
        };

        let bin = e.call_f32(NORMALIZE, 0, 0, &[x.to_bits()]);
        let ours = normalize_2pi(x);
        assert_f32(bin, ours, &format!("normalize_2pi case={case} x={x:?}"));
        count += 1;
    }
    eprintln!("camera_normalize_2pi: {count} bit-exact asserts");
}
