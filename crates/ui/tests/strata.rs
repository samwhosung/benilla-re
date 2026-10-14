//! The strata and levels of `CSimpleTop` (`systems/ui/strata.md`) run from the exe:
//! `CSimpleFrame::SetFrameStrata` (`0x76a470`), `SetFrameLevel` (`0x76a4f0`), `RaiseFrame`
//! (`0x7650f0`), the update order of `OnLayerUpdate` (`0x765650`), `CompactStrata` (`0x764eb0`) and
//! `IsAncestorOf` (`0x767010`).
//!
//! A test builds a frame tree and the top's strata structure in emulator memory, runs the exe's
//! function with `CSimpleTop::HideFrame` (`0x764920`) and `ShowFrame` (`0x764780`) stubbed to log
//! the frame they get, and compares the log, the changed words and the order with the Rust
//! functions. `CompactStrata` runs with the real `SetFrameLevel`, whose changes to the levels the
//! walk then reads are part of what it computes; it is compared with `compact_strata` and with an
//! independent compaction. Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::strata::{
    compact_strata as model_level_compact, is_descendant as lib_is_descendant,
    layer_update_order as model_traverse, raise_frame as model_autoraise,
    set_frame_level as model_set_level, set_frame_strata as model_set_strata, AutoFrame, BucketOp,
    LevelFrame, StrataFrame,
};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

// addresses (the container ops the recursion cluster drives).
const REMOVE: u64 = 0x764920;

const ADD: u64 = 0x764780;

// ───────────────────────────── set_frame_strata ─────────────────────────────

/// A frame in a strata subtree spec: stratum, visible flag, child frame indices (list order).
#[derive(Clone)]
struct SSpec {
    strata: u32,
    visible: bool,
    children: Vec<usize>,
}

/// Build the subtree in emu memory from `specs` (frame 0 is the root). Each frame gets its strata
/// `+0xc0`, visible `+0xd4`, screen-root `+0xa0` (the stubbed bucket call's ecx) and its `+0x300`
/// child-list of `{next@+4, frame@+8}` nodes drawn from `nodes`.
fn build_strata(emu: &mut Emu, root: u64, frames: &[u64], nodes: &[u64], specs: &[SSpec]) {
    let mut ni = 0usize;
    for (i, s) in specs.iter().enumerate() {
        let f = frames[i];
        emu.write_u32(f + 0xc0, s.strata);
        emu.write_u32(f + 0xd4, s.visible as u32);
        emu.write_u32(f + 0xa0, root as u32);
        if s.children.is_empty() {
            emu.write_u32(f + 0x300, 0);
        } else {
            emu.write_u32(f + 0x300, nodes[ni] as u32);
            for (k, &c) in s.children.iter().enumerate() {
                let node = nodes[ni];
                ni += 1;
                emu.write_u32(node + 8, frames[c] as u32);
                let nxt = if k + 1 < s.children.len() {
                    nodes[ni] as u32
                } else {
                    0
                };
                emu.write_u32(node + 4, nxt);
            }
        }
    }
}

/// Extract the bucket op sequence from the stub log, mapping each `[esp+4]` frame pointer to its
/// index. Asserts the `flag` arg (`[esp+8]`) is 1.
fn bin_ops(emu: &Emu, frames: &[u64]) -> Vec<BucketOp> {
    emu.calls()
        .iter()
        .filter_map(|c| {
            let id = frames.iter().position(|&f| f as u32 == c.args[0])?;
            match c.addr {
                REMOVE => Some((c.args[1], BucketOp::Remove(id))),
                ADD => Some((c.args[1], BucketOp::Add(id))),
                _ => None,
            }
        })
        .map(|(flag, op)| {
            assert_eq!(flag, 1, "bucket op flag must be 1");
            op
        })
        .collect()
}

#[test]
fn set_frame_strata() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(REMOVE, 0, 2); // FUN_00764920 remove(root; frame, 1) -> ret 8
    emu.stub(ADD, 0, 2); // FUN_00764780 add(root; frame, 1)    -> ret 8
    let root = emu.alloc(0x10);
    const NF: usize = 12;
    const NN: usize = 16;
    let frames: Vec<u64> = (0..NF).map(|_| emu.alloc(0x320)).collect();
    let nodes: Vec<u64> = (0..NN).map(|_| emu.alloc(0x10)).collect();

    // (specs, new_strata) cases. frame 0 is the call target.
    let mut cases: Vec<(Vec<SSpec>, u32)> = vec![
        // unchanged strata -> no op, no recursion.
        (
            vec![SSpec {
                strata: 3,
                visible: true,
                children: vec![],
            }],
            3,
        ),
        // single visible frame, changed -> remove,add.
        (
            vec![SSpec {
                strata: 3,
                visible: true,
                children: vec![],
            }],
            5,
        ),
        // single hidden frame, changed -> NO remove/add (gated), strata still set.
        (
            vec![SSpec {
                strata: 3,
                visible: false,
                children: vec![],
            }],
            5,
        ),
        // parent + two children, all visible, all change -> 3 (remove,add) pairs, dfs order.
        (
            vec![
                SSpec {
                    strata: 3,
                    visible: true,
                    children: vec![1, 2],
                },
                SSpec {
                    strata: 3,
                    visible: true,
                    children: vec![],
                },
                SSpec {
                    strata: 3,
                    visible: true,
                    children: vec![],
                },
            ],
            7,
        ),
        // 3-level chain, mixed visibility.
        (
            vec![
                SSpec {
                    strata: 1,
                    visible: true,
                    children: vec![1],
                },
                SSpec {
                    strata: 1,
                    visible: false,
                    children: vec![2],
                },
                SSpec {
                    strata: 1,
                    visible: true,
                    children: vec![],
                },
            ],
            4,
        ),
        // a mid child already AT the target strata -> blocks its subtree (no op + no recursion below).
        (
            vec![
                SSpec {
                    strata: 0,
                    visible: true,
                    children: vec![1],
                },
                SSpec {
                    strata: 6,
                    visible: true,
                    children: vec![2],
                }, // already == 6 -> short-circuits, frame 2 untouched
                SSpec {
                    strata: 0,
                    visible: true,
                    children: vec![],
                },
            ],
            6,
        ),
    ];

    // fuzz: random acyclic trees rooted at 0 (parent[i] < i), random strata/visibility.
    let mut s = 0x76a4_7000u32;
    for _ in 0..1500 {
        let n = 1 + (next(&mut s) >> 8) as usize % 7; // 1..7 frames
        let mut children = vec![Vec::new(); n];
        for i in 1..n {
            let p = (next(&mut s) >> 8) as usize % i;
            children[p].push(i);
        }
        let specs: Vec<SSpec> = (0..n)
            .map(|i| SSpec {
                strata: (next(&mut s) >> 8) % 9,
                visible: next(&mut s) & 0x10000 != 0,
                children: children[i].clone(),
            })
            .collect();
        let new_strata = (next(&mut s) >> 8) % 9;
        cases.push((specs, new_strata));
    }

    let mut tally = [0u32; 4]; // [no-op, op-no-recurse, multilevel, hidden-skip]
    for (ci, (specs, new_strata)) in cases.iter().enumerate() {
        build_strata(&mut emu, root, &frames, &nodes, specs);
        emu.clear_calls();
        emu.call(0x76a470, frames[0] as u32, 0, &[*new_strata]);

        let got = bin_ops(&emu, &frames);
        let bin_strata: Vec<u32> = (0..specs.len())
            .map(|i| emu.read_u32(frames[i] + 0xc0))
            .collect();

        let mut mf: Vec<StrataFrame> = specs
            .iter()
            .map(|s| StrataFrame {
                strata: s.strata,
                visible: s.visible,
                children: s.children.clone(),
            })
            .collect();
        let mut ops = Vec::new();
        model_set_strata(&mut mf, 0, *new_strata, &mut ops);
        let model_strata: Vec<u32> = mf.iter().map(|f| f.strata).collect();

        assert_eq!(got, ops, "set_frame_strata case {ci}: op sequence");
        assert_eq!(
            bin_strata, model_strata,
            "set_frame_strata case {ci}: final strata"
        );

        if ops.is_empty() {
            tally[0] += 1;
        } else if specs.len() == 1 {
            tally[1] += 1;
        } else {
            tally[2] += 1;
        }
        if specs.iter().any(|s| !s.visible) {
            tally[3] += 1;
        }
    }
    eprintln!(
        "set_frame_strata: {} bit-exact; [no-op={} single-op={} multilevel={} (hidden-present={})]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2],
        tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "set_frame_strata leg {i} never hit");
    }
}

