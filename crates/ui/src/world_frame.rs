//! The vector and scalar arithmetic inside the world frame's pick, light, fade, render and reticle
//! functions (`0x482200` to `0x4837b0`): squared lengths, a normalize, vec3 sums and differences, an
//! affine corner combine, change tests and a few scales. Each function says which slice of which
//! function it is. The x87 keeps a sum live as an f64 on the stack, so only the `fstp dword` stores
//! narrow to `f32`.
//! Entries: `systems/ui/worldcursor.md`, `systems/ui/worldpick.md`, `systems/ui/idle.md`,
//! `systems/ui/groundreticle.md`, `systems/lighting/daynight.md#light-state`.

/// `0x801360`: 0.001 as an `f32`, the tick scale.
const DT: f32 = f32::from_bits(0x3a83_126f);
/// `0x7ff9d8`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);
/// `0x8026c8`: 1/255 as an `f32`.
const INV255: f32 = f32::from_bits(0x3b80_8081);
/// `0x8022cc`: 1000.0.
const SCALE_1000: f32 = f32::from_bits(0x447a_0000);
/// `0x7ffa24`: 0.5.
const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `0x801628`: 2.0, the half-width of the reticle's third component.
const TWO: f32 = f32::from_bits(0x4000_0000);

/// `a*a + b*b + c*c` at 53 bits, stored as an `f32`: the squared length of
/// `ClassifyHoveredUnit` (`0x4822f7` to `0x48230a`), of `ClassifyHoveredCorpse` (`0x4827af` to
/// `0x4827c7`) and of `StampLightState` (`0x482b57` to `0x482b67`).
pub fn sq_len3(a: f32, b: f32, c: f32) -> f32 {
    sq_len3_f64(a, b, c) as f32
}

/// The same sum as the f64 that `StampLightState`'s `fsqrt` reads, not narrowed.
pub fn sq_len3_f64(a: f32, b: f32, c: f32) -> f64 {
    let (a, b, c) = (f64::from(a), f64::from(b), f64::from(c));
    a * a + b * b + c * c
}

/// `StampLightState` (`0x482ad0`, `0x482b57` to `0x482b86`): the vector divided by its length. The
/// reciprocal `1.0 / sqrt(sum)` (`fdivr` reverses the divide) stays an f64 across the three
/// multiplies, and each product is stored as an `f32`.
pub fn normalize3(a: f32, b: f32, c: f32) -> [f32; 3] {
    let len = sq_len3_f64(a, b, c).sqrt();
    let recip = f64::from(ONE) / len;
    [a, b, c].map(|v| (f64::from(v) * recip) as f32)
}

/// `v * v` at 53 bits, left on the stack for the `fcomp` against the squared range
/// (`ClassifyHoveredUnit` `0x4825cf` to `0x4825db`, `ClassifyHoveredCorpse` `0x482875` to `0x48287d`).
pub fn square_f64(v: f32) -> f64 {
    f64::from(v) * f64::from(v)
}

/// The per-component difference `a - b` of `ClassifyHoveredCorpse` (`0x482789` to `0x4827a7`), each
/// stored as an `f32`.
pub fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|i| (f64::from(a[i]) - f64::from(b[i])) as f32)
}

/// The per-component sum `v + w` of `CGWorldFrame::OnRender` (`0x482ea0`, `0x4830be` to `0x4830e0`),
/// each stored as an `f32`.
pub fn add3(v: [f32; 3], w: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|i| (f64::from(v[i]) + f64::from(w[i])) as f32)
}

/// `StampLightState` (`0x482b02` to `0x482b0e`): a tick count `{n, 0}` by `fild qword` times 0.001,
/// stored as an `f32`.
pub fn tick_scale(n: u32) -> f32 {
    (f64::from(n) * f64::from(DT)) as f32
}

/// `ApplySelfFade` (`0x482c10`, `0x482c79` to `0x482c85`): a flag byte, 0 or 1, times the `f32` 1/255,
/// stored as an `f32`.
pub fn bool_inv255(b: bool) -> f32 {
    (f64::from(u8::from(b)) * f64::from(INV255)) as f32
}

/// `CGWorldFrame::OnRender` (`0x4831d0` to `0x4831eb`): `a * b + acc`, stored as an `f32`.
pub fn mul_add(a: f32, b: f32, acc: f32) -> f32 {
    ((f64::from(a) * f64::from(b)) + f64::from(acc)) as f32
}

