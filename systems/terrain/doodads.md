# terrain: doodad defs

How a placed doodad of a map object group comes to exist on the map: its def, its model and the
events that model raises, the lights of the group, and the def's lifetime. The terrain's own
doodads (the ADT's) are `files#doodad-placements`; the culling and drawing of defs is in `scene`
and their light in `lights`.

A doodad def has the key (doodad index, instance id + 1) at `+0xb4` and `+0xc8`, its position
`+0x24` and scale `+0x30`, its sphere `+0x5c` and `+0x68`, its model `+0x88`, its flags word `+0xc`,
its matrices `+0xcc` and `+0x10c`, its looping sound handle `+0x168`, and the node `+0x16c` that
links it into the scene.

Roots: `0x694e90`, `0x695aa0`, `0x695c00`, `0x6952a0`, `0x698c50`, `0x6a0840`

## Group doodad def

`0x694e90` makes the def for one doodad of a map object group, or returns the one already made
for the same key. A new def takes the record's position moved by the group matrix, its scale and
a matrix of translate, rotate (the record's quaternion) and scale, multiplied by the group matrix. The def's directional
light is the record's colour raised to a floor of 0x70 and its ambient the colour lowered to a cap
of 0x60 (`0x6a77e0`). It then makes the def's model and registers the def in the pending list.

Addresses: `0x694e90`, `0x694f0e`, `0x694f74`, `0x694fb4`, `0x694fcd`, `0x694fd8`, `0x69500d`, `0x695040`, `0x69506a`, `0x695078`, `0x695083`, `0x6950be`, `0x6950c6`, `0x6950cf`, `0x6950e9`, `0x6a07f0`, `0x6a77e0`, `0x695100`, `0x7bcc60`, `0x7bdc40`, `0x7bddb0`, `0x7bdd00`, `0x7bce30`
Confidence: verified
Proof: one
Test: none

`ret 0xc`. The key is checked first in the table at `0xca7ddc` (`0x694ed8`, `0x694ee3`): the same
index and instance id return the def. `0x694f74` stores the key; `0x694fcd` moves the position by the
matrix (`0x7bcc60`); `0x694fd8` copies the scale; `0x69500d` sets flags to 1 and `0x695013` clears the model.
The matrix at `+0xcc` is identity, then translated by the record (`0x69506a`), rotated by its quaternion
(`0x695078`), scaled (`0x695083`) and multiplied by the group matrix (`0x6950be`). `0x6950c6` to
`0x6950e3` call `0x6a77e0` with the floor 0x70 (`0x6950cf`) and the cap 0x60.

## Doodad model

`0x695100` makes a def's model from the path, sets its base matrix, and installs the done
callback `0x6951b0`, the event callback `0x6951e0` and the lighting callback `0x672a20` with the def
as argument. It starts the model's default animation and, when asked, makes it a priority load.
`0x6951b0` restarts the animation when its first argument is 0.

Addresses: `0x695100`, `0x695107`, `0x69510f`, `0x69512c`, `0x695140`, `0x695153`, `0x695164`, `0x695180`, `0x6951b0`, `0x6951cc`, `0x707350`, `0x710620`, `0x711bb0`, `0x7133a0`, `0x7134b0`, `0x7121a0`, `0x7103d0`
Confidence: verified
Proof: one
Test: none

`ret 4`. `0x69510f` creates the model in the world's scene (`0xc7b298`); it returns at once when
that fails (`0x695120`). `0x695140` sets the done callback, `0x695153` the event callback, `0x695164` the
lighting callback; `0x695180` calls the animation start with −1, 0, 1.0, 0, 1, 1; `0x695196` calls
`0x7103d0` with 1 and 0 when the load flag is set. In `0x6951b0` the same animation start runs
(`0x6951cc`).

## Doodad model events

`0x6951e0` handles the sound events of a doodad model by the event code. `$DSO` plays the sound
at the position with volume 1.0; `$DSL` starts a looping sound emitter for the def, replacing the
current one if the sound differs, or moves it; `$DSE` ends the emitter.

Addresses: `0x6951e0`, `0x6951e6`, `0x6951f3`, `0x6951fa`, `0x695212`, `0x69522d`, `0x69523c`, `0x69525b`, `0x69526c`, `0x695284`, `0x458870`, `0x461d80`, `0x461f80`, `0x462000`, `0x462120`
Confidence: verified
Proof: one
Test: none

`ret 0x18`. The codes are compared as `cmp eax,0x45534424`, `0x4c534424` and `0x4f534424`, which are
the little-endian dwords of "$DSE", "$DSL" and "$DSO" (`0x6951e6`, `0x6951f3`, `0x6951fa`). `0x695212` is
`0x458870`; the loop case reads the def's `+0x168` handle (`0x695220`), compares its sound id
`0x462120` with the event's (`0x695232`), frees a differing one (`0x69523c`), adds a new emitter
`0x461d80` (`0x69525b`) or moves the existing (`0x462000`); `$DSE` frees (`0x695284`).

## Doodad bounds

`0x6952a0` sets a def's world bounds from its model. The sphere centre is the model's bounding
centre moved by the def's matrix, the radius the model's bounding radius times the scale, and the
two boxes (the bounding and the collision box) are the model's moved by the matrix into `+0x44` and
`+0x14c`.

Addresses: `0x6952a0`, `0x695325`, `0x695340`, `0x695366`, `0x695385`, `0x69538f`, `0x6953a0`, `0x6953af`, `0x6953c0`, `0x713640`, `0x713680`, `0x713700`, `0x6dc540`, `0x7bca80`
Confidence: verified
Proof: one
Test: none

`0x695325` takes the bounding box (`0x713640`), `0x695340` the sphere (`0x713680`: centre then
radius), `0x695366` the collision box (`0x713700`); `0x695385` moves the centre by `+0xcc`; `0x69538f`
multiplies the radius by `+0x30` and `0x6953a0` stores it at `+0x68`; the boxes are moved at
`0x6953af` and `0x6953c0`.

## Group doodads

`0x695aa0` makes the defs of a map object group. For each doodad index of the group it takes only
those in the first doodad set or in the object's current set, and none outside every set; it makes the def with `0x694e90`, links it into
the group's doodad list and sets its light flags: an interior group's def gets bit 2 unless it has
bit 4; any other group's def gets bit 4, bit 2 cleared and intensity 1.0. It then sets bit 3 of the
group's flags.

Addresses: `0x695aa0`, `0x695ad0`, `0x695ae0`, `0x695aec`, `0x695b1e`, `0x695b2f`, `0x695b94`, `0x695ba4`, `0x695bb3`, `0x695bc3`, `0x695bea`, `0x6a4fb0`, `0x6a0450`
Confidence: inferred
Proof: none
Test: none

`ret 8`. `0x6a4fb0` returns the 0-based index of the set whose range holds the doodad index, or 0xffffffff
(`0x6a500d`) for an index in no set; a 0 passes (`0x695aea`), anything else must equal the object's `+0x124`
(`0x695aec`). `0x6953e0` stores 0 there (`0x6954a7`), `0x695650` the record's word `+0x3a` (`0x695753`). The def is
`0x694e90(index, record, path, id + 1, matrix)` (`0x695b1e`). `0x695b94` tests the object's `+0xc`
bit 2; `0x695ba4` and `0x695bb3` set the flags; `0x695bea` ORs bit 3 into the group's flags.

## Group lights

`0x695c00` makes the point lights of a map object group. For each light index it takes only type 0
entries not yet made and builds a map light at the entry's position moved by the object's matrix,
with start and end from the entry (the end raised by 0.1 when the two differ by under 0.001), the
falloff 1 / (end − start) and the colours times the entry's intensity, and links it to the group.
It then sets bit 4 of the group's flags.

Addresses: `0x695c00`, `0x695c3d`, `0x695c61`, `0x695c8e`, `0x695c99`, `0x695cb1`, `0x695cc1`, `0x695ce2`, `0x695d03`, `0x695d0a`, `0x695d11`, `0x695d1d`, `0x695d8c`, `0x695e01`, `0x695e08`, `0x695e8a`, `0x6c09b0`, `0x6c0bb0`, `0x6c0ae0`, `0x801360`
Confidence: inferred
Proof: none
Test: none

`ret 8`. `0x695c3d` skips an entry whose type byte is nonzero; `0x695c61` makes the light and
`0x695c8e` moves its position. `0x695cb1` is the start/end gap test against 0.001 (`0x801360`),
`0x695cc1` the +0.1, `0x695ce2` the reciprocal. `0x695d03` to `0x695d11` set the attenuation
fields 0, 0.7 and 0.03. `0x695d1d` and `0x695d8c` scale the entry's two colours by its intensity
into the light's `+0x3c` and `+0x48`. `0x695e01` places an interior group's light and `0x695e08`
another's; `0x695e8a` ORs bit 4 into the group's flags.

## Group def creation

`0x695960(root, instance)` makes a def group for each group of a placed map object. Each def group is
allocated (`0x6a0ca0`), given a link node targeting the instance (`0x6a0450`) and linked into the
instance's list (`+0x134`, link offset `+0x130`). Its sphere at `+0x5c` (centre) and `+0x68` (radius) is
the group's (`0x6a44b0`) with the centre moved to the world by the instance matrix `+0x94`; its world
box at `+0x44` is the group's box moved by the same matrix. It stores the group index at `+0x7c` and
the instance's word `+0x150` at `+0x88`, clears `+0xc` and sets its light class from the group's
flags in the root's group record: bit 4 with 0x48, else bit 2.

Addresses: `0x695960`, `0x695986`, `0x69598f`, `0x695999`, `0x6959d3`, `0x6959ea`, `0x6959f1`, `0x695a1f`, `0x695a36`, `0x695a45`, `0x695a54`, `0x695a5c`, `0x695a65`, `0x695a6e`, `0x695a74`, `0x695a79`, `0x695a7d`, `0x695a83`, `0x6a0ca0`, `0x6a0450`, `0x6a44b0`, `0x6a45b0`, `0x6a4640`, `0x6dc540`, `0x7bcc60`
Confidence: verified
Proof: one
Test: none

`ret 0`, `ecx` the root and `edx` the instance, looping the root's group count `+0x168`. `0x695a1f`
calls `0x6a44b0(rec+0x5c, index)` and `0x695a36` `0x7bcc60` with `+0x94`; `0x695a45` takes the box
(`0x6a45b0`) and `0x695a54` moves it into `rec+0x44`. `0x6a4640` returns the first dword of the group
record, and `0x695a79 test al,0x48` picks `or [rec+0xc],4` (`0x695a83`) or `,2` (`0x695a7d`).
`0x685d70` finds the def group by `+0x7c` (`portals#group-submit`), and the sphere is the query the
group draw's light gather uses (`groupdraw#group-light-gather`).

## Map light record

A map object's `MOLT` record is 0x30 bytes: type `+0x0`, two flags `+0x2` and `+0x3`, colour bytes
`+0x4` to `+0x6`, position `+0x8`, intensity `+0x14`, a quaternion `+0x18` that nothing reads, and the
attenuation start `+0x28` and end `+0x2c`. Only type 0 makes a light. With flag `+0x2` the light's
`+0x3c` triple is the bytes `+0x6`, `+0x5`, `+0x4` over 255 times the intensity, and with flag `+0x3`
the triple at `+0x48` is the same. The light's falloff in the scene grid is the constants 0, 0.7 and
0.03 (`+0x54`, `+0x58`, `+0x5c`) whatever the start and end say; those only set its own range triple.

Addresses: `0x695c00`, `0x695c2f`, `0x695c3d`, `0x695c6a`, `0x695c93`, `0x695cb1`, `0x695cc1`, `0x695cdc`, `0x695cfe`, `0x695d03`, `0x695d0a`, `0x695d11`, `0x695d18`, `0x695d1f`, `0x695d2e`, `0x695d87`, `0x695d8e`, `0x698f50`, `0x699110`, `0x699120`, `0x699146`, `0x71b650`, `0x6c3c5d`, `0x801360`, `0x801620`, `0x8026c8`
Confidence: verified
Proof: one
Test: none

`0x695c00` and `0x698f50` both walk the group's references (`+0xe4`, count `+0x140`) over the root's
records (`+0x148`) and call `0x71b650`. `0x6c3c5d` stores the record pointer at the root's `+0x148` and
the count at `+0x180` (size over 0x30).
In `0x695c00`, `0x695c3d` skips a nonzero type, `0x695d18` and `0x695d87` test the two flags, and the
three colour bytes are read at `0x695d1f`, `0x695d23` and `0x695d27`, each times 1/255 (`0x8026c8`) and the intensity
(`0x695d2e`); the light's own range is formed from `0x28` and `0x2c` (`0x695c93`), a gap under 0.001
getting 0.1 added to the end. `0x695d03`, `0x695d0a` and `0x695d11` store 0, `0x3f333333` and
`0x3cf5c28f`. `0x698f50` reads the same records at `0x699120` and registers them at `0x699146`.

## Group re-pose

`0x698f50(position, heading)` poses a placed map object again. It rebuilds the matrices `+0x24`, `+0x94` and
`+0xd4`, moves each group's sphere and box and each map light of the object to the new place, and calls
`0x6a2070` for each group def. That poses every doodad def of the group's list (`+0xb0`, link offset `+0xa8`)
with `0x6a81d0`: a def with a loaded model gets the position `+0x24` as its local translation
(`+0x13c`) times the object matrix, the matrix `+0xcc` as its preserved local matrix `+0x10c` times it, and new
bounds. The sun scale `+0xa4` is not touched.

Addresses: `0x698f50`, `0x699146`, `0x6991a9`, `0x6991b8`, `0x6991ca`, `0x6991db`, `0x6991e4`, `0x6a2070`, `0x6a207b`, `0x6a209d`, `0x6a81d0`, `0x6a81dc`, `0x6a81f2`, `0x6a8215`, `0x6a8227`, `0x6a822c`, `0x6709a0`, `0x5f50a0`, `0x7bcc60`, `0x7bc6a0`
Confidence: verified
Proof: one
Test: none

`0x6a81d0` returns at once for a def without a model (`0x6a81dc`, `0x6a81e4`). `0x7bcc60` writes the transformed
vector back through its input pointer, so `0x6a8215` leaves the world position in `+0x24`. `0x694e90` saved
the local matrix at `+0x10c` before it multiplied by the object matrix, which is what `0x6a8227` reuses. The
transport tick `0x5f50a0` reaches `0x698f50` through `0x6709a0` when the object's class bits `+0x8` hold 8.
The sun scale stays what the pending drain set, so a moving map object keeps the verdict of its first frame.

## Pending doodads

`0x698c50` finishes the doodad defs whose models have loaded. For each def in the pending list
with a model it waits until the model is ready, then sets its bounds (`0x6952a0`), puts a def that
is not interior-lit and stands in the chunk's shadow at intensity 0.5, grows the boxes of the
objects that hold it (`0x698b30`) and sets bits 0 and 7 of its flags. It then takes the def off the
pending list.

Addresses: `0x698c50`, `0x698c75`, `0x698c8c`, `0x698c97`, `0x698c9c`, `0x698ca5`, `0x698cae`, `0x698cb4`, `0x698cbc`, `0x698cc1`, `0x698cf9`, `0x7103d0`, `0x6952a0`, `0x69b350`, `0x698b30`, `0x810a64`
Confidence: verified
Proof: one
Test: none

`0x698c8c` calls `0x7103d0` with 0 and 0 and skips the def when it returns 0 (not ready,
`0x698c93`). `0x698c9c` tests the interior bit, `0x698ca5` the shadow, `0x698cb4` stores `0x810a64`. `0x698b30` walks the def's links (`+0x20`) and `0x698b70` grows each owner's box to cover the def's.
`0x698cc1` is `or byte [ebx+0xc],0x81`.

## Doodad shadow point

A doodad def is tested against the chunk shadow map at one point, its position at `+0x24`, which is the
placement origin and no point of its box. The creator stores it as (17066.666 − f+0x10, 17066.666 − f+0x08, f+0x0c);
a repeated placement copies the translation row of the new matrix into it. A set bit means shadowed:
the shadow test (`queries#shadow-test`) returns 1 for it, and a def in shadow that is not interior-lit gets intensity 0.5.

Addresses: `0x694bc0`, `0x694c8b`, `0x694cbe`, `0x698d20`, `0x698d24`, `0x698d3e`, `0x6b00f0`, `0x6b0100`, `0x6b014b`, `0x69b350`, `0x698ca5`, `0x698cb4`, `0x69e280`, `0x69e48d`, `0x810a64`
Confidence: verified
Proof: one
Test: none

`0x694c8b` to `0x694cbe` store the negated x and y fields and the z field, then add the origin passed in
(the triple (17066.666, 17066.666, 0) that `0x6b00f0` builds at `0x6b0100` and passes at `0x6b014b`). `0x698d20` sets bit 0 of `+0xc` (`0x698d24`),
and its copy of `+0x30`, `+0x34`, `+0x38` into `+0x24` is `0x698d28` to `0x698d3e`. `0x698ca5` asks `0x69b350` with `+0x24`
and stores `0x810a64` (0.5) in `+0xa4`; `0x69e48d` does the same for an entity's `+0x24`.

## Doodad def construction

`0x6a7d00` constructs a doodad def on the shared map base (`0x6c3020`). It gives the def its
point light list, sets its cull state `+0x94` to 1, sets type bit 0x40 in `+0x8`, clears the
model, the sound handle, the lighting callback `+0x174` and the lists, makes the matrices
identity, and stores the doodad def vtable `0x810a74`.

Addresses: `0x6a7d00`, `0x6a7d0a`, `0x6a7d14`, `0x6a7d21`, `0x6a7d55`, `0x6a7d79`, `0x6a7e0e`, `0x6a7eb0`, `0x6a7eb9`, `0x6a7ec8`, `0x6c3020`, `0x810a74`
Confidence: verified
Proof: one
Test: none

`0x6a7d0a` calls the base; `0x6a7d14` and `0x6a7d21` make the list at `+0x7c` with link offset 0x7c. `0x6a7d55` stores
the base vtable first and `0x6a7eb0` the def's `0x810a74` last. `0x6a7d79` sets `+0x94` to 1, `0x6a7e0e`
onward the identity entries (1.0 at `+0xcc`, `+0xe0`, `+0xf4`, `+0x108`, `+0x10c`, `+0x120`, `+0x134`,
`+0x148`), `0x6a7eb9` ORs 0x40 into `+0x8`, `0x6a7ec8` clears `+0x174`.

## Doodad def release

`0x6a0840` frees a def: it removes its sound emitter, unlinks its list nodes (`+0x10`, `+0xb8`
and `+0xc0`), destroys it through the first virtual slot and returns it to the pool.

Addresses: `0x6a0840`, `0x6a084e`, `0x6a0853`, `0x6a088c`, `0x6a0896`, `0x6a08d5`, `0x6a0920`, `0x6a0929`, `0x461f80`, `0x463b00`
Confidence: verified
Proof: one
Test: none

`0x6a084e` removes the emitter handle `+0x168` when set; the three list unlinks are at `0x6a0853`,
`0x6a088c`/`0x6a0896` and `0x6a08d5`; `0x6a0920` calls the destructor slot with 0 and `0x6a0929`
returns the block to its pool.

## Vector maximum

`0x6992c0` writes the per-component maximum of two vectors, each component the first vector's
when it is strictly greater than the second's and the second's otherwise, so a NaN takes the
second.

Addresses: `0x6992c0`, `0x6992cd`, `0x6992d1`, `0x6992f7`, `0x6992fb`
Confidence: verified
Proof: one
Test: none

`0x6992cd` is `fcompp` of the two z values and `0x6992d1` is `test ah,5; jp`: it keeps the first's z when
the second is below it, else the second's. The x test (`0x6992f7`, `0x6992fb`) is `test ah,0x41; jne`: the second's when the first is at or
below it.
