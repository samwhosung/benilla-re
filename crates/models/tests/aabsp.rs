//! The BSP descents `AaBspQueryBoxNode` (`0x6bc1c0`) and `AaBspIntersectSegmentNode` (`0x6bc370`),
//! run whole on trees laid out in memory. The leaf test (`0x6b8c60`, `0x6b88e0`) is stubbed to
//! return 0 and the per-face callback (`0x6bca50`, `0x6bc700`) to return at once; the result is the
//! order of the face indices the callback receives. The crossing arithmetic of `0x6bc370` is also
//! run as a slice, since a dispatch order seldom shows a rounding of the split point, and so is
//! each leaf's scaled hit.

#![allow(clippy::needless_range_loop)] // the loops index the client's memory layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::aabsp::{
    aabsp_box_query, aabsp_segment_clip, aabsp_segment_split, leaf_hit_scale, AaBspNode, Box6,
};
use common::{assert_f32, fuzz_wide, next};

/// A node as written to memory.
#[derive(Clone, Copy)]
struct RawNode {
    flags: u16,
    neg: u16,
    pos: u16,
    leaf_count: u16,
    face_start: u32,
    split: f32,
}

impl RawNode {
    fn internal(axis: u16, split: f32, neg: u16, pos: u16) -> RawNode {
        RawNode {
            flags: axis & 3,
            neg,
            pos,
            leaf_count: 0,
            face_start: 0,
            split,
        }
    }

    fn leaf(face_start: u32, leaf_count: u16) -> RawNode {
        RawNode {
            flags: 4,
            neg: 0,
            pos: 0,
            leaf_count,
            face_start,
            split: 0.0,
        }
    }

    fn bytes(self) -> [u8; 0x10] {
        let mut b = [0u8; 0x10];
        b[0..2].copy_from_slice(&self.flags.to_le_bytes());
        b[2..4].copy_from_slice(&self.neg.to_le_bytes());
        b[4..6].copy_from_slice(&self.pos.to_le_bytes());
        b[6..8].copy_from_slice(&self.leaf_count.to_le_bytes());
        b[8..12].copy_from_slice(&self.face_start.to_le_bytes());
        b[12..16].copy_from_slice(&self.split.to_le_bytes());
        b
    }
}

fn decode(nodes: &[RawNode]) -> Vec<AaBspNode> {
    nodes
        .iter()
        .map(|n| AaBspNode::from_bytes(&n.bytes()))
        .collect()
}

/// Writes the tree and returns the query: its first dword points at the tree, whose `+0x4` is the
/// node array and `+0x8` the face indices; `+0x4` of the query is the leaf test's `this`.
fn lay_tree(emu: &mut Emu, nodes: &[RawNode], faces: &[u16]) -> u32 {
    let node_base = emu.alloc((nodes.len().max(1) * 0x10) as u64);
    for (i, n) in nodes.iter().enumerate() {
        emu.write(node_base + (i * 0x10) as u64, &n.bytes());
    }
    let face_base = emu.alloc((faces.len().max(1) * 2) as u64);
    for (i, f) in faces.iter().enumerate() {
        emu.write(face_base + (i * 2) as u64, &f.to_le_bytes());
    }
    let t = emu.alloc(0x10);
    emu.write_u32(t + 4, node_base as u32);
    emu.write_u32(t + 8, face_base as u32);
    let ctx = emu.alloc(0x10);
    emu.write_u32(ctx, t as u32);
    emu.write_u32(ctx + 4, ctx as u32);
    ctx as u32
}

fn write_box(emu: &mut Emu, va: u64, b: &Box6) {
    for (i, v) in b.iter().enumerate() {
        emu.write(va + (i * 4) as u64, &v.to_le_bytes());
    }
}

/// The face indices the stubbed callback at `cb` received, in order.
fn dispatched(emu: &Emu, cb: u64) -> Vec<u16> {
    emu.calls()
        .iter()
        .filter(|c| c.addr == cb)
        .map(|c| c.args[0] as u16)
        .collect()
}

