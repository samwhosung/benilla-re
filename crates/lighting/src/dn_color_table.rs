//! `BuildLightTable` (`0x6d2260`, `systems/lighting/daynight.md#light-table-build`): the build of
//! the colour, sky and fog table at `0xce9c0c` from the Light records around the camera. It runs
//! the band gather `0x6d64d0`, the day blend `0x6d67c0` (`dn_sky_blend::sky_weighted_blend`), the
//! area-light drain `0x6d2d00` with its heap and the overblend `0x6d30e0`
//! (`dn_sky_blend::cloud_record_overblend`), then `BuildLightColours` and the three colour scales.
//! The band evaluations are `dn_bands`, the colour scale and `BuildLightColours` are `dn_color`.
//!
//! The light state is a set of globals in the exe; here the build takes the ones it reads in
//! `DnInput`, with the container arrays it dereferences, and returns every field it writes in
//! `DnOutput`. The x87 runs at 53 bits, `fstp dword` stores `f32`, and `fistp` rounds to nearest,
//! ties to even.

use crate::dn_bands::{dn_band_lerp_bgra, dn_band_lerp_scalar, dn_track_eval};
use crate::dn_color::{dn_color_to_bgra, dn_fog_derive, FogDerive, FogPrior};
use crate::dn_sky_blend::{cloud_record_overblend, color_blend_leaf, sky_weighted_blend};
use crate::x87::fistp;

/// `[0x811630]`, 2880: half-minutes in a day.
const C_2880: f32 = f32::from_bits(0x4534_0000);
/// `[0x86f2c8]`, 0.5.
const C_HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x81162c]`, −30.
const C_NEG30: f32 = f32::from_bits(0xc1f0_0000);
/// `[0x811628]`, −1/30.
const C_NEG_1_30: f32 = f32::from_bits(0xbd08_8889);
/// `[0x8029b0]`, 0.25.
const C_QUARTER: f32 = f32::from_bits(0x3e80_0000);
/// `[0x8012cc]`, 0.75.
const C_0_75: f32 = f32::from_bits(0x3f40_0000);
/// `[0x7ffe58]`, 255.
const C_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x8029cc]`, 512.
const C_512: f32 = f32::from_bits(0x4400_0000);
/// `[0x7ff9d8]`, 1.
const C_ONE: f32 = 1.0;

/// The bits of `x` stored as `f32`.
#[inline]
fn f32bits(x: f64) -> u32 {
    (x as f32).to_bits()
}

/// The table slot of a global in `0xce9c0c..0xce9c98`: slot `k` is `0xce9c0c + 4k`, and the same
/// offset in a record.
#[inline]
fn slot(va: u32) -> usize {
    ((va - 0xce9c0c) / 4) as usize
}

/// The half-minute of the day: `phase·2880` stored as `f32`, minus 0.5, `fistp`: `round(f32(phase ·
/// 2880) − 0.5)`, ties to even, so an exact odd integer comes out one low; NaN and values beyond
/// `i32` give `0x80000000`.
fn half_minute(phase: f32) -> i32 {
    let scaled = (f64::from(phase) * f64::from(C_2880)) as f32;
    let biased = f64::from(scaled) - f64::from(C_HALF);
    fistp(biased)
}

/// A band as `0x6d62e0` and `0x6d6480` read it: the count, the keys in half-minutes and the values
/// (a packed colour, or `f32` bits). A missing band is the lerps' empty case.
#[derive(Clone, Debug, Default)]
pub struct BandSub {
    pub count: i32,
    pub keys: Vec<i32>,
    pub values: Vec<u32>,
}

/// A LightParams row (the gather's `[esp+4]`). The struct's `+0x1c..` IS the index array
/// (`0x6d6ab0` reads `[row+0x1c+4k]`), and the gather's direct copies `+0x70`/`+0x74` read
/// `[row+0x1c]`/`[row+0x20]` — i.e. `idx[0]`/`idx[1]` (the same memory). So: `group_id`
/// (`[row+0]`); `w4` (`[row+4]` i32 → f32 to slot `+0x50`); `w8`/`wc` (`[row+8]`/ `[row+0xc]` →
/// slots `+0x78`/`+0x88`); `w10_18` (`[row+0x10..+0x18]`, 3 dwords → slots `+0x64..+0x6c`); `idx`
/// (`[row+0x1c..]`, where `idx[0]`/`idx[1]` also feed slots `+0x70`/`+0x74`).
#[derive(Clone, Debug, Default)]
pub struct LightParamsRow {
    pub group_id: i32,
    pub w4: i32,
    pub w8: u32,
    pub wc: u32,
    pub w10_18: [u32; 3],
    pub idx: [i32; 4],
}

