# charactermodel: equipment

A component keeps the display record of each worn item by body slot, turns armor records into texture cells and geoset values, and hangs item models (helm, shoulders, quiver, weapons,
shield) at the model's attachment points. This doc covers the entry points, the body slot table and the layer choosers that fill the cells, the held items, the item model loads, the hand grips and the guild tabard. The ten
groups and their cells are blitted by `composite`; the geosets the records select are `geosets`.

Roots: none

## Entry points

Four callers set or clear a slot. `0x479010(inventory type, display id)` clears the type's slot, then, for a positive id, sets it: an armor, cloak or tabard type goes to `0x478aa0`, a weapon type to a held-item loader.
`0x478cb0(equipment slot, id)` and `0x478bd0(equipment slot)` set and clear by equipment slot; weapons, rings, trinkets and the neck have no body slot there. `0x478aa0(body slot, id)` looks up the display record and
calls `0x478ad0(body slot, record)`, which stores it, marks `+0x0a` and fills the cells. `0x478960(body slot)` clears.

| equipment slot | body slot |
|---|---|
| 0 head | 0 |
| 2 shoulders | 1 |
| 3 shirt | 2 |
| 4 chest | 3 |
| 5 waist | 4 |
| 6 legs | 5 |
| 7 feet | 6 |
| 8 wrists | 7 |
| 9 hands | 8 |
| 18 tabard | 9 |
| 14 back | 10 |
| quiver (record only) | 11 |

Addresses: `0x479010`, `0x478cb0`, `0x478bd0`, `0x478aa0`, `0x478ad0`, `0x478960`, `0x478f20`, `0x478c98`, `0x478c68`, `0x478da4`, `0x478d74`, `0x479158`, `0x47911c`, `0x478fec`, `0x478fb8`
Confidence: verified
Proof: one
Test: none

The byte tables `0x478c98` and `0x478da4` are equal (`00 0b 01 02 03 04 05 06 07 08 0b 0b 0b 0b 09 0b 0b 0b 0a`), and their jump tables put the arm for entry 9 at `0x478d63`/`0x478c5b`, which pushes body slot 10, and entry 10's at `0x478d54`/`0x478c50`, which pushes 9, so the back (entry 14) is slot 10 and
the tabard (18) slot 9. The set table `0x479158` and jump table `0x47911c` map inventory types: head 1, shoulders 3, shirt 4, chest 5, waist 6, legs 7, feet 8, wrists 9, hands 10, cloak 16, tabard 19 and robe 20 (the chest) to their body slot, weapons 13, 17 and 21 to `0x479e80`, shields 14 to `0x479f00`
and off-hand weapons 22 to `0x479ec0`; the rest do nothing. The clear table `0x478fec` and `0x478fb8` is the same without weapons. `0x479010` calls `0x478f20` at `0x47901b`, and stops at `0x479025 jle` for a non-positive id and at `0x47902e ja` above type 0x1a.
`0x478ad0` special-cases slot 0 (the helm, `0x4799a0`), 1 (shoulders, `0x479d10`), 10 (cloak, `0x479b40`) and 11 (quiver, `0x479c50`) and walks the layer choosers for the rest; a baked component handles only slots 0 and 1 (`0x478aef`, `0x478af6`).

## Layer choosers

A record carries eight texture names, one per composite group `g` (`+0x38 + 4·g`). For body slots 2 to 9, `0x478ad0` calls chooser `g` as `(body slot, record, 1)` for every group whose name is not empty and whose cell in the slot table is not -1. A chooser sets the item's cell at
column `c` of group `g`'s row and ORs `1 << g` into `+0x10`; with the store flag 0 it clears the cell instead. Group 6 also marks `+0x0a` when it overrides the column. The column is the table value `[0x803bf8 + 0x28·slot + 4·g]`, with the exceptions below.

| slot | g0 | g1 | g2 | g3 | g4 | g5 | g6 | g7 |
|---|---|---|---|---|---|---|---|---|
| 2 shirt | 0 | 0 | - | 0 | 0 | - | - | - |
| 3 chest | 1 | 1 | - | 1 | 1 | 1 | 1 | - |
| 4 waist | - | - | - | - | - | 2 | - | - |
| 5 legs | - | - | - | - | - | 0 | 0 | - |
| 6 feet | - | - | - | - | - | - | 2 | 0 |
| 7 wrists | - | 2 | - | - | - | - | - | - |
| 8 hands | - | 3 | 0 | - | - | - | - | - |
| 9 tabard | - | - | - | 4 | 4 | - | - | - |

