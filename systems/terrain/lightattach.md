# terrain: light node attach

How a map light node finds the map object group it stands in: the node's state sync when its object
moves, the two points it probes from, the down-ray attach of units and doodads, the containment attach
of game objects, the face sets of the queries they use and the two legs of the footprint bake. The code
is the light node file from `0x6717d0` and `0x6a86d0` to `0x6a91e8`, with the group queries of the
map object (`models/wmo`). A light node keeps its position at `+0x24`, its world box at `+0x44` and
`+0x50`, the box centre at `+0x5c`, the radius at `+0x68` and two probe points at `+0xa8` and `+0x5c`.

Roots: `0x6717d0`, `0x6a86d0`, `0x6a8a20`, `0x6a8c10`, `0x6a8ed0`, `0x69e4c0`

## Node sync

`0x6717d0(matrix, local box, point, force)` stores the node's state from the object's world matrix. The
position `+0x24` is the matrix translation; the scale `+0x30` is the length of its first row. The world
box `+0x44`/`+0x50` is the local box under the matrix, with centre `+0x5c` and radius `+0x68` to the maximum corner. A local box with any minimum above its maximum, or
a NaN, collapses box and centre onto the position and the radius to 0. `+0xa8` is the matrix applied to the
point argument. The node is relit (`0x69e280`) when the position moved by a squared step of 0.001 or more, or the
force argument is set; a NaN step relights too (`0x67182d`).

Addresses: `0x6717d0`, `0x6717e3`, `0x671822`, `0x671842`, `0x671873`, `0x6718f7`, `0x671933`, `0x67193d`, `0x671973`, `0x671981`, `0x671989`, `0x6719b9`, `0x6719cc`, `0x6719f5`, `0x6719fa`, `0x671a3a`, `0x671a48`, `0x671a63`, `0x671a73`, `0x6dc540`, `0x6dc470`, `0x621160`, `0x7bca80`, `0x7ffa24`
Confidence: verified
Proof: one
Test: `terrain::camera_set_difftest`

`ret 0xc`, `ecx` the node and `edx` the matrix. The local box compares `min > max` per axis with `test
ah,0x41; jp` (`0x67193d`, `0x67194e`, `0x67195f`), which jumps for a larger minimum and for NaN. `0x6dc540` seeds
both corners with the matrix translation and `0x6dc470` folds the three columns over the box. `0x621160`
adds the corners and `0x671989` to `0x6719a8` halve each sum. `0x7bca80` is the affine point transform
(`out = p·M₃ₓ₃ + translation`). The six call sites are listed with `terrain/mapentities#light-node-classes`.

`camera_set_difftest` runs `0x6717d0` in six stretches between `0x6717da` and `0x6719f8`.

## Node probe points

A node probes from two points that are neither the box centre. The down-ray attach starts at `+0xa8`,
and the containment attach at the centre `+0x5c`. A game object's sync passes the model's bounding box
(header `+0xb4`) as the local box and the centre of its collision box (header `+0xd0`) as the point, so the
centre follows the bounding box and `+0xa8` the collision box; the two coincide with the position
when the collision box is all zero. The footprint bake takes a third point, `+0x24`.

Addresses: `0x5f7910`, `0x5f7997`, `0x5f79ab`, `0x5f79c6`, `0x5f79da`, `0x5f7a11`, `0x5f7a16`, `0x5f7a2b`, `0x713640`, `0x713700`, `0x5f8cf0`, `0x4549a0`
Confidence: verified
Proof: one
Test: none

`0x713640` copies the six floats at the model header's `+0xb4` and `0x713700` those at `+0xd0` (`add esi,0xb4`
at `0x713661`, `add esi,0xd0` at `0x713721`), each through `[[model+0x30]+0x130]`. `0x5f79ab` stores the
first at `+0xc0` of the object; `0x5f79df` to `0x5f7a11` average the corners of the second into a point
(`0x5f8cf0` with 0.5); `0x5f7a1c` pushes the force flag 1. The second accessor reads its model from `+0xdc` of
the object, else `+0xd8` (`0x5f79c6`).

## Down-ray attach

`0x6a8a20` finds what a unit, player or doodad node stands on. It traces two segments that both start at
`+0xa8`: the first ends 1000 below it, and only when the terrain trace and the map object trace
(`0x6a8840`) both miss the second runs, ending at the box centre's x and y at its z plus the radius plus 1000.
The map object result wins an equal or NaN comparison of the two fractions, the terrain a strictly smaller one; a lone hit stands. A map
object hit links a membership record to the hit group and classifies the node from it; a terrain hit links
the chunk; a miss links nothing.

