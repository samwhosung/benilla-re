# models: drawing

How a scene's sorted elements reach the device: the per-list draw, the texture, lighting, fog and
material state each element sets, the batch draw with its bone palette or software skinning, and
the draw legs of ribbons, particles and custom draws.

The draw lives in M2Scene.cpp (`scene`) with the skinning routines and vertex fills of M2Model.cpp.
`0x708900` draws one list with a 0x335c-byte draw state on the stack; the state remembers the
previous element so unchanged device state is not set again. A batch is a 24-byte record of the
selected view, a submesh a 32-byte record (`shared`). Device calls go through the graphics layer
(`gx`), named here by their state numbers.

Roots: `0x708900`, `0x71a720`, `0x71a9e0`

## List draw

`0x708900(k)` resets the draw state, draws list `k`, and with `k = 0` also the grouped list after
it. A list draw saves the device view, sets an identity view and texture matrix, clears four render
states, and with vertex shaders copies the view into shader constants. It then draws each element
through its kind and copies the element's keys to the previous-element slots; a batched run
(kind 2) advances past the run's length. It ends by restoring the view.

Addresses: `0x708900`, `0x70b0e0`, `0x70b360`, `0x70b728`, `0x70cb30`, `0x70cf70`, `0x70d330`, `0x70d820`, `0x70d8b0`, `0x70d960`, `0x70b330`, `0x70b350`, `0x70dbf0`
Confidence: verified
Proof: one
Test: `models::decal_texgen_pack_difftest`, `models::unit_length_gate_difftest`

`0x708900` calls `0x70b0e0` on the stack state, then `0x70b360(k, elements, list data, list
count)` with the arrays at scene `+0x54 + 16k` and `+0x50 + 16k`, and again with `+0x44`/`+0x40`
for `k = 0`. `0x70b360` stores the element at `+0x3300`, its kind at `+0x3308`, the instance at
`+0x3310` and dispatches through the six-entry table `0x70b728`: kinds 0 to 5 go to `0x70cb30`,
`0x70cf70`, `0x70d330`, `0x70d820`, `0x70d8b0` and `0x70d960`.

Of `0x70cf70`, `decal_texgen_pack_difftest` runs `0x70d07e`–`0x70d2d0`; of `0x70d330`,
`unit_length_gate_difftest` runs `0x70d717`–`0x70d761`.

## Texture stages

An element of kind 0 to 2 binds a stage per texture of its unit. Stage `k` binds the instance
texture `+0xa4[i]`, `i` the header's texture lookup entry at unit `+0x10 + k`, or 0 for a null
handle, combine `0x811f8c` by blend mode (0, 1, 1, 1, 1, 5, 5), or for version 0x101 the array
`+0x148` at `k`, and the scene float `+0x14`. Stages from the count up to 1 bind 0; other kinds bind
0 with combine 1 on stages 0 and 1. A lookup entry of 0x8000 or more is inverted into a texture
frame state index, but zero-extended first, so the word read lies 0x100014 to 0x1ffff4 bytes before
those states and indexes `+0xa4` unbounded.

Addresses: `0x70b740`, `0x811f8c`, `0x70b750`, `0x70b777`, `0x70b78c`, `0x70b794`, `0x70b7bd`, `0x70b7c2`, `0x70b7ed`, `0x70b823`, `0x70ba91`
Confidence: verified
Proof: one
Test: none

`0x70b777` reads the lookup word and `jge` keeps a non-negative one. Otherwise `movzx eax,ax; not
eax; shl eax,5` (`0x70b78c`) and `movzx eax,word [eax+edx+0xc]` (`0x70b794`) with `edx` the
instance's `+0xac`, the 32-byte texture frame states whose word `+0xc` the animate pass sets
(`0x715eb0`). A nonzero handle resolves through `0x44acf0`, else `0x70b7bd` passes 0. The states are
`0x17 + k` (texture, `0x70b7ed`), `0x1f + k` (combine) and `0x27 + k` (the float); `0x70b7c2`
compares the version with 0x101. `0x70b823` binds 0 while `k < 2`. A kind above 2 (`0x70b750`) takes
`0x70ba91`: states `0x17`, `0x1f`, `0x27`, `0x2f` and `0x37` plus `k` get 0, 1, the float, 0 and 0,
and `0x58b230(k)` loads identity.

## Texture coordinates

