# collision: steps

What the walk resolver does at the end of a substep when the ground below is not where it left it:
the step-down reach, the election between a silent snap and a fall, the steep-support bit, and
the step-off recourse that decides whether a steep contact below is walked or fallen from. The
loop, the probe and the lookahead are `collision/walking`; the sweep is `collision/contact`.

Roots: none

## Step-down election

After the horizontal loop `0x6367b0` sweeps straight down by the reach `L` (`collision/walking#settle-and-the-fall-election`),
lowers `pos.z` by the achieved distance at once (`0x636e52`), then elects. No committed face:
the fall flag is set. A committed face: its world record's `n.z` is compared with cos 50°
(`0x80dffc`); above it, or NaN, `0x4000000` is cleared and the step is a silent snap. At or below
it the step probe `0x636100` runs with that normal and the current horizontal heading: 0 returns the
budget, 1 sets the fall flag, 2 sets `0x4000000`. The tail starts a fall, `0x7c61f0(0)`, when the
last hit index is not below the face count or the fall flag is set.

Addresses: `0x636dcd`, `0x636e45`, `0x636e4c`, `0x636e52`, `0x636e63`, `0x636e6d`, `0x636ecf`, `0x636ed2`, `0x636eda`, `0x636edf`, `0x636f03`, `0x636f08`, `0x636f10`, `0x636f21`, `0x636f36`, `0x636f3c`, `0x636f49`, `0x637120`, `0x637124`, `0x637129`, `0xc4e534`, `0xc4e530`, `0x80dffc`
Confidence: verified
Proof: one
Test: `collision::walkmode_difftest`

The pos.z write `0x636e52 fld [pos.z]; fsub [ebp+0x10]; fstp` happens whatever the face, so a fall from a
ledge starts `L` below the edge. `0x636ecf` loads `n.z` of the record at `index·0x34 + [0xc4e534]`
(`0x636ec2` to `0x636ecd`). A vertical face cannot be committed by a down sweep (next entries),
so walking off a vertical edge finds the ground below within `L` or nothing. The ordering at
`0x636f08` is `sub eax,edi (0); je 0x637129`, `dec eax; je 0x636e6d`, `dec eax; jne 0x636e74`,
`or [esi+0x40],0x4000000` (`0x636f21`). The path calls only `0x632ba0` (three sites), `0x6173b0`
(two), `0x617430`, `0x636100`, `0x7c5cd0`, `0x5fa550`, `0x7c61b0` and `0x7c61f0`: a snap sets no fall
flag, and `0x633220` and `0x633240`, which set `0x4000`, have callers only in the fall code (`0x6359f5`,
`0x635a91`, `0x635bb3`).

## Step probe returns

The step probe `0x636100` returns 0, 1 or 2 and nothing else, from eight returns, and only the
returns of 1 and 2 restore the position. 0 comes from a sweep that found no valid world region
(not a blocked one): the position then stays where the probe's partial sweeps left it, and the
caller returns the budget with no re-anchor or facing. 1 means refused, with the entry position
back. 2 means allowed, with the entry position back, and records the entry z at `+0x80` when
`0x4000000` is still clear. `0x635f80` returns the verdict of its lookahead without leaving the mover
falling.

Addresses: `0x636100`, `0x63610e`, `0x63623c`, `0x6362da`, `0x6362fa`, `0x636349`, `0x6364b8`, `0x636570`, `0x6365dc`, `0x63660d`, `0x6365c0`, `0x6365df`, `0x6365f0`, `0x6365f9`, `0x6365fc`, `0x635f80`, `0x635ea0`, `0x635f10`, `0x63605a`
Confidence: verified
Proof: one
Test: `collision::multipass_difftest`