Addresses: `0x6a8a20`, `0x6a8a29`, `0x6a8a51`, `0x6a8a54`, `0x6a8a81`, `0x6a8a9f`, `0x6a8aa6`, `0x6a8aae`, `0x6a8ab0`, `0x6a8ad1`, `0x6a8ae6`, `0x6a8b04`, `0x6a8b11`, `0x6a8b15`, `0x6a8b1d`, `0x6a8b20`, `0x6a8b2d`, `0x6a8b3a`, `0x6a8b47`, `0x6a8bcd`, `0x6a8bd6`, `0x69c320`, `0x6a8840`, `0x8022cc`
Confidence: verified
Proof: one
Test: `terrain::cmapdoodaddef_ground_attach_difftest`

The start is copied from `+0xa8` (`0x6a8a29` to `0x6a8a4d`) and never changed. `0x8022cc` is 1000.0, subtracted
at `0x6a8a54`, and added to `+0x68` at `0x6a8ab8` and to the centre's z at `0x6a8ad1`. Both queries share the
fraction slots seeded 1.0 (`0x6a8a73`, `0x6a8a7a`). The second pair runs when `0x6a8aa6` and `0x6a8aae` find
both results zero. `fcomp` with `test ah,5; jp` (`0x6a8b1d`) keeps the map object for a larger, equal or NaN
terrain fraction. `0x6a8b2d je` takes the terrain leg.

`cmapdoodaddef_ground_attach_difftest` runs `0x6a8a54` and `0x6a8ab8` to `0x6a8add`.

## Containment attach

A game object's node (`+0x90` bit 0x2000, set at creation from the object type) takes `0x6a8c10`, which
asks `0x6a8ed0` for the group, then classifies the node from the found record, bakes its light for
the interior class and stores the hit face at `+0xe0`. With no hit the node is of the exterior class. The
nearest hit over every map object and group is the answer: `0x6a8ed0` shares one fraction, starting at
1.0 and never reset, across all its queries, and publishes the object, the record and the point
`centre − 1000·t` on the downward segment.

Addresses: `0x6a8c10`, `0x6a8c3b`, `0x6a8c4d`, `0x6a8c52`, `0x6a8c6f`, `0x6a8c7c`, `0x6a8c8e`, `0x6a8ed0`, `0x6a8edc`, `0x6a8efd`, `0x6a8f1d`, `0x6a90c9`, `0x6a9111`, `0x6a9134`, `0x6a9136`, `0x6a87f0`, `0x8022cc`
Confidence: verified
Proof: one
Test: none

`0x6a8c10` passes the out pointers to `0x6a8ed0` (`0x6a8c3b`), classifies from the found record (`0x6a8c4d`) and
calls the bake slot (`0x6a8c6f`) only when bit 2 of `+0xc` is set; with no hit it sets the exterior bit 4
(`0x6a8c8e`). `0x6a8edc` takes the centre `+0x5c` and `0x6a8efd` subtracts 1000 (`0x8022cc`). The publish at
`0x6a90c9` forms `centre + (end − centre)·t` on the world segment whatever direction the hit ray had, so a hit
of the upward retry also reports a point below the centre. Nothing breaks either loop on the found flag.

## Containment walk

`0x6a8ed0` visits each loaded map object whose root box the node's box overlaps, and in it each group record, in the
order the records were inserted, whose group box overlaps and which is loaded. The box query decides first; if
it misses, the segment query. A group that passes is linked to the node. A group flagged 0x8 publishes
nothing; for the others the face ray runs down the segment and, on a miss, up from the same start to 1000 above
it. The last record to hit publishes, which is the nearest hit, and a tie goes to the later record.

Addresses: `0x6a8f91`, `0x6a8f9c`, `0x6a8fb0`, `0x6a8fc7`, `0x6a8fff`, `0x6a9014`, `0x6a902e`, `0x6a904c`, `0x6a905c`, `0x6a9081`, `0x6a908d`, `0x6a90bd`, `0x6a9138`, `0x6a913b`, `0x6a91b5`, `0x6a91ce`, `0x6a4670`, `0x6a4830`, `0x6a3ee0`, `0x6a3f10`, `0x6a3a40`, `0x6a0450`, `0x6a22f3`, `0x6959d3`, `0xca7da4`, `0xca7d9c`, `0x6bc780`
Confidence: verified
Proof: one
Test: `terrain::floordrop_down_probe_difftest`, `terrain::floordrop_lerp_difftest`, `terrain::floordrop_offset_difftest`

