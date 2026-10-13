//! `NTempest::CRandom::uint32` (`0x4531e0`) run whole: the state in and out through `ecx`, the
//! result in `eax`, with the table read from the exe.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_random::{crandom_uint32, TABLE_VA};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

#[test]
fn crandom_uint32_advance() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = emu.read(u64::from(TABLE_VA), 0x100);
    let state = emu.alloc(0x10);

    let run = |emu: &mut Emu, acc: u32, idx: u32| -> (u32, u32, u32) {
        emu.write_u32(state, acc);
        emu.write_u32(state + 4, idx);
        let eax = emu.call(0x4531e0, state as u32, 0, &[]);
        (emu.read_u32(state), emu.read_u32(state + 4), eax)
    };

    let mut total = 0u32;
    // Lanes at 0, at 0xff, and at each step exactly and one under.
    for &(acc, idx) in &[
        (0u32, 0x0000_0000u32),
        (0, 0xffff_ffff),
        (0, 0x0001_0203),
        (0xdead_beef, 0x04_0c_18_1c), // each lane at its step: 0
        (0x1234_5678, 0x03_0b_17_1b), // one under: the modulus is added
        (1, 0x8080_8080),
        (0xffff_ffff, 0xfefd_fcfb),
    ] {
        let got = run(&mut emu, acc, idx);
        let want = crandom_uint32(acc, idx, &table);
        assert_eq!(got, want, "acc={acc:#x} idx={idx:#x}");
        total += 1;
    }

    // Random states, every third one the previous result, so the lanes also walk.
    let mut seed = 0x4531_e0e0u32;
    let mut acc = next(&mut seed);
    let mut idx = next(&mut seed);
    for _ in 0..8000 {
        let got = run(&mut emu, acc, idx);
        let want = crandom_uint32(acc, idx, &table);
        assert_eq!(got, want, "fuzz acc={acc:#x} idx={idx:#x}");
        total += 1;
        if total.is_multiple_of(3) {
            // the next call takes this one's state
            acc = got.0;
            idx = got.1;
        } else {
            acc = next(&mut seed);
            idx = next(&mut seed);
        }
    }
    eprintln!("crandom_uint32_advance: {total} bit-exact (4-lane table rotl/xor accumulate)");
}
