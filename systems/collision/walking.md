# collision: walking

The ground and fall modes of the collision resolution layer, the unnamed object file from
`0x6312b0` through `0x637a60`. The resolve entry `0x634040` sweeps a substep's move through
`0x633840`, which gathers the candidate faces, then hands the substep to one mode: a mover with
flag 0x2000 (falling) of `+0x40` to the fall step `0x635b00`, any other to the walk resolver
`0x6367b0`, unless a spline or the flags 0x200800 send it to `0x634640`. Both modes take
`(time, ms, dist, dir)` on the `CMovement` and return the milliseconds they consumed.

The walk resolver sweeps, deflects off what it hits, tests steps with the probe `0x636100`,
settles onto the ground, and decides when the mover starts to fall. Its helpers give the rise
along a face (`0x635c00`), the horizontal push-out from a wall (`0x635d80`), the bevel contact
direction (`0x636610`) and a simulated fall (`0x635f80`). The vector, box and plane helpers at the
end of the file serve the whole layer; `0x6372d0`, `0x637480` and `0x6376a0` are also called from
outside collision. Every sweep is `0x632ba0` (contact), which reports the distance travelled, the
index of the face hit and the mover's contact planes. H below is the step height `0x617430`
returns.

Roots: `0x6372d0`, `0x637480`, `0x6376a0`

## Fall step

`0x635b00` advances a falling mover by `ms`. On a transport (`+0x38` nonzero) it calls
`0x633200`, and returns 0 when that detaches the mover; with `ms` 0 it returns 0. Otherwise
`F = 0x7c6140(+0x78 + ms)`, how far the fall curve at the new fall time lies below the current
z, and the step is `(float(dist·dir.x), float(dist·dir.y), float(−F))`. When the step is shorter
than 2^-22 only `+0x78` grows by `ms`, and `ms` is returned. Otherwise, with no candidate face (`0xc4e530` zero) the
step is added to `+0x10`, `+0x78` grows and `0x633240` runs, returning `ms`; with faces,
`0x635600(time, ms, step, 1)` moves the mover and its result is returned. Both paths then set
`+0x24` to (0, 0, 1).

Addresses: `0x635b00`, `0x635b4f`, `0x635b6b`, `0x635b85`, `0x635be1`, `0x7c6140`, `0x7c6110`, `0x633200`, `0x633240`, `0x635600`
Confidence: verified
Proof: one
Test: `collision::fall_mode_difftest`

`0x7c6140` returns `0x7c6110(ms, z)`, which with flag 0x2000 adds `z − +0x7c` to the curve's
downward distance D(T): the new z is `+0x7c − D(T)`, and the rising part of a jump, where D is
negative, raises z. `fchs` at `0x635b4f` negates the returned `st0`; `fst [ebp-0x4]` stores −F
while the register keeps it, and the length squares register times store, then adds dy² and dx²
(53 bits), `fsqrt`, `fabs`. At `0x635b6b`, `fcomp 2^-22 (0x34800000); test ah,5; jp 0x635b85`
falls through only below the threshold: equal and NaN move. The adds at `0x635b8e` to
`0x635ba8` round each coordinate to float. The vector's z is never read. `0x635be1` to
`0x635bef` store 0, 0, `0x3f800000`. `ret 0x10`. That `0x633200` detaches through `0x6173b0` and
that `0x633240` raises flag 0x4000 belong to the resolve topic.

## Walk resolver loop

`0x6367b0` moves a grounded mover `dist` along `(dir.x, dir.y, 0)` within `ms`; `time` is unused.
With `|dist| < 2^-20` it returns `ms` at once. With no candidate face it starts a fall,
`0x7c61f0(0)`, and returns 0. A hovering mover (flag 0x40000000) first sweeps 1.0 down and keeps
the travel c0. Each sweep adds `float(step·t)` to the position. Time is two floats, `rem = ms`
and `used = 0`.

| sweep result | effect |
|---|---|
| no hit | `used += rem`; with `ms − used ≥ 1`, resume `dir` with `rem = ms − used` and length `rem/ms·dist` (`0x63694d`); else settle (`0x636dcd`) |
| hit | `dt = t/len·rem`, `used += dt`, `rem −= dt`, `len −= t`, deflect, sweep again |
| hit, `ms − used < 1` | end, re-anchoring unless `|ms − used| < 2^-20` |
| counter past 5 | end, re-anchoring |

