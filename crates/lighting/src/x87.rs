//! The x87 conversions the lighting code stores through.

pub(crate) use benilla_re_cmath::x87::{fistp, ftol};

/// `floor` (`0x73ff3f`) then `__ftol`: round down, then the low dword of the `fistp qword`.
#[inline]
pub(crate) fn ftol_floor(v: f64) -> i32 {
    ftol(v.floor())
}
