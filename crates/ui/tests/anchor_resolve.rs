//! `CLayoutFrame::CalculateRect` (`0x767a20`) run whole over a synthesized frame and anchor object
//! graph, with the six edge resolvers, the anchor scans and the virtual `RelativePointGetX` and
//! `RelativePointGetY` dispatch all running from the exe, and the resolved rectangle compared bit
//! for bit with `calculate_rect`.
//!
//! The leaf test of `CalculateRect` replaces the four edge resolvers with `fld [cell]; ret` islands;
//! this test runs the composition: calculate, edge, anchor scan, `call [anchor vtable + 4]`,
//! `call [geometry vtable + 0x1c]`, combine. The object graph carries the real vtable addresses
//! (geometry `0x81c400`, anchor `0x81c44c`) so every indirect call reaches the real method
//! (`systems/ui/anchors.md#edge-resolution-from-points`).
#![allow(clippy::float_cmp, clippy::needless_range_loop)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::anchors::{calculate_rect, Anchor, FrameLayout, ResolveOutcome};

const ASSEMBLE: u64 = 0x767a20;
const GEOM_VT: u32 = 0x81c400; // CLayoutFrame geometry vtable (slot 0x1c=GetWidth, 0x20=GetHeight, 0x24=pred→0)
const ANCHOR_VT: u32 = 0x81c44c; // CAnchor vtable (slot 4=ResolveX 0x7a2f90, 8=ResolveY 0x7a3070)
const UNSET_SLOT: u64 = 0xcf550c; // +Inf sentinel (.bss, runtime-init — the test must seed it)
const INF_BITS: u32 = 0x7f80_0000;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// One anchor in a test config: its slot (own point-id 0..8), offsets, the point on `relativeTo`,
/// and `relativeTo`'s cached rect `[bottom, left, top, right]`.
#[derive(Clone, Copy)]
struct AnchorCfg {
    point_id: usize,
    x_off: f32,
    y_off: f32,
    rel_point: u32,
    rel_rect: [f32; 4],
}

#[derive(Clone)]
struct Cfg {
    anchors: Vec<AnchorCfg>,
    width: f32,
    height: f32,
    scale: f32,
    clamp: bool,
    extent_x: f32,
    extent_y: f32,
}

impl Cfg {
    /// The matching `FrameLayout` for `calculate_rect` (mirror=false: the real geom vtable's `+0x24`
    /// normalize predicate `0x46ff60` returns 0 for every ordinary frame).
    fn frame_layout(&self) -> FrameLayout {
        let mut anchors = [None; 9];
        for a in &self.anchors {
            anchors[a.point_id] = Some(Anchor {
                x_off: a.x_off,
                y_off: a.y_off,
                rel_point: a.rel_point,
                rel_rect: a.rel_rect,
                mirror: false,
            });
        }
        FrameLayout {
            anchors,
            width: self.width,
            height: self.height,
            scale: self.scale,
            clamp: self.clamp,
            extent_x: self.extent_x,
            extent_y: self.extent_y,
        }
    }
}

/// A fixed object-graph pool (one child G, 9 CAnchors, 9 root frames) whose fields are rewritten
/// per case — bounded memory across thousands of cases.
struct Pool {
    child: u64,
    canchor: [u64; 9],
    root: [u64; 9],
    out: u64,
}

fn build_pool(emu: &mut Emu) -> Pool {
    emu.write_u32(UNSET_SLOT, INF_BITS);
    let child = emu.alloc(0x60);
    emu.write_u32(child, GEOM_VT);
    let mut canchor = [0u64; 9];
    let mut root = [0u64; 9];
    for i in 0..9 {
        let c = emu.alloc(0x14);
        emu.write_u32(c, ANCHOR_VT);
        canchor[i] = c;
        let r = emu.alloc(0x50);
        emu.write_u32(r, GEOM_VT);
        emu.write_u32(r + 0x3c, 0x1); // flags: bit0=cache-valid, bit1=0 not-dirty (no force-resolve)
        root[i] = r;
    }
    let out = emu.alloc(0x10);
    Pool {
        child,
        canchor,
        root,
        out,
    }
}

