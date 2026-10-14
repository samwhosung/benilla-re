//! `0x6cffc0` UpdateCloudTexture (`systems/lighting/clouds.md#cloud-texture`): the band of cloud
//! rows regenerated every 0.1 s, as a whole-function transcription over a flat image of the cloud
//! state.
//!
//! The function builds one lattice record per octave from the permutation table, walks the tile
//! toroidally adding fade-interpolated gradient noise (one row of octave records at a time) into a
//! float scanline buffer, and quantizes that buffer through the palette into the output byte tile.
//! The cloud-data refresh `0x6d0970`, the colour refresh `0x6cfb00` and the tile upload `0x58ac70`
//! are not run; [`CloudMem::stubs`] records their calls in order.
//!
//! Values are f64 on the x87 stack and round to f32 at each store. The palette index is the raw bit
//! pattern of `f32(v * 64 + 128 + 512)` shifted right by 14, low byte: not a numeric truncation.
//!
//! Tables: the permutation `0x86f2d0` is 256 bytes and every index is masked with `0xff`; the base
//! table `0x86f3dc` holds `u16` entries read at `[cfg+0x10] * 5 + column`; the gradient `0xce92d8`,
//! fade `0xce8dd8` and palette `0xce91d8` tables are built at run time and passed in.
//!
//! One octave record is 0x54 bytes, indexed from the array origin the three loops share:
//!
//! | off  | field |
//! |------|-------|
//! | 0x02 | `i16` rolling lattice key, advanced by the `i16` at 0x08 each step |
//! | 0x03 | `u8` current permutation key |
//! | 0x04 | `u16` row phase; the fade index A is its low byte, the lattice cell its high byte (0x05) |
//! | 0x06 | `u16` fade index B, the low byte of `[cfg+0xa0]` |
//! | 0x08 | `i16` per-column key step (`base_table[row + octave]`); 0x0a holds the per-row step |
//! | 0x10, 0x16 | `u16` lattice cell and its successor, set per row |
//! | 0x1a | `f32` octave amplitude, `1 / 2^octave` |
//! | 0x1e, 0x22, 0x26, 0x2a | `u32` gradient seeds x0, x1, y0, y1 |
//! | 0x2e, 0x36, 0x3e, 0x46 | `f32` corner gradients g00, g10, g01, g11; each followed by its delta |
//! | 0x4e | `u32` current key |
//! | 0x52 | `u32` key the corners were last built for, -1 at the start of a row |

/// `[0x8115b4]` = 0.1: the countdown `[cfg+0xb0]` stored back once the band regenerates.
pub const REGEN_RESET: u32 = 0x3dcc_cccd;
/// The constant stored to `[cfg+0xa8]` at the end (4.1666665).
pub const TAIL_A8: u32 = 0x4085_5555;
/// The constant stored to `[cfg+0xac]` at the end (8.333333).
pub const TAIL_AC: u32 = 0x4105_5555;

const REC: usize = 0x54;

/// One call to a function the transcription does not run, recorded in call order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stub {
    /// The callee: `0x6d0970`, `0x6cfb00` or `0x58ac70`.
    pub addr: u32,
    /// `ecx` at the call: `cfg` for the first two, `[cfg+0xb4]` for the upload.
    pub ecx: u32,
}

