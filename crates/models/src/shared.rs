//! The shared model (`systems/models/shared.md`): the vertex shader id of
//! `CM2Shared::GetVertexShaderId` (`0x71de50`).

/// The digit of texture stage `k` in a batch's vertex shader id (`0x71dea5` to `0x71ded9`):
/// 0 from the batch's texture count on, 2 when the coordinate lookup entry is above 2, 1 when the
/// transform lookup entry is below the header's transform count, else 0.
pub fn stage_digit(
    k: u32,
    texture_count: u16,
    coord_lookup: u16,
    transform_lookup: u16,
    transform_count: u32,
) -> u32 {
    if k >= u32::from(texture_count) {
        0
    } else if coord_lookup > 2 {
        2
    } else if u32::from(transform_lookup) < transform_count {
        1
    } else {
        0
    }
}

/// `0x71de50` from the digits to the id (`0x71dede` to `0x71df18`): with `F` the complement of the
/// material's flags word (a 16-bit `not`), `Z = (F >> 1 & 1) + 2·(F & 1 + 2·(influences + 5·P))`
/// and the id `5·(3·(3·Z + t0) + t1)`. `P` is 1 when the cache granted pixel shaders, and
/// `influences` is the submesh's word at `+0x10`.
pub fn vertex_shader_id(
    pixel_shaders: bool,
    material_flags: u16,
    influences: u16,
    digits: [u32; 2],
) -> u32 {
    let f = u32::from(!material_flags);
    let pixel = u32::from(pixel_shaders);
    let lit = f & 1;
    let fog = (f >> 1) & 1;
    let z = fog.wrapping_add(2u32.wrapping_mul(
        lit.wrapping_add(2u32.wrapping_mul(u32::from(influences).wrapping_add(5 * pixel))),
    ));
    let id = z.wrapping_mul(3).wrapping_add(digits[0]);
    let id = id.wrapping_mul(3).wrapping_add(digits[1]);
    id.wrapping_mul(5)
}
