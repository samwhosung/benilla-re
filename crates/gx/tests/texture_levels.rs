//! The Direct3D device's level range for a texture, `0x5a0880`, run whole against
//! `benilla_re_gx::texlevels` (`systems/gx/textures.md#device-texture-upload`). The record is a
//! block holding the width (`+0x24`), height (`+0x28`) and flags (`+0x3c`), the device a block
//! holding the skip count (`+0x31c`). Every filter mode crosses bits 5 and 6, and the sides
//! cross powers of two, their neighbours and the ends of the range. A record 0 by 0 on the chain
//! path never returns, so only its one-level arm runs.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_gx::texlevels::texture_levels;

mod common;
use common::next;

const LEVELS: u64 = 0x5a0880;

const SIDES: [u32; 21] = [
    0,
    1,
    2,
    3,
    4,
    5,
    7,
    8,
    9,
    63,
    64,
    65,
    255,
    256,
    257,
    1000,
    4096,
    0x7fff_ffff,
    0x8000_0000,
    0x8000_0001,
    0xffff_ffff,
];
const SKIPS: [u32; 11] = [0, 1, 2, 3, 5, 8, 12, 31, 32, 40, 0xffff_ffff];

#[test]
fn d3d_texture_levels_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let tex = e.alloc(0x100);
    let dev = e.alloc(0x400);
    let out = e.alloc(0x20);
    let mut s = 0x5a08_8000_u32;
    let mut flag_words: Vec<u32> = (0..0x80).collect();
    for _ in 0..64 {
        flag_words.push(next(&mut s));
    }
    let mut cases = 0u32;
    for &flags in &flag_words {
        let mut sides: Vec<(u32, u32)> = Vec::new();
        for &w in &SIDES {
            for &h in &[0, 1, 5, 64, 100, 0x8000_0000] {
                sides.push((w, h));
            }
        }
        for _ in 0..8 {
            sides.push((next(&mut s) >> (next(&mut s) >> 27), next(&mut s) >> 20));
        }
        for (w, h) in sides {
            let one_level = ((flags & 7) <= 1 || flags & 0x40 != 0) && flags & 0x20 == 0;
            let longest = w.max(h);
            if longest == 0 && !one_level {
                assert_eq!(texture_levels(w, h, flags, 0), None);
                continue;
            }
            for &skip in &SKIPS {
                e.write_u32(tex + 0x24, w);
                e.write_u32(tex + 0x28, h);
                e.write_u32(tex + 0x3c, flags);
                e.write_u32(dev + 0x31c, skip);
                for i in 0..4 {
                    e.write_u32(out + 4 * i, 0xdead_0000 + i as u32);
                }
                let ret = e.call(
                    LEVELS,
                    dev as u32,
                    0,
                    &[
                        tex as u32,
                        out as u32,
                        (out + 4) as u32,
                        (out + 8) as u32,
                        (out + 12) as u32,
                    ],
                );
                let _ = ret;
                let got = [
                    e.read_u32(out),
                    e.read_u32(out + 4),
                    e.read_u32(out + 8),
                    e.read_u32(out + 12),
                ];
                let want = texture_levels(w, h, flags, skip).expect("a finite chain");
                assert_eq!(
                    got,
                    [want.width, want.height, want.first, want.end],
                    "{w}x{h} flags {flags:#x} skip {skip:#x}"
                );
                cases += 1;
            }
        }
    }
    assert!(cases > 100_000, "{cases} cases");
}
