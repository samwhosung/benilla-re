# collision: airborne

How a displacement that includes a fall is resolved against the world, and the triangle tests
the airborne and walking resolvers share.

These functions sit in the resolution layer's object file, from `0x633220` to `0x635600`, beside
the substep loop of `collision/resolve`. The falling resolver `0x635b00` and the walking code
`0x635f80` (both `collision/walking`) hand `0x635600` a displacement whose z already holds the
fall. It moves in steps: each sweeps the triangle list the substep's query gathered
(`0x6351a0`), takes its time from the fall arc (`0x635450`), and against a steep contact adds a
horizontal push (`0x635090`). A step ends the call by landing, by striking the head, by
stalling, by a failed sweep, or by running out of distance or time. The triangle tests and the falling-far latch
`0x633240` are also called by the walking code. None of these functions is entered from
outside collision.

Roots: none

## Airborne resolver loop

`0x635600(time, ms, D, report)` resolves D over ms. At entry: T = float(ms·0.001) seconds, t0 =
float(`+0x78`·0.001), U = 0, L = float(|D|), and the fixed h = float(|D.xy|) and |D.z|; it saves
`+0x84`, `+0x68`, `+0x6c` and `+0x5c`. Each step:

| step | action |
|---|---|
| 1 | L < 2^-20 ends the call with T |
| 2 | u = float(D_i·(1/L)); a = L; t = float(t0 + U) |
| 3 | `0x6351a0(t, &D, &a, &push, &guid, &land, &head)` returns 0, 1 or 2; 2 ends the call |
| 4 | on 1, τ = `0x635450(T − U, t, &D, &a, head, h_s)`; on 0, τ = T − U |
| 5 | `+0x10` += D, per component; U = float(U + τ) |
| 6 | head ends through `0x633220`, land through `0x7c6290`; U + 0.0005 ≥ T ends with T |
| 7 | L −= a, the stall rules run, and the next step starts |

Addresses: `0x635600`, `0x63561b`, `0x635633`, `0x635662`, `0x6356b2`, `0x6356c2`, `0x6356e1`, `0x635721`, `0x635734`, `0x635742`, `0x635763`, `0x635768`, `0x635788`, `0x635790`, `0x635799`, `0x6357aa`, `0x6357b8`
Confidence: verified
Proof: one
Test: `collision::fall_integrator_difftest`, `collision::fall_integrator_freefly_damage_difftest`

`0x63561b` and `0x635633` take ms and `+0x78` through `fild qword`, times 0.001, to floats.
`0x635662` is `0x4549f0`, x² + y² + z², and its root is stored as L; `0x6356b2` roots D.y² +
D.x². `0x6356c2`, `fabs; fcomp 2^-20; test ah,5; jnp`, leaves for L < 2^-20 only. `0x6356e1`
divides 1.0 by L. `0x635721` loads t = float(t0 + U) into `edi`. `0x635742` is `cmp eax,2; je`,
with T − U already on the stack; `0x635763` calls `0x635450` on any other nonzero result. From
`0x635768` each component is a float add. `0x635788` keeps τ on the stack while it stores U.
`0x635790` and `0x635799` test head and land. `0x6357aa`, `fcomp T; test ah,1; je`, ends on
U + 0.0005 ≥ T (0x3a03126f); NaN goes on. `0x6357b8` stores L − a. h_s, pushed at `0x63574c`, is
`[ebp-0x14]`: float(|D.xy|) at entry, then each heading update's h' (`0x63590e`); a forced drop
leaves it, and only the stall rules' c ≥ 2 test reads the fixed h.

## Stall rules

A step whose τ is at most 0.0005, or NaN, is a stall: c += 1; any other step sets c = 1.

| state | next move |
|---|---|
| c ≤ 5 | D = (u.x·L + push.x, u.y·L + push.y, u.z·L) |
| c ≥ 2 as well | end through `0x7c6290` when the next step would end farther, horizontally, from the position at entry than both h + 2^-20 and 1.1866661·\|D.z\| at entry |
| c > 5, and a drop already forced or the last horizontal length below 2^-20 | end through `0x7c6290` |
| c > 5 otherwise | force a drop: `+0x84` = 0, D = (0, 0, u.z·L), L = \|D.z\|, c = 0; the next step starts at once |

Otherwise the heading update follows.

