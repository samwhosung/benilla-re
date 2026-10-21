//! The squared-distance gates of the interface (`systems/ui/distancegates.md`): the range tests
//! scattered across the frame classes and the producers that keep a rounded squared distance.
//!
//! **The kernel.** Two `C3Vector` positions are differenced per axis, each difference squared, the
//! three squares summed and the sum compared with a squared threshold, in x87 registers at 53-bit
//! precision. A difference of two `f32` is exact in `f64`, a product of two 24-bit values is exact,
//! and the sum of three such squares is exact in the regimes the gates see, so the host kernel runs
//! in `f64`. The **order of the three squares** differs from site to site, so it is written out
//! per function. A few sites store a value to `f32` before they compare (`0x494200`, `0x4b0c30`,
//! `0x5116e0` round `dist^2` or its deltas); `0x4b1060` and `0x493d30` store a rounded key.
//!
//! **The status-word branch.** A gate ends with `fcom` or `fcompp`, `fnstsw ax`, `test ah, mask`
//! and a conditional jump. `fcom(a, b)` sets C0 (`0x01` in `ah`) when `a < b`, C3 (`0x40`) when
//! equal, and C0, C2 (`0x04`) and C3 together when either operand is a NaN. [`fcom`] reproduces
//! those bits and each gate applies its own mask, so the NaN leg matches the binary, which a plain
//! `<` or `==` would get wrong.

/// x87 `fcom(a, b)` condition codes `(C0, C2, C3)` as they land in `ah` (`0x01`, `0x04`, `0x40`).
/// `a` is the top-of-stack operand and `b` the compared one. A NaN in either sets all three.
#[inline]
pub fn fcom(a: f64, b: f64) -> (bool, bool, bool) {
    if a.is_nan() || b.is_nan() {
        (true, true, true)
    } else if a < b {
        (true, false, false)
    } else if a == b {
        (false, false, true)
    } else {
        (false, false, false)
    }
}

/// One squared axis difference, in `f64`.
#[inline]
fn sq(a: f32, b: f32) -> f64 {
    let d = f64::from(a) - f64::from(b);
    d * d
}

/// `dist^2` of two positions, summed as `(o0^2 + o1^2) + o2^2` where `o` lists the axes (0 = x,
/// 1 = y, 2 = z) in the order the call site's `fld`/`faddp` chain adds them.
#[inline]
fn d2(p: [f32; 3], q: [f32; 3], o: [usize; 3]) -> f64 {
    (sq(p[o[0]], q[o[0]]) + sq(p[o[1]], q[o[1]])) + sq(p[o[2]], q[o[2]])
}

// ---- the comparison gates: each returns the condition its action is taken on ----

/// The gate in `0x48edf0` (`uicamera/cinematic#playing-a-shot`): the area is prepared (`0x66fc80`)
/// when `dist^2 > 30^2`. `p` is the camera's old position (`+8/+0xc/+0x10`) and `q` the shot's eye.
/// The order is `(dz^2 + dy^2) + dx^2` (`0x48eea5`); the threshold is `30` (`0x804554`) squared
/// inline. `fcompp` leaves `thr^2` in `st0` and `dist^2` in `st1`, and `test ah, 5; jp` skips
/// unless C0 and C2 differ.
pub fn shot_area_gate(p: [f32; 3], q: [f32; 3], c30: f32) -> bool {
    let dist2 = d2(p, q, [2, 1, 0]);
    let thr2 = f64::from(c30) * f64::from(c30);
    let (c0, c2, _) = fcom(thr2, dist2);
    c0 != c2
}

/// The gate in `0x48f080` (`uicamera/cinematic#stepping-and-ending`): the area under the camera's
/// target is prepared when `dist^2 > 30^2`. `p` is the camera's position and `q` the target's. The
/// order is `(dz^2 + dy^2) + dx^2` (`0x48f0e0`); the threshold is `30` (`0x804558`) squared inline,
/// compared as in [`shot_area_gate`].
pub fn cinematic_end_gate(p: [f32; 3], q: [f32; 3], c30: f32) -> bool {
    let dist2 = d2(p, q, [2, 1, 0]);
    let thr2 = f64::from(c30) * f64::from(c30);
    let (c0, c2, _) = fcom(thr2, dist2);
    c0 != c2
}

