# minimap: draw

How the interface frame draws the minimap tiles. Outdoors the four tile records are drawn straight to the frame, each through the tile texture and the circular mask. Indoors the interior tiles are first composited into a 256 by 256 texture, and one quad shows the middle of that texture through the mask. The tile records, the interior window and the indoor byte are the `update` and `tiles` docs; the mask and icon textures, the blips and the arrows are `ui/minimap` and `ui/minimapblips`; the render states and matrix stacks the draw calls set are `gx/renderstate`.

Roots: none, the draw functions belong to the interface frame file and are called from the frame render

## Draw families

The frame render `0x4ec950` draws one family of tiles per frame, chosen by the indoor byte `0xbc8300`. The tile draw runs only when two device readiness values are non-zero; the rest of the render continues without it. When the redraw byte `0xbc8308` is set the draw marks the whole interior target dirty and clears the byte. It then pushes the render states, turns culling, lighting, fog, depth test and depth write off, sets blend mode 2 and builds the tint colour. Indoors it draws the interior target once through the blit; outdoors it draws the records 0 to 3 of the tile array at `0xbc83a8`. The other family is not drawn.

Addresses: `0x4ec950`, `0x4ecec5`, `0x4eced2`, `0x4ecedf`, `0x4ecef8`, `0x4ecefd`, `0x4ecc8d`, `0x4eccce`, `0x4ecd59`, `0x4ecd69`, `0x4ed01d`, `0x4ed031`, `0x4ed047`, `0xbc8300`, `0xbc8308`, `0xbc83a8`, `0x58ac70`, `0x4ecc14`
Confidence: verified
Proof: one
Test: none

`0x4ecec5` and `0x4eced2` call the two readiness getters and jump past the tile draw to the common tail when either returns 0. `0x4ecef8` calls `GxTexUpdateRect` with four zeros, which sets the texture's dirty rectangle to its whole size (`gx/textures`). `0x4ecc8d` pushes the state stack mark; the five states and the blend mode are set at `0x4ecc99` to `0x4eccd8`, and mode 2 sets the alpha reference 1 (`gx/renderstate#blend-alpha-and-depth-tables`). `0x4ecd59` loads the indoor byte, which the update wrote at `0x4ecc14` (`update#frame-update`), and `0x4ecd69` jumps to the outdoor loop when it is clear. The loop at `0x4ed031` runs four times with a stride of `0xa4` and calls `0x4eae10` at `0x4ed047`; the indoor arm calls the blit at `0x4ed01d` and jumps over the loop.

## Family exclusion

The two tile families share one array of 512 records, and the test the interior list uses, mask `0x2` of the flags and a texture, does not tell them apart: the outdoor loader sets mask `0x2` too. The update keeps them apart. It loads outdoor tiles only while the indoor byte is clear. The first indoor frame always rebuilds the interior, because the outdoor branch leaves the cached identity words at -1, and the rebuild clears mask `0x2` of all 512 records. A record the interior loader never filled also has an empty box, so its quad has no area.

Addresses: `0x6da5e0`, `0x6da5f8`, `0x6da600`, `0x6da675`, `0x6d95cd`, `0x6d9a5f`, `0x6d9a62`, `0x6d9a75`, `0x6d999f`, `0x6d9582`, `0x6d958d`, `0x6d9592`, `0x6d94da`, `0x6d94e5`, `0x6d94ef`, `0x6d9502`, `0x6d9724`, `0x6d9730`, `0x6da0b5`, `0x4ead10`, `0x4ea430`, `0x4ea442`
Confidence: verified
Proof: one
Test: none

