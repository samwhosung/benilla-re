# models: the scene

A scene: the set of model instances one view draws, its clock, its animate list, and the per-frame
pass that animates the instances and sorts what they draw into three lists.

M2Scene.cpp runs from `0x706f10` through `0x70dc40`. A scene is a 0x1d6c-byte object the cache
allocates (`cache`); the world frame, the glue screens and the interface's model frames each hold
one. Each frame the owner advances the scene's clock (`0x7074b0`), runs the collect pass
(`0x707680`) and draws the lists (`0x708900`); the draw itself is `render`. An instance is a
`CM2Model` (`model`), its file data a shared model (`shared`).

Roots: `0x707320`, `0x707350`, `0x707400`, `0x707480`, `0x707490`, `0x7074b0`, `0x7075f0`, `0x707600`, `0x707680`, `0x7089b0`, `0x7089c0`, `0x7099e0`, `0x70a060`, `0x70a0f0`, `0x70a230`, `0x70a250`, `0x70a5a0`, `0x70aa00`, `0x70aa30`, `0x70ae10`, `0x70b350`, `0x70dbf0`

## Scene object

The constructor stores the cache at `+0x4`, sets the reference count `+0x0` to 1, the view
`+0x9c` and the scene matrix `+0xdc` to identity, and the 1800-entry shader id memo at `+0x14c` to
−1. `0x707320` decrements the count and destroys and frees the scene at zero; the destructor
releases every instance on the model list `+0x8`, then frees the element, sort and callback arrays.

| offset | holds |
|---|---|
| `+0x00c` | clock, milliseconds |
| `+0x010` | frame counter |
| `+0x018`, `+0x01c` | global light list, 64×64 light grid |
| `+0x020` | animate list (link `+0x44`/`+0x48` in the instance) |
| `+0x024`, `+0x028` | draw queue, emitter queue (links `+0x54`/`+0x58`, `+0x3dc`/`+0x3e0`) |
| `+0x02c` to `+0x034` | element array (64-byte `M2Element`) |
| `+0x03c`, `+0x04c`, `+0x05c`, `+0x06c` | grouped list, list 0, list 1, list 2 (index arrays) |
| `+0x07c` to `+0x088` | queued callbacks (48-byte records) |
| `+0x11c`, `+0x120` | special-batch callback and its argument |

Addresses: `0x706ff0`, `0x7071a0`, `0x707320`, `0x707310`, `0x70a230`
Confidence: verified
Proof: one
Test: none

The identity stores are 1.0 at `+0x9c`, `+0xb0`, `+0xc4`, `+0xd8` and at `+0xdc`, `+0xf0`,
`+0x104`, `+0x118`; `0x706ff0` fills 0x708 dwords from `+0x14c` with −1. The destructor loops
`0x7103a0` on the head of `+0x8` until the list is empty. The list offsets are the links each
writer uses: `0x710b90` for `+0x20`, `0x707680` for `+0x24` and `+0x28`, `0x70a280` for `+0x7c`.

## Model creation

`0x707350(path, flags)` asks the cache for the file's shared model with `flags & 4` as the open flag,
which lets the file and its textures come from loose files on disk (`storm/mount#which-opens-ask-the-index`). When that fails it loads `Spells\ErrorCube.mdx` with open flag 0 instead. It allocates a
0x428-byte instance, constructs it and initializes it on the scene; a failed initialization
destroys the instance and returns 0. The lookup's own reference is released either way.
`0x707400(source, flags)` does the same with the source instance's shared model, and the new
instance copies the source's attachments.

Addresses: `0x707350`, `0x707400`, `0x706a50`, `0x70dd70`, `0x70ea00`, `0x70e170`, `0x71d480`
Confidence: verified
Proof: one
Test: none

The allocation passes line 0x92 in `0x707350` and 0xaf in `0x707400`. The fallback pushes the
string `Spells\ErrorCube.mdx` and 0 for the open flag before a second `0x706a50`.
`0x70ea00` receives the scene, the shared model, the source (0 from a path) and the flags.
`0x71d480` runs on the shared model before the return.

## Scene advance

`0x7074b0(dt)` does nothing for `dt` 0. Otherwise it adds `dt` to the clock, evicting released
shared models, runs `CM2Model::Update(dt)` on every instance of the animate list, then runs the
callbacks the updates queued. The queue is sorted by each record's time key, largest first, ties in
queue order; a kind-0 record calls `fn(ecx=model, edx=a, b, c, d, e, f)`, a kind-1 record passes a
copy of its position before the last three arguments. Each record's instance is then released and
the queue emptied.

