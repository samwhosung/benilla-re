//! `CM2Model::EnsureAnimated` (`0x714000`) (`systems/models/animation.md#ensure-animated`): it
//! animates the instance's ancestors first, then the instance itself with a parent matrix taken
//! from its parent's attachment.
//!
//! - `0x714009`: when `[this+0x40]` equals the scene's frame `[[this+0x2c]+0x10]`, it returns.
//! - `0x714019`: with a parent `[this+0x1cc]`, it recurses on the parent. Without one it calls
//!   `CM2Model::Animate` (`0x714260`) with the identity transform and `[this+0x2c]+0x9c` as the
//!   parent matrix.
//! - `0x71406e`: the frame test again after the recursion.
//! - With a parent and `[this+0x10]` nonzero, the instance animates. It composes
//!   (`0x71407d`) when `[parent+0x10]` is nonzero, `[parent+0x40]` equals the scene's frame and
//!   `[this+0x1d4]` is not 0xffff. The attachment record `[parent+0x30]`, `+0x130`, `+0x108` at
//!   `[this+0x1d4] * 0x30` names a bone matrix `[parent+0x94] + word[rec+4] * 0x40`, which is
//!   copied and has the record's offset `(+8, +0xc, +0x10)` added to its translation
//!   (`0x7140ec` to `0x714149`). Otherwise the parent matrix is `[parent]+0xfc` (`0x714160`). In
//!   both the colour arguments are the parent's `+0x1a0` and `+0x1ac` and the alpha is
//!   `[parent+0x19c]`.
//! - `0x714183`: after that call it returns if the instance's frame now equals the scene's.
//! - `0x71418e`: with a parent, `[this+0xfc]` becomes a copy of `[parent+0xfc]`. Without one it
//!   becomes `[this+0xbc]` times `[this+0x2c]+0x9c`, by `0x7bc6a0`.
//!
//! The translation sums run at 53 bits and narrow once at each `fstp dword`. `CM2Model::Animate`
//! is an argument: the test records each call's leg, instance, matrix and alpha.

use benilla_re_cmath::multiply::multiply_matrix4;

/// An attachment record (`[parent+0x30]`, `+0x130`, `+0x108`, stride 0x30). The compose leg reads
/// the bone word at `+4` and the offset at `+8`, `+0xc` and `+0x10`.
#[derive(Clone, Copy, Debug, Default)]
pub struct AttachmentRecord {
    /// `word[rec+4]`: the bone whose matrix is at `[parent+0x94] + bone * 0x40`.
    pub bone: u16,
    /// `rec+8`, `+0xc`, `+0x10`: the offset, carried through the matrix's 3×3 into its
    /// translation.
    pub offset: [f32; 3],
}

/// The fields of a `CM2Model` instance that `EnsureAnimated` reads and writes. What it reads
/// through `[this+0x2c]`, `[parent+0x30]` and `[parent+0x94]` is in [`InstanceGraph`].
#[derive(Clone, Debug, Default)]
pub struct Instance {
    /// `+0x10`: the instance is loaded. It gates the whole stage for this instance, and the
    /// compose leg for the parent.
    pub loaded: u32,
    /// `+0x40`: the frame the instance was last animated in, compared with the scene's frame.
    pub frame: u32,
    /// `+0x1cc`: the parent (`None` is null).
    pub parent: Option<usize>,
    /// `+0x1d4`: the attachment index; 0xffff takes the parent's world matrix.
    pub attachment_index: u32,
    /// `+0xbc`: the base matrix, the left operand of the product when there is no parent.
    pub base_matrix: [f32; 16],
    /// `+0xfc`: the world matrix, the output and the copy source for children.
    pub world_matrix: [f32; 16],
    /// `+0x19c`: the alpha, passed on to a child.
    pub alpha: f32,
}

/// The instances, and what they read through `[this+0x2c]`, `[parent+0x30]` and `[parent+0x94]`.
pub struct InstanceGraph<'a> {
    pub instances: &'a mut [Instance],
    /// `[[this+0x2c]+0x10]`: the scene's frame.
    pub scene_frame: u32,
    /// `[this+0x2c]+0x9c`: the matrix passed to `Animate` with no parent, and the right operand
    /// of the product.
    pub scene_matrix: [f32; 16],
    /// The attachment records (`[parent+0x30]`, `+0x130`, `+0x108`), indexed by `[this+0x1d4]`.
    pub attachments: &'a [AttachmentRecord],
    /// The bone matrices (`[parent+0x94]`, stride 0x40), indexed by `word[rec+4]`.
    pub palette: &'a [[f32; 16]],
}

/// Which path called `Animate`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimateLeg {
    /// No parent: the identity transform and `[this+0x2c]+0x9c`.
    Identity,
    /// The composed attachment matrix.
    Compose,
    /// The parent's world matrix `[parent]+0xfc`.
    Simple,
}