`0x6da5f8` and `0x6da600` clear mask `0x2` of the first four records only, and `0x6da675` sets it on a record the outdoor tile fills. The calls of the outdoor loader are `0x6d999f`, reached when the indoor byte is clear (`0x6d95cd`), and `0x6d9a75`, guarded by `0x6d9a5f` and `0x6d9a62`. The outdoor branch stores -1 in the two cached words (`0x6d958d`, `0x6d9592`), which the indoor branch compares with the live identity (`0x6d94da`, `0x6d94e5`); a difference marks the frame changed (`0x6d94ef`) and clears every record's key (`0x6d9502`). `0x6d9724` and `0x6d9730` clear bit 2 of the 512 records. The record constructor `0x4ead10`, run for all 512 by `0x4ea430`, zeroes the six box floats at `+0x70`, and the interior loader fills them with the copy at `0x6da0b5`; no instruction of the outdoor loader range `0x6da380` to `0x6da7b0` has an operand at `+0x70` to `+0x84`.

## Tile order

`0x4ebeb0` builds the list of interior tiles to draw. It scans the 512 records and keeps those with mask `0x2` of the flags word at `+0x94` set and a non-zero texture handle at `+0x90`. It inserts each into a singly linked list, linked through `+0x88`, in ascending order of the distance at `+0x8c`, before the first node whose distance is not smaller. The head of the list is drawn first, so the tile of the player's own group, which has the distance 3.4028235e38, is drawn last and lies on top.

Addresses: `0x4ebeb0`, `0x4ebec3`, `0x4ebecf`, `0x4ebee0`, `0x4ebeee`, `0x4ebefc`, `0x4ebf04`, `0x4ec3ea`, `0x4ec3ff`, `0x4ebf20`, `0xbc8438`
Confidence: verified
Proof: one
Test: none

`0x4ebeb7` starts the scan at `0xbc8438`, the handle field of record 0, and `0x4ebec3` tests `+4` from it, which is the flags word. `0x4ebee0` compares the candidate's distance with the node's by `fcomp`; `0x4ebeee` and the following jump insert before the node on less or equal and advance on greater or unordered. `0x4ebefe` copies the previous link into the candidate and `0x4ebf04` stores the candidate there. The caller clears the head (`0x4ec3e3`), calls the builder (`0x4ec3ea`) and walks the list through `+0x88` (`0x4ec3ff`), drawing each record with `0x4ebf20`. The distances are set by `tiles#interior-tiles`.

## Interior tile quad

`0x4ebf20(record, colour)` draws one interior tile as a four-vertex triangle strip. The positions are the corners of the record's bounding box in x and y with z 0. The texture coordinates are inset by half a texel on every side, `0.5 / width` and `0.5 / height`, where the width and height are the tile texture's. One colour serves all four vertices.

| vertex | position | texture coordinate |
|---|---|---|
| 0 | min x, min y | `du`, `1 - dv` |
| 1 | max x, min y | `1 - du`, `1 - dv` |
| 2 | max x, max y | `1 - du`, `dv` |
| 3 | min x, max y | `du`, `dv` |

Addresses: `0x4ebf20`, `0x4ebf29`, `0x4ebf3a`, `0x4ebf72`, `0x4ebf89`, `0x4ebfb0`, `0x4ebfca`, `0x4ebfe3`, `0x4ec03e`, `0x4ec04a`, `0x4ec06e`, `0x4ec07f`, `0x4ec078`, `0x58a2a0`, `0x58a2e0`, `0x84c524`
Confidence: verified
Proof: one
Test: none

The vertex positions are stored from `+0x70` (min x), `+0x74` (min y), `+0x7c` (max x) and `+0x80` (max y) with the z words set to 0 (`0x4ebf29` to `0x4ebf6c`); the z bounds at `+0x78` and `+0x84` are never read. The record's handle at `+0x90` becomes the texture object (`0x4ebf89`); the draw returns when none comes back. `0x4ebfb0` reads the texture's size, and `0x4ebfca` and `0x4ebfe3` divide 0.5 by the width and height. The texture is bound to stage 0 (`0x4ec03e`) and depth test is set to 0 (`0x4ec04a`). `0x4ec06e` submits four vertices with the position stride 12, the texture stride 8 and the colour pointer the caller passed, kept in `edi` (`0x4ebf3a`), with stride 0. `0x4ec07f` draws the index list at `0x84c524`, the words 0, 1, 3, 2, as a strip.

