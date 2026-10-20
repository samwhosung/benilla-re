//! `ColourCloudBand` (`0x6cfb00`) and `BuildDiscQuad` (`0x6d1710`) whole, against
//! `sky_dome_vtx_color_build` and `sky_dome_geom_template`. `ColourCloudBand` runs with `ProjectCloudLight` (`0x6cf9c0`),
//! `ProjectToCloudTexture` (`0x6cf870`), `EvalSkyTrack` (`0x6cf6c0`) and the seed in the emulator;
//! the sun position and the lookup factor the transcription takes are read from the same functions
//! on the same state, and the colour buffer is compared word for word. `BuildDiscQuad` has its
//! first-use flag set so its immediates are skipped, and the scaled rows are compared.
//!
//! Host double (`docs/METHOD.md#the-harness`): `sky_dome_vtx_color_build_difftest` runs `fcos` at
//! `0x6cf8ab` and `0x6cf9dc` and `fpatan` at `0x741866`. The emulator computes each in host
//! `double`, so where its result reaches a compared value the host's math stands in for the x87's,
//! and the test shows the code around it.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::dn_sky_vtx::{
    sky_dome_frame, sky_dome_geom_template, sky_dome_vtx_color_build, SkyDomeObj,
};
use common::next;

// The globals `0x6cfb00` reads.
/// The time of day: picks the sun or the moon, and is the phase of the lookup.
const G_B64: u64 = 0xce9b64;
const DIR_LO: [u64; 3] = [0xce9858, 0xce985c, 0xce9860];
const DIR_HI: [u64; 3] = [0xce9878, 0xce987c, 0xce9880];
const ORIGIN: [u64; 3] = [0xce9b78, 0xce9b7c, 0xce9b80];
/// Eleven palette bytes.
const PALETTE: u64 = 0xce9c30;
const G_BCC: u64 = 0xce9bcc;
/// The track `0x6cf6c0` reads here, eight entries of eight bytes.
const G_LOOKUP: u64 = 0xce9ab8;

/// The corner rows the loop scales, at `0xce8be8 + 0xc·i`; output row `i` is at `out_a + 0xc·i`.
const TMPL_ROW0: u64 = 0xce8be8;
/// The block the function copies rows from, `0xce8bb8` to `0xce8be0`.
const COPY_SRC: u64 = 0xce8bb8;
const INIT_FLAG: u64 = 0xce8c30;

