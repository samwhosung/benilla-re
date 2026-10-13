# collision: commands

The movement commands: the setters that the command queue, the status handlers and the mover
call to change a `CMovement`'s flags word `+0x40`, its speeds, facing and pitch, its spline, and
its swim, root and feather-fall state. They sit at the end of the shared movement object file,
after the fall code of the `falling` topic.

A command changes flag bits and then refreshes two derived quantities through the kinematics
methods: the anchor, which also recomputes the direction, and the current speed. While the
mover falls, both refreshes keep the direction and the speed, so a command taken in the air
changes the flags and little else: the stop commands and the start commands with a direction
already held record a deferred bit instead. Two flag setters, `0x7c6500` and `0x7c6510`, are
also called from the unit object code; every other command is reached from collision.

Roots: `0x7c6500`, `0x7c6510`

## Movement flag bits

The bits of `+0x40` the commands and the fall code set, clear or test. "Deferred" bits are set
in place of a press or release while falling (0x2000).

| bits | set by | cleared by | tested by |
|---|---|---|---|
| 0x1, 0x2 forward, back | `0x7c6ae0` | `0x7c6c00`, `0x7c7340`, `0x7c69a0` | `0x7c6ba0`, `0x7c6b70` |
| 0x4, 0x8 strafe | `0x7c6c50` | `0x7c6ce0`, `0x7c7340`, `0x7c69a0` | `0x7c6d30`, `0x7c6d00` |
| 0x10, 0x20 turn | `0x7c6d90` | `0x7c6dd0`, `0x7c6f30`, `0x7c7340`, `0x7c69a0` | |
| 0x40, 0x80 pitch | `0x7c6df0` | `0x7c6e30`, `0x7c6f70`, `0x7c6e80`, `0x7c7340`, `0x7c69a0` | |
| 0x100 walk | `0x7c71c0` with 0 | `0x7c71c0` with 1 | `0x7c4c90` |
| 0x400, 0x800 | `0x7c7200`, `0x7c7240` with 0 | the same with 1 | `0x7c61c0` (0x800) |
| 0x1000 root | `0x7c7340` | `0x7c7370` | `0x7c61c0`, `0x7c6230` |
| 0x2000 falling, 0x4000 | `0x7c61f0` (0x2000) | `0x7c6290` | most of this file |
| 0x8000, 0x10000 deferred stops | `0x7c6ba0`, `0x7c6d30` | the start and force-stop commands, `0x7c7340`, `0x7c69a0` | |
| 0x20000 to 0x100000 deferred starts | `0x7c6ae0`, `0x7c6c50` | the same, the stops, `0x7c7340`, `0x7c69a0` | |
| 0x200000 swim, 0x4000000 | `0x7c6e50` (0x200000) | `0x7c6e80`, `0x7c61f0`, `0x7c69a0`, `0x7c6e50` | `0x7c6230`, `0x7c6df0` |
| 0x8000000 to 0x40000000 | `0x7c6500`, `0x7c7280`, `0x7c72e0`, `0x7c7310` | the same | fall code, `0x7c6230`, `0x7c6f70` |

Addresses: `0x7c6ae0`, `0x7c6c50`, `0x7c6d90`, `0x7c6df0`, `0x7c71c0`, `0x7c7340`, `0x7c7370`, `0x7c61f0`, `0x7c6290`, `0x7c6e50`, `0x7c6500`, `0x7c6510`
Confidence: verified
Proof: one
Test: none

Each set or clear cell is an `or` or `and` on `+0x40` in the named function, each test cell a
`test`; the entries below give the masks. A nonzero argument picks the lower bit of each pair: 0x1 (`0x7c6b3e`), 0x4 (`0x7c6cae`),
0x10 (`0x7c6da0`), 0x40 (`0x7c6e09`). 0x200000 is cleared by `0x7c61f0`'s `and 0xfbdfffff`
along with 0x4000000; 0x4000000 is also cleared by `0x7c6e50` and, before a fall starts, by
`0x7c6ba0` and `0x7c6d30`. No `or` in this file sets 0x4000 or 0x4000000. `0x7c6340` reads
0x4000000 to send `+0x80`. The walk bit selects the walk speed in `0x7c4c90` (`0x7c4d11`
`test dh,1`). `0x7c6500` and `0x7c6510` are one `or` and one `and` with 0x8000000 and `ret`.

