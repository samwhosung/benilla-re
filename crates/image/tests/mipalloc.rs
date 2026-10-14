//! Bit-exact tests of the BLP and mip allocation and layout functions against `WoW.exe`. The inputs
//! are synthesized in the emulator's scratch arena, the exe's function runs, and the populated
//! table, pixel or out-param region is compared byte for byte with the transcription. `SMemAlloc` is
//! stubbed to return a known scratch address, which makes the allocate-and-populate path
//! deterministic.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_image::mipalloc::{
    box_downsample, compute_mip_byte_size, format_translate, populate_mip_table,
};
use benilla_re_image::mipgeo::{level_count, mip_block_size};
use common::{fuzz_u32, next};

const ALLOC: u64 = 0x5a4af0; // (ecx=fmt, edx=w, [esp+4]=h, [esp+8]=file, [esp+0xc]=line) ret 0xc
const POPULATE: u64 = 0x5a4bb0; // (ecx=fmt, edx=w, [esp+4]=h, [esp+8]=table) ret 8
const BOX: u64 = 0x5a4c30; // __fastcall(ecx=dst, edx=dstW, [esp+4]=dstH, [esp+8]=src, [esp+0xc]=srcW, [esp+0x10]=srcH)
const FORMAT_LUT_FN: u64 = 0x5a80d0; // __fastcall(ecx=idx) -> u32
const COMPUTE: u64 = 0x5a8140; // thiscall(ecx=this{w@+0x10,h@+0x14}, [esp+4]=fmt, +8=level, +0xc=&size, +0x10=&row) ret 0x10
const SMEMALLOC: u64 = 0x6462e0; // SMemAlloc(size, file, line, flags) __stdcall ret 0x10

const FMTS: [u32; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 9];
const DIMS: [(u32, u32); 11] = [
    (1, 1),
    (2, 1),
    (1, 2),
    (256, 256),
    (4, 4),
    (5, 3),
    (8, 1),
    (64, 64),
    (2, 2),
    (7, 7),
    (512, 16),
];

// ---- 0x5a4af0 — alloc + populate the absolute-pointer mip table (SMemAlloc stubbed) -------------

#[test]
fn image_mip_alloc_populate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // One known scratch buffer is handed back by the SMemAlloc stub for every case.
    let base = e.alloc(0x4000);
    e.stub_alloc(SMEMALLOC, base, 0x1000, 4);
    for &fmt in &FMTS {
        for &(w, h) in &DIMS {
            let count = level_count(w, h);
            e.reset_bump(base);
            e.clear_calls();
            // sentinel the table region so we know the binary actually wrote it.
            e.write(base, &vec![0xCCu8; (count * 4) as usize]);
            e.call(ALLOC, fmt, w, &[h, 0xAABB_CCDD, 0x1234]);

            let got: Vec<u32> = (0..count)
                .map(|i| e.read_u32(base + u64::from(i) * 4))
                .collect();
            let expect = populate_mip_table(fmt, w, h, base as u32);
            assert_eq!(got, expect, "alloc table fmt={fmt} {w}x{h}");
            // the requested allocation size is mip_sum + count*4 = mip_block_size.
            assert_eq!(
                e.calls()[0].args[0],
                mip_block_size(fmt, w, h),
                "alloc size fmt={fmt} {w}x{h}"
            );
        }
    }
}

// ---- 0x5a4bb0 — populate an existing caller-supplied table (no alloc) --------------------------

#[test]
fn image_mip_populate_existing() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let table = e.alloc(0x4000);
    for &fmt in &FMTS {
        for &(w, h) in &DIMS {
            let count = level_count(w, h);
            e.write(table, &vec![0xCCu8; (count * 4) as usize]);
            e.call(POPULATE, fmt, w, &[h, table as u32]);

            let got: Vec<u32> = (0..count)
                .map(|i| e.read_u32(table + u64::from(i) * 4))
                .collect();
            let expect = populate_mip_table(fmt, w, h, table as u32);
            assert_eq!(got, expect, "populate table fmt={fmt} {w}x{h}");
        }
    }
}

// ---- 0x5a4c30 — alpha-weighted box-filter mip downsample of a BGRA image -----------------------