/// `CheckAreaSpiritHealer` (`0x4923b0`, `ui/distancegates#thresholds`): the cached area spirit
/// healer is dropped when `dist^2 > thr^2`. The order is `(dx^2 + dz^2) + dy^2` (`0x492459`). The
/// threshold is `thr = (c20 * c11) as f32` (20 and 1.1, `0x8044d0` and `0x804580`), stored at
/// `[ebp-4]` and squared inline. `fcompp` leaves `thr^2` in `st0`; `test ah, 5; jp` drops unless
/// C0 and C2 differ.
pub fn check_area_spirit_healer(p: [f32; 3], q: [f32; 3], c20: f32, c11: f32) -> bool {
    let dist2 = d2(p, q, [0, 2, 1]);
    let thr = (f64::from(c20) * f64::from(c11)) as f32;
    let thr2 = f64::from(thr) * f64::from(thr);
    let (c0, c2, _) = fcom(thr2, dist2);
    c0 != c2
}

/// `SelectAreaSpiritHealer` (`0x4924c0`): the spirit guide is chosen when `dist^2 <= 20^2`. The
/// order is `(dz^2 + dx^2) + dy^2` (`0x492537`); the threshold is `20` (`0x8044d0`) squared. The
/// action runs when the parity of C0 and C2 is even (`test ah, 5; jnp`), which is `dist^2 <=
/// thr^2` and also an unordered compare.
pub fn select_area_spirit_healer(p: [f32; 3], q: [f32; 3], c20: f32) -> bool {
    let dist2 = d2(p, q, [2, 0, 1]);
    let thr2 = f64::from(c20) * f64::from(c20);
    let (c0, c2, _) = fcom(thr2, dist2);
    c0 == c2
}

/// `CheckInteractionLeash` (`0x493230`, `ui/questshare#the-offer-leash`): the interaction closes
/// when `dist^2 > thr`. `p` is the interaction target's position and `q` the player's. The order is
/// `(dx^2 + dz^2) + dy^2` (`0x4932e2`). The threshold is the cell `0xb4e2c0`, already squared
/// (`0x4930d0` stores it). `fcomp` leaves `dist^2` in `st0`; `test ah, 0x41; jne` closes when C0
/// and C3 are both clear.
pub fn check_interaction_leash(p: [f32; 3], q: [f32; 3], thr: f32) -> bool {
    let dist2 = d2(p, q, [0, 2, 1]);
    let (c0, _, c3) = fcom(dist2, f64::from(thr));
    !c0 && !c3
}

/// `CanReadUnitInventory` (`0x4944a0`, `ui/distancegates#the-inspect-range`): the range test is
/// `dist^2 <= thr`. `p` is the target's position and `q` the active player's. The order is
/// `(dz^2 + dx^2) + dy^2` (`0x494515`). The threshold is the cell `0xb4d918`, already squared.
/// `test ah, 0x41; je` returns 0 when C0 and C3 are both clear, so the function answers true for
/// less, equal and unordered.
pub fn inspect_in_range(p: [f32; 3], q: [f32; 3], thr: f32) -> bool {
    let dist2 = d2(p, q, [2, 0, 1]);
    let (c0, _, c3) = fcom(dist2, f64::from(thr));
    c0 || c3
}

/// The magnitude guard of `CheckBattlefieldListLeash` (`0x4aa090`, `ui/battlefield`): the stored
/// vector `vec = [0xb6e870, 0xb6e874, 0xb6e878]` has `mag = |(y^2 + z^2) + x^2|` (`0x4aa096`,
/// `fabs`), and the function goes on only if `mag >= eps` (`0x8029d4`): `fcomp` leaves `mag` in
/// `st0` and `test ah, 5; jnp` skips unless the parity of C0 and C2 is even.
pub fn battlefield_leash_guard(vec: [f32; 3], eps: f32) -> bool {
    let mag = (sq(vec[1], 0.0) + sq(vec[2], 0.0)) + sq(vec[0], 0.0);
    let mag = mag.abs();
    let (c0, c2, _) = fcom(mag, f64::from(eps));
    c0 == c2
}

