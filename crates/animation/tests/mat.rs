//! The 4×4 products `0x74a7c6`, `0x7507bb` and `0x754a66`, the first-call selection of the math
//! slots, and the quaternion matrix `0x74b6bb` run whole
//! (`systems/models/animation.md#dispatched-matrix-math`, `systems/animation/pose.md#rotation-leg`)
//! and compared bit for bit with `m2_matmul4x4` and `m2_quat_to_mat4`. Both are `cdecl` bodies
//! called directly, `stack_args[0]` being `[ebp+8]`.

mod common;

use benilla_re_animation::mat::{m2_matmul4x4, m2_matmul4x4_base, m2_quat_to_mat4};
use benilla_re_cmath::multiply::matmul4x4_sse2;
use benilla_re_difftest::{Emu, Wow};
use common::{assert_f32, fuzz_f32, next};

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

type Product = fn(&mut [f32; 16], &[f32; 16], &[f32; 16]);

/// Runs the product at `va` whole beside `ours` over disjoint and aliased outputs, structured
/// matrices, matrices of the special values, and a fuzz sweep.
fn product_difftest(va: u64, ours_fn: Product, seed: u32, name: &str) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let arena = emu.alloc(0x100);
    // three disjoint 0x40-byte matrix slots; aliasing combos collapse pointers onto these.
    let slot_a = arena;
    let slot_b = arena + 0x40;
    let slot_c = arena + 0x80;

    let mut s = seed;
    let mut checked = 0u32;

    // structured magnitudes paired with each aliasing combo, then a fuzz sweep. `combo`:
    // 0 = all-disjoint, 1 = a==b, 2 = a==c, 3 = a==b==c.
    let structured: [([f32; 16], [f32; 16]); 4] = [
        (IDENTITY, IDENTITY),
        (IDENTITY, ZERO),
        (ZERO, IDENTITY),
        (IDENTITY, IDENTITY),
    ];

    // Specials: both zeros, the infinities, a NaN, a denormal, the largest and smallest normals,
    // and magnitudes that cancel, so the order of the sums shows.
    let specials: [f32; 12] = [
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(1),
        f32::MAX,
        f32::MIN_POSITIVE,
        1.0e8,
        -1.0e8,
        1.0,
        3.0e-8,
    ];

    for case in 0..480u32 {
        let combo = case % 4;
        // for a==b==c the single shared region must hold ONE matrix, so b and c coincide.
        let (b, c) = if case < 4 {
            structured[case as usize]
        } else if case >= 240 {
            let pick = |s: &mut u32| specials[(next(s) >> 8) as usize % specials.len()];
            let mut m = [[0f32; 16]; 2];
            for e in m.iter_mut().flat_map(|x| x.iter_mut()) {
                *e = if next(&mut s) >> 30 == 0 {
                    fuzz_f32(&mut s)
                } else {
                    pick(&mut s)
                };
            }
            (m[0], m[1])
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
        emu.call(va, 0, 0, &[a_va as u32, b_va as u32, c_va as u32]);
        let bin = read_mat(&emu, a_va);

        let mut ours = [0f32; 16];
        ours_fn(&mut ours, &b, &c);

        for k in 0..16 {
            assert_f32(
                bin[k],
                ours[k],
                &format!("{name} case {case} combo {combo} elem {k}"),
            );
        }
        checked += 1;
    }

    eprintln!("{name}: {checked} products bit-exact vs WoW.exe @ {va:#x}");
}

#[test]
fn m2_matmul4x4_difftest() {
    product_difftest(0x74a7c6, m2_matmul4x4, 0x74a7_c600, "m2_matmul4x4");
}

#[test]
fn m2_matmul4x4_base_difftest() {
    product_difftest(
        0x7507bb,
        m2_matmul4x4_base,
        0x7507_bb00,
        "m2_matmul4x4_base",
    );
}

#[test]
fn m2_matmul4x4_sse2_difftest() {
    product_difftest(0x754a66, matmul4x4_sse2, 0x754a_6600, "m2_matmul4x4_sse2");
}

/// `0x750930` stores `0x7507bb` at `[table + 0xc]` and nothing else of the product; the
/// first-call selection `0x74fb49` on the emulated processor, which reports SSE2 in `cpuid`
/// leaf 1 `edx` bit 26 and SSE in bit 25, leaves the SSE2 set: product `0x754a66`, inverse
/// `0x754cd2`, blend `0x74d11a`, rotation `0x74b6bb`.
#[test]
fn math_slots_selection_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let code = emu.alloc(0x10);
    // `mov eax,1; cpuid; mov eax,edx; ret`
    emu.write(code, &[0xb8, 1, 0, 0, 0, 0x0f, 0xa2, 0x89, 0xd0, 0xc3]);
    let edx = emu.call(code, 0, 0, &[]);
    assert_eq!(edx >> 25 & 3, 3, "emulated cpuid reports SSE and SSE2");

    let table = emu.alloc(0x100);
    for k in 0..0x40 {
        emu.write_u32(table + 4 * k, 0x74a7_c6c6);
    }
    emu.call(0x750930, 0, 0, &[table as u32]);
    let want = [
        (0x00, 0x75047f),
        (0x04, 0x750521),
        (0x08, 0x75060f),
        (0x0c, 0x7507bb),
        (0x10, 0x7508a0),
        (0x14, 0x7504ea),
        (0x18, 0x7505ae),
        (0x1c, 0x7506ef),
        (0x8c, 0x75060f),
    ];
    for k in 0..0x40u64 {
        let got = emu.read_u32(table + 4 * k);
        let exp = want
            .iter()
            .find(|w| w.0 == 4 * k)
            .map_or(0x74a7_c6c6, |w| w.1);
        assert_eq!(got, exp, "0x750930 table +{:#x}", 4 * k);
    }

    let mut emu = Emu::new(&wow);
    assert_eq!(
        benilla_re_difftest::select_math_slots(&mut emu),
        0x754a66,
        "product"
    );
    assert_eq!(emu.read_u32(0x876544), 0x754cd2, "inverse");
    assert_eq!(emu.read_u32(0x876588), 0x74d11a, "blend");
    assert_eq!(emu.read_u32(0x876594), 0x74b6bb, "rotation");
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
