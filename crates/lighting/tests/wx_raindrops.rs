//! The per-drop quad geometry of `0x676160` against the exe: one slice from `0x676410` to
//! `0x676932` over the loop body, on a frame, a drop record and a vertex buffer laid out in the
//! emulator, with the vector store `0x4549a0` run for real. The fade gate (the exe's outcome is
//! read from whether vertex A was written) and every stored value are compared bit for bit.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::wx_raindrops::{rain_drop_quad, DropQuad};
use common::next;

/// Compare two f32 bit for bit, any two NaN equal.
fn assert_field(binary: f32, ours: f32, tag: &str, field: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{tag}.{field}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// A fuzzed f32: random bits, and finite values of several scales and signs.
fn fuzz(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let bits = next(s);
    match pick {
        0 => f32::from_bits(bits),
        1 => (f64::from(bits >> 4) / f64::from(1u32 << 20) - 128.0) as f32,
        2 => (f64::from(bits >> 8) / f64::from(1u32 << 23) * 2000.0 - 1000.0) as f32,
        3 => (f64::from(bits >> 8) / f64::from(1u32 << 23) * 2.0 - 1.0) as f32,
        4 => f32::from_bits(0x3f80_0000 | (bits >> 9)),
        _ => -f32::from_bits(0x3f00_0000 | (bits >> 9)),
    }
}

/// Drops inside, outside and on the edges of the fade window (`t` equal to `f2edge` and to the high
/// limit), zero-length vectors, -0, NaN and huge positions, and a fuzz over bands around the window
/// and over the whole bit space, with the identity and random texture transforms.
#[test]
#[allow(clippy::too_many_lines)]
fn rain_drop_quad_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400) + 0x200;
    let esp_top = frame + 0x140;
    let drop_rec = emu.alloc(0x80) + 0x40;
    let vbuf = emu.alloc(0x400) + 0x80;

    let mut s = 0x5744_4e53_u32;
    let mut emits = 0u32;

    let run = |emu: &mut Emu,
               drop_pos: [f32; 3],
               drop_v: [f32; 3],
               f2edge: f32,
               f3edge: f32,
               cam: [f32; 3],
               wind: f32,
               t: f32,
               mat: [[f32; 3]; 3],
               tag: &str,
               emits: &mut u32| {
        emu.write_u32(frame - 0x88, cam[0].to_bits());
        emu.write_u32(frame - 0x84, cam[1].to_bits());
        emu.write_u32(frame - 0x80, cam[2].to_bits());
        emu.write_u32(frame - 0x90, wind.to_bits());
        emu.write_u32(frame - 0x18, t.to_bits());
        emu.write_u32(frame - 0x9c, 0.5f32.to_bits());
        emu.write_u32(frame - 0x64, mat[0][0].to_bits());
        emu.write_u32(frame - 0x60, mat[0][1].to_bits());
        emu.write_u32(frame - 0x5c, mat[0][2].to_bits());
        emu.write_u32(frame - 0x70, mat[1][0].to_bits());
        emu.write_u32(frame - 0x6c, mat[1][1].to_bits());
        emu.write_u32(frame - 0x68, mat[1][2].to_bits());
        emu.write_u32(frame - 0x7c, mat[2][0].to_bits());
        emu.write_u32(frame - 0x78, mat[2][1].to_bits());
        emu.write_u32(frame - 0x74, mat[2][2].to_bits());

        emu.write_u32(drop_rec - 0x14, drop_pos[0].to_bits());
        emu.write_u32(drop_rec - 0x10, drop_pos[1].to_bits());
        emu.write_u32(drop_rec - 0xc, drop_pos[2].to_bits());
        emu.write_u32(drop_rec - 0x8, drop_v[0].to_bits());
        emu.write_u32(drop_rec - 0x4, drop_v[1].to_bits());
        emu.write_u32(drop_rec, drop_v[2].to_bits());
        emu.write_u32(drop_rec + 0x8, f2edge.to_bits());
        emu.write_u32(drop_rec + 0xc, f3edge.to_bits());

        for off in [
            0x48u64, 0x44, 0x40, 0x114, 0x110, 0x10c, 0x24, 0x20, 0x1c, 0x14, 0x10, 0xc, 0x30,
            0x2c, 0x28, 0x108, 0x104, 0x100, 0xcc, 0xc8, 0xc4, 0xfc, 0xf8, 0xf4, 0x144, 0x140,
            0x13c, 0x4,
        ] {
            emu.write_u32(frame - off, 0xdead_beef);
        }
        emu.write_u32(frame - 0x58, 0xdead_beef);
        emu.write_u32(frame - 0x54, 0xdead_beef);
        emu.write_u32(frame - 0x50, 0xdead_beef);

        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.set_reg(RegisterX86::EDI, drop_rec);
        emu.set_reg(RegisterX86::ESI, vbuf);
        emu.set_reg(RegisterX86::EBX, vbuf + 0xc);
        emu.run(0x676410, 0x676932);

        let (emit, ours) = rain_drop_quad(drop_pos, drop_v, f2edge, f3edge, cam, wind, t, mat);

        let bin_delta = [
            emu.read_f32(frame - 0x58),
            emu.read_f32(frame - 0x54),
            emu.read_f32(frame - 0x50),
        ];
        for (k, &bd) in bin_delta.iter().enumerate() {
            assert_field(bd, ours.delta[k], tag, &format!("delta[{k}]"));
        }

        let bin_emit = emu.read_u32(frame - 0x48) != 0xdead_beef;
        assert_eq!(
            bin_emit, emit,
            "{tag}: the exe's fade gate disagrees with the transcription"
        );
        if emit {
            *emits += 1;
            let binary = DropQuad {
                delta: bin_delta,
                span: ours.span,
                span2: emu.read_f32(frame - 0x4),
                vertex_a: [
                    emu.read_f32(frame - 0x48),
                    emu.read_f32(frame - 0x44),
                    emu.read_f32(frame - 0x40),
                ],
                vertex_b: [
                    emu.read_f32(frame - 0x114),
                    emu.read_f32(frame - 0x110),
                    emu.read_f32(frame - 0x10c),
                ],
                n_a: [
                    emu.read_f32(frame - 0x24),
                    emu.read_f32(frame - 0x20),
                    emu.read_f32(frame - 0x1c),
                ],
                n_f: [
                    emu.read_f32(frame - 0x14),
                    emu.read_f32(frame - 0x10),
                    emu.read_f32(frame - 0xc),
                ],
                cr: [
                    emu.read_f32(frame - 0x30),
                    emu.read_f32(frame - 0x2c),
                    emu.read_f32(frame - 0x28),
                ],
                corner0: [
                    emu.read_f32(frame - 0x108),
                    emu.read_f32(frame - 0x104),
                    emu.read_f32(frame - 0x100),
                ],
                corner1: [
                    emu.read_f32(frame - 0xcc),
                    emu.read_f32(frame - 0xc8),
                    emu.read_f32(frame - 0xc4),
                ],
                proj: [
                    emu.read_f32(frame - 0xfc),
                    emu.read_f32(frame - 0xf8),
                    emu.read_f32(frame - 0xf4),
                ],
                matvec: [
                    emu.read_f32(frame - 0x144),
                    emu.read_f32(frame - 0x140),
                    emu.read_f32(frame - 0x13c),
                ],
                finalv: [
                    emu.read_f32(vbuf + 0x28),
                    emu.read_f32(vbuf + 0x2c),
                    emu.read_f32(vbuf + 0x30),
                ],
            };
            assert_field(binary.span2, ours.span2, tag, "span2");
            for k in 0..3 {
                assert_field(
                    binary.vertex_a[k],
                    ours.vertex_a[k],
                    tag,
                    &format!("vA[{k}]"),
                );
                assert_field(
                    binary.vertex_b[k],
                    ours.vertex_b[k],
                    tag,
                    &format!("vB[{k}]"),
                );
                assert_field(binary.n_a[k], ours.n_a[k], tag, &format!("nA[{k}]"));
                assert_field(binary.n_f[k], ours.n_f[k], tag, &format!("nF[{k}]"));
                assert_field(binary.cr[k], ours.cr[k], tag, &format!("cr[{k}]"));
                assert_field(binary.corner0[k], ours.corner0[k], tag, &format!("c0[{k}]"));
                assert_field(binary.corner1[k], ours.corner1[k], tag, &format!("c1[{k}]"));
                assert_field(binary.proj[k], ours.proj[k], tag, &format!("proj[{k}]"));
                assert_field(binary.matvec[k], ours.matvec[k], tag, &format!("mv[{k}]"));
                assert_field(binary.finalv[k], ours.finalv[k], tag, &format!("fin[{k}]"));
            }
        }
    };

    let ident: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    type P = ([f32; 3], [f32; 3], f32, f32, [f32; 3], f32, f32);
    let probes: [P; 12] = [
        (
            [10.0, 20.0, 30.0],
            [1.0, 2.0, 3.0],
            0.0,
            1.0,
            [1.0, 2.0, 3.0],
            0.5,
            0.5,
        ),
        (
            [5.0, 6.0, 7.0],
            [1.0, 0.0, 0.0],
            0.0,
            1.0,
            [0.0, 0.0, 0.0],
            1.0,
            0.0,
        ),
        (
            [5.0, 6.0, 7.0],
            [0.0, 1.0, 1.0],
            0.0,
            1.0,
            [0.0, 0.0, 0.0],
            1.0,
            1.0,
        ),
        (
            [1.0, 2.0, 3.0],
            [2.0, 2.0, 2.0],
            0.0,
            1.0,
            [1.0, 2.0, 3.0],
            0.5,
            0.5,
        ),
        (
            [5.0, 5.0, 5.0],
            [0.0, 0.0, 0.0],
            0.0,
            1.0,
            [0.0, 0.0, 0.0],
            0.0,
            0.5,
        ),
        (
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            0.0,
            1.0,
            [0.0, 0.0, 0.0],
            0.0,
            0.0,
        ),
        (
            [5.0, 6.0, 7.0],
            [1.0, 1.0, 1.0],
            -1.0,
            1.0,
            [0.0, 0.0, 0.0],
            0.5,
            -0.5,
        ),
        (
            [5.0, 6.0, 7.0],
            [1.0, 1.0, 1.0],
            0.0,
            0.5,
            [0.0, 0.0, 0.0],
            0.5,
            2.0,
        ),
        (
            [5.0, 6.0, 7.0],
            [-0.0, -0.0, -0.0],
            0.0,
            1.0,
            [-0.0, -0.0, -0.0],
            0.0,
            0.5,
        ),
        (
            [f32::NAN, 1.0, 2.0],
            [1.0, 1.0, 1.0],
            0.0,
            1.0,
            [0.0, 0.0, 0.0],
            0.5,
            0.5,
        ),
        (
            [5.0, 6.0, 7.0],
            [1.0, 1.0, 1.0],
            0.0,
            1.0,
            [0.0, 0.0, 0.0],
            0.5,
            f32::NAN,
        ),
        (
            [1.0e20, -1.0e20, 1.0e20],
            [1.0e10, 1.0e10, 1.0e10],
            0.0,
            1.0,
            [0.0, 0.0, 0.0],
            0.5,
            0.5,
        ),
    ];
    for &(fp, fv, f2, f3, cam, wind, t) in &probes {
        run(
            &mut emu, fp, fv, f2, f3, cam, wind, t, ident, "probe", &mut emits,
        );
    }

    for _ in 0..600 {
        let fp = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let fv = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let cam = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let (f2, f3, wind, t) = if (next(&mut s) >> 8) & 1 == 0 {
            let band =
                |s: &mut u32| (f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 2.0 - 0.5) as f32;
            (band(&mut s), band(&mut s) + 1.0, fuzz(&mut s), band(&mut s))
        } else {
            (fuzz(&mut s), fuzz(&mut s), fuzz(&mut s), fuzz(&mut s))
        };
        let mat = if (next(&mut s) >> 8) & 1 == 0 {
            ident
        } else {
            [
                [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
                [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
                [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            ]
        };
        run(
            &mut emu, fp, fv, f2, f3, cam, wind, t, mat, "fuzz", &mut emits,
        );
    }
    assert!(
        emits > 50,
        "too few emit-path cases ({emits}) — gate band miscalibrated"
    );
}
