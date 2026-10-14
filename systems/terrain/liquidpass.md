# terrain: liquid passes

What the client sets on the device to draw the map's liquids: the three passes over the terrain
liquid slots (river, ocean and magma), the pass over the map object group liquids, the lights a
slot is drawn under, the two swatch textures the water passes bind, the sky gradient texture
made beside them and the search for the nearest liquid. The buffers, the vertex fills and the
strips are in `liquid`.

A device state is the id the client passes to `0x589e60` (or `0x589e30` and `0x589e00` for a
float or a colour). The ids this doc uses: 0x03 specular material, 0x04 shininess, 0x07 blend,
0x0e lighting, 0x0f fog, 0x12 depth write, 0x14 face culling, 0x17 texture on stage 0 and 0x18
on stage 1, 0x20 the combine preset of stage 1, 0x3f the pixel program.

Roots: `0x685010`, `0x6851b0`, `0x6855a0`, `0x684cd0`, `0x68a830`, `0x68c480`, `0x68af10`

## Liquid draw passes

The scene draws the terrain liquids in three passes over the slots of a layer's list: the ocean
`0x685010` (type 1), the river `0x6851b0` (type 0) and the magma `0x6855a0` (type 2). Each returns at once
for an empty list, pushes the device state, turns face culling off and pops at the end. Ocean and
river bind their swatch on stage 0. With the fancy-water byte set they bind the pixel program, set the
specular material white, the shininess 6.0, blend 2 (source alpha, one minus source alpha) and
depth write off; otherwise no program and stage 1 combine preset 3 (add). The magma turns
lighting and blending off and binds nothing.

Addresses: `0x685010`, `0xc7cb14`, `0x68503a`, `0x68504a`, `0x685067`, `0x685080`, `0x68508a`, `0x68509a`, `0x6850a6`, `0x6850b9`, `0x6851b0`, `0x6851da`, `0x6851ea`, `0x685206`, `0x68521c`, `0x6852a7`, `0x685440`, `0x685454`, `0x6855a0`, `0x6855ca`, `0x6855d6`, `0x6855e2`, `0x68de40`, `0xc9a324`, `0xc81b7c`, `0xc81b90`, `0xc89d68`, `0xc7cb1c`, `0xc7cb28`, `0xc7cb34`, `0x8102e8`
Confidence: verified
Proof: one
Test: none

`0x68503a` and `0x6851da` are `xor edx,edx; mov ecx,0x14`; `0x6855ca` is the same for the magma.
`0x68504a` and `0x6851ea` bind stage 0 (state 0x17). The fancy leg is `0x685067` (ocean) and `0x685206` (river):
`0x685080` sets the material (state 3, 0xffffffff), `0x68508a` the shininess (state 4, 6.0 at `0x8102e8`),
`0x68509a` blend 2 and `0x6850a6` depth write 0. The other leg is `0x6850b9` for the ocean and
`0x685454` for the river: program 0, state 0x20 = 3. In the river pass the extra texture branch at
`0x685257` runs only when `0xc800ec` holds a texture, which nothing sets; the pass then binds the
program (`0x685440`). The magma sets states 0x0e and 7 to 0 (`0x6855d6`, `0x6855e2`). Blend value 2
is source alpha, one minus source alpha: the tables `0x85c1f8` and `0x85c224` hold 0x302 and 0x303
at index 2. Every pass draws a slot with `0x68de40` (ocean 1, river 0, magma 2).

## Slot draw

For each slot a pass resets the slot's idle time `+0x30` to 0 and unlinks its node `+0x20`. With world flag
0x1000000 set it translates the world matrix to the slot's chunk origin less the camera
position, gathers the slot's lights and draws it.

Addresses: `0x685010`, `0x6850ef`, `0x6850f2`, `0x68512e`, `0x685144`, `0x68517a`, `0x685185`, `0x68df70`, `0x58b230`, `0x58b250`
Confidence: verified
Proof: one
Test: none

`0x6850f2` stores 0 in `+0x30`; the unlink is `0x685109` to `0x685127`. `0x68512e` tests the flag. The
chunk origin is the three floats at `[slot + 0x1c] + 0x6c`, each less the camera `0xc7cf20` (`0x685144`
to `0x685172`), set with `0x58b250` for matrix 8. `0x68517a` gathers the lights, and `0x685185`
draws.

## Magma slot draw

