# ui: draw order

The full order in which the interface draws and updates: the intrusive lists that hold a level's
frames, the five layer batches each level owns, what each batch holds and in what order it is
drained, and the frames that draw themselves. `strata` has the strata and levels, the raise and the
update pass; this has what decides which of two overlapping regions is on top.

A level is a node of 0x11c bytes in its stratum's array. It lists its visible frames twice, in
list A while an update walk is in flight and list B otherwise, and owns five 0x30-byte batches, one
for each draw layer, that every frame of the level fills. The draw pass of `CSimpleTop` rebuilds the
dirty batches and submits them, strata 0 to 8, levels in ascending order, layers 0 to 4.

Roots: `0x7657d0`, `0x765920`, `0x76fb00`, `0x7731a0`, `0x7731d0`, `0x76b3a0`, `0x76b3f0`, `0x773110`, `0x7668c0`

## Level lists

A list is a header of three dwords: the link offset, the terminator's next pointer (the head, header+4)
and its previous pointer (the tail, header+8, tagged). A member's link is the next link and the previous link.
A low bit of 1 marks the terminator. A walk starts at the tail and follows previous links, `[link+4]`. The insert takes
a mode and a reference; a null reference means the terminator, so mode 2 puts a member at the head and mode 1 at the
tail. A frame is linked at the head of its level. Every list in these docs uses these names.

Addresses: `0x7668c0`, `0x7668d3`, `0x76691e`, `0x76691f`, `0x766921`, `0x766937`, `0x766880`, `0x7669a0`, `0x766aa0`, `0x764780`, `0x7648b3`, `0x7648c9`, `0x76a4b6`, `0x76a54c`, `0x76ae85`, `0x76ae10`, `0x76a470`, `0x76a4f0`
Confidence: verified
Proof: one
Test: `ui::set_frame_strata`, `ui::set_frame_level`

`0x7668c0(list, object, mode, reference)` first unlinks the object, then loads the reference's link
or `ecx+4` (`0x766918`), and `dec esi; je 0x766937` takes the before-path for mode 1 only. `0x764780`
inserts into list B with mode 2 and reference 0 (`0x7648c9`) and, while the update walk holds a cursor on the level,
into list A by an inline head insert (`0x7648b3`). Its three callers are `0x76a4b6`, `0x76a54c` and `0x76ae85`: a
stratum change, a level change that changes the level, and a show. A frame's links are `+0x30c`
(next) and `+0x310` (previous), zeroed by the constructor.

`ui::set_frame_strata` and `ui::set_frame_level` run `0x76a470` and `0x76a4f0` whole, with the strata join `0x764780` and leave `0x764920` stubbed to log the frame.

## Update and draw walk

Both walks start at the tail of list B, the oldest link, and follow the previous links toward the
head, so within a level the frame linked last is visited last and is drawn on top. The update walk
runs each frame's update slot; frames linked during it wait in list A, which is spliced onto the head
of list B after the level.

Addresses: `0x765650`, `0x7656e5`, `0x7656eb`, `0x7656f7`, `0x765707`, `0x765710`, `0x76572d`, `0x76573e`, `0x765920`, `0x7659bb`, `0x7659d9`, `0x7659ed`, `0x7659f5`, `0x764920`, `0x76498f`
Confidence: verified
Proof: one
Test: `ui::render_traverse_order`

`0x765650` loads `[node+0x14]`, list B's last link, reads the next frame from `[frame+0x310]` into
the node's cursor `+0x18` before calling slot 0x38 (`0x765707` to `0x765710`), and the removal `0x764920`
moves the cursor when the frame it holds leaves (`0x76498f`). The splice at `0x76573e` makes list A's links the head
of B and empties A. `0x765920` loads the same `[node+0x14]` (`0x7659bb`) and steps with `[link+4]` (`0x7659f5`).
Linking at the head and walking from the tail make the order by link time, oldest first: every show,
stratum change and level change that changes the level relinks a frame, so the key is the time of
the last link, not creation.

`ui::render_traverse_order` runs `0x765650` whole, with each frame's update slot stubbed to log the frame, and reads the order.

## Layer batches

The draw pass rebuilds a level when its strata record asks, and only the layers whose bit is set in
the level's mask `+0x10c`. A rebuild empties the batch, asks every frame of the level without the
self-drawing bit for its regions of that layer in list order, sorts the batch, and then, whether
rebuilt or not, relinks every non-empty batch of the level at the head of the level's batch list in
layer order. The batches are submitted from the tail, so layers go out 0 to 4 whatever was
rebuilt.