/// The band/handle containers the gather + column helpers dereference (the binary's `{ptr,bound}`
/// global pairs); every index is bounds-checked exactly as the helpers do.
#[derive(Clone, Debug, Default)]
pub struct Containers {
    /// `{0xce9d7c, 0xce9d80}` — the handle array of LightParams rows (`0x6d6ab0`).
    pub rows: Vec<LightParamsRow>,
    pub rows_bound: i32,
    /// `{0xce9da4, 0xce9da8}` — the color-band container (`0x6d6ad0`).
    pub color_bands: Vec<BandSub>,
    pub color_bound: i32,
    /// `{0xce9d90, 0xce9d94}` — the scalar-band container (`0x6d6b00`).
    pub scalar_bands: Vec<BandSub>,
    pub scalar_bound: i32,
}

/// `0x6d6ab0` (ecx=row, edx=k): `i = row.idx[k]`; `0 ≤ i ≤ rows_bound` → `rows[i]`, else None.
fn col_row<'a>(c: &'a Containers, row: &LightParamsRow, k: usize) -> Option<&'a LightParamsRow> {
    let i = row.idx[k];
    if i < 0 || i > c.rows_bound {
        return None;
    }
    c.rows.get(i as usize)
}

/// `0x6d6ad0` (ecx=row, edx=bandIdx): `i = bandIdx + 18*groupId − 17`; `0 ≤ i ≤ color_bound` →
/// `color_bands[i]`, else None.
fn color_band(c: &Containers, group_id: i32, band_idx: i32) -> Option<&BandSub> {
    let i = band_idx + 18 * group_id - 17;
    if i < 0 || i > c.color_bound {
        return None;
    }
    c.color_bands.get(i as usize)
}

/// `0x6d6b00` (ecx=row, edx=scalarIdx): `i = scalarIdx + 6*groupId − 5`; `0 ≤ i ≤ scalar_bound` →
/// `scalar_bands[i]`, else None.
fn scalar_band(c: &Containers, group_id: i32, scalar_idx: i32) -> Option<&BandSub> {
    let i = scalar_idx + 6 * group_id - 5;
    if i < 0 || i > c.scalar_bound {
        return None;
    }
    c.scalar_bands.get(i as usize)
}

/// One color band at `half_min` via `dn_band_lerp_bgra` (`0x6d62e0`): null → `0xff000000`.
fn eval_color(band: Option<&BandSub>, half_min: i32) -> u32 {
    match band {
        None => 0xff00_0000,
        Some(b) => dn_band_lerp_bgra(&b.keys, &b.values, b.count, half_min),
    }
}

/// One scalar band at `half_min` via `dn_band_lerp_scalar` (`0x6d6480`); the gather caller `fstp
/// dword`s the st0 result → f32. Null → `0.0`.
fn eval_scalar(band: Option<&BandSub>, half_min: i32) -> u32 {
    let v = match band {
        None => 0.0f32,
        Some(b) => {
            let values: Vec<f32> = b.values.iter().map(|&x| f32::from_bits(x)).collect();
            dn_band_lerp_scalar(&b.keys, &values, b.count, half_min) as f32
        }
    };
    v.to_bits()
}

/// The 0x90-byte gather record: 0x24 dwords (color slots packed, scalar/f32 slots raw bits).
type Record = [u32; 0x24];