Addresses: `0x7074b0`, `0x707490`, `0x719370`, `0x71f860`, `0x70a5a0`, `0x70a280`, `0x70a400`, `0x483659`, `0x483679`, `0x76d846`
Confidence: verified
Proof: one
Test: `ui::textglue_482_unit_convert`

The world frame computes `dt` as `ftol(f32[ui+0x3f4] · 1000.0 + 0.5)` (`0x483659` to `0x483679`), rounding to the nearest millisecond; the model frame path truncates (`ui/model`, `0x76d846`). A delta that converts to a whole millisecond less a rounding error comes back one short on the truncating path and exact on the world path; how many do depends on the x87 precision the frame runs at. The sort fills an index array with 0 to n−1 and calls `0x71f860` with the comparator `0x70a5a0`,
which returns −1 when record i's `+0x24` is greater (unsigned), 1 when smaller, then compares the
indices. Records are 0x30 bytes at `+0x84`; their kind is `+0x0`, the function `+0x4`, the
instance `+0x8`. `0x70a280` and `0x70a400` add a reference to the instance when they queue it, and
cap the queue at 0x10000 records.

Bit-exact coverage: `ui::textglue_482_unit_convert` covers `0x483659` to `0x483671` of `0x483460`.

## View rebase

The collect pass increments the frame counter and clears the shader id memo when the cache's mode
changed. It reads the device view into `+0x9c` and moves its translation so the view is relative to
the origin `p` the caller passes: `m30 −= (m10·py + m20·pz) + m00·px`, `m31 −= (m11·py + m01·px)
+ m21·pz`, `m32 −= (m12·py + m02·px) + m22·pz`, each sum at the FPU's 53-bit precision (control
word 0x023F) and each result rounded to a float. It then sets the scene matrix `+0xdc` with the
dispatched routine `0x74ad38(+0xdc, 0, view)`, which has a matrix inverse's arguments and whose
result is not transcribed (`animation`, `Dispatched matrix math`).

Addresses: `0x707680`, `0x707696`, `0x7076a2`, `0x7076f8`, `0x707722`, `0x70774c`, `0x707752`, `0x74ad38`
Confidence: verified
Proof: one
Test: `models::negate_difftest`, `models::plane_dist_difftest`, `models::radius_difftest`, `models::scale_mul_d1_difftest`, `models::scale_mul_f_difftest`, `models::sum_of_squares_difftest`, `models::sum_of_squares_e_difftest`, `models::vec_add_difftest`, `models::view_rebase_difftest`

`m[r][c]` is `+0x9c + 16r + 4c`. At `0x7076cb`: `fld +0xac; fmul p.y; fld +0xbc; fmul p.z; faddp;
fld +0x9c; fmul p.x; faddp; fsubr +0xcc; fstp +0xcc`. The y row loads `+0xb0`·p.y, `+0xa0`·p.x,
then `+0xc0`·p.z; the z row `+0xb4`, `+0xa4`, `+0xc4`. The memo reset at `0x7076a2` fills 0x708
dwords with −1 and copies the mode to `+0x148`.

Of `0x707680`, `view_rebase_difftest` runs `0x7076cb`–`0x707752`, `sum_of_squares_difftest`
`0x7078e5`–`0x707944`, `vec_add_difftest` `0x707915`–`0x707944`, `radius_difftest`
`0x707959`–`0x707971`, `plane_dist_difftest` `0x707981`–`0x7084db`, `negate_difftest`
`0x7079c3`–`0x7079d1`, `scale_mul_d1_difftest` `0x707afc`–`0x707b17`, `sum_of_squares_e_difftest`
`0x707d85`–`0x707da6`, `scale_mul_f_difftest` `0x7080e5`–`0x7080fc`.

## Scene origin

The world scene `0xc7b298` is rebased on the camera position. The frame passes the address of the
camera's `+0x8` triple to `0x707680`; every other caller passes a zero triple, so a model frame stays
camera-local. The device view it reads has no translation: `0x50ab70` builds it with the eye at
(0, 0, 0). The scene view is therefore the rotation with the translation row `−p·R`, each component one
float store, and each instance's root matrix is its placement (the absolute map position in row 3)
times that matrix through `0x74a7c0`. No double is stored on the path.

