# terrain: world

The `CWorld` entry points in `World.cpp`: what the client does once at start-up, each frame and
when a map loads, and the small queries and callbacks the rest of the client calls on a world
object. The sky and fog hand-off of the frame is in `lighting`.

Roots: `0x66f6c0`, `0x66fbe0`, `0x66fd50`, `0x672730`, `0x672df0`, `0x672ef0`

## World initialization

`0x66f6c0` runs once at start-up. It ORs 0x7100b73 into the world flags, sets the alpha reference
to 0x80, `maxLOD` to 3 and the far chunk level to 2, squares the detail doodad horizon (70.0 × 70.0
= 4900), reads the shadow level and the fog end from option objects, calls the scene table fill and
the map initializer, builds the drift cloud and the weather manager, hands two callbacks to the
model scene and registers 13 console commands.

Addresses: `0x66f6c0`, `0x66f6cb`, `0x66f8ae`, `0x66f8b8`, `0x66f971`, `0x66f9af`, `0x66f9dc`, `0x66f9f5`, `0x66fa06`, `0x680610`, `0x691b10`, `0x706e10`, `0x70a230`, `0x70a250`, `0x63f9e0`, `0x58ab10`, `0x6729f0`, `0x66fff3`, `0x672590`, `0x673290`
Confidence: verified
Proof: one
Test: none

The flags are `or eax,0x7100b73` at `0x66f6cb`; `0x66f8ae` and `0x66f8b8` store 3 and 2 in the
level globals. The drift cloud is 0xfa40 bytes (`World.cpp` line 0x12d, `0x66f971`), built by
`0x68e5a0` with the size base 1/36, the edge 30.0 and `Textures\WaterPoop02.blp` (`0x66f9af`); the
weather manager is 0x84 bytes (line 0x130) built by `0x67b740` (`0x66f9dc`). `0x70a230` gets
`0x672b30` and `0x70a250` gets `0x672b60` (`0x66f9f5`, `0x66fa06`) on the scene `0x706e10` made. The
commands, in registration order, are `showDetailDoodads`, `maxLOD`, `showCull`, `setShadow`,
`waterMaxLOD`, `waterWaves`, `waterSpecular`, `waterRipples`, `waterParticulates`, `showShadow`,
`showLowDetail`, `showSimpleDoodads` and `detailDoodadAlpha`; `setShadow` runs the shadow colour
setter `0x673290`, which reads four floats from 0 to 1 (a NaN passes) and packs them first float
in the top byte. The world frame writes the shadow colour back from the light state's `+0x194` (`0x66fff3` through
`0x672590`), so a `setShadow` value lasts one frame. The option table of 64 dwords, all −1, goes to `0x58ab10` with the query
`0x6729f0`.

## Map load

`0x66fbe0` sizes the area of interest at the given position, copies the four new window bounds to
the previous ones, so the first frame after a load is not a teleport, and calls the map loader
`0x6941f0` with the map id and its stack argument.

Addresses: `0x66fbe0`, `0x66fbe8`, `0x66fc1e`, `0x6941f0`
Confidence: verified
Proof: one
Test: none

`ret 8`. `0x66fbe8` calls `0x672730` with `ecx` the position; `0x66fc1e` calls the loader with `ecx`
the map id and `edx` the stack argument.

## World frame

`0x66fd50` runs each frame with a camera position and a camera. It stores the frame time, sizes the
area of interest at the position, sets the teleport flag, advances the frame counter and the eight
liquid texture offsets, captures the scene camera, updates the sky and fog, runs the map update and
sets the fog end. With world flag 0x2000000 set and the camera liquid state not 0xf, it advects
the drift cloud; the weather update always runs.

Addresses: `0x66fd50`, `0xc7b2c8`, `0x672700`, `0x672730`, `0x680bc0`, `0x66ff60`, `0x698390`, `0x68e930`, `0x67be40`, `0x66fdea`, `0x66fdfc`, `0x66fe10`, `0x66fe35`, `0x66fe7d`, `0x66fe8d`, `0x66fe99`, `0x66feec`
Confidence: verified
Proof: one
Test: none

