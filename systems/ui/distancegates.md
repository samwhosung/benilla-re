# ui: distance gates and range checks

Most interface range tests compare one squared 3D distance with a threshold. This doc is the shared arithmetic, the thresholds with their operators, and the two Lua range checks, `CanInspect` and `CheckInteractDistance`, including what they answer for a token that names no object.

Roots: `0x4944a0`, `0x48a1b0`, `0x48ba00`, `0x4923b0`, `0x4924c0`, `0x492130`

## The kernel

Every gate loads two positions through slot `+0x14` of the object (a `C3Vector` getter), subtracts them, squares each difference, sums the three squares and compares the sum with a squared threshold, in x87 registers. The client runs the x87 at 53-bit precision (control word 0x027F), not 80-bit extended: the squares of the float differences are exact and each add rounds.
The compare is `fcomp` or `fcompp` followed by `fnstsw` and a mask of `ah`: `test ah,0x41` with `je` is the gate for greater than, and `test ah,5` with `jp` or `jnp` separates less than and unordered from the rest. The order of the three squares differs from site to site.

Addresses: `0x4944a0`, `0x4944f5`, `0x494502`, `0x494505`, `0x494525`, `0x494531`, `0x494536`, `0x48bafd`, `0x48bb0a`, `0x48bb0f`, `0x4923b0`, `0x492130`, `0x492196`, `0x4924c0`, `0x494200`, `0x4942a4`, `0x5116e0`, `0x511709`
Confidence: verified
Proof: one
Test: `ui::dist2_494200`, `ui::glue_492_492130_sqdist_radius`

Two sites store the squared sum to a 32-bit float before the compare (`fstp dword` at `0x4942a4` in `0x494200` and at `0x511709` in `0x5116e0`), so those compare the rounded value. A NaN is unordered: `0x4944a0` passes it (C3 is set, so the `je` after `test ah,0x41` is not taken) and `0x48ba00` gives nil.

`ui::dist2_494200` runs the slice `0x494275` to `0x4942a7`, up to the store of the rounded squared distance.

## Thresholds

Each threshold is the square of a constant, folded by a static initialiser (`fld [c]; fmul [c]; fstp [cell]`) or squared inline.

| site | distance from the player to | squared threshold | passes when |
|---|---|---|---|
| `0x4944a0` | a player unit that it can cooperate with, or itself | `0xb4d918` = 10² | at most |
| `0x48ba00` | the unit, by distance index 1 to 4 | 10², 11.111111², 10², 30² | strictly less |
| `0x4924c0` | a spirit guide (NPC flag 0x40) that may assist it | 20² | at most |
| `0x4923b0` | the cached area spirit healer | (20 * 1.1)² as a float | at most |
| `0x492130` | the cached corpse position | `0xb4e2ac` = 40² | at most |
| `0x48edf0` | camera eye to the new shot's position | 30² | not above (else the areas unload) |
| `0x48f080` | camera eye to the camera's followed unit | 30² | not above |

Addresses: `0x4944a0`, `0xb4d918`, `0x48cc80`, `0x48cc86`, `0x48cc8c`, `0x804498`, `0x48ba30`, `0x48ba4e`, `0x48ba53`, `0x48ba58`, `0x48ba5d`, `0x48ba63`, `0x48ba69`, `0xb4e3cc`, `0xb4e29c`, `0xb4e258`, `0xb4e2a0`, `0x48cbb0`, `0x48cb50`, `0x48cd10`, `0x4924c0`, `0x492506`, `0x492547`, `0x4923b0`, `0x492406`, `0x492411`, `0x8044d0`, `0x804580`, `0x492130`, `0x48edf0`, `0x48eeb5`, `0x48eec9`, `0x48f080`, `0x48f0f0`, `0x804554`, `0x804558`, `0x804490`, `0x80448c`, `0x8044a4`
Confidence: verified
Proof: one
Test: `ui::dist2_4923b0`, `ui::dist2_4924c0`