A short hit has `dt < 1`; the counter starts at 0, is 1 after any longer hit, grows per short
hit, and past 5 ends the loop.

Addresses: `0x6367b0`, `0x6367ce`, `0x6367df`, `0x636893`, `0x6368d2`, `0x636926`, `0x63694d`, `0x636993`, `0x6369e1`, `0x6369f7`, `0x636a25`, `0x636f77`, `0x636d90`
Confidence: verified
Proof: one
Test: `collision::walkmode_difftest`, `collision::walkmode_cos80_difftest`, `collision::walkmode_sweep_miss_difftest`

`0x6367ce`: `fcomp 2^-20 (0x35800000); test ah,5; jnp 0x637129` returns for `|dist|` below it;
NaN goes on. `0x6367d4` tests `0xc4e530`. The step is `0x4549a0` of `dir·t` products, then three
`fadd/fstp` on `+0x10`. `cmp ecx,[0xc4e530]; jb` at `0x636926` separates hits. On no hit,
`fcom 1.0; test ah,1; jne 0x636dcd` settles below 1 ms; `0x63694d` to `0x63698b` reload `dir`
with z 0 and store `x/ms·dist` (`fdiv [ebp-0x60]; fmul [ebp+0x10]`). On a hit `0x636993` keeps
`dt` in `st0`; `0x6369e1` sets the counter `[ebp-0x7c]` to 1 when `dt ≥ 1`, else increments it,
and `ja 0x636f2d` past 5. `0x636a25` (`jnp 0x636f77`) ends for `ms − used < 1`; `0x636f77`
re-anchors (`[ebp-0x20] = 1`) unless the difference is under 2^-20. A failed sweep returns `ms`
without the end of the walk. `ret 0x10`.

## Deflection on a hit

The hit face's normal n is the `0xc4e534` entry. The new step is
`M = dir·len + R`; `len = |M|`; `dir = M/len`; the horizontal direction becomes `M_h/|M_h|`, or
`M_h` when `|M_h| < 2^-22`. `len < 0.001` re-anchors and ends. The horizontal budget starts at `dist` and loses each hit's horizontal travel; when it exceeds `|M_h|` by more than
2^-20 the walk re-anchors, and the budget becomes `|M_h|`. With flag 0x4000000 and `M.z > 2^-20`,
a rise past `+0x80 + H` scales `len` by `(+0x80 + H − z)/M.z`, re-anchoring when that height
differs from `M.z` by 2^-20 or more; a cut `len` below 0.001 makes the mover fall.

| case | R | `rem` |
|---|---|---|
| `0x6334a0` accepts | `0x635c00(dir, &len, n, 0x636610(planes))` | `·len_new/len_old` |
| rejects, out flag set | end; the mover falls | |
| rejects; `0x636100` → 0 | return `ms` | |
| rejects; → 2 | sets 0x4000000, then as accepted | `·len_new/len_old` |
| rejects; → 1 | `0x635d80(dir, len, n)` | kept |

A rejection that reaches the probe re-anchors.

Addresses: `0x636a58`, `0x636a99`, `0x636ada`, `0x636b10`, `0x636b6e`, `0x636bbf`, `0x636b99`, `0x636c14`, `0x636c88`, `0x636cfc`, `0x636d29`, `0x636d5c`, `0x636f91`, `0x636f99`
Confidence: verified
Proof: one
Test: none

`0x636a58` pushes `&[ebp-0x30]`, the out flag. `0x636b10` stores 1 into `[ebp-0x20]` before
`0x636b6e`. `M` is three `fadd` on `R` stored as floats; `len = √((Mz² + float(My²)) +
float(Mx²))` (`0x636c88`), `jnp 0x636f91` below 0.001. `0x636cfc` is `d8 e9`, `fsubr st,st(1)`:
`st0 = st(1) − st0`, the old budget minus `float(|M_h|)`, then `fcomp 2^-20; test ah,0x41; jne`.
The cap runs only for `H + float(+0x80) < float(M.z + z)` (`test ah,5; jp`);
`0x636d5c` is `d8 f9`, `fdivr st,st(1)`: `st0 = (H + +0x80 − z)/M.z`. `0x636f99` stores 1 into
`[ebp-0x30]`. `0x6334a0` takes the hit's index and a pointer it writes, not n; what it accepts is
in the airborne topic.

