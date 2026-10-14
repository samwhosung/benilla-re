//! The pieces of `UpdateMapHighlight` (`0x4a7fa0`, `systems/ui/worldmap.md#hover-and-click`) that
//! are plain arithmetic: the hit test on the zone rectangle, the texture size and its power-of-two
//! round-up, and the UV scale and offset of the highlight.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64`; the binary stores to `f32` only at the
//! `fst`/`fstp dword` sites named below, and the pushed Lua numbers are `f64`. `fistp` rounds to
//! nearest even, and `__ftol` truncates.

// The `.rdata` constants of the projection, each an `f32`.
const RECIP_X: f32 = f32::from_bits(0x3ac4_3730); // 0x7ffdb4, about 1/668
const RECIP_Y: f32 = f32::from_bits(0x3a82_cf75); // 0x7ffdb8, about 1/1002
const SCALE_U: f32 = f32::from_bits(0x3c82_cf75); // 0x806528, about 0.015968
const SCALE_V: f32 = f32::from_bits(0x3cc4_3730); // 0x806524, about 0.023952
const OFF_U: f32 = f32::from_bits(0x41fa_8000); // 0x806530, 31.3125
const OFF_V: f32 = f32::from_bits(0x41a7_0000); // 0x80652c, 20.875
const TILE: f32 = f32::from_bits(0x4200_0000); // 0x806534, 32.0

/// The zone rectangle normalized (`0x4a8041` to `0x4a806e`): each edge is `(int)edge * recip`
/// stored as `f32` (`fild; fmul [recip]; fstp [m32]`). The x edges (`+0x34`, `+0x3c`) use
/// `RECIP_X` and the y edges (`+0x38`, `+0x40`) use `RECIP_Y`. Returns `[a, b, c, d]`, the stored
/// `[ebp-0x24]`, `[ebp-0x28]`, `[ebp-0x1c]`, `[ebp-0x20]`: `[e38 * ry, e34 * rx, e40 * ry,
/// e3c * rx]`.
pub fn normalize_zone_rect(e34: i32, e38: i32, e3c: i32, e40: i32) -> [f32; 4] {
    let nx = |e: i32| (f64::from(e) * f64::from(RECIP_X)) as f32;
    let ny = |e: i32| (f64::from(e) * f64::from(RECIP_Y)) as f32;
    [ny(e38), nx(e34), ny(e40), nx(e3c)]
}

/// The hit test (`0x4a8071` to `0x4a80c9`) of the cursor `(p1, p2)` against `[a, b, c, d]` from
/// [`normalize_zone_rect`]. A rectangle with `a == c` or `b == d` is rejected (the `test ah, 0x44;
/// jnp` tests fire on an exact equal only); then `a <= p1 <= c` and `b <= p2 <= d` must hold, and a
/// NaN fails, as the unordered mask takes the miss path.
#[allow(clippy::float_cmp)]
pub fn point_in_zone_rect(p1: f32, p2: f32, a: f32, b: f32, c: f32, d: f32) -> bool {
    (c != a) && (d != b) && (p1 >= a) && (p1 <= c) && (p2 >= b) && (p2 <= d)
}

/// The texture height from a zone rectangle (`0x4a82ff` to `0x4a8339`, selector nonzero), the value
/// the power-of-two round-up [`round_up_texture_dim`] takes. It is `__ftol((NUM * K) / DEN)` with
/// `K = 128.0`, `NUM = y0 - y1` and `DEN = x0 - x1` from the record's floats `[edi+0x10]`,
/// `[edi+0x14]`, `[edi+0x18]`, `[edi+0x1c]`. The numerator stays at register precision (`fst` without a
/// pop at `0x4a830e`) and the denominator is stored as `f32` (`fstp` at `0x4a8305`).
pub fn texture_height_from_rect(x0: f32, x1: f32, y0: f32, y1: f32) -> i32 {
    let den = (f64::from(x0) - f64::from(x1)) as f32;
    let num = f64::from(y0) - f64::from(y1);
    ((num * 128.0) / f64::from(den)).trunc() as i32
}