// ───────────────────────────── set_frame_level ─────────────────────────────

/// A frame in a level subtree spec.
#[derive(Clone)]
struct LSpec {
    strata: u32,
    level: u32,
    visible: bool,
    children: Vec<usize>,
}

fn build_level(emu: &mut Emu, root: u64, frames: &[u64], nodes: &[u64], specs: &[LSpec]) {
    let mut ni = 0usize;
    for (i, s) in specs.iter().enumerate() {
        let f = frames[i];
        emu.write_u32(f + 0xc0, s.strata);
        emu.write_u32(f + 0xc4, s.level);
        emu.write_u32(f + 0xd4, s.visible as u32);
        emu.write_u32(f + 0xa0, root as u32);
        if s.children.is_empty() {
            emu.write_u32(f + 0x300, 0);
        } else {
            emu.write_u32(f + 0x300, nodes[ni] as u32);
            for (k, &c) in s.children.iter().enumerate() {
                let node = nodes[ni];
                ni += 1;
                emu.write_u32(node + 8, frames[c] as u32);
                let nxt = if k + 1 < s.children.len() {
                    nodes[ni] as u32
                } else {
                    0
                };
                emu.write_u32(node + 4, nxt);
            }
        }
    }
}

#[test]
fn set_frame_level() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(REMOVE, 0, 2);
    emu.stub(ADD, 0, 2);
    let root = emu.alloc(0x10);
    const NF: usize = 12;
    const NN: usize = 16;
    let frames: Vec<u64> = (0..NF).map(|_| emu.alloc(0x320)).collect();
    let nodes: Vec<u64> = (0..NN).map(|_| emu.alloc(0x10)).collect();

    // (specs, new_level, propagate). new_level signed (negative exercises the clamp).
    let mut cases: Vec<(Vec<LSpec>, i32, bool)> = vec![
        // single visible frame, raise.
        (
            vec![LSpec {
                strata: 3,
                level: 2,
                visible: true,
                children: vec![],
            }],
            5,
            false,
        ),
        // clamp: negative -> 0.
        (
            vec![LSpec {
                strata: 3,
                level: 4,
                visible: true,
                children: vec![],
            }],
            -3,
            true,
        ),
        // unchanged -> no op.
        (
            vec![LSpec {
                strata: 3,
                level: 4,
                visible: true,
                children: vec![],
            }],
            4,
            true,
        ),
        // hidden -> no remove/add, level still set.
        (
            vec![LSpec {
                strata: 3,
                level: 1,
                visible: false,
                children: vec![],
            }],
            6,
            true,
        ),
        // propagate ON, same-strata children shift by +delta.
        (
            vec![
                LSpec {
                    strata: 2,
                    level: 1,
                    visible: true,
                    children: vec![1, 2],
                },
                LSpec {
                    strata: 2,
                    level: 3,
                    visible: true,
                    children: vec![],
                },
                LSpec {
                    strata: 2,
                    level: 5,
                    visible: true,
                    children: vec![],
                },
            ],
            4,
            true,
        ),
        // propagate ON but a CROSS-strata child -> untouched.
        (
            vec![
                LSpec {
                    strata: 2,
                    level: 1,
                    visible: true,
                    children: vec![1, 2],
                },
                LSpec {
                    strata: 2,
                    level: 3,
                    visible: true,
                    children: vec![],
                },
                LSpec {
                    strata: 5,
                    level: 3,
                    visible: true,
                    children: vec![],
                }, // cross-strata: skipped
            ],
            6,
            true,
        ),
        // propagate OFF -> children untouched even if same strata.
        (
            vec![
                LSpec {
                    strata: 2,
                    level: 1,
                    visible: true,
                    children: vec![1],
                },
                LSpec {
                    strata: 2,
                    level: 3,
                    visible: true,
                    children: vec![],
                },
            ],
            7,
            false,
        ),
        // negative delta with clamp-in-recursion: parent 5->0 (delta -5); child at 3 -> 3-5 clamps to 0.
        (
            vec![
                LSpec {
                    strata: 1,
                    level: 5,
                    visible: true,
                    children: vec![1],
                },
                LSpec {
                    strata: 1,
                    level: 3,
                    visible: true,
                    children: vec![],
                },
            ],
            0,
            true,
        ),
    ];

    // fuzz: random trees, mixed strata, random levels, signed new_level, random propagate.
    let mut s = 0x76a4_f000u32;
    for _ in 0..1500 {
        let n = 1 + (next(&mut s) >> 8) as usize % 6;
        let mut children = vec![Vec::new(); n];
        for i in 1..n {
            let p = (next(&mut s) >> 8) as usize % i;
            children[p].push(i);
        }
        let specs: Vec<LSpec> = (0..n)
            .map(|i| LSpec {
                strata: (next(&mut s) >> 8) % 3, // few strata -> same/cross mix
                level: (next(&mut s) >> 8) % 12,
                visible: next(&mut s) & 0x10000 != 0,
                children: children[i].clone(),
            })
            .collect();
        let new_level = ((next(&mut s) >> 8) % 16) as i32 - 3; // -3..12
        let propagate = next(&mut s) & 0x20000 != 0;
        cases.push((specs, new_level, propagate));
    }

    let mut tally = [0u32; 4]; // [clamp, propagate-recurse, no-op, cross-strata-skip]
    for (ci, (specs, new_level, propagate)) in cases.iter().enumerate() {
        build_level(&mut emu, root, &frames, &nodes, specs);
        emu.clear_calls();
        emu.call(
            0x76a4f0,
            frames[0] as u32,
            0,
            &[*new_level as u32, *propagate as u32],
        );

        let got = bin_ops(&emu, &frames);
        let bin_level: Vec<u32> = (0..specs.len())
            .map(|i| emu.read_u32(frames[i] + 0xc4))
            .collect();

        let mut mf: Vec<LevelFrame> = specs
            .iter()
            .map(|s| LevelFrame {
                strata: s.strata,
                level: s.level as i64,
                visible: s.visible,
                children: s.children.clone(),
            })
            .collect();
        let mut ops = Vec::new();
        model_set_level(&mut mf, 0, *new_level as i64, *propagate, &mut ops);
        let model_level: Vec<u32> = mf.iter().map(|f| f.level as u32).collect();

        assert_eq!(got, ops, "set_frame_level case {ci}: op sequence");
        assert_eq!(
            bin_level, model_level,
            "set_frame_level case {ci}: final level"
        );

        if *new_level < 0 {
            tally[0] += 1;
        }
        if *propagate && specs.len() > 1 {
            tally[1] += 1;
        }
        if ops.is_empty() {
            tally[2] += 1;
        }
        // a same-this-strata parent with a cross-strata child present
        if specs.len() > 1 && specs[1..].iter().any(|c| c.strata != specs[0].strata) {
            tally[3] += 1;
        }
    }
    eprintln!(
        "set_frame_level: {} bit-exact; [clamp={} propagate={} no-op={} cross-strata-present={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2],
        tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "set_frame_level leg {i} never hit");
    }
}

