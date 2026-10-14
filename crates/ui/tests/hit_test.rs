//! `CSimpleFrame::IsMouseOver` (`0x76b020`, `systems/ui/input.md#hit-test`) and
//! `CSimpleTop::RegisterForEvent` (`0x764aa0`, `#input-lists`) run from the exe.
//!
//! The point test runs whole on a synthesized frame, its parent and a grandparent (the nearest
//! parent without the self-drawing bit gives the clip rectangle); the rectangle builder `0x4ef370` is
//! four stores and runs unstubbed. `RegisterForEvent` runs with the node allocation (`0x6462e0`)
//! replaced by a bump allocator and its list pre-sized, and the test reads the list back to
//! recover the list index and the insert position. Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::hit_test::{
    point_in_rect as lib_point_in_rect,
    register_for_event_insert as lib_input_hittest_index_insert, BatchInsert,
};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

// ───────────────────────────── point_in_rect ─────────────────────────────

const CLIP_FLAG: u32 = 0x2000; // frame +0xb4 bit: clip-to-ancestor

/// Write a `[bottom, left, top, right]` rect at `base+0xd8..+0xe4`.
fn write_rect(emu: &mut Emu, base: u64, rect: [f32; 4]) {
    for (k, v) in rect.iter().enumerate() {
        emu.write_u32(base + 0xd8 + (k as u64) * 4, v.to_bits());
    }
}

