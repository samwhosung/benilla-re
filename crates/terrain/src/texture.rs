//! The alpha and shadow decoders and the combined chunk texture
//! (`systems/terrain/chunk.md#alpha-maps`, `#shadow-maps`, `#combined-chunk-texture`).
//!
//! The tables: the nibble masks and shifts `0x810ba4`, `0x810bac` (`[0xf, 0xf0]`, `[4, 0]`, low
//! nibble first, moved to the byte's top); the shadow bits low first; the shadow byte table
//! `0x810bf4`, `[0xff, 0x00]`; the shadow texel table `0x86b590`, `[0x0000, 0xffff]`.

/// Alpha sample `k`, moved to the byte's top nibble.
#[inline]
fn nib(mcal: &[u8], k: usize) -> u8 {
    const MASK: [u8; 2] = [0x0f, 0xf0];
    const SHIFT: [u32; 2] = [4, 0];
    (mcal[k >> 1] & MASK[k & 1]).wrapping_shl(SHIFT[k & 1])
}

/// Shadow bit `k`, low bit first.
#[inline]
fn bit(mcsh: &[u8], k: usize) -> usize {
    usize::from((mcsh[k >> 3] >> (k & 7)) & 1)
}

/// `DecodeAlpha64` (`0x6b0ac0`): 64×64 texels, each the sample in the top byte ORed with 0xfff.
pub fn mcal_decode64(out: &mut [u16; 4096], mcal: &[u8; 2048]) {
    for (k, o) in out.iter_mut().enumerate() {
        *o = (u16::from(nib(mcal, k)) << 8) | 0xfff;
    }
}

/// `DecodeAlpha64Half` (`0x6b0b30`): 32×32 texels, 31 even samples and an edge sample at `k+3` a
/// row; the next row starts at `k+4+0x40`, or `k+0x44+0x40` entering the last.
pub fn mcal_decode32(out: &mut [u16; 1024], mcal: &[u8; 2048]) {
    let mut k = 0usize;
    let mut o = 0usize;
    let mut row = 0usize;
    loop {
        for _ in 0..31 {
            out[o] = (u16::from(nib(mcal, k)) << 8) | 0xfff;
            o += 1;
            k += 2;
        }
        let last = k - 2;
        out[o] = (u16::from(nib(mcal, last + 3)) << 8) | 0xfff;
        o += 1;
        let mut next = last + 4;
        if row == 0x3c {
            next = last + 0x44;
        }
        row += 2;
        if row >= 0x40 {
            break;
        }
        k = next + 0x40;
    }
}

/// `DecodeAlpha63` (`0x6b0bf0`): 63×63 texels, each row's last repeated and row 62 copied to 63.
pub fn mcal_decode63(out: &mut [u16; 4096], mcal: &[u8; 2048]) {
    let mut k = 0usize;
    let mut o = 0usize;
    for _ in 0..63 {
        for _ in 0..63 {
            out[o] = (u16::from(nib(mcal, k)) << 8) | 0xfff;
            o += 1;
            k += 1;
        }
        out[o] = out[o - 1];
        o += 1;
        k += 1;
    }
    for c in 0..64 {
        out[63 * 64 + c] = out[62 * 64 + c];
    }
}

/// `DecodeAlpha63Half` (`0x6b0c80`): 32×32 texels from the even samples, rows `0x42` apart.
pub fn mcal_decode32_63(out: &mut [u16; 1024], mcal: &[u8; 2048]) {
    let mut k = 0usize;
    let mut o = 0usize;
    for _ in 0..32 {
        let mut last = 0usize;
        for _ in 0..32 {
            last = k;
            out[o] = (u16::from(nib(mcal, k)) << 8) | 0xfff;
            o += 1;
            k += 2;
        }
        k = last + 0x42;
    }
}

/// `DecodeShadow64` (`0x6b0d30`): 64×64 texels, each bit selecting from `0x86b590`.
pub fn mcsh_decode(out: &mut [u16; 4096], mcsh: &[u8; 512]) {
    const LUT: [u16; 2] = [0x0000, 0xffff];
    for (k, o) in out.iter_mut().enumerate() {
        *o = LUT[bit(mcsh, k)];
    }
}

/// `DecodeShadow64Half` (`0x6b0da0`): 32×32 texels with `DecodeAlpha64Half`'s walk.
pub fn mcsh_decode32(out: &mut [u16; 1024], mcsh: &[u8; 512]) {
    const LUT: [u16; 2] = [0x0000, 0xffff];
    let mut k = 0usize;
    let mut o = 0usize;
    let mut row = 0usize;
    loop {
        for _ in 0..31 {
            out[o] = LUT[bit(mcsh, k)];
            o += 1;
            k += 2;
        }
        let last = k - 2;
        out[o] = LUT[bit(mcsh, last + 3)];
        o += 1;
        let mut next = last + 4;
        if row == 0x3c {
            next = last + 0x44;
        }
        row += 2;
        if row >= 0x40 {
            break;
        }
        k = next + 0x40;
    }
}

