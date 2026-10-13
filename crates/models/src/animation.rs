//! A model's animation (`systems/models/animation.md`): the fade and key timing of
//! `CM2Model::PlayAnimation` (`0x7121a0`), the key bone timing of `CM2Model::SetKeyBoneAnimTime`
//! (`0x7127f0`) and `CM2Model::SetKeyBoneAnimSpeed` (`0x712910`), the event windows of
//! `CM2Model::Update` (`0x719370`) and the transforms of `CM2Model::UpdateAfterAnimate`
//! (`0x718960`).

use crate::x87::ftol;

/// 9.9999997e-6 (`0x80c5c8`): the least rate a key bone divides by.
pub const KEYFRAME_EPS: f32 = f32::from_bits(0x3727_c5ac);
/// 0.001 (`0x801360`): `0x718960`'s count scale.
pub const COUNT_SCALE: f32 = f32::from_bits(0x3a83_126f);
/// π (`0x81203c`).
pub const PI: f32 = f32::from_bits(0x4049_0fdb);

/// `0x7121a0` from `0x712352` to `0x7123a2` (and its copy from `0x712579`;
/// `systems/models/animation.md#playing-an-animation`): the blend slot's fade. `t = n·rate` is
/// stored as `f32` by `fst`; below 0 (the unrounded value, ordered, `jp` at `0x712369`) the fade is
/// 0; above 1 (the stored value, `jne` at `0x712381`) it is 1; otherwise, NaN included, `(3 −
/// 2t)·t·t` from the stored `t`. Times `weight`, left in the register for [`anim_fade_resets`].
pub fn anim_fade(n: i32, rate: f32, weight: f32) -> f64 {
    let live = f64::from(n) * f64::from(rate);
    let t = f64::from(live as f32);
    let pre = if live < 0.0 {
        0.0
    } else if t > 1.0 {
        1.0
    } else {
        (3.0 - (t + t)) * t * t
    };
    pre * f64::from(weight)
}

/// `0x7121a0` at `0x7123a2` and `0x7125c9`: the blend slot is copied over the primary when the fade
/// is at most 0.5 (`test ah,0x41; jp`, so a NaN keeps it).
pub fn anim_fade_resets(fade: f64) -> bool {
    fade <= 0.5
}

/// `0x7121a0` from `0x712718` to `0x712739`: `1/v` when `|v|` is above 9.9999997e-6 (`test
/// ah,0x41; jne`, so equal or NaN gives 0), stored as `f32`.
pub fn keyframe_ratio(p: f32) -> f32 {
    keyframe_ratio_live(f64::from(p)) as f32
}

/// [`keyframe_ratio`] as the register holds it.
fn keyframe_ratio_live(g: f64) -> f64 {
    if g.abs() > f64::from(KEYFRAME_EPS) {
        1.0 / g
    } else {
        0.0
    }
}

/// `0x7121a0` from `0x71260c` to `0x712623` (and `0x71266b`): `1/count` with the count loaded as a
/// 64-bit integer, stored as `f32` at `+0x104`.
pub fn rate_reciprocal(count: u32) -> f32 {
    (1.0 / f64::from(count)) as f32
}

/// What [`keyframe_timing`] stores at the key bone's `+0x10` to `+0x1c`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyframeTiming {
    /// `+0x18`: the speed.
    pub value: f32,
    /// `+0x1c`: its rate, [`keyframe_ratio`].
    pub ratio: f32,
    /// `+0x10`: `now − __ftol(offset·|rate|)`.
    pub start: i32,
    /// `+0x14`: `__ftol(length·|rate|) + start`.
    pub end: i32,
}

