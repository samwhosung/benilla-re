# objectlayer: game objects

`CGGameObject_C`: the game object, the per-type handler it builds from `GAMEOBJECT_TYPE_ID`, its
server record and data columns, its world transform, the watchers and messages that change it,
and how the player interacts with it: whether it highlights, the cursor, whether it is usable, and
using it, locks included. The handlers' slots, animation and per-type behaviour are
[gameobjecttypes](gameobjecttypes.md); transports are [transports](transports.md).

GameObject_C.cpp's code passes its source path from `0x5f2f80` to `0x5f8ae0`. Before it lie its
initializers and class template code the file instantiates (`0x5f1630` to `0x5f2e20`); after it
come more initializers and the ship path code (`transports`). The type and data-column tables
live in a small file after Effect_C.cpp, `0x6219f0` to `0x621b60`. `[go+0x110]` points at field 6:
`+0x20` is `GAMEOBJECT_STATE`, `+0x3c` `GAMEOBJECT_TYPE_ID`, by the client's own field table
`0x83b880`.

Roots: `0x5f6f30`, `0x5f7c40`, `0x5f8820`, `0x5f8b20`, `0x5f8660`, `0x5f8760`, `0x5f8790`, `0x5f8260`, `0x5f8180`, `0x5f8100`, `0x5f8c30`, `0x5f8c40`, `0x5f8c90`, `0x47cff0`

## The game object

The constructor builds the object part, 20 empty watcher lists at `+0x118`, an identity world
matrix at `+0x218`, an empty world box at `+0x260`, and copies the update's position and facing
into `GAMEOBJECT_POS_X` to `GAMEOBJECT_FACING`. `+0x27c` takes `GAMEOBJECT_STATE`; with update flag
0x2, `+0x278` takes the block's last u32 minus the time. It then builds the type handler at
`+0x210`. The destructor cancels a pending record request, deletes the handler, unlinks the
transport link `+0x208` and the lists, and runs the object's.

| member | holds |
|---|---|
| `+0x210` | the type handler |
| `+0x214` | the record, once it arrives |
| `+0x280` | an animation event waiting for the model |
| `+0x284` | 1 when the object collides |

Addresses: `0x5f6f30`, `0x5f6f48`, `0x5f6fd2`, `0x5f7013`, `0x5f703e`, `0x5f7046`, `0x5f706e`, `0x5f7750`, `0x5f7780`, `0x5f6ef0`, `0x6139a0`, `0x613b40`, `0x83b880`
Confidence: verified
Proof: one
Test: none

`0x5f6f30(time, block)` calls `0x6139a0`, loops `0x14` heads from `+0x118` through 0x469070, zeros
`+0x208` to `+0x214`, stores 1.0 at `+0x218`, `+0x22c`, `+0x240`, `+0x254` and 0 between, zeros
`+0x260` to `+0x274`, loads `[fields+0x20]` into `+0x27c` (`0x5f7013`), zeros `+0x280` and `+0x284`,
then passes the GUID to 0x468270 (`0x5f703e`). `0x5f7046` copies `block+0x24` to `+0x30`;
`0x5f706e` tests `[block+0xd8]` bit 2 and stores `[block+0xf0] − time`. `0x5f7780` calls the
record cache's cancel 0x558bf0 when `+0x214` is null, the handler's slot 0 with 1, unlinks `+0x208`
and the 20 lists and calls `0x613b40`. `0x83b880` holds 20-byte records: name, index in the
section, count, type, flags.

## Building the type handler

The constructor switches on `GAMEOBJECT_TYPE_ID` from 0 to 30: each case allocates the type's
handler and runs its constructor with the object. Type 21 and any type above 30 get a shared
static handler and log `BADBASEGAMEOBJECT`. A failed allocation leaves no handler. Every built
handler starts with a vtable, the object at `+0x4`, an error at `+0x8` and a use range at `+0xc`,
by default `ERR_USE_LOCKED` and 5.0 yards. The static handler has only its vtable; its `+0x4` to
`+0xc` stay zero, unread because its slots 5 and 6 return 0.

Addresses: `0x5f708f`, `0x5f7098`, `0x5f76cc`, `0x5f768d`, `0x5f76a0`, `0x5f2f50`, `0xc4d840`, `0x5f36d0`, `0x5f36e0`, `0x5f3730`, `0x5f36f0`, `0x5f3700`, `0x80b100`, `0x80b188`, `0x80b084`
Confidence: verified
Proof: one
Test: none

