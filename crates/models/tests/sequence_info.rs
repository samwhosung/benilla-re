//! `CM2Model::GetSequenceInfo` (`0x711a20`, `systems/models/model.md#sequence-info`) run whole,
//! its `ResolveAnimationId` (`0x711bf0`) and `M2LookupIndex` (`0x710310`) calls included. The
//! resolved dword the transcription starts from is the exe's own `0x711bf0` on the same model,
//! which `animation::m2_bone_classify_difftest` covers; the record is compared word for word.

#![allow(clippy::needless_range_loop)] // index loops follow the record layout

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_models::model::sequence_info;
use common::Lcg;

const FN_VA: u64 = 0x711a20;
const RESOLVE_VA: u64 = 0x711bf0;
/// `[0xc0e070]`, the animation table, and `[0xc0e074]`, its highest id.
const ROWS_VA: u64 = 0x00c0_e070;
const MAX_ID_VA: u64 = 0x00c0_e074;

const N_SEQ_MAX: usize = 12;
const LOOKUP_LEN_MAX: usize = 0xa0;

struct Arena {
    model: u64,
    shared: u64,
    header: u64,
    seqs: u64,
    lookup: u64,
    playable: u64,
    rows: u64,
    row_structs: u64,
    out: u64,
    resolved: u64,
}

struct Case {
    row0: [f32; 3],
    sequences: Vec<[u32; 17]>,
    lookup: Vec<u16>,
    playable: Vec<u32>,
    id: u32,
    variation: u32,
}

fn write_f32(e: &mut Emu, va: u64, v: f32) {
    e.write_u32(va, v.to_bits());
}

fn run_case(e: &mut Emu, a: &Arena, c: &Case, ctx: &str) {
    e.write_u32(a.model + 0x10, 1); // loaded: no wait
    e.write_u32(a.model + 0x30, a.shared as u32);
    for k in 0..3 {
        write_f32(e, a.model + 0xbc + 4 * k as u64, c.row0[k]);
    }
    e.write_u32(a.shared + 0x130, a.header as u32);
    e.write_u32(a.header + 0x1c, c.sequences.len() as u32);
    e.write_u32(a.header + 0x20, a.seqs as u32);
    e.write_u32(a.header + 0x24, c.lookup.len() as u32);
    e.write_u32(a.header + 0x28, a.lookup as u32);
    e.write_u32(a.header + 0x2c, c.playable.len() as u32);
    e.write_u32(a.header + 0x30, a.playable as u32);
    for (i, s) in c.sequences.iter().enumerate() {
        for (k, &w) in s.iter().enumerate() {
            e.write_u32(a.seqs + 0x44 * i as u64 + 4 * k as u64, w);
        }
    }
    // The lookup's words past its count stay readable as 0xffff: the default probes entry 0x93.
    for i in 0..LOOKUP_LEN_MAX {
        let w = c.lookup.get(i).copied().unwrap_or(0xffff);
        e.write(a.lookup + 2 * i as u64, &w.to_le_bytes());
    }
    for (i, &w) in c.playable.iter().enumerate() {
        e.write_u32(a.playable + 4 * i as u64, w);
    }
    for k in 0..15u64 {
        e.write_u32(a.out + 4 * k, 0xdead_beef);
    }

    e.call(RESOLVE_VA, a.model as u32, 0, &[a.resolved as u32, c.id]);
    let resolved = e.read_u32(a.resolved);
    e.call(FN_VA, a.model as u32, 0, &[c.id, c.variation, a.out as u32]);

    let ours = sequence_info(resolved, c.variation, &c.lookup, &c.sequences, c.row0);
    for k in 0..15 {
        let bin = e.read_u32(a.out + 4 * k as u64);
        let mine = ours[k];
        let both_nan = (k == 2 || (3..13).contains(&k))
            && f32::from_bits(bin).is_nan()
            && f32::from_bits(mine).is_nan();
        assert!(
            bin == mine || both_nan,
            "{ctx}: word {k}: binary={bin:#010x} ours={mine:#010x} (resolved {resolved:#x})"
        );
    }
}

fn sequence(rng: &mut Lcg, n_seq: usize) -> [u32; 17] {
    let mut s = [0u32; 17];
    s[0] = rng.next() % 0x30;
    s[1] = rng.next();
    s[2] = if rng.next() & 1 == 0 {
        s[1].wrapping_add(rng.next() % 5000)
    } else {
        rng.next()
    };
    s[3] = rng.f32_mixed().to_bits();
    s[4] = rng.next();
    for w in s.iter_mut().take(16).skip(5) {
        *w = rng.f32_mixed().to_bits();
    }
    // The next variation: in the table, past it, or 0xffff; the high half is the next field.
    let next = match rng.next() >> 30 {
        0 => 0xffff,
        1 => (n_seq + (rng.next() % 3) as usize) as u32,
        _ => rng.next() % n_seq as u32,
    };
    s[16] = (rng.next() & 0xffff_0000) | next;
    s
}

