# objectlayer: swimming

How a unit comes to swim: the per-unit depth decision, the flag bits that gate it, which units run
it and how each starts a swim. The mover's swim state, its commands and its movement are
`collision/handlers` and `collision/swimming`; this doc holds the decision that sits on the unit.

Roots: none

## Swim decision

`0x6030c0(time, inLiquid, depth, force)` decides the unit's swim state from the liquid depth over
its feet, `h` being the collision height `+0xa5c` (`CMovement+0xb4`). While `0x400` is set in the
movement flags it does nothing. A unit not swimming starts when `depth > 0.75·h`, a field-flags bit
of the gate below is set and the re-entry guard `0x7c5de0` is false. A swimming unit stops when it
is not in liquid, or `depth < 0.75·h − 1/36`, or none of the gate bits is set; at a depth equal
to the threshold it stays. The two thresholds differ by 1/36 yard (0.0277778), so between them a
unit keeps its state.

Addresses: `0x6030c0`, `0x6030c9`, `0x6030d2`, `0x6030db`, `0x6030e0`, `0x6030ec`, `0x6030f2`, `0x6030f4`, `0x6030f7`, `0x603100`, `0x603106`, `0x603132`, `0x603145`, `0x6031b4`, `0x6031bb`, `0x6031c0`, `0x6031c8`, `0x6031eb`, `0x6031f5`, `0x6031fa`, `0x7c5de0`, `0x60df70`, `0x60dff0`, `0x8012cc`, `0x7ff9d0`
Confidence: verified
Proof: one
Test: none

`0x6030d2 test ah,4` is bit `0x400`; its `jne 0x6031fa` precedes the swimming test `0x6030db`. The
thresholds are formed once: `h·0.75` (`0x8012cc` is 0.75) stays on the x87 stack and `h·0.75 − 1/36`
(`0x7ff9d0`, `fsubr`) is stored at `[ebp-4]`. Enter compares the depth with the stack value, `fcomp`
then `test ah,0x41; jne` (`0x603106`), so equal and NaN do not enter. Leave compares with `[ebp-4]`
(`0x6031c0`) and tests `ah,5; jnp` (`0x6031c8`): only `depth <` threshold jumps to the stop at
`0x6031eb`, equal and greater fall to the flag test. `0x603132` calls `0x7c5de0` and a nonzero
result skips `0x60df70`.

## Swim flag gate

Three bits of `UNIT_FIELD_FLAGS` (descriptor index 46, `[[unit+0x110]+0xa0]`) say whether a unit may
swim: `0x8` (player controlled), `0x800` (pet in combat) and `0x8000`. Entering needs one of them
set. Leaving is forced when none is set, at any depth: with all three clear, a unit already in the
water stops at the next decision. The scan below finds no other test of bit `0x8000` of this field.

Addresses: `0x603111`, `0x603117`, `0x603120`, `0x603128`, `0x60312a`, `0x60312c`, `0x6031ca`, `0x6031d6`, `0x6031dc`, `0x6031e4`, `0x6031e9`, `0x5fb922`, `0x5fb928`, `0x5fad25`, `0x5fad28`, `0x516410`, `0x51642e`, `0x5ff790`, `0x5ff79f`
Confidence: verified
Proof: one
Test: none

`0x5fad25 add ebx,0x18` and `0x5fad28 mov [esi+0x110],ebx` make `[unit+0x110]` the values block
plus six dwords, so `+0xa0` is index 46. The three tests share their success target: `0x603119 js
0x60312c` (bit 15), `0x603123 jne 0x60312c` (bit 3), `0x60312a je 0x60314a` (bit 11 clear skips).
On the leave leg the same bits keep the unit swimming (`0x6031d8`, `0x6031e2`, `0x6031e9` jump to
`0x6031fa`). The Lua function table pairs the name `UnitPlayerControlled` with `0x516410`, whose body
(`0x51642e`) shifts the same field by 3. `0x5ff790 test ah,0x8` on the flags change selects the
pet-attack events (`objectlayer/units`). A scan of each load of `[reg+0xa0]`
or `[reg+0xa1]` for a bit-15 test in the next 12 instructions finds `0x5fb922`, `0x603111` and
`0x6031d0`, and two other matches whose tested register is another one (`0x6e4614`, `0x6e6643`).

## Swim decision subjects

The decision runs for every registered unit, not for the local player alone. `0x6030c0` has one
caller, `0x5fac70` (at `0x5facec`), and `0x5fac70` two: the per-mover frame step `0x616620`
(`0x616795`, argument 0) and the movement status core `0x618c30` (`0x618e77`, argument 1, only when
the unit has no direction bit `0xff` and is not falling). Both take the unit from the mover's back
pointer at `+0x15c`. The frame step runs for each mover on the manager's list, which the monster
move apply `0x6187a0` joins through `0x619ca0`, so a creature moved by a spline is re-decided every
frame from its own liquid query, feet height and collision height.