`cmp ecx,0x1e; ja 0x5f76a0; jmp [ecx*4+0x5f76cc]`. Each case pushes the line 0xa21 + 3·k, the
file and its size to `SMemAlloc`, calls the handler constructor and stores `eax` at `+0x210`; a
null allocation jumps to `0x5f768d`, which stores 0. Entry 21 of the table is `0x5f76a0`: it stores
`0xc4d840` and calls `SysMsgPrintf(1, 2, "BADBASEGAMEOBJECT|%d", type)`. The initializer `0x5f36e0`
stores the vtable `0x80b188` there and nothing else, and its exit callback `0x5f3730` the base
`0x80b100`; `0x80b188`'s slots 5 and 6 are `0x5f36f0` and `0x5f3700`, `xor al,al`.
`0x5f2f50(go)` stores `0x80b100`, the object, `0xdb` at `+0x8` and `[0x80b084]` (5.0) at `+0xc`.

## Handler types 0 to 10

The handlers by type, with the client's own type names, their size, constructor and vtable.
Family A handlers animate the model (`gameobjecttypes`); B do not. Each handler's range and error
are the defaults unless the table says.

| type | name | size | constructor | vtable | destructor | family, range, error |
|---|---|---|---|---|---|---|
| 0 | door | 0x1c | `0x5f4290` | `0x80b2a0` | `0x5f42c0` | A, `ERR_DOOR_LOCKED` |
| 1 | button | 0x1c | `0x5f43d0` | `0x80b330` | `0x5f4400` | A, `ERR_BUTTON_LOCKED` |
| 2 | questgiver | 0x1c | `0x5f45a0` | `0x80b570` | `0x5f45d0` | A, 5.5555553 |
| 3 | chest | 0x1c | `0x5f4430` | `0x80b3c0` | `0x5f4460` | A |
| 4 | binder | 0x10 | `0x5f4770` | `0x80b600` | `0x5f47a0` | B, 10.0 |
| 5 | generic | 0x10 | `0x5f47d0` | `0x80b688` | `0x5f4800` | B |
| 6 | trap | 0x1c | `0x5f4490` | `0x80b450` | `0x5f44c0` | A |
| 7 | chair | 0x4c | `0x5f55f0` | `0x80b820` | `0x5f5640` | B, 100.0 |
| 8 | spellFocus | 0x1c | `0x5f57b0` | `0x80b8a8` | `0x5f5800` | A |
| 9 | text | 0x1c | `0x5f5830` | `0x80b938` | `0x5f5860` | A, 5.5555553 |
| 10 | goober | 0x1c | `0x5f5a50` | `0x80b9c8` | `0x5f5a80` | A |

Addresses: `0x5f37c0`, `0x5f37f0`, `0x5f4290`, `0x5f43d0`, `0x5f45a0`, `0x5f4430`, `0x5f4770`, `0x5f47d0`, `0x5f4490`, `0x5f55f0`, `0x5f57b0`, `0x5f5830`, `0x5f5a50`, `0x80b2a0`, `0x80b330`, `0x80b570`, `0x80b3c0`, `0x80b600`, `0x80b688`, `0x80b450`, `0x80b820`, `0x80b8a8`, `0x80b938`, `0x80b9c8`, `0x80d79c`, `0x5f42c0`, `0x5f4400`, `0x5f45d0`, `0x5f4460`, `0x5f47a0`, `0x5f4800`, `0x5f44c0`, `0x5f5640`, `0x5f5800`, `0x5f5860`, `0x5f5a80`
Confidence: verified
Proof: one
Test: none

The sizes are the cases' `push` before `SMemAlloc`; each constructor calls `0x5f2f50` and stores
the vtable (`0x5f42a6` for the door). The door stores 0xdc and the button 0xdd at `+0x8`; the
binder `[0x80b088]`, the chair `[0x80b0b0]`, the quest giver and text `[0x80b07c]` at `+0xc`. Family
A constructors zero `+0x18`, and their vtables run 36 slots against B's 34; slot 2 is `0x5f37e0` in
every A vtable and `0x5f9da0` in every B. The names are the strings at `+0x0` of the 20-byte
records of `0x80d79c`. Slot 0 is the deleting destructor: A's call `0x5f37c0`, which stores
`0x80b210`, stops the looping sound and stores `0x80b100`, as does `0x80b210`'s own `0x5f37f0`; B's
store `0x80b100`; each frees on bit 0.

