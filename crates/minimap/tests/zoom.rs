//! The zoom radius (`0x6da9b0` and its identical twin `0x6dab70`) against the exe, whole functions,
//! both branches of `0xceaa60`.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_minimap::zoom::zoom_to_scale;
use common::{assert_f32, g, next};

const ZOOM_TO_SCALE_A: u64 = 0x6da9b0;
const ZOOM_TO_SCALE_B: u64 = 0x6dab70; // byte-identical twin

fn check(e: &mut Emu, va: u64, locked: bool, zoom: u32, zoom_l: u32, ctx: &str) {
    e.write_u32(g::ZOOM_LOCKED_FLAG, if locked { 1 } else { 0 });
    e.write_u32(g::ZOOM, zoom);
    e.write_u32(g::ZOOM_L, zoom_l);
    let b = e.call_f32(va, 0, 0, &[]);
    let o = zoom_to_scale(locked, zoom, zoom_l);
    assert_f32(b, o, ctx);
}

#[test]
fn zoom_to_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s: u32 = 0x6da9_0001;
    for i in 0..2000u32 {
        // Cover both twins, both branches, all 6 valid zoom indices.
        let zoom = (next(&mut s) >> 8) % 6;
        let zoom_l = (next(&mut s) >> 8) % 6;
        check(
            &mut e,
            ZOOM_TO_SCALE_A,
            false,
            zoom,
            zoom_l,
            &format!("zoomA.unlocked[{i}] z={zoom}"),
        );
        check(
            &mut e,
            ZOOM_TO_SCALE_A,
            true,
            zoom,
            zoom_l,
            &format!("zoomA.locked[{i}] zl={zoom_l}"),
        );
        check(
            &mut e,
            ZOOM_TO_SCALE_B,
            false,
            zoom,
            zoom_l,
            &format!("zoomB.unlocked[{i}] z={zoom}"),
        );
        check(
            &mut e,
            ZOOM_TO_SCALE_B,
            true,
            zoom,
            zoom_l,
            &format!("zoomB.locked[{i}] zl={zoom_l}"),
        );
    }
}
