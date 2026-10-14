# terrain: map entities

How a unit, game object or doodad on the map gets its place and its light: the flags that say
which map object group it stands in, the light it is given, and the way that light follows the
entity's surroundings as it moves. The scene that culls and draws entities is in `scene`; the
callbacks that hand a model its light are `world#model-lighting-callback`.

An entity keeps its position at `+0x24` (and `+0x5c` for its sphere), its type flags at `+0x8`
(0x20 entity, 0x40 doodad), its attach flags at `+0xc` (a word: 1 gather map object lights, 2
lit as an interior, 4 lit by the sun), its state at `+0x90` (0x1 indoors, 0x10 in shadow, 0x20 in
liquid, 0x40 and 0x80 liquid kind bits, 0x100 fishable liquid, 0x800 blended with the sky), its
liquid height at `+0x98`, its colours `+0x9c` (ambient) and `+0xa0` (directional), its intensity
`+0xa4`, its targets `+0xf4` and `+0xf8`, and its liquid type at `+0xf0`.

Roots: `0x69e280`, `0x69e770`, `0x6a86d0`, `0x6a8c10`, `0x6a8840`, `0x69cdb0`

## Group claim flags

`0x6a87f0` sets an entity's light flags from the map object group it stands in. A group whose
header flags (`+0x10`) have the exterior bit 0x8 sets `+0xc` bit 4. Any other group sets the
indoors bit of `+0x90`, and sets `+0xc` bit 4 when the group has bit 0x40 (exterior lit), else bit 2.

Addresses: `0x6a87f0`, `0x6a8805`, `0x6a880a`, `0x6a880d`, `0x6a8811`, `0x6a8820`, `0x6a8823`, `0x6a882d`, `0x6a8836`, `0x6a4a00`
Confidence: verified
Proof: one
Test: none

`0x6a8805` looks the group record up by its key `+0x7c`; `0x6a880d` tests 0x8, `0x6a8823` 0x40.
`ret 4`; the entity is `ecx`, the map object `edx` and the group node the stack argument.

## Entity placement

`0x6a86d0` re-places an entity: it unlinks the chunk nodes, clears a doodad's `+0xdc`, and places
it by its type, with bit 0x2000 of `+0x90` through `0x6a8c10`, otherwise `0x6a8a20`. `0x6a8c10`
links the entity to the map object group that holds it (`0x6a8ed0`), sets the light flags, bakes an
interior-lit entity's light and stores the face word `+0xe0`. `0x6a8a20` probes down against the
terrain (`0x69c320`) and the map objects (`0x6a8840`), twice at most; a map object hit stands
when the terrain was missed or is not nearer, else the entity attaches to the chunk.

Addresses: `0x6a86d0`, `0x6a86f4`, `0x6a8704`, `0x6a8714`, `0x6a871c`, `0x6a8721`, `0x6a8c10`, `0x6a8c3b`, `0x6a8c4d`, `0x6a8c6f`, `0x6a8c7c`, `0x6a8c8e`, `0x6a8a20`, `0x6a8a81`, `0x6a8a9f`, `0x6a8b18`, `0x6a8b20`, `0x6a8b47`, `0x6a8bb4`, `0x6a8bc7`, `0x6a8bd6`, `0x6a8be1`, `0x6a8bed`, `0x6a8c01`, `0x6a8ed0`, `0x6a8840`, `0x69c320`, `0x6a92d0`, `0x6a91f0`, `0x69b1c0`
Confidence: inferred
Proof: none
Test: none

The arbitration at `0x6a8b18` is `fcomp` of the terrain fraction `[ebp-0x10]` with the object
fraction `[ebp-0x4]`, `test ah,5; jp`: a larger, equal or NaN terrain fraction keeps the map object
(`0x6a8b22` drops it otherwise). A map object hit takes `0x6a92d0`, the light flags, a face trace
and the face word through `0x6a2600` and `0x6a26c0`; `0x6a8bc7` sets bit 9 of `+0xc` (`+0xd` bit 1),
the attached-to-object mark that skips the shadow sample, and the terrain leg (`0x6a8bed`) clears
it. What `0x6a92d0`, `0x6a91f0` and the second probe's start are is not read here.

## Map object trace

