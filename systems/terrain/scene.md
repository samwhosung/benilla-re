# terrain: scene

How the client picks what the map shows each frame: the scene driver and its walk over the depth
bands, the cull of entities and doodads, the volumes they test against, and the drains that draw
them. Terrain chunks, the horizon and the depth buckets are in `cull`; the liquid passes are in
`liquid`; what the camera sits in is in `camstate`.

An entity is a placed object the client moves (units, game objects, corpses): its model at `+0x88`,
flags at `+0x90`, cull state at `+0x94`, sphere centre at `+0x5c`, radius `+0x68` and view depth `+0x78`,
linked into the scene by the node at `+0xfc`. A doodad def (CMapDoodadDef) is a placed doodad: the
same centre, radius and depth, linked by the node at `+0x16c`. The cull state is set to 1 for every
scene node each frame and bit 0 is cleared when the node is drawn; `0x6704c0` reads it.

Roots: `0x681070`, `0x682fa0`, `0x6809c0`, `0x6821f0`, `0x682ef0`, `0x682f60`, `0x683dd0`, `0x683f80`

## Scene driver

`0x681070` draws the world. It zeroes the counters, sets the horizon far depth to the far clip less
33.333332 and the fov term `fog/cos(fov/2) − fog`. Outdoors it walks the scene once with the
rectangle (0, 0, 1, 1). Inside a map object it copies the object's deferred portal windows to its
region array and walks once per window, with the horizon feed the window's farthest plane
distance plus 33.333332; with no window it walks nothing. It then drains the band lists and
draws the passes in the order of `Frame order`.

Addresses: `0x681070`, `0x6810d8`, `0x6810f5`, `0x681101`, `0x681130`, `0x68118b`, `0x681194`, `0x681199`, `0x6811a9`, `0x6811b6`, `0x6811ca`, `0x6811e3`, `0x6811ff`, `0x681204`, `0x681209`, `0x685fe0`, `0x685a90`, `0x682fa0`, `0x680390`
Confidence: verified
Proof: one
Test: `terrain::render_near_bias_difftest`, `terrain::render_fog_dist_difftest`, `terrain::render_leg_fog_offset_difftest`, `terrain::render_pos_accumulate_difftest`

`0x681101` branches on the camera's map object group claim. Set: `0x681130` runs the interior cull
`0x685a90`; `rep movsd` copies the windows (5 dwords: low y, low x, high y, high x, the plane distance) from
`0xcbe324` to the region array (`0x68118b`) and stores the count (`0x681194`); `0x681199 jbe
0x681204` skips the walk at 0. Each window sets `0xc7b66c` to its fifth dword plus 33.333332
(`0x6811a9`), resets the horizon and walks (`0x6811b6`). Clear: `0x6811ca` stores 0 in the feed
and the rectangle at `0x6811e3` to `0x6811ff`. `0x681204` runs the drain `0x680390` in both.

## Frame order

The frame `0x483460` runs the terrain parts in this order: the fog states `0x66ff20`, the scene
`0x6701b0` (`0x681070`), the fading models `0x672ef0`, the footprints `0x670240`, the model scene's
advance and collect, then the world stage `0x6701c0` (`0x6813d0`). With the camera liquid state 0xf
(dry) it draws model list 2, the liquids `0x6701d0`, the weather `0x670210`, then list 1; otherwise
list 1, the weather, the liquids, list 2. The glare, the lightning, the particulates `0x6701e0`
and the name plates follow. The scene `0x681070` draws terrain `0x684510`, map objects `0x684840`,
liquid type 2 `0x6855a0`, doodad defs `0x683f80`, entities `0x683dd0` and the far band `0x6841a0`,
and its sky gate draws the dome `0x6d4940` only while the camera is dry.

