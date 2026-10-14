# terrain: map lights

The light an entity or a doodad def gives its model while it stands on the map: the callbacks the
model calls, how a map object group's vertex colours become the def's ambient and directional
colour, the point lights of the map and the fog of the group the camera is in. The state that
selects between them is in `mapentities`.

Roots: `0x6a7300`, `0x6a8050`, `0x6c31e0`, `0x6a7650`, `0x6a8410`, `0x69e4c0`, `0x6a7ac0`, `0x69de20`

## Entity lighting

`0x6a7300` adds an entity's light to a model's lighting record (the stack argument). With `+0xc`
bit 4 (lit by the sun) it adds the scene light block's directional colour times the intensity
`+0xa4`, with the block's direction. Otherwise the colour is the entity's `+0xa0` bytes, as byte /
255, times `+0xa4`, and the direction the global one `0xca816c`, or with `+0x90` bit 0x800 the
global direction plus (sun direction − global) × (`+0xa3` / 255), normalized. It also adds the
ambient `+0x9c` and each point light of the entity's list, and sets the interior fog on the record
when the camera's fog claim is the entity's map object.

Addresses: `0x6a7300`, `0x6a730a`, `0x6a7316`, `0x6a7349`, `0x6a7357`, `0x6a738d`, `0x6a73bd`, `0x6a7402`, `0x6a7413`, `0x6a7421`, `0x6a7454`, `0x6a7459`, `0x6a74a3`, `0x6a74fa`, `0x6a751a`, `0x6a7520`, `0x6a7533`, `0x6a7546`, `0x6a75a1`, `0x6a75ee`, `0x6a75fd`, `0x6a7629`, `0x6a79c0`, `0x69d490`, `0x71bc70`, `0x71bce0`, `0x71bf90`, `0x71c110`, `0xca816c`
Confidence: verified
Proof: one
Test: `terrain::apply_map_exterior_difftest`, `terrain::apply_map_b1_dir_scaled_difftest`, `terrain::apply_map_b2_daynight_blend_difftest`, `terrain::apply_map_b3_normalize_difftest`, `terrain::apply_map_b4_b5_colors_difftest`

`0x6a730a` tests `+0xc` bit 1 and gathers the map object lights (`0x6a79c0`); `0x6a7316` bit 4. With
bit 4 the colour is the block's `+0xb8`, `+0xbc`, `+0xc0` times `+0xa4` (`0x6a7357` to `0x6a7388`) and the
direction the block's `+0xa0` (`0x6a73a4`). Without it (`0x6a73bd`) the three bytes `+0xa2`, `+0xa1`, `+0xa0`
are each times 1/255 and then `+0xa4`; the third product is stored first and reloaded
(`0x6a7402`). `0x6a7413` loads `0xca816c` to `0xca8174`; `0x6a7421` tests `+0x90` bit 0x800; the
blend and the normalization (`fsqrt; fdivr 1.0` at `0x6a74fa`, kept in the register) end at
`0x6a7518`. `0x6a7520` asks `0x69d490` for the entity's group; `0x6a7533` and `0x6a7546` test the
state's `+0x7c` for zero and the camera claim `0xc7b748` against the group; then
`0x71c110` gets the fog colour from the state's bytes `+0x82`, `+0x81`, `+0x80` and the range
`+0x84`, `+0x88`. The ambient bytes are `+0x9e`, `+0x9d`, `+0x9c` (`0x6a75a6` to `0x6a75ee`);
`0x6a75fd` adds the directional pair, and `0x6a7608` onward the lights of the list at `+0x84`
with a nonzero `+0x64`.

## Doodad def lighting

`0x6a8050` is the doodad def's light slot. It gathers map object lights when `+0xc` bit 1 is set. With
bit 2 (lit as an interior) it adds the ambient `+0x9c` bytes and the directional `+0xa0` bytes
with the global direction `0xca816c`. Otherwise it adds the scene light block's ambient colour and its
directional colour times `+0xa4`, with the block's direction. Then it adds each point light of
the list at `+0x84` whose `+0x64` is set.