/// The same from the matched area record `ebx` (`0x4a82ed` to `0x4a8339`, selector 0), with `K =
/// 256.0`: `DENOM = ([ebx+0xc] - [ebx+0x8]) + 1` stored as `f32` and `NUM = ([ebx+0x14] -
/// [ebx+0x10]) + 1` at register precision, both exact `fild` integers. With selector 0 and no
/// matched zone the value keeps its default 128, set at `0x4a82b0`.
pub fn texture_height_from_area(denom_int: i32, num_int: i32) -> i32 {
    let den = f64::from(denom_int) as f32;
    let num = f64::from(num_int);
    ((num * 256.0) / f64::from(den)).trunc() as i32
}

/// The power-of-two round-up of the texture size (`0x4a833c` to `0x4a83ac`), integer arithmetic. `n`
/// is the derived size (`[ebp-0xc]`): its low byte is rounded up to a power of two and the high bits
/// (for `n > 256`) are added back as `((n >> 8) + 1) << 8`.
pub fn round_up_texture_dim(n: u32) -> u32 {
    let lo = n & 0xff;
    if lo == 0 {
        return n; // 0x4a834b: je 0x4a83aa
    }
    let mut edi = if lo & (lo - 1) == 0 {
        lo // 0x4a834d: already a power of two
    } else {
        // 0x4a8354: the highest set bit of lo, plus one.
        let mut hb = 8u32;
        loop {
            hb -= 1;
            if lo & (1 << hb) != 0 || hb == 0 {
                break;
            }
        }
        1u32 << (hb + 1)
    };
    // 0x4a837d: the doubling loop until edi * 8 >= lo, which these inputs never run.
    while edi.wrapping_mul(8) < lo {
        edi = edi.wrapping_add(edi);
    }
    if n > 0x100 {
        edi = edi.wrapping_add(((n >> 8) + 1) << 8); // 0x4a839f
    }
    edi
}

/// The six numbers `UpdateMapHighlight` pushes in the area-grid branch (`0x4a83b9` to `0x4a858e`,
/// `[ebp-0x14] == 0` with an overlay entry `ebx != 0`), as `f64`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HighlightUvArea {
    /// The aspect-fit pair, `w <= h ? (w/h, 1) : (1, h/w)` (`0x4a83b9` to `0x4a83fa`).
    pub aspect: [f64; 2],
    /// `u_scale = w * scale * SCALE_U` (`0x4a8443`).
    pub u_scale: f64,
    /// `v_scale = h * scale * SCALE_V` (`0x4a8457`).
    pub v_scale: f64,
    /// `u_off = u_off0 * SCALE_U`, `u_off0 = ((e8 * scale) + (OFF_U - scale * 32)) + c18` as `f32`.
    pub u_off: f64,
    /// `v_off = v_off0 * SCALE_V`, `v_off0 = ((e10 * scale) + (OFF_V - scale * 32)) + c1c` as `f32`.
    pub v_off: f64,
}

/// The aspect fit and the UV scale and offset of the area-grid branch (`0x4a83b9` to `0x4a858e`).
/// `w` and `h` are the rectangle extents (`[ebp-0x4]`, `[ebp-0x8]`); the overlay entry's fields are
/// `e8` and `e10` (`i32`, `+0x8` and `+0x10`), `c18` and `c1c` (`f32`, `+0x18` and `+0x1c`) and
/// `scale` (`f32`, `+0x20`). Every product is `f64` and is pushed as `f64`; only `u_off0` and
/// `v_off0` are stored to `f32` mid-chain (`fstp [ebp-0x14]`, `[ebp-0x10]`).
#[allow(clippy::float_cmp)]
pub fn highlight_uv_area(
    w: f32,
    h: f32,
    e8: i32,
    e10: i32,
    c18: f32,
    c1c: f32,
    scale: f32,
) -> HighlightUvArea {
    let (wf, hf) = (f64::from(w), f64::from(h));
    let aspect = if w <= h {
        [wf / hf, 1.0]
    } else {
        [1.0, hf / wf]
    };
    let s = f64::from(scale);
    let off_term = |off: f32| f64::from(off) - s * f64::from(TILE);
    let u_off0 = ((f64::from(e8) * s + off_term(OFF_U)) + f64::from(c18)) as f32;
    let v_off0 = ((f64::from(e10) * s + off_term(OFF_V)) + f64::from(c1c)) as f32;
    HighlightUvArea {
        aspect,
        u_scale: (wf * s) * f64::from(SCALE_U),
        v_scale: (hf * s) * f64::from(SCALE_V),
        u_off: f64::from(u_off0) * f64::from(SCALE_U),
        v_off: f64::from(v_off0) * f64::from(SCALE_V),
    }
}

