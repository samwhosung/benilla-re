//! The `WorldMap\World%d` art tile geometry, `0x4081e0`.
//!
//! Builds the 4 by 3 grid of tile quads: a corner position array (`0x8828f8`, 12 floats per tile: x, y, 0
//! for four corners) and a texture coordinate array (`0x882be8`, 8 floats per tile). The map is 1002 by
//! 668 pixels in 256 pixel tiles; the returned `uv` array is the exe's position array and `quad` its
//! texture coordinate array.

const KU: f32 = f32::from_bits(0x3a82_cf75); // 1/1002 (0x7ffdb8)
const KV: f32 = f32::from_bits(0x3ac4_3730); // 1/668  (0x7ffdb4)
const ONE: f32 = f32::from_bits(0x3f80_0000);
const W_COL3: f32 = f32::from_bits(0x3f6a_0000); // 0.9140625 — last-col quad width
const H_ROW2: f32 = f32::from_bits(0x3f1c_0000); // 0.609375 — last-row quad height (0x7ffdb0)

/// The tile geometry of `0x4081e0`. Returns `(uv, quad)` for 12 tiles in row-major order, index
/// `col + row * 4`: `uv` has 12 floats per tile (4 corner positions `(x, y, 0)` in the unit box), `quad`
/// 8 (4 texture coordinates `(u, v)`). `fild` widens an integer to `f64`, `fmul` and `fsubr` run in `f64`, and
/// `fstp` stores `f32`.
pub fn worldmap_tile_geometry() -> ([f32; 144], [f32; 96]) {
    let mut uv = [0f32; 144];
    let mut quad = [0f32; 96];
    for row in 0..3i32 {
        for col in 0..4i32 {
            let idx = (col + row * 4) as usize;
            let xmax = (col * 0x100 + 0x100).min(0x3ea); // min(col * 256 + 256, 1002)
            let ymax = (row * 0x100 + 0x100).min(0x29c); // min(row * 256 + 256, 668)
            let u0 = ((col * 0x100) as f64 * KU as f64) as f32;
            let u1 = (xmax as f64 * KU as f64) as f32;
            let v0 = (ONE as f64 - (row * 0x100) as f64 * KV as f64) as f32; // top edge
            let v1 = (ONE as f64 - ymax as f64 * KV as f64) as f32; // bottom edge
            let b = idx * 12;
            uv[b..b + 12].copy_from_slice(&[u0, v1, 0.0, u1, v1, 0.0, u0, v0, 0.0, u1, v0, 0.0]);
            let w = if col == 3 { W_COL3 } else { ONE };
            let h = if row == 2 { H_ROW2 } else { ONE };
            let q = idx * 8;
            quad[q..q + 8].copy_from_slice(&[0.0, h, w, h, 0.0, 0.0, w, 0.0]);
        }
    }
    (uv, quad)
}