Addresses: `0x4791c0`, `0x479210`, `0x479290`, `0x479300`, `0x479350`, `0x4793a0`, `0x4793f0`, `0x479520`, `0x4792e0`, `0x4792f0`, `0x803bf8`, `0xb42424`, `0x478b95`, `0x478ba0`, `0x478bb0`, `0x479238`, `0x479248`, `0x47941d`, `0x47943b`, `0x479468`, `0x479529`, `0x479538`, `0x476c5a`, `0x479496`, `0x4794b9`, `0x4794cb`, `0x479508`
Confidence: verified
Proof: one
Test: none

`-` is the table value -1; rows 0, 1, 10 and 11 of the table are never used. `0x478b95 cmp BYTE PTR [ecx],0` tests the first byte of the name, not the pointer. The overrides: hands whose record has geoset group 0 use column 6 in group 1 (`0x479238`), a chest with group 0 column 5 (`0x479248`). In group 6, feet with group 0
use column 3 (`0x47941d`), a chest with group 2 column 4 (`0x47943b`), and legs with group 2 column 3 under a chest with group 2 (`0x479468`), else 4. Group 7 clears its cell on every call when `+0x0b` is set (`0x479538`), which is the race flag of `component#create-build-and-delete`.
On the clear path group 6 first clears column `c` (`0x479496`); clearing the chest (`0x47949b cmp edi,3`) then clears column 3 (`0x4794b9`), re-stores the legs record at column 4 when it has group 2 (`0x4794cb`) and re-stores the feet record at column 3 when it has group 0 (`0x479508`). The legs branch (`0x4794e8`) reads `+0x4bc`, which unequip has already cleared, so it stores nothing.
`0x4792e0` and `0x4792f0` are the table's slots 8 and 9, `ret 0xc` and never indexed.

## Cell store

`0x478900(group, record, column)` records an item on cell `16·group + column` of the equipment grid unless the component is baked. When a record already sits on the cell and its texture name pointer for the group equals the new record's, nothing changes; otherwise the old cell is released and the record is stored for `component#section-loading` to turn into a cell. `0x479180(group, column)` releases the cell and clears the record. `0x47a5c0(first)`
clears columns `first`-1 down to 2 of groups 3 and 4.

Addresses: `0x478900`, `0x479180`, `0x47a5c0`, `0x47b010`, `0x47891a`, `0x47892f`, `0x478935`, `0x479190`, `0x47a5ce`
Confidence: verified
Proof: one
Test: none

`0x47891a shl eax,4` and `0x47891d lea esi,[eax+edx]` form the index, used as `[edi + 4·esi + 0x228]` for the cell and `+0x248` for the record. `0x47892f cmp eax,[ebx+ecx*4+0x38]` compares the two name pointers, not the strings. The body slot never enters the index: two items that write one column share the cell. `0x47a5ce` starts at `+0x2e8 + 4·first`
and steps down by 4 for `first - 1` columns (`0x47a5d5`, `0x47a602`), for the group 3 row, the group 4 row 0x40 later (`0x47a5ed`) and their records (`0x47a5f2`, `0x47a5ff`).

## Unequip

`0x478960(body slot)` does nothing for an empty slot. Otherwise it marks `+0x0a`. Slot 0 puts the customization geosets back, detaches attachment 11 and clears the record; slot 1 detaches attachments 6 and 5; slot 11 detaches 0x1a. Slots 9 and 10 reset `+0x180` to `0x5dd`, and slot 9 clears the tabard
emblem cells. The other slots, and 9, clear the record and call all eight choosers with the old record and the store flag 0.

Addresses: `0x478960`, `0x47a380`, `0x47a5c0`, `0x478540`, `0x478740`, `0x4789df`, `0x478a0b`, `0x478a2a`, `0x478a46`, `0x478a58`, `0x478a72`, `0x478a8f`
Confidence: verified
Proof: one
Test: none

`0x47896a` reads the record and `0x478974 cmp eax,edi; je 0x478a97` leaves for an empty slot. The head branch is `0x47897c` to `0x4789df`, calling `0x47a380(model, 0xb)` at `0x47899c`. `0x4789f4 mov edx,6` and `0x478a01 mov edx,5` detach the shoulders, `0x478a20 mov edx,0x1a` the quiver. The choosers are called from `0x478a7c`
with the loop bound `0xb42444`, eight entries, record `ebp+8`, store flag 0 (`0x478a84`). `0x478a72` clears the record before the choosers run, so the choosers see the slot as empty.

## Held items