Addresses: `0x483460`, `0x48367e`, `0x483684`, `0x483687`, `0x48368a`, `0x707680`, `0x7076c6`, `0x50ab70`, `0x50acc6`, `0x50acf2`, `0x50acfa`, `0x5c3e70`, `0x5c40eb`, `0x7bdc40`, `0x714260`, `0x707789`, `0x7077b6`, `0x71438d`, `0x71439b`, `0x74a7c0`, `0x754a66`, `0x59961c`, `0x599627`, `0x404100`, `0xc7b298`
Confidence: verified
Proof: one
Test: `ui::glue_50a_50ab70_proj_aspect`

`0x48367e` loads the camera from `[world frame + 0x65b8]` and `0x483684` adds 8. The other eleven
callers (`0x46c4c6`, `0x50596a`, `0x50c7ad`, `0x50f0df`, `0x52557a`, `0x525640`, `0x6d4b4f`,
`0x6d4c1a`, `0x76ce39`, `0x76d619`, `0x7e619b`) store zero floats before the call, or for
`0x525640` and `0x76d619` earlier in the function (`0x525518`, `0x76d3da`). `0x50acc6` to `0x50acd4`
zero the eye vector passed to `0x5c3e70` (`0x50acf2`), whose translation `−eye` goes through `0x7bdc40` (`0x5c40eb`). `0x714260` gets `&[scene + 0x9c]` (`0x707789`, call
`0x7077b6`) and composes `0x74a7c0(out, placement, view)` (`0x71439b`). With the SSE2 body `0x754a66`
each product of the translation row rounds to a float before the sums. The device is created with flags
`0x22` or `0x52` (`0x59961c`, `0x599627`), both with the floating-point-preserve bit, so the control word
set at `0x404100` holds; no instruction loads the SSE control register.

## Animate pass

Every unattached instance on the animate list is animated with the view as its parent matrix,
white colour multipliers, black emissive and alpha 1.0. With the helper thread granted, the thread
animates the odd positions of the list while the caller animates the even ones, and the caller
waits for it. Then each instance on the list runs `UpdateAfterAnimate`, and the list is drained
with each instance's lighting update.

Addresses: `0x707680`, `0x707600`, `0x706cd0`, `0x706d00`, `0x714260`, `0x718960`, `0x7192b0`, `0x707761`, `0x7077bb`
Confidence: verified
Proof: one
Test: none

`0x707761` submits `0x707600` with the scene. The caller's loop at `0x707772` skips an instance
whose `+0x1cc` is set and steps two links (`0x7077bb`, `0x7077c2`); `0x707600` starts one link
in and steps two. The arguments of `0x714260` are the view, (1,1,1), (0,0,0) and 1.0. The drain pops
the head, clears its `+0x44` and `+0x48`, and calls `0x7192b0`.

## Element collection

Each ready instance on the draw queue gives one 64-byte element per visible batch and ribbon and one
for its custom draw, each on the emitter queue one per particle emitter. A batch's alpha is the
instance alpha, times its colour's animated alpha when its colour index is in range, times its
transparency weight when it has textures; alpha ≤ 0 skips it, a NaN does not. A batch, ribbon or
emitter of blend 0 or 1 and alpha at least 0.99999 goes to list 0, as does an opaque custom draw;
otherwise a batch goes to list 1 above the clip plane, list 2 below, possibly both; the rest to list
1 if above, else 2.

| kind | element |
|---|---|
| 0 | a batch |
| 1 | a batch with flag 4, when the scene has a special callback; translucent, list 1, or list 2 under a plane |
| 2 | an opaque batch of a batchable instance, flag 0x10, cache bit 0x20, no lights: the grouped list |
| 3 | a ribbon emitter whose visible byte is set, alpha the instance's times its animated alpha |
| 4 | a particle emitter that still draws, of an instance with alpha above 0 |
| 5 | the instance's custom draw, alpha 1.0 |

Addresses: `0x707680`, `0x710450`, `0x808120`, `0x70da00`, `0x707b48`, `0x707b5c`, `0x707af0`, `0x707b12`, `0x707b26`, `0x707e31`, `0x7080cf`, `0x7080e5`, `0x708516`, `0x708530`, `0x7084cf`
Confidence: verified
Proof: one
Test: `models::plane_dist_difftest`, `models::scale_mul_d1_difftest`, `models::scale_mul_f_difftest`

