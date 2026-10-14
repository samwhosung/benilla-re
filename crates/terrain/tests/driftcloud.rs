//! `SetDriftCloudCount` (`0x68e650`) whole, against `drift_cloud_count`: the count is poisoned at
//! `this + 0xfa00`, the function runs, and the stored integer is compared. The argument runs over
//! zero, both signs, the `__ftol` overflow near 2^63/4000, NaN, the infinities and a fuzz.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_terrain::driftcloud::drift_cloud_count;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

#[test]
fn drift_cloud_set_fa00_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x1_0000);

    let mut run = |arg: f32| {
        emu.write_u32(this + 0xfa00, 0xdead_beef);
        emu.call(0x68e650, this as u32, 0, &[arg.to_bits()]);
        let binary = emu.read_u32(this + 0xfa00) as i32;
        assert_eq!(
            binary,
            drift_cloud_count(arg),
            "arg={arg} ({:#010x})",
            arg.to_bits()
        );
    };

    for a in [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.25,
        -0.25,
        0.99999994,
        1000.0,
        -1000.0,
        2.305_843e15,
        2.4e15,
        f32::from_bits(0x5a0f_ffff),
        f32::from_bits(0x5a10_0000),
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
    ] {
        run(a);
    }
    let mut s = 0x6472_6966u32;
    for _ in 0..500 {
        run(f32::from_bits(next(&mut s)));
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(((m * 2.0 - 1.0) * 10_000.0) as f32);
    }
}
