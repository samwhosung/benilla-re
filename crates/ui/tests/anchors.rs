//! The leaves of the anchor and rectangle resolution (`systems/ui/anchors.md`) run from the exe:
//! the combiners `CombineEdge` (`0x767440`) and `CombineCenter` (`0x7672d0`), the relative-point
//! coordinates `RelativePointGetX` (`0x7a2f90`) and `RelativePointGetY` (`0x7a3070`), the end of
//! `CLayoutFrame::CalculateRect` (`0x767a20`), `CLayoutFrame::OnFrameResize` (`0x768d20`) and
//! `CSimpleFrame::OnFrameSizeChanged` (`0x76b580`).
//!
//! The combiners and the coordinates return in `st0`, which their callers consume unrounded, so
//! they are compared as `f64` through `Emu::call_f64`. The `+Inf` sentinel lives in the cell
//! `0xcf550c`, set at run time, so the test writes it before each call. The arguments are drawn
//! from finite values, the sentinel, 0.0 and NaN. A NaN payload is not part of the specification
//! (the x87's internal quiet NaN is not Rust's), so two NaNs compare equal, but a NaN on one side
//! only still fails. Every draw comes off the high bits of an LCG, and each leg is tallied and
//! required to be hit.
#![allow(clippy::float_cmp, clippy::needless_range_loop)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::anchors::{
    assemble_rect as assemble, combine_center, combine_edge, on_frame_resize as recomp,
    on_frame_size_changed as apply_rect, relative_point_get_x as resolve_x,
    relative_point_get_y as resolve_y,
};

/// The UNSET sentinel slot (`0xcf550c`), set from `[0x81d56c] = 0x7f800000` at runtime.
const UNSET_SLOT: u64 = 0xcf550c;
const INF_BITS: u32 = 0x7f80_0000;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// Draw one argument from {finite, +Inf-sentinel, 0.0, NaN} (sentinel ~25%).
fn val(s: &mut u32) -> f32 {
    match (next(s) >> 8) % 8 {
        0 | 1 => f32::INFINITY, // the UNSET sentinel (~25%)
        2 => 0.0,
        3 => f32::NAN,
        _ => {
            let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
            m * 240.0 - 120.0
        }
    }
}

/// Span tends to 0.0 more often (the no-explicit-size case).
fn span(s: &mut u32) -> f32 {
    match (next(s) >> 8) % 6 {
        0 | 1 => 0.0,
        2 => f32::INFINITY,
        3 => f32::NAN,
        _ => {
            let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
            m * 120.0 - 60.0
        }
    }
}

/// Compare a transcribed `f64` against the binary's `st0`: bit-exact, except both-NaN (routing
/// matched; payload unspeced) passes.
fn assert_bits(mine: f64, bin: f64, ctx: &str) {
    if mine.is_nan() && bin.is_nan() {
        return;
    }
    assert_eq!(
        mine.to_bits(),
        bin.to_bits(),
        "{ctx}: rust {:#018x} ({mine}) vs binary {:#018x} ({bin})",
        mine.to_bits(),
        bin.to_bits()
    );
}

#[test]
fn layout_combine_edge() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(UNSET_SLOT, INF_BITS);

    let inf = f32::INFINITY;
    // Explicit probes — one per leg + sentinel-equality + NaN routing that lands on a clean return.
    let mut cases: Vec<(f32, f32, f32)> = vec![
        (10.0, 50.0, 7.0),     // leg1: opp+span (opp finite, span!=0)
        (10.0, inf, 7.0),      // leg2: span*0.5+center (opp unset, span!=0)
        (10.0, 50.0, 0.0),     // leg3: 2center-opp (span==0, opp finite)
        (inf, inf, 0.0),       // leg4: all unset -> +Inf
        (inf, 50.0, 0.0),      // center unset, span 0 -> +Inf
        (10.0, inf, 0.0),      // opp unset, span 0, center finite, opp unset -> +Inf
        (10.0, 50.0, -0.0),    // -0.0 span == 0.0 -> leg3 (IEEE -0==0)
        (f32::NAN, inf, 0.0),  // NaN center routes != +Inf, span 0, opp unset -> +Inf (clean)
        (10.0, f32::NAN, 5.0), // NaN opp: opp!=Inf true, span!=0 -> opp+span = NaN (routing)
        (inf, 50.0, f32::NAN), // NaN span: span!=0 true, opp finite -> opp+span
    ];
    let mut s = 0x5a17_c0deu32;
    for _ in 0..4000 {
        cases.push((val(&mut s), val(&mut s), span(&mut s)));
    }

    let mut tally = [0u32; 4];
    let mut checked = 0u32;
    for (i, &(c, o, sp)) in cases.iter().enumerate() {
        let bin = emu.call_f64(0x767440, 0, 0, &[c.to_bits(), o.to_bits(), sp.to_bits()]);
        let mine = combine_edge(c, o, sp);
        assert_bits(mine, bin, &format!("combine_edge case {i} ({c},{o},{sp})"));
        // classify by the model's branch
        let leg = if o != f32::INFINITY && sp != 0.0 {
            0
        } else if c != f32::INFINITY && sp != 0.0 {
            1
        } else if c != f32::INFINITY && o != f32::INFINITY {
            2
        } else {
            3
        };
        tally[leg] += 1;
        checked += 1;
    }
    eprintln!(
        "combine_edge: {checked} bit-exact; [opp+span={} span/2+c={} 2c-opp={} +Inf={}]",
        tally[0], tally[1], tally[2], tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "combine_edge leg {i} never hit");
    }
}