/// A random tree of depth at most 3 over the split range 0 to 10, with faces numbered from 100; a
/// child is sometimes 0xffff.
fn random_tree(s: &mut u32) -> (Vec<RawNode>, Vec<u16>) {
    fn build(s: &mut u32, depth: u32, nodes: &mut Vec<RawNode>, faces: &mut Vec<u16>) -> u16 {
        let make_leaf = depth >= 3 || (next(s) >> 8).is_multiple_of(3);
        if make_leaf {
            let cnt = ((next(s) >> 8) % 3) as u16;
            let start = faces.len() as u32;
            for _ in 0..cnt {
                let f = (faces.len() as u16).wrapping_add(100);
                faces.push(f);
            }
            let idx = nodes.len() as u16;
            nodes.push(RawNode::leaf(start, cnt));
            return idx;
        }
        let axis = ((next(s) >> 8) % 3) as u16;
        let split = ((next(s) >> 8) % 1000) as f32 / 100.0;
        let idx = nodes.len() as u16;
        nodes.push(RawNode::internal(axis, split, 0, 0));
        let prune_neg = (next(s) >> 8).is_multiple_of(8);
        let prune_pos = (next(s) >> 8).is_multiple_of(8);
        let neg = if prune_neg {
            0xffff
        } else {
            build(s, depth + 1, nodes, faces)
        };
        let pos = if prune_pos {
            0xffff
        } else {
            build(s, depth + 1, nodes, faces)
        };
        nodes[idx as usize].neg = neg;
        nodes[idx as usize].pos = pos;
        idx
    }
    let mut nodes = Vec::new();
    let mut faces = Vec::new();
    build(s, 0, &mut nodes, &mut faces);
    if faces.is_empty() {
        faces.push(0);
    }
    (nodes, faces)
}

/// A random box, minimum at most maximum, from −2 to 12 on a 0.01 grid.
fn random_box(s: &mut u32) -> Box6 {
    let mut b = [0.0f32; 6];
    for k in 0..3 {
        let lo = (next(s) >> 8) % 1400;
        let hi = lo + (next(s) >> 8) % 600;
        b[k] = lo as f32 / 100.0 - 2.0;
        b[k + 3] = hi as f32 / 100.0 - 2.0;
    }
    b
}

/// Root split on x at 5: leaf {10, 11} below, leaf {12} above.
fn tree3(axis: u16, split: f32) -> (Vec<RawNode>, Vec<u16>) {
    (
        vec![
            RawNode::internal(axis, split, 1, 2),
            RawNode::leaf(0, 2),
            RawNode::leaf(2, 1),
        ],
        vec![10, 11, 12],
    )
}

/// Root split on x at 5; below it a split on y at 3 over leaves {20} and {21, 22}; above it {23}.
fn tree_deep() -> (Vec<RawNode>, Vec<u16>) {
    (
        vec![
            RawNode::internal(0, 5.0, 1, 4),
            RawNode::internal(1, 3.0, 2, 3),
            RawNode::leaf(0, 1),
            RawNode::leaf(1, 2),
            RawNode::leaf(3, 1),
        ],
        vec![20, 21, 22, 23],
    )
}

