# ui: Minimap blips, arrows and dots

What the minimap draws on its disc besides the map: the area landmarks, the rim arrows, the party and
pet arrows, the corpse and quest markers and the dots for tracked units. The `Minimap` widget's methods,
zoom and ping are in `minimap`.

Two object files do the work. `Minimap.cpp` (`Game\GameClient`) builds the blip lists from the
`AreaPOI` table and three static records. It runs from its first initializer `0x6d85d0` to the end of
`0x6dbbe0`; the code after it, from `0x6dbcf0`, is record serialisation called from DBCache. The frame
code, `MinimapFrame.cpp`, draws them. Positions are world yards from the player; a blip's disc position is
the yards over the zoom radius times half the widget (`The north-up disc`).

Roots: `0x4ec950`, `0x4edbc0`, `0x4eb0c0`, `0x4eaa90`, `0x4ebc00`

## Blip candidates

When the world is entered or the continent changes, the landmark candidates are the `AreaPOI` rows whose
continent is the current map and whose flag bit 0 is set. No faction, area or importance test applies.
Rows are taken from the last to the first, and three static records follow them unconditionally: slot 0
the selected quest's marker, slot 1 the gossip marker, slot 2 the corpse. Each record is 0x74 bytes. The
three are cleared and given a name buffer of 0x40 bytes.

Addresses: `0x6d8e10`, `0x6d8f26`, `0x6d8f45`, `0x6d8f4a`, `0x6d8f70`, `0x6d8f92`, `0x6d8fa8`, `0x6d8fb0`, `0x6d8fcb`, `0xcea69c`, `0xceaaa8`, `0xcea760`, `0xcea7d4`, `0xcea848`, `0xc0e054`, `0xc0e058`, `0xc0e080`, `0x6dabc0`
Confidence: verified
Proof: one
Test: none

`0x6d8e10(continent)` runs the scan twice, the first pass to count (`0x6d8f1a` stores the count at
`0xceaaa8`) and the second to fill the pointer array `0xcea69c`. The table is `0xc0e054` with the count at
`0xc0e058` and a stride of 0x74 (`0x6d8f23`); the index counts down (`0x6d8f26`, `0x6d8f69`). The only
tests are `cmp [rec+0x1c], continent` (`0x6d8f45`) and `test byte [rec+0x20], 1` (`0x6d8f4a`). After the
scan `0x6d8f70` adds 3 to the count. The loop at `0x6d8f92` points the next three array slots at
`0xcea760`, `0xcea7d4` and `0xcea848` (stride 0x74), zeroes 0x1d dwords of each (`0x6d8fb0`) and points the
record's name for the client locale (`[0xc0e080]`, `0x6d8fcb`) at `0xcea6a0 + 0x40·i`.

## Setting a static blip

A static blip is set with a position, an icon, a rank and a name; a position of (0, 0) hides it. Slot 1,
the gossip marker, also starts a 480 second lifetime and rebuilds the world map's landmark list. The
corpse marker is slot 2 with icon 7, flag 2 and rank −1, and exists only while the corpse position the
server sent is cached and the player is on that map; otherwise it is set to (0, 0). The quest marker is
slot 0 with icon 5 and no flag: it follows the quest selected in the quest log and shows only as a rim
arrow.

Addresses: `0x6dac10`, `0x6dac1e`, `0x6dac27`, `0x6dac33`, `0x6dac45`, `0x6dac62`, `0x6dac7e`, `0x6dac86`, `0x6dac98`, `0x6dac9d`, `0x6dacb0`, `0x6dacd0`, `0x4defd3`, `0x4df0f7`, `0x492010`, `0x492072`, `0x492081`, `0x4e28aa`, `0xbb7480`, `0xceaaa4`, `0xceaab0`, `0x6dad00`
Confidence: verified
Proof: one
Test: none