## Handler types 11 to 20

| type | name | size | constructor | vtable | destructor | family, range, error |
|---|---|---|---|---|---|---|
| 11 | transport | 0x5c | `0x5f5ab0` | `0x80ba58` | `0x5f5c80` | B, over the map object's |
| 12 | areaDamage | 0x1c | `0x5f44f0` | `0x80b4e0` | `0x5f4520` | A, 0.0 |
| 13 | camera | 0x10 | `0x5f6570` | `0x80bae0` | `0x5f6590` | B |
| 14 | mapobject | 0x14 | `0x5f4860` | `0x80b710` | `0x5f48d0` | B |
| 15 | moTransport | 0x60 | `0x5f4ab0` | `0x80b798` | `0x5f4b90` | B, over the map object's |
| 16 | duelFlag | 0x1c | `0x5f65c0` | `0x80bb68` | `0x5f6610` | A |
| 17 | fishingNode | 0x1c | `0x5f66b0` | `0x80bc80` | `0x5f66e0` | A, 100.0 |
| 18 | ritual | 0x1c | `0x5f6760` | `0x80bd10` | `0x5f6790` | A |
| 19 | mailbox | 0x1c | `0x5f67c0` | `0x80bda0` | `0x5f67f0` | A |
| 20 | auctionHouse | 0x10 | `0x5f6880` | `0x80be30` | `0x5f68b0` | B |

Addresses: `0x5f5ab0`, `0x5f44f0`, `0x5f6570`, `0x5f4860`, `0x5f4ab0`, `0x5f65c0`, `0x5f66b0`, `0x5f6760`, `0x5f67c0`, `0x5f6880`, `0x80ba58`, `0x80b4e0`, `0x80bae0`, `0x80b710`, `0x80b798`, `0x80bb68`, `0x80bc80`, `0x80bd10`, `0x80bda0`, `0x80be30`, `0x5f5c80`, `0x5f4520`, `0x5f6590`, `0x5f48d0`, `0x5f4b90`, `0x5f6610`, `0x5f66e0`, `0x5f6790`, `0x5f67f0`, `0x5f68b0`
Confidence: verified
Proof: one
Test: `objectlayer::g3_transport_ctor_add`

The transport constructors run the map object's `0x5f4860` first, which stores `0x80b710`, and
store their own vtable over it (`0x5f5b1a`, `0x5f4b0a`). The area damage constructor zeros `+0xc`;
the fishing node stores `[0x80b0b0]`. Slices: g3_transport_ctor_add `0x5f5bce`-`0x5f5bf4`.

## Handler types 21 to 30

| type | name | size | constructor | vtable | destructor | family, range, error |
|---|---|---|---|---|---|---|
| 21 | guardPost | none | static `0xc4d840` | `0x80b188` | `0x5f3740` | B |
| 22 | spellCaster | 0x10 | `0x5f68e0` | `0x80beb8` | `0x5f6900` | B |
| 23 | meetingStone | 0x1c | `0x5f6930` | `0x80bf40` | `0x5f6960` | A |
| 24 | flagStand | 0x1c | `0x5f6bf0` | `0x80bfd0` | `0x5f6c20` | A, 5.5555553 |
| 25 | fishingHole | 0x10 | `0x5f6640` | `0x80bbf8` | `0x5f6680` | B |
| 26 | flagDrop | 0x1c | `0x5f6c50` | `0x80c060` | `0x5f6c80` | A, 5.5555553 |
| 27 | minigame | 0x1c | `0x5f6cb0` | `0x80c0f0` | `0x5f6ce0` | A, 5.5555553 |
| 28 | lotteryKiosk | 0x1c | `0x5f6db0` | `0x80c210` | `0x5f6de0` | A |
| 29 | capturePoint | 0x1c | `0x5f6d10` | `0x80c180` | `0x5f6d50` | A |
| 30 | auraGenerator | 0x1c | `0x5f6e70` | `0x80c2a0` | `0x5f6ec0` | A |

