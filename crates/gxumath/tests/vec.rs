//! `0x5c6540` run whole: four words stored through the output pointer. The words are random in
//! [−100, 100), and are compared bit for bit.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_gxumath::vec::c4_set;
use common::{assert_arr, fuzz};

const C4_SET: u64 = 0x5c6540; // __thiscall: ecx=dst, stack x,y,z,w

fn rd4(e: &Emu, va: u64) -> [f32; 4] {
    [
        e.read_f32(va),
        e.read_f32(va + 4),
        e.read_f32(va + 8),
        e.read_f32(va + 12),
    ]
}

#[test]
fn vec_helpers_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let dst = e.alloc(0x10);
    let mut s: u32 = 0x5c65_0001;
    for i in 0..4000u32 {
        let v = [
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 100.0),
        ];
        e.write(dst, &[0u8; 16]);
        e.call(
            C4_SET,
            dst as u32,
            0,
            &[
                v[0].to_bits(),
                v[1].to_bits(),
                v[2].to_bits(),
                v[3].to_bits(),
            ],
        );
        assert_arr(
            &rd4(&e, dst),
            &c4_set(v[0], v[1], v[2], v[3]),
            &format!("c4_set[{i}]"),
        );
    }
}