/// The four numbers pushed in the zone branch (`0x4a850b` to `0x4a858e`), reached when the hovered
/// entry's selector `zone+0x8` (`[ebp-0x14]`, tested at `0x4a83ac`) is nonzero. It is an inverse
/// lerp of where the zone's rectangle sits inside its parent's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HighlightUvZone {
    /// `u_scale = recip_x * w` (`0x4a853e`), `w` the zone rectangle's x extent.
    pub u_scale: f64,
    /// `v_scale = recip_y_f64 * h` (`0x4a855d`), `h` the y extent.
    pub v_scale: f64,
    /// `u_off = recip_x * (parent.X0 - zone.X0)` (`0x4a8568` to `0x4a856e`).
    pub u_off: f64,
    /// `v_off = recip_y_f32 * (parent.Y0 - zone.Y0)` (`0x4a857b` to `0x4a857e`).
    pub v_off: f64,
}

/// The zone-branch rectangle inverse lerp (`0x4a850b` to `0x4a858e`). `parent` is the matched row of
/// the table at `0xc0d5b4` (stride `0x20`; its `+0x4` equals the zone's and its `+0x8` is 0), and
/// `zone` is the hovered entry; each is `[X0, X1, Y0, Y1]`, the floats at `+0x10`, `+0x14`, `+0x18`
/// and `+0x1c`.
///
/// The x and y legs round differently:
///
/// - `span_x = parent.X0 - parent.X1` stays at register precision; `recip_x = (1 / span_x) as f32`
///   (`fstp [ebp-0x10]`, popped), and both x outputs reload that `f32`;
/// - `span_y = (parent.Y0 - parent.Y1) as f32` (`fstp [ebp-0x18]`); `recip_y = 1 / span_y` stays at
///   register precision for `v_scale` and is also stored as `f32` (`fst [ebp-0xc]`, no pop), and
///   `v_off` reloads the `f32`;
/// - `w = (zone.X0 - zone.X1) as f32` and `h = (zone.Y0 - zone.Y1) as f32` (`0x4a8319`), and the
///   two differences `parent - zone` are stored as `f32` (`fstp [ebp-0x14]`, `[ebp-0x1c]`).
///
/// The four products are `f64`.
pub fn highlight_uv_zone(parent: [f32; 4], zone: [f32; 4]) -> HighlightUvZone {
    let [px0, px1, py0, py1] = parent;
    let [zx0, zx1, zy0, zy1] = zone;

    let w = (f64::from(zx0) - f64::from(zx1)) as f32;
    let h = (f64::from(zy0) - f64::from(zy1)) as f32;

    let span_x = f64::from(px0) - f64::from(px1);
    let span_y = (f64::from(py0) - f64::from(py1)) as f32;
    let dx = (f64::from(px0) - f64::from(zx0)) as f32;
    let dy = (f64::from(py0) - f64::from(zy0)) as f32;

    let recip_x = (1.0_f64 / span_x) as f32;
    let recip_y_f64 = 1.0_f64 / f64::from(span_y);
    let recip_y_f32 = recip_y_f64 as f32;

    HighlightUvZone {
        u_scale: f64::from(recip_x) * f64::from(w),
        v_scale: recip_y_f64 * f64::from(h),
        u_off: f64::from(recip_x) * f64::from(dx),
        v_off: f64::from(recip_y_f32) * f64::from(dy),
    }
}

/// The zone-branch ratio pushed just before the inverse lerp (`0x4a84a0` to `0x4a84bf`). `dim`
/// (`[ebp-0xc]`, the `__ftol` result) is read by `fild qword` of a quadword with a zero high dword,
/// so as an unsigned 32-bit value; `potdim` (`edi`, the round-up of [`round_up_texture_dim`]) is the
/// signed `i32` divisor of `fidiv`. The result is pushed as `f64`.
pub fn highlight_dim_ratio(dim: u32, potdim: i32) -> f64 {
    (i64::from(dim) as f64) / f64::from(potdim)
}
