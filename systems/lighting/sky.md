# lighting: sky

The sky the world draws behind everything: the gradient dome and its colours, the sky pass that
draws the stars, discs, gradient and clouds in a far depth slice, and the skybox models that
replace them. The dome is the object at `0xce99a0`; the skybox entries are the hash table at
`0xce99fc`.

Roots: `0x6d4940`, `0x6d20b0`, `0x6d4810`

## Sky dome

The dome is a cap of one apex vertex and five rings of 24 columns, built once at radius 1.0. Ring k
lies at polar angle θk = 0, 0.17π, 0.20π, 0.23π, 0.24π, 0.25π from the zenith, column c at
φ = 2π·c/24, and a vertex is (sin φ·sin θ·r, cos φ·sin θ·r, cos θ·r − cos 0.25π), so the rim sits
at the eye's height. The apex ring stops after one column. The index strip pairs, for each ring
after the apex and c = 0 to 24, the previous ring's column c mod 24 with this ring's, the apex
always 0: 250 indices, with the vertex count 121 and the index count 300 stored for the draw.

Addresses: `0x6d0d10`, `0x6d0d28`, `0x6d0daa`, `0x81152c`, `0x811540`, `0x8115bc`, `0x8026bc`, `0x6d1c9f`
Confidence: verified
Proof: one
Test: none

`0x6d0d10(r)` stores 24 columns at `+0x24` (`0x6d0d28`), sizes the vertex array to 168, the colours
to 168 and the indices to 300 (`0x6d0daa`). It walks the six floats at `0x81152c` times π, with
cos(`[0x811540]`·π), 0.25π, as the drop, and the column angle c·(1/24)·π doubled. A ring whose
angle is within `[0x8026bc]` of 0 or of π breaks after its first vertex. The index count of a ring
is skipped for the pole. The initializer calls it with r = 1.0 (`0x6d1c9f`).

## Sky colours

The table build colours the dome each frame. With g the glow track times table `+0x50`, G[k] is
band 2 + k moved toward band 3 by g, k = 1 to 4. The apex is band 2 through the HSV scale at 1.0.
Ring k at column c takes band 2 + k varied by w, the dome track at phase heading/2π + 0.25 − c/24
wrapped into [0, 1): w ≥ 0 moves it toward G[k] by (1 − w)·g; w < 0 gives G[k] moved by −w·g
toward G[k] moved toward band 2 by 0.7·g. The rim and one more colour are band 7. The rings take
the screen tint (`sky-colour-tracks`).

Addresses: `0x6d0f50`, `0x6d0f7b`, `0x6d0f87`, `0x6d0ff7`, `0x6d1015`, `0x6d10af`, `0x6d10e3`, `0x6d10ef`, `0x6d1111`, `0x6d1117`, `0x6d111d`, `0x6d115e`, `0x6d117a`, `0x6d118a`, `0x6d11a2`, `0x6d11c6`, `0x6d125c`, `0x6d1282`, `0x6d1297`, `0x6d12c1`, `0x6d12c9`, `0x6d13bc`, `0x6d1484`, `0x6d14de`, `0x6d1508`, `0x6d1550`, `0x6d5270`, `0x8030a0`, `0x811614`, `0x7ffd7c`
Confidence: verified
Proof: one
Test: none

`0x6d0f7b` evaluates `0xce9b2c` and `0x6d0f87` multiplies by `[0xce9c5c]`. G is built per channel
by `0x6d15f0(band 3 + i, band 3, g)` with round(f32 − 0.5) and packed by `0x6d5270`. The apex is
`0x6d1620(band 2, 1.0)` (`0x6d10e3`). The phase is `[0xce9b90]·[0x811614]` (1/2π) + 0.25, less 1
when above 1 (`0x6d111d`), plus 1 when below 0 (`0x6d115e`), stepping by −1/24 (`0x8030a0` over the
column count). `0x6d118a` takes the w < 0 arm. The w ≥ 0 lerp is `0x6d15f0(band, G, (1 − w)·g)`
(`0x6d11c6`); the other arm builds P = `0x6d15f0(G, band 2, 0.7·g)` (`0x7ffd7c`) and then
`0x6d15f0(G, P, −w·g)` (`0x6d13bc`); a byte scale it computes first at `0x6d12c1` is overwritten.
Each takes `0x690ae0` with the tint (`0x6d1282`). The rim loop is `0x6d1508`.

