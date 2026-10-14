//! The `FontString` and `Font` methods whose arithmetic is more than a layout conversion
//! (`systems/ui/fontstring.md#lua-methods`): the guarded height setter and the setters that store
//! only when the new value differs from the field by more than an epsilon. The conversions and the
//! colour packs they share are in [`crate::ui_scale`] and [`crate::color`].

use crate::ui_scale::{logical_to_device, LOGICAL_BASE, LOGICAL_WIDTH, UI_SCALE};

/// `0x8029d4`, 2^-22 as an `f32`: below it `|new - old|` counts as unchanged.
const CHANGE_EPS: f32 = f32::from_bits(0x3480_0000);

/// `FontString::SetTextHeight` (`0x79e3c0`): the converted height is stored only for a positive
/// script number. The number is `fst`-stored to an `f32` for the conversion, but the `fcomp` against
/// 0.0 reads the f64 still in `st0`, and `test ah,0x41; jp` is taken for a positive value and for NaN:
/// zero and every negative number take the error path. Returns `None` there.
pub fn set_text_height_guarded(lua: f64) -> Option<f32> {
    if lua > 0.0 || lua.is_nan() {
        Some(logical_to_device(lua as f32))
    } else {
        None
    }
}

/// What a setter with a change test stores and decides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetChange {
    /// The `f32` the field receives.
    pub stored: f32,
    /// Whether `|new - old| >= 2^-22` or NaN: `fcomp` against the epsilon and `test ah,5; jp` is taken
    /// for a difference that is not below it, and the store follows without consulting the widget
    /// flag at `+0x38`.
    pub changed: bool,
}

/// `|delta|` against the epsilon: `jp` is taken when it is not below it, NaN included.
fn delta_ge_eps(delta: f64) -> bool {
    delta >= f64::from(CHANGE_EPS) || delta.is_nan()
}

/// The setter at `0x79fb40` (field `+0x50`): the converted value stays an f64 on the stack, so the
/// change test subtracts the field from the unnarrowed product, and `fstp` narrows it into the field.
pub fn set_scaled_changed(arg: f32, old_field: f32) -> SetChange {
    let denom = f64::from(LOGICAL_BASE) * f64::from(LOGICAL_WIDTH);
    let q32 = (f64::from(arg) / denom) as f32;
    let new_live = f64::from(UI_SCALE) * f64::from(q32);
    SetChange {
        stored: new_live as f32,
        changed: delta_ge_eps((new_live - f64::from(old_field)).abs()),
    }
}

/// The setter at `0x79f210` (field `+0x48`): the product is stored as an `f32` first and the change
/// test reloads that `f32`.
pub fn set_scaled_changed_spilled(arg: f32, old_field: f32) -> SetChange {
    let new_f32 = logical_to_device(arg);
    SetChange {
        stored: new_f32,
        changed: delta_ge_eps((f64::from(new_f32) - f64::from(old_field)).abs()),
    }
}