type Case = (&'static str, Vec<RawNode>, Vec<u16>, Box6, Box6);

fn box_query_cases() -> Vec<Case> {
    let full: Box6 = [0.0, 0.0, 0.0, 10.0, 10.0, 10.0];
    let nan = f32::NAN;
    let (t3, f3) = tree3(0, 5.0);
    let (td, fd) = tree_deep();
    let case = |label, q: Box6| (label, t3.clone(), f3.clone(), q, full);
    vec![
        case("below", [0.0, 0.0, 0.0, 2.0, 10.0, 10.0]),
        case("above", [6.0, 0.0, 0.0, 9.0, 10.0, 10.0]),
        case("straddle", [3.0, 0.0, 0.0, 8.0, 10.0, 10.0]),
        case("no-overlap", [20.0, 0.0, 0.0, 30.0, 10.0, 10.0]),
        case("lo-on-split", [5.0, 0.0, 0.0, 8.0, 10.0, 10.0]),
        case("hi-on-split", [1.0, 0.0, 0.0, 5.0, 10.0, 10.0]),
        case("touch-box-min", [-3.0, 0.0, 0.0, 0.0, 10.0, 10.0]),
        case("touch-box-max", [10.0, 0.0, 0.0, 13.0, 10.0, 10.0]),
        case("nan-lo", [nan, 0.0, 0.0, 2.0, 10.0, 10.0]),
        case("nan-hi", [1.0, 0.0, 0.0, nan, 10.0, 10.0]),
        case("nan-both", [nan, 0.0, 0.0, nan, 10.0, 10.0]),
        (
            "nan-split",
            tree3(0, nan).0,
            f3.clone(),
            [1.0, 0.0, 0.0, 2.0, 10.0, 10.0],
            full,
        ),
        (
            "nan-box",
            t3.clone(),
            f3.clone(),
            [1.0, 0.0, 0.0, 2.0, 10.0, 10.0],
            [nan, 0.0, 0.0, nan, 10.0, 10.0],
        ),
        (
            "deep-straddle",
            td.clone(),
            fd.clone(),
            [0.0, 0.0, 0.0, 9.0, 9.0, 9.0],
            full,
        ),
        (
            "deep-neg-below-y",
            td,
            fd,
            [0.0, 0.0, 0.0, 2.0, 2.0, 9.0],
            full,
        ),
        (
            "leaf-root",
            vec![RawNode::leaf(0, 2)],
            vec![30, 31],
            full,
            full,
        ),
    ]
}

/// `0x6bc1c0` whole: `ecx` the query, then the node, the query box and the node's box on the stack.
#[test]
fn aabsp_box_query_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x6b8c60, 0, 2);
    emu.stub(0x6bca50, 0, 1);
    let qbox_va = emu.alloc(0x20);
    let nbox_va = emu.alloc(0x20);

    let run = |emu: &mut Emu, nodes: &[RawNode], faces: &[u16], q: &Box6, n: &Box6, ctx: &str| {
        let query = lay_tree(emu, nodes, faces);
        write_box(emu, qbox_va, q);
        write_box(emu, nbox_va, n);
        emu.clear_calls();
        emu.call(0x6bc1c0, query, 0, &[0, qbox_va as u32, nbox_va as u32]);
        let binary = dispatched(emu, 0x6bca50);
        let mut ours = Vec::new();
        aabsp_box_query(&decode(nodes), faces, 0, q, n, &mut |_| false, &mut |f| {
            ours.push(f)
        });
        assert_eq!(binary, ours, "{ctx}: q={q:?} n={n:?}");
    };

    for (label, nodes, faces, q, n) in &box_query_cases() {
        run(&mut emu, nodes, faces, q, n, label);
    }
    let s = &mut 0x626f_7831u32;
    for i in 0..400 {
        let (nodes, faces) = random_tree(s);
        let q = random_box(s);
        let n = random_box(s);
        run(&mut emu, &nodes, &faces, &q, &n, &format!("fuzz[{i}]"));
    }
}