`0x6a8840` finds the nearest map object face a segment crosses. For each loaded map object whose
box takes the segment it moves the segment into the object's space, and for each group of the
object whose box takes it traces the group's faces (`0x6b92b0`, mode 4); a hit with a smaller
fraction becomes the answer: the fraction, the map object, the group node and the first batch
word. It returns whether an object was found.

Addresses: `0x6a8840`, `0x6a886a`, `0x6a889b`, `0x6a88c6`, `0x6a88d2`, `0x6a8914`, `0x6a892b`, `0x6a8994`, `0x6a89a3`, `0x6a89aa`, `0x6a89bb`, `0x6a8a12`, `0x6c3330`, `0x6a47e0`, `0x6a4a00`, `0x6b92b0`, `0xc62560`
Confidence: inferred
Proof: none
Test: none

`ret 0x10`. The fraction is the first stack word, preset to 1.0 (`0x6a8864`). The loop is the
global map object list `0xca7da4`; `0x6a889b` is the box test `0x6c3330`, `0x6a88c6` and `0x6a88d2`
move the two ends by the object's `+0xd4`, `0x6a8914` is the group box test, and `0x6a8994` the
face trace. `0x6a89a3` and `0x6a89aa` (`test ah,5; jp`) accept only a strictly smaller fraction.
`0x6a89bb` copies the batch word. The face trace itself is the map object's, in `models`.

## Chunk doodad trace queue

`0x69cdb0` queues a chunk's doodads for the segment trace. A def is queued when its flags word
`+0xc` has bit 0x80 and not 0x100, it has a model and its stamp `+0x8c` differs from the query's
`0xc89f20`; the mode follows its collision data (`+0x180`, `+0x184`) and the query mask: with data
3 for bit 0x100000, 1 for 0x600000; without, 3 for bit 1 and 1, or 2 under 0x1000000, for bits 2
and 4. It then stamps the def.

Addresses: `0x69cdb0`, `0x69cdf0`, `0x69cdf4`, `0x69cdfb`, `0x69ce1b`, `0x69ce2a`, `0x69ce39`, `0x69ce45`, `0x69ce4a`, `0x69ce5d`, `0x69ce6b`, `0x713cb0`, `0xc89f20`
Confidence: verified
Proof: one
Test: none

`0x69cdf0` tests `ah` bit 0 (0x100), `0x69cdf4` the sign of `al` (0x80), `0x69cdfb` the stamp. The
branches on the mask are `0x69ce1b`, `0x69ce2a`, `0x69ce39`, `0x69ce45`, `0x69ce4a`; `0x69ce5d` calls
the model queue `0x713cb0` with the mode and 0. What the mode means to the model is not read here.

## Entity environment

`0x69e280` recomputes what an entity stands in and where its light is going. It clears the
indoors, shadow, liquid and kind bits and the light bits 2 and 4, sets light bit 0, and re-places the
entity. Indoors it takes the group's interior light (`0x69dc30`); outdoors a liquid hit
(`0x69b6d0`) sets bit 0x20 (0x100 for fishable), the height `+0x98`, the type `+0xf0` and bits
0x40 and 0x80 from the (area, type) record. An interior-lit entity's target intensity is 1.0.
Otherwise the target colour `+0xf4` is the global map light's `+0xac`, the target intensity
`+0xf8` 2.5, or 0.5 in the chunk's shadow (bit 0x10); an attached entity takes 2.5 unsampled.

Addresses: `0x69e280`, `0x69e295`, `0x69e2af`, `0x69e2bf`, `0x69e2e1`, `0x69e2f3`, `0x69e307`, `0x69e31b`, `0x69e32d`, `0x69e349`, `0x69e35f`, `0x69e36b`, `0x69e37a`, `0x69e384`, `0x69e47d`, `0x69e483`, `0x69e48d`, `0x69e49b`, `0x69e4a1`, `0x69e4b3`, `0x69b6d0`, `0x6703a0`, `0x7ccf40`, `0x69dc30`, `0x69b350`, `0x6a86d0`, `0xc89ef0`, `0x810a64`, `0x810a68`
Confidence: verified
Proof: one
Test: `terrain::env_update_color_difftest`