/// The function at `0x483430` (`0x48343a` to `0x48344b`): an angle from a virtual call becomes
/// `fcos` into `+0`, `fsin` into `+4` and 0.0 into `+8`. The x87 transcendentals differ by CPU, so a
/// test compares with the host's functions structurally (`METHOD.md#the-harness`).
pub fn cos_sin(angle: f32) -> [f32; 3] {
    let a = f64::from(angle);
    [a.cos() as f32, a.sin() as f32, 0.0]
}

/// The function at `0x483340` (`0x4833c5` to `0x4833ea`): `sel * (m94 * m90)` stored as an `f32`,
/// where `sel` is `+0x9c` when the flag at `+0xdc` is set and `+0x98` otherwise. `sel` stays on
/// `st1` until the last multiply.
pub fn gated_triple(sel: f32, m94: f32, m90: f32) -> f32 {
    (f64::from(sel) * (f64::from(m94) * f64::from(m90))) as f32
}

/// `RenderWorldFrame` (`0x483460`, `0x483499` to `0x4834c1`): `fld [a]; fsub [b]; fcomp 0.0;
/// test ah,0x44; jnp`. The mask is C2 and C3, `jnp` is taken for an odd count, so it is taken only
/// when `a - b` is zero and ordered; a NaN sets both bits and falls through.
pub fn delta_is_zero(a: f32, b: f32) -> bool {
    f64::from(a) - f64::from(b) == 0.0
}

/// `RenderWorldFrame` (`0x4834c7` to `0x4834ef`): `fcomp` of `[edi+0x3a0]` against `[edi+0x3a8]`
/// with `test ah,5; jp`: taken for an even count of C0 and C2, so for `a >= b` and for NaN.
pub fn cmp_ge_or_nan(a: f32, b: f32) -> bool {
    use core::cmp::Ordering;
    !matches!(
        f64::from(a).partial_cmp(&f64::from(b)),
        Some(Ordering::Less)
    )
}

/// `RenderWorldFrame` (`0x48357a` to `0x4835f2`): four affine outputs `delta * frac + base` where the
/// deltas `dx = ax - ax0` and `dy = ay - ay0` stay f64 on the stack across their legs and the
/// fractions and bases are `f32`. `flag_set` swaps the pairing of the y fractions: set (`0x48358d`)
/// they are `1.0 - field_a` and `1.0 - field_b`, clear (`0x4835a7`) `field_b` and `field_a`. Returns
/// `[leg1 * dy + by, leg2 * dy + by, g3 * dx + bx, g4 * dx + bx]`, the stores at `[esp+0xc]`,
/// `[esp+8]`, `[esp+4]` and `[esp]`.
#[allow(clippy::too_many_arguments)]
pub fn affine_combine(
    ax: f32,
    ax0: f32,
    ay: f32,
    ay0: f32,
    field_a: f32,
    field_b: f32,
    g3: f32,
    g4: f32,
    bx: f32,
    by: f32,
    flag_set: bool,
) -> [f32; 4] {
    let dx = f64::from(ax) - f64::from(ax0);
    let dy = f64::from(ay) - f64::from(ay0);
    let one = f64::from(ONE);
    let (frac1, frac2) = if flag_set {
        (one - f64::from(field_a), one - f64::from(field_b))
    } else {
        (f64::from(field_b), f64::from(field_a))
    };
    let (by, bx) = (f64::from(by), f64::from(bx));
    [
        (frac1 * dy + by) as f32,
        (frac2 * dy + by) as f32,
        (f64::from(g3) * dx + bx) as f32,
        (f64::from(g4) * dx + bx) as f32,
    ]
}

/// `RenderWorldFrame` (`0x483659` to `0x483671`): `field * 1000.0 + 0.5`, truncated by `__ftol`.
pub fn unit_convert_round(field: f32) -> i32 {
    let v = f64::from(field) * f64::from(SCALE_1000) + f64::from(HALF);
    v.trunc() as i32
}

/// `RenderWorldFrame` (`0x48368f` to `0x48369a`): `fld [0xc7b4a0]; fchs; fstp [esp]`.
pub fn negate(v: f32) -> f32 {
    (-f64::from(v)) as f32
}

/// The ground reticle's corner build (`0x4837b0`, `0x48387e` to `0x4838e7`): from a centre and a
/// half-width that is the argument for x and y and the constant 2.0 (`0x801628`) for z, `plus` adds
/// and `minus` subtracts, each component stored as an `f32`.
pub fn corner_build(center: [f32; 3], hw01: f32) -> ([f32; 3], [f32; 3]) {
    let hw = [hw01, hw01, TWO];
    (
        [0, 1, 2].map(|i| (f64::from(center[i]) + f64::from(hw[i])) as f32),
        [0, 1, 2].map(|i| (f64::from(center[i]) - f64::from(hw[i])) as f32),
    )
}
