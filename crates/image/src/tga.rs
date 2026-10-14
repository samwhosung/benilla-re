//! The TGA codec (`Images\tga.cpp`).
//!
//! The TGA CImage header is overlaid at struct `+0x08`. The struct-touching functions here take that
//! struct as a raw memory image (`hdr: &mut [u8]`, offsets below) plus the relevant pixel buffers as
//! slices, which is what the tests synthesize in the emulator; the heap pointers in the struct
//! (`+0x00` SFile, `+0x04` pixels, `+0x1c` idBuf, `+0x40` palette) are environment detail, so these
//! functions never dereference them (the caller passes the buffers directly) and the tests mask
//! them out of the header compare.
//!
//! Two integer idioms recur in the exe:
//! - `bytes_per_pixel = (bpp + 7) >> 3` (the `add 7; cdq; and 7; sar 3` sequence; `bpp` is a
//!   zero-extended byte, so the value is always positive and the rounding correction is a no-op).
//! - `colormap entry bytes = (min(entrySize/3, 8) * 3) >> 3` (the `/3` is the signed
//!   `imul 0x55555556` magic, here over a positive byte, so plain integer division).

// TGA CImage header field offsets (struct base; header overlaid at +0x08).
const O_PIXELS: usize = 0x04; // pixels ptr (heap; not dereferenced here)
const O_CMAP_TYPE: usize = 0x09; // colorMapType u8
const O_IMAGE_TYPE: usize = 0x0a; // imageType u8
const O_CMAP_FIRST: usize = 0x0b; // colorMapFirstIndex u16
const O_CMAP_LEN: usize = 0x0d; // colorMapLength u16
const O_CMAP_ENTRY: usize = 0x0f; // colorMapEntrySize u8 (bits)
const O_WIDTH: usize = 0x14; // width u16
const O_HEIGHT: usize = 0x16; // height u16
const O_BPP: usize = 0x18; // bpp u8 (mutated during decode)
const O_DESCRIPTOR: usize = 0x19; // descriptor u8 (&0xf attr, bit4 hflip, bit5 top-origin)
const O_FOOTER_EXT: usize = 0x20; // footer: extension-area offset dword
const O_FOOTER_DEV: usize = 0x24; // footer: developer-area offset dword
const O_FOOTER_SIG: usize = 0x28; // footer: 18-byte "TRUEVISION-XFILE." signature
const O_SIZE: usize = 0x3c; // sizeBytes u32

/// The TGA v2 footer signature `SStrCopy`'d in by [`init_from_source`] (`0x85c854`): the 17-char
/// `"TRUEVISION-XFILE."` plus its NUL terminator = the full 18 bytes the `SStrCopy(dst, src, 0x12)`
/// writes (the string is shorter than the cap, so all 17 chars + NUL land and nothing is padded).
const FOOTER_SIG: &[u8; 18] = b"TRUEVISION-XFILE.\0";