The ready test is `0x710450(model, 0, 0)`. The colour alpha is the colour state's `+0x3c`
(`0x707af0`); the weight, read when unit `+0xe` is nonzero (`0x707b12`), is transparency state
`+0xc` at the header's transparency lookup `+0xa8` entry for unit `+0x14` (`0x707b26`). The opaque
test compares the material's blend word with 1 and the alpha with the float at `0x808120`. A kind 1
element reads the lighting record's plane flag `+0x19c` (`0x707e31`). Kind 3 walks the shared
header's `+0x134` ribbons with state stride 0xd0, tests the state's byte `+0xbc` (`0x7080cf`), and
multiplies by state `+0x3c` when the record's `+0x4c` count is nonzero, clamping a negative alpha to
0 (`0x7080e5`); its material is the record's first entry of `+0x20`. Kind 4 walks the emitter
queue's `+0x13c` emitters with stride 0x16c and needs the state's `+0x164` (`0x708516`) and an alpha
above 0 (`0x708530`); its material is the record's blend `+0x28`. An emitter is above unless its
centre lies more than `r` below the plane (`0x7084cf`), whatever the cache's bit 2. Kind 5 needs a
draw callback at instance `+0x1b8` and goes to list 0 when `+0x1c0` is set.

Of `0x707680`, `plane_dist_difftest` runs `0x707981`–`0x7084db`, `scale_mul_d1_difftest`
`0x707afc`–`0x707b17`, `scale_mul_f_difftest` `0x7080e5`–`0x7080fc`.

## Element depth keys

Each element carries two depth keys for the transparent order, `+0x14` and `+0x18`. The instance key
`+0x84` is the squared length of its world matrix's translation row, computed each frame, or for an
instance attached to a parent created without flag 1 the parent's key. A translucent batch sets
`+0x18` to the squared length of its submesh's centre `+0x14` taken through the bone matrix of its
centre bone `+0x12`, an opaque one to the instance key; `+0x14` is the instance key when depth fill
applies to the instance and the batch is not kind 1, else `+0x18`. Ribbon, emitter and custom
elements carry the instance key in both.

Addresses: `0x7143b8`, `0x7143e2`, `0x707d6a`, `0x707d80`, `0x707d9d`, `0x707da8`, `0x707dbf`, `0x707dca`, `0x708184`, `0x7085c9`, `0x70830b`
Confidence: verified
Proof: one
Test: `models::sum_of_squares_e_difftest`

`0x7143c0` loads instance `+0x12c` to `+0x134` (row 3 of `+0xfc`) and stores `(x·x + y·y) + z·z` at
`0x7143e2`; `0x7143ac` tests the parent's `+0x4` with 1 and copies its `+0x84` when clear. The batch
key transforms submesh `+0x14` by bone matrix `+0x94[+0x12]` with `TransformPoint` (`0x707d80`) and
sums the squares in the same order (`0x707d9d`), each sum at 53 bits and the result stored as a
float. `0x707db1` tests the instance's depth fill and the kind 1 flag before `0x707dbf` or
`0x707dca`.

Of `0x707680`, `sum_of_squares_e_difftest` runs `0x707d85`–`0x707da6`.

## Depth fill copies

When depth fill applies to an instance, every translucent batch other than kind 1 whose material
lacks flag 0x10 gets a second element: its `+0x18` becomes the largest float, and a copy with `+0x8`
= 1 goes to the same lists. Depth fill applies when the cache's feature bit 1 is granted, the
instance was created without flag 1, and its `+0x1c4` is set, which the constructor sets and no live
code clears. The copy draws with colour writes off and blending off, keeping the alpha test of blend
1 (`render`, `Material state`), and the transparent order puts it before every element of equal
`+0x14`.

Addresses: `0x707a3c`, `0x707a4e`, `0x707f89`, `0x707f9a`, `0x707fa7`, `0x707fb3`, `0x707ff0`, `0x707ff7`, `0x710da0`
Confidence: verified
Proof: one
Test: none

`0x707a3c` tests the cache's `+0x4` with 1, the instance's `+0x4` with 1 and `+0x1c4` (`0x707a4e`)
into `[ebp-0x8]`. After the batch's list append, `0x707f89` needs that flag, `0x707f8f` a clear kind
1 flag, `0x707f9a` the translucent flag and `0x707fa7` material bit 0x10 clear. `0x707fb3` stores
0x7f7fffff in the element's `+0x18` before `rep movs` copies it (`0x707ff0`), so both carry it;
`0x707ff7` sets the copy's `+0x8`. Its only setter, `0x710da0`, has no caller (`overview`, Unreached
code).

