# collision: contact

The geometry the movement resolvers sweep: the volume a unit occupies, the class mask of its
world queries, the gather of world facets around a move, and the first contact of the moving
volume with them.

The code is the low end of the resolution layer, `0x631440` through `0x632f80`. The volume is a
square box of half-width r (`CMovement` +0xb0) and height H (+0xb4), closed below by an inverted
square pyramid whose apex is the unit's position. A plane is four floats (n.x, n.y, n.z, d), and a
point q is inside it when n·q + d ≤ 0. A world facet is a 52-byte record: its plane at +0x00 and
its three vertices at +0x10. The world query fills two facet lists, `0xc4e52c` (solid) and
`0xc4e564` (liquid, for a swimmer), each a count at +4 and records at +8, and the box the solid
list covers is cached in `0xc4e5a0`. A contact is found by clipping each facet to the prism one
face of the volume sweeps, then measuring along the move how far that face travels to reach what
is left.

Nothing outside collision calls this code; the resolvers and the mover's fit test reach it.

Roots: none.

## Volume planes

`0x631440(p, H, r, out)` writes the nine planes of the volume at position p: a box of half-width
r around p up to p.z + H, closed below by four bevels through p. With a = 0.8796419
(0x3f613036, `0x80dfe4`) and b = 0.4756366 (0x3ef386a4, `0x80dfe0`), the sine and cosine of
61.6°, a bevel rises a/b = 1.849399 per yard out from p. Each d is one 53-bit sum in the order
shown, stored as a float; the products by 0.0 carry a NaN or infinite coordinate into d.

| plane | normal | d |
|---|---|---|
| 0 | (−1, 0, 0) | p.x − r |
| 1 | (1, 0, 0) | −p.x − r |
| 2 | (0, 1, 0) | −p.y − r |
| 3 | (0, −1, 0) | p.y − r |
| 4 | (0, 0, 1) | −p.z − H |
| 5 | (−a, 0, −b) | p.x·a + p.z·b + p.y·0 |
| 6 | (a, 0, −b) | p.z·b − p.x·a + p.y·0 |
| 7 | (0, a, −b) | p.x·0 + p.z·b − p.y·a |
| 8 | (0, −a, −b) | p.x·0 + p.z·b + p.y·a |

Addresses: `0x631440`, `0x80dfe0`, `0x80dfe4`, `0x63145f`, `0x6314cc`, `0x63154c`, `0x6315b3`
Confidence: verified
Proof: one
Test: `collision::kdop_difftest`

`ret 0x10`, and ecx is loaded from the first argument before use. Plane 0 is `mov [eax],
0xbf800000` and `0x63145f`, the store of `fld [p.x]; fsub r`, so the third argument is r; plane 4
subtracts the second (`0x6314cc`). The bevel normals come from locals set to 0xbf613036 and
0xbef386a4 (`0x631490`, `0x63149d`) and the zero in edx. Plane 6's d ends in `fsubp st(1),st`
(`de e9`, `0x63154c`), st(1) − st(0), which is p.z·b − p.x·a; plane 7's (`0x6315b3`, also `de e9`) is
(p.x·0 + p.z·b) − p.y·a. The product of two floats is exact at 53 bits, so each
d rounds twice, at the sum and at the float store. `0x631670` passes its own arguments, height then
radius (`0x631787` to `0x63179d`); `0x631be0` passes +0xb0 as the radius.

## Volume vertices and faces

`0x631be0(p, H, planes, verts, idx)`, on a `CMovement`, writes the planes of `0x631440(p, H, r)`
with r = +0xb0, nine vertices and 32 index bytes. V0 = p is the apex. V1 to V4 are the corners
(p.x − r, p.y − r), (p.x − r, p.y + r), (p.x + r, p.y + r) and (p.x + r, p.y − r) at the waist
p.z + r·1.849399 (0x3fecb91b, `0x80c740`); V5 to V8 are the same corners at p.z + H. The index
bytes list the faces of the planes in plane order:

| bytes | vertices | plane |
|---|---|---|
| 0 to 3 | 1, 2, 6, 5 | 0, x − r |
| 4 to 7 | 3, 4, 8, 7 | 1, x + r |
| 8 to 11 | 2, 3, 7, 6 | 2, y + r |
| 12 to 15 | 4, 1, 5, 8 | 3, y − r |
| 16 to 19 | 5, 6, 7, 8 | 4, top |
| 20 to 22 | 0, 1, 2 | 5 |
| 23 to 25 | 0, 3, 4 | 6 |
| 26 to 28 | 0, 2, 3 | 7 |
| 29 to 31 | 0, 4, 1 | 8 |

Addresses: `0x631be0`, `0x80c740`, `0x631c00`, `0x631c0b`, `0x631c4d`, `0x631cf9`, `0x631de6`, `0x631e59`
Confidence: verified
Proof: one
Test: `collision::resolve_clip_difftest`

`ret 0x14`. `0x631c00` calls `0x631440` with r pushed from `[edi+0xb0]`. `0x631c0b fmul
[0x80c740]` leaves r·1.849399 on the x87 stack unrounded; V1 to V4 each add it to p.z (`0x631c4d`
is V1's) and V4's store pops it (`0x631cf9`), so the waist z is float(p.z + r·1.849399) taken at
53 bits. Each x and y is float(±r + p). The index bytes are the store block `0x631de6` to `0x631e59`,
with bl = 3, dl = 4 and cl = 7, then cl = 0 from `0x631df4`. A face's index list and its plane
agree: V1 and V2 have x = p.x − r, the face of plane 0 and of bevel 5.

## Collision class mask

`0x6315f0` returns the class mask of a `CMovement`'s world queries, built in this order; a pitch
equal to the bound, or NaN, adds nothing.

| step | test | mask |
|---|---|---|
| 1 | `0x5fa550` nonzero for the owning unit (+0x15c) | 0x100111, else 0x102111 |
| 2 | flags (+0x40) have 0x10000000 and lack 0x200000, pitch (+0x20) above −0.6457718 (0xbf25514d, `0x80dfe8`, −37°) | or 0x30000 |
| 3 | the unit's type mask has 0x10, a player, and the player's flags have 0x10 | or 0x8000 |

`0x5fa550` reads the unit field block at +0x110, the descriptors from +0x18, whose +0xa0 holds
the unit flags. It returns 0 when flag 0x2 is clear and any of 0xc00004 is set; under flag
0x1000000, the answer for the unit whose GUID is at the block's +0x10 (+0x20 when that is zero),
0 when no unit has it; otherwise 1 only for a player-type unit with a zero GUID at the block's
+0x10 and flag 0x1 clear.

Addresses: `0x6315f0`, `0x5fa550`, `0x80dfe8`, `0x6315fa`, `0x631608`, `0x63160d`, `0x631610`, `0x631617`, `0x63161e`, `0x63162c`, `0x63162e`, `0x631648`, `0x631658`
Confidence: verified
Proof: one
Test: `collision::mask_select_difftest`

`0x6315fa mov edi,0x100111`; `0x631606 jne 0x63160d` skips `0x631608 mov edi,0x102111` on a
nonzero `al`, and both paths load the flags at `0x63160d`. `0x631610 test eax,0x10000000; je`,
`0x631617 test eax,0x200000; jne`, then `0x63161e fld [esi+0x20]; fcomp [0x80dfe8]; fnstsw; test
ah,0x41; jne` at `0x63162c`: C0 (below), C3 (equal) or the unordered set skips `0x63162e or
edi,0x30000`. `0x63163d` to `0x631643` test bit 4 of the unit's type field, the dword at +8 of its
descriptors; `0x631648` loads the pointer at +0xe68, which the player setup sets to the
descriptors + 0x2f0 (`mov [esi+0xe68],ebx`), where the player fields begin; `0x631658 or edi,0x8000`. That 0x200000,
0x10000000 and player flag 0x10 are swimming, water walking and the ghost flag is inferred from
their other readers; what each mask bit selects is decided inside the world query, not read here.

## Polygon clip

A polygon holds up to 15 vertices (12 bytes each from +0x00), a tag per vertex (+0xb4) and a
count (+0xf0). `0x6318c0(plane, poly, tag)` takes s = −(n·v + d) per vertex, rounded to float.
With no vertex, or every s above −1/720, the polygon is unchanged; else with every s below 1/720
it is emptied; a NaN s takes part in neither test. Otherwise each edge (prev, cur), from (last,
first), is clipped with s ≥ 0 or NaN inside:

| prev | cur | emits |
|---|---|---|
| in | in | cur |
| out | in | the crossing when s_cur > 1/720, then cur |
| in | out | the crossing when s_prev > 1/720 |
| out | out | nothing |

A crossing takes the tag. Fewer than three vertices empty the polygon. `0x631870(planes, n, poly)`
clips by planes 0 to n − 1 with tag i, returning 0 once empty and 1 otherwise; `0x632980` skips
one index.

Addresses: `0x6318c0`, `0x631870`, `0x632980`, `0x80dfec`, `0x80dff0`, `0x6318f2`, `0x631909`, `0x631938`, `0x63194c`, `0x6319e7`, `0x631a36`, `0x631a3f`, `0x631b6b`, `0x631b01`
Confidence: verified
Proof: one
Test: `collision::resolve_clip_difftest`, `collision::sibling_pure_difftest`

s is `fld v.x; fmul n.x; fld v.z; fmul n.z; faddp; fld v.y; fmul n.y; faddp; fadd d; fchs`
(`0x6318f2` to `0x631909`). The minimum test `fcomp st(1); test ah,0x41; jne` stores only when the
minimum exceeds s, the maximum test `test ah,5; jp` only when the maximum is below s. `0x631938`
compares the minimum with −1/720 (0xbab60b61, `0x80dff0`) and `je` returns when it is above;
`0x63194c` compares the maximum with 1/720 (0x3ab60b61, `0x80dfec`) and `jnp` empties when it is
below. The loop tests s_prev, then s_cur, against 0.0 with `test ah,5` (`0x6319e7`): parity is odd
only below zero. The crossing is prev − t·(cur − prev) with t = float(s_prev / (s_cur − s_prev)),
`fdivr` (`d8 7d f8`, `0x631a3f`). Out to in rounds Δz to float first (`0x631a36`), x as
prev.x − float(Δx·t), and keeps Δy·t unrounded; in to out rounds each Δ·t (`0x631b6b` to
`0x631b79`). `0x631b01` empties a result below three. `0x631870` and `0x632980` pass the plane
index as the tag and test +0xf0 after each call.

## Obstruction test

`0x631670(H, r)` tells whether a `CMovement`'s volume of height H and radius r meets solid
geometry. It writes the box (−r, −r, 0) to (r, r, H) into `0xc4e5a0`, moves it by the position
`0x7c4930` returns, and calls `0x6721b0(box, 0xc4e52c, mask, 0)` with the mask of `0x6315f0`. A
zero return returns 1. Otherwise it builds the planes `0x631440(+0x10, H, r)` and returns
`0x6317c0(planes, 9)`: 1 at the first gathered facet that keeps any part after the clip by all
nine planes, else 0. On a transport the box is at the world position `0x7c4930` gives, the planes
at the stored +0x10.

Addresses: `0x631670`, `0x6317c0`, `0x7c4930`, `0x6721b0`, `0xc4e5a0`, `0xc4e52c`, `0x63168f`, `0x6316c5`, `0x63172f`, `0x631738`, `0x63179d`, `0x6317ad`, `0x631816`, `0x617460`
Confidence: verified
Proof: one
Test: `collision::query_aabb_difftest`

`ret 8`. The box is −r at `0x63168f` (`fst`) and `0x631698`, 0 at `0x6316b5`, r at `0x6316a0` and
`0x6316ac`, H at `0x6316bf`; each component then adds `0x7c4930`'s vector (`0x6316c5`,
`0x6316ca` to `0x631729`). `0x63172f` calls the query; `0x631738` returns 1 for a zero `al`. The nine
local planes start as (0, 0, 1, 0); `0x63179d` overwrites them. `0x6317c0` walks the count
at `0xc4e530`, copying each record's 36 bytes from +0x10 into a polygon of three vertices tagged −1
(`0x631816`), and returns 1 on the first nonzero `0x631870`. Its one caller, the mover's
`0x617460`, passes the height and radius it scales. The test runs `0x631685` to `0x63172f`,
with `0x7c4930` stubbed.