/// One combined 4444 texel: the shadow byte, then the three alpha layers; an absent input reads
/// as 0xff.
#[inline]
fn combine_texel(sh: u8, l1: u8, l2: u8, l3: u8) -> u16 {
    (u16::from(sh & 0xf0) << 8)
        | (u16::from(l1 & 0xf0) << 4)
        | u16::from(l2 & 0xf0)
        | u16::from(l3 >> 4)
}

#[inline]
fn layer_at(layer: Option<&[u8; 2048]>, k: usize) -> u8 {
    match layer {
        Some(d) => nib(d, k),
        None => 0xff,
    }
}

#[inline]
fn shadow_at(mcsh: Option<&[u8; 512]>, k: usize) -> u8 {
    const LUT: [u8; 2] = [0xff, 0x00]; // 0x810bf4
    match mcsh {
        Some(d) => LUT[bit(d, k)],
        None => 0xff,
    }
}

/// `PackCombined64` (`0x6b03d0`): 64×64 texels `(shadow, layer 1, layer 2, layer 3)`, the layers
/// from the record's `+4`, `+8` and `+0xc`.
pub fn alpha_texel_64(
    out: &mut [u16; 4096],
    layers: [Option<&[u8; 2048]>; 3],
    mcsh: Option<&[u8; 512]>,
) {
    for (k, o) in out.iter_mut().enumerate() {
        *o = combine_texel(
            shadow_at(mcsh, k),
            layer_at(layers[0], k),
            layer_at(layers[1], k),
            layer_at(layers[2], k),
        );
    }
}

/// `PackCombined64Half` (`0x6b0510`): 32×32 combined texels with `DecodeAlpha64Half`'s walk.
pub fn alpha_texel_32_edge(
    out: &mut [u16; 1024],
    layers: [Option<&[u8; 2048]>; 3],
    mcsh: Option<&[u8; 512]>,
) {
    let mut k = 0usize;
    let mut o = 0usize;
    let mut row = 0usize;
    loop {
        for _ in 0..31 {
            out[o] = combine_texel(
                shadow_at(mcsh, k),
                layer_at(layers[0], k),
                layer_at(layers[1], k),
                layer_at(layers[2], k),
            );
            o += 1;
            k += 2;
        }
        let last = k - 2;
        let e = last + 3;
        out[o] = combine_texel(
            shadow_at(mcsh, e),
            layer_at(layers[0], e),
            layer_at(layers[1], e),
            layer_at(layers[2], e),
        );
        o += 1;
        let mut next = last + 4;
        if row == 0x3c {
            next = last + 0x44;
        }
        row += 2;
        if row >= 0x40 {
            break;
        }
        k = next + 0x40;
    }
}

/// `PackCombined63` (`0x6b0770`): 63×63 combined texels, each row's last repeated and row 62
/// copied to 63.
pub fn alpha_texel_dup(
    out: &mut [u16; 4096],
    layers: [Option<&[u8; 2048]>; 3],
    mcsh: Option<&[u8; 512]>,
) {
    let mut k = 0usize;
    let mut o = 0usize;
    for _ in 0..63 {
        for _ in 0..63 {
            out[o] = combine_texel(
                shadow_at(mcsh, k),
                layer_at(layers[0], k),
                layer_at(layers[1], k),
                layer_at(layers[2], k),
            );
            o += 1;
            k += 1;
        }
        out[o] = out[o - 1];
        o += 1;
        k += 1;
    }
    for c in 0..64 {
        out[63 * 64 + c] = out[62 * 64 + c];
    }
}

/// `PackCombined63Half` (`0x6b08d0`): 32×32 combined texels from the even samples, rows `0x42`
/// apart.
pub fn alpha_texel_32(
    out: &mut [u16; 1024],
    layers: [Option<&[u8; 2048]>; 3],
    mcsh: Option<&[u8; 512]>,
) {
    let mut k = 0usize;
    let mut o = 0usize;
    for _ in 0..32 {
        let mut last = 0usize;
        for _ in 0..32 {
            last = k;
            out[o] = combine_texel(
                shadow_at(mcsh, k),
                layer_at(layers[0], k),
                layer_at(layers[1], k),
                layer_at(layers[2], k),
            );
            o += 1;
            k += 2;
        }
        k = last + 0x42;
    }
}
