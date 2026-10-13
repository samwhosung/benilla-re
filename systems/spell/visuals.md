# spell: visuals

Area shard emitters, chain beams, the screen tint, camera-shake groups and the precast kit.

`SpellVisuals.cpp` (`0x6eb650` to `0x6ed410`) draws the spell effects that are not a kit model on
a unit: the shard emitters of ground spells (the image's `BlizzardObject`, spawning `Shard`s), the
chain and beam effects (`LightningObject`, holding `LightningObject::Bolt`s), a full-screen colour
tint, and a kit's camera-shake group. It also chooses a spell's `SpellVisual` record and plays the
precast kit when a cast starts. Its state is two intrusive lists (`0xcee96c` emitters, `0xcee94c`
chains), two pooled allocators (`0xcee928` emitters, `0xcee8f0` shards), the shared lightning
renderer `0xcee948`, the `spellEffectLevel` CVar pointer `0xcee9a8` and the tint state `0xcee8d4`.
The client's startup and context stop create and destroy the state; the world frame updates the
lists and renders the beams; the dynamic-object code starts and stops emitters; the unit's kit
code creates chains, tints and shakes; the spell code plays the precast kit. Times are unsigned
milliseconds compared unsigned, unless a float is named.

Roots: `0x6ec0e0`, `0x6ec130`, `0x6ec1e0`, `0x6ec910`, `0x6eca20`, `0x6ecb20`, `0x6ecb40`, `0x6ecb90`, `0x6ece10`, `0x6ece30`, `0x6ecf20`, the stored callbacks `0x6eb840`, `0x6ebe30`, `0x6ebe70`, and the `_atexit` destructors `0x6eb710`, `0x6eb7f0`, `0x6eb830`, `0x6ec040`, `0x6ec410`, `0x6ec450`

## Startup and shutdown

At startup `0x6ec0e0` allocates the 32-byte lightning renderer, constructs it with `0x7b0760`
and stores it in `0xcee948` (0 when the allocation fails), then stores the `spellEffectLevel`
CVar's record in `0xcee9a8`. At context stop `0x6ec130`, in order: destroys and frees every
`LightningObject` whatever its reference count; destroys and frees the renderer; sets the
last-update time `0x870e20` to -1; destroys every `BlizzardObject` and returns it to its pool;
frees both pools' blocks; clears `0xcee9a8`. It leaves `0xcee948` holding the freed pointer.

Addresses: `0x6ec0e0`, `0x6ec130`, `0x402ad0`, `0x4036c0`, `0x7b0760`, `0x7b0780`, `0x63de30`, `0x760400`, `0xcee948`, `0xcee9a8`, `0x870e20`
Confidence: verified
Proof: one
Test: none

`0x6ec0e0` is `SMemAlloc(0x20, "…SpellVisuals.cpp", 0x152, 0)`, `0x7b0760` on a non-null result,
and `CVar::Lookup` (`0x63de30`) of `spellEffectLevel`; its one caller is `0x402b74` in `0x402ad0`.
`0x6ec130`'s one caller is `0x4036c5` in `0x4036c0`. It walks `0xcee954` calling `0x6ecf40` and
`SMemFree` (tag `.?AULightningObject@@`, line -2); calls `0x7b0780` and `SMemFree` (tag
`delete`, line -1) on `0xcee948`; stores -1 at `0x870e20` (`0x6ec178`); walks `0xcee974` calling
`0x6eb850` and `0x7604f0` on `0xcee928`; calls `0x760400` on `0xcee928` and `0xcee8f0` with
their type tags; stores 0 at `0xcee9a8` (`0x6ec1cf`). Nothing there writes `0xcee948`.

## Static lists and pools

Four static initializers build the emitter list `0xcee96c` and the chain list `0xcee94c`, each a
link offset of 0 and a terminator link whose previous-link pointer is itself and whose next is its
own address with bit 0 set, and the emitter pool `0xcee928` (0x144-byte elements, 8 per block) and
shard pool `0xcee8f0` (0x20 bytes, 32 per block). Each registers a destructor with `_atexit`: a
list's unlinks every node, leaving its memory, then the terminator; a pool's calls `0x7603f0`.
`0x6ec410` and `0x6ec450`, registered for two initializers that zero a dword, return.

Addresses: `0x6eb6d0`, `0x6eb710`, `0x6eb7c0`, `0x6eb7f0`, `0x6eb810`, `0x6eb830`, `0x6ec000`, `0x6ec040`, `0x6ec410`, `0x6ec450`, `0x6ec400`, `0x6ec440`, `0x760390`, `0x7603f0`
Confidence: verified
Proof: one
Test: none

`0x6eb6d0` stores 0 at `0xcee96c`, `0xcee970` at `0xcee970` and `0xcee970|1` at `0xcee974`;
`0x6ec000` does the same at `0xcee94c`. `0x760390(size, count)` stores `max(size, 4)` and
`max(count, 1)` and zeroes the live count, block list and free list; `0x6eb7c0` passes
`(0x144, 8)`, `0x6eb810` `(0x20, 0x20)`. `0x6eb710` and `0x6ec040` unlink from the first node
until the terminator's next is marked, then unlink the terminator. `0x6eb7f0` and `0x6eb830` load
the pool into `ecx` and jump to `0x7603f0`. `0x6ec410` and `0x6ec450` are a lone `ret`, pushed to
`_atexit` by `0x6ec400` and `0x6ec440`.

## Frame update

The world frame's `0x483460` calls `0x6ecb20` and then `0x6eca20`. `0x6ecb20` renders the beams
from the world camera's position. `0x6eca20` reads the clock once as `now`, then: updates every
chain, unlinking and releasing each whose update returns 0; updates the renderer with
`dt = (int32)(now − last) · 0.001f`, 0 when `last` is -1, and stores `now` as `last`; updates the
tint; updates every emitter, destroying each whose update returns 0 and returning it to its pool.

Addresses: `0x6eca20`, `0x6ecb20`, `0x483460`, `0x870e20`, `0x6eca51`, `0x6eca83`, `0x6ecaa7`, `0x6ecac3`, `0x6ecace`, `0x6ecaf2`, `0x7b0a40`, `0x7b0a80`, `0x4818f0`
Confidence: verified
Proof: one
Test: none

The calls are `0x48372c` and `0x48376a`. `0x6ecb20` is `0x7b0a80(0xcee948, 0x4818f0() + 8)`. In
`0x6eca20` each node's next is read before its update; `0x6eca51` is `0x6ec460(now)`, and a zero
`al` unlinks through `0x6ed410` and calls `0x6ed030` (`0x6eca83`). `0x6ecaa7` is `fild` of the
signed difference, `fmul` 0.001 (`0x3a83126f`), `fstp` a float, passed to `0x7b0a40` at
`0x6ecac3`. `0x6ecace` is `0x6ec870(now)` on `0xcee8d4`. `0x6ecaf2` is `0x6ebad0`; on zero,
`0x6eb850` and `0x7604f0` on `0xcee928`.

## Area shard emitter creation

A dynamic object starts an emitter from the first kit procedure of kind 9 in its spell's visual
kit: `0x6ece30(position, radius, type, rate)`, with its radius, `type` the low byte of
`bits(f32(param0 + 512.0)) >> 14` and `rate` the raw `param1`. `0x6ece30` takes an emitter from
the pool, zeroes it, links it at the tail and initializes it with the path `0x870e24[type]`,
unbounded: types 0 to 6 are `Spells\` `Blizzard_Impact_Base`, `RainOfFire_Impact_Base`,
`CallLightning_Impact`, `FlamestrikeSmall_Impact_Base`, `DeathAndDecay_Area_Base`,
`ArcaneShot_Area`, `StarShards_Impact_Base` (`.mdx`); type 7 reads the first path's bytes as a
pointer. A failed allocation is not tested. The dynamic object's stop calls `0x6ecf20`, which sets
the rate to 0.0 and clears its handle.

Addresses: `0x5d55c0`, `0x5d5640`, `0x5d5665`, `0x5d5690`, `0x5d5700`, `0x6ece30`, `0x6ecef4`, `0x6ecf20`, `0x870e24`, `0x760450`
Confidence: verified
Proof: one
Test: none

`0x5d5640` scans the kinds at `+0x3c` for 9; `0x5d5665` adds 512.0 to the float at `+0x4c` and
`0x5d567a` shifts its bits; `+0x5c` and the radius are pushed raw; `ecx` is the vtable `+0x14`
position. The result goes to `+0x198`, which `0x5d5713` hands to `0x6ecf20`. In `0x6ece30`,
`0x6ece4b` is `0x760450(0, ".?AUBlizzardObject@@", -2)`; `0x6ece56` to `0x6ecead` zero every
dword of `+0x00` to `+0x138` but the path, `+0x11c` and `+0x124`, and make `+0x138` an empty list;
`0x6ecee2` to `0x6ecf03` link at the tail; `0x6ecef4` loads `[type·4 + 0x870e24]` with no bound.
The seven dwords point at the seven strings after them. `edx` is not read. `0x6ecf20` stores 0 at
`[[ecx]+0x124]` and at `[ecx]`.

## Emitter initialization

`0x6eb930(position, path, radius, rate)` copies the position to `+0x110`, the radius to `+0x11c`
and the path (at most 0x104 bytes) to `+0x08`, and stores at `+0x124` the rate scaled by the
`spellEffectLevel` record's integer at `+0x28`: ×0.33 at 0, ×0.66 at 1, unchanged otherwise or
with no record. It creates a world object with the draw callback `0x6eb840` and the emitter as
argument, stores it at `+0x10c`, and sets its transform to a translation to the position and its
box to `(−r, −r, −r)` to `(r, r, r)` with centre `0.5·((−r) + r)` per axis.

Addresses: `0x6eb930`, `0x6eb95e`, `0x6eb967`, `0x6eb981`, `0x81199c`, `0x6eb989`, `0x6eb98f`, `0x6eb99f`, `0x6eba20`, `0x6ebab8`, `0x670db0`, `0x6717d0`, `0x64a5a0`, `0x5f8cf0`
Confidence: verified
Proof: one
Test: `terrain::camera_set_difftest`

`0x6eb95e` loads `0xcee9a8`; null goes to `0x6eb989` (`fld` the rate alone). `0x6eb967` reads
`[record+0x28]`: 0 multiplies by 0.33 (`0x3ea8f5c3`), 1 by 0.66 (`0x3f28f5c3`, the constant `0x81199c` that `0x6eb981` multiplies by), at 53 bits;
`0x6eb98f` stores a float. The value is read once, here. `0x6eb99f` is `SStrCopy(+8, path,
0x104)`. `0x6eba20` calls `0x670db0` with `ecx` 0, `edx` `0x6eb840` and the stack
`(this, 0, 0, 0, 0xe)`. The matrix at `[ebp-0x7c]` is identity with the position as its fourth
row. `0x6eba2b` negates the radius and `0x6eba46` to `0x6eba78` add the radius back to each copy
before `0x5f8cf0` scales by 0.5, so the centre is 0, or NaN for an infinite or NaN radius.
`0x6ebab8` is `0x6717d0(+0x10c, matrix, box, centre, 1)`.

`terrain::camera_set_difftest` runs six stretches from `0x6717da` to `0x6719f8`.

## Shard emission rate

`0x6ebad0` adds `frameSeconds · rate` to the accumulator `+0x120` and, while the sum is at least
1.0, spawns a shard and subtracts 1.0. It returns 1 while the rate is not 0.0 (NaN counts as not
0.0) or the shard list is not empty, else 0. A NaN accumulator never spawns again.

Addresses: `0x6ebad0`, `0x6ebad6`, `0x6ebaee`, `0x6ebaf4`, `0x6ebafc`, `0x6ebdbb`, `0x6ebdc9`, `0x6ebdda`, `0x6ebde2`, `0x6ebde7`, `0xc62510`, `0x66fcc0`
Confidence: verified
Proof: one
Test: none

`0x6ebad6` loads the float `0xc62510`, which `0x66fcc0` stores each world update, `fmul` the rate
and `fadd` the accumulator at 53 bits; `0x6ebaee` is `fst` to `+0x120`, and `0x6ebaf4` compares
the register, not the stored float, with 1.0: `test ah,1; jne` skips when C0 is set, below 1.0
or unordered. The loop tail (`0x6ebdac` to `0x6ebdcc`) is `fld`, `fsub` 1.0, `fst` and the same
compare. `0x6ebdda` compares the rate with 0.0: `test ah,0x44; jp` returns 1 unless only C3 is
set. With a zero rate, `0x6ebde7` reads the shard list's first link `+0x140` and returns 0 when it
is null or marked.

## Shard placement

Each shard draws `u` and then a unit vector `(c, s)` from the client random state `0x882664`;
`p = (x + u·c·r, y + u·s·r, z + 16.666666)` and `q = (x, y, z + 0.16666667)` from the emitter's
position and radius. A trace from `q` to `p` that hits gives `a = q + (p − q)·(0.95·f)`, `f` its
fraction, else `a = p`. A second trace from `a` to `(a.x, a.y, a.z − 33.333332)` that hits writes
the shard's position; a miss puts the shard at the emitter's position. The radial draw is `u`, not
`√u`, so shards gather toward the centre.

Addresses: `0x6ebb64`, `0x6ebb83`, `0x6ebb8b`, `0x6ebbad`, `0x6ebbcd`, `0x6ebbee`, `0x6ebc29`, `0x6ebc64`, `0x6ebcdb`, `0x6ebd04`, `0x6ebd0d`, `0x4531e0`, `0x44fd00`, `0x672170`, `0x69bff0`, `0x882664`
Confidence: verified
Proof: one
Test: none

`0x6ebb64` calls `0x4531e0` on `0x882664`; `and 0x7fffff; or 0x3f800000` makes `m` in `[1, 2)` and
`0x6ebb8b` is `fsub` 1.0 (`d8 25`), so `u = m − 1`. `0x6ebb83` is `0x44fd00` on the same state.
`u·s` is stored as a float at `0x6ebbad` before the multiply by `r`; `u·c·r` stays in the
register. The constants are `0x41855555`, `0x3e2aaaab`, `0x3f733333` and `0x42055555`. `0x672170`
passes its point output, fraction and flags to `0x69bff0` with the start in `ecx` and the end in
`edx`, dropping its first stack argument; both calls pass flags 0x1f0111 and a fraction of 1.0.
The first trace's point output is a stack temporary; the second's is the shard's `+0x08`. Each vector is
rounded to floats before the next step (`0x6ebc59`, `0x6ebc89`).

## Shard model, start and end

Each shard is constructed with its start time unset and gets a model of the emitter's path with the event callback
`0x6ebe30` and the done callback `0x6ebe70`, and the start time `worldTime + 2·b`,
`b = low byte of bits(f32(255·u + 512.0)) >> 14` for a fresh `u`, or 1 when that is 0. The draw
callback skips shards before their start; the first draw starts the model's animation, and every
draw sets the model's translation to the shard. The event callback counts every call in
`0xcee944` and plays the sound only for a `$SND` event on a count divisible by 4. The done
callback, given 0 as its second argument, closes the model and frees the shard.

Addresses: `0x6ebe10`, `0x6ebd2f`, `0x6ebd42`, `0x6ebd54`, `0x6ebd7b`, `0x6ebda2`, `0x6eb840`, `0x6ebee0`, `0x6ebf41`, `0x6ebfd9`, `0x6ebe30`, `0x6ebe70`, `0x6eb850`, `0xcee944`, `0xc7b2c0`, `0xc7b298`, `0x707350`, `0x7133a0`, `0x711bb0`, `0x7121a0`, `0x710c50`, `0x710b90`, `0x710620`, `0x7103a0`, `0x458870`, `0x671ac0`
Confidence: verified
Proof: one
Test: none

`0x6ebd2f` is `0x707350(path, 0)` on `[0xc7b298]`, kept at `+0x14`; `0x6ebd42` stores `0x6ebe30`
with two zeros, `0x6ebd54` stores `0x6ebe70` with the shard as a 64-bit argument. `b` is 0 to 255.
`0x6ebee0` compares `[0xc7b2c0]` with `+0x18` (`jb`), calls `0x7121a0(−1, 0, −1, 0, 1.0, 1, 1)`
once under `+0x1c`, then `0x710c50(1)`, `0x710b90(1)` and `0x710620` with the translation.
`0x6eb840` calls `0x6ebee0` with its `ecx`, returns 1, `ret 0xc`. `0x6ebe30` increments first,
tests `al,3`, compares `0x444e5324` (`$SND`) and calls `0x458870(arg2, arg3, −1, 1.0)`.
`0x6eb850` closes and frees every shard, releases `+0x10c` through `0x671ac0`, and unlinks.

## Chain effect creation

Kit procedure kinds 0 and 12 call `0x6ecb90` with the chain id, strand count and persistent flag
from params 0, 1 and 2 (each `low byte of bits(f32(p + 512.0)) >> 14`, the flag that byte's
being nonzero), the targets and the spell. For each strand up to 3, `0x6ecbd0` allocates a
0x50-byte `LightningObject` with one reference, links it at the tail, and stores the GUIDs,
the end `now + n·life`, the record's fields 1 to 4, `life·0.001f`, its texture, spell and flag. A persistent object gains a reference and fills the owner's next slot. It returns at
once for a null caster or targets, no targets, spell 0, no strands, or a chain id with no record.

Addresses: `0x60d7c0`, `0x60da79`, `0x60db6d`, `0x6ecb90`, `0x6ecbd0`, `0x6ecc0e`, `0x6ecc2e`, `0x6ecc65`, `0x6ecc9b`, `0x6eccd1`, `0x6ecd30`, `0x6ecd6a`, `0x6ecd8b`, `0x6ecdaa`, `0x6ecdcc`, `0x60dc20`, `0x449d90`
Confidence: verified
Proof: one
Test: none

`0x60d7fc` maps kinds 0 and 12 through the byte table `0x60dc20` to `0x60da79`, which decodes
`+0x6c`, `+0x5c` and `+0x4c` from `0x60db19` and passes the owner's `+0x30` slots with capacity 3.
`0x6ecb90` reorders its arguments and calls `0x6ecbd0` unless the caster or count is 0. There,
`0x6ecc0e` to `0x6ecc2e` bound the id against the `SpellChainEffects` store and test the record,
`0x6ecc65` caps the strands at 3, `0x6ecc9b` is `SMemAlloc(0x50, …, −2, 8)` and `0x6eccd1` is
`0x6ed360(node, 2, 0)`. `0x6ecd30` is `count·[rec+0x14] + now`; `0x6ecd6a` is `fild [rec+0x14]`,
`fmul` 0.001. `0x6ecd8b` is `TextureCreate([rec+0x1c])`. The loop leaves when the slot index
reaches the capacity (`0x6ecdcc`), so a capacity of 0 ends it after one strand.

## Chain hop table

`0x6ec980` fills the GUID array as `[caster, t[n−1], …, t[0]]`, reversing the target list, and
gives hop `i` (0 to `n − 1`) the endpoints `i` and `i + 1`, the start `now + i·stagger`, the end
`start + life` and no renderer handle (-1); `stagger` is the chain record's `+0x18` and `life` its
`+0x14`, as 32-bit integers.

Addresses: `0x6ec980`, `0x6ec9a7`, `0x6ec9c0`, `0x6ec9cc`, `0x6ec9da`, `0x6ec9e8`, `0x6ec9eb`, `0x6ec9f4`, `0x6ec9fd`, `0x6ecd25`, `0x605780`
Confidence: verified
Proof: one
Test: none

`0x6ec9a7` points `esi` at `targets + 8n − 8` and the loop steps it back by 8; `0x6ec9cc` stores
it at index `i + 1`. `0x6ec9c0` stores -1 at `+0x0c`. `0x6ec9da` loads `[rec+0x18]`, `imul` by
`i`, adds the clock and stores at `+0x04` (`0x6ec9e8`); `0x6ec9eb` adds `[rec+0x14]` and stores at
`+0x08`. `0x6ec9f4` stores the previous hop's second index as the first, 0 for hop 0, and
`0x6ec9fd` stores `i + 1`. `0x6ecd25` copies the caster's GUID to element 0. The unit's target
array is itself filled from the end of its source list by `0x605780`, so the order is the
source's.

## Chain update per frame

For each hop `0x6ec460` looks up both GUIDs (type mask 1), marking an object a unit when its type
bits have bit 3. A non-persistent hop outside `[start, end)`, or a hop missing either object, which
it hides, goes to the end test. Otherwise it is shown and its ends
placed: hop 0's start by `0x6ec6f0`, every other unit end by `0x6ec780` with the chain's spell, a
non-unit end at its position. A hop without a handle creates its bolt with the record's values and
colour `0xffffffff`; one with a handle moves it. The end test destroys a non-persistent hop's bolt
at or past its end. The result is 0 when not persistent and `now ≥ +0x28`, else 1.

Addresses: `0x6ec460`, `0x6ec4c0`, `0x6ec4e6`, `0x6ec4fd`, `0x6ec520`, `0x6ec52d`, `0x6ec536`, `0x6ec54e`, `0x6ec563`, `0x6ec595`, `0x6ec644`, `0x6ec65f`, `0x6ec678`, `0x6ec69b`, `0x6ec6b9`, `0x468460`, `0x6acdd0`, `0x7b07f0`, `0x7b0950`, `0x7b0ac0`, `0x7b0bd0`
Confidence: verified
Proof: one
Test: none

`0x6ec4c0` and `0x6ec4e6` call `0x468460` with mask 1 and lines 0x1e9 and 0x1ea; `0x6ec4fd`
tests bit 3 of the descriptor's `+0x08`. `0x6ec52d` (`jb`) and `0x6ec536` (`jae`) go to the end
test. `0x6ec54e` is `0x7b0bd0(handle, 0)`, `0x6ec563` `0x7b0bd0(handle, 1)`. `0x6ec595` picks
`0x6ec6f0` for hop 0; the target end always takes `0x6ec780`; a non-unit takes vtable `+0x14`.
`0x6ec644` builds the colour from -1; `0x6ec65f` is `0x7b07f0(A, B, +0x2c, +0x30, colour, +0x34,
+0x38, +0x3c, +0x40, 0, 0)`, `ret 0x2c`, its result kept at `+0x0c`; `0x6ec678` is
`0x7b0950(handle, A, B)`. `0x6ec69b` is `0x7b0ac0` when not persistent, `now ≥ end` and the
handle is not -1.

## Chain endpoints

`0x6ec6f0` gives the caster's end: the position when the unit has no model, the `$CSL` event's
position when the model has it, else the position with `z + f32(+0x90 · +0xbc · 0.75)`, the
product at 53 bits and both results rounded to floats. `0x6ec780` gives a spell's impact end: for
a spell id with no record, attachment 0x22 when the model has it, else the position; otherwise
the `SpellVisual` record's `+0x24` indexes the attachments of `0x860a18` (20, 34, 19, 21, 22, 17,
23, 24, 25, 15, 16), unbounded, and the end is that attachment, else the position. A found spell
never falls back to 0x22.

Addresses: `0x6ec6f0`, `0x6ec6fb`, `0x6ec718`, `0x6ec73e`, `0x6ec758`, `0x6ec761`, `0x6ec780`, `0x6ec7b5`, `0x6ec7fe`, `0x6ec81a`, `0x860a18`, `0x7130e0`, `0x7131b0`, `0x712cb0`, `0x712d50`, `0x60d450`
Confidence: verified
Proof: one
Test: none

The model is `[unit+0xd8]`; `0x6ec6f0` tests it for null, `0x6ec780` does not. `0x6ec718` pushes
`0x4c534324` (`$CSL`) to `0x7130e0` and `0x7131b0`. `0x6ec73e` is `fld +0x90`, `fmul +0xbc`,
`fmul` 0.75 (`0x3f400000`), `fstp` a float (`0x6ec758`); vtable `+0x14` then writes the position
and `0x6ec761` adds the float to its z. The id is bounded against the Spell store; `0x6ec7b5`
pushes 0x22 to `0x712cb0` and `0x712d50`. `0x6ec7fe` picks the record through `0x60d450` for a
unit and `0x6ec1e0` otherwise; `0x6ec81a` loads `[[record+0x24]·4 + 0x860a18]` with no bound. The
table's twelfth dword is a pointer.

## Spell visual record choice

`0x6ec1e0` returns a Spell record's `SpellVisual` record: the id at `+0x1cc`, replaced by the one
at `+0x1d0` when the stored `violenceLevel` is below 2 (signed) and that id is nonzero; an id
outside the store gives 0.

Addresses: `0x6ec1e0`, `0x6ec1ea`, `0x6ec1ef`, `0x6ec202`, `0x6c5b50`, `0x6c5af0`, `0xce8534`
Confidence: verified
Proof: one
Test: none

`0x6ec1ea` calls `0x6c5b50`, which returns the dword at `0xce8534`, written by the `violenceLevel` CVar
callback `0x6c5af0` (`tables/cvars.tsv`) with a clamp. `0x6ec1ef` is `cmp eax,2; jge`. `0x6ec202`
and the next compare bound the id against the `SpellVisual` store, signed, and the id map gives
the record. The function has twelve call sites, nine outside spell.

## Chain lifetime

A `LightningObject` starts with one reference, the list's; a persistent one gains a second for
its owner. The update's zero unlinks it and drops the list's. The owner's teardown calls
`0x6ece10` on each of its three slots and clears them: the chain becomes non-persistent with its
end set to `now`, and drops the owner's reference. At zero references `0x6ecf40` destroys every
bolt, frees both arrays, closes the texture and unlinks, and the memory is freed.

Addresses: `0x6ed020`, `0x6ed030`, `0x6ece10`, `0x6ecf40`, `0x6ed060`, `0x6ed000`, `0x6208b0`, `0x6135f0`, `0x41aed0`
Confidence: verified
Proof: one
Test: none

`0x6ed020` is `inc [ecx+0x4c]`. `0x6ed030` decrements and at zero calls `0x6ecf40` and
`SMemFree(this, "delete", −1, 0)`. `0x6ece10` stores the clock at `+0x28` and 0 at `+0x48` and
tail-jumps to `0x6ed030`; its one caller, `0x6208c6` in `0x6208b0`, then zeroes the slot.
`0x6ecf40` calls `0x7b0ac0` for each handle that is not -1, then `0x6ed060` and `0x6135f0`, which
free the hop and GUID data, zeroing each array, and `0x41aed0` on `+0x40`; its later frees of both
arrays' data find null pointers. The unlink runs from `0x6ecfda`; `0x6ed000` falls inside it.

## Screen tint

Kit procedure kind 6 calls `0x6ec910(colour, spellId, fraction)` with `ftol(param0) | 0xff000000`
and `param1`. With `d` the spell's cast time from `0x6e3340`, it stores the colour and
`t0 = now`, `t1 = now + ftol(d·fraction)`, `t2 = now + d`, `t3 = now + d + 100`. Each frame
`0x6ec870` passes the colour and a level to `0x6d45c0`: `(now − t0)/(t1 − t0)` before `t1`, 1.0
before `t2`, `1 − (now − t2)/(t3 − t2)` before `t3`; after that 0.0 once, clearing `t0` and `t3`.
Nothing is passed while `t0 = t3`. `0x6d45c0` stores the colour and the low byte of
`bits(f32(255·level + 512.0)) >> 14`.

Addresses: `0x60da55`, `0x6ec910`, `0x6ec922`, `0x6ec94a`, `0x6ec870`, `0x6ec882`, `0x6ec89f`, `0x6ec8aa`, `0x6ec8de`, `0x6ec8e9`, `0x6ec8fe`, `0x6d45c0`, `0x597ac0`, `0x6e3340`, `0x40a2b0`, `0xcee8d4`
Confidence: verified
Proof: one
Test: none

`0x60d7fc` maps kind 6 to `0x60da55`. `0x6ec922` is `0x6e3340(spellId, 0, 0)`; `0x6ec94a` is
`fild` of `d` as an unsigned 64-bit integer, `fmul` the float fraction, `__ftol`. `0x6ec89f` is
`fild` of `now − t0` zero-extended to 64 bits and `0x6ec8aa` `fidiv` the signed 32-bit `t1 − t0`;
`0x6ec8de` is `d8 2d`, `fsubr`, so `1.0 − st0`. Each level is stored as a float. The compares
are `jae`, `jb`, `jae` against `t1`, `t2`, `t3`. `0x597ac0` copies the colour into the argument.

## Camera shake groups

`0x6ecb40(groupId, position)` looks up a `SpellEffectCameraShakes` record and, for each of its
three shake ids in order, skips a zero and otherwise calls `0x511d40(id, position)` on the world
camera. An id outside the store or naming no record does nothing. The kit player `0x60edf0` and
the effect object update `0x620be0` call it.

Addresses: `0x6ecb40`, `0x6ecb4d`, `0x6ecb65`, `0x6ecb74`, `0x60edf0`, `0x620be0`, `0x511d40`
Confidence: verified
Proof: one
Test: none

`0x6ecb4d` bounds `ecx` signed against the store; the loop runs three times from `record+4`
(`0x6ecb65`), pushing the position and id before `0x4818f0` and calling `0x511d40` on its result
(`0x6ecb74`). `0x60f4e6` in `0x60edf0` passes `[esi+0x38]` when positive, with the position from
vtable `+0x14` or a supplied one; `0x620e11` in `0x620be0` passes `[[this+0x1c]+0x38]` when above
0 and not -1, with `this+0x48`.

## Precast visual at cast start

`0x6ec220(spellId, targets, unit)` does nothing for a null or non-unit object. It calls the
combat-log function `0x62b1b0`; prints `NOSPELLIDFOUND|%d` and returns for an unknown spell;
picks the visual through `0x60d450`, printing `SPELLVISUALIDNOTFOUND|%d` and returning with none;
records the spell on the unit through `0x60d020`; plays the visual's `+0x04` kit through
`0x60edf0(spell, kit, 4, 0)` when it exists. For the active player, spell attribute bit 0x400000
and a unit target (flag 2 at `targets+0x14`, a nonzero GUID at `+0x18`), it calls `0x60fed0`
with that GUID.

Addresses: `0x6ec220`, `0x6ec242`, `0x6ec254`, `0x6ec27f`, `0x6ec29f`, `0x6ec2a8`, `0x6ec2b4`, `0x6ec2cf`, `0x6ec2fb`, `0x6ec30e`, `0x6ec332`, `0x62b1b0`, `0x44cbd0`, `0x60d020`, `0x5fa6d0`, `0x60fed0`, `0x6e54f0`, `0x6e7700`, `0x6e7640`
Confidence: verified
Proof: one
Test: none

Callers: `0x6e5898` in `0x6e54f0` (topic `cast`), for the active player when the spell differs
from `[unit+0xc8c]`; `0x6e7908` and `0x6e79be` in `0x6e7700`, which `0x6e7640` runs for
`SMSG_SPELL_START` (`0x131`), unless bit 0 of its flags byte `[ebp-0x6]` is set. The messages go
through `SysMsgPrintf(1, 2, …)`. `0x6ec2a8` pushes the spell id before the null test; it is
`0x60d020`'s argument, which that function stores at `+0xc8c` while setting `0x400` in `+0xd58`.
`0x6ec2cf` is that function's only call site, so every spell with a visual sets the bit.

## List and array helpers

A link is two dwords at the node's start: the previous link's address, and the next node or the
terminator's address with bit 0 set. Unlinking repairs both neighbours and zeroes the pair. A
chain's GUID and hop arrays hold capacity, count, data and a growth quantum. Growing to `m` with
rounding rounds `m` up to the quantum: the stored one, or for 0 the largest power of two not above
`m` when `m` is below 32 (16 for hops), else 32 (16), which is stored. The data is reallocated in
place, or allocated, copied and the old block freed.

Addresses: `0x6ed080`, `0x6ed2d0`, `0x6ed2a0`, `0x6ed310`, `0x6ed410`, `0x6ed340`, `0x6ed360`, `0x6ece00`, `0x6ed050`, `0x6ed0c0`, `0x6ed1b0`, `0x646320`
Confidence: verified
Proof: one
Test: none

`0x6ed080` and `0x6ed2d0` are the same unlink. `0x6ed2a0`, `0x6ed310` and `0x6ed410` return the
link after a link, from the link offset or -1 to derive it. `0x6ed340` returns a node, or the
terminator for null. `0x6ed360(node, where, relative)` relinks the node after the relative (or
terminator) for `where` 1, else before it. `0x6ece00` and `0x6ed050` zero two and four dwords.
`0x6ed0c0` (8-byte elements, tag `._K`) and `0x6ed1b0` (16-byte, the `Bolt` tag) call
`SMemReAlloc(…, 0x10)` and on null allocate, copy `min(m, count)` elements and free the old block.

## Unreached functions

`0x6ec350`, which would end a unit's precast visual (clearing its recorded spell through
`0x60d040`, then `0x614150`, `0x611590` and `0x60fb60`), is reached by nothing: no call, jump or
stored address targets it, and `0x6ec220` above it ends in `ret 4` and padding.

Addresses: `0x6ec350`, `0x6ec39e`, `0x60d040`, `0x614150`, `0x611590`, `0x60fb60`, `0x60fc50`, `0x6ea080`, `0x64e850`
Confidence: verified
Proof: one
Test: none

A scan of `.text` for `e8` and `e9` targeting `0x6ec350` and of every section for the dword
`0x006ec350` find nothing; the same scans find `0x6ec220`'s three calls and `0x6eb840`'s stored
address at `0x6eb9bc`. `0x6ec39e` would call `0x60d040(spell, 1, 1)`, which clears `+0xc8c` and
the bit only when they hold that spell; nine live call sites reach `0x60d040`. Its other paths
call `0x64e850(0x57)` for a null unit, `0x6ea080` and `0x60fc50`.