// ───────────────────────────── overlap_autoraise ─────────────────────────────

/// A frame config in an overlap scene.
#[derive(Clone)]
struct ACfg {
    toplevel: bool,
    autoraise: bool,
    parent: Option<usize>,
    level: u32,
    rect: [f32; 4],
}

/// An overlap_autoraise scene.
struct AScene {
    frames: Vec<ACfg>, // index 0 = subject (the call arg)
    level_count: u32,
    levels: Vec<Vec<usize>>, // levels[lvl] = candidate frame indices (insertion order)
    do_scan: bool,
}

#[test]
fn overlap_autoraise() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    const STRATA: u32 = 3;
    // raise side-effect sinks: level_compact (thiscall arg strata, ret 4) + the raising
    // set_frame_level (thiscall args (levelCount, 1), ret 8). Both LOG only.
    emu.stub(0x764eb0, 0, 1);
    emu.stub(0x76a4f0, 0, 2);

    // rects as [v0, v1, v2, v3] (box [v0,v2] x [v1,v3]).
    let r_base = [0.0f32, 0.0, 10.0, 10.0]; // the toplevel's rect
    let r_over = [5.0f32, 5.0, 15.0, 15.0]; // overlaps r_base
    let r_disj = [20.0f32, 20.0, 30.0, 30.0]; // disjoint from r_base

    let scenes: Vec<AScene> = vec![
        // 0: subject is toplevel + autoraise, doScan, an overlapping candidate -> raise.
        AScene {
            frames: vec![
                ACfg {
                    toplevel: true,
                    autoraise: false,
                    parent: None,
                    level: 0,
                    rect: r_base,
                },
                ACfg {
                    toplevel: false,
                    autoraise: false,
                    parent: None,
                    level: 1,
                    rect: r_over,
                },
            ],
            level_count: 2,
            levels: vec![vec![], vec![1]],
            do_scan: true,
        },
        // 1: toplevel, doScan, only a DISJOINT candidate -> no overlap -> no raise.
        AScene {
            frames: vec![
                ACfg {
                    toplevel: true,
                    autoraise: true,
                    parent: None,
                    level: 0,
                    rect: r_base,
                },
                ACfg {
                    toplevel: false,
                    autoraise: false,
                    parent: None,
                    level: 1,
                    rect: r_disj,
                },
            ],
            level_count: 2,
            levels: vec![vec![], vec![1]],
            do_scan: true,
        },
        // 2: subject NOT toplevel, has a toplevel ANCESTOR; overlapping candidate -> raise on ancestor.
        AScene {
            frames: vec![
                ACfg {
                    toplevel: false,
                    autoraise: false,
                    parent: Some(1),
                    level: 0,
                    rect: r_base,
                },
                ACfg {
                    toplevel: true,
                    autoraise: false,
                    parent: None,
                    level: 0,
                    rect: r_base,
                },
                ACfg {
                    toplevel: false,
                    autoraise: false,
                    parent: None,
                    level: 1,
                    rect: r_over,
                },
            ],
            level_count: 2,
            levels: vec![vec![], vec![2]],
            do_scan: true,
        },
        // 3: subject NOT toplevel, NO toplevel ancestor -> return 0, no raise.
        AScene {
            frames: vec![
                ACfg {
                    toplevel: false,
                    autoraise: true,
                    parent: Some(1),
                    level: 0,
                    rect: r_base,
                },
                ACfg {
                    toplevel: false,
                    autoraise: false,
                    parent: None,
                    level: 0,
                    rect: r_base,
                },
            ],
            level_count: 1,
            levels: vec![vec![]],
            do_scan: true,
        },
        // 4: doScan=0, autoraise pre-set -> raise WITHOUT scanning.
        AScene {
            frames: vec![ACfg {
                toplevel: true,
                autoraise: true,
                parent: None,
                level: 0,
                rect: r_base,
            }],
            level_count: 1,
            levels: vec![vec![]],
            do_scan: false,
        },
        // 5: doScan=0, autoraise NOT set -> no raise.
        AScene {
            frames: vec![ACfg {
                toplevel: true,
                autoraise: false,
                parent: None,
                level: 0,
                rect: r_base,
            }],
            level_count: 1,
            levels: vec![vec![]],
            do_scan: false,
        },
        // 6: candidate == the toplevel itself in its own level list -> skipped (no self-overlap).
        AScene {
            frames: vec![ACfg {
                toplevel: true,
                autoraise: false,
                parent: None,
                level: 0,
                rect: r_base,
            }],
            level_count: 1,
            levels: vec![vec![0]],
            do_scan: true,
        },
        // 7: candidate is a DESCENDANT of the toplevel (overlapping) -> excluded -> no raise.
        AScene {
            frames: vec![
                ACfg {
                    toplevel: true,
                    autoraise: false,
                    parent: None,
                    level: 0,
                    rect: r_base,
                },
                ACfg {
                    toplevel: false,
                    autoraise: false,
                    parent: Some(0),
                    level: 1,
                    rect: r_over,
                },
            ],
            level_count: 2,
            levels: vec![vec![], vec![1]],
            do_scan: true,
        },
        // 8: multi-level scan; overlap on a higher level, multiple candidates per level.
        AScene {
            frames: vec![
                ACfg {
                    toplevel: true,
                    autoraise: false,
                    parent: None,
                    level: 1,
                    rect: r_base,
                },
                ACfg {
                    toplevel: false,
                    autoraise: false,
                    parent: None,
                    level: 1,
                    rect: r_disj,
                },
                ACfg {
                    toplevel: false,
                    autoraise: false,
                    parent: None,
                    level: 2,
                    rect: r_disj,
                },
                ACfg {
                    toplevel: false,
                    autoraise: false,
                    parent: None,
                    level: 2,
                    rect: r_over,
                },
            ],
            level_count: 3,
            levels: vec![vec![], vec![1], vec![2, 3]],
            do_scan: true,
        },
        // 9: start level == levelCount (nothing to scan) -> no overlap, no raise.
        AScene {
            frames: vec![
                ACfg {
                    toplevel: true,
                    autoraise: false,
                    parent: None,
                    level: 2,
                    rect: r_base,
                },
                ACfg {
                    toplevel: false,
                    autoraise: false,
                    parent: None,
                    level: 0,
                    rect: r_over,
                },
            ],
            level_count: 2,
            levels: vec![vec![1], vec![]],
            do_scan: true,
        },
    ];

    let mut tally = [0u32; 4]; // [raise, no-raise(scan), no-toplevel, no-scan]
    for (ci, sc) in scenes.iter().enumerate() {
        let root = emu.alloc(0x1200);
        emu.write_u32(root + 0x1100, 0);
        let fr: Vec<u64> = sc.frames.iter().map(|_| emu.alloc(0x320)).collect();
        for (i, cfg) in sc.frames.iter().enumerate() {
            let f = fr[i];
            let mut b4 = 0u32;
            if cfg.toplevel {
                b4 |= 1;
            }
            if cfg.autoraise {
                b4 |= 0x10;
            }
            emu.write_u32(f + 0xb4, b4);
            emu.write_u32(f + 0x9c, cfg.parent.map(|p| fr[p] as u32).unwrap_or(0));
            emu.write_u32(f + 0xc0, STRATA);
            emu.write_u32(f + 0xc4, cfg.level);
            emu.write_u32(f + 0x60, 1); // cache valid, not dirty -> getter copies +0x64, no resolve
            for (k, v) in cfg.rect.iter().enumerate() {
                emu.write_u32(f + 0x64 + (k as u64) * 4, v.to_bits());
            }
            emu.write_u32(f + 0x310, 0);
        }
        // strata bucket + levelArray + per-level nodes + candidate lists.
        let bucket = emu.alloc(0x20);
        emu.write_u32(root + 0xcd4 + (STRATA as u64) * 4, bucket as u32);
        emu.write_u32(bucket + 8, sc.level_count);
        let larr = emu.alloc((sc.level_count as u64 + 1) * 4);
        emu.write_u32(bucket + 0x14, larr as u32);
        for lvl in 0..sc.level_count as usize {
            let node = emu.alloc(0x20);
            emu.write_u32(node + 0xc, 0x30c); // FUN_00766aa0 link base -> next at frame+0x310
            let list = sc.levels.get(lvl).cloned().unwrap_or_default();
            if list.is_empty() {
                emu.write_u32(node + 0x14, 0);
            } else {
                emu.write_u32(node + 0x14, fr[list[0]] as u32);
                for w in 0..list.len() {
                    let nxt = if w + 1 < list.len() {
                        fr[list[w + 1]] as u32
                    } else {
                        0
                    };
                    emu.write_u32(fr[list[w]] + 0x310, nxt);
                }
            }
            emu.write_u32(larr + (lvl as u64) * 4, node as u32);
        }

        emu.clear_calls();
        let ret = emu.call(0x7650f0, root as u32, 0, &[fr[0] as u32, sc.do_scan as u32]);
        let bin_lc = emu.calls().iter().any(|c| c.addr == 0x764eb0);
        let bin_sl = emu.calls().iter().find(|c| c.addr == 0x76a4f0).copied();
        let bin_1100 = emu.read_u32(root + 0x1100);

        let mut mf: Vec<AutoFrame> = sc
            .frames
            .iter()
            .map(|c| AutoFrame {
                toplevel: c.toplevel,
                autoraise: c.autoraise,
                parent: c.parent,
                strata: STRATA,
                level: c.level,
                rect: c.rect,
            })
            .collect();
        let o = model_autoraise(&mut mf, &sc.levels, sc.level_count, 0, sc.do_scan);

        assert_eq!(ret, o.ret, "overlap_autoraise scene {ci}: ret");
        assert_eq!(
            bin_lc, o.raise,
            "overlap_autoraise scene {ci}: level_compact fired"
        );
        assert_eq!(
            bin_sl.is_some(),
            o.raise,
            "overlap_autoraise scene {ci}: set_frame_level fired"
        );
        if let Some(c) = bin_sl {
            assert_eq!(
                c.args[0],
                o.set_level_arg.unwrap(),
                "scene {ci}: set_frame_level newLevel (= levelCount)"
            );
            assert_eq!(c.args[1], 1, "scene {ci}: set_frame_level propagate=1");
        }
        assert_eq!(
            bin_1100 != 0,
            o.raise,
            "overlap_autoraise scene {ci}: root+0x1100"
        );

        if o.raise {
            tally[0] += 1;
        } else if o.ret == 0 {
            tally[2] += 1;
        } else if sc.do_scan {
            tally[1] += 1;
        } else {
            tally[3] += 1;
        }
    }
    eprintln!(
        "overlap_autoraise: {} scenes bit-exact; [raise={} no-overlap={} no-toplevel={} no-scan={}]",
        scenes.len(),
        tally[0],
        tally[1],
        tally[2],
        tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "overlap_autoraise leg {i} never hit");
    }
}

