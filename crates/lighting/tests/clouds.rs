//! `0x6cffc0` UpdateCloudTexture against the exe, whole function. A cloud state and its four
//! buffers are laid out in the emulator and in the flat image `CloudMem` at the same addresses; the
//! table fixtures are written to their fixed addresses, the three callees are stubbed, and after
//! the call the whole arena is compared byte for byte, along with the order and `ecx` of the
//! stubbed calls.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::clouds::{cloud_coverage_noise, CloudMem, Stub};
use common::next;

/// The gradient table, built at run time.
const GRADIENT_VA: u64 = 0x00ce_92d8;
/// The fade table, built at run time.
const FADE_VA: u64 = 0x00ce_8dd8;
/// The palette, built at run time.
const PALETTE_VA: u64 = 0x00ce_91d8;
/// The permutation table, read from the image.
const PERM_VA: u64 = 0x0086_f2d0;
/// The base table, read from the image.
const BASE_TABLE_VA: u64 = 0x0086_f3dc;
/// The countdown decrement.
const DEC_VA: u64 = 0x00ce_9b9c;
/// Copied to `[cfg+0xa4]`.
const CE9BD0_VA: u64 = 0x00ce_9bd0;
/// The refresh argument, which the stub ignores.
const CE9C64_VA: u64 = 0x00ce_9c64;
/// The size of the working set and its four buffers.
const ARENA: usize = 0x10000;

/// The cloud-data refresh, stubbed.
const FN_REFRESH: u64 = 0x006d_0970;
/// The colour refresh, stubbed.
const FN_CHEAP: u64 = 0x006c_fb00;
/// The tile upload, stubbed.
const FN_UPLOAD: u64 = 0x0058_ac70;

/// A fuzzed value in [0, 1.25).
fn fuzz_unit(s: &mut u32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (m * 1.25) as f32
}

