# collision: resolve

How a requested displacement becomes a committed position against the world.

The resolution layer is the unnamed object file from `0x6312b0` through `0x637a60`. This topic
covers its functions from `0x633200` through `0x635600`; 20 of the 30, the airborne resolver
`0x635600`, its helpers and the triangle tests it shares with the walking code, are in
`collision/airborne`. The mover hands
each frame's displacement to `0x634040`, which cuts it into substeps. Each substep gathers the
world triangles inside a swept box (`0x633840`) into the list `0xc4e52c`, then runs one
resolver: the free-move resolver `0x634640` for a swimmer, a mover with flag 0x800 or a flying spline, the falling
resolver `0x635b00` while 0x2000 is set, else the walking resolver `0x6367b0`. The resolvers
write the position `+0x10` of the `CMovement` themselves; the entry only loops, reports and
refreshes the ground normal. The one function entered from outside collision is `0x6343b0`,
which the transport game object runs on each mover it carries.

Roots: `0x6343b0`

## Substep loop

`0x634040(time, ms, d)` returns the milliseconds it resolved (0 for ms = 0, and 0 when the flags hold no bit of 0x200f at entry). With L = |d| at 53
bits, the speed is v = float(L / (ms·0.001)) and the direction u = float(d_i·(1/L)) per
component, or (0, 0, 0) when L < 2^-20. The elapsed time e starts at 0, and while e < ms and the
flags `+0x40` hold a bit of 0x200f, one substep runs these steps in order:

| step | action |
|---|---|
| 1 | r = ms − e, dist = float(r·0.001·v) |
| 2 | `0x633840(dist, r, u)`; on 0: `0x615ae0(r)`, `+0x58` −= r, e = ms, and the loop ends |
| 3 | save the GUID `+0x38`, the speed `+0x84`, the flags, and F = (0x2000 set and `+0xa0` ≠ 0) |
| 4 | one resolver (Resolver dispatch); e += its return |
| 5 | `0x61a820(time + e, ms − e, saved flags, F, GUID changed)` |
| 6 | with no active spline, a changed `+0x84`: `0x7c5c20(0)`, and the loop ends |
| 7 | `+0x58` = 0 and flags & 0xf: `+0x58` = ms − e |

Addresses: `0x634040`, `0x616cb0`, `0x6340a6`, `0x6340ba`, `0x634100`, `0x634122`, `0x634154`, `0x634177`, `0x634260`, `0x63427b`, `0x634285`, `0x634299`, `0x6342a3`
Confidence: verified
Proof: one
Test: none

The caller `0x616cb0` passes the anchor plus delta minus the position. ms·0.001 is `fild qword`
times 0.001 (0x3a83126f) at 53 bits; `0x6340a6` is `d8 f9`, `fdivr st,st(1)`, so st0 = L /
(ms·0.001), stored as a float. `fabs; fcomp 2^-20 (0x35800000); test ah,5; jnp` skips the
normalization only below 2^-20: equal and NaN normalize. `0x6340bc` `fdivr` m32 1.0 gives 1/L.
The gate `0x634100`, `test dword [esi+0x40],0x200f; je`, runs before the first substep too.
`0x634122` is `fild qword r`, `fmul 0.001`, `fmul v`, `fstp dword`. At `0x634154`, `test eax,eax;
je 0x6342a3`. F is `fcomp 0.0; test ah,0x44; jp` (`0x634177`): unequal or NaN keeps 1. The
speed test `0x63427b` is the same idiom, so NaN counts as changed; an active spline (`+0xa4`
non-null, its `+0x18` bit 0x4 clear) skips it (`0x634265`). The loop test `0x634299` is `cmp
edi,ebx; jb`, unsigned.

## Resolver dispatch

Each substep runs exactly one resolver, chosen in this order, where S is a flying spline:
`+0xa4` non-null, its `+0x18` with bit 0x4 clear and bit 0x200 set.

| condition | resolver | arguments after `this` |
|---|---|---|
| S, or flags & 0x200800 | `0x634640` | time + e, r, dist, u by value |
| flags & 0x2000 | `0x635b00` | time + e, r, dist, pointer to (u.x, u.y) |
| otherwise | `0x6367b0` | time + e, r, dist, pointer to (u.x, u.y) |

The walking and falling resolvers receive only u's horizontal pair. Each returns the
milliseconds it consumed. A swimmer is a mover with 0x200000, the bit a fall's start clears
(`0x7c61f0`) as it sets 0x2000; `0x633840` adds a liquid query for it.