// ───────────────────────────── render_traverse_order ─────────────────────────────

#[test]
fn render_traverse_order() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // FUN_00768ed0 (the pre/post flush) -> no-op. FUN_007660d0 (final) is gated on root+0x1100 != 0.
    emu.stub(0x768ed0, 0, 0);

    let root = emu.alloc(0x1300);
    emu.write_u32(root + 0xcd0, 0); // empty deferred list -> skip the first drain loop
    emu.write_u32(root + 0x1100, 0); // skip FUN_007660d0

    // populate: strata -> levels -> frames (insertion order). expected = strata→level→insertion.
    let spec: Vec<(usize, Vec<Vec<usize>>)> = vec![
        (1, vec![vec![0, 1], vec![], vec![2]]), // strata1: lvl0 {f0,f1}, lvl1 {}, lvl2 {f2}
        (3, vec![vec![3]]),                     // strata3: lvl0 {f3}
        (5, vec![vec![4, 5, 6]]),               // strata5: lvl0 {f4,f5,f6}
    ];
    let nframes = 7usize;

    // each frame: a unique primary vtable whose +0x38 is a unique Draw island (stub LOGs it).
    let mut islands = Vec::new();
    let mut fr = Vec::new();
    for _ in 0..nframes {
        let island = emu.alloc(8);
        emu.stub(island, 0, 1); // Draw(this, drawArg) -> ret 4
        let vt = emu.alloc(0x40);
        emu.write_u32(vt + 0x38, island as u32);
        let f = emu.alloc(0x320);
        emu.write_u32(f, vt as u32);
        emu.write_u32(f + 0x310, 0);
        islands.push(island);
        fr.push(f);
    }

    // an empty bucket (levelCount 0) for the unpopulated strata.
    let empty_bucket = emu.alloc(0x20);
    emu.write_u32(empty_bucket + 8, 0);
    for s in 0..9u64 {
        emu.write_u32(root + 0xcd4 + s * 4, empty_bucket as u32);
    }

    // 9 strata x levels: build a strata_levels grid for the model (default empty).
    let mut grid: Vec<Vec<Vec<usize>>> = vec![Vec::new(); 9];
    for (strata, levels) in &spec {
        let bucket = emu.alloc(0x20);
        emu.write_u32(root + 0xcd4 + (*strata as u64) * 4, bucket as u32);
        emu.write_u32(bucket + 8, levels.len() as u32);
        let larr = emu.alloc((levels.len() as u64 + 1) * 4);
        emu.write_u32(bucket + 0x14, larr as u32);
        for (lvl, list) in levels.iter().enumerate() {
            let node = emu.alloc(0x20);
            // [node]==[node+0xc] (assert passes) and [node+4]==node+4 (empty circular -> skip the
            // list-maintenance rotation). [node+0x14] = the draw head.
            emu.write_u32(node, 0x30c);
            emu.write_u32(node + 0xc, 0x30c);
            emu.write_u32(node + 4, (node + 4) as u32);
            if list.is_empty() {
                emu.write_u32(node + 0x14, 0);
            } else {
                emu.write_u32(node + 0x14, fr[list[0]] as u32);
                for w in 0..list.len() {
                    let nxt = if w + 1 < list.len() {
                        fr[list[w + 1]] as u32
                    } else {
                        0
                    };
                    emu.write_u32(fr[list[w]] + 0x310, nxt);
                }
            }
            emu.write_u32(larr + (lvl as u64) * 4, node as u32);
        }
        grid[*strata] = levels.clone();
    }

    emu.clear_calls();
    emu.call(0x765650, root as u32, 0, &[0x1234]); // drawArg passthrough

    // recover the binary's draw order from the island call sequence.
    let bin_order: Vec<usize> = emu
        .calls()
        .iter()
        .filter_map(|c| islands.iter().position(|&i| i == c.addr))
        .collect();
    // every Draw saw drawArg as its first stack arg.
    for c in emu.calls() {
        if islands.contains(&c.addr) {
            assert_eq!(
                c.args[0], 0x1234,
                "render_traverse_order: drawArg passthrough"
            );
        }
    }

    let model = model_traverse(&grid);
    assert_eq!(bin_order, model, "render_traverse_order: draw order");
    assert_eq!(
        bin_order.len(),
        nframes,
        "render_traverse_order: all frames drawn once"
    );
    eprintln!(
        "render_traverse_order: {} frames bit-exact draw order {:?} (strata->level->insertion)",
        nframes, bin_order
    );
}