Addresses: `0x483460`, `0x483601`, `0x48361d`, `0x483622`, `0x483654`, `0x4836ca`, `0x4836d1`, `0x4836d6`, `0x4836e4`, `0x4836e9`, `0x4836f9`, `0x4836fe`, `0x66ff20`, `0x6701b0`, `0x6701c0`, `0x6701d0`, `0x6701e0`, `0x670210`, `0x670240`, `0x672ef0`, `0x672470`, `0x6812b1`, `0x6812b6`, `0x6812bb`, `0x6812c0`, `0x6812c5`, `0x6812ca`, `0x6812a4`, `0x6812ac`, `0x6d4940`, `0x684510`, `0x684840`, `0x6855a0`, `0x6841a0`, `0x6813d0`, `0x708900`
Confidence: verified
Proof: one
Test: none

`0x4836d6` is `cmp eax,0xf` on `0x672470`'s result. Dry (`0x4836dd`): `push 2`, `0x708900`, `0x6701d0`
(`0x4836e4`), `0x670210`, `push 1`, `0x708900`; the other leg (`0x4836f2`) pushes 1, then the
weather, then `0x6701d0` (`0x4836fe`), then 2. The six calls at `0x6812b1` to `0x6812ca` run
unconditionally in the order above. `0x6812a4` compares `0xc7f288` with 0xf before the dome call.
`0x6813d0` draws the detail doodads and has no caller but its thunk.

## Frame clear colour

Before the passes the scene clears the device (mask 3) with a colour chosen from the fog: with the weather-visible byte
`0xca80c4` clear (indoors, no exterior in view) the second fog colour (`+0x80`); with it set,
black when the camera is dry and the first fog colour (`+0x70`) when it is under liquid. The previous clear colour
is put back after. The sky and the clouds are drawn only with the byte set and the camera dry.

Addresses: `0x681231`, `0x68123d`, `0x68123f`, `0x68124d`, `0x681254`, `0x68125e`, `0x681263`, `0x681268`, `0x681275`, `0x68127a`, `0x68128c`, `0x6812a4`, `0x6812ac`, `0x58a920`, `0x58a970`, `0x6d48b0`, `0xca80c4`
Confidence: verified
Proof: one
Test: none

`0x681231` compares `0xca80c4` with 0; equal, `0x681254` loads the state's `+0x80`. Otherwise `0x68123f`
compares the camera liquid state with 0xf: equal stores 0 (`0x681249`), else `0x68124d` loads `+0x70`.
`0x68125e` sets the colour, `0x681263` clears with the mask 3 and `0x681275` restores the colour. `0x68127a`
skips the sky when the byte is 0 (`0x6812b1`); `0x6812a4` and `0x6812ac` run the dome call for a dry camera.

## Scene walk

`0x682fa0` walks one rectangle. It returns for a rectangle that is not wider and taller than 0.01,
pushes and builds the frustum volume for the rectangle, and visits the 32 depth bands front to
back. Per band: entities, doodad defs, map objects, liquid layers 1, 0 and 2, then terrain chunks.
After the bands it pops the volume and tail-jumps to the low-detail tile walk.

Addresses: `0x682fa0`, `0x682fa8`, `0x682fbb`, `0x682fca`, `0x682fd1`, `0x682fe2`, `0x682fe9`, `0x682ff2`, `0x682ffe`, `0x683007`, `0x683013`, `0x68301a`, `0x68302f`, `0x682380`, `0x682490`, `0x682f90`, `0x683040`, `0x683340`, `0x683700`, `0x6856c0`, `0x683ab0`, `0x683bf0`
Confidence: verified
Proof: one
Test: `terrain::viewport_gate_difftest`

`0x682fa8` and `0x682fbb` compare the width and height with 0.01 (`test ah,5; jnp`: a smaller
value returns, NaN walks). The loop runs 0x20 times over the bands at `0xc7bd40`, 0x6c bytes
apart. The calls are `0x683340`, `0x683700`, `0x6856c0` with the rectangle, `0x683ab0` with edx 1,
0, 2, then `0x683bf0`. A band's index is the depth `d` of `cull#depth-buckets` and the walk
visits 0 first, so what is nearer is linked before what is farther.

## Entity cull

