# collision: kinematics

How a `CMovement` turns its movement flags, its speeds and the time since its anchors into a
position, a facing and a pitch.

The code opens the shared movement object file, `0x7c4850` through `0x7c5cd0`: the initializer of
the first 172 bytes of `CMovement`, the transport-space transforms, the speed and rate pickers, the
direction basis, the displacement dispatcher and its four appliers, the spline follower and the
anchor snapshot. Only the elapsed time accumulates per substep: the facing and the pitch are the anchor values
plus a rate times the whole time since the anchors, and the displacement is measured from the
anchor position over that same time. Outside collision the client reaches only the transforms and
the speed selector; the rest is called by the mover, the status and queue code and the movement
command setters.

Roots: `0x7c4930`, `0x7c4a00`, `0x7c4ae0`, `0x7c4b30`, `0x7c4b80`, `0x7c4c90`

## Shared constructor

`0x7c4850(position, facing, owner)` fills `CMovement` from `+0x00` to `+0xab` and returns it.
`+0x34`, `+0x58`, `+0x80` and `+0xac` are not written.

| fields | value |
|---|---|
| `+0x00` to `+0x0c` (two list links) | 0 |
| `+0x10`, `+0x44` (position, anchor position) | `position[0..2]` |
| `+0x1c`, `+0x50` (facing, anchor facing) | `facing` |
| `+0x20`, `+0x54` (pitch, anchor pitch) | 0 |
| `+0x24` (up vector) | (0, 0, 1.0) |
| `+0x30` | `owner`, a pointer to a 64-bit GUID |
| `+0x38`, `+0x3c`, `+0x40` (transport, flags) | 0 |
| `+0x5c` to `+0x6c` (direction, heading basis) | 0 |
| `+0x70`, `+0x74` (pitch basis) | 1.0, 0 |
| `+0x78`, `+0x7c` | 0, `position[2]` |
| `+0x84` to `+0xa4` (speed, five speeds, turn rate, vertical, spline) | 0 |
| `+0xa8` | the client clock, `0x42c010()` |

Addresses: `0x7c4850`, `0x7c48e8`, `0x616fa0`, `0x42c010`
Confidence: verified
Proof: one
Test: none

`ret 0xc`, `this` in `ecx`. The 1.0 stores are `mov dword,0x3f800000` at `0x7c488f` and `0x7c48cf`;
`+0x7c` copies `position[2]` (`0x7c48dc`). The run `0x7c48e2` to `0x7c4912` stores `eax` (0) at
`+0x84` through `+0xa4`; `0x7c48e8` is the walk speed's store. The one caller is `0x616fa0`
(`0x616fb5`), which then fills `+0xb0` onward and calls `0x7c5a20(0)`, setting the basis and the
direction from the facing.

## Transport space transforms

With a zero transport GUID (`+0x38`, `+0x3c`) each transform is the identity. Otherwise `0x630ac0`
builds the transport's 4×4 matrix `M`, and:

| function | result |
|---|---|
| `0x7c4930(out, in)` | `in · M`, by `0x7bca80` |
| `0x7c4a00(out, in)` | `in` through the rigid inverse of M, `0x7bd820(…, 1.0)` |
| `0x7c4ae0(f)` | `F + f`, `F` the transport's facing from `0x630b70` |
| `0x7c4b30(f)` | `f − F` |
| `0x7c4b80(out)` | the `+0x24` vector through `M`'s 3×3 part |

The facings wrap once: above 2π subtracts 2π; else below 0 adds 2π; exactly 2π and NaN pass.
`M` is built from the transport object's position and facing, so `0x7c4930` and `0x7c4ae0` map
transport space to world and the other two map back.