/// The distance gate of `0x4aa090`, after the guard: `q` is the active player and `vec` the stored
/// vector. The order is `(dx^2 + dy^2) + dz^2` (`0x4aa115`). The threshold is the cell
/// `0xb6e854`, already squared. `test ah, 0x41; jne` takes the reset (the vector zeroed and
/// `0x703e50` called) when C0 and C3 are both clear, which is `dist^2 > thr`.
pub fn battlefield_leash_distance(vec: [f32; 3], q: [f32; 3], thr: f32) -> bool {
    let dist2 = d2(q, vec, [0, 1, 2]);
    let (c0, _, c3) = fcom(dist2, f64::from(thr));
    !c0 && !c3
}

/// `CheckBattlefieldListLeash` (`0x4aa090`): the reset runs when the guard passes and the distance
/// is beyond the threshold.
pub fn check_battlefield_list_leash(vec: [f32; 3], q: [f32; 3], eps: f32, thr: f32) -> bool {
    battlefield_leash_guard(vec, eps) && battlefield_leash_distance(vec, q, thr)
}

/// `ClassifyObjectDot` (`0x4eaa90`, `ui/minimapblips#object-dots`): the callback records an object
/// when `dist^2 <= r^2`. `desc` is the descriptor's position (`+0xc/+0x10/+0x14`), `q` the
/// object's, and `r` the descriptor's radius (`+0x18`). The order is `(dz^2 + dy^2) + dx^2`
/// (`0x4eaaf9`); `r^2` comes from an inline `fmul`. `test ah, 5; jnp` rejects unless the parity of
/// C0 and C2 is even.
pub fn classify_object_dot(desc: [f32; 3], q: [f32; 3], r: f32) -> bool {
    let dist2 = d2(q, desc, [2, 1, 0]);
    let r2 = f64::from(r) * f64::from(r);
    let (c0, c2, _) = fcom(r2, dist2);
    c0 == c2
}

/// `CSimpleFrame::OnMouseMove` (`0x76b320`, `ui/mouse`): the two-dimensional drag-commit gate.
/// `pt` is the event's position (`+0x24`, `+0x28`) and `anchor` the frame's press position
/// (`+0x10c`, `+0x110`). The order is `dy^2 + dx^2` (`0x76b35e`). The threshold is the cell
/// `0xcf0c14`, already squared. `test ah, 1; jne` skips when C0 is set, so the drag commits when
/// `dist^2 >= thr`; a NaN sets C0 and does not commit.
pub fn drag_commit_gate(pt: [f32; 2], anchor: [f32; 2], thr: f32) -> bool {
    let dx = sq(pt[0], anchor[0]);
    let dy = sq(pt[1], anchor[1]);
    let dist2 = dy + dx;
    let (c0, _, _) = fcom(dist2, f64::from(thr));
    !c0
}

/// The deltas and the result of [`fade_gate`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FadeGate {
    /// `obj.z - owner.z` stored as `f32` (`fstp [ebp-4]`, `0x4b0ca8`).
    pub dz: f32,
    /// `obj.y - owner.y` as `f32` (`fstp [ebp-8]`, `0x4b0c9f`).
    pub dy: f32,
    /// `obj.x - owner.x` as `f32` (`fstp [ebp-0xc]`, `0x4b0c96`).
    pub dx: f32,
    /// The fade-out is taken (`dist^2 >= 400`); otherwise the fade-in. Meaningful only when the
    /// line-of-sight test passed.
    pub fade_out: bool,
}

