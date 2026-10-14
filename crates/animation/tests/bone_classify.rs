//! `CM2Model::ResolveAnimationId` (`0x711bf0`) run whole
//! (`systems/models/animation.md#animation-id-resolution`). The exe's code runs on a model
//! (`this`, `+0x30`, `+0x130` the header and its arrays) with the animation table planted at the
//! globals `0xc0e070` and `0xc0e074`; the result dword is compared with `m2_bone_classify`.
//!
//! Cases: an id below the playable count, returned with its high word whole; an id that ends the
//! walk at once; walks that end in each of the four classes; loops by a null row, a row whose
//! fallback is itself, and a revisit; a negative fallback; the highest id included and the next
//! one excluded; the three defaults (0, 0x93 and `[hdr+0x20][0]`, trimmed to its low word); and
//! a sweep of random chains.

mod common;

use benilla_re_animation::bone_classify::{m2_bone_classify, AnimHeader, AnimationRow};
use benilla_re_difftest::{Emu, Wow};
use common::next;

const FN_VA: u64 = 0x711bf0;
/// `[0xc0e070]`, the animation table, and `[0xc0e074]`, its highest id (included).
const FLAG_TABLE_BASE_VA: u64 = 0x00c0_e070;
const FLAG_TABLE_COUNT_VA: u64 = 0x00c0_e074;

/// One synthesized classify case.
struct Case {
    playable_count: u32,
    playable: Vec<u32>,
    lookup: Vec<u16>,
    default_ids: Vec<u32>,
    lookup_len: u32,
    /// The animation table: one slot per id `0..=count` (`None` is a null pointer).
    flags: Vec<Option<AnimationRow>>,
    id: u32,
}

/// Runs a case through the exe and the transcription and compares the result dword.
fn run_case(emu: &mut Emu, arena: u64, c: &Case, ctx: &str) {
    // The arena: `this` at +0, the model data at +0x200 (`+0x130` is read), the header at +0x400,
    // the playable table at +0x500, the lookup at +0x800 (0x100 words), the default ids at +0xa00,
    // the row pointers at +0xb00, the result at +0xf00, and the rows at +0x1000 (0x20 apart).
    let this = arena;
    let modeldata = arena + 0x200;
    let header = arena + 0x400;
    let playable = arena + 0x500;
    let lookup = arena + 0x800;
    let default_ids = arena + 0xa00;
    let flag_ptrs = arena + 0xb00;
    let out = arena + 0xf00;
    let flag_structs = arena + 0x1000;

    // `this`, `+0x30`, `+0x130` lead to the header.
    emu.write_u32(this + 0x30, modeldata as u32);
    emu.write_u32(modeldata + 0x130, header as u32);
    // The header fields.
    emu.write_u32(header + 0x20, default_ids as u32);
    emu.write_u32(header + 0x24, c.lookup_len);
    emu.write_u32(header + 0x28, lookup as u32);
    emu.write_u32(header + 0x2c, c.playable_count);
    emu.write_u32(header + 0x30, playable as u32);

    for (i, &v) in c.playable.iter().enumerate() {
        emu.write_u32(playable + (i as u64) * 4, v);
    }
    for (i, &w) in c.lookup.iter().enumerate() {
        emu.write(lookup + (i as u64) * 2, &w.to_le_bytes());
    }
    for (i, &v) in c.default_ids.iter().enumerate() {
        emu.write_u32(default_ids + (i as u64) * 4, v);
    }
    // The animation table: pointers (null is 0) to rows holding the flags at `+0x14` and the
    // fallback at `+0x18`.
    for (i, f) in c.flags.iter().enumerate() {
        match f {
            None => emu.write_u32(flag_ptrs + (i as u64) * 4, 0),
            Some(e) => {
                let st = flag_structs + (i as u64) * 0x20;
                emu.write_u32(st + 0x14, e.flags);
                emu.write_u32(st + 0x18, e.fallback);
                emu.write_u32(flag_ptrs + (i as u64) * 4, st as u32);
            }
        }
    }
    // The globals: the table and its highest id. The table is indexed `0..=count`, so it has
    // `count + 1` slots.
    let count = (c.flags.len() as u32).saturating_sub(1);
    emu.write_u32(FLAG_TABLE_BASE_VA, flag_ptrs as u32);
    emu.write_u32(FLAG_TABLE_COUNT_VA, count);

    // The result slot is poisoned so a missing write cannot match.
    emu.write_u32(out, 0xdead_beef);

    emu.clear_calls();
    // `thiscall`: `ecx` is `this`, `[ebp+8]` the result pointer, `[ebp+0xc]` the id; `ret 8`.
    emu.call(FN_VA, this as u32, 0, &[out as u32, c.id]);
    let bin = emu.read_u32(out);

    let hdr = AnimHeader {
        playable_count: c.playable_count,
        playable: &c.playable,
        lookup: &c.lookup,
        default_ids: &c.default_ids,
        lookup_len: c.lookup_len,
    };
    let ours = m2_bone_classify(&hdr, &c.flags, count, c.id);

    assert_eq!(
        bin, ours,
        "{ctx}: out dword (bin={bin:#010x} ours={ours:#010x})"
    );
}