Addresses: `0x6a8050`, `0x6a805a`, `0x6a8066`, `0x6a8073`, `0x6a80bb`, `0x6a80d8`, `0x6a810c`, `0x6a812f`, `0x6a8134`, `0x6a8186`, `0x6a8191`, `0x6a81b6`, `0x71bc70`, `0x71bce0`, `0x71bf90`
Confidence: verified
Proof: one
Test: `terrain::add_lights_interior_difftest`, `terrain::add_lights_exterior_difftest`

`ret 4`. `0x6a8073` reads `+0x9e`, `+0x9d`, `+0x9c`; `0x6a80d8` pushes the global direction;
the exterior leg (`0x6a810c`) reads the block at `0xc89ef0 + 0x7c`: ambient at `+0x30`, direction
`+0x24`, colour `+0x3c` times the intensity `+0xa4` (`0x6a8134`); `0x6a8186` adds the directional.

## Map object fog

`0x6c31e0` is the entity's and the def's fog slot. For an interior-lit entity (`+0xc` bit 2) in a
group def that holds the interior fog claim (`+0x98`), the fog colour is the light state's
block 2 (bytes `+0x82`, `+0x81`, `+0x80`, range `+0x84`, `+0x88`); every other entity gets block 1
(`+0x72`, `+0x71`, `+0x70`, `+0x74`, `+0x78`). It sets the record's fog with `0x71c110`.

Addresses: `0x6c31e0`, `0x6c31ee`, `0x6c3212`, `0x6c3229`, `0x6c3233`, `0x6c327a`, `0x6c32b8`, `0x6c3300`, `0x71c110`, `0x6d48b0`
Confidence: verified
Proof: one
Test: `terrain::fog_select_outdoor_difftest`, `terrain::fog_select_indoor_difftest`, `terrain::fog_select_fallthrough_difftest`

`0x6c31ee` tests `+0xc` bit 2; `0x6c3212` finds the first group def (`+0x8` bit 0x10) among the
entity's nodes, `0x6c3229` loads its `+0x98`; zero or no group goes to `0x6c327a` (block 1);
`0x6c3233` is block 2. A non-interior entity goes to `0x6c32b8` (block 1).

## Water plane

`0x6a7650` is the def's water slot. Unless `+0x90` bit 0x1000 is set it finds the liquid at the
def, indoors the group's liquid height (`0x6b9f10`) moved to world space, outdoors the terrain
liquid (`0x69b6d0`); a hit sets bit 0x20 (0x100 if fishable) and the height `+0x98`, and bit 0x1000 is
set. A def with bit 0x20 sets the record's clip flag `+0x19c` to 1 and its plane `+0x1a0` to
(0, 0, 1, −height).

Addresses: `0x6a7650`, `0x6a7659`, `0x6a7668`, `0x6a76a2`, `0x6a76d7`, `0x6a76e4`, `0x6a7700`, `0x6a7718`, `0x6a772e`, `0x6a7743`, `0x6a774c`, `0x6a7772`, `0x6a777c`, `0x6a7788`, `0x6a7792`, `0x6a79c0`, `0x6b9f10`, `0x69b6d0`, `0x69d4f0`
Confidence: verified
Proof: one
Test: `terrain::clip_plane_negate_difftest`

`0x6a7659` loads `+0x90` and `0x6a765f` tests bit 12. `0x6a7668` masks `0xfffffedf`.
`0x6a76a2` finds the group; `0x6a76d7` calls the group liquid height; type 0xf (`0x6a76e4`) is no liquid.
`0x6a7788` stores 1 at `+0x19c`; `0x6a7792` negates the height for the plane's fourth float.

## Interior light bake

