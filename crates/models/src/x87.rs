//! The x87 conversions the models code stores through.

pub(crate) use benilla_re_cmath::x87::{fistp, ftol};

/// `floor` (`0x73ff3f`) and then `__ftol`, the pair the liquid grid maps a coordinate to a cell
/// with.
#[inline]
pub(crate) fn floor_ftol(v: f32) -> i32 {
    ftol(f64::from(v).floor())
}
