//! The frame pass (`systems/core/framepass.md`): the elapsed time it hands its layers and the
//! walk that decides which screen layers it calls, over the region calls at `0x65c610` and after.
//!
//! A rectangle is `[left, bottom, right, top]`, as the layer record and the region calls store it.
//! The negated compares are the x87 tests' own: each takes the NaN side the client's `jcc` takes.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

/// A rectangle as `[left, bottom, right, top]`.
pub type RectF = [f32; 4];

/// The frame-time handler `0x442330`: `fld [0x885608]; fadd dword [dt]; fstp [0x885608]`, the
/// sum at 53 bits rounded once to `f32`, which equals the `f32` sum: two `f32` added at 53 bits
/// and rounded to 24 round once in effect, since 53 ≥ 2·24 + 2.
#[must_use]
pub fn frame_time_accumulate(acc: f32, dt: f32) -> f32 {
    (f64::from(acc) + f64::from(dt)) as f32
}

/// Where `ScrnLayerCreate` (`0x442800`) links a layer of priority `p` into a list held head to
/// tail as `priorities`: walking from the tail, after the first record whose priority is not
/// greater than `p` (`fcomp; test ah,0x41; jne`, so a NaN on either side stops too), else at the
/// head. The list runs from the lowest priority to the highest, and among equal priorities the
/// newest is nearest the tail.
#[must_use]
pub fn layer_insert_index(priorities: &[f32], p: f32) -> usize {
    for (i, &q) in priorities.iter().enumerate().rev() {
        if !(q > p) {
            return i + 1;
        }
    }
    0
}

/// The region as the frame pass uses it: the pieces left of the unit square after each
/// covering layer's rectangle is taken out.
#[derive(Debug, Clone, Default)]
pub struct Region {
    pieces: Vec<RectF>,
}

fn nonempty(r: &RectF) -> bool {
    r[0] < r[2] && r[1] < r[3]
}

impl Region {
    /// `0x65bd90` with mode 2: a rectangle with `left < right` and `bottom < top`
    /// joins as a piece; any other is ignored.
    pub fn add(&mut self, r: RectF) {
        if nonempty(&r) {
            self.pieces.push(r);
        }
    }

    /// `0x65bd90` with mode 4: the part of the region inside a rectangle with `left < right`
    /// and `bottom < top` is taken out; any other rectangle changes nothing.
    pub fn subtract(&mut self, r: RectF) {
        if !nonempty(&r) {
            return;
        }
        let mut out = Vec::with_capacity(self.pieces.len());
        for p in self.pieces.drain(..) {
            let overlaps = p[0] < r[2] && r[0] < p[2] && p[1] < r[3] && r[1] < p[3];
            if !overlaps {
                out.push(p);
                continue;
            }
            let (l, rt) = (p[0].max(r[0]), p[2].min(r[2]));
            for q in [
                [p[0], p[1], r[0], p[3]],
                [r[2], p[1], p[2], p[3]],
                [l, p[1], rt, r[1]],
                [l, r[3], rt, p[3]],
            ] {
                if nonempty(&q) {
                    out.push(q);
                }
            }
        }
        self.pieces = out;
    }

    /// `0x65ca80`, the bounding rectangle: from `[f32::MAX, f32::MAX, 2^-126, 2^-126]`, each
    /// piece takes `left` and `bottom` down when not greater (`test ah,0x41; jne`) and `right` and
    /// `top` up when not less (`test ah,0x5; jp`); a result without `left < right` and
    /// `bottom < top` is all zeros.
    #[must_use]
    pub fn bounding_rect(&self) -> RectF {
        let tiny = f32::from_bits(0x0080_0000);
        let mut b = [f32::MAX, f32::MAX, tiny, tiny];
        for p in &self.pieces {
            for i in 0..2 {
                if !(p[i] > b[i]) {
                    b[i] = p[i];
                }
            }
            for i in 2..4 {
                if !(p[i] < b[i]) {
                    b[i] = p[i];
                }
            }
        }
        if nonempty(&b) {
            b
        } else {
            [0.0; 4]
        }
    }
}

/// A screen layer as the frame pass reads it: its rectangle `+0x08`, its flags `+0x2c`.
#[derive(Debug, Clone, Copy)]
pub struct Layer {
    pub rect: RectF,
    pub flags: u32,
}

/// The layer walk of `RunFramePass` (`0x442350`) over a list held head (lowest priority) to tail:
/// for each layer, the visible rectangle `+0x18` it stores and whether its callback runs.
///
/// The first walk goes from the tail with a region holding the unit square: each layer's visible
/// rectangle is the region's bounding rectangle clamped to its own (`left` and `bottom` the
/// larger, `right` and `top` the smaller, the layer's on a NaN), and a layer whose flag bit 0 is
/// clear then takes its own rectangle out of the region. The second walk, from the head, calls a
/// layer unless its visible rectangle has `right` not above `left` or `top` not above `bottom`.
#[must_use]
pub fn frame_pass_layers(layers: &[Layer]) -> Vec<(RectF, bool)> {
    let mut region = Region::default();
    region.add([0.0, 0.0, 1.0, 1.0]);
    let mut out = vec![([0.0; 4], false); layers.len()];
    for (i, layer) in layers.iter().enumerate().rev() {
        let b = region.bounding_rect();
        let r = layer.rect;
        let v = [
            if r[0] < b[0] { b[0] } else { r[0] },
            if r[1] < b[1] { b[1] } else { r[1] },
            if r[2] > b[2] { b[2] } else { r[2] },
            if r[3] > b[3] { b[3] } else { r[3] },
        ];
        let draws = !(v[2] <= v[0]) && !(v[3] <= v[1]);
        out[i] = (v, draws);
        if layer.flags & 1 == 0 {
            region.subtract(r);
        }
    }
    out
}