/// `alpha`: 0 = force every source alpha to 0 (the ΣA==0 plain-average path); 1 = force a non-zero
/// alpha (the ΣA≠0 alpha-weighted path); 2 = fully fuzzed bytes (mostly ΣA≠0).
fn check_box(
    e: &mut Emu,
    src_va: u64,
    dst_va: u64,
    dims: (u32, u32, u32, u32),
    alpha: u8,
    s: &mut u32,
) {
    let (dst_w, dst_h, bw, bh) = dims;
    let src_w = dst_w * bw;
    let src_h = dst_h * bh;
    let src_bytes = (src_w * src_h * 4) as usize;
    let dst_bytes = (dst_w * dst_h * 4) as usize;

    let mut src = vec![0u8; src_bytes];
    for (i, byte) in src.iter_mut().enumerate() {
        let v = (next(s) >> 16) as u8;
        *byte = if i % 4 == 3 {
            match alpha {
                0 => 0,
                1 => v | 0x80, // guarantee non-zero
                _ => v,
            }
        } else {
            v
        };
    }
    e.write(src_va, &src);
    let sentinel = vec![0x5Au8; dst_bytes];
    e.write(dst_va, &sentinel);

    e.call(
        BOX,
        dst_va as u32,
        dst_w,
        &[dst_h, src_va as u32, src_w, src_h],
    );
    let bin = e.read(dst_va, dst_bytes);

    let mut ours = sentinel.clone();
    box_downsample(&mut ours, dst_w, dst_h, &src, src_w, src_h);
    assert_eq!(
        bin, ours,
        "box dst={dst_w}x{dst_h} block={bw}x{bh} alpha={alpha}"
    );
}

#[test]
fn image_box_downsample() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let src_va = e.alloc(0x10000);
    let dst_va = e.alloc(0x10000);
    let mut s = 0x1357_9bdfu32;

    // deterministic boundary probes: 1x1 dst (whole image), 1x1 block (identity), odd ratios.
    let cases: &[(u32, u32, u32, u32)] = &[
        (1, 1, 1, 1),
        (1, 1, 2, 2),
        (1, 1, 4, 4),
        (1, 1, 3, 5),
        (2, 2, 2, 2),
        (2, 1, 3, 1),
        (3, 3, 1, 1),
        (2, 2, 3, 3),
        (4, 2, 2, 3),
        (4, 4, 2, 2),
        (8, 8, 1, 1),
        (3, 2, 5, 1),
    ];
    for &(dw, dh, bw, bh) in cases {
        for alpha in [0u8, 1, 2] {
            check_box(&mut e, src_va, dst_va, (dw, dh, bw, bh), alpha, &mut s);
        }
    }

    // fuzz: random small dst dims and block ratios, all three alpha regimes.
    for _ in 0..1500 {
        let dw = fuzz_u32(&mut s, 6) + 1;
        let dh = fuzz_u32(&mut s, 6) + 1;
        let bw = fuzz_u32(&mut s, 5) + 1;
        let bh = fuzz_u32(&mut s, 5) + 1;
        let alpha = (fuzz_u32(&mut s, 3)) as u8;
        check_box(&mut e, src_va, dst_va, (dw, dh, bw, bh), alpha, &mut s);
    }
}

// ---- 0x5a80d0 — BLP format-translate LUT (.data table at 0x85c8c8) -----------------------------

#[test]
fn image_format_lut() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // diff over the valid index range (both format enums span 0..=9). This also byte-verifies the
    // embedded FORMAT_LUT against the binary's .data bytes at 0x85c8c8.
    for idx in 0..=9u32 {
        assert_eq!(
            e.call(FORMAT_LUT_FN, idx, 0, &[]),
            format_translate(idx),
            "format LUT idx={idx}"
        );
    }
}

// ---- 0x5a8140 — BLP ComputeMipByteSize -------------------------------------------------------

#[test]
fn image_compute_mip_byte_size() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x20);
    let size = e.alloc(4);
    let row = e.alloc(4);
    for &(w, h) in &DIMS {
        e.write_u32(this + 0x10, w);
        e.write_u32(this + 0x14, h);
        for fmt in 0..=10u32 {
            for level in 0..8u32 {
                e.write_u32(size, 0xDEAD_BEEF);
                e.write_u32(row, 0xFEED_FACE);
                let ret = e.call(
                    COMPUTE,
                    this as u32,
                    0,
                    &[fmt, level, size as u32, row as u32],
                );
                let (esz, erow, eret) = compute_mip_byte_size(w, h, fmt, level);
                assert_eq!(ret, eret, "cms ret {w}x{h} fmt={fmt} l={level}");
                assert_eq!(
                    e.read_u32(size),
                    esz,
                    "cms size {w}x{h} fmt={fmt} l={level}"
                );
                assert_eq!(e.read_u32(row), erow, "cms row {w}x{h} fmt={fmt} l={level}");
            }
        }
    }
}
