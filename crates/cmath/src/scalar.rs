//! The scalar file's integer helper: the high dword of an unsigned 32 by 32 multiply.

/// `MulHigh32` (`0x455c70`): `mov eax,ecx; mul edx; mov eax,edx; ret`, the high 32 bits of the
/// unsigned 64-bit product of `a` and `b`.
pub fn mul_high32(a: u32, b: u32) -> u32 {
    ((u64::from(a) * u64::from(b)) >> 32) as u32
}
