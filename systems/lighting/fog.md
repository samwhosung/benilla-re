# lighting: fog and the world light

What the world takes from the light table each frame: the light colours derived from it, the scene
fog, the interior fog the camera crossfades into, and the device light and fog states the world
commits. The world's sky update `0x66ff60` (terrain) runs the table build, then the fog update,
then commits the colours.

Roots: `0x6cee30`, `0x6d4910`, `0x6d4920`

## Light colours

After each table build `0x6d37e0` derives the light colours at state `+0x168` onward: diffuse
(`+0x174`) is colour band 0 and ambient (`+0x178`) band 1. The window diffuse (`+0x17c`) is the
8-bit blend of band 0 toward band 1 at a = 0x80 with a zero top byte; the window ambient (`+0x180`)
is band 1 blended toward band 0 at 0x80, then 16 added to each channel saturating at 255, top
byte 0xff. `+0x184` to `+0x190` hold ambient with its HSV saturation ×0.33 and value ×1.25 as
three floats and 1.0, and `+0x194` is band 1 with each channel (c + 3)·85 >> 8 under the red byte
of band 8.

Addresses: `0x6d37e0`, `0x6d37f3`, `0x6d37f8`, `0x6d3804`, `0x6d380a`, `0x6d381a`, `0x6d3824`, `0x6d382c`, `0x6d3856`, `0x6d385f`, `0x6d38c5`, `0x6d3945`, `0x6d3998`, `0x6d39a0`, `0x6d39b2`, `0x6d39bb`, `0x6d39f7`, `0x6d51e0`, `0x6d51f1`, `0x6d5264`, `0x808300`, `0x80c5d0`
Confidence: verified
Proof: one
Test: `lighting::dn_channel_lerp8_difftest`, `lighting::dn_fog_ambient_darkened_difftest`, `lighting::dn_fog_derive_difftest`

`0x6d37e0` stores `[0xce9c0c]` at `0xce9cd4` and `0xce9cdc`, `[0xce9c10]` at `0xce9cd8` and
`0xce9ce0` (`0x6d37f3` to `0x6d380a`). `0x6d51e0(dst, a, src)` is the 8-bit blend whose result has
a zero top byte (`0x6d5264`) and which copies the whole source dword at a = 255 (`0x6d51f1`). The
sum `x + 0xff101010` with the carries `(x ^ sum ^ 0xfffefeff) & 0x1010100` removed and their
borrows spread into a mask (`0x6d3824` to `0x6d3856`) adds 0x10 to each low byte with saturation
and 0xff to the top byte. `0x6d38c5` is the first `imul 0x55`. The HSV pass is `0x7bbc80`,
`fmul [0x808300]` (0.33) and `[0x80c5d0]` (1.25), then `0x7bbd60` (`0x6d39bb`).

## Light snapshot

`0x6d37e0` copies a snapshot to state `+0x13c` before it computes the HSV colour: the sun direction,
diffuse, ambient, the two window colours, and the HSV colour and its 1.0 as the previous frame left
them.

Addresses: `0x6d3864`, `0x6d386f`, `0x6d3879`, `0x6d3883`, `0x6d388d`, `0x6d3899`, `0x6d38a5`, `0x6d38b1`, `0x6d38c8`, `0x6d38d4`, `0x6d38f0`, `0x6d38fc`
Confidence: verified
Proof: one
Test: `lighting::dn_fog_ambient_darkened_difftest`, `lighting::dn_fog_derive_difftest`

The copies `0xce9c9c` to `0xce9cc4` from `0xce9cc8` to `0xce9cf0` run at `0x6d3864` to `0x6d38fc`,
before the stores of `0xce9ce4` to `0xce9cf0` at the function's end, so the snapshot's last four
dwords lag one build.

## Scene fog

The scene fog is end = min(scalar band 0, far clip), start = scalar band 1 × end, and colour band 7,
at state `+0x78`, `+0x74` and `+0x70`. A NaN band end takes the far clip. Scalar band 0 is already
multiplied by f32(1/36) (`tables#fog-distances-at-load`); the start is a fraction of the end and can be
negative.

Addresses: `0x6cee30`, `0x6cee3c`, `0x6cee47`, `0x6cee61`, `0x6cee6d`, `0x6cee76`, `0x6cee83`, `0xce9bd0`, `0xce9bd4`, `0xce9bd8`, `0xce9b94`
Confidence: verified
Proof: one
Test: `lighting::dn_blend_difftest`, `lighting::dn_blue_premul_difftest`, `lighting::dn_case2_premul_difftest`, `lighting::dn_fog_color_lerp_difftest`, `lighting::dn_phase_alpha_delta_difftest`, `lighting::dn_phase_fade_arg_difftest`, `lighting::dn_repack_scale_difftest`