Addresses: `0x7c4930`, `0x7c4a00`, `0x7c4ae0`, `0x7c4b30`, `0x7c4b80`, `0x630ac0`, `0x630b70`, `0x7bca80`, `0x7bd820`, `0xcf5d00`, `0x7c5dd0`
Confidence: verified
Proof: one
Test: `collision::tf_facing_to_transport_difftest`, `collision::tf_facing_from_transport_difftest`, `collision::tf_dir_to_transport_rotation_difftest`, `cmath::mat4_affine_invert_uscale`

`0x7c4b4d` is `d8 6d 08`, `fsubr m32`: `f − F`. The wrap is `fcom 2π; test ah,0x41; jne` (taken
for less, equal and NaN), then `fcom 0.0; test ah,5; jp` (falls into `fadd 2π` only for less).
The result returns in `st0` unrounded. `0x7c4b80` computes `out.x = (m0·x + m8·z) + m4·y`, `out.y`
from `m1, m9, m5`, `out.z` from `m2, m10, m6`, each stored as a float. `0x7bca80` is `out = v·M`
plus row 3, x = ((m8·z + m4·y) + x·m0) + m12, y = ((m9·z + m1·x) + m5·y) + m13,
z = ((m10·z + m2·x) + m6·y) + m14, each kept at 53 bits until its one float store. `0xcf5d00` holds 2π (`0x40c90fdb`); its one writer is the
static initializer `0x7c5dd0`, π + π. `0x630ac0` starts from the identity, translates by the
object's vtable slot `+0x14` and rotates by its slot `+0x18`; that these slots are the transport's
position and facing is inferred from the calls; `0x7bd820(…, 1.0)` is the rigid inverse of the
contact topic's facet gather.
The tests run `0x7c4c2e` to `0x7c4c7c`.

`cmath::mat4_affine_invert_uscale` runs `0x7bd820` whole.

## Current speed selector

`0x7c4c90(walk)` returns the current speed from the flags `+0x40`, first match:

| test | result |
|---|---|
| no bit of `0xf` | 0.0 |
| spline `+0xa4` set, its `+0x18` bit 2 clear | its `+0x30 / +0x24 · 1000`, unrounded |
| `0x00200000` and `0x2` | `+0x98` if `+0x98 ≤ +0x94`, else `+0x94` |
| `0x00200000` | `+0x94` |
| `0x100` or `walk ≠ 0` | `+0x88` if `+0x8c > +0x88`, else `+0x8c` |
| `0x2` | `+0x90` if `+0x90 ≤ +0x8c`, else `+0x8c` |
| otherwise | `+0x8c` |

`0x7c5c20(force)` stores `0x7c4c90(force)` at `+0x84` as a float, unless `0x2000` is set and `force` is 0.

Addresses: `0x7c4c90`, `0x7c4c99`, `0x7c4caa`, `0x7c4cd9`, `0x7c4d11`, `0x7c4d19`, `0x7c4d1d`, `0x7c4d4d`, `0x7c5c20`, `0x7c6ae0`
Confidence: verified
Proof: one
Test: `collision::move_setters_difftest`

`0x7c4c99` tests `dl,0xf`. `0x7c4caa` reads the spline: `fld [+0x30]; fidiv dword [+0x24]` (a
signed 32-bit divisor) `; fmul 1000.0` (`0x447a0000`). `0x7c4cd9` tests swimming, `0x7c4d11` bit
`0x100`, `0x7c4d19` the argument; both jump to `0x7c4d4d`: `fld +0x88; fld +0x8c; fcompp; test
ah,0x41; jne` returns `+0x8c` for `+0x8c ≤ +0x88` or NaN. The backward pairs at `0x7c4ce6` and
`0x7c4d1d` are `fld back; fcomp fwd; test ah,0x41; jp`, which returns the back field for less or
equal and the forward field for greater or NaN. So a tie takes `+0x8c` in the walk pair and the
back field in the backward pairs. `0x7c5c20`: `test ah,0x20; je` or `test eax,eax; je` skips the
store. `0x7c6ae0` passes 1 only while falling with no direction bit, its standstill air steer.

