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
entity is attached to a node, `0x69e280` runs after.

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
clamps (`0x69e8ef`). `0x69e90d` tests the node link and calls `0x69e280`.

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
