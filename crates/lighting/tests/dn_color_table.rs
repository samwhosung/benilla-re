//! `BuildLightTable` (`0x6d2260`) whole, against `dn_color_table_build`: the globals it reads
//! (mode, phase, camera, lights, the band and row containers, the tracks, the light state fog
//! copies) are laid out in a fresh emulator, the function runs, and every field it writes is read
//! back and compared. `InternSkybox` (`0x6d4640`) is stubbed to return `SKYBOX_STUB_TOKEN`, the sky
//! gradient consumer `0x6d0f50` to nothing, and the allocator the light heap uses to a bump
//! allocator; everything else, including the gather, both blends, the heap, `BuildLightColours` and
//! the band and track evaluations, runs in the emulator.
//!
//! The main test climbs a ladder of paths: (a) no data, (b) one light with one-key bands, (c)
//! two-key bands over a phase sweep, (d) the day blend, (e) the override byte at 0, 0x80 and 0xff
//! with and without the alpha stamp, (g) modes 2, 3 and another nibble, (h) the full mode word 0xf,
//! and (f) the area-light drain with one to four heap lights. The edges test follows.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::dn_color::FogPrior;
use benilla_re_lighting::dn_color_table::{
    dn_color_table_build, BandSub, Containers, DnInput, LightParamsRow, LightRow, SKYBOX_STUB_TOKEN,
};
use common::next;

// The globals the build reads and writes.
const G_MODE: u64 = 0xc7f288;
const G_PHASE: u64 = 0xce9b64;
const G_CAM_X: u64 = 0xce9b78;
const G_CAM_Y: u64 = 0xce9b7c;
const G_CAM_Z: u64 = 0xce9b80;
const G_LIGHT_COUNT: u64 = 0xce98dc;
const G_LIGHT_ARRAY: u64 = 0xce98e0;
const G_BAC: u64 = 0xce9bac;
const G_BB0: u64 = 0xce9bb0;
const G_BCC: u64 = 0xce9bcc;
/// A byte; the override colour is the dword at `0xce9ba5`.
const G_BA4: u64 = 0xce9ba4;
const G_BA5: u64 = 0xce9ba5;
const G_ROWS_PTR: u64 = 0xce9d7c;
const G_ROWS_BOUND: u64 = 0xce9d80;
const G_COLOR_PTR: u64 = 0xce9da4;
const G_COLOR_BOUND: u64 = 0xce9da8;
const G_SCALAR_PTR: u64 = 0xce9d90;
const G_SCALAR_BOUND: u64 = 0xce9d94;
const G_REMAP_BOUND: u64 = 0xce9db4;
const G_REMAP_PTR: u64 = 0xce9db8;
const G_REMAP_HI: u64 = 0xce9dbc;
const G_TRACK4: u64 = 0xce9a34;
const G_TRACK2: u64 = 0xce9a24;
const G_PRIOR_BD0: u64 = 0xce9bd0;
const G_PRIOR_CD8: u64 = 0xce9cd8;
const G_PRIOR_CD4: u64 = 0xce9cd4;
// What `BuildLightColours` reads: the sun and the prior fog. Its alpha byte `0xce9c5a` comes from
// the table the build makes.
const G_SUN_X: u64 = 0xce9cc8;
const G_SUN_Y: u64 = 0xce9ccc;
const G_SUN_Z: u64 = 0xce9cd0;
const G_PRIOR_FOG0: u64 = 0xce9ce4;
const G_PRIOR_FOG1: u64 = 0xce9ce8;
const G_PRIOR_FOG2: u64 = 0xce9cec;
const G_PRIOR_FOG3: u64 = 0xce9cf0;
// What the test reads back.
const G_TABLE: u64 = 0xce9c0c;
const G_BCAST: [u64; 4] = [0xce9864, 0xce97c0, 0xce9884, 0xce9710];
const G_ALPHA_BYTES: [u64; 5] = [0xce9867, 0xce97c3, 0xce9887, 0xce9713, 0xce98a7];
const G_BBC0: u64 = 0xce9bbc;
const G_BB4: u64 = 0xce9bb4;
const G_BB8: u64 = 0xce9bb8;
const G_D0C: u64 = 0xce9d0c;
const G_D10: u64 = 0xce9d10;
// `[0xce9cd4]` and `[0xce9cd8]` are overwritten by the colour scales after `BuildLightColours`, so
// they are compared as the scales' outputs, not as fog fields.
const G_FOG_CDC: u64 = 0xce9cdc;
const G_FOG_CE0: u64 = 0xce9ce0;
const G_FOG_CF4: u64 = 0xce9cf4;
const G_FOG_INTERLEAVE: u64 = 0xce9c9c;

fn write_byte(emu: &mut Emu, va: u64, b: u8) {
    let word = va & !0x3;
    let off = (va & 0x3) as u32 * 8;
    let cur = emu.read_u32(word);
    emu.write_u32(word, (cur & !(0xffu32 << off)) | ((b as u32) << off));
}