/// A base case above the playable count: the walk starts at `id`, the lookup holds `lookup[i] = i`
/// below `lookup_len` and 0xffff from there, and the table is `count + 1` null rows. Variant 0
/// has `lookup[0]` not 0xffff, 1 has it 0xffff with entry 0x93 not, and 2 has both 0xffff, so the
/// default is the dword `default_ids[0]`.
fn base_case(s: &mut u32, count: u32, lookup_len: u32, id: u32, seed_variant: u32) -> Case {
    let playable_count = 4;
    let mut lookup: Vec<u16> = (0..0x100u32)
        .map(|i| if i < lookup_len { i as u16 } else { 0xffff })
        .collect();
    match seed_variant {
        0 => lookup[0] = 7, // not 0xffff: the default is 0, and id 0 ends a walk
        1 => {
            lookup[0] = 0xffff;
            lookup[0x93] = 5; // not 0xffff: the default is 0x93
        }
        _ => {
            lookup[0] = 0xffff;
            // The default is `default_ids[0]`, a full dword that the result trims.
            lookup[0x93] = 0xffff;
        }
    }
    Case {
        playable_count,
        playable: (0..playable_count).map(|_| next(s)).collect(),
        lookup,
        default_ids: vec![next(s), 0, 0, 0],
        lookup_len,
        flags: vec![None; (count + 1) as usize],
        id,
    }
}