The mask `0xfffffe0e` (`0x69e295`) clears bits 0, 4 to 8 of `+0x90`; the word at `+0xc` is masked
with `0xfff9` and given bit 0 (`0x69e2a3` to `0x69e2ab`). `0x69e32d` and `0x69e349` store bits 6 and 7 from the
record's field `+0x8`. `0x69e36b` stores 1.0 for an interior-lit entity. The colour is 0xFF over the bytes of clamp(v, 0, 1) × 255 + 0.5
per channel (`0x69e392` to `0x69e452`) and packed with `0xffffff00` (`0x69e45b`); `0x69e483` tests `+0xd` bit 1, `0x69e48d` the shadow, `0x69e49b`
stores `[0x810a64]` = 0.5 and sets bit 0x10, `0x69e4b3` stores `[0x810a68]` = 2.5.

## Entity light ramp

`0x69e770` moves an entity's light toward its targets each frame. Each byte of the ambient word
`+0x9c` steps toward the target `+0xf4` by max(1, trunc(2.0 × dt × 255)) and stops at it. With
no channel changing and the entity not indoors, the ambient word and its target become the day
light's ambient `+0x178`. The intensity `+0xa4` moves toward `+0xf8` by 3.3333333 × dt and stops at it. If the
entity's attachment list `+0x20` is empty (odd) or unset (zero), `0x69e280` runs after.

Addresses: `0x69e770`, `0x69e776`, `0x69e78b`, `0x69e790`, `0x69e7ee`, `0x69e800`, `0x69e813`, `0x69e825`, `0x69e841`, `0x69e869`, `0x69e86e`, `0x69e880`, `0x69e8a5`, `0x69e8b2`, `0x69e8c8`, `0x69e8e1`, `0x69e8ef`, `0x69e90d`, `0x69e913`, `0x6d48b0`, `0x810804`, `0x810808`
Confidence: verified
Proof: one
Test: `terrain::color_step_magnitude_difftest`, `terrain::intensity_ramp_difftest`

`0x69e776` forms `2.0 × dt × 255`, truncated by `0x40a2b0` and floored at 1 (`0x69e790`). The
channel loop at `0x69e800` runs three times over the byte differences; a positive difference
adds the step and caps at the target (`0x69e813`), a negative one subtracts and floors
(`0x69e825`). The settled test is `0x69e850` (indoors) and the flag in `ebx` (`0x69e865`); the day
ambient is `[state + 0x178]` (`0x69e86e`). The intensity step is `[0x810808] × dt`
(`0x69e880`); `0x69e8a5` holds on equal; a lower value rises and clamps (`0x69e8c8`), a higher or NaN one falls and
clamps (`0x69e8ef`). `0x69e906` loads `+0x20`; it is odd (`0x69e909`) or zero (`0x69e90d`) when the list is empty or unset, and then `0x69e911` calls `0x69e280`, otherwise `0x69e918` returns.

## Entity ambient source

An entity's ambient word `+0x9c` and its target `+0xf4` start and stay at the day light's ambient
outdoors. The creator `0x670db0` packs the three floats at `[0xc89ef0] + 0xac` into both. `0x69e280`
repacks `+0xf4` from them on every re-placement that is not indoors. Bit 0 of `+0x90`, set only by
`0x6a87f0`, stops the per-frame copy of `[state + 0x178]` in `0x69e770`.

| standing in | `+0xc` set | `+0x90` bit 0 | ambient |
|---|---|---|---|
| open terrain (`0x6a91f0`) | 4 | clear | day ambient, copied when settled |
| exterior group | 4 | clear | day ambient, copied when settled |
| exterior-lit group | 4 | set | day ambient target, ramp only |
| other group | 2 | set | group colour at most 0x60 per channel, ramp only |

Addresses: `0x670db0`, `0x670fbe`, `0x670fc4`, `0x69e37a`, `0x69e45b`, `0x69e47d`, `0x69e850`, `0x69e863`, `0x69e874`, `0x69e87a`, `0x6a8820`, `0x6a8b9d`, `0x6a8bb4`, `0x6a8bd6`, `0x6a92ae`, `0x6a92b2`, `0xc89ef0`
Confidence: verified
Proof: one
Test: none

`0x69e37a` loads the floats `+0xac`, `+0xb0`, `+0xb4` and clamps each to 0..255 as in "Entity
environment"; `+0xac` becomes byte 2, so `+0xf4` reads as 0xFFRRGGBB like `+0x178`. `0x670fbe` and
`0x670fc4` store the same dword. `0x69e850` skips the copy when `+0x90` bit 0 is set or a channel moved;
else `0x69e874` and `0x69e87a` store the dword. `0x6a8820` is the only `or` of bit 0; the open
terrain arm (`0x6a8bd6`) never reaches it. The bake slot call `0x6a8bb4` needs `+0xc` bit 2
(`0x6a8b9d`), so only the last row bakes.

