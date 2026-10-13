//! `CMovement::SweepCarrierMove` (`0x6343b0`) from its entry to `0x634481`, before the box at
//! `0xc4e5a0` is united with the shifted one: the box and the push at `[ebp-0x24]`. The calls
//! inside, `TransformPoint` (`0x7bca80`), `IsVector3Finite` (`0x6337d0`) and `SetVector3`
//! (`0x4549a0`), run unstubbed.

use benilla_re_collision::carrier_sweep::{start_falling, StartFalling};
use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// A stack pointer inside the harness's stack, for slices that push.
const SLICE_ESP: u64 = 0x7008_0000;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A random `f32` in `[-scale, scale)`.
fn fz(s: &mut u32, scale: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    ((m * 2.0 - 1.0) * f64::from(scale)) as f32
}

/// Asserts the same bits, or NaN on both sides.
fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={binary} ({:#010x}) ours={ours} ({:#010x})",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// One case: the mover's position and sizes, the distance, the direction and the matrix.
struct Case {
    base: [f32; 3],
    mat: [f32; 16],
    ext_xy: f32,
    ext_z: f32,
    scale: f32,
    dir: [f32; 3],
}

/// The nine stored floats, over random and fixed cases: zero and negative sizes, signed zeros,
/// identity and translation matrices.
#[test]
fn carrier_sweep_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this_va = emu.alloc(0x200);
    let dir_va = emu.alloc(0x20);
    let mat_va = emu.alloc(0x80);

    let ident: [f32; 16] = {
        let mut m = [0.0f32; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[15] = 1.0;
        m
    };

    let translate: [f32; 16] = {
        let mut m = ident;
        m[12] = 10.0;
        m[13] = 20.0;
        m[14] = 30.0;
        m
    };

    let rot: [f32; 16] = [
        0.3, 1.2, -0.7, 4.0, -1.1, 0.6, 2.2, 5.0, 0.9, -0.4, 1.5, 6.0, 10.0, 20.0, 30.0, 1.0,
    ];

    let mut cases: Vec<Case> = vec![
        Case {
            base: [1.0, 2.0, 3.0],
            mat: ident,
            ext_xy: 0.5,
            ext_z: 2.0,
            scale: 1.5,
            dir: [0.1, 0.2, 0.3],
        },
        Case {
            base: [1.0, 2.0, 3.0],
            mat: ident,
            ext_xy: 0.0,
            ext_z: 2.0,
            scale: 1.0,
            dir: [1.0, 0.0, 0.0],
        },
        Case {
            base: [1.0, 2.0, 3.0],
            mat: ident,
            ext_xy: 0.5,
            ext_z: 0.0,
            scale: 1.0,
            dir: [0.0, 1.0, 0.0],
        },
        Case {
            base: [5.0, -5.0, 0.0],
            mat: ident,
            ext_xy: 0.0,
            ext_z: 0.0,
            scale: 0.0,
            dir: [0.0, 0.0, 1.0],
        },
        Case {
            base: [1.0, 2.0, 3.0],
            mat: ident,
            ext_xy: -0.5,
            ext_z: -2.0,
            scale: -1.0,
            dir: [0.3, 0.3, 0.3],
        },
        Case {
            base: [-1.0, -2.0, -3.0],
            mat: ident,
            ext_xy: 1.0,
            ext_z: 1.0,
            scale: 2.0,
            dir: [-0.5, 0.5, -0.5],
        },
        Case {
            base: [0.0, 0.0, 0.0],
            mat: ident,
            ext_xy: 0.0,
            ext_z: 0.0,
            scale: 1.0,
            dir: [-0.0, 0.0, -0.0],
        },
        Case {
            base: [1.0, 2.0, 3.0],
            mat: translate,
            ext_xy: 0.5,
            ext_z: 2.0,
            scale: 1.0,
            dir: [1.0, 1.0, 1.0],
        },
        Case {
            base: [1.0, -2.0, 0.5],
            mat: rot,
            ext_xy: 0.75,
            ext_z: 3.0,
            scale: 1.5,
            dir: [0.1, 0.2, 0.3],
        },
        Case {
            base: [-7.3, 4.1, 9.9],
            mat: rot,
            ext_xy: 2.5,
            ext_z: 0.25,
            scale: -3.0,
            dir: [-1.2, 0.4, 2.7],
        },
        Case {
            base: [-0.0, 0.0, -0.0],
            mat: ident,
            ext_xy: 0.0,
            ext_z: 0.0,
            scale: 1.0,
            dir: [0.0, 0.0, 0.0],
        },
        Case {
            base: [16000.0, -16000.0, 8000.0],
            mat: ident,
            ext_xy: 0.001,
            ext_z: 100.0,
            scale: 250.0,
            dir: [0.0001, -0.0002, 0.5],
        },
        Case {
            base: [12345.678, -9876.543, 4321.0],
            mat: rot,
            ext_xy: 33.0,
            ext_z: 0.5,
            scale: 0.001,
            dir: [9999.0, -9999.0, 1.0],
        },
        Case {
            base: [0.5, 0.5, 0.5],
            mat: ident,
            ext_xy: 1e-6,
            ext_z: 1e-6,
            scale: 1e6,
            dir: [1e-7, 2e-7, 3e-7],
        },
        Case {
            base: [f32::NAN, 0.0, 0.0],
            mat: ident,
            ext_xy: 0.5,
            ext_z: 2.0,
            scale: 1.0,
            dir: [1.0, 0.0, 0.0],
        },
        Case {
            base: [0.0, f32::INFINITY, 0.0],
            mat: ident,
            ext_xy: 0.5,
            ext_z: 2.0,
            scale: 1.0,
            dir: [0.0, 1.0, 0.0],
        },
        Case {
            base: [0.0, 0.0, f32::NEG_INFINITY],
            mat: ident,
            ext_xy: 0.5,
            ext_z: 2.0,
            scale: 1.0,
            dir: [0.0, 0.0, 1.0],
        },
        Case {
            base: [1.0, 2.0, 3.0],
            mat: ident,
            ext_xy: f32::NAN,
            ext_z: 2.0,
            scale: 1.0,
            dir: [1.0, 1.0, 1.0],
        },
        Case {
            base: [1.0, 2.0, 3.0],
            mat: ident,
            ext_xy: 0.5,
            ext_z: 2.0,
            scale: f32::NAN,
            dir: [1.0, 1.0, 1.0],
        },
    ];

    let mut s = 0x6343_b001u32;
    for _ in 0..3000 {
        let cscale = match next(&mut s) >> 8 & 3 {
            0 => 1.0,
            1 => 50.0,
            2 => 2000.0,
            _ => 16000.0,
        };
        let base = [fz(&mut s, cscale), fz(&mut s, cscale), fz(&mut s, cscale)];

        let mat = if next(&mut s) >> 8 & 1 == 0 {
            ident
        } else {
            let mut m = [0.0f32; 16];
            for v in m.iter_mut() {
                *v = fz(&mut s, 4.0);
            }

            m[12] = fz(&mut s, cscale);
            m[13] = fz(&mut s, cscale);
            m[14] = fz(&mut s, cscale);
            m
        };
        let escale = match next(&mut s) >> 8 & 3 {
            0 => 0.0,
            1 => 1.0,
            2 => 20.0,
            _ => 200.0,
        };
        let ext_xy = fz(&mut s, escale);
        let ext_z = fz(&mut s, escale);
        let smag = if next(&mut s) >> 8 & 1 == 0 {
            5.0
        } else {
            500.0
        };
        let scale = fz(&mut s, smag);
        let dir = [fz(&mut s, 2.0), fz(&mut s, 2.0), fz(&mut s, 2.0)];
        cases.push(Case {
            base,
            mat,
            ext_xy,
            ext_z,
            scale,
            dir,
        });
    }

    let mut asserts = 0usize;
    let ebp = SLICE_ESP - 4;
    for (ci, c) in cases.iter().enumerate() {
        for (k, v) in c.base.iter().enumerate() {
            emu.write_u32(this_va + 0x10 + 4 * k as u64, v.to_bits());
        }
        emu.write_u32(this_va + 0xb0, c.ext_xy.to_bits());
        emu.write_u32(this_va + 0xb4, c.ext_z.to_bits());
        for (k, v) in c.mat.iter().enumerate() {
            emu.write_u32(mat_va + 4 * k as u64, v.to_bits());
        }
        for (k, v) in c.dir.iter().enumerate() {
            emu.write_u32(dir_va + 4 * k as u64, v.to_bits());
        }

        for k in 0..6 {
            emu.write_u32(0xc4e5a0 + 4 * k, 0xdead_beef);
        }
        for k in 0..3u64 {
            emu.write_u32(ebp - 0x24 + 4 * k, 0xdead_beef);
        }

        emu.write_u32(SLICE_ESP + 4, c.scale.to_bits());
        emu.write_u32(SLICE_ESP + 8, dir_va as u32);
        emu.write_u32(SLICE_ESP + 0xc, mat_va as u32);
        emu.set_reg(RegisterX86::ECX, this_va);
        emu.set_reg(RegisterX86::ESP, SLICE_ESP);

        emu.run(0x6343b0, 0x634481);

        let bin = StartFalling {
            aabb_min: [
                emu.read_f32(0xc4e5a0),
                emu.read_f32(0xc4e5a4),
                emu.read_f32(0xc4e5a8),
            ],
            aabb_max: [
                emu.read_f32(0xc4e5ac),
                emu.read_f32(0xc4e5b0),
                emu.read_f32(0xc4e5b4),
            ],
            swept_disp: [
                emu.read_f32(ebp - 0x24),
                emu.read_f32(ebp - 0x20),
                emu.read_f32(ebp - 0x1c),
            ],
        };

        let ours = start_falling(&c.base, &c.mat, c.ext_xy, c.ext_z, c.scale, &c.dir);

        let ctx = format!(
            "case[{ci}] base={:?} ext_xy={} ext_z={} scale={} dir={:?}",
            c.base, c.ext_xy, c.ext_z, c.scale, c.dir
        );
        for k in 0..3 {
            assert_f32(
                bin.aabb_min[k],
                ours.aabb_min[k],
                &format!("{ctx} min[{k}]"),
            );
            assert_f32(
                bin.aabb_max[k],
                ours.aabb_max[k],
                &format!("{ctx} max[{k}]"),
            );
            assert_f32(
                bin.swept_disp[k],
                ours.swept_disp[k],
                &format!("{ctx} disp[{k}]"),
            );
        }
        asserts += 9;
    }

    eprintln!(
        "startfalling: {asserts} bit-exact asserts over {} cases (6 AABB + 3 swept-disp floats each)",
        cases.len()
    );
}
