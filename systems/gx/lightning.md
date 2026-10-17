# gx: lightning

`Lightning.cpp` (`0x7af5c0` through `0x7b0d90`, between the particle registries of `models` and the particle emitters) draws chain-lightning bolts: a jagged polyline between two points, swept into a camera-facing ribbon and drawn additively with a scrolling texture. A renderer object holds the bolts and hands out integer handles; the spell visuals (`spell/visuals`) create, move, show, update and render them. The file's static initializers are `0x7af990` (+∞), `0x7b0c10` (2π), `0x7b0c30` (1/2π) and `0x7af5c0`, which seeds its random generator at `0xcf57b0` with 0 once at start-up.

Roots: `0x7b0760`, `0x7b0780`, `0x7b07f0`, `0x7b0950`, `0x7b0a40`, `0x7b0a80`, `0x7b0ac0`, `0x7b0bd0`, `0x7af5c0`

## Static initializers

The object file's run in the initializer table is `0x7af990` (copies +∞ from `0x81d814` to `0xcf57ac`), `0x7b0c10` (`2π` to `0xcf5850`), `0x7b0c30` (`1 / 2π` to `0xcf57b8`) and `0x7af5c0`, which seeds the generator and registers an empty exit hook with `_atexit`. The render function lazily builds its four static objects on first use, each with an empty exit hook: the identity matrix `0xcf5768` (hook `0x7b06f0`), the saved view `0xcf57c0` (`0x7b06e0`), the camera-relative view `0xcf5800` (`0x7b06d0`) and the normal `(0, 0, 1)` at `0xcf5840` (`0x7b06c0`). The static centre line has its own hook, `0x7afc90`.

Addresses: `0x7af5c0`, `0x7af5d0`, `0x7af5e0`, `0x7af990`, `0x7b0c10`, `0x7b0c30`, `0xcf5768`, `0xcf57c0`, `0xcf5800`, `0x7b06c0`, `0x7b06d0`, `0x7b06e0`, `0x7b06f0`, `0x7afc90`, `0x7af5f0`, `0x7af9a0`, `0x7b0c20`, `0x7b0c40`
Confidence: verified
Proof: one
Test: none

The guard bits are the bytes at `0xcf57a8` (bits 0 to 3 for the four objects) and `0xcf584c` (the static array). The generator's seed call is `NTempest::CRndSeed::SetSeed(0xcf57b0, 0)` at `0x7af5d0`.

## Renderer and bolt objects

The renderer is 32 bytes: a bolt-pointer array (`+4` count, `+8` data) and a free-handle array (`+0x14` count, `+0x18` data). A bolt is `0x74` bytes: `+0` start, `+0xc` end, `+0x18` segment length, `+0x1c` width, `+0x20` colour (alpha at `+0x23`), `+0x24` jitter scale, `+0x28` period in seconds, `+0x2c` a float nothing reads, `+0x30`, `+0x3c`, `+0x48` and `+0x54` four arrays (below), `+0x60` the clock, `+0x64` the texture handle, `+0x68` and `+0x6c` a callback and its argument, `+0x70` flags: bit 0 rebuild, bit 1 active. A new bolt has width 1.0 and flags 1, so it is drawn only after `ShowBolt`.

Addresses: `0x7b0760`, `0x7b0780`, `0x7b07f0`, `0x7b0950`, `0x7b09c0`, `0x7b0ac0`, `0x7b0bd0`, `0x7b0700`, `0x7b0730`, `0x7af600`, `0x7af660`, `0x7b0c60`, `0x7b0c00`
Confidence: verified
Proof: one
Test: none

Bit 1 survives handle reuse: `DestroyBolt` leaves the bolt's flags alone and `CreateBolt` on a reused handle only ORs in bit 0, so a shown bolt is drawn again without a `ShowBolt`.

The arrays are the centre-line points (count `+0x34`, data `+0x38`, 12 bytes each), the ribbon vertices (`+0x40`, `+0x44`), the texture coordinates (`+0x4c`, `+0x50`, 8 bytes) and the indices (`+0x58`, `+0x5c`, 2 bytes).

`CreateBolt(start, end, segment, width, colour, jitter, period, extra, texture, callback, argument)` (`0x7b07f0`, `ret 0x2c`) reuses the last handle on the free array, or allocates a bolt (`SMemAlloc(0x74, "…Lightning.cpp", 0x139)`) and appends it; it stores the arguments, sets the rebuild bit, takes a reference on the texture handle (`0x7b0700`, releasing the old one) and installs the callback (`0x7b09c0`). It returns the handle, the array index. `MoveBolt(handle, start, end)` writes the non-null points and sets the rebuild bit (error `0x57` if both are null). `ShowBolt(handle, on)` sets or clears flag bit 1 for a live handle. `DestroyBolt(handle)` sets bit 0 of the array entry and pushes the handle on the free array; the bolt memory stays for reuse, and `Update` and `Render` skip entries with bit 0 set.

## Bolt shape

`0x7af6d0(points)` fills `points` with the bolt's centre line. With `d = end - start`, `n = ftol(|d| / segment + 2.0)` and `s = |d| * jitter` (`[bolt+0x24]`), it makes `n + 1` points: the first is `start`, the last is `end`, and point `i` from 1 to `n - 1` is `(start + (d * i) * (1 / n)) + (r1, r2, r3) * s`, each `r` drawn from `0x4531e0` on the file's generator: the low 23 bits of the draw ORed into `0x3f800000` give a float in `[1, 2)`, which becomes `f - 2.0` when the draw is non-negative and `2.0 - f` when it is negative. Every bolt shares the generator, which starts from seed 0.