/// Lays out a band: the count at `+4`, 16 keys at `+8`, 16 values at `+0x48`. Returns its address.
fn write_band(emu: &mut Emu, b: &BandSub) -> u64 {
    let p = emu.alloc(0x90);
    emu.write_u32(p + 4, b.count as u32);
    for k in 0..16usize {
        emu.write_u32(p + 8 + 4 * k as u64, *b.keys.get(k).unwrap_or(&0) as u32);
        emu.write_u32(p + 0x48 + 4 * k as u64, *b.values.get(k).unwrap_or(&0));
    }
    p
}

/// Lays out an array of band pointers and writes the `{pointer, bound}` globals.
fn write_band_container(emu: &mut Emu, bands: &[BandSub], bound: i32, ptr_g: u64, bound_g: u64) {
    let arr = emu.alloc(((bands.len().max(1)) * 4) as u64);
    for (i, b) in bands.iter().enumerate() {
        let bp = write_band(emu, b);
        emu.write_u32(arr + 4 * i as u64, bp as u32);
    }
    emu.write_u32(ptr_g, arr as u32);
    emu.write_u32(bound_g, bound as u32);
}

/// Lays out a Light row: the group at `+0`, an integer at `+4`, dwords at `+8` and `+0xc`, three at
/// `+0x10`, and the index array at `+0x1c` (whose first two entries are also the gather's `+0x70`
/// and `+0x74`). Returns its address.
fn write_row(emu: &mut Emu, r: &LightParamsRow) -> u64 {
    let p = emu.alloc(0x40);
    emu.write_u32(p, r.group_id as u32);
    emu.write_u32(p + 4, r.w4 as u32);
    emu.write_u32(p + 8, r.w8);
    emu.write_u32(p + 0xc, r.wc);
    for k in 0..3usize {
        emu.write_u32(p + 0x10 + 4 * k as u64, r.w10_18[k]);
    }
    for k in 0..4usize {
        emu.write_u32(p + 0x1c + 4 * k as u64, r.idx[k] as u32);
    }
    p
}

/// Lays out the rows container: `0xce9d7c` points to the array of rows, `0xce9d80` is the bound.
fn write_rows_container(emu: &mut Emu, rows: &[LightParamsRow], bound: i32) {
    let arr = emu.alloc(((rows.len().max(1)) * 4) as u64);
    for (i, r) in rows.iter().enumerate() {
        let rp = write_row(emu, r);
        emu.write_u32(arr + 4 * i as u64, rp as u32);
    }
    emu.write_u32(G_ROWS_PTR, arr as u32);
    emu.write_u32(G_ROWS_BOUND, bound as u32);
}

/// Lays out the lights: `0xce98e0` points to an array of pointers to the lights, each with the
/// group at `+0`, the position at `+8`, the radii at `+0x14` and `+0x18` and the indices at
/// `+0x1c`.
fn write_lights(emu: &mut Emu, lights: &[LightRow]) {
    let ptr_arr = emu.alloc(((lights.len().max(1)) * 4) as u64);
    for (i, l) in lights.iter().enumerate() {
        let st = emu.alloc(0x40);
        emu.write_u32(st, l.group_id as u32);
        emu.write_u32(st + 8, l.pos[0].to_bits());
        emu.write_u32(st + 0xc, l.pos[1].to_bits());
        emu.write_u32(st + 0x10, l.pos[2].to_bits());
        emu.write_u32(st + 0x14, l.inner.to_bits());
        emu.write_u32(st + 0x18, l.outer.to_bits());
        for k in 0..4usize {
            emu.write_u32(st + 0x1c + 4 * k as u64, l.idx[k] as u32);
        }
        emu.write_u32(ptr_arr + 4 * i as u64, st as u32);
    }
    emu.write_u32(G_LIGHT_ARRAY, ptr_arr as u32);
}

/// A fresh emulator for each case, since the build changes the light state and the allocation arena
/// cannot be rewound. The stubs: `InternSkybox` returns the token, `0x6d0f50` does nothing,
/// `0x6462e0` (the allocator the heap uses) hands out 0x800-byte blocks, and `0x646430` (its free)
/// does nothing.
fn fresh(wow: &Wow) -> Emu<'_> {
    let mut emu = Emu::new(wow);
    emu.stub(0x6d4640, SKYBOX_STUB_TOKEN, 0);
    emu.stub(0x6d0f50, 0, 0);
    emu.stub_alloc(0x6462e0, ALLOC_BASE, 0x800, 4);
    emu.stub(0x646430, 0, 4);
    emu
}

/// The allocator stub's base: 0xc0000 into the scratch arena, clear of the case's own allocations.
const ALLOC_BASE: u64 = 0x7200_0000 + 0xc_0000;

