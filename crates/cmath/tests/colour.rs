//! `RgbToHsv` (`0x7bbc80`), `HsvToRgb` (`0x7bbd60`) and the minor axis `0x7bf700` against the
//! exe, over every combination of a set of edge channels (both zeros, NaN, the infinities, ties,
//! the 360 wrap) and a fuzz. A NaN result matches any NaN.

use benilla_re_cmath::colour::{hsv_to_rgb, rgb_to_hsv};
use benilla_re_cmath::raymesh::minor_axis;
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

fn assert_bits(binary: f32, ours: f32, what: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// Channels for the colour conversions: both zeros, ties, a channel below and above 1, NaN and the
/// infinities.
const EDGE: [f32; 12] = [
    0.0,
    -0.0,
    0.25,
    0.5,
    1.0,
    -0.5,
    -1.0,
    2.0,
    1e-30,
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
];

fn triples(seed: u32, fuzz: usize, lo: f32, hi: f32) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for &a in &EDGE {
        for &b in &EDGE {
            for &c in &EDGE {
                out.push([a, b, c]);
            }
        }
    }
    let mut s = seed;
    for _ in 0..fuzz {
        out.push([
            frand(&mut s, lo, hi),
            frand(&mut s, lo, hi),
            frand(&mut s, lo, hi),
        ]);
    }
    out
}

fn write3(e: &mut Emu, va: u64, v: &[f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        e.write_u32(va + 4 * i as u64, c.to_bits());
    }
}

fn read3(e: &Emu, va: u64) -> [f32; 3] {
    [0, 1, 2].map(|i| e.read_f32(va + 4 * i))
}

/// `0x7bbc80` whole, `ecx` the colour and `edx` the output. Ties between channels, a zero value
/// (the `jnp` on the value), a zero saturation (the `jp`), a negative hue (the `jp` on the 360
/// add, which excludes −0 and NaN), and NaN channels run among the edge combinations.
#[test]
fn rgb_to_hsv_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rgb = e.alloc(0x10);
    let out = e.alloc(0x10);
    let mut cases = triples(0x7bbc_8001, 4000, -1.0, 1.0);
    let mut s = 0x7bbc_8002u32;
    for _ in 0..2000 {
        cases.push([
            frand(&mut s, 0.0, 1.0),
            frand(&mut s, 0.0, 1.0),
            frand(&mut s, 0.0, 1.0),
        ]);
    }
    for c in &cases {
        write3(&mut e, rgb, c);
        e.write(out, &[0xaa; 12]);
        e.call(0x7bbc80, rgb as u32, out as u32, &[]);
        let ours = rgb_to_hsv(c);
        let bin = read3(&e, out);
        for k in 0..3 {
            assert_bits(bin[k], ours[k], &format!("rgb_to_hsv {c:?} [{k}]"));
        }
    }
}

/// `0x7bbd60` whole, `ecx` the colour and `edx` the output. Hue 360 exactly and a step either
/// side, the sector boundaries at multiples of 60, saturation exactly 0 and −0, NaN, the infinities
/// and the cap at sector 5 run among the edge combinations and a fuzz of hues in [−100, 500].
#[test]
fn hsv_to_rgb_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let hsv = e.alloc(0x10);
    let out = e.alloc(0x10);
    let mut cases: Vec<[f32; 3]> = Vec::new();
    let hues = [
        0.0,
        -0.0,
        59.999996,
        60.0,
        60.000004,
        119.99999,
        120.0,
        180.0,
        240.0,
        300.0,
        359.99997,
        360.0,
        360.00003,
        -60.0,
        720.0,
        1e30,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    let unit = [
        0.0,
        -0.0,
        1e-30,
        0.25,
        0.5,
        1.0,
        2.0,
        -0.5,
        f32::NAN,
        f32::INFINITY,
    ];
    for &h in &hues {
        for &sa in &unit {
            for &v in &unit {
                cases.push([h, sa, v]);
            }
        }
    }
    let mut s = 0x7bbd_6001u32;
    for _ in 0..6000 {
        cases.push([
            frand(&mut s, -100.0, 500.0),
            frand(&mut s, 0.0, 1.0),
            frand(&mut s, 0.0, 1.0),
        ]);
    }
    for c in &cases {
        write3(&mut e, hsv, c);
        e.write(out, &[0xaa; 12]);
        e.call(0x7bbd60, hsv as u32, out as u32, &[]);
        let ours = hsv_to_rgb(c);
        let bin = read3(&e, out);
        for k in 0..3 {
            assert_bits(bin[k], ours[k], &format!("hsv_to_rgb {c:?} [{k}]"));
        }
    }
}

/// `0x7bf700` whole, `ecx` the vector, `eax` the index. Ties between magnitudes, the sign of
/// each component, a zero and NaN in each position run among the edge combinations.
#[test]
fn minor_axis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let v = e.alloc(0x10);
    for c in triples(0x7bf7_0001, 4000, -10.0, 10.0) {
        write3(&mut e, v, &c);
        let bin = e.call(0x7bf700, v as u32, 0, &[]);
        assert_eq!(bin as usize, minor_axis(&c), "minor_axis {c:?}");
    }
}
