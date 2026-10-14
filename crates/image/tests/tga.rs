//! Bit-exact tests of the TGA codec against `WoW.exe`. The struct-touching functions are tested by
//! synthesizing a TGA CImage header (raw bytes) and the pixel buffers, and stubbing the host calls
//! (`SMemAlloc` to a scratch bump, `SMemFree` and `SErrSetLastError` to no-ops; `memcpy`, `SStrCopy`
//! and the guarded accessor run natively). Each case writes the inputs, runs the exe, reads back the
//! destination buffer and the mutated header, and compares them byte for byte with the transcription
//! on the same input.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_image::tga::{
    alpha_strip, append_alpha, flip, init_from_source, palette_expand, pixel_offset, rle_decode,
};
use common::{fuzz_u32, next};

const PIXEL_OFFSET: u64 = 0x5a3b40; // __fastcall(ecx=struct)
const PALETTE_EXPAND: u64 = 0x5a3820; // __thiscall(ecx=struct, [esp+4]=flags) ret 4
const APPEND_ALPHA: u64 = 0x5a3b80; // __thiscall(ecx=struct, dst, src, alpha) ret 0xc
const RLE_DECODE: u64 = 0x5a3ed0; // __thiscall(ecx=struct, src, dst) ret 8
const FLIP: u64 = 0x5a4070; // __thiscall(ecx=struct, target) ret 4
const ALPHA_STRIP: u64 = 0x5a4250; // __thiscall(ecx=struct) ret
const INIT_FROM_SOURCE: u64 = 0x5a43c0; // __thiscall(ecx=struct, src,w,h,bpp,attr,flag5,flag4) ret 0x1c

const SMEM_ALLOC: u64 = 0x6462e0; // ret 0x10 (size,file,line,flags)
const SMEM_FREE: u64 = 0x646430; // ret 0x10 (ptr,file,line,flags)
const SERR: u64 = 0x64e850; // ret 4 (code)

const HDR_LEN: usize = 0x80;

fn set16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
fn set32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

/// Header compare with the env-dependent heap-pointer fields masked: `+0x00` SFile, `+0x04` pixels,
/// `+0x1c` idBuf (the binary writes a fresh `SMemAlloc` VA into `+0x04` that the transcription —
/// which takes the buffer as a slice — never reproduces).
fn hdr_eq(emu: &[u8], mine: &[u8]) -> bool {
    let mut a = emu.to_vec();
    let mut b = mine.to_vec();
    for o in [0usize, 1, 2, 3, 4, 5, 6, 7, 0x1c, 0x1d, 0x1e, 0x1f] {
        a[o] = 0;
        b[o] = 0;
    }
    a == b
}

fn bpx_of(bpp: u8) -> usize {
    ((u32::from(bpp) + 7) >> 3) as usize
}