The map object loop starts at `0xca7da4` and steps by `0xca7d9c` (`0x6a91ce`). A def's group records are the list
`{+0x130 link offset 0xc, +0x134, +0x138}` (`0x6a22f3`); the walk starts at `+0x138` (`0x6a8fc7`) and follows
the second word of each link (`0x6a91b5`), which with the head insertion of `0x6959d3` is oldest first. The
box tests are `0x6a4670` and `0x6a4830` (touching counts, NaN rejects). Records that pass the box or segment
query are linked at `0x6a913b`, the exterior ones and those whose rays missed too. The up retry keeps the
start and adds 1000 to the local z for its end (`0x6a9093`). The leaf accepts a hit not farther than the
shared fraction (`0x6bc780`, `test ah,0x41; jp`).

`floordrop_down_probe_difftest` runs `0x6a8f00`, `floordrop_offset_difftest` `0x6a9093` and `floordrop_lerp_difftest` `0x6a90c9` to `0x6a9128`.

## Containment queries

The three tests of the containment attach use different face sets. A face is tested when its flag byte
shares no bit with the mask. The box query (`0x6a3ee0`) and the segment query (`0x6a3f10`) pass the literal 0x86,
which skips faces flagged 2 and 4; the box query is an any-hit test of each face's own box against the node's, the
segment query a nearest-hit ray that writes the shared fraction. The face ray `0x6a3a40`, which every caller
passes the class word 0x20, derives the mask 0x88 from it, so it skips faces flagged 8 and tests those flagged
2 and 4. A group can pass the box or segment test on a face the ray then skips.

Addresses: `0x6a3ee0`, `0x6a3efb`, `0x6a3f10`, `0x6a3f4f`, `0x6a3a40`, `0x6a3a4a`, `0x6a3a5b`, `0x6a3a63`, `0x6a3a6b`, `0x6a3ada`, `0x6a3af1`, `0x6b9530`, `0x6b9548`, `0x6b95e0`, `0x6b92b0`, `0x6bc1c0`, `0x6bca50`, `0x6bca63`, `0x6b8b70`, `0x6bc700`, `0x6bc716`, `0x6a2649`, `0x6a9081`, `0x6a90bd`
Confidence: verified
Proof: one
Test: none

`0x6b9530` ORs 0x80 into the mask for the visited bit it sets on faces (`0x6b9548`) and descends `0x6bc1c0`; its
leaf `0x6bca63` tests the mask and `0x6b8b70` is a sign test of the three vertices against the box on each
axis, so a face counts when its own box overlaps (touching counts). `0x6b95e0` builds the ray context the same
way and descends `0x6bc370` to the leaf `0x6bc700`. The mask builder of `0x6a3a40` starts at 0xee (0xc6 with class
bit 0x10) and clears 0x24 for bit 0x20, 0x02 without bit 0x40 and 0x40 without bit 0x4000 (`0x6a3a4a` to
`0x6a3a70`). The three callers of the face ray are `0x6a2649`, `0x6a9081` and `0x6a90bd`.

## Bake legs

The footprint bake of the interior class (`0x69e4c0` for an entity node) has two legs chosen by its face
pointer argument. Without one, it moves the node position `+0x24` into the object's space with `+0xd4` once and
traces from 1.0 above it to 12.0 below it in that space. With one, it moves the point the caller found into the
object's space and samples that face, tracing nothing. Both attach lanes pass a face pointer, so a bake always
samples the face and point its lane found. The def's twin `0x6a8410` moves both ends of its 1.5 above and 1760 below
trace separately.

Addresses: `0x69e4c0`, `0x69e4c8`, `0x69e4da`, `0x69e4ea`, `0x69e4f0`, `0x69e4f8`, `0x69e515`, `0x69e57d`, `0x69e584`, `0x69e594`, `0x69e5b5`, `0x6a4cc0`, `0x6a4ceb`, `0x6a4cfe`, `0x6a4dc0`, `0x6a4dd8`, `0x6a8410`, `0x6a8bb4`, `0x6a8c6f`, `0x6a845d`, `0x6a8481`, `0x7ff9d8`, `0x80c4e8`, `0x80308c`, `0x80fec8`
Confidence: verified
Proof: one
Test: none

The samplers refuse a group flagged 0x48 (`0x6a4ceb`, `0x6a4dd8`). `0x6a4cc0` traces one group with class word 0 and mask 8
(`0x6a4cfe`), which with the visited bit rejects faces flagged 8 or 0x80, and takes the colour of the nearest
hit; `0x6a4dc0` samples the face given. `0x69e4da jne 0x69e584` takes the second leg for a face pointer. The first leg's single transform is at
`0x69e4f0`, and `fadd 1.0` and `fsub 12.0` (`0x69e4f8`, `0x69e515`) act on the transformed z, so they follow the
object's own up axis. Of the dispatches of the bake slot (offset `0x10`) in the light node band only `0x6a8bb4` and
`0x6a8c6f` exist, and both push a pointer. `0x6a8410` adds 1.5 and subtracts 1760 before its two transforms
(`0x6a845d`, `0x6a8481`).