Addresses: `0x6357c9`, `0x6357da`, `0x6357dc`, `0x635801`, `0x635828`, `0x635866`, `0x635873`, `0x635887`, `0x635896`, `0x6358a0`, `0x6358bb`, `0x6358c1`, `0x6358ef`, `0x7c6290`
Confidence: verified
Proof: one
Test: none

`0x6357c9`, `fcomp 0.0005; test ah,0x41; jne`, sends τ ≤ 0.0005 and NaN to `0x635896`, `inc
ebx; cmp ebx,5; jbe`. `0x6357da` sets c = 1, and the `cmp ebx,edi` at `0x6357dc` feeds the
`jbe` at `0x635828`, which skips the check while c ≤ 1. From `0x635801`, x and y add push to
the unrounded u·L; u.z·L is rounded and stored alone. `0x635866` roots ((pos.y − y0) + D.y)² +
float((pos.x − x0) + D.x)², with x0 and y0 the position at entry. `0x635873` and `0x635887`,
`fcompp; test ah,0x41`, need the root strictly greater; NaN goes on. `0x6358a0` tests the forced
flag, `0x6358bb` the length `[ebp-0x14]`. `0x6358c1` writes 0 to `+0x84` and keeps u.z·L at 53
bits for |D.z|; `0x6358ef` jumps to the loop head. push is 0 at entry and only a push hit rewrites it, so
every later step adds the last push again. `0x7c6290` clears 0x6000 while 0x2000 is set,
then calls `0x7c5cd0(0)` and `0x7c5c20(0)`.

## Heading update after contact