`ret 4`. `0x672700` pushes the frame seconds into a ring of 30 floats. The previous and new windows
are tested for overlap: the teleport flag is 1 when the larger of the two minima is at least the
smaller of the two maxima on either axis (`0x66fdea`), so a shared single index counts as
disjoint. The frame counter increments at `0x66fdfc`. The eight direction pairs at `0x867918` are
(−1,0), (−1,1), (0,1), (1,1), (1,0), (1,−1), (0,−1), (−1,−1); each offset pair in the 16-byte
records from `0xc5a480` adds the frame seconds times its direction (`0x66fe10`), and a component
of 64.0 or more resets to 0 (`0x66fe35`, `test ah,1`), so a negative one only decreases.
`0x66ff60` gets force = the dirty flag OR the teleport flag (`0x66fe7d`); `0x698390` runs with 1
(`0x66fe8d`). The fog end is the light state's `+0x78` when its `+0x7c` is 0, else the larger of
`+0x78` and `+0x88` (`0x66fe99`).

## Area of interest

`0x672730` centres two windows on a position in chunk indices, index = round((17066.666 − p) × 0.03
− 0.5) per axis, the first from p.x. The inner window is the index ± r, r = 1 + trunc(far clip ×
0.03) (`0x6725d0`); the outer is the index ± max(r + 2, 8). Both clamp to 0 to 1023. It also
rebuilds the position boxes: ± the far clip, those ± 33.333332, ± 150 and ± the far clip in three axes.

Addresses: `0x672730`, `0x6725d0`, `0x6728a4`, `0x6728cb`, `0x6728e3`, `0x672924`, `0x672969`, `0x672971`, `0x6725d9`, `0x6725f4`, `0x672602`, `0x672608`, `0x67260e`, `0x672614`
Confidence: verified
Proof: one
Test: none

`0x6728a4` forms −(p.y − 17066.666) and the p.x term likewise; each multiplies by 0.03, subtracts
0.5 and `fistp` rounds to nearest. The bounds are `idx − r` and `idx + r`, clamped at 0 and at
0x3ff (`0x672924` on); the outer ones use `max(r + 2, 8)` (`0x672969`, `0x672971`). `0x6725d0` stores
the far clip when it differs from the stored one, or is NaN (`0x6725d9`), then both half-extents =
1 − `ftol(far × −0.03)` (`0x6725f4`, `0x672602`) and three dirty flags (`0x672608` to `0x672614`).

## World flags

The world flags `0xc7b2a4` gate the passes. At start-up 0x7100b73 is ored in, which sets bits 0x1, 0x2, 0x10, 0x20, 0x40, 0x100,
0x200, 0x800, 0x100000 and the liquid, weather and low-detail bits. These are the bits something sets
after that, the setting that does, and what tests the bit:

| bit | set by | tested by |
|---|---|---|
| 0x4 | `lod` setting | chunk level, mesh selection |
| 0x20 | `showCull` | the horizon steps, chunk visibility |
| 0x40 | `mapShadows` setting, `showShadow` | chunk load, shadow pass |
| 0x400 | `showfootprints` setting | footprints, footstep |
| 0x100000 | `showDetailDoodads` | the chunk pass |
| 0x2000000 | `waterParticulates` | the particulate draw, camera probe |
| 0x4000000 | `showLowDetail` | the far band |
| 0x80000000 | `anisotropic` setting | the chunk and map object passes |

Addresses: `0x66f6c6`, `0x673250`, `0x673a30`, `0x673050`, `0x673770`, `0x673a70`, `0x688920`, `0x688a70`, `0x6892e0`, `0x689110`, `0x6afb11`, `0x6beb56`, `0x6af6ef`, `0x6bf5a0`, `0x69a3e9`, `0x5fc01b`, `0x6847c9`, `0x66fee0`, `0x6701e0`, `0x680afa`, `0x6844bc`, `0x68451e`, `0x68484e`
Confidence: verified
Proof: one
Test: none

The sets are the `or` and `and` of the named handler on `0xc7b2a4`; `lod`, `mapShadows` and `showfootprints` are
registered settings with callbacks `0x688920`, `0x688a70` and `0x6892e0`. The tests: `0x6afb11` and `0x6beb56` read bit 4,
`0x6af6ef` and `0x6bf5a0` bit 0x40, `0x69a3e9` and `0x5fc01b` bit 0x400 (`test ah,4`), `0x6847c9` bit 0x100000,
`0x66fee0`, `0x6701e0`, `0x680afa` bit 0x2000000, `0x6844bc` bit 0x4000000 and the passes at `0x68451e` and
`0x68484e` the sign.

## Other world flags