`0x683340` culls a band's entities and `0x6834e0` those in a map object group (`ecx` the group,
`edx` its record). Both skip an entity whose `+0x94` is 0 and store the view depth of its sphere,
camera plane · centre + d − radius, at `+0x78`. An entity inside the frustum (`0x682ef0` is 0) and
not hidden by the horizon (`0x686000` below 2) clears bit 0 of `+0x94` and moves to the entity
draw list. Otherwise, with a model, it is set animated when its centre is within squared distance
10.0 of the camera and not animated beyond; the group variant also moves the entity to the
second list.

Addresses: `0x683340`, `0x6834e0`, `0xc7cb08`, `0x68337c`, `0x683388`, `0x6833b8`, `0x6833bb`, `0x6833d0`, `0x6833dd`, `0x683461`, `0x683466`, `0x683470`, `0x6834ab`, `0x68352d`, `0x68353c`, `0x68357d`, `0x683592`, `0x683616`, `0x683648`, `0x6836b4`, `0x6836d6`, `0x682ef0`, `0x686000`, `0x710b90`, `0x4549f0`
Confidence: verified
Proof: one
Test: `terrain::camplane_sort_key_difftest`, `terrain::near_audibility_jp_difftest`, `terrain::scenewalk_camplane_sort_key_difftest`, `terrain::scenewalk_near_audibility_diffs_difftest`

`0x683388` multiplies the plane's second and third terms and the first by the centre and subtracts the
radius (`0x6833b8`) into `+0x78`. `0x6833bb` and `0x6833d0` are the two tests; `0x683461` links
the draw list at `0xc7cb0c`. The animate test at `0x683470` compares the squared distance with
`0x810178` (10.0); `0x6834ab` is `test ah,5; jp`, so only a smaller value passes 1 to `0x710b90`.
`0x68352d` skips an indoor entity when the group record's `+0x10` has bit 8; `0x683648` links the
culled one to the second list `0xc7cb44`.

## Doodad def cull

`0x683700` culls a band's doodad defs and `0x6838f0` those of a group. A def not seen this frame
(its `+0x164` differs from the frame counter) gets the stamp and `+0x94` of 1. A def with neither a
model nor a transform callback (`+0x88`, `+0x174`) is skipped. The others get the depth stored
and the same two tests; a def that passes clears bit 0 of `+0x94`, moves to the doodad def draw
list `0xc7cae8` and increments the visible counter `0xc7b600`. A def that fails with a model is
set animated when its `+0x90` bit 9 is set or its centre is within squared distance 10.0
(`0x81017c`) of the camera.

Addresses: `0x683700`, `0x6838f0`, `0xc7cae4`, `0x683747`, `0x68374b`, `0x683751`, `0x68375b`, `0x683767`, `0x68376f`, `0x6837ae`, `0x6837d5`, `0x683862`, `0x6838ab`, `0x68392f`, `0x683942`, `0x683972`, `0x6839a5`, `0x6839cc`, `0x683a1b`, `0x683a43`, `0x683a72`, `0x683a8f`, `0x710b90`
Confidence: verified
Proof: one
Test: `terrain::scenewalk_camplane_sort_key_difftest`, `terrain::scenewalk_near_audibility_jnp_difftest`, `terrain::camplane_sort_key_difftest`, `terrain::near_audibility_jnp_difftest`

`0x683747` through `0x683751` compare `+0x164` with the frame counter `0xc7b2c8`. The group variant
tests the byte at `+0xc` for its sign (`0x683929`) and passes edx = 0x10 to the horizon test
(`0x6839bf`); `0x683a43` tests `+0x90` bit 9 (`test ah,2`, set animates) and `0x683a83` is
`jnp`, passing 1 for a smaller distance. A NaN distance leaves the model not animated.

## Sphere and box tests

