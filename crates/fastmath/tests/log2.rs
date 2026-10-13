//! `FastLog2` (`0x454c80`) run whole through `call_f64`, against [`log2`] with the tables and
//! constants read from the exe.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_fastmath::{log2, Log2Tables};

fn next(s: &mut u64) -> u64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *s
}

#[test]
fn log2_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let lut = emu.read(0x0080_2be0, 512); // 64 qwords
    let corr = emu.read(0x0080_2de0, 512);
    let rd64 =
        |emu: &Emu, addr: u64| -> f64 { f64::from_le_bytes(emu.read(addr, 8).try_into().unwrap()) };
    let coeffs = [
        rd64(&emu, 0x0080_2ff8), // c1 = 1/ln2
        rd64(&emu, 0x0080_3000), // c2
        rd64(&emu, 0x0080_3008), // c3
        rd64(&emu, 0x0080_3010), // c4
        rd64(&emu, 0x0080_3018), // c5
        rd64(&emu, 0x0080_3020), // c6
    ];
    let threshold = rd64(&emu, 0x0080_2ff0);
    let t = Log2Tables {
        lut: &lut,
        corr: &corr,
        coeffs,
        threshold,
    };

    let run = |emu: &mut Emu, x: f64| -> f64 {
        let b = x.to_bits();
        emu.call_f64(
            0x0045_4c80,
            0,
            0,
            &[(b & 0xffff_ffff) as u32, (b >> 32) as u32],
        )
    };

    let mut total = 0u32;
    let mut neg = 0u32; // the -inf results
    let check = |emu: &mut Emu, x: f64, total: &mut u32, neg: &mut u32| {
        let got = run(emu, x);
        let want = log2(x, &t);
        assert_eq!(got.to_bits(), want.to_bits(), "x={x} ({:#x})", x.to_bits());
        *total += 1;
        if x.is_nan() || x <= 0.0 {
            *neg += 1;
        }
    };

    // Powers of 2, 1.0, the threshold, the -inf inputs, and values at a table bucket's edge.
    let probes: &[f64] = &[
        1.0,
        2.0,
        4.0,
        8.0,
        0.5,
        0.25,
        1024.0,
        1e300,
        1e-300, // normal
        3.0,
        7.0,
        1.5,
        1.999_999,
        1.000_001,
        0.999_999,
        1.333_333_333, // mid-range / buckets
        0.0,
        -0.0,
        -1.0,
        -1e300,
        1e-307,
        1e-308, // -inf paths (x <= 1e-307)
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::from_bits(0x3ff0_0000_0000_0001), // edges
    ];
    for &x in probes {
        check(&mut emu, x, &mut total, &mut neg);
    }
    // NaN compares unordered and gives -inf.
    check(&mut emu, f64::NAN, &mut total, &mut neg);

    // Random values over a wide exponent range, so every bucket is hit, one in 16 negated.
    let mut st = 0x4c80_0000_0000_0001u64;
    for _ in 0..20000 {
        let raw = next(&mut st);
        // A positive f64 in [2^-200, 2^200).
        let mantissa = raw & 0x000f_ffff_ffff_ffff;
        let expo = 0x33b + ((next(&mut st) >> 40) % 0x190);
        let mut x = f64::from_bits((expo << 52) | mantissa);
        if next(&mut st) & 0xf == 0 {
            x = -x;
        }
        check(&mut emu, x, &mut total, &mut neg);
    }
    eprintln!("log2_diff: {total} bit-exact ({neg} -inf paths: x<=0 / tiny / NaN)");
}
