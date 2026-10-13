# models: the instance

A model instance, `CM2Model`: what the rest of the client holds for each model it shows. This doc
covers its construction, the calls it defers until its file is loaded, its transform, geosets,
textures, attachments, events and bounds. Its animation is `animation`, its lighting `lights`.

An instance is 0x428 bytes, allocated by a scene (`scene`) and bound to a shared model
(`shared`); M2Model.cpp holds its code from `0x70dc60` through `0x71b070`. Every public call that
needs the shared model either waits for it, through `0x710520`, or queues itself until the load
ends. The fixed-up header of the shared model is `md`.

Roots: `0x710330`, `0x710350`, `0x710370`, `0x710390`, `0x7103a0`, `0x7103d0`, `0x710450`, `0x7105a0`, `0x7105c0`, `0x7105d0`, `0x710600`, `0x710620`, `0x710650`, `0x7106c0`, `0x710b80`, `0x710b90`, `0x710bf0`, `0x710c00`, `0x710c50`, `0x710c90`, `0x710ca0`, `0x710cb0`, `0x710cf0`, `0x710d40`, `0x710db0`, `0x710de0`, `0x710e00`, `0x710e20`, `0x710e50`, `0x710ec0`, `0x7110d0`, `0x711230`, `0x711960`, `0x711a20`, `0x711bb0`, `0x711d20`, `0x711f10`, `0x711fe0`, `0x712090`, `0x712150`, `0x712910`, `0x712b00`, `0x712cb0`, `0x712cf0`, `0x712d50`, `0x712de0`, `0x712ed0`, `0x712ee0`, `0x712ef0`, `0x712f00`, `0x712f30`, `0x712f70`, `0x713020`, `0x713070`, `0x7130a0`, `0x7130e0`, `0x7131b0`, `0x7133a0`, `0x7134b0`, `0x7134d0`, `0x713540`, `0x713590`, `0x7135c0`, `0x713640`, `0x713680`, `0x713700`, `0x7137c0`, `0x713cb0`, `0x718910`

## Instance object

The constructor sets the reference count to 1, the base, world and transport matrices to
identity, the diffuse multiplier to (1, 1, 1), the depth-fill flag and the two loaded-children
flags to 1, and picks the skinning routine. `Initialize` links the instance to its scene, adds a
reference to the shared model and, for a copy, to the source instance, and attaches a new instance
of each of the source's children at the same attachment id.

| offset | holds |
|---|---|
| `+0x010`, `+0x014` | loaded, textures loaded |
| `+0x02c`, `+0x030`, `+0x034` | scene, shared model, copy source |
| `+0x038`, `+0x03c` | deferred call queue: head, address of the tail link |
| `+0x040` | frame of the last animation |
| `+0x090`, `+0x094` | bone states (0x118 each), bone matrices (64 bytes each) |
| `+0x0bc`, `+0x0fc`, `+0x13c` | base matrix, world matrix, transport matrix |
| `+0x180`, `+0x184`, `+0x190` | alpha, diffuse multiplier, emissive colour |
| `+0x1cc`, `+0x1d0`, `+0x1d4` | parent, attachment id, attachment index in the parent |
| `+0x1dc`, `+0x1e4` | first child, next sibling |
| `+0x3b8` | lighting record in use |

Addresses: `0x70dd70`, `0x70ea00`, `0x70e170`, `0x710390`, `0x7103a0`
Confidence: verified
Proof: one
Test: none

The identity stores are 1.0 on the diagonals of `+0xbc`, `+0xfc` and `+0x13c`; `+0x1c4`,
`+0x1e8` and `+0x1ec` get 1. `0x70ea00(scene, shared, source, flags)` stores the scene at `+0x2c`,
calls `0x71d430` on the shared model and stores it at `+0x30`, stores `+0x40 = scene+0x10 − 1`,
and for each child of the source creates `0x707400(child, 0)` and attaches it at child `+0x1d0`.
`0x7103a0` decrements `+0x0` and at zero runs `0x70e170` and frees the instance.

## Instance arrays

Initialization allocates the instance's animated state from the header's counts; a failed allocation
fails it. Each element array starts zeroed or with its states cleared, except the visibility words
(`Geoset visibility`).