#[test]
fn layout_combine_center() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(UNSET_SLOT, INF_BITS);

    let inf = f32::INFINITY;
    let mut cases: Vec<(f32, f32, f32)> = vec![
        (10.0, 50.0, 7.0),    // leg1: (lo+hi)*0.5 (both finite)
        (10.0, inf, 7.0),     // leg2: span*0.5+lo (hi unset, span!=0)
        (inf, 50.0, 7.0),     // leg3: hi-span*0.5 (lo unset, hi finite, span!=0)
        (inf, inf, 7.0),      // leg4: +Inf (both unset)
        (10.0, inf, 0.0),     // lo finite, hi unset, span 0 -> +Inf
        (inf, 50.0, 0.0),     // lo unset, hi finite, span 0 -> +Inf
        (10.0, 50.0, -0.0),   // both finite -> leg1 (span irrelevant)
        (f32::NAN, inf, 7.0), // NaN lo != +Inf -> span!=0 -> span*0.5+lo = NaN (routing)
        (inf, f32::NAN, 7.0), // lo unset, NaN hi != +Inf, span!=0 -> hi-span*0.5 = NaN (routing)
        (inf, inf, f32::NAN), // both unset -> +Inf
    ];
    let mut s = 0x0bad_f00du32;
    for _ in 0..4000 {
        cases.push((val(&mut s), val(&mut s), span(&mut s)));
    }

    let mut tally = [0u32; 4];
    let mut checked = 0u32;
    for (i, &(lo, hi, sp)) in cases.iter().enumerate() {
        let bin = emu.call_f64(0x7672d0, 0, 0, &[lo.to_bits(), hi.to_bits(), sp.to_bits()]);
        let mine = combine_center(lo, hi, sp);
        assert_bits(
            mine,
            bin,
            &format!("combine_center case {i} ({lo},{hi},{sp})"),
        );
        let leg = if lo != f32::INFINITY && hi != f32::INFINITY {
            0
        } else if lo != f32::INFINITY && sp != 0.0 {
            1
        } else if hi != f32::INFINITY && sp != 0.0 {
            2
        } else {
            3
        };
        tally[leg] += 1;
        checked += 1;
    }
    eprintln!(
        "combine_center: {checked} bit-exact; [(lo+hi)/2={} span/2+lo={} hi-span/2={} +Inf={}]",
        tally[0], tally[1], tally[2], tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "combine_center leg {i} never hit");
    }
}

/// Build the shared CAnchor + relativeTo frame + fake vtable for the ResolveX/Y diffs. The
/// predicate (relativeTo's geom-vtable[+0x24], normalize-to-local-origin) is a `mov eax,[cell];ret`
/// stub so each case controls it via `pred_cell`. relativeTo's geom-flags `+0x3c` carry bit0 =
/// rect-cache-valid (GetRect 0x768320's gate) with bit1 (dirty) kept clear so the on-demand
/// force-resolve (0x768060, which would walk the deferred-layout queue) is never entered.
struct AnchorRig {
    anchor: u64,    // CAnchor: +4 xOff, +8 yOff, +0xc relativeTo, +0x10 relativePoint
    rt: u64,        // relativeTo frame: +0 vtable, +0x3c flags, +0x40.. cachedRect
    pred_cell: u64, // 0/1 -> the normalize predicate's return
}
fn anchor_rig(emu: &mut Emu) -> AnchorRig {
    emu.write_u32(UNSET_SLOT, INF_BITS);
    let pred_cell = emu.alloc(4);
    let pred_stub = emu.alloc(8);
    let mut stub = vec![0xa1u8]; // mov eax, ds:[pred_cell]
    stub.extend_from_slice(&(pred_cell as u32).to_le_bytes());
    stub.push(0xc3); // ret (no args)
    emu.write(pred_stub, &stub);
    let vtable = emu.alloc(0x40);
    emu.write_u32(vtable + 0x24, pred_stub as u32);
    let rt = emu.alloc(0x80);
    emu.write_u32(rt, vtable as u32);
    let anchor = emu.alloc(0x20);
    emu.write_u32(anchor + 0xc, rt as u32);
    AnchorRig {
        anchor,
        rt,
        pred_cell,
    }
}