#[inline]
fn ru8(h: &[u8], o: usize) -> u8 {
    h[o]
}
#[inline]
fn ru16(h: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([h[o], h[o + 1]])
}
#[inline]
fn wu8(h: &mut [u8], o: usize, v: u8) {
    h[o] = v;
}
#[inline]
fn wu16(h: &mut [u8], o: usize, v: u16) {
    h[o..o + 2].copy_from_slice(&v.to_le_bytes());
}
#[inline]
fn wu32(h: &mut [u8], o: usize, v: u32) {
    h[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

/// `bytes_per_pixel = (bpp + 7) >> 3` — the binary's pervasive depth→byte-stride idiom.
#[inline]
fn bytes_per_pixel(bpp: u8) -> u32 {
    (u32::from(bpp) + 7) >> 3
}

/// Colormap entry size in bytes: `(min(entrySize/3, 8) * 3) >> 3`. `entrySize` is the palette
/// entry depth in bits; the `/3` then `*3` is the binary's literal sequence (not an identity —
/// the `min(.,8)` clamp and the truncating divide make e.g. 16→1, 24/32→3, 8→0).
#[inline]
fn cmap_entry_bytes(entry_size: u8) -> u32 {
    let d = (u32::from(entry_size) / 3).min(8);
    (d * 3) >> 3
}

/// Pixel-data file offset = `entryBytes·colorMapLength + idLength + 0x12`. `0x5a3b40`
/// (`__fastcall`, pure arithmetic on three header fields).
pub fn pixel_offset(color_map_entry_size: u8, color_map_length: u16, id_length: u8) -> u32 {
    cmap_entry_bytes(color_map_entry_size)
        .wrapping_mul(u32::from(color_map_length))
        .wrapping_add(u32::from(id_length))
        .wrapping_add(0x12)
}

/// TGA RLE-decode loop. `0x5a3ed0` (`__thiscall`; reads `width`/`height`/`bpp`, normalizes
/// `imageType -= 8` on exact fill). A run packet (`b & 0x80` set) repeats one source pixel
/// `(b & 0x7f) + 1` times; a raw packet copies `b + 1` literal pixels. Returns 1 once the pixel
/// budget `width·height` is exactly consumed (and decrements `imageType`), 0 on overrun
/// (`SErrSetLastError`). `dst` must hold `width·height·bytes_per_pixel` bytes.
pub fn rle_decode(hdr: &mut [u8], src: &[u8], dst: &mut [u8]) -> u32 {
    let w = u32::from(ru16(hdr, O_WIDTH));
    let h = u32::from(ru16(hdr, O_HEIGHT));
    let bpx = bytes_per_pixel(ru8(hdr, O_BPP)) as usize;
    let mut remaining: i32 = w.wrapping_mul(h) as i32;

    let normalize = |hdr: &mut [u8]| {
        let it = ru8(hdr, O_IMAGE_TYPE);
        wu8(hdr, O_IMAGE_TYPE, it.wrapping_add(0xf8)); // += 0xf8 == -= 8 (mod 256)
    };

    if remaining == 0 {
        normalize(hdr);
        return 1;
    }
    let mut si = 0usize;
    let mut di = 0usize;
    loop {
        let b = src[si];
        si += 1;
        if (b as i8) < 0 {
            // run packet: repeat one pixel (b & 0x7f) + 1 times
            let run = (i32::from(b & 0x7f)) + 1;
            remaining -= run;
            for _ in 0..run {
                dst[di..di + bpx].copy_from_slice(&src[si..si + bpx]);
                di += bpx;
            }
            si += bpx; // source advances by exactly one pixel
        } else {
            // raw packet: b + 1 literal pixels
            let count = (i32::from(b)) + 1;
            remaining -= count;
            let total = (count as usize) * bpx;
            dst[di..di + total].copy_from_slice(&src[si..si + total]);
            si += total;
            di += total;
        }
        if remaining < 0 {
            return 0;
        }
        if remaining == 0 {
            normalize(hdr);
            return 1;
        }
    }
}

/// Append-alpha interleave (24→32). `0x5a3b80` (`__thiscall`; ecx=struct, then dst, src,
/// alpha-source). For each of `width·height` pixels, copies `bytes_per_pixel` colour bytes from
/// `src` to `dst` then appends one alpha byte — `0xff` when `alpha` is `None` (opaque pad) else
/// the next byte of `alpha`. `dst` advances by `bpp_bytes + 1`, `src` by `bpp_bytes`, `alpha` by 1.
/// Then rewrites the header: descriptor `(d & 0xf8) | 8`, `bpp += 8 - (d & 0xf)` (byte wrap),
/// `sizeBytes = bytes_per_pixel(newBpp)·w·h`. Mirrors the binary's `memmove`; all live call sites
/// pass `dst <= src`, so an ascending byte copy reproduces it exactly.
pub fn append_alpha(hdr: &mut [u8], dst: &mut [u8], src: &[u8], alpha: Option<&[u8]>) {
    let wh = (u32::from(ru16(hdr, O_WIDTH)) * u32::from(ru16(hdr, O_HEIGHT))) as usize;
    let bpx = bytes_per_pixel(ru8(hdr, O_BPP)) as usize;
    for (pix, (dchunk, schunk)) in dst
        .chunks_mut(bpx + 1)
        .zip(src.chunks(bpx))
        .take(wh)
        .enumerate()
    {
        dchunk[..bpx].copy_from_slice(schunk); // colour bytes
        dchunk[bpx] = match alpha {
            Some(a) => a[pix], // alpha-source byte
            None => 0xff,      // opaque pad
        };
    }
    let old_desc = ru8(hdr, O_DESCRIPTOR);
    let new_desc = (old_desc & 0xf8) | 8;
    wu8(hdr, O_DESCRIPTOR, new_desc);
    let new_bpp = ru8(hdr, O_BPP).wrapping_add(8u8.wrapping_sub(old_desc & 0xf));
    wu8(hdr, O_BPP, new_bpp);
    wu32(
        hdr,
        O_SIZE,
        bytes_per_pixel(new_bpp).wrapping_mul(wh as u32),
    );
}

/// Paletted→RGB expand. `0x5a3820` (`__thiscall`; ecx=struct, arg0=flags). `out` is the
/// freshly-allocated destination (the binary's `SMemAlloc`); `src_idx` is the per-pixel palette
/// indices (old `pixels`), `palette` the colormap. `flags & 1` requests an appended alpha plane:
/// the expanded colours are written starting `width·height` bytes into `out`, then interleaved
/// down to the front with an opaque `0xff` alpha (the in-place [`append_alpha`] step; `dst < src`
/// so an ascending copy matches). Per pixel: copy `entryBytes` bytes from
/// `palette[(idx - firstIndex)·entryBytes]`. Rewrites header fields (bpp, palette=0, cmapLen=0,
/// cmapType=0, imageType=2, sizeBytes, and — only on the alpha path — descriptor).
pub fn palette_expand(hdr: &mut [u8], src_idx: &[u8], palette: &[u8], flags: u32, out: &mut [u8]) {
    let width = u32::from(ru16(hdr, O_WIDTH));
    let height = u32::from(ru16(hdr, O_HEIGHT));
    let descriptor = ru8(hdr, O_DESCRIPTOR);
    let entry_bytes = cmap_entry_bytes(ru8(hdr, O_CMAP_ENTRY)) as usize;
    let first_index = u32::from(ru16(hdr, O_CMAP_FIRST));

    let attr = u32::from(descriptor & 0xf);
    let attr_bpp = attr + 0x18; // = (descriptor & 0xf) + 24
    let alpha_flag = flags & 1;
    let wh = (width * height) as usize;

    // Colour bytes are written starting `wh * alpha_flag` into the buffer.
    let mut dst = wh * alpha_flag as usize;
    for &index in src_idx.iter().take(wh) {
        let idx = u32::from(index);
        let pal_off = (idx.wrapping_sub(first_index)).wrapping_mul(entry_bytes as u32) as usize;
        out[dst..dst + entry_bytes].copy_from_slice(&palette[pal_off..pal_off + entry_bytes]);
        dst += entry_bytes;
    }

    wu8(hdr, O_BPP, attr_bpp as u8);
    wu32(
        hdr,
        O_SIZE,
        bytes_per_pixel(attr_bpp as u8).wrapping_mul(wh as u32),
    );
    wu32(hdr, 0x40, 0); // palette ptr cleared
    wu16(hdr, O_CMAP_LEN, 0);
    wu8(hdr, O_CMAP_TYPE, 0);
    wu8(hdr, O_IMAGE_TYPE, 2);

    if alpha_flag != 0 {
        // Inline opaque-pad interleave (the binary calls 0x5a3b80 with dst=out, src=out+wh,
        // alpha=NULL). bpp is `attr_bpp` here; dst (offset 0) < src (offset wh), so ascending.
        let bpx = bytes_per_pixel(attr_bpp as u8) as usize;
        let mut dpos = 0usize;
        let mut spos = wh;
        for _ in 0..wh {
            // in-place; dst (offset 0) < src (offset wh), so memmove == the binary's forward copy
            out.copy_within(spos..spos + bpx, dpos);
            out[dpos + bpx] = 0xff;
            dpos += bpx + 1;
            spos += bpx;
        }
        let new_desc = (descriptor & 0xf8) | 8;
        wu8(hdr, O_DESCRIPTOR, new_desc);
        let new_bpp = (attr_bpp as u8).wrapping_add(8u8.wrapping_sub(descriptor & 0xf));
        wu8(hdr, O_BPP, new_bpp);
        wu32(
            hdr,
            O_SIZE,
            bytes_per_pixel(new_bpp).wrapping_mul(wh as u32),
        );
    }
}

/// Vertical flip / set-origin. `0x5a4070` (`__thiscall`; ecx=struct, arg0=target origin).
/// Returns `(ret, flipped)`: a no-op (origin already as requested) returns `(1, None)`; an
/// undecoded RLE `imageType` (9/10/11) returns `(0, None)`; otherwise reverses the row order into
/// a fresh buffer (`stride = bytes_per_pixel·width`), and the header's descriptor bit5 is set to
/// `target != 0`. The new descriptor is written into `hdr` in all non-error cases.
pub fn flip(hdr: &mut [u8], src: &[u8], target: u32) -> (u32, Option<Vec<u8>>) {
    let descriptor = ru8(hdr, O_DESCRIPTOR);
    let bit5 = descriptor & 0x20 != 0;
    let want = target != 0;
    if bit5 == want {
        return (1, None); // origin already matches — nothing to do
    }
    let it = ru8(hdr, O_IMAGE_TYPE);
    if (9..=0xb).contains(&it) {
        return (0, None); // RLE not yet decoded
    }
    let bpx = bytes_per_pixel(ru8(hdr, O_BPP));
    let w = u32::from(ru16(hdr, O_WIDTH));
    let h = u32::from(ru16(hdr, O_HEIGHT));
    let stride = (bpx * w) as usize;
    let mut out = vec![0u8; stride * h as usize];
    for i in 0..h as usize {
        let s = i * stride;
        let d = (h as usize - 1 - i) * stride;
        out[d..d + stride].copy_from_slice(&src[s..s + stride]);
    }
    let new_desc = (descriptor & !0x20) | ((want as u8) << 5);
    wu8(hdr, O_DESCRIPTOR, new_desc);
    (1, Some(out))
}

/// 32→24bpp alpha-strip, in place. `0x5a4250` (`__thiscall`; ecx=struct). Requires the alpha
/// attribute nibble == 8 (32bpp+8 alpha); if the nibble is 0 it is first inferred from `bpp`
/// (`attr = (bpp - 8) & 0xf`, written back) — a no-op early-return `1` when `bpp` is already 24.
/// On a non-8 attribute returns `0`. The strip packs `dst += 3, src += 4` per pixel over the same
/// buffer (`dst <= src`, ascending), drops `bpp` by 8, and clears the descriptor's attr nibble.
pub fn alpha_strip(hdr: &mut [u8], buf: &mut [u8]) -> u32 {
    let descriptor = ru8(hdr, O_DESCRIPTOR);
    let mut desc = descriptor;
    if descriptor & 0xf == 0 {
        let bpp = ru8(hdr, O_BPP);
        if bpp == 0x18 {
            return 1; // already 24bpp, no alpha
        }
        desc = (descriptor & 0xf0) | (bpp.wrapping_sub(8) & 0xf);
        wu8(hdr, O_DESCRIPTOR, desc);
    }
    if desc & 0xf != 8 {
        return 0;
    }
    let wh = (u32::from(ru16(hdr, O_WIDTH)) * u32::from(ru16(hdr, O_HEIGHT))) as usize;
    let new_bpp = ru8(hdr, O_BPP).wrapping_add(0xf8); // -= 8
    wu8(hdr, O_BPP, new_bpp);
    wu8(hdr, O_DESCRIPTOR, desc & 0xf0);
    let bpx = bytes_per_pixel(new_bpp) as usize;
    let mut dpos = 0usize;
    let mut spos = 0usize;
    for _ in 0..wh {
        // in-place; dst (+3/px) trails src (+4/px), so memmove == the binary's forward copy
        buf.copy_within(spos..spos + bpx, dpos);
        dpos += bpx;
        spos += bpx + 1;
    }
    1
}

/// TGA CImage init-from-source. `0x5a43c0` (`__thiscall`; ecx=struct, then src, width,
/// height, bpp, attr, flag5, flag4). Validates (`src != null`, `bpp ∈ {0x18, 0x20}`,
/// `attr ∈ {0, 8}` → else `0`), clears the header `[0x8..0x1a)`, packs the descriptor
/// (attr nibble = `attr`, bit4 = `flag4 != 0`, bit5 = `flag5 != 0`), sets `imageType = 2`,
/// dims, `sizeBytes = bytes_per_pixel·height·width`, copies `sizeBytes` bytes of `src` into the
/// freshly-allocated `out`, zeroes the footer offset dwords, and stamps the 18-byte footer
/// signature. Returns 1 on success.
#[allow(clippy::too_many_arguments)]
pub fn init_from_source(
    hdr: &mut [u8],
    src: &[u8],
    out: &mut [u8],
    width: u16,
    height: u16,
    bpp: u8,
    attr: u8,
    flag5: u32,
    flag4: u32,
) -> u32 {
    if (bpp != 0x20 && bpp != 0x18) || (attr != 0 && attr != 8) {
        return 0;
    }
    hdr[0x8..0x1a].fill(0);
    wu8(hdr, O_BPP, bpp);
    // descriptor: low nibble = attr, bit4 = flag4, bit5 = flag5; bits 6,7 from current (now 0).
    let cur = ru8(hdr, O_DESCRIPTOR); // == 0 (just cleared)
    let mut d = (cur & 0xf0) | (attr & 0xf);
    d = (d & !0x10) | (((flag4 != 0) as u8) << 4);
    d = (d & !0x20) | (((flag5 != 0) as u8) << 5);
    wu8(hdr, O_DESCRIPTOR, d);
    wu8(hdr, O_IMAGE_TYPE, 2);
    wu16(hdr, O_WIDTH, width);
    wu16(hdr, O_HEIGHT, height);
    let size = bytes_per_pixel(bpp)
        .wrapping_mul(u32::from(height))
        .wrapping_mul(u32::from(width));
    wu32(hdr, O_SIZE, size);
    let n = size as usize;
    out[..n].copy_from_slice(&src[..n]);
    wu32(hdr, O_FOOTER_EXT, 0);
    wu32(hdr, O_FOOTER_DEV, 0);
    hdr[O_FOOTER_SIG..O_FOOTER_SIG + 18].copy_from_slice(FOOTER_SIG);
    let _ = O_PIXELS; // documented offset; pixels ptr is environment detail
    1
}
