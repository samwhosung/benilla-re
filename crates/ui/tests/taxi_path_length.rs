//! The summing loop of `TaxiPathLength` (`0x4dbbd0`, `systems/ui/taxiroute.md#route-search`) run from
//! the exe.
//!
//! The function reaches its run of points through two table lookups and a hash probe, all in `.bss`.
//! The test seeds that chain so that the loop at `0x4dbc68` runs over chosen points, and compares
//! the returned length with `taxi_path_length`; a missed key returns the constant
//! `PATH_LENGTH_NONE`. Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::taxi_map::{taxi_path_length as arclen, PATH_LENGTH_NONE as ARCLEN_ABSENT};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// f32 in `[lo, hi)` from the LCG high bits.
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

fn assert_bits(mine: f32, bin: f32, ctx: &str) {
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

// ---- geo_4dbbd0: the `.data` lookup chain we seed so the Σ√ loop runs over our points ----
const G_TABLE_A: u64 = 0x00bb_4f10; // [ecx<<5 + 0xbb4f10] -> ptrA

const G_TABLE_B: u64 = 0x00b7_365c; // [4*((cval<<8)+cval) + 0xb7365c] -> ptrB

const G_MASK: u64 = 0x00bb_4ae8; // hashmap mask (-1 => absent-key sentinel)

const G_BUCKETS: u64 = 0x00bb_4ae0; // hashmap bucket-table base

/// Seed the lookup chain (indices ecx=edx=0) so the hashmap resolves to `node`. `key` is chosen
/// so `key & mask` gives `hash`; the bucket slot `[buckets + 12*hash + 8]` points at `node`, and
/// `[node] == key` makes the first probe match (no chain walk). Returns nothing; writes globals.
fn seed_lookup(
    emu: &mut Emu,
    ptr_a: u64,
    ptr_b: u64,
    buckets: u64,
    key: u32,
    mask: u32,
    node: u64,
) {
    emu.write_u32(G_TABLE_A, ptr_a as u32);
    emu.write_u32(ptr_a, 0); // cval = 0 -> index 0 into table B
    emu.write_u32(G_TABLE_B, ptr_b as u32);
    emu.write_u32(ptr_b, key); // the hashmap key
    emu.write_u32(G_MASK, mask);
    emu.write_u32(G_BUCKETS, buckets as u32);
    let hash = key & mask;
    emu.write_u32(buckets + 12 * u64::from(hash) + 8, node as u32);
    emu.write_u32(node, key); // [node] == key -> probe matches
}

#[test]
fn geo_4dbbd0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let ptr_a = emu.alloc(0x10);
    let ptr_b = emu.alloc(0x10);
    let buckets = emu.alloc(0x400);
    let node = emu.alloc(0x40);
    let points = emu.alloc(0x24 * 300); // stride 0x24, xyz at element +0x10
    const KEY: u32 = 0x1234;
    const MASK: u32 = 0xff;
    seed_lookup(&mut emu, ptr_a, ptr_b, buckets, KEY, MASK, node);
    emu.write_u32(node + 0x18, points as u32); // [node+0x18] -> points

    // Write a point array into scratch (xyz at element offset +0x10) and return it as a Vec.
    let write_points = |emu: &mut Emu, pts: &[[f32; 3]]| {
        for (i, p) in pts.iter().enumerate() {
            let base = points + 0x24 * i as u64 + 0x10;
            emu.write_u32(base, p[0].to_bits());
            emu.write_u32(base + 4, p[1].to_bits());
            emu.write_u32(base + 8, p[2].to_bits());
        }
    };

    // Hand-picked segment counts straddling the loop's edges.
    let mut cases: Vec<Vec<[f32; 3]>> = vec![
        vec![],                                 // count 0 -> 0.0
        vec![[1.0, 2.0, 3.0]],                  // count 1 -> 0.0 (no segment)
        vec![[0.0, 0.0, 0.0], [3.0, 4.0, 0.0]], // 1 seg, len 5.0
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
        ], // 3 segs
        vec![[-100.5, 7.25, 3.0], [200.0, -50.0, 9.0], [0.0, 0.0, 0.0]],
    ];
    let mut s = 0x4dbb_00d0u32;
    for _ in 0..1500 {
        let count = ((next(&mut s) >> 8) % 60) as usize; // 0..59 points
        let mut pts = Vec::with_capacity(count);
        for _ in 0..count {
            pts.push([
                rng(&mut s, -5000.0, 5000.0),
                rng(&mut s, -5000.0, 5000.0),
                rng(&mut s, -5000.0, 5000.0),
            ]);
        }
        cases.push(pts);
    }

    let mut tally = [0u32; 4]; // [count0, count1, count2, count>2]
    for (i, pts) in cases.iter().enumerate() {
        assert!(pts.len() <= 300);
        write_points(&mut emu, pts);
        emu.write_u32(node + 0x1c, pts.len() as u32); // [node+0x1c] = count
        let mine = arclen(pts);
        let bin = emu.call_f32(0x4dbbd0, 0, 0, &[]);
        assert_bits(
            mine,
            bin,
            &format!("geo_4dbbd0 case {i} (count={})", pts.len()),
        );
        match pts.len() {
            0 => tally[0] += 1,
            1 => tally[1] += 1,
            2 => tally[2] += 1,
            _ => tally[3] += 1,
        }
    }

    // Absent-key sentinel: mask == -1 short-circuits to `fld [0x807668]` (9.99e10).
    emu.write_u32(G_MASK, 0xffff_ffff);
    let bin_absent = emu.call_f32(0x4dbbd0, 0, 0, &[]);
    assert_bits(ARCLEN_ABSENT, bin_absent, "geo_4dbbd0 absent-key sentinel");
    emu.write_u32(G_MASK, MASK); // restore (not strictly needed)

    eprintln!(
        "geo_4dbbd0: {} bit-exact (Σ√ arc-length; Δx pre-square f32 + per-iter sum f32; \
         sentinel=9.99e10); [count0={} count1={} count2={} count>2={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2],
        tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "geo_4dbbd0 segment-count bucket {i} never hit");
    }
}
