//! `PlaneFromTriangle` (`0x637480`) run whole on edge triangles and random ones within ±1000:
//! `__thiscall`, ecx = out, then the three points.

use benilla_re_collision::resolve_toolkit::plane_from_points;
use benilla_re_difftest::{Emu, Wow};

/// The LCG; random floats are drawn whole from its bits and kept when finite.
struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn f32(&mut self) -> f32 {
        loop {
            let f = f32::from_bits(self.next());
            if f.is_finite() {
                return f;
            }
        }
    }
}

/// Asserts the same bits, or NaN on both sides.
fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

#[test]
fn plane_from_tri_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let a_p = emu.alloc(0x10);
    let b_p = emu.alloc(0x10);
    let c_p = emu.alloc(0x10);
    let out_p = emu.alloc(0x40);

    let mut rng = Lcg(0x1234_5678);
    let edge_tris: Vec<[[f32; 3]; 3]> = vec![
        [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        [[10.0, 5.0, 2.0], [10.0, 5.0, 9.0], [3.0, 5.0, 2.0]], // flat in y
        [[-5.0, -5.0, -5.0], [5.0, -4.0, -6.0], [-4.0, 6.0, -5.5]],
    ];
    let mut count = 0;
    for i in 0..80 {
        let tri = if i < edge_tris.len() {
            edge_tris[i]
        } else {
            [
                [rng.f32() % 1e3, rng.f32() % 1e3, rng.f32() % 1e3],
                [rng.f32() % 1e3, rng.f32() % 1e3, rng.f32() % 1e3],
                [rng.f32() % 1e3, rng.f32() % 1e3, rng.f32() % 1e3],
            ]
        };
        for (p, v) in [(a_p, tri[0]), (b_p, tri[1]), (c_p, tri[2])] {
            for (k, &comp) in v.iter().enumerate() {
                emu.write_u32(p + 4 * k as u64, comp.to_bits());
            }
        }
        emu.call(
            0x0063_7480,
            out_p as u32,
            0,
            &[a_p as u32, b_p as u32, c_p as u32],
        );
        let mut bin = [0f32; 4];
        for (k, slot) in bin.iter_mut().enumerate() {
            *slot = emu.read_f32(out_p + 4 * k as u64);
        }
        let p = plane_from_points(&tri[0], &tri[1], &tri[2]);
        let ours = [p.n[0], p.n[1], p.n[2], p.d];
        for k in 0..4 {
            assert_f32(bin[k], ours[k], &format!("plane[{k}] tri#{i}"));
        }
        count += 1;
    }
    assert!(count > 10);
}