Addresses: `0x6341a0`, `0x6341b6`, `0x6341dc`, `0x6341e3`, `0x6341ff`, `0x63421e`, `0x7c61f0`
Confidence: verified
Proof: one
Test: none

`0x6341a0` to `0x6341b4` test the spline and jump to the free-move call on S; `0x6341b6` is
`test ecx,0x200800`; `0x6341e3` tests the 0x2000 bit saved at `0x63415f`. Before `0x6341dc`,
`sub esp,0xc` copies u, then dist, r and `ebx` = time + e (`0x63412a`) are pushed. Before
`0x6341ff` and `0x63421e` only u.x and u.y are copied, to `[ebp-0x2c]` and `[ebp-0x34]`, and
their addresses pushed. `0x7c61f0` ands the flags with 0xfbdfffff and ors 0x2000. The same
predicate in `0x616cb0` (`0x616cec` to `0x616d03`) decides whether the caller keeps d.z.

## Commit after the loop

After the loop, unless S holds, `0x6332a0` refreshes the ground normal `+0x24`. Then
`0x7c4930` writes the world position into a local that nothing reads: the resolvers have
already written `+0x10`. `0x634030`, to which other callers hand that same world position, is
an empty function.

Addresses: `0x6342c4`, `0x6342da`, `0x6342eb`, `0x7c4930`, `0x634030`, `0x616be1`, `0x616dce`, `0x617512`, `0x617f7a`
Confidence: verified
Proof: one
Test: none

`0x6342c4` to `0x6342d8` repeat the S test before `0x6342da`. `0x6342eb` passes `&[ebp-0x44]`
and `&+0x10`; the function then returns `[ebp+0x10]`, e, and `[ebp-0x44]` is not read again.
`0x7c4930` copies the position when `+0x38` and `+0x3c` are zero and otherwise maps it through
the transport's matrix. `0x634030` is `ret 4`. Its four call sites, `0x616be1`, `0x616dce`,
`0x617512` and `0x617f7a`, each push the address `0x7c4930` has just filled.

## Ground normal refresh

`0x6332a0` describes the unit's box at its committed position as six planes, with outward
normals and d = −(n·point): x ∈ [x − r, x + r], y ∈ [y − r, y + r], z ∈ [z, z + h], r = `+0xb0`,
h = `+0xb4`. It passes them, count 6, to `0x637140` and stores the returned vector in `+0x24`.
`0x637140` (walking) averages the normals of the triangles in `0xc4e52c` whose z exceeds
0.017452406 (0x3c8ef859) and which `0x631870` accepts against the planes, and normalizes the
sum; with none it returns (0, 0, 1). The list is the one the last query left.

Addresses: `0x6332a0`, `0x6333ad`, `0x6333b4`, `0x637140`, `0x631870`, `0x6174b0`
Confidence: verified
Proof: one
Test: none

The planes, as normal and d: (1, 0, 0), −x − r (from `0x6332db`, `fchs` then `fsub`); (−1, 0, 0), x − r;
(0, 1, 0), −y − r; (0, −1, 0), y − r; (0, 0, 1), −z − h; (0, 0, −1), z. Each d is rounded to a
float. `0x6333b4` adds 0x24 to `esi` and copies the three dwords `0x6333ad` returned. In
`0x637140`, `fcomp; test ah,0x41; jnp` skips z ≤ 0.017452406 and keeps NaN. The sum is float per
triangle, scaled by 1/count, then by 1/√(z² + y² + x²). The constructor `0x7c4850` sets `+0x24`
to (0, 0, 1). `0x6174b0` writes r as width·scale·0.5 and h as height·scale.

## Swept query frame

`0x633840(dist, ms, u)` starts from p = `+0x10`. On a transport (GUID `+0x38` nonzero) it maps p
to world space through the transport's matrix from `0x630ac0` with `0x7bcc60`, and u through
the matrix's rotation rows, each component rounded. `0x6337d0` then tests p's components with
`__finite`, and the result is discarded. The base box, kept in `0xc4e5a0` as min then max, is
[p.x − r, p.y − r, p.z] to [p.x + r, p.y + r, p.z + h], r = `+0xb0`, h = `+0xb4`. The move is
m = float(u_i·dist) per component.

Addresses: `0x633840`, `0x6338e2`, `0x633902`, `0x633907`, `0x63397d`, `0x6339b0`, `0x633a24`, `0x6337d0`, `0x630ac0`, `0x7bcc60`, `0x40a1a4`, `0xc4e5a0`
Confidence: verified
Proof: one
Test: none