## Facet gather

`0x631e70(m)`, m a move by value, fills the facet lists for a sweep of a `CMovement` by m, with
r = +0xb0 and H = +0xb4, in these steps. It returns 1 unless the solid query fails.

| step | effect |
|---|---|
| 1 | on a transport (+0x38 nonzero): p (+0x10) and m to world by `0x630ac0`'s matrix M |
| 2 | e = p + m; when the cached box `0xc4e5a0` holds (e.x − r, e.y − r, e.z) and (e.x + r, e.y + r, e.z + H): return 1 |
| 3 | query box: that box grown by 1/6 on every side, joined with the cached box |
| 4 | solid query into `0xc4e52c`, which empties the list first; failure returns 0 with the cache unchanged, so a later box inside it returns 1 over the emptied list |
| 5 | a swimmer (0x200000): liquid query into `0xc4e564`, its return ignored, every liquid plane reversed |
| 6 | the cache becomes the query box |
| 7 | on a transport: each solid facet into transport space by M⁻¹ |

Addresses: `0x631e70`, `0x630ac0`, `0x7bcc60`, `0x637350`, `0x637300`, `0x6372d0`, `0x6373b0`, `0x6721b0`, `0x6315f0`, `0x6320a0`, `0x6320e0`, `0x6320fe`, `0x6721c7`, `0x6721e3`, `0x637330`, `0x597ad0`, `0x63214a`, `0x7bd820`, `0x7bd700`, `0x7bdc40`, `0xc4e5a0`, `0xc4e52c`, `0xc4e564`
Confidence: verified
Proof: one
Test: `collision::static_overlap_difftest`, `collision::static_overlap_transport_difftest`, `cmath::mat4_affine_invert`