## Anchor and speed refresh

`0x7c5cd0(e)` copies the position, facing and pitch to the anchor `+0x44`, `+0x50`, `+0x54`,
zeroes the anchor time `+0x58`, and recomputes the direction through `0x7c5a20(e)`.
`0x7c5c20(e)` recomputes the current speed `+0x84` through `0x7c4c90(e)`. While 0x2000 is set
and e is 0, both keep their old values; the anchor itself is always taken. Only the start
commands pass e = 1, and only when falling with no direction held. The order per command:

| command | sequence |
|---|---|
| `0x7c6ae0`, `0x7c6c50` | flags, anchor(e), speed(e) |
| `0x7c6c00`, `0x7c6ce0` | flags, speed(0), anchor(0) |
| `0x7c6d90`, `0x7c6dd0`, `0x7c6df0`, `0x7c6e30` | flags, anchor(0) |
| `0x7c6e50` | flags, anchor(0), `0x7c6290`, speed(0) |
| `0x7c6e80` | flags, pitch 0, `0x7c61c0`, speed(0) |
| `0x7c6f30` | facing, anchor(0) unless falling, clear 0x30 |
| `0x7c6f70` | pitch, anchor(0) if swimming, clear 0xc0 |
| `0x7c6ff0` | rate, anchor(0) |
| `0x7c7030` to `0x7c7170`, `0x7c71c0` | value, anchor(0), speed(0) |
| `0x7c7240` | flags, anchor(0) if they changed |
| `0x7c7340` | 0x1000, `0x7c6290`, wipe, speed(0) |
| `0x7c69a0` | pose, flags, `0x7c6290`, anchor(0), speed(0) |

Addresses: `0x7c5cd0`, `0x7c5c20`, `0x7c5a20`, `0x7c4c90`, `0x7c6c10`, `0x7c6c17`
Confidence: verified
Proof: one
Test: none

`0x7c5cd0` stores `+0x10` to `+0x44`, `+0x1c` to `+0x50`, `+0x20` to `+0x54` and 0 to `+0x58`
before `call 0x7c5a20`. `0x7c5c20` tests `ah,0x20`, then the argument, and skips
`call 0x7c4c90; fstp [ecx+0x84]` for 0 while falling; `0x7c5a20` does the same at `0x7c5a2c`.
In `0x7c6c00` the speed refresh `0x7c6c10` comes first, and `0x7c6c17` calls the anchor with
`ecx` not reloaded: `0x7c5c20` and `0x7c4c90` never write `ecx`, so `this` carries over. The
same holds in `0x7c6ce0`. With e = 1, `0x7c4c90`, past its spline and swim tests, takes its walk-speed arm (`0x7c4d16`
`test eax,eax; jne 0x7c4d4d`): the smaller of `+0x88` and `+0x8c`, `+0x8c` on a tie or NaN.

## Start and stop move

`0x7c6ae0(forward)` clears 0x8000, 0x20000 and 0x40000. Grounded, or falling with no bit of 0xf
held, it sets 0x1 and clears 0x2 for a nonzero argument (the reverse for zero), refreshes with
e = 1 when falling (`0x7c6afc`), and returns 1. Falling with a direction held, it records
0x20000 (forward) or 0x40000 (back) unless already moving that way, and returns 0. `0x7c6ba0`
without 0x1 or 0x2 clears the deferred starts. Otherwise it clears a set 0x4000000 and runs
`0x7c61c0`; then, falling, it swaps the deferred starts for 0x8000, and grounded it runs
`0x7c6c00` and returns 1, its only 1. `0x7c6c00` clears 0x1, 0x2 and 0x8000,
refreshes, and ends a live spline (flag 0x4), falling if it has 0x200.

