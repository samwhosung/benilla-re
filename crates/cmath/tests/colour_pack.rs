//! The packed colour conversions against the exe, each run whole: three channels to a word
//! (`0x7bbec0`) and a word back to three channels (`0x7bbf20`). The channels are the edge fills and
//! random bit patterns, so values below 0, above 1 and past the byte range run; the words are random.

mod common;

use benilla_re_cmath::colour;
use common::*;

#[test]
fn color_pack_rgb() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let cols = synth_n(3);
    for c in &cols {
        d.put(d.a, c);
        d.emu.call(0x7bbec0, d.dst as u32, 0, &[d.a as u32]); // ecx=dst color, [ebp+8]=rgb
        let got = d.emu.read_u32(d.dst);
        assert_eq!(
            got,
            colour::color_pack_rgb(arr(c.as_slice())),
            "color_pack_rgb {c:?}: binary={got:#010x}"
        );
    }
    eprintln!(
        "color_pack_rgb: {} cases bit-exact vs WoW.exe @ 0x7bbec0",
        cols.len()
    );
}

#[test]
fn color_unpack_rgb() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut d = Dev::new(&wow);
    let mut s: u32 = 0x1357_9bdf;
    for _ in 0..2000 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        d.emu.write_u32(d.a, s); // ecx=src color (4 bytes)
        d.emu.call(0x7bbf20, d.a as u32, 0, &[d.dst as u32]); // [ebp+8]=dst vec3
        assert_slice(
            &d.out_n(3),
            &colour::color_unpack_rgb(s),
            "color_unpack_rgb",
        );
    }
    eprintln!("color_unpack_rgb: 2000 cases bit-exact vs WoW.exe @ 0x7bbf20");
}