## Turn and pitch rates

`0x7c5c50` returns the turn rate: `+0x9c` with `0x10` set, else the negated `+0x9c` with `0x20`,
else 0.0. `0x7c5c90` returns the pitch rate the same way from `0x40` and `0x80`, from the same
`+0x9c`. Either is multiplied by 0.75 (`0x3f400000`) when any bit of `0x200f` is set: a direction
bit or falling. The result returns unrounded in `st0`.

Addresses: `0x7c5c50`, `0x7c5c90`, `0x7c5c73`
Confidence: verified
Proof: one
Test: `collision::get_strafe_speed_difftest`, `collision::get_pitch_rate_difftest`

`0x7c5c73` and `0x7c5cb3` are `test eax,0x200f; je` around `fmul 0.75`; `0x7c5c9f` tests the
pitch-down bit as `test al,al; jns`. Both read `this` from `ecx` and take no arguments (`ret`).
The negation is `fchs`.

## Facing and pitch integration

For `t` seconds since the anchors and a turn rate `ω` (the float `*override` when the pointer is
not null, else `0x7c5c50()` rounded to a float), `0x7c4f30` stores
`+0x1c = fl(r)` with `r = fmod(ω·t + +0x50, 2π)`, or `fl(r + 2π)` when `r < 0`.

`0x7c4f80` takes `θ = 0x7c5c90()·t + +0x54` and stores `+0x20`: −π/2 for `θ < −π/2`, `fl(θ)` for
`θ < π/2`, else π/2. A NaN `θ` stores π/2.

Both read the anchors and write the live values; nothing reads the old `+0x1c` or `+0x20`.

Addresses: `0x7c4f30`, `0x7c4f80`, `0x73f90a`, `0x7436c0`
Confidence: verified
Proof: one
Test: `collision::strafe_turn_difftest`, `collision::strafe_turn_trajectory`, `collision::pitch_integrate_difftest`, `collision::pitch_integrate_trajectory`

`0x7c4f4e` to `0x7c4f5d`: `fld ω; fmul t; fadd [+0x50]; fld 2π; call 0x73f90a`, the C runtime's
`fmod(st1, st0)`, whose result keeps the dividend's sign. `fcom 0.0; test ah,5; jp` adds 2π only
for less; NaN passes. `0x7c4f91`: `fcom −π/2` (`0xbfc90fdb`), `test ah,5; jp`; then `0x7c4fac`:
`fcom π/2` (`0x3fc90fdb`), same test, falling into the `fstp` only for less. The clamps store the
constants' dwords. `ω·t` and the sum stay at 53 bits; `t` is a float.

## Displacement dispatcher

`0x7c5360(ms, override, out)` returns 0 for `ms = 0`. Else `t = fl(ms · 0.001)` (`0x3a83126f`, `ms`
unsigned), and builds a mode. With `0x30` set or `override` not null it calls
`0x7c4f30(t, override)` and sets bit 2 unless `0x2000` (falling) is set; with `0xc0` it calls
`0x7c4f80(t)` and sets bit 8; `0x3` sets bit 1 and `0xc` bit 4. The mode picks the displacement:

| modes (1 move, 2 turn, 4 strafe, 8 pitch) | handler | returns |
|---|---|---|
| 0 | none | 1 |
| 1, 4, 5, 12 | `0x7c4ed0`, straight line | 1 |
| 2, 8, 10 | none | 0 |
| 3, 6, 7, 14 | `0x7c52c0`, turn arc | 1 |
| 9, 13 | `0x7c5180`, pitch arc | 1 |
| 11, 15 | `0x7c4fd0`, turn and pitch | 1 |

With no handler `out` is untouched; every caller zeroes it first.