## Sky colour tracks

The sky colours read two tracks of six keys, built at start-up:

| track | keys |
|---|---|
| `0xce9b2c`, glow | 03:00 0, 06:30 1, 07:00 0, 20:30 0, 21:30 1, 23:59 0 |
| `0xce9af8`, around the dome | 0.125 1, 0.375 0, 0.5 −0.5, 0.625 −0.7, 0.75 −0.5, 0.875 0 |

Addresses: `0x6ce120`, `0x6ce210`, `0xce9b2c`, `0xce9af8`
Confidence: verified
Proof: one
Test: none

`0x6ce120` stores the glow track's six keys and `0x6ce210` the other's, each a time and a value,
at start-up; `0x6d0f7b` and the column loop evaluate them through `0x6cf6c0`.

## Sky pass

When the sky is on, the pass sets the depth range to [0.975, 0.98] and draws, in order, the stars,
the sun disc, the white moon, moon02, the gradient dome and the clouds, unless a loaded skybox has
a weight above 0.99. It then draws the skyboxes and restores the viewport. The world scene runs it
before the terrain and map objects when the camera is dry, and the low-detail areas run it too.

Addresses: `0x6d4940`, `0x6d494d`, `0x6d4995`, `0x6d49a6`, `0x6d49c1`, `0x6d49cd`, `0x6d49ee`, `0x6d4a21`, `0x6d4a3d`, `0x6d4a44`, `0x6d4a4e`, `0x6d4a58`, `0x6d4a62`, `0x6d4a6c`, `0x6d4a76`, `0x6d4c6d`, `0x80fec0`, `0x80fec4`, `0x808aac`, `0x681070`, `0x6812ac`, `0x6bddcf`, `0x58b060`, `0x58af60`, `0x710450`
Confidence: verified
Proof: one
Test: none

`0x6d494d` returns when `[0xce9cf8]` is 0. `0x58b060` reads the viewport (`0x6d4995`); `0x6d49c1`
sets it with depth `[0x80fec0]` − `[0x80fec4]` to `[0x80fec0]` (0.98 − 0.005, 0.98). A skybox counts
when its entry has a model, `0x710450` reports it ready and its weight is above `[0x808aac]` (0.99)
(`0x6d49ee`, `0x6d4a21`). The draws are `0x7e6120` on `0xce96d8`, `0x7e5b90` on the three discs,
`0x7e5e80` on `0xce99a0` and `0x7e5f80` on `0xce98e8` (`0x6d4a44` to `0x6d4a76`). `0x681070`
reaches `0x6812ac` only when the liquid word is 0xf.

## Gradient draw

`0x7e5e80` draws the dome with its vertex colours as a strip of 300 indices, with render states
0xe, 0xf, 0x14 and 0x12 at 0 and state 7, the blend, at 3, after giving the device transform
slot 8 the vector (6.6666665, 6.6666665, 6.6666665).

Addresses: `0x7e5e80`, `0x7e5ea5`, `0x7e5eba`, `0x7e5ecb`, `0x7e5efe`, `0x7e5f1e`, `0x7e5f2a`, `0x7e5f49`, `0x7e5f5d`
Confidence: verified
Proof: one
Test: none

`0x58b200(8)`, `0x58b230(8)` and `0x58b260(8, &(0x40d55555 ×3))` (`0x7e5eba`) are the transform
calls; what slot 8 holds is not read. The states are set at `0x7e5ecb` to
`0x7e5efe`. `0x58a3d0` takes `+0x8` and `[+0x28]` vertices and the colours `+0x14` at stride 4;
`0x58a750` the indices `+0x20` and `[+0x2a]`; `0x58a830` draws type 4 from 0 (`0x7e5f5d`).

