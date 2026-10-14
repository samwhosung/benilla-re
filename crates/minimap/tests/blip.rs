//! The tile record's LOD distance (`0x6d9e30`) against the exe, whole function, over a `0xa4`-stride
//! record array: the midpoint write, the `FLT_MAX` current-tile write and the not-found return of 0.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_minimap::blip::blip_lod_distance;
use common::{assert_f32, fuzz, next};

const BLIP_LOD_DISTANCE: u64 = 0x6d9e30; // __fastcall(rec* ecx, matchKey edx, stack: key3*, bias)
const STRIDE: u64 = 0xa4;
const N: u64 = 8;

#[test]
fn blip_lod_distance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let arr = e.alloc(STRIDE * N);
    let key3 = e.alloc(0x24);
    let mut s: u32 = 0x6d9e_0001;
    for i in 0..4000u32 {
        // distinct nonzero tile key {mapId,col,row}
        let k0 = 1 + (next(&mut s) >> 16);
        let k1 = 1 + (next(&mut s) >> 16);
        let k2 = 1 + (next(&mut s) >> 16);
        let z14 = fuzz(&mut s, 2000.0);
        let z20 = fuzz(&mut s, 2000.0);
        let bias = fuzz(&mut s, 2000.0);
        let mi = (next(&mut s) >> 8) % (N as u32); // matching slot index

        // zero the array, plant the matching key at slot mi
        e.write(arr, &vec![0u8; (STRIDE * N) as usize]);
        let rec = arr + STRIDE * mi as u64;
        e.write_u32(rec + 0xa0, k0);
        e.write_u32(rec + 0x98, k1);
        e.write_u32(rec + 0x9c, k2);
        // key3 record
        e.write_u32(key3, k0);
        e.write_u32(key3 + 4, k1);
        e.write_u32(key3 + 8, k2);
        e.write_u32(key3 + 0x14, z14.to_bits());
        e.write_u32(key3 + 0x20, z20.to_bits());

        // branch 1: midpoint (matchKey != k0)
        e.write_u32(rec + 0x8c, 0);
        let ret = e.call(
            BLIP_LOD_DISTANCE,
            arr as u32,
            k0.wrapping_add(7),
            &[key3 as u32, bias.to_bits()],
        );
        assert_eq!(ret, rec as u32, "blip_lod[{i}] return slot");
        assert_f32(
            e.read_f32(rec + 0x8c),
            blip_lod_distance(z14, z20, bias),
            &format!("blip_lod[{i}] z14={z14} z20={z20} bias={bias}"),
        );

        // branch 2: FLT_MAX (matchKey == k0)
        e.write_u32(rec + 0x8c, 0);
        e.call(
            BLIP_LOD_DISTANCE,
            arr as u32,
            k0,
            &[key3 as u32, bias.to_bits()],
        );
        assert_eq!(
            e.read_u32(rec + 0x8c),
            0x7f7f_ffff,
            "blip_lod[{i}] FLT_MAX branch"
        );
    }

    // branch 3: not found → return 0
    e.write(arr, &vec![0u8; (STRIDE * N) as usize]);
    e.write_u32(key3, 0x1234);
    e.write_u32(key3 + 4, 0x5678);
    e.write_u32(key3 + 8, 0x9abc);
    let ret = e.call(BLIP_LOD_DISTANCE, arr as u32, 0, &[key3 as u32, 0u32]);
    assert_eq!(ret, 0, "blip_lod not-found → 0");
}
