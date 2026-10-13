//! The arithmetic of the world's map object liquids: the grid walks of `DrawGroupMagma`
//! (`0x6b68f0`), `DrawExteriorGroupLiquid` (`0x6b6630`) and `DrawInteriorGroupLiquid` (`0x6b6420`),
//! the alpha ramp `BuildGroupDepthRamp` (`0x6b6b60`), the distances of `FindNearestGroupLiquid`
//! (`0x68adc0`), and the plane of `SetDefWaterPlane` (`0x6a7650`). A liquid's grid vertices are
//! 4.1666665 apart (`0x810cc0`, `0x810298`).

/// 4.1666665 (`0x810cc0`, `0x40855555`): the spacing of a liquid's grid vertices.
pub const MLIQ_CELL_STEP: f32 = f32::from_bits(0x4085_5555);
/// 1/256 (`0x80a8ac`): a vertex's texture coordinate unit.
pub const MLIQ_UV_SCALE: f32 = f32::from_bits(0x3b80_0000);
/// 0.1 (`0x801620`, `0x3dcccccd`).
pub const MLIQ_RENDER_SCALE: f32 = f32::from_bits(0x3dcc_cccd);

/// `0x6b68f0` at `0x6b69a7` and `0x6b69c5`: a vertex's 16-bit texture coordinate, `fild` of it
/// sign-extended, times 1/256, stored as `f32`.
pub fn mliq_render_uv(tex_i16: i32, uv_scale: f32) -> f32 {
    (f64::from(tex_i16) * f64::from(uv_scale)) as f32
}

/// `a + b` stored as `f32`: a running grid coordinate one step further, at `0x6b68f0`'s
/// `0x6b69d4` and `0x6b69ec`, `0x6b6630`'s `0x6b67b4` and `0x6b67dd`, and `0x6b6420`'s `0x6b652c`
/// and `0x6b6555`.
pub fn mliq_advance(a: f32, b: f32) -> f32 {
    a + b
}

/// `0x6b68f0` at `0x6b6ac7`: the value `0x73f90a` leaves in `st0` times 0.1, stored as `f32`.
pub fn mliq_render_material_scale(render_value: f32, render_scale: f32) -> f32 {
    render_value * render_scale
}

/// `0x6b6b60`: the 256-byte ramp at `0xca7f10` from the light state's `+0x114` (near) and `+0x118`
/// (far), which `GetLightState` (`0x6d48b0`) returns. Each end becomes a byte as the bits of
/// `f32(x·255 + 512) >> 14`; the ramp starts at `near << 8` and steps by `far − near` a byte.
pub fn mliq_depth_alpha_lut(near: f32, far: f32) -> [u8; 256] {
    let byte = |x: f32| (((f64::from(x) * 255.0 + 512.0) as f32).to_bits() >> 14) as i32 & 0xff;
    let (near_b, far_b) = (byte(near), byte(far));
    let step = ((far_b - near_b) << 8) >> 8;
    let mut acc = near_b << 8;
    let mut lut = [0u8; 256];
    for slot in lut.iter_mut() {
        *slot = (acc >> 8) as u8;
        acc = acc.wrapping_add(step);
    }
    lut
}

/// `0x68adc0` at `0x68ae49` to `0x68ae5e`: a grid vertex's coordinate `cell·4.1666665 + base`,
/// kept in the register.
pub fn mliq_np_grid_coord(cell: i32, step: f32, base: f32) -> f64 {
    f64::from(cell) * f64::from(step) + f64::from(base)
}

/// `0x68adc0` at `0x68ae66` and `0x68ae6b`: the grid coordinate less the query's, stored as `f32`.
pub fn mliq_np_delta(grid_coord: f64, query: f32) -> f32 {
    (grid_coord - f64::from(query)) as f32
}

/// `0x68adc0` from `0x68ae71` to `0x68ae8f`: `dx² + (dy² + dz²)` in the register, with `dz` stored
/// as `f32` by `fst` and squared as the unrounded value times the stored one.
pub fn mliq_np_dist2(fv5: f32, fv6: f32, dz_live: f64) -> f64 {
    let dz_sq = dz_live * f64::from(dz_live as f32);
    let s1 = f64::from(fv6) * f64::from(fv6) + dz_sq;
    f64::from(fv5) * f64::from(fv5) + s1
}

/// `0x6a7650` from `0x6a7792` to `0x6a77c5`: the definition's `+0x98` negated and stored as `f32`,
/// the `w` of the clip plane `(0, 0, 1, w)`.
pub fn clip_plane_negate(scalar: f32) -> f32 {
    -scalar
}
