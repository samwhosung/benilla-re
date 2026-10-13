//! `NTempest::CRndSeed::SetSeed` (`0x44ea80`) and then six steps of `NTempest::CRandom::uint32`
//! (`0x4531e0`) on the same state, both called whole, over random seeds.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_random::{crandom_uint32, srand_seed_state, TABLE_VA};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

#[test]
fn prng_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = emu.read(u64::from(TABLE_VA), 0x100);
    let state = emu.alloc(0x10);
    let mut s = 0xc0ff_ee11u32;
    for _ in 0..400u32 {
        let seed = next(&mut s);
        // ecx = the state, the seed on the stack
        emu.call(0x44ea80, state as u32, 0, &[seed]);
        let (mut acc, mut idx) = srand_seed_state(seed);
        assert_eq!(emu.read_u32(state), acc, "seed {seed:#x} accumulator");
        assert_eq!(emu.read_u32(state + 4), idx, "seed {seed:#x} lanes");
        for step in 0..6u32 {
            let eax = emu.call(0x4531e0, state as u32, 0, &[]);
            let (a, i, r) = crandom_uint32(acc, idx, &table);
            (acc, idx) = (a, i);
            assert_eq!(
                emu.read_u32(state),
                acc,
                "seed {seed:#x} step {step} accumulator"
            );
            assert_eq!(
                emu.read_u32(state + 4),
                idx,
                "seed {seed:#x} step {step} lanes"
            );
            assert_eq!(eax, r, "seed {seed:#x} step {step} eax");
        }
    }
}
