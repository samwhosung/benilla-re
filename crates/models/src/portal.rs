//! The arithmetic of a map object's portal crossing `CMapObj::CrossPortals` (`0x6a3f80`), of its
//! footprint tiling `CMapObj::FootprintCells` (`0x6a5270`), and `CeilLog2` (`0x6a5590`), which caps
//! the tiling's cell counts.

/// `0x6a3f80` from `0x6a3f8a` to `0x6a4018`: the segment's direction stored as `f32`,
/// `len = f32(√((dy² + dz²) + dx²))`, `inv = f32(1/len)` and the unit direction `f32(d·inv)`.
/// Returns `(unit, len, inv)`.
pub fn portal_seg_normalize(seg: &[f32; 6]) -> ([f32; 3], f32, f32) {
    let [dx, dy, dz] = [0, 1, 2].map(|i| f64::from(seg[i + 3] - seg[i]));
    let len = ((dy * dy + dz * dz) + dx * dx).sqrt() as f32;
    let inv = (1.0 / f64::from(len)) as f32;
    let unit = [dx, dy, dz].map(|d| (d * f64::from(inv)) as f32);
    (unit, len, inv)
}

/// `0x6a3f80` from `0x6a4018` to `0x6a4045`: the length times the caller's scale, stored as `f32`.
pub fn portal_extent_scale(len: f32, scalar: f32) -> f32 {
    len * scalar
}

/// `0x6a3f80` from `0x6a4150` to `0x6a417f`: `((ny·py + nz·pz) + nx·px) + d` in the register,
/// compared with 0 by `test ah,1; jne`: true (the branch taken) when the point is below the
/// portal's plane or the sum is NaN.
pub fn portal_plane_side(plane: &[f32; 4], pt: &[f32; 3]) -> bool {
    let sum = ((f64::from(plane[1]) * f64::from(pt[1]) + f64::from(plane[2]) * f64::from(pt[2]))
        + f64::from(plane[0]) * f64::from(pt[0]))
        + f64::from(plane[3]);
    !(sum >= 0.0)
}

/// `0x6a3f80` from `0x6a41ec` to `0x6a41f7`: the best hit's `t` times `inv`, stored as `f32`.
pub fn portal_result_scale(t_hit: f32, inv: f32) -> f32 {
    t_hit * inv
}

/// `0x6a5270` from `0x6a52a6` to `0x6a52d5`: `r = 1/[0xca7eec]` kept in the register, the box's
/// extents `f32((max − min)·r)` in x and y, the difference unrounded, and `f32(256·ext_y)`. Returns
/// `(ext_x, ext_y, 256·ext_y)`.
pub fn footprint_extent_scale(aabb: &[f32; 6], cell_recip_divisor: f32) -> (f32, f32, f32) {
    let recip = 1.0 / f64::from(cell_recip_divisor);
    let ext_x = ((f64::from(aabb[3]) - f64::from(aabb[0])) * recip) as f32;
    let ext_y = ((f64::from(aabb[4]) - f64::from(aabb[1])) * recip) as f32;
    (ext_x, ext_y, ext_y * 256.0)
}

/// `0x6a5270` from `0x6a5337` to `0x6a5343`: `f32(256·ext_x)`.
pub fn footprint_count_scale(ext_x: f32) -> f32 {
    ext_x * 256.0
}

/// `0x6a5270` at `0x6a53a9` and `0x6a53c0`: `ceil` (`0x73fdf5`) of an extent plus 512, stored as
/// `f32`, whose bits `>> 14` give the cell count.
pub fn footprint_count_bias(ceil_ext: f32) -> f32 {
    (f64::from(ceil_ext) + 512.0) as f32
}

/// `0x6a5270` at `0x6a53dc` and `0x6a53f8`: a cell count times the cell size `[0xca7ebc]`, stored
/// as `f32`.
pub fn footprint_cell_size(count: i32, cell_size: f32) -> f32 {
    (f64::from(count) * f64::from(cell_size)) as f32
}

/// `0x6a5270` from `0x6a541d` to `0x6a54e1`: cell `(ix, iy)` of an `nx`×`ny` tiling of cells
/// `cell_w`×`cell_h` from the corner `+0x14`, `+0x18`: `x0 = f32(ix·w + base_x)`,
/// `y0 = f32(f32(iy·h) + base_y)`, `x1 = f32(x0 + w)`, `y1 = f32(y0 + h)`; a cell on the tiling's
/// edge grows outward by `[0xca8098]`, set once to `f32(2·[0xca7ebc])`. Returns `[x0, y0, x1, y1]`.
#[allow(clippy::too_many_arguments)]
pub fn footprint_cell_rect(
    ix: i32,
    iy: i32,
    nx: i32,
    ny: i32,
    cell_w: f32,
    cell_h: f32,
    base_x: f32,
    base_y: f32,
    cell_size: f32,
) -> [f32; 4] {
    let row_y = (f64::from(iy) * f64::from(cell_h)) as f32;
    let x0 = (f64::from(ix) * f64::from(cell_w) + f64::from(base_x)) as f32;
    let y0 = row_y + base_y;
    let x1 = x0 + cell_w;
    let y1 = y0 + cell_h;
    let border = cell_size + cell_size;
    [
        if ix == 0 { x0 - border } else { x0 },
        if iy == 0 { y0 - border } else { y0 },
        if ix == nx - 1 { x1 + border } else { x1 },
        if iy == ny - 1 { y1 + border } else { y1 },
    ]
}

/// `0x6a5590(x)`: `ceil(log2 x)`. `log10(x)/log10(2)` through `fyl2x`, then `ceil` (`0x73fdf5`) and
/// `__ftol`. The `fyl2x` quotient can differ from `log2` in its last bits, but it is exact at the
/// powers of two, the only points where `ceil` turns, so the count is the same for every positive
/// `x`; `+∞` gives the integer indefinite's low dword, 0. ±0 gives `−∞` (`fyl2x` signals a zero
/// divide, masked) and so 0 too, which no test shows: the emulator's `fyl2x` of 0 pops nothing.
pub fn mip_level_count(x: f32) -> i32 {
    crate::x87::ftol(f64::from(x).log2().ceil())
}
