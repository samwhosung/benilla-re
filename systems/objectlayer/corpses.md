# objectlayer: corpses

`CGCorpse_C`, the object type 7: a player's body or skeleton on the ground. It dresses a character
model from the corpse fields or loads a skeleton, lies down in the `Dead` or `Drowned` pose, draws
a grey decal, shows a sparkle while it holds loot, answers a right click, and tells the interface
which corpse is the player's own. The code is Corpse_C.cpp, `0x5d5f60` to `0x5d7120`; the
constructors and the update that call it are in [classes](classes.md) and [update](update.md).

Roots: `0x5d5f60`, `0x5d6050`, `0x5d6260`, `0x5d6850`, `0x5d68c0`, `0x5d6bf0`, `0x5d6fe0`

## Members and fields

A corpse's field block starts at field 6, so a field's offset in `[+0x110]` is `4·(index − 6)`:
the owner GUID `+0x0`, the facing `+0x8`, the position `+0xc`, the display id `+0x18`, 19 item
slots from `+0x1c`, the byte fields `+0x68` and `+0x6c`, the guild `+0x70`, the flags `+0x74` and
the dynamic flags `+0x78`. The object adds 32 watcher lists of 12 bytes from `+0x118` and the
members `+0x298` (the normal is cached), `+0x29c` (the character component), `+0x2a0` to `+0x2a8`
(that normal, (0, 0, 1) at first), `+0x2ac` (submerged), `+0x2b0` (the decal radius) and `+0x2b4`
(the loot sparkle).

Addresses: `0x5d6050`, `0x5d60ce`, `0x5d7690`, `0x5d76c0`, `0x5d7120`, `0x5d6220`, `0x5d6130`, `0x5d6100`, `0x5d6bb0`
Confidence: verified
Proof: one
Test: none

`0x5d6050(fields, movement)` runs the base constructor, empties the 32 lists, zeroes `+0x298` to
`+0x2b4` but 1.0 at `+0x2a8`, stores the vtable `0x80ab80` and copies the position at `+0x24` and
the facing at `+0x30` of the create message's movement record into the field block. Slot 5
(`0x5d7690`) returns the block's position, slot 6 (`0x5d76c0`) its facing. `0x5d7120` returns
`+0x8` of the `ChrRaces` row (store `0xc0dee0`) the byte at `+0x69` selects, the race's faction
template, or 0; its callers are `0x5166cd` and `0x6064e9`. `0x5d6130` is the destructor body: it deletes the component,
removes the sparkle, unlinks the lists and runs the base destructor; `0x5d6100` (slot 0) calls it
and frees on bit 0. `0x5d6bb0` frees the facet buffers at exit.

## Field watchers

Two watchers are put on every corpse's flags at start-up. A change of the flags (offset `0x74`)
rebuilds the body; a change of the dynamic flags (offset `0x78`) adds or removes the loot sparkle.
Bit 0 of the flags marks a skeleton, bits 3 and 4 hide the helm and the cloak, bit 5 admits the
spell click below; bit 0 of the dynamic flags marks loot.

Addresses: `0x5d5f60`, `0x5d5fa0`, `0x5d5fe0`, `0x5d6d60`, `0x5d6de0`, `0x5d6e20`, `0x468070`, `0x401657`
Confidence: verified
Proof: one
Test: none

`0x5d5f60`, called from `0x401657`, calls `ClntObjMgrSetTypeMirrorHandler(7, offset, 4, callback)`
twice, with `0x5d5fa0` for `0x74` and `0x5d5fe0` for `0x78`. Each resolves its GUID with mask 0x80
and calls `0x5d6d60` or `0x5d6de0` with the old bytes. `0x5d6e20` returns bit 0 of the dynamic
flags. `0x5d6de0` acts only when that bit differs from the old value: it adds the sparkle if
the bit is now set, else removes it. `0x5d6d60` removes the sparkle, fades the node out, deletes
the component, clears the normal's latch, loads the slot 10 model, installs it with `0x613cf0`,
adds the sparkle if lootable and creates the node.

