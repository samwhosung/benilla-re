//! The plane record constructors against the exe, each run whole: the plane through three points
//! (`0x7bf860`) and the record of one scalar (`0x7bf7b0`). The points are the edge fills and random
//! bit patterns, so degenerate and non-finite planes run; the scalars include the infinities and
//! zeros. A NaN result matches any NaN.

mod common;

use benilla_re_cmath::plane;
use common::*;

#[test]
fn plane_from_3pts() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let pts = synth_n(3);
    let p2base = d.a as u32 + 0x40; // a second slot inside d.a's buffer
    let mut checked = 0u64;
    for (k, p0) in pts.iter().enumerate() {
        let p1 = &pts[(k + 1) % pts.len()];
        let p2 = &pts[(k + 2) % pts.len()];
        d.put(d.a, p0);
        d.put(d.b, p1);
        for (i, &v) in p2.iter().enumerate() {
            d.emu
                .write_u32(u64::from(p2base) + i as u64 * 4, v.to_bits());
        }
        // ecx=dst, [ebp+8]=p0, [ebp+0xc]=p1, [ebp+0x10]=p2
        d.emu
            .call(0x7bf860, d.dst as u32, 0, &[d.a as u32, d.b as u32, p2base]);
        assert_slice(
            &d.out_n(13),
            &plane::plane_from_3pts(arr(p0.as_slice()), arr(p1.as_slice()), arr(p2.as_slice())),
            "plane_from_3pts",
        );
        checked += 1;
    }
    eprintln!("plane_from_3pts: {checked} cases bit-exact vs WoW.exe @ 0x7bf860");
}

#[test]
fn scalar_bcast_struct13() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    // The infinities and zeros reach the `(s + s)·0` term, which is NaN for an infinite `s`.
    let mut scalars = synth_scalars();
    scalars.extend_from_slice(&[f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0, 1e30, -1e30]);
    for &s in &scalars {
        // __thiscall: ecx=out(13 f32), [ebp+8]=s by value.
        d.emu.call(0x7bf7b0, d.dst as u32, 0, &[s.to_bits()]);
        assert_slice(
            &d.out_n(13),
            &plane::scalar_bcast_struct13(s),
            "scalar_bcast_struct13",
        );
    }
    eprintln!(
        "scalar_bcast_struct13: {} cases bit-exact vs WoW.exe @ 0x7bf7b0",
        scalars.len()
    );
}