/// `0x6d64d0` — `dn_band_gather`: fill a fresh record from `(half_min, row)`. 18 color bands (with
/// the idx-8 → +0x4c discontinuity), 6 scalar bands, then the direct row copies (`daynight.md#light-record`).
fn gather(c: &Containers, row: &LightParamsRow, half_min: i32) -> Record {
    let mut r: Record = [0; 0x24];

    // Color bands: idx 0..7 → +0x00..+0x1c; idx 8 → +0x4c; idx 9..0x11 → +0x20..+0x40.
    let color_off = |idx: i32| -> usize {
        match idx {
            0..=7 => (idx as usize) * 4,
            8 => 0x4c,
            9..=0x11 => 0x20 + (idx as usize - 9) * 4,
            _ => unreachable!(),
        }
    };
    for idx in 0..=0x11i32 {
        r[color_off(idx) / 4] = eval_color(color_band(c, row.group_id, idx), half_min);
    }

    // Scalar bands → +0x44, +0x48, +0x54, +0x58, +0x5c, +0x60.
    const SCALAR_OFF: [usize; 6] = [0x44, 0x48, 0x54, 0x58, 0x5c, 0x60];
    for (idx, &off) in SCALAR_OFF.iter().enumerate() {
        r[off / 4] = eval_scalar(scalar_band(c, row.group_id, idx as i32), half_min);
    }

    // Direct row copies (0x6d6761..0x6d67ac).
    r[0x50 / 4] = (row.w4 as f32).to_bits(); // fild [row+4]; fstp [+0x50]
    r[0x64 / 4] = row.w10_18[0]; // [row+0x10]
    r[0x68 / 4] = row.w10_18[1]; // [row+0x14]
    r[0x6c / 4] = row.w10_18[2]; // [row+0x18]
    r[0x70 / 4] = row.idx[0] as u32; // [row+0x1c] == idx[0]
    r[0x74 / 4] = row.idx[1] as u32; // [row+0x20] == idx[1]
    r[0x78 / 4] = row.w8; // [row+8]
    r[0x7c / 4] = C_ONE.to_bits(); // 1.0
    r[0x88 / 4] = row.wc; // [row+0xc]
    r[0x8c / 4] = C_ONE.to_bits(); // 1.0
    r
}

/// `0x69eb70`: the replace at weight 0xff, `(dst & 0xff000000) | (src & 0xffffff)`.
fn replace24(dst: u32, src: u32) -> u32 {
    (dst & 0xff00_0000) | (src & 0x00ff_ffff)
}

/// Every global `BuildLightTable` reads, and the arrays it dereferences.
#[derive(Clone, Debug, Default)]
pub struct DnInput {
    /// `[0xc7f288]`, the mode word; its low nibble is 0 or 1 for the day path, 2 or 3 for a gather
    /// of one row, and 0xf (the whole word) forces the full path.
    pub mode: u32,
    /// `[0xce9b64]`, the time of day as a fraction.
    pub phase: f32,
    /// `[0xce98dc]`, the light count.
    pub light_count: i32,
    /// The lights `[0xce98e0]` points to; light 0 gives the row the subject is gathered from.
    pub lights: Vec<LightRow>,
    /// `[0xce9bac]`, an override row; none when absent.
    pub params_bac: Option<LightParamsRow>,
    /// `[0xce9bb0]`: -1 copies the whole record, any other value only the tail.
    pub bb0: i32,
    /// `[0xce9bcc]`, the day blend weight `UpdateStormWeight` (`0x6d4500`) writes. The blend runs
    /// when it is above 0; the alpha stamp runs when it is not equal to 0 (NaN included).
    pub bcc: f32,
    /// The camera `[0xce9b78]`, `[0xce9b7c]`, `[0xce9b80]`; z also sets the sun ramp.
    pub cam: [f32; 3],
    /// `[0xce9ba4]`, the override blend byte, and the packed override colour at `[0xce9ba5]`.
    pub ba4: u8,
    pub ba5: u32,
    /// The band and row containers.
    pub containers: Containers,
    /// The skybox remap: `[0xce9db4]` bounds an id (0 skips the lookup), `[0xce9dbc]` the table.
    pub remap_bound: i32,
    pub remap_hi: i32,
    /// The tracks behind `GetNightFraction` (`0x6d4860`, four entries) and `0x6d4890` (two).
    pub track4: [[f32; 2]; 4],
    pub track2: [[f32; 2]; 2],
    /// The light state `BuildLightColours` copies; its alpha byte `[0xce9c5a]` comes from the
    /// table.
    pub fog_prior: FogPrior,
    /// `[0xce9bd0]` before the build: the first colour scale reads it; the other two read what
    /// `BuildLightColours` wrote.
    pub prior_bd0: u32,
}