`0x6338e2` skips the transport branch when `+0x38 | +0x3c` is zero. `0x633902` transforms
`[ebp-0x24]`, the copy of p, in place. `0x633907` starts the rotation: z' = (m22·u.z + m12·u.y)
+ m02·u.x, each component stored as a float, with the matrix rows at 16-byte strides from
`[ebp-0x88]`. `0x6337d0` loads each component, widens it to a double for `0x40a1a4` and stops
at the first non-finite one; after `0x63397d` nothing tests `eax`. `0x6339b0` subtracts r from
p.x; the stores to `0xc4e5a0` to `0xc4e5b4` follow. `0x633a24` builds m from three `fmul
[ebp+0x8]` products.

## Swept box growth

The box grows by mode, the first matching row, and is then united as stated:

| mode | growth |
|---|---|
| flags & 0x2000 | m.z −= f, f the drop `0x7c6140(+0x78 + ms)` still due; box ∪= box + m; 0x40000000 lowers min z by 1.0; t = −1.1917536·m.z, and t > 0 widens x and y by t on each side |
| flags & 0x200000 | s = dist/2, c = p + u·s; B = [c − s, c + s], x and y widened by q = r·√2, max z raised by max(q, h); box ∪= B |
| otherwise | g = max(k·1.1917536, r + 1/720); box ∪= box + u·(g + dist); w = r·√2 + dist/2, c = p + u·dist/2: box ∪= [c.x − w, c.y − w, c.z] to [c.x + w, c.y + w, c.z]; max z += max(2·k, dist); min z −= k + 1.1917536·dist; 0x40000000 lowers min z by 1.0 |

1.1917536 is tan 50° (0x3f988b62), √2 is 0x3fb504f3, and k is `0x617430`'s return: `+0xb8`,
which `0x6174b0` writes as a scale ratio, or 2.0 when `0x5fa550` rejects the unit. Every comparison picks its second operand on NaN, and t > 0 is false. Last, every min
drops and every max rises by 1/720 (0x3ab60b61).

Addresses: `0x633a3a`, `0x633a8b`, `0x633aa9`, `0x633ac4`, `0x633b5e`, `0x633c28`, `0x633c7b`, `0x633c97`, `0x633dab`, `0x633e0d`, `0x633e35`, `0x633e45`, `0x633e54`, `0x633b14`, `0x6373b0`, `0x617430`, `0x7c6140`
Confidence: verified
Proof: one
Test: none

Unions are `0x6373b0`, a per-axis min and max (`0x633a8b`). `0x633aa9` tests 0x40000000.
`0x633ac4` multiplies by −1.1917536 (0xbf988b62); `fcom 0.0; test ah,0x41; jne` widens only for
t > 0. Swimming: s = float(dist·0.5), and `0x633c28`, `fcomp q; test ah,5; jp`, takes h unless h <
q, so NaN gives h. Walking: g is k·1.1917536 (53 bits) only when `0x633c97`, `test ah,0x41; jne`, finds it
above r + 1/720, else that float; `0x633dab` adds dist/2 to r·√2 at 53 bits; `0x633e0d` doubles
k, and dist wins on equality or NaN. `0x633e35` fetches k again; `0x633e45` is `d8 2d`,
`fsubr`: min z − (k + 1.1917536·dist). `0x633e54` tests 0x40000000. `0x633b14`
applies the 1/720 skin through `0x637300` and `0x6372d0`.

## World query and results

The query `0x6721b0(box, list, mask, 0)` clears a list and fills it; a 0 return makes
`0x633840` return 0. The mask is `0x6315f0`'s. Then:

| list | contents and treatment |
|---|---|
| `0xc4e52c` | `+0x04` count, `+0x08` pointer to 52-byte triangles (normal, d, three vertices), `+0x18` pointer to a GUID per triangle |
| `0xc4e564` | a swimmer only: queried with mask 0x30000, the bits `0x6315f0` adds for water walking; return ignored; each normal and d negated |

On a transport each triangle in `0xc4e52c`, never `0xc4e564`, is mapped back to transport
space: the vertices through the rigid inverse from `0x7bd820(M, 1.0)`, the normal by its rotation
rows, and d = −(n·v0). The function then returns 1.

Addresses: `0x633b3b`, `0x633b46`, `0x633b4b`, `0x633e76`, `0x633e90`, `0x633eb3`, `0x633f12`, `0x633f35`, `0x63400a`, `0x6721b0`, `0x6315f0`, `0x637330`, `0x7bd820`, `0xc4e52c`, `0xc4e564`
Confidence: verified
Proof: one
Test: none