## Settle and the fall election

The settle sweeps down `s = float(t·1.8493990 + 1/36)`, t the last sweep's travel (`0x636dda`),
plus 1.0 for a hovering mover (flag tested at `0x636dd2`) and plus H with flag 0x4000000
(`0x636dfc`), all from one read of `+0x40` (`0x636dcd`). The sweep (`0x636e45`) travels d, and z
loses it (`0x636e52`).

| below | effect |
|---|---|
| nothing within s | no-floor flag: the mover falls from the lowered z |
| floor with n.z > cos 50° | clears 0x4000000 (`0x636edf`) |
| floor at or below cos 50° | `0x636100(dir_h, n)`: 0 returns `ms`; 1 no-floor flag; 2 sets 0x4000000 (`0x636f21`) |

A hovering mover then takes d back (`0x636e81`) and loses `d − 1` when that is not negative
(`0x636ea9`): it ends at `z − max(d − 1, 0)` and is never raised here (`0x636e74`).

Addresses: `0x636dcd`, `0x636dd2`, `0x636dda`, `0x636dfc`, `0x636e45`, `0x636e52`, `0x636e67`, `0x636ed2`, `0x636edf`, `0x636f03`, `0x636f21`, `0x636e74`, `0x636e81`, `0x636ea9`
Confidence: verified
Proof: one
Test: none

`0x636dda` is `fmul [0x80c740]`, then `fadd 1/36 (0x3ce38e39)`, `fstp`; the hover and H addends
each round to float. A failed sweep returns `ms`. `0x636e52` is `fsub [ebp+0x10]; fstp [esi+0x18]`.
`0x636e67` takes `hit == count`: owner GUID 0, `[ebp-0x30] = 1`. `0x636ed2`: `fcomp cos 50°;
test ah,0x41; jnp 0x636ee8` sends equal and below to the probe; NaN counts as walkable.
`0x636f03` calls `0x636100`; 1 reaches `0x636e6d`. The hover arm: `fadd` d back, `fld d; fsub 1.0;
fcom 0.0; test ah,5; jnp` skips a negative `d − 1`, else `z = float(z − (d − 1))`.

## Walk exit and hover climb

At the end, a last sweep without a hit or a set no-floor flag starts a fall, `0x7c61f0(0)`, and
returns `ms`. A set re-anchor flag calls `0x7c5cd0(0)`, passes the owner GUID of the last face
hit to `0x6173b0` as the transport, and returns `ms`. Otherwise a hovering mover sweeps 1.0 down
(`0x636fa1`): a face at or below the slope limit (cos 50° when `0x5fa550(+0x15c)` is nonzero,
cos 80° otherwise) makes it fall. With clearance `c1 < 1` it rises by
`clamp(float(v·(ms·0.001) + c0), 0, 1 − c1)`, v from `0x7c61b0` (7.0 by default) and c0 the
clearance before the move; the up sweep's travel is added to z (`0x6370f1`). Then `0x6173b0` runs as
above.

Addresses: `0x636f36`, `0x636f49`, `0x636f52`, `0x636f57`, `0x636fa1`, `0x63700d`, `0x63703f`, `0x637068`, `0x637081`, `0x63709e`, `0x637119`, `0x6370f1`, `0x6370fe`, `0x637124`, `0x7c61f0`, `0x7c5cd0`, `0x6173b0`, `0x7c61b0`
Confidence: verified
Proof: one
Test: none

`0x636f36`: `cmp ecx,[0xc4e530]; jae 0x637120`; `0x636f49` tests `[ebp-0x30]`, `0x636f52` the
re-anchor flag. The GUID is the dword pair at `[0xc4e544] + 8·hit`, read at each hit, at the
settle and here; no hit gives 0. `0x63703f` is `test ah,0x41; jnp 0x637120`. `0x637068` climbs
only for `c1 < 1` (`jp` skips greater, equal, NaN). `0x637081` adds c0 (`[ebp-0x78]`). A
negative rise becomes 0 (`0x63709e`); `0x637119` keeps a rise below `1 − c1` and stores
`float(1 − c1)` otherwise, NaN included. `0x7c61f0` sets 0x2000, zeroes the fall time and starts
the fall at the current z (falling topic); `0x7c5cd0` is `CMovement::UpdateAnchors`.