/// `0x6d1710` with the init flag set (the first-use blocks call the exit-hook thunk, which touches
/// CRT state this does not set up), the corner rows seeded where the loop reads them, and scales of
/// 0, -0, 1, negatives, halves, large and small, and a fuzz.
#[test]
fn sky_dome_geom_template_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let this = emu.alloc(0x40);
    let out_a = emu.alloc(0x100);
    let out_b = emu.alloc(0x100);
    let out_c = emu.alloc(0x100);
    let out14 = emu.alloc(0x10);
    let out18 = emu.alloc(0x10);
    let out1c = emu.alloc(0x10);

    let mut s = 0x7674_786d_u32;

    let run = |emu: &mut Emu, scale: f32, tmpl: [[f32; 3]; 6], ctx: &str| {
        emu.write(INIT_FLAG, &[0x03]);
        for (i, row) in tmpl.iter().enumerate() {
            for (k, &v) in row.iter().enumerate() {
                emu.write_u32(TMPL_ROW0 + 0x0c * i as u64 + k as u64 * 4, v.to_bits());
            }
        }
        for j in 0..12u64 {
            emu.write_u32(COPY_SRC + j * 4, 0x3f80_0000 | (j as u32));
        }
        emu.write_u32(this + 0x14, scale.to_bits());
        emu.write_u32(this + 0x0c, out_c as u32);
        for b in [out_a, out_b, out_c] {
            for j in 0..0x40u64 {
                emu.write_u32(b + j * 4, 0);
            }
        }
        emu.call(
            0x6d1710,
            this as u32,
            0,
            &[
                out_a as u32,
                out_b as u32,
                out_c as u32,
                out14 as u32,
                out18 as u32,
                out1c as u32,
            ],
        );

        let ours = sky_dome_geom_template(scale, &tmpl);

        for (i, orow) in ours.iter().enumerate() {
            let base = out_a + 0x0c * i as u64;
            for (k, &ov) in orow.iter().enumerate() {
                let got = emu.read_f32(base + k as u64 * 4);
                assert_eq!(
                    got.to_bits(),
                    ov.to_bits(),
                    "{ctx} row{i} ch{k}: binary={:#010x} ours={:#010x}",
                    got.to_bits(),
                    ov.to_bits()
                );
            }
        }
    };

    let cube: [[f32; 3]; 6] = [
        [0.0, -0.5, 0.5],
        [0.0, 0.5, 0.5],
        [0.0, -0.5, -0.5],
        [0.0, 0.5, -0.5],
        [0.0, -0.5, 99.0],
        [0.0, 0.5, 99.0],
    ];
    for sc in [0.0f32, 1.0, -1.0, 0.5, 2.0, 100.0, -3.5, 1e6, 1e-6, -0.0] {
        run(&mut emu, sc, cube, &format!("cube s={sc}"));
    }
    for case in 0..300 {
        let sc = {
            let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
            match (next(&mut s) >> 8) % 4 {
                0 => (m * 2.0 - 1.0) as f32,
                1 => (m * 200.0 - 100.0) as f32,
                2 => (m * 2.0e6 - 1.0e6) as f32,
                _ => (m * 1.0e-3) as f32,
            }
        };
        let mut t = [[0.0f32; 3]; 6];
        for row in &mut t {
            for v in row.iter_mut() {
                let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
                *v = (m * 200.0 - 100.0) as f32;
            }
        }
        run(&mut emu, sc, t, &format!("case {case}"));
    }
}