`fld [0xce9c50]`, `fcomp [0xce9b94]`, `test ah,5`, `jp` takes the far clip for band end ≥ far clip
or unordered (`0x6cee47`). `fld [0xce9c54]`, `fmul [0xce9bd8]`, `fstp [0xce9bd4]`; `[0xce9c28]` to
`0xce9bd0` (`0x6cee76`).
The tests run `0x6cee30` in 7 slices between `0x6cee61` and `0x6cf17e`.

## Interior fog

The camera crossfades into the fog of the map object group the active player stands in. When the
world's fog query returns 1, the target is the group's record 1 in water or ocean, in magma with
query flag 0x100 and in slime with 0x10, else record 0; magma and slime without the flag keep the
scene fog. The target's end is min(record end, far clip), its start the record's fraction of it;
in magma or slime the scene fog takes the target too. The weight `+0x7c` moves 0.25 per second
toward 1 while the query holds, else toward 0. The drawn fog `+0x80` to `+0x88` is the scene fog
moved toward the target by it.

Addresses: `0x6cee97`, `0x6ceeaa`, `0x6ceeb9`, `0x6ceec1`, `0x6ceed3`, `0x6ceedb`, `0x6cef0c`, `0x6cef12`, `0x6cef36`, `0x6ceeee`, `0x6cef01`, `0x6cef56`, `0x6cef68`, `0x6cef6f`, `0x6cef90`, `0x6cefb5`, `0x6cefc2`, `0x6cefcb`, `0x6cefe8`, `0x6ceff9`, `0x6cf011`, `0x6cf034`, `0x6cf054`, `0x6cf069`, `0x6cf093`, `0x6cf0c4`, `0x6cf310`, `0x6705d0`, `0x8115b0`
Confidence: verified
Proof: one
Test: `lighting::dn_blend_difftest`, `lighting::dn_blue_premul_difftest`, `lighting::dn_case2_premul_difftest`, `lighting::dn_fog_color_lerp_difftest`, `lighting::dn_phase_alpha_delta_difftest`, `lighting::dn_phase_fade_arg_difftest`, `lighting::dn_repack_scale_difftest`

The colour moves by the byte bits(f32(w·255 + 512)) >> 14 of the weight.

`0x6cf310` initializes two {end, fraction, colour} records in a stack block, which `0x6705d0`
fills with the query's flags and records, an out-byte and an owner (`0x6ceeaa`); a result other than 1 goes to the fade (`0x6ceeb9`). The
liquid word, masked with 3 unless 0xf (`0x6ceec1`), picks the index at `0x6ceed3` to `0x6cef41`.
`0x6cef01 jp` takes the far clip for end ≥ far clip. A set out-byte stores the owner at
`0xce9bf8` and `0xce9c08` (`0x6cefc2`) and ramps `0xce9bdc` by `[0x8115b0]` (0.25) times the frame
time; a clear one ramps down (`0x6cf011`), and at or below 0 clears `0xce9bf8`, `0xce9c00` and the
weight (`0x6cf034`). `0x6cf054` to `0x6cf093` lerp end and start; `0x690ae0` blends the colour only
for a nonzero byte (`0x6cf0c4`).
The tests run `0x6cee30` in 7 slices between `0x6cee61` and `0x6cf17e`.

## Dead fog modifiers

The fog update has two modifiers that never run: one that scales both fogs' colours by a byte, and
one that fades both fogs toward a colour over a time window, shrinking start and end. Their gate,
the flag word `0xce99f8`, is only ever cleared. `0x6d4910`, the world's call when the sun glare
dims the light, does nothing.

Addresses: `0x6cf0c9`, `0x6cf0ef`, `0x6cf11a`, `0x6cf138`, `0x6cf17c`, `0x6cf1a0`, `0x6cf290`, `0x6d4920`, `0x6d4910`, `0x6d1d2d`, `0x6d1bba`, `0xce99f8`, `0xce99d0`, `0x454d40`
Confidence: verified
Proof: one
Test: `lighting::cloud_color_modulate_difftest`, `lighting::dn_blend_difftest`, `lighting::dn_blue_premul_difftest`, `lighting::dn_case2_premul_difftest`, `lighting::dn_fog_color_lerp_difftest`, `lighting::dn_phase_alpha_delta_difftest`, `lighting::dn_phase_fade_arg_difftest`, `lighting::dn_repack_scale_difftest`, `lighting::fog_color_modulate_difftest`

A census of `0xce99f8` in the listing finds seven instructions: tests at `0x6cf0c9`, `0x6cf0ef`,
`0x6d0f80` and `0x7e601e`, the store of ebx at `0x6d1d2d`, which `0x6d1bba` zeroed, and the load
and store of `0x6d4920`, which clears bit cl. The control, the light reference `0xce9bb0`, finds its
known four. No code holding the sky gradient's address `0xce99a0` writes its `+0x58`. With bit 1,
`0x6cf290` would scale each channel by the byte of 1 − `[0xce99d4]`; with bit 0, `0x6cf1a0` would
take k = 1 − 1/`0x454d40`(((clock − `[0xce99e8]`) / `[0xce99d8]`)·7.2134752), clamped, into
`0xce99d0` (`0x6cf17c`). `0x6d4910` is `ret 8`.
The tests run `0x6cee30` in 7 slices between `0x6cee61` and `0x6cf17e`.