/// The fade gate of `0x4b0c30` (a `ChatBubbleFrame.cpp` function): `owner` and `obj` are the two
/// positions. Each **delta is stored as `f32`** (`0x4b0c8f` to `0x4b0ca8`), and `dist^2` is then
/// `(dz^2 + dy^2) + dx^2` (`0x4b0cb4`) over those `f32` deltas, kept at register precision for the
/// compare. The threshold is `400` (`0x806798`, already squared). `test ah, 5; jnp 0x4b0cec` takes
/// the fade-in unless the parity of C0 and C2 is even, so the fade-out is `dist^2 >= 400`. A failed
/// line-of-sight test (`0x6704c0`) forces the fade-out whatever this returns.
pub fn fade_gate(owner: [f32; 3], obj: [f32; 3], c400: f32) -> FadeGate {
    let dx = (f64::from(obj[0]) - f64::from(owner[0])) as f32;
    let dy = (f64::from(obj[1]) - f64::from(owner[1])) as f32;
    let dz = (f64::from(obj[2]) - f64::from(owner[2])) as f32;
    let dist2 = (f64::from(dz) * f64::from(dz) + f64::from(dy) * f64::from(dy))
        + f64::from(dx) * f64::from(dx);
    let thr = f64::from(c400);
    let (c0, c2, _) = fcom(dist2, thr);
    FadeGate {
        dz,
        dy,
        dx,
        fade_out: c0 == c2,
    }
}

/// `0x494200` (a `GameUI.cpp` function): the squared distance from `p` to `q`, in the order
/// `(dz^2 + dy^2) + dx^2` (`0x494294`), then **stored as `f32`** (`fstp [ebp-4]`, `0x4942a4`). The
/// candidate test that follows compares that rounded value with a radius squared, a cone and a
/// line of sight; this is the value the compare reads.
pub fn candidate_distance_sq(p: [f32; 3], q: [f32; 3]) -> f32 {
    d2(p, q, [2, 1, 0]) as f32
}

/// The `f64` of the falloff base at `0x808ad0`: the `double` widening of the `float` 0.7, that is
/// `f64::from(0.7f32)` = `0.699999988079071` (`0x3fe6666660000000`), not `0.7_f64`.
pub const FALLOFF_BASE: f64 = f64::from_bits(0x3fe6_6666_6000_0000);

/// What [`shake_falloff`] returns and writes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShakeFalloff {
    /// `dist^2` stored as `f32` (`fstp [ebp+8]`, `0x511709`).
    pub dist2: f32,
    /// The function's return: true unless `dist^2 > outer^2`.
    pub ret: bool,
    /// The falloff branch ran (`dist^2 > inner^2`).
    pub falloff: bool,
    /// `*scale` after the call. The binary writes it only in the falloff regime (`inner^2 <
    /// dist^2 <= outer^2`); otherwise it is the input.
    pub scale_out: f32,
}

/// `ShakeFalloff` (`0x5116e0`, `uicamera/effects#distance-falloff`), called
/// `(eye, shake position, &scale, denom, inner^2, _, outer^2)`.
///
/// The order is `(dx^2 + dz^2) + dy^2` (`0x5116f9`), and `fstp [ebp+8]` rounds `dist^2` to `f32`
/// (`0x511709`). `fcomp` against `outer^2` with `test ah, 0x41; jne` continues unless C0 and C3
/// are both clear, so the function returns 0 when `dist^2 > outer^2`. Then `fcomp` against
/// `inner^2` runs the falloff when `dist^2 > inner^2`. The falloff (`0x511732` to `0x511750`):
///
/// 1. `fld [ebp+8]; fsqrt` takes the root of the register value; the `fst [ebp+8]` that follows
///    re-stores it as `f32` and the stored copy is not read;
/// 2. `t = (sqrt(dist^2) - denom) / denom`, `denom` widened from `f32`;
/// 3. `0x73f940` is the C runtime's `pow`: with `st1 = 0.7f` ([`FALLOFF_BASE`]) and `st0 = t` it
///    computes `2^(t * log2(0.7f))` with `fyl2x`, `f2xm1` and `fscale`, which is `0.7^t`. The host
///    kernel is `(t * log2(base)).exp2()`. The emulator evaluates `fyl2x` and `f2xm1` in its own
///    soft-float and not through the host's maths library, so the factor can differ from the host's
///    by one ulp of `f64`; the rounding to `f32` in step 4 absorbs that on every case the test runs;
/// 4. `*scale = (factor * f64(scale_in)) as f32` (`fmul [eax]; fstp [eax]`).
pub fn shake_falloff(
    p: [f32; 3],
    q: [f32; 3],
    scale_in: f32,
    denom: f32,
    inner2: f32,
    outer2: f32,
) -> ShakeFalloff {
    let dist2 = d2(p, q, [0, 2, 1]) as f32;
    let d = f64::from(dist2);
    let (c0o, _, c3o) = fcom(d, f64::from(outer2));
    let ret = c0o || c3o;
    let mut falloff = false;
    let mut scale_out = scale_in;
    if ret {
        let (c0i, _, c3i) = fcom(d, f64::from(inner2));
        falloff = !(c0i || c3i);
        if falloff {
            let denom = f64::from(denom);
            let t = (d.sqrt() - denom) / denom;
            let factor = (t * FALLOFF_BASE.log2()).exp2();
            scale_out = (factor * f64::from(scale_in)) as f32;
        }
    }
    ShakeFalloff {
        dist2,
        ret,
        falloff,
        scale_out,
    }
}