The remaining bits have no live setter or tester of the plain kind. Bits 0x1, 0x2, 0x100 and 0x1000000 (doodads, chunks,
map objects, liquids) stay at their defaults, and each is read where its pass begins. Bit 0x100 also gates the
camera down-ray, the area id query and the group liquid pass; 0x1000 the group portal debug draw. The commands that
would toggle these bits, the debug ones and the two flag setters are registered nowhere, so their bits
cannot change. Six settings reach bits nothing tests in the word: `fullAlpha` 0x4000, `doodadAnim` 0x8000, `triangleStrips` 0x10000,
`trilinear` 0x800000, `specular` 0x8000000 and `pixelShaders` 0x10000000.

Addresses: `0x6730f0`, `0x673130`, `0x673430`, `0x673670`, `0x6730a0`, `0x673470`, `0x6734b0`, `0x6731c0`, `0x673170`, `0x673910`, `0x6737f0`, `0x673820`, `0x684610`, `0x68407b`, `0x6849e1`, `0x684e3f`, `0x6821f6`, `0x6702f9`, `0x6b4043`, `0x68512e`, `0x483513`, `0x6b402d`, `0x6b38b6`, `0x68136c`, `0x688960`, `0x688a10`, `0x689400`, `0x688ce0`, `0x688e20`, `0x688f40`
Confidence: verified
Proof: one
Test: none

A scan of the image for the handlers' addresses as data finds none for the toggles at `0x6730a0` to `0x673910`
and the two setters `0x6737f0` and `0x673820`, which flip 0x20000000 and 0x40000000; the registered commands are
thirteen (`world#world-initialization`). `0x483513` tests 0x20000000, `0x6b402d` 0x40000000, `0x6b38b6` 0x80000
and `0x68136c` 0x200000. `0x6849e1`, `0x684e3f`, `0x6821f6` and `0x6702f9` test `ah,1` (0x100), `0x6b4043` `ah,0x10`
(0x1000) and `0x684610` bit 2.

## Distance settings

Four registered settings bound the world's distances, each rejecting a value outside its range with a message.
`farclip` (default 350) takes 177 to 777 and sets the world far clip. `horizonfarclip` (2112) takes the far
clip plus 528 up to 2112 and sets the far plane of the low-detail projection. `nearclip` (0.1) takes 0.01 to
0.33 and `textureLodDist` (777.0) takes 80 to 777; their world copies are never read. The camera takes its
near and far planes from the `nearclip` and `farclip` records, not from the world copies. `lodDist` is in `lod`.

Addresses: `0x688470`, `0x688d40`, `0x689250`, `0x688d90`, `0x6891f0`, `0x6725c0`, `0x6726c0`, `0x6725d0`, `0x672630`, `0x6830d9`, `0x684240`, `0x511bd4`, `0x511be5`, `0x50b708`, `0x50b728`, `0xc7b498`, `0xc7b49c`, `0xc7b480`, `0x86795c`, `0xbe0cc0`, `0xbe1078`
Confidence: verified
Proof: one
Test: none

`0x688470` registers `farclip` with callback `0x688d40`, `nearclip` with `0x688d90`, `textureLodDist` with
`0x6891f0` and `horizonfarclip` with `0x689250`. Each parses the text and compares in both directions with
inclusive bounds (`test ah,5` then `test ah,0x41`): `0x81021c` 177.0 and `0x80fed8` 777.0, `0x8029d0` 0.01
and `0x808300` 0.33, `0x80fedc` 80.0 and 777.0, and the far clip `0xc7b498` plus `0x810228` (528.0) to
`0x810224` (2112.0). A pass calls the setter (`0x6725d0`, `0x6725c0`, `0x6726c0`, `0x672630`) and returns 1;
`farclip`, `nearclip` and `horizonfarclip` print their range and return 0, while `textureLodDist` prints
and stores 80.0. `0x6725c0` stores `0xc7b480` and `0x6726c0` stores `0x86795c`, each referenced
nowhere else. `0xc7b49c` is read at `0x6830d9` and `0x684240`. `0x511bd4` and `0x511be5` copy
the cached `nearclip` and `farclip` records' value at +0x24 (`0xbe1078`, `0xbe0cc0`, looked up at
`0x50b708` and `0x50b728`) into the camera's +0x38 and +0x3c every frame.

## Detail doodad and alpha settings

`0x6725a0` stores the detail doodad density and, when it differs, invalidates the detail doodads
through `0x6b1d20`. `0x6739a0` reads one integer from the command text and stores it as the alpha
reference when it is 0 to 255, else prints "Alpha ref range 0 - 255.".

Addresses: `0x6725a0`, `0x6b1d20`, `0x6739a0`, `0x6739b9`, `0x6739d5`, `0xc7b2fc`
Confidence: verified
Proof: one
Test: none