Addresses: `0x5f68e0`, `0x5f6930`, `0x5f6bf0`, `0x5f6640`, `0x5f6c50`, `0x5f6cb0`, `0x5f6db0`, `0x5f6d10`, `0x5f6e70`, `0x80b188`, `0x80beb8`, `0x80bf40`, `0x80bfd0`, `0x80bbf8`, `0x80c060`, `0x80c0f0`, `0x80c210`, `0x80c180`, `0x80c2a0`, `0x80b210`, `0x5f3740`, `0x5f6900`, `0x5f6960`, `0x5f6c20`, `0x5f6680`, `0x5f6c80`, `0x5f6ce0`, `0x5f6de0`, `0x5f6d50`, `0x5f6ec0`
Confidence: verified
Proof: one
Test: none

The vtables lie back to back from `0x80b100` to `0x80c330`, bounded by the constructors' stores:
twelve of 0x88 bytes (the base, the static one and B) and 21 of 0x90 (A, and `0x80b210`, which only
the family A destructor `0x5f37c0` stores) fill the 0x1230 bytes exactly.

## The game object record

A game object's record comes from the game object record cache by its entry: the creation hook
and slot 3 ask for it, and its arrival stores it at `+0x214` and calls the handler's slot 24. The
record holds the name at `+0x8`, a cursor name at `+0x18` and 24 data dwords from `+0x1c`. A type names its data columns by semantic ids:
`0x621b00(type, id)` gives the column, or −1, and `0x5f8150(column)` reads it, 0 without a record or
for a column past 23.

Addresses: `0x5f7c40`, `0x5f7d81`, `0x5f78a0`, `0x5f7fb0`, `0x5f7ed0`, `0x5f8150`, `0x621b00`, `0x5f8100`, `0x80c9fc`, `0x80d79c`
Confidence: verified
Proof: one
Test: none

`0x5f7c40` asks 0x558560 on the cache 0xc0e318 with the entry, the GUID and the callback `0x5f78a0`,
which finds the object (mask 0x20) and calls `0x5f7fb0`. `0x5f8150` tests `cmp ecx,0x18; jae` and
reads `[rec+4·i+0x1c]`. `0x621b00` takes `ecx` below 31 and `edx` below 87, then scans the type's id
array at `[0x80d7a4 + 20·type]`, counted by `+0x80d7a0`. `0x80c9fc` holds 87 16-byte records naming
each id: 1 `startOpen`, 3 `autoClose`, 4 `open`, 0xb `chairslots`, 0xf `pageID`, 0x12
`highlight`, 0x13 `floatingTooltip`, 0x16 `customAnim`, 0x23 `taxiPathID`, 0x24 `moveSpeed`, 0x2b
`accelRate`, 0x33 and 0x34 `minLevel` and `maxLevel`. `0x5f8100` returns the name or `""`.

## World transform and box

When the model loads, the object's world matrix becomes the translation by its position, the
rotation by `GAMEOBJECT_ROTATION` and the scale by `OBJECT_FIELD_SCALE_X`; the model gets it and
the world entity its box. The world box `+0x260` is the model's collision box carried by the
matrix, and the object collides when the handler's slot 1 says so and the box is non-empty on
every axis. The art kit's textures and models go on, and an animation event waiting for the
model plays.

Addresses: `0x5f7910`, `0x5f7962`, `0x5f7979`, `0x5f798b`, `0x5f79ad`, `0x5f7a2b`, `0x5f7a60`, `0x5f7a6d`, `0x5f7aac`, `0x5f7ac0`, `0x5f7acd`, `0x5f7add`, `0x5fa050`, `0x5f85f0`, `0x5fa010`, `0x7bdc40`, `0x7bddb0`, `0x7bdd00`, `0x6717d0`, `0x710620`
Confidence: verified
Proof: one
Test: `objectlayer::go_animate_bounds_difftest`