`0x47a0c0(model, record, slot, sheathe type, sheathed, shield, ranged)` hangs a weapon or shield model on a character model. Slot 15 is the main hand (attachment 1), 16 the off hand (attachment 2, or 0 for a shield with the shield flag), 17 the ranged slot (1 when the ranged flag is set, else 2). The sheath attachment comes from
`0x47a070(sheathe type, hand flag)`. It detaches both the hand and the sheath attachment first, picks the sheath one when `sheathed` is set, returns -1 when the model is loaded and lacks the attachment, and loads the record's weapon or shield model, with the record's `+0x04` name and `+0x0c` texture.
The attachment id is returned.

Addresses: `0x47a0c0`, `0x47a070`, `0x47a380`, `0x47a310`, `0x47a230`, `0x479e80`, `0x479ec0`, `0x479f00`, `0x47a0a8`, `0x838d50`, `0x838d8c`, `0x7103d0`, `0x712cb0`, `0x7130a0`, `0x4798c0`
Confidence: verified
Proof: one
Test: none

`0x47a0f3 sub eax,0xf` splits the slots. `0x47a070(sheathe type, flag)` returns -1 for type 0, `0x1b`, `0x1f` and `0x21` for types 1 to 3, each less 1 when the hand flag is set, `0x1c` for 4 and -1 above (`0x47a084` to `0x47a09c`, table `0x47a0a8`). `0x47a149` and `0x47a153` call `0x47a380` for the hand and sheath ids; `0x47a380(model, id)` opens the hand
of attachment 0 or 2 (left) or 1 (right) with `0x4796b0`, and detaches the attachment when the model is loaded and has it. `0x47a15d mov al,[ebp+0x10]` picks the attachment, `0x47a16f call 0x7103d0` and `0x47a17b call 0x712cb0` test it. `0x47a310(model, slot, ...)` does the same detaches for slots 15 to 17 and `0x47a230` detaches attachments
0xf to 0x12, 0x1d, 0x13 to 0x19, 0x22 and 0x23, then sets the model's emissive colour to 0, its diffuse colour to 1 and its alpha to 1.0 (`0x47a2d9`, `0x47a2f9`, `0x47a305`). `0x479e80` stores the record at `+0x4d8` and calls `0x47a0c0` with slot 15; `0x479ec0` and `0x479f00` store it at `+0x4dc` with slot 16, the shield one with the shield flag.

## Item models

`0x4798c0(model, attach id, model path, texture path, visual id)` creates a model from the path, binds the texture as replaceable type 2, and, when the visual id is positive, attaches that visual's effect models (`0x479700`: up to five, at attachments 0 to 4 of the item model). It detaches whatever sits at the attach id on the character model and attaches the new one.
An item whose texture fails to load is not attached. Attachment ids 1 and 2 then close the right and left hand (`hand grips`). The cloak is a texture on the character model itself (`0x479b40`, type 2); shoulders are two models at attachments 6 and 5 (`0x479d10`) and the quiver one at 0x1a (`0x479c50`).

Addresses: `0x4798c0`, `0x479700`, `0x479b40`, `0x479c50`, `0x479d10`, `0x479f40`, `0x7105c0`, `0x707350`, `0x712f70`, `0x7103a0`, `0x710ec0`, `0x449d90`, `0x713070`, `0x838cbc`, `0x838cd8`, `0x838cf4`, `0x838d10`, `0x838d30`, `0x82e424`, `0xc0db70`, `0xc0db74`, `0xc0db84`, `0xc0db88`
Confidence: verified
Proof: one
Test: none