Addresses: `0x7c6ae0`, `0x7c6afc`, `0x7c6ba0`, `0x7c6c00`, `0x7c6b70`, `0x7c61c0`
Confidence: verified
Proof: one
Test: none

`0x7c6ae9` `and edx,0xfff97fff`, stored at `0x7c6af7` before any branch. `0x7c6af4`
`test ah,0x20` and `0x7c6afc` `test al,0xf`; a held nibble with the argument nonzero tests
`al,1` and either returns or does `or eax,0x20000` (`0x7c6b0b`); zero tests `al,2`, then
`or eax,0x40000`. `0x7c6b2f` `mov edi,1` is the falling arm; `0x7c6b3b`/`0x7c6b43` set the
bits; `push edi` feeds both refreshes. In `0x7c6ba0`: `test al,3`, `test eax,0x60000`;
`0x7c6bc4` `and 0xfbffffff`, `call 0x7c61c0`; the reload `0x7c6bd3` then `and 0xfff9ffff`,
`or 0x8000`. `0x7c6c00`: `and 0xffff7ffc`; spline `+0x18` `test al,4`, `or eax,4`, then
`test ch,2` and `jmp 0x7c61c0`. `0x7c6b70` runs it when `test byte [ebp+8],3` is nonzero and
`test ah,0x20` is zero, and returns 1 then.

## Start and stop strafe

`0x7c6c50(left)` mirrors `0x7c6ae0` on the strafe pair: it clears 0x10000, 0x80000 and 0x100000,
sets 0x4 and clears 0x8 for a nonzero argument (0x8 and clears 0x4 for zero), and while falling
with a direction held records 0x80000 or 0x100000 instead and returns 0. `0x7c6d30` mirrors
`0x7c6ba0`: without 0x4 or 0x8 it clears the deferred strafe starts; otherwise it clears a set
0x4000000 and runs `0x7c61c0`, then records 0x10000 while falling or runs `0x7c6ce0`. `0x7c6ce0`
clears 0x4, 0x8 and 0x10000 and refreshes; unlike `0x7c6c00` it does not touch the spline.
`0x7c6d00(bits)` runs `0x7c6ce0` for bits with 0x4 or 0x8 when not falling.

Addresses: `0x7c6c50`, `0x7c6d30`, `0x7c6ce0`, `0x7c6d00`
Confidence: verified
Proof: one
Test: none

`0x7c6c59` `and edx,0xffe6ffff`; the deferral tests `al,4` and `al,8` and does
`or eax,0x80000` (`0x7c6c7b`) or `or eax,0x100000` (`0x7c6c8f`); the falling arm is `0x7c6c9f`
`mov edi,1`. `0x7c6d30`: `test al,0xc`, `test eax,0x180000` and `and eax,0xffe7ffff`;
`0x7c6d54` `and 0xfbffffff`, `call 0x7c61c0`; `0x7c6d6b` `and 0xffe7ffff`, `or 0x10000`.
`0x7c6ce3` `and eax,0xfffefff3`, then `0x7c6ced` `call 0x7c5c20` and `0x7c6cf4`
`call 0x7c5cd0`, and `ret`. `0x7c6d03` `test byte [ebp+8],0xc`.

## Turn and pitch commands

`0x7c6d90(left)` sets 0x10 and clears 0x20 for a nonzero argument, 0x20 and clears 0x10 for
zero, and re-anchors; it tests no other bit, so turning starts while rooted or falling.
`0x7c6dd0` clears 0x10 and 0x20 and re-anchors only when one was set, and returns 1 then, else
0. `0x7c6df0(up)` acts only while swimming (0x200000): it sets 0x40 and clears 0x80 for nonzero,
0x80 and clears 0x40 for zero, and re-anchors. `0x7c6e30` is `0x7c6dd0` for 0x40 and 0x80.
None of the four touches the speed.