/// `0x493d30` (a `GameUI.cpp` function): the squared distance between `a` and `b` as the `f32` it
/// passes to the virtual call at `vtbl+0x70`. The order is `(dz^2 + dx^2) + dy^2` (`0x493def`),
/// then `fstp [esp]` rounds it (`0x493e06`). The deltas are `b - a`; the squares make the sign
/// immaterial.
pub fn unit_distance_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    d2(b, a, [2, 0, 1]) as f32
}

/// `0x4b1060` (a `ChatBubbleFrame.cpp` function): the sort key stored at `unit+0x350`
/// (`0x4b12c0`). After `0x4549a0` turns the player-to-unit delta into the unit's local space,
/// `local = [x, y, z]` (`[ebp-0x24]`, `[ebp-0x20]`, `[ebp-0x1c]`), the key is `(y^2 + z^2) + x^2`
/// (`0x4b12a8`), stored as `f32`.
pub fn bubble_sort_key(local: [f32; 3]) -> f32 {
    let [x, y, z] = local;
    let key =
        (f64::from(y) * f64::from(y) + f64::from(z) * f64::from(z)) + f64::from(x) * f64::from(x);
    key as f32
}

// ---- the inline gates of the Lua range checks ----

/// The squared distance of `CanInspect` (`0x48a1b0`, slice `0x48a26f` to `0x48a27d`) and
/// `CheckInteractDistance` (`0x48ba00`, `0x48badd` to `0x48bafb`): `(dz^2 + dx^2) + dy^2` with
/// `d = q - p` per axis. The operands are `f32` and at 53-bit precision nothing is stored to `f32`
/// in the kernel, as the `fcomp` reads the register, so the `f64` value is the binary's.
pub fn script_distance_sq(q: [f32; 3], p: [f32; 3]) -> f64 {
    let dx = f64::from(q[0]) - f64::from(p[0]);
    let dy = f64::from(q[1]) - f64::from(p[1]);
    let dz = f64::from(q[2]) - f64::from(p[2]);
    (dz * dz + dx * dx) + dy * dy
}

/// The inline range gate of `CanInspect` (`0x48a1b0`): after `fcomp d^2, [0xb4d918]`, `test ah,
/// 0x41; jne` skips the out-of-range action (the message `0x7d` through `0x496720`) when C0 or C3
/// is set, so the action runs when `thr < d^2` and not for a NaN. `threshold` is the cell
/// `0xb4d918`, 100.0 (10^2) once the initializer has run.
pub fn can_inspect_out_of_range(q: [f32; 3], p: [f32; 3], threshold: f32) -> bool {
    f64::from(threshold) < script_distance_sq(q, p)
}

/// The range gate of `CheckInteractDistance` (`0x48ba00`): `fcomp d^2, [esi*4 + 0xb4e3cc]` and
/// `test ah, 5; jp` jumps to the out-of-range path unless `d^2 < thr`, so the unit is in range only
/// for a strict, ordered less-than. `threshold` is `INTERACT_TABLE[distIndex - 1]` for the index 1 to 4.
pub fn check_interact_in_range(q: [f32; 3], p: [f32; 3], threshold: f32) -> bool {
    script_distance_sq(q, p) < f64::from(threshold)
}