## Plane classification

When the instance's lighting record has a clip plane, its bounding sphere is the shared header's
box centre transformed by the instance matrix, with radius `sqrt(row0·row0)` times the header
radius. With `d` the signed distance of the centre, bit 2 of the cache's feature mask `+0x4` puts
the instance above unless `d < −r` and below when `d ≤ r`, so a NaN `d` is neither; without it,
above unless `d < 0`, and below otherwise, so a NaN `d` is below. Without a plane every instance is
above.

Addresses: `0x707680`, `0x7079bf`, `0x7079ca`, `0x7079da`, `0x7079ed`
Confidence: verified
Proof: one
Test: `models::negate_difftest`, `models::plane_dist_difftest`

The centre is `0.5·(min + max)` of the header box at `+0xb4`/`+0xc0`, each sum rounded to a float,
then `TransformPoint` by the instance matrix `+0xfc`. The dot product is `(x·x + y·y) + z·z` of
`+0xfc..+0x104`, stored as a float before `fsqrt`. The distance is `((cz·nz + cy·ny) + cx·nx) + d`
with the plane at lighting `+0x1a0`. `0x7079bf` tests the cache's `+0x4` with 2; `fcompp` at
`0x7079ca` clears "above" on C0, which a NaN sets, and `fcomp r` at `0x7079da` sets "below" when
exactly one of C0 and C3 is set.

Of `0x707680`, `plane_dist_difftest` runs `0x707981`–`0x7084db`, `negate_difftest`
`0x7079c3`–`0x7079d1`.

## Batch grouping

Before sorting, the grouped list (kind 2 elements) is grouped by inputs: a 251-bucket hash of the
shared model, batch index, fog range, fog colour, spherical harmonic terms, colour and the instance
colours finds each element's first equal element, the list is sorted by that representative, and
each run of equal elements becomes one element with the run length, drawn as one batched run.
Lists 0, 1 and 2 are then sorted: list 0 by the opaque order, lists 1 and 2 by the transparent
order.

Addresses: `0x70a600`, `0x70a710`, `0x70aa00`, `0x70aa30`, `0x70ae10`, `0xcefff8`, `0x873184`
Confidence: verified
Proof: one
Test: none

`0x70a600` folds `h = h·19 + x` over the dwords listed with `imul` and `add`; `0x70a710` compares
the same fields. The buckets at `0xcefff8` start at −1 and probe linearly. The orders are
`Element sort orders`.

## Element sort orders

Each order compares two elements key by key, returning at the first difference. Unsigned unless
marked; `mode` is the scene's copy `+0x148` of the cache mode.

| order | keys |
|---|---|
| transparent | under mode bit 4, the priority of the kind in `0x873184`; depth `+0x14` descending; `+0x8` descending (signed); `+0x28` ascending (signed); under mode bit 2, `+0x38`; depth `+0x18` descending; instance `+0x4`; kind (signed); for kinds 0 to 2, the batch's `+0xc` word; then the opaque order |
| opaque, all kinds | kind (signed) |
| opaque, kind 0 | under mode bit 1, vertex shader `+0x38`; the shared header; `+0x34` (signed); instance `+0x4`; the submesh's `+0xe`; vertex shader `+0x38`; pixel shader `+0x3c` |
| opaque, kinds 0 and 1 | material flags word; blend word; each of the first `min(nA, nB)` textures by handle; texture count `+0xe`; each of the first `min(nA, nB)` texture unit entries; the batch pointer |
| opaque, kind 3 | each of the first `min` ribbon textures by handle; the ribbon's texture count; the ribbon index `+0x1c` |
| opaque, kind 4 | the emitter index `+0x1c` |

Addresses: `0x70ae10`, `0x70aa30`, `0x873184`, `0x41af70`
Confidence: verified
Proof: one
Test: none

The float keys compare with `fcomp` and `test ah,0x41` (greater returns −1) then `test ah,5` (less
returns 1), so a NaN depth compares equal. Kind 1 skips the kind-0 keys (`jne 0x70ab7d`). A batch's
material is header `+0x88` at batch `+0xa`; its textures are instance `+0xa4` at the header lookup
`+0x98` from batch `+0x10`, 0 past header `+0x5c`, compared with `0x41af70`; texture units are the
header lookup `+0xa0` from batch `+0x12`. Kind 3 reads the ribbon records `+0x138` (0xdc bytes):
count `+0x14`, texture indices `+0x18`. Other kinds compare equal.