#[test]
fn anchor_resolve_x() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rig = anchor_rig(&mut emu);

    // explicit probes: each column × mirror, GetRect-fail, out-of-range point.
    struct C {
        scale: f32,
        x_off: f32,
        pt: u32,
        valid: bool,
        rect: [f32; 4],
        mirror: bool,
    }
    let r0 = [3.0f32, 100.0, 480.0, 260.0]; // [bottom, left, top, right]
    let mut cases: Vec<C> = vec![
        C {
            scale: 1.0,
            x_off: 5.0,
            pt: 0,
            valid: true,
            rect: r0,
            mirror: false,
        }, // left
        C {
            scale: 1.0,
            x_off: 5.0,
            pt: 1,
            valid: true,
            rect: r0,
            mirror: false,
        }, // center
        C {
            scale: 1.0,
            x_off: 5.0,
            pt: 2,
            valid: true,
            rect: r0,
            mirror: false,
        }, // right
        C {
            scale: 2.0,
            x_off: -7.5,
            pt: 4,
            valid: true,
            rect: r0,
            mirror: true,
        }, // center+mirror
        C {
            scale: 1.5,
            x_off: 9.0,
            pt: 6,
            valid: true,
            rect: r0,
            mirror: true,
        }, // left+mirror
        C {
            scale: 1.0,
            x_off: 0.0,
            pt: 0,
            valid: false,
            rect: r0,
            mirror: false,
        }, // GetRect fail
        C {
            scale: 1.0,
            x_off: 1.0,
            pt: 9,
            valid: true,
            rect: r0,
            mirror: false,
        }, // pt>8
    ];
    let mut s = 0x7a2f_0090u32;
    for _ in 0..3000 {
        let pt = (next(&mut s) >> 8) % 11; // 0..10 (10 exercises the >8 reject)
        let rect = [
            (next(&mut s) >> 8) as f32 / 5000.0,
            (next(&mut s) >> 8) as f32 / 4000.0,
            (next(&mut s) >> 8) as f32 / 5000.0,
            (next(&mut s) >> 8) as f32 / 4000.0,
        ];
        cases.push(C {
            scale: (next(&mut s) >> 8) as f32 / (1u32 << 22) as f32,
            x_off: ((next(&mut s) >> 8) as f32 / (1u32 << 20) as f32) - 200.0,
            pt,
            valid: next(&mut s) & 0x700 != 0, // mostly valid
            rect,
            mirror: next(&mut s) & 0x40 != 0,
        });
    }

    let mut tally = [0u32; 6]; // invalid, oob, left, center, right, mirror
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(rig.anchor + 4, c.x_off.to_bits());
        emu.write_u32(rig.anchor + 0x10, c.pt);
        emu.write_u32(rig.rt + 0x3c, if c.valid { 0x1 } else { 0x0 });
        for (k, v) in c.rect.iter().enumerate() {
            emu.write_u32(rig.rt + 0x40 + (k as u64) * 4, v.to_bits());
        }
        emu.write_u32(rig.pred_cell, c.mirror as u32);
        let bin = emu.call_f64(0x7a2f90, rig.anchor as u32, 0, &[c.scale.to_bits()]);
        let rect = c.valid.then_some(c.rect);
        let mine = resolve_x(c.scale, c.x_off, c.pt, rect, c.mirror);
        assert_bits(mine, bin, &format!("resolve_x case {i} pt={}", c.pt));
        if !c.valid {
            tally[0] += 1;
        } else if c.pt > 8 {
            tally[1] += 1;
        } else if c.mirror {
            tally[5] += 1;
        } else {
            tally[2 + X_COL_T[c.pt as usize] as usize] += 1;
        }
    }
    eprintln!(
        "resolve_x: {} bit-exact; [invalid={} oob={} left={} center={} right={} mirror={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2],
        tally[3],
        tally[4],
        tally[5]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "resolve_x leg {i} never hit");
    }
}