/// `0x6cfb00` over small grids. The table byte `t` is 0 at a first vertex (the word is kept) and
/// elsewhere (the previous word with alpha 0); the cosine is on both sides of 0; a bright palette
/// forces the clamp at 1; a zero palette gives zero; and the time of day runs across, onto and
/// either side of the two thresholds that pick the sun or the moon, and NaN.
#[test]
fn sky_dome_vtx_color_build_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // The buffers are allocated once; allocating per case would exhaust the arena.
    let this = emu.alloc(0x80);
    let color_buf = emu.alloc(0x1000);
    let table_buf = emu.alloc(0x400);
    let src_buf = emu.alloc(0x1000);
    let p_buf = emu.alloc(0x10);
    let v_buf = emu.alloc(0x10);
    let rp_out = emu.alloc(0x10);
    let uv_out = emu.alloc(0x10);

    // A track with a non-zero factor, so the intensity is non-zero and the sun term runs.
    for j in 0..8u64 {
        emu.write_u32(G_LOOKUP + j * 8, (0.1f32 * j as f32).to_bits());
        emu.write_u32(
            G_LOOKUP + j * 8 + 4,
            (0.4f32 + 0.06f32 * j as f32).to_bits(),
        );
    }

    let mut s = 0x7674_7863_u32;

    #[allow(clippy::too_many_arguments)]
    let run = |emu: &mut Emu,
               rows: u32,
               cols: u32,
               f18: u32,
               f20: u32,
               g_b64: f32,
               dir_lo: [f32; 3],
               dir_hi: [f32; 3],
               origin: [f32; 3],
               palette: [u8; 11],
               bcc: f32,
               table: &[u8],
               src: &[[f32; 2]],
               ctx: &str| {
        let count = (rows * cols) as usize;
        let start = (f18 as u64) << f20;
        assert!(start == 0, "test keeps the start offset 0");

        emu.write_u32(this + 0x14, rows);
        emu.write_u32(this + 0x18, f18);
        emu.write_u32(this + 0x1c, cols);
        emu.write_u32(this + 0x20, f20);
        emu.write_u32(this + 0x38, color_buf as u32);
        emu.write_u32(this + 0x44, table_buf as u32);
        emu.write_u32(this + 0x68, src_buf as u32);

        emu.write_u32(G_B64, g_b64.to_bits());
        for (g, v) in DIR_LO.iter().zip(dir_lo.iter()) {
            emu.write_u32(*g, v.to_bits());
        }
        for (g, v) in DIR_HI.iter().zip(dir_hi.iter()) {
            emu.write_u32(*g, v.to_bits());
        }
        for (g, v) in ORIGIN.iter().zip(origin.iter()) {
            emu.write_u32(*g, v.to_bits());
        }
        emu.write(PALETTE, &palette);
        emu.write_u32(G_BCC, bcc.to_bits());

        for (i, &t) in table.iter().enumerate().take(count) {
            emu.write(table_buf + i as u64, &[t]);
        }
        for (i, sp) in src.iter().enumerate().take(count) {
            emu.write_u32(src_buf + (i as u64) * 8, sp[0].to_bits());
            emu.write_u32(src_buf + (i as u64) * 8 + 4, sp[1].to_bits());
        }
        for j in 0..(count as u64 + 4) {
            emu.write_u32(color_buf + j * 4, 0);
        }

        let lookup_factor = emu.call_f64(0x6cf6c0, G_LOOKUP as u32, 8, &[g_b64.to_bits()]);
        let frame = sky_dome_frame(&palette, bcc, lookup_factor);

        let g = f64::from(g_b64);
        let lo = f64::from(f32::from_bits(0x3e6a_aaab)) - f64::from(f32::from_bits(0x3ce3_8e39));
        let hi = f64::from(f32::from_bits(0x3f65_5555)) + f64::from(f32::from_bits(0x3ce3_8e39));
        let pick_lo = lo <= g && hi >= g;
        let dir = if pick_lo { dir_lo } else { dir_hi };
        let d = [
            (f64::from(dir[0]) - f64::from(origin[0])) as f32,
            (f64::from(dir[1]) - f64::from(origin[1])) as f32,
            (f64::from(dir[2]) - f64::from(origin[2])) as f32,
        ];
        for o in [0u64, 4, 8] {
            emu.write_u32(p_buf + o, 0);
        }
        emu.write_u32(v_buf, d[0].to_bits());
        emu.write_u32(v_buf + 4, d[1].to_bits());
        emu.write_u32(v_buf + 8, d[2].to_bits());
        emu.call(0x6cf9c0, 0, 0, &[p_buf as u32, v_buf as u32, rp_out as u32]);
        emu.call(0x6cf870, this as u32, 0, &[rp_out as u32, uv_out as u32]);
        let sun_dir = [emu.read_f32(uv_out), emu.read_f32(uv_out + 4)];

        emu.call(0x6cfb00, this as u32, 0, &[]);

        let obj = SkyDomeObj {
            f14: rows,
            f18,
            f1c: cols,
            f20,
        };
        let prev = vec![[0u8; 4]; count];
        let ours = sky_dome_vtx_color_build(&obj, &frame, sun_dir, src, table, &prev);

        for (g, &ourg) in ours.iter().enumerate().take(count) {
            let got = emu.read_u32(color_buf + g as u64 * 4);
            let ov = u32::from_le_bytes(ourg);
            assert_eq!(got, ov, "{ctx} vtx{g}: binary={got:#010x} ours={ov:#010x}");
        }
    };

    let palette: [u8; 11] = [200, 180, 160, 220, 210, 190, 100, 120, 60, 70, 80];
    let mk_src = |n: usize, s: &mut u32| -> Vec<[f32; 2]> {
        (0..n)
            .map(|_| {
                let a = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
                let b = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
                [(a * 2.0 - 1.0) as f32, (b * 2.0 - 1.0) as f32]
            })
            .collect()
    };

    {
        let rows = 2u32;
        let cols = 3u32;
        let n = (rows * cols) as usize;
        let table: Vec<u8> = vec![10, 50, 128, 200, 255, 99];
        let src = mk_src(n, &mut s);
        run(
            &mut emu,
            rows,
            cols,
            0,
            0,
            0.5,
            [0.1, 0.2, 0.3],
            [0.4, 0.5, 0.6],
            [0.05, 0.0, -0.1],
            palette,
            0.4,
            &table,
            &src,
            "edge mid",
        );
    }
    {
        let rows = 1u32;
        let cols = 4u32;
        let n = (rows * cols) as usize;
        let table: Vec<u8> = vec![0, 80, 0, 200];
        let src = mk_src(n, &mut s);
        run(
            &mut emu,
            rows,
            cols,
            0,
            0,
            0.5,
            [0.1, 0.2, 0.3],
            [0.4, 0.5, 0.6],
            [0.0, 0.0, 0.0],
            palette,
            0.3,
            &table,
            &src,
            "edge t0",
        );
    }
    {
        let rows = 1u32;
        let cols = 3u32;
        let n = (rows * cols) as usize;
        let bright: [u8; 11] = [255; 11];
        let table: Vec<u8> = vec![1, 128, 254];
        let src = mk_src(n, &mut s);
        run(
            &mut emu,
            rows,
            cols,
            0,
            0,
            0.5,
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            bright,
            0.1,
            &table,
            &src,
            "edge bright",
        );
    }
    {
        let rows = 1u32;
        let cols = 3u32;
        let n = (rows * cols) as usize;
        let zero: [u8; 11] = [0; 11];
        let table: Vec<u8> = vec![40, 90, 160];
        let src = mk_src(n, &mut s);
        run(
            &mut emu,
            rows,
            cols,
            0,
            0,
            0.5,
            [0.2, 0.3, 0.4],
            [0.5, 0.6, 0.7],
            [0.0, 0.0, 0.0],
            zero,
            0.5,
            &table,
            &src,
            "edge zero",
        );
    }
    let lo =
        (f64::from(f32::from_bits(0x3e6a_aaab)) - f64::from(f32::from_bits(0x3ce3_8e39))) as f32;
    let hi =
        (f64::from(f32::from_bits(0x3f65_5555)) + f64::from(f32::from_bits(0x3ce3_8e39))) as f32;
    let mut phases = vec![-1.0f32, 0.0, 0.2, 0.5, 0.9, 1.5, f32::NAN, f32::INFINITY];
    for base in [lo, hi] {
        for d in [-2i32, -1, 0, 1, 2] {
            phases.push(f32::from_bits((base.to_bits() as i32 + d) as u32));
        }
    }
    for &g in &phases {
        let rows = 1u32;
        let cols = 3u32;
        let n = (rows * cols) as usize;
        let table: Vec<u8> = vec![30, 130, 230];
        let src = mk_src(n, &mut s);
        run(
            &mut emu,
            rows,
            cols,
            0,
            0,
            g,
            [0.1, 0.2, 0.3],
            [0.7, 0.8, 0.9],
            [0.0, 0.0, 0.0],
            palette,
            0.35,
            &table,
            &src,
            &format!("edge g={g}"),
        );
    }

    for case in 0..120 {
        let rows = 1 + (next(&mut s) >> 8) % 3;
        let cols = 1 + (next(&mut s) >> 8) % 4;
        let n = (rows * cols) as usize;
        let g_b64 = {
            let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
            (m * 2.0 - 0.5) as f32
        };
        let mut pal = [0u8; 11];
        for p in &mut pal {
            *p = (next(&mut s) >> 16) as u8;
        }
        let bcc = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32;
        let table: Vec<u8> = (0..n)
            .map(|i| {
                let is_first_of_row = (i % cols as usize) == 0;
                if (next(&mut s) >> 8).is_multiple_of(5) && !is_first_of_row {
                    0
                } else {
                    1 + ((next(&mut s) >> 16) as u8 % 255)
                }
            })
            .collect();
        let src = mk_src(n, &mut s);
        let dir_lo = [
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
        ];
        let dir_hi = [
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
        ];
        run(
            &mut emu,
            rows,
            cols,
            0,
            0,
            g_b64,
            dir_lo,
            dir_hi,
            [0.0, 0.0, 0.0],
            pal,
            bcc,
            &table,
            &src,
            &format!("case {case}"),
        );
    }
}
