//! `FillShadowRamp` (`0x6d81a0`) and `FillShadowGreyRamp` (`0x6d82d0`) whole, against
//! `shadow_ramp_pixel` and `shadow_ramp_gray_pixel`: each fills the ramp buffer at `0xce9e0c`,
//! which the test reads back and compares pixel for pixel. A width of 1 divides 0 by 0 and runs the
//! NaN path of every threshold compare.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::shadow_blob_fp::shadow_ramp_pixel;

/// The ramp buffer both functions write.
const RAMP: u64 = 0x00ce_9e0c;

/// `0x6d81a0` in mode 1 with `n` columns.
#[test]
fn sb_shadow_ramp_0x6d81a0_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out1 = e.alloc(4);
    let out2 = e.alloc(4);
    let gen = |e: &mut Emu, n: u32| -> Vec<u32> {
        // ecx = the mode (1), edx = n; the stack holds the count (1), a pad, a flag (0), a pad and
        // the two outputs.
        e.call(0x6d81a0, 1, n, &[1, 0, 0, 0, out1 as u32, out2 as u32]);
        let b = e.read(RAMP, (4 * n) as usize);
        (0..n as usize)
            .map(|k| u32::from_le_bytes(b[k * 4..k * 4 + 4].try_into().unwrap()))
            .collect()
    };
    // n = 1 (0/0, NaN) and every width to 64, which puts columns on and either side of 2 and 10.
    for n in 1..=64u32 {
        let bin = gen(&mut e, n);
        for i in 0..n {
            assert_eq!(
                bin[i as usize],
                shadow_ramp_pixel(i, n),
                "ramp n={n} i={i}: bin={:#010x} ours={:#010x}",
                bin[i as usize],
                shadow_ramp_pixel(i, n)
            );
        }
    }
}

/// `0x6d82d0` in mode 1 with `n` columns.
#[test]
fn sb_shadow_ramp_gray_0x6d82d0_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out1 = e.alloc(4);
    let out2 = e.alloc(4);
    let gen = |e: &mut Emu, n: u32| -> Vec<u32> {
        e.call(0x6d82d0, 1, n, &[1, 0, 0, 0, out1 as u32, out2 as u32]);
        let b = e.read(RAMP, (4 * n) as usize);
        (0..n as usize)
            .map(|k| u32::from_le_bytes(b[k * 4..k * 4 + 4].try_into().unwrap()))
            .collect()
    };
    for n in 1..=64u32 {
        let bin = gen(&mut e, n);
        for i in 0..n {
            assert_eq!(
                bin[i as usize],
                benilla_re_lighting::shadow_blob_fp::shadow_ramp_gray_pixel(i, n),
                "gray-ramp n={n} i={i}: bin={:#010x} ours={:#010x}",
                bin[i as usize],
                benilla_re_lighting::shadow_blob_fp::shadow_ramp_gray_pixel(i, n)
            );
        }
    }
}