## Composite pass

`0x4ec090(colour)` renders all listed interior tiles into the bound target. It saves the projection and view matrices, sets an orthographic projection of half-extent 1.5 times the radius `0xbc830c` with depth range -1000 to 1000, and an identity view. On world matrix stack 8 it loads the orientation matrix `0xbc8328`, then translates by the negated position `0xbc8310` transformed through `0xbc8368`. It resets the render states, draws the listed tiles in order and restores the matrices and the state stack.

Addresses: `0x4ec090`, `0x4ec12b`, `0x4ec130`, `0x4ec136`, `0x4ec1fe`, `0x4ec209`, `0x4ec281`, `0x4ec32c`, `0x4ec331`, `0x4ec336`, `0x4ec38b`, `0x4ec395`, `0x4ec3a4`, `0x4ec3b6`, `0x4ec3db`, `0x4ec3f6`, `0x4ec40e`, `0x4ec416`, `0x4ec421`, `0x4ec426`, `0xbc830c`, `0xbc8310`, `0xbc8328`, `0xbc8368`, `0x5c3d90`, `0x58b200`, `0x58b230`, `0x58b270`, `0x58b250`, `0x5a1480`
Confidence: verified
Proof: one
Test: none

`0x4ec12b` saves the projection and `0x4ec281` the view. `0x4ec130` loads the radius and `0x4ec136` multiplies it by 1.5; `0x4ec1fe` calls `BuildOrthoMatrix` with left and bottom the negated half-extent, right and top the half-extent, near -1000 and far 1000, and `0x4ec209` and `0x4ec32c` install that projection and an identity view. Stack 8 is the world stack: the device hands its top, at `+0x1898`, to the world transform (`0x5a1480`). `0x4ec38b` pushes it, `0x4ec395` loads the identity, `0x4ec3a4` multiplies by `0xbc8328`, and `0x4ec3b6` transforms `0xbc8310` through `0xbc8368`; `0x4ec3db` translates by the negated point, so a tile position `v` lands at `(v - t) * M2`. `0x4ec40e` pops the stack; `0x4ec416` and `0x4ec421` restore the saved view and projection.

## Composite states

The composite resets every render state, then turns off culling, lighting, fog, depth test and depth write, and sets blend mode 1. Mode 1 draws without blending and with the alpha reference 224, so a fragment is written whole when its alpha is at least 224 and not at all otherwise. The vertex colour is white with the frame's alpha, and stage 0 combines it with the texel by modulation on colour and alpha. The colour write mask is the caller's, because the reset leaves it unchanged.

Addresses: `0x4ec336`, `0x4ec33b`, `0x4ec347`, `0x4ec353`, `0x4ec35f`, `0x4ec36b`, `0x4ec377`, `0x4ec381`, `0x589f60`, `0x85ad20`, `0x4ec901`, `0x4ec8c6`, `0x4ec8cf`, `0xbdcbb8`
Confidence: verified
Proof: one
Test: none

`0x4ec336` is the only call of the full reset `0x589f60` in the client; it sets ids 14, 15, 16 and 20 to 1 and id 7 to 0 (`gx/renderstate#reset-defaults`), and the composite then sets ids 20, 14, 15, 16 and 18 to 0 (`0x4ec33b` to `0x4ec36b`) and id 7 to 1 (`0x4ec377`). Blend mode 1 disables blending and the cascade stores `s_alphaRefByBlendMode[1]`, 224, as the alpha reference; the test is greater or equal. Id 19, the colour mask, is not set. The colour is built by the callback: `0x4ec8c6` loads the frame object from `0xbdcbb8`, `0x4ec8cf` reads its alpha byte at `+0xc8`, and `0x4ec901` sets the three colour bytes of the alpha shifted to the top byte to 255. The preset of stage 0 is 1 (modulate, modulate) from the reset, so the fragment alpha is the vertex alpha times the texel alpha; a frame alpha below 224 therefore draws nothing.