/// Seeds every global from `inp` into a fresh emulator, runs `0x6d2260`, and compares the fields it
/// writes with `dn_color_table_build`.
fn run_case(wow: &Wow, inp: &DnInput, ctx: &str) {
    let mut emu = fresh(wow);
    let emu = &mut emu;
    emu.write_u32(G_MODE, inp.mode);
    emu.write_u32(G_PHASE, inp.phase.to_bits());
    emu.write_u32(G_CAM_X, inp.cam[0].to_bits());
    emu.write_u32(G_CAM_Y, inp.cam[1].to_bits());
    emu.write_u32(G_CAM_Z, inp.cam[2].to_bits());
    emu.write_u32(G_LIGHT_COUNT, inp.light_count as u32);
    emu.write_u32(G_BB0, inp.bb0 as u32);
    emu.write_u32(G_BCC, inp.bcc.to_bits());
    write_byte(emu, G_BA4, inp.ba4);
    {
        let word = G_BA5 & !0x3;
        // `0xce9ba5` is byte 1 of the dword at `0xce9ba4`: byte 0 is `ba4`, bytes 1 to 3 are the
        // override colour.
        let v = (u32::from(inp.ba4)) | ((inp.ba5 & 0x00ff_ffff) << 8);
        emu.write_u32(word, v);
    }
    emu.write_u32(G_BAC, 0);
    let bac_row;
    if let Some(r) = &inp.params_bac {
        bac_row = write_row(emu, r);
        emu.write_u32(G_BAC, bac_row as u32);
    }

    write_rows_container(emu, &inp.containers.rows, inp.containers.rows_bound);
    write_band_container(
        emu,
        &inp.containers.color_bands,
        inp.containers.color_bound,
        G_COLOR_PTR,
        G_COLOR_BOUND,
    );
    write_band_container(
        emu,
        &inp.containers.scalar_bands,
        inp.containers.scalar_bound,
        G_SCALAR_PTR,
        G_SCALAR_BOUND,
    );
    write_lights(emu, &inp.lights);

    emu.write_u32(G_REMAP_BOUND, inp.remap_bound as u32);
    emu.write_u32(G_REMAP_HI, inp.remap_hi as u32);
    emu.write_u32(G_REMAP_PTR, 0);

    for (k, e) in inp.track4.iter().enumerate() {
        emu.write_u32(G_TRACK4 + 8 * k as u64, e[0].to_bits());
        emu.write_u32(G_TRACK4 + 8 * k as u64 + 4, e[1].to_bits());
    }
    for (k, e) in inp.track2.iter().enumerate() {
        emu.write_u32(G_TRACK2 + 8 * k as u64, e[0].to_bits());
        emu.write_u32(G_TRACK2 + 8 * k as u64 + 4, e[1].to_bits());
    }

    emu.write_u32(G_PRIOR_BD0, inp.prior_bd0);
    emu.write_u32(G_SUN_X, inp.fog_prior.sun[0]);
    emu.write_u32(G_SUN_Y, inp.fog_prior.sun[1]);
    emu.write_u32(G_SUN_Z, inp.fog_prior.sun[2]);
    emu.write_u32(G_PRIOR_FOG0, inp.fog_prior.prior_fog[0]);
    emu.write_u32(G_PRIOR_FOG1, inp.fog_prior.prior_fog[1]);
    emu.write_u32(G_PRIOR_FOG2, inp.fog_prior.prior_fog[2]);
    emu.write_u32(G_PRIOR_FOG3, inp.fog_prior.prior_fog[3]);

    emu.call(0x6d2260, 0, 0, &[]);

    let ours = dn_color_table_build(inp);

    // Bit for bit, but any NaN matches any NaN: the scalar bands, the track samples and the fog
    // floats can be NaN, whose sign and payload differ between the x87 and the host.
    let chk = |b: u32, o: u32, name: &str| {
        if f32::from_bits(b).is_nan() && f32::from_bits(o).is_nan() {
            return;
        }
        assert_eq!(b, o, "{ctx}: {name}: binary={b:#010x} ours={o:#010x}");
    };

    for k in 0..0x24u64 {
        chk(
            emu.read_u32(G_TABLE + 4 * k),
            ours.table[k as usize],
            &format!("table[{k:#x}] @ {:#x}", G_TABLE + 4 * k),
        );
    }
    chk(emu.read_u32(G_PRIOR_BD0), ours.bd0, "bd0 (0xce9bd0)");
    chk(emu.read_u32(G_PRIOR_CD8), ours.cd8, "cd8 (0xce9cd8)");
    chk(emu.read_u32(G_PRIOR_CD4), ours.cd4, "cd4 (0xce9cd4)");
    for (k, &va) in G_BCAST.iter().enumerate() {
        chk(
            emu.read_u32(va),
            ours.bcast[k],
            &format!("bcast[{k}] @ {va:#x}"),
        );
    }
    if let Some(a) = ours.alpha_bytes {
        for &va in &G_ALPHA_BYTES {
            let b = (emu.read_u32(va & !0x3) >> ((va & 0x3) * 8)) as u8;
            assert_eq!(b, a, "{ctx}: alpha byte @ {va:#x}");
        }
    }
    for k in 0..4u64 {
        chk(
            emu.read_u32(G_BBC0 + 4 * k),
            ours.bbc[k as usize],
            &format!("bbc[{k}] @ {:#x}", G_BBC0 + 4 * k),
        );
    }
    chk(emu.read_u32(G_BB4), ours.bb4, "bb4 (0xce9bb4)");
    chk(emu.read_u32(G_BB8), ours.bb8, "bb8 (0xce9bb8)");
    chk(emu.read_u32(G_D0C), ours.d0c, "d0c (0xce9d0c)");
    chk(emu.read_u32(G_D10), ours.d10, "d10 (0xce9d10)");

    chk(emu.read_u32(G_FOG_CDC), ours.fog.cdc, "fog.cdc");
    chk(emu.read_u32(G_FOG_CE0), ours.fog.ce0, "fog.ce0");
    chk(emu.read_u32(G_FOG_CF4), ours.fog.cf4, "fog.cf4");
    for k in 0..11u64 {
        chk(
            emu.read_u32(G_FOG_INTERLEAVE + 4 * k),
            ours.fog.interleave[k as usize],
            &format!("fog.interleave[{k}]"),
        );
    }
}

