//! `BoxContainsTriangle` (`0x6b8b70`).

/// `0x6b8b70` (ecx = the box, minimum then maximum; edx, then the stack, the three points; `ret 8`):
/// true when the three points lie outside one side of the box. Per axis it ANDs the sign bits of
/// the stored `f32(max − p)` and of the stored `f32(p − min)` over the points; either AND nonzero
/// returns 1.
pub fn aabb_points3_separated(aabb: &[f32; 6], points: [&[f32; 3]; 3]) -> bool {
    let min = [aabb[0], aabb[1], aabb[2]];
    let max = [aabb[3], aabb[4], aabb[5]];
    for c in 0..3 {
        let mut all_above = 0x8000_0000u32; // edi
        let mut all_below = 0x8000_0000u32; // ebx
        for p in points.iter() {
            let max_minus_p = (f64::from(max[c]) - f64::from(p[c])) as f32;
            let p_minus_min = (f64::from(p[c]) - f64::from(min[c])) as f32;
            all_above &= max_minus_p.to_bits() & 0x8000_0000;
            all_below &= p_minus_min.to_bits() & 0x8000_0000;
        }
        if all_above != 0 || all_below != 0 {
            return true;
        }
    }
    false
}