## Interior target

The interior target is a 256 by 256 texture with one level, created once with a redraw callback. Its format is 1 when the device capabilities word at `+0x70` is set, else 4 when `+0x7c` is, else 0. The callback acts only for mode 1: it makes the texture render target 0, sets a full viewport, clears the colour to black with the frame's alpha, draws the tiles with the composite pass and restores the target, the clear colour and the viewport. The initial radius is 10.0, which the first interior rebuild replaces.

Addresses: `0x4ed9d0`, `0x4ed9d5`, `0x4ed9df`, `0x4ed9e6`, `0x4ed9eb`, `0x4ed9f7`, `0x4eda03`, `0x4eda16`, `0x4eda2e`, `0x4eda3a`, `0x4eda42`, `0x4eda4a`, `0x4ec840`, `0x4ec846`, `0x4ec890`, `0x4ec89f`, `0x4ec8b9`, `0x4ec8c1`, `0x4ec8e5`, `0x4ec8ef`, `0x4ec8fc`, `0x4ec90d`, `0x4ec918`, `0x4ec935`, `0xbc8304`, `0xbc8264`, `0x58aa30`, `0x58a970`, `0x5a0b17`, `0x5a0b21`, `0x6d98e5`, `0x6d98f5`, `0x4eda80`, `0x4edada`
Confidence: verified
Proof: one
Test: none

`0x4ed9d5` sets the active flag `0xbc8264`; `0x4ed9df` and `0x4ed9e6` build the blip candidates of the current map; `0x4ed9eb` stores 10.0 in `0xbc830c`. `0x4ed9f7` to `0x4eda16` choose the format, `0x4eda2e` packs the flags (filter mode 1, bit 7 set, anisotropy 1) and `0x4eda4a` creates the texture of 256 by 256 with the callback `0x4ec840` (`0x4eda3a`) and the handle stored at `0xbc8304`. The device upload loop calls a texture's callback with mode 1 for each level (`0x5a0b17`), then skips the copy when flag bit 7 is set (`0x5a0b21`), so the callback is the draw. `0x4ec846` returns for any other mode. `0x4ec8c1` saves the clear colour; `0x4ec8e5` sets the frame alpha shifted to bits 24 to 31, so the clear is black with that alpha, and `0x4ec8ef` clears colour only (mask 1). `0x4ec90d` composites with the white colour. The first interior rebuild stores the radius from the indoor table at `0x6d98f5`, after setting the redraw byte at `0x6d98e5`. `0x4eda80` destroys the texture (`0x4edada`).

## Interior blit

`0x4ec440(radius, offset, colour)` draws the interior target onto the frame as one quad of corners (+1, +1) and (-1, -1) scaled by half the radius, under an orthographic projection of the same half-extent. Stage 0 shows the target and stage 1 the circular mask, both with texture coordinates generated from the camera-space position. The target is read over its middle two thirds, which is a radius of the full radius in model units, shifted by the offset; the mask maps the whole quad to its extent.

Addresses: `0x4ec440`, `0x4ec4d3`, `0x4ec4ea`, `0x4ec568`, `0x4ec570`, `0x4ec575`, `0x4ec584`, `0x4ec593`, `0x4ec598`, `0x4ec5aa`, `0x4ec5c9`, `0x4ec5e8`, `0x4ec607`, `0x4ec612`, `0x4ec61d`, `0x4ec62c`, `0x4ec63b`, `0x4ec64a`, `0x4ec65c`, `0x4ec668`, `0x4ec672`, `0x4ec694`, `0x4ec6b6`, `0x4ec6d8`, `0x4ec6e0`, `0x4ec6f7`, `0x4ec701`, `0x4ec71e`, `0x5a14f1`, `0xbc7968`
Confidence: verified
Proof: one
Test: none