`0x682ef0` returns 0 when a sphere (centre `ecx`, radius on the stack) lies inside the six
planes of the active frustum volume and 1 when it does not. `0x682f60` does the same for an
oriented box through `0x6869c0`; it takes the box (ecx), its 3 by 3 basis (edx) and a position (stack, `ret 4`). `0x686b80` is the plane test: 0 when the centre is behind a
plane by more than the radius, else 3. `0x686000` returns 2 when the sphere is hidden behind the
horizon and 0 otherwise.

Addresses: `0x682ef0`, `0x682f23`, `0x682f28`, `0x682f60`, `0x682f7b`, `0x6869c0`, `0x686b80`, `0x686bab`, `0x686bb0`, `0x686000`, `0x68600d`, `0x68601b`, `0x686032`, `0x68604f`, `0x686064`, `0x686086`, `0x6860a4`, `0x6860b5`, `0x6860e9`, `0x686107`, `0x686121`, `0x686124`, `0x686150`, `0x686166`, `0xc7cfe0`, `0xc7d308`
Confidence: verified
Proof: one
Test: `terrain::sphere_frustum`, `terrain::horizon_span_test`, `terrain::horizon_non_finite_probes`

The volume is `[0xc7cfe0] × 0xfc + 0xc7d308`; `neg eax; sbb eax,eax; inc eax` turns the test's 3
into 0 and its 0 into 1 (`0x682f28`, `0x682f7b`). `0x686b80` negates the radius and compares
`n·c + d` of each plane with it (`fcomp`, `test ah,5; jnp` at `0x686bb0`: a smaller value rejects,
NaN passes). `0x686000` needs world flag 0x20 and a radius of at least 2.4e-7 (`0x8029d4`) and the
camera forward z in [−0.9, 0.9] (`0x686032`, `0x68604f`); the centre goes through `0xc7b700`
(`0x686064`); without flags bit 3 a centre nearer than z = 50 is visible (`0x6860a4`). The
vertical top is y/z plus the transformed radius's first term over z, where (r, r, 0) goes
through the projection matrix at `0xc7d280` (`0x686086`). The columns run from
round(64·(x/z − rx/z) − 0.5) + 0xa0 to round(64·(x/z + rx/z) − 0.5) + 0xa1, clamped to 0 to 0x13f (`0x6860e9` to
`0x686124`), and the sphere is hidden when each of them has a horizon value of at least the top (`0x686150`
`jnp` leaves at the first lower one, `0x686166` returns 2).

## Draw lists

Each pass bucket of the frame is a list of records with a link node at a fixed offset inside
each record. The header is three words: the node offset, the tail and the head. An empty list holds
the address of its second word as the tail and that address with bit 0 set as the head; bit 0 of a
head or next pointer marks the end. The head and the next links hold record bases, the tail and the
previous links hold node addresses. A walk appends at the tail and drains from the head, so
records come out in the order they were added.

Addresses: `0x6878e0`, `0x6878e5`, `0x6878f4`, `0x683445`, `0x68344a`, `0x683455`, `0x68345b`, `0x684840`, `0x6849a3`, `0x6849a8`, `0x687760`, `0x687890`
Confidence: verified
Proof: one
Test: none

`0x6878e0(header)` stores the tail `header + 4` and the head `(header + 4) | 1` and clears the
offset; the offset is stored by the list's own init. The append at `0x683445` to `0x68345b` sets the
new node's previous link to the old tail, its next link to the old tail's next, the old tail's next to
the record and the tail to the new node. A drain such as `0x684840` loads the head, tests bit 0 and
zero, reads the next record as the word at record + offset + 4 (`0x6849a8`) and unlinks the record
as it goes. `0x687760` and `0x687890` return the node of the next record: its base plus the given
offset, or for a negative offset the node's own offset, recovered from the previous node
(`0x68777c` to `0x687781`); at the end they return the end marker without bit 0.

## Band list drain

`0x680390` empties the 32 bands after the last window: it unlinks every chunk, liquid, doodad
def and map object node from its band list, and moves each entity still in a band to the second
entity list at `0xc7cb44`.

Addresses: `0x680390`, `0x6803e6`, `0x680440`, `0x6804a6`, `0x68052b`, `0x6805b3`, `0x6805ce`
Confidence: verified
Proof: one
Test: none