| offset | array, per element |
|---|---|
| `+0x064`, `+0x06c` | one word per global sequence (its time), one word per sequence |
| `+0x090`, `+0x094` | bone states (0x118, the bone's flags copied to `+0xf4`), bone matrices (64) |
| `+0x098`, `+0x09c` | visibility words |
| `+0x0a0`, `+0x0a8`, `+0x0ac` | colour (0x50), transparency (0x20) and texture frame (0x20) states |
| `+0x0a4` | texture handles, each taken from the copy source or the shared model with a reference |
| `+0x0b0`, `+0x0b4` | texture transform states (0x98), texture transform matrices (64) |
| `+0x1c8`, `+0x1f0` | attachment visibility states (0x20), event states (0x20) |
| `+0x200` | light states (0x170): a light at `+0x104` of the record's type, `+0x3c` and `+0x8c` 1.0 |
| `+0x3c4` | camera states (0x84) with a camera at `+0x80` given the record's `+0x4`, `+0xc`, `+0x8` |
| `+0x3c8`, `+0x3cc` | ribbon states (0xd0), ribbon emitters |
| `+0x3d0`, `+0x3d4` | particle states (0x16c), particle emitters |

Addresses: `0x70ebd0`, `0x70ec30`, `0x70ec7a`, `0x70ecde`, `0x70ed0d`, `0x70ed35`, `0x70ee9d`, `0x70eecd`, `0x70ef16`, `0x70ef7d`, `0x70efea`, `0x70f05f`, `0x70f07e`, `0x70f0e9`, `0x70f15a`, `0x70f1c9`, `0x70f22a`, `0x70f2e7`, `0x70f336`, `0x70f418`, `0x70f44b`, `0x70f927`, `0x70f95c`
Confidence: verified
Proof: one
Test: none

Each array is `SMemAlloc` of the header count times the element size, each with its own line, and a
count dword before the bone, colour, texture transform, light, camera and ribbon state arrays.
`0x70ed0d` copies bone record `+0x4` into state `+0xf4`. `0x70ef16` stores `0x41af50` of copy source
`+0xa4[i]` or shared model `+0x13c[i]`, 0 for a null handle. The light loop calls `0x71b5f0` with
the scene and `0x71b620` with the record's 16-bit `+0x0` (`0x70f22a`), stores 1.0 at `+0x3c` and
`+0x8c` and 1 at byte `+0xec`. The camera loop creates the camera with `0x7ac930` and sets its
properties 4, 3 and 2 from record `+0x4`, `+0xc` and `+0x8` (`0x70f336`).

## Deferred calls

A call that changes animation state while the shared model is loading appends an 80-byte record:
its kind, the scene clock at the time, and its arguments. When the load ends, the instance builds
its state, plays its default animation, and replays the queue in order with the scene clock set back
to each record's time.

| kind | call |
|---|---|
| 0 | `SetReplaceableTexture` |
| 1 | `SetGeosetRange` |
| 2 | `BuildBatches` |
| 3 | `SetKeyBoneFlags` |
| 4 | `PlayAnimation` |
| 5, 6 | `SetKeyBoneAnimTime`, `SetKeyBoneAnimSpeed` |
| 7 | `SetKeyBoneOverride` |
| 8 | `SetLightEnabled` |
| 9 | `Update` |

Addresses: `0x70ebd0`, `0x7101c5`, `0x7102e8`, `0x70dd40`, `0x70dd50`, `0x710ec0`, `0x7110d0`, `0x711230`, `0x711f10`, `0x7121a0`, `0x7127f0`, `0x712910`, `0x712b00`, `0x713430`, `0x719370`
Confidence: verified
Proof: one
Test: `models::bone_timed_value_difftest`, `models::m2_set_bone_timed_difftest`, `models::part_span_deltas_difftest`

Each deferring function allocates the record with its own line number, runs `0x70dd40` on it, then
stores the kind at `+0x0`, 0 at `+0x4`, scene `+0xc` at `+0x8` and its arguments from `+0xc`, and
links it through `+0x3c`. The replay at the end of `0x70ebd0` saves the scene clock, and for each
record stores `+0x8` into scene `+0xc` and jumps through the ten-entry table `0x7102e8` at
`0x7101c5`; after each call it runs `0x70dd50` and frees the record. The default animation is
`PlayAnimation(−1, id, 0, 0, 1.0, 0, 1)` with the id from `0x711bf0`.

Of `0x70ebd0`, `part_span_deltas_difftest` runs `0x70ff3f`–`0x710029`; `m2_set_bone_timed_difftest`
runs `0x7127f0` whole; of `0x712910`, `bone_timed_value_difftest` runs `0x7129db`–`0x712a82`.

## Load state

`IsLoaded(wait, recursive)` returns whether the shared model is loaded, waiting for it first when
asked, and with `recursive` also whether every child is; the first success latches a flag.
`IsReady` also requires every texture of the instance and its children to be loaded. The loaded
callback runs once the instance is loaded, then the instance's parent's, up the chain.

Addresses: `0x7103d0`, `0x710450`, `0x710520`, `0x710560`, `0x7105a0`, `0x8732ac`
Confidence: verified
Proof: one
Test: none

`0x710520(name)` waits on the shared model's read request and logs `Model2: CM2Model::%s
stalled: %s` (`0x8732ac`) with the file name. `0x7103d0` latches `+0x1e8`, `0x710450` latches
`+0x14` and `+0x1ec`; textures are tested with `0x44acf0(texture, wait, 0)`. `0x710560` calls
`+0x24(ecx=instance, edx=+0x28)`, clears `+0x24`, and moves to `+0x1cc`.

## Base transform

`0x710650` builds the base matrix as identity, translated, rotated by an angle about an axis, and
scaled. `0x7106c0` does the same with a ground normal: for header flag mode 1 it rebuilds the
rotation's rows so the third row is the normal, for mode 3 it does so through `0x710a80`, and while
a key bone plays an animation with flags 2, 4 or 8 it blends the conformed rotation with the plain
one by the animation's progress.

Addresses: `0x710650`, `0x7106c0`, `0x710a80`, `0x711fe0`
Confidence: inferred
Proof: none
Test: `models::billboard_tangent_frame_difftest`, `models::m2_orthonormal_basis_difftest`, `models::query_attach_event_difftest`, `models::spline_complement_difftest`, `models::spline_param_fdiv_difftest`, `models::spline_param_fidiv_difftest`

`0x710a80` sets row 1 to `n × row0` normalized unless its length is under 2.384186e-7 (`jnp` at
`0x710b00`, so a NaN length normalizes), row 0 to `row1 × n`, and row 2 to `n`. The blend weight
in `0x7106c0` reads the state from `0x711fe0`, whose frame is `__ftol((now − +0xa8)·+0xb0) +
+0xb8` of the entry (`0x71204d`).

Of `0x7106c0`, `billboard_tangent_frame_difftest` runs `0x710769`–`0x7108ae`,
`spline_param_fdiv_difftest` `0x710968`–`0x710983`, `spline_param_fidiv_difftest`
`0x71098b`–`0x71099e`, `spline_complement_difftest` `0x7109db`–`0x710a1a`;
`m2_orthonormal_basis_difftest` runs `0x710a80` whole; of `0x711fe0`, `query_attach_event_difftest`
runs `0x712041`–`0x712089`.

## Reference frame

An instance can live in a moving frame, the world matrix of the transport its owner rides.
`SetTransportFrame(m)` copies `m` into `+0x13c` and makes it the frame; a null `m` clears the
frame. When a loaded instance gains a frame, its particles and ribbon vertices are moved into the
frame's space by its inverse; when it loses one, they are moved back by the frame. Changing one
frame for another moves nothing. Children with a resolved attachment take the same frame, an
attached child copies its parent's on every animate, and the emitters receive it as their frame.

Addresses: `0x718910`, `0x718870`, `0x718888`, `0x7188b9`, `0x7188d5`, `0x7187f0`, `0x7142bb`, `0x719147`
Confidence: verified
Proof: one
Test: none

`0x718870(frame)` returns at once for an unloaded instance (`+0x10`). With a frame it stores it
at `+0x17c` and, when `+0x17c` was null, passes `InvertAffineMatrix(frame)` to `0x7187f0`; with
none it passes the old frame to `0x7187f0` and stores 0. `0x7187f0` calls `0x7b5e60` for every
particle emitter and `0x7b7bc0` for every ribbon. The walk at `0x7188d5` recurses into children
whose `+0x1d4` is not 0xffff. `0x719147` passes `+0x17c` to `0x7b5230`.

## Geoset visibility

An instance holds one visibility word per submesh of the selected view at `+0x98` and, when the
selected view is not view 0, one per submesh of view 0 at `+0x9c`; initialization fills both with 1,
or copies the copy source's words. `SetGeosetRange(min, max, visible)` stores `visible` in each word
whose submesh id, the unsigned 32-bit `+0x0`, lies in `[min, max]` and whose zero-ness differs; a
change to `+0x98` frees the own batches. A batch is drawn only when the word at its texture unit's
submesh index `+0x4` is nonzero. Besides its own deferred replay, `SetGeosetRange` is called only
from the character component code, so any other model keeps every word at 1 and draws every submesh.

Addresses: `0x70ebd0`, `0x70ed7f`, `0x70edb5`, `0x70edce`, `0x70edfe`, `0x7110d0`, `0x711106`, `0x711109`, `0x711135`, `0x7111aa`, `0x7111c7`, `0x707adb`, `0x7101ee`
Confidence: verified
Proof: one
Test: none

`0x70ed7f` stores 1 with `rep stos` over the selected view's `+0x18` words and `0x70edb5` copies
copy source `+0x98` over them; `0x70edce` skips `+0x9c` when the selected view is view 0, else
`0x70edfe` fills it. `0x7110d0` defers itself as kind 1 while unloaded; it reads submesh `+0x0`
(`0x711106`), keeps it when neither `min > id` nor `id > max` (`ja`, `0x711109`), and writes the
loop index, not the id (`0x711135`, `0x7111aa`); only the first loop sets the changed flag that
calls `0x711880` (`0x7111c7`). The live element path tests `+0x98` (`0x707adb`); `0x711230` tests it
while building. The census of `e8`/`e9` targets and stored dwords for `0x7110d0` finds five calls,
`0x7101ee` in the replay and four in CharacterComponent.cpp's code, and no stored address.

## Own batches

`BuildBatches` gives an instance its own batch table when merging shrinks it: walking the selected
view's texture units in order, it keeps each visible one and drops one that matches the last kept
one in colour index, material, texture combination, transparency, texture transform and the
submesh's first bone lookup. Only when some unit was dropped does it build the table, with its own
index buffer and shader ids; otherwise the instance keeps none. Building first frees the old table
and clears the instance's batchable flag `+0xb8`.

Addresses: `0x711230`, `0x711293`, `0x71129b`, `0x711302`, `0x711323`, `0x711371`, `0x711780`, `0x711880`, `0x719930`
Confidence: verified
Proof: one
Test: none

`0x711230` defers itself as kind 2 while unloaded, calls `0x711880` (`0x711293`) and stores 0 at
`+0xb8` (`0x71129b`). Its counting pass tests visibility (`0x711302`) and calls `0x711780`
(`0x711323`), which compares unit words `+0x8`, `+0xa`, `+0x10`, `+0x14`, `+0x16` and submesh
`+0xe`; `0x711371` returns when the kept count is not below the visible count. The table is `+0x3ec`
(24-byte units), `+0x3f4` (32-byte submeshes), the index buffer `+0x400`/`+0x404` and the shader ids
`+0x408`/`+0x40c`; `0x711880` frees them all. The element path takes `+0x3ec` when `+0x404` is set.

## Batchable flag

An instance's batches can join a batched run (`scene`, kind 2) only while its flag `+0xb8` is set.
Initialization sets it when the instance was created without flag 2, its shared model allows more
than one instance per batched draw (`+0x160` above 1), and its copy source, if any, has it set.
Building own batches and binding a replaceable texture clear it; nothing sets it again.

Addresses: `0x7100b5`, `0x7100be`, `0x7100ce`, `0x7100e6`, `0x71129b`, `0x710ff1`, `0x707c11`
Confidence: verified
Proof: one
Test: none

`0x7100b5` tests `+0x4` with 2, `0x7100be` compares shared `+0x160` with 1 (`jbe`), `0x7100ce` reads
the copy source's `+0xb8`, and `0x7100e6` stores the result. Of the stores to `+0xb8` in
M2Model.cpp's code the fourth, `0x712423`, writes a bone state; `0x707c11` reads the flag on the
element path.

## Replaceable textures

`SetReplaceableTexture(type, texture)` binds the texture to every model texture of that replaceable
type, to the ribbons, and to each particle emitter whose texture is of that type; it marks the
textures not loaded when the new texture is not, and clears the batchable flag `+0xb8`.
`GetReplaceableTexture` returns the texture of an id through the header's replaceable lookup.
Callers bind only the types 1, 2, 6, 8, 11, 12, 13 and 14; a texture of any other type keeps the
handle initialization gave it, from its file name or 0.

Addresses: `0x710ec0`, `0x710e50`, `0x7b7950`, `0x7b4d20`, `0x710fa1`, `0x710ff1`
Confidence: verified
Proof: one
Test: none

The texture records are 16 bytes at header `+0x60` with the type at `+0x0`; each match closes
`+0xa4[i]` and stores the new handle with a reference. Ribbons get `0x7b7950(ribbon, type,
texture)`; an emitter whose record's texture index `+0x16` names a texture of the type gets
`0x7b4d20` with that texture (`0x710fa1`). `0x710ff1` stores 0 at `+0xb8`. `0x710e50` looks the id
up in header `+0x7c` with `0x710310`. A census of `e8` calls to `0x710ec0` finds eleven and no
stored address: the replay of deferred kind 0, and ten in other systems' code pushing 1, 8, 6, 1, 2,
2, 2, 14 and, in two loops of three, 11 to 13.

## Attachments

`Attach(parent, id)` takes an instance off the animate list, stores the parent and id, resolves
the id to the parent's attachment index (0xffff while the parent loads) and links it first among
the parent's children. An attached instance animates through its parent. `GetAttachmentPosition`
transforms the attachment's offset by its bone matrix and the scene matrix.

Addresses: `0x712f70`, `0x713020`, `0x713070`, `0x7130a0`, `0x712cb0`, `0x712cf0`, `0x712d50`, `0x712de0`, `0x712f00`
Confidence: verified
Proof: one
Test: `models::attach_fold_712de0_difftest`

Attachments are 48-byte records at header `+0x108`, counted at `+0x104`, with the bone at `+0x4` and the offset at
`+0x8`, looked up through header `+0x10c`. `0x712f70` calls `0x710b90(0)` and stores `+0x1cc`,
`+0x1d0`, `+0x1d4`, `+0x1d8 = 1`. `0x712de0` adds `R·offset` to the bone matrix's translation and
multiplies by the scene matrix. `0x713020` unlinks, clears the parent and sets the id to −1.

Of `0x712de0`, `attach_fold_712de0_difftest` runs `0x712e49`–`0x712eac`.

## Events

`HasEvent(id)` searches the header's 44-byte events for an id. `GetEventPosition` transforms an
event's offset by its bone and the scene matrix, or returns zero for a missing id.

Addresses: `0x7130e0`, `0x7131b0`, `0x713260`, `0x713130`, `0x7133a0`
Confidence: verified
Proof: one
Test: none

The events are at header `+0x118` with count `+0x114`: id `+0x0`, bone `+0x8`, offset `+0xc`.
`0x7131b0` runs `0x714000` before reading the bone matrix. `0x7133a0` stores the event callback
and its arguments at `+0x1f4`, `+0x1f8`, `+0x1fc`.

## Bounds

`GetBoundingSphere` returns the centre of the header's bounding box and its radius: x and y are
`(min + max)·0.5` from a sum at the FPU's 53-bit precision, z rounds the sum to a float first. The radius is
header `+0xcc`. `GetBoundingBox` and `GetCollisionBox` copy the six floats at `+0xb4` and `+0xd0`.

Addresses: `0x713680`, `0x713640`, `0x713700`, `0x713740`
Confidence: verified
Proof: one
Test: `models::bbox_center_difftest`

`0x713680`: `fld +0xb4; fadd +0xc0` and `fld +0xc4; fadd +0xb8` stay on the stack, `fld +0xc8;
fadd +0xbc; fstp` rounds, then each is multiplied by 0.5 and stored. The radius is `+0xcc`.

Of `0x713680`, `bbox_center_difftest` runs `0x7136a0`–`0x7136fd`.

## Collision facets

`GatherCollisionFacets(box, matrix, facets)` transforms the header's collision vertices by a
matrix once into a cached array, gives each an outcode against the box, and emits each collision
triangle whose vertices do not share an outside side.

Addresses: `0x7137c0`
Confidence: inferred
Proof: none
Test: `models::shadow_normalize_axes_difftest`, `models::shadow_transform_normal_difftest`

The caches are `+0x3e4` (vertices) and `+0x3e8` (outcodes), allocated on first use. The outcode
bits are 1 and 2 for x, 4 and 8 for y, 0x10 and 0x20 for z.

Of `0x7137c0`, `shadow_normalize_axes_difftest` runs `0x713996`–`0x713a5b`,
`shadow_transform_normal_difftest` `0x713bbd`–`0x713c64`.

## Hit-test mode

`SetHitTestMode(mode, a, b)` links the instance into the scene's hit-test list when `mode` is
nonzero and stores the mode and its two arguments.

Addresses: `0x713cb0`, `0x713c80`
Confidence: verified
Proof: one
Test: none

`0x713cb0` links through `+0x414`/`+0x418` into scene `+0x130` and stores `+0x410`, `+0x420`,
`+0x41c`.