The 10, 11.111111, 10 and 30 of `CheckInteractDistance` are the cells `0xb4d918`, `0xb4e29c`, `0xb4e258` and `0xb4e2a0`, copied into the table `0xb4e3cc` once, at the first call (`0x48ba30`). `0x4923b0` keeps an area spirit healer to 22.000001 yards and `0x4924c0` accepts one within 20, so a healer leaves at 22 and is chosen at 20. `0x4924c0` asks `CGUnit_C::CanAssist_Unit` with the guide as `this` and the player as the argument (`0x492506`).
The cinematic gates are `uicamera/cinematic`; the corpse gate is `deathui#corpse-range-events`. The squares of `0x48edf0` and `0x48f080` are formed inline from 30.0.

`ui::dist2_4923b0` runs the slice `0x492448` to `0x492475` and `ui::dist2_4924c0` the slice `0x492527` to `0x492559`, each the distance test.

## The inspect range

`0x4944a0(unit)` answers whether a unit may be inspected: the active player exists, the unit is not null, its type word has the player bit, it is the player itself or `CanCooperate(player, unit)` holds, and the squared distance is at most 100. It answers 0 for any failure and 1 otherwise.

Addresses: `0x4944a0`, `0x4944c7`, `0x4944cb`, `0x4944d8`, `0x4944dd`, `0x4944e4`, `0x494536`, `0x606ba0`, `0x4c8331`, `0x4c8600`, `0x4c86f8`, `0x4c895e`, `0x4c8af6`, `0x4c8ca7`, `0x53306a`
Confidence: verified
Proof: one
Test: `ui::dist2_4944a0`

It has eight callers: `CanInspect` (`0x48a2a1`) and the seven at `0x4c8331` to `0x53306a`.

`ui::dist2_4944a0` runs the slice `0x494505` to `0x494531` after running the initializer `0x48cc80` that sets the threshold.

## CanInspect

`CanInspect(unit)` raises `Usage: CanInspect(unit)` unless the argument is a string or number. It resolves the token (`0x515940`). When there is no object it shows an error and then still runs `0x4944a0` with a null unit, which answers 0: the error is `ERR_UNIT_NOT_FOUND` for a nonempty token and `ERR_GENERIC_NO_TARGET` for an empty one.
Otherwise it runs `0x4944a0` on the object. It pushes the number 1 for a true answer and nil for a false one, one value either way.

Addresses: `0x48a1b0`, `0x48a1ca`, `0x48a1f1`, `0x48a1fa`, `0x48a29f`, `0x48a2a1`, `0x48a2d6`, `0x48a2df`, `0x48a2e4`, `0x48a2ee`, `0x48a2f3`, `0x48a2fd`, `0x496720`
Confidence: verified
Proof: one
Test: `ui::caninspect_dist2`

The error ids are 295 and 184 of `tables/messages.tsv` (both kind 2). A party member who is out of range has a roster GUID but no object (`party#tables-and-the-party-functions`), so polling `CanInspect("party3")` shows an error line on each call.

`ui::caninspect_dist2` runs the slice `0x48a25f` to `0x48a289`, the inline distance test that precedes the call of `0x4944a0`.

## CheckInteractDistance

`CheckInteractDistance(unit, index)` raises `Usage: CheckInteractDistance("unit", distIndex)` for a non-string unit or a non-number index. It pushes nil with no error when there is no active player, when the token resolves to no object, or when the truncated index is not 1 to 4 (an unsigned compare of `index - 1` against 4).
Otherwise it pushes the number 1 when the squared distance is below the indexed threshold, else nil. A string index that reads as a number counts.

Addresses: `0x48ba00`, `0x48ba09`, `0x48ba1e`, `0x48bb42`, `0x48bab7`, `0x48babe`, `0x48bac2`, `0x48bb2f`, `0x48baaf`, `0x48bafd`
Confidence: verified
Proof: one
Test: `ui::check_interact_dist2`

`0x40a2b0` truncates toward zero, so 1.9 is index 1 and 0.5 is index 0 (nil). The unit guard is an explicit test and shows no error, unlike `CanInspect`.

`ui::check_interact_dist2` runs the slice `0x48badd` to `0x48bb08`, the distance test.
