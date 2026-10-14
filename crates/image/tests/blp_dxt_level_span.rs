//! The DXT level span law against three shipped weather textures.
//!
//! For every stored mip level of a DXTC BLP2 the client hands the GPU
//! `span(L) = bits * max(4, wL) * max(4, hL) >> 3` bytes read from `file[mipOffsets[L] ..]`: the
//! shared level-size function `0x5a4a00` on the re-upload path, the GL `imageSize` arithmetic at
//! `0x59f502` to `0x59f515`, and the D3D block copier `0x5a5780` all compute that number, and none
//! of them reads the header's own `mipSizes[L]`. The arithmetic halves are tested bit for bit
//! (`image_level_size`, `image_dxt35_block_copy`); what those cannot reach is the other half: that
//! the shipped file stores two levels shorter than their span, that the span then runs exactly onto
//! the following levels' blocks, and that it never runs past the file. This test pins those facts,
//! so a drift in the transcription or in the BLP header offsets fails loudly.
//!
//! The files come from the install, so each test reads them from a fixture path
//! (`BENILLA_RE_BLP_RAINDROP01`, `BENILLA_RE_BLP_SNOWFLAKE01`, `BENILLA_RE_BLP_SNOWMIST01`) and
//! skips without one.

use benilla_re_difftest::fixture;
use benilla_re_image::convertmatrix::dxt_block_copy_16;
use benilla_re_image::mipgeo::{bpp_by_format, level_count, level_size};

/// The on-disk BLP2 header (the client copies these 1172 bytes to `CBLPFile+0x4`, so in-memory
/// offsets are these plus 4: `mipOffsets` at `+0x18`, `mipSizes` at `+0x58`).
struct Blp2 {
    compression: u8,
    alpha_depth: u8,
    preferred_format: u8,
    has_mips: u8,
    width: u32,
    height: u32,
    mip_offsets: [u32; 16],
    mip_sizes: [u32; 16],
}

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn parse(b: &[u8]) -> Blp2 {
    assert_eq!(&b[..4], b"BLP2");
    assert_eq!(u32le(b, 4), 1, "BLP2 version");
    let mut mip_offsets = [0u32; 16];
    let mut mip_sizes = [0u32; 16];
    for i in 0..16 {
        mip_offsets[i] = u32le(b, 0x14 + 4 * i);
        mip_sizes[i] = u32le(b, 0x54 + 4 * i);
    }
    Blp2 {
        compression: b[8],
        alpha_depth: b[9],
        preferred_format: b[0xa],
        has_mips: b[0xb],
        width: u32le(b, 0xc),
        height: u32le(b, 0x10),
        mip_offsets,
        mip_sizes,
    }
}

/// The image-format id the texture service hands the decoder for a DXTC file on an S3TC device
/// (`0x44a790`: `preferredFormat 0 → 0`, `1 → 1`, `7 → 7`; `bpp_by_format` = 4/8/8).
fn image_format(h: &Blp2) -> u32 {
    assert_eq!(h.compression, 2, "DXTC file");
    match h.preferred_format {
        0 => 0,
        1 => 1,
        7 => 7,
        other => panic!("preferredFormat {other} is not a DXT class"),
    }
}

