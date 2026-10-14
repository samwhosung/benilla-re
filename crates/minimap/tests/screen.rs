//! The world to screen scale (`0x6da380`) against the exe, whole function, with the tile-window calls
//! stubbed: the blip pass (`0x6d9a90`), the world to tile map (`0x6da540`) and the tile shift
//! (`0x6da5e0`) do nothing, and the tile to world map (`0x6da5b0`) returns a synthetic world buffer.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_minimap::screen::world_to_screen_scale;
use common::{assert_f32, fuzz, next};

const WORLD_TO_SCREEN: u64 = 0x6da380; // __fastcall(continent ecx, player* edx, p3, p4, p5)
const BLIP_PASS: u64 = 0x6d9a90; // void(void)
const WORLD_TO_TILE: u64 = 0x6da540; // fastcall, 0 stack
const TILE_SHIFT: u64 = 0x6da5e0; // fastcall + 1 stack
const TILE_TO_WORLD: u64 = 0x6da5b0; // returns out ptr in eax
const ZOOM: u64 = 0x0086_f698;

#[test]
fn world_to_screen_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let tw = e.alloc(0x10); // synthetic tile_to_world output
    let player = e.alloc(0x10);
    let p3 = e.alloc(0x10);
    let p4 = e.alloc(0x10);

    e.stub(BLIP_PASS, 0, 0);
    e.stub(WORLD_TO_TILE, 0, 0);
    e.stub(TILE_SHIFT, 0, 1);
    e.stub(TILE_TO_WORLD, tw as u32, 0); // eax := &tw

    let mut s: u32 = 0x6da3_0001;
    for i in 0..4000u32 {
        let twv = [fuzz(&mut s, 34000.0), fuzz(&mut s, 34000.0), 0.0];
        let plv = [fuzz(&mut s, 34000.0), fuzz(&mut s, 34000.0)];
        let zoom = (next(&mut s) >> 8) % 6;
        e.write_u32(tw, twv[0].to_bits());
        e.write_u32(tw + 4, twv[1].to_bits());
        e.write_u32(tw + 8, twv[2].to_bits());
        e.write_u32(player, plv[0].to_bits());
        e.write_u32(player + 4, plv[1].to_bits());
        e.write_u32(ZOOM, zoom);
        e.write(p3, &[0u8; 8]);
        e.write(p4, &[0u8; 4]);
        e.call(
            WORLD_TO_SCREEN,
            0,
            player as u32,
            &[p3 as u32, p4 as u32, 0],
        );
        let (scale, off) = world_to_screen_scale(twv, plv, zoom);
        assert_f32(e.read_f32(p4), scale, &format!("w2s[{i}] scale z={zoom}"));
        assert_f32(e.read_f32(p3), off[0], &format!("w2s[{i}] off0"));
        assert_f32(e.read_f32(p3 + 4), off[1], &format!("w2s[{i}] off1"));
    }
}