`0x68dca0` draws one slot of the magma pass. With the lava frames loaded (`0xc89daa`) it binds the
current frame of type 6 on stage 0, fills the slot's vertex buffer with `0x68d890` and its index
buffer with `0x68d9b0` when they are empty, pushes an identity texture matrix on stage 0 with the
matrix enable (state 0x37) on, and draws the 81 vertices as a strip in format 5. The lava of a terrain
slot does not scroll.

Addresses: `0x68dca0`, `0x68dcb0`, `0x68dcbe`, `0x68dcd3`, `0x68dcde`, `0x68dcf5`, `0x68dd1a`, `0x68ddb6`, `0x68ddc5`, `0x68ddd5`, `0x68de12`, `0x68de1e`, `0x68de25`, `0x68aac0`, `0x68d890`, `0x68d9b0`, `0x58b210`, `0x58b220`, `0xc89daa`
Confidence: verified
Proof: one
Test: none

The matrix at `0x68dd46` to `0x68ddaf` is the identity, all sixteen elements written, and `0x68ddb6`
pushes it (`0x58b210` with stage 0); `0x68de1e` clears state 0x37 and `0x68de25` pops it. The draw
record is {4, 0, index count `+0x408`, 0, 0x50} (`0x68ddf4` to `0x68de0c`). The group water's magma
arm is the only liquid that scrolls (`groupdraw`); the frame arithmetic at `0x68dd26` to `0x68dd3e`
is computed and not used.

## Slot lights

`0x68df70` builds the lighting a liquid slot is drawn with. It makes a model lighting record at
the slot's owning chunk (`[slot + 0x1c]`): x and y from `+0x5c` and `+0x60`, z the middle of the
slot's two heights `+0x4` and `+0x8`, radius 23.57 (0x41bc8fb3); sets its fog from the light
state's `+0x70`, `+0x74` and `+0x78`; and, when the record's view depth less twice its radius is
below the fog end, adds the global map light and each light of the chunk's list whose `+0xdc`
is set; it finishes the record at the camera.

Addresses: `0x68df70`, `0x68df81`, `0x68df92`, `0x68dfac`, `0x68e08e`, `0x68e0f8`, `0x68e100`, `0x68e128`, `0x68e145`, `0x68e183`, `0x68e1a6`, `0x70a0f0`, `0x71c110`, `0x71bf90`, `0x71c730`, `0xc89ef0`
Confidence: inferred
Proof: none
Test: `terrain::light_mid_y_difftest`, `terrain::doodad_plane_value_difftest`, `terrain::doodad_gate_difftest`, `terrain::doodad_color_normalize_difftest`

`0x68dfac` stores 0x41bc8fb3 as the radius; the middle height is `(max − min) × 0.5 + min`
(`0x68df92`). `0x68e128` is `fcomp 0xc62964; test ah,5; jp`, which skips the lights for a depth at
or above the fog end. `0x68e145` adds the light of `0xc89ef0 + 0x7c`; `0x68e183` those of
the chunk's list at `+0x104`. What the light record callees do beyond their call order is not read here.

## Group liquid pass

`0x684cd0` draws the liquids of the map object groups in the frame's list (`0xc7cafc`). It builds
the interior alpha ramp `0x6b6b60` when the list is not empty. For each group def it unlinks the
node `+0xd4`, and with world flag 0x100 set it sets matrix 8 to the camera translation times the
map object's matrix `+0x94`, makes a lighting record at the def, calls the def's fog and light
slots, finishes at the camera and has the map object draw the group's liquid.

Addresses: `0x684cd0`, `0xc7cafc`, `0x684dd7`, `0x684e0a`, `0x684e44`, `0x684f0a`, `0x684f3e`, `0x684f4a`, `0x684f6a`, `0x684f76`, `0x684f8a`, `0x684f9c`, `0x684fac`, `0x684fba`, `0x684fca`, `0x684fe0`, `0x684fe5`, `0x6b6b60`, `0x6b4060`, `0xca7f00`, `0x71c730`
Confidence: verified
Proof: one
Test: none

`0x684dd7` calls `0x6b6b60` when the list is non-empty; `0x684e44` tests bit 8 of the world flags.
`0x684f0a` forms the camera-relative matrix product with the object's `+0x94` and `0x684f3e` sets
matrix 8. The fog block flag `0xca7f00` is 1 when the light state's `+0x7c` is nonzero, the
def has an interior fog claim (`+0x98`) and the map object equals the state's `+0x98`, else 0
(`0x684f4a` to `0x684f76`). `0x684fac` and `0x684fba` call the def's vtable slots 1 and 2;
`0x684fca` finishes the lighting record; `0x684fe0` calls the group draw `0x6b4060` with the group
key `+0x7c` and the flags `+0x94`; the pass has no render state of its own.

