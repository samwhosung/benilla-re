//! The FreeType arithmetic the client's font layer reaches: the 16.16 multiply (0x7cd0e0), the
//! vector transform (0x7d0340), the TrueType encoding lookup (0x7d8e00), the kerning scale
//! (0x7ce830), and the mono rasterizer's precision setting (0x7dabe0) and span fill (0x7dac90).
//! Every value is a 32-bit integer and every operation wraps as the exe's does. Each `pub fn`
//! cites its source address.

/// `FT_MulFix` (0x7cd0e0): `a · b / 65536` with the sign of `a ^ b`. Returns `a` unchanged when it
/// is 0 or when `b` is exactly `0x10000`. With `|a| <= 0x800` and `|b| <= 0x100000` (unsigned
/// compares at 0x7cd0ff and 0x7cd106) the product is rounded, `(|a|·|b| + 0x8000) >> 16`; otherwise
/// it is built from 16-bit halves and truncated, with no rounding term (0x7cd11d to 0x7cd146).
pub fn mul_fix(a: i32, b: i32) -> i32 {
    if a == 0 || b == 0x10000 {
        return a;
    }
    let negative = (a ^ b) < 0;
    let ua = a.wrapping_abs() as u32;
    let ub = b.wrapping_abs() as u32;
    let r = if ua <= 0x800 && ub <= 0x10_0000 {
        ua.wrapping_mul(ub).wrapping_add(0x8000) >> 16
    } else {
        let al = ua & 0xffff;
        ((ub & 0xffff).wrapping_mul(al) >> 16)
            .wrapping_add((ub >> 16).wrapping_mul(al))
            .wrapping_add((ua >> 16).wrapping_mul(ub))
    };
    if negative {
        (r as i32).wrapping_neg()
    } else {
        r as i32
    }
}

/// `FT_Vector_Transform` (0x7d0340): `x' = x·m[0] + y·m[1]`, `y' = x·m[2] + y·m[3]`, each product a
/// [`mul_fix`] and each sum wrapping. `m` is the matrix's four dwords in memory order, offsets 0,
/// 4, 8 and 0xc.
pub fn vector_transform(v: (i32, i32), m: [i32; 4]) -> (i32, i32) {
    let (x, y) = v;
    let ny = mul_fix(x, m[2]).wrapping_add(mul_fix(y, m[3]));
    let nx = mul_fix(y, m[1]).wrapping_add(mul_fix(x, m[0]));
    (nx, ny)
}

/// The TrueType encoding table at 0x81eb70: platform, encoding (`u32::MAX` for any) and the tag.
pub const ENCODINGS: [(u32, u32, u32); 10] = [
    (2, u32::MAX, u32::from_be_bytes(*b"unic")),
    (0, u32::MAX, u32::from_be_bytes(*b"unic")),
    (1, 0, u32::from_be_bytes(*b"armn")),
    (3, 0, u32::from_be_bytes(*b"symb")),
    (3, 1, u32::from_be_bytes(*b"unic")),
    (3, 2, u32::from_be_bytes(*b"sjis")),
    (3, 3, u32::from_be_bytes(*b"gb  ")),
    (3, 4, u32::from_be_bytes(*b"big5")),
    (3, 5, u32::from_be_bytes(*b"wans")),
    (3, 6, u32::from_be_bytes(*b"joha")),
];

/// The encoding tag of a cmap record (0x7d8e00): the tag of the first row of [`ENCODINGS`] whose
/// platform equals `platform` and whose encoding equals `encoding` or is `u32::MAX`, else 0.
pub fn encoding_tag(platform: u32, encoding: u32) -> u32 {
    ENCODINGS
        .iter()
        .find(|&&(p, e, _)| p == platform && (e == encoding || e == u32::MAX))
        .map_or(0, |&(_, _, tag)| tag)
}

