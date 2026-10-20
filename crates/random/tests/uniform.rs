//! `RandomUnitVector3` (`0x44fe20`) run whole: `ecx` the output, `edx` the state, compared with
//! the host's `fsqrt` and `fsincos` stand-ins bit for bit.
//!
//! Host double (`docs/METHOD.md#the-harness`): `rand_unit_vector_3d_diff` runs `fsincos` at
//! `0x45001c`. The emulator computes each in host `double`, so where its result reaches a compared
//! value the host's math stands in for the x87's, and the test shows the code around it.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_random::{rand_unit_vector_3d, TABLE_VA};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

#[test]
fn rand_unit_vector_3d_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = emu.read(u64::from(TABLE_VA), 0x100);
    let vec_out = emu.alloc(0x10); // ecx, (x, y, z)
    let state = emu.alloc(0x10); // edx, written back

    // Returns the vector's bits and the new state.
    let run = |emu: &mut Emu, acc: u32, idx: u32| -> ([u32; 3], (u32, u32)) {
        emu.write_u32(vec_out, 0);
        emu.write_u32(vec_out + 4, 0);
        emu.write_u32(vec_out + 8, 0);
        emu.write_u32(state, acc);
        emu.write_u32(state + 4, idx);
        emu.set_reg(RegisterX86::EAX, 0);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.call(0x44fe20, vec_out as u32, state as u32, &[]);
        let raw = emu.read(vec_out, 12);
        let f = |i: usize| u32::from_le_bytes([raw[i], raw[i + 1], raw[i + 2], raw[i + 3]]);
        (
            [f(0), f(4), f(8)],
            (emu.read_u32(state), emu.read_u32(state + 4)),
        )
    };

    let mut total = 0u32;
    let fixed = [
        (0u32, 0x0000_0000u32),
        (0, 0xffff_ffff),
        (0xdead_beef, 0x04_0c_18_1c),
        (0x8000_0000, 0x0102_0304),
        (0x7fff_ffff, 0xfefd_fcfb),
        (0x1234_5678, 0x9abc_def0),
    ];
    for &(acc, idx) in &fixed {
        let (got_v, got_s) = run(&mut emu, acc, idx);
        let (want_v, want_s) = rand_unit_vector_3d(acc, idx, &table);
        let want_bits = [
            want_v[0].to_bits(),
            want_v[1].to_bits(),
            want_v[2].to_bits(),
        ];
        assert_eq!(got_v, want_bits, "unit-vec acc={acc:#x} idx={idx:#x}");
        assert_eq!(got_s, want_s, "unit-vec state acc={acc:#x} idx={idx:#x}");
        total += 1;
    }

    let mut seed = 0x44fe_2000u32;
    for _ in 0..5000 {
        let acc = next(&mut seed);
        let idx = next(&mut seed);
        let (got_v, got_s) = run(&mut emu, acc, idx);
        let (want_v, want_s) = rand_unit_vector_3d(acc, idx, &table);
        let want_bits = [
            want_v[0].to_bits(),
            want_v[1].to_bits(),
            want_v[2].to_bits(),
        ];
        assert_eq!(got_v, want_bits, "unit-vec fuzz acc={acc:#x} idx={idx:#x}");
        assert_eq!(
            got_s, want_s,
            "unit-vec fuzz state acc={acc:#x} idx={idx:#x}"
        );
        total += 1;
    }
    eprintln!("rand_unit_vector_3d_diff: {total} structural (z signed-uniform + 2π angle; r=sqrt(1-z²) & (cos,sin) via libm)");
}
