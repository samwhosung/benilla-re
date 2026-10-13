//! The cell rectangles of the box and volume queries and the negation of `TestMapObjectBox`
//! (`systems/terrain/collide.md#other-world-queries`).
//!
//! Both rectangles turn four extremes into cell indices, `fistp(f32(0.24·neg) − 0.5)` with `neg =
//! −(c − 17066.666)`. The first three negations are stored as `f32` and reloaded for the product;
//! the fourth stays in the register (`fmul st,st(1)`), so its product takes the unrounded value.

use crate::x87::fistp;

/// `[0x7ffab4]`: the map's half extent, 17066.666 (`0x46855555`).
pub const MAP_CENTER_F32: f32 = f32::from_bits(0x4685_5555);

/// 0.5 before each `fistp`: `[0x86aa2c]`, `[0x86aac4]` and `[0x86b6c0]` all hold it.
const FISTP_BIAS: f32 = 0.5;

/// World units to cells, 0.24 (`0x3e75c28f`): `[0x810ae4]`, `[0x810b44]` and `[0x810d2c]` all hold
/// it.
pub const CELL_SCALE: f32 = f32::from_bits(0x3e75_c28f);

/// `GatherVolumeFacets` (`0x6ac060`) from `0x6ac0b5` to `0x6ac13f`: the cell indices of the box
/// `0x7c1450` builds. `extremes` is in load order, `[ebp-0x44]`, `[ebp-0x40]`, `[ebp-0x50]`,
/// `[ebp-0x4c]`, the last kept in the register; the result is in store order, `[ebp-0x18]`,
/// `[ebp-0x14]`, `[ebp-0x10]`, `[ebp-0xc]`, from the fourth extreme to the first.
pub fn aabb8_cell_indices(extremes: [f32; 4]) -> [i32; 4] {
    let center = f64::from(MAP_CENTER_F32);
    let scale = f64::from(CELL_SCALE);
    let bias = f64::from(FISTP_BIAS);

    let mut idx = [0i32; 3];
    for (c, slot) in idx.iter_mut().enumerate() {
        let neg_f32 = (-(f64::from(extremes[c]) - center)) as f32;
        let prod = (scale * f64::from(neg_f32)) as f32;
        *slot = fistp(f64::from(prod) - bias);
    }
    let neg3_live = -(f64::from(extremes[3]) - center);
    let prod3 = (scale * neg3_live) as f32;
    let idx3 = fistp(f64::from(prod3) - bias);

    [idx3, idx[2], idx[1], idx[0]]
}

/// `TestMapObjectBox` (`0x6ad330`) from `0x6ad381` to `0x6ad3a8`: three components negated, `fld;
/// fchs; fstp` each, before the call to `0x4549a0`. `x` is `[ebp-0x24]`, `[ebp-0x20]`,
/// `[ebp-0x1c]`, stored to `[esp]`, `[esp+4]`, `[esp+8]`.
pub fn negate3(x: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| (-f64::from(x[i])) as f32)
}

/// `TestTerrainBox` (`0x6ad4e0`) from `0x6ad4f0` to `0x6ad57d`: the same indices from the box
/// itself, loaded from `+0xc`, `+0x10`, `+0x0` and `+0x4`, with the same constants.
pub fn box_cell_indices(extremes: [f32; 4]) -> [i32; 4] {
    aabb8_cell_indices(extremes)
}