`0x633b3b` passes the mask; `0x633b46` calls the query with `edx` = `0xc4e52c`, and
`0x633b4b`, `test al,al; jne`, returns 0 otherwise. `0x633e76` tests 0x200000; `0x633e90`
queries with 0x30000 into `0xc4e564`. Per triangle `0x637330` writes the negated normal and
`0x633eb3` the negated `+0xc`, walking with stride 0x34. `0x633f12` inverts the matrix of
`0x630ac0`; from `0x633f35` three `0x7bcc60` calls map the vertices at `+0x10`, `+0x1c` and
`+0x28` in place, and the normal is x' = (m00·x + m20·z) + m10·y. `0x63400a` stores −((n'.z·v0.z
+ n'.y·v0.y) + n'.x·v0.x). `0x6721b0` zeroes the counts at `+0x04` and `+0x14` and steps 0x34 through `+0x08` before it
queries (`0x6721c7` to `0x6721e3`); the triangle layout is read here from these consumers, and
the world search is terrain's ([world triangle query](../terrain/collide.md#world-triangle-query)).

## Free-move resolver

`0x634640(time, ms, dist, u)` returns 0 when ms is 0, or on a transport when `0x633200`
returns nonzero; otherwise it returns ms. Below |dist| = 2^-20 it returns at once. Else it sweeps
`0xc4e52c` along u for dist with `0x632ba0` (contact); a hit index equal to the list's count is
no hit. When the world list has not been hit in this call, a swimmer, unless S holds or flag
0x800 is set, also sweeps `0xc4e564` for the remaining distance. Without a hit the position
moves by u·a, a the allowed distance, and the call ends; a hit moves it by u·a and slides. A
failed sweep returns ms at once; after the first, nothing has moved.

Addresses: `0x634640`, `0x634656`, `0x634683`, `0x6346f0`, `0x634703`, `0x634714`, `0x634734`, `0x63475a`, `0x63476f`, `0x634786`, `0x632ba0`, `0x6376a0`, `0x633200`
Confidence: verified
Proof: one
Test: none

`0x634656` calls `0x633200` only when `+0x38 | +0x3c` is nonzero. `0x634683` is `test ah,5;
jp`: greater, equal and NaN proceed. `0x6346f0` passes the list, &u, dist, &hit, seven planes
from `0x6376a0`, &count, &a and a null start. `0x634703`, `cmp edx,[0xc4e530]; jb`, marks a
world hit in `[ebp-0x24]`; `0x634714` skips the liquid sweep once it is set; `0x634734` is `test
ah,0x8`. `0x63475a` sweeps `0xc4e564` for `[ebp-0x4]`. `0x63476f` compares the index with that
list's count. The move adds u.x·a and u.y·a at 53 bits to x and y but rounds u.z·a to a float
first (`0x634786`). `time` is never read.

## Free-move slide

After a hit, with T = float(ms) and U the time used, the call takes these steps:

| step | action |
|---|---|
| 1 | t = a/rem·(T − U), then rem −= a |
| 2 | t + 2^-20 > 1: U += t, stall count c = 1; else c += 1, ending at c > 5 |
| 3 | T − U < 1 ends |
| 4 | p = −(n·u)·rem·n (`0x634300`), n the normal of the triangle hit, in its list |
| 5 | m = u·rem + p, rem = float(\|m\|); rem < 2^-20 ends |
| 6 | u = float(m·(1/rem)), 1/rem kept at 53 bits (`0x6348c5`, `0x6348f5`); sweep `0xc4e52c` only, and loop as for the first sweep |

`0x7c5cd0(0)` runs at every end except a failed sweep, and, before any slide, a move without a
hit or a time end with |T − U| < 2^-20.

Addresses: `0x634300`, `0x6347d8`, `0x6347ef`, `0x634807`, `0x634813`, `0x634825`, `0x63484c`, `0x634335`, `0x6348ae`, `0x6348c3`, `0x63490c`, `0x634919`, `0x634925`, `0x63493e`, `0x7c5cd0`
Confidence: verified
Proof: one
Test: none