## Face terrain type

`0x6a26c0` returns the terrain type of a map object face: for a group key and a face index it
reads the face's material byte from the group's face table and returns the dword at `+0x20` of
that material's 0x40-byte record in the root's material table. It returns −1 when the group is not
found or has no face table.

Addresses: `0x6a26c0`, `0x6a26d2`, `0x6a26e3`, `0x6a26e7`, `0x6a26f0`, `0x6a26fc`, `0x6a2706`, `0x6a270c`, `0x6a270f`, `0x6a4a00`
Confidence: verified
Proof: one
Test: none

`ret 8`. `0x6a26d2` calls the group lookup `0x6a4a00` with the key; `0x6a26e7` loads the group
record's `+0xc4`; the face's word is at `+2 × index` and its high byte is the material
(`0x6a26fc`, `+1`); the record is `[root + 0x1d8] + (material << 6)` and `0x6a270f` reads its `+0x20`.

## Ground id outcomes

`0x6a8a20` ends by storing a ground id at `+0xdc` of the entity, when `[+8] & 0x20` is set. The id
starts at −1. A map object hit gives the face's terrain type (`0x6a26c0`), which is 0 (Dirt) for a
material that carries 0: only −1 is the silent value. It stays −1 when the winning group has no
face the material trace accepts or the group is not loaded. A terrain hit gives the dominant
layer's terrain type, and −1 when `0x69b1c0` fails. `0x6706d0` hands on every id but −1. A unit
copies it to `+0xc60`: `0x5fa730` falls back to −1 with no entity or no id, `0x5fba95` keeps the old
value on failure.

Addresses: `0x6a8b25`, `0x6a8b2a`, `0x6a8b87`, `0x6a8b89`, `0x6a8b9a`, `0x6a8bb7`, `0x6a8bc7`, `0x6a8bea`, `0x6a8bf1`, `0x6a8bfb`, `0x6706d0`, `0x6706dd`, `0x5fa730`, `0x5fa74c`, `0x5fba89`, `0x5fba95`, `0x5fc06e`, `0x5fc20f`, `0x62341d`, `0x623749`
Confidence: verified
Proof: one
Test: none

