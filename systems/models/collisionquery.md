# models: WMO movement query

What the mover's box query asks of a map object (a WMO) and its groups: which groups it visits,
which faces it keeps, and what it emits. The query enters from the world trace
`terrain/collide#world-triangle-query`; the segment queries of the camera and the ray pick use the
same groups through `models/wmo#bsp-segment-descent` and filter differently.

Roots: none

## Group enumeration

`0x6a3b10(out, box, placement, class)` walks a map object's groups and keeps those that pass two
gates. A group's bounds in the group info array (`+0x130`, stride 0x20) must overlap the query box
(a closed overlap, so a box inside a group's bounds always passes, and equal edges pass), and the
group must be loaded: `0x6a4a00` is called with unloaded groups disallowed and returns 0 for a
group whose file flag `+0x164` is clear, which is skipped without error. No flag of the group's
header or of its info entry is read. A kept group goes to `0x6b91c0`; the result is the OR of its answers.

Addresses: `0x6a3b10`, `0x6a3b4f`, `0x6a3b55`, `0x6a3b70`, `0x6a3b7b`, `0x6a3b97`, `0x6a3bbc`, `0x6a3bbf`, `0x6a3bdb`, `0x6a4a00`, `0x6a4a03`, `0x6a4a17`, `0x6a4a21`, `0x6b91c0`
Confidence: verified
Proof: one
Test: none

The six compares, from `0x6a3b70` on, load a bound of the group and `fcomp` a box float; the first
three use `test ah,0x41; jp` to skip a min above the box max, the last three `test ah,1; jne` to skip a
max below the box min. `0x6a3bbc push 0; push ebx; call 0x6a4a00` passes the group index and the
disallow flag. `0x6a4a00` returns 0 when `[root+0x1d4]` is clear, else loads the group pointer at
`+0x1f4 + 4·i` and returns it when `[group+0x164]` is set, or when its second argument is set. The
only writer of the flag's set is `0x6c40a4`, in the group setup; the load request is made from the
render side (`0x69885d`, `0x6a4a30`, `0x6c3e20`).

## Placement gates

A placement is skipped or the whole trace fails before any group is read. Per placement
`0x6aaab0` skips it when its disabled bit is set (`0x6aab61 test [def+0xd],1`), when it has no
root, and when its world box or the transformed query box misses (`0x6acb40`, `0x699330`,
`0x6a4670`). It fails the whole trace, returning 0 so that terrain too is not gathered for that substep,
when the root's flag `+0x1d4` is clear or the placement's list head `+0x138` is tagged or null.
The transform of the query box into the placement's frame only grows it.

Addresses: `0x6aaab0`, `0x6aab61`, `0x6aab6b`, `0x6aab87`, `0x6aab9a`, `0x6aabaa`, `0x6aabb2`, `0x6aabb8`, `0x6aabbe`, `0x6aabc0`, `0x6aabc6`, `0x6aabc8`, `0x6aada7`, `0x6aa8b0`, `0x6aa911`, `0x6aa913`, `0x6aaa97`, `0x6721b0`, `0x6a4670`, `0x6acb40`, `0x699330`
Confidence: inferred
Proof: none
Test: none

The branch layout is read; the meaning of `+0x138` as the placement's readiness is not
derived. `0x6aada7` is `xor al,al; ret 8`. `0x6aa8b0` tests the result (`0x6aa911 test al,al; je
0x6aaa97`) before its terrain loop runs.

## Face mask

A face is rejected by a mask that is a function of the 32-bit query class alone. Every
map-object query entry builds it the same way: `0xee`; `0xc6` when class bit `0x10`; clear `0x24`
when `0x20`; clear `0x02` unless `0x40`; clear `0x40` unless `0x4000`. The mover's class
(0x100111, 0x102111) gives `0x84`: only the transient visited bit `0x80` and detail faces
(flag `0x04`) are rejected. The camera and line of sight class 0x100171 gives `0x82`: the visited bit and
faces with flag `0x02`, which are the no-camera-collision faces; it keeps detail faces. A face is
skipped when its flag byte has any bit of the mask.

Addresses: `0x6a3b16`, `0x6a3b1b`, `0x6a3b23`, `0x6a3b2c`, `0x6a3b33`, `0x6a3b3a`, `0x6a3b3e`, `0x6a3b42`, `0x6a3b46`, `0x6a3b4b`, `0x6a3839`, `0x6a3a4f`, `0x6a3d08`, `0x6a3dd5`, `0x6b9208`, `0x6bca63`, `0x6bc716`, `0x50e61a`, `0x6315f0`, `0x6a37b0`, `0x6a3a40`, `0x6a3cf0`, `0x6a3dc0`
Confidence: verified
Proof: one
Test: none

`0x6a3b3a and [ebp-8],0xffffffdb` clears `0x24`, `0x6a3b42 and ...,0xfffffffd` clears `0x02`, `0x6a3b4b
and ...,0xffffffbf` clears `0x40`. `0x6b9208 or edx,0x80` makes the mask a word and stores it in the
leaf context at `+0x14`; the leaf tests `test [ecx+0x14],dl` (`0x6bca63`) against the face's flag byte, the
segment leaf `0x6bc716` against `[ctx+0x50]`. The same constants appear in `0x6a37b0` (the segment
intersect, `0x6a3839`), `0x6a3a40`, `0x6a3cf0` and `0x6a3dc0`. The camera's class is built at `0x50e61a`
(`add esi,0x100171`); `0x69c0f0` passes the unmasked class to `0x6a37b0`, so bits `0x10`, `0x20` and `0x40` reach the builder.
The face flag names (detail, no camera collision) are from the file format and not re-derived.

## Box descent and face test

The box descent tests a face only by its triangle's bounding box. `0x6b91c0` descends the group's
tree with `0x6bc1c0`, whose leaf `0x6bca50` skips each face by the mask and the visited bit and calls
`0x6b8b70` on the face's three vertices and the whole query box. That kernel reports separated when,
on any coordinate axis, all three vertices lie beyond the box on one side, by the sign bit of the
difference, so a touching triangle is kept. It has no plane, edge, winding or epsilon test. A kept index is stored, `0x6b8e50` packs the survivors and
`0x671cc0` emits one 52-byte facet per face (normal from the cross product, `d = −n·v0`, three world
vertices) with no orientation test.

Addresses: `0x6b91c0`, `0x6b9224`, `0x6b923b`, `0x6bc1c0`, `0x6bc1e8`, `0x6bc21d`, `0x6bca50`, `0x6bca63`, `0x6bca6c`, `0x6bca72`, `0x6bca88`, `0x6bca9a`, `0x6bcad1`, `0x6bcadc`, `0x6b8b70`, `0x6b8bae`, `0x6b8e50`, `0x671cc0`, `0x6aaca3`, `0x6b8c60`, `0xce26e8`, `0xce26e0`, `0xcde648`, `0xce66fc`
Confidence: verified
Proof: one
Test: none

Each of `0x6b91c0`, `0x6bca50` and `0x6b8b70` has one call site (`0x6a3bdb`, `0x6bc21d`, `0x6bcad1`). The
visited stack `0xce26e8` holds 0x2000 entries: a full stack sets bit 0 of the context's status word
(`0x6bca80`) and the leaf stops. `0x6bca9a or [mopy],0x80` sets the visited bit, undone after the query
(`0x6b9267` to `0x6b9295`). `0x6b8b70` masks the difference signs with `0x80000000` per vertex and
returns 1 on all-above or all-below. `0x6bc1e8` first calls `0x6b8c60`, which returns 0 when the cache
pointer `0xca03e4` is null, which it is unless the console enabled the cache.

## Liquid tiles by class bit

A map object group's liquid tiles join a query only when the class word asks for their type. `0x6b91c0` passes a word with any of 0xf0000 on to `0x6bb170`, which builds a four-entry
table of 0x10000, 0x20000, 0x40000 and 0x80000 and keeps a tile when the table entry its type picks is in the word. The type is the low nibble of the tile's flag byte, 0xf meaning no
liquid, masked with 3 to index the table. So the four liquid types are class bits 16 to 19, and water walking asks for 0x30000, the first two.

Addresses: `0x6b91c0`, `0x6b9243`, `0x6bb170`, `0x6bb455`, `0x6bb4a7`, `0x6bb4aa`, `0x6bb4b3`, `0x6bb4b6`, `0x6bb4b9`, `0x6bb4bd`, `0x6315f0`
Confidence: verified
Proof: one
Test: none

`0x6b9243 test eax,0xf0000` gates the call. `0x6bb455` to `0x6bb46a` store the four constants at `[ebp-0x58]` to `[ebp-0x4c]`. `0x6bb4a7 and eax,0xf` and `0x6bb4aa cmp eax,0xf; je` skip an empty tile,
`0x6bb4b6 and eax,3` takes the index, and `0x6bb4b9 test [ebp+eax*4-0x58],edx` with `edx` the class word (`0x6bb4b3 mov edx,[ebp+0x14]`) keeps or skips the tile (`je 0x6bb62f`).

## Group flags on the movement path

No flag of a group header is read between the world trace and the face test. The two queries that do
gate a group by its flags are the segment intersect `0x6a37b0`, which rejects a group when
`(flags & mask) != 0` with a mask its caller passes, and the containment attach `0x6a8ed0`, which
skips a group with flag `0x8`. The movement box query has no such filter: a group whose bounds contain
the query box is visited whatever its header says.

Addresses: `0x6a37b0`, `0x6a388d`, `0x6a389f`, `0x6a38a2`, `0x6a8ed0`, `0x6a9014`, `0x6a905c`, `0x6a9060`, `0x6a3b10`, `0x6a4a00`, `0x6b91c0`, `0x6b8e50`
Confidence: verified
Proof: one
Test: none

Of the offsets read off the group pointer on the chain, `0x6a3b10` reads none, `0x6a4a00` reads
`+0x164`, `0x6b91c0` reads `+0xc4`, `+0xc8`, `+0xd0`, `+0x44` and `+0x90`, and `0x6b8e50` `+0xc8`,
`+0xd0` and `+0xd4`. `0x6a389f test [esi+0x10],eax` takes the caller's mask from `[ebp+0x18]`;
`0x6a905c test [eax+0x10],8` is the attach query's fixed test and shows the scan can see such a gate.
The two-sided segment-triangle test `0x7c29f0` has three callers (`0x6bc764`, `0x6b8acf`, `0x709fc5`),
none on this path, and `0x7c2e90` has none.