/// A base input with plausible tracks and fog state, which a case changes.
fn base_input(s: &mut u32) -> DnInput {
    let f = |s: &mut u32| (f64::from(next(s) >> 8) / f64::from(1u32 << 24)) as f32;
    DnInput {
        mode: 0,
        phase: f(s),
        light_count: 0,
        lights: Vec::new(),
        params_bac: None,
        bb0: -1,
        bcc: -1.0,
        cam: [f(s) * 100.0, f(s) * 100.0, f(s) * 60.0 - 30.0],
        ba4: 0,
        ba5: 0,
        containers: Containers::default(),
        remap_bound: 0,
        remap_hi: 0,
        track4: [[0.0, f(s)], [0.25, f(s)], [0.5, f(s)], [0.75, f(s)]],
        track2: [[0.0, f(s)], [1.0, f(s)]],
        fog_prior: FogPrior {
            sun: [next(s), next(s), next(s)],
            prior_fog: [next(s), next(s), next(s), next(s)],
        },
        prior_bd0: 0xff80_4020 | (next(s) & 0xff),
    }
}

/// A band of `count` ascending distinct keys in [0, 2880) with colour or scalar values.
fn synth_band(s: &mut u32, count: usize, color: bool) -> BandSub {
    let mut keys: Vec<i32> = (0..count).map(|_| ((next(s) >> 8) % 2880) as i32).collect();
    keys.sort_unstable();
    keys.dedup();
    while keys.len() < count {
        keys.push((keys.last().copied().unwrap_or(0) + 7) % 2880);
        keys.sort_unstable();
        keys.dedup();
    }
    keys.truncate(count);
    let values: Vec<u32> = (0..count.max(1))
        .map(|_| {
            if color {
                next(s)
            } else {
                next(s) & 0x7fbf_ffff
            }
        })
        .collect();
    BandSub {
        count: count as i32,
        keys,
        values,
    }
}

/// Containers carrying the 18 colour and 6 scalar bands of group 1 (colour `idx + 1`, scalar `idx +
/// 1`).
fn synth_containers(s: &mut u32, key_count: usize) -> Containers {
    let mut color_bands = vec![synth_band(s, 0, true)];
    for _ in 0..18 {
        color_bands.push(synth_band(s, key_count, true));
    }
    let mut scalar_bands = vec![synth_band(s, 0, false)];
    for _ in 0..6 {
        scalar_bands.push(synth_band(s, key_count, false));
    }
    let color_bound = (color_bands.len() - 1) as i32;
    let scalar_bound = (scalar_bands.len() - 1) as i32;
    Containers {
        rows: Vec::new(),
        rows_bound: 0,
        color_bands,
        color_bound,
        scalar_bands,
        scalar_bound,
    }
}

/// Containers carrying the bands of groups 1 to `n_groups`, so each light's group gathers its own
/// record. Colour band `idx + 18g − 17`, scalar band `idx + 6g − 5`, group by group.
fn synth_containers_groups(s: &mut u32, key_count: usize, n_groups: usize) -> Containers {
    let mut color_bands = vec![synth_band(s, 0, true)];
    let mut scalar_bands = vec![synth_band(s, 0, false)];
    for _g in 0..n_groups {
        for _ in 0..18 {
            color_bands.push(synth_band(s, key_count, true));
        }
        for _ in 0..6 {
            scalar_bands.push(synth_band(s, key_count, false));
        }
    }
    let color_bound = (color_bands.len() - 1) as i32;
    let scalar_bound = (scalar_bands.len() - 1) as i32;
    Containers {
        rows: Vec::new(),
        rows_bound: 0,
        color_bands,
        color_bound,
        scalar_bands,
        scalar_bound,
    }
}

