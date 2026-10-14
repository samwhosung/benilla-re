# objectlayer: classes

The object class hierarchy: eight object types share one base, `CGObject_C`, whose members fill
`+0x0` to `+0x110`; each subclass adds its own members and then the field array. The object
manager allocates every type but the active player from a per-type pool, constructs it through
the type's constructor, and installs the type's vtable last. The base vtable has 31 slots; units
add 15 and players one more, game objects add three. Items and containers also carry a
one-slot secondary vtable at `+0x110`.

The code lives in `Object_C.cpp` (`0x613880` to `0x6151e0`) and at the tail of the
`ObjectMgrClient.cpp` run, which holds the base vtable's trivial default slots (`0x469ed0` to
`0x46a090`). The image names the pools by class: `CGObject_C` to `CGCorpse_C`, including
`CGContainer_C` and `CGDynamicObject_C`.

Roots: the base vtable slots `0x46a090`, `0x6145e0`, `0x614640`, `0x614e80`, `0x469ed0`, `0x469ee0`, `0x469f00`, `0x469f10`, `0x469f20`, `0x469f40`, `0x469f50`, `0x469f60`, `0x469f80`, `0x6151b0`, `0x614a90`, `0x614cd0`, `0x6146c0`, `0x469f90`, `0x469fa0`, `0x469fb0`, `0x469fc0`, `0x469fd0`, `0x469fe0`, `0x469ff0`, `0x46a000`, `0x46a010`, `0x46a050`, `0x46a060`, `0x46a070`, `0x46a080`, `0x614fe0`, and the unit slots `0x613650`, `0x613660`, `0x613670`, `0x613680`, `0x6136b0`

## Class sizes and pools

Each type's allocation size is its members, its field array and a second per-field array, in
bytes `base + 4·N`; N comes from a small function per type. At startup the manager creates one
pool per type 1 to 7 with that size, the class name and a block count. An object comes from its
type's pool, which stores the pool handle at `+0x10`; the active player instead gets a heap block
of 26648 bytes.

| type | class | base | N | size | block |
|---|---|---|---|---|---|
| 0 | `CGObject_C` | 0x128 | 3 | 308 (no pool) | |
| 1 | `CGItem_C` | 0x408 | 35 | 1172 | 512 |
| 2 | `CGContainer_C` | 0x8c8 | 107 | 2676 | 32 |
| 3 | `CGUnit_C` | 0x1158 | 170 | 5120 | 64 |
| 4 | `CGPlayer_C` | 0x2508 | 465 | 11340 | 64 |
| 5 | `CGGameObject_C` | 0x2f0 | 8 | 784 | 64 |
| 6 | `CGDynamicObject_C` | 0x1e0 | 3 | 492 | 32 |
| 7 | `CGCorpse_C` | 0x350 | 6 | 872 | 32 |

Addresses: `0x4641c0`, `0xb4141c`, `0x467d40`, `0x467d5c`, `0x467d68`, `0x836708`, `0x836728`, `0xb232e8`, `0x463920`, `0x466e00`, `0x466e3f`, `0x466f80`, `0x466fa4`, `0x463a60`, `0x463b30`
Confidence: verified
Proof: one
Test: none

`0x4641c0` stores `lea [eax*4+base]` of eight calls into `0xb4141c` to `0xb41438`; the calls
return 3, 3+0x20, 3+0x20+0x48, 3+0xa7, 3+0xa7+0x127, 3+5, 3 and 3+3. Each base is the class's
field array offset plus 4 per field (item `+0x348` + 48·4 = 0x408). `0x467d40` loops `esi` 4 to
0x1c: `0x463920(ecx = [0xb4141c+esi], edx = [0x836728+esi], name = [0x836708+esi])`, storing the
pool at `[0xb232e4+esi]` (`0x467d68`), so `0xb4141c` (type 0) is never read. `0x463920` stores
the size at record `+0x10` and the count at `+0x14`. For the active player's GUID, `0x466e00`
calls `SMemAlloc(0x2550 + 4·1108 + 0x3178)` (`0x466e3f`); otherwise `0x463a60` on
`[0xb232e4+4·type]` (`0x466f80`), `0x463b30` maps the handle to memory and `0x466fa4` stores the
handle at `+0x10`.

## Constructors and final vtables

Every subclass constructor runs its parent's first, then its own members, then stores its vtable
over the parent's. The container and player constructors start from the item and unit
constructors. The vtable each stores last, with its slot count:

| type | constructor | vtable | slots |
|---|---|---|---|
| object | `0x6139a0` | `0x803640` | 31 |
| item | `0x5d81f0` | `0x80ada8`, `+0x110`: `0x80ada0` | 31, 1 |
| container | `0x5d7730` | `0x80ac88`, `+0x110`: `0x80ac84` | 31, 1 |
| unit | `0x5fad50` | `0x80c4f8` | 46 |
| player | `0x5dd2e0` | `0x80af78` | 47 |
| game object | `0x5f6f30` | `0x80c330` | 34 |
| dynamic object | `0x5d5140` | `0x80aa80` | 31 |
| corpse | `0x5d6050` | `0x80ab80` | 31 |