## Rise along a face

`0x635c00(out, dir, &len, n, flip, B)` gives the height change h of moving `len` along `dir` on
the plane n, `h = float(−(n·dir)·len / n.z)`, and limits it to H. When `flip` is nonzero,
`n.z ≤ cos 50°` and `|B|² ≥ 2^-22`, n is replaced by −B, the contact vector, so the rise
follows B's slope, `h = float(−(B·dir)·len / B.z)`, instead of the face's. For `|n.z| < 2^-22`, h is −FLT_MAX when the numerator is negative, else +FLT_MAX. The
output is `(0·X, 0·X, X)` (`0x635d5b`): x and y are −0 for a negative X and NaN for a NaN
one.

| case, in test order | `len` | X |
|---|---|---|
| flag 0x4000000 and h < 0 | 0 | H |
| H < h | `float(H/h·len)` | H |
| h < −H | `float(−(H/h·len))` | −H |
| otherwise, NaN included | kept | h |

Addresses: `0x635c00`, `0x635c0e`, `0x635c35`, `0x635c45`, `0x635c57`, `0x635c7d`, `0x635c80`, `0x635cac`, `0x635cc1`, `0x635cdc`, `0x635d06`, `0x635d33`, `0x635d5b`, `0x617430`
Confidence: verified
Proof: one
Test: `collision::speed_select_difftest`

`|B|²` is stored as a float over the B pointer (`0x635c2a`). `0x635c35`: `fcomp cos 50°; test
ah,0x41; jp` skips the flip above it and for NaN; `0x635c45`: `test ah,5; jnp` skips it below
2^-22. The replacement loads `ecx = [ebp+0x24]` (`0x635c0e`), the B pointer, and stores
`−B.x`, `−B.y`, `−B.z` over the by-value n (`0x635c57` to `0x635c7d`). The numerator is
`((−n.z·d.z) + (−n.y·d.y)) + (−n.x·d.x)` times `len` (`0x635c80`). `0x635cc1` picks ±FLT_MAX (`0xff7fffff`,
`0x7f7fffff`): zero gives +. `0x635d06`: `H; fcomp h; test ah,5; jp` takes the H branch only
for `H < h`; `0x635d33`: `−H; fcomp h; test ah,0x41; jne` keeps h unless `−H > h`. `0x617430`
returns `+0xb8` when `0x5fa550(+0x15c)` is nonzero, else 2.0 (`0x40000000`). `ret 0x20`.

## Horizontal push-out from a wall

`0x635d80(out, dir, len, n)` writes the horizontal correction that cancels the part of the move
`dir·len` that enters the plane n, plus 0.001 yd. A plane with `n.z < 0` and `−n.z > cos 50°`
gives (0, 0, 0). Otherwise `u = (n.x, n.y)/√(n.x² + n.y²)`, or the raw pair when the root is
under 2^-22; `c = (u.y·n.y + u.x·n.x) + n.z·0`; `p = −(n·dir)·len`; `k = p/c + 0.001`, or
`p + 0.001` when `|c| < 2^-22`; out = `(float(u.x·k), float(u.y·k), 0)`. It returns `out` and
does not read `this`.

Addresses: `0x635d80`, `0x635d98`, `0x635dbd`, `0x635de1`, `0x635e4c`, `0x635e5d`, `0x635e63`, `0x4549f0`, `0x454980`
Confidence: verified
Proof: one
Test: `collision::walk_helpers_difftest`

`0x635d98` (`test ah,5; jp`) passes `n.z ≥ 0` and NaN; `0x635dbd` (`test ah,0x41; jnp`) zeroes
only for `−n.z > cos 50°`. `0x635de1` calls `0x454980`, `x² + y²` of the pair. `u.x` is stored
as a float, `u.y` stays in the register. `0x635e5d` is `de f9`, `fdivp st(1),st`:
`st(1) = st(1)/st0 = p/c`. `0x635e63` adds 0.001 (`0x3a83126f`). `0x4549f0` is the 3D
counterpart, `x² + y² + z²`, which the step probe uses. `ret 0x10`.