## The corpse's model path

Slot 10 gives the model file. A skeleton (flags bit 0) uses
`World\Generic\PassiveDoodads\DeathSkeletons\%s%sDeathSkeleton.mdx` with the race's file string
(`ChrRaces` row `+0x3c`, indexed by the byte at `+0x69`) and the gender word `Male`, `Female` or
`NOSEX` for the byte at `+0x6a`. Any other corpse uses the model of its display id: the
`CreatureDisplayInfo` row's `+0x4` indexes the model store, whose row `+0x8` is the path. A miss
prints `INVALIDPLAYERDISPLAYID|%d|%d|%d` or `INVALIDPLAYERMODELRECORD|%d|%d|%d` and gives no path.

Addresses: `0x5d6700`, `0x5d6730`, `0x5d6759`, `0x5d6789`, `0x5d67c4`, `0x85fb30`, `0x856450`
Confidence: verified
Proof: one
Test: none

`0x5d6700` loads the flags at `0x5d6706` (`test byte [eax+0x74],1`). The skeleton arm formats
into the buffer `0xc4c000` with `SStrPrintf` (`0x5d674b`, size 0x104); the table `0x856450` holds
the three gender words. The other arm reads the display id at `+0x18` (`0x5d6759`) and prints the
two errors at `0x5d6789` and `0x5d67c4`.

## Dressing the body

`0x5d6260` is the create hook (the notify pass of [update](update.md)). After the base hook
it builds, unless the flags mark a skeleton, a character component from the race, gender, skin,
face, hair and facial-hair bytes of fields `+0x69` to `+0x6f` and the body model. It then
sets the pose and dresses the component from the 19 item slots. Slot 0 is skipped with flags bit 3,
slot 14 with bit 4 and slot 17 always; slots 15 and 16 attach a weapon model only when an item object has the field value as its GUID. A tabard in
slot 18 whose display row has bit 0 of `+0x24` installs the guild emblem.

Addresses: `0x5d6260`, `0x5d627c`, `0x5d6297`, `0x5d63bc`, `0x5d641d`, `0x5d6480`, `0x5d649b`, `0x5d64ad`, `0x5d64e2`, `0x5d6494`, `0x475fb0`, `0x476b90`, `0x478cb0`, `0x47a0c0`, `0x465e4f`
Confidence: verified
Proof: one
Test: none

`0x5d6297` calls `NewCharacterComponent` and `0x5d63bc` `InitCharacterComponent` on a block built
from the bytes (`0x5d635c` to `0x5d6398`). The loop `0x5d6417` runs 19 times: it takes the low 24
bits of `+0x1c + 4·slot`, indexes the display store `0xc0dc10`, skips a missing row, then slot 17
(`0x5d644e`); slots 15 and 16 (`0x5d6457`, `0x5d645c`) go to `0x5d649b`, which looks up the GUID with high dword 0 and low dword the whole field value
(mask 2, `0x5d64ad`) and, only if an item object has that GUID, calls
`AttachItemModel` (`0x5d64e2`); the others call `SetCharacterComponentItem` (`0x5d6480`) after the
bit 3 and bit 4 tests (`0x5d6465`, `0x5d6470`). `0x5d6494` is the tabard call to `0x5d6ec0`.

## The pose

A corpse lies in animation 132 `Drowned` when it is submerged, else 6 `Dead`. The test, made once
when the corpse is created and again when its model loads, is `0x5d6540(node, position)`: the node
must hold liquid, and its liquid level must stand more than 2/3 above the corpse's height. The
result stays in `+0x2ac` and also decides whether a skeleton is drawn, through slot 16 below: the world frame's pass `0x4815d0` calls slot 16 with `list` at `0x4815ff` and tests the result at `0x48161d`, and a 0 skips the draw list.