/// One call of `Animate` (`0x714260`) with what `EnsureAnimated` computes and passes.
#[derive(Clone, Copy, Debug)]
pub struct AnimateCall {
    pub leg: AnimateLeg,
    /// `ecx`: the instance that is animated.
    pub instance: usize,
    /// The composed matrix on the compose path. `None` on the others, whose matrix is a plain
    /// reference (`[this+0x2c]+0x9c`, `[parent]+0xfc`).
    pub src_matrix: Option<[f32; 16]>,
    /// The alpha argument: 1.0 with no parent, else `[parent+0x19c]`. At the compose call `edx`
    /// is the parent (`0x714102` to `0x71414c`: `lea ecx,[edx+0x1ac]; add edx,0x1a0; ...; push
    /// [edx-4]`), so the colours are the parent's `+0x1a0` and `+0x1ac` and the alpha its
    /// `+0x19c`, as on the other path.
    pub alpha: Option<f32>,
}

/// `0x714000`: animates instance `idx` after its ancestors. `animate` receives each call of
/// `Animate`.
pub fn m2_bone_propagate(
    graph: &mut InstanceGraph,
    idx: usize,
    animate: &mut dyn FnMut(AnimateCall),
) {
    // Already animated in this frame (`0x714009`): nothing changes.
    if graph.instances[idx].frame == graph.scene_frame {
        return;
    }

    let parent = graph.instances[idx].parent;

    // The parent first (`0x714019`), or the identity call (`0x71402a`).
    match parent {
        Some(p) => m2_bone_propagate(graph, p, animate),
        None => animate(AnimateCall {
            leg: AnimateLeg::Identity,
            instance: idx,
            src_matrix: None,
            alpha: Some(1.0),
        }),
    }

    // The frame again (`0x71406e`).
    if graph.instances[idx].frame == graph.scene_frame {
        return;
    }

    // With a parent, `[this+0x10]` gates the whole stage: an instance that is not loaded makes no
    // call and goes to the copy. The parent's loaded flag and frame and the attachment index
    // only choose between the composed matrix and the parent's world matrix.
    if let Some(p) = parent {
        if graph.instances[idx].loaded != 0 {
            let compose = graph.instances[p].loaded != 0
                && graph.instances[p].frame == graph.scene_frame
                && graph.instances[idx].attachment_index != 0xffff;

            if compose {
                // The attachment record, its bone matrix, and the offset carried into the
                // translation (`0x7140ec` to `0x714149`).
                let rec = graph.attachments[graph.instances[idx].attachment_index as usize];
                let src = graph.palette[rec.bone as usize];
                let mut m = src;
                let f = f64::from;
                let (v0, v1, v2) = (f(rec.offset[0]), f(rec.offset[1]), f(rec.offset[2]));
                // Translation x is `m8·v2 + m4·v1 + m0·v0 + m12`, y and z the same on columns 1
                // and 2, each at 53 bits and narrowed once.
                m[12] = (f(src[8]) * v2 + f(src[4]) * v1 + f(src[0]) * v0 + f(src[12])) as f32;
                m[13] = (f(src[9]) * v2 + f(src[5]) * v1 + f(src[1]) * v0 + f(src[13])) as f32;
                m[14] = (f(src[10]) * v2 + f(src[6]) * v1 + f(src[2]) * v0 + f(src[14])) as f32;
                animate(AnimateCall {
                    leg: AnimateLeg::Compose,
                    instance: idx,
                    src_matrix: Some(m),
                    // `[edx-4]` with `edx = parent + 0x1a0` is `[parent+0x19c]`.
                    alpha: Some(graph.instances[p].alpha),
                });
            } else {
                // The parent's world matrix `[parent]+0xfc` and alpha `[parent+0x19c]`.
                animate(AnimateCall {
                    leg: AnimateLeg::Simple,
                    instance: idx,
                    src_matrix: None,
                    alpha: Some(graph.instances[p].alpha),
                });
            }

            // The frame test after the call (`0x714183`) is reached only from the two call paths;
            // the skips at `0x714085` and `0x714090` jump to the copy at `0x71418e`. `Animate`
            // stores the scene's frame in `[this+0x40]` at its normal exit
            // (`systems/animation/kernel.md#animate-gates`), so after a call this returns without
            // the copy. The test's `Animate` leaves the frame as it was.
            if graph.instances[idx].frame == graph.scene_frame {
                return;
            }
        }
    }

    // The world matrix (`0x71418e`).
    match parent {
        // With a parent: a copy of its world matrix (16 dwords).
        Some(p) => {
            let pw = graph.instances[p].world_matrix;
            graph.instances[idx].world_matrix = pw;
        }
        // Without one: `0x7bc6a0` with `edx = [this+0xbc]` and `[ebp+8] = [this+0x2c]+0x9c`.
        None => {
            let base = graph.instances[idx].base_matrix;
            graph.instances[idx].world_matrix = multiply_matrix4(&base, &graph.scene_matrix);
        }
    }
}