`0x6dac10(slot in ecx, flags in edx; x, y, icon, rank, name)` writes the record `slot·0x74`: the flags at
`+0x20`, x and y at `+0x10`, `+0x14` (`0x6dac27`, `0x6dac33`), the continent from `0x86f694`, the icon and
rank at `+0x08`, `+0x04`, and copies the name (`0x40` bytes, `0x6dac62`). Slot 1 stores `time + 0x1e0` at
`0xceaaa4` (`0x6dac7e`) and calls `0x4a67a0`; every call runs `0x6d9a90` and sets the dirty flag
`0xceaab0` (`0x6dac98`, `0x6dac9d`). `0x6dacb0` is slot 0, icon 5, flags 0; `0x6dacd0` is slot 2, flags 2,
icon 7, rank −1, named by the string `CORPSE_RED`. `0x6dacd0` has two callers, both in `0x492010`
(`0x492072`, `0x492081`); `0x6dacb0` has `0x4defd3` and `0x4df0f7`, which read and clear on the selected
quest id `0xbb7480`. Slot 1's only setter is the gossip point handler (`0x4e28aa`).

## Clearing the gossip marker

The gossip marker is removed when the player comes within 10 yards of it. The distance is compared
squared with 100.0, and only strictly less clears it, so exactly 10 yards leaves it. Clearing zeroes its
position and expiry and rebuilds the world map's landmark list. The marker also expires 480 seconds after
it is set.

Addresses: `0x6d93a0`, `0x6d99aa`, `0x6d9a08`, `0x6d9a12`, `0x6d9a15`, `0x6d9a4c`, `0x6d9a51`, `0x6d93b9`, `0x6d9411`, `0xceaaa4`, `0x806b10`, `0x4a67a0`
Confidence: verified
Proof: one
Test: none

In `0x6d93a0`, `0x6d99aa` loads slot 1's x (`+0x10` of `0xcea7d4`) and tests it against 0; the block runs unless x
and y are both zero. After the distance, `fcomp [0x806b10]` (100.0, `0x6d9a08`), `test ah,5` and `jp`
(`0x6d9a12`, `0x6d9a15`) skip the clear for greater, equal and unordered results. The clear stores 0 in
slot 1's x and y and in `0xceaaa4`, then calls `0x4a67a0` (`0x6d9a4c`). The expiry test is at
`0x6d93b9` to `0x6d9411`.

## Placing the blips

Each candidate with a nonzero position and no unmet world state is placed by its distance over the zoom
radius `R`. At 0.8 or less it is in range: with flag bit 1 it joins the icon list, else it is dropped.
Farther than 0.8 it is kept as a direction-arrow candidate. Of those candidates at most 3 are kept and
sorted by rank, a signed compare with the lowest first, then by distance. A candidate farther than 694.444
yards is dropped, except the corpse record. The arrows' bearings come from `atan2`.

Addresses: `0x6d9a90`, `0x6d9ae6`, `0x6d9af6`, `0x6d9b27`, `0x6d9b2e`, `0x6d9b6d`, `0x6d9b75`, `0x6d9b78`, `0x6d9baa`, `0x6d9bd7`, `0x6d9b98`, `0x6d9ba1`, `0x6d9c87`, `0x6d9cb5`, `0x6d9cc2`, `0x6d9cee`, `0x6d9d41`, `0x6d9dd1`, `0x4c5810`, `0x47f220`, `0x811730`, `0x8116cc`, `0xcea670`, `0xcea690`, `0xcea684`, `0xcea650`, `0xcea65c`, `0xceaaac`, `0x6daa00`
Confidence: verified
Proof: one
Test: none