`0x6a8410` (a doodad def) and `0x69e4c0` (an entity) bake a map object group's vertex colour into the
light. They trace down into the group (`0x6a4cc0`, or `0x6a4dc0` for a known face) and take the
surface colour; `0x6a77e0` makes the directional `+0xa0` (largest channel raised to at least 0xa8) and
the ambient (at most 0x60), at `+0x9c` for a def and the target `+0xf4` for an entity. The colour's
alpha w is the sky weight: 255 copies the day light's colour, other nonzero w blends each channel
toward `+0x174` and `+0x178` by w / 256. w is kept at `+0xa3` with `+0x90` bit 0x800.

Addresses: `0x6a8410`, `0x6a8420`, `0x6a8436`, `0x6a8475`, `0x6a84df`, `0x6a84e6`, `0x6a8517`, `0x6a8524`, `0x6a853f`, `0x6a8547`, `0x6a8574`, `0x6a85a8`, `0x6a85e8`, `0x6a860b`, `0x6a8647`, `0x6a8692`, `0x6a86a0`, `0x6a86b9`, `0x69e4c0`, `0x69e4e0`, `0x69e515`, `0x69e57d`, `0x69e5c2`, `0x69e5dd`, `0x69e5e2`, `0x69e61b`, `0x69e64b`, `0x69e6b3`, `0x69e6e5`, `0x69e745`, `0x69e758`, `0x6a77e0`, `0x6a4cc0`, `0x6a4dc0`, `0x69eb70`, `0x80308c`, `0x80c4e8`, `0x80fec8`
Confidence: verified
Proof: one
Test: `terrain::cmapobj_skyocc_offsets_difftest`, `terrain::cmapobj_vert_offsets_difftest`, `terrain::wmo_mocv_normalize_alpha_difftest`

`ret 0x10`. The def's probe starts 1.5 above its position (`0x80308c`) and ends 1760 below
(`0x80fec8`, `0x6a8475`); the entity's starts 1.0 above and ends 12.0 below (`0x80c4e8`,
`0x69e515`). The pushes at `0x6a8524` to `0x6a853f` are `0x6a77e0`'s: source colour, directional
output `+0xa0`, floor 0xa8, ambient output, cap 0x60 (entity `0x69e5c2` to `0x69e5dd`, output
`+0xf4`). `0x6a8547` tests the sky flag; `0x6a8574` copies the RGB of the day colour for 255; the
blend `(diff × w) >> 8 + cur` is `0x6a85a8` (and `0x69e64b`).

## Light colour packing

`0x6a77e0` returns two colours from one. With M the largest channel byte (at least 1), the first
output is the source, and when M is below the floor its HSV value is multiplied by floor / M and
converted back to bytes (alpha 0xff); the second is the source, and when M is above the cap each
channel c becomes (c × s + 0xff) >> 8 with s = round(cap × 255 / M − 0.5).

Addresses: `0x6a77e0`, `0x6a77ec`, `0x6a780e`, `0x6a7836`, `0x6a7892`, `0x6a78ab`, `0x6a78b7`, `0x6a78c1`, `0x6a7922`, `0x6a7929`, `0x6a7947`, `0x6a7950`, `0x6a7956`, `0x6a796b`, `0x7bbc80`, `0x7bbd60`, `0x86aa24`
Confidence: verified
Proof: one
Test: none

`ret 0x14`: the source, the first output, the floor, the second output and the cap. `0x6a780e` tests M
against 1 (`jae`), `0x6a7836` the floor (`jae` skips the raise). The raise converts the bytes with
`0x7bbc80` (`0x6a7892`), scales V by `floor / M` as `fild; fidiv` (`0x6a78ab`) and `0x7bbd60`, then packs each
channel as the byte of f32(c × 255 + 512) >> 14 (`0x6a78c1`). The lowering is at `0x6a7929` (`jbe`
skips it); `0x6a7947` forms cap × 255 / M, `0x6a7950` subtracts 0.5 (`0x86aa24`) and `fistp` rounds.

## Point lights