Addresses: `0x6139a0`, `0x613ab1`, `0x5d81f0`, `0x5d8250`, `0x5d8256`, `0x5d7730`, `0x5d77ad`, `0x5d77b3`, `0x5fad50`, `0x5fb12b`, `0x5dd2e0`, `0x5dd473`, `0x5f6f30`, `0x5f7031`, `0x5d5140`, `0x5d5196`, `0x5d6050`, `0x5d60ce`, `0x803640`, `0x80ada8`, `0x80ada0`, `0x80ac88`, `0x80ac84`, `0x80c4f8`, `0x80af78`, `0x80c330`, `0x80aa80`, `0x80ab80`
Confidence: verified
Proof: one
Test: none

The stores are `mov [esi],imm` at the addresses listed. A vtable ends where its next dword is not
a code address or begins another stored vtable: the base, dynamic object and corpse tables are
followed by 0, +inf and 1/1.5; the unit's 47th dword is a float; the game object's 35th is +inf;
the item's 32nd, `0x80ae24`, is the one-slot vtable the item constructor stores at `+0x110`
first (`__purecall`); the player's 48th, `0x80b034`, is a vtable the player constructor
stores at `+0x1c9c`. The container's table begins right after its `+0x110` table. Live code
reaches the player's last slot (`call [eax+0xb8]` in `0x5ec430`). The image holds no
reference to an address inside these tables other than their starts.

## Base slots 0 to 10

The base vtable's first eleven slots, what the base body does, and the role its callers give it.

| slot | base | base body | role |
|---|---|---|---|
| 0 `+0x00` | `0x46a090` | destructor, frees on bit 0 | deleting destructor |
| 1 `+0x04` | `0x6145e0` | hands the world node to a fade-out | deactivate |
| 2 `+0x08` | `0x614640` | resets scale and fade, rebuilds the node | reactivate |
| 3 `+0x0c` | `0x614e80` | clears `+0xe4` bit 1 | end of reactivation |
| 4 `+0x10` | `0x469ed0` | returns 0 | player and container return a member block |
| 5 `+0x14` | `0x469ee0` | writes (0, 0, 0), returns the pointer | position |
| 6 `+0x18` | `0x469f00` | returns 0.0 | facing |
| 7 `+0x1c` | `0x469f10` | returns field 4 as a float | scale field |
| 8 `+0x20` | `0x469f20` | writes (0, 0, 1) | ground normal |
| 9 `+0x24` | `0x469f40` | returns 0 in `edx:eax` | a GUID; units read `+0x9e0` |
| 10 `+0x28` | `0x469f50` | returns 0 | model path |

Addresses: `0x46a090`, `0x6145e0`, `0x614640`, `0x614e80`, `0x469ed0`, `0x469ee0`, `0x469f00`, `0x469f10`, `0x469f20`, `0x469f40`, `0x469f50`, `0x613e2a`, `0x613e3d`, `0x613f82`, `0x613fb0`, `0x614ce2`
Confidence: verified
Proof: one
Test: `objectlayer::world_model_scale_difftest`

The bodies are as listed (`0x469f10` is `mov eax,[ecx+8]; fld [eax+0x10]`). The roles of 5, 6
and 8 come from `0x613ef0` and `0x614cd0`: slot 5 fills the translation (`0x613f82`), slot 6
gives the rotation angle (`0x613fb0`), slot 8 gives the normal tested against 0.3572124
(`0x614ce2`). Slot 10's result is the path passed to the model cache, skipped when 0 (`0x613e2a`,
`0x613e3d`). Slot 4's role is the overrides' alone: `lea eax,[ecx+0x1d38]` (player) and
`[ecx+0x6c8]` (container); slot 9's unit override returns `[+0x9e0]:[+0x9e4]`.

## Base slots 11 to 20

The middle slots. Several defaults are constants whose role only their overrides show.

| slot | base | base body | role |
|---|---|---|---|
| 11 `+0x2c` | `0x469f60` | writes -1, returns 1 | items keep it, units override |
| 12 `+0x30` | `0x469f80` | nothing | draw the selection circle |
| 13 `+0x34` | `0x6151b0` | refreshes bounds for the current model | model loaded |
| 14 `+0x38` | `0x614a90` | per-frame fade, scale ease, missile fade | per-frame render update |
| 15 `+0x3c` | `0x614cd0` | smooths the normal, places the model | ground conform |
| 16 `+0x40` | `0x6146c0` | returns the argument's bit 0 | units and corpses filter it |
| 17 `+0x44` | `0x469f90` | tail-jumps to slot 6 | render facing |
| 18 `+0x48` | `0x469fa0` | nothing | units override |
| 19 `+0x4c` | `0x469fb0` | returns 1 | units, game objects override |
| 20 `+0x50` | `0x469fc0` | returns 1, one argument | units, game objects override |