`0x5f7910(model)` resets `+0x218`, calls `TranslateMatrix` with slot 5's point, `RotateMatrixByQuaternion`
with `fields+0x10` and `ScaleMatrix` with slot 7; the model's bounding box goes to `+0xc0` and its
height to `+0xbc` (`0x5f79ad`). With a world object at `+0xe0`, `SetWorldObjectBounds` takes the
matrix, the box and its centre, `(min + max)·0.5`. `0x5f7a60` calls 0x6dc540(matrix, box, `+0x260`).
The non-empty test compares min with max per axis, `fcomp` with `test ah,5; jp` (a NaN fails).
Then `0x5f7b00(model, GAMEOBJECT_ARTKIT)`, the handler's slot 30, `0x5f8c50(+0x280)` and
`CM2Model::SetUserData(1)`. Slot 25 (`0x5fa050`) copies `+0x218`; slot 20 (`0x5f85f0`) returns
`+0x284`, but 0 for a door given flag 0x8000; slot 19 (`0x5fa010`) is `+0x284` or slot 21. Slices: go_animate_bounds_difftest `0x5f79ad`-`0x5f7a01`.

## Object slots

Deactivation runs the handler's slot 26 once the record is there, else cancels the record request,
then the object's own. Reactivation runs the object's own, then the handler's slot 25 and, with a
model, 0x613e10; without the record it passes the GUID to 0x468270 again. The model path is the
display row's, none for types 11, 14 and 15, whose world entity carries the model. Slot 23
returns whether the `floatingTooltip` column is set. `0x5f8c90(x)` hands the world object at
`+0xe0` to `CWorld::UpdateLightObject`.

Addresses: `0x5f7e40`, `0x5f7e80`, `0x5f80e0`, `0x5f8630`, `0x5f8c90`, `0x5f8090`, `0x6145e0`, `0x614640`, `0x671a90`
Confidence: verified
Proof: one
Test: none

`0x5f7e40` calls `[handler]+0x68` when `+0x214` is set, else 0x558bf0(entry, `0x5f78a0`, object)
on 0xc0e318, and jumps to `0x6145e0`. `0x5f7e80` calls `0x614640`, then `[handler]+0x64` and
0x613e10 when `+0xdc` or `+0xd8` holds a model. Slot 10, `0x5f80e0`, returns 0 for type 11 and for
14 and 15 (`jle 0xd`, `jle 0xf`) and jumps to `0x5f8090` otherwise. `0x5f8630` reads column
`0x621b00(type, 0x13)` and returns it as 0 or 1. `0x5f8c90` is `ret 4`, called from 0x482d2c.

## Watchers and messages

At start-up the game object code registers three message handlers and four type watchers;
shutdown clears them. A custom animation (0xb3) or despawn (0x215) message sends an animation
event; 0x1df passes the object's GUID to the interface.

| on | handler | does |
|---|---|---|
| 0x1df | `0x5f88c0` | 0x4e32e0(&guid, 0) |
| 0xb3 | `0x5f8930` | event `n + 2` for `n` below 4 |
| 0x215 | `0x5f8990` | event 6 |
| `GAMEOBJECT_FLAGS` | `0x5f8a30` | handler slot 8 with the changed bits |
| `GAMEOBJECT_STATE` | `0x5f89e0` | `+0x27c`, handler slot 10 (old, new) |
| `GAMEOBJECT_ARTKIT` | `0x5f8a80` | the art kit on the loaded model |
| `GAMEOBJECT_ANIMPROGRESS` | `0x5f8ae0` | handler slot 9 |

Addresses: `0x5f8820`, `0x5f8b20`, `0x5f88c0`, `0x5f8930`, `0x5f8990`, `0x5f89e0`, `0x5f8a30`, `0x5f8a80`, `0x5f8ae0`, `0x5f8bd0`, `0x5f8c00`, `0x5f8ba0`, `0x5f8bc0`, `0x5f8c50`, `0x5ab650`, `0x5ab670`, `0x468070`, `0x468240`
Confidence: verified
Proof: one
Test: none

`0x5f8820` pairs `ecx` opcodes with `edx` handlers for `0x5ab650` and calls `0x468070(5, offset,
4, watcher, 0, 0)` for offsets 0x20, 0xc, 0x44, 0x48. 0xb3 reads a GUID and a u32 `n`. `0x5f8bd0`
stores a new state and calls the handler's slot 10 only when it differs. `0x5f8c50(event)` with the
model loaded calls the handler's slot 15 with `[0x80b0e0 + 4·event]` and clears `+0x280`; without
it stores the event at `+0x280`. The table maps events 0 to 7 to −1, 0, 8, 9, 10, 11, 12, 0.

## Interactable and mouseover