/// A light as `0xce98e0`'s pointers lead to it. `group_id` and `idx` are read as a Light row's
/// (`[+0]`, `[+0x1c + 4k]` by `0x6d6ab0`); `pos` (`+0x8`, `+0xc`, `+0x10`) and the inner and outer
/// radii (`+0x14`, `+0x18`) are read by the drain `0x6d2d00` and `0x6d6a90`.
#[derive(Clone, Debug, Default)]
pub struct LightRow {
    pub group_id: i32,
    pub idx: [i32; 4],
    pub pos: [f32; 3],
    pub inner: f32,
    pub outer: f32,
}

/// Every field `BuildLightTable` writes.
#[derive(Clone, Debug, PartialEq)]
pub struct DnOutput {
    /// `0xce9c0c..0xce9c98`: the table, 0x24 dwords.
    pub table: [u32; 0x24],
    /// `[0xce9bd0]`, `[0xce9cd8]`, `[0xce9cd4]`: the three colour scales' targets.
    pub bd0: u32,
    pub cd8: u32,
    pub cd4: u32,
    /// `[0xce9864]`, `[0xce97c0]`, `[0xce9884]`, `[0xce9710]`: the broadcast colour.
    pub bcast: [u32; 4],
    /// The alpha byte stamped into byte 3 of those and `[0xce98a7]`; none when not stamped.
    pub alpha_bytes: Option<u8>,
    /// `[0xce9bbc]`, `[0xce9bc0]`, `[0xce9bc4]`, `[0xce9bc8]`: the handle and weight of the two
    /// skybox pairs, and `[0xce9bb4]`, `[0xce9bb8]`.
    pub bbc: [u32; 4],
    pub bb4: u32,
    pub bb8: u32,
    /// `[0xce9d0c]`, `[0xce9d10]`: the two track samples.
    pub d0c: u32,
    pub d10: u32,
    /// What `BuildLightColours` wrote.
    pub fog: FogDerive,
}

/// What `InternSkybox` (`0x6d4640`) returns in the test, which stubs it: the same token reaches the
/// output wherever the handle is stored.
pub const SKYBOX_STUB_TOKEN: u32 = 0x6d46_4040;

/// The max-heap `0x6d2d00` keeps the lights inside their outer radius in: the exe's own object,
/// slots 1 and up with slot 0 unused, so the size counts that slot. Entries are `{distance,
/// light}`.
struct LightHeap {
    slots: Vec<(f32, usize)>,
}

impl LightHeap {
    fn new() -> Self {
        LightHeap {
            slots: vec![(0.0, 0)],
        }
    }

    /// `0x6d2dad` to `0x6d2e0c`: appends and sifts up. A new distance moves above a parent unless
    /// it is below it (`fcomp`, `test ah,1; jne`, which also stops on NaN), so equals move up.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    fn push(&mut self, dist: f32, light: usize) {
        self.slots.push((dist, light));
        let mut i = self.slots.len() - 1;
        while i > 1 {
            let parent = i >> 1;
            if !(dist >= self.slots[parent].0) {
                break;
            }
            self.slots[i] = self.slots[parent];
            i = parent;
        }
        self.slots[i] = (dist, light);
    }

    /// `0x6d2e40` to `0x6d2ed7`: takes slot 1, moves the last slot out and sifts it down from the
    /// root. The larger child is the right one when it is not below the left (`test ah,1; jne`
    /// keeps the left on a smaller or unordered right); the moved entry stops when it is not below
    /// that child.
    fn pop(&mut self) -> (f32, usize) {
        let root = self.slots[1];
        let last = *self.slots.last().expect("a slot");
        self.slots.pop();
        let size = self.slots.len();
        if size >= 2 {
            let n = size - 1;
            let half = n >> 1;
            let mut i = 1;
            if half >= i {
                loop {
                    let mut child = 2 * i;
                    if child < n && self.slots[child + 1].0 >= self.slots[child].0 {
                        child += 1;
                    }
                    if last.0 >= self.slots[child].0 {
                        break;
                    }
                    self.slots[i] = self.slots[child];
                    i = child;
                    if child > half {
                        break;
                    }
                }
            }
            self.slots[i] = last;
        }
        root
    }
}

