//! `0x6c73f0`'s floating-point slice — the **world-text anchor-lift**.
//!
//! In `SubmitWorldText`'s real-add path (`0x6c73f0`), after the unit position is fetched
//! (`0x608640`, delegated), the world-text anchor's Z is dropped by one-third of a unit before the
//! WORLDTEXTSTRING is constructed — so the floating text/name anchors just below the unit's origin:
//!
//! ```text
//!   6c743e: d9 45 fc          fld   dword [ebp-0x4]        ; z (f32, widened to PC_53 f64 on st0)
//!   6c7444: d8 25 40 7a 80 00 fsub  dword [0x807a40]       ; - (1/3 as f32, widened)
//!   6c7451: d9 5d fc          fstp  dword [ebp-0x4]        ; z' stored back as f32
//! ```
//!
//! The constant at `0x807a40` is the **f32** `0x3eaaaaab` (`0.333333343267…`, the nearest f32 to 1/3),
//! widened exactly to f64 by the `fsub dword`. Both operands are f32 widened to PC_53 f64 (FPCW=0x027F,
//! the harness invariant); the result is rounded to f32 at the `fstp dword`. So `z' = (f64(z) - f64(1/3_f32))
//! as f32`. `z` is the unit-position Z. Returns the lifted anchor Z as the stored f32.
#![allow(clippy::float_cmp)]

/// `0x807a40` (`.rdata`) — the anchor-lift offset, the **f32** nearest to `1/3` (`0x3eaaaaab`).
pub const THIRD_F32_BITS: u32 = 0x3eaa_aaab;

#[inline]
fn third() -> f64 {
    f64::from(f32::from_bits(THIRD_F32_BITS)) // fsub dword: f32 operand widened exactly to f64
}

/// `0x6c73f0` anchor-lift: `z' = z - 1/3` (PC_53 f64 subtract of the two widened f32 operands,
/// narrowed to f32 at the store). `z` is the unit-position Z (f32). Returns the lifted f32.
pub fn anchor_lift(z: f32) -> f32 {
    (f64::from(z) - third()) as f32 // fld dword; fsub dword [1/3 f32]; fstp dword
}