/// The cloud state as a flat little-endian byte image indexed as the function indexes `esi`. The
/// float tile (`+0x50`), the previous-row buffer (`+0x5c`), the derivative buffer (`+0x68`) and the
/// output byte tile (`+0x44`) live in the same image at addresses the caller chooses, so a
/// comparison with the emulator is a byte compare of `mem`.
pub struct CloudMem {
    /// The bytes.
    pub mem: Vec<u8>,
    /// The address `mem` begins at: an address is the index `va - base`.
    pub base: u32,
    /// The permutation table at `0x86f2d0`.
    pub perm: [u8; 256],
    /// The `u16` base table at `0x86f3dc`.
    pub base_table: Vec<u16>,
    /// The gradient table at `0xce92d8`, indexed by a permutation byte.
    pub gradient: Vec<f32>,
    /// The fade table at `0xce8dd8`, indexed by a byte.
    pub fade: Vec<f32>,
    /// The palette at `0xce91d8`.
    pub palette: [u8; 256],
    /// `[0xce9b9c]`: the decrement taken from `[cfg+0xb0]` on each call, as f32 bits.
    pub dec_ce9b9c: u32,
    /// `[0xce9bd0]`: copied to `[cfg+0xa4]` at the end.
    pub g_ce9bd0: u32,
    /// The calls the function made to the functions it does not run.
    pub stubs: Vec<Stub>,
}