`0x6d9a90` runs only with a continent set (`0x86f694`). Both coordinates zero hide the record
(`jp 0x6d9b27` falls through only on equality, `0x6d9ae6`, `0x6d9af6`); a nonzero `+0x70` must pass
`0x4c5810`, a table lookup of that id that returns the entry's value (`0x6d9b2e`). The ratio is
`sqrt(dx²+dy²)/R` against 0.8 (`0x6d9b6d`); `test ah,0x41; jne` (`0x6d9b78`) is taken for less and equal,
so the split is inclusive at 0.8. Flag bit 1 is tested at `0x6d9baa`; an in-range record is appended to
`0xcea670` (`0x6d9bd7`), an out-of-range one sets `0xcea684[i]` (`0x6d9ba1`). The pass at `0x6d9c87`
walks the `0xcea684` flags; `fcom 694.444` (`0x6d9cb5`) skips the farther ones unless the record is
`0xcea848` (`0x6d9cc2`). The insertion compare `jge` (`0x6d9cee`) and the final `jg` (`0x6d9d41`) are
signed on `[rec+4]`, then the distance breaks ties. The kept indices go to `0xcea65c` and the bearings from
`0x47f220` to `0xcea650` (`0x6d9dd1`), the count to `0xceaaac`.

## The arrow list

The arrow records the frame reads are rebuilt only when the list changed. Each is 0x4c bytes: the
landmark's name at `+0x00`, the bearing at `+0x40`, a flag at `+0x44` that is 1 for the gossip marker and
the corpse, and at `+0x48` the static slot index 0, 1 or 2, or −2 for a landmark row. Between rebuilds only
the bearings are refreshed, and the rebuild reports whether it ran.

Addresses: `0x6daa20`, `0x6daa24`, `0x6daa3d`, `0x6daa5d`, `0x6daaad`, `0x6daad5`, `0x6daae1`, `0x6daaf4`, `0x6daafb`, `0x6dab0b`, `0x6dab0e`, `0x6dab26`, `0x6dab41`, `0x4ed2db`, `0xceaab0`, `0xcea650`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

`0x6daa20` tests the dirty flag `0xceaab0` (`0x6daa24`). Clear, it copies `0xcea650[i]` into each record's
`+0x40` and returns 0 (`0x6daa3d` to `0x6daa5d`). Set, it clears the flag, copies the name pointer at
`rec + 0x28 + 4·locale` (`0x6daad5`), stores the bearing (`0x6daae1`), sets `+0x44` from the compares with
`0xcea7d4` and `0xcea848` (`0x6daaf4`, `0x6daafb`), stores −2 at `+0x48` (`0x6dab0e`) and scans the three
static records for the source pointer (`0x6dab26` to `0x6dab41`), storing its index. It returns 1
(`0x6dab5d`) and has one caller, `0x4ed2db`.

## The rim arrows

A rim arrow is one of the model frames in the minimap's three-frame array, each showing the same
`Rotating-MinimapArrow` model. The art is chosen by an animation id: 167 (CorpseArrow) for the corpse,
166 (Arrow) for a landmark, and 168 (GuideArrow) for every other static slot, the quest marker and the
gossip marker. Each arrow's frame level is the minimap's level plus 3 plus its static slot index, so from
top to bottom: corpse, gossip, quest, party and pet, landmark. The arrows are re-aimed every frame
and re-sequenced only when the list was rebuilt.

Addresses: `0x4ec950`, `0x4ed2e0`, `0x4ed2e2`, `0x4ed308`, `0x4ed335`, `0x4ed344`, `0x4ed349`, `0x4ed353`, `0x4ed35a`, `0x4ed360`, `0x4ed36c`, `0x4ed36f`, `0x4ed37b`, `0x4ed404`, `0x4ed452`, `0x4ed4f7`, `0x76cf50`, `0x76a4f0`, `0x4ee1a3`, `0x4ede48`, `0x4edee0`, `0xbc8300`
Confidence: verified
Proof: one
Test: `ui::glue_4ec950_rotate`, `ui::glue_4ec950_scale_loop`, `ui::glue_4ec950_unproject`

