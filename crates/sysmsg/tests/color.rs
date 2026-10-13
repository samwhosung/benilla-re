//! `0x44cca0` against [`color_channel`]: every byte value in each of the four colour records.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_sysmsg::color_channel;

const COLOR_LOOKUP: u64 = 0x44cca0;
const COLOR_TABLE: u64 = 0x0083_5908; // 4 records of 8 bytes: u8 R, G, B, pad, f32 duration

#[test]
fn color_lookup_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let outr = e.alloc(4);
    let outg = e.alloc(4);
    let outb = e.alloc(4);

    let mut s: u32 = 0xc010_0001;
    let next = |s: &mut u32| {
        *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *s
    };
    for i in 0..6000u32 {
        let idx = (next(&mut s) >> 8) % 4;
        let r = (next(&mut s) >> 8) as u8;
        let g = (next(&mut s) >> 8) as u8;
        let b = (next(&mut s) >> 8) as u8;
        let rec = COLOR_TABLE + u64::from(idx) * 8;
        e.write(rec, &[r, g, b, 0]);
        e.write_u32(outr, 0xdead_beef);
        e.write_u32(outg, 0xdead_beef);
        e.write_u32(outb, 0xdead_beef);
        // __fastcall: ecx = index, edx = &r, then &g and &b on the stack; ret 8.
        e.call(COLOR_LOOKUP, idx, outr as u32, &[outg as u32, outb as u32]);
        let (br, bg, bb) = (e.read_f32(outr), e.read_f32(outg), e.read_f32(outb));
        assert_eq!(
            br.to_bits(),
            color_channel(r).to_bits(),
            "color_lookup[{i}] idx={idx} R={r}"
        );
        assert_eq!(
            bg.to_bits(),
            color_channel(g).to_bits(),
            "color_lookup[{i}] idx={idx} G={g}"
        );
        assert_eq!(
            bb.to_bits(),
            color_channel(b).to_bits(),
            "color_lookup[{i}] idx={idx} B={b}"
        );
    }
}