## Step-up probe

`0x636100(dir_h, n)` tests a step against the face n and returns 0 when a sweep fails, 1 when
blocked, 2 when the step is allowed. It saves the position and restores it on 1 and 2; on 0 it
stays wherever the probe left it. The reach is `s = max(float(r + 1/720), float(H·tan 50°))`, r = `+0xb0`
(`0x636147` to `0x636190`, held in `ebx` at `0x636193`).

| step | sweep and effect |
|---|---|
| 1, only if 0 ≤ n.z ≤ cos 50° | s along `−n_h/|n_h|`; D is that when it hits a face with exactly n, else `dir_h` |
| 2 | H up: travel u; `u < 2^-22` returns 1 |
| 3 | z += u; s along D: travel f; position += D·f |
| 4, if `|f − s| ≥ 2^-20` and `0x6334a0` accepts | `0x635c00` deflects `s − f`; one sweep along the result; f gains its horizontal travel, u its z |
| 5 | a face still hit and `f ≤ r` returns 1 |
| 6 | `u + 0.01` down: travel d; z −= d |
| 7 | no floor, `n.z > cos 50°`, or a nonzero `0x635f80(D, start, float(u − d))`: returns 2, storing the start z at `+0x80` unless 0x4000000 is set; else 1 |

Addresses: `0x636100`, `0x636147`, `0x636190`, `0x636193`, `0x6361a1`, `0x63622d`, `0x63623f`, `0x6362cb`, `0x6362ed`, `0x63633a`, `0x63637d`, `0x6363c5`, `0x636411`, `0x6364a9`, `0x636516`, `0x636561`, `0x6365a1`, `0x6365b7`, `0x6365c0`, `0x6365df`, `0x6365fc`
Confidence: verified
Proof: one
Test: `collision::multipass_difftest`, `collision::multipass_sweep_miss_difftest`

`0x636147` loads `+0xb0`, adds 1/720 and stores; `0x636168` compares `H·tan 50°` with it (`test
ah,0x41; jne` keeps the radius term unless the product is greater, NaN included). `0x6361a1` (`jp`)
skips step 1 above cos 50° and for NaN, `test ah,1` below 0. `0x63623f` to `0x63627b` compare all
three normal components (`test ah,0x44`). `0x6362ed` returns 1 for u below 2^-22. `0x63637d` skips
step 4 when f is within 2^-20 of s. `0x636516` (`test ah,0x41; jne 0x6365c0`) returns 1 for
`f ≤ r` or NaN. `0x636524` adds 0.01 (`0x3c23d70a`). `0x6365a1` (`jp`) returns 2 above cos 50°.
`ret 0x10`.

## Fall-away lookahead

`0x635f80(dir, start, h)` returns 1 when a free fall from the current state would carry the mover
at least r = `+0xb0` horizontally from `start`, in a direction whose dot with `dir` exceeds cos 10°.
It simulates `T = round(1000·√(((h + 0.1) + (h + 0.1))·(1/19.291105)))` ms, half away from zero:
it saves 56 bytes of state (`0x635ea0`), starts a fall (`0x7c61f0(0)`), takes `D = 0x7c6140(T)`,
calls `0x635600(0, T − done, (+0x68, +0x6c, −D), 0)` while flag 0x2000 holds and `done < T`, and
restores the state (`0x635f10`). The position, `+0x7c` and `+0xa0` stay as the fall left them,
and skipped time can still be reported.

Addresses: `0x635f80`, `0x6359e7`, `0x635a44`, `0x615ae0`, `0x635faf`, `0x635fbf`, `0x635fee`, `0x636049`, `0x63605a`, `0x6360a3`, `0x6360d4`, `0x635ea0`, `0x635f10`
Confidence: verified
Proof: one
Test: `collision::resolve_fall_land_difftest`, `collision::state_copies_difftest`