Addresses: `0x7af6d0`, `0x7af70f`, `0x7af71c`, `0x7af745`, `0x7af748`, `0x7af8cc`, `0x7af91e`, `0x7af935`, `0x7af94b`, `0x4531e0`, `0x4549a0`, `0x4549f0`, `0xcf57b0`, `0x7af5d0`, `0x801628`
Confidence: verified
Proof: one
Test: `gx::lightning_segment_subdivide_difftest`

The jitter scale grows with the bolt: `fsqrt` (`0x7af70f`) leaves `|d|` on the stack, a copy becomes `n` through `__ftol` (`0x7af71c`), `1 / n` goes to `[ebp-0x4]` (`0x7af745`) and `|d|` times `[bolt+0x24]` to `[ebp-0x8]` (`0x7af748`). Each draw is multiplied by `[ebp-0x8]` (`0x7af91e`, `0x7af935`, `0x7af94b`); `[ebp-0x4]` only scales the step along the centre line (`0x7af8cc` to `0x7af8e3`), whose product `d * i` is stored as a float before it, as is every later product. The test runs the whole function.

The three draws per point are consumed in the order x, y, z of the jitter vector (`[ebp-0x14]`, `[ebp-0x10]`, `[ebp-0xc]` stored to `[esp]`, `[esp+4]`, `[esp+8]`). The segment count has no upper clamp. The generator's state is a global object that only `0x7af5d0` seeds.

## Bolt update

`0x7af9b0(dt)` runs when flag bit 1 is set. With a non-zero period the clock `+0x60` becomes `fmod(dt + clock, period)`, otherwise 0. With the rebuild bit set it rebuilds the centre line, sizes the ribbon arrays for `2 * points` vertices, writes texture coordinates (the pair at vertex `k` gets `u = k / (2 * points - 2)`, `v` 0 and 1; the first two vertices `(0, 0.5)`, the last two `(1, 0.5)`) and the indices `0, 1, 2, ...`, and clears the bit. Every update then builds a second centre line and sets each interior point to `0.75 * old + 0.25 * new`.

Addresses: `0x7af9b0`, `0x73f90a`, `0x8012cc`, `0x8029b0`, `0x7af990`, `0x7b0d50`, `0x7b0d90`
Confidence: verified
Proof: one
Test: none

The `u` values are `fild(k) * (1 / (vertexCount - 2))` for the even index `k` of each pair, so `u` runs 0 to 1 along the bolt. `0x73f90a` is `fmod`. The new line is built in a static array at `0xcf5854` with the same generator.

## Bolt render

`0x7afcb0(handle, eye)` draws an active bolt. It calls the callback, if set, as `callback(argument, handle, &start, &end)` and sets the rebuild bit. It saves the device view matrix, sets the view to the identity and transforms every centre-line point by a translation by `-eye` times the saved view. For each segment it forms the view-space perpendicular `(-dy, dx, 0)`, normalises it only when its length is above 0.001, and scales it by the width `w`. A shared point's vertex pair ends at `point ± (w1 + w2) / 2`; the first and last pairs are set to the end points.

Addresses: `0x7afcb0`, `0x58b0b0`, `0x58b050`, `0x7bc6a0`, `0x7bca80`, `0x7ffa24`, `0x801360`
Confidence: verified
Proof: one
Test: none

The half is the constant `0.5` (`0x7ffa24`), the width is `[bolt+0x1c]` and the length floor `0x801360`; a straight bolt is therefore `2 * width` wide. The ribbon is built in the vertex array at `+0x44` with `0x18` bytes per point.

## Bolt draw state

`0x7afcb0` pushes the render states (`0x589f40`) and sets: id 1 to the alpha byte shifted to the top, id 2 to the colour, cull off (id `0x14`), depth write off (`0x12`), fog off (`0xf`), blend mode 3 (source alpha, one) and the bolt's texture, loaded at once with `0x44acf0(texture, 1)`, on stage 0 (id `0x17`). It translates stage 0's texture by `(-clock / period, 0, 0)` (period 1.0 when 0), builds the vertices with `0x58a2a0` (12-byte positions, a shared normal `(0, 0, 1)`, texture coordinates at stride 8: format 3), draws the indices as a triangle strip (`0x58a2e0`, topology 4), pops the states and restores the view.

Addresses: `0x7afcb0`, `0x589f40`, `0x589f50`, `0x589e00`, `0x589e60`, `0x589e80`, `0x44acf0`, `0x58b250`, `0x58a2a0`, `0x58a2e0`, `0xcf5840`
Confidence: verified
Proof: one
Test: none

With lighting on, a vertex format with a normal and no colour, material diffuse equal to `alpha << 24` and emissive equal to the bolt colour, the Direct3D lighting output is the emissive colour at the alpha of the diffuse (`draw.md`, Lighting at draw time). The matrix work is the stage matrix calls `0x58b200`, `0x58b220`, `0x58b230` with ids 8 and 0.

## Renderer frame calls

`Update(dt)` (`0x7b0a40`) calls the bolt update for every live bolt, last handle first. `Render(eye)` (`0x7b0a80`) calls the bolt render for every live bolt, last handle first, with the handle as the first argument. `Init` (`0x7b0760`) zeroes the 32 bytes; `Destroy` (`0x7b0780`) destroys every bolt object, frees the arrays and the entries. The spell visuals call `Update` from `0x6eca20` with a time step in seconds and `Render` from `0x6ecb20` with the eye position.

Addresses: `0x7b0a40`, `0x7b0a80`, `0x7b0760`, `0x7b0780`, `0x6eca20`, `0x6ecb20`
Confidence: verified
Proof: one
Test: none

`0x7b09e0`, `0x7b0a00` and `0x7b0a20`, which set the colour, read the colour and read the float at `+0x2c`, have no caller; their rows are `unreached`.