#[test]
fn point_in_rect() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // frame f -> parent p -> grandparent g (g always lacks the clip flag); a point slot.
    let f = emu.alloc(0x100);
    let p = emu.alloc(0x100);
    let g = emu.alloc(0x100);
    let pt = emu.alloc(8);
    emu.write_u32(g + 0xb4, 0); // g lacks 0x2000 (always the clip base when reached)
    emu.write_u32(g + 0x9c, 0); // g.parent = null

    struct C {
        cache_valid: bool,
        clip_self: bool,
        own: [f32; 4],
        parent_null: bool, // f.parent == 0 (clip path → None)
        p_has_flag: bool,  // parent has 0x2000 (→ walk continues to g)
        p_rect: [f32; 4],
        g_rect: [f32; 4],
        px: f32,
        py: f32,
    }
    // own/p/g rects as [bottom, left, top, right].
    let r_own = [10.0f32, 20.0, 200.0, 300.0];
    let r_p = [30.0f32, 40.0, 150.0, 250.0];
    let r_g = [0.0f32, 0.0, 180.0, 280.0];
    let mut cases: Vec<C> = vec![
        // cache invalid -> false (regardless of point).
        C {
            cache_valid: false,
            clip_self: false,
            own: r_own,
            parent_null: true,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 100.0,
            py: 100.0,
        },
        // no clip, point well inside.
        C {
            cache_valid: true,
            clip_self: false,
            own: r_own,
            parent_null: true,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 100.0,
            py: 100.0,
        },
        // no clip, outside each edge.
        C {
            cache_valid: true,
            clip_self: false,
            own: r_own,
            parent_null: true,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 19.0,
            py: 100.0,
        }, // x < left
        C {
            cache_valid: true,
            clip_self: false,
            own: r_own,
            parent_null: true,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 301.0,
            py: 100.0,
        }, // x > right
        C {
            cache_valid: true,
            clip_self: false,
            own: r_own,
            parent_null: true,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 100.0,
            py: 9.0,
        }, // y < bottom
        C {
            cache_valid: true,
            clip_self: false,
            own: r_own,
            parent_null: true,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 100.0,
            py: 201.0,
        }, // y > top
        // no clip, on the boundary (== edge) -> inside.
        C {
            cache_valid: true,
            clip_self: false,
            own: r_own,
            parent_null: true,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 20.0,
            py: 10.0,
        },
        C {
            cache_valid: true,
            clip_self: false,
            own: r_own,
            parent_null: true,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 300.0,
            py: 200.0,
        },
        // no clip, NaN coordinate -> outside.
        C {
            cache_valid: true,
            clip_self: false,
            own: r_own,
            parent_null: true,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: f32::NAN,
            py: 100.0,
        },
        // clip, parent lacks flag -> intersect own∩p; point inside the intersection.
        C {
            cache_valid: true,
            clip_self: true,
            own: r_own,
            parent_null: false,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 100.0,
            py: 100.0,
        },
        // clip, point inside own but outside own∩p (x within own [20,300] but > p.right 250).
        C {
            cache_valid: true,
            clip_self: true,
            own: r_own,
            parent_null: false,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 280.0,
            py: 100.0,
        },
        // clip, parent HAS flag -> walk to g; point inside own∩g.
        C {
            cache_valid: true,
            clip_self: true,
            own: r_own,
            parent_null: false,
            p_has_flag: true,
            p_rect: r_p,
            g_rect: r_g,
            px: 100.0,
            py: 100.0,
        },
        // clip, no parent -> None -> false.
        C {
            cache_valid: true,
            clip_self: true,
            own: r_own,
            parent_null: true,
            p_has_flag: false,
            p_rect: r_p,
            g_rect: r_g,
            px: 100.0,
            py: 100.0,
        },
    ];

    // fuzz: random rects, flags, point (sometimes inside, sometimes NaN/boundary).
    let mut s = 0x76b0_2000u32;
    let frect = |s: &mut u32| -> [f32; 4] {
        let lo_b = (next(s) >> 8) as f32 / 200.0;
        let lo_l = (next(s) >> 8) as f32 / 200.0;
        let hi_t = lo_b + 1.0 + (next(s) >> 8) as f32 / 400.0;
        let hi_r = lo_l + 1.0 + (next(s) >> 8) as f32 / 400.0;
        [lo_b, lo_l, hi_t, hi_r]
    };
    for _ in 0..4000 {
        let own = frect(&mut s);
        let p_rect = frect(&mut s);
        let g_rect = frect(&mut s);
        // point: bias toward the own rect's interior, with NaN/extreme draws.
        let px = match (next(&mut s) >> 8) % 8 {
            0 => f32::NAN,
            1 => own[1], // == left (boundary)
            2 => own[3], // == right
            _ => {
                own[1] + (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * (own[3] - own[1] + 40.0)
                    - 20.0
            }
        };
        let py = match (next(&mut s) >> 8) % 8 {
            0 => f32::NAN,
            1 => own[0],
            2 => own[2],
            _ => {
                own[0] + (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * (own[2] - own[0] + 40.0)
                    - 20.0
            }
        };
        cases.push(C {
            cache_valid: next(&mut s) & 0x10000 != 0,
            clip_self: next(&mut s) & 0x100 != 0,
            own,
            parent_null: next(&mut s) & 0x100 == 0, // ~50% null parent (exercises None)
            p_has_flag: next(&mut s) & 0x200 != 0,
            p_rect,
            g_rect,
            px,
            py,
        });
    }

    let mut tally = [0u32; 6]; // [cache_invalid, noclip_in, noclip_out, clip_in, clip_out, clip_none]
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(f + 0x60, if c.cache_valid { 1 } else { 0 });
        emu.write_u32(f + 0xb4, if c.clip_self { CLIP_FLAG } else { 0 });
        write_rect(&mut emu, f, c.own);
        emu.write_u32(f + 0x9c, if c.parent_null { 0 } else { p as u32 });
        emu.write_u32(p + 0xb4, if c.p_has_flag { CLIP_FLAG } else { 0 });
        write_rect(&mut emu, p, c.p_rect);
        emu.write_u32(p + 0x9c, g as u32);
        write_rect(&mut emu, g, c.g_rect);
        emu.write_u32(pt, c.px.to_bits());
        emu.write_u32(pt + 4, c.py.to_bits());

        let bin = emu.call(0x76b020, f as u32, 0, &[pt as u32]) != 0;

        // resolve the clip ancestor the way the binary's parent walk does.
        let clip = if c.clip_self {
            if c.parent_null {
                None
            } else if c.p_has_flag {
                Some(c.g_rect) // p has flag -> walk to g (lacks it)
            } else {
                Some(c.p_rect)
            }
        } else {
            None
        };
        let mine = lib_point_in_rect(c.cache_valid, c.clip_self, c.own, clip, c.px, c.py);
        assert_eq!(
            bin, mine,
            "point_in_rect case {i}: binary {bin} vs rust {mine} \
             (cache={} clip={} null={} pflag={} pt=({},{}))",
            c.cache_valid, c.clip_self, c.parent_null, c.p_has_flag, c.px, c.py
        );

        let leg = if !c.cache_valid {
            0
        } else if !c.clip_self {
            if mine {
                1
            } else {
                2
            }
        } else if clip.is_none() {
            5
        } else if mine {
            3
        } else {
            4
        };
        tally[leg] += 1;
    }
    eprintln!(
        "point_in_rect: {} bit-exact; [cache_invalid={} noclip_in={} noclip_out={} clip_in={} clip_out={} clip_none={}]",
        cases.len(), tally[0], tally[1], tally[2], tally[3], tally[4], tally[5]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "point_in_rect leg {i} never hit");
    }
}

