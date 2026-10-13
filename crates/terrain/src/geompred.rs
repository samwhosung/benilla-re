//! Three tests and builders with their own arithmetic: `BuildFootprintBox` (`0x69a200`),
//! `DrawMapEntity` (`0x6c0cc0`) and `TraceWorldPlanes` (`0x6aa160`,
//! `systems/terrain/collide.md#other-world-queries`).

/// `BuildFootprintBox` (`0x69a200`, ecx = out, edx = the extent, the centre on the stack): `h =
/// 0.5·√(ex² + ey²)` kept in the register, and the box `[cx − h, cy − h, cz − 0.3, cx + h, cy + h,
/// cz + 0.3]`, 0.3 the `f32` at `[0x7ffd78]` and 0.5 `[0x7ffa24]`.
pub fn decal_query_box(center: [f32; 3], extent_xy: [f32; 2]) -> [f32; 6] {
    let ex = f64::from(extent_xy[0]);
    let ey = f64::from(extent_xy[1]);
    let half = 0.5_f64 * (ex * ex + ey * ey).sqrt();
    let z_ext = f64::from(0.3_f32);
    let cx = f64::from(center[0]);
    let cy = f64::from(center[1]);
    let cz = f64::from(center[2]);
    [
        (cx - half) as f32,
        (cy - half) as f32,
        (cz - z_ext) as f32,
        (cx + half) as f32,
        (cy + half) as f32,
        (cz + z_ext) as f32,
    ]
}

/// `DrawMapEntity` (`0x6c0cc0`) from `0x6c0cce`: after the first virtual call, `[esi+0x78] −
/// [esi+0x68]` against `[0xc62964]`, `test ah,5; jp 0x6c0ce9`. The jump skips the second virtual
/// call (`[vtbl+8]`) on an even count of C0 and C2, so the call is made only when the difference is
/// below the threshold; equal, above and NaN skip it. True when the call is made.
pub fn vtable_pred_dispatch_second(field_78: f32, field_68: f32, threshold: f32) -> bool {
    let diff = f64::from(field_78) - f64::from(field_68);
    let thr = f64::from(threshold);
    let unordered = diff.is_nan() || thr.is_nan();
    let c0 = (diff < thr) || unordered;
    let c2 = unordered;
    (u8::from(c0) + u8::from(c2)) % 2 == 1
}

/// `TraceWorldPlanes` (`0x6aa160`, ecx = the segment, start then end): its length `√((dz² + dx²) +
/// dy²)` at 53 bits against 0.01 (`[0x8029d0]`), `test ah,5; jp 0x6aa1ad`. The jump goes on to the
/// traces on an even count of C0 and C2: a length at or above 0.01, or NaN, goes on; a shorter
/// segment returns 0 at once. True when it goes on.
pub fn facet_seg_gate_proceeds(a: [f32; 3], b: [f32; 3]) -> bool {
    let dx = f64::from(b[0]) - f64::from(a[0]);
    let dy = f64::from(b[1]) - f64::from(a[1]);
    let dz = f64::from(b[2]) - f64::from(a[2]);
    let len = ((dz * dz + dx * dx) + dy * dy).sqrt();
    let thr = f64::from(0.01_f32);
    let unordered = len.is_nan();
    let c0 = (len < thr) || unordered;
    let c2 = unordered;
    (u8::from(c0) + u8::from(c2)) % 2 == 0
}