Addresses: `0x7657d0`, `0x765879`, `0x7658ad`, `0x7658c6`, `0x7658e2`, `0x7658ef`, `0x765920`, `0x765929`, `0x765939`, `0x7659b0`, `0x7659b6`, `0x7659fc`, `0x765a01`, `0x765a06`, `0x765a47`, `0x765a6a`, `0x7731d0`, `0x7731a0`, `0x7731c0`, `0x73f727`, `0x76fb00`
Confidence: verified
Proof: one
Test: none

`0x765920` returns at once for a zero mask (`0x765931`), loops the layer counter to 5 (`0x765a6a`) over
the batch at `node+0x1c+0x30·layer`, and for a set bit calls `0x7731d0`, the frame walk with slot
0x44 (`0x7659ed`) and `0x7731a0`. `0x7731d0` zeroes the quad count, frees the text sub-batch and
empties the callback list. `0x7731a0` runs `qsort` (`0x73f727`) over the quad array, records of 0x1c bytes,
with the comparator `0x7731c0`, which subtracts the first dwords. `0x765a01` onward unlinks and head-inserts each
batch with a nonzero count (`0x765a47`). `0x7657d0` submits the level's batches from `[node+0x118]`
(`0x7658c6`) with `0x76fb00`, levels ascending.

## Batch construction

A layer batch is 0x30 bytes. `0x772e80` constructs one: it zeroes the counts and the array, makes `+0x20` an empty callback list (its
head points at itself, `+0x24` holds that address tagged with bit 0) and stores the vtable `0x81c7ac`, whose slot 0 is `0x772ec0`. The quad
array is four dwords at `+0x8`: capacity, count `+0xc`, records `+0x10` and a growth step `+0x14`. `0x773280(extra, round)` grows it to hold
`extra` more records, and `0x772ef0` destroys a batch: it empties it with `0x7731d0`, unlinks the callback list head and frees the records.

Addresses: `0x772e80`, `0x772eaa`, `0x772ea1`, `0x772ec0`, `0x772ef0`, `0x772efb`, `0x772fbf`, `0x772fd0`, `0x773002`, `0x773280`, `0x7732a8`, `0x7732ec`, `0x7732ff`, `0x773314`, `0x773346`, `0x77335b`, `0x7731d0`, `0x81c7ac`, `0x878e7c`
Confidence: verified
Proof: one
Test: none

`0x772fd0` calls `0x773280(extra, 1)` when the count plus one passes the capacity (`0x773002`). `0x773280` returns at once when the new size does not exceed `[this]` (`0x773294`); with a nonzero second argument it rounds the size up to a multiple of the step, which is 9 once the size is 9 or more, else the largest power of two not above it. The step test is `cmp esi,9` (`0x7732a8`); `0x7732b2` to
`0x7732bd` clear the lowest set bit until one remains; the division by the step is `0x7732db` and the size is raised by `step − remainder`. The record size is `imul 0x1c` (`0x7732ec`); the realloc is `0x7732ff`, the fallback
allocation `0x773314`, the copy `rep movs` of 7 dwords per record (`0x773346`) and the old block freed at `0x77335b`.

## What a batch drains

A batch holds three arrays and drains them in this order: first the quads of every texture in it,
sorted by texture handle, then the text of every font string, in the order the frames were walked,
then the registered render callbacks. So within one level and layer every texture draws before any
text and the textures' order is by handle, not by frame or declaration; only text keeps the frame
order and, within a frame, the order the regions were last registered.

Addresses: `0x76fb00`, `0x76fb0b`, `0x76fb13`, `0x76fb20`, `0x76fb75`, `0x76fbdb`, `0x76fc03`, `0x76fc08`, `0x76fc0f`, `0x76fc14`, `0x76fc2e`, `0x773080`, `0x7730e5`, `0x7730f2`, `0x772fd0`, `0x773110`, `0x772e50`, `0x7706e0`, `0x5c1d60`, `0x5c1e60`, `0x5c1ef0`
Confidence: verified
Proof: one
Test: none