The handler's slot 5 says whether the object can be interacted with at all; most types use one
test. The active player must exist; the object's faction must not be hostile to it, except a
trap, which must have a lock and be hostile, and whose creator, when a player present without
unit flag 0x1000, refuses. Then `GAMEOBJECT_FLAGS` 0x1 or 0x10 refuses, as does a pending
release, and flag 0x4 requires dynamic flag 0x1. Slot 3, the mouseover test, forwards to slot 5 except as
`gameobjecttypes` lists.

Addresses: `0x5f2f80`, `0x5f2fae`, `0x5f2fc9`, `0x5f2fdb`, `0x5f3013`, `0x5f3029`, `0x5f3034`, `0x5f303a`, `0x5f304f`, `0x5f8620`, `0x5f8800`, `0x5f9db0`, `0x5f7fd0`, `0x614f40`
Confidence: verified
Proof: one
Test: none

`0x5f2f80` gets the player (mask 0x10); `0x5f2fae cmp [+0x3c],6`. `0x614f40` returns bit 0x10 of `+0xe4`. Trap: `0x5f8180` must return a
lock and `CGGameObject_C::ObjectReaction` at most 1 (`0x5f2fc9 jg`); `and ecx,0xfffffff` on the
`GAMEOBJECT_CREATED_BY` high dword equal to itself marks a player (`0x5f2fdb`), found by mask 0x10,
and `0x5f3013` tests its `UNIT_FIELD_FLAGS` bit 12. Others: `0x5f3029 jle` refuses a reaction of 1
or less. `0x5f3034 test [+0xc],0x11`; `0x5f303a` calls `0x614f40`; `0x5f304f` tests flag 0x4,
then `[+0x34]` bit 0. The object's slot 21, `0x5f8620`, calls the handler's slot 3, which is
`0x5f9db0` (a jump through slot 5) in most vtables.

## Cursor

The cursor over a game object is 0 when it is not interactable. A record cursor name gives the
custom cursor 41, drawn from `Interface\Cursor\<name>.blp`, or `Unable<name>.blp` when not usable.
Otherwise a lock with a `LockType` cursor name gives that cursor, or its `Unable` form when the
lock is no key and the object is not usable; without one, `Interact` or `UnableInteract`.

Addresses: `0x5f8760`, `0x5f8790`, `0x5f8810`, `0x5f3070`, `0x5f30a5`, `0x5f30cd`, `0x5f30f0`, `0x5f3112`, `0xc0dac8`, `0x853b88`
Confidence: verified
Proof: one
Test: none

`0x5f8760` tests `[rec+0x18]` and returns `neg al; sbb; and eax,0x29` of slot 5; else the handler's
slot 4. `0x5f8790(buffer, size)` formats the path with `0x5f8810`, slot 6 with (0, 0). `0x5f3070`
reads slot 6 into `edi`, takes the lock's index 0 (`0x5f30a5`) in `LockType` (`0xc0dac8`) and its
`+0x70`; type 1 (`0x5f30cd`) or usable copies the name, else `"Unable%s"` (`0x5f30f0`), and the
cursor is its id in `0x853b88`. `0x5f3112` returns 5 or 25.

## Usable

The handler's slot 6 decides whether the player may use the object now and the range to use it
from, refusing with an error: a dead player; a locked-out client; for an object without a lock, a
mounted player (but at a mailbox) or a shapeshift form without the item flag; combat when the
object forbids it; an immune player at a chest or an object whose `noDamageImmune` column is set;
a locked object whose lock the player cannot open, with the handler's error; a destroyed object;
or distance past the range.

Addresses: `0x5f3130`, `0x5f315f`, `0x5f316e`, `0x5f31a2`, `0x5f31b2`, `0x5f31df`, `0x5f3231`, `0x5f326c`, `0x5f32a6`, `0x5f32e5`, `0x5f3328`, `0x5f3346`, `0x5f3392`, `0x5f5970`, `0x5f59a0`, `0x47cff0`, `0x605f30`, `0x6e3480`
Confidence: verified
Proof: one
Test: `objectlayer::go_canuse_dist2_difftest`