The returns at `0x63623c`, `0x6362da`, `0x636349`, `0x6364b8` and `0x636570` follow a sweep whose result was
0 (`0x632ba0`'s failure); `0x6362fa` and `0x6365dc` return 1, `0x63660d` returns 2. `0x6365c0` to
`0x6365ce` and `0x6365df` to `0x6365ed` copy the saved position `[ebp-0x40]` back through `edi`. At
`0x6365f0 test [esi+0x40],0x4000000; jne` the store `mov [esi+0x80],ecx` (the saved z) is skipped when the
bit is set. The saved position is written once, at the entry (`0x63611b` to `0x636121`). `0x635ea0`
saves `+0x40` and `0x635f10` restores it at `0x63605a` before both exits of `0x635f80`.
`0x635f80` has one call site (`0x6365b7`), and `0x636100` two (`0x636b6e`, `0x636f03`).

## Steep support bit

Flag `0x4000000` means the mover's support is a steep contact the step probe allowed, not a
walkable floor. Only the two allowed arms set it, `0x636bbf` in the slide loop and `0x636f21` in
the settle. A walkable settle (`0x636edf`), the slope test (`0x633507`, `0x633566`) and a fall start
(`0x7c6206`) clear it. While set it adds H to the down reach (`0x636dfc`), lets the rise function
return a pop (`collision/climb#rise-sources`), and keeps `+0x80` from being rewritten. Only
these two `or` sites on `0x4000000` exist in the movement code.

Addresses: `0x636bbf`, `0x636f21`, `0x636edf`, `0x633507`, `0x633566`, `0x7c6206`, `0x636dfc`, `0x636d10`, `0x6365f0`, `0x635c0e`
Confidence: verified
Proof: one
Test: none

An `or` immediate scan over the unit, movement and resolution code and the shared movement file for masks with
`0x4000000` finds `0x636bbf` and `0x636f21` and no other on a `CMovement` flag word (the `and` clears are
`0x636edf`, `0x7c6206` and the sites in `collision/commands`). The reader `0x635c0e`'s flag test
and `0x6365f0` are the other consumers.

## Down-probe manifold

A sweep straight down can only report bevel planes. In `0x632ba0` a mover plane takes part only when
`dot(displacement, n) > 0` strictly (`0x632dd5`, `0x6322f7`), a world face only when `dot ≤ −1e-5`
(`0x63276d`). For `(0, 0, −1)` the box sides (`n.z` = 0) and the top cap (`n.z` = +1) fail and the
four bevels (`n.z` = −0.4756366) pass, and each committed manifold entry is a verbatim copy of the
plane. `0x636610` compares each entry's z with the bevel constant by the same bit pattern, so it
reports all-bevel for every non-empty down manifold. The face record `0xc4e534` is filled by the
world trace only, so the classified normal at `0x636ecf` is the world face's.

Addresses: `0x632ba0`, `0x632dd5`, `0x632de0`, `0x632280`, `0x6322f7`, `0x632302`, `0x63276d`, `0x632778`, `0x632e57`, `0x632e6e`, `0x632e94`, `0x632eae`, `0x636610`, `0x636635`, `0x63664a`, `0x6721b0`, `0x6722f5`, `0x6722fa`, `0x6327f0`, `0xc4e534`, `0x80e014`
Confidence: verified
Proof: one
Test: `collision::kdop_difftest`

Both ordered compares read `fcomp [0x7ffd74]; test ah,0x41; jnp`. `0x6722f5 mov ecx,0xd; 0x6722fa rep movs`
copies a 13-dword record (0x34 bytes) into `0xc4e534`. The committed index has one writer,
`0x6327f0 mov ecx,[ebp+0x1c]; mov [ecx],ebx`, with `ebx` the world loop's counter seeded to the face
count. `0x636635` loads `fld [edi]; fsub [0x80e014]; fabs` and `0x63664a` bails when the residual is
2^-20 or more. A caller that classifies the manifold instead of the record sees `n.z` = −0.4756366 on every
step and refuses it against cos 50°.

## Probe sweep order

The step probe's reach does not depend on the frame's travel: `ebx = max(H·1.1917536, r + 1/720)`, with `H` from `0x617430` and `r` the mover's `+0xb0`; a tie or NaN takes `r + 1/720`.
A face with `n.z` in [0, cos 50°] first runs the heading sweep, Phase A (`0x63622d`), along `(−n.x, −n.y)/|n_h|` for `ebx`. That heading is kept only when the sweep re-hits the same face with an
exactly equal normal; a miss or another face restores the caller's heading, and any other `n.z` skips the sweep. The lift, Phase B (`0x6362cb`), rises by `H` and writes the achieved rise over the
normal slot. The forward sweep, Phase C, runs along the kept heading for `ebx`.

Addresses: `0x636100`, `0x636144`, `0x636147`, `0x63614f`, `0x636163`, `0x636168`, `0x63616e`, `0x636176`, `0x63618a`, `0x636190`, `0x636196`, `0x6361b2`, `0x6361ee`, `0x6361fb`, `0x63622d`, `0x636257`, `0x63627b`, `0x63627d`, `0x6362cb`, `0x6362dd`, `0x636311`, `0x636327`, `0x63633a`, `0x617430`, `0x80dfec`, `0x80e008`, `0x80dffc`
Confidence: verified
Proof: one
Test: none

`0x636147 fld [esi+0xb0]; fadd [0x80dfec]` stores `r + 1/720` at `[ebp-4]`; `0x636168 fmul [0x80e008]` forms `H·1.1917536` and `0x63616e fcomp [ebp-4]` with `test ah,0x41; jne 0x63618a` keeps the stored value for a tie or NaN.
`0x636196 fcomp [0x80dffc]` with `jp 0x63628b` skips Phase A for `n.z` above cos 50° or NaN, and `0x6361b2 test ah,1; jne` for a negative one. The exact-equality tests `0x636257` to `0x63627b` are three
`fcomp` with `test ah,0x44; jp 0x63627d`. `0x636327 push ebx` is the forward sweep's distance; the lift's `0x6362c4` sweep pushes `H`. The probe's by-value normal is its `[ebp+0xc]` to `[ebp+0x14]`.

## Step-off bound

A fall after a ledge is elected by the reach `L`; a steep contact inside `L` is walked only when the step probe allows it. The probe down-sweeps by the lift plus 0.01 after its forward
sweep; a clear or walkable result allows the step. A kerb whose remaining drop is under H lands on its top, returns 2 and does not fall. A long steep slope leaves the sweep blocked; the
lookahead then tests a free fall against the uphill heading, and a downhill drift refuses it. Which result a slope gives at a given pitch is not settled.

Addresses: `0x636100`, `0x63628d`, `0x636290`, `0x6364f7`, `0x6364fd`, `0x63651f`, `0x636524`, `0x635f80`, `0x6360d4`, `0x8029d0`, `0x80e01c`
Confidence: inferred
Proof: none
Test: none

Phase B passes `&[ebp+0x14]` as the sweep's out slot (`0x63628d lea eax,[ebp+0x14]`), so the slot holds the lift when the down sweep runs
(`0x63651f fld [ebp+0x14]; fadd 0.01`), plus each reproject rise added at `0x6364fa`. The worked kerb and slope cases compose these branches and are not traced
against geometry.

## Walk position writes

The walk resolver changes the mover's position at seven places only, and every one is relative.
After each sweep it adds `dir · achieved` to x, y and z. The settle subtracts the down sweep's
achieved distance from z, unconditionally once the query succeeded. A hovering mover adds the
achieved distance back and subtracts `achieved − 1.0` when that is not negative, and a hover servo
adds its achieved rise. No write assigns an absolute floor height. The length left to sweep and the
time left keep the ratio the call started with, so the horizontal advance of one call sums to at
most its `dist` argument; the pop adds length only through the vertical response.

Addresses: `0x6367b0`, `0x636909`, `0x636915`, `0x63691e`, `0x636e5b`, `0x636e87`, `0x636ea9`, `0x6370f1`, `0x63681d`, `0x636993`, `0x6369ab`, `0x6369ae`, `0x636c14`, `0x63698b`, `0x632bc9`
Confidence: verified
Proof: one
Test: none

A scan of `[esi+0x10]`, `[esi+0x14]` and `[esi+0x18]` stores inside `0x6367b0` to `0x637140`
finds exactly these seven. `0x63681d` seeds the length with `[ebp+0x10]`; a hit takes `Δt = (a/L)·t`
(`0x636993`, `0x63699b fdiv`, `0x6369ab fmul`) and `L ← L − a` (`0x6369ae` to `0x6369b4`); a deflection
rescales the time by `L'/L` (`0x636c14`); the miss reset `0x63698b` sets `L = (remaining/total)·dist`.
`0x632ba0` seeds its best-t with the requested length (`0x632bc9`), so an achieved distance is
never above the request.

## Settle reach

The reach of the end-of-call down sweep is `d·1.849399 + 1/36`, plus 1.0 while hovering and plus H
while `0x4000000` is set, and the H addend is gated on that one test of a flag word read once. The
sweep that ends a step-up frame carries H, because the latch is set before the settle and the
settle clears it only after the reach is sized, unless a walkable hit of the slope test
(`0x6334a0`) cleared it earlier in the call. Then the reach is at least `H + 1/36`, above the rise,
and the frame ends where the down sweep first rests on geometry: back on the ground it left, on
the obstacle's top, or in a fall.

Addresses: `0x632f3a`, `0x632f53`, `0x636dcd`, `0x636dd2`, `0x636dd7`, `0x636dda`, `0x636de0`, `0x636de9`, `0x636df7`, `0x636dfc`, `0x636e00`, `0x636944`, `0x636d90`, `0x636edf`, `0x636bbf`, `0x6368a3`
Confidence: verified
Proof: one
Test: none

`0x636dcd` has one predecessor, the miss arm `0x636944`, so `d` is the full requested length of the
last segment, except that the sweep writes 0 for a distance below 1/720 (`0x632f3a` to `0x632f53`), so a last
segment shorter than 1/720 gives a reach of exactly 1/36; the flags are read at `0x636dcd` and tested at `0x636dd2` and `0x636df7`. At 60 frames
a second and 7 yards a second the reach without bits is 0.2435410 yards (`d` = 0.1166667). The
frame's height is `entry z + rise − achieved down`, with `rise ≤ H`: a down sweep that finds
nothing drops the whole reach and falls; one that finds the original ground leaves the net rise 0.