`ret 0xc`. M starts as the identity; `0x7bcc60` maps p in place and m becomes m·M by rows, x =
(m.z·M[8] + m.y·M[4]) + m.x·M[0], each rounded to float; e is float(p + m). The containment test
is `0x637350` (inclusive, NaN fails) at `0x63201d` and `0x63202f`. The growth is `0x637300` and
`0x6372d0` with 0x3e2aaaab (0.16666667); the join is `0x6373b0`, followed by a call to an empty
function. The solid query takes the mask of `0x6315f0` (`0x6320a0`) and a null fourth argument.
`0x6721b0` empties the list it is given (`0x6721c7` to `0x6721e3`) before it queries. The
liquid query uses the mask 0x30000 and its return is not tested (`0x6320e0`); `0x6320fe fchs` negates d, `0x637330` the normal, and
`0x597ad0` builds the plane copied back over the record. `0x63214a` stores the cache. On a
transport `0x7bd820(M, 1.0)` takes its unit-scale branch `0x7bd700`: Rᵀ, then `0x7bdc40` adds −t·Rᵀ, M⁻¹ for an orthonormal R; `0x7bcc60` moves the three vertices in place; the normal
becomes n·M⁻¹ by rows, each rounded to float; d = −((n.z·v0.z + n.y·v0.y) + n.x·v0.x)
(`0x632265`). The liquid list is not transformed.

## Face prisms

`0x632460(V, tri, face, s, out)` and `0x632f80(V, quad, face, s, out)` bound the prism a volume
face sweeps along s. Each edge A = V[k_j] to B = V[k_(j+1)], with A' = A + s, gives one plane:

| case | result |
|---|---|
| squared length of (B − A) × (A' − A) below 2^−20 (0x35800000) | return 0; NaN continues |
| otherwise | plane through A, B, A', normal normalize((B − A) × (A' − A)) by `0x637480` |
| a face vertex off the edge strictly on its positive side | the plane reversed with d |
| after the edges, out[3] or out[4] | the face's normal, d = −n·(V[k_0] + s), V[k_0] + s unrounded |