`0x5f3130(error, range)`: `IsDeadOrGhost` gives 0x7e; `0xb4b3e4` 0 gives 0x7f. No lock: a positive
`UNIT_FIELD_MOUNTDISPLAYID` with `0x47cff0` false (type not 19) gives 0x19f; a form without bit 0x8
0x1a0. Unit flag 0x80000 with `0x5f5970` false (`notInCombat` set) gives 0x1a1; 0x80000000 with
`0x5f59a0` false (a chest, or `noDamageImmune` set) 0x169. `GAMEOBJECT_FLAGS` bit 0x2 with
`0x5f83d0` returning 1 gives `[handler+8]`. The range is `[handler+0xc]`, or the opening spell's
maximum (`0x5f3328`) for the distance test only: the range handed back is always `[handler+0xc]`,
written at `0x5f32f4` before the spell's; without a spell, state 2 gives 0xe5. `0x5f3346`: `(dx² + dy² + dz²)`
against range², `test ah,5; jp`; farther gives 0xe1, a NaN passes. Slices: go_canuse_dist2_difftest `0x5f3346`-`0x5f3377`.

## Using an object while mounted

A mounted player is refused an object only when it has no lock row, and the refusal is silent.
The test is the existence of the Lock row, not its contents: a row whose slots are all empty still
lets a mounted player use the object. Only the mailbox, type 19, is exempt without a row. The
refusal, error 0x19f, has no text in the 1.12.1 global strings, so the message is empty and is
dropped before any event, and nothing is sent. A locked object opened by a spell reaches the cast,
whose validator reports "mounted".

Addresses: `0x5f3130`, `0x5f3195`, `0x5f319c`, `0x5f31a8`, `0x5f31bb`, `0x5f31d0`, `0x5f8180`, `0x47cff0`, `0x496720`, `0x4945b0`, `0x703bf0`, `0xc0dae4`, `0xc0dae8`, `0x6094f0`, `0x488021`
Confidence: verified
Proof: one
Test: none

`0x5f3195` calls `0x5f8180`, which returns the pointer `[0xc0dae4 + lockId*4]`, or 0 for no lock
field, a lock id of 0, a negative one, or one past `0xc0dae8`. `0x5f319c` jumps past the mount and
form tests when it is not 0. `0x5f31a8` reads the mount display, `0x5f31bb` asks `0x47cff0` (type
equals 0x13), and `0x5f31d0` stores 0x19f. `0x488021` registers the error row at `0xb4b498 + 0x19f*0x14`
with the key string `ERR_NOT_WHILE_MOUNTED`; `0x496720` reads the global through `0x703bf0`, which
leaves the empty string when no global exists, and `0x4945b0` returns on an empty message.

## Using a game object

For an interactable object, the object's slot 24 sets the player's auto-loot byte from its
argument and interacts: when usable, the handler's slot 7 uses it; out of range, except a fishing
node, the player walks to the handler's range, never the opening spell's, and tries again on
arrival; any other refusal shows its error. The default use casts the lock's opening spell with its
item on the object, or, with no spell, sends 0xb1 with the object's GUID. A lock the player
cannot open shows why.

Addresses: `0x5f8660`, `0x5f86b0`, `0x5f86dc`, `0x5f86f6`, `0x5f873e`, `0x5f33e0`, `0x5f341a`, `0x5f343b`, `0x5f3448`, `0x5f35bb`, `0x5f35e4`, `0x610300`, `0x5df460`, `0x6e5a90`, `0x6e5b40`, `0x5ab630`, `0x60faa4`
Confidence: verified
Proof: one
Test: none

Slot 24, `0x5f8660(modifier)`, returns unless slot 5 holds, then calls 0x5df460 with
`modifier != 0` and `0x5f86b0`. `0x5f86f6`
compares the error with 0xe1 and `0x5f873e` calls `CGUnit_C::WalkToGameObject(&guid, range)` with
that handler range. The walk's arrival, `CGUnit_C::CompleteInteractMove`, calls `0x5f86b0` again at
`0x60faa4`, without slot 5's test.
`0x5f33e0` calls `0x5f83d0(&spell, &item, 0, 0, &index)`; unlocked, `CastSpellAsActivePlayer(spell,
item, 0, 0)` and `BindSpellTargetObject`, or the 0xb1 packet (`0x5f35e4`). Locked: when the slot's
action fails `0x5f81d0`, a chest in state 0 gives `ERR_CHEST_IN_USE`. Key type 1 gives
`ERR_USE_LOCKED_WITH_ITEM_S` with the item's name; type 2 `ERR_USE_LOCKED_WITH_SPELL_KNOWN_SI`
with the lock type's name and the skill, or `ERR_USE_LOCKED_WITH_SPELL_S`; others
`ERR_USE_CANT_OPEN`.