/// The table at `0xb4e3cc`, built at the first call (`0x48ba30`) from the cells `0xb4d918`,
/// `0xb4e29c`, `0xb4e258` and `0xb4e2a0`: the squares of 10, 11.1111, 10 and 30. The index is
/// `distIndex - 1`.
pub const INTERACT_TABLE: [f32; 4] = [100.0, 123.45678, 100.0, 900.0];

// ---- CanInspect, whole ----

/// `ERR_UNIT_NOT_FOUND`, `ERR_GENERIC_NO_TARGET`, `ERR_INVALID_INSPECT_TARGET` and
/// `ERR_OUT_OF_RANGE`: the four messages `CanInspect` shows through `0x496720`
/// (`tables/messages.tsv` ids 295, 184, 297 and 125).
pub const MSG_UNIT_NOT_FOUND: u32 = 295;
pub const MSG_GENERIC_NO_TARGET: u32 = 184;
pub const MSG_INVALID_INSPECT_TARGET: u32 = 297;
pub const MSG_OUT_OF_RANGE: u32 = 125;

/// The object a token names, as `CanInspect` and `0x4944a0` read it.
#[derive(Clone, Copy, Debug)]
pub struct InspectTarget {
    /// The type word has the player bit (`shr ecx,4; test cl,1`).
    pub is_player: bool,
    /// The object is the active player's own.
    pub is_self: bool,
    /// `CanCooperate(player, target)`, asked only of another object.
    pub cooperates: bool,
    /// The position the vtable slot `+0x14` returns.
    pub pos: [f32; 3],
}

/// What `CanInspect` sees for one call.
#[derive(Clone, Copy, Debug)]
pub struct InspectWorld {
    /// The argument is a string or a number.
    pub arg_ok: bool,
    /// The token's first byte is 0.
    pub token_empty: bool,
    /// The object `GetUnitFromToken` finds.
    pub target: Option<InspectTarget>,
    /// The active player's position, when its object exists.
    pub player: Option<[f32; 3]>,
}

/// One effect of a `CanInspect` call, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectEffect {
    /// `luaL_error` with the usage text.
    Usage,
    /// A message through `0x496720`.
    Message(u32),
    /// `lua_pushnumber(1.0)`.
    PushOne,
    /// `lua_pushnil`.
    PushNil,
}

/// `CanInspect` (`0x48a1b0`), whole: the usage error, or at most one message and then the push of
/// `0x4944a0`'s answer.
///
/// With no object the message is 295 for a nonempty token and 184 for an empty one. With an object
/// the first failing test gives 297: no active player object, an object without the player bit, or
/// another object than the player's that `CanCooperate` refuses; a 297 skips the range test. Then
/// `100 < d^2` gives 125 (a NaN does not). The answer repeats the tests, so it is true exactly when
/// none failed.
pub fn can_inspect(w: &InspectWorld) -> Vec<InspectEffect> {
    use InspectEffect::{Message, PushNil, PushOne, Usage};
    if !w.arg_ok {
        return vec![Usage];
    }
    let mut out = Vec::new();
    let Some(t) = w.target else {
        out.push(Message(if w.token_empty {
            MSG_GENERIC_NO_TARGET
        } else {
            MSG_UNIT_NOT_FOUND
        }));
        out.push(PushNil);
        return out;
    };
    let refused = w.player.is_none() || !t.is_player || (!t.is_self && !t.cooperates);
    if refused {
        out.push(Message(MSG_INVALID_INSPECT_TARGET));
        out.push(PushNil);
        return out;
    }
    let player = w.player.expect("checked above");
    let far = can_inspect_out_of_range(player, t.pos, 100.0);
    if far {
        out.push(Message(MSG_OUT_OF_RANGE));
    }
    out.push(if far { PushNil } else { PushOne });
    out
}
