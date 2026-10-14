//! Scale and visibility propagation of `CSimpleFrame` (`systems/ui/visibility.md#scale`,
//! `#showing-a-frame`, `#hiding-a-frame`): the result each of three kernels computes for a frame.
//!
//! Each runs `__thiscall` on the frame. What the binary does after that, writing the value,
//! moving the frame between strata levels, dirtying the draw layers, recursing into the region list
//! (`+0x1b8`) and the frame list (`+0x300`) and calling `OnShow` or `OnHide`, is not part of the
//! value computed here.
//!
//! The order of those steps is part of the behaviour. The flag is written first, before the child
//! walks, so each child sees a visible parent; then the two child walks; and the frame's own
//! `OnShow` or `OnHide` last. The cascade is post-order: a descendant's script runs before its
//! ancestor's. The walks run over live links, with no snapshot (no `sub esp, N`, no `rep movs`), and
//! `next` is read from the current node after the per-node call returns.
//!
//! The scale kernel runs in `f64`: the product of two `f32` is exact in a 53-bit mantissa, so the
//! one `as f32` at the binary's `fstp dword` is the only rounding. The visibility kernels are
//! integer and boolean.

/// 2^-22, the `f32` at `0x8029d4`: the scale write and the recursion are skipped when the new scale
/// is within this of the current one. The same cell gates `OnFrameSizeChanged`.
const SCALE_EPS: f64 = f32::from_bits(0x3480_0000) as f64;

/// What [`effective_scale`] decides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectiveScale {
    /// `parent_scale * own_scale`, stored as `f32` (`fstp [ebp-4]`); always computed, written and
    /// propagated only when `apply`.
    pub new_scale: f32,
    /// Whether the change is applied: the scale is written to the layout scale (`+0x7c`) through
    /// `SetLayoutScale` (`0x768360`) and handed to the child lists. False is the `xor al, al; ret`
    /// of the within-epsilon exit; this is the `eax` return.
    pub apply: bool,
}

/// `CSimpleFrame::UpdateScale` (`0x76ac90`, `visibility#scale`, the argument `force`): the
/// effective scale `parent_scale (+0xbc) * own_scale (+0xb8)` is stored as `f32` and handed to
/// `SetLayoutScale` and the children (`0x76ac50`, with `(new, force)`). `force` (the stack argument,
/// tested as a byte) skips the check. Without it the function skips when `|new - cur| < 2^-22`
/// (`fld layoutScale (+0x7c); fsubr new; fabs; fcomp eps; test ah, 5; jp`) and applies for a
/// difference of 2^-22 or more, for equality of the two, and for a NaN. Returns 1 on apply and 0 on
/// the short exit.
pub fn effective_scale(
    parent_scale: f32,
    own_scale: f32,
    cur_scale: f32,
    force: bool,
) -> EffectiveScale {
    let new_scale = (f64::from(parent_scale) * f64::from(own_scale)) as f32;
    // `<` is false for a NaN difference, so a NaN applies.
    let within_eps = (f64::from(new_scale) - f64::from(cur_scale)).abs() < SCALE_EPS;
    EffectiveScale {
        new_scale,
        apply: force || !within_eps,
    }
}

/// What a visibility transition decides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectiveVisible {
    /// The function's `eax`.
    pub eax: u32,
    /// `effectiveVisible (+0xd4)` afterwards.
    pub effective: u32,
    /// Whether the transition path runs: the move between strata levels, the layer dirtying, the
    /// child recursion and `OnShow` or `OnHide`. False means the frame was already in the target
    /// state.
    pub transition: bool,
}

/// `SimpleFrameShowNow` (`0x76ae10`, `visibility#showing-a-frame`): recomputes the effective
/// visibility under the shown rule. `new_visible = shown (+0xd0) && (parent (+0x9c) == 0 ||
/// parent.effectiveVisible (+0xd4))`. A false result returns 0 and leaves `+0xd4` alone. Otherwise
/// a frame already visible returns 1 with no transition, and a hidden one writes `+0xd4 = 1` and
/// makes the transition: it joins its stratum level (`0x764780`), dirties the draw layers, recurses
/// into the lists and calls `OnShow` (primary vtable `+0x30`). Returns `eax = new_visible`.
///
/// `parent` is `None` when `parent (+0x9c) == 0`, a frame anchored to the screen root, which counts
/// as visible, else `Some(parent.effectiveVisible != 0)`.
pub fn effective_visible_show(
    shown: bool,
    parent: Option<bool>,
    cur_effective: u32,
) -> EffectiveVisible {
    let new_visible = shown && parent.unwrap_or(true);
    if !new_visible {
        return EffectiveVisible {
            eax: 0,
            effective: cur_effective,
            transition: false,
        };
    }
    let transition = cur_effective == 0;
    EffectiveVisible {
        eax: 1,
        effective: if transition { 1 } else { cur_effective },
        transition,
    }
}

/// `CSimpleFrame::Hide` (`0x76ad50`, `visibility#hiding-a-frame`): forces the frame effectively
/// hidden. It does not recompute from `shown` and the parent: it is the path for a cleared `shown`
/// or a hidden parent. If `+0xd4 != 0` it writes `+0xd4 = 0` and makes the transition: it leaves its
/// stratum level (`0x764920`), recurses into the lists and calls `OnHide` (primary vtable `+0x34`);
/// if it is already 0 nothing happens. `+0xd4` ends at 0 either way, and the function returns 1.
pub fn effective_visible_hide(cur_effective: u32) -> EffectiveVisible {
    EffectiveVisible {
        eax: 1,
        effective: 0,
        transition: cur_effective != 0,
    }
}
