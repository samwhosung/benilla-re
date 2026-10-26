//! The WMOAreaTable comparator `0x7ccdb0`, run whole on two rows in scratch: equal keys, each key
//! deciding with the others equal, both signs, the group id -1 of a whole-WMO row, differences
//! that wrap, and fuzz.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_terrain::wmoarea::wmo_area_compare;
use common::next;

const WMO_AREA_COMPARE: u64 = 0x7ccdb0;

#[test]
fn wmo_area_compare_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let a = e.alloc(0x50);
    let b = e.alloc(0x50);
    let check = |e: &mut Emu, ka: [i32; 3], kb: [i32; 3]| {
        for (i, (&x, &y)) in ka.iter().zip(&kb).enumerate() {
            e.write_u32(a + 4 + 4 * i as u64, x as u32);
            e.write_u32(b + 4 + 4 * i as u64, y as u32);
        }
        let bin = e.call(WMO_AREA_COMPARE, 0, 0, &[a as u32, b as u32]) as i32;
        assert_eq!(bin, wmo_area_compare(ka, kb), "{ka:?} against {kb:?}");
    };
    let values = [0, 1, -1, 2, 5000, i32::MAX, i32::MIN];
    for &x in &values {
        for &y in &values {
            check(&mut e, [x, 0, 0], [y, 0, 0]);
            check(&mut e, [7, x, 0], [7, y, 0]);
            check(&mut e, [7, 3, x], [7, 3, y]);
            check(&mut e, [x, y, -1], [x, y, 12]);
        }
    }
    let mut s = 0x7ccd_b0b0u32;
    for _ in 0..2000 {
        let mut k = || (next(&mut s) >> (24 + (next(&mut s) >> 30))) as i32 - 8;
        let ka = [k(), k(), k()];
        let kb = [k(), k(), k()];
        check(&mut e, ka, kb);
    }
}