/// `0x7121a0` from `0x7126d2` to `0x71279c` (`systems/models/animation.md#playing-an-animation`):
/// the speed `param6`, negated for types 1 and 3 and zero for types 2 and 3; its rate in the
/// register; and the stamps from `|rate|` (unrounded) times the offset `i14` and times the length
/// `q` (a 64-bit load of a dword, so unsigned).
pub fn keyframe_timing(
    param6: f32,
    type16: i16,
    i14: i32,
    q: u32,
    frametime: i32,
) -> KeyframeTiming {
    let mut g = f64::from(param6);
    if type16 == 1 || type16 == 3 {
        g = -g;
    }
    if type16 == 2 || type16 == 3 {
        g = 0.0;
    }
    let ratio = keyframe_ratio_live(g);
    let abs_r = ratio.abs();
    let start = frametime.wrapping_sub(ftol(f64::from(i14) * abs_r));
    let end = ftol(f64::from(q) * abs_r).wrapping_add(start);
    KeyframeTiming {
        value: g as f32,
        ratio: ratio as f32,
        start,
        end,
    }
}

/// What [`m2_set_bone_timed`] stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoneTiming {
    /// `+0xa8`.
    pub a8: i32,
    /// `+0xac`.
    pub ac: i32,
}

/// `0x7127f0` (`systems/models/model.md#deferred-calls`) on a key bone that plays: with `s =
/// |+0xb4|` in the register, `+0xa8 = now − __ftol(frame·s)` and `+0xac = __ftol(span·s) + +0xa8`,
/// where `span` is the 32-bit product `(end − start)·+0xbc` loaded as an unsigned 64-bit integer.
pub fn m2_set_bone_timed(
    scale_f32: f32,
    frame: i32,
    base: i32,
    span0: i32,
    stride: i32,
) -> BoneTiming {
    let scale = f64::from(scale_f32).abs();
    let a8 = base.wrapping_sub(ftol(f64::from(frame) * scale));
    let span = f64::from(span0.wrapping_mul(stride) as u32);
    BoneTiming {
        a8,
        ac: ftol(span * scale).wrapping_add(a8),
    }
}

/// What [`bone_timed_value`] stores at `+0xa8` to `+0xb4`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneTimedValue {
    pub start: i32,
    pub end: i32,
    /// `+0xb0`: the speed copied.
    pub param_bits: u32,
    /// `+0xb4`: the rate.
    pub ratio: f32,
}

/// `0x712910` from `0x7129db` to `0x712a7c` (`systems/models/model.md#deferred-calls`): the key
/// bone's frame at the old speed, `k = __ftol((now − +0xa8)·+0xb0) + +0xb8`; the new rate `1/p`
/// when `|p|` is above 9.9999997e-6 (`test ah,0x41; jne`, so equal or NaN gives 0), in the
/// register; then `+0xa8 = now − __ftol(k·|rate|)` and `+0xac = __ftol(span·|rate|) + +0xa8`,
/// `span` the 32-bit `(t8 − t4)·bc` loaded unsigned.
#[allow(clippy::too_many_arguments)]
pub fn bone_timed_value(
    time: i32,
    a0: i32,
    scale: f32,
    b8: i32,
    bc: i32,
    t4: i32,
    t8: i32,
    p: f32,
) -> BoneTimedValue {
    let k0b = ftol(f64::from(time.wrapping_sub(a0)) * f64::from(scale)).wrapping_add(b8);
    let span = f64::from(t8.wrapping_sub(t4).wrapping_mul(bc) as u32);
    let ratio = keyframe_ratio_live(f64::from(p));
    let abs_ratio = ratio.abs();
    let start = time.wrapping_sub(ftol(f64::from(k0b) * abs_ratio));
    BoneTimedValue {
        start,
        end: ftol(span * abs_ratio).wrapping_add(start),
        param_bits: p.to_bits(),
        ratio: ratio as f32,
    }
}

/// `0x719370` from `0x71952c` and from `0x719554`
/// (`systems/models/animation.md#update-and-events`): a window bound in the bone's frames,
/// `__ftol((bound − +0xa8)·+0xb0) + +0xb8`.
pub fn bone_emit_frame(bound: i32, a8: i32, scale: f32, b8: i32) -> i32 {
    ftol(f64::from(bound.wrapping_sub(a8)) * f64::from(scale)).wrapping_add(b8)
}

