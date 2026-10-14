# ui: the ground-target reticle

What the world shows at the pointer while a spell waits for a ground position: a state word that picks the picture, the area
decal projected onto the world, and the passes that draw it.

This is `WorldFrame.cpp`. The pick, the commit and the cancel are `spell/targeting`; the radius is
`spell/targeting#area-reticle-radius`; the cursor modes are `ui/worldcursor`.

Roots: `0x4820f0` (place), `0x4837b0` (draw), `0x483460` (render the world)

## The reticle state

The state word `0x83dc2c` says which reticle picture is drawn: 0 in range, 1 out of range, 3 none. The hover pass of the world frame stores 3 before
every pick, so a pick that finds nothing under the pointer, or a frame that is not targeting a location, leaves nothing drawn. `0x4820f0` is the only code that stores
0 or 1: it runs when the pick is a ground position while a location is wanted, tests the position against the spell's range and stores 0 with the
cast cursor or 1 with the unable-to-cast cursor. The position goes to `0xb4b3a0`; the radius to `0xb4b3b0`: the cast radius kept to 20 in range, 0 out of range.

Addresses: `0x4820f0`, `0x481790`, `0x6e6810`, `0x6e6350`, `0x523c20`, `0x83dc2c`, `0xb4b3a0`, `0xb4b3b0`
Confidence: verified
Proof: one
Test: none

`0x481840` stores 3 in `0x83dc2c` before the pick `0x481190`. `0x4820f9` and `0x482106` leave through `0x4821e1` when the world is not targeting a location. `0x482113` to `0x482140` copy the pick's position to `0xb4b3a0` to `0xb4b3a8`. `0x48214b` tests the range; `0x482154` stores 0, `0x482163` compares the radius with 20 (0x804478), `0x482182` stores 20, `0x482193` stores 1, `0x48219d` stores the radius 0, `0x4821a7` selects mode 22 and `0x4821ac` sets the mode. Over a unit during a location-only cast the pick reports the ground behind it.

## The area decal

When no placement model exists, `0x4837b0` draws the area decal: a texture projected onto the world over a box centred on the picked position.
The box reaches the radius in x and y, or 1.3888889 yards when the radius is 0, and 2 yards up and down. The texture is chosen by the state: the acceptable
one in range, the unacceptable one out of range. It is drawn in white with render state 7 set to 2; `0x6d7330` projects it onto terrain, buildings and water. With a placement model the same function instead
tints the model: green at alpha 0x40 in range, red out of range.

Addresses: `0x4837b0`, `0x6d7330`, `0x6e6970`, `0x670cf0`, `0xb4b344`, `0xb4b3a0`
Confidence: verified
Proof: one
Test: none

`0x4837ba` fetches the placement model; `0x4837e9` positions the tint with the model's own box and `0x483811` and `0x48381a` pick the colour 0x4000ff00 or 0x40ff0000 by the state. `0x483841` to `0x483858` take the radius or the default 0x3fb1c71c. `0x48387e` to `0x4838ed` build the two corners with the position and 2 (0x801628). `0x4838f0` to `0x483917` set render states 0xe, 0x12 and 7. `0x483922` indexes the texture table `0xb4b344` by the state and `0x483940` binds the texture at stage 0x17. `0x48395a` calls the projector with colour 0xffffffff.

## The decal passes

The world render draws the decal twice. The first draw, with the receiver class 0x200122 (terrain and buildings), runs after the opaque model pass and before the water and the transparent model passes; the second, with 0xf0000 (liquid surfaces), runs after them. Each runs only for a state below 2 whose texture exists, so a state of 3 draws nothing. The two textures are `Interface\SpellShadow\Spell-Shadow-Acceptable.blp` for state 0 and `Spell-Shadow-Unacceptable.blp` for state 1, loaded when the world frame is built into `0xb4b344` and `0xb4b348`.

Addresses: `0x483460`, `0x4837b0`, `0x481c00`, `0xb4b344`, `0x83dc2c`
Confidence: verified
Proof: one
Test: none

`0x4836b0` to `0x4836c5` test the state and the texture and call `0x4837b0` with 0x200122; `0x4836ca` onward draw the doodads and, by the submersion type, the water and the two transparent passes (`0x4836f4`, `0x483707` through `0x708900`). `0x48370c` to `0x483727` repeat the test and call with 0xf0000. `0x481c00` to `0x481c3f` load two paths from the table at 0x83dc24 (the strings at 0x83dc64 and 0x83dc30) into `0xb4b344` and `0xb4b348`.