`0x635faf`: `fcom 0.0; test ah,0x41; jne` subtracts 0.5 at or below 0 and for NaN, else adds it;
`__ftol` truncates at `0x635fbf`. The constants are 0.1 (`0x3dcccccd`), 1/g (`0x3d54536a`) and
1000.0 (`0x447a0000`). `0x635ea0` copies `+0x44` to `+0x58`, `+0x78`, `+0x5c` to `+0x6c`, `+0x40`
and `+0x84` into a 56-byte record; `0x635f10` copies them back. The distance is `√(dy·float(dy) +
dx_f²)` with `dx_f = float(x − start.x)`; `0x6360a3` (`test ah,5; jp`) returns 0 below r. The
unit offset `(dx_f, float(dy))/dist` dotted with `dir` (`dir.y·u.y + u.x·dir.x`) is compared at
`0x6360d4` with cos 10° (`test ah,0x41; jne` returns 0 at or below, NaN included). `ret 0xc`.
The report argument 0 gates only `0x635600`'s report at `0x635a44`; its lost-time path calls
`0x615ae0` at `0x6359e7` before that gate.

## Bevel contact direction

`0x636610(planes, count, out)` (`ecx`, `edx`, one stack argument) returns 0 unless
`1 ≤ count ≤ 4` and every plane's z lies strictly within 2^-20 of −0.4756366, the mover's bevel
planes. Then it returns 1 with out:

| count | out |
|---|---|
| 1 | plane 0's normal |
| 2 | `(float((p1 + p0).x·K), float((p1 + p0).y·K), float(float(p1.z + p0.z)·K))`, K = 0.6385560 |
| 3 | the normal of the one plane with `|y| < 2^-20` when exactly one has it, else of the first plane with `|y| ≥ 2^-20` and `|x| < 2^-20` |
| 4 | (0, 0, 0) |

Addresses: `0x636610`, `0x636626`, `0x63664a`, `0x636664`, `0x636714`, `0x63672c`, `0x63673d`, `0x636765`
Confidence: verified
Proof: one
Test: `collision::walk_helpers_difftest`

`0x636626` rejects counts above 4; `0x63664a` (`test ah,5; jp`) rejects a distance of 2^-20 or
more and NaN. The jump table `0x63679c` holds `0x63666b`, `0x63668c`, `0x6366fb`, `0x6366e2`
for counts 1 to 4. K (`0x3f237868`) is `1/√(2a² + 4b²)` for a bevel normal `(a, 0, −b)`, b =
0.4756366 and `a = √(1 − b²)`: the length of the sum of two perpendicular bevels. For count 3,
`0x636714` stores indices at `[ebp + 4·n − 8]`, two slots below the saved frame pointer, so a third
plane with `|y| < 2^-20` overwrites `[ebp]`; `0x636765` reads `[ebp-0x14]`, unwritten when no
plane reached `0x63672c`.

## Averaged ground normal

`0x637140(out, planes, count)` returns the mean normal of the candidate faces with n.z above
cos 89° whose triangle survives clipping by the planes (`0x631870`), normalized; (0, 0, 1) when
none does. The sums are float adds in face order; the mean is `sum·(1/k)` and the result
`mean·(1/|mean|)`. `0x6332a0` (resolve) passes the six planes of the mover's box and stores the
result at `+0x24`. It does not read `this`.

Addresses: `0x637140`, `0x6371a4`, `0x6371c3`, `0x6371e4`, `0x637230`, `0x637257`, `0x637271`, `0x637287`, `0x631870`, `0x6332a0`
Confidence: verified
Proof: one
Test: `collision::walkable_normal_avg_difftest`

`0x6371a4`: `fcomp cos 89° (0x3c8ef859); test ah,0x41; jnp` skips at or below; NaN is kept. Each
face entry is 0x34 bytes: the normal at `+0x00` and three vertices at `+0x10`, which
`0x6371c3` copies into a polygon with count 3. `0x637257` forms `1/k` from `fild`. x stays in the
register, `y_f` is stored, and `z` is stored by `fst` at `0x637271` while the register keeps it:
`|mean|² = z·z_f + y_f² + x·x`. At `0x637287`, `1/|mean|` is stored by `fst`; out.x uses the
register, out.y and out.z the stored `y_f`, `z_f` and the stored reciprocal. `ret 0xc`.

## Slope thresholds

The constants of these functions, all `.rdata`:

