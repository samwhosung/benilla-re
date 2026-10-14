//! The world map's projections (`systems/ui/worldmap.md#hover-and-click`,
//! `#unit-and-corpse-positions`) run from the exe: `0x4a6ec0`, `0x4a72b0` and `ProjectWorldPosition`
//! (`0x4a7360`).
//!
//! Each function reads the map state and chooses a record. The test seeds the image globals (the
//! continent index `0x84506c`, the continent table `0xb6e668` with stride `0x10024`, the row
//! pointers `0xc0d5bc` and the sub-entry list `0xc0d5a0` with stride `0x34`, all `.bss` and zero in
//! the file) so that the exe reaches its kernel with the same record fields the Rust function is
//! given, and compares the projected `f32` (for `0x4a6ec0`, the integer cell index) exactly. Two
//! NaNs compare equal. Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp)]
#![allow(clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::worldmap_projection::{
    project_world_position as to_uv, project_world_position_view as arrow,
    zone_cell_index as area_idx, WorldProjection as Map4a7360,
};

// --- the map-state-machine globals (all in `.data`, seeded before each call) ---
const G_CONTINENT: u64 = 0x0084_506c; // current continent index (-1 = world map, !=-1 = a zone)

const G_COUNT: u64 = 0x00b6_e664; // continent count

const G_BASE: u64 = 0x00b6_e668; // continent table base (stride 0x10024)

const G_IDX0: u64 = 0x0084_5070; // record-index selector A

const G_SUBBASE: u64 = 0x00c0_d5a0; // sub-entry list base (stride 0x34)

const G_SUBCOUNT: u64 = 0x00c0_d5a4; // sub-entry count

const G_RECTABLE: u64 = 0x00c0_d5bc; // record-pointer table base

const G_RECMAX: u64 = 0x00c0_d5c0; // max valid record index

const STRIDE: u64 = 0x1_0024; // continent table stride

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A finite float in `[-range, range)` from the LCG's high bits.
fn ff(s: &mut u32, range: f32) -> f32 {
    let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
    m * 2.0 * range - range
}

/// A normalized coordinate straddling `[0,1]` (so the clamp/range branches fire).
fn fuv(s: &mut u32) -> f32 {
    ff(s, 0.8) + 0.5 // [-0.3, 1.3)
}

/// A non-zero, finite zoom of either sign.
fn fzoom(s: &mut u32) -> f32 {
    let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
    let mag = m * 5.0 + 0.05;
    if next(s) & 0x800 != 0 {
        -mag
    } else {
        mag
    }
}

fn assert_f32(mine: f32, bin: f32, ctx: &str) {
    if mine.is_nan() && bin.is_nan() {
        return;
    }
    assert_eq!(
        mine.to_bits(),
        bin.to_bits(),
        "{ctx}: rust {:#010x} ({mine}) vs binary {:#010x} ({bin})",
        mine.to_bits(),
        bin.to_bits()
    );
}

fn wf(emu: &mut Emu, va: u64, v: f32) {
    emu.write_u32(va, v.to_bits());
}