The vertex off the edge is the triangle's k_(j+2), the quad's k_(j−1). Both return 1. The callers
clip by the edge planes alone, so the region is the face outline extruded along s both ways.

Addresses: `0x632460`, `0x632f80`, `0x637480`, `0x63256f`, `0x6325b5`, `0x6325d4`, `0x632614`, `0x632629`, `0x6326a2`, `0x6326e1`, `0x633126`, `0x6331a1`, `0x632dc0`, `0x6322e3`, `0x633080`, `0x6330c6`, `0x63313b`, `0x6331e8`
Confidence: verified
Proof: one
Test: `collision::edge_planes_difftest`

Both are `ret 0xc` with the vertices in ecx and the index bytes in edx. A' is float(A + s) and
A' − A is recomputed as floats. The cross product's three `fsubp st(1),st` (`de e9`, `0x63256f`,
`0x632581`, `0x632593`) give z = u.y·w.x − w.y·u.x, y = w.z·u.x − u.z·w.x, x = u.z·w.y − u.y·w.z for
u = A' − A and w = B − A, that is w × u. `0x6325b5` compares (c.y² + c.z²) + c.x² with
9.5367432·10⁻⁷ by `test ah,5; jnp` (below returns 0). `0x6325d4` calls `0x637480(A, B, A')`. The
side dot sums (z, y, x) in the triangle (`0x632614`) and (z, x, y) in the quad (`0x633126`); the
test at `0x632629`, `test ah,0x41; jne`, keeps the plane at or below zero and on NaN. For the
same s·n, the box-face test of `0x632ba0` sums (z, y, x) (`0x632dc0`) and the bevel test of
`0x632280` (z, x, y) (`0x6322e3`). `0x6326e1` stores the cap's d as −((z·n.z + y·n.y) + x·n.x)
of V[k_0] + s, a sum left unrounded on the stack (`0x6326a2`, `0x6331a1`). The quad runs the same
code for four edges (`0x633080`, `0x6330c6`, `0x63313b`, `0x6331e8`), the neighbours taken by
`(j ± 1) & 3`.

## Contact distance

`0x6329e0(q, plane, dir)` returns (n·q + d)/(n·dir) at 53 bits, or n·q + d undivided when
abs(n·dir) is below 2^−22 (0x34800000); a NaN n·dir divides. For a volume face and a unit dir it is
the travel after which the face reaches q, negative behind it. `0x632830(poly, dir, planes, k,
best)` measures each vertex of the polygon against plane k:

| step | effect |
|---|---|
| 1 | m = *best; at each t below m, m = t when t > 0, else 0; a NaN t is skipped |
| 2 | some t above −2^−20 (0xb5800000, `0x80dff4`): go to step 4 |
| 3 | clip by the other eight planes; nothing left, or every remaining t at or below −1/36 (0xbce38e39) or NaN: return 0 |
| 4 | m below *best, or *best NaN: *best = m, return 1; else return 0 |

Addresses: `0x6329e0`, `0x632830`, `0x632980`, `0x80dff4`, `0x6329fa`, `0x632a15`, `0x632a28`, `0x63287e`, `0x632888`, `0x632895`, `0x6328a1`, `0x6328d7`, `0x632910`, `0x63294a`
Confidence: verified
Proof: one
Test: `collision::resolve_toolkit_difftest`, `collision::toi_difftest`

`0x6329e0` is `ret 4` with q in ecx and the plane in edx: n·q is (q.z·n.z + q.y·n.y) + q.x·n.x and
`0x6329fa fadd d`; `0x632a15` compares |n·dir| with the bound, `test ah,5; jp 0x632a28` divides
above, at or NaN, and `0x632a28 fdivp st(1),st` (`de f9`) is st(1)/st(0). In `0x632830`, `0x63287e
fcom m; test ah,5; jp` skips t at or above m and NaN; `0x632888 fcom 0.0; test ah,0x41; jne` stores 0
for t ≤ 0, else `0x632895 fst` rounds t into m while the stack keeps it; `0x6328a1 fcomp
[0x80dff4]; test ah,0x41; jne` clears the all-behind flag only for t above it. `0x6328d7` calls
`0x632980` with nine planes, skipping k; `0x632910` compares with −1/36. `0x63294a fcomp [best];
test ah,1; jne` stores on C0, below or unordered.