`0x6a7ac0` adds a map point light to a lighting context, attenuated by distance from the context's
point `+0x24` raised by 1.1666666. A light whose falloff `+0xf0` is 0 is added unchanged. Otherwise the intensity is
1.0 inside `+0xe8`, 1 − (d − start) × falloff between `+0xe8` and `+0xec`, and the light is
dropped from `+0xec` outward. The added record is a copy of the light's 27 dwords with its
direction set from the offset and its colour `+0xb8` times the intensity at `+0x40`. `0x69e1c0`
is the radius fall-off of a fog: 1 below `+0x10`, 1 − (d − `+0x10`)/(`+0x14` − `+0x10`) up to `+0x14`, d clamped to
[0, `+0x14`].

Addresses: `0x6a7ac0`, `0x6a7ad2`, `0x6a7ae5`, `0x6a7b14`, `0x6a7b57`, `0x6a7b5c`, `0x6a7b69`, `0x6a7b7b`, `0x6a7b86`, `0x6a7b9e`, `0x6a7bf5`, `0x6a7c1d`, `0x6a7c82`, `0x6a7ce8`, `0x69e1c0`, `0x69e1c9`, `0x69e1d4`, `0x69e1e2`, `0x69e1ea`, `0x69e20d`, `0x6a0ba0`, `0x71b6a0`, `0x71b780`, `0x71b620`, `0x810a70`
Confidence: verified
Proof: one
Test: `terrain::chunk_add_light_difftest`, `terrain::chunk_add_light_falloff_difftest`, `terrain::wmo_molt_attenuation_difftest`

`0x6a7ae5` adds `0x810a70` (1.1666666) to the point's z; `0x6a7b57` stores the distance with `fst`.
`0x6a7b5c` compares with `+0xe8` (`test ah,5; jp` falls to the second test for a larger or NaN
distance); `0x6a7b7b` with `+0xec` (a larger or NaN distance leaves at `0x6a7ce8`); `0x6a7b86` forms
the intensity. `0x6a7bf5` copies 27 dwords (`rep movs`). In `0x69e1c0`, `0x69e1c9` is the compare with 0
(negative becomes 0), `0x69e1d4` the compare with the end (at or past it, the end), `0x69e1e2` the compare with the start
(below it returns 1.0), `0x69e20d` the quotient.

## Camera interior fog

`0x69de20` evaluates the fog of the map object holding the camera (`0xc7b748`). For each of the
claimed groups it reads the group's four fog indices (`+0x34`..`+0x37`), and for each nonzero index
whose fog record is inside its larger radius and not flagged 1 puts (distance, index) into a
min-ordered heap; it then blends the records nearest first by the falloff `0x69e1c0` into the output
fog.

Addresses: `0x69de20`, `0x69de26`, `0x69de4b`, `0x69de7d`, `0x69de8b`, `0x69dec4`, `0x69ded2`, `0x69df08`, `0x69df70`, `0x69df90`, `0x69dfac`, `0x69dfdc`, `0x69dfe6`, `0x69dffa`, `0x69e042`, `0x69e145`, `0x69e16c`, `0x69e19e`, `0x69e240`, `0x69efd0`, `0x69ea40`, `0x69e1c0`
Confidence: inferred
Proof: none
Test: `terrain::light_distance_difftest`

`ret 8`. `0x69de26` returns 0 with no claim `0xc7b748` or a model flag `+0x190` equal to 1 (`0x69de4b`). The
groups come from the window list (`0xc7cd84`); `0x69de8b` ORs a flag from each group's `+0x10 & 0x48`
into the first output byte. The fog records are `+0x158`, 0x30 bytes each (`0x69dfa6`); the
distance is from the camera moved into the object (`0x69dec4`); `0x69dfdc` compares it with the
record's `+0x14` and `0x69dfe6` tests the record flag bit 0. The heap push is `0x69dffa` onward,
the pops and blends `0x69e07d` to `0x69e195`. How the records blend and what the heap order is for
are not read here.