The bands are 0x6c bytes apart from `ecx + 0x14`, 0x20 of them (`0x6803a2`). The lists with nodes
at `+0xd4`, `+0x20`, `+0x16c` and `+0x160` are unlinked with `0x687860`, `0x687830`, `0x6876b0` and
`0x687800`; the last list's nodes, at `+0xfc`, are unlinked from the band (`0x68056b` on) and
appended to the list at `ecx + 0xe04` (`0x6805b3`, `0x6805ce`).

## Entity draw

`0x683dd0` draws the entities. For each in the draw list it unlinks the node, sets the model
animated and visible when `+0x90` bit 3 is clear, installs the lighting callback `0x672a20`
with the entity as its argument, and calls the entity's callback `+0xb4` with edx = 1 and
its four arguments; when the callback is null or returns 0 and the model is not hidden (`+0x90`
bits 3 and 10), it runs the decal pass `0x6d78f0`. The second list gets the same callback
with edx = 2 and nothing else. Each entity increments the drawn counter `0xc7b744`.

Addresses: `0x683dd0`, `0x683dd1`, `0x683e0e`, `0x683e47`, `0x683e49`, `0x683e5f`, `0x683e63`, `0x683e74`, `0x683e9e`, `0x683ea3`, `0x683ec3`, `0x683ec8`, `0x683ed5`, `0x683f65`, `0x683f6c`, `0x6d78f0`, `0x672a20`, `0x710c50`, `0x7134b0`
Confidence: verified
Proof: one
Test: none

`0x683e49` calls `0x710b90` with 1; `0x683e5f` forms `(~flags >> 3) & 1` for `0x710c50`; `0x683e74`
installs the callback. The callback is called with `ecx` its argument `+0xb8` and three more
words `+0xc0`, `+0xc4`, `+0xc8` pushed (`0x683e9e`, edx = 1); `0x683ec3` calls `0x6d78f0` when
`+0x90` bit 3 is clear and bit 10 set. The second loop is `0x683ed5` on, with edx = 2 at `0x683f65`.

## Object election

An entity is drawn only if the scene elects it into the draw list, and the election is the pass number.
The walk moves the entities in sight to the draw list; the drain `0x680390` moves every one left in a band
to the second list. `0x683dd0` calls each entity's callback `0x481540` with pass 1 for the first list and 2
for the second. The pass-2 model's visible flag `+0x50` is written 0, and the model update then does not
link it into the scene's draw list. With the camera in a sealed map object with no exterior window, no
walk runs and every outdoor entity is in the second list.

Addresses: `0x683dd0`, `0x680390`, `0x683340`, `0x6834e0`, `0x481540`, `0x4815d0`, `0x4815ff`, `0x48174e`, `0x6146c0`, `0x6146c6`, `0x710c50`, `0x718960`, `0x719202`, `0x719207`, `0x681199`, `0x681204`
Confidence: verified
Proof: one
Test: none

`0x681199` skips the walk when the camera's map object has no window, and `0x681204` runs the drain on
every path. `0x4815ff` calls slot `+0x40` of the entity's class with the pass; `0x6146c6` is `and eax,1`.
`0x4815d0` passes the result on to `0x710c50` (`0x48174e`), and `0x481540` itself calls `0x710c50` with 0
when the entity is hidden or has no model ready. `0x719202` loads the model's `+0x50`; `0x719207`
skips the link into the scene's list `+0x24` for 0. The list walks of `0x683dd0` call `0x710b90` only for the
first list. An entity inside the camera's own map object group is placed by `0x6834e0` the same way: those
in the frustum go to the first list and the rest to the second.

## Doodad def draw

`0x683f80` draws the doodad defs: each is unlinked, its transform callback `+0x174` called and its
model's matrix set. With world flag 1 set, d is the horizontal distance of the centre from the camera less the
radius r, and r picks the alpha 1 − (d − s)/n, capped at 1.0: s = 40, n = 10 for r ≤ 0.5; s = 100,
n = 25 for r ≤ 2.5; s = 150 (the far clip less 50 when that is under 200), n = 50 for r ≤ 7.0; a
larger r never fades. Above 0 the model is set animated, visible and lit with the alpha; else it stays as it was.