/// A row for the full path: group 1, with index 0 for every kind.
fn synth_row(s: &mut u32) -> LightParamsRow {
    LightParamsRow {
        group_id: 1,
        w4: (next(s) >> 8) as i32 % 64,
        // The gather's `+0x78` is read as a skybox id; keeping it non-negative lets a zero bound
        // skip it, where a negative id would index the null remap table.
        w8: next(s) & 0x7fff_ffff,
        wc: next(s),
        w10_18: [next(s), next(s), next(s)],
        idx: [0, 0, 0, 0],
    }
}

/// The ladder.
#[test]
fn dn_color_table_build_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x6374_6230u32;

    // (a) No lights and no override row.
    for case in 0..40 {
        let inp = base_input(&mut s);
        run_case(&wow, &inp, &format!("(a) no-data case {case}"));
    }

    // (b) One light, one-key bands, mode 0 or 1, the whole-record copy.
    for case in 0..60 {
        let mut inp = base_input(&mut s);
        inp.mode = case % 2; // 0 or 1
        inp.light_count = 1;
        inp.lights = vec![LightRow {
            group_id: 1,
            pos: [0.0, 0.0, 0.0],
            inner: 1.0,
            outer: 1.0,
            ..Default::default()
        }];
        inp.containers = synth_containers(&mut s, 1);
        inp.containers.rows = vec![synth_row(&mut s)];
        inp.containers.rows_bound = 1;
        inp.bcc = -1.0; // no day blend
        run_case(&wow, &inp, &format!("(b) one-light case {case}"));
    }

    // (c) Two-key bands over a phase sweep, including the wrap.
    for case in 0..80 {
        let mut inp = base_input(&mut s);
        inp.mode = case % 2;
        inp.light_count = 1;
        inp.lights = vec![LightRow {
            group_id: 1,
            outer: 1.0,
            ..Default::default()
        }];
        inp.containers = synth_containers(&mut s, 2);
        inp.containers.rows = vec![synth_row(&mut s)];
        inp.containers.rows_bound = 1;
        inp.phase = match case % 4 {
            0 => 0.0,
            1 => 0.999,
            2 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32,
            _ => 1.0,
        };
        inp.bcc = -1.0;
        run_case(&wow, &inp, &format!("(c) 2-key case {case}"));
    }

    // (d) The day blend: the second gather and `sky_weighted_blend`.
    for case in 0..60 {
        let mut inp = base_input(&mut s);
        inp.mode = case % 2;
        inp.light_count = 1;
        inp.lights = vec![LightRow {
            group_id: 1,
            outer: 1.0,
            ..Default::default()
        }];
        inp.containers = synth_containers(&mut s, 2);
        // Four rows with indices 0 to 3, so the second gather resolves a distinct row.
        inp.containers.rows = (0..4).map(|_| synth_row(&mut s)).collect();
        for r in inp.containers.rows.iter_mut() {
            r.idx = [0, 1, 2, 3];
        }
        inp.containers.rows_bound = 4;
        inp.bcc = match case % 3 {
            0 => 0.5,
            1 => 1.0,
            _ => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32 * 0.9 + 0.05,
        };
        run_case(&wow, &inp, &format!("(d) day-blend case {case}"));
    }

    // (e) The override byte at 0, 0x80 and 0xff, and the alpha stamp on half the cases.
    for case in 0..90 {
        let mut inp = base_input(&mut s);
        inp.mode = case % 2;
        inp.light_count = 1;
        inp.lights = vec![LightRow {
            group_id: 1,
            outer: 1.0,
            ..Default::default()
        }];
        inp.containers = synth_containers(&mut s, 2);
        inp.containers.rows = vec![synth_row(&mut s)];
        inp.containers.rows_bound = 1;
        inp.ba4 = match case % 3 {
            0 => 0,
            1 => 0x80,
            _ => 0xff,
        };
        inp.ba5 = next(&mut s) & 0x00ff_ffff;
        inp.bcc = if case % 2 == 0 { 0.5 } else { -1.0 };
        run_case(&wow, &inp, &format!("(e) alpha case {case}"));
    }

    // (g) Modes 2 and 3 gather one row; another nibble goes to fog alone.
    for case in 0..60 {
        let mut inp = base_input(&mut s);
        inp.mode = match case % 3 {
            0 => 2,
            1 => 3,
            _ => 5,
        };
        inp.light_count = 1;
        inp.lights = vec![LightRow {
            group_id: 1,
            outer: 1.0,
            ..Default::default()
        }];
        inp.containers = synth_containers(&mut s, 2);
        // Modes 2 and 3 gather element 7 and 6 of the rows; a bound of 7 keeps the row non-null,
        // since a null row would fault in `0x6d6ad0`.
        inp.containers.rows = (0..8).map(|_| synth_row(&mut s)).collect();
        inp.containers.rows_bound = 7;
        inp.bcc = -1.0;
        run_case(&wow, &inp, &format!("(g) mode case {case}"));
    }

    // (h) The mode word 0xf forces the full path.
    for case in 0..40 {
        let mut inp = base_input(&mut s);
        inp.mode = 0xf;
        inp.light_count = 1;
        inp.lights = vec![LightRow {
            group_id: 1,
            outer: 1.0,
            ..Default::default()
        }];
        inp.containers = synth_containers(&mut s, 2);
        inp.containers.rows = vec![synth_row(&mut s)];
        inp.containers.rows_bound = 1;
        inp.bcc = -1.0;
        run_case(&wow, &inp, &format!("(h) full-0xf case {case}"));
    }

    rung_f_multilight(&wow, &mut s);

    // (f) The area-light drain.
}

