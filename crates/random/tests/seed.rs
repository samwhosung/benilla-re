//! `NTempest::CRndSeed::SetSeed` (`0x44ea80`) run whole: `__thiscall`, the seed on the stack.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_random::srand_seed_state;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

#[test]
fn srand_seed_state_init() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let state = emu.alloc(0x10);

    let run = |emu: &mut Emu, seed: u32| -> (u32, u32) {
        emu.write_u32(state, 0xdead_beef);
        emu.write_u32(state + 4, 0xdead_beef);
        emu.call(0x44ea80, state as u32, 0, &[seed]); // ret 4
        (emu.read_u32(state), emu.read_u32(state + 4))
    };

    let mut total = 0u32;
    for &seed in &[
        0u32,
        1,
        46,
        47,
        48,
        60,
        61,
        62,
        0xffff_ffff,
        0x8000_0000,
        0x1234_5678,
    ] {
        assert_eq!(
            run(&mut emu, seed),
            srand_seed_state(seed),
            "seed={seed:#x}"
        );
        total += 1;
    }
    let mut st = 0x44ea_8000u32;
    for _ in 0..6000 {
        let seed = next(&mut st);
        assert_eq!(
            run(&mut emu, seed),
            srand_seed_state(seed),
            "fuzz seed={seed:#x}"
        );
        total += 1;
    }
    eprintln!("srand_seed_state_init: {total} bit-exact");
}
