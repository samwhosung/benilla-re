# terrain: entity facets

How game objects take part in the mover's collision: the entity leg of the facet gather, the owner
id that every facet carries, and the one rule that removes some of them. The gather is
`terrain/collide`; the object that is asked is `objectlayer/gameobjects`.

Roots: none

## Entity gather

A game object's collision mesh is solid to the mover. The entity leg of the gather, `0x6abe60`, runs
when the query class has any bit of 0xf00000 (the mover's does), walks a chunk's or placement's list
of entity records and calls the callback `0xc91f54` for each whose box overlaps the query box. The
callback is `0x483340`: it finds the object by GUID, asks its vtable slot `+0x50` with the class
word, requires a game object type, and returns the model and box. `0x6abfb0` then runs the model's
collision-mesh gather `0x7137c0`, which appends a 52-byte facet per triangle to the same list as
terrain and map objects, and tags each new facet with the object's GUID in the owner array.

Addresses: `0x6abe60`, `0x6abe7b`, `0x6abe82`, `0x6abeaa`, `0x6abeb1`, `0x6abf0b`, `0x6abf79`, `0x6abfb0`, `0x6abfc8`, `0x6ac037`, `0x6ac045`, `0x7137c0`, `0xc91f54`, `0x483340`, `0x483244`, `0x483249`, `0x670da0`, `0x483364`, `0x48337b`, `0x483386`, `0x483392`
Confidence: verified
Proof: one
Test: `ui::textglue_482_gated_triple`

`0x483244 mov ecx,0x483340; call 0x670da0` is the only writer of `0xc91f54` (`0x670da0` is `mov
[0xc91f54],ecx; ret`). `0x6abe7b test [ebp+0xc],0xf00000; je` guards the leg, `0x6abeaa test [esi+0x90],4`
selects records whose owner is a game object. `0x483364 call 0x468460` uses `ecx` 1 (any object), `0x48337b
call [edx+0x50]` passes the class word, and `0x483386` to `0x483392` test the type mask bit `0x20`. The
owner fill is `0x6ac037` (count) and `0x6ac045` (the GUID dwords).

Bit-exact coverage: `ui::textglue_482_gated_triple` covers `0x4833dc` to `0x4833f1` of `0x483340`.

## Facet owner ids

Every blocking facet has an owner id in a parallel array. The facet list keeps a second array of
8-byte ids, one per facet, and the facet appenders fill it: terrain leaves zero, the doodad leg
writes the record's id, the entity leg the game object's GUID. The walk resolver reads both with one index, the
facet at `index·0x34` and the owner at `index·8`, and uses the owner to elect the transport the mover
stands on (`collision/steps`). So a landing on a moving deck is resolved from the owner of the
facet that won the sweep.

Addresses: `0x636eb1`, `0x636eb6`, `0x636eb9`, `0x636ec2`, `0x636ecd`, `0x6abd90`, `0x6abe24`, `0x6abe3c`, `0x671cc0`, `0x6720c4`, `0x6720d3`, `0x6ac037`, `0x6ac045`, `0x6721e3`, `0x6721e6`, `0xc4e530`, `0xc4e540`, `0xc4e544`, `0xc4e534`
Confidence: verified
Proof: one
Test: none

`0x6721e3 mov [ebx+4],esi` and `0x6721e6 mov [ebx+0x14],esi` zero the two counts at entry. The readers
of `0xc4e544` are in the resolvers (`0x6352dd`, `0x636996`, `0x636eb1`, `0x637045`, and the list destructor
`0x631300`); `0x636eb1 mov eax,[0xc4e544]` and `0x636ec8 mov eax,[0xc4e534]` use the same `ecx`.

## Ghost doors

A ghost does not collide with doors. The class word of the mover has `0x8000` for a ghost player
(`0x631658`), and the game object class's slot `+0x50`, `0x5f85f0`, returns 0 when that bit is set
and the object's type id is 0 (a door); `0x483340` then drops the object, so it adds no facets and
no owner. For any other combination the slot returns the object's cull flag `[+0x284]`, set while
its collision box is not degenerate. Nothing else about a ghost's collision differs. The slot is
`0x469fc0` (always 1) in the other object classes and `0x5f1eb0` (always 0) in units and players.

Addresses: `0x631658`, `0x5f85f0`, `0x5f85f6`, `0x5f85f8`, `0x5f85fa`, `0x5f8600`, `0x5f8605`, `0x5f860d`, `0x5f7aac`, `0x5f6ef0`, `0x5f6f08`, `0x483340`, `0x469fc0`, `0x5f1eb0`, `0x80c380`
Confidence: verified
Proof: one
Test: none

`0x5f85f6 test ah,ah; jns 0x5f860d` reads bit 15 through the sign flag, `0x5f8600 mov edx,[eax+0x3c]`
reads the type id (`[unit+0x110]` is the field block plus 0x18, `0x5f6f08`, so `+0x3c` is field 21), and
`0x5f860d mov eax,[ecx+0x284]` returns the cull flag, written once a frame at `0x5f7aac`. The address
`0x5f85f0` occurs once in the data, at `0x80c380`, the `+0x50` slot of the game object class's vtable.
The bit reaches the entity leg only with the mover's class (`0x6315f0`); the other word that carries
it, `0x6343b0`'s, has no 0xf00000 bit, so `0x6abe60` skips the leg.

## Entity pools

Map entities come from three pools the map initializer registers: doodad definitions (`0x198`-byte records, 5000 of them), entities (`0x108` bytes,
128) and map object definitions (`0x168` bytes, 64). Each constructor tags `[rec+8]`: `0x6a7d00` the doodad definition with `0x40`, `0x69d590` the
entity with `0x20` and `0x6a2200` the map object definition with `0x08`. The keyed insert `0x6a8ca0` files a record by those bits, `0x20` into the
chunk's list at `+0xf4` and `0x40` into the list at `+0xdc`, and a map object definition into neither. The doodad leg `0x6abc40` walks the `+0xdc` list (a placement's
`+0xa8`) and the entity leg `0x6abe60` the `+0xf4` list (a placement's `+0xb4`), so no record is visited by both.

Addresses: `0x691f45`, `0x691f84`, `0x692002`, `0x463920`, `0x6a7d00`, `0x6a7eb0`, `0x6a7eb9`, `0x69d590`, `0x69d6b1`, `0x69d6d9`, `0x6a2200`, `0x6a2368`, `0x6a236c`, `0x6a8ca0`, `0x6a8ddf`, `0x6a8e2d`, `0x6abc40`, `0x6abe60`, `0x6ab49b`, `0x6ab4c7`, `0x6aad46`, `0x6aad67`
Confidence: verified
Proof: one
Test: none

`0x691f40 mov ecx,0x198; mov edx,0x1388` and its two siblings at `0x691f7a` and `0x691ff8` pass the element size and the count with a name string (WDOODADDEF, WENTITY and WMAPOBJDEF) to `0x463920`. The tag stores are `0x6a7eb9 or eax,0x40`, `0x69d6d9 or eax,0x20` and
`0x6a2368 or [esi+8],8`, beside the vtable stores `0x6a7eb0`, `0x69d6b1` and `0x6a236c`. In `0x6a8ca0`, `0x6a8ddf test al,0x20` takes the `+0xf4` list and `0x6a8e2d test al,0x40` the
`+0xdc` list. The chunk gather passes `ebx+0xdc` and `ebx+0xf4` (`0x6ab49b`, `0x6ab4c7`), and the placement walk `esi+0xa8` and `esi+0xb4`.

## Game object entities

Every game object gets an entity when it is built. `0x613e10` runs for an object with an empty record at `+0xe0`, takes the flag value 0xb for a game object
(type mask bit `0x20`) or 2 for a dynamic object (bit `0x40`), and calls the factory `0x670db0` with the model and the GUID. The factory takes an entity from
`0x6a0930`, stores the GUID at `+0xc0` and `+0xc4` and the model at `+0x88`, and sets bit `0x4` of `+0x90` for flag bit 0 and bit `0x2000` for flag bit 3. So
`+0x90 & 4` is set for a game object only, which gates the entity leg.

Addresses: `0x613e10`, `0x613e6c`, `0x613e8e`, `0x613e98`, `0x613eb1`, `0x613ed2`, `0x613edb`, `0x670db0`, `0x670dcc`, `0x670ddc`, `0x670de5`, `0x670e18`, `0x670e6b`, `0x670e8c`, `0x670e9c`, `0x6a0930`, `0x6a86d0`, `0x6a8714`, `0x6a8c10`, `0x6a8721`, `0x6a8a20`, `0x6abeaa`
Confidence: verified
Proof: one
Test: none

`0x613e98 test dl,1` on the type mask shifted by 5 loads `edi = 0xb`, and `0x613ea7 test cl,1` after a shift by 6 loads `edi = 2`. The record goes to `[esi+0xe0]` (`0x613edb`); when it is already set,
`0x613e6c` logs an error with the GUID instead. The flag arithmetic is `0x670e6b` (`and eax,1`, `shl 2`), `0x670e8c and ecx,0xfffffbfb` (which clears `0x400` and `0x4`), `0x670e97 shl edx,9` (`0x400` unless
flag bit 1) and `0x670e9c shl ebx,0xd`. `0x2000` sends the record to the per-chunk insert. `0x6a8714 test ah,0x20` splits `0x6a86d0` between `0x6a8c10` (which reaches `0x6a8ca0`) and `0x6a8a20`. `0x6abeaa test [esi+0x90],4` is the leg's gate.

## Game object doodads

A game object's doodad definition is registered only for the transport types. `0x5f4970` calls `0x5f8030` with the object's GUID for types 11 and 15 and a null GUID
for the rest, and `0x5f8030` gives `0x6707c0` a zero register flag (`0x5f8058 push 0`), so `0x6707c0` builds the definition and stops. A model that is not a `.wmo`
becomes a doodad definition (`0x694980`) with the GUID at `+0x180` and `+0x184` and `0x2000` in `+0x90`; a `.wmo` becomes a map object definition (`0x6953e0`) with the
GUID at `+0x158`. The registration `0x698e20` has two callers: `0x6708f1`, skipped by the zero flag, and `0x6709a0`, whose four callers are the two transport ticks and a
spell model placement (`objectlayer/transports`).

Addresses: `0x5f4970`, `0x5f497f`, `0x5f4984`, `0x5f4989`, `0x5f8030`, `0x5f8058`, `0x5f8076`, `0x6707c0`, `0x6707c9`, `0x6707d3`, `0x6707ec`, `0x670868`, `0x670872`, `0x6708ad`, `0x6708cf`, `0x6708e7`, `0x6708f1`, `0x694980`, `0x6953e0`, `0x69545c`, `0x6aab3c`, `0xca7d98`, `0x698e20`, `0x698d20`, `0x6709a0`, `0x6709db`, `0x6a86d0`
Confidence: verified
Proof: one
Test: none

The callers by scan: `0x6707c0` has three (`0x5f8076`, `0x6e518f` and `0x6e5a6e`, the last two pushing zeros), `0x694980` one (`0x6708ad`), `0x698e20` two, `0x6709a0` four, and `0x6a86d0` five (`0x698e11`
in `0x698d20`, which has no caller, `0x698f2e` in `0x698e20`, the two light creators `0x671334` and `0x6716a2`, and `0x69e2af`). `0x6707d3 call 0x64b4a0` compares the model name with the string `.wmo`. A `.wmo` game object of type 11 or 15 is also an ordinary
map object placement: `0x6953e0` links the definition into `0xca7d98` (`0x69545c`), the list `0x6aaab0` walks (`0x6aab3c`).

## World query class words

Each caller of the world query `0x6721b0` pushes its own class word. The movement code pushes the mask of `0x6315f0` for the obstruction test (`0x63172f`), the facet gather
(`0x6320b1`) and the swept box (`0x633b46`), and the liquid-only word 0x30000 for its liquid queries (`0x6320e0`, `0x633e90`). The carrier sweep (`0x63451f`) pushes the word
of `0x6343b0`, 0x111 or 0x2111 with 0x8000 for a ghost. Outside movement, the walk to attack (`0x60fe25`) and a corpse's slot 8 (`0x5d6ab0`) push 0x100111, and the footprint decal
(`0x699ee1`) pushes 0x200122. The camera trace builds 0x100171 (`0x50e61a`) and the pick 0x1000114 or 0x10f0114 (`0x480e19`, `0x480e20`).

Addresses: `0x6721b0`, `0x6315f0`, `0x63172f`, `0x6316d4`, `0x6320a5`, `0x6320b1`, `0x6320d0`, `0x6320e0`, `0x633b3b`, `0x633b46`, `0x633e81`, `0x633e90`, `0x6343b0`, `0x63450c`, `0x63451f`, `0x60fe18`, `0x60fe25`, `0x5d6a9d`, `0x5d6ab0`, `0x699ed1`, `0x699ee1`, `0x50e61a`, `0x480e19`, `0x480e1e`, `0x480e20`, `0x69ce90`, `0x69cf2e`, `0xc91f54`
Confidence: verified
Proof: one
Test: none

The pushes are `0x5d6a9d push 0x100111`, `0x60fe18 push 0x100111`, `0x6320d0 push 0x30000`, `0x633e81 push 0x30000` and `0x699ed1 push 0x200122`; the mask users push the result of `0x6315f0`
(`0x6316d4 push edi`, `0x6320a5 push eax`, `0x633b3b push eax`), and `0x634514 push esi` the carrier word, which `0x63450c or esi,0x8000` extends. The two pick words differ by the branch at
`0x480e1e`. Bit 0x8000 is in no other word, and 0x111, 0x2111 and 0x30000 have no 0xf00000 bit, so the entity leg skips them. The second reader of the entity callback `0xc91f54` is
the segment trace's game object pass `0x69ce90` (`0x69cf2e`).

## Mover and other world queries

The mover's world is terrain, map-object groups, liquid layers (while water walking), impassable
walls, and game object meshes through the entity leg; it has no M2 doodad mesh other than owner-tagged
records, and no ray test. The camera pull-in and line of sight use the segment query, which adds the
M2 scene ray (`0x7089c0`), and the mouse pick uses only that ray. A game object being clickable or
blocking the camera is therefore not evidence of what blocks the body.

Addresses: `0x6721b0`, `0x6aa8b0`, `0x6aaab0`, `0x6aadc0`, `0x7089c0`, `0x480c2e`, `0x69c2ca`, `0x69cd6a`, `0x672170`, `0x69bff0`, `0x50e570`, `0x63162e`
Confidence: verified
Proof: one
Test: none

`0x6721b0` has nine call sites (`0x5d6ab0`, `0x60fe25`, `0x63172f`, `0x6320b1`, `0x6320e0`, `0x633b46`,
`0x633e90`, `0x63451f`, `0x699ee1`), and `0x7089c0` three, none in the movement files: the pick
(`0x480c2e`) and the segment helpers (`0x69c2ca`, `0x69cd6a`).

## Seated units

The mover never reads the stand state, so a unit the server has sitting is simulated like a standing
one: gravity, ground resolve and depenetration run, and the fall gate may start a fall. Nothing in the
movement files tests it. The stand state is the vtable slot `+0xa4`: `0x60be50` for units (the byte
`[[unit+0x110]+0x210]`) and `0x5ed570` for players (a local cache `+0x1d68` for the active player,
else the same byte). The only bit that stops the fall resolve is root.

Addresses: `0x60be50`, `0x5ed570`, `0x5ed57d`, `0x5ed59a`, `0x80c59c`, `0x80b01c`, `0x7c61c0`, `0x7c61f0`, `0x7c7340`
Confidence: verified
Proof: one
Test: none

A scan of the listing finds 19 byte accesses of `[reg+0x210]` and 44 `call [reg+0xa4]` sites; none lies
in the movement object files (`0x615730` to `0x61b110`, `0x6307a0` to `0x637a60`, the shared movement
file), and the ones in `0x6106b0`, `0x60e0a0` and `0x611130` are unit code outside the mover. The control is
that the same scan finds the tests that read the stand state elsewhere. `0x5ed570` is stored in the player
vtable at `0x80b01c` and `0x60be50` at `0x80c59c`.

## Gather class bits

The class word of a world query selects which gather legs run. In `0x6aaab0`, any of 0xf0 in the low
byte selects the map object group facets, any of 0xf0000 the map-object liquid, any of 0xf0000f the
doodad leg `0x6abc40` and any of 0xf00000 the entity leg `0x6abe60`. In the chunk gather `0x6aadc0`, any
of 0xf00 selects the terrain grid, bit 16 + i the liquid layer i, and the same two masks the doodads
and entities. Bit 0x4000 picks the filtered leg of the world query (`0x6721f0`). The mover's word,
0x100111, runs groups, terrain, doodads and entities but not liquid; water walking adds 0x30000 and a
ghost 0x8000, which only the entity leg reads.

Addresses: `0x6aac5c`, `0x6aacba`, `0x6aad37`, `0x6aad4c`, `0x6aad58`, `0x6aad6d`, `0x6aaedf`, `0x6ab3f7`, `0x6ab418`, `0x6ab48f`, `0x6ab4bb`, `0x6abc4c`, `0x6abcc7`, `0x6abccf`, `0x6721f0`, `0x6315f0`
Confidence: verified
Proof: one
Test: none

The tests are quoted above as `test al,0xf0` (`0x6aac5c`), `test eax,0xf0000` (`0x6aacba`), `test
eax,0xf0000f` (`0x6aad37`), `test eax,0xf00000` (`0x6aad58`), `test ah,0xf` (`0x6aaedf`) and, in the chunk
gather, the same two doodad and entity tests at `0x6ab48f` and `0x6ab4bb`. A doodad record with an owner id
is taken by the 0xf00000 group and one without by the low nibble (`0x6abcc7`, `0x6abccf`). The bits
`0x1000` and `0x2000` have no reader in the closure of the word's functions (`collision/contact#collision-class-mask`
builds it).