#[test]
fn anchor_resolve_y() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rig = anchor_rig(&mut emu);

    struct C {
        scale: f32,
        y_off: f32,
        pt: u32,
        valid: bool,
        rect: [f32; 4],
        mirror: bool,
    }
    let r0 = [3.0f32, 100.0, 480.0, 260.0];
    let mut cases: Vec<C> = vec![
        C {
            scale: 1.0,
            y_off: 5.0,
            pt: 0,
            valid: true,
            rect: r0,
            mirror: false,
        }, // top
        C {
            scale: 1.0,
            y_off: 5.0,
            pt: 3,
            valid: true,
            rect: r0,
            mirror: false,
        }, // center
        C {
            scale: 1.0,
            y_off: 5.0,
            pt: 6,
            valid: true,
            rect: r0,
            mirror: false,
        }, // bottom
        C {
            scale: 2.0,
            y_off: -7.5,
            pt: 4,
            valid: true,
            rect: r0,
            mirror: true,
        }, // center+mirror
        C {
            scale: 1.5,
            y_off: 9.0,
            pt: 0,
            valid: true,
            rect: r0,
            mirror: true,
        }, // top+mirror
        C {
            scale: 1.0,
            y_off: 0.0,
            pt: 0,
            valid: false,
            rect: r0,
            mirror: false,
        }, // GetRect fail
        C {
            scale: 1.0,
            y_off: 1.0,
            pt: 9,
            valid: true,
            rect: r0,
            mirror: false,
        }, // pt>8
    ];
    let mut s = 0x7a30_0070u32;
    for _ in 0..3000 {
        let pt = (next(&mut s) >> 8) % 11;
        let rect = [
            (next(&mut s) >> 8) as f32 / 4000.0,
            (next(&mut s) >> 8) as f32 / 5000.0,
            (next(&mut s) >> 8) as f32 / 4000.0,
            (next(&mut s) >> 8) as f32 / 5000.0,
        ];
        cases.push(C {
            scale: (next(&mut s) >> 8) as f32 / (1u32 << 22) as f32,
            y_off: ((next(&mut s) >> 8) as f32 / (1u32 << 20) as f32) - 200.0,
            pt,
            valid: next(&mut s) & 0x700 != 0,
            rect,
            mirror: next(&mut s) & 0x40 != 0,
        });
    }

    let mut tally = [0u32; 6]; // invalid, oob, top, center, bottom, mirror
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(rig.anchor + 8, c.y_off.to_bits());
        emu.write_u32(rig.anchor + 0x10, c.pt);
        emu.write_u32(rig.rt + 0x3c, if c.valid { 0x1 } else { 0x0 });
        for (k, v) in c.rect.iter().enumerate() {
            emu.write_u32(rig.rt + 0x40 + (k as u64) * 4, v.to_bits());
        }
        emu.write_u32(rig.pred_cell, c.mirror as u32);
        let bin = emu.call_f64(0x7a3070, rig.anchor as u32, 0, &[c.scale.to_bits()]);
        let rect = c.valid.then_some(c.rect);
        let mine = resolve_y(c.scale, c.y_off, c.pt, rect, c.mirror);
        assert_bits(mine, bin, &format!("resolve_y case {i} pt={}", c.pt));
        if !c.valid {
            tally[0] += 1;
        } else if c.pt > 8 {
            tally[1] += 1;
        } else if c.mirror {
            tally[5] += 1;
        } else {
            tally[2 + Y_ROW_T[c.pt as usize] as usize] += 1;
        }
    }
    eprintln!(
        "resolve_y: {} bit-exact; [invalid={} oob={} top={} center={} bottom={} mirror={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2],
        tally[3],
        tally[4],
        tally[5]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "resolve_y leg {i} never hit");
    }
}

// point-id → column/row, mirrored from the lib's tables for the coverage tally.
const X_COL_T: [u8; 9] = [0, 1, 2, 0, 1, 2, 0, 1, 2];
const Y_ROW_T: [u8; 9] = [0, 0, 0, 1, 1, 1, 2, 2, 2];

/// Overwrite a callee with `fld dword ptr [cell]; ret N` (a per-case `st0` f32 oracle; `ret_n` =
/// bytes the callee cleans, 0 ⇒ bare `ret`).
fn patch_fld(emu: &mut Emu, addr: u64, cell: u64, ret_n: u16) {
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    if ret_n == 0 {
        code.push(0xc3);
    } else {
        code.push(0xc2);
        code.extend_from_slice(&ret_n.to_le_bytes());
    }
    emu.write(addr, &code);
}