// ───────────────────────────── level_compact_multigap (DEFERRED resolution) ─────────────────────────────

const LC_STRATA: u32 = 3;

/// Build the strata bucket + (empty-headed) levelArray + master chain for a chain-driven occupancy
/// case. Returns `(bucket, frames)`. Frames are INVISIBLE (`+0xd4 = 0`) so the real `set_frame_level`
/// needs no bucket container. Occupancy is purely the master chain (`+0xc0`/`+0xc4`).
fn build_lc(
    emu: &mut Emu,
    level_count: u32,
    chain: &[(u32, u32)],
    visible: &[bool],
) -> (u64, Vec<u64>) {
    let top = emu.alloc(0xd00);
    let bucket = emu.alloc(0x20);
    emu.write_u32(top + 0xcd4 + (LC_STRATA as u64) * 4, bucket as u32);
    emu.write_u32(bucket + 8, level_count);
    emu.write_u32(top + 0xcbc, 0x308); // chain link offset (next at frame+0x30c)

    if level_count > 0 {
        let larr = emu.alloc((level_count as u64) * 4);
        emu.write_u32(bucket + 0x14, larr as u32);
        for i in 0..level_count as usize {
            let node = emu.alloc(0x20);
            emu.write_u32(node + 0x14, 0); // empty array head -> occupancy via chain only
            emu.write_u32(larr + (i as u64) * 4, node as u32);
        }
    }

    let frames: Vec<u64> = chain.iter().map(|_| emu.alloc(0x320)).collect();
    emu.write_u32(top + 0xcc4, frames.first().map(|&a| a as u32).unwrap_or(0));
    for (k, &(strata, level)) in chain.iter().enumerate() {
        emu.write_u32(frames[k] + 0xc0, strata);
        emu.write_u32(frames[k] + 0xc4, level);
        // +0xd4 gates ONLY the bucket unlink/relink inside 0x76a4f0 (0x76a519 / 0x76a541);
        // the +0xc4 write at 0x76a53b is unconditional. A `true` here therefore fires the
        // stubbed remove/add, and must NOT change the level the frame ends up at.
        emu.write_u32(frames[k] + 0xd4, u32::from(visible[k]));
        emu.write_u32(frames[k] + 0xa0, top as u32); // 0x76a51d / 0x76a543 read it for ecx
        emu.write_u32(frames[k] + 0x300, 0); // no child list
        let nxt = if k + 1 < frames.len() {
            frames[k + 1] as u32
        } else {
            0
        };
        emu.write_u32(frames[k] + 0x30c, nxt);
    }
    (top, frames)
}

