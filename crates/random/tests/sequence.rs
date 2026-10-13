//! `NTempest::CRndSeed::SetSeed` (`0x44ea80`) followed by eight calls of
//! `NTempest::CRandom::uint32` (`0x4531e0`) on the same state, and `RandomUnitVector3` (`0x44fe20`)
//! from states written directly, as an emitter's `CRandom` and a particle's draws use them.

#![allow(clippy::float_cmp)]
#![allow(clippy::needless_range_loop)] // the loops index the client's memory layout

use benilla_re_difftest::{Emu, Wow};
use benilla_re_random::{crandom_uint32, rand_unit_vector_3d, srand_seed_state, TABLE_VA};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// Each seed through `0x44ea80`, then eight steps through `0x4531e0`: `eax` and both words of the
/// state compared after every step.
#[test]
fn seeded_sequence_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = emu.read(u64::from(TABLE_VA), 0x100 + 3);
    let state = emu.alloc(0x10);
    let mut s = 0x51a4_b3c2u32;
    for _ in 0..300 {
        let seed = next(&mut s);
        emu.call(0x44ea80, state as u32, 0, &[seed]);
        let (mut acc, mut idx) = srand_seed_state(seed);
        assert_eq!(
            (emu.read_u32(state), emu.read_u32(state + 4)),
            (acc, idx),
            "seed={seed:#x}"
        );
        for step in 0..8 {
            let eax = emu.call(0x4531e0, state as u32, 0, &[]);
            let (a, i, ours) = crandom_uint32(acc, idx, &table);
            (acc, idx) = (a, i);
            assert_eq!(eax, ours, "seed={seed:#x} step={step}");
            assert_eq!(emu.read_u32(state), acc, "seed={seed:#x} step={step}");
            assert_eq!(emu.read_u32(state + 4), idx, "seed={seed:#x} step={step}");
        }
    }
}

/// `0x44fe20` whole on states with either sign of the accumulator and on fuzz: the vector and
/// the state read back.
#[test]
fn unit_vector_3d_states_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = emu.read(u64::from(TABLE_VA), 0x100);
    let out = emu.alloc(0x10);
    let st = emu.alloc(0x10);
    let structured: &[(u32, u32)] = &[
        (0x0000_0000, 0x0000_0000),
        (0xffff_ffff, 0x1234_5678),
        (0x8000_0000, 0x89ab_cdef),
        (0x7fff_ffff, 0xdead_beef),
    ];
    let mut s = 0x44fe_2001u32;
    let fuzz: Vec<(u32, u32)> = (0..400).map(|_| (next(&mut s), next(&mut s))).collect();
    for &(acc, idx) in structured.iter().chain(fuzz.iter()) {
        emu.write_u32(st, acc);
        emu.write_u32(st + 4, idx);
        emu.call(0x44fe20, out as u32, st as u32, &[]);
        let (v, (a, i)) = rand_unit_vector_3d(acc, idx, &table);
        for k in 0..3 {
            let got = emu.read_u32(out + 4 * k as u64);
            assert_eq!(got, v[k].to_bits(), "acc={acc:#x} idx={idx:#x} [{k}]");
        }
        assert_eq!(
            (emu.read_u32(st), emu.read_u32(st + 4)),
            (a, i),
            "acc={acc:#x} idx={idx:#x}"
        );
    }
}