/// `0x719370` from `0x719724` and from `0x719844`: `__ftol(|+0xb4|·n)`, `n` a signed
/// time difference (`fimul dword`).
pub fn abs_int_scale(f: f32, n: i32) -> i32 {
    ftol(f64::from(f).abs() * f64::from(n))
}

/// `(a0·b0 + a1·b1) + a2·b2` in the register.
fn dot3(a0: f32, b0: f32, a1: f32, b1: f32, a2: f32, b2: f32) -> f64 {
    (f64::from(a0) * f64::from(b0) + f64::from(a1) * f64::from(b1)) + f64::from(a2) * f64::from(b2)
}

/// `0x718960` from `0x718a7e` to `0x718ae4` (`systems/models/animation.md#pushing-animated-state`):
/// a camera's offset negated and taken through the scene matrix's rotation, stored to `[ebp-0x34]`,
/// `[ebp-0x30]` and `[ebp-0x2c]`: `(vz·m8 + vy·m4) + vx·m0`, `(vx·m1 + vz·m9) + vy·m5`, `(vx·m2 +
/// vz·m10) + vy·m6`, each stored as `f32`.
pub fn camera_xform(src: [f32; 3], m: &[f32; 16]) -> [f32; 3] {
    let [vx, vy, vz] = src.map(|c| -c);
    [
        dot3(vz, m[8], vy, m[4], vx, m[0]) as f32,
        dot3(vx, m[1], vz, m[9], vy, m[5]) as f32,
        dot3(vx, m[2], vz, m[10], vy, m[6]) as f32,
    ]
}

/// `0x718960` from `0x718b66` and `0x718bcc`: a light's position and colours plus the base, each
/// `fld; fadd; fstp`.
pub fn add2(a: f32, b: f32) -> f32 {
    a + b
}

/// `0x718960` from `0x718c78` to `0x718c92`: the count `+0x88` loaded as a 64-bit integer times
/// 0.001, stored as `f32`.
pub fn count_scale(count: i64) -> f32 {
    (count as f64 * f64::from(COUNT_SCALE)) as f32
}

/// `0x718960` from `0x718cf0` to `0x718cfa`: a ribbon's `+0x3c` times the instance's `+0x19c`.
pub fn field_scale(field: f32, gscale: f32) -> f32 {
    field * gscale
}

/// `0x718960` from `0x718d6d` to `0x718dd0`: a ribbon's offset through its bone matrix, rows
/// `((m4·vy + m0·vx) + m8·vz) + t0`, `((m1·vx + m9·vz) + m5·vy) + t1`,
/// `((m2·vx + m10·vz) + m6·vy) + t2`, each stored as `f32`.
pub fn ribbon_bone_xform(m: &[f32; 16], v: [f32; 3], t: [f32; 3]) -> [f32; 3] {
    let [vx, vy, vz] = v;
    [
        (dot3(m[4], vy, m[0], vx, m[8], vz) + f64::from(t[0])) as f32,
        (dot3(m[1], vx, m[9], vz, m[5], vy) + f64::from(t[1])) as f32,
        (dot3(m[2], vx, m[10], vz, m[6], vy) + f64::from(t[2])) as f32,
    ]
}

/// `0x718960` from `0x7190a9` to `0x71910c`: an emitter's offset through its bone matrix, as
/// [`ribbon_bone_xform`] with the first row `((m0·vx + m8·vz) + m4·vy) + t0`.
pub fn particle_bone_xform(m: &[f32; 16], v: [f32; 3], t: [f32; 3]) -> [f32; 3] {
    let [vx, vy, vz] = v;
    [
        (dot3(m[0], vx, m[8], vz, m[4], vy) + f64::from(t[0])) as f32,
        (dot3(m[1], vx, m[9], vz, m[5], vy) + f64::from(t[1])) as f32,
        (dot3(m[2], vx, m[10], vz, m[6], vy) + f64::from(t[2])) as f32,
    ]
}

/// `0x718960` from `0x719114` to `0x71912a`: π times 0.5, stored as `f32`.
pub fn half_pi() -> f32 {
    PI * 0.5
}