Addresses: `0x5d6540`, `0x5d658a`, `0x5d63d9`, `0x5d63eb`, `0x5d6402`, `0x5d6850`, `0x5d6887`, `0x5d67e0`, `0x5d6809`, `0x5d67ef`, `0x5d67fd`, `0x5d6813`, `0x5d682b`, `0x683e9e`, `0x683f65`, `0x481540`, `0x477ab0`, `0x477860`, `0x4815d0`, `0x4815ff`, `0x48161d`, `0x670630`, `0x69e280`, `0x69b6d0`, `0x80abfc`
Confidence: verified
Proof: one
Test: none

`0x5d6540` calls `0x670630` with the node: it returns 0 without a node, or when bit 5 of the
node's `+0x90` is clear; else it fills the height `[ecx+0x98]`. `0x69e280` sets that bit and the
height from `GetLiquidStatus` at the node's position (`0x69e2e1`, `0x69e2f3`). `0x5d6587`
subtracts the corpse's `z`, and `0x5d658a` compares with 0.6666667 (`0x80abfc`) with `test ah,0x41`:
true only above it. `0x5d63eb` stores the latch and `0x5d63f7` chooses `0x84` or `6`. Slot 13
(`0x5d6850`), the model-loaded hook, repeats the test through the latch and the play call
(`0x5d6887`), then stores half the model's bounding-sphere radius, `+0xc · 0.5`, at `+0x2b0`.
Slot 16 (`0x5d67e0(list)`) answers the world frame's pass, whose argument `list` is 1 in the first walk of the render
records and 2 in the second (`0x683e9e`, `0x683f65`, passed on by `0x481540`). With bit 0 of `list` clear
(`0x5d67ef`) it returns 0 for every corpse, after `0x477ab0` on the component when there is one
(`0x5d67fd`). With the bit set it returns 0 when the latch `+0x2ac` is nonzero and bit 0 of the flags word
`+0x74` is set (`0x5d6813` to `0x5d681d`); otherwise 1 with no component, else 1 only when `0x477860(component, 0)`
returns nonzero (`0x5d682b`).

## The ground normal

Slot 8 (`0x5d68c0`) gives the corpse's ground normal, worked out once. With the model loaded, a
skeleton that is not submerged first snaps its position to the ground: it traces from half a unit
above the position to ten units below, and a hit becomes the new position. It then queries the
world triangles inside the model's box, scaled and moved to the position and lowered by half its
height, and the normal is the mean of the normals of the triangles that face up.

Addresses: `0x5d68c0`, `0x5d68d1`, `0x5d6908`, `0x5d695e`, `0x5d6967`, `0x5d69eb`, `0x5d69f4`, `0x5d6a22`, `0x5d6ab0`, `0x5d6af7`, `0x5d6b7d`, `0x5d6b87`, `0x672170`, `0x6721b0`, `0xc4bfa4`
Confidence: verified
Proof: one
Test: none

`0x5d68d1` and `0x5d6908` return the stored normal if `+0x298` is set, or the model is missing or not loaded.
`0x5d695e` tests the skeleton bit and `0x5d6967` the submerged latch. `0x5d69eb` calls
`TraceWorldSegment` with flags 0x100111; `0x5d69f4` writes the hit to the position. `0x5d6ab0`
calls `QueryWorldTriangles` into the list `0xc4bfa4` with the same flags; `0x5d6af7` sums the
first three dwords of each 0x34-byte triangle record whose third is above zero, and the means
are stored at `+0x2a0`, `+0x2a4` and `+0x2a8`. `0x5d6b7d` sets the latch; a failed query keeps
(0, 0, 1). The latch is cleared by slot 2 and by `0x5d6d60`.

## The ground decal

Slot 12 (`0x5d6fe0`) draws the corpse's decal when the base update calls it: a box centred on the
position, `±s` wide and `±2s` high with `s` = the object scale times `+0x2b0`, projected on the
terrain and map-object triangles inside it with the selection circle's textures and the grey
`0xff7f7f7f`. The base update calls it only for the object the target GUID names.