A stage takes generated coordinates when the header's coordinate lookup `+0xa0` at unit `+0x12 + k`
is above 2: on the vertex shader lane its two constants are (0.5, 0, 0, 0.5) and (0, 0.5, 0, 0.5),
else texture generation mode 6. Otherwise a transform lookup `+0xb0` at unit `+0x16 + k` below the
header's transform count selects the instance's matrix `+0xb4[i]`: the shader lane takes its columns
0 and 1 as the two constants, the fixed lane loads the whole matrix. Any other stage gets identity.
The setup is skipped when the element's kind, shared model, unit `+0x12` and `+0x16` equal the
previous element's and its texture count is not above the last one.

Addresses: `0x70b833`, `0x70b8bd`, `0x70b8d0`, `0x70b8f1`, `0x70b90b`, `0x70b91f`, `0x70b9b4`, `0x70b9e8`, `0x70ba0e`, `0x70ba47`
Confidence: verified
Proof: one
Test: none

`0x70b8bd` is `cmp word [lookup],2; jbe`. The shader constants are stage `k`'s pair at `+0xb0 +
32k`, vertex constants c(6 + 2k) and c(7 + 2k) (`0x70ba47` keeps the window from c6 to c(6 +
2·count)). `0x70b91f` copies matrix offsets 0x0, 0x10, 0x20, 0x30, then 0x4, 0x14, 0x24, 0x34: with
a row-major matrix and the row vector `(u, v, 0, 1)` these give `u'` and `v'`. The fixed lane sets
state `0x2f + k` (generation) to 0 or 6 and `0x37 + k` (matrix on) to 1 with `0x58b240(k, matrix)`
(`0x70b9b4`), or to 0 with `0x58b230(k)` (`0x70ba0e`). `0x70b9e8` writes the identity pair (1, 0, 0,
0) and (0, 1, 0, 0). The animated matrix itself is `animation`, `Texture transforms`.

## Lighting and fog

Lighting is on unless the material has flag 1 or its blend is 5 or 6. Fog is on unless the
material has flag 2 or the lighting record has no fog, and its mode comes from the blend: modes
0 to 2 fog to the record's colour, 3 and 4 to black, 5 to white and 6 to grey 0x808080. The fog
range is the record's start `+0x184` and end `+0x188`. An element straddling the clip plane sets
it as a user clip plane, negated for list 2.

Addresses: `0x70baf0`, `0x811fa8`, `0x811fc4`, `0x70c17c`, `0x70bdeb`
Confidence: verified
Proof: one
Test: `models::batch_alpha_resolve_difftest`, `models::batch_clamp_pack_difftest`, `models::batch_negate4_difftest`

The tables are `0x811fa8` (1, 1, 1, 1, 1, 0, 0) for lighting and `0x811fc4` (1, 1, 1, 2, 2, 3,
4) for the fog mode, which `0x70bdeb` dispatches through `0x70c17c`. Mode 1 packs the colour at
`+0x190`, each component 0 at or below 0, 255 at or above 1, else `ftol(x·255 + 0.5)`; modes 2, 3
and 4 store 0, 0xffffff and 0x808080. With vertex shaders the fog constants are `−+0x18c` and
`+0x188 · +0x18c`. The clip plane is lighting `+0x1a0`, its four floats negated when `k = 2`.

Of `0x70baf0`, `batch_alpha_resolve_difftest` runs `0x70bdbb`–`0x70bde5`,
`batch_clamp_pack_difftest` `0x70beb5`–`0x70bfaa`, `batch_negate4_difftest` `0x70c10f`–`0x70c14c`.

## Lighting lanes

A lit material takes one of three lanes. With the vertex shader flag set it packs the lighting
record's harmonic terms into vertex constants c10 to c16, or pixel constants c1 to c7 when a pixel
shader is bound, writes the nearest point lights to c17 to c27, and turns fixed lighting off.
Without it, it commits the record to the device lights (`0x71c730(0, 0)`) with lighting on. An unlit
material turns lighting off when `0x589ca0` returns nonzero; otherwise it sets light 0 white along
(0, 0, −1) with black ambient, disables lights 1 to 3 and turns lighting on. Only kinds 0 and 2 set
the flag.

Addresses: `0x70bb94`, `0x70bba4`, `0x70bbb7`, `0x70bbf8`, `0x70bc19`, `0x70bcf3`, `0x70bdf6`, `0x70be05`, `0x70be91`, `0x70b607`, `0x70cb71`, `0x70d371`, `0x71c4e0`
Confidence: verified
Proof: one
Test: none

`0x70b360` zeroes the flag `+0x32f0` before each dispatch (`0x70b607`); `0x70cb30` and `0x70d330`
set it to the cache feature bit 8 (`0x70cb71`, `0x70d371`). The light at near slot `i` gives c(17 +
i) = its diffuse `+0x3c` with w 1.0, c(21 + i) = its view position `+0x18` with w 1.0, and component
`i` of c25, c26 and c27 its attenuation terms `+0x54`, `+0x58`, `+0x5c`; unused slots get zero
colour (`0x70bcf3`). Constant `r` lives at `+0x50 + 16r` of the draw state, a pixel constant at
`+0x3248 + 16r`. The unlit light (`0x70be91`, `0x589d80`) has diffuse 0xffffff and ambient
0xff000000; lighting is state 0xe. A census of `+0x32f0` in the listing finds four writers,
`0x70b23c` (the state's constructor), `0x70b607`, `0x70cb71` and `0x70d371`, and the same pattern
finds its 18 reads.

## Material state

| state | value |
|---|---|
| colour write mask | 0 for a depth-only element, else 0xf |
| blend | the table `0x811fe0` at `7k + blend`; for a depth-only element, 1 when blend is 1 |
| alpha reference | blend 0: 0; blend 1: `ftol(alpha · 224.0)`; otherwise 1 |
| cull | on unless material flag 4 |
| depth test | on unless material flag 8 or instance flag 2 |
| depth write | on unless material flag 0x10 or instance flag 2 |
| normalize | set only without vertex shaders and for kinds other than 2: off when row 0 of a one-bone submesh's bone matrix, or for a kind above 2 of the instance's world matrix, has `|len² − 1| ≤ 0.001` or NaN |

The blend table holds 0, 1, 2, 10, 3, 4, 5 for list 0 and 2, 2, 2, 10, 3, 4, 5 for lists 1 and
2. Blend 5 draws with diffuse 0 and emissive 1, blend 6 with emissive 0.5; other blends use the
instance diffuse times the colour animation and the instance emissive, and an unlit material adds
the diffuse into the emissive and zeroes the diffuse.

Addresses: `0x70c190`, `0x811fe0`, `0x70c20f`, `0x70c259`, `0x70c2bc`, `0x70c390`
Confidence: verified
Proof: one
Test: `models::color_scale_difftest`, `models::normalize_residual_difftest`, `models::rgba_pack_value_difftest`, `models::scale_add_difftest`

`0x70c1f1` compares the blend word with 1 for an element with `+0x8` set; `0x70c20f` reads
`[ecx*4+0x811fe0]` with `ecx = 7k + blend`. The alpha reference multiplies element `+0x10` by 224.0
(`0x812034`) and truncates with `__ftol`. The cull state is `(~flags >> 2) & 1`; depth test and
write test material bits 8 and 0x10 and instance `+0x4` bit 2. The normalize test squares row 0 of
the bone matrix, subtracts 1.0, takes `fabs` and compares with 0.001 (`0x801360`); it is skipped
when `+0x32f0` is set or the kind is 2 (`0x70c319`), and a kind above 2 tests instance `+0xfc`
(`0x70c569`). The colour states pack each channel clamped to [0, 1] as `x·255 + 0.5`.

Of `0x70c190`, `color_scale_difftest` runs `0x70c256`–`0x70c25f`, `normalize_residual_difftest`
`0x70c36a`–`0x70c5a8`, `scale_add_difftest` `0x70c631`–`0x70c69c`, `rgba_pack_value_difftest`
`0x70c77e`–`0x70ca07`.

## Material colours

The material step commits two colours: for kinds 0 to 2 the diffuse `D` and emissive `E` of
`Material state`, with the element alpha; for kinds 3 to 5 white with the element alpha and black.
With a pixel shader bound `D` and the alpha go to pixel constant c8 and `E` with w 0 to c9; on the
vertex shader lane to vertex constants c28 and c29; otherwise to device states 1 and 2 as packed
bytes, with state 4 set to 0.0. A kind 0 to 2 element whose kind, shared model, shader flags, lit
flag, blend, texture unit colour index, alpha and instance colours `+0x1a0` to `+0x1b4` equal the
previous element's commits nothing.

Addresses: `0x70c3b7`, `0x70c4d3`, `0x70c60a`, `0x70c657`, `0x70c696`, `0x70c712`, `0x70c8ad`, `0x70c997`, `0x70c9ae`, `0x70ca20`
Confidence: verified
Proof: one
Test: `models::scale_add_difftest`

`0x70c3b7` compares the kind with 2 and `jg 0x70c9ae`. `D` starts as instance `+0x1a0` and `E` as
`+0x1ac` (`0x70c4d3`); blend 5 sets `D` = 0 and `E` = 1, blend 6 `E` = 0.5; others multiply `D` by
the colour state at unit `+0x8` when it is below header `+0x54` (`0x70c60a`); a clear lit flag
`+0x3328` adds `D` into `E` and zeroes `D` (`0x70c657`). The pixel lane writes `+0x32c8` and
`+0x32d8` (`0x70c696`), the vertex lane `+0x210` and `+0x220` (`0x70c712`). The fixed lane clamps
each channel and the alpha to [0, 1], packs `ftol(x·255 + 0.5)`, and sets state 1 (`0x70c8ad`) and
state 2 with alpha byte 0 (`0x70c997`). Kinds above 2 pack `(a << 24) | 0xffffff` into state 1
(`0x70ca20`), 0 into state 2 and 0.0 into state 4.

Of `0x70c190`, `scale_add_difftest` runs `0x70c631`–`0x70c69c`.

## Model specular

A model draw sets no specular colour. Its material step writes the device colour states 1 (diffuse) and 2
(emissive) and the shininess state 4 only, and the light's third colour, which the commit hands the
device as the specular of light 0, is filled only by the directional branch of the light add, from a
light of the model's own. The world light's apply adds to the ambient and the diffuse sums alone. The
specular colour state 3 is set at twelve sites, all in the map code between `0x685080` and `0x6bfab1`.

Addresses: `0x70c8ad`, `0x70c997`, `0x70ca20`, `0x71bf60`, `0x71c0ed`, `0x71c7a8`, `0x6a8050`, `0x6a7300`, `0x685080`, `0x68522f`, `0x6b50a4`, `0x6b50f8`, `0x6b55db`, `0x6b560f`, `0x6b6672`, `0x6b68b4`, `0x6bf307`, `0x6bf562`, `0x6bfa3f`, `0x6bfab1`, `0x589e00`
Confidence: verified
Proof: one
Test: none

The scan is every `call 0x589e00` with the nearest earlier `mov ecx,N` in eight instructions: N = 3 at the
twelve map sites, N = 1 at `0x70c8ad` and `0x70ca20` and N = 2 at `0x70c997` in the model code (the
control). `0x71bf60` has one caller in the image, `0x71c0ed`, which follows `0x71bc70` and `0x71bce0` for
the light's `+0x30` and `+0x3c`; `0x71c7a8` passes `+0x6c` of the record as the third colour. The
world light apply `0x6a8050` calls `0x71bc70`, `0x71bce0` and `0x71bf90`, and `0x6a7300` the same
three.

## Batch draw

A batch element binds its textures, lighting and material. With vertex shaders it loads the
submesh's bones as a palette of 4×3 matrices and selects the batch's shaders; otherwise it fills a
vertex buffer by skinning the submesh on the CPU. It draws the submesh's triangles as an indexed
triangle list, from the instance's own index buffer when the instance builds its own batches.

Addresses: `0x70cb30`, `0x719930`, `0x719ac0`, `0x719b20`, `0x71dab0`, `0x71dc80`, `0x70cc14`, `0x70ce72`, `0x70cf1e`
Confidence: verified
Proof: one
Test: none

The palette takes header `+0x90` lookups from submesh `+0xe` for `+0xc` bones, each a transposed 4×3
of the instance's bone matrix: bone `j` fills vertex constants c(31 + 3j) to c(33 + 3j) with the
matrix's columns 0 to 2 (`0x70cc14`). The vertex shader is cache `+0x102c[elem+0x38]`, the pixel
shader `+0x1030[elem+0x3c]` when the pixel flag is set, which needs cache bit 0x10 and an id other
than −1. With vertex shaders the shared model's vertex buffer is bound (`0x70ce72`); without, a
batch with two distinct texture coordinate sets fills through `0x719ac0`, others through `0x719b20`.
The draw passes the start index `+0x8` and count `+0xa` of the submesh, and the vertex range from
submesh `+0x4` for `+0x6` vertices, or from 0 without shaders (`0x70cf1e`).

## Software skinning

A vertex is 48 bytes: position, four weight bytes, four bone index bytes, normal and texture
coordinates. Its matrix is the bone matrices summed by weight `w/255`, starting from the first
weight and stopping at the first zero weight; a vertex with the same weights and indices as the
previous one reuses its matrix. The output is 32 bytes: position, normal and the texture coordinates
copied. Two routines do this, and the processor picks one when the first instance is built: on a
processor with MMX and SSE, every one the client runs on today, the SSE routine `0x71a9e0`; else
the x87 routine `0x71a720`, which the SSE routine also calls for an output buffer not 16-byte
aligned.

| routine | arithmetic | position, per column | normal, per column |
|---|---|---|---|
| SSE `0x71a9e0` | single precision, `mulps`/`addps` | `((x·m0 + m3) + y·m1) + z·m2` | `(nx·m0 + ny·m1) + nz·m2` |
| x87 `0x71a720` | the FPU's 53-bit precision, partial sums stored as floats | `((m20·z + m10·y) + m00·x) + m30` | `(m00·nx + m20·nz) + m10·ny` |

Addresses: `0x71a720`, `0x71a9e0`, `0x71a460`, `0xcf04c8`, `0x70dd70`, `0x8026c8`, `0x71aaa6`, `0x71aabc`, `0x71aba1`, `0x71abcd`, `0x429bfc`, `0x429c06`
Confidence: verified
Proof: one
Test: `models::m2_vertex_skin_difftest`, `models::m2_vertex_transform_difftest`, `models::m2_vertex_transform_sse_difftest`

The constructor `0x70dd70` stores `0x71a720` in `0xcf04c8` and replaces it with `0x71a9e0` when
`OsGetCpuCapabilities()` has bits 2 and 4, which `0x429b90` sets from CPUID leaf 1 EDX bits 23
(MMX, `0x429bfc`) and 25 (SSE, `0x429c06`). `0x71a9e0` widens the weight bytes with `punpcklbw`,
converts them with `cvtpi2ps` (`0x71aaa6`) and multiplies by 1/255 (`0x71aabc`); each bone's four
rows are scaled with `mulps` and added in weight order. The position starts as `x·row0 + row3`
(`0x71aba1`), the normal as `nx·row0` (`0x71abcd`), and the results are written with `movntps`.
`0x71a720` scales by `fild` of the byte times 1/255 (`0x8026c8`) and accumulates the twelve used
entries of each 64-byte bone matrix. `0x71a460` writes 40-byte vertices with a second coordinate set
on the x87 path.

`m2_vertex_skin_difftest` runs `0x71a460` whole; `m2_vertex_transform_difftest` runs `0x71a720`
whole; `m2_vertex_transform_sse_difftest` runs `0x71a9e0` whole.

## Ribbon, particle and custom draws

A ribbon, particle or custom element sets its material state, offsets the view to the emitter's
frame, draws, and restores the view and texture matrix. A particle element's material takes its
blend from the emitter record and the flags 4, plus 1 when the record's flag 1 is set, plus 2 when
its flag 8 is set, plus 0x10 when its blend is above 1.

Addresses: `0x70ca50`, `0x70cb10`, `0x70d820`, `0x70d8b0`, `0x70d960`, `0x7b80c0`, `0x7b4b40`
Confidence: verified
Proof: one
Test: `models::model_matrix_translate_difftest`

`0x70ca50` runs the texture, lighting and material setup, adds `R·offset` to the view's translation
(`((m20·oz + m10·oy) + m00·ox) + m30` per column) and sets it. `0x70d820` draws
`0x7b80c0(ribbon, reference frame)`, `0x70d8b0` draws `0x7b4b40(emitter, reference frame)` after
building the material from the record's `+0x4` flags and `+0x28` blend, and `0x70d960` calls the
instance's draw callback `+0x1b8`. `0x70cb10` ends each.

Of `0x70ca50`, `model_matrix_translate_difftest` runs `0x70ca9b`–`0x70caef`.

## Custom element light

A custom element (kind 5) is lit by the collector of its own model, staged just before its callback
runs, so the result does not depend on draw order. The element uses the context's blank material
(flags 0, blend 0), which is lit, and a lane value of 0, so `0x70baf0` always takes the
fixed-function commit `0x71c730`. A model's collector `+0x3b8` is its own `+0x204` or, for a model with
a parent whose flag `+0x4` bit 0 is clear, the parent's. A custom element on a held item whose parent link is
the wearer draws under the wearer's light. The redundant-state skip never holds for a non-null collector. Emission is set to 0.

Addresses: `0x70d960`, `0x70d982`, `0x70d98d`, `0x70d99a`, `0x70ca50`, `0x70ca61`, `0x70ca68`, `0x70ca30`, `0x70baf0`, `0x70bb0a`, `0x70bb7a`, `0x70bb96`, `0x70bba4`, `0x70bdf6`, `0x70b281`, `0x70b2c9`, `0x70b2d0`, `0x70b5e0`, `0x70b607`, `0x70b68c`, `0x70b6aa`, `0x718960`, `0x718982`, `0x718995`, `0x7189a9`, `0x710db0`, `0x70e0b8`, `0x70e786`
Confidence: verified
Proof: one
Test: none

`0x70d982` calls `0x70ca50`, which runs `0x70b740`, `0x70baf0` and `0x70c190`; only then
`0x70d99a` calls `[model + 0x1b8]` with `[model + 0x1bc]` (stored by `0x710db0`). No other handler of
kind 5 stores `[ctx + 0x3348]`, so the loop's default `&[ctx + 0x3358]` (`0x70b5e0`) stays, and its
two words are stored only as 0 (`0x70b2c9`, `0x70b2d0`). `[ctx + 0x32f0]` is zeroed each element
(`0x70b607`) and stored only by kinds 0 and 2 (`0x70cb71`, `0x70d371`). `0x70bb96` falls through
for lit, `0x70bba4` jumps to `0x70bdf6`. The skip at `0x70bb7a` compares the current collector with
`[ctx + 0x3324]`, which only the constructor stores (0, `0x70b281`); `0x70b68c` reads it back. The
collector is stored by `0x718982` to `0x7189a9`; the constructor and `0x70e786` store 0. Kinds 3,
4 and 5 commit state 2 as 0 at `0x70ca30`.

## Grouped and projected draws

A batched run draws several instances of one shared model with one state setup, loading each
instance's palette into constants. A special batch builds a texture matrix from its skinned depth
range widened by 6.0 at each end and calls the scene's special callback around its draw.

Addresses: `0x70d330`, `0x70cf70`, `0x719c20`, `0x71dfa0`
Confidence: inferred
Proof: none
Test: none

`0x70cf70` calls `0x719c20` for the submesh bounds, subtracts 6.0 from the minimum z and adds it
to the maximum, and calls scene `+0x11c`. The per-instance constant layout of `0x70d330` was not
transcribed.

## Projected bounds

`0x719c20` returns 0 for a submesh of fewer than four vertices. Otherwise it skins each vertex as
software skinning does, multiplies the blend matrix in place by the scene matrix `+0xdc`, and keeps
the least and greatest of `((m00·x + m20·z) + m10·y) + m30` and its y and z columns. The first three
also keep their position less the scene's `+0x10c` and their texture coordinates. The blend writes
entries 0 to 2 of each row; the last entry of each row is seeded (0, 0, 0, 1) once and written only
by the product, so each vertex's product reads the previous vertex's. It stays (0, 0, 0, 1) while
the scene matrix's last column is (0, 0, 0, 1).

Addresses: `0x719c20`, `0x719c2d`, `0x719c5f`, `0x719dcb`, `0x719e04`, `0x719f45`, `0x719f59`, `0x71a00b`, `0x71a016`, `0x71a093`, `0x74a7c0`, `0x754a66`, `0x8026c8`
Confidence: verified
Proof: one
Test: `models::m2_decal_projection_difftest`

`0x719c5f` seeds the 16 floats at `ebp−0x54` to identity before the loop. `0x719e04` scales the
first weight byte by 1/255 unconditionally and the loop at `0x719e80` adds the next ones until a
zero, each times twelve entries of the bone matrix at model `+0x94` plus 64·index. `0x719f45` calls
`0x74a7c0(B, B, scene + 0xdc)`. `0x719dcb` compares the weight word `+0xc` and the index word
`+0x10` as integers with the previous vertex's, both starting at 0, and on a match skips to
`0x719f59` with the matrix kept; a first vertex with both words 0 keeps the seeded identity. The
bounds start at ±3.402823e38 and each compare skips on equal or NaN. The x sum stays unrounded on
the FPU stack into its compares and the `+0x10c` subtraction (`0x71a016`); y and z go through
floats. The three corners feed the code from `0x71a093`.

`m2_decal_projection_difftest` runs `0x719c20` whole. The slot `0x876504` that `0x74a7c0` jumps through
is filled by the exe's own selection `0x74fb49`, which leaves `0x754a66`; the product in `render.rs` is
that routine's transcription.