fn segment_clip_cases() -> Vec<Case> {
    let nbox: Box6 = [0.0, 0.0, 0.0, 10.0, 10.0, 10.0];
    let nan = f32::NAN;
    let eps = f32::from_bits(0x3c23_d70a);
    let (t3, f3) = tree3(0, 5.0);
    let (t0, _) = tree3(0, 0.0);
    let (td, fd) = tree_deep();
    let case = |label, s: Box6| (label, t3.clone(), f3.clone(), s, nbox);
    let at0 = |label, s: Box6| (label, t0.clone(), f3.clone(), s, nbox);
    vec![
        case("both-below", [1.0, 1.0, 1.0, 2.0, 2.0, 2.0]),
        case("both-above", [6.0, 1.0, 1.0, 8.0, 9.0, 9.0]),
        case("cross-neg-first", [2.0, 2.0, 2.0, 8.0, 8.0, 8.0]),
        case("cross-pos-first", [8.0, 8.0, 8.0, 2.0, 2.0, 2.0]),
        case("coplanar", [5.0, 2.0, 2.0, 5.0, 8.0, 8.0]),
        case("gate-out", [50.0, 1.0, 1.0, 60.0, 2.0, 2.0]),
        case("gate-out-below", [-5.0, 1.0, 1.0, -3.0, 2.0, 2.0]),
        // the ends exactly 0.01 outside the box: the difference equals the margin and continues
        case("gate-at-min", [-eps, 1.0, 1.0, -eps, 2.0, 2.0]),
        case("gate-at-max", [10.0 + eps, 1.0, 1.0, 10.0 + eps, 2.0, 2.0]),
        // the ends' distances from a split at 0 exactly ±0.01: within the band
        at0("split-at-pos-eps", [eps, 1.0, 1.0, eps, 2.0, 2.0]),
        at0("split-at-neg-eps", [-eps, 1.0, 1.0, -eps, 2.0, 2.0]),
        at0("one-end-at-eps", [eps, 1.0, 1.0, 3.0, 2.0, 2.0]),
        at0("other-end-at-neg-eps", [3.0, 1.0, 1.0, -eps, 2.0, 2.0]),
        case("nan-start", [nan, 1.0, 1.0, 2.0, 2.0, 2.0]),
        case("nan-end", [2.0, 1.0, 1.0, nan, 2.0, 2.0]),
        case("nan-both", [nan, 1.0, 1.0, nan, 2.0, 2.0]),
        (
            "nan-split",
            tree3(0, nan).0,
            f3.clone(),
            [2.0, 1.0, 1.0, 8.0, 2.0, 2.0],
            nbox,
        ),
        (
            "nan-box",
            t3.clone(),
            f3.clone(),
            [2.0, 1.0, 1.0, 8.0, 2.0, 2.0],
            [nan, 0.0, 0.0, nan, 10.0, 10.0],
        ),
        ("deep-cross", td, fd, [1.0, 1.0, 1.0, 8.0, 8.0, 8.0], nbox),
        (
            "leaf-root",
            vec![RawNode::leaf(0, 3)],
            vec![30, 31, 32],
            [1.0, 1.0, 1.0, 2.0, 2.0, 2.0],
            nbox,
        ),
    ]
}

/// `0x6bc370` whole: `ecx` the query, then the node, the segment and the node's box on the stack.
#[test]
fn aabsp_segment_clip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x6b88e0, 0, 2);
    emu.stub(0x6bc700, 0, 1);
    let seg_va = emu.alloc(0x20);
    let nbox_va = emu.alloc(0x20);

    let run = |emu: &mut Emu, nodes: &[RawNode], faces: &[u16], s: &Box6, n: &Box6, ctx: &str| {
        let query = lay_tree(emu, nodes, faces);
        write_box(emu, seg_va, s);
        write_box(emu, nbox_va, n);
        emu.clear_calls();
        emu.call(0x6bc370, query, 0, &[0, seg_va as u32, nbox_va as u32]);
        let binary = dispatched(emu, 0x6bc700);
        let mut ours = Vec::new();
        aabsp_segment_clip(&decode(nodes), faces, 0, s, n, &mut |_| false, &mut |f| {
            ours.push(f)
        });
        assert_eq!(binary, ours, "{ctx}: seg={s:?} n={n:?}");
    };

    for (label, nodes, faces, s, n) in &segment_clip_cases() {
        run(&mut emu, nodes, faces, s, n, label);
    }
    let s = &mut 0x7365_6731u32;
    for i in 0..400 {
        let (nodes, faces) = random_tree(s);
        let seg = random_box(s);
        let n = random_box(s);
        run(&mut emu, &nodes, &faces, &seg, &n, &format!("fuzz[{i}]"));
    }
}