`0x4ed353` compares the record's `+0x48` with 2 and, equal, pushes 0xa7 for `0x76cf50(SetSequence)`
(`0x4ed35a`, `0x4ed360`); otherwise `setne cl` against −2 and `lea ecx,[ecx+ecx+0xa6]` give 0xa6 for −2 and
0xa8 for every other value (`0x4ed36c`, `0x4ed36f`, `0x4ed37b`). `0x4ed335` to `0x4ed344` pass `+0x48` plus
`[minimap+0x334]` to `SetFrameLevel`; `0x4ee1a3` and `0x4ede48` store the minimap's level plus 3 there, so
levels run from `L + 1` (−2) to `L + 5` (corpse). The sequence calls sit below the rebuild test
(`0x4ed2e0`, `0x4ed2e2`); the rotation loop `0x4ed404` to `0x4ed4b3` (`fsincos`, `0x4ed452`, `0x4ed4f7`)
does not. With the byte `0xbc8300` set only records whose `+0x44` is set get an arrow (`0x4ed308`). Ids 165
to 168 are GroupArrow, Arrow, CorpseArrow and GuideArrow; the model's two-step id lookup
(`models/animation#playing-an-animation`) maps each to a sequence that shows one of its quads.

## Party and pet arrows

The party arrows are five frames: slots 0 to 3 are the other party members and slot 4 is the player's
pet, the charmed unit or else the summoned one, never a corpse and never a raid member. Each shows
`GroupArrow` (165), set once at setup and never changed. A record is a rim arrow when its distance over
the radius is above 0.8 and an in-range dot at 0.8 or less. A member that cannot be found falls back to a
cached record; a missing pet is hidden.

Addresses: `0x6dad10`, `0x6dad2b`, `0x6dad4e`, `0x6dad60`, `0x6dad8a`, `0x6dad8c`, `0x6dad92`, `0x6dad9c`, `0x6dadab`, `0x6dadc8`, `0x6dae25`, `0x6daf54`, `0x6daf5c`, `0x6daf67`, `0x6daf78`, `0x6daf7f`, `0x6dafb9`, `0x4e81a0`, `0x468460`, `0x4ee1de`, `0x4ee1e3`, `0xbc7660`, `0xbc6f48`
Confidence: verified
Proof: one
Test: none

`0x6dad10` writes 0x50-byte records at `0xbc7660` for `edi` in 0 to 4 (`0x6dafb9`). It fetches the active
player (type mask 0x10) and returns without one (`0x6dad4e`). For `edi == 4` (`0x6dad60`) it reads the
player's `+0x110` field block: the 64-bit at `[block]` (`UNIT_FIELD_CHARM`), or `[block + 8]`
(`UNIT_FIELD_SUMMON`) when that is 0 (`0x6dad8c` to `0x6dad9c`); otherwise `0x4e81a0(edi)` gives the party
member GUID. Both are fetched with type mask 8, units only (`0x6dadc8`). `0x4e81a0` serves indices 0 to
3. Slot 4 has no cached fallback (`0x6dae25`). The ratio is compared with 0.8 (`0x6daf5c`, `0x6daf67`
`jne` on less or equal) before the arrow flag (`0x6daf78`) and bearing (`0x6daf7f`). `0x4ee1e3` arms 165.

## Object dots

A dot is drawn for each nearby unit, player or game object that passes the classifier. A unit or player
is skipped when the player charmed it or else summoned it, when its health is at or below 0 as a signed
value, or when bit 2 of byte 3 of `UNIT_FIELD_BYTES_1` is set. Quest status 7 gives the gold cell 3;
otherwise a unit the player tracks gets the red cell 1, and a game object whose lock skill the player
tracks gets cell 0. Party members are cell 4; cell 2 has no producer. An object beyond the zoom radius in
three dimensions, or the player itself, is omitted.