/// Proper (intended) gap compaction reference: occupied levels of `LC_STRATA` renumber to a dense
/// `0..n`; `count = #occupied`. (Independent of both models — a cross-check on the live result.)
fn proper_compaction(chain: &[(u32, u32)]) -> (u32, Vec<u32>) {
    let mut occ: Vec<u32> = chain
        .iter()
        .filter(|&&(s, _)| s == LC_STRATA)
        .map(|&(_, l)| l)
        .collect();
    occ.sort_unstable();
    occ.dedup();
    let finals = chain
        .iter()
        .map(|&(s, l)| {
            if s == LC_STRATA {
                occ.iter().position(|&x| x == l).unwrap() as u32
            } else {
                l
            }
        })
        .collect();
    (occ.len() as u32, finals)
}

#[test]
fn level_compact() {
    let Some(wow) = Wow::load() else {
        return;
    };

    // Single- and multi-gap chain-occupancy cases (all frames are LC_STRATA). The multi-gap ones are
    // the cases a model that does not see the levels change during the walk gets wrong.
    let cases: Vec<(u32, Vec<(u32, u32)>)> = vec![
        (
            6,
            vec![
                (LC_STRATA, 0),
                (LC_STRATA, 2),
                (LC_STRATA, 3),
                (LC_STRATA, 5),
            ],
        ), // two separate gaps (1, 4)
        (4, vec![(LC_STRATA, 0), (LC_STRATA, 2)]), // gap@1 + trailing
        (4, vec![(LC_STRATA, 1), (LC_STRATA, 3)]), // leading gap + gap
        (5, vec![(LC_STRATA, 0), (LC_STRATA, 2), (LC_STRATA, 4)]), // gaps @1,@3
        (
            6,
            vec![
                (LC_STRATA, 0),
                (LC_STRATA, 1),
                (LC_STRATA, 4),
                (LC_STRATA, 5),
            ],
        ), // single contiguous gap
        (6, vec![(LC_STRATA, 2), (LC_STRATA, 3), (LC_STRATA, 5)]), // leading 2-gap, then gap
    ];

    // LIVE callee: run 0x764eb0 with the REAL set_frame_level (0x76a4f0) — no stubs — so the resumed
    // outer pass re-reads the live-mutated +0xc4. Diff the final bucket state against both the model
    // and an independent proper-compaction reference (so the model can't be circularly self-confirming).
    let mut emu = Emu::new(&wow);
    let mut multigap_seen = 0u32;
    for (ci, (level_count, chain)) in cases.iter().enumerate() {
        let vis = vec![false; chain.len()];
        let (top, frames) = build_lc(&mut emu, *level_count, chain, &vis);
        let bucket = emu.read_u32(top + 0xcd4 + (LC_STRATA as u64) * 4) as u64;
        emu.call(0x764eb0, top as u32, 0, &[LC_STRATA]);
        let bin_count = emu.read_u32(bucket + 8);
        let bin_levels: Vec<u32> = frames.iter().map(|&f| emu.read_u32(f + 0xc4)).collect();

        let occ = vec![false; *level_count as usize];
        let m = model_level_compact(*level_count, &occ, chain, LC_STRATA);
        assert_eq!(
            bin_count, m.new_level_count,
            "level_compact case {ci}: count vs model"
        );
        assert_eq!(
            bin_levels, m.final_levels,
            "level_compact case {ci}: final +0xc4 levels vs model"
        );

        let (pc, pl) = proper_compaction(chain);
        assert_eq!(
            bin_count, pc,
            "level_compact case {ci}: count vs proper compaction"
        );
        assert_eq!(
            bin_levels, pl,
            "level_compact case {ci}: levels vs proper compaction"
        );

        // count separate gap regions to confirm the multi-gap branch is actually exercised.
        let mut occ_levels: Vec<u32> = chain.iter().map(|&(_, l)| l).collect();
        occ_levels.sort_unstable();
        occ_levels.dedup();
        let gaps = (1..occ_levels.len())
            .filter(|&k| occ_levels[k] != occ_levels[k - 1] + 1)
            .count();
        if gaps >= 2 {
            multigap_seen += 1;
        }
    }
    eprintln!(
        "level_compact: {} cases bit-exact vs binary (live set_frame_level callee + proper-compaction \
         cross-check); {multigap_seen} multi-gap cases",
        cases.len()
    );
    assert!(
        multigap_seen >= 2,
        "level_compact: must exercise the multiple-separate-gap branch (the resolved DEFERRED)"
    );
}