## World light commit

After the fog update `0x66ff60` dims ambient and diffuse by the sun glare and writes the world's
device light record: its direction is the sun direction and its three colours (`models/lights#light-object`)
are ambient, diffuse and colour band 9, each channel times 1/255. The dim is a = the byte of
f32((1 − f)·255 + 512) >> 14 with f = 0.35·g, g the sun glare's look factor
(`celestial#glare-update`); each channel becomes (c·a + 0xff) >> 8, so f = 0 is the identity.
On the full-recompute path f is 0.

Addresses: `0x66ff60`, `0x66ff7d`, `0x66ffa9`, `0x66ffaf`, `0x66ffc3`, `0x66ffde`, `0x66ffed`, `0x66fff8`, `0x670026`, `0x67004a`, `0x67008b`, `0x6700a3`, `0x6700a8`, `0x6700f4`, `0x67014e`, `0x670750`, `0x672590`, `0x71b6a0`, `0xc89ef0`, `0x7ffd70`, `0xce984c`
Confidence: verified
Proof: one
Test: none

`0x66ff60(flag, eye)`: with the flag set it stamps the eye and recomputes with cl = 1 (`0x66ff7d`);
otherwise `fld [0xce984c]`, `fmul [0x7ffd70]` (0.35) (`0x66ffaf`) and, nonzero, `0x6d4910`
(`0x66ffde`). It passes state `+0x194` to `0x672590` (`0x66ffed`), dims `+0x178` through
`0x670750` (`0x670026`) and `+0x174` inline (`0x67004a` to `0x67008b`), sets the direction of
`[0xc89ef0]+0x7c` from `+0x168` (`0x6700a3`), and writes `+0x30` from `+0x178`, `+0x3c` from
`+0x174` and `+0x48` from `+0xcc`, byte 2 first, each `fild` times 1/255 (`0x6700a8`, `0x6700f4`,
`0x67014e`).

## Fog states

`0x66ff20` commits the scene fog to the device: start, end and colour as fog states 0xa, 0xb and
0xd. The start goes as it is, so a negative start fraction in a light record
fogs the near field too. The map object batches commit the interior-blended fog `+0x84`, `+0x88` and `+0x80` the same
way, and a material with flag 0x20 draws with the window colours: the device light's diffuse takes
`+0x17c` and its ambient `+0x180`, and both return to diffuse and ambient after the batch.

Addresses: `0x66ff20`, `0x66ff28`, `0x66ff36`, `0x66ff44`, `0x6b5190`, `0x6b51cd`, `0x6b51de`, `0x6b51ef`, `0x6b52f7`, `0x6b530b`, `0x6b5370`, `0x6b53bc`, `0x6b56b7`, `0x6b5723`, `0x6b5765`, `0x71b7c0`
Confidence: verified
Proof: one
Test: none

`0x66ff20` pushes `[state+0x74]` to state 0xa, `[state+0x78]` to 0xb and `[state+0x70]` to 0xd.
`0x6b5190` does the same with `+0x84`, `+0x88` and `+0x80` (`0x6b51cd` to `0x6b51ef`), tests the
material's 0x20 (`0x6b52f7`), writes `+0x17c` to the light's `+0x3c` and `+0x180` to `+0x30`
(`0x6b530b`, `0x6b5370`), commits through `0x71b7c0`, and after the batch writes `+0x174` and
`+0x178` back (`0x6b56b7`, `0x6b5723`, `0x6b5765`).

## Water colours

The table's `+0x68` to `+0x74`, LightParams columns 5 to 8, reach the liquid draws as bytes
the bits of f32(v·255 + 512) >> 14: the liquid texture pass reads `+0x68`, `+0x70` and `+0x74`,
and the map object liquid ramp `+0x68` and `+0x6c`.

Addresses: `0x68a830`, `0x68a87d`, `0x68a895`, `0x68a8b3`, `0x6b6b60`, `0x6b6b6a`, `0x6b6b7f`
Confidence: verified
Proof: one
Test: none

`0x68a830` loads state `+0x11c`, `+0x114` and `+0x120` (`0x68a87d`, `0x68a895`, `0x68a8b3`), each
`fmul` 255.0, `fadd` 512.0, `fstp` and `shr 0xe`. `0x6b6b60` loads `+0x114` and `+0x118`
(`0x6b6b6a`, `0x6b6b7f`). State `+0x114` is table `+0x68`.