/// Builds a multi-light case and runs it. `n_heap` lights beyond light 0 sit along +x at `dists`
/// from a camera at the origin; `cloud_ids` are the rows' `+0x78` ids and `radii` the `(inner,
/// outer)` of each light; `bcc > 0` enables the day blend of every gathered record. Each light has
/// its own group, so its record differs, and the whole-record copy puts all 0x24 slots in the
/// comparison.
#[allow(clippy::too_many_arguments)]
fn run_multilight(
    wow: &Wow,
    s: &mut u32,
    mode: u32,
    n_heap: usize,
    cloud_ids: &[u32],
    radii: &[(f32, f32)],
    dists: &[f32],
    bcc: f32,
    ctx: &str,
) {
    let total = n_heap + 1;
    let want_day = (mode & 0xf) == 0 || (mode & 0xf) == 1;
    let mut inp = base_input(s);
    inp.mode = mode;
    inp.light_count = total as i32;
    inp.bcc = bcc;
    inp.cam = [0.0, 0.0, 0.0];
    inp.bb0 = -1;

    // One group per light for the primary row, and another per light for the day blend's second
    // row.
    let n_groups = total * 2;
    inp.containers = synth_containers_groups(s, 2, n_groups);

    // Light L's primary row is at index L (group L + 1), its second row at `total + L`.
    let mut rows: Vec<LightParamsRow> = Vec::new();
    for li in 0..total {
        let mut r = synth_row(s);
        r.group_id = (li + 1) as i32;
        r.w8 = *cloud_ids.get(li).unwrap_or(&0);
        rows.push(r);
    }
    for li in 0..total {
        let mut r = synth_row(s);
        r.group_id = (total + li + 1) as i32;
        rows.push(r);
    }
    inp.containers.rows = rows;
    inp.containers.rows_bound = (total * 2) as i32;

    // The indices `[L, L, total + L, total + L]` send the primary row (kind 0 or 1) to row L and the
    // second (kind 2 or 3) to row `total + L`.
    let mut lights = Vec::new();
    for li in 0..total {
        let (inner, outer) = radii[li];
        lights.push(LightRow {
            group_id: (li + 1) as i32,
            idx: [
                li as i32,
                li as i32,
                (total + li) as i32,
                (total + li) as i32,
            ],
            pos: [dists[li], 0.0, 0.0],
            inner,
            outer,
        });
    }
    inp.lights = lights;
    let _ = want_day;
    run_case(wow, &inp, ctx);
}

