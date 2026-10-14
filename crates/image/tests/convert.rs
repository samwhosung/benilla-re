//! Bit-exact test of the BGRA8888-to-RGB565 converter against `WoW.exe`. The dims, source and
//! destination are allocated in the emulator's scratch arena, a fuzzed BGRA source is written, the
//! exe's function runs, and its output buffer is compared with the transcription over the full
//! written region, including the unrolled path's odd-width extra pixel.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_image::convert::bgra8888_to_rgb565;
use common::next;

const RGB565: u64 = 0x5a5330; // __fastcall(ecx=&dims{w,h}, edx=src, [esp+4]=srcStride, [esp+8]=dst, [esp+0xc]=dstStride)

fn packed_width(w: u32) -> u32 {
    if w < 2 {
        w
    } else {
        (w + 1) & !1
    }
}

fn check_565(wow: &Wow, w: u32, h: u32, s: &mut u32) {
    let pw = packed_width(w) as usize;
    let src_stride = pw * 4;
    let dst_stride = pw * 2;
    let src_bytes = src_stride * h as usize;
    let dst_bytes = dst_stride * h as usize;

    let mut e = Emu::new(wow); // fresh arena per case
    let dims = e.alloc(8);
    e.write_u32(dims, w);
    e.write_u32(dims + 4, h);
    let src_va = e.alloc(src_bytes.max(4) as u64);
    let dst_va = e.alloc(dst_bytes.max(4) as u64);

    let mut src = vec![0u8; src_bytes];
    for b in src.iter_mut() {
        *b = (next(s) >> 16) as u8;
    }
    e.write(src_va, &src);
    let sentinel = vec![0xAAu8; dst_bytes];
    e.write(dst_va, &sentinel);

    e.call(
        RGB565,
        dims as u32,
        src_va as u32,
        &[src_stride as u32, dst_va as u32, dst_stride as u32],
    );
    let bin = e.read(dst_va, dst_bytes);

    let mut ours = sentinel.clone();
    bgra8888_to_rgb565(w, h, &src, src_stride, &mut ours, dst_stride);
    assert_eq!(bin, ours, "rgb565 {w}x{h}");
}

#[test]
fn image_bgra8888_to_rgb565() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0xabcd_0001u32;
    // exercise simple path (w<2), unrolled even, unrolled odd (+ the rounded-up extra pixel), multi-row.
    for &(w, h) in &[
        (1, 1),
        (2, 1),
        (3, 1),
        (2, 2),
        (4, 4),
        (5, 3),
        (7, 2),
        (1, 4),
        (8, 8),
        (2, 4),
        (6, 1),
        (9, 5),
    ] {
        check_565(&wow, w, h, &mut s);
    }
}