#[test]
fn layout_assemble_rect() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(UNSET_SLOT, INF_BITS);
    // the four edge resolvers + the two screen-extent fns become per-case `fld [cell]; ret` oracles.
    let cl = emu.alloc(4); // LEFT  (0x7673d0)
    let cb = emu.alloc(4); // BOTTOM(0x7675b0)
    let cr = emu.alloc(4); // RIGHT (0x767540)
    let ct = emu.alloc(4); // TOP   (0x7674d0)
    let cex = emu.alloc(4); // extent X (0x41ae60)
    let cey = emu.alloc(4); // extent Y (0x41ae70)
    patch_fld(&mut emu, 0x7673d0, cl, 0);
    patch_fld(&mut emu, 0x7675b0, cb, 0);
    patch_fld(&mut emu, 0x767540, cr, 0);
    patch_fld(&mut emu, 0x7674d0, ct, 0);
    patch_fld(&mut emu, 0x41ae60, cex, 4);
    patch_fld(&mut emu, 0x41ae70, cey, 4);
    let geom = emu.alloc(0x80);
    let out = emu.alloc(0x10);

    struct C {
        edges: [f32; 4], // [bottom, left, top, right]
        clamp: bool,
        ex: f32,
        ey: f32,
    }
    let inf = f32::INFINITY;
    let mut cases: Vec<C> = vec![
        // no clamp: rect = raw edges.
        C {
            edges: [3.0, 100.0, 480.0, 260.0],
            clamp: false,
            ex: 0.8,
            ey: 0.6,
        },
        // clamp, fully in-bounds: unchanged.
        C {
            edges: [0.1, 0.1, 0.5, 0.5],
            clamp: true,
            ex: 0.8,
            ey: 0.6,
        },
        // clamp, left<0: shift X.
        C {
            edges: [0.1, -0.2, 0.5, 0.3],
            clamp: true,
            ex: 0.8,
            ey: 0.6,
        },
        // clamp, bottom<0: shift Y.
        C {
            edges: [-0.2, 0.1, 0.3, 0.5],
            clamp: true,
            ex: 0.8,
            ey: 0.6,
        },
        // clamp, right>extent: clamp X high.
        C {
            edges: [0.1, 0.1, 0.5, 1.5],
            clamp: true,
            ex: 0.8,
            ey: 0.6,
        },
        // clamp, top>extent: clamp Y high.
        C {
            edges: [0.1, 0.1, 1.2, 0.5],
            clamp: true,
            ex: 0.8,
            ey: 0.6,
        },
        // a resolver fails -> ok=false.
        C {
            edges: [0.1, inf, 0.5, 0.5],
            clamp: false,
            ex: 0.8,
            ey: 0.6,
        },
        C {
            edges: [0.1, 0.1, inf, 0.5],
            clamp: true,
            ex: 0.8,
            ey: 0.6,
        },
    ];
    let mut s = 0x767a_0020u32;
    for _ in 0..3000 {
        let f = |s: &mut u32| (next(s) >> 8) as f32 / (1u32 << 24) as f32 * 2.4 - 0.8;
        let fail = next(&mut s) & 0x700 == 0;
        let mut edges = [f(&mut s), f(&mut s), f(&mut s), f(&mut s)];
        if fail {
            edges[(next(&mut s) >> 8) as usize % 4] = inf;
        }
        cases.push(C {
            edges,
            clamp: next(&mut s) & 0x40 != 0,
            ex: (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 1.2,
            ey: (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 1.0,
        });
    }

    let mut tally = [0u32; 3]; // [fail, ok_no_clamp, ok_clamp]
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(cb, c.edges[0].to_bits());
        emu.write_u32(cl, c.edges[1].to_bits());
        emu.write_u32(ct, c.edges[2].to_bits());
        emu.write_u32(cr, c.edges[3].to_bits());
        emu.write_u32(cex, c.ex.to_bits());
        emu.write_u32(cey, c.ey.to_bits());
        emu.write_u32(geom + 0x3c, if c.clamp { 0x10 } else { 0 });
        let eax = emu.call(0x767a20, geom as u32, 0, &[out as u32]);
        let bin_rect: [f32; 4] = core::array::from_fn(|k| emu.read_f32(out + (k as u64) * 4));

        let o = assemble(c.edges, c.clamp, c.ex, c.ey);
        assert_eq!(
            eax != 0,
            o.ok,
            "assemble case {i}: ok {} vs {}",
            eax != 0,
            o.ok
        );
        if o.ok {
            for k in 0..4 {
                assert_eq!(
                    bin_rect[k].to_bits(),
                    o.rect[k].to_bits(),
                    "assemble case {i}: rect[{k}] {:#010x} vs {:#010x}",
                    bin_rect[k].to_bits(),
                    o.rect[k].to_bits()
                );
            }
            tally[1 + c.clamp as usize] += 1;
        } else {
            tally[0] += 1;
        }
    }
    eprintln!(
        "assemble_rect: {} bit-exact; [fail={} ok_no_clamp={} ok_clamp={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "assemble leg {i} never hit");
    }
}