#[test]
fn raindrop01_short_levels_take_the_completed_span() {
    let Some(file) = fixture("BENILLA_RE_BLP_RAINDROP01") else {
        return;
    };
    let name = r"textures\Weather\RainDrop01.blp";
    eprintln!("{name}: {} bytes", file.len());
    assert_eq!(file.len(), 3940);
    let h = parse(&file);
    assert_eq!(
        (
            h.compression,
            h.alpha_depth,
            h.preferred_format,
            h.has_mips,
            h.width,
            h.height
        ),
        (2, 8, 1, 1, 16, 128),
        "compression / alphaDepth / preferredFormat / hasMips / w / h"
    );
    assert_eq!(
        &h.mip_offsets[..8],
        &[1172, 3220, 3732, 3860, 3876, 3892, 3908, 3924]
    );
    assert_eq!(&h.mip_sizes[..8], &[2048, 512, 128, 16, 16, 16, 16, 16]);
    assert!(h.mip_offsets[8..].iter().all(|&o| o == 0));
    assert!(h.mip_sizes[8..].iter().all(|&s| s == 0));

    // The level count includes the 1×1, and the file stores exactly that many levels.
    let count = level_count(h.width, h.height);
    assert_eq!(count, 8);
    assert_eq!(h.mip_sizes.iter().filter(|&&s| s != 0).count(), 8);

    // span(L) = bits · max(4,wL) · max(4,hL) >> 3 with bits = 8 for DXT3 (image format 1).
    let fmt = image_format(&h);
    assert_eq!(bpp_by_format(fmt), 8);
    let expected_span = [2048u32, 512, 128, 64, 32, 16, 16, 16];
    let mut short = Vec::new();
    for l in 0..8u32 {
        let wl = (h.width >> l).max(1);
        let hl = (h.height >> l).max(1);
        let span = level_size(l, h.width, h.height, fmt);
        assert_eq!(span, expected_span[l as usize], "span of level {l}");
        assert_eq!(
            span,
            8 * wl.max(4) * hl.max(4) / 8,
            "block-grid arithmetic at level {l}"
        );
        let off = h.mip_offsets[l as usize] as usize;
        let stored = h.mip_sizes[l as usize];
        if stored < span {
            short.push(l);
        }
        // The read never leaves the file — for this asset the last level ends exactly at EOF.
        assert!(
            off + span as usize <= file.len(),
            "level {l} span crosses EOF"
        );
        // The D3D copier `0x5a5780` moves exactly the same count: fill a sentinel-padded
        // destination and check the copy stopped at `span`.
        let mut dst = vec![0xAAu8; span as usize + 16];
        dxt_block_copy_16(wl, hl, &file[off..off + span as usize], &mut dst);
        assert_eq!(&dst[..span as usize], &file[off..off + span as usize]);
        assert!(
            dst[span as usize..].iter().all(|&b| b == 0xAA),
            "copier over-ran at level {l}"
        );
    }
    assert_eq!(short, [3, 4], "the two short levels");
    // What the completion reads: level 3's 64 bytes are the stored blocks of levels 3, 4, 5 and 6;
    // level 4's 32 bytes are those of levels 4 and 5.
    assert_eq!(h.mip_offsets[3] + expected_span[3], h.mip_offsets[7]);
    assert_eq!(h.mip_offsets[4] + expected_span[4], h.mip_offsets[6]);
    assert_eq!(h.mip_offsets[7] + expected_span[7], file.len() as u32);
}

/// Controls that could have failed: two square weather textures, which must store every level at
/// its full span.
#[test]
fn square_weather_dxt_chains_store_every_level_at_its_span() {
    for (variable, name, w, h, levels) in [
        (
            "BENILLA_RE_BLP_SNOWFLAKE01",
            r"textures\Weather\SnowFlake01.blp",
            64u32,
            64u32,
            7u32,
        ),
        (
            "BENILLA_RE_BLP_SNOWMIST01",
            r"textures\Weather\SnowMist01.blp",
            256,
            256,
            9,
        ),
    ] {
        let Some(file) = fixture(variable) else {
            return;
        };
        let hdr = parse(&file);
        assert_eq!(
            (hdr.compression, hdr.width, hdr.height),
            (2, w, h),
            "{name}"
        );
        let fmt = image_format(&hdr);
        assert_eq!(level_count(w, h), levels, "{name}");
        for l in 0..levels {
            let span = level_size(l, w, h, fmt);
            assert_eq!(
                hdr.mip_sizes[l as usize], span,
                "{name} level {l} stored at its span"
            );
            assert!(
                hdr.mip_offsets[l as usize] as usize + span as usize <= file.len(),
                "{name} level {l} within the file"
            );
        }
        assert_eq!(
            hdr.mip_sizes[levels as usize], 0,
            "{name} stores exactly {levels} levels"
        );
    }
}