Addresses: `0x7c5360`, `0x7c53af`, `0x7c53d9`, `0x7c53e6`, `0x7c5468`, `0x7c547c`, `0x616de0`, `0x618bb0`, `0x619090`, `0x618920`
Confidence: verified
Proof: one
Test: `collision::tf_substep_dt_difftest`

`0x7c538a` is `fild qword` of `ms` with a zero high dword. `0x7c53af`: `test ah,0x20; jne` keeps
bit 2 clear while falling, so a falling turn still turns the facing but the path stays straight.
`0x7c53d9`: `dec edi; cmp edi,0xe; ja`, so mode 0 wraps and exits with 1. `0x7c53e6` indexes the
dword table `0x7c5468` (`0x7c53f8`, `0x7c53ed`, `0x7c5412`, `0x7c5430`, `0x7c544a`) by the byte
table `0x7c547c`: `00 01 02 00 00 02 02 01 03 01 04 00 03 02 04`. `0x7c53ed` returns 0. The
callers `0x616de0`, `0x618bb0` and `0x619090` zero `out` and pass the elapsed `+0x58` (`0x619090`
adds a time difference to it) and `0x618920()`, which is `+0x144` when the float `+0x148` is not 0
(NaN counts as not 0), else null.
The tests run `0x7c538a` to `0x7c5396`.

## Straight line displacement

`0x7c4ed0(t, out)`, with `s = +0x84` and `d = +0x5c`: `out.x = fl((t·d.x)·s)`,
`out.y = fl((t·d.y)·s)`, `out.z = fl(fl(t·d.z)·s)`.

Addresses: `0x7c4ed0`
Confidence: verified
Proof: one
Test: `collision::linear_move_difftest`

The three products `t·d` are formed first; `t·d.z` is stored as a float (`0x7c4ef1`) and reloaded,
while `t·d.x` and `t·d.y` meet `s` in the register (`fmul st,st(2)` at `0x7c4ef6`, `fmul st,st(1)`
at `0x7c4efe`). `ret 8`.

## Turn arc displacement

`0x7c52c0(t, override, out)` moves along a circle. With `s = +0x84`, `ω` the float `*override` or
the unrounded `0x7c5c50()`, `(hx, hy) = (+0x68, +0x6c)` and `(pc, ps) = (+0x70, +0x74)`:
`R = fl(s/ω)`, `φ = fl(ω·t)`, `A = sin φ·R`, `B = R − cos φ·R`, then

- `out.x = fl((A·hx − B·hy)·pc)`
- `out.y = fl((B·hx + A·hy)·pc)`
- `out.z = fl((s·ps)·t)`

The arc bends toward `(−hy, hx)` for `ω > 0`. `sin φ` and `cos φ` are floats.

Addresses: `0x7c52c0`
Confidence: verified
Proof: one
Test: `collision::simple_turn_move_difftest`

`0x7c52ea` `d8 f1` is `fdiv st,st(1)`, `st0 = s/ω`. `0x7c5307` `fsincos` leaves the cosine in
`st0`, stored first to `[ebp−4]`. `0x7c531c` `d8 6d f8` is `fsubr m32`: `R − cos φ·R`.
`0x7c5329` `de e9` is `fsubp st(1),st`: `A·hx − B·hy`. A zero `ω` gives an infinite or NaN `R`.

## Pitch arc displacement

`0x7c5180(t, out)`, with `p = 0x7c5c90()`: `Rv = fl((1/p)·s)`, `Δ = fl(p·t)`, `θ = p·t + +0x54`.
Past a clamp the time over it is `τ = fl((θ − π/2)·fl(1/p))` and `Δ = fl(π/2 − +0x54)` for
`θ > π/2`, or `τ = fl((θ + π/2)·fl(1/p))` and `Δ = fl(−π/2 − +0x54)` for `θ < −π/2`; else
`τ = 0` (NaN included). With `H = sin Δ·Rv`, `V = Rv − cos Δ·Rv` and `P = H·pc − V·ps`:
`out = (fl(P·hx), fl(P·hy), z)`, `z = V·pc + H·ps`. When `|τ| ≥ 2⁻²⁰` or NaN, `z` becomes
`z + τ·s` if `|Δ + +0x54 − π/2| < 2⁻²⁰`, else `z − τ·s`: a straight climb or dive at the clamp.