With `S` the radius argument, `0x4ec4d3` halves it and `0x4ec4ea` stores the half back, so every later use is `h = S / 2`. `0x4ec568` builds the projection of half-extent `h` and depth -1000 to 1000. `0x4ec584` and `0x4ec593` set the coordinate mode of stage 0 to 3, camera-space position, and the state id `0x37` to 1, which the texture matrix flush reads (`0x5a14f1`); `0x4ec63b` and `0x4ec64a` do the same for stage 1. `0x4ec598` binds the target to stage 0, `0x4ec668` the mask `0xbc7968` (`0x4ec65c`) to stage 1, and `0x4ec5aa`, `0x4ec672` and `0x4ec701` push the stacks 0, 1 and 8. Stack 0 takes a translation (0, 1), a scale (1, -1), the translation by the offset and the scale `1 / h` times one third (`0x4ec5c9`, `0x4ec5e8`, `0x4ec607`, `0x4ec612`, `0x4ec61d`); the last applies first, so stage 0 reads `u = 2x / (3S) + ox` and `v = 1 - (2y / (3S) + oy)`. Stack 1 takes (0, 1), (1, -1), (0.5, 0.5) and the scale `1 / h` times 0.5 (`0x4ec6e0`), so stage 1 reads `u = 0.5 + x / S` and `v = 0.5 - y / S`. Stack 8 is scaled by `h` (`0x4ec71e`).

## Blit quad and offset

The blit quad has the corners (-1, -1), (-1, 1), (1, 1) and (1, -1), written once into a static array. It is drawn as a strip in the order 0, 1, 3, 2 with one colour. The caller shifts the target by the player's movement since the composite: the offset is the player position less the composite position, rotated through the two matrices, divided by 3 times the radius, plus 0.5 on each axis.

Addresses: `0x4ec740`, `0x4ec7b8`, `0x4ec7e2`, `0x4ec7f3`, `0x4ec7fd`, `0x4ec823`, `0x4ec830`, `0x4ecfd8`, `0x4ecfe9`, `0x4ed011`, `0x4ed01a`, `0x4ed01d`, `0xbc7938`, `0xbc8260`, `0xbc831c`, `0xbc8310`, `0xbc8368`, `0xbc8328`, `0x84c52c`, `0x84c534`
Confidence: verified
Proof: one
Test: none

The array at `0xbc7938` has four vertices of three words, written under the guard mask `0x1` of `0xbc8260` (`0x4ec740`) with the exit handler `0x4ec830` (`0x4ec7b8`). `0x4ec7e2` submits it with a position stride 12 and one colour, and `0x4ec7f3` draws the index list `0x84c52c`, the words 0, 1, 3, 2. `0x4ec7fd` restores the saved projection and `0x4ec823` pops the state stack. The caller builds the offset at `0x4ecfd8` to `0x4ed011`: the player position `0xbc831c` less the composite position `0xbc8310`, carried through `0xbc8368` and then `0xbc8328`, divided by `3S` (`0x84c534` is one third and `0x4ecfd8` divides 1 by the radius), plus 0.5 on each axis. `0x4ed01a` passes the radius and `0x4ed01d` calls the blit.

## Draw tint

The outdoor tiles are drawn in a tint taken from the light state. The second light colour `c2` is pushed toward white by `t = min(L + 0x60, 0xff)` over 256, where `L = (77 R + 151 G + 28 B) >> 8` is the luminance of `c2`. The first light colour `c1` is then pushed toward that result by `0xc0` over 256. The alpha byte of the result is the frame's alpha. The interior blit is drawn in white with the same alpha.