The step's horizontal part sets the heading and speed: with h' = float(√(D.y² + D.x²)) at least
2^-20, n = float(D.xy·(1/h')) with 1/h' kept at 53 bits, `+0x84` becomes (n.y·`+0x6c` + n.x·`+0x68`)·`+0x84`, or 0 when that is
negative, and `+0x68`, `+0x6c` = n, `+0x5c` = (n.x, n.y, 0). Below 2^-20, `+0x84` = 0 and `+0x68`,
`+0x6c`, `+0x5c` take D.x, D.y unscaled. Then L = float(√((D.y² + D.x²) + D.z²)), reusing the
unrounded horizontal sum (`0x63590a`, `0x635979`).

Addresses: `0x6358f6`, `0x63590a`, `0x63590e`, `0x635920`, `0x635979`, `0x63591e`, `0x635931`, `0x635942`, `0x635956`, `0x63595b`, `0x635965`, `0x63597b`, `0x6359a6`
Confidence: verified
Proof: one
Test: none

`0x6358f6` squares D.y and D.x at 53 bits; `0x63590e` stores the root as a float, and
`0x63591e`, `test ah,5; jnp`, sends below 2^-20 to `0x63595b`. The scale 1/h' divides by the
float. `0x635931` is `fmul [ebp-0xc]; fst`, so n.y is stored rounded while the register keeps
it for the product with `+0x6c`; n.x is read back as the float. `0x635942` multiplies by
`+0x84` and stores it with `fst`; `0x635956`, `fcomp 0.0; test ah,5; jp`, writes 0 only for a
negative value: zero and NaN stay. `0x635965` adds D.z² to the unrounded D.y² + D.x²;
`0x63597b` and the stores after it write `+0x68`, `+0x6c` and `+0x5c` to `+0x64`, the last 0.0.
`0x6359a6` stores L.

## Airborne exit and report

The call returns n = trunc(x·1000 + 0.5), or − 0.5 when x·1000 ≤ 0 or NaN, with x = T when
distance or time ran out and x = U for a head strike, a landing or a stall end. On a sweep
failure, r, the same rounding of T − U, is taken off `+0x58` and `+0x78` and passed to
`0x615ae0`, and x = T. Unless the head struck, `+0x78` += n. Then:

| condition | action |
|---|---|
| report, head strike | `0x61a800(time + n)` |
| report, landing GUID nonzero | `0x6173b0(guid)` |
| report, 0x2000 clear | a nonzero `0x61ab20()` returns n at once |
| report, 0x2000 set | `0x633240`; if 0x2000 is still set, `+0xa0` ≠ 0 and flags & 0xf, the saved `+0x84`, `+0x68`, `+0x6c`, `+0x5c` return |
| a step ever continued the loop | `0x7c5cd0(0)` |

Addresses: `0x6359ae`, `0x6359b4`, `0x6359cf`, `0x6359e7`, `0x635a08`, `0x635a30`, `0x635a3a`, `0x635a44`, `0x635a59`, `0x635a6e`, `0x635a7d`, `0x635a91`, `0x635aa4`, `0x635ab1`, `0x635ae0`, `0x40a2b0`, `0x615ae0`, `0x61a800`, `0x61ab20`, `0x633240`
Confidence: verified
Proof: one
Test: none

`0x6359ae` and `0x635a08` multiply by 1000.0 (0x447a0000); `fcom 0.0; test ah,0x41; jne` adds
0.5 only for a positive product, and `0x6359cf` and `0x635a30` call `__ftol` (`0x40a2b0`), which
truncates. `0x6359e7` passes r after subtracting it from `+0x58` and `+0x78`. `0x635a3a` skips
the `+0x78` add while head is set. `0x635a44` tests report. `0x635a59` passes time + n;
`0x635a6e` runs when the GUID's two halves are not both 0. `0x635a7d` returns `edi` directly on
nonzero, bypassing the test at `0x635add`. `0x635aa4`, `fcomp 0.0; test ah,0x44; jnp`, skips the restore
only for `+0xa0` = 0; `0x635ab1` is `test cl,0xf`. `0x635add` loads the flag set at
`0x6357c3`. `0x635b00` passes report 1, `0x635f80` passes 0 for both time and report.

## Airborne step sweep

`0x6351a0` clears land and head. For a < 2^-22 it returns 0 with guid 0. Else u =
float(D_i·(1/a)) and `0x632ba0` (contact) sweeps `0xc4e52c` along u for a, writing the allowed
distance into a. Then:

| outcome | result |
|---|---|
| sweep failed | D = 0; return 2 |
| no hit | guid = 0; return 0 |
| hit, `0x633720` finds triangle i floor under `+0x10` + D | D = float(u_i·a); land = 1; guid = i's GUID if a contact plane has z = −0.4756366 (`0x635410`), else 0; return 1 |
| hit, rising: u.z > 0, Ta − t ≥ 0 and (Ta − t)·`+0x84` ≥ a, Ta = `0x7c5da0()`, and a contact plane of z = 1 (`0x6353d0`) | D = float(u_i·a); guid = 0; head = 1; return 1 |
| any other hit | D = float(u_i·a); guid = 0; push = `0x635090(…)`; return 1 |

Addresses: `0x6351a0`, `0x6351d8`, `0x635245`, `0x63524c`, `0x635267`, `0x63528a`, `0x6352b9`, `0x6352d4`, `0x6352dd`, `0x635323`, `0x635342`, `0x63534a`, `0x63535d`, `0x63536e`, `0x6353ad`, `0x6353d0`, `0x635410`, `0x7c5da0`, `0x632ba0`
Confidence: verified
Proof: one
Test: `collision::walk_slide_difftest`, `collision::walk_slide_sweep_miss_difftest`, `collision::leaf_predicates_difftest`

`0x6351d8`, `test ah,5; jnp`, returns 0 for |a| < 2^-22 (0x34800000). `0x635245` passes the
list, &u, a, &i, seven planes, &count, the address of a and a null start; `0x63524c` returns 2
after storing zeros. `0x635267` compares i with `[0xc4e530]`. `0x63528a` multiplies u by the new
a. The plane tests read z at `+0x8` of each 16-byte plane: `0x635410` against −0.4756366
(0xbef386a4) and `0x6353d0` against 1.0, each `fabs; fcomp 2^-20; jnp` on the difference.
`0x6352dd` reads the GUID from the array at list `+0x18`, eight bytes per triangle. `0x635323`
needs u.z > 0; `0x63534a`, `test ah,1; jne`, needs Ta − t ≥ 0; `0x63535d` needs (Ta − t)·`+0x84`
≥ a. NaN fails each. `0x6353ad` passes the planes and their count.

## Step time from the fall

`0x635450(R, t, &D, &a, head, h)` returns the step time in seconds, at 53 bits. k =
float(|D.xy|/h·R) for h > 2^-20, else 0. tf = `0x7c5f50`(float((`+0x7c` − `+0x18`) − D.z),
rising), the arc time at which the fall reaches the step's end height, with rising =
`0x635550(t, R, D, head)`. Then, with d = tf − t:

| case | result |
|---|---|
| tf ≤ t | D = 0, a = 0, return 0 |
| d > R | return R |
| d < k | D.x, D.y scaled by d/k, a = float(\|D\|), return d |
| otherwise, NaN included | return d |

`0x635550` returns 1 when head is set, or when `+0xa0` < 0 (or NaN) and, with Ta =
float(`0x7c5da0()`), either t + R < Ta, or t ≤ Ta and ((Ta − t)·`+0x84`)² > D.y² + D.x².

Addresses: `0x635450`, `0x63546c`, `0x635485`, `0x6354a5`, `0x6354be`, `0x6354cb`, `0x6354f7`, `0x63550e`, `0x635516`, `0x635545`, `0x635550`, `0x63557a`, `0x635597`, `0x6355b0`, `0x6355e7`, `0x7c5f50`
Confidence: verified
Proof: one
Test: `collision::fall_step_difftest`, `collision::fall_feasible_difftest`

`0x63546c` calls `0x635550` and keeps its return in `ecx`, which `0x6354b1` pushes as
`0x7c5f50`'s second argument. `0x635485`, `test ah,0x41; jne`, needs h > 2^-20; `0x6354a5`
divides the root by h, multiplies by R and stores. `0x6354cb`, `test ah,0x41; jp`, takes the
zero branch for tf < t and tf = t. `0x6354f7` returns R when d > R; `0x63550e`, `test ah,5;
jp`, scales only for d < k. `0x635516` is `d8 f9`, `fdivr st,st(1)`: d/k. D.x and D.y are stored
with `fst`, and `0x635545` stores the root of (sD.y² + sD.x²) + D.z² over the unrounded
products. In `0x635550`, `0x63557a` needs `+0xa0` < 0 or NaN; `0x635597` returns 1 for t + R <
Ta; `0x6355b0` returns 0 for t > Ta; `0x6355e7` needs the square strictly greater.

## Horizontal wall push

`0x635090(out, u, a, L, i, planes, count)` writes a horizontal push for the remaining move v =
u·(L − a). The wall normal w is `0x634ae0`'s when the hit triangle i is steep (`0x6336a0`), else
−planes[0].n. With ĥ = w.xy/|w.xy|, or w.xy itself when |w.xy| < 2^-22, q = −(w·u)·(L − a) /
(w.y·ĥ.y + ĥ.x·w.x + w.z·0.0), undivided when that denominator is below 2^-22, and out = (ĥ.x·q,
ĥ.y·q). For |w.xy| ≥ 2^-22 this is −(w·v)·w.xy/|w.xy|²: the move's part into w undone by
horizontal motion alone. `0x635600` adds it to x and y only.

Addresses: `0x635090`, `0x63509e`, `0x6350b6`, `0x6350c5`, `0x6350fa`, `0x635112`, `0x635122`, `0x63514e`, `0x635166`, `0x63517d`, `0x63517f`, `0x63518d`, `0x6336a0`
Confidence: verified
Proof: one
Test: `collision::ground_slide_difftest`

`0x63509e` calls `0x6336a0(i)`; `0x6350b6` calls `0x634ae0`, and `0x6350c5` negates the first
plane's normal. `0x6350fa` is `0x454980`, w.x² + w.y²; its root is not rounded. `0x635112`,
`test ah,5; jnp`, skips the normalization below 2^-22; ĥ.x is stored as a float and `0x635122`
leaves ĥ.y in the register. The numerator is ((−w.z·u.z + −w.y·u.y) + −w.x·u.x)·(L − a) at
`0x63514e`. `0x635166` is `fmul [0x7ffd74]`, 0.0, so w.z contributes nothing unless it is
infinite or NaN. `0x63517d` skips the division below 2^-22; `0x63517f` is `de f9`, `fdivp
st(1),st`: numerator over denominator. `0x63518d` stores out.x, then out.y.

## Wall normal from contacts

`0x634ae0(out, i, planes, count)` picks the wall normal from the contact planes `0x632ba0`
reported: count 3 or 4 gives triangle i's normal; 0, 1 or more than 4 give −planes[0].n. For 2,
c = planes[0].n × planes[1].n, normalized. It gives triangle i's normal when |c.z| < 2^-20, when
|c·n_i| < 2^-20, or when |c.z − 1| ≥ 2^-20 and `0x634960` finds i's plane within 1/720 of the
position or of a corner (x ± r, y ± r, z + 1.8493990·r). Else P = the point on all three planes
(`0x634fc0`), e = the edge of i nearest the line through P along c (`0x634da0`), w = c × e
normalized, and it returns −w when w·planes[0].n > 0, else w; −planes[0].n when |c × e| < 2^-22.

Addresses: `0x634ae0`, `0x634ae9`, `0x634b3b`, `0x634b99`, `0x634ba9`, `0x634bd4`, `0x634bfd`, `0x634c03`, `0x634c21`, `0x634c33`, `0x634c4a`, `0x634c91`, `0x634d06`, `0x634d6b`, `0x634960`
Confidence: verified
Proof: one
Test: `collision::slide_normal_difftest`, `collision::flush_probes_difftest`

`0x634ae9` switches on the count: 2 to `0x634b27`, 3 and 4 to `0x634d6b`, the rest to
`0x634b02`. `0x634b3b` is `de e9`, `fsubp st(1),st`, st1 − st0: c.z = n0.x·n1.y − n0.y·n1.x,
and so on. `0x634b99` is `fst`, so the |c.z| test at `0x634ba9` reads the unrounded value.
`0x634bd4` takes |(c.y·n.y + c.z·n.z) + c.x·n.x|. `0x634bfd` skips `0x634c03` for |c.z − 1| <
2^-20. `0x634c21` and `0x634c33` call the helpers; `0x634c4a` starts w = c × e with the same
`de e9` order. `0x634c91`, `test ah,5; jp`, normalizes for |c × e| ≥ 2^-22. `0x634d06` keeps w
for w·n0 ≤ 0 and NaN. In `0x634960` the height is r·1.8493990 (0x3fecb91b) and the test is
`fabs; fabs; fcomp 1/720`.

## Crease helpers

`0x634fc0(out, tri, A, B)` solves the three planes' normals as rows against −d, through the
determinant `0x7be0e0` and the inverse `0x7be2a0`, with no zero test. `0x634da0(out, i, P, c)`
walks triangle i's edges e_k = v_k − v_(k+1 mod 3), skipping any shorter than 2^-22, and keeps
the unit edge with the smallest squared distance: for an edge parallel to c (|e·c − 1| <
2^-20), `0x634f40(v_k, P, c)`, the squared distance from v_k to the line through P along c;
otherwise (k·v_k − k·P)², k = e × c, each dot summed z, y, x. The search starts at FLT_MAX and keeps only a strictly smaller
value; with none it returns (0, 0, 0).

Addresses: `0x634fc0`, `0x63501e`, `0x63502e`, `0x63504c`, `0x634da0`, `0x634dcd`, `0x634de0`, `0x634e2d`, `0x634e7b`, `0x634e81`, `0x634e9d`, `0x634ee3`, `0x634eed`, `0x634f40`, `0x634f72`, `0x634f7a`, `0x634f8b`, `0x634f92`, `0x7be0e0`, `0x7be2a0`, `0x7be1e0`
Confidence: verified
Proof: one
Test: `collision::sibling_pure_difftest`, `collision::slide_normal_difftest`, `cmath::det3b`, `cmath::inv3`, `cmath::adj3t`

`0x7be2a0(out, det)` scales the adjugate `0x7be1e0` writes by float(1/det).

`0x634fc0` copies the three normals to a 3×3 matrix and −d of each to a vector; `0x63501e`
takes the determinant, `0x63502e` the inverse scaled by its reciprocal, and `0x63504c` stores
out.x = (m02·b.z + m01·b.y) + m00·b.x. In `0x634da0`, `0x634dcd` stores 0x7f7fffff and
`0x634de0` reads the next-vertex index from {1, 2, 0}. `0x634e2d` skips short edges. `0x634e7b`,
`test ah,5; jp`, takes the cross branch unless parallel; `0x634e81` calls `0x634f40` with
`ecx` = v_k and `edx` = P. `0x634e9d` and the next two `de e9` give e × c; `0x634ee3` squares.
`0x634eed`, `test ah,5; jp`, keeps only a smaller value. `0x634f40` projects v_k − P on c, and
rounds t·c.y and t·c.z to floats (`0x634f72`, `0x634f7a`), the projected point's z
(`0x634f8b`) and dx = v.x − (t·c.x + P.x) (`0x634f92`) before the squared distance.

## Triangle class tests

For triangle i of `0xc4e52c` with normal z = n.z, the slope limit s is cos 50° = 0.64278764
(0x3f248dbb) when `0x5fa550` returns nonzero for the unit, else cos 80° = 0.17364818 (0x3e31d0d4):

| function | returns |
|---|---|
| `0x6336a0(i)` | 1 when −cos 50° ≤ n.z ≤ cos 50°, else 0, NaN included |
| `0x6335d0(i, p)` | 1 when n·p + d is at most 1/12, or NaN, for each of the first three planes `0x632460` builds for i |
| `0x633760(p, i, unit)` | 0 when n.z > s, or NaN, and `0x6335d0(i, p)`; else 1 |
| `0x633720(i, D)` | 1 when `0x633760(+0x10 + D, i, +0x15c)` returns 0 |
| `0x6333d0(i)` | 1 when a top corner (x ± r, y ± r, z + h) lies within 1/720 of i's plane |
| `0x6334a0(i, &f)` | n.z > s or NaN: 1, clearing 0x4000000 when `0x6335d0(i, +0x10)`; n.z ≥ 0: flags & 0x4000000; n.z < 0 without `0x6333d0(i)`: the same; with it: f = 1 and 0x4000000 cleared if it was set, then 1 when −n.z > cos 50° or NaN, else 0 |

Addresses: `0x6336c3`, `0x6336d3`, `0x633701`, `0x63364a`, `0x633671`, `0x633784`, `0x6337a5`, `0x633755`, `0x633463`, `0x633475`, `0x6334f9`, `0x633507`, `0x633526`, `0x633566`, `0x633595`, `0x632460`, `0x5fa550`
Confidence: verified
Proof: one
Test: `collision::ground_slide_difftest`, `collision::point_in_tri_difftest`, `collision::walkable_hit_difftest`, `collision::flush_probes_difftest`, `collision::slope_classify_difftest`, `objectlayer::dot3_difftest`, `terrain::plane_offset_negate_difftest`, `terrain::side_plane_d_negate_difftest`, `terrain::scene_dot_negate_difftest`, `terrain::wmo_plane_offset_negate_difftest`

`0x6336c3`, `test ah,0x41; jp`, returns 0 for n.z > cos 50° and NaN; `0x6336d3` returns 1 for
n.z ≥ 0; `0x633701` tests −n.z the same way. `0x63364a` passes the vertices, the index bytes
{0, 1, 2}, the triangle, (0, 0, 1) and four planes; `0x633671` compares (n.x·p.x + n.z·p.z) +
n.y·p.y + d with 1/12 (0x3daaaaab), `test ah,0x41; je` returning 0 above it. `0x633784` and
`0x6337a5` (`jnp`) give the slope test, NaN counting as floor; `0x633755` is `neg; sbb; inc`.
`0x633463` calls `0x602630`, a dot product; d is added and the sum tested at `0x633475`. In `0x6334a0`, `0x6334f9` is the
slope test, `0x633507` and `0x633566` clear 0x4000000, `0x633526` tests n.z < 0 with `test ah,1`,
and `0x633595` is `test ah,0x41; jp`.

## Fall latches

`0x633220` restarts the fall from rest: `+0xa0` = 0, `+0x78` = 0, `+0x7c` = `+0x18`, and flag
0x4000 set. `0x633240` sets 0x4000 by one of two tests. It does nothing when the flags hold 0x4000
or 0x20000000. With 0x2000 set and `+0xa0` ≠ 0 (NaN included) it sets 0x4000 when `+0x7c` −
0.11111111 ≥ `+0x18`, NaN included; otherwise, falling or not, when `+0x78` ≥ 500.

Addresses: `0x633220`, `0x633231`, `0x633240`, `0x633243`, `0x63325e`, `0x633266`, `0x633271`, `0x633280`
Confidence: verified
Proof: one
Test: `collision::resolve_state_difftest`

`0x633231` is `or dword [ecx+0x40],0x4000`, after the stores of 0 and of `+0x18`. `0x633243`
is `test edx,0x20004000; jne` to the return. `0x63325e`, `test ah,0x44; jnp`, sends `+0xa0` = 0
to the timer test. `0x633266` subtracts 1/9 (0x3de38e39); `0x633271`, `fcomp [ecx+0x18]; test
ah,5; jnp`, returns only for `+0x7c` − 1/9 < `+0x18`. `0x633280` is `cmp dword [ecx+0x78],0x1f4;
jb`, unsigned. `0x635600` calls `0x633220` on a head strike and, with report set, `0x633240` while 0x2000 holds;
`0x635b00` calls `0x633240` after a fall step taken without sweeping, when the list is empty.
