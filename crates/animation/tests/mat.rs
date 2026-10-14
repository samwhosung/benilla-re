//! The 4×4 product `0x74a7c6` and the quaternion matrix `0x74b6bb` run whole
//! (`systems/models/animation.md#dispatched-matrix-math`, `systems/animation/pose.md#rotation-leg`)
//! and compared bit for bit with `m2_matmul4x4` and `m2_quat_to_mat4`. Both are `cdecl` bodies
//! called directly, `stack_args[0]` being `[ebp+8]`.

mod common;

use benilla_re_animation::mat::{m2_matmul4x4, m2_quat_to_mat4};
use benilla_re_difftest::{Emu, Wow};
use common::{assert_f32, fuzz_f32};

fn write_mat(emu: &mut Emu, va: u64, m: &[f32; 16]) {
    for (i, &v) in m.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, v.to_bits());
    }
}

fn read_mat(emu: &Emu, va: u64) -> [f32; 16] {
    let mut m = [0f32; 16];
    for (i, e) in m.iter_mut().enumerate() {
        *e = emu.read_f32(va + (i as u64) * 4);
    }
    m
}

fn poison(emu: &mut Emu, va: u64) {
    for k in 0..16u64 {
        emu.write_u32(va + k * 4, 0xdead_beef);
    }
}

fn fuzz_mat(s: &mut u32) -> [f32; 16] {
    let mut m = [0f32; 16];
    for e in m.iter_mut() {
        *e = fuzz_f32(s);
    }
    m
}

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0,
];
const ZERO: [f32; 16] = [0.0; 16];

#[test]
fn m2_matmul4x4_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let arena = emu.alloc(0x100);
    // three disjoint 0x40-byte matrix slots; aliasing combos collapse pointers onto these.
    let slot_a = arena;
    let slot_b = arena + 0x40;
    let slot_c = arena + 0x80;

    let mut s = 0x74a7_c600u32;
    let mut checked = 0u32;

    // structured magnitudes paired with each aliasing combo, then a fuzz sweep. `combo`:
    // 0 = all-disjoint, 1 = a==b, 2 = a==c, 3 = a==b==c.
    let structured: [([f32; 16], [f32; 16]); 4] = [
        (IDENTITY, IDENTITY),
        (IDENTITY, ZERO),
        (ZERO, IDENTITY),
        (IDENTITY, IDENTITY),
    ];

    for case in 0..240u32 {
        let combo = case % 4;
        // for a==b==c the single shared region must hold ONE matrix, so b and c coincide.
        let (b, c) = if case < 4 {
            structured[case as usize]
        } else {
            let b = fuzz_mat(&mut s);
            let c = if combo == 3 { b } else { fuzz_mat(&mut s) };
            (b, c)
        };
        let (b, c) = if combo == 3 { (b, b) } else { (b, c) };

        // pointer aliasing per combo; out region (a) is poisoned, then inputs written over it
        // (so an aliased input region holds its input, a disjoint out region stays poisoned).
        let (a_va, b_va, c_va) = match combo {
            0 => (slot_a, slot_b, slot_c),
            1 => (slot_b, slot_b, slot_c), // a == b
            2 => (slot_c, slot_b, slot_c), // a == c
            _ => (slot_b, slot_b, slot_b), // a == b == c
        };
        poison(&mut emu, a_va);
        write_mat(&mut emu, b_va, &b);
        write_mat(&mut emu, c_va, &c);

        emu.clear_calls();
        emu.call(0x74a7c6, 0, 0, &[a_va as u32, b_va as u32, c_va as u32]);
        let bin = read_mat(&emu, a_va);

        let mut ours = [0f32; 16];
        m2_matmul4x4(&mut ours, &b, &c);

        for k in 0..16 {
            assert_f32(
                bin[k],
                ours[k],
                &format!("matmul case {case} combo {combo} elem {k}"),
            );
        }
        checked += 1;
    }

    eprintln!("m2_matmul4x4: {checked} products bit-exact vs WoW.exe @ 0x74a7c6");
}

#[test]
fn m2_quat_to_mat4_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let arena = emu.alloc(0x80);
    let out_va = arena;
    let q_va = arena + 0x40;

    let mut s = 0x74b6_bb00u32;
    let mut checked = 0u32;

    // structured quaternion families: identity, ±unit axes, then fuzzed (non-normalized,
    // negative-w, mixed magnitudes) drawn from fuzz_f32.
    let mut quats: Vec<[f32; 4]> = vec![
        [0.0, 0.0, 0.0, 1.0],  // identity
        [1.0, 0.0, 0.0, 0.0],  // +x axis (180°)
        [-1.0, 0.0, 0.0, 0.0], // −x
        [0.0, 1.0, 0.0, 0.0],  // +y
        [0.0, -1.0, 0.0, 0.0], // −y
        [0.0, 0.0, 1.0, 0.0],  // +z
        [0.0, 0.0, -1.0, 0.0], // −z
        [0.0, 0.0, 0.0, -1.0], // negative-w identity
        // a normalized 45° about y: (0, sin22.5, 0, cos22.5)
        [0.0, 0.382_683_43, 0.0, 0.923_879_5],
    ];
    for _ in 0..240 {
        quats.push([
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
        ]);
    }

    for (idx, q) in quats.iter().enumerate() {
        poison(&mut emu, out_va);
        for (i, &v) in q.iter().enumerate() {
            emu.write_u32(q_va + (i as u64) * 4, v.to_bits());
        }

        emu.clear_calls();
        emu.call(0x74b6bb, 0, 0, &[out_va as u32, q_va as u32]);
        let bin = read_mat(&emu, out_va);

        let mut ours = [0f32; 16];
        m2_quat_to_mat4(&mut ours, q);

        for k in 0..16 {
            assert_f32(bin[k], ours[k], &format!("quat case {idx} elem {k}"));
        }
        checked += 1;
    }

    eprintln!("m2_quat_to_mat4: {checked} conversions bit-exact vs WoW.exe @ 0x74b6bb");
}