#[test]
fn sequence_info_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let a = Arena {
        model: e.alloc(0x428),
        shared: e.alloc(0x140),
        header: e.alloc(0x160),
        seqs: e.alloc(0x44 * N_SEQ_MAX as u64),
        lookup: e.alloc(2 * LOOKUP_LEN_MAX as u64),
        playable: e.alloc(0x40),
        rows: e.alloc(4 * 0x31),
        row_structs: e.alloc(0x20 * 0x31),
        out: e.alloc(0x40),
        resolved: e.alloc(0x10),
    };
    // The animation table: ids 0 to 0x30, every third row present, falling back to id − 1, flag
    // 0x10 on every fifth; the rest null.
    e.write_u32(ROWS_VA, a.rows as u32);
    e.write_u32(MAX_ID_VA, 0x30);
    for i in 0..0x31u64 {
        let row = a.row_structs + 0x20 * i;
        if i % 3 == 1 {
            e.write_u32(row + 0x14, if i % 5 == 0 { 0x10 } else { 0 });
            e.write_u32(row + 0x18, i as u32 - 1);
            e.write_u32(a.rows + 4 * i, row as u32);
        } else {
            e.write_u32(a.rows + 4 * i, 0);
        }
    }

    // Deterministic probes: a unit row 0, an id through the playable table, a mapped id, a
    // variation chain, a chain that leaves the table, a lookup miss, and an index past the table.
    let unit = |s: &mut [u32; 17]| {
        s[1] = 100;
        s[2] = 1100;
        s[3] = 2.5f32.to_bits();
        s[4] = 0x20;
        for k in 0..6 {
            s[9 + k] = (k as f32 - 2.0).to_bits();
        }
        s[15] = 3.0f32.to_bits();
    };
    let mut seqs = vec![[0u32; 17]; 4];
    for (i, s) in seqs.iter_mut().enumerate() {
        unit(s);
        s[0] = i as u32;
        s[16] = (i as u32 + 1) % 4;
    }
    seqs[3][16] = 0xffff;
    let lookup: Vec<u16> = vec![0, 1, 0xffff, 3, 2, 9];
    let base = Case {
        row0: [1.0, 0.0, 0.0],
        sequences: seqs.clone(),
        lookup: lookup.clone(),
        playable: vec![],
        id: 1,
        variation: 0,
    };
    run_case(&mut e, &a, &base, "mapped id");
    for v in 0..6 {
        run_case(
            &mut e,
            &a,
            &Case {
                variation: v,
                ..clone(&base)
            },
            &format!("variation {v}"),
        );
    }
    run_case(
        &mut e,
        &a,
        &Case {
            id: 5,
            ..clone(&base)
        },
        "index past the table",
    );
    run_case(
        &mut e,
        &a,
        &Case {
            id: 2,
            ..clone(&base)
        },
        "lookup miss resolves",
    );
    run_case(
        &mut e,
        &a,
        &Case {
            playable: vec![0x0001_0003, 4],
            id: 0,
            ..clone(&base)
        },
        "playable",
    );
    run_case(
        &mut e,
        &a,
        &Case {
            playable: vec![0x0003_0001, 5],
            id: 1,
            ..clone(&base)
        },
        "playable past",
    );
    run_case(
        &mut e,
        &a,
        &Case {
            row0: [3.0, 4.0, 12.0],
            ..clone(&base)
        },
        "row0 length 13",
    );
    run_case(
        &mut e,
        &a,
        &Case {
            row0: [0.0, 0.0, 0.0],
            ..clone(&base)
        },
        "row0 zero",
    );
    run_case(
        &mut e,
        &a,
        &Case {
            row0: [f32::NAN, 1.0, 0.0],
            ..clone(&base)
        },
        "row0 NaN",
    );
    run_case(
        &mut e,
        &a,
        &Case {
            row0: [f32::INFINITY, 0.0, 0.0],
            ..clone(&base)
        },
        "row0 inf",
    );

    let mut rng = Lcg(0x711a_2000);
    for i in 0..3000 {
        let n_seq = 1 + (rng.next() % N_SEQ_MAX as u32) as usize;
        let sequences: Vec<[u32; 17]> = (0..n_seq).map(|_| sequence(&mut rng, n_seq)).collect();
        let n_look = 1 + (rng.next() % 0x40) as usize;
        let lookup: Vec<u16> = (0..n_look)
            .map(|_| match rng.next() >> 29 {
                0 => 0xffff,
                1 => (n_seq + 1) as u16,
                _ => (rng.next() % n_seq as u32) as u16,
            })
            .collect();
        let n_play = (rng.next() % 4) as usize;
        let playable: Vec<u32> = (0..n_play)
            .map(|_| ((rng.next() % 4) << 16) | (rng.next() % 0x48))
            .collect();
        // Ids 1 and above, so a walk never reads the null row the table holds at 0.
        let id = 1 + rng.next() % 0x48;
        let variation = rng.next() % 6;
        let row0 = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        let c = Case {
            row0,
            sequences,
            lookup,
            playable,
            id,
            variation,
        };
        run_case(&mut e, &a, &c, &format!("fuzz[{i}]"));
    }
}

fn clone(c: &Case) -> Case {
    Case {
        row0: c.row0,
        sequences: c.sequences.clone(),
        lookup: c.lookup.clone(),
        playable: c.playable.clone(),
        id: c.id,
        variation: c.variation,
    }
}
