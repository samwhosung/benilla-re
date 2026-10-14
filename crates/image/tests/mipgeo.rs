//! Bit-exact tests of the mip-geometry functions against `WoW.exe`. All are `__fastcall` or
//! stack-argument integer functions; `EAX` is the result.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_image::mipgeo::{bpp_by_format, level_count, level_size, mip_block_size, mip_sum};
use common::fuzz_u32;

const BPP: u64 = 0x5a49b0; // __fastcall(ecx=fmt)
const LEVEL_SIZE: u64 = 0x5a4a00; // __fastcall(ecx=l, edx=w, [esp+4]=h, [esp+8]=fmt) ret 8
const MIP_SUM: u64 = 0x5a4a80; // (ecx=count, edx=w, [esp+4]=h, [esp+8]=fmt) ret 8
const LEVEL_COUNT: u64 = 0x5a4ac0; // __fastcall(ecx=w, edx=h)
const BLOCK_SIZE: u64 = 0x5a4b80; // (ecx=fmt, edx=w, [esp+4]=h) ret 4

const FMTS: [u32; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 9];
const DIMS: [(u32, u32); 9] = [
    (1, 1),
    (256, 256),
    (4, 4),
    (5, 3),
    (8, 1),
    (64, 64),
    (2, 2),
    (7, 7),
    (512, 16),
];

#[test]
fn image_bpp_by_format() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for fmt in 0..=14u32 {
        assert_eq!(
            e.call(BPP, fmt, 0, &[]),
            bpp_by_format(fmt),
            "bpp fmt={fmt}"
        );
    }
}

#[test]
fn image_level_count() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for &(w, h) in &[
        (1, 1),
        (2, 1),
        (1, 2),
        (256, 256),
        (512, 128),
        (1024, 1),
        (3, 7),
        (1, 1),
    ] {
        assert_eq!(
            e.call(LEVEL_COUNT, w, h, &[]),
            level_count(w, h),
            "lc {w}x{h}"
        );
    }
    let mut s = 0x1234_5678u32;
    for _ in 0..3000 {
        let w = fuzz_u32(&mut s, 1025);
        let h = fuzz_u32(&mut s, 1025);
        assert_eq!(
            e.call(LEVEL_COUNT, w, h, &[]),
            level_count(w, h),
            "lc {w}x{h}"
        );
    }
}

#[test]
fn image_level_size() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for &fmt in &FMTS {
        for l in 0..13u32 {
            for &(w, h) in &DIMS {
                assert_eq!(
                    e.call(LEVEL_SIZE, l, w, &[h, fmt]),
                    level_size(l, w, h, fmt),
                    "ls l={l} {w}x{h} fmt={fmt}"
                );
            }
        }
    }
    let mut s = 0xfeed_0001u32;
    for _ in 0..4000 {
        let l = fuzz_u32(&mut s, 16);
        let w = fuzz_u32(&mut s, 1024) + 1;
        let h = fuzz_u32(&mut s, 1024) + 1;
        let fmt = FMTS[(fuzz_u32(&mut s, 100) as usize) % FMTS.len()];
        assert_eq!(
            e.call(LEVEL_SIZE, l, w, &[h, fmt]),
            level_size(l, w, h, fmt),
            "ls l={l} {w}x{h} fmt={fmt}"
        );
    }
}

#[test]
fn image_mip_sum() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for &fmt in &FMTS {
        for count in 0..12u32 {
            for &(w, h) in &DIMS {
                assert_eq!(
                    e.call(MIP_SUM, count, w, &[h, fmt]),
                    mip_sum(count, w, h, fmt),
                    "sum c={count} {w}x{h} fmt={fmt}"
                );
            }
        }
    }
}

#[test]
fn image_mip_block_size() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for &fmt in &FMTS {
        for &(w, h) in &DIMS {
            assert_eq!(
                e.call(BLOCK_SIZE, fmt, w, &[h]),
                mip_block_size(fmt, w, h),
                "block {w}x{h} fmt={fmt}"
            );
        }
    }
}