#[test]
fn geo_4a6ec0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let base = emu.alloc(0x8_0000); // continent table + area bitmap
    let rectable = emu.alloc(0x800); // record-pointer table
    let record = emu.alloc(0x40); // the selected record (floats at +0x10..+0x1c)
    const N: u64 = 7; // the fixed record index we route to

    emu.write_u32(G_BASE, base as u32);
    emu.write_u32(G_RECTABLE, rectable as u32);
    emu.write_u32(G_RECMAX, 256);
    emu.write_u32(G_IDX0, 0xffff_ffff); // <0 -> n = *(base + C*0x10024 + 4)
    emu.write_u32(rectable + 4 * N, record as u32);

    // (u, v, continent, [r10,r14,r18,r1c]). Probes: interior success, du==0, dv==0, range fail.
    struct C {
        u: f32,
        v: f32,
        cont: i32,
        rec: [f32; 4],
    }
    let mut cases: Vec<C> = vec![
        C {
            u: 0.5,
            v: 0.5,
            cont: 0,
            rec: [16000.0, -16000.0, 16000.0, -16000.0],
        }, // success
        C {
            u: 0.25,
            v: 0.75,
            cont: 1,
            rec: [12000.0, -4000.0, 9000.0, -9000.0],
        }, // success
        C {
            u: 0.5,
            v: 0.5,
            cont: 2,
            rec: [100.0, 100.0, 9000.0, -9000.0],
        }, // du==0 -> None
        C {
            u: 0.5,
            v: 0.5,
            cont: 2,
            rec: [9000.0, -9000.0, 50.0, 50.0],
        }, // dv==0 -> None
        C {
            u: 0.0,
            v: 0.5,
            cont: 3,
            rec: [20000.0, 0.0, 9000.0, -9000.0],
        }, // f<0 -> range fail
        C {
            u: 1.0,
            v: 0.5,
            cont: 0,
            rec: [0.0, 20000.0, 9000.0, -9000.0],
        }, // f<0 (uc=1) -> range fail
        C {
            u: -0.4,
            v: 1.4,
            cont: 1,
            rec: [16000.0, -16000.0, 16000.0, -16000.0],
        }, // clamp both
    ];
    let mut s = 0x4a6e_00c0u32;
    for _ in 0..4000 {
        let cont = ((next(&mut s) >> 8) % 4) as i32;
        // `sum = (1-uc)·r10 + uc·r14` is a convex combo of r10,r14, so |sum| <= max(|r10|,|r14|);
        // a range straddling the 1/(2·C544) ≈ 17066 threshold makes the f/g-range-check branch fire.
        let mut rec = [
            ff(&mut s, 20000.0),
            ff(&mut s, 20000.0),
            ff(&mut s, 20000.0),
            ff(&mut s, 20000.0),
        ];
        // ~1/6 collapse a denominator to zero exactly.
        match (next(&mut s) >> 8) % 6 {
            0 => rec[1] = rec[0], // r14 = r10 -> du == 0
            1 => rec[3] = rec[2], // r1c = r18 -> dv == 0
            _ => {}
        }
        cases.push(C {
            u: fuv(&mut s),
            v: fuv(&mut s),
            cont,
            rec,
        });
    }

    let mut tally = [0u32; 5]; // [success, none, clamp-applied, du0/dv0-exact, range-fail]
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(G_CONTINENT, c.cont as u32);
        emu.write_u32(base + (c.cont as u64) * STRIDE + 4, N as u32); // n-slot (reset every case)
        for (k, &f) in c.rec.iter().enumerate() {
            wf(&mut emu, record + 0x10 + 4 * k as u64, f);
        }
        let mine = area_idx(c.u, c.v, c.cont, c.rec);
        let expect = match mine {
            None => 0u32,
            Some(idx) => {
                assert!(idx >= 0, "case {i}: negative idx {idx}");
                let off = 4 * idx as u64 + 0x14;
                assert!(off < 0x8_0000, "case {i}: idx {idx} out of seeded region");
                let marker = 0x4000_0000u32.wrapping_add(idx as u32);
                emu.write_u32(base + off, marker); // seed an identity (+offset) area bitmap
                marker
            }
        };
        let bin = emu.call(0x4a6ec0, 0, 0, &[c.u.to_bits(), c.v.to_bits()]);
        assert_eq!(
            bin, expect,
            "geo_4a6ec0 case {i} (u={} v={} cont={} rec={:?}): binary {bin:#010x} vs expect {expect:#010x} (mine={mine:?})",
            c.u, c.v, c.cont, c.rec
        );
        match mine {
            Some(_) => tally[0] += 1,
            None => tally[1] += 1,
        }
        if c.u < 0.0 || c.u > 1.0 || c.v < 0.0 || c.v > 1.0 {
            tally[2] += 1;
        }
        let du = (f64::from(c.rec[0]) - f64::from(c.rec[1])) as f32;
        let dv = (f64::from(c.rec[2]) - f64::from(c.rec[3])) as f32;
        let du0 = du == 0.0 || dv == 0.0;
        if du0 {
            tally[3] += 1;
        }
        if mine.is_none() && !du0 {
            tally[4] += 1; // None despite finite du/dv => the f/g range-check rejected it
        }
    }
    eprintln!(
        "geo_4a6ec0: {} bit-exact (area-bitmap index; clamp + du/dv + f/g f32 sites + __ftol); \
         [success={} none={} clamp={} du0/dv0={} range-fail={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2],
        tally[3],
        tally[4]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "geo_4a6ec0 coverage bucket {i} never hit");
    }
}