`0x4798da call 0x7105c0` and `0x4798e1 call 0x707350` make the model; `0x479917 call 0x449d90` loads the texture, `0x479924 call 0x7103a0` drops the model when it fails, `0x479935 call 0x710ec0` binds type 2. `0x479944` tests the visual id; `0x479955 call 0x7130a0` detaches, `0x47995e call 0x712f70` attaches, `0x479965 call 0x7103a0` releases the creation reference.
`0x479700` detaches the item model's children (`0x713070`), looks the visual up in `[0xc0db70]` (bound `[0xc0db74]`), takes its five effect ids (`+4` on), looks each up in `[0xc0db84]` (bound `[0xc0db88]`) and attaches a model made from the record's `+4` path (`0x47978c call 0x712f70`). `0x479f40(model, record, kind)` makes a loose model for an ammo (0x18) or weapon (0x19) record from the record's second name and texture, plays animation 0 on it
and returns it. The directories are `Item\ObjectComponents\Cape\`, `Quiver\`, `Shoulder\`, `Head\`, `Weapon\`, `Shield\` and `Ammo\`.

## Hand grips

A held weapon closes its hand. `0x479660(hand)` plays animation 15, HandsClosed, as a primary play with a drawn variation and no blend on the finger key bones of a hand, 8 to 12 for hand 0 (right) and 13 to 17 for hand 1. `0x4796b0(hand)` plays animation -1 on the same key bones, which stops their request. `0x4798c0` closes the right hand
after attaching at attachment 1 and the left after attachment 2; `0x47a380` opens the hand of the attachment it detaches. A shield on attachment 0 attaches without closing a hand.

Addresses: `0x479660`, `0x4796b0`, `0x7121a0`, `0x479683`, `0x479695`, `0x4796d3`, `0x4796e5`, `0x47996f`, `0x479971`, `0x479986`, `0x47998d`
Confidence: verified
Proof: one
Test: none

The pushes before `0x479695 call 0x7121a0` are 1, 0, 1.0, 0, -1, 15 and the key bone: primary, blend, speed, offset, variation, id, key bone (`playback`). `0x4796e5` is identical with id -1 (`0x4796e0 push 0xffffffff`). `0x479667 jne` splits the hands: 8 and 12 at `0x479669`, 13 and 17 at `0x479675`. `0x47996a cmp edi,1` closes hand 0 and `0x479981 cmp edi,2`
hand 1, in `0x4798c0`. `0x47a39d` opens through `0x4796b0`.

## Guild tabard

`0x47a610(emblem style, emblem color, border style, border color, background color)` fills the tabard cells of groups 3 and 4 from the guild emblem files `Textures\GuildEmblems\Background_%02d_TU_U`, `Border_%02d_%02d_TU_U` and `Emblem_%02d_%02d_TU_U` for group 3, and the `_TL_U` names for group 4, as columns 2 (background), 3 (border) and 4 (emblem) of the row, loaded as kind 9.
It first clears columns 4 to 2 with `0x47a5c0(4)` and marks groups 3 and 4 dirty. The path builders are `0x47a4d0` (background), `0x47a570` (border) and `0x47a520` (emblem). `0x47a860` clears the tabard preview flag `+0x0c` and the same cells.

Addresses: `0x47a610`, `0x47a4d0`, `0x47a520`, `0x47a570`, `0x47a860`, `0x47a5c0`, `0x838dd8`, `0x838dac`, `0x838e30`, `0x838e04`, `0x838e88`, `0x838e5c`, `0x47a616`, `0x47a653`, `0x47a6a9`, `0x47a706`, `0x47a761`, `0x47a7c2`, `0x47a81d`, `0x47a84b`, `0x6d6d20`, `0x472ca4`, `0x504852`, `0x5d648a`, `0x5e075c`
Confidence: verified
Proof: one
Test: none

The arguments are read as `ebp+0x18` background color, `ebp+0x14`/`ebp+0x10` border color and style, `ebp+0xc`/`ebp+8` emblem color and style (`0x47a61f`, `0x47a6d0`, `0x47a6d3`, `0x47a78c`, `0x47a78f`). The loads are `0x47a653` (background, group 3), `0x47a6a9` (group 4), `0x47a706`, `0x47a761` (emblem) and `0x47a7c2`, `0x47a81d` (border). Each path builder writes an empty string for a type other than 3 and 4 and formats with `0x64a7f0`. `0x47a84b`
ORs `0x18` into `+0x10`. Four of the seven callers test bit 0 of the tabard display's `+0x24` first (`0x472ca4`, `0x504852`, `0x5d648a`, `0x5e075c`) and fetch the fields with `0x6d6d20`, which fails when any of the five cache values (`+0x2e4` to `+0x2f4`) is -1. `0x47a860` is called from `0x5e0857`.

## Creature skins

`0x4797b0(model, display record, model data)` binds up to three skin textures of a creature display. The directory is the part of the model data's path (`+8`) up to its last backslash; each non-empty texture name at display `+0x18`, `+0x1c`, `+0x20` is joined to it, loaded, and bound to replaceable type 11, 12 or 13. It does nothing when any of the three
arguments is null.

Addresses: `0x4797b0`, `0x479812`, `0x479865`, `0x479878`, `0x479891`, `0x8362d8`, `0x882748`, `0x64a5a0`, `0x64a450`
Confidence: verified
Proof: one
Test: none

`0x479812 mov dl,0x5c` and `0x47981a call 0x64a450` find the last backslash and `0x479823 mov [eax+1],0` cuts after it. `0x479865 call 0x64a7f0` joins with the format `%s%s%s` (`0x8362d8`) and the empty string `0x882748`, `0x47987e call 0x449d90` loads, and `0x479891 call 0x710ec0` binds with `edi + 0xb`.