/// Write a case's fields into the pool and run the binary's `assemble`; return `(eax, out_rect)`.
fn run_binary(emu: &mut Emu, p: &Pool, c: &Cfg) -> (u32, [f32; 4]) {
    // child G: reset guard, set flags (clamp bit only), size + scale.
    emu.write_u32(p.child + 0x28, 0); // recursion guard cleared
    emu.write_u32(p.child + 0x3c, if c.clamp { 0x10 } else { 0 });
    emu.write_u32(p.child + 0x50, c.width.to_bits());
    emu.write_u32(p.child + 0x54, c.height.to_bits());
    emu.write_u32(p.child + 0x58, c.scale.to_bits());
    // anchorPoints[9] at child+4 + id*4 — clear all, then wire the active anchors.
    for id in 0..9u64 {
        emu.write_u32(p.child + 4 + id * 4, 0);
    }
    for a in &c.anchors {
        let cc = p.canchor[a.point_id];
        let rr = p.root[a.point_id];
        emu.write_u32(cc + 4, a.x_off.to_bits());
        emu.write_u32(cc + 8, a.y_off.to_bits());
        emu.write_u32(cc + 0xc, rr as u32);
        emu.write_u32(cc + 0x10, a.rel_point);
        for k in 0..4u64 {
            emu.write_u32(rr + 0x40 + k * 4, a.rel_rect[k as usize].to_bits());
        }
        emu.write_u32(p.child + 4 + (a.point_id as u64) * 4, cc as u32);
    }
    // clamp screen extents come from 0x41ae60/0x41ae70 (the binary reads .data 0x832a44/0x832a48);
    // when clamp is exercised we set extent_x/y to match those slots so the Rust side agrees.
    if c.clamp {
        emu.write_u32(0x832a44, c.extent_x.to_bits());
        emu.write_u32(0x832a48, c.extent_y.to_bits());
    }
    let eax = emu.call(ASSEMBLE, p.child as u32, 0, &[p.out as u32]);
    let rect = core::array::from_fn(|k| emu.read_f32(p.out + (k as u64) * 4));
    (eax, rect)
}

fn check(emu: &mut Emu, p: &Pool, c: &Cfg, ctx: &str) {
    let (eax, bin_rect) = run_binary(emu, p, c);
    let mine = calculate_rect(&c.frame_layout());
    match mine {
        ResolveOutcome::Unresolvable => {
            assert_eq!(eax, 0, "{ctx}: rust Unresolvable but binary eax={eax}");
        }
        ResolveOutcome::Resolved(rect) => {
            assert_eq!(
                eax, 1,
                "{ctx}: rust Resolved but binary eax=0 (unresolvable)"
            );
            for k in 0..4 {
                assert_eq!(
                    bin_rect[k].to_bits(),
                    rect[k].to_bits(),
                    "{ctx}: rect[{k}] binary {:#010x} ({}) vs rust {:#010x} ({})",
                    bin_rect[k].to_bits(),
                    bin_rect[k],
                    rect[k].to_bits(),
                    rect[k]
                );
            }
        }
    }
}

/// True iff edge `e` (one of LEFT/BOTTOM/RIGHT/TOP) has a direct anchor among its 3 scan ids.
fn edge_direct(c: &Cfg, ids: [usize; 3]) -> bool {
    c.anchors
        .iter()
        .any(|a| ids.contains(&a.point_id) && a.rel_point <= 8)
}