Addresses: `0x5d6fe0`, `0x5d6fef`, `0x5d6ff7`, `0x5d7001`, `0x5d70de`, `0x5d70ea`, `0x5d7105`, `0x614ada`, `0x6d7330`, `0x615070`, `0xc4da80`, `0x589f40`, `0x589f50`
Confidence: verified
Proof: one
Test: none

`0x5d7001` multiplies the scale field `[obj+8]+0x10` by `+0x2b0`; the box corners are built from
the position of slot 5 with ±s in x and y and ±2s in z. `0x5d70de` calls `0x615070` for the yaw
matrix, `0x5d70ea` `0x614e00` for the render state, and `0x5d7105` `DrawDecal(box, &0xc4da80,
matrix, 0.5, 0x200122)` between the state push `0x589f40` and pop `0x589f50`. The tint is the
pointer to `0xc4da80`, which holds 0xff7f7f7f. `0x614ada` is the only caller.

## The loot sparkle

A corpse with loot carries an extra model, the `Loot Art` effect model (hard-coded effect 4),
attached to attachment 19 of the body. `0x5d6e30` makes it once, `0x5d6e80` detaches and frees
it. It exists from creation and reactivation while bit 0 of the dynamic flags is set, and follows
that bit through the watcher.

Addresses: `0x5d6e30`, `0x5d6e42`, `0x5d6e5a`, `0x5d6e74`, `0x5d6e80`, `0x5d6e8d`, `0x61f470`, `0x712f70`, `0x713020`
Confidence: verified
Proof: one
Test: none

`0x5d6e30` returns if `+0x2b4` is set; else `0x5d6e42` takes `GetHardcodedEffectRecord(4)`, its
path at `+0x8` goes to `CreateModelFromPath` on the body's scene (`0x5d6e5a`) and `0x5d6e74`
calls `CM2Model::Attach(parent = body, attachment = 0x13)`. `0x5d6e80` detaches when the sparkle
has a parent and closes it.

## The player's own corpse

When the corpse's owner is the active player and the flags do not mark a skeleton, the corpse
publishes its own GUID to the interface (`0x4920d0`), which `RetrieveCorpse` sends. Creation and
reactivation publish; deactivation publishes 0.

Addresses: `0x5d6260`, `0x5d6503`, `0x5d652b`, `0x5d65b0`, `0x5d65d8`, `0x5d6600`, `0x5d666a`, `0x4920d0`
Confidence: verified
Proof: one
Test: none

`0x5d6509` tests the skeleton bit, `0x5d6511` loads the owner, and `0x5d6519` compares it with
`ClntObjMgrGetActivePlayer`; `0x5d652b` calls `0x4920d0(corpse GUID)`. Slot 1 (`0x5d65b0`) pushes 0
and 0 at `0x5d65d8` under the same test, then removes the sparkle, fades the node out and runs
the base deactivation. Slot 2 (`0x5d6600`) runs the base reactivation, clears `+0x298`, adds the
sparkle, makes the node if a model exists and publishes at `0x5d666a`.

## Fading out the node

`0x5d6ce0` ends the node. A corpse without a component, or one whose composite texture has loaded
and whose component updates, conforms with argument 0 and hands the node to `FadeOutModel` at the
fade alpha `+0xfc`; any other corpse releases the node at once.

Addresses: `0x5d6ce0`, `0x5d6cfd`, `0x5d6d04`, `0x5d6d0e`, `0x5d6d1d`, `0x5d6d34`, `0x5d6d37`, `0x614603`, `0x5d6d47`, `0x44b4c0`, `0x477860`, `0x671ac0`, `0x672df0`
Confidence: verified
Proof: one
Test: none