| address | value | dword | angle | role |
|---|---|---|---|---|
| `0x80dffc` | 0.6427876 | `0x3f248dbb` | cos 50° | walkable limit; flip and push-out tests |
| `0x80e000` | 0.1736482 | `0x3e31d0d4` | cos 80° | limit when `0x5fa550` is 0 |
| `0x80e008` | 1.1917536 | `0x3f988b62` | tan 50° | step reach `H·tan 50°` |
| `0x80c740` | 1.8493990 | `0x3fecb91b` | tan 61.6° | settle depth per yard |
| `0x80e014` | −0.4756366 | `0xbef386a4` | −sin 28.4° | bevel plane z |
| `0x80e024` | 0.6385560 | `0x3f237868` | | two-bevel normalizer |
| `0x80e01c` | 0.9848077 | `0x3f7c1c5c` | cos 10° | fall-away cone |
| `0x80e028` | 0.0174524 | `0x3c8ef859` | cos 89° | ground-normal filter |
| `0x80dfec` | 0.0013889 | `0x3ab60b61` | | 1/720, step reach floor |

Addresses: `0x80dffc`, `0x80e000`, `0x80e008`, `0x80c740`, `0x80e014`, `0x80e024`, `0x80e01c`, `0x80e028`, `0x80dfec`
Confidence: verified
Proof: one
Test: none

Read by their bytes. 1.8493990 is `a/b` for the bevel normal above, so a bevel face rises
61.6° from horizontal. The tolerances are 2^-20 (`0x35800000`) and 2^-22 (`0x34800000`); the
other literals are 1/36 (`0x3ce38e39`), 0.01, 0.1, 0.5, 0.001, 1.0, 1000.0 and 1/g. Every floor
test against cos 50° is strict: a face at exactly the limit is steep.

## Vector, box and plane helpers

`0x6372d0` adds a scalar to each component of a vector, `0x637300` subtracts it, each result
rounded to float; `0x637330` writes the negation. `0x637350` returns 1 when a point lies within a
box's min and max on all three axes, bounds included; NaN fails. `0x6373b0(out, a, b)` (`ecx`,
`edx`, one stack argument) writes, per component, a's bound when it lies strictly outside b's,
else b's: the union of two boxes. `0x637480(p0, p1, p2)` writes the plane `n = (p1 − p0) ×
(p2 − p0)`, normalized, and `d = −((n.z·p0.z + n.y·p0.y) + n.x·p0.x)`; nothing guards a
degenerate triangle. `0x6376a0` sets a plane to (0, 0, 1, 0).

Addresses: `0x6372d0`, `0x637300`, `0x637330`, `0x637350`, `0x6373b0`, `0x637480`, `0x6374c1`, `0x63751c`, `0x6376a0`
Confidence: verified
Proof: one
Test: `collision::resolve_toolkit_difftest`, `terrain::dda_cell_intersect_first_difftest`, `terrain::chunk_gather_aabb_difftest`, `collision::plane_from_tri_difftest`

In `0x637350`, min tests are `test ah,1; jne` and max tests `test ah,0x41; jp`. In `0x6373b0`
the ties and NaN take b in all six components. In `0x637480`, `0x6374c1` is `de e9`, `fsubp
st(1),st`: `st(1) − st0`, so x is `e1.y·e2.z − e1.z·e2.y`; `e1.x` and `e1.y` are rounded to
float, the rest kept in registers. The components are rounded at their stores, scaled by
`1/√(n.x² + n.y² + n.z²)` and stored with `fst` (`0x63751c`), and d uses the unrounded products.
`0x6372d0`, `0x637300` and `0x6376a0` return `this`; `0x637330` returns out; `0x637480` returns nothing.

## No swim or liquid test

None of these functions reads the swim flag or a liquid height: a swimming mover never reaches
them, since `0x634040` sends the flags 0x200800 to `0x634640` (`0x6341b6`). The flag tests on
`+0x40` in these functions are 0x2000, 0x4000000 and 0x40000000 only.

Addresses: `0x6341b6`, `0x634640`
Confidence: verified
Proof: one
Test: none

Census over the listing of the 17 functions, `0x635b00` to the end of `0x6376a0`: no operand
0x200000, 0x200800 or 0x10000000, and no `.data` reference other than `0xc4e52c`, `0xc4e530`,
`0xc4e534` and `0xc4e544`. Control: the same pattern for 0x40000000 finds its four tests (`0x63683c`, `0x636dd2`, `0x636e74`,
`0x636fa1`). The decision against the liquid height belongs to the resolve topic.
