//! `0x454d40` run whole through `call_f64`, against [`exp2`] with the table and constants read from
//! the exe: `0x8029e0`, `0x803028` to `0x803050`, 1023.0 at `0x803058`, `+inf` at `0x875e90` and
//! the `f32` `0.0` at `0x7ffd74`.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_fastmath::{exp2, Exp2Tables};

fn next(s: &mut u64) -> u64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *s
}

#[test]
fn exp2_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let lut = emu.read(0x0080_29e0, 512); // 64 qwords
    let rd64 =
        |emu: &Emu, addr: u64| -> f64 { f64::from_le_bytes(emu.read(addr, 8).try_into().unwrap()) };
    let coeffs = [
        rd64(&emu, 0x0080_3028), // c0
        rd64(&emu, 0x0080_3030), // c1
        rd64(&emu, 0x0080_3038), // c2
        rd64(&emu, 0x0080_3040), // c3
        rd64(&emu, 0x0080_3048), // c4
        rd64(&emu, 0x0080_3050), // c5
    ];
    let bias = rd64(&emu, 0x0080_3058); // 1023.0
    let inf = rd64(&emu, 0x0087_5e90);
    // 0x7ffd74 is read by `fld dword`, an f32.
    let underflow = f64::from(emu.read_f32(0x007f_fd74));
    let t = Exp2Tables {
        lut: &lut,
        coeffs,
        bias,
        inf,
        underflow,
    };

    let run = |emu: &mut Emu, x: f64| -> f64 {
        let b = x.to_bits();
        emu.call_f64(
            0x0045_4d40,
            0,
            0,
            &[(b & 0xffff_ffff) as u32, (b >> 32) as u32],
        )
    };

    let mut total = 0u32;
    let mut over = 0u32; // the +inf results
    let mut under = 0u32; // the 0.0 results
    let check = |emu: &mut Emu, x: f64, total: &mut u32, over: &mut u32, under: &mut u32| {
        let got = run(emu, x);
        let want = exp2(x, &t);
        assert_eq!(got.to_bits(), want.to_bits(), "x={x} ({:#x})", x.to_bits());
        *total += 1;
        if got.is_infinite() {
            *over += 1;
        }
        if got == 0.0 {
            *under += 1;
        }
    };

    // Integers, zero, fractions, exact table entries, negatives, and both range edges.
    let probes: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        2.0,
        3.0,
        10.0,
        -1.0,
        -2.0,
        -10.0,
        0.5,
        0.25,
        -0.5,
        0.123,
        1.5,
        -1.5,
        100.0,
        -100.0,
        // Multiples of 1/64 land on a table entry with rf = 0.
        1.0 / 64.0,
        32.0 / 64.0,
        63.0 / 64.0,
        5.0 + 17.0 / 64.0,
        // Overflow: in [1025,1026) the scale itself is +inf; from 1026 the range test gives +inf.
        1023.0,
        1024.0,
        1024.9999,
        1025.0,
        1025.5,
        1025.9999,
        1026.0,
        1100.0,
        1e300,
        // Underflow: n <= 0 from x <= -1023 gives 0.0.
        -1022.0,
        -1022.9999,
        -1023.0,
        -1023.0001,
        -1024.0,
        -1100.0,
        -1e300,
        // Fractions either side of a bucket edge.
        2.0 + 1.0 / 128.0,
        2.0 + 1.0 / 127.0,
    ];
    for &x in probes {
        check(&mut emu, x, &mut total, &mut over, &mut under);
    }

    // Random values in about [-1200, 1200), past both edges.
    let mut st = 0x4d40_0000_0000_0001u64;
    for _ in 0..40000 {
        let raw = next(&mut st);
        // A random magnitude and fraction.
        let bits = raw & 0x7fff_ffff_ffff_ffff;
        let frac = (bits as f64) / (i64::MAX as f64); // [0,1)
        let sign = if next(&mut st) & 1 == 0 { 1.0 } else { -1.0 };
        let mag = (next(&mut st) % 1200) as f64 + frac;
        let x = sign * mag;
        check(&mut emu, x, &mut total, &mut over, &mut under);
    }

    // A sweep of [-1030, 1030], fine enough to hit every bucket and n, and both edges.
    let mut x = -1030.0;
    while x <= 1030.0 {
        check(&mut emu, x, &mut total, &mut over, &mut under);
        x += 0.0007;
    }

    eprintln!("exp2_diff: {total} bit-exact ({over} overflow→+inf, {under} underflow→0.0)");
}