Addresses: `0x7c5180`
Confidence: verified
Proof: one
Test: `collision::pitch_arc_difftest`

`0x7c519d` `fdiv st,st(1)` gives `1/p`, kept by `fst` as a float and used unrounded for `Rv`.
`0x7c51b1`: `fcom π/2; test ah,0x41; jne` enters the upper clamp only for greater; `0x7c51d8`:
`fcom −π/2; test ah,5; jp` the lower only for less. `0x7c5232` is `fsubr m32`. The `τ` test
`0x7c526a` is `fabs; fcomp 2⁻²⁰` (`0x35800000`), `test ah,5; jnp` skipping for less. The
correction adds to the register `z` kept by `fst` at `0x7c5262`: `fadd st,st(1)` or
`0x7c52a6` `d8 e9`, `fsubr st,st(1)`, `z − τ·s`. `ret 8`.

## Turn and pitch displacement

`0x7c4fd0(t, override, out)` combines both arcs. It takes `ω` as a float, `Rh = fl(s/ω)`, and
`Rv`, `Δ`, `τ` as `0x7c5180` does; then `φ = fl((t − τ)·ω)`, `A = 0.5·(sin Δ·Rv + sin φ·Rh)`,
`B = cos φ·Rh − Rh`, `C = fl(Rv − cos Δ·Rv)`, `X = fl(A·hx − B·hy)`, `Y = B·hx + A·hy`:

- `out.x = fl(X·pc − C·ps)`
- `out.y = fl(Y)`
- `out.z = C·pc + X·ps`, then the `τ` correction of `0x7c5180`

`B` has the opposite sign of `0x7c52c0`'s, so the arc bends toward `(hy, −hx)` for `ω > 0`,
away from the side the facing turns to.

Addresses: `0x7c4fd0`
Confidence: verified
Proof: one
Test: `collision::turn_pitch_arc_difftest`

`0x7c50da` is `d8 65 e0`, `fsub m32`: `cos φ·Rh − Rh`, against `fsubr` (`d8 6d`) at `0x7c50e3`
for `C` and at `0x7c531c` in `0x7c52c0`. `0x7c50f3` and `0x7c5110` are `de e9`,
`st1 = st1 − st0`. `X` and `C` are stored as floats (`[ebp−0x34]`, `[ebp−0x38]`) and reloaded;
`Y` stays in the register. `0x7c50ce` multiplies by 0.5 (`0x3f000000`). `ret 0xc`.

## Direction and heading basis

`0x7c5880` sets the basis `h = (+0x68, +0x6c) = (fl(cos F), fl(sin F))`, `F = +0x50`. Swimming
(`0x00200000`) with `|P| ≥ 2⁻²⁰` or NaN, `P = +0x54`, it sets `(+0x70, +0x74) = (fl(cos P),
fl(sin P))` and `d = +0x5c = (fl(h.x·+0x70), fl(h.y·+0x70), +0x74)`; else `d = (h, 0)` and
`(+0x70, +0x74) = (1, 0)`. `0x7c5a20(force)` calls it and then turns `d` and `h` by the
direction bits, first match (`−` applies for `0x2`, `√½` = 0.70710677, `0x3f3504f3`):

| bits | `d` | `h` |
|---|---|---|
| `0x2000` with `force` 0 | not written | not written |
| `0x3` and `0xc` | `(±d + (s, 0))·√½` | `(±h + s)·√½` |
| `0x3` | `±d` | `±h` |
| `0x4` | `(s, 0)`, `s = (−sin F, cos F)` | `s` |
| `0x8` | `(s, 0)`, `s = (sin F, −cos F)` | `s` |