/// `0x6d2260()`, over `DnInput`.
///
/// - The nibble is `mode & 0xf`; the day path is nibble 0 or 1. With no lights and no override row
///   the table is the no-data default.
/// - A nibble of 2 or 3 with the mode not 0xf gathers one row (element 7 or 6 of the rows) and goes
///   to fog; other nibbles go to fog with the table untouched.
/// - Otherwise the subject row is resolved through light 0 (`0x6d6ab0`) and gathered at the
///   half-minute `round(f32(phase·2880) − 0.5)`; with `bcc > 0` the second row is gathered and
///   `sky_weighted_blend` mixes it in. The drain `0x6d2d00` pushes lights 1 up with `dist < outer`
///   (strictly; `jp` after `test ah,5`) into the heap, where `dist` is `√((dz² + dy²) + dx²)` of
///   the camera minus the light, stored as `f32`; then pops while the slot count is above 1,
///   gathering and blending each popped light's rows and overblending the record by `w = 1` when
///   `dist` is not above the inner radius (`test ah,0x41; jne`, NaN included), otherwise `1 − (dist
///   − inner)/(outer − inner)`, stored as `f32`.
/// - `bb0 == -1` copies the record; otherwise the tail comes from the override row or the subject
///   row's gather, and a skybox id in `(0, remap_bound)` interns a handle.
/// - Then `BuildLightColours` runs from the table. On the day path the sun ramp is 1, or, at a full
///   mode word of 1, `1 − t·(−1/30)` with `t` the camera z: 0 above 0, −30 at or below −30 or NaN,
///   else z; three colour scales follow. A non-zero override byte blends its colour into the
///   targets. The alpha byte `((1 − bcc)·255 + 512)` bits `>> 14` is stamped when `bcc != 0`. Two
///   skybox pairs are interned for ids that are non-zero, below the bound, with weights above 0.
///   The two tracks are sampled.
pub fn dn_color_table_build(inp: &DnInput) -> DnOutput {
    let nibble = (inp.mode & 0xf) as i32;
    let want_day = nibble == 0 || nibble == 1;

    let mut bbc = [0u32; 4];
    let mut bb4 = 0u32;
    let mut bb8 = 0u32;
    let mut table: [u32; 0x24] = [0; 0x24];

    let no_data = inp.light_count == 0 && inp.params_bac.is_none();

    if no_data {
        table = [0xffff_ffff; 0x24];
        table[slot(0xce9c60)] = 0;
        table[slot(0xce9c64)] = 0;
        table[slot(0xce9c68)] = 0;
        table[slot(0xce9c54)] = 0x3f00_0000;
        table[slot(0xce9c50)] = 0x5015_02f9;
        table[slot(0xce9c5c)] = 0;
        table[slot(0xce9c6c)] = 0;
        table[slot(0xce9c10)] = 0xff40_4040;
        table[slot(0xce9c70)] = 0x3f00_0000;
        table[slot(0xce9c7c)] = 0x3f40_0000;
        table[slot(0xce9c80)] = 0x3f80_0000;
        table[slot(0xce9c74)] = 0x3f00_0000;
        table[slot(0xce9c78)] = 0x3f80_0000;
        table[slot(0xce9c94)] = 0;
        table[slot(0xce9c84)] = 0;
        table[slot(0xce9c8c)] = 0;
    } else if !want_day && inp.mode != 0xf {
        // A row past the bound reads a null row in the exe, which faults, so the tests keep it
        // within the bound.
        if nibble == 2 || nibble == 3 {
            let half_min = half_minute(inp.phase);
            let row = if nibble == 2 {
                if inp.containers.rows_bound >= 7 {
                    inp.containers.rows.get(7).cloned().unwrap_or_default()
                } else {
                    LightParamsRow::default()
                }
            } else if inp.containers.rows_bound >= 6 {
                inp.containers.rows.get(6).cloned().unwrap_or_default()
            } else {
                LightParamsRow::default()
            };
            table = gather(&inp.containers, &row, half_min);
        }
    } else {
        let half_min = half_minute(inp.phase);
        let l0 = inp.lights.first().cloned().unwrap_or_default();
        let base_row = LightParamsRow {
            group_id: l0.group_id,
            idx: l0.idx,
            ..Default::default()
        };
        let k = usize::from(want_day);
        let subject = col_row(&inp.containers, &base_row, k)
            .cloned()
            .unwrap_or_default();
        let mut rec = gather(&inp.containers, &subject, half_min);

        if inp.bcc > 0.0 {
            let k2 = if want_day { 3 } else { 2 };
            let sec_row = col_row(&inp.containers, &base_row, k2)
                .cloned()
                .unwrap_or_default();
            let sec = gather(&inp.containers, &sec_row, half_min);
            sky_weighted_blend(&mut rec, &sec, inp.bcc);
        }

        let mut heap = LightHeap::new();
        for i in 1..(inp.light_count.max(0) as usize) {
            if let Some(l) = inp.lights.get(i) {
                let dx = f64::from(inp.cam[0]) - f64::from(l.pos[0]);
                let dy = f64::from(inp.cam[1]) - f64::from(l.pos[1]);
                let dz = f64::from(inp.cam[2]) - f64::from(l.pos[2]);
                let dist = ((dz * dz + dy * dy) + dx * dx).sqrt() as f32;
                if dist < l.outer {
                    heap.push(dist, i);
                }
            }
        }
        while heap.slots.len() > 1 {
            let (dist, li) = heap.pop();
            let l = &inp.lights[li];
            let light_row = LightParamsRow {
                group_id: l.group_id,
                idx: l.idx,
                ..Default::default()
            };
            let prow = col_row(&inp.containers, &light_row, k)
                .cloned()
                .unwrap_or_default();
            let mut popped = gather(&inp.containers, &prow, half_min);
            if inp.bcc > 0.0 {
                let k2 = if want_day { 3 } else { 2 };
                let srow = col_row(&inp.containers, &light_row, k2)
                    .cloned()
                    .unwrap_or_default();
                let sec = gather(&inp.containers, &srow, half_min);
                sky_weighted_blend(&mut popped, &sec, inp.bcc);
            }
            let w = if dist > l.inner {
                let num = f64::from(dist) - f64::from(l.inner);
                let den = f64::from(l.outer) - f64::from(l.inner);
                (f64::from(C_ONE) - num / den) as f32
            } else {
                C_ONE
            };
            cloud_record_overblend(&mut rec, &popped, w);
        }

        if inp.bb0 == -1 {
            table = rec;
        } else {
            let params = inp
                .params_bac
                .clone()
                .or_else(|| col_row(&inp.containers, &base_row, k).cloned());
            if let Some(params) = params {
                let tg = gather(&inp.containers, &params, half_min);
                table[slot(0xce9c70)] = tg[0x64 / 4];
                table[slot(0xce9c74)] = tg[0x68 / 4];
                table[slot(0xce9c78)] = tg[0x6c / 4];
                table[slot(0xce9c7c)] = tg[0x70 / 4];
                table[slot(0xce9c80)] = tg[0x74 / 4];
                table[slot(0xce9c84)] = tg[0x78 / 4];
                table[slot(0xce9c88)] = tg[0x7c / 4];
                table[slot(0xce9c8c)] = tg[0x80 / 4];
                table[slot(0xce9c90)] = tg[0x84 / 4];
                table[slot(0xce9c94)] = tg[0x88 / 4];
                table[slot(0xce9c98)] = tg[0x8c / 4];
                let id = params.w8 as i32;
                if id != 0 && id < inp.remap_bound {
                    bb4 = SKYBOX_STUB_TOKEN;
                    bb8 = 0x3f80_0000;
                }
            }
        }
    }

    // The fog alpha byte `[0xce9c5a]` is byte 2 of the table slot `0xce9c58`.
    let fog_alpha = ((table[slot(0xce9c58)] >> 16) & 0xff) as u8;
    let fog = dn_fog_derive(table[0], table[1], fog_alpha, &inp.fog_prior);

    // `BuildLightColours` has just written `[0xce9cd4]` and `[0xce9cd8]`; the scales read those.
    // `[0xce9bd0]` is untouched by it.
    let mut bd0 = inp.prior_bd0;
    let mut cd8 = fog.cd8;
    let mut cd4 = fog.cd4;

    // The sun ramp fires at a full mode word of 1 (`mov eax,esi; dec eax; jne` at `0x6d2817`).
    // The camera z is clamped by the ladder at `0x6d2823`: above 0 gives 0, at or below −30 and NaN
    // give −30, otherwise z.
    let mut ramp = C_ONE;
    if want_day && inp.mode == 1 {
        let z = inp.cam[2];
        let t = if z > 0.0 {
            0.0f32
        } else if z > C_NEG30 {
            z
        } else {
            C_NEG30
        };
        ramp = (f64::from(C_ONE) - f64::from(t) * f64::from(C_NEG_1_30)) as f32;
    }

    if want_day {
        let fac = (f64::from(ramp) + f64::from(C_ONE)) * f64::from(C_HALF);
        let fac = fac as f32;
        bd0 = dn_color_to_bgra(bd0, fac);
        cd8 = dn_color_to_bgra(cd8, fac);
        let fac3 = (f64::from(ramp) * f64::from(C_QUARTER) + f64::from(C_0_75)) as f32;
        cd4 = dn_color_to_bgra(cd4, fac3);
    }

    // The broadcast colour is table slot `0xce9c2c`; a non-zero override byte blends the override
    // colour into five targets, `0x690ae0` for the first and the replace `0x69eb70` at 0xff.
    let base = table[slot(0xce9c2c)];
    let mut bcast = [base, base, base, base];
    let a = u32::from(inp.ba4);
    if a != 0 {
        let src = inp.ba5;
        bd0 = color_blend_leaf(bd0, src, a);
        let leg = |dst: u32| {
            if a == 0xff {
                replace24(dst, src)
            } else {
                color_blend_leaf(dst, src, a)
            }
        };
        cd8 = leg(cd8);
        cd4 = leg(cd4);
        bcast[0] = leg(bcast[0]);
        bcast[2] = leg(bcast[2]);
    }

    // The alpha stamp gate at `0x6d2c2c` is `test ah,0x44; jnp`: it skips only `bcc == 0`.
    let alpha_bytes = if inp.bcc != 0.0 {
        let v =
            f32bits((f64::from(C_ONE) - f64::from(inp.bcc)) * f64::from(C_255) + f64::from(C_512));
        let a8 = ((v >> 14) & 0xff) as u8;
        for b in bcast.iter_mut() {
            *b = (*b & 0x00ff_ffff) | (u32::from(a8) << 24);
        }
        Some(a8)
    } else {
        None
    };

    // The two pairs at `0xce9c84` and `0xce9c8c` (stride 8): the handle goes to `[0xce9bbc]` and
    // `[0xce9bc0]`, the weight to `[0xce9bc4]` and `[0xce9bc8]`.
    {
        let pairs = [(0xce9c84u32, 0xce9c88u32), (0xce9c8cu32, 0xce9c90u32)];
        for (it, (idva, wva)) in pairs.into_iter().enumerate() {
            let id = table[slot(idva)] as i32;
            let weight = f32::from_bits(table[slot(wva)]);
            if id != 0 && weight > 0.0 && id < inp.remap_bound {
                let (h, w) = if it == 0 { (0, 2) } else { (1, 3) };
                bbc[h] = SKYBOX_STUB_TOKEN;
                bbc[w] = weight.to_bits();
            }
        }
    }

    let d0c = (dn_track_eval(&inp.track4, 4, inp.phase) as f32).to_bits();
    let d10 = (dn_track_eval(&inp.track2, 2, inp.phase) as f32).to_bits();

    DnOutput {
        table,
        bd0,
        cd8,
        cd4,
        bcast,
        alpha_bytes,
        bbc,
        bb4,
        bb8,
        d0c,
        d10,
        fog,
    }
}
