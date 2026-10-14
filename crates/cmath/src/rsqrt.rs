//! The reciprocal square root estimate `0x456330` (`systems/lighting/clouds.md#cloud-colours`
//! reaches it from `ColourCloudBand`).

/// `0x456330` RsqrtEstimate (`ecx` = `magic`, the `f32` on the stack, `ret 4`): the float whose
/// bits are `magic` less the argument's bits shifted right by one and masked with `0x3fffffff`. The
/// callers pass `0x5f3997bb`. Integer operations only, so exact, and it takes any bits.
#[must_use]
pub fn rsqrt_estimate(magic: u32, x: f32) -> f32 {
    let shifted = (x.to_bits() >> 1) & 0x3fff_ffff;
    f32::from_bits(magic.wrapping_sub(shifted))
}