Addresses: `0x469f60`, `0x469f80`, `0x6151b0`, `0x614a90`, `0x614cd0`, `0x6146c0`, `0x469f90`, `0x469fa0`, `0x469fb0`, `0x469fc0`, `0x614ada`, `0x613d79`, `0x614dd8`
Confidence: verified
Proof: one
Test: none

Slot 12 is called only for the object the selection GUID names, with the selection-circle CVar
on (`0x614ada`), and the unit and corpse overrides (`0x608e00`, `0x5d6fe0`) call the circle's
render setup `0x614e00`. Slot 13 is called from the model-load callback with the model
(`0x613d79`). Slot 17 supplies the facing that the ground conform passes to the model placement
(`0x614dd8`). `0x6146c0` is `mov eax,[ebp+8]; and eax,1; ret 4`.

## Base slots 21 to 30

The last base slots. Slot 22 is the selectability test: only units and players override it.

| slot | base | base body | role |
|---|---|---|---|
| 21 `+0x54` | `0x469fd0` | returns 0 | units, game objects, corpses override |
| 22 `+0x58` | `0x469fe0` | returns 0 | selectable |
| 23 `+0x5c` | `0x469ff0` | returns 0 | game objects override |
| 24 `+0x60` | `0x46a000` | nothing, one argument | items, units, game objects, corpses |
| 25 `+0x64` | `0x46a010` | writes a 4×4 identity | game objects copy `+0x218` |
| 26 `+0x68` | `0x46a050` | returns 1 | fade in on appear |
| 27 `+0x6c` | `0x46a060` | returns 1.0 | appear alpha |
| 28 `+0x70` | `0x46a070` | returns 0 | items, units, game objects override |
| 29 `+0x74` | `0x46a080` | returns 0, one argument | items, game objects override |
| 30 `+0x78` | `0x614fe0` | tests the current model | model complete |

Addresses: `0x469fd0`, `0x469fe0`, `0x469ff0`, `0x46a000`, `0x46a010`, `0x46a050`, `0x46a060`, `0x46a070`, `0x46a080`, `0x614fe0`, `0x613b04`, `0x613b15`
Confidence: verified
Proof: one
Test: none

Slots 26 and 27 feed `0x613af0`: `call [eax+0x68]`, `neg; sbb; and eax,0x7d0` turns a nonzero
answer into 2000 ms, and `call [edx+0x6c]` gives the target alpha (`0x613b04`, `0x613b15`). The
units' slot 26 (`0x5f1ee0`) returns 0 when field 46 bit 1 or byte `+0x213` bit 1 of the unit's
fields is set, and their slot 27 (`0x60d2d0`) returns a byte over 255 from `[+0xb34]+0x14`, else
1.0. Slot 22 is called by the selection setter through `call [eax+0x58]`. The game object's slot
25 is `rep movs` of 16 dwords from `+0x218`. The roles of 21, 23, 24, 28 and 29 are not settled.

## Overrides by class

The slots each class replaces relative to the base vtable, and the slots it adds. The container
also keeps the item's bodies for 1, 3, 10, 11, 24, 28 and 29; the player keeps the unit's bodies
for every slot it does not list.

| class | overridden base slots | added slots |
|---|---|---|
| `CGItem_C` | 0, 1, 3, 10, 11, 24, 28, 29 | none |
| container | 0, 1, 2, 3, 4, 10, 11, 24, 28, 29 | none |
| `CGUnit_C` | all but 4, 7, 23, 25, 29, 30 | 31 to 45 |
| `CGPlayer_C` | as the unit, and 0 to 4, 14, 16 its own | 31 to 45 its own, 46 |
| `CGGameObject_C` | 0, 1, 2, 3, 5, 6, 10, 13, 15, 19 to 21, 23 to 26, 28, 29 | 31 to 33 |
| dynamic object | 0, 1, 2, 5, 6, 7, 10, 13 | none |
| `CGCorpse_C` | 0, 1, 2, 5, 6, 8, 10, 12, 13, 16, 21, 24 | none |

Addresses: `0x803640`, `0x80ada8`, `0x80ac88`, `0x80c4f8`, `0x80af78`, `0x80c330`, `0x80aa80`, `0x80ab80`
Confidence: verified
Proof: one
Test: none

