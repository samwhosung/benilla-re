//! `CMapChunk::LoadNormals` (`0x6aff10`, `systems/terrain/chunk.md#chunk-normals`) and the area
//! fields of `LoadArea` (`0x6c2720`, `systems/terrain/files.md#tile-load`).

/// `[0x810c08]`: 1/127 (`0x3c010204`).
const MCNR_SCALE: f32 = f32::from_bits(0x3c01_0204);
/// `[0x8105b0]`: −33.333332 (`0xc2055555`), a chunk's width negated.
const ORIGIN_SLOPE: f32 = f32::from_bits(0xc205_5555);
/// `[0x7ffab4]`: 17066.666 (`0x46855555`).
const MAP_CENTER: f32 = f32::from_bits(0x4685_5555);

/// `CMapChunk::LoadNormals` (`0x6aff10`): the 145 normals' signed bytes at `[this+0xf18]`, each
/// `f32(byte·(1/127))` (`movsx; fild; fmul; fstp`), to `this+0x170`.
pub fn mcnr_normals(src: &[i8; 435]) -> [f32; 435] {
    src.map(|b| (f64::from(b) * f64::from(MCNR_SCALE)) as f32)
}

/// The fields `0x6c2720` sets on the area it makes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AreaOrigin {
    /// `+0x7c`: the origin from the row.
    pub origin0: f32,
    /// `+0x80`: the origin from the column.
    pub origin1: f32,
    /// `+0x88`: the column.
    pub tile_x: u32,
    /// `+0x8c`: the row.
    pub tile_y: u32,
    /// `+0x90`: the column times 16.
    pub chunk_base_x: u32,
    /// `+0x94`: the row times 16.
    pub chunk_base_y: u32,
}

/// `LoadArea` (`0x6c2720`, ecx = the column, edx = the row): the indices, their `shl 4`, and each
/// origin `f32(i·−33.333332 + 17066.666)` with `i` the signed `fild` of the shifted index, the row's
/// to `+0x7c` and the column's to `+0x80`.
pub fn adt_area_origin(tile_x: u32, tile_y: u32) -> AreaOrigin {
    let cbx = tile_x.wrapping_shl(4);
    let cby = tile_y.wrapping_shl(4);
    AreaOrigin {
        origin0: origin_leg(cby),
        origin1: origin_leg(cbx),
        tile_x,
        tile_y,
        chunk_base_x: cbx,
        chunk_base_y: cby,
    }
}

/// `fild i; fmul [0x8105b0]; fadd [0x7ffab4]; fstp dword`.
fn origin_leg(i: u32) -> f32 {
    (f64::from(i as i32) * f64::from(ORIGIN_SLOPE) + f64::from(MAP_CENTER)) as f32
}