Addresses: `0x4eccdd`, `0x4ecce2`, `0x4eccf1`, `0x4ecd08`, `0x4ecd0b`, `0x4ecd11`, `0x4ecd1f`, `0x4ecd22`, `0x4ecd3d`, `0x4ecd49`, `0x4ecd54`, `0x4ecd66`, `0x4ecf14`, `0x4ecf1d`, `0x4ef7d0`, `0x6d48b0`, `0xce9b60`
Confidence: verified
Proof: one
Test: none

`0x4eccdd` gets the light state `0xce9b60`; `0x4ecce2` loads `c1` from `+0x148` and `0x4eccf1` loads `c2` from `+0x14c`. `0x4ecd08`, `0x4ecd0b` and `0x4ecd11` weigh the blue, green and red bytes by 28, 151 and 77, `0x4ecd1f` adds 0x60 and `0x4ecd22` clamps to 255. `0x4ecd3d` calls `0x4ef7d0` on `c2` with `t` and white, and `0x4ecd54` calls it on `c1` with `0xc0` and the result. `0x4ef7d0` moves each of the three colour channels of its target by `((source - target) * t) >> 8` and keeps the target's alpha. `0x4ecd66` stores the frame alpha in the result's top byte. The indoor colour is the alpha shifted to the top byte with the three colour bytes set to 255 (`0x4ecf14`, `0x4ecf1d`).

## Outdoor tile draw

`0x4eae10(record, colour)` draws one outdoor tile when mask `0x1` of the flags at `+0x94` and the handle at `+0x90` are set. The record holds its own four vertices: positions from `+0`, texture coordinates for the tile from `+0x30` and for the mask from `+0x50`. Stage 0 gets the tile and stage 1 the circular mask, both with preset 1. The four vertices are drawn as a strip in the order 0, 1, 2, 3, and stage 1 is cleared afterwards.

Addresses: `0x4eae10`, `0x4eae19`, `0x4eae26`, `0x4eae34`, `0x4eae4c`, `0x4eae5b`, `0x4eae74`, `0x4eae88`, `0x4eae92`, `0x4eae98`, `0x4eaec7`, `0x4eaed1`, `0x4eaee4`, `0x84c51c`
Confidence: verified
Proof: one
Test: none

`0x4eae19` and `0x4eae26` test the flag and the handle, and `0x4eae34` resolves the texture object, returning when none comes back. `0x4eae4c` and `0x4eae5b` bind it to stage 0 with the preset 1; `0x4eae74` and `0x4eae88` bind the mask `0xbc7968` to stage 1 with the preset 1. `0x4eaec7` submits four vertices with the position stride 12 at the record, a fixed normal (0, 0, 1) with stride 0, the colour with stride 0 and two texture arrays of stride 8 at `+0x30` and `+0x50`. `0x4eaed1` draws the index list `0x84c51c`, the words 0, 1, 2, 3, as a strip, and `0x4eaee4` sets stage 1's texture id back to 0.

## Tile sampler

Both tile families create their textures with the filter mode 1 and anisotropy 1, wrapping clamped, one level, and without the force flag. Mode 1 is bilinear with no mip filter, so level 0 is the only level sampled, and the force flag would otherwise replace the mode by the global setting.

Addresses: `0x6da047`, `0x6da05a`, `0x6da74f`, `0x6da764`, `0x449d90`, `0x449ae0`, `0x58a980`
Confidence: verified
Proof: one
Test: none

The flags are packed by `0x58a980` from eight values: mode, wrap in U, wrap in V, three single bits, bit 7 and anisotropy. The interior loader pushes zeros, 1 for anisotropy and 1 for the mode last (`0x6da01a` to `0x6da02e`), and calls `0x58a980` at `0x6da047`; the outdoor loader does the same at `0x6da74f`. Both call `TextureCreate` `0x449d90` with the force argument 0 (`0x6da05a`, `0x6da764`), and `0x449ae0` leaves the mode alone when the force argument is 0 (`gx/textures#texture-flags-word`). The modes 0 and 1 upload one level, and the wrap bits clear mean clamp (`gx/renderstate#stage-sampler-state`).