/// Rung (f): the drain with one to four heap lights.
fn rung_f_multilight(wow: &Wow, s: &mut u32) {
    // One heap light: it is popped and overblended, since the drain runs while the slot count,
    // which includes slot 0, is above 1.
    for case in 0..20 {
        let mode = case % 2;
        run_multilight(
            wow,
            s,
            mode,
            1,
            &[0, 12],
            &[(5.0, 50.0), (2.0, 40.0)],
            &[0.0, 20.0],
            -1.0,
            &format!("(f) one-heap-light case {case}"),
        );
    }

    // Two heap lights with distinct cloud ids and fractional falloff, no day blend.
    for case in 0..30 {
        let mode = case % 2;
        run_multilight(
            wow,
            s,
            mode,
            2,
            &[0, 10, 20],
            &[(5.0, 50.0), (2.0, 40.0), (3.0, 45.0)],
            &[0.0, 15.0, 25.0],
            -1.0,
            &format!("(f) 2-light insert case {case}"),
        );
    }

    // Three heap lights: fractional falloff, a distance inside the inner radius (w = 1), and the
    // farthest-first order.
    for case in 0..30 {
        let mode = case % 2;
        run_multilight(
            wow,
            s,
            mode,
            3,
            &[0, 11, 22, 33],
            &[(5.0, 60.0), (1.0, 50.0), (8.0, 55.0), (3.0, 58.0)],
            &[0.0, 4.0, 30.0, 50.0],
            -1.0,
            &format!("(f) 3-light order case {case}"),
        );
    }

    // The cloud accumulator: three heap lights share one id and all have w = 1, so the sum clamps
    // to exactly 1.0.
    for case in 0..30 {
        let mode = case % 2;
        run_multilight(
            wow,
            s,
            mode,
            3,
            &[0, 7, 7, 7],
            &[(5.0, 60.0), (40.0, 60.0), (40.0, 60.0), (40.0, 60.0)],
            &[0.0, 10.0, 20.0, 30.0],
            -1.0,
            &format!("(f) cloud-accumulate case {case}"),
        );
    }

    // Four distinct ids: the ones after the first two find both pairs taken and are dropped.
    for case in 0..20 {
        let mode = case % 2;
        run_multilight(
            wow,
            s,
            mode,
            4,
            &[0, 41, 42, 43, 44],
            &[
                (5.0, 70.0),
                (2.0, 60.0),
                (3.0, 62.0),
                (4.0, 64.0),
                (5.0, 66.0),
            ],
            &[0.0, 8.0, 20.0, 35.0, 55.0],
            -1.0,
            &format!("(f) cloud-drop case {case}"),
        );
    }

    // The day blend on (`bcc > 0`): each gathered record is blended with its second row first.
    for case in 0..30 {
        let mode = case % 2;
        let bcc = match case % 3 {
            0 => 0.5,
            1 => 1.0,
            _ => (f64::from(next(s) >> 8) / f64::from(1u32 << 24)) as f32 * 0.9 + 0.05,
        };
        run_multilight(
            wow,
            s,
            mode,
            2,
            &[0, 13, 26],
            &[(5.0, 50.0), (2.0, 40.0), (3.0, 45.0)],
            &[0.0, 12.0, 28.0],
            bcc,
            &format!("(f) day-blend multi case {case}"),
        );
    }

    // A light whose inner and outer radii are both 30, at distance 10, with another at 30.
    for case in 0..20 {
        let mode = case % 2;
        run_multilight(
            wow,
            s,
            mode,
            2,
            &[0, 9, 19],
            &[(5.0, 50.0), (30.0, 30.0), (3.0, 45.0)],
            &[0.0, 10.0, 30.0],
            -1.0,
            &format!("(f) equal radii case {case}"),
        );
    }
}