Addresses: `0x7c5880`, `0x7c5a20`, `0x4549a0`
Confidence: verified
Proof: one
Test: `collision::velocity_basis_difftest`, `collision::velocity_vector_difftest`

`0x7c58cc`: `fabs; fcomp 2⁻²⁰; test ah,5; jnp` takes the flat branch only for less. In `0x7c5a20`,
the side vector swaps `+0x68` and `+0x6c` and negates one by `test byte [+0x40],4`
(`0x7c5ae0`). In the diagonal case the forward pair is saved first and negated for `0x2`
(`fchs`, `0x4549a0` for the direction), the side vector is built from the un-negated basis, and
`+0x5c = (side.x + d.x, d.y + side.y, d.z)`, `+0x68 = fl(h.x + side.x)`, `+0x6c` likewise. Each of
the five floats is then multiplied by `0x3f3504f3` and rounded (`0x7c5b35` to `0x7c5b6d`).

## Anchors and elapsed time

`0x7c5cd0(force)` copies the live position, facing and pitch (`+0x10` to `+0x20`) to the anchors
(`+0x44` to `+0x54`), zeroes the elapsed time `+0x58`, calls `0x7c5a20(force)`, and calls the
empty `0x6307a0` with the owner's GUID.

Each substep the caller adds its time to `+0x58` (`0x618bb0` only while a bit of `0xf` is set) and
asks `0x7c5360` for the displacement over the whole `+0x58`. `0x616cb0` adds it to the anchor position and subtracts the live `+0x10`,
and the resolver `0x634040` moves `+0x10` by that step. So position, facing and pitch are
recomputed from the anchors every substep; only `+0x58` accumulates.

Addresses: `0x7c5cd0`, `0x6307a0`, `0x616cb0`, `0x634040`, `0x616e3c`, `0x618be0`, `0x7c61f0`
Confidence: verified
Proof: one
Test: none

`0x7c5cfe` stores 0 at `+0x58` before `call 0x7c5a20`. `0x6307a0` is `ret 8`. `0x616e3c` and
`0x618be0` are the stores into `+0x58` before each dispatch; `0x616cb0` computes
`(+0x44 + out) − +0x10` per component, x and y rounded once, z with the sum rounded first,
and zeroes the z step unless swimming, `0x800` or a flying spline, before calling `0x634040`. `0x7c61f0`, which starts a fall,
re-anchors with `0x7c5cd0(0)` first, so a fall's path is measured from its takeoff.

## Spline follower

`0x7c5490(now, out)` returns `flags & 0x200f` without writing when `+0x40` has no bit of `0x3`, and
1 without writing when the spline's `+0x18` bit 31 is set. Else it sets the spline's `+0x20` to
`now − +0x1c` and its progress `u`: 1.0 for a zero duration `+0x24`, 0 for a negative elapsed
time, 1.0 (setting `+0x18` bit 0) at or past the duration, else `fl(elapsed/duration)`. It
evaluates the curve at `u`, writes the tangent to `+0x5c` and `+0x1c = atan2(ty, tx)` (+2π when
negative), and returns `out = point − anchor` and 1. Swimming, `+0x20 = fl(asin tz)`; else, with
spline bit `0x200`, it rolls `+0x24` about the tangent by a bank toward the point 1000 ms ahead.

Addresses: `0x7c5490`, `0x7c5623`, `0x616af0`, `0x453480`, `0x453390`, `0x454980`, `0x7be490`, `0x7405a0`, `0x741820`
Confidence: inferred
Proof: none
Test: `collision::spline_follow_frac1`, `collision::spline_follow_frac2`, `collision::spline_follow_facing_wrap`, `collision::spline_follow_turn`, `collision::spline_follow_mat3_mul_vec`, `collision::spline_follow_relative_out`