Addresses: `0x683f80`, `0x810188`, `0x81018c`, `0x683f86`, `0x683f95`, `0x683fb3`, `0x683fc4`, `0x683fd6`, `0x684049`, `0x68405f`, `0x684076`, `0x68407b`, `0x684092`, `0x6840ab`, `0x6840b1`, `0x6840b7`, `0x6840cb`, `0x6840e6`, `0x6840d8`, `0x6840f3`, `0x6840fe`, `0x684107`, `0x684113`, `0x684120`, `0x68412c`, `0x684144`, `0x68414d`, `0x684155`, `0x684162`, `0x684173`, `0x684182`, `0x710620`, `0x710b90`, `0x710c50`, `0x710cb0`, `0x8101a8`, `0x810190`
Confidence: verified
Proof: one
Test: `terrain::doodad_fade_difftest`

The band limits are `0x8101a0` (50.0) less `0x810194` (10.0), `0x8101a4` (125.0) less `0x810198`
(25.0), `0x8101a8` (200.0) less `0x81019c` (50.0); `0x683fb3` to `0x683fd2` replace the third with
the far clip less 50.0 when the far clip is below 200.0 (`test ah,5; jp` skips for equal, larger
and NaN). `0x6840ab` takes the square root of the squared x and y offsets from `+0x5c` and
`+0x60`; `0x6840b1` subtracts the radius. The radius tests at `0x6840b7`, `0x6840cb`,
`0x6840e6` compare with 7.0, 0.5 and 2.5 and take the band for a value at or below. `0x684107`
forms 1 − t; `0x684113` caps it at 1.0 and `0x68412c` to `0x684144` skip a value of 0 or less
or NaN. `0x684155` is the one place a placed doodad's model is linked into the scene's worklist
(`0x710b90` with 1); `0x684162` sets it visible, `0x684173` installs the deferred draw `0x672a20` with the def
as its argument and `0x684182` sets its alpha. A def skipped here is never linked, so its model is neither animated nor ticked that frame. The bucket `0xc7caec` is emptied as it drains; a def
in it is in no other list, since all share the def's node at `+0x16c`.

## Liquid layer cull

`0x683ab0` culls one liquid layer (edx 1, 0 or 2) of a band. For each slot in the band's list for
the layer it builds the slot's box, and a slot that is inside the frustum (`0x682f40` is 0) and not
hidden by the horizon (`0x686180` with flags 0) leaves the band list and joins the layer's draw
list.

Addresses: `0x683ab0`, `0x683ac3`, `0x683b2b`, `0x683b33`, `0x683b45`, `0x683b52`, `0x683b8b`, `0x683bbc`, `0x683bcd`, `0x68df40`, `0x682f40`, `0x686180`
Confidence: inferred
Proof: none
Test: none

`0x683ac3` reads the band list head at `band + layer × 0xc + 0x38`. The box comes from `0x68df40`
and is not read here. `0x683b33` and `0x683b45` are the two tests; the band list node `+0x20` is
unlinked at `0x683b52`, and `0x683bbc` takes the layer's list offset from `0xc7cb14 + layer × 0xc`
and `0x683bcd` stores its tail at `0xc7cb18 + layer × 0xc`.

## Portal volumes

A volume is six planes, 16 bytes each (n, d), then eight corners from `+0x60`, 12 bytes apart. The corners
are in the frustum corner order: c0 to c3 on the near plane (bottom-left, top-left, top-right, bottom-right) and
c4 to c7 the same on the far plane. `0x682930` makes the eight corners of a portal window from the frustum
rays: two passes of four corners, each narrowed to the window's rectangle. It hands them to `0x6865f0`, which
copies them to the volume at the top of the frustum stack and builds the planes with `0x686640`, then sorts a local
copy of the corners that nothing reads.