/// The kerning `FT_Get_Kerning` (0x7ce830) returns for the driver's raw vector `raw` in font
/// units: mode 2 returns it as it is; any other mode scales x by `x_scale` and y by `y_scale` with
/// [`mul_fix`], and every mode but 1 then rounds each to a whole pixel, `(v + 32) & -64`.
pub fn kerning(raw: (i32, i32), mode: u32, x_scale: i32, y_scale: i32) -> (i32, i32) {
    if mode == 2 {
        return raw;
    }
    let (x, y) = (mul_fix(raw.0, x_scale), mul_fix(raw.1, y_scale));
    if mode == 1 {
        return (x, y);
    }
    (x.wrapping_add(32) & -64, y.wrapping_add(32) & -64)
}

/// The mono rasterizer's precision fields as 0x7dabe0 stores them at the raster's `+0x0` to
/// `+0x18`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Precision {
    pub bits: i32,
    pub precision: i32,
    pub half: i32,
    pub mask: i32,
    pub shift: i32,
    pub step: i32,
    pub jitter: i32,
}

/// `Set_High_Precision` (0x7dabe0): 10 bits, step 0x80 and jitter 24 when `high`, else 6 bits,
/// step 0x20 and jitter 2; then `precision = 1 << bits`, `half = precision / 2`, `mask =
/// -precision` and `shift = bits - 6`.
pub fn set_high_precision(high: bool) -> Precision {
    let (bits, step, jitter) = if high { (10, 0x80, 0x18) } else { (6, 0x20, 2) };
    let precision = 1i32 << bits;
    Precision {
        bits,
        precision,
        half: precision / 2,
        mask: -precision,
        shift: bits - 6,
        step,
        jitter,
    }
}

/// What a vertical span (0x7dac90) does to a row: the bytes it ORs, from `first`, and the new
/// bounds of the touched byte columns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub first: i32,
    pub bytes: Vec<u8>,
    pub min_x: i16,
    pub max_x: i16,
}

/// `Vertical_Sweep_Span` (0x7dac90): fills the pixels of one row between two crossings `x1` and
/// `x2`, in raster units. `e1 = ceil(x1)` and `e2 = floor(x2)` in pixels, except that `e2 = e1`
/// when `x2 - precision - x1 <= jitter`; a span with `e2 < 0` or `e1 >= width` fills nothing
/// (`None`); otherwise `e1` and `e2` are clamped to the row and every bit from `e1` to `e2` is
/// set, the leftmost pixel in a byte's bit 7. `min_x` and `max_x` are the byte bounds before the
/// span; the returned ones take the span's first and last byte as signed 16-bit values.
pub fn vertical_span(
    p: &Precision,
    x1: i32,
    x2: i32,
    width: u16,
    min_x: i16,
    max_x: i16,
) -> Option<Span> {
    let e1 = (x1.wrapping_add(p.precision).wrapping_sub(1) & p.mask) >> p.bits;
    let e2 = if x2.wrapping_sub(p.precision).wrapping_sub(x1) <= p.jitter {
        e1
    } else {
        (x2 & p.mask) >> p.bits
    };
    let w = i32::from(width);
    if e2 < 0 || e1 >= w {
        return None;
    }
    let e1 = e1.max(0);
    let e2 = if e2 >= w { w - 1 } else { e2 };
    let f1 = (0xffu32 >> (e1 & 7)) as u8;
    let f2 = !((0x7fu32 >> (e2 & 7)) as u8);
    let (c1, c2) = (e1 >> 3, e2 >> 3);
    let min_x = if min_x > c1 as i16 { c1 as i16 } else { min_x };
    let max_x = if max_x < c2 as i16 { c2 as i16 } else { max_x };
    let first = i32::from(c1 as i16);
    let dx = c2.wrapping_sub(c1) as i16;
    let bytes = if dx > 0 {
        let mut b = vec![f1];
        b.extend(std::iter::repeat_n(0xff, (dx - 1) as usize));
        b.push(f2);
        b
    } else {
        vec![f1 & f2]
    };
    Some(Span {
        first,
        bytes,
        min_x,
        max_x,
    })
}