#[test]
fn geo_4a72b0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let sublist = emu.alloc(0x40);
    let out0 = emu.alloc(0x10);
    let out1 = emu.alloc(0x10);
    const KEY: u32 = 0x77;

    emu.write_u32(G_SUBBASE, sublist as u32);
    emu.write_u32(G_SUBCOUNT, 1);
    emu.write_u32(sublist + 4, KEY); // the entry the helper's list search matches on ECX

    let mut s = 0x4a72_00b0u32;
    let mut tally = [0u32; 2]; // [zoom>0, zoom<0]
    let mut checked = 0u32;
    for i in 0..5000 {
        let r18 = ff(&mut s, 16000.0);
        let r1c = ff(&mut s, 16000.0);
        let zoom = fzoom(&mut s);
        let arg0 = ff(&mut s, 17000.0);
        let arg1 = ff(&mut s, 17000.0);
        wf(&mut emu, sublist + 0x18, r18);
        wf(&mut emu, sublist + 0x1c, r1c);
        wf(&mut emu, sublist + 0x20, zoom);
        let mine = arrow(r18, r1c, zoom, arg0, arg1);
        emu.write_u32(out0, 0xdead_beef);
        emu.write_u32(out1, 0xdead_beef);
        // ECX=KEY, EDX=out0 ptr; stack: arg0, arg1, out1 ptr.
        emu.call(
            0x4a72b0,
            KEY,
            out0 as u32,
            &[arg0.to_bits(), arg1.to_bits(), out1 as u32],
        );
        let bin = [emu.read_f32(out0), emu.read_f32(out1)];
        assert_f32(mine[0], bin[0], &format!("geo_4a72b0 case {i} out[0]"));
        assert_f32(mine[1], bin[1], &format!("geo_4a72b0 case {i} out[1]"));
        tally[if zoom > 0.0 { 0 } else { 1 }] += 1;
        checked += 1;
    }
    eprintln!(
        "geo_4a72b0: {checked} bit-exact (axis-swapped world->normalized; 2 fstp->f32); \
         [zoom+={} zoom-={}]",
        tally[0], tally[1]
    );
    assert!(
        tally[0] > 0 && tally[1] > 0,
        "geo_4a72b0 zoom-sign coverage gap"
    );
}

