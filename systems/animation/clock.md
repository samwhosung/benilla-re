# animation: bone clocks

What a bone's state holds and how its two clocks are computed. Every bone of an instance has a state
of 0x118 bytes with two sequence slots, the primary and the blend. The head of the bone loop turns
each slot's record, cursors, speed and offset into a time and a range slot, copies them from the
parent for a bone without a record, ends the blend slot when its fade is over, and picks the blend
weight that the samplers use. The code is `0x714530` through `0x71492d` in `CM2Model::Animate`.
`models/animation#sequence-time` and `#blend-weight` state the formulas in short; this doc gives the
branches and the order.

Roots: none

## Bone state

A bone state is 0x118 bytes: three track states, then the primary slot, the blend slot laid out the
same, the fade and the active list link. A time is a signed 32-bit millisecond count, a speed a float.
The constructor sets the primary and blend records, slots and bone indices (`+0xa0`, `+0xcc`) to −1, the cursors, speeds and offsets to 0,
the repeats to 1, both completion latches to 1, the fade scale to 1.0, the requested id to −1 and the
list next index to −1. It never writes the inverse speeds `+0xb4` and `+0xe0`.

| offset | holds |
|---|---|
| `+0x00`, `+0x30`, `+0x68` | track states: translation (0x30 bytes), rotation (0x38), scale (0x30) |
| `+0x98`, `+0x9c`, `+0xa0` | primary time, range slot, bone stamp |
| `+0xa4` | primary record index, −1 for none |
| `+0xa8`, `+0xac` | primary cursors: start stamp, end stamp |
| `+0xb0`, `+0xb4` | speed and its inverse, floats |
| `+0xb8`, `+0xbc`, `+0xc0` | offset, repeats, completion latch |
| `+0xc4` to `+0xec` | the blend slot, laid out as `+0x98` to `+0xc0` |
| `+0xf0`, `+0xf4` | override matrix pointer, flags |
| `+0xf8`, `+0xfc` | requested animation id and variation |
| `+0x100`, `+0x104`, `+0x108` | fade end stamp, fade rate, fade scale |
| `+0x10c`, `+0x110`, `+0x114` | blend weight, list link cell, next bone index |

Addresses: `0x71b0a0`, `0x71b136`, `0x71b157`, `0x71b163`, `0x71b174`, `0x71b180`, `0x71b192`, `0x71b1aa`, `0x71b1b0`, `0x71b1e6`, `0x70ecc7`, `0x70ed0d`, `0x7125d6`, `0x7127b1`
Confidence: verified
Proof: one
Test: none

`0x70ecc7` calls the constructor for each bone, and `0x70ed0d` copies the bone record's flags into
`+0xf4`. `PlayAnimation` copies `+0x98` through `+0xc0` (11 dwords) over `+0xc4` through `+0xec` at
`0x7125d6`, so the blend slot receives the whole outgoing slot, record and cursors included. The state's
three track states are the shapes of `tracks`; the constructor clears each by its value size.

## Clock legs

A bone whose primary record `+0xa4` is not −1 computes its time from the record `r` (header
sequences, 0x44 bytes each), `now` the scene clock `[[+0x2c]+0xc]`, the cursors `lo` and `hi`, the
speed and the offset. With record flag bit 0 clear, or set with `hi − now > 0` and `lo − now ≤ 0`, the
time is the loop leg with base `now`. Set with `hi − now > 0` and `lo − now > 0` it is the loop leg
with base `lo`. Set with `hi − now ≤ 0` it is the clamp leg. The time goes to `+0x98`, the record to
`+0x9c` and the bone index to `+0xa0`.

Addresses: `0x714530`, `0x714555`, `0x714577`, `0x714585`, `0x71458c`, `0x71458e`, `0x714597`, `0x7145a1`, `0x7145e5`, `0x7145e9`, `0x7145eb`, `0x7145f1`, `0x714633`, `0x714639`, `0x714642`, `0x714648`
Confidence: verified
Proof: one
Test: none

`start` and `end` are `r+0x4` and `r+0x8`. The flag test `0x714585` takes `je 0x7145f1` for a clear bit,
and `0x7145f1` is the loop leg, so a clear flag loops. A set flag takes the clamp leg only when
`0x714597 sub; test; jg` finds `hi ≤ now`. When `lo > now` the leg sets `ecx = lo` at `0x7145eb`, so
the loop numerator is 0. The tests at `0x714597`/`0x7145a1`, `0x7145e5`/`0x7145e9` are signed, on
differences that wrap. The three stores at `0x714639`, `0x714642` and `0x714648` end every non-inherit path.

## Loop and clamp legs

The loop leg gives `start` when `end ≤ start` (signed). Otherwise it gives `start + (s mod (end − start))`
with `s = ftol(float(base − lo) · speed) + offset` and an unsigned remainder. The clamp leg computes
`s = ftol(float(hi − lo) · speed) + offset` and gives `start` if `s < 0`, `end` if `s > end − start`
(signed), and else `start + s`. The conversion is a signed 32-bit `fild` of the difference, one `fmul` by
the float speed with no narrowing, then `__ftol`, which truncates toward zero.

