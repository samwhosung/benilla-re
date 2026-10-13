//! The texture coordinate arithmetic of `CMapChunk::DrawLayers` (`0x6bef30`) and
//! `CMapChunk::DrawLayersShader` (`0x6bf690`), the same in both
//! (`systems/terrain/render.md#chunk-layer-draw`), and the camera offset of `SetupWorldStages`
//! (`0x6813d0`).

/// `[0x8030a0]`: −1.0.
pub const TEXGEN_NEG_ONE: f32 = -1.0;
/// `[0x810e8c]`: −0.125 (`0xbe000000`).
pub const DETAIL_TEXGEN_SCALE: f32 = f32::from_bits(0xbe00_0000);
/// `[0x7ff9d8]`: 1.0.
pub const TEXGEN_ONE: f32 = 1.0;

/// `0x6bef30` at `0x6bf035` and `0x6bf690` at `0x6bf6aa`: `−1.0/[esi+0x84c]`, stored.
pub fn texgen_scale_recip(texscale: f32) -> f32 {
    (f64::from(TEXGEN_NEG_ONE) / f64::from(texscale)) as f32
}

/// `0x6bef30` at `0x6bef4c`: `[esi+0x78] − [esi+0x68]`, left in the register for the compare with
/// `[0xc62964]`.
pub fn lod_gate_subtract(a: f32, b: f32) -> f64 {
    f64::from(a) - f64::from(b)
}

/// `0x6bef30` at `0x6bf0b8` and `0x6bf690` at `0x6bf748`: the stored scale negated, the argument of
/// `0x7bdd00`.
pub fn neg_fvar1(fvar1: f32) -> f32 {
    (-f64::from(fvar1)) as f32
}

/// `0x6bef30` at `0x6bf18d` and `0x6bf690` at `0x6bf82e`: the stored scale times −0.125.
pub fn detail_texgen_scale(fvar1: f32) -> f32 {
    (f64::from(fvar1) * f64::from(DETAIL_TEXGEN_SCALE)) as f32
}

/// `0x6bef30` from `0x6bf0c5` and `0x6bf1d9`, and `0x6bf690` from `0x6bf755` and `0x6bf87b`: a
/// component of `−(c − cam)`, the camera at `0xc7cf20`, stored.
pub fn cam_relative_negate(coord: f32, cam: f32) -> f32 {
    (-(f64::from(coord) - f64::from(cam))) as f32
}

/// `0x6bef30` from `0x6bf3e6` and `0x6bf690` from `0x6bf98e`: `(1.0/scale)/[0x86c318 + 4·stage]`
/// kept in the register, and each component of the stage's vector times it, stored.
pub fn texgen_apply(fvar1: f32, divisor: f32, tg: [f32; 3]) -> [f32; 3] {
    let recip = f64::from(TEXGEN_ONE) / f64::from(fvar1) / f64::from(divisor);
    tg.map(|c| (f64::from(c) * recip) as f32)
}

/// `SetupWorldStages` (`0x6813d0`) from `0x68151b`: `[ebx+0x6c] − cam` per component, stored.
pub fn cam_relative_delta(coord: f32, cam: f32) -> f32 {
    (f64::from(coord) - f64::from(cam)) as f32
}