/// The compares and conversions of `0x6d2260` and the drain `0x6d2d00` at their edges, each case
/// run whole against `dn_color_table_build`:
/// - the half-minute `round(f32(phase·2880) − 0.5)` at exact integers, odd and even (ties to even,
///   so an odd one comes out one low), and at NaN and the infinities (`fistp` gives `0x80000000`);
/// - lights at equal distances, which the heap orders by its own sifts rather than by a sort;
/// - a light exactly at its outer radius (`dist < outer` is strict, so it is left out), at its
///   inner radius (weight 1) and with NaN or infinite positions;
/// - the sun ramp at a full mode word of 1 with the camera z at 0, −0, just below and above −30, at
///   −30, and NaN (which gives −30);
/// - the day blend weight at 0, −0 and NaN, which gate the blend (`> 0`) and the alpha stamp (`!=
///   0`) differently.
#[test]
fn dn_color_table_edges_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x6564_6765u32;

    for (case, &k) in [1u32, 2, 3, 719, 720, 721, 1439, 2879, 2880, 5760]
        .iter()
        .enumerate()
    {
        let mut inp = base_input(&mut s);
        inp.mode = (case % 2) as u32;
        inp.light_count = 1;
        inp.lights = vec![LightRow {
            group_id: 1,
            outer: 1.0,
            ..Default::default()
        }];
        inp.containers = synth_containers(&mut s, 3);
        inp.containers.rows = vec![synth_row(&mut s)];
        inp.containers.rows_bound = 1;
        inp.phase = (f64::from(k) / 2880.0) as f32;
        run_case(&wow, &inp, &format!("phase {k}/2880"));
    }
    for phase in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.0, -1.0, 1e30] {
        let mut inp = base_input(&mut s);
        inp.light_count = 1;
        inp.lights = vec![LightRow {
            group_id: 1,
            outer: 1.0,
            ..Default::default()
        }];
        inp.containers = synth_containers(&mut s, 3);
        inp.containers.rows = vec![synth_row(&mut s)];
        inp.containers.rows_bound = 1;
        inp.phase = phase;
        run_case(&wow, &inp, &format!("phase {phase}"));
    }

    for case in 0..12 {
        let mode = case % 2;
        let radii = [(5.0, 60.0), (1.0, 50.0), (2.0, 55.0), (3.0, 58.0)];
        run_multilight(
            &wow,
            &mut s,
            mode,
            3,
            &[0, 11, 22, 33],
            &radii,
            &[0.0, 20.0, 20.0, 20.0],
            -1.0,
            &format!("ties, three lights at 20 (case {case})"),
        );
        run_multilight(
            &wow,
            &mut s,
            mode,
            4,
            &[0, 11, 22, 33, 44],
            &[
                (5.0, 60.0),
                (1.0, 50.0),
                (2.0, 55.0),
                (3.0, 58.0),
                (4.0, 59.0),
            ],
            &[0.0, 10.0, 30.0, 10.0, 30.0],
            0.5,
            &format!("ties, two pairs (case {case})"),
        );
        run_multilight(
            &wow,
            &mut s,
            mode,
            5,
            &[0, 5, 6, 7, 8, 9],
            &[
                (5.0, 60.0),
                (1.0, 50.0),
                (2.0, 55.0),
                (3.0, 58.0),
                (4.0, 59.0),
                (6.0, 61.0),
            ],
            &[0.0, 40.0, 10.0, 40.0, 25.0, 10.0],
            -1.0,
            &format!("ties, a descending run (case {case})"),
        );
        // Distances {20, 30, 10, 20} pop in the order 2, 1, 4, 3, which a stable sort would give
        // as 2, 4, 1, 3; {20, 20, 30, 30, 20} pop as 4, 3, 1, 2, 5.
        run_multilight(
            &wow,
            &mut s,
            mode,
            4,
            &[0, 21, 22, 23, 24],
            &[
                (5.0, 60.0),
                (1.0, 50.0),
                (2.0, 55.0),
                (3.0, 58.0),
                (4.0, 59.0),
            ],
            &[0.0, 20.0, 30.0, 10.0, 20.0],
            -1.0,
            &format!("ties that a sort orders differently, four lights (case {case})"),
        );
        run_multilight(
            &wow,
            &mut s,
            mode,
            5,
            &[0, 31, 32, 33, 34, 35],
            &[
                (5.0, 60.0),
                (1.0, 50.0),
                (2.0, 55.0),
                (3.0, 58.0),
                (4.0, 59.0),
                (6.0, 61.0),
            ],
            &[0.0, 20.0, 20.0, 30.0, 30.0, 20.0],
            -1.0,
            &format!("ties that a sort orders differently, five lights (case {case})"),
        );
        run_multilight(
            &wow,
            &mut s,
            mode,
            2,
            &[0, 9, 19],
            &[(5.0, 50.0), (2.0, 30.0), (3.0, 45.0)],
            &[0.0, 30.0, 10.0],
            -1.0,
            &format!("one light exactly at its outer radius (case {case})"),
        );
        run_multilight(
            &wow,
            &mut s,
            mode,
            2,
            &[0, 9, 19],
            &[(5.0, 50.0), (30.0, 50.0), (3.0, 45.0)],
            &[0.0, 30.0, 10.0],
            -1.0,
            &format!("one light exactly at its inner radius (case {case})"),
        );
        run_multilight(
            &wow,
            &mut s,
            mode,
            2,
            &[0, 9, 19],
            &[(5.0, 50.0), (2.0, 40.0), (3.0, 45.0)],
            &[0.0, f32::NAN, f32::INFINITY],
            -1.0,
            &format!("NaN and infinite positions (case {case})"),
        );
        run_multilight(
            &wow,
            &mut s,
            mode,
            2,
            &[0, 9, 19],
            &[(f32::NAN, 50.0), (40.0, f32::NAN), (3.0, 45.0)],
            &[0.0, 10.0, 20.0],
            -1.0,
            &format!("NaN radii (case {case})"),
        );
    }

    for z in [
        0.0,
        -0.0,
        1e-30,
        -1e-30,
        -29.999998,
        -30.0,
        -30.000002,
        -100.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        let mut inp = base_input(&mut s);
        inp.mode = 1;
        inp.light_count = 1;
        inp.lights = vec![LightRow {
            group_id: 1,
            outer: 1.0,
            ..Default::default()
        }];
        inp.containers = synth_containers(&mut s, 2);
        inp.containers.rows = vec![synth_row(&mut s)];
        inp.containers.rows_bound = 1;
        inp.cam[2] = z;
        run_case(&wow, &inp, &format!("camera z {z}"));
    }

    for bcc in [0.0, -0.0, f32::NAN, f32::INFINITY, 1e-30, -1e-30, 2.0] {
        for mode in [0, 1] {
            let mut inp = base_input(&mut s);
            inp.mode = mode;
            inp.light_count = 1;
            inp.lights = vec![LightRow {
                group_id: 1,
                outer: 1.0,
                idx: [0, 1, 2, 3],
                ..Default::default()
            }];
            inp.containers = synth_containers(&mut s, 2);
            inp.containers.rows = (0..4).map(|_| synth_row(&mut s)).collect();
            for r in inp.containers.rows.iter_mut() {
                r.idx = [0, 1, 2, 3];
            }
            inp.containers.rows_bound = 4;
            inp.bcc = bcc;
            run_case(&wow, &inp, &format!("bcc {bcc} mode {mode}"));
        }
    }
}