// ───────── level_compact: what it renumbers, and what it leaves behind ─────────

/// **Hidden frames are renumbered too.** `0x764eb0` enumerates the screen root's MASTER FRAME LIST
/// (head `root+0xcc4`, link offset `root+0xcbc`) — the list a frame joins in its constructor
/// (`0x76930d call 0x7646a0`) and leaves in its destructor (`0x7693d6 call 0x764710`) — and filters
/// it on `+0xc0 == strata` and `+0xc4 == level` ONLY (`0x764f9d`, `0x764fab`). There is no `+0xd4`
/// test anywhere in its body, and `0x76a4f0` writes `+0xc4` unconditionally (`0x76a53b`), gating
/// only the bucket unlink/relink on `+0xd4` (`0x76a519` / `0x76a541`).
///
/// So a frame that is NOT effectively visible — not linked into any of the stratum's level nodes —
/// still (a) makes its level count as OCCUPIED (`0x764f18`/`0x764f20`, the global-list arm of the
/// occupancy test) and (b) has its `+0xc4` shifted by the same delta as its visible siblings.
/// This test pins that: the same chain is run all-hidden, all-visible and mixed, and the resulting
/// levels must be IDENTICAL in all three — only the bucket remove/add op log may differ.
#[test]
fn level_compact_ignores_visibility() {
    let Some(wow) = Wow::load() else {
        return;
    };

    // (initial bucket count, chain of (strata, level))
    let cases: Vec<(u32, Vec<(u32, u32)>)> = vec![
        (
            3,
            vec![
                (LC_STRATA, 1),
                (LC_STRATA, 1),
                (LC_STRATA, 2),
                (LC_STRATA, 2),
                (LC_STRATA, 2),
            ],
        ), // the DIALOG shape: level 0 empty, everything else at 1..2
        (
            6,
            vec![
                (LC_STRATA, 0),
                (LC_STRATA, 2),
                (LC_STRATA, 3),
                (LC_STRATA, 5),
            ],
        ), // two separate gaps
        (
            5,
            vec![(LC_STRATA, 1), (LC_STRATA, 3), (LC_STRATA, 4), (2, 1)],
        ), // a foreign-strata frame in the same chain must be untouched
    ];

    let mut emu = Emu::new(&wow);
    // the bucket container ops 0x76a4f0 drives when +0xd4 is set — logged, not executed.
    emu.stub(ADD, 1, 2);
    emu.stub(REMOVE, 1, 2);

    let mut hidden_moved = 0u32;
    let mut ops_seen = 0u32;
    for (ci, (level_count, chain)) in cases.iter().enumerate() {
        let n = chain.len();
        // three visibility assignments over the identical chain.
        let vis_sets: Vec<Vec<bool>> = vec![
            vec![false; n],
            vec![true; n],
            (0..n).map(|k| k % 2 == 0).collect(),
        ];

        let mut results: Vec<Vec<u32>> = Vec::new();
        for (vi, vis) in vis_sets.iter().enumerate() {
            let (top, frames) = build_lc(&mut emu, *level_count, chain, vis);
            let bucket = emu.read_u32(top + 0xcd4 + (LC_STRATA as u64) * 4) as u64;
            emu.clear_calls();
            emu.call(0x764eb0, top as u32, 0, &[LC_STRATA]);

            let levels: Vec<u32> = frames.iter().map(|&f| emu.read_u32(f + 0xc4)).collect();
            let count = emu.read_u32(bucket + 8);

            // the model is visibility-blind by construction; the binary must be too.
            let occ = vec![false; *level_count as usize];
            let m = model_level_compact(*level_count, &occ, chain, LC_STRATA);
            assert_eq!(
                levels, m.final_levels,
                "level_compact_ignores_visibility case {ci}/{vi}: final +0xc4"
            );
            assert_eq!(
                count, m.new_level_count,
                "level_compact_ignores_visibility case {ci}/{vi}: bucket count"
            );

            // every logged bucket op belongs to a VISIBLE frame — and to no other.
            for c in emu.calls() {
                let f = u64::from(c.args[0]);
                let k = frames
                    .iter()
                    .position(|&a| a == f)
                    .expect("op on a chain frame");
                assert!(
                    vis[k],
                    "level_compact_ignores_visibility case {ci}/{vi}: bucket op on a hidden frame"
                );
                ops_seen += 1;
            }
            // a hidden frame that actually moved is the whole point of the test.
            for k in 0..n {
                if !vis[k] && chain[k].0 == LC_STRATA && levels[k] != chain[k].1 {
                    hidden_moved += 1;
                }
            }
            results.push(levels);
        }

        assert_eq!(
            results[0], results[1],
            "level_compact_ignores_visibility case {ci}: all-hidden vs all-visible levels differ"
        );
        assert_eq!(
            results[0], results[2],
            "level_compact_ignores_visibility case {ci}: all-hidden vs mixed levels differ"
        );
    }

    assert!(
        hidden_moved > 0,
        "level_compact_ignores_visibility: no hidden frame was renumbered — case set is inert"
    );
    assert!(
        ops_seen > 0,
        "level_compact_ignores_visibility: no bucket op fired — the +0xd4 arm was never exercised"
    );
    eprintln!(
        "level_compact_ignores_visibility: {} chains × 3 visibility assignments bit-exact; \
         {hidden_moved} hidden-frame renumbers, {ops_seen} visible-frame bucket ops",
        cases.len()
    );
}