Addresses: `0x4eaa90`, `0x4eaa9e`, `0x4eaaaf`, `0x4eab0d`, `0x4eab1c`, `0x4eab28`, `0x4eab2b`, `0x4eab43`, `0x4eab49`, `0x4eab56`, `0x4eab59`, `0x4eab72`, `0x4eab7e`, `0x4eab8a`, `0x4eab96`, `0x4eabe5`, `0x4eabfd`, `0x4eac0d`, `0x4eac1e`, `0x4eac24`, `0x4eac31`, `0x4eac3a`, `0x4eac48`, `0x4eac55`, `0x5ed210`, `0x5ed2b0`, `0x670540`, `0x4ec950`, `0x4ed6a8`, `0x4ed6b0`, `0xbc82b0`
Confidence: verified
Proof: one
Test: `ui::dist2_4eaa90`

The callback `0x4eaa90` returns 1 on every skip, so enumeration continues. It rejects the player's own GUID
(`0x4eaa9e` to `0x4eaaaf`) and a 3-D squared distance above the range squared (`fcompp`, `test ah,5; jnp`,
`0x4eab0d`, `0x4eab1c`). The type is read at `[[obj+8]+8]` (`0x4eab28`): 9 unit, 0x19 player (after
`0x4918d0`, the party test, `0x4eabd8`), 0x21 game object, else skip. The game object path takes category 0
after `0x5ed2b0` (`0x4eab43` to `0x4eab56`). The unit path loads `[obj+0x110]`; `+0x10` is charmed-by or,
when 0, `+0x18` summoned-by, compared with the player (`0x4eabe5` to `0x4eac0d`); health `+0x40` (`jle`,
`0x4eac1e`); `byte [+0x213] & 4` (`0x4eac24`); then `cmp [obj+0xcb8], 7` (`0x4eac31`) gives category 3
(`0x4eac3a`), else `0x5ed210` (`0x4eac48`) gives 1 (`0x4eac55`). All converge at `0x4eab59`; `0x670540`
sets the subzone colour flag, and a non-null stored subzone must equal the object's (`0x4eab7e`,
`0x4eab8a`). The record is `category·16 + 0xbc82b0` (`0x4eab96`). The dispatch is `0x4ed6a8`, the party
append `0x4ed6b0`.

`ui::dist2_4eaa90` runs the slice `0x4eaae5` to `0x4eab15`, the distance test.

## Dots, cadence and tooltip

Dot positions are computed when the classifier runs, about once a second, and drawn unchanged every frame
after the arrows, so dots do not pan smoothly; landmark icons are projected every frame. A dot is white,
or grey when it is in a different subzone from the player, and the grey is also the tooltip's wrap colour.
Hovering the disc shows an engine-built tooltip with a line for every arrow, party dot, landmark and dot
the cursor is over, named from the object or the name cache.

Addresses: `0x4ed573`, `0x4ed588`, `0x4ed6a8`, `0x4ed7b7`, `0x4ebc00`, `0x4ed1ae`, `0x4ed12a`, `0x4ed25f`, `0x4eb0c0`, `0x4eb19f`, `0x4eb30b`, `0x4eb31a`, `0x4eb33c`, `0x4eb587`, `0x4eb7bb`, `0x4eb7cf`, `0x4eb856`, `0x4eb869`, `0x530380`, `0x52ffe0`, `0x55f080`, `0x84c69c`, `0xb4b3c4`, `0xc0e228`
Confidence: verified
Proof: one
Test: `ui::glue_4ebc00_gradient`

The rebuild block, including the classifier dispatch (`0x4ed6a8`), sits behind the throttle at `0x4ed573`:
it is skipped while less than 1000 ms (`0x4ed588`) have passed since the last run;
`0x4ed7b7` calls `0x4ebc00`, the dot draw, last. The landmark icon loop projects with `0x4eaa30` every
frame (`0x4ed1ae`), colours with one white dword written once (`0x4ed12a`, read at `0x4ed25f`). `0x4eb0c0`
reads the cursor (`0x4eb19f`), keeps the elements it is over as bit masks (`0x4eb507` to `0x4eb526`),
sets the owner of the global tooltip `0xb4b3c4` with anchor mode 6 (`0x52ffe0`, `0x4eb587`), resolves a dot's GUID with `0x468460` and calls the object's name method
(`0x4eb31a`), or the name cache `0x55f080` on `0xc0e228` (`0x4eb33c`); a record whose colour flag at `+0x10`
is set is wrapped in `|cffb0b0b0` (`0x4eb7cf`). The text goes to `0x530380` (`0x4eb856`) and the tooltip
is shown through slot 0x88 (`0x4eb869`).