#[test]
fn geo_4a7360() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let base = emu.alloc(0x4_0000);
    let rectable = emu.alloc(0x800);
    let record = emu.alloc(0x40);
    let sublist = emu.alloc(0x40);
    let incoords = emu.alloc(0x10);
    let out0 = emu.alloc(0x10);
    let out1 = emu.alloc(0x10);
    const N: u64 = 3;
    const KEY: u32 = 0x99;

    emu.write_u32(G_BASE, base as u32);
    emu.write_u32(G_RECTABLE, rectable as u32);
    emu.write_u32(G_RECMAX, 256);
    emu.write_u32(G_IDX0, 0xffff_ffff);
    emu.write_u32(G_COUNT, 1);
    emu.write_u32(rectable + 4 * N, record as u32);
    emu.write_u32(base, KEY); // *(base+0) == key (zone slot check)
    emu.write_u32(base + 4, N as u32); // zone n-slot
    emu.write_u32(record + 4, KEY); // record id == key (zone final check)
    emu.write_u32(sublist + 4, KEY); // sub-entry id (world helper match)
    emu.write_u32(G_SUBBASE, sublist as u32);

    let mut tally = [0u32; 4]; // [zone in-range, zone zeroed, zone div0-guard, world]
    let mut checked = 0u32;
    let mut s = 0x4a73_0060u32;

    // -------- Zone mode (continent 0, helper disabled): out = 1 - (in-lo)/(hi-lo), joint clamp ----
    emu.write_u32(G_CONTINENT, 0);
    emu.write_u32(G_SUBCOUNT, 0); // helper returns 0 -> zone path
    for i in 0..3000 {
        let mut rec = [
            ff(&mut s, 1000.0),
            ff(&mut s, 1000.0),
            ff(&mut s, 1000.0),
            ff(&mut s, 1000.0),
        ];
        let (mut in0, mut in1) = (ff(&mut s, 1200.0), ff(&mut s, 1200.0));
        // ~1/5 force a div0-guard (denom or num zero) exactly.
        match (next(&mut s) >> 8) % 5 {
            0 => rec[0] = rec[1], // r10 == r14 -> denomU 0
            1 => rec[2] = rec[3], // r18 == r1c -> denomV 0
            2 => in1 = rec[1],    // in1 == r14 -> numU 0
            3 => in0 = rec[3],    // in0 == r1c -> numV 0
            _ => {}
        }
        for (k, &f) in rec.iter().enumerate() {
            wf(&mut emu, record + 0x10 + 4 * k as u64, f);
        }
        wf(&mut emu, incoords, in0);
        wf(&mut emu, incoords + 4, in1);
        let mine = to_uv(
            Map4a7360::Zone {
                r10: rec[0],
                r14: rec[1],
                r18: rec[2],
                r1c: rec[3],
            },
            in0,
            in1,
        );
        emu.write_u32(out0, 0xdead_beef);
        emu.write_u32(out1, 0xdead_beef);
        emu.call(0x4a7360, incoords as u32, KEY, &[out0 as u32, out1 as u32]);
        let bin = [emu.read_f32(out0), emu.read_f32(out1)];
        assert_f32(mine[0], bin[0], &format!("geo_4a7360 zone case {i} out[0]"));
        assert_f32(mine[1], bin[1], &format!("geo_4a7360 zone case {i} out[1]"));
        if mine[0] == 0.0 && mine[1] == 0.0 {
            tally[1] += 1;
        } else {
            tally[0] += 1;
        }
        let denom_u = (f64::from(rec[0]) - f64::from(rec[1])) as f32;
        let num_u = (f64::from(in1) - f64::from(rec[1])) as f32;
        let denom_v = (f64::from(rec[2]) - f64::from(rec[3])) as f32;
        let num_v = (f64::from(in0) - f64::from(rec[3])) as f32;
        if denom_u == 0.0 || num_u == 0.0 || denom_v == 0.0 || num_v == 0.0 {
            tally[2] += 1;
        }
        checked += 1;
    }

    // -------- World mode (continent -1, helper finds the sub-entry): the forward affine map -------
    emu.write_u32(G_CONTINENT, 0xffff_ffff);
    emu.write_u32(G_SUBCOUNT, 1);
    for i in 0..2500 {
        let r18 = ff(&mut s, 16000.0);
        let r1c = ff(&mut s, 16000.0);
        let zoom = fzoom(&mut s);
        let (in0, in1) = (ff(&mut s, 1.5), ff(&mut s, 1.5));
        wf(&mut emu, sublist + 0x18, r18);
        wf(&mut emu, sublist + 0x1c, r1c);
        wf(&mut emu, sublist + 0x20, zoom);
        wf(&mut emu, incoords, in0);
        wf(&mut emu, incoords + 4, in1);
        let mine = to_uv(Map4a7360::World { r18, r1c, zoom }, in0, in1);
        emu.write_u32(out0, 0xdead_beef);
        emu.write_u32(out1, 0xdead_beef);
        emu.call(0x4a7360, incoords as u32, KEY, &[out0 as u32, out1 as u32]);
        let bin = [emu.read_f32(out0), emu.read_f32(out1)];
        assert_f32(
            mine[0],
            bin[0],
            &format!("geo_4a7360 world case {i} out[0]"),
        );
        assert_f32(
            mine[1],
            bin[1],
            &format!("geo_4a7360 world case {i} out[1]"),
        );
        tally[3] += 1;
        checked += 1;
    }

    eprintln!(
        "geo_4a7360: {checked} bit-exact (zone inv-lerp+guard+joint-clamp, world affine; \
         num/denom + result fstp->f32); [zone-in={} zone-zeroed={} div0-guard={} world={}]",
        tally[0], tally[1], tally[2], tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "geo_4a7360 coverage bucket {i} never hit");
    }
}