## Liquid swatch textures

`0x68c4c0` makes two 8×64 textures that `0x68a830` fills: the river swatch (`0xc89d68`) and the ocean
swatch (`0xc81b90`). The callback runs for level 1 only. Row k is the shallow colour plus
floor(k × (deep − shallow)/64) on each of the colour and alpha bytes; the colours are the light
state's words at `+0xe8`, `+0xec` (river, colour bands 16 and 17 of the light record) and `+0xe0`, `+0xe4` (ocean, bands 14 and 15) and the alphas the
LightParams floats at `+0x114`, `+0x118` (river) and `+0x11c`, `+0x120` (ocean), each as the byte
of f32(v × 255 + 512) >> 14. On the last row the ocean swatch alone has its brightness
multiplied by 0.9 in HSV and its alpha forced to 255.

Addresses: `0x68a830`, `0x68a87d`, `0x68a895`, `0x68a8b3`, `0x68a8d1`, `0x68a8fb`, `0x68a902`, `0x68a909`, `0x68a915`, `0x68a97b`, `0x68a9c6`, `0x68a9cb`, `0x68a9d9`, `0x68aa13`, `0x68aa22`, `0x68aa2e`, `0x68aa87`, `0x68c6aa`, `0x68c6c5`, `0x68c6e3`, `0x68c6f3`, `0x68c4c0`, `0x7bbf20`, `0x7bbc80`, `0x7bbd60`, `0x7bbec0`, `0xc7f3b8`, `0x8102ec`
Confidence: verified
Proof: one
Test: none

The callback tests `dec eax; jne` on its level (`0x68a870`). The stack word at `[ebp+0x14]` is the
selector, 1 for the river (`0x68c6b4`) and 0 for the ocean (`0x68c6e3`); the light state is indexed by
selector × 8 (`0x68a8fb`, `0x68a902`) for the two colours and, by selector, from the four alpha
bytes (`0x68a909`, `0x68a915`). The row step is `((end − start) << 8) >> 6` added to `start << 8`
(`0x68a915` to `0x68a975`); the pixel is the high byte of each accumulator. The final-row test is
the row index against count − 1 (`0x68a9c6`) and the selector 0 (`0x68a9cb`); it runs `0x7bbf20`
(unpack), `0x7bbc80` (RGB to HSV), multiplies V by 0.9 (`0x68aa13`, constant `0x8102ec`),
`0x7bbd60` (HSV to RGB) and `0x7bbec0` (pack, alpha 255). Each row is written `width` times
(`0x68aa43`) into `0xc7f3b8`.

## Sky gradient texture

`0x68c4c0` also makes the 64-row sky gradient texture `0xc7fbb8`, which `0x68c480` fills through
`0x68c250` for level 1. The profile is 32 rows mirrored about the middle: row r (0 at the middle)
is the lerp between the five band colours of the light state at `+0xb4` over the intervals 0,
0.17, 0.20, 0.23, 0.24, 0.25 of 4 × 32 rows, each channel × 182/256, with the alpha byte the top
byte of the alpha ramp at index 2r.

Addresses: `0x68c250`, `0x68c480`, `0x68c4c0`, `0x68c66a`, `0x68c684`, `0x68c28a`, `0x68c2a9`, `0x68c2b2`, `0x68c341`, `0x68c360`, `0x68c3ca`, `0x68c3db`, `0x68c42c`, `0x690ae0`, `0x811530`, `0x81152c`, `0xc7fbb8`, `0xc7fbc0`, `0xc80978`
Confidence: inferred
Proof: none
Test: `terrain::color_lerp_i_difftest`

`0x68c480` runs only for level 1 with the stack word `[ebp+0x10]` 0 (`0x68c48a`) and stores
the pitch, 4 × width, and the buffer `0xc80978`. `0x68c250` finds the middle row from the
height (`0x68c265`), the five interval ends from the table at `0x81152c` (0, 0.17, 0.2, 0.23, 0.24,
0.25) times 4.0 (`0x80306c`) times the half height (`0x68c2a9` to `0x68c2c4`); inside an interval
the parameter steps by 1/(rows + 1) (`0x68c2ee`) and `0x690ae0` blends the band colour with the next
by its byte (`0x68c341`); `0x68c360` scales each channel by 0xb6 (`(c·0xb6 + 0xff) >> 8`); the
alpha is the byte at `[ramp + 4 × index + 7]` with the index the byte of f32(r/half × 64 + 512)
(`0x68c3ca`). Each row is filled `width` times (`0x68c3db`) going toward row 0, and `0x68c42c`
copies the rows to the second half in mirror order. Where the texture is bound is not read here.