`0x6a8b25` seeds `[ebp-0xc]` with −1. A map object win runs `0x6a2600`; `0x6a8b89` skips
`0x6a26c0` when it finds no face, and `0x6a26c0` itself returns −1 for a group that `0x6a4a00` does
not give back (`force` 0), then `0x6a8b9a` stores the result with no check. The terrain win stores
−1 at `0x6a8bea` when `0x69b1c0` returns 0. `0x6706dd` compares with −1 only. `0x5fba95` is reached
only when `0x6706d0` returns 1, so there is no fallback store. Four places read `+0xc60`: the
footprint terrain argument (`0x5fc06e`), the spray (`0x5fc20f`: negative or above the id bound
`0xc0d688` ends it; the row's `+0xc` when `0x5fc350` returns nonzero, else `+0x8`), the footstep
sound (`0x62341d`) and the death thud (`0x623749`, which indexes with the row's `+0x10`).

## Face masks

A face is a candidate for a trace when its flag byte has none of the mask bits set. The nearest-object
probe `0x6a8840` passes mask 4 to `0x6b92b0`, which ORs in 0x80 and so rejects faces with 0x84.
The material trace `0x6a2600` passes 0x20 to `0x6a3a40`, which builds mask 0x88. The bit 0x80 is a
visited mark the leaf writes into the loaded face bytes and the query clears when it ends.

Addresses: `0x6a896a`, `0x6a8994`, `0x6b930a`, `0x6b9313`, `0x6b9319`, `0x6bc710`, `0x6bc713`, `0x6bc716`, `0x6bc719`, `0x6bc73d`, `0x6bc740`, `0x6b9284`, `0x6b928a`, `0x6a2638`, `0x6a2649`, `0x6a3a4f`, `0x6a3a60`, `0x6a3a68`, `0x6a3a70`, `0x6a3ada`, `0x6a3a40`, `0x6a2600`
Confidence: verified
Proof: one
Test: none

`0x6a3a4f` starts at 0xee (0xc6 when the flag has 0x10); a flag with 0x20 clears 0x24
(`0x6a3a60`), a flag without bit 6 clears 0x02 and one without bit 14 clears 0x40, which leaves 0x88
for flag 0x20. `0x6a3a40` hands its edi to `0x6b92b0` as the last argument (`0x6a3ada`).
In `0x6bc700` a face with any mask bit set jumps to the reject exit (`0x6bc719`); an accepted one is
logged and has 0x80 ORed in (`0x6bc73d`), and `0x6b9284` clears the bit on each logged face. Read over the
seven groups of `Snow_Inn.wmo` (23 260 faces): the mask 0x84 accepts 7 605 faces, 2 899 of which
have material 255 and bit 0x08, past the 20-record material array; the mask 0x88 accepts 20 361 and
none of them has material 255. The ground read therefore never reaches the missing bound check of
`0x6a26c0` in that building, and the 20 materials of the root all carry terrain type 10 (None).

## Map object box test

`0x6ad330` tests a box against the loaded map objects. For each map object not marked skipped
(`+0xd` bit 0) it moves the box into the object's space with the object's inverse axes and
origin, and when the object's bounds overlap it (`0x6a4670`) collects the facets the box
touches (`0x6a3b10`) into the triangle batch, keeping whether any object did. It returns that.

Addresses: `0x6ad330`, `0x6ad3f1`, `0x6ad419`, `0x6ad444`, `0x6ad466`, `0x6ad48c`, `0x6ad4a4`, `0x6a4670`, `0x6a3b10`, `0x6dc3f0`, `0x5f8d20`
Confidence: inferred
Proof: none
Test: none

`ret 4`. The box and its flags come in through `ecx`, `edx` and the stack word (the facet mask,
passed on to `0x6a3b10` at `0x6ad4a4`). `0x6ad3f1` skips an object with `+0xd` bit 0; `0x6ad419`
moves the centre by the object's matrix, `0x6ad444` copies its nine axis floats, `0x6ad466`
transforms the box (`0x6dc3f0`). How `0x6a3b10` selects facets is the map object's, in `models`.

## Entity attachment list

An entity node keeps what it is attached to in a list at `+0x18` (link offset 0x14), `+0x1c` and `+0x20`.
Empty is the odd pointer `(&node+0x1c) | 1`, never 0; a walk starts at the first record inserted and
follows `*(record + 0x18)`. `0x6a86d0` drains the list and refills it in one call. An entity without
`+0x90` bit 0x2000 takes `0x6a8a20`: a map object hit links one record to the hit's group and then
classifies the entity from that group (`0x6a87f0`), a terrain hit links the chunk, a miss links
nothing. An entity with the bit takes `0x6a8c10`, a record for each group it touches. `0x69e770`
re-places an entity only when its list is empty.

Addresses: `0x6c3020`, `0x6c3045`, `0x6c3047`, `0x6c3051`, `0x6a0450`, `0x6a04f2`, `0x6a04f9`, `0x6a86d0`, `0x6a86f4`, `0x6a8714`, `0x6a8a20`, `0x6a8b2d`, `0x6a8b3a`, `0x6a8b47`, `0x6a8bcd`, `0x6a8bd6`, `0x6a92d0`, `0x6a91f0`, `0x6a87f0`, `0x6a8c10`, `0x69e770`, `0x69e906`, `0x69e90f`, `0x69e913`, `0x69e280`, `0x69e2a3`, `0x69e2af`, `0x6717d0`, `0x671822`, `0x671a6a`, `0x671a73`
Confidence: verified
Proof: one
Test: none

`0x6c3020`, the constructor every node class calls, stores the link offset 0x14 (`0x6c3047`), the head
`+0x1c` pointing at itself (`0x6c3045`) and the tagged head in `+0x20` (`0x6c3051`). `0x6a0450`
inserts after the head's last record (`0x6a04f2`, `0x6a04f9`), so `+0x20` becomes the first record. `0x6a92d0` and
`0x6a91f0` each have one caller (`0x6a8b3a`, `0x6a8bd6`) and call `0x6a0450` with the node as owner.
`0x69e280` clears bits 2 and 4 of `+0xc` (`0x69e2a3`) and calls `0x6a86d0` (`0x69e2af`); `0x6a8b47`
classifies from the object just linked, so the flag and the record come from one hit. `0x69e90f jne`
skips `0x69e280` for a linked node. `0x6717d0` stores the position and calls `0x69e280` (`0x671a73`) only when
the squared step from the stored position is at least 0.001 (`0x671822`, `test ah,5; jp`, a NaN counts) or its
force argument is set (`0x671a6a`).

## Entity fog lane

`0x6c31e0` takes the interior fog for an entity node flagged `+0xc` bit 2 by walking its list to the
first record whose target has bit 0x10 in its `+0x8` word, and then reading that target's `+0x98`,
the stamp of the group fog flag. A zero stamp, no such record or an empty list gives the scene fog, and
the walk does not go on to a later record. Bit 0x10 is the map object group record's class tag; its
constructor `0x6a1ca0` sets it and no other code sets it in a dword at `+0x8`. `0x69d4f0` walks the
same list with the same test and does go on.

Addresses: `0x6c31e0`, `0x6c31ee`, `0x6c31f8`, `0x6c3206`, `0x6c3212`, `0x6c3216`, `0x6c3222`, `0x6c3229`, `0x6c3231`, `0x6c327a`, `0x6c3233`, `0x6a1ca0`, `0x6a1d7b`, `0x69d4f0`, `0x69d529`, `0x69d52c`, `0x69d567`, `0x69d56c`, `0x695960`, `0x695986`, `0x695a5c`, `0x6a0ca0`
Confidence: verified
Proof: one
Test: none

`0x6c3216 jne 0x6c3222` leaves the loop for good: no jump returns from `0x6c3222` to `0x6c3206`, so
`0x6c3231 je 0x6c327a` ends the walk. `0x69d567` instead falls into the advance `0x69d56c` on a miss.
The scan for `or dword [reg+8],0x10` finds `0x6a1d7b` alone (the one other hit, `0x5109ce`, is a stack
word). The target record is the per-(instance, group) one `0x685d70` stamps (`terrain/portals#group-submit`):
`0x695960` makes one per group (`0x695986` to the constructor through `0x6a0ca0`, the group index stored at
`0x695a5c`), and `0x685d70` finds it by that index.

## Light node classes

Two classes share the map light base. The entity node (vtable `0x810810`, `0x69d590`, 0x108 bytes) is
the light of units, players, game objects, missiles and effects; the doodad def (`0x810a74`, `0x6a7d00`,
0x198 bytes) is the light of placed doodads. They differ in slots 2 (`0x6a7300`, `0x6a8050`) and 4
(`0x69e4c0`, `0x6a8410`), and on a def `+0xf8` is `m[2][3]` of the matrix at `+0xcc`, not a target. An entity node's sun
scale `+0xa4` ramps at 3.3333 per second toward its target `+0xf8`: 2.5 lit, 0.5 shadowed, 1.0 interior. A def's
is assigned: 1.0 at creation, 0.5 once if the pending drain finds it in shadow, never ramped.

Addresses: `0x810810`, `0x810824`, `0x810a74`, `0x69d590`, `0x69d6b1`, `0x6a7d00`, `0x6a7eb0`, `0x69e4ad`, `0x69e496`, `0x69e4b3`, `0x698cae`, `0x69e770`, `0x69e880`, `0x671a90`, `0x60a97f`, `0x5f8c9a`, `0x6717d0`, `0x671a73`, `0x482cd0`, `0x6b01bb`, `0x695bb3`, `0x698c9c`, `0x810a64`, `0x810a68`, `0x6a7e0e`
Confidence: verified
Proof: one
Test: `terrain::env_update_color_difftest`, `terrain::color_step_magnitude_difftest`, `terrain::intensity_ramp_difftest`

The constructors store the base vtable `0x810824` and then their own (`0x69d6b1`, `0x6a7eb0`). The constant
2.5 has one reader in the image, `0x69e4ad`; 0.5 has two, `0x69e496` and `0x698cae`. `0x6a7e0e` onward set the
def's matrix diagonal at `+0xcc`, `+0xe0`, `+0xf4` and `+0x108`, so `+0xf8` lies in its third row. `0x69e280`
has two entries, `0x671a73` and `0x69e913`; `0x69e770` has one, the thunk `0x671a90`, called at `0x60a97f` and
`0x5f8c9a` by the animators `0x482cd0` dispatches by object type. `0x6717d0` has six callers, each loading the
light pointer of a game object, unit, missile, effect or the zone record. A def's own gate is `0x698c9c` (interior bit).