`0x5d6cfd` calls `0x44b4c0` on the composite texture `[component+0x184]`, which is 0 unless the
texture has loaded; a zero releases the node (`0x5d6d04`, `0x5d6d1d`). Else
`UpdateCharacterComponent(component, 0)` (`0x5d6d0e`) must return nonzero, or the node is
released. Slot 15 then runs with 0 (`0x5d6d34`) and `FadeOutModel(node, +0xfc)` (`0x5d6d47`); `0x5d6d37` loads `+0xfc`, the fade's
target alpha, where the base deactivation passes the current alpha `+0xf4` (`0x614603`).

## The guild emblem

`0x5d6ec0(retry)` installs the owner's guild tabard on the component. It asks the name cache for
the owner's emblem (`GetGuildEmblem`, owner GUID, guild at `+0x70`); on a hit it calls
`SetCharacterComponentEmblem` with the five emblem values. On a miss, when `retry` is set, it
asks again with the callback `0x5d6f90`, which resolves the corpse and calls `0x5d6ec0(0)`, so a
second miss ends it.

Addresses: `0x5d6ec0`, `0x5d6f00`, `0x5d6f24`, `0x5d6f45`, `0x5d6f81`, `0x5d6f90`, `0x5d6fad`, `0x5d6fba`, `0x6d6d20`, `0x47a610`
Confidence: verified
Proof: one
Test: none

`0x5d6f00` calls `GetGuildEmblem`; `0x5d6f24` installs. `0x5d6f45` allocates 8 bytes for the corpse
GUID, `0x5d6f81` repeats the lookup with `0x5d6f90`, which frees the record (`0x5d6fcf`).

## Right-clicking a corpse

Slot 24 (`0x5d6bf0(shift)`) is the corpse's click. With no player nothing happens. A lootable
corpse (dynamic flags bit 0) clicked by an unmounted player is looted: a player who is not
standing gets error 133 (`ERR_LOOT_NOTSTANDING`); else, when the player is the active mover,
auto-loot is set to the shift state, and the player loots with the auto-interact setting. Anything
else with flags bit 5 set, the interact spell known and the corpse not assistable, goes to the
interact spell.

Addresses: `0x5d6bf0`, `0x5d6c22`, `0x5d6c3b`, `0x5d6c4a`, `0x5d6c68`, `0x5d6c78`, `0x5d6c88`, `0x5d6c9f`, `0x5d6ca7`, `0x5d6cb3`, `0x5d6cd2`, `0x5df460`, `0x5df130`, `0x60f900`, `0x6067d0`, `0x5f05e0`, `0x5ed570`, `0xb700e8`
Confidence: verified
Proof: one
Test: none

`0x5d6c22` reads the mount display (field 133, `+0x1fc`) and `0x5d6c2e` the lootable bit; both
must allow looting (display 0 or less, bit set). `0x5d6c3b` calls the player's stand state (slot
`+0xa4`); nonzero shows error 0x85 through `DisplayError` (`0x5d6c88`). `0x5d6c4a` compares the
player's GUID with the active mover; `0x5d6c68` calls `SetAutoLoot(shift)`, `0x5d6c78`
`LootCorpse(corpse, CanAutoInteract)`. The second arm needs flags bit 5 (`0x5d6c9f`), `0xb700e8`
nonzero (`0x5d6ca7`) and `CanAssist_Corpse` false (`0x5d6cb3`), then calls `InteractWithSpell(corpse,
CanAutoInteract, 1, shift)` (`0x5d6cd2`).

## Selection and the mouse

A corpse is never selectable: slot 22 returns 0. Slot 21 lets the mouse pick it: true unless the
flags mark a skeleton that holds no loot.

Addresses: `0x5d76d0`, `0x5d76d6`, `0x5d76dc`, `0x469fe0`
Confidence: verified
Proof: one
Test: none

Slot 22 of the corpse table is `0x469fe0` (`xor eax,eax; ret`). `0x5d76d6` tests bit 0 of the
flags: clear returns 1; set returns the lootable bit through `0x5d76dc`.