// ──────────────────────── input_hittest_index_insert ────────────────────────

#[test]
fn input_hittest_index_insert() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // CSimpleTop with its 36-bucket batch array at +0xd10 (each bucket 0x1c bytes) + the +0x1100
    // flag. Allocate generously past +0x1108.
    let top = emu.alloc(0x1200);
    let dummy = emu.alloc(0x80); // shared empty data buffer for the 35 unused buckets
                                 // node SMemAlloc (0x6462e0) -> bump allocator; reset to `new_node` each case.
    let new_node = emu.alloc(0x40);
    emu.stub_alloc(0x6462e0, new_node, 0x40, 4); // ret 0x10 (callee cleans 4 dword args)

    let bucket_at = |b: u32| top + 0xd10 + (b as u64) * 0x1c;
    // init all 36 buckets to a skip-grow empty state (count != +0x14 sentinel).
    for b in 0..36u32 {
        let bk = bucket_at(b);
        emu.write_u32(bk, 0);
        emu.write_u32(bk + 0x4, 0);
        emu.write_u32(bk + 0x8, dummy as u32); // data ptr (valid, never grown)
        emu.write_u32(bk + 0x10, 0); // count
        emu.write_u32(bk + 0x14, 0x7fff_ffff); // != count -> grow path skipped
        emu.write_u32(bk + 0x18, 0); // high-water
    }

    let frame = emu.alloc(0x100);

    struct C {
        strata: u32,
        cat: u32,
        level_arg: u32, // 0xffff_ffff => use frame_level
        frame_level: u32,
        existing: Vec<u32>, // existing entry levels (kept descending), in array order
        flag: u32,          // arg2; flag==0 && cat==2 sets top+0x1100
    }
    let mut cases: Vec<C> = vec![
        // empty bucket -> insert at 0.
        C {
            strata: 3,
            cat: 0,
            level_arg: 5,
            frame_level: 0,
            existing: vec![],
            flag: 1,
        },
        // insert at front (highest level).
        C {
            strata: 0,
            cat: 0,
            level_arg: 9,
            frame_level: 0,
            existing: vec![5, 3, 1],
            flag: 1,
        },
        // insert in the middle.
        C {
            strata: 8,
            cat: 1,
            level_arg: 4,
            frame_level: 0,
            existing: vec![6, 5, 2],
            flag: 1,
        },
        // insert at the end (lowest).
        C {
            strata: 2,
            cat: 3,
            level_arg: 0,
            frame_level: 0,
            existing: vec![6, 4, 2],
            flag: 1,
        },
        // equal level -> goes after equals (cmp;ja is strict >).
        C {
            strata: 1,
            cat: 2,
            level_arg: 4,
            frame_level: 0,
            existing: vec![6, 4, 4, 1],
            flag: 1,
        },
        // level = -1 -> use frame_level (7) -> front-ish.
        C {
            strata: 5,
            cat: 0,
            level_arg: 0xffff_ffff,
            frame_level: 7,
            existing: vec![8, 7, 3],
            flag: 1,
        },
        // flag==0 && cat==2 -> sets top+0x1100.
        C {
            strata: 4,
            cat: 2,
            level_arg: 2,
            frame_level: 0,
            existing: vec![5, 1],
            flag: 0,
        },
    ];
    let mut s = 0x764a_a000u32;
    for _ in 0..1500 {
        let strata = (next(&mut s) >> 8) % 9;
        let cat = (next(&mut s) >> 8) % 4;
        let n = (next(&mut s) >> 8) as usize % 6;
        // build a descending list of distinct-ish levels.
        let mut lvl = 30u32;
        let mut existing = Vec::with_capacity(n);
        for _ in 0..n {
            existing.push(lvl);
            lvl = lvl.saturating_sub(1 + (next(&mut s) >> 8) % 3);
        }
        let use_self = next(&mut s) & 0x10000 != 0;
        let frame_level = (next(&mut s) >> 8) % 33;
        let level_arg = if use_self {
            0xffff_ffff
        } else {
            (next(&mut s) >> 8) % 33
        };
        cases.push(C {
            strata,
            cat,
            level_arg,
            frame_level,
            existing,
            flag: 1,
        });
    }

    let mut tally = [0u32; 3]; // [front, middle, end/append]
    for (i, c) in cases.iter().enumerate() {
        let bidx = c.strata + c.cat * 9;
        let bk = bucket_at(bidx);
        let n = c.existing.len();
        // build the bucket's node array: each entry -> an 8-byte {frame, level} node.
        let buf = emu.alloc(((n + 8) * 4) as u64);
        for (k, &lv) in c.existing.iter().enumerate() {
            let node = emu.alloc(8);
            emu.write_u32(node, frame as u32);
            emu.write_u32(node + 4, lv);
            emu.write_u32(buf + (k as u64) * 4, node as u32);
        }
        emu.write_u32(bk + 0x8, buf as u32);
        emu.write_u32(bk + 0x10, n as u32); // count
        emu.write_u32(bk + 0x14, n as u32 + 5); // capacity sentinel != count
        emu.write_u32(bk + 0x18, n as u32); // high-water
        emu.write_u32(frame + 0xc0, c.strata);
        emu.write_u32(frame + 0xc4, c.frame_level);
        emu.write_u32(top + 0x1100, 0);

        emu.reset_bump(new_node);
        emu.clear_calls();
        emu.call(
            0x764aa0,
            top as u32,
            0,
            &[frame as u32, c.cat, c.flag, c.level_arg],
        );

        // recover the binary's choice: the bucket count must have grown by 1, and the new node
        // (== new_node, the stub's first hand-out) sits at the insert position.
        let bin_count = emu.read_u32(bk + 0x10);
        assert_eq!(
            bin_count,
            n as u32 + 1,
            "input_hittest_index_insert case {i}: bucket {bidx} count {bin_count} != {}",
            n + 1
        );
        let mut bin_pos = None;
        for k in 0..=n {
            if emu.read_u32(buf + (k as u64) * 4) == new_node as u32 {
                bin_pos = Some(k);
                break;
            }
        }
        let bin_pos = bin_pos.expect("new node not found in bucket array");
        // node payload {frame, level} the binary wrote.
        let bin_node_frame = emu.read_u32(new_node);
        let bin_node_level = emu.read_u32(new_node + 4);

        let mine: BatchInsert = lib_input_hittest_index_insert(
            c.strata,
            c.cat,
            c.level_arg,
            c.frame_level,
            &c.existing,
        );
        assert_eq!(
            mine.bucket_index, bidx,
            "input_hittest_index_insert case {i}: bucket_index {} != {bidx}",
            mine.bucket_index
        );
        assert_eq!(
            bin_pos, mine.insert_pos,
            "input_hittest_index_insert case {i}: insert_pos binary {bin_pos} vs rust {} \
             (level_arg={} frame_level={} existing={:?})",
            mine.insert_pos, c.level_arg, c.frame_level, c.existing
        );
        let want_level = if c.level_arg == 0xffff_ffff {
            c.frame_level
        } else {
            c.level_arg
        };
        assert_eq!(bin_node_frame, frame as u32, "case {i}: node.frame");
        assert_eq!(bin_node_level, want_level, "case {i}: node.level");

        // flag side effect.
        if c.flag == 0 && c.cat == 2 {
            assert_eq!(
                emu.read_u32(top + 0x1100),
                1,
                "input_hittest_index_insert case {i}: +0x1100 flag not set"
            );
        }

        let leg = if mine.insert_pos == 0 {
            0
        } else if mine.insert_pos >= n {
            2
        } else {
            1
        };
        tally[leg] += 1;
    }
    eprintln!(
        "input_hittest_index_insert: {} bit-exact; [front={} middle={} end={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "input_hittest_index_insert leg {i} never hit");
    }
}