// ---------------------------------------------------------------------------------------------
// 0x5a3b40 — pixel-data file offset (pure arithmetic on header fields)
// ---------------------------------------------------------------------------------------------
#[test]
fn image_tga_pixel_offset() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let hv = e.alloc(HDR_LEN as u64);
    let run = |e: &mut Emu, es: u8, len: u16, idl: u8| -> u32 {
        let mut h = vec![0u8; HDR_LEN];
        h[0x0f] = es;
        set16(&mut h, 0x0d, len);
        h[0x08] = idl;
        e.write(hv, &h);
        e.call(PIXEL_OFFSET, hv as u32, 0, &[])
    };
    for &(es, len, idl) in &[
        (24u8, 256u16, 0u8),
        (32, 256, 18),
        (16, 128, 5),
        (8, 16, 0),
        (24, 0, 0),
        (15, 300, 255),
        (0, 10, 1),
        (255, 65535, 255),
        (12, 64, 7),
        (23, 1, 1),
    ] {
        assert_eq!(
            run(&mut e, es, len, idl),
            pixel_offset(es, len, idl),
            "off es={es} len={len} idl={idl}"
        );
    }
    let mut s = 0xabcd_0001u32;
    for _ in 0..3000 {
        let es = fuzz_u32(&mut s, 256) as u8;
        let len = fuzz_u32(&mut s, 65536) as u16;
        let idl = fuzz_u32(&mut s, 256) as u8;
        assert_eq!(
            run(&mut e, es, len, idl),
            pixel_offset(es, len, idl),
            "fuzz es={es} len={len} idl={idl}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 0x5a3ed0 — RLE decode loop
// ---------------------------------------------------------------------------------------------

/// Build a valid RLE-compressed stream decoding to exactly `wh` pixels (mixed run/raw packets).
fn build_rle_stream(wh: usize, bpx: usize, s: &mut u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut remaining = wh;
    let mut toggle = 0u32;
    while remaining > 0 {
        toggle += 1;
        let mut cnt = 1 + (next(s) >> 20) as usize % 128;
        if cnt > remaining {
            cnt = remaining;
        }
        if toggle.is_multiple_of(2) {
            // run packet: one source pixel repeated `cnt` times
            out.push(0x80 | ((cnt - 1) as u8));
            for _ in 0..bpx {
                out.push((next(s) >> 16) as u8);
            }
        } else {
            // raw packet: `cnt` literal pixels
            out.push((cnt - 1) as u8);
            for _ in 0..cnt * bpx {
                out.push((next(s) >> 16) as u8);
            }
        }
        remaining -= cnt;
    }
    out
}

#[test]
fn image_tga_rle_decode() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(SERR, 0, 1); // the overrun error path calls SErrSetLastError
    let hv = e.alloc(HDR_LEN as u64);
    let sv = e.alloc(0x8000);
    let dv = e.alloc(0x8000);
    let mut s = 0x1357_9bdfu32;

    let check =
        |e: &mut Emu, w: u16, h: u16, bpp: u8, itype: u8, stream: &[u8], slack_px: usize| {
            let bpx = bpx_of(bpp);
            let wh = w as usize * h as usize;
            let dst_bytes = ((wh + slack_px) * bpx).max(4);
            // src buffer
            let mut sbuf = vec![0u8; 0x8000];
            sbuf[..stream.len()].copy_from_slice(stream);
            e.write(sv, &sbuf);
            let sentinel = vec![0x5Au8; dst_bytes];
            e.write(dv, &sentinel);

            let mut hh = vec![0u8; HDR_LEN];
            set16(&mut hh, 0x14, w);
            set16(&mut hh, 0x16, h);
            hh[0x18] = bpp;
            hh[0x0a] = itype;
            e.write(hv, &hh);

            let bin_ret = e.call(RLE_DECODE, hv as u32, 0, &[sv as u32, dv as u32]);
            let bin_dst = e.read(dv, dst_bytes);
            let bin_hdr = e.read(hv, HDR_LEN);

            let mut mine_hdr = hh.clone();
            let mut my_dst = sentinel.clone();
            let my_ret = rle_decode(&mut mine_hdr, stream, &mut my_dst);

            assert_eq!(
                bin_ret, my_ret,
                "rle_decode ret {w}x{h} bpp={bpp} it={itype}"
            );
            assert_eq!(
                bin_dst, my_dst,
                "rle_decode dst {w}x{h} bpp={bpp} it={itype}"
            );
            assert!(
                hdr_eq(&bin_hdr, &mine_hdr),
                "rle_decode hdr {w}x{h} bpp={bpp} it={itype}"
            );
        };

    // boundary / branch cases (success): empty, 1px, run, raw, mixed, all imageType variants.
    for &(w, h, bpp, it) in &[
        (0u16, 0u16, 32u8, 10u8),
        (1, 1, 32, 10),
        (2, 1, 24, 10),
        (4, 4, 32, 9),
        (5, 3, 16, 11),
        (7, 2, 8, 10),
        (8, 1, 24, 10),
        (16, 16, 32, 10),
        (3, 5, 8, 9),
    ] {
        let bpx = bpx_of(bpp);
        let wh = w as usize * h as usize;
        let stream = build_rle_stream(wh, bpx, &mut s);
        check(&mut e, w, h, bpp, it, &stream, 0);
    }

    // explicit max-run boundary: a single run packet of exactly 128 px (b = 0xFF), w*h = 128.
    {
        let bpx = 4usize;
        let mut stream = vec![0xFFu8];
        stream.resize(stream.len() + bpx, 0xAB);
        check(&mut e, 128, 1, 32, 10, &stream, 0);
    }
    // explicit max raw packet of exactly 128 literal px (b = 0x7F).
    {
        let bpx = 4usize;
        let mut stream = vec![0x7Fu8];
        for i in 0..128 * bpx {
            stream.push((i * 7 + 3) as u8);
        }
        check(&mut e, 128, 1, 32, 10, &stream, 0);
    }
    // overshoot/error: run of 8 into a 4-pixel budget — both write 8px then return 0.
    {
        let bpx = 4usize;
        let mut stream = vec![0x80u8 | 7];
        stream.resize(stream.len() + bpx, 0xCD);
        check(&mut e, 4, 1, 32, 10, &stream, 128);
    }

    // fuzz: random valid streams over random dims.
    for _ in 0..200 {
        let w = (fuzz_u32(&mut s, 12) + 1) as u16;
        let h = (fuzz_u32(&mut s, 12) + 1) as u16;
        let bpp = [8u8, 16, 24, 32][fuzz_u32(&mut s, 4) as usize];
        let it = [9u8, 10, 11][fuzz_u32(&mut s, 3) as usize];
        let bpx = bpx_of(bpp);
        let wh = w as usize * h as usize;
        let stream = build_rle_stream(wh, bpx, &mut s);
        check(&mut e, w, h, bpp, it, &stream, 0);
    }
}

// ---------------------------------------------------------------------------------------------
// 0x5a3b80 — append-alpha interleave (24→32; opaque-pad or alpha-source)
// ---------------------------------------------------------------------------------------------
#[test]
fn image_tga_append_alpha() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let hv = e.alloc(HDR_LEN as u64);
    let dstv = e.alloc(0x4000);
    let srcv = e.alloc(0x4000);
    let alphav = e.alloc(0x2000);

    for &(w, h, bpp, desc) in &[
        (2u16, 2u16, 24u8, 0u8),
        (4, 4, 24, 0),
        (1, 1, 24, 0),
        (3, 3, 24, 8),
        (2, 2, 16, 0),
        (5, 1, 8, 0),
        (0, 0, 24, 0),
        (4, 4, 32, 8),
        (6, 2, 24, 4),
    ] {
        let bpx = bpx_of(bpp);
        let wh = w as usize * h as usize;
        let dst_bytes = (wh * (bpx + 1)).max(4);
        let mut src = vec![0u8; (wh * bpx).max(4)];
        for (i, b) in src.iter_mut().enumerate() {
            *b = (i as u32).wrapping_mul(101).wrapping_add(7) as u8;
        }
        let mut alpha = vec![0u8; wh.max(1)];
        for (i, b) in alpha.iter_mut().enumerate() {
            *b = (i as u32).wrapping_mul(53).wrapping_add(0x20) as u8;
        }

        for use_alpha in [false, true] {
            let sentinel = vec![0xAAu8; dst_bytes];
            e.write(dstv, &sentinel);
            e.write(srcv, &src);
            e.write(alphav, &alpha);
            let mut hh = vec![0u8; HDR_LEN];
            set16(&mut hh, 0x14, w);
            set16(&mut hh, 0x16, h);
            hh[0x18] = bpp;
            hh[0x19] = desc;
            e.write(hv, &hh);

            let alpha_arg = if use_alpha { alphav as u32 } else { 0 };
            e.call(
                APPEND_ALPHA,
                hv as u32,
                0,
                &[dstv as u32, srcv as u32, alpha_arg],
            );
            let bin_dst = e.read(dstv, dst_bytes);
            let bin_hdr = e.read(hv, HDR_LEN);

            let mut mine_hdr = hh.clone();
            let mut my_dst = sentinel.clone();
            let alpha_opt = if use_alpha {
                Some(alpha.as_slice())
            } else {
                None
            };
            append_alpha(&mut mine_hdr, &mut my_dst, &src, alpha_opt);

            assert_eq!(
                bin_dst, my_dst,
                "append_alpha dst {w}x{h} bpp={bpp} desc={desc} a={use_alpha}"
            );
            assert!(
                hdr_eq(&bin_hdr, &mine_hdr),
                "append_alpha hdr {w}x{h} bpp={bpp} desc={desc} a={use_alpha}"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 0x5a3820 — paletted→RGB expand (optional appended alpha plane)
// ---------------------------------------------------------------------------------------------
#[test]
fn image_tga_palette_expand() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let hv = e.alloc(HDR_LEN as u64);
    let idxv = e.alloc(0x4000); // src palette indices (old pixels)
    let palv = e.alloc(0x4000); // colormap
    let allocv = e.alloc(0x8000); // SMemAlloc destination
    e.stub_alloc(SMEM_ALLOC, allocv, 0x8000, 4);
    e.stub(SMEM_FREE, 0, 4);
    e.stub(SERR, 0, 1);

    // (width, height, descriptor, colorMapEntrySize, flags)
    for &(w, h, desc, entry_size, flags) in &[
        (4u16, 4u16, 0u8, 24u8, 0u32),
        (4, 4, 0, 24, 1),
        (1, 1, 0, 24, 0),
        (2, 3, 0, 24, 1),
        (8, 8, 0, 24, 0),
        (5, 1, 0, 16, 0), // entryBytes = 1 (partial-fill of the 3-byte alloc; sentinel tail)
        (4, 4, 8, 24, 0), // attr=8 → bpp 32, 3-byte writes into a 4-byte alloc
        (0, 0, 0, 24, 1), // empty (alpha path still rewrites header)
        (3, 2, 0, 32, 0), // entryBytes = 3 (32-bit palette caps at 3)
    ] {
        let entry_bytes = ((((u32::from(entry_size) / 3).min(8)) * 3) >> 3) as usize;
        let attr_bpp = (u32::from(desc & 0xf)) + 0x18;
        let wh = w as usize * h as usize;
        let alloc_size = (((attr_bpp >> 3) + (flags & 1)) as usize * wh).max(4);

        // palette indices, first index 0, values 0..256
        let mut idx = vec![0u8; wh.max(1)];
        let mut s = 0x99u32;
        for b in idx.iter_mut() {
            *b = (next(&mut s) >> 16) as u8;
        }
        // colormap: enough for index 255 * entry_bytes (+ slack)
        let mut pal = vec![0u8; 256 * entry_bytes.max(1) + 16];
        for (i, b) in pal.iter_mut().enumerate() {
            *b = (i as u32).wrapping_mul(31).wrapping_add(5) as u8;
        }

        e.write(idxv, &idx);
        e.write(palv, &pal);
        let sentinel = vec![0xC3u8; alloc_size];
        e.write(allocv, &sentinel);
        e.reset_bump(allocv);

        let mut hh = vec![0u8; HDR_LEN];
        set32(&mut hh, 0x04, idxv as u32); // pixels = src indices
        set32(&mut hh, 0x40, palv as u32); // palette
        set16(&mut hh, 0x14, w);
        set16(&mut hh, 0x16, h);
        hh[0x19] = desc;
        hh[0x0f] = entry_size;
        set16(&mut hh, 0x0b, 0); // colorMapFirstIndex = 0
        e.write(hv, &hh);

        e.call(PALETTE_EXPAND, hv as u32, 0, &[flags]);
        let bin_out = e.read(allocv, alloc_size);
        let bin_hdr = e.read(hv, HDR_LEN);

        let mut mine_hdr = hh.clone();
        let mut my_out = sentinel.clone();
        palette_expand(&mut mine_hdr, &idx, &pal, flags, &mut my_out);

        assert_eq!(
            bin_out, my_out,
            "expand out {w}x{h} desc={desc} es={entry_size} f={flags}"
        );
        assert!(
            hdr_eq(&bin_hdr, &mine_hdr),
            "expand hdr {w}x{h} desc={desc} es={entry_size} f={flags}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 0x5a4070 — vertical flip / set-origin
// ---------------------------------------------------------------------------------------------
#[test]
fn image_tga_flip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let hv = e.alloc(HDR_LEN as u64);
    let srcv = e.alloc(0x8000);
    let allocv = e.alloc(0x8000);
    e.stub_alloc(SMEM_ALLOC, allocv, 0x8000, 4);
    e.stub(SMEM_FREE, 0, 4);
    e.stub(SERR, 0, 1);

    // (width, height, bpp, descriptor, imageType, target)
    for &(w, h, bpp, desc, itype, target) in &[
        (4u16, 4u16, 32u8, 0u8, 2u8, 1u32), // flip (bit5 0 -> want 1)
        (4, 4, 32, 0x20, 2, 0),             // flip (bit5 1 -> want 0)
        (3, 5, 24, 0, 2, 1),                // flip, odd dims
        (1, 1, 32, 0, 2, 1),                // flip, single px
        (4, 4, 32, 0, 2, 0),                // no-op (bit5 0, want 0)
        (4, 4, 32, 0x20, 2, 5),             // no-op (bit5 1, want !=0)
        (4, 4, 32, 0, 10, 1),               // error (RLE imageType)
        (8, 2, 16, 0x20, 2, 0),             // flip
        (2, 6, 8, 0, 2, 1),                 // flip
    ] {
        let bpx = bpx_of(bpp);
        let stride = bpx * w as usize;
        let total = stride * h as usize;
        let mut src = vec![0u8; total.max(4)];
        for (i, b) in src.iter_mut().enumerate() {
            *b = (i as u32).wrapping_mul(73).wrapping_add(17) as u8;
        }
        e.write(srcv, &src);
        let sentinel = vec![0x9Eu8; total.max(4)];
        e.write(allocv, &sentinel);
        e.reset_bump(allocv);

        let mut hh = vec![0u8; HDR_LEN];
        set32(&mut hh, 0x04, srcv as u32);
        set16(&mut hh, 0x14, w);
        set16(&mut hh, 0x16, h);
        hh[0x18] = bpp;
        hh[0x19] = desc;
        hh[0x0a] = itype;
        e.write(hv, &hh);

        let bin_ret = e.call(FLIP, hv as u32, 0, &[target]);
        let bin_hdr = e.read(hv, HDR_LEN);

        let mut mine_hdr = hh.clone();
        let (my_ret, flipped) = flip(&mut mine_hdr, &src, target);

        assert_eq!(
            bin_ret, my_ret,
            "flip ret {w}x{h} bpp={bpp} desc={desc} it={itype} t={target}"
        );
        assert!(
            hdr_eq(&bin_hdr, &mine_hdr),
            "flip hdr {w}x{h} bpp={bpp} desc={desc} it={itype} t={target}"
        );
        if let Some(out) = flipped {
            let bin_out = e.read(allocv, total);
            assert_eq!(
                bin_out, out,
                "flip out {w}x{h} bpp={bpp} desc={desc} t={target}"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 0x5a4250 — 32→24bpp alpha-strip in place
// ---------------------------------------------------------------------------------------------
#[test]
fn image_tga_alpha_strip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let hv = e.alloc(HDR_LEN as u64);
    let bufv = e.alloc(0x8000);
    e.stub(SERR, 0, 1);

    // (width, height, bpp, descriptor)
    for &(w, h, bpp, desc) in &[
        (4u16, 4u16, 32u8, 8u8), // strip (attr=8)
        (1, 1, 32, 8),           // strip single px
        (3, 5, 32, 8),           // strip odd dims
        (4, 4, 24, 0),           // no-op (already 24bpp, attr 0)
        (4, 4, 32, 0),           // fixup attr from bpp (0->8) then strip
        (4, 4, 16, 0),           // fixup attr (16-8=8) then 16->8 strip
        (4, 4, 32, 1),           // error (attr=1, not 0/8)
        (4, 4, 32, 4),           // error (attr=4)
        (2, 2, 32, 8),
    ] {
        let bpx = bpx_of(bpp);
        let buf_bytes = (w as usize * h as usize * bpx).max(4);
        let mut buf = vec![0u8; buf_bytes];
        for (i, b) in buf.iter_mut().enumerate() {
            *b = (i as u32).wrapping_mul(91).wrapping_add(13) as u8;
        }
        e.write(bufv, &buf);
        let mut hh = vec![0u8; HDR_LEN];
        set32(&mut hh, 0x04, bufv as u32);
        set16(&mut hh, 0x14, w);
        set16(&mut hh, 0x16, h);
        hh[0x18] = bpp;
        hh[0x19] = desc;
        e.write(hv, &hh);

        let bin_ret = e.call(ALPHA_STRIP, hv as u32, 0, &[]);
        let bin_buf = e.read(bufv, buf_bytes);
        let bin_hdr = e.read(hv, HDR_LEN);

        let mut mine_hdr = hh.clone();
        let mut my_buf = buf.clone();
        let my_ret = alpha_strip(&mut mine_hdr, &mut my_buf);

        assert_eq!(
            bin_ret, my_ret,
            "alpha_strip ret {w}x{h} bpp={bpp} desc={desc}"
        );
        assert_eq!(
            bin_buf, my_buf,
            "alpha_strip buf {w}x{h} bpp={bpp} desc={desc}"
        );
        assert!(
            hdr_eq(&bin_hdr, &mine_hdr),
            "alpha_strip hdr {w}x{h} bpp={bpp} desc={desc}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 0x5a43c0 — TGA CImage init from source (validate, descriptor pack, alloc+copy, footer)
// ---------------------------------------------------------------------------------------------
#[test]
fn image_tga_init_from_source() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let hv = e.alloc(HDR_LEN as u64);
    let srcv = e.alloc(0x8000);
    let allocv = e.alloc(0x8000);
    e.stub_alloc(SMEM_ALLOC, allocv, 0x8000, 4);
    e.stub(SMEM_FREE, 0, 4);
    e.stub(SERR, 0, 1);

    // valid (bpp 0x18/0x20, attr 0/8) + invalid (bad bpp / bad attr).
    // (width, height, bpp, attr, flag5, flag4, valid)
    for &(w, h, bpp, attr, f5, f4, valid) in &[
        (4u16, 4u16, 0x20u8, 0u8, 0u32, 0u32, true),
        (4, 4, 0x20, 8, 1, 0, true),
        (4, 4, 0x18, 0, 0, 1, true),
        (3, 5, 0x18, 8, 1, 1, true),
        (1, 1, 0x20, 0, 0, 0, true),
        (8, 8, 0x20, 8, 7, 3, true), // nonzero flags (predicate, not value)
        (4, 4, 16, 0, 0, 0, false),  // bad bpp
        (4, 4, 0x20, 4, 0, 0, false), // bad attr
        (2, 2, 0x18, 0, 0, 0, true),
    ] {
        let bpx = bpx_of(bpp);
        let size = bpx * w as usize * h as usize;
        let mut src = vec![0u8; size.max(4)];
        for (i, b) in src.iter_mut().enumerate() {
            *b = (i as u32).wrapping_mul(59).wrapping_add(3) as u8;
        }
        e.write(srcv, &src);
        let sentinel = vec![0xD7u8; size.max(4)];
        e.write(allocv, &sentinel);
        e.reset_bump(allocv);

        // start from a pre-dirtied header (sentinel) so the field-clears are observable.
        let mut hh = vec![0x6Bu8; HDR_LEN];
        set32(&mut hh, 0x00, 0); // SFile (masked anyway)
        set32(&mut hh, 0x04, 0); // no old pixels -> no SMemFree
        e.write(hv, &hh);

        let bin_ret = e.call(
            INIT_FROM_SOURCE,
            hv as u32,
            0,
            &[
                srcv as u32,
                u32::from(w),
                u32::from(h),
                u32::from(bpp),
                u32::from(attr),
                f5,
                f4,
            ],
        );
        let bin_hdr = e.read(hv, HDR_LEN);

        let mut mine_hdr = hh.clone();
        let mut my_out = sentinel.clone();
        let my_ret = init_from_source(&mut mine_hdr, &src, &mut my_out, w, h, bpp, attr, f5, f4);

        assert_eq!(
            bin_ret, my_ret,
            "init ret {w}x{h} bpp={bpp} attr={attr} valid={valid}"
        );
        assert!(
            hdr_eq(&bin_hdr, &mine_hdr),
            "init hdr {w}x{h} bpp={bpp} attr={attr} valid={valid}"
        );
        if valid {
            let bin_out = e.read(allocv, size);
            assert_eq!(
                bin_out,
                my_out[..size],
                "init out {w}x{h} bpp={bpp} attr={attr}"
            );
        }
    }

    // null source -> binary returns 0 with header untouched (the transcription's null is "no call").
    {
        let mut hh = vec![0x6Bu8; HDR_LEN];
        set32(&mut hh, 0x04, 0);
        e.write(hv, &hh);
        let bin_ret = e.call(INIT_FROM_SOURCE, hv as u32, 0, &[0, 4, 4, 0x20, 0, 0, 0]);
        let bin_hdr = e.read(hv, HDR_LEN);
        assert_eq!(bin_ret, 0, "init null-src ret");
        assert!(hdr_eq(&bin_hdr, &hh), "init null-src hdr untouched");
    }
}