## Locks

A lock is the `Lock.dbc` row its data column `open` names. Each of its eight slots has a key type,
an index, a skill and an action. A skill slot (type 2) opens with a known spell whose effect 33
names the index and whose points reach the slot's skill, or 5 times the level; an item slot
(type 1) with an item of the index's entry in the inventory. The first opener found wins; a lock
with key slots and no opener is closed.

Addresses: `0x5f8180`, `0x5f83d0`, `0x5f8450`, `0x5f84be`, `0x5f84e0`, `0x5f850f`, `0x5f8547`, `0x5f855f`, `0x5f81d0`, `0x5da5e0`, `0x5da69a`, `0x5da734`, `0x6e3760`, `0x622270`, `0xc0dadc`
Confidence: verified
Proof: one
Test: none

`0x5f8180` reads column `0x621b00(type, 4)` into `0xc0dadc`. `0x5f83d0(spell, points, required, item,
index)` walks slots 0 to 7: type at `+0x4`, index `+0x24`, skill `+0x44`, action `+0x64`; a slot
whose action fails `0x5f81d0` is skipped. Spells come in the order of the known-spell list
0xb700b8, effects 0 to 2 (`+0xf4`, misc `+0x1a8`); `GetSpellEffectPointsInt` gives the points and
`0x5f850f cmp eax,esi; jge` wins. `0x5f84be` takes the level from `GAMEOBJECT_LEVEL`.
`FindInventoryItemByEntry` finds a key; its use spell is reported. `0x5da5e0` is the same walk for
an item's lock, with no action test and the level an argument.

## Lock actions and spell targets

`0x5f81d0(action)` tests a lock slot's action against the object's state: action 4 needs the
destroyed state 2; the others refuse it, actions 0, 1 and 3 also need state 1; action 0 needs
`GAMEOBJECT_FLAGS` bit 0x2 clear, 1 needs it set, 2 needs state 0. `0x5f8260(&guid, spell)`, the
pending spell's target test, passes when an effect 33 matches a skill slot, or an effect 59 an
item slot of the item the GUID names, whose action holds.

Addresses: `0x5f81d0`, `0x5f81d6`, `0x5f81e1`, `0x5f81ef`, `0x5f820e`, `0x5f8228`, `0x5f8242`, `0x5f8260`, `0x5f82c0`, `0x5f831c`
Confidence: verified
Proof: one
Test: none

`0x5f81d6 cmp eax,4`; `0x5f81e1 cmp edx,2` on `+0x27c`; `0x5f81ef cmp eax,3; jne`, then `cmp edx,1`;
`0x5f820e` tests `[+0xc]` bit 1 for action 0, `0x5f8228` for 1; `0x5f8242` needs state 0 for 2;
any other action passes. `0x5f82c0 cmp [ebx],0x21` and `0x5f831c cmp [ebx],0x3b` read the spell's
effects; slot types 2 and 1 match the misc value and the item's entry.

## Unreached game object code

The game object functions the [census](overview.md#unreached-functions) finds unreached are a
field copy, a level map, a type name getter, the handler's slot 16 tail and its three bodies, and
the type table's lookups by name and id.

Addresses: `0x5f7f50`, `0x5f7f60`, `0x5f7f90`, `0x5f8120`, `0x5f8140`, `0x5f3680`, `0x5f3710`, `0x5f4280`, `0x6219f0`, `0x621a10`, `0x621a50`, `0x621a60`, `0x621a70`, `0x621a90`, `0x621ad0`, `0x621af0`, `0x621b40`, `0x621b50`
Confidence: verified
Proof: one
Test: none

`0x621a70`'s only site is the jump in `0x5f8120`. Slot 16's bodies, `0x5f3680`
(`""`), `0x5f3710` (`"Unknown Object Type"`) and `0x5f4280` (the name at `0x860818` of state
`+0x10`), stand only in vtables, and only `0x5f8140` calls the slot. `0x6219f0` returns the name of
a data id below 87, `0x621a10` the id of a name, `0x621a70` the name of a type below 31,
`0x621a90` the type of a name, `0x621ad0` a type's column count, `0x621af0` and `0x621b40` an entry
of its two arrays.