/// **The compaction does NOT always pack into `[0, count)`.** When the shift's delta is wide enough
/// to swallow an interior hole, the cursor resumes at the saved pre-gap index (`0x764fd3
/// mov edi,[ebp-8]`) — which sits at or past the shrunk count (`0x764fd8 sub ebx,esi`) — and the
/// walk ends with the hole still there. `bucket+0x8` is therefore a HIGH-WATER BOUND
/// (`initialCount − Σdelta`, or the first trailing-free index at `0x764f62`), strictly greater than
/// every occupied level but NOT equal to the number of occupied levels.
#[test]
fn level_compact_leaves_interior_gaps() {
    let Some(wow) = Wow::load() else {
        return;
    };

    /// `(initial bucket count, chain of (strata, level), expected final count, expected final levels)`.
    type NonPackingCase = (u32, Vec<(u32, u32)>, u32, Vec<u32>);

    let cases: Vec<NonPackingCase> = vec![
        // gap [1,3] is 3 wide; the shift moves {4,6} -> {1,3} and the cursor resumes at 4 == the
        // new count, so level 2 stays empty and the count stays 4 for 3 occupied levels.
        (
            7,
            vec![(LC_STRATA, 0), (LC_STRATA, 4), (LC_STRATA, 6)],
            4,
            vec![0, 1, 3],
        ),
        // same shape one level deeper, with a survivor above the hole.
        (
            8,
            vec![
                (LC_STRATA, 0),
                (LC_STRATA, 4),
                (LC_STRATA, 6),
                (LC_STRATA, 7),
            ],
            5,
            vec![0, 1, 3, 4],
        ),
    ];

    let mut emu = Emu::new(&wow);
    for (ci, (level_count, chain, want_count, want_levels)) in cases.iter().enumerate() {
        let vis = vec![false; chain.len()];
        let (top, frames) = build_lc(&mut emu, *level_count, chain, &vis);
        let bucket = emu.read_u32(top + 0xcd4 + (LC_STRATA as u64) * 4) as u64;
        emu.call(0x764eb0, top as u32, 0, &[LC_STRATA]);

        let count = emu.read_u32(bucket + 8);
        let levels: Vec<u32> = frames.iter().map(|&f| emu.read_u32(f + 0xc4)).collect();

        assert_eq!(count, *want_count, "leaves_interior_gaps case {ci}: count");
        assert_eq!(
            levels, *want_levels,
            "leaves_interior_gaps case {ci}: levels"
        );

        let m = model_level_compact(
            *level_count,
            &vec![false; *level_count as usize],
            chain,
            LC_STRATA,
        );
        assert_eq!(
            count, m.new_level_count,
            "leaves_interior_gaps case {ci}: count vs model"
        );
        assert_eq!(
            levels, m.final_levels,
            "leaves_interior_gaps case {ci}: levels vs model"
        );

        // the case earns its keep only while it still contradicts a proper packing.
        let (pc, pl) = proper_compaction(chain);
        assert!(
            count != pc || levels != pl,
            "leaves_interior_gaps case {ci}: now packs properly — the case no longer tests anything"
        );
        // but the invariant the Raise depends on must still hold.
        let max_occupied = levels
            .iter()
            .zip(chain.iter())
            .filter(|(_, c)| c.0 == LC_STRATA)
            .map(|(l, _)| *l)
            .max()
            .unwrap();
        assert!(
            count > max_occupied,
            "leaves_interior_gaps case {ci}: count {count} must exceed every occupied level \
             ({max_occupied}) — the Raise writes it as the next free level"
        );
    }
    eprintln!(
        "level_compact_leaves_interior_gaps: {} non-packing cases bit-exact vs binary",
        cases.len()
    );
}

// ───────────────────────────── is_descendant ─────────────────────────────

#[test]
fn is_descendant() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // a reusable pool of frames; per case we relink the +0x9c chain and pick a candidate.
    const POOL: usize = 10;
    let frames: Vec<u64> = (0..POOL).map(|_| emu.alloc(0xa0)).collect();

    struct C {
        len: usize, // chain length: this=frames[0] .. frames[len-1] (frames[len-1].parent=0)
        cand: u32,  // candidate pointer (a frame addr or 0)
    }
    let mut cases: Vec<C> = vec![
        C {
            len: 1,
            cand: frames[0] as u32,
        }, // this.parent==0 -> false
        C { len: 1, cand: 0 }, // this.parent==0 -> false
        C {
            len: 4,
            cand: frames[1] as u32,
        }, // direct parent -> true
        C {
            len: 4,
            cand: frames[3] as u32,
        }, // top ancestor -> true
        C {
            len: 4,
            cand: frames[2] as u32,
        }, // mid ancestor -> true
        C {
            len: 4,
            cand: frames[0] as u32,
        }, // self -> false
        C {
            len: 4,
            cand: frames[9] as u32,
        }, // unrelated -> false
        C { len: 4, cand: 0 }, // candidate 0 -> false
        C {
            len: 8,
            cand: frames[7] as u32,
        }, // deep chain, top -> true
        C {
            len: 8,
            cand: frames[8] as u32,
        }, // deep chain, just past top -> false
    ];
    let mut s = 0x7670_1000u32;
    for _ in 0..2000 {
        let len = 1 + (next(&mut s) >> 8) as usize % 8; // 1..8
        let pick = (next(&mut s) >> 8) % 11;
        let cand = if pick == 10 {
            0
        } else {
            frames[pick as usize] as u32
        };
        cases.push(C { len, cand });
    }

    let mut tally = [0u32; 2]; // [false, true]
    for (i, c) in cases.iter().enumerate() {
        // build the chain frames[0]->frames[1]->...->frames[len-1]->null.
        for k in 0..c.len {
            let parent = if k + 1 < c.len {
                frames[k + 1] as u32
            } else {
                0
            };
            emu.write_u32(frames[k] + 0x9c, parent);
        }
        let ancestors: Vec<u32> = (1..c.len).map(|k| frames[k] as u32).collect();

        let bin = emu.call(0x767010, frames[0] as u32, 0, &[c.cand]) != 0;
        let mine = lib_is_descendant(&ancestors, c.cand);
        assert_eq!(
            bin, mine,
            "is_descendant case {i}: binary {bin} vs rust {mine} (len={} cand={:#x})",
            c.len, c.cand
        );
        tally[mine as usize] += 1;
    }
    eprintln!(
        "is_descendant: {} bit-exact; [false={} true={}]",
        cases.len(),
        tally[0],
        tally[1]
    );
    assert!(
        tally[0] > 0 && tally[1] > 0,
        "is_descendant: both legs must hit"
    );
}
