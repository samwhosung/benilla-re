//! A group's axis-aligned BSP (`systems/models/wmo.md#bsp-segment-descent`): the box descent
//! `AaBspQueryBoxNode` (`0x6bc1c0`) and the segment descent `AaBspIntersectSegmentNode`
//! (`0x6bc370`). Both are `__thiscall` on a query whose first dword points at the tree: `+0x4` the
//! node array, `+0x8` the leaf face indices (16-bit). A leaf's own test and its per-face callback
//! are the caller's closures.

/// −0.01 (`0x810ccc`), the segment descent's margin below a split or a box face.
const NEG_EPS: f32 = f32::from_bits(0xbc23_d70a);
/// 0.01 (`0x8029d0`), the margin above.
const POS_EPS: f32 = f32::from_bits(0x3c23_d70a);

/// A child index that names no child.
const NO_CHILD: u16 = 0xffff;

/// One 16-byte BSP node.
#[derive(Clone, Copy, Debug)]
pub struct AaBspNode {
    /// `+0x0`: 0x4 a leaf, the low two bits the split axis.
    pub flags: u16,
    /// `+0x2`: the child below the split, 0xffff for none.
    pub neg_child: u16,
    /// `+0x4`: the child above the split, 0xffff for none.
    pub pos_child: u16,
    /// `+0x6`: a leaf's face count.
    pub leaf_face_count: u16,
    /// `+0x8`: a leaf's first entry in the face indices.
    pub face_start: u32,
    /// `+0xc`: the split coordinate.
    pub split_coord: f32,
}

impl AaBspNode {
    /// The node from its 16 bytes.
    pub fn from_bytes(b: &[u8; 0x10]) -> AaBspNode {
        AaBspNode {
            flags: u16::from_le_bytes([b[0], b[1]]),
            neg_child: u16::from_le_bytes([b[2], b[3]]),
            pos_child: u16::from_le_bytes([b[4], b[5]]),
            leaf_face_count: u16::from_le_bytes([b[6], b[7]]),
            face_start: u32::from_le_bytes([b[8], b[9], b[10], b[11]]),
            split_coord: f32::from_le_bytes([b[12], b[13], b[14], b[15]]),
        }
    }

    fn is_leaf(self) -> bool {
        self.flags & 4 != 0
    }

    fn axis(self) -> usize {
        (self.flags & 3) as usize
    }
}

/// Six floats: a box's minimum and maximum, or a segment's start and end.
pub type Box6 = [f32; 6];

/// Calls `face_cb` with each of a leaf's face indices, in order.
fn dispatch_leaf(node: AaBspNode, faces: &[u16], face_cb: &mut dyn FnMut(u16)) {
    let start = node.face_start as usize;
    for k in 0..node.leaf_face_count as usize {
        face_cb(faces[start + k]);
    }
}

/// `0x6bc1c0(node, query, box)`: a box descent. An inner node on axis `a` returns when
/// `query.max[a] < box.min[a]` or `box.max[a] < query.min[a]`, each ordered (`jnp` at `0x6bc24c`,
/// `je` at `0x6bc25e`), so a NaN continues. Its children's boxes are the node's with `min[a]`
/// (above) or `max[a]` (below) set to the split. A query with `min[a] > split` takes the child
/// above alone; else one with `max[a] < split` (ordered, `jp` at `0x6bc2d0`) the child below alone;
/// else both, the one above first, each with the query clipped at the split. A leaf calls `pred`
/// (`0x6b8c60`) and, when it returns false, `face_cb` (`0x6bca50`) for each face.
#[allow(clippy::too_many_arguments, clippy::neg_cmp_op_on_partial_ord)]
pub fn aabsp_box_query(
    nodes: &[AaBspNode],
    faces: &[u16],
    node_index: u16,
    qbox: &Box6,
    nbox: &Box6,
    pred: &mut dyn FnMut(u16) -> bool,
    face_cb: &mut dyn FnMut(u16),
) {
    let node = nodes[node_index as usize];
    if node.is_leaf() {
        if !pred(node_index) {
            dispatch_leaf(node, faces, face_cb);
        }
        return;
    }
    let a = node.axis();
    if qbox[a + 3] < nbox[a] || nbox[a + 3] < qbox[a] {
        return;
    }
    let split = node.split_coord;
    let mut neg_box = *nbox;
    neg_box[a + 3] = split;
    let mut pos_box = *nbox;
    pos_box[a] = split;

    // `!(x > split)` and `!(x < split)` keep the unordered case on the side the branches give it.
    if !(qbox[a] > split) {
        if !(qbox[a + 3] < split) {
            if node.pos_child != NO_CHILD {
                let mut q = *qbox;
                q[a] = split;
                aabsp_box_query(nodes, faces, node.pos_child, &q, &pos_box, pred, face_cb);
            }
            if node.neg_child != NO_CHILD {
                let mut q = *qbox;
                q[a + 3] = split;
                aabsp_box_query(nodes, faces, node.neg_child, &q, &neg_box, pred, face_cb);
            }
        } else if node.neg_child != NO_CHILD {
            aabsp_box_query(nodes, faces, node.neg_child, qbox, &neg_box, pred, face_cb);
        }
    } else if node.pos_child != NO_CHILD {
        aabsp_box_query(nodes, faces, node.pos_child, qbox, &pos_box, pred, face_cb);
    }
}