## Skyboxes

Three skybox slots can draw: the override slot (the override light's, weight 1.0),
and the two pair slots from the table, of which the world sets slot 0 to the map object group's
skybox path at the interior fog weight and clears slot 1. Each slot with a loaded model and a
weight above 0 draws animated and visible at alpha = weight, placed at the camera with no
translation, lists 0 and 1, after which the device clears with flag 2. An override slot at weight
1.0 or more skips the pair slots. The scene clock advances by the milliseconds since the last pass.

Addresses: `0x6d4a88`, `0x6d4a9e`, `0x6d4acc`, `0x6d4b0b`, `0x6d4b2b`, `0x6d4b4f`, `0x6d4b5c`, `0x6d4b69`, `0x6d4b87`, `0x6d4bba`, `0x6d4bf6`, `0x6d4c50`, `0x681282`, `0x681292`, `0x68129f`, `0x6b4301`, `0xca8080`, `0x7074b0`, `0x707680`, `0x708900`, `0x710cb0`, `0x58a970`
Confidence: verified
Proof: one
Test: none

`0x7074b0` advances `0xce9d54` (`0x6d4a9e`). `0x6d4acc jp` skips the pairs at an override weight of
1.0 or more. A pair draws when its weight passes `fcomp 0.0`, `test ah,0x41` (`0x6d4b0b`):
`0x710b90(1)`, `0x710c50(1)`, `0x710cb0(weight)` (`0x6d4b2b`), `0x707680` with a zeroed vector
(`0x6d4b4f`), `0x708900(0)` and `(1)`, the flags back to 0, and `0x58a970(2)` (`0x6d4b87`). The
override repeats it (`0x6d4bba` to `0x6d4c50`). `0x681292` calls `0x6d4810(0, [0xca8080], state
+0x7c)` and `0x68129f` `0x6d4810(1, 0, …)`; the map object walk stores `0xca8080` at `0x6b4301`.

## Sky shutdown

At a map unload the sky releases every interned skybox's model and frees its entry, releases the
skybox scene and clears the three slot handles; the first time, it also releases the cloud
texture, the three disc textures, both glares and the star model.

Addresses: `0x6d20b0`, `0x6d2101`, `0x6d2178`, `0x6d218c`, `0x6d219a`, `0x6d21a6`, `0x6d21ac`, `0x6d21b2`, `0x6d21b9`, `0x6d21c3`, `0x697ac0`, `0x697bd5`, `0x7103a0`, `0x707320`
Confidence: verified
Proof: one
Test: none

`0x697ac0` calls `0x6d20b0` (`0x697bd5`). For each entry it calls `0x7103a0` on `+0x18`
(`0x6d2101`), unlinks the entry and calls the table's slot 0 on it (`0x6d2178`); then `0x707320` on
`0xce9d54` (`0x6d218c`) and zeroes `0xce9bbc`, `0xce9d54`, `0xce9bb4` and `0xce9bc0`. With
`[0xce9d5c]` set (`0x6d21b2`) it calls `0x6d08c0`, `0x7e5e70` on each disc, `0x7e57c0` on each glare
and `0x7e61c0` on the stars, and clears the flag. The weights are not cleared.

## Skybox table

The interned skybox entries live in a hash table created with four buckets on first use; each
entry holds the path's hash at `+0x0`, the path at `+0x14` and the model at `+0x18`. The table's
constructor, destructor and growth are the template code of the day-night file.

Addresses: `0x6ce760`, `0x6ce830`, `0x6d4da0`, `0x6d4e10`, `0x6d4ed0`, `0x6d5090`, `0x6d55a0`, `0x6d5820`, `0x8115c4`
Confidence: inferred
Proof: none
Test: none

`0x6ce760` and `0x6ce830` are the static constructor and destructor; the method table `0x8115c4`
holds `0x6d4da0`, `0x6d4e10`, `0x6d5090` and `0x6d4ed0`. The entry fields are the ones `0x6d4640`
reads and writes (`tables#skybox-models`); the growth `0x6d55a0` and `0x6d5820` are not read.