Addresses: `0x7c6d90`, `0x7c6dd0`, `0x7c6df0`, `0x7c6e30`
Confidence: verified
Proof: one
Test: none

`0x7c6d9d` `and eax,0xffffffdf; or eax,0x10` and `0x7c6db4` `and edx,0xffffffef; or edx,0x20`,
each followed by `push 0; call 0x7c5cd0`; `ret 4`. `0x7c6dd3` `test al,0x30`, `0x7c6dda`
`and eax,0xffffffcf`. `0x7c6df6` `test eax,0x200000; je 0x7c6e2c`, then `0x7c6e04`
`and 0xffffff7f; or 0x40` or `0x7c6e1a` `and 0xffffffbf; or 0x80`. `0x7c6e33` `test al,0xc0`,
`0x7c6e3a` `and eax,0xffffff3f`.

## Swimming on and off

`0x7c6e50` clears 0x4000000 and sets 0x200000, re-anchors, runs the landing `0x7c6290` (which
acts only when 0x2000 is set), and updates the speed. `0x7c6e80` clears 0x200000, 0x40 and 0x80,
zeroes the pitch `+0x20` and the anchor pitch `+0x54`, calls `0x7c61c0`, and updates the speed.
When `0x7c61c0` refuses (root, falling, swimming or 0x800 set, or a live spline with 0x200), nothing re-anchors: the anchor keeps
its position and facing with its pitch zeroed, and the direction keeps its swim tilt.

Addresses: `0x7c6e50`, `0x7c6e80`, `0x7c6e9a`
Confidence: verified
Proof: one
Test: none

`0x7c6e56` `and eax,0xfbffffff`, `0x7c6e5b` `or eax,0x200000`, then `call 0x7c5cd0`,
`call 0x7c6290`, `call 0x7c5c20`. `0x7c6e83` `and [esi+0x40],0xffdfff3f`; `0x7c6e8c` and
`0x7c6e93` store 0 at `+0x54` and `+0x20`; `0x7c6e9a` `call 0x7c61c0` with its result
ignored; `0x7c6ea3` `call 0x7c5c20`. `0x7c61c0` re-anchors only inside `0x7c61f0`.

## Facing and pitch setters

`0x7c6f30(f)` stores f at `+0x1c` when |f − `+0x1c`| is 2⁻²⁰ or more, or NaN, and then
re-anchors unless falling; it always clears 0x10 and 0x20. `0x7c6eb0(f)` subtracts the
transport's facing (`0x630b70`, or 0.0 without a transport at `+0x38`), rounds to float, wraps
once (minus 2π above 2π, plus 2π below 0), and calls `0x7c6f30`. `0x7c6f70(p)` returns 0 after
clearing 0x40 and 0x80 when |p − `+0x20`| is below 2⁻²⁰. Otherwise it stores p (`0x7c6f91`).
Swimming, it re-anchors. Not swimming, with water walking (0x10000000), none of 0x200f, and
p ≤ −0.6457718 (−37°, `0x80dfe8`), it calls `0x7c61c0` and returns 1 when that does. It clears
0x40 and 0x80 on every path. It applies no clamp.

Addresses: `0x7c6f30`, `0x7c6eb0`, `0x7c6f70`, `0x7c6f49`, `0x7c6f89`, `0x7c6f91`, `0x7c6fcf`, `0x80dfe8`
Confidence: verified
Proof: one
Test: `collision::tf_setfacing_should_store_difftest`, `collision::tf_setfacing_to_world_difftest`, `collision::tf_setpitch_should_store_difftest`