| plane | side | normal | d = −n·anchor |
|---|---|---|---|
| 0 | top | `(c5 − c1) × (c6 − c1)` | c1 |
| 1 | bottom | `(c7 − c0) × (c4 − c0)` | c0 |
| 2 | left | `(c4 − c0) × (c5 − c0)` | c0 |
| 3 | right | `(c6 − c3) × (c7 − c3)` | c3 |
| 4 | far | `(c4 − c5) × (c6 − c5)` | c5 |
| 5 | near | −(plane 4 normal) | c2 |

Addresses: `0x682930`, `0x682d6e`, `0x682d73`, `0x6865f0`, `0x686625`, `0x686640`, `0x686659`, `0x6866bb`, `0x686714`, `0x686775`, `0x6867c6`, `0x6867ce`, `0x686816`, `0x637480`, `0x682490`, `0x672130`, `0x6720f0`, `0x4841f0`, `0x602630`, `0x6866c1`, `0x68671f`, `0x68677a`
Confidence: verified
Proof: one
Test: `terrain::frustum_push_rect_difftest`, `terrain::side_plane_d_negate_difftest`, `terrain::plane5_difftest`

`0x682d6e` selects the volume `[0xc7cfe0] × 0xfc + 0xc7d308` and `0x682d73` is the sort of 8
elements of 12 bytes. `0x686659` is the `0x637480` call with the corners at `+0x6c`, `+0x9c` and
`+0xa8` (c1, c5, c6; the standard cross, normalised). Planes 1 to 4 are built at `0x686665`, `0x6866c1`, `0x68671f` and
`0x68677a`: two edges by `0x4841f0` (the first built, then the second), `n = second × first` through the
leaf `0x672130` (a cross whose component order is not the textbook's), then `0x6720f0` normalises and `0x602630` dots
the anchor, negated at `0x6866bb`, `0x686714`, `0x686775` and `0x6867c6`. `0x6867ce` to `0x686816` negate plane 4's
normal for plane 5 and take d from the corner at `+0x78`.

## Exterior map objects

`0x6856c0` runs on a band's map object defs, for the rectangle `edx`. A def whose box at `+0x44` is
inside the frustum and not hidden by the horizon (flags 1) gets the camera's position and a far
point moved into its space, and their unit direction and plane offset stored as the cull ray. It
sets the group callback `0x685d70` through `0x6b3880`, seeds its exterior groups with `0x6b3dd0`,
and unlinks the def from the band. `0x685d70` runs per group: it links the group to the frame's
group list with a copy of the current frustum volume, and culls the group's doodad defs
(`0x6838f0`, when world flags hold 0x400081) and entities (`0x6834e0`).

Addresses: `0x6856c0`, `0x68583a`, `0x68584e`, `0x685926`, `0x685a15`, `0x685a29`, `0x685a3c`, `0x685a4f`, `0x6b3880`, `0x6b3dd0`, `0x685d70`, `0x685dc1`, `0x685f17`, `0x685faf`, `0x685fc1`, `0x685fcb`, `0x6838f0`, `0x6834e0`
Confidence: inferred
Proof: none
Test: `terrain::wmo_corner_ray_negate_difftest`, `terrain::wmo_dir_len2_gate_difftest`, `terrain::wmo_dir_normalize_difftest`, `terrain::wmo_plane_offset_negate_difftest`
 
The two tests are `0x68583a` (frustum) and `0x68584e` (horizon, edx = 1). `0x685926` moves the
camera and `0xc7d118` through the def's matrix at `+0xd4`; `0x685a15` takes their dot product with
the direction. `0x685a29` stores the callback and `0x685a3c` seeds. In `0x685d70` the group is found
in the def's list by its index (`ecx`, `+0x7c`); `0x685dc1` branches on an existing frustum slot
(`+0xd0`); `0x685faf` tests `0xc7b2a4` against 0x400081 before `0x685fc1` and `0x685fcb`. The
frustum slot copy is not read here.