## Blip sizes

The sizes are fixed when the frame is constructed and do not follow a later aspect change. With `base` =
`G48 · 5/3` (1.0 at 4:3), a landmark icon is `0.0125·base` of a layout unit, a dot `0.00625·base` and the
disc `0.11·base`; on the 140-unit stock widget that is 16, 8 and 140.8 units. Party dots are scaled 1.3
and the arrows orbit at 0.8 of the half disc, 56.32 units. The eight arrow frames get the disc size as
their width and height and a model scale of 0.6; the player arrow gets none, an offset of 0.001 and its
model scale is `base`.

Addresses: `0x4edbc0`, `0x4edc42`, `0x4edc47`, `0x4edc4d`, `0x4edc5f`, `0x4edc6f`, `0x4edc7b`, `0x4edc9a`, `0x4edca9`, `0x4edcaf`, `0x4edcbb`, `0x4edcc1`, `0x4edcd3`, `0x4edce5`, `0x4edeb2`, `0x4edeba`, `0x4edfec`, `0x807a4c`, `0x807a48`, `0x807a44`, `0x807a28`, `0xbc7658`, `0xbc82a8`, `0xbc7970`, `0xbc762c`, `0xbc7630`, `0xbc8274`
Confidence: verified
Proof: one
Test: `ui::glue_4edbc0_unit_scale_spread`

`0x4edc42` is `0x41ae70(1.0)` and `0x4edc47` multiplies by 5/3. The products with `0x807a4c` (0.0125), `0x807a48`
(0.00625) and `0x807a44` (0.11) are stored at `0xbc7658` (`0x4edc6f`), `0xbc82a8` (`0x4edc9a`) and
`0xbc7970` (`0x4edcaf`); the halves are `0xbc762c`, `0xbc7630` and `0xbc8274` (`0x4edcc1`, `0x4edcd3`,
`0x4edce5`) and the corner tables follow from them. The arrow frames' scale is the float 0.6 at `0x807a28`
(`0x4edeb2`, `0x4edeba`); the player arrow's is `0x4edfec`. Nothing else writes these cells.

## The north-up disc

The disc does not rotate. A blip's position is the world offset swapped to the screen axes: from the
disc centre, the screen x is `k − (k/R)·(entity y − player y)` and screen y up is `k + (k/R)·(entity x −
player x)`, with `k` half the widget width times the layout scale. The only rotations are the player
arrow, which turns with the player's facing, and the rim arrows, each turned to its bearing. The
trigonometry in the minimap is the bearing `atan2` and the `fsincos` that turns that bearing back into a
rim position.

Addresses: `0x4eaa30`, `0x6d9dd1`, `0x6daf7f`, `0x4ed452`, `0x4ed4f7`, `0x4ed17a`, `0x47f220`
Confidence: verified
Proof: one
Test: `ui::geo_4eaa30`

`0x4eaa30` is the whole projection, 86 bytes with no angle: `out.x = k − (k/R)·dB`, `out.y = k + (k/R)·dA`
with `dA = A_entity − A_player`, `dB = B_entity − B_player`, `k = width·scale·0.5`. The two `atan2` sites are
`0x6d9dd1` and `0x6daf7f`, the two `fsincos` `0x4ed452` and `0x4ed4f7`. The width is the frame's geometry
getter (`0x4ed17a`, slot 0x1c).

`ui::geo_4eaa30` runs `0x4eaa30` whole.