## Vertex shader id remap

A vertex shader id is seven mixed-radix digits (`shared`, Vertex shader id), most significant first
with radices 5, 2, 2, 3, 3, 5 after a leading digit. When the cache mode has bit 8 or 0x10, the
scene rewrites digit 1, the submesh's bone-weight count: to 4 under bit 8; under 0x10, to 4 when it
was nonzero and 0 otherwise. The result is memoized per id.

Addresses: `0x70aff0`, `0x70b040`
Confidence: verified
Proof: one
Test: none

`0x70aff0` stores `id mod 5`, then `mod 3` twice, `& 1` twice, `mod 5` and the quotient into
digits 6 to 0. `0x70b040` returns the id unchanged unless `+0x148 & 0x18`, reads the memo
`+0x14c[id]`, and recomposes `(((((5·d0 + c)·2 + d2)·2 + d3)·3 + d4)·3 + d5)·5 + d6`.

## Ray hits

`0x7089c0(p0, p1, &f, retry)` finds the nearest triangle of the hit-test instances on the segment
within `f`, writes its fraction and returns the instance's user value, else 0. Candidates are
that entry's spheres placed by the world matrix, which an unanimated mode 3 instance without a
parent rebuilds; other unanimated instances are skipped. Mode 3 tests the collision triangles.
Modes 1 and 2 skin each visible submesh of view 0 on the CPU and test its triangles, skipping
units with flag 8, alpha 0 and, for mode 2, blends other than 0. With nothing hit and
`retry` set, a second pass covers the whole segment with fattened meshes, and a higher priority
beats a nearer hit.

Addresses: `0x7089c0`, `0x708bd9`, `0x708c11`, `0x708ff2`, `0x70902f`, `0x70903b`, `0x70905d`, `0x7090c2`, `0x709229`, `0x70998d`, `0x7093bd`, `0x70942d`, `0x708fc3`, `0x70960c`, `0x7089b0`, `0x713cb0`
Confidence: verified
Proof: one
Test: `models::aabb_corner_sum_difftest`, `models::alpha_scale_difftest`, `models::hit_ray_setup_difftest`, `models::hit_sphere_difftest`, `models::hit_sphere_paths_difftest`, `models::hit_tbest_init_difftest`, `models::hit_world_distance_difftest`, `models::mat_x_dir_difftest`, `models::scale_t_dot_difftest`, `models::skin_accum_difftest`, `models::skin_first_difftest`, `models::tri_predicate_difftest`, `models::tri_predicate_paths_difftest`, `models::tworld_recover_difftest`

Short input returns 0 as in `Segment collision`, and every call empties the hit list. `0x708bd9` compares instance `+0x40` with scene `+0x10`; `0x708c11` builds `+0xfc` from `+0xbc`
and scene `+0x9c`. The candidate loop stops at the first near distance not below the best on the
first pass only (`0x708fa3`). `0x70902f` tests unit flag 8, `0x70903b` mode 2's blend word,
`0x70905d` the visibility words `+0x9c`, else `+0x98`; `0x7090c2` skips alpha 0. `0x709229` scales
each bone weight byte by 1/255 for a weighted sum of bone matrices. `0x70998d` reruns with the
full length when `retry` is set. The pass counter `[ebp-0x48]` is 1 in the second pass: `0x7093bd`
adds the vertex normal (`+0x14` to `+0x1c`), turned by the rotation rows of the blended matrix and
unscaled, to the skinned position (`0x70942d` to `0x709442`); `0x708fc3` skips a candidate of lower
priority than the best hit and takes a higher one, and `0x70960c` does the same per triangle, an equal
priority needing the nearer hit. The inline triangle test was not transcribed.

