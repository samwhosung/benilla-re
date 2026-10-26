//! `FirstLiquidTileType` (`0x6ba970`, `systems/models/wmo.md#group-liquid-draw-type`) run whole on
//! a group whose `+0x100` and `+0x11c` hold a tile count and the tile flag bytes.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_models::wmo::first_liquid_tile_type;
use common::Lcg;

const FN_VA: u64 = 0x6ba970;

fn run(e: &mut Emu, group: u64, tiles_va: u64, tiles: &[u8], count: i32, ctx: &str) {
    e.write(tiles_va, tiles);
    e.write_u32(group + 0x100, count as u32);
    e.write_u32(group + 0x11c, tiles_va as u32);
    let bin = e.call(FN_VA, group as u32, 0, &[]);
    let ours = first_liquid_tile_type(tiles, count);
    assert_eq!(bin, ours, "{ctx}: tiles {tiles:02x?} count {count}");
}

#[test]
fn first_liquid_tile_type_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let group = e.alloc(0x170);
    let tiles_va = e.alloc(0x100);
    // A count of 0 or below scans nothing; holes of every high nibble are skipped; the scan stops
    // at the count even when a wet tile follows.
    run(&mut e, group, tiles_va, &[0x04], 0, "zero count");
    run(&mut e, group, tiles_va, &[0x04], -1, "negative count");
    run(
        &mut e,
        group,
        tiles_va,
        &[0x0f, 0x8f, 0xff, 0x46],
        4,
        "holes then 6",
    );
    run(
        &mut e,
        group,
        tiles_va,
        &[0x0f, 0x0f, 0x03],
        2,
        "wet past the count",
    );
    run(
        &mut e,
        group,
        tiles_va,
        &[0xf0, 0x0f],
        2,
        "type 0 with high bits",
    );
    let mut rng = Lcg(0x6ba9_7000);
    for i in 0..5000 {
        let n = 1 + (rng.next() % 0x40) as usize;
        let tiles: Vec<u8> = (0..n)
            .map(|_| {
                let b = rng.byte();
                if rng.next() >> 30 != 0 {
                    b | 0x0f
                } else {
                    b
                }
            })
            .collect();
        let count = (rng.next() % (n as u32 + 1)) as i32;
        run(
            &mut e,
            group,
            tiles_va,
            &tiles,
            count,
            &format!("fuzz[{i}]"),
        );
    }
}