## Nearest facet per face

`0x632700(list, dir, prism, n, planes, k, best, hit)` finds, for volume face k, the facet the face
reaches first. A record is skipped unless n·dir ≤ −1·10⁻⁵ (0xb727c5ac, `0x80c5c4`) or NaN: only a
facet facing against the motion counts. Its three vertices, tagged −1, are clipped by the n prism
planes; what is left goes to `0x632830` with the distance FLT_MAX (0x7f7fffff). A distance at or
below *best, never NaN, is stored in *best with the record's index in *hit. Returns 1 when any
record was stored. Two more dwords, the owner's GUID, are passed and never read.

Addresses: `0x632700`, `0x632830`, `0x631870`, `0x80c5c4`, `0x632753`, `0x63276d`, `0x6327ae`, `0x6327cc`, `0x6327e2`, `0x63234d`, `0x632e37`
Confidence: verified
Proof: one
Test: `collision::per_face_toi_difftest`

`ret 0x20`, the list in ecx (count at +4, records at +8) and dir in edx; the seventh and eighth
stack dwords are not read. `0x632753` to `0x63276b` form (n.z·dir.z + n.y·dir.y) + n.x·dir.x, and
`0x63276d fcomp [0x80c5c4]; test ah,0x41; je` skips only above the bound. `0x6327ae` clips with
`0x631870`; `0x6327cc` stores 0x7f7fffff. `0x6327e2 fcomp [best]; test ah,0x41; jp` skips above or
unordered and takes equal. *best enters as the caller's cap, the sweep length, for each face. The
callers are `0x63234d` in `0x632280`, with three prism planes, and `0x632e37` in `0x632ba0`, with
four.

## Bevel contacts and the band

`0x632280(list, dir, len, s, planes, V, idx, hit, out, count, best)`, on a `CMovement`, runs the
four bevels (planes 5 to 8, triangles from index byte 20). A bevel is leading when s·n > 0 or NaN;
`0x632460` builds its prism and `0x632700` searches it with the cap len. A face finding a facet at
t joins the contact list by the band rule, which `0x632ba0` applies to the box faces too. With
b = *best, in order:

| test | effect |
|---|---|
| t < b − 1/720 | the list becomes this face's plane alone |
| else t < b + 1/720 | the plane is appended |
| then t < b | *best = t; with more than one entry, the last and the first swap |

The tests are at 53 bits; a NaN changes nothing. The list holds the volume's own face planes;
the facet is known only by hit.

Addresses: `0x632280`, `0x632460`, `0x632700`, `0x6322ff`, `0x632311`, `0x63235f`, `0x632396`, `0x6323d3`, `0x6323eb`
Confidence: verified
Proof: one
Test: `collision::candidate_collect_difftest`

`ret 0x2c`. The faces are planes + 0x50 onward, stepping 16, with the index bytes from +0x14,
stepping 3. The leading test is (n.z·s.z + n.x·s.x) + n.y·s.y against 0.0 with `test ah,0x41; jnp`
(`0x6322ff`), skipping below or equal. `0x632311` builds the prism; `0x63234d` passes the face
index + 5 and len, reloaded for each face. `0x63235f fsub [0x80dfec]; fcomp t; test ah,0x41; jne`
resets the list only when b − 1/720 > t; `0x632396` appends only when b + 1/720 > t; `0x6323d3
fcomp b; test ah,5; jp` updates only when t < b; `0x6323eb` swaps entry count − 1 with entry 0.
hit ends as the index stored by the last face that found anything, not always the nearest facet.

## Earliest contact sweep