`0x7c6f36` `fld [esi+0x1c]`, `0x7c6f39` `d8 6d 08` `fsubr [ebp+8]` (f − facing), `fabs`,
`fcomp` 2⁻²⁰ (`0x35800000`), `test ah,5`, `jnp 0x7c6f60` at `0x7c6f49`: odd parity is C0 alone,
less, which skips the store; equal and unordered store. `0x7c6f54` `test ah,0x20` guards the
anchor; `0x7c6f60` `and 0xffffffcf`. `0x7c6f70` repeats the deadband at `0x7c6f89` and stores
at `0x7c6f91`; `0x7c6f97` `test eax,0x200000`; `0x7c6fb3` `test eax,0x10000000`, `0x7c6fba`
`test eax,0x200f`, then `fcomp [0x80dfe8]`, `test ah,0x41`, `jp` at `0x7c6fcf`: taken for
greater and unordered, so less and equal start the fall. In `0x7c6eb0`, `0x7c6ed4` `d8 e1` is
f − transport facing, stored to the argument slot at `0x7c6ed6`.
The tests run `0x7c6f36` to `0x7c6f49` and `0x7c6f76` to `0x7c6f8c`.

## Speed and run-mode setters

Five setters store a speed and one the turn rate, each only when the new value differs from the
field by 2⁻²² or more, or is NaN; they return 1 then and 0 otherwise. All re-anchor; the speed
setters also update the current speed, which while falling keeps its value. The fields are named
from their use in `0x7c4c90` and `0x7c5c50`:

| setter | field |
|---|---|
| `0x7c70d0` | `+0x88` walk |
| `0x7c7030` | `+0x8c` run |
| `0x7c7080` | `+0x90` run back |
| `0x7c7120` | `+0x94` swim |
| `0x7c7170` | `+0x98` swim back |
| `0x7c6ff0` | `+0x9c` turn rate, no speed update |

`0x7c71c0(run)` clears the walk bit 0x100 for a nonzero argument and sets it for zero, then
re-anchors and updates the speed.

Addresses: `0x7c70d0`, `0x7c7030`, `0x7c7080`, `0x7c7120`, `0x7c7170`, `0x7c6ff0`, `0x7c71c0`, `0x7c5c50`, `0x7c7009`
Confidence: verified
Proof: one
Test: `collision::move_setters_difftest`

Each setter is `fld [field]`, `fsubr [ebp+8]`, `fabs`, `fcomp` 2⁻²² (`0x34800000`), `test ah,5`,
`jp` to the store: taken for greater, equal and unordered (`0x7c7009` in `0x7c6ff0`). The store
is a dword copy, then `push 0; call 0x7c5cd0` and, except in `0x7c6ff0`, `push 0;
call 0x7c5c20`. In `0x7c4c90`, swimming picks `+0x94`, or the smaller of `+0x98` and `+0x94`
backwards; walking picks the smaller of `+0x88` and `+0x8c`; otherwise `+0x8c`, or the smaller
of `+0x90` and `+0x8c` backwards. `0x7c5c50` loads `+0x9c` for 0x10 and its negation for 0x20.
`0x7c71c0`: `and eax,0xfffffeff` or `or eax,0x100`, stored at `0x7c71e0`, then both refreshes.

## Root and state flags

`0x7c7340` sets 0x1000, runs the landing `0x7c6290`, then clears 0xff, 0x8000 and 0x10000 to
0x100000 (direction, turn, pitch and the deferred bits), and updates the speed; it does not
re-anchor after the wipe. `0x7c7370(fall)` clears 0x1000 and, for a nonzero argument, calls
`0x7c61c0`, which can now start a fall because root is already clear. `0x7c7240(clear)` clears
0x800 for nonzero or sets it for zero, stores the word either way, and re-anchors and returns 1
only when the word changed. `0x7c7280`, `0x7c72e0` and `0x7c7310` set their bit for a nonzero
argument and clear it for zero: 0x10000000 water walking, 0x20000000 the 7.0 terminal speed,
0x40000000 hover. `0x7c6500` and `0x7c6510` set and clear 0x8000000.

Addresses: `0x7c7340`, `0x7c7370`, `0x7c7240`, `0x7c7280`, `0x7c72e0`, `0x7c7310`, `0x7c6500`, `0x7c6510`, `0x7c7384`, `0x619a40`
Confidence: verified
Proof: one
Test: none