Of `0x7089c0`, `hit_ray_setup_difftest` runs `0x708a17`–`0x708ab6`, `hit_sphere_difftest`
`0x708a1a`–`0x709929`, `aabb_corner_sum_difftest` `0x708c4f`–`0x708cf8`, `hit_sphere_paths_difftest`
`0x708d4c`–`0x708e96`, `scale_t_dot_difftest` `0x708eba`–`0x708f5d`, `hit_tbest_init_difftest`
`0x708f5f`–`0x708f7c`, `alpha_scale_difftest` `0x709099`–`0x7090cd`, `skin_first_difftest`
`0x709224`–`0x7092b4`, `skin_accum_difftest` `0x7092d0`–`0x7093a6`, `mat_x_dir_difftest`
`0x7093c8`–`0x7094be`, `tworld_recover_difftest` `0x709445`–`0x7097f8`, `tri_predicate_difftest`
`0x70951f`–`0x709929`, `tri_predicate_paths_difftest` `0x70951f`–`0x70996b`,
`hit_world_distance_difftest` `0x7099c4`–`0x7099db`.

## Segment collision

`0x7099e0(p0, p1, &f)` finds the nearest collision triangle of the scene's hit-test instances on
the segment within the fraction `f`, writes the hit's fraction to `f` and returns the instance's
user value `+0x41c`, else 0. A segment or `f` under 1e-5 returns 0. Each listed, loaded instance is
first a sphere: its current sequence's bounds, or the model's when that radius is 0, through the
base matrix; the sphere must meet the line with its near point before the segment end and its far
point not behind the start. Candidates sorted by near then far distance are tested while their
near distance is under the best hit, and only mode 3 instances test triangles. Every call empties
the hit list.

Addresses: `0x7099e0`, `0x709a00`, `0x709bc4`, `0x709da8`, `0x709e64`, `0x709ec1`, `0x709fc5`, `0x70a060`, `0x7c29f0`, `0x7bcf90`, `0x7bd6c0`
Confidence: verified
Proof: one
Test: `models::half_scale_difftest`, `models::segment_ray_setup_difftest`, `models::segment_sphere_difftest`, `models::segment_sphere_paths_difftest`, `models::segment_tbest_init_difftest`, `models::segment_world_distance_difftest`, `models::tri_invray_difftest`, `models::tri_t_world_difftest`, `cmath::det4`, `cmath::mat4_inverse_from_det`

The sphere centre is half of box min plus max of sequence `+0x24` (index from bone state `+0xa4`),
radius `+0x3c`, else header `+0xb4` and `+0xcc`; the radius scales by the length of base row 0.
`0x709da8` misses when the squared distance from the line exceeds r². `0x70a060` breaks ties by
index. `0x709ec1` skips modes other than 3; the segment goes into model space through the inverse
base matrix and `0x7c29f0` tests the header's collision triangles (`+0xf0`, `+0xf8`) with
tolerance 1e-6, keeping hits with t ≥ 0.

Of `0x7099e0`, `segment_ray_setup_difftest` runs `0x709a38`–`0x709ad7`, `half_scale_difftest`
`0x709c22`–`0x709cc0`, `segment_sphere_difftest` `0x709cf2`–`0x709e00`,
`segment_sphere_paths_difftest` `0x709cf2`–`0x709e40`, `segment_tbest_init_difftest`
`0x709e7e`–`0x709e90`, `tri_invray_difftest` `0x709f1a`–`0x709faa`, `tri_t_world_difftest`
`0x709fde`–`0x709fef`, `segment_world_distance_difftest` `0x70a037`–`0x70a04f`.

`cmath::det4` runs `0x7bcf90` whole and `cmath::mat4_inverse_from_det` runs `0x7bd6c0` whole.

## Ray triangle test

The triangle test is two-sided Möller–Trumbore: with `e1 = v1 − v0`, `e2 = v2 − v0` and
`p = d × e2`, it misses when `|e1·p| < 1e-6`; with `s = o − v0` and `q = s × e1`,
`u = (s·p)/det` and `v = (d·q)/det` must each be at least `−ε`, and `u` and `u + v` at most
`1 + ε`; `t = (e2·q)/det` and `(u, v)` are written when asked. It does not bound `t`.

Addresses: `0x7c29f0`, `0x7c2adc`, `0x7c2b55`, `0x7c2bc8`, `0x81d9bc`, `0x80e2e4`
Confidence: verified
Proof: one
Test: none

`0x7c29f0(ray, vertices, indices, &t, &uv, ε)` reads the ray's origin at `+0x0` and direction at
`+0xc`. `0x7c2adc` passes on `det ≤ −1e-6` or `det ≥ 1e-6` (`0x81d9bc`, `0x80e2e4`).
`0x7c2b55` multiplies by `1/det` and compares with `−ε` and `1 + ε`. Map object leaves
(`0x6b8acf`, `0x6bc764`) call it too.