#[test]
fn layout_recompute() {
    let Some(wow) = Wow::load() else {
        return;
    };

    struct C {
        flags_in: u32,
        cached: [f32; 4], // [bottom, left, top, right]
        new_rect: [f32; 4],
    }
    let r0 = [3.0f32, 100.0, 480.0, 260.0];
    let mut cases: Vec<C> = vec![
        // valid + unchanged (new == cached) -> no apply, eax 1.
        C {
            flags_in: 0x1,
            cached: r0,
            new_rect: r0,
        },
        // valid + changed -> store + apply.
        C {
            flags_in: 0x1,
            cached: r0,
            new_rect: [3.0, 100.0, 480.0, 999.0],
        },
        // not valid (first resolve, bit0 = 0) -> apply regardless.
        C {
            flags_in: 0x0,
            cached: r0,
            new_rect: r0,
        },
        // valid + sub-ε change -> unchanged.
        C {
            flags_in: 0x1,
            cached: r0,
            new_rect: [3.0, 100.0, 480.0, 260.0 + 1e-6],
        },
        // dirty/failed bits set incoming -> cleared on success.
        C {
            flags_in: 0xF,
            cached: r0,
            new_rect: [10.0, 100.0, 480.0, 260.0],
        },
    ];
    let mut s = 0x768d_0020u32;
    for _ in 0..2000 {
        let f = |s: &mut u32| (next(s) >> 8) as f32 / 4000.0;
        let cached = [f(&mut s), f(&mut s), f(&mut s), f(&mut s)];
        // new_rect: usually a perturbation of cached (often sub-ε), sometimes equal/large.
        let new_rect = match next(&mut s) % 4 {
            0 => cached,
            1 => core::array::from_fn(|k| cached[k] + 1e-7), // sub-ε
            _ => core::array::from_fn(|k| {
                cached[k] + ((next(&mut s) >> 8) as f32 / (1u32 << 26) as f32 - 0.5) * 2.0
            }),
        };
        cases.push(C {
            flags_in: next(&mut s) & 0xF,
            cached,
            new_rect,
        });
    }

    // Phase A: assemble succeeds (stub_write_through fills the out-param rect, returns 1).
    let mut emu = Emu::new(&wow);
    let apply_stub = emu.alloc(8);
    emu.stub(apply_stub, 0, 1); // ApplyRect(&oldRect): thiscall, 1 stack arg
    let vtable = emu.alloc(0x10);
    emu.write_u32(vtable, apply_stub as u32); // geom vtable slot 0 = ApplyRect
    let payload = emu.alloc(0x10);
    emu.stub_write_through(0x767a20, 0, payload, 16, 1, 1); // assemble: fill &newRect, ret 1
    let geom = emu.alloc(0x80);
    emu.write_u32(geom, vtable as u32);

    let mut tally = [0u32; 3]; // [unchanged, changed/first-apply, _]
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(geom + 0x3c, c.flags_in);
        for (k, v) in c.cached.iter().enumerate() {
            emu.write_u32(geom + 0x40 + (k as u64) * 4, v.to_bits());
        }
        for (k, v) in c.new_rect.iter().enumerate() {
            emu.write_u32(payload + (k as u64) * 4, v.to_bits());
        }
        emu.clear_calls();
        let eax = emu.call(0x768d20, geom as u32, 0, &[]);
        let bin_flags = emu.read_u32(geom + 0x3c);
        let bin_cached: [f32; 4] =
            core::array::from_fn(|k| emu.read_f32(geom + 0x40 + (k as u64) * 4));
        let bin_applied = emu.calls().iter().any(|cc| cc.addr == apply_stub);

        let o = recomp(c.flags_in, c.cached, true, c.new_rect);
        assert_eq!(eax, o.eax, "recompute case {i}: eax {eax} vs {}", o.eax);
        assert_eq!(
            bin_flags, o.flags,
            "recompute case {i}: flags {bin_flags:#x} vs {:#x}",
            o.flags
        );
        assert_eq!(
            bin_applied, o.applied,
            "recompute case {i}: applied {bin_applied} vs {}",
            o.applied
        );
        for k in 0..4 {
            assert_eq!(
                bin_cached[k].to_bits(),
                o.cached[k].to_bits(),
                "recompute case {i}: cached[{k}] {:#010x} vs {:#010x}",
                bin_cached[k].to_bits(),
                o.cached[k].to_bits()
            );
        }
        tally[o.applied as usize] += 1;
    }

    // Phase B: assemble fails (ret 0) -> resolution-failed flags, no apply, cache untouched.
    let mut emu2 = Emu::new(&wow);
    let apply_stub2 = emu2.alloc(8);
    emu2.stub(apply_stub2, 0, 1);
    let vtable2 = emu2.alloc(0x10);
    emu2.write_u32(vtable2, apply_stub2 as u32);
    emu2.stub(0x767a20, 0, 1); // assemble fails (eax = 0)
    let geom2 = emu2.alloc(0x80);
    emu2.write_u32(geom2, vtable2 as u32);
    let mut fail_checked = 0u32;
    for (i, c) in cases.iter().enumerate().take(40) {
        emu2.write_u32(geom2 + 0x3c, c.flags_in);
        for (k, v) in c.cached.iter().enumerate() {
            emu2.write_u32(geom2 + 0x40 + (k as u64) * 4, v.to_bits());
        }
        emu2.clear_calls();
        let eax = emu2.call(0x768d20, geom2 as u32, 0, &[]);
        let bin_flags = emu2.read_u32(geom2 + 0x3c);
        let bin_applied = emu2.calls().iter().any(|cc| cc.addr == apply_stub2);
        // assemble failed: the locally-zeroed newRect is irrelevant; assert the fail contract.
        let o = recomp(c.flags_in, c.cached, false, [0.0; 4]);
        assert_eq!(eax, 0, "recompute(fail) case {i}: eax {eax}");
        assert_eq!(eax, o.eax);
        assert_eq!(
            bin_flags, o.flags,
            "recompute(fail) case {i}: flags {bin_flags:#x} vs {:#x}",
            o.flags
        );
        assert!(
            !bin_applied,
            "recompute(fail) case {i}: ApplyRect must not fire"
        );
        fail_checked += 1;
    }

    eprintln!(
        "recompute: {} ok-path bit-exact [unchanged={} applied={}], {} fail-path bit-exact",
        cases.len(),
        tally[0],
        tally[1],
        fail_checked
    );
    assert!(
        tally[0] > 0 && tally[1] > 0,
        "recompute: both change-legs must be hit"
    );
}