`0x7c7343` `or [esi+0x40],0x1000`, `0x7c734a` `call 0x7c6290`, `0x7c734f`
`and [esi+0x40],0xffe07f00`, `0x7c735a` `call 0x7c5c20`. `0x7c7379` `and edx,0xffffefff` is
stored before `0x7c7384` `je` tests the argument and `0x7c7386` calls `0x7c61c0`. The relayed
unroot `0x619a40` pushes 1 (`0x619a41`); the two queue paths push 0 or 1. `0x7c7240`: `cmp eax,edx`
at `0x7c725d`, the store at `0x7c725f`, `jne 0x7c726a`. The three bit setters are `or` with the
bit or `and` with its complement and `ret 4`, with no refresh.

## Position reset

`0x7c69a0(pos, facing, free)` stores the position `+0x10` and the facing `+0x1c` as given, zeroes
the pitch, and clears 0xff, 0x8000, 0x10000 to 0x200000 and 0x4000000. It then runs the landing
`0x7c6290`, re-anchors, and updates the speed. A live spline (present, flag 0x4 clear) is freed
through `0x7c6310` when `free` is nonzero, and otherwise marked ended with flag 0x4. Its one
caller is `0x617e90`.

Addresses: `0x7c69a0`, `0x7c69c2`, `0x617e90`
Confidence: verified
Proof: one
Test: none

The position is three dword copies (`0x7c69ae` to `0x7c69bc`), the facing a dword at `0x7c69c9`,
the pitch `0x7c69cc` `mov [esi+0x20],0`. `0x7c69c2` `and eax,0xfbc07f00` keeps 0x100 to 0x7f00,
the falling bits among them, so `0x7c69d6` `call 0x7c6290` still sees a fall. Then
`0x7c69df` `call 0x7c5cd0`, `0x7c69e8` `call 0x7c5c20`; the spline test `0x7c69fa`
`test al,4`, the argument test `0x7c6a01`, and `0x7c6a11` `or eax,4`. `ret 0xc`.

## Spline commands

`+0xa4` holds a 0x70-byte spline record that `0x630c40` allocates, or null. `0x7c62c0(p)`
creates it when absent, zeroes its flags at +0x18 and stores the point p at +0x58. `0x7c6310`
frees it through `0x630ca0`, nulls `+0xa4` and updates the speed. `0x7c6a20(x)` stores x at +0x28 when the record exists, then calls `0x453360` with no points. `0x7c6a50` stores the flags at
+0x18 and three values at +0x1c, +0x24 and +0x28, zeroes +0x20, sets +0x54 to 1 when the flags
have 0x200 and to 0 otherwise, passes the points to `0x453360`, and then calls `0x7c71c0` with the flags' 0x100 bit
(`0x7c6ac2`) and `0x7c6ae0(1)`. A record whose flags have 0x200 and not 0x4 refuses falls and
jumps.

Addresses: `0x7c62c0`, `0x7c6310`, `0x7c6a20`, `0x7c6a50`, `0x7c6ac2`, `0x630c40`, `0x630ca0`, `0x6187a0`
Confidence: verified
Proof: one
Test: none

`0x453360` calls the path object's vtable slot `+0x1c`, which is not read here.

`0x630c40` passes 0x70 to the allocator with the `Movement_C.cpp` path. `0x7c62e4` zeroes
+0x18; `0x7c62f6` to `0x7c6301` copy the point. In `0x7c6a20` the path call at `0x7c6a40` runs
on `[+0xa4]+0x2c` whether or not the record exists. `0x7c6a50`: `0x7c6a63` stores the fifth
argument at +0x18, `0x7c6a6c` the first at +0x1c, `0x7c6a83` the fourth at +0x24, `0x7c6abf` the
sixth at +0x28; `0x7c6a8f` `test dh,2`; the path call gets the second and third. `0x7c6ac2`
`and edi,0x100` is the run argument, so a spline without 0x100 sets the walk bit. Its caller
is `0x6187a0`.