`0x632ba0(list, dir, len, hit, out, count, dist, pos)` sweeps a `CMovement`'s volume from pos
(+0x10 when null) along the unit vector dir for len. It returns in dist the travel to the first
contact, and in out and count the leading faces that touch within 1/720 of it, by these steps. A
NaN len sweeps 1/36, finds nothing and returns a NaN dist.

| step | effect |
|---|---|
| 1 | count = 0; when abs(len) < 2^−20: dist = 0, hit = the list's count, return 1 |
| 2 | H = +0xb4; three quarters of it for a swimmer (0x200000) sweeping `0xc4e564` |
| 3 | else, with flags 0x2000 and 0x40000000: drop D by `0x632a30` (failure returns 0); H + D, pos.z − D |
| 4 | s = dir · max(len, 1/36) |
| 5 | the volume `0x631be0(pos, H)` |
| 6 | when pos is null: `0x631e70(s)`; failure: dist = 0, return 0 |
| 7 | hit = the list's count, best = len; the bevels by `0x632280` |
| 8 | box faces 0 to 4 with s·n > 0 or NaN: prism `0x632f80`, `0x632700` with cap len, the band rule |
| 9 | dist = best, or 0 when best < 1/720; return 1 |

Addresses: `0x632ba0`, `0x632a30`, `0x631be0`, `0x631e70`, `0x632280`, `0x632f80`, `0x632700`, `0xc4e564`, `0x632bb3`, `0x632c64`, `0x632c6f`, `0x632c7a`, `0x632cba`, `0x632d07`, `0x632d29`, `0x632d74`, `0x632ddd`, `0x632f3d`
Confidence: verified
Proof: one
Test: `collision::earliest_contact_difftest`

`ret 0x20`. `0x632bb3` compares abs(len) with 9.5367432·10⁻⁷, `test ah,5; jp` continuing above, at or
NaN. `0x632c64 cmp ebx,0xc4e564` gates `0x632c6f fmul [0.75]` (0x3f400000) under 0x200000;
`0x632c7a test ah,0x20` and `test eax,0x40000000` gate the drop, called with D = 1.0. `0x632cba
fcomp [1/36]` (0x3ce38e39), `test ah,0x41; jne`, takes 1/36 for len below, equal or NaN; s keeps
the factor on the stack for its three products, each stored as a float. `0x632d07` builds the
volume, `0x632d29` gathers, `0x632d74` runs the bevels. The box loop's leading test `0x632ddd`
matches the bevels'. `0x632f3d` compares best with 1/720, `test ah,5; jp`, storing 0.0 only below.
That 0x200000, 0x2000 and 0x40000000 are swimming, falling and hover is inferred from their other
readers.

## Hover drop

`0x632a30(list, D, pos)` measures how far the volume at pos (+0x10 when null) can drop straight
down, up to its cap of 1 yard. It builds the volume of height +0xb4 with `0x631be0`, gathers for
the move (0, 0, −1) when pos is null (failure stores 0 in D and returns 0), and runs `0x632280` with
direction and sweep (0, 0, −1), cap 1.0 and D as the best, keeping the contact planes in a local
list. Only the four bevels can face downward. A D below 1/720 becomes 0. Returns 1. `0x632ba0`
calls it with D = 1.0.

Addresses: `0x632a30`, `0x631be0`, `0x631e70`, `0x632280`, `0x632acc`, `0x632af6`, `0x632b17`, `0x632b23`, `0x632b6c`, `0x632b73`
Confidence: verified
Proof: one
Test: `collision::step_up_difftest`

`ret 0xc`, the list, the float pointer D and the position. `0x632acc` to `0x632aef` set two local
vectors to (0, 0, −1.0) (0xbf800000); `0x632af6` builds the volume, `0x632b17` gathers, and
`0x632b23` stores the zero `eax` as D on failure. `0x632b6c` passes 1.0 (0x3f800000) as the cap,
the facet count as the starting hit and seven local planes as the list. `0x632b73` compares D with
1/720 under `test ah,5; jp`, storing 0 only below. The box faces have normals with no downward
part, so a sweep along −z never has them leading; this function does not try them.
