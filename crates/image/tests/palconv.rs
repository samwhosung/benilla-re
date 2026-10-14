//! Bit-exact tests of the BLP paletted-to-RGB and paletted-to-BGRA converters against `WoW.exe`.
//! Each converter is `__thiscall(ecx=this)` where `this` carries the palette at `+0x98` (256 x u32
//! BGRA) and `alphaDepth` at `+0xd`; it reads the index plane (and, where relevant, a trailing alpha
//! plane) from `src` and writes the device-format output into `dst`. The test synthesizes a fuzzed
//! palette, index and alpha planes and a fuzzed `dst` (the 2-bit-alpha plane of the last converter
//! preserves the untouched bits of `dst`, so its initial content is load-bearing: the exe and the
//! transcription start from the same bytes), runs the exe, reads `dst` back, and compares it
//! byte for byte with the transcription.
//!
//! ABIs (read from the `ret` operands): the count-based expanders are
//! `ecx=this, [+8]=dst, [+0xc]=src, [+0x10]=count` (`ret 0xc`); the Floyd-Steinberg converters are
//! `ecx=this, [+8]=dst, [+0xc]=src, [+0x10]=w, [+0x14]=h` (`ret 0x10`).

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_image::palconv;
use common::next;

const A8: u64 = 0x5a8790; // expand -> BGRA8888 + 8-bit alpha plane
const ADEPTH: u64 = 0x5a87d0; // expand -> BGRA8888 + alphaDepth expand
const FS444: u64 = 0x5a92d0; // Floyd-Steinberg -> RGB444
const FS555: u64 = 0x5a9580; // Floyd-Steinberg -> RGB555
const FS565: u64 = 0x5a9860; // Floyd-Steinberg -> RGB565
const FS9: u64 = 0x5a9a80; // Floyd-Steinberg -> RGB565 + 2-bit alpha plane

const MAX_AREA: usize = 4096; // 64x64 — caps every test dim
const THIS_LEN: u64 = 0x4c0;
const PAL_OFF: u64 = 0x98;
const ADEPTH_OFF: u64 = 0xd;

/// Count-based shapes (index-plane length) — includes the full 256-index sweep.
const COUNTS: [usize; 12] = [1, 2, 3, 4, 7, 8, 15, 16, 255, 256, 1000, 4096];
/// (w, h) shapes — exercises the FS two-row ping-pong (h ≥ 2), odd widths, and tall/wide strips.
const DIMS: [(usize, usize); 17] = [
    (1, 1),
    (2, 1),
    (1, 2),
    (2, 2),
    (3, 3),
    (4, 4),
    (5, 7),
    (8, 8),
    (16, 3),
    (3, 16),
    (17, 13),
    (33, 9),
    (9, 33),
    (64, 1),
    (1, 64),
    (64, 64),
    (40, 40),
];

struct Ctx<'a> {
    e: Emu<'a>,
    this: u64,
    src_va: u64,
    dst_va: u64,
}

impl<'a> Ctx<'a> {
    fn new(wow: &'a Wow) -> Ctx<'a> {
        let mut e = Emu::new(wow);
        let this = e.alloc(THIS_LEN);
        let src_va = e.alloc(2 * MAX_AREA as u64); // index plane + alpha plane
        let dst_va = e.alloc(MAX_AREA as u64 * 4); // BGRA8888 worst case
        Ctx {
            e,
            this,
            src_va,
            dst_va,
        }
    }

    /// Run one case: write the inputs, invoke the binary, return its `dst[..out_len]`.
    fn run(
        &mut self,
        pal: &[u8],
        adepth: u8,
        src: &[u8],
        dst_init: &[u8],
        fn_va: u64,
        args: &[u32],
    ) -> Vec<u8> {
        self.e.write(self.this + PAL_OFF, pal);
        self.e.write(self.this + ADEPTH_OFF, &[adepth]);
        self.e.write(self.src_va, src);
        self.e.write(self.dst_va, dst_init);
        self.e.call(fn_va, self.this as u32, 0, args);
        self.e.read(self.dst_va, dst_init.len())
    }
}

fn fuzz(s: &mut u32, n: usize) -> Vec<u8> {
    (0..n).map(|_| (next(s) >> 16) as u8).collect()
}

/// A fuzzed 256×u32 BGRA palette (1024 bytes).
fn palette(s: &mut u32) -> Vec<u8> {
    fuzz(s, 1024)
}

/// Index plane: the 256-sweep for count==256, otherwise fuzzed indices.
fn index_plane(s: &mut u32, count: usize) -> Vec<u8> {
    if count == 256 {
        (0..256).map(|i| i as u8).collect()
    } else {
        fuzz(s, count)
    }
}

// ------------------------------------------------------------------------------------------------
// Palette-expand → BGRA8888
// ------------------------------------------------------------------------------------------------

#[test]
fn image_palconv_expand_bgra8888_a8() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut c = Ctx::new(&wow);
    let mut s = 0x5a87_9001u32;
    for &count in &COUNTS {
        let pal = palette(&mut s);
        let mut src = index_plane(&mut s, count);
        src.extend(fuzz(&mut s, count)); // 8-bit alpha plane
        let dst_init = fuzz(&mut s, count * 4);
        let mut ours = dst_init.clone();
        palconv::expand_bgra8888_a8(&pal[..], &mut ours, &src, count);
        let bin = c.run(
            &pal,
            0,
            &src,
            &dst_init,
            A8,
            &[c.dst_va as u32, c.src_va as u32, count as u32],
        );
        assert_eq!(bin, ours, "expand_a8 count={count}");
    }
}

