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
Test: none

`0x483244 mov ecx,0x483340; call 0x670da0` is the only writer of `0xc91f54` (`0x670da0` is `mov
[0xc91f54],ecx; ret`). `0x6abe7b test [ebp+0xc],0xf00000; je` guards the leg, `0x6abeaa test [esi+0x90],4`
selects records whose owner is a game object. `0x483364 call 0x468460` uses `ecx` 1 (any object), `0x48337b
call [edx+0x50]` passes the class word, and `0x483386` to `0x483392` test the type mask bit `0x20`. The
owner fill is `0x6ac037` (count) and `0x6ac045` (the GUID dwords).

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

## Entity records and doodads

A game object is gathered as an entity record, and its doodad record is registered only for the two
transport types. The doodad leg `0x6abc40` and the entity leg walk two different lists of a chunk or
a placement (offsets `+0xdc` and `+0xf4`), and no record is in both. Every game object gets an entity
record when it is built (`0x613e10`, for type mask bit `0x20`). Its doodad record is created too, but
only transports (types 11 and 15) are registered, by their per-frame ticks, so a door is seen only
through the entity leg. A game object whose model is a `.wmo` also joins the global map-object list
and collides as a placement.

Addresses: `0x6abc40`, `0x6abd90`, `0x6abe60`, `0x613e10`, `0x613e98`, `0x670db0`, `0x670dcc`, `0x6a8ca0`, `0x6a8ddf`, `0x6a8e2d`, `0x5f4970`, `0x5f497f`, `0x5f4984`, `0x5f8030`, `0x6707c0`, `0x6707d3`, `0x6953e0`, `0x698e20`, `0x6709a0`, `0x5f52b8`, `0x5f603c`, `0x5f619e`, `0x4821d5`
Confidence: inferred
Proof: none
Test: none

The tags are read from the record constructors and the keyed inserts: the entity constructor stores class
tag `0x20` and the doodad constructor `0x40`, and `0x6a8ca0` files a record by those tags. `0x698e20`, which
registers a doodad record, has two callers, one skipped for a game object (`0x5f8058 push 0`) and the
other inside `0x6709a0`, whose callers are the transport ticks `0x5f52b8`, `0x5f603c`, `0x5f619e` and a spell model
placement `0x4821d5`. The ownership claims rest on those readings, not on a walk of every pool user.

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