#[test]
fn apply_rect_screen() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // frame base; geom subobject sits at frame+0x24 (so the binary's `esi-0x24` reaches the frame's
    // primary vtable). The four side-effect sub-calls are stubbed: the ancestor force-resolve
    // pre-step (0x768cf0), the deferred-dependent notify (0x77f0c0), the strata-dirty mark
    // (0x764a20), and OnSizeChanged (frame primary-vtable +0x48). OnSizeChanged's stub records the
    // call + its (width, height) f32 args so the test can verify whether it fires and the values.
    let onsize_stub = emu.alloc(8);
    emu.stub(onsize_stub, 0, 2); // thiscall(width,height): 2 stack args
    emu.stub(0x768cf0, 0, 1);
    emu.stub(0x77f0c0, 0, 1);
    emu.stub(0x764a20, 0, 1);
    let vtable = emu.alloc(0x80);
    emu.write_u32(vtable + 0x48, onsize_stub as u32);
    let frame = emu.alloc(0x200);
    emu.write_u32(frame, vtable as u32); // frame primary vtable @ +0
    let geom = frame + 0x24;
    emu.write_u32(geom + 0x34, 0); // child list (pre-step no-op anyway; stubbed)
    emu.write_u32(geom + 0x188, 0); // deferred-dependent gate (notify skipped; stubbed)
    let old_va = emu.alloc(0x10);

    struct C {
        cached: [f32; 4], // [bottom, left, top, right]
        inset: [f32; 4],  // [bottom, left, top, right]
        scale: f32,
        old: [f32; 4],
    }
    let c0 = [3.0f32, 100.0, 480.0, 260.0];
    let i0 = [1.0f32, 2.0, 1.5, 2.5];
    let mut cases: Vec<C> = vec![
        // size unchanged (old == cached) -> not fired.
        C {
            cached: c0,
            inset: i0,
            scale: 1.0,
            old: c0,
        },
        // width changed only -> fired.
        C {
            cached: c0,
            inset: i0,
            scale: 1.25,
            old: [3.0, 100.0, 480.0, 300.0],
        },
        // height changed only -> fired.
        C {
            cached: c0,
            inset: i0,
            scale: 0.8,
            old: [10.0, 100.0, 480.0, 260.0],
        },
        // both changed -> fired.
        C {
            cached: c0,
            inset: i0,
            scale: 2.0,
            old: [0.0, 0.0, 50.0, 50.0],
        },
        // zero insets, scale 1 -> screen == cached.
        C {
            cached: c0,
            inset: [0.0; 4],
            scale: 1.0,
            old: c0,
        },
    ];
    let mut s = 0x76b5_0080u32;
    for _ in 0..2500 {
        let base = [
            (next(&mut s) >> 8) as f32 / 5000.0,
            (next(&mut s) >> 8) as f32 / 4000.0,
            (next(&mut s) >> 8) as f32 / 5000.0,
            (next(&mut s) >> 8) as f32 / 4000.0,
        ];
        // old: usually a small perturbation (often within ε on one axis), sometimes equal.
        let same = next(&mut s) & 0x300 == 0;
        let old = if same {
            base
        } else {
            [
                base[0] + ((next(&mut s) >> 8) as f32 / (1u32 << 26) as f32 - 0.5) * 4.0,
                base[1] + ((next(&mut s) >> 8) as f32 / (1u32 << 26) as f32 - 0.5) * 4.0,
                base[2] + ((next(&mut s) >> 8) as f32 / (1u32 << 26) as f32 - 0.5) * 4.0,
                base[3] + ((next(&mut s) >> 8) as f32 / (1u32 << 26) as f32 - 0.5) * 4.0,
            ]
        };
        cases.push(C {
            cached: base,
            inset: [
                (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 6.0,
                (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 6.0,
                (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 6.0,
                (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 6.0,
            ],
            scale: (next(&mut s) >> 8) as f32 / (1u32 << 22) as f32,
            old,
        });
    }

    let mut tally = [0u32; 2]; // [not_fired, fired]
    for (i, c) in cases.iter().enumerate() {
        for (k, v) in c.cached.iter().enumerate() {
            emu.write_u32(geom + 0x40 + (k as u64) * 4, v.to_bits());
        }
        emu.write_u32(geom + 0xc4, c.inset[0].to_bits());
        emu.write_u32(geom + 0xc8, c.inset[1].to_bits());
        emu.write_u32(geom + 0xcc, c.inset[2].to_bits());
        emu.write_u32(geom + 0xd0, c.inset[3].to_bits());
        emu.write_u32(geom + 0x58, c.scale.to_bits());
        for (k, v) in c.old.iter().enumerate() {
            emu.write_u32(old_va + (k as u64) * 4, v.to_bits());
        }
        emu.clear_calls();
        emu.call(0x76b580, geom as u32, 0, &[old_va as u32]);

        let bin_screen: [f32; 4] =
            core::array::from_fn(|k| emu.read_f32(geom + 0xb4 + (k as u64) * 4));
        let onsize = emu.calls().iter().find(|c| c.addr == onsize_stub).copied();
        let bin_fired = onsize.is_some();

        let o = apply_rect(c.cached, c.inset, c.scale, c.old);
        for k in 0..4 {
            assert_eq!(
                bin_screen[k].to_bits(),
                o.screen[k].to_bits(),
                "apply case {i}: screen[{k}] {:#010x} vs {:#010x}",
                bin_screen[k].to_bits(),
                o.screen[k].to_bits()
            );
        }
        assert_eq!(
            bin_fired, o.fired,
            "apply case {i}: fired {bin_fired} vs {}",
            o.fired
        );
        if let Some(call) = onsize {
            assert_eq!(
                call.args[0],
                o.size.0.to_bits(),
                "apply case {i}: width {:#010x} vs {:#010x}",
                call.args[0],
                o.size.0.to_bits()
            );
            assert_eq!(
                call.args[1],
                o.size.1.to_bits(),
                "apply case {i}: height {:#010x} vs {:#010x}",
                call.args[1],
                o.size.1.to_bits()
            );
        }
        tally[o.fired as usize] += 1;
    }
    eprintln!(
        "apply_rect_screen: {} bit-exact; [not_fired={} fired={}]",
        cases.len(),
        tally[0],
        tally[1]
    );
    assert!(
        tally[0] > 0 && tally[1] > 0,
        "apply: both fire-legs must be hit"
    );
}