Addresses: `0x6030c0`, `0x5facec`, `0x5fac70`, `0x5fac7a`, `0x5fac85`, `0x5facb7`, `0x5facd1`, `0x5facd4`, `0x5facda`, `0x616795`, `0x616792`, `0x618e62`, `0x618e69`, `0x618e6e`, `0x618e77`, `0x616620`, `0x615b10`, `0x616800`, `0x401ae7`, `0x401af1`, `0x41fca0`, `0x6187a0`, `0x618801`, `0x618808`, `0x619ca0`, `0x670630`, `0x615b4d`, `0x615b6a`
Confidence: verified
Proof: one
Test: none

Relative-call scans over `.text`: `0x6030c0` one site, `0x5fac70` two (`0x616795`, `0x618e77`), each
with no stored address. `0x616800` has no call; its address is the immediate of `0x401ae7`, passed
to the registrar `0x41fca0` (`0x401af1`). `0x5fac70` runs `0x5fa730`, then the liquid query
`0x670630` on the unit's node (`+0xe0`, type 15), and forms `surface − feetZ` through the vtable
`+0x14` position (`0x5facd4`, `0x5facd7`), or 0 outside liquid. `0x615b10` calls `0x616620` when
bit `0x8000000` of `+0x40` is clear (`0x615b4d`, `0x615b6a`).

## Swim start arms

A started swim is applied by one store but reaches it by one of three arms. For the unit that is
the active mover, `0x60df70` queues it through `0x619fb0` (and signals tutorial flag `0x1b`). For
any other unit it calls `0x61a130` at once when its force argument is nonzero or a spline with
bit `0x4` clear is running, and queues it through `0x61a030` only for a unit with no spline or a
finished one. A spline-driven creature therefore starts swimming in the same call on both paths.
A unit that initializes dead and not a player also starts at once, when it lies deeper than
`0.75·h` and a gate bit is set.

Addresses: `0x60df70`, `0x60df78`, `0x60df83`, `0x60df95`, `0x60dfa1`, `0x60dfaa`, `0x60dfad`, `0x60dfbd`, `0x60dfc1`, `0x60dfd1`, `0x60dfe0`, `0x61a130`, `0x61a030`, `0x619fb0`, `0x5fb880`, `0x5fb893`, `0x5fb89c`, `0x5fb8a8`, `0x5fb8ad`, `0x5fb8eb`, `0x5fb905`, `0x5fb913`, `0x5fb943`, `0x4b5390`
Confidence: verified
Proof: one
Test: none

`0x60df70` compares the object's GUID words with `0xc4da98` and `0xc4da9c`; a match takes
`add ecx,0x9a8; call 0x619fb0` and then `0x4b5390` with `ecx` 0x1b. Otherwise `0x60dfaa` tests the
force argument, then `[CMovement+0xa4]` (null queues) and its bit `0x4` (clear applies at once).
`[unit+0x118]` is `unit+0x9a8` (`collision/mover`). The initializer copy `0x5fb880` skips a unit
with TYPEMASK_PLAYER (`shr ecx,4; test cl,1`) or health above 0 (`0x5fb8a8`, `0x5fb8ad`), queries
liquid, compares `depth` with `h·0.75` (`fcompp`, `test ah,0x41`) and applies the same gate before
`0x61a130`.

## Camera water bits

`0x511ad0(unit)` is a camera method with the same shape as the swim decision and no part in it. It clears bits 0x100000 and 0x200000 of the camera's flags at `+0x90`, takes the unit's
liquid surface from `0x670630`, and returns `depth = surface − feet z` (0 for no unit or outside liquid). With `t` the camera's threshold at `+0x1c8`, it sets 0x100000 when `depth < t + 2/9`
and 0x200000 when `t + 2/9 ≤ depth < t + 5/9`; a deeper unit sets neither. The callers are the camera update `0x50ca90`, which discards the result, and the pull-in solver `0x50e8d0`.

Addresses: `0x511ad0`, `0x511ae3`, `0x511b39`, `0x511b58`, `0x511b61`, `0x511b76`, `0x511b83`, `0x511b90`, `0x511ba4`, `0x670630`, `0x50ca90`, `0x50d0a9`, `0x50e8d0`, `0x50eb45`, `0x8089d0`, `0x8089d4`
Confidence: verified
Proof: one
Test: none

`0x511ae3 and ecx,0xffcfffff` clears both bits. `0x511b39 call 0x670630` returns the surface in the local at `[ebp+8]`, and `0x511b58 call [edx+0x14]` is the unit's position. The compare chain is
`0x511b61 fcom [esi+0x1c8]` (`depth` below `t` jumps to the 0x100000 store at `0x511ba4`), then `0x511b76 fcom [0x8089d0]` (2/9, the same store) and `0x511b83 fcomp [0x8089d4]` (5/9,
the 0x200000 store at `0x511b90`). `0x50d0a9` is followed by `fstp st(0)` and `0x50eb45` stores the result.
