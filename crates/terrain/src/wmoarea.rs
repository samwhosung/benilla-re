//! The WMOAreaTable lookup's comparator (`0x7ccdb0`).

/// The order the binary search over WMOAreaTable rows uses (`0x7ccdb0`): the WMO id at `+0x4`,
/// then the name set at `+0x8`, then the group id at `+0xc`, each compared by its 32-bit
/// difference `a - b`, which wraps. The first nonzero difference is the result, else 0.
pub fn wmo_area_compare(a: [i32; 3], b: [i32; 3]) -> i32 {
    let d = a[0].wrapping_sub(b[0]);
    if d != 0 {
        return d;
    }
    let d = a[1].wrapping_sub(b[1]);
    if d != 0 {
        return d;
    }
    a[2].wrapping_sub(b[2])
}