`0x76fb00` skips the quad loop for a zero count (`0x76fb13`), binds the texture only when the
handle changes (`0x76fb75`), and after the bracket `0x76fc03` drains the text sub-batch `[batch+0x18]` with
`0x5c1ef0` (`0x76fc0f`) and then calls each node of the callback list `[batch+0x24]` (`0x76fc2e`). A
texture appends a quad with `0x772fd0` through its emit `0x7706e0`; a font string appends its text with
`0x773080`, which creates the sub-batch with `0x5c1d60` and adds with `0x5c1e60`. A model frame
registers its scene draw as a callback of layer 2, and the world frame registers the 3D world as
one on layer 0 (`worldframe`). `qsort` is not stable, so equal handles may swap.

## Regions within a frame

A frame keeps a list of regions for each draw layer. A region is linked at the head when it is registered
and drawn from the tail, so regions of one layer come out in the order of their last registration, oldest first,
not the order of declaration. Hiding and showing a texture or font string, or giving it a new draw layer,
unregisters and registers it, so it moves to the end of its layer and draws last. There is no finer sort.
A frame's layers are enabled by five switches, set for layers 0 to 3 by the constructor and clear for
HIGHLIGHT; a region on a disabled layer is not drawn.

Addresses: `0x77fc60`, `0x77fc95`, `0x77fcb0`, `0x76a910`, `0x76feab`, `0x76febc`, `0x770ff0`, `0x771001`, `0x76b3a0`, `0x76b3a6`, `0x76b3ad`, `0x76b3b6`, `0x76b3db`, `0x76b3e1`, `0x76b3e4`, `0x76a860`, `0x76a88a`, `0x76a8f3`, `0x77fcf9`, `0x769322`, `0x76932e`, `0x76933e`, `0x811a80`, `0x811a84`, `0x6f18b0`, `0x6f1900`
Confidence: verified
Proof: one
Test: none

`0x76b3a0(batch, layer)` reads the switch at `+0x198 + 4·layer` (`0x76b3a6`), returns for zero, and walks
the layer's list at `+0x1c0 + 12·layer` from the last link, calling region slot 0x28 (`0x76b3e1`). The constructor stores 1 at
`+0x198` to `+0x1a4` and 0 at `+0x1a8` (`0x769322` to `0x76933e`). `0x77fcb0` registers a region: it links with `0x76a860` at `0x77fcf9`, only if `[region+0xc4]` is nonzero, the owning frame
`[region+0x9c]` has `[frame+0xd4]` nonzero and the region is not registered already (`[region+0xc8]`). `0x77fc60` unregisters it
with `0x76a910` (`0x77fc95`). The show and hide of a texture are `0x76feab` and `0x76febc`, of a font string `0x770ff0` and `0x771001`.
`0x76a860(region, layer)` takes two
arguments and its insert has no comparator; its only caller is `0x77fcf9`, and `0x76a910(region, layer)` is its inverse. The layer names are the pairs at
`0x811a80`: BACKGROUND 0, BORDER 1, ARTWORK 2, OVERLAY 3, HIGHLIGHT 4.

## Frames that draw themselves

A frame whose flags hold `0x2000` is left out of the level batches. The ScrollFrame sets it on its
scroll child and on all of the child's descendants, and sets `0x4000` on the child alone. Such a tree
is drawn by `0x76b3f0`: it rebuilds the frame's own five batches, submits them, then calls itself
through slot 0x40 on each visible child in link order that does not hold `0x4000`.

Addresses: `0x76b3f0`, `0x76b3f8`, `0x76b459`, `0x76b4d2`, `0x76b4dd`, `0x76b4ee`, `0x76b509`, `0x76b513`, `0x76a400`, `0x76a410`, `0x76a422`, `0x786ce0`, `0x786cf4`, `0x786d0a`, `0x78754c`, `0x76ab10`, `0x7659d9`
Confidence: verified
Proof: one
Test: none

`0x76a400(a, b)` sets the bit `0x2000` to `a` (`0x76a410`) and, for `b ≥ 0`, `0x4000` to `b` (`0x76a422`), marks the
five layers dirty when `a` is set and recurses with `(a, −1)` over the children. `0x786ce0` calls it with
`(0, 0)` on the old scroll child and `(1, 1)` on the new. The ScrollFrame's draw calls the child's
slot 0x40 at `0x78754c`. `0x76b3f0` uses the frame's own mask `+0x1f8` and batches from `+0x1fc`, and
reads the children from `[frame+0x300]`, a tail-to-head walk (previous links) in creation order, the walk `GetChildren` makes. `SetParent` passes the
parent's `0x2000` to a new child (`0x76ab10`).