/// 400 randomized states over 2 to 256 columns, 2 to 5 rows, 1 to 4 octaves and varied scroll, seed
/// and bias. The countdown gate takes a positive remainder (no regeneration), exactly 0, -0, NaN
/// and a negative remainder, and `[cfg+0x2c] = 0` returns early.
#[test]
fn cloud_coverage_noise_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    emu.stub(FN_REFRESH, 0, 1);
    emu.stub(FN_CHEAP, 0, 0);
    emu.stub(FN_UPLOAD, 0, 4);

    let mut perm = [0u8; 256];
    for (i, p) in perm.iter_mut().enumerate() {
        *p = emu.read_u32(PERM_VA + i as u64) as u8;
    }
    let base_table: Vec<u16> = (0..256)
        .map(|i| emu.read_u32(BASE_TABLE_VA + 2 * i as u64) as u16)
        .collect();

    let base = emu.alloc(ARENA as u64) as u32;
    let cfg = base;
    let float_tile = base + 0x0400;
    let prev_row = base + 0x4000;
    let deriv = base + 0x4400;
    let out_tile = base + 0xa400;

    let mut s = 0x636c_6f75u32;
    let mut zero_clear_seen = false;
    let mut wrap_seen = false;
    let mut deriv_seen = false;

    for case in 0..400 {
        let stride_shift = 1 + (next(&mut s) >> 8) % 8;
        let cols = 1u32 << stride_shift;
        let rows = 2 + (next(&mut s) >> 8) % 4;
        let octaves = 1 + (next(&mut s) >> 8) % 4;
        let idx10 = (next(&mut s) >> 8) % 8;
        let seed_a1 = (next(&mut s) >> 8) as u8;
        let a0 = (next(&mut s) >> 8) as u16 & 0xff;
        let tile_x = (next(&mut s) >> 8) % 8;
        let countdown_fires = case % 5 != 0;
        let coverage_2c = case % 7 == 0;
        let bias = (next(&mut s) >> 8) as u8 % 4;

        let mut gradient = vec![0f32; 256];
        let mut fade = vec![0f32; 256];
        let mut palette = [0u8; 256];
        for g in gradient.iter_mut() {
            *g = fuzz_unit(&mut s) * 2.0 - 1.0;
        }
        for f in fade.iter_mut() {
            *f = fuzz_unit(&mut s).clamp(0.0, 1.0);
        }
        for p in palette.iter_mut() {
            *p = (next(&mut s) >> 8) as u8;
        }
        let (b0_bits, dec_bits) = match (countdown_fires, case % 4) {
            (false, 0) => (0x3d4c_cccdu32, 0x0000_0000u32),
            (false, _) => (0x0000_0001, 0x0000_0000),
            (true, 0) => (0x3d4c_cccd, 0x3dcc_cccd),
            (true, 1) => (0x3dcc_cccd, 0x3dcc_cccd),
            (true, 2) => (0x7fc0_0000, 0x3dcc_cccd),
            (true, _) => (0x8000_0000, 0x0000_0000),
        };
        let ce9bd0 = next(&mut s);

        for (i, g) in gradient.iter().enumerate() {
            emu.write_u32(GRADIENT_VA + 4 * i as u64, g.to_bits());
        }
        for (i, f) in fade.iter().enumerate() {
            emu.write_u32(FADE_VA + 4 * i as u64, f.to_bits());
        }
        emu.write(PALETTE_VA, &palette);
        emu.write_u32(DEC_VA, dec_bits);
        emu.write_u32(CE9BD0_VA, ce9bd0);
        emu.write_u32(CE9C64_VA, 0xdead_beef);

        let mut m = CloudMem {
            mem: vec![0u8; ARENA],
            base,
            perm,
            base_table: base_table.clone(),
            gradient: gradient.clone(),
            fade: fade.clone(),
            palette,
            dec_ce9b9c: dec_bits,
            g_ce9bd0: ce9bd0,
            stubs: Vec::new(),
        };

        let zeros = vec![0u8; ARENA];
        emu.write(base as u64, &zeros);

        let set32 = |emu: &mut Emu, m: &mut CloudMem, off: u32, v: u32| {
            emu.write_u32((base + off) as u64, v);
            m.mem[off as usize..off as usize + 4].copy_from_slice(&v.to_le_bytes());
        };
        let set8 = |emu: &mut Emu, m: &mut CloudMem, off: u32, v: u8| {
            emu.write(u64::from(base + off), &[v]);
            m.mem[off as usize] = v;
        };

        set32(&mut emu, &mut m, 0xb0, b0_bits);
        set32(&mut emu, &mut m, 0x2c, if coverage_2c { 0 } else { 1 });
        set32(&mut emu, &mut m, 0x28, octaves);
        set32(&mut emu, &mut m, 0x14, rows);
        set32(&mut emu, &mut m, 0x1c, cols);
        set32(&mut emu, &mut m, 0x20, stride_shift);
        set32(&mut emu, &mut m, 0x18, tile_x);
        set32(&mut emu, &mut m, 0x10, idx10);
        set8(&mut emu, &mut m, 0xa1, seed_a1);
        set32(&mut emu, &mut m, 0xa0, u32::from(a0));
        set8(&mut emu, &mut m, 0x08, bias);
        set32(&mut emu, &mut m, 0x50, float_tile);
        set32(&mut emu, &mut m, 0x5c, prev_row);
        set32(&mut emu, &mut m, 0x68, deriv);
        set32(&mut emu, &mut m, 0x44, out_tile);
        set32(&mut emu, &mut m, 0xb4, 0xcafe_0000 | case);

        emu.clear_calls();
        emu.call(0x006c_ffc0, cfg, 0, &[]);

        cloud_coverage_noise(&mut m, cfg);

        let fired = !coverage_2c && countdown_fires;
        if coverage_2c || !countdown_fires {
            assert!(
                m.stubs.is_empty(),
                "case {case}: model ran the body on an early-out path"
            );
        }

        let bin_arena = {
            let mut b = vec![0u8; ARENA];
            for w in 0..ARENA / 4 {
                let v = emu.read_u32(base as u64 + (w * 4) as u64);
                b[w * 4..w * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            b
        };
        if bin_arena != m.mem {
            let i = bin_arena
                .iter()
                .zip(&m.mem)
                .position(|(a, b)| a != b)
                .unwrap();
            let region = match i as u32 {
                x if x < 0x100 => format!("struct +0x{:x}", x),
                x if (float_tile - base..prev_row - base).contains(&x) => {
                    format!("float_tile +0x{:x}", x - (float_tile - base))
                }
                x if (prev_row - base..deriv - base).contains(&x) => {
                    format!("prev_row +0x{:x}", x - (prev_row - base))
                }
                x if (deriv - base..out_tile - base).contains(&x) => {
                    format!("deriv +0x{:x}", x - (deriv - base))
                }
                x => format!("out_tile +0x{:x}", x - (out_tile - base)),
            };
            panic!(
                "case {case} (cols={cols} rows={rows} oct={octaves} tx={tile_x} idx10={idx10} \
                 seed=0x{seed_a1:x} fired={fired}): first diff at arena+0x{i:x} ({region}): \
                 binary=0x{:02x} ours=0x{:02x}",
                bin_arena[i], m.mem[i]
            );
        }

        if fired {
            assert_eq!(
                m.stubs,
                vec![
                    Stub {
                        addr: FN_REFRESH as u32,
                        ecx: cfg
                    },
                    Stub {
                        addr: FN_CHEAP as u32,
                        ecx: cfg
                    },
                    Stub {
                        addr: FN_UPLOAD as u32,
                        ecx: 0xcafe_0000 | case
                    },
                ],
                "case {case}: stub invocation mismatch"
            );
            if (rows * cols) & 0x3fff_ffff != 0 {
                zero_clear_seen = true;
            }
            if cols <= tile_x + rows {
                wrap_seen = true;
            }
            if octaves >= 3 {
                deriv_seen = true;
            }
        }
    }
    assert!(
        zero_clear_seen && wrap_seen && deriv_seen,
        "the clear, the scroll wrap and the derivative leg all ran"
    );
}
