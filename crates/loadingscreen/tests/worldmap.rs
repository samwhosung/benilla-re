//! The world map tile geometry (`0x4081e0`) against the exe, whole function: it builds the corner
//! position and texture coordinate arrays (`uv` and `quad` here), and both are read back and compared.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_loadingscreen::worldmap::worldmap_tile_geometry;
use common::assert_f32;

const WORLDMAP_TILE_GEOMETRY: u64 = 0x4081e0; // void(void)
const UV_ARRAY: u64 = 0x0088_28f8; // corner positions, 12 floats/tile × 12 tiles
const QUAD_ARRAY: u64 = 0x0088_2be8; // texture coordinates, 8 floats/tile × 12 tiles

#[test]
fn worldmap_tile_geometry_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // The builder takes no input, so one call suffices; sentinels go in first.
    for k in 0..144u64 {
        e.write_u32(UV_ARRAY + 4 * k, 0xdead_beef);
    }
    for k in 0..96u64 {
        e.write_u32(QUAD_ARRAY + 4 * k, 0xdead_beef);
    }
    e.call(WORLDMAP_TILE_GEOMETRY, 0, 0, &[]);
    let (uv, quad) = worldmap_tile_geometry();
    for (k, &o) in uv.iter().enumerate() {
        assert_f32(
            e.read_f32(UV_ARRAY + 4 * k as u64),
            o,
            &format!("worldmap uv[{k}]"),
        );
    }
    for (k, &o) in quad.iter().enumerate() {
        assert_f32(
            e.read_f32(QUAD_ARRAY + 4 * k as u64),
            o,
            &format!("worldmap quad[{k}]"),
        );
    }
}