## Nearest liquid cells

`0x68af10` finds, for a point, the nearest liquid cell of each type among the areas near it, and
`0x68b0d0` does it inside one area. The point maps to an area index (((17066.666 − p) × 0.24 − 0.5)
rounded, `>> 7`, 64 × 64 areas); the search takes the 3 × 3 areas around it, skipping empty ones
and those whose box misses the radius sphere. In an area it takes ceil(radius × 0.24) chunks
either side (clamped to 16 × 16) and keeps, per type, the cell of the chunks' four slots (flag
nibble not 0xf) nearest the point, at its corner and the slot's middle height, as an offset.

Addresses: `0x68af10`, `0x68b0d0`, `0x8102a0`, `0x68af20`, `0x68af36`, `0x68af8f`, `0x68b075`, `0x68b091`, `0x68b0f7`, `0x68b13f`, `0x68b1e4`, `0x68b21f`, `0x68b231`, `0x68b24a`, `0x68b295`, `0x68b29d`, `0x68b2a2`, `0x7c2040`, `0x7ffab4`, `0x810354`, `0x810290`, `0xc96318`
Confidence: verified
Proof: one
Test: none

The area index is the `fistp` of the scaled value less 0.5, shifted by 7 and masked by 0x3f
(`0x68af8f`); the loops run from index − 1 to index + 1, clamped (`0x68afa1`), over `0xc96318`.
`0x68b075` tests the area box (`0x7c2040`) from `+0x7c`, `+0x80` and the size `0x810284`. In
`0x68b0d0` a chunk coordinate is ((17066.666 − p) × 0.03 − 0.5) (`0x8102a4`, `0x869ffc`), `fistp` and masked by
0xf (`0x68b109`, `0x68b121`); the half-width is `ceil` of radius × 0.24 (`0x810354`, `0x68b13f`). The chunk
table is the area's `+0x278`; the slots are the chunk's four pointers at `+0x118` (`0x68b1e4`). A cell's
nibble is `[slot + 0x10][row × 8 + column] & 0xf` (`0x68b21f`); `0x68b231` stores 1 in the type's found
dword. The corner is the chunk origin `+0x6c`, `+0x70` less row and column times 4.1666665 (`0x810290`), z
`([slot + 8] + [slot + 4]) × 0.5` (`0x68b25e`). `0x68b29d` (`test ah,5; jp`) replaces the stored distance
only for a smaller one, NaN never. The radius never filters cells. A nibble above 11 writes past the
caller's 12-entry arrays; `0x6723f0` keeps distances in its local at `ebp − 0x34`, so nibbles 12, 13 and 14
index its indoor counter, saved `ebp` and return address.

## Map object liquid at a point

`0x69b520` asks the loaded map objects for liquid at a point. For each map object in the frame's
list it moves the point into the object's space with its `+0xd4` matrix and asks the model
for liquid (mask 0x2000); the first hit writes the liquid surface height, moved back to world space through the
`+0x94` matrix, to the height out argument and returns 1; with no hit it returns 0.

Addresses: `0x69b520`, `0xca7d9c`, `0x69b527`, `0x69b558`, `0x69b575`, `0x69b57a`, `0x69b5b2`, `0x69b5ba`, `0x6a4e00`, `0x7bca80`, `0x7bcc60`
Confidence: verified
Proof: one
Test: none

`0x69b527` loads the list head; the link offset is the list's first dword (`0x69b57e`). `0x69b558`
transforms the point by `+0xd4`; `0x69b575` calls the model query `0x6a4e00` with 0x2000; `0x69b5b2`
transforms the hit back by `+0x94` and `0x69b5ba` stores z.

## Liquid shutdown

`0x68c720` releases the sky gradient and the two swatch textures, the animated frames of each of
the 12 liquid types, the liquid lists and the pixel programs, and then the ripples.

Addresses: `0x68c720`, `0x68c728`, `0x68c73b`, `0x68c74c`, `0x68c762`, `0x68c785`, `0x68c7ee`, `0x68c81b`, `0x68ac20`, `0x68f9d0`
Confidence: verified
Proof: one
Test: none

`0x68c728`, `0x68c73b` and `0x68c74c` destroy the textures at `0xc7fbb8`, `0xc89d68` and `0xc81b90`; `0x68c762`
runs `0x68ac20` for types 0 to 0xb; `0x68c81b` tail-jumps to the ripple shutdown `0x68f9d0`.