#[test]
fn m2_bone_classify_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let arena = emu.alloc(0x2000);

    let mut s = 0x711b_f000u32;
    let mut checked = 0u32;

    // An id below the playable count: the full 32-bit dword, across counts and boundary ids.
    for case in 0..120u32 {
        let playable_count = 1 + (next(&mut s) >> 8) % 64;
        let playable: Vec<u32> = (0..playable_count).map(|_| next(&mut s)).collect();
        let id = match case % 4 {
            0 => 0,
            1 => playable_count - 1,
            _ => (next(&mut s) >> 8) % playable_count,
        };
        let c = Case {
            playable_count,
            playable,
            lookup: vec![0xffff; 0x100],
            default_ids: vec![0; 4],
            lookup_len: (next(&mut s) >> 8) % 64,
            flags: vec![None; 8],
            id,
        };
        run_case(&mut emu, arena, &c, &format!("fast {case}"));
        checked += 1;
    }

    // The walk. (a) A loop through a null row, with all three defaults (class 0).
    for variant in 0..3u32 {
        let c = base_case(&mut s, 16, 4, 10, variant);
        run_case(&mut emu, arena, &c, &format!("seed variant {variant}"));
        checked += 1;
    }
    // (b) The walk ends at once: an id in `[playable_count, lookup_len)` with a word.
    {
        let mut c = base_case(&mut s, 16, 8, 6, 2);
        c.lookup[6] = 3;
        run_case(&mut emu, arena, &c, "immediate early-break");
        checked += 1;
    }
    // (c) Walks that end in each class, with `lookup_len` 4, the chain at ids from 4, and the
    // walk ending at id 2 (`lookup[2] = 2`; default variant 2 rewrites only entries 0 and 0x93).
    for (class_tag, chains) in [
        // Class 0: one hop, no flags; the direction stays +1.
        (0u32, vec![(10u32, 0u32, 2u32)]),
        // Class 1: one flag `0x10`; the accumulator is 1 and the direction -1.
        (1, vec![(10, 0x10, 2)]),
        // Class 2: `0x10` then `0x20`; the accumulator is 0 and the direction 0.
        (2, vec![(10, 0x10, 11), (11, 0x20, 2)]),
        // Class 3: one flag `0x20`; the accumulator is 1 and the direction 0.
        (3, vec![(10, 0x20, 2)]),
    ] {
        let mut c = base_case(&mut s, 16, 4, 10, 0);
        c.lookup[2] = 2;
        for (idx, flags, nxt) in &chains {
            c.flags[*idx as usize] = Some(AnimationRow {
                flags: *flags,
                fallback: *nxt,
            });
        }
        run_case(&mut emu, arena, &c, &format!("class {class_tag} walk"));
        checked += 1;
    }
    // (d) A loop through a row whose fallback is itself (the default, class 0).
    {
        let mut c = base_case(&mut s, 16, 4, 10, 1);
        c.flags[10] = Some(AnimationRow {
            flags: 0x30,
            fallback: 10,
        });
        run_case(&mut emu, arena, &c, "self-link");
        checked += 1;
    }
    // (e) A loop through a revisit: ids 10 and 11 fall back to each other.
    {
        let mut c = base_case(&mut s, 16, 4, 10, 0);
        c.flags[10] = Some(AnimationRow {
            flags: 0x10,
            fallback: 11,
        });
        c.flags[11] = Some(AnimationRow {
            flags: 0x20,
            fallback: 10,
        });
        run_case(&mut emu, arena, &c, "revisit 2-cycle");
        checked += 1;
    }
    // (f) A negative fallback (0xffffffff): the signed test finds no row, and the walk loops.
    {
        let mut c = base_case(&mut s, 16, 4, 10, 0);
        c.flags[10] = Some(AnimationRow {
            flags: 0,
            fallback: 0xffff_ffff,
        });
        run_case(&mut emu, arena, &c, "negative link");
        checked += 1;
    }
    // (g) The highest id is included: a cursor equal to `count` finds its row, and `count + 1`
    // does not.
    {
        let count = 12u32;
        let mut c = base_case(&mut s, count, 4, count, 0);
        c.lookup[2] = 2;
        c.flags[count as usize] = Some(AnimationRow {
            flags: 0x10,
            fallback: 2,
        }); // at the highest id: the walk continues
        run_case(&mut emu, arena, &c, "cursor == count (inclusive)");
        let c2 = base_case(&mut s, count, 4, count + 1, 0); // beyond: no row, and the default
        run_case(&mut emu, arena, &c2, "cursor == count+1 (excluded)");
        checked += 2;
    }

    // Random chains, whichever way they end.
    for case in 0..160u32 {
        let count = 8 + (next(&mut s) >> 8) % 24; // at most 31, under the 208 visited flags
        let lookup_len = 2 + (next(&mut s) >> 8) % 6;
        let mut c = base_case(
            &mut s,
            count,
            lookup_len,
            count.min(4 + case % 20),
            case % 3,
        );
        // Words below `lookup_len` that end a walk.
        for i in 0..lookup_len {
            if (next(&mut s) >> 8).is_multiple_of(2) {
                c.lookup[i as usize] = i as u16;
            } else {
                c.lookup[i as usize] = 0xffff;
            }
        }
        if case % 3 == 0 {
            c.lookup[0] = 0xffff; // keeps the default's probe in half the cases
        }
        // Random rows over `[0, count]`: fallbacks mostly in the table, sometimes outside it.
        for i in 0..=count {
            if (next(&mut s) >> 8).is_multiple_of(3) {
                continue; // a null row
            }
            let next_link = match (next(&mut s) >> 8) % 8 {
                0 => 0xffff_ffff,                         // negative
                1 => count + 1 + (next(&mut s) >> 8) % 4, // outside the table
                2 => i,                                   // itself
                _ => (next(&mut s) >> 8) % (count + 1),   // in the table, cycles included
            };
            let flags = ((next(&mut s) >> 8) % 4) << 4; // 0 / 0x10 / 0x20 / 0x30
            c.flags[i as usize] = Some(AnimationRow {
                flags,
                fallback: next_link,
            });
        }
        run_case(&mut emu, arena, &c, &format!("fuzz chain {case}"));
        checked += 1;
    }

    eprintln!(
        "m2_bone_classify: {checked} classifies bit-exact vs WoW.exe @ 0x711bf0 \
         (fast path + the full chain leg)"
    );
}