impl CloudMem {
    fn off(&self, va: u32) -> usize {
        (va - self.base) as usize
    }
    fn r32(&self, va: u32) -> u32 {
        let i = self.off(va);
        u32::from_le_bytes([
            self.mem[i],
            self.mem[i + 1],
            self.mem[i + 2],
            self.mem[i + 3],
        ])
    }
    fn ri32(&self, va: u32) -> i32 {
        self.r32(va) as i32
    }
    fn rf32(&self, va: u32) -> f32 {
        f32::from_bits(self.r32(va))
    }
    fn rb(&self, va: u32) -> u8 {
        self.mem[self.off(va)]
    }
    fn w32(&mut self, va: u32, v: u32) {
        let i = self.off(va);
        self.mem[i..i + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn wf32(&mut self, va: u32, v: f32) {
        self.w32(va, v.to_bits());
    }
    fn w16(&mut self, va: u32, v: u16) {
        let i = self.off(va);
        self.mem[i..i + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn perm(&self, idx: u32) -> u8 {
        self.perm[(idx & 0xff) as usize]
    }
}

fn rec_r16(rec: &[u8], b: usize, off: usize) -> u16 {
    u16::from_le_bytes([rec[b + off], rec[b + off + 1]])
}
fn rec_ri16(rec: &[u8], b: usize, off: usize) -> i16 {
    rec_r16(rec, b, off) as i16
}
fn rec_r32(rec: &[u8], b: usize, off: usize) -> u32 {
    u32::from_le_bytes([
        rec[b + off],
        rec[b + off + 1],
        rec[b + off + 2],
        rec[b + off + 3],
    ])
}
fn rec_rf32(rec: &[u8], b: usize, off: usize) -> f32 {
    f32::from_bits(rec_r32(rec, b, off))
}
fn rec_w16(rec: &mut [u8], b: usize, off: usize, v: u16) {
    rec[b + off..b + off + 2].copy_from_slice(&v.to_le_bytes());
}
fn rec_w32(rec: &mut [u8], b: usize, off: usize, v: u32) {
    rec[b + off..b + off + 4].copy_from_slice(&v.to_le_bytes());
}
fn rec_wf32(rec: &mut [u8], b: usize, off: usize, v: f32) {
    rec_w32(rec, b, off, v.to_bits());
}

/// `0x6cffc0` UpdateCloudTexture (`systems/lighting/clouds.md#cloud-texture`), the whole function
/// over the image `mem` with the state at `cfg`. It returns at once when `[cfg+0x2c]` is 0. The
/// countdown `[cfg+0xb0]` loses `[0xce9b9c]` and is stored; `fcomp 0.0; test ah,0x41; je` returns
/// when the difference, still wide in st0, is above 0, so a difference of 0, below 0 or NaN
/// regenerates: the octave records are seeded, the float tile is cleared, every row and column adds
/// each octave's noise (the third octave also writes two slopes, `(previous - current) *
/// 2^([cfg+0x20] - 7)`, against the previous column and the previous row), the tile is quantized
/// through the palette (a negative index gives 0), then the scroll `[cfg+0x18]` advances by the
/// rows and wraps to 0, bumping `[cfg+0xa0]`, when it reaches the column count (`cmp; jb`,
/// unsigned).
pub fn cloud_coverage_noise(mem: &mut CloudMem, cfg: u32) {
    if mem.ri32(cfg + 0x2c) == 0 {
        return;
    }
    let dec = f32::from_bits(mem.dec_ce9b9c);
    let v_b0_live = f64::from(mem.rf32(cfg + 0xb0)) - f64::from(dec);
    let v_b0 = v_b0_live as f32;
    mem.wf32(cfg + 0xb0, v_b0);
    if v_b0_live > 0.0 {
        return;
    }
    mem.w32(cfg + 0xb0, REGEN_RESET);

    mem.stubs.push(Stub {
        addr: 0x006d_0970,
        ecx: cfg,
    });

    let seed = mem.rb(cfg + 0xa1);
    let local_1c = u32::from(mem.perm(u32::from(seed) + 1));
    let local_18 = u32::from(mem.perm(u32::from(seed)));

    let octaves = mem.r32(cfg + 0x28) as usize;
    let rows = mem.r32(cfg + 0x14) as usize;
    let cols = mem.r32(cfg + 0x1c) as usize;
    let stride_shift = mem.r32(cfg + 0x20) & 0x1f;
    let tile_x = mem.r32(cfg + 0x18);

    let mut rec = vec![0u8; octaves.max(1) * REC + REC];

    if octaves != 0 {
        let bt_row = (mem.ri32(cfg + 0x10) as usize) * 5;
        for c in 0..octaves {
            let b = c * REC;
            let key = u32::from(mem.base_table[bt_row + c]);
            rec_w16(&mut rec, b, 0x08, key as u16);
            rec_w16(&mut rec, b, 0x0a, key as u16);
            rec_w16(&mut rec, b, 0x0e, u16::from(seed));
            rec_w16(&mut rec, b, 0x12, u16::from(seed));
            rec_w16(
                &mut rec,
                b,
                0x04,
                (tile_x.wrapping_mul(key) & 0xffff) as u16,
            );
            rec_w16(&mut rec, b, 0x06, mem.r32(cfg + 0xa0) as u16);
            let v10 = ((key << stride_shift) >> 8)
                .wrapping_add(2)
                .wrapping_add(u32::from(seed));
            rec_w16(&mut rec, b, 0x14, v10 as u16);
            rec_w16(&mut rec, b, 0x18, (u32::from(seed) + 1) as u16);
            let denom = (1i32 << (c as u32 & 0x1f)) as f64;
            rec_wf32(&mut rec, b, 0x1a, (1.0f64 / denom) as f32);
        }
    }

    {
        let tile = mem.r32(cfg + 0x50);
        let start = tile.wrapping_add(tile_x.wrapping_mul(cols as u32).wrapping_mul(4));
        let count = (rows as u32).wrapping_mul(cols as u32) & 0x3fff_ffff;
        for k in 0..count {
            mem.w32(start.wrapping_add(k * 4), 0);
        }
    }

    for row in 0..rows {
        let ivar9 = (tile_x.wrapping_add(row as u32)) << stride_shift;
        let row_tile = mem.r32(cfg + 0x50).wrapping_add(ivar9.wrapping_mul(4));
        let row_deriv = mem.r32(cfg + 0x68).wrapping_add(ivar9.wrapping_mul(8));

        if octaves != 0 {
            for c in 0..octaves {
                let b = c * REC;
                let bv = rec[b + 0x05];
                let uv13 = u32::from(bv);
                rec_w16(&mut rec, b, 0x10, uv13 as u16);
                rec_w32(&mut rec, b, 0x1e, u32::from(mem.perm(local_18 + uv13)));
                let uv16 = u32::from(bv.wrapping_add(1));
                rec_w16(&mut rec, b, 0x16, uv16 as u16);
                rec_w32(&mut rec, b, 0x22, u32::from(mem.perm(local_18 + uv16)));
                rec_w32(&mut rec, b, 0x26, u32::from(mem.perm(uv13 + local_1c)));
                rec_w32(&mut rec, b, 0x2a, u32::from(mem.perm(uv16 + local_1c)));
                rec_w16(&mut rec, b, 0x02, mem.r32(cfg + 0xa0) as u16);
                rec_w32(&mut rec, b, 0x52, 0xffff_ffff);
            }
        }

        let mut prev_accum = 0.0f32;
        for col in 0..cols {
            let mut local_10 = 0u32;
            if octaves != 0 {
                for _oct in 0..octaves {
                    let b = (local_10 as usize) * REC;
                    let local_2c = u32::from(rec[b + 0x04]);
                    let local_28 = u32::from(rec[b + 0x06]);
                    let uvar14 = u32::from(rec[b + 0x03]);
                    rec_w32(&mut rec, b, 0x4e, uvar14);
                    if uvar14 != rec_r32(&rec, b, 0x52) {
                        rec_w32(&mut rec, b, 0x52, uvar14);
                        let s_x0 = rec_r32(&rec, b, 0x1e).wrapping_add(uvar14);
                        let g00 = mem.gradient[mem.perm(s_x0) as usize];
                        rec_wf32(&mut rec, b, 0x2e, g00);
                        rec_wf32(
                            &mut rec,
                            b,
                            0x32,
                            mem.gradient[mem.perm(s_x0 + 1) as usize] - g00,
                        );
                        let s_x1 = rec_r32(&rec, b, 0x22).wrapping_add(uvar14);
                        let g10 = mem.gradient[mem.perm(s_x1) as usize];
                        rec_wf32(&mut rec, b, 0x36, g10);
                        rec_wf32(
                            &mut rec,
                            b,
                            0x3a,
                            mem.gradient[mem.perm(s_x1 + 1) as usize] - g10,
                        );
                        let s_y0 = rec_r32(&rec, b, 0x26).wrapping_add(uvar14);
                        let g01 = mem.gradient[mem.perm(s_y0) as usize];
                        rec_wf32(&mut rec, b, 0x3e, g01);
                        rec_wf32(
                            &mut rec,
                            b,
                            0x42,
                            mem.gradient[mem.perm(s_y0 + 1) as usize] - g01,
                        );
                        let s_y1 = uvar14.wrapping_add(rec_r32(&rec, b, 0x2a));
                        let g11 = mem.gradient[mem.perm(s_y1) as usize];
                        rec_wf32(&mut rec, b, 0x46, g11);
                        rec_wf32(
                            &mut rec,
                            b,
                            0x4a,
                            mem.gradient[mem.perm(s_y1 + 1) as usize] - g11,
                        );
                    }
                    let xfade_idx = u32::from(rec_r16(&rec, b, 0x02)) & 0xff;
                    let f_x = f64::from(mem.fade[xfade_idx as usize]);
                    let g00 = f64::from(rec_rf32(&rec, b, 0x2e));
                    let g00d = f64::from(rec_rf32(&rec, b, 0x32));
                    let g10 = f64::from(rec_rf32(&rec, b, 0x36));
                    let g10d = f64::from(rec_rf32(&rec, b, 0x3a));
                    let g01 = f64::from(rec_rf32(&rec, b, 0x3e));
                    let g01d = f64::from(rec_rf32(&rec, b, 0x42));
                    let g11 = f64::from(rec_rf32(&rec, b, 0x46));
                    let g11d = f64::from(rec_rf32(&rec, b, 0x4a));
                    let f_a = f64::from(mem.fade[local_2c as usize]);
                    let f_b = f64::from(mem.fade[local_28 as usize]);

                    let v7 = f_x * g00d + g00;
                    let v8_f32 = f64::from((f_x * g01d + g01) as f32);
                    let v7b = ((f_x * g10d + g10) - v7) * f_a + v7;
                    let nk = rec_ri16(&rec, b, 0x02).wrapping_add(rec_ri16(&rec, b, 0x08));
                    rec_w16(&mut rec, b, 0x02, nk as u16);
                    let amp = f64::from(rec_rf32(&rec, b, 0x1a));
                    let acc_in = f64::from(mem.rf32(row_tile.wrapping_add(col as u32 * 4)));
                    let local_14 =
                        ((((f_x * g11d + g11) - v8_f32) * f_a + v8_f32 - v7b) * f_b + v7b) * amp
                            + acc_in;
                    let local_14_f32 = local_14 as f32;
                    mem.wf32(row_tile.wrapping_add(col as u32 * 4), local_14_f32);

                    if local_10 == 2 {
                        let scale = (1i32 << ((stride_shift.wrapping_sub(7)) & 0x1f)) as f64;
                        let d0 = ((f64::from(prev_accum) - f64::from(local_14_f32)) * scale) as f32;
                        mem.wf32(row_deriv.wrapping_add(col as u32 * 8), d0);
                        let prevrow = mem.r32(cfg + 0x5c);
                        let pr = f64::from(mem.rf32(prevrow.wrapping_add(col as u32 * 4)));
                        let cur = f64::from(mem.rf32(row_tile.wrapping_add(col as u32 * 4)));
                        let d1 = ((pr - cur) * scale) as f32;
                        mem.wf32(row_deriv.wrapping_add(col as u32 * 8 + 4), d1);
                        prev_accum = mem.rf32(row_tile.wrapping_add(col as u32 * 4));
                        let tv = mem.r32(row_tile.wrapping_add(col as u32 * 4));
                        mem.w32(prevrow.wrapping_add(col as u32 * 4), tv);
                    }
                    local_10 += 1;
                }
            }
        }

        if octaves != 0 {
            for c in 0..octaves {
                let b = c * REC;
                let nk = rec_ri16(&rec, b, 0x04).wrapping_add(rec_ri16(&rec, b, 0x0a));
                rec_w16(&mut rec, b, 0x04, nk as u16);
            }
        }
    }

    {
        let out_base0 = mem
            .r32(cfg + 0x44)
            .wrapping_add(tile_x.wrapping_mul(cols as u32));
        let src_base0 = mem
            .r32(cfg + 0x50)
            .wrapping_add(tile_x.wrapping_mul(cols as u32).wrapping_mul(4));
        let bias = u32::from(mem.rb(cfg + 8));
        let mut out_row = out_base0;
        let mut src_row = src_base0;
        for _row in 0..rows {
            for col in 0..cols {
                let v = mem.rf32(src_row.wrapping_add(col as u32 * 4));
                let q = ((f64::from(v) * 64.0 + 128.0 + 512.0) as f32).to_bits();
                let idx = ((q >> 0xe) & 0xff) as i32 - bias as i32;
                let out = if idx >= 0 {
                    mem.palette[idx as usize]
                } else {
                    0
                };
                let oi = mem.off(out_row.wrapping_add(col as u32));
                mem.mem[oi] = out;
            }
            out_row = out_row.wrapping_add(cols as u32);
            src_row = src_row.wrapping_add((cols as u32).wrapping_mul(4));
        }
    }

    mem.stubs.push(Stub {
        addr: 0x006c_fb00,
        ecx: cfg,
    });
    mem.stubs.push(Stub {
        addr: 0x0058_ac70,
        ecx: mem.r32(cfg + 0xb4),
    });

    let new_x = tile_x.wrapping_add(rows as u32);
    mem.w32(cfg + 0x18, new_x);
    if cols as u32 <= new_x {
        mem.w16(cfg + 0xa0, (mem.r32(cfg + 0xa0) as u16).wrapping_add(1));
        mem.w32(cfg + 0x18, 0);
    }

    mem.w32(cfg + 0xa4, mem.g_ce9bd0);
    mem.w32(cfg + 0xac, TAIL_AC);
    mem.w32(cfg + 0xa8, TAIL_A8);
}