/// `0x6bc56e` to `0x6bc5c1`: `[ebp+0x10]` and `[ebp+8]` the ends' distances from the split, `ecx`
/// and `[ebp+0xc]` the segment; `t` is read back from `[ebp+8]` and `P` from `[ebp-0x10]`.
/// Distances whose difference is not a `f32` show a model that rounds it.
#[test]
fn aabsp_segment_split_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let seg_va = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run = |e: &mut Emu, seg: &Box6, f2: f32, f3: f32, ctx: &str| {
        write_box(e, seg_va, seg);
        e.write_u32(ebp + 0x10, f2.to_bits());
        e.write_u32(ebp + 0x8, f3.to_bits());
        e.write_u32(ebp + 0xc, seg_va as u32);
        e.set_reg(RegisterX86::ECX, seg_va);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x80);
        e.run(0x6bc56e, 0x6bc5c1);
        let (t, p) = aabsp_segment_split(seg, f2, f3);
        assert_f32(e.read_f32(ebp + 0x8), t, &format!("{ctx}: t"));
        for c in 0..3 {
            let got = e.read_f32(ebp - 0x10 + 4 * c as u64);
            assert_f32(got, p[c], &format!("{ctx}: P[{c}]"));
        }
    };

    let seg: Box6 = [1.0, 2.0, 3.0, 9.0, -4.0, 7.5];
    run(&mut e, &seg, -3.0, 5.0, "whole");
    run(&mut e, &seg, 1.000_000_1, -0.3, "inexact difference");
    run(&mut e, &seg, 3.0e7, -0.7, "far start");
    run(&mut e, &seg, f32::NAN, 1.0, "nan f2");
    run(&mut e, &seg, 0.02, 0.02, "equal");
    let s = &mut 0x6bc5_6e01u32;
    for i in 0..3000 {
        let mut b = [0.0f32; 6];
        for v in b.iter_mut() {
            *v = f32::from_bits(next(s) >> 9 | 0x4000_0000) - 3.0;
        }
        let f2 = f32::from_bits((next(s) >> 9) | 0x3f80_0000) - 1.5;
        let f3 = -f32::from_bits((next(s) >> 9) | 0x3f80_0000) + 0.5;
        run(&mut e, &b, f2, f3, &format!("fuzz[{i}]"));
    }
}

/// Runs a leaf's scaled-hit stretch from `start` to `until`: `t` at `[ebp+t_disp]`, the query in
/// `esi` with the scale at `+0x48` and the output pointer at `+0x10`.
fn leaf_scale_case(start: u64, until: u64, t_disp: u64, seed: u32) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let query = e.alloc(0x80);
    let result = e.alloc(0x10);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let run = |e: &mut Emu, t: f32, scale: f32, ctx: &str| {
        e.write_u32(ebp + t_disp, t.to_bits());
        e.write_u32(query + 0x48, scale.to_bits());
        e.write_u32(query + 0x10, result as u32);
        e.write_u32(result, 0xdead_beef);
        e.set_reg(RegisterX86::ESI, query);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(start, until);
        assert_f32(e.read_f32(result), leaf_hit_scale(t, scale), ctx);
    };
    run(&mut e, 0.0, 5.0, "t=0");
    run(&mut e, 1.0, 5.0, "t=1");
    run(&mut e, 2.0, 0.5, "t=2 scale=0.5");
    run(&mut e, -3.0, 4.0, "negative t");
    run(&mut e, 1.0e20, 1.0e20, "overflow");
    run(&mut e, f32::NAN, 2.0, "nan t");
    let mut s = seed;
    for i in 0..5000u32 {
        let t = fuzz_wide(&mut s);
        let scale = fuzz_wide(&mut s);
        run(&mut e, t, scale, &format!("fuzz[{i}] t={t} scale={scale}"));
    }
}

/// `0x6b8af8` to `0x6b8b1b` of `CMapObjGroup::IntersectLeaf` (`0x6b88e0`), `t` at `[ebp+0xc]`.
#[test]
fn wmobsp_leaf_tscale_difftest() {
    leaf_scale_case(0x6b8af8, 0x6b8b1b, 0xc, 0x6b88_e0e0);
}

/// `0x6bc78d` to `0x6bc7ac` of `AaBspIntersectLeaf` (`0x6bc700`), `t` at `[ebp+8]`.
#[test]
fn aabsp_leaf_tscale_difftest() {
    leaf_scale_case(0x6bc78d, 0x6bc7ac, 0x8, 0x6bc7_00c7);
}