`0x6347d8` computes a/rem·(T − U) at 53 bits. `0x6347ef`, `fcomp 1.0; test ah,0x41; jne`,
treats t + 2^-20 ≤ 1 and NaN as a stall; `0x634807` stores c = 1, `0x634813` increments and
leaves on `ja` past 5. `0x634825`, `fcomp 1.0; test ah,5; jnp`, sends T − U < 1 to `0x634925`; NaN slides.
In `0x634300`, k = ((−n.z·u.z + −n.y·u.y) + −n.x·u.x)·rem; `0x634335` is `fst`, so p.x and p.y
use the float k and p.z the register. m.x = float(u.x·rem + p.x), m.y = float(p.y + u.y·rem),
m.z = float(float(u.z·rem) + p.z). `0x6348ae` takes √((m.z² + m.y²) + m.x²); `0x6348c3` ends
below 2^-20. `0x634919` returns after a failed `0x63490c` without `0x7c5cd0`; `0x634925`,
`test ah,5; jp`, reaches `0x63493e` unless |T − U| < 2^-20, where a slide is required.

## Carrier push

`0x6343b0(dist, u, M)` is called only from `0x5f5f10`, in the transport game object's code,
once per mover in a list it walks, with a distance, a unit direction and a matrix M applied to
`+0x10`. It builds the box of p = M(`+0x10`), unites it with that box shifted by u·dist,
adds no skin, and queries `0xc4e52c` with 0x111, or 0x2111 when `0x5fa550` returns 0, plus 0x8000
under `0x6315f0`'s unit test: never 0x100000. After a failed query it returns. Else it sweeps
from p, without testing whether the sweep failed; on a hit, unless a spline is active, `+0x10` += u·a, `0x7c61c0` tries to start a fall
(then `0x619ca0` and `0x602c40`), and `0x617100(0, 0)` runs.

Addresses: `0x6343b0`, `0x5f5f10`, `0x5f615e`, `0x6343c8`, `0x6344d5`, `0x63450c`, `0x63451f`, `0x634575`, `0x6345b6`, `0x6345c3`, `0x6345c8`, `0x6345cb`, `0x632ca2`, `0x6345d3`, `0x634610`, `0x63462b`, `0x7bca80`, `0x5fa550`, `0x7c61c0`
Confidence: verified
Proof: one
Test: none

`0x5f615e` is the one call site, inside a loop over the transport's list that passes the list
entry in `ecx`; above it, `0x5f60f2` to `0x5f612a` take dist as a square root and store u, a
vector divided by dist. `0x6343c8` is `0x7bca80`, a transform into a new vector. The mask is set at `0x6344d5`
and or'd at `0x63450c`; `0x63451f` queries. On 0, `0x634575` writes p + u·dist into a local
that is never read, and the function returns. `0x6345c3` sweeps with p as the start and its
return is never tested. The hit index is written over the M argument's slot `[ebp+0x10]`
(`0x6345b6`), which the sweep's failure exit `0x632ca2` leaves unwritten, so after a failed
sweep `0x6345c8` reloads the M pointer and `0x6345cb` compares that stale value with the count.
`0x6345cb` returns when the index equals the count, `0x6345d3` when the spline is active (non-null, bit 0x4
clear). The move rounds u.z·a first. `0x634610` calls `0x7c61c0`; `0x63462b` is reached either
way.

## Transport detach test

`0x633200` returns 0 when the dword at `+0x0c`, the second dword of the link at `+0x08`, is
0. Otherwise it calls the mover's transport setter `0x6173b0(0, 0)` with a null GUID
and returns 1 when that returns nonzero. `0x634640` and `0x635b00` call it on a transport and
return 0 when it returns 1, so `0x634040` runs the same remaining time again after `0x61a820` has seen the
GUID change.

Addresses: `0x633200`, `0x633213`, `0x6173b0`, `0x634656`, `0x635b14`, `0x61a820`
Confidence: verified
Proof: one
Test: none

`0x633200` is `mov eax,[ecx+0xc]; test eax,eax; jne`, then pushes two zeros and calls
`0x6173b0`; `0x633213` is `neg eax; sbb eax,eax; neg eax`, which turns a nonzero return into 1.
`0x6173b0` with a null GUID, read only as far as its gates, calls `0x617170(0, 0)` for the
active player with no active spline when `0x630ba0`, given the position and the GUID, returns 0. A 0 return adds nothing
to e, so the loop condition still holds.

## Unreached stub

`0x633830` is `ret 0x14`, the same argument count as `0x633840` beside it, and nothing
reaches it.

Addresses: `0x633830`
Confidence: verified
Proof: one
Test: none

Census over the raw image: every `e8` and `e9` byte in `.text` whose relative target is
`0x633830`, and every 4-byte value at any offset of every section equal to it: zero of each.
The same scans find the four call sites of `0x634030`, and the stored-address scan finds
`0x514120` at `0x850154` and `0x7acb30` at `0x81d7f4`. The instruction before it, `0x633820`, is
`ret 4` followed by padding, so nothing falls into it.