Read slot by slot against `0x803640`. The unit keeps `0x469ed0`, `0x469f10`, `0x469ff0`,
`0x46a010`, `0x46a080` and `0x614fe0`. The player differs from the unit at slots 0 to 4
(`0x5dd500`, `0x5e9aa0`, `0x5e9bc0`, `0x5dec90`, `0x5f1f70`), 14 (`0x5ec950`), 16 (`0x5ec8f0`)
and every added slot. Several overrides call the base body first, as the unit's slot 14 does at
`0x60800c`. The dynamic object is the only class that replaces slot 7, with `fld [ecx+0x194]`.

## Secondary and member vtables

The item and the container hold a second vtable pointer at `+0x110`, a base subobject with one
virtual whose body subtracts 0x110 from `this` and calls the item's own method. The item
constructor stores a pure-call table there first and the final one after its own members. The
player constructor stores two member vtables, `0x80b044` at `+0x1c74` and `0x80b034` at
`+0x1c9c`, four slots each.

Addresses: `0x5d820d`, `0x5d8256`, `0x5d77b3`, `0x80ae24`, `0x80ada0`, `0x80ac84`, `0x5dd353`, `0x5dd3a2`, `0x80b044`, `0x80b034`
Confidence: verified
Proof: one
Test: none

`0x5d820d` stores `0x80ae24`, whose one slot is `__purecall` (`0x40a2d7`); `0x5d8256` stores
`0x80ada0` and `0x5d77b3` stores `0x80ac84`, both of whose one slot is `0x5da2a0`:
`add ecx,0xfffffef0; call 0x5da260`. `0x5dd353` and `0x5dd3a2` store the player's two member
tables; `0x80b034` is followed directly by `0x80b044`.

## Type id and mask

`0x613880` stores the type id at `+0x14`, then, for ids 0 to 7, the type mask into field 2: 1, 3,
7, 9, 0x19, 0x21, 0x41, 0x81. An id above 7 stores only `+0x14`.

Addresses: `0x613880`, `0x613889`, `0x61388e`
Confidence: verified
Proof: one
Test: none

`0x613889` is `mov [ecx+0x14],eax` before the bound `cmp eax,7; ja`; `0x61388e` jumps through
the eight-entry table at `0x613908`, each case `mov [[ecx+8]+8],imm; ret 4`. Its one caller is the
manager's `0x466320`.

## Field storage

`0x613980` stores the field array pointer at `+0x8` and the second per-field array at `+0xc`.
`0x6142e0` writes one field by index and returns 1. Neither checks the index. The unit, game
object, dynamic object and corpse storage functions then point `+0x110` at field 6, their own
section, and `+0x114` at their section's watch copy, past the object's 3; the player's runs the
unit's and points `+0xe68` at field 188 and `+0xe6c` past the unit's 170 watched fields. Code of
these classes reads its own fields through these pointers: `[[unit+0x110]+4·k]` is field 6 + k.

Addresses: `0x613980`, `0x6142e0`, `0x5fad10`, `0x5fad28`, `0x5fad37`, `0x5f6ef0`, `0x5d5100`, `0x5d6220`, `0x5dd2a0`, `0x5dd2bb`, `0x5dd2ca`
Confidence: verified
Proof: one
Test: none

`0x613980` is `mov [ecx+8],arg1; mov [ecx+0xc],arg2; ret 8`, called by `0x466c70` and the
per-class storage functions. `0x6142e0` is `mov eax,[ecx+8]; mov [eax+arg1*4],arg2; mov eax,1;
ret 8`. `0x5fad10(fields, copy)` calls `0x613980`, adds 0x18 to the field
pointer and stores it at `+0x110` (`0x5fad28`), then stores `copy + 4·0x47f2b0()` at `+0x114`
(`0x5fad37`); `0x5f6ef0`, `0x5d5100` and `0x5d6220` are the same bytes. `0x5dd2a0` calls
`0x5fad10`, stores `fields + 0x2f0` at `+0xe68` (`0x5dd2bb`) and `copy + 4·0x47db10()` at `+0xe6c`
(`0x5dd2ca`). The callers are the manager's `0x466c70` and the player's storage.

## Unit vtable extension

The unit's slots 31 to 33 are empty bodies taking two arguments. Slots 43 and 44 answer a
level-based pair: the first output is field 34 times 5, the second 0; slot 44 takes and ignores
one more argument first. The player replaces all five.

Addresses: `0x613650`, `0x613660`, `0x613670`, `0x613680`, `0x61368f`, `0x6136b0`
Confidence: verified
Proof: one
Test: none

`0x613650` to `0x613670` are `ret 8`, stored only at `0x80c574` to `0x80c57c`. `0x613680` loads
`[[ecx+0x110]+0x70]`, field 6 + 28 = 34, computes `lea ecx,[eax+eax*4]` (`0x61368f`), stores it
through the first argument and 0 through the second, `ret 8`; `0x6136b0` does the same through
its second and third arguments, `ret 0xc`. Field 34 is the level by the server's enumeration.