`0x6739b9` is `cmp eax,0xff; jbe`, so 255 is accepted; `0x6739d5` stores it. `0x6725a0` tail calls
`0x6b1d20` after the store.

## Entity queries

Wrappers over a map entity (`ecx`). `0x6704c0` returns 1 when the entity is non-null and its `+0x94`
is 0. `0x670630` returns 0 unless `+0x90` bit 5 is set; then it copies `+0x98`, the three floats at
`+0xe4`, `+0xf0` and bit 8 of `+0x90` to its outputs and returns 1. `0x670540` returns 0 unless
`+0x90` bit 0 (indoors) is set; then `0x69db30` writes three words naming the entity's map object
group, `[[model + 0x120] + 0x20]`, `[map object model + 0x118] + 4` and the group key `+0x7c`, and
returns 1 if a group is found. `0x672470` writes the camera liquid state to its argument when
non-null and returns it.

Addresses: `0x6704c0`, `0x670630`, `0x670540`, `0x69db30`, `0x69d4f0`, `0x672470`
Confidence: verified
Proof: one
Test: none

`0x69d4f0` walks the entity's node list `+0x20` for the first group node (`+0x8` flags bit 0x10),
then that group's own list for the map object and writes the object, its model `+0x118`, the
group and the group record `0x6a4a00` finds by `+0x7c`. Its callers compare `0x670540`'s words between
two entities.

## World thunks

Single-call forwards. `0x6723b0` calls the box test `0x6ad2c0` with its argument. `0x6723d0`
calls the liquid query `0x69b6d0` and returns the low byte. `0x6701d0` jumps to the liquid phase
`0x6816d0` and `0x670240` to the footprint draw `0x69a3e0`. `0x6701e0` jumps to the drift cloud draw
`0x68efe0` when world flag 0x2000000 is set and the camera liquid state is not 0xf, and returns
otherwise. `0x672640` returns at once with `ret 4` and `0x6953d0` with `ret`.

Addresses: `0x6723b0`, `0x6723d0`, `0x6701d0`, `0x6701e0`, `0x670240`, `0x672640`, `0x6953d0`, `0x6ad2c0`, `0x69b6d0`, `0x6816d0`, `0x69a3e0`, `0x68efe0`
Confidence: verified
Proof: one
Test: none

`0x6723d0` passes its second stack word twice: as the depth out and as the pointer `0x69b6d0`
takes third. `0x6701e0` tests `0xc7b2a4` bit 0x2000000 and `0xc7f288` against 0xf.

## Nearest liquid

`0x6723f0` returns 1 after filling an out array of 12 dwords with the nearest liquid per type
around a point. The local search state starts as 12 dwords of 0x7f7fffff and the caller's array as
zero. An indoor entity first runs `0x69dd70` over its map object groups with a counter of 9999;
when the counter stays below 2 or the entity is outdoors, the terrain search `0x68af10` runs.

Addresses: `0x6723f0`, `0x672417`, `0x67242f`, `0x67243b`, `0x672456`, `0x69dd70`, `0x68af10`, `0x68ac70`
Confidence: verified
Proof: one
Test: none

`ret 8`. The distances are a local of 12 dwords at `ebp − 0x34`, the 0x270f counter at `ebp − 4` just above it. `0x672417` tests `+0x90` bit 0. Indoors `0x67242f` stores 0x270f as the counter and
`0x69dd70` calls `0x68ac70` per group; `0x67243b` skips the terrain search when the counter is 2 or
more. `0x69dd70` visits each group node of the entity and passes the group's key, its model's
liquid data and the entity position through the model's matrix.

## Model fade-out

`0x672df0` fades a removed model: when the model is ready and the fade value is at least 0.01
(or NaN) it records the model, the clock time and the value, and installs `0x672d20` as the model's
callback; otherwise it releases the model at once. `0x672ef0` runs each frame: a record older
than 2000 ms releases its model; a younger one sets the model's alpha to (3 − 2x)x², x = (1 −
age × 0.0005) × the value, with 0 for x ≤ 0 and 1 for x > 1.

Addresses: `0x672df0`, `0x672ef0`, `0x672d20`, `0x672e11`, `0x672e21`, `0x672e2c`, `0x672e76`, `0x672ec0`, `0x672f36`, `0x672fd3`, `0x672fdc`, `0x672fe2`, `0x672fe8`, `0x673009`, `0x673017`, `0x673022`, `0x673035`, `0x671ac0`, `0x710450`, `0x710cb0`
Confidence: verified
Proof: one
Test: none