#[test]
fn layout_composed_resolve() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let pool = build_pool(&mut emu);

    let root_screen = [0.0f32, 0.0, 600.0, 800.0]; // [bottom, left, top, right]

    // ── Explicit probes ───────────────────────────────────────────────────────────────────────
    // 1. single TOPLEFT (id0) + explicit W/H → LEFT/TOP direct, RIGHT/BOTTOM derived from size.
    let p1 = Cfg {
        anchors: vec![AnchorCfg {
            point_id: 0,
            x_off: 10.0,
            y_off: -5.0,
            rel_point: 0,
            rel_rect: root_screen,
        }],
        width: 200.0,
        height: 50.0,
        scale: 1.0,
        clamp: false,
        extent_x: 0.8,
        extent_y: 0.6,
    };
    check(&mut emu, &pool, &p1, "probe1 single-TOPLEFT+size");

    // 2. two opposing anchors TOPLEFT(id0)+BOTTOMRIGHT(id8) → all 4 edges direct, W/H ignored.
    let p2 = Cfg {
        anchors: vec![
            AnchorCfg {
                point_id: 0,
                x_off: 10.0,
                y_off: -5.0,
                rel_point: 0,
                rel_rect: root_screen,
            },
            AnchorCfg {
                point_id: 8,
                x_off: -20.0,
                y_off: 15.0,
                rel_point: 8,
                rel_rect: root_screen,
            },
        ],
        width: 12345.0, // ignored
        height: 999.0,  // ignored
        scale: 1.0,
        clamp: false,
        extent_x: 0.8,
        extent_y: 0.6,
    };
    check(&mut emu, &pool, &p2, "probe2 two-opposing");

    // 3. CENTER anchor (id4) + explicit W/H → every edge derives from center + half-size.
    let p3 = Cfg {
        anchors: vec![AnchorCfg {
            point_id: 4,
            x_off: 3.0,
            y_off: 7.0,
            rel_point: 4,
            rel_rect: root_screen,
        }],
        width: 120.0,
        height: 80.0,
        scale: 1.25,
        clamp: false,
        extent_x: 0.8,
        extent_y: 0.6,
    };
    check(&mut emu, &pool, &p3, "probe3 center-anchor+size");

    // 4. no anchors → under-constrained → unresolvable (eax=0).
    let p4 = Cfg {
        anchors: vec![],
        width: 100.0,
        height: 100.0,
        scale: 1.0,
        clamp: false,
        extent_x: 0.8,
        extent_y: 0.6,
    };
    check(&mut emu, &pool, &p4, "probe4 no-anchors-unresolvable");

    // 5. single LEFT-point (id3) anchor + W/H, no height anchor → RIGHT derives, TOP/BOTTOM derive.
    let p5 = Cfg {
        anchors: vec![AnchorCfg {
            point_id: 3,
            x_off: 4.0,
            y_off: 2.0,
            rel_point: 3,
            rel_rect: root_screen,
        }],
        width: 0.0, // no explicit width → RIGHT unresolvable via size → whole rect unresolvable
        height: 0.0,
        scale: 1.0,
        clamp: false,
        extent_x: 0.8,
        extent_y: 0.6,
    };
    check(&mut emu, &pool, &p5, "probe5 left-only-no-size");

    // 6. clamp: small rect anchored so an edge runs past the extent / below 0.
    let p6 = Cfg {
        anchors: vec![AnchorCfg {
            point_id: 0,
            x_off: -50.0, // LEFT goes negative → clamp shifts X
            y_off: 5.0,
            rel_point: 0,
            rel_rect: [0.0, 0.0, 600.0, 700.0],
        }],
        width: 900.0, // RIGHT past extent
        height: 40.0,
        scale: 1.0,
        clamp: true,
        extent_x: 800.0,
        extent_y: 600.0,
    };
    check(&mut emu, &pool, &p6, "probe6 clamp");

    // ── Fuzz ──────────────────────────────────────────────────────────────────────────────────
    let mut s = 0x1a70_c0de_u32;
    let mut n_resolved = 0u32;
    let mut n_unresolved = 0u32;
    let mut n_all_direct = 0u32;
    let mut n_derived = 0u32;
    let mut n_center_anchor = 0u32;
    let mut n_clamp = 0u32;

    for _ in 0..6000 {
        // 0..4 anchors on distinct point-ids.
        let k = (next(&mut s) >> 8) % 5;
        let mut used = [false; 9];
        let mut anchors: Vec<AnchorCfg> = Vec::new();
        for _ in 0..k {
            let id = ((next(&mut s) >> 8) % 9) as usize;
            if used[id] {
                continue;
            }
            used[id] = true;
            // mostly valid rel_point (0..8); occasionally >8 to exercise the reject→sentinel path.
            let rp = if next(&mut s) & 0x700 == 0 {
                9 + (next(&mut s) >> 8) % 3
            } else {
                (next(&mut s) >> 8) % 9
            };
            let coord = |s: &mut u32, sc: f32| (next(s) >> 8) as f32 / (1u32 << 24) as f32 * sc;
            let rel_rect = [
                coord(&mut s, 600.0),
                coord(&mut s, 800.0),
                coord(&mut s, 600.0) + 200.0,
                coord(&mut s, 800.0) + 200.0,
            ];
            anchors.push(AnchorCfg {
                point_id: id,
                x_off: coord(&mut s, 200.0) - 100.0,
                y_off: coord(&mut s, 200.0) - 100.0,
                rel_point: rp,
                rel_rect,
            });
        }
        // size: often explicit, sometimes 0 (no-explicit-size → derive needs opposite/center).
        let sz = |s: &mut u32| {
            if next(s) & 0x300 == 0 {
                0.0
            } else {
                (next(s) >> 8) as f32 / (1u32 << 24) as f32 * 400.0
            }
        };
        let clamp = next(&mut s) & 0x40 != 0;
        let cfg = Cfg {
            anchors,
            width: sz(&mut s),
            height: sz(&mut s),
            scale: (next(&mut s) >> 8) as f32 / (1u32 << 23) as f32 * 2.0,
            clamp,
            extent_x: 600.0 + (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 400.0,
            extent_y: 400.0 + (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 400.0,
        };

        // tally (computed from the config, independent of the diff).
        let ld = edge_direct(&cfg, IDS_LEFT);
        let rd = edge_direct(&cfg, IDS_RIGHT);
        let td = edge_direct(&cfg, IDS_TOP);
        let bd = edge_direct(&cfg, IDS_BOTTOM);
        if ld && rd && td && bd {
            n_all_direct += 1;
        }
        if !(ld && rd && td && bd) {
            n_derived += 1;
        }
        if cfg
            .anchors
            .iter()
            .any(|a| matches!(a.point_id, 1 | 3 | 4 | 5 | 7) && a.rel_point <= 8)
        {
            n_center_anchor += 1;
        }
        if clamp {
            n_clamp += 1;
        }
        match calculate_rect(&cfg.frame_layout()) {
            ResolveOutcome::Resolved(_) => n_resolved += 1,
            ResolveOutcome::Unresolvable => n_unresolved += 1,
        }
        check(&mut emu, &pool, &cfg, "fuzz");
    }

    eprintln!(
        "layout_composed_resolve: 6006 cases bit-exact; \
         [resolved={n_resolved} unresolvable={n_unresolved} all_direct={n_all_direct} \
         derived={n_derived} center_anchor={n_center_anchor} clamp={n_clamp}]"
    );
    assert!(n_resolved > 0 && n_unresolved > 0, "both eax legs must hit");
    assert!(n_all_direct > 0, "two-opposing (all-direct) leg never hit");
    assert!(n_derived > 0, "derived-from-size leg never hit");
    assert!(n_center_anchor > 0, "center-anchor leg never hit");
    assert!(n_clamp > 0, "clamp leg never hit");
}

const IDS_LEFT: [usize; 3] = [0, 3, 6];
const IDS_RIGHT: [usize; 3] = [2, 5, 8];
const IDS_TOP: [usize; 3] = [0, 1, 2];
const IDS_BOTTOM: [usize; 3] = [6, 7, 8];
