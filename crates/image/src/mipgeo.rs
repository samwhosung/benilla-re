//! Mip-chain geometry: the format-agnostic integer size and count arithmetic shared by the BLP decode
//! and the texture service. `fmt` is the gx device-format id (its meaning belongs to the graphics
//! layer; this layer consumes it).

/// Bits-per-pixel for a gx device-format id. `0x5a49b0` (jump table; default 0 for fmt > 7).
pub fn bpp_by_format(fmt: u32) -> u32 {
    match fmt {
        0 => 4,
        1 | 6 | 7 => 8,
        2 => 0x20,
        3..=5 => 0x10,
        _ => 0,
    }
}

/// Byte size of mip level `l` of a `w x h` image in gx format `fmt`. `0x5a4a00`.
/// `wl = max(1, w >> l)`, `hl = max(1, h >> l)`; DXT formats {0,1,7} clamp each dim to ≥4 (the 4×4
/// block minimum); fmt 9 = RGB565 + 2-bit alpha plane (`wl·hl·2 + max(1, wl·hl>>2)`); else `bpp·wl·hl>>3`.
pub fn level_size(l: u32, w: u32, h: u32, fmt: u32) -> u32 {
    let mut wl = w >> (l & 0x1f);
    if wl == 0 {
        wl = 1;
    }
    let mut hl = h >> (l & 0x1f);
    if hl == 0 {
        hl = 1;
    }
    if fmt == 0 || fmt == 1 || fmt == 7 {
        if wl < 5 {
            wl = 4;
        }
        if hl < 5 {
            hl = 4;
        }
    }
    if fmt == 9 {
        let mut u = wl.wrapping_mul(hl) >> 2;
        if u == 0 {
            u = 1;
        }
        return u.wrapping_add(wl.wrapping_mul(hl).wrapping_mul(2));
    }
    bpp_by_format(fmt).wrapping_mul(wl).wrapping_mul(hl) >> 3
}

/// Number of mip levels in the full chain for `w x h` = `floor(log2(max(w,h))) + 1`. `0x5a4ac0`
/// (halve-and-clamp-to-≥1 loop; count incremented once per iteration).
pub fn level_count(w: u32, h: u32) -> u32 {
    let (mut w, mut h) = (w, h);
    let mut n = 1u32;
    while w > 1 || h > 1 {
        w >>= 1;
        n += 1;
        if w == 0 {
            w = 1;
        }
        h >>= 1;
        if h == 0 {
            h = 1;
        }
    }
    n
}

/// Σ of mip level sizes for levels `0..count`. `0x5a4a80`.
pub fn mip_sum(count: u32, w: u32, h: u32, fmt: u32) -> u32 {
    let mut sum = 0u32;
    let mut i = 0u32;
    while i < count {
        sum = sum.wrapping_add(level_size(i, w, h, fmt));
        i += 1;
    }
    sum
}

/// Total mip-block byte size = pixel data + the `count`-entry 4-byte pointer table. `0x5a4b80`.
pub fn mip_block_size(fmt: u32, w: u32, h: u32) -> u32 {
    let count = level_count(w, h);
    mip_sum(count, w, h, fmt).wrapping_add(count.wrapping_mul(4))
}