`0x672e11` tests `0x710450` (ready); `0x672e21` is `fcomp 0.01` and `test ah,5; jnp`, which jumps to
the release `0x672ed5` below 0.01 only. A record is 20 bytes: model, `0x42c010` time, value.
`0x672ec0` stores the callback and its argument at `+0xb4` and `+0xb8`. In `0x672ef0`,
`0x672f36` is `cmp eax,0x7d0; jle`, so the release comes strictly after 2000 ms; `0x672fd3` forms
(1 − age × 0.0005) × value (`0x80c698`, then `fsubr 1.0`), `0x672fe8` to `0x673009` pick 0 or
1, and `0x673017` to `0x673025` compute (3 − 2x)x² with 3.0 at `0x80297c`; `0x673035` is
`0x710cb0`. Records and models of expired entries return to the free list.

## Model lighting callback

A model on the map takes its light from `0x672a20`, which `0x6716f0` installs as the model's
lighting callback when the model is set (and removes from the old model). With a node it calls
the node's virtual slots 1 (fog) and 3 (water plane), and slot 2 (the map lights) only when the
node's `+0x78 − +0x68` is below the fog end; if the node's `+0x8` has bit 0x20 it sets `+0x1b0` of
the model's lighting record to 1. Without a node it adds the day-night ambient `+0x178` and the
directional `+0x174` with direction `+0x168` of the light state.

Addresses: `0x6716f0`, `0x672a20`, `0x672a36`, `0x672a3f`, `0x672a51`, `0x672a59`, `0x672a5c`, `0x672ac7`, `0x672b1b`, `0x7134b0`, `0x670fd5`, `0x671344`, `0x6716b2`, `0x683e6f`, `0x68416e`, `0x69515f`, `0x719313`, `0x710390`, `0x7103a0`, `0x6c31e0`, `0x6a7300`, `0x6a7650`, `0x6a8050`, `0x71bc70`, `0x71bce0`, `0x810810`, `0x810a74`
Confidence: verified
Proof: one
Test: none

`0x6716f0` releases the old model's callback (`0x7134b0` with 0) and reference, stores the new
model at `+0x88`, installs the callback with the node as argument and takes a reference. The other installers
are the node creator (`0x670fd5`, when the model is set), both light object creators (`0x671344`,
`0x6716b2`), `DrawEntityModels` for each visible node with a model on every frame (`0x683e6f`),
`DrawDoodadModels` (`0x68416e`) and `CreateDoodadModel` (`0x69515f`); each pushes the node, so the no-node arm
runs only for a zero argument. A model whose callback is 0 gets no fill (`0x719313`). The
calls at `0x672a36`, `0x672a51` and `0x672a59` are `[vtable+4]`, `[vtable+8]` and `[vtable+0xc]`; in the vtables
`0x810810` (map entity) and `0x810a74` (doodad def) slot 1 is `0x6c31e0` and slot 3 `0x6a7650`,
slot 2 `0x6a7300` and `0x6a8050`. The gate compares `+0x78 − +0x68` with the fog end `0xc62964` (`test ah,5; jp`): slot 2 runs
only when the difference is below it; equal, larger and NaN skip.
`0x672ac7` and `0x672b1b` add the two colours, each channel byte × 1/255 in the order +0x17a,
+0x179, +0x178 (ambient) and +0x176, +0x175, +0x174.

## Scene callbacks

The model scene calls two world callbacks. `0x672b30` is the special-batch callback: it runs the
decal batch pass `0x6d7480` with flags 0x200122 when its second argument is nonzero or the world
mode `0xc5a470` is 2 or more. `0x672b60` is the ground callback: it traces the segment from the position
down 20 yards against the planes (mask 0x100111) and, on a hit at fraction t, writes z − 20t to the
second argument and returns 1; it returns 0 on a miss.

Addresses: `0x672b30`, `0x672b43`, `0x672b60`, `0x672b7d`, `0x672c09`, `0x672c24`, `0x6d7480`, `0x672190`
Confidence: verified
Proof: one
Test: none

`0x672b3a` compares `0xc5a470` with 2 (`jb`, so 2 passes). `0x672b7d` subtracts 20.0
(`0x804478`) from the position's z to make the segment end; `0x672c09` calls the trace thunk
`0x672190` with mask 0x100111; `0x672c24` to `0x672c2d` store (end z − start z) × t + start z.