`u` is `fild`/`fidiv` of signed dwords. `0x453480` clamps `u` to [0, 1] and calls the curve's
slot `+0x18` on a 4×4 frame whose first row is preset to `+0x5c`; rows 0 and 3 are read back.
The bank: `0x7c5623` adds 1000 ms, clamps `u` to [0, 1] (NaN to 1), takes the point through
`0x453390`, and forms `v` from the world `+0x10` (`0x7c4930`) to it. The horizontal tangent `h`
and `v` are each scaled by 1/len, kept at 53 bits, only when their length is at least 2⁻²²
(`0x34800000`), each component then rounded to a float; `c = fl(v.y·h.y + v.x·h.x)` clamped to
[−1, 1] (NaN to 1); `cross = v.y·h.x − h.y·v.x` stays unrounded on the stack. The angle is `2·acos c` for `cross < 0` or NaN,
else `−2·acos c`, clamped to ±π/2 (NaN to π/2). A zero `v` gives −π/2. `0x7be490` builds the
rotation and the frame's third row times it becomes `+0x24`. These callee readings are inferred.
The tests run `0x7c5509` to `0x7c551c`, `0x7c55a5` to `0x7c55ca`, `0x7c562c` to `0x7c566d`,
`0x7c569c` to `0x7c57c6`, `0x7c57d9` to `0x7c5841` and `0x7c5841` to `0x7c5871`.

## Teardown

`0x7c5950` releases a `CMovement`'s links: it ends the spline through `0x7c6310` and unlinks the
links at `+0x00` and `+0x08`, each when its first dword is set. When the second dword of the
`+0x08` link was set and the transport GUID is not 0, it calls the empty `0x630c10` with the
owner's and the transport's GUIDs. It then zeroes `+0x38` and `+0x3c`, clears flag `0x08000000`
through `0x7c6510`, and clears bit 31 of `+0x40`.

`0x7c4e90` jumps to `0x7c4ea0`, which closes the file at offset `0x104` of the movement globals
(`0x630a40`) and zeroes it.

Addresses: `0x7c5950`, `0x617060`, `0x7c6310`, `0x630c10`, `0x7c6510`, `0x7c4e90`, `0x7c4ea0`, `0x616910`, `0x630a40`, `0x73f285`
Confidence: verified
Proof: one
Test: none

The links unlink by the list rule: a tagged or null next pointer is the next link itself
(`and eax,0xfffffffe`), else it is offset by this link's offset in the previous node. A cleared
`+0x0c` zeroes the GUID passed (`0x7c599b`), and a zero GUID skips the call. `0x630c10` is
`ret 0x10`. `0x617060`, the one caller, then destroys the list at `+0x150`. `0x616910` calls
`0x7c4e90`; `0x7c4ea0` calls `0x73f285` (`fclose`) when the handle is not null.

## Unreached functions

Nothing reaches `0x7c4d80`, which opens a log named by the movement globals' path with `__fall`
before its extension, mode `wt`, into offset `0x108` and stores its two arguments at `0x110`
and `0x114`; nor the empty stubs `0x7c4e30`, `0x7c4e40`, `0x7c4e50`, `0x7c4e60`, `0x7c4e70` and
`0x7c4e80`, which return 0 or nothing.

Addresses: `0x7c4d80`, `0x7c4e30`, `0x7c4e40`, `0x7c4e50`, `0x7c4e60`, `0x7c4e70`, `0x7c4e80`, `0x73f333`
Confidence: verified
Proof: one
Test: none

Each address has no `e8` or `e9` site targeting it and no little-endian occurrence anywhere in the
image; the same scans find `0x7c5360` called three times and `0x7acb30` stored at `0x81d7f4`. Each
starts after padding that follows a `ret`. `0x7c4d80` calls `0x73f333` (`fopen`) only while the
handle is null; the strings `__fall` and `wt` sit at 0x87d89c and 0x835140.