#[test]
fn image_palconv_expand_bgra8888_adepth() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut c = Ctx::new(&wow);
    let mut s = 0x5a87_d001u32;
    for &adepth in &[0u8, 1, 2, 4, 8] {
        for &count in &COUNTS {
            let pal = palette(&mut s);
            let mut src = index_plane(&mut s, count);
            src.extend(fuzz(&mut s, count)); // alpha plane (>= any depth needs)
            let dst_init = fuzz(&mut s, count * 4);
            let mut ours = dst_init.clone();
            palconv::expand_bgra8888_adepth(&pal[..], adepth, &mut ours, &src, count);
            let bin = c.run(
                &pal,
                adepth,
                &src,
                &dst_init,
                ADEPTH,
                &[c.dst_va as u32, c.src_va as u32, count as u32],
            );
            assert_eq!(bin, ours, "expand_adepth d={adepth} count={count}");
        }
    }
}

// ------------------------------------------------------------------------------------------------
// Floyd-Steinberg converters (w/h-based)
// ------------------------------------------------------------------------------------------------

#[test]
fn image_palconv_fs_rgb444() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut c = Ctx::new(&wow);
    let mut s = 0x5a92_d001u32;
    for &adepth in &[0u8, 1, 4, 8] {
        for &(w, h) in &DIMS {
            let count = w * h;
            let pal = palette(&mut s);
            let mut src = fuzz(&mut s, count);
            src.extend(fuzz(&mut s, count));
            let dst_init = fuzz(&mut s, count * 2);
            let mut ours = dst_init.clone();
            palconv::fs_rgb444(&pal[..], adepth, &mut ours, &src, w, h);
            let bin = c.run(
                &pal,
                adepth,
                &src,
                &dst_init,
                FS444,
                &[c.dst_va as u32, c.src_va as u32, w as u32, h as u32],
            );
            assert_eq!(bin, ours, "fs444 d={adepth} {w}x{h}");
        }
    }
}

#[test]
fn image_palconv_fs_rgb555() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut c = Ctx::new(&wow);
    let mut s = 0x5a95_8001u32;
    for &adepth in &[0u8, 1, 4, 8] {
        for &(w, h) in &DIMS {
            let count = w * h;
            let pal = palette(&mut s);
            let mut src = fuzz(&mut s, count);
            src.extend(fuzz(&mut s, count));
            let dst_init = fuzz(&mut s, count * 2);
            let mut ours = dst_init.clone();
            palconv::fs_rgb555(&pal[..], adepth, &mut ours, &src, w, h);
            let bin = c.run(
                &pal,
                adepth,
                &src,
                &dst_init,
                FS555,
                &[c.dst_va as u32, c.src_va as u32, w as u32, h as u32],
            );
            assert_eq!(bin, ours, "fs555 d={adepth} {w}x{h}");
        }
    }
}

#[test]
fn image_palconv_fs_rgb565() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut c = Ctx::new(&wow);
    let mut s = 0x5a98_6001u32;
    for &(w, h) in &DIMS {
        let count = w * h;
        let pal = palette(&mut s);
        let src = fuzz(&mut s, count);
        let dst_init = fuzz(&mut s, count * 2);
        let mut ours = dst_init.clone();
        palconv::fs_rgb565(&pal[..], &mut ours, &src, w, h);
        let bin = c.run(
            &pal,
            0,
            &src,
            &dst_init,
            FS565,
            &[c.dst_va as u32, c.src_va as u32, w as u32, h as u32],
        );
        assert_eq!(bin, ours, "fs565 {w}x{h}");
    }
}

#[test]
fn image_palconv_fs_rgb565_a2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut c = Ctx::new(&wow);
    let mut s = 0x5a9a_8001u32;
    for &adepth in &[0u8, 1, 4, 8] {
        for &(w, h) in &DIMS {
            let count = w * h;
            // color plane (count*2 bytes) + 2-bit alpha plane (ceil(count/4) bytes)
            let out_len = count * 2 + count.div_ceil(4);
            let pal = palette(&mut s);
            let mut src = fuzz(&mut s, count);
            src.extend(fuzz(&mut s, count));
            let dst_init = fuzz(&mut s, out_len);
            let mut ours = dst_init.clone();
            palconv::fs_rgb565_a2(&pal[..], adepth, &mut ours, &src, w, h);
            let bin = c.run(
                &pal,
                adepth,
                &src,
                &dst_init,
                FS9,
                &[c.dst_va as u32, c.src_va as u32, w as u32, h as u32],
            );
            assert_eq!(bin, ours, "fs9 d={adepth} {w}x{h}");
        }
    }
}