Addresses: `0x7145ab`, `0x7145ae`, `0x7145b4`, `0x7145b9`, `0x7145bf`, `0x7145d7`, `0x7145f7`, `0x7145f9`, `0x71460a`, `0x71460d`, `0x714613`, `0x714623`, `0x71462a`, `0x71462c`, `0x714631`, `0x40a2b0`
Confidence: verified
Proof: one
Test: none

The loop leg's divisor is `end − start` (`0x714626 sub ecx,edx` with `edx = start`) and the remainder is
rebased on `start` (`0x71462c`), not on the cursors. `0x714628 xor edx,edx; 0x71462a div ecx` is
unsigned, so a negative `s` divides as a large unsigned value. The degenerate case `0x7145f7 cmp edx,eax;
0x7145ff jle 0x714631` stores `start`. The clamp leg's tests are `0x7145bf jns` (for `s ≥ 0`) and
`0x7145d7 cmp eax,edx; 0x7145d9 jle`.

## Blend clock

A bone whose blend record `+0xd0` is not −1 runs the same computation on the blend fields: cursors
`+0xd4` and `+0xd8`, speed `+0xdc`, offset `+0xe4`, record `+0xd0`. It writes the time to `+0xc4` and
the record to `+0xc8`; it writes no stamp. Then it ends the slot: when `now − [+0x100] ≥ 0` (signed) it
stores −1 in `+0xd0`. The fade end `+0x100` is the end of the fade, so the slot ends in the pass the fade
reaches its end, before the weight is chosen.

Addresses: `0x7146b2`, `0x7146b8`, `0x7146c1`, `0x7146cb`, `0x7146d7`, `0x7146e6`, `0x7146f2`, `0x714711`, `0x714714`, `0x71471f`, `0x714790`, `0x714792`, `0x714799`, `0x71479f`, `0x7147a5`, `0x7147b1`, `0x7147b7`, `0x7147b9`
Confidence: verified
Proof: one
Test: none

The blend leg mirrors the primary leg instruction for instruction (`0x7146f4` to `0x714797`) on the
blend fields. `0x7147b1 sub ecx,[esi+0x100]` leaves the sign for `js 0x714820`; only a non-negative
difference reaches `0x7147b9`. The pause slide has added to `+0xd4` and `+0xd8` before the record is read
(`kernel#pause-slide`).

## Inherited clocks

A bone without a primary record copies `+0x98`, `+0x9c` and `+0xa0` from its parent's state, the parent
being the bone record's parent word `+0x8`. A parent word that is not below the bone count copies bone
0's, except bone 0, which keeps its values. A bone without a blend record copies `+0xc4` and `+0xc8` the
same way, except that bone 0 with an out-of-range parent copies its own `+0x98` and `+0x9c`. The loop
reads the parent's state of the same pass, so a parent stored before its children hands them this pass's
time, and one stored after hands them the previous pass's.

Addresses: `0x71455b`, `0x714650`, `0x714654`, `0x714657`, `0x714661`, `0x71468c`, `0x71468e`, `0x7146bb`, `0x7147c5`, `0x7147cc`, `0x7147cf`, `0x7147df`, `0x7147fe`, `0x714800`, `0x714808`, `0x714814`, `0x71481a`
Confidence: verified
Proof: one
Test: none

The primary leg compares the parent word with `[ebp+0x14]`, the bone count. The blend leg reloads the
count from header `+0x34` (`0x7147cc`), since the primary leg stores its record pointer in that slot. The primary leg's bone
0 case is `0x71468c jbe 0x7146b2`, which writes nothing. The blend leg's is `0x7147fe jbe 0x714808`,
which copies the primary time and slot into the blend fields.

## Weight selection

The blend weight `+0x10c` is chosen after both clocks. A bone with neither record takes its parent's
weight, or bone 0's for an out-of-range parent, or 0 for bone 0. Otherwise `rem = [+0x100] − now`: the
weight is 0 when `rem ≤ 0` or when the primary and blend clocks are equal in time and in slot, and else
the smoothstep of `models/animation#blend-weight` on `x = f32(float(rem) · [+0x104])`, scaled by
`[+0x108]`. It falls from the scale to 0 as the fade end nears.

Addresses: `0x714820`, `0x714826`, `0x71482b`, `0x714834`, `0x71483b`, `0x71484c`, `0x71485e`, `0x714863`, `0x714880`, `0x714883`, `0x714891`, `0x714897`, `0x7148a3`, `0x7148b1`, `0x7148b3`, `0x7148bc`, `0x7148bf`, `0x7148e3`, `0x714904`, `0x71491b`, `0x714923`
Confidence: verified
Proof: one
Test: none

`0x714880` runs when either record is set. `rem` is `fild dword` then `fmul [+0x104]` then `fst dword`: the
product is narrowed once and every later test reads the narrowed value (`0x7148bc`). `x < 0` takes
`0x7148cc` (0 times the scale); `x ≤ 1` and unordered take the smoothstep `0x714904`
(`(3 − 2x)·x·x·scale`, multiplied in that order); `x > 1` takes `0x7148f0` (1 times the scale). The
equal-clock test is `0x714897`/`0x7148a5`: `+0x98 = +0xc4` and `+0x9c = +0xc8`. The weight is also 0
for a bone whose blend slot was just cleared, since `rem ≤ 0` holds.
