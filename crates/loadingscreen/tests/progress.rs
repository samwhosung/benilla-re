//! The progress fraction (`0x406920`) against the exe, whole function: the three progress globals
//! are set and the bar fill read back, over both the unphased and phased branches and the clamp to
//! `[0, 1]`.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_loadingscreen::progress::progress_fraction;
use common::{assert_f32, fuzz, g, next};

const PROGRESS_FRACTION: u64 = 0x406920; // void(void)

#[test]
fn progress_fraction_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s: u32 = 0x4069_0001;
    for i in 0..8000u32 {
        // phase nonzero half the time (high bit); inputs span the clamp range [-0.5, 1.5)
        let phase = if (next(&mut s) >> 8) & 1 == 0 {
            0.0
        } else {
            fuzz(&mut s, 1.5)
        };
        let a = fuzz(&mut s, 1.5) + 0.5;
        let b = fuzz(&mut s, 1.5) + 0.5;
        e.write_u32(g::PROGRESS_PHASE, phase.to_bits());
        e.write_u32(g::PROGRESS_A, a.to_bits());
        e.write_u32(g::PROGRESS_B, b.to_bits());
        e.write_u32(g::BAR_FILL, 0xdead_beef);
        e.call(PROGRESS_FRACTION, 0, 0, &[]);
        assert_f32(
            e.read_f32(g::BAR_FILL),
            progress_fraction(phase, a, b),
            &format!("progress_fraction[{i}] phase={phase} a={a} b={b}"),
        );
    }
}

/// -0.0 and NaN: a sum that is exactly -0 stays -0, and a NaN gives 1.0. The -0 sum takes a phase
/// of -2^-149 (its half rounds to -0 in the `f32` store) with both inputs -0.
#[test]
fn progress_fraction_signed_zero_and_nan_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let tiny = f32::from_bits(0x8000_0001);
    let cases = [
        (tiny, -0.0, -0.0),
        (0.0, -0.0, -0.0),
        (-0.0, -0.0, -0.0),
        (0.0, f32::NAN, 0.0),
        (0.0, 0.0, f32::NAN),
        (tiny, f32::NAN, f32::NAN),
        (0.0, f32::INFINITY, f32::NEG_INFINITY),
        (0.0, f32::NEG_INFINITY, 0.0),
        (0.0, 1.0, 1.0),
    ];
    for (phase, a, b) in cases {
        e.write_u32(g::PROGRESS_PHASE, phase.to_bits());
        e.write_u32(g::PROGRESS_A, a.to_bits());
        e.write_u32(g::PROGRESS_B, b.to_bits());
        e.write_u32(g::BAR_FILL, 0xdead_beef);
        e.call(PROGRESS_FRACTION, 0, 0, &[]);
        assert_f32(
            e.read_f32(g::BAR_FILL),
            progress_fraction(phase, a, b),
            &format!("phase={phase} a={a} b={b}"),
        );
    }
}