/// `0x6bc56e` to `0x6bc5c1`, the crossing of `0x6bc370`: `t = f2/(f2 − f3)` with the difference
/// kept in the register (`fsub`, then `fdivr`), stored as `f32`; then `P = A + (B − A)·t` through
/// `0x4841f0`, `0x4549a0` and `0x621160`, each component stored as `f32`. `f2` and `f3` are the
/// stored distances of the segment's ends from the split. Returns `(t, P)`.
pub fn aabsp_segment_split(seg: &Box6, f2: f32, f3: f32) -> (f32, [f32; 3]) {
    let t = (f64::from(f2) / (f64::from(f2) - f64::from(f3))) as f32;
    let mut p = [0.0f32; 3];
    for (c, pc) in p.iter_mut().enumerate() {
        let dir = seg[c + 3] - seg[c];
        *pc = seg[c] + dir * t;
    }
    (t, p)
}

/// `0x6bc370(node, segment, box)`: a segment descent (`systems/models/wmo.md#bsp-segment-descent`).
///
/// A leaf calls `pred` (`0x6b88e0`) and, when it returns false, `face_cb` (`0x6bc700`) for each
/// face. An inner node on axis `a` returns when both ends lie below `box.min[a]` by more than 0.01,
/// or both above `box.max[a]` by more than 0.01: the differences stay in the register, and each
/// compare is ordered (`jp` and `jnp` from `0x6bc403` to `0x6bc444`), so a NaN continues. With `f2`
/// and `f3` the ends' stored distances from the split:
///
/// - either within `[−0.01, 0.01]` (`0x6bc485` to `0x6bc4c3`): the child above, then the child
///   below, with the whole segment;
/// - both above 0.01: the child above alone; both below −0.01: the child below alone;
/// - otherwise, NaN included, the segment splits at `P` ([`aabsp_segment_split`]): with `f2 > 0`
///   (`0x6bc5c1`) the child above takes `[A, P]` and the child below `[P, B]`; else the child below
///   takes `[A, P]` first and the child above `[P, B]`.
///
/// The child above gets the node's box with `min[a]` set to the split, the child below the box with
/// `max[a]` set to it.
#[allow(clippy::too_many_arguments)]
pub fn aabsp_segment_clip(
    nodes: &[AaBspNode],
    faces: &[u16],
    node_index: u16,
    seg: &Box6,
    nbox: &Box6,
    pred: &mut dyn FnMut(u16) -> bool,
    face_cb: &mut dyn FnMut(u16),
) {
    let node = nodes[node_index as usize];
    if node.is_leaf() {
        if !pred(node_index) {
            dispatch_leaf(node, faces, face_cb);
        }
        return;
    }

    let a = node.axis();
    let neg = f64::from(NEG_EPS);
    let pos = f64::from(POS_EPS);
    let below_a = f64::from(seg[a]) - f64::from(nbox[a]);
    let below_b = f64::from(seg[a + 3]) - f64::from(nbox[a]);
    if below_a < neg && below_b < neg {
        return;
    }
    let above_a = f64::from(nbox[a + 3]) - f64::from(seg[a]);
    let above_b = f64::from(nbox[a + 3]) - f64::from(seg[a + 3]);
    if above_a < pos && above_b < pos {
        return;
    }

    let split = node.split_coord;
    let f2 = seg[a] - split;
    let f3 = seg[a + 3] - split;

    let mut box_upper = *nbox;
    box_upper[a] = split;
    let mut box_lower = *nbox;
    box_lower[a + 3] = split;

    let mut visit = |child: u16, s: &Box6, b: &Box6| {
        if child != NO_CHILD {
            aabsp_segment_clip(nodes, faces, child, s, b, pred, face_cb);
        }
    };

    let near = |x: f32| (NEG_EPS..=POS_EPS).contains(&x);
    if near(f2) || near(f3) {
        visit(node.pos_child, seg, &box_upper);
        visit(node.neg_child, seg, &box_lower);
        return;
    }
    if f2 > POS_EPS && f3 > POS_EPS {
        visit(node.pos_child, seg, &box_upper);
        return;
    }
    if f2 < NEG_EPS && f3 < NEG_EPS {
        visit(node.neg_child, seg, &box_lower);
        return;
    }

    let (_, p) = aabsp_segment_split(seg, f2, f3);
    let first: Box6 = [seg[0], seg[1], seg[2], p[0], p[1], p[2]];
    let second: Box6 = [p[0], p[1], p[2], seg[3], seg[4], seg[5]];
    if 0.0 < f2 {
        visit(node.pos_child, &first, &box_upper);
        visit(node.neg_child, &second, &box_lower);
    } else {
        visit(node.neg_child, &first, &box_lower);
        visit(node.pos_child, &second, &box_upper);
    }
}

/// The scaled hit a leaf stores: `CMapObjGroup::IntersectLeaf` (`0x6b88e0`) from `0x6b8af8` to
/// `0x6b8b1b` and `AaBspIntersectLeaf` (`0x6bc700`) from `0x6bc78d` to `0x6bc7ac`. A hit at
/// `0 ≤ t ≤` the query's best `+0x4c` becomes the best, and `t` times the query's `+0x48` is stored
/// as `f32` through its `+0x10`.
pub fn leaf_hit_scale(t: f32, scale: f32) -> f32 {
    (f64::from(t) * f64::from(scale)) as f32
}
