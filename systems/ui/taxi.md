# ui: the taxi map

The taxi map (`TaxiMapFrame.cpp`, `0x4db2d0` to `0x4dcf3d`): the cells it keeps, the show-nodes message that opens it,
the node slots it builds, the map art and the position of a node, the node types, the Lua functions and the close.
Routes, fares and the flight request are `ui/taxiroute`; the route frame's class is `ui/taxiframe`. The message
readers are in `objectlayer/interaction#taxis`.

Roots: `0x4dcf20`, `0x4db710`, `0x4dc0f0`, `0x4db340`, `0x4db5b0`, `0x4db700`

## Taxi map state

The map keeps its state in fixed cells. `0xbb4ab8` holds the GUID of the flight master whose map is open, 0 for none, and
`0xbb4a80` the `TaxiNodes` row of the node nearest that master. `0xbb4a88` is the known-node mask, four u64 words: node id
n is bit (n-1) mod 64 of word (n-1) div 64. `0xbb4aa8` is the number of node slots and `0xbb4f10` the slot array, up to
256 slots of 0x20 bytes. `0xbb4a78` holds the map texture handle, `0xb73a28` the map rect in world units and `0xbb4a70`
the interaction range, the square of 5.5555553 yards.

| slot offset | field |
|---|---|
| +0x00 | the `TaxiNodes` row |
| +0x04, +0x08 | map u and v, floats |
| +0x0c | byte, 1 when no `TaxiPath` joins the current node to this one |
| +0x0d | byte, 1 when a route reaches the node |
| +0x10 | the route's fare sum, copper |
| +0x14 | the route, an array of node ids |
| +0x18 | the number of nodes in the route |
| +0x1c | the route's length, float |

Addresses: `0x4db2d0`, `0x4db340`, `0x4db35f`, `0x4db36d`, `0x4db630`, `0x4db953`, `0x4db961`, `0x4db978`, `0x4db995`, `0x4db9b0`, `0x4db9bc`, `0x4db9d5`, `0x4db9db`, `0x4db9e1`, `0x4db9e8`, `0x4dc360`, `0x4dd430`, `0xbb4ab8`, `0xbb4a80`, `0xbb4a88`, `0xbb4aa8`, `0xbb4f10`, `0xbb4a78`, `0xb73a28`, `0xbb4a70`
Confidence: verified
Proof: one
Test: none

A new slot starts with fare sum 999999 (`0x4db9d5`), node count 999999 (`0x4db9db`), route flag 0 (`0x4db9e1`) and length
1e8 (`0x4cbebc20`, `0x4db9e8`). The no-path byte is 0 when `TaxiPath` joins the current node to the slot's node
(`0x4db9b0`) and 1 otherwise (`0x4db9bc`). Game start zeroes the GUID and count (`0x4db35f`, `0x4db36d`); the range
cell is set once to 5.5555553 squared (`0x4dd430`).

## The show-nodes message

`SMSG_SHOWTAXINODES` (0x1a9) is a u32 flag. With the flag nonzero a raw 8-byte flight master GUID (not packed) and a u32
id of the nearest node follow. Then come 32 bytes of known-node mask, read as four 8-byte words. With the flag zero the
client prints the known nodes to the console and opens nothing. Otherwise it counts the set bits of the four words: fewer
than two show message 0xb5 (`ERR_TAXINOPATHS`) and clear the gossip target; two or more open the map for the GUID and node.
The dispatcher returns without reading when there is no active player.

Addresses: `0x5ece60`, `0x5ece92`, `0x5ecea8`, `0x5ecec2`, `0x5eceed`, `0x5ecf30`, `0x5ecf39`, `0x5ecf3f`, `0x5ecf55`, `0x5ecf6f`, `0x5ecf7f`, `0x5e3320`, `0x5e3ebb`, `0x5e3ee4`, `0x5e4a92`, `0x4e1fa0`
Confidence: verified
Proof: one
Test: none

The flag read is `0x5ece92`, the GUID read `0x5ecea8` (`GetGuid`, eight bytes, no packing), the node read `0x5ecec2` and
the mask read `0x5eceed`, four calls of `GetGuid` into a 0x20-byte stack buffer. The bit count loops 64 times per word
(`0x5ecf1f`). The mask is passed to the builder untouched (`0x5ecf6f`). The arm `0x5e3ebb` looks the active player up
(`0x5e3ee4` passes it as the receiver); a missing player takes `0x5e4a92`, which returns 1 with the body unread.

## Opening the map

The builder `0x4db710(guid, nearestNode, mask)` returns at once when the GUID equals the stored one. A different master
closes the open map first. It copies the mask into the stored
cell, sets the current row to the `TaxiNodes` row of the nearest node, empties the route line arrays and builds the slots.
When at least one slot exists and the map texture loads, it seeds the routes, makes the master the interaction target
at the range cell, stores the GUID, builds the route lines of every slot, signals `TAXIMAP_OPENED` (306) and sets
tutorial flag 0x22. With no slot or no texture the slot count is 0 and the GUID stays 0.

Addresses: `0x4db710`, `0x4db716`, `0x4db73a`, `0x4db754`, `0x4db78e`, `0x4db7a0`, `0x4dba39`, `0x4dba42`, `0x4dba4b`, `0x4dba5c`, `0x4dba63`, `0x4dba82`, `0x4dba96`, `0x4dbaa2`, `0x4dbab0`, `0xc0d5a0`, `0x4dc010`, `0x4dc5c0`, `0x4930d0`, `0x4b5390`, `0x703e50`
Confidence: verified
Proof: one
Test: `ui::glue_geom_4db710_bound_diff`, `ui::glue_geom_4db710_bounds_norm`

The nearest node is looked up with the id bound of `TaxiNodes`, and an id without a row stores null. When
`WorldMapContinent` has rows, the next step reads the row's continent (`0x4db7ec`) without a null test, so a nearest node
the table lacks faults the client. Each route line array is emptied by `0x4dd2f0` per segment (`0x4db7b7`).

## Which nodes show

A set bit of the mask becomes a slot when its `TaxiNodes` row exists, lies on the continent of the current node, and has
x and y inside the rect, ends included. The rect is the `WorldMapContinent` row whose map id (`+0x04`) equals the current
node's continent: lowest x `+0x24`, lowest y `+0x28`, highest x `+0x2c`, highest y `+0x30`. A zero span in x or y, or no
such row, stops the build. A bit whose row exists but fails the continent or rect test is cleared in the stored mask; a bit
with no row is left.

Addresses: `0x4db7d8`, `0x4db7ec`, `0x4db803`, `0x4db818`, `0x4db853`, `0x4db870`, `0x4db8aa`, `0x4db8d4`, `0x4db8e6`, `0x4db8fa`, `0x4db906`, `0x4db91a`, `0x4db92e`, `0x4db942`, `0x4db9fa`, `0xc0d5a0`, `0xc0d6c0`
Confidence: verified
Proof: one
Test: none

The ids run 1 to 256 over the four words (`0x4db8aa` shifts a 64-bit one, `0x4dba11` bounds 64 bits a word). The
accept tests are `fcomp` pairs: x at most the highest (`0x4db906`, `test ah,0x41; je`) and at least the lowest (`0x4db91a`,
`test ah,0x5; jnp`), likewise y. A failing bit is cleared at `0x4db9fa` with `not; and`.

## Map art and positions

The map texture is the file `Interface\TaxiFrame\TAXIMAP%d.blp` with the continent id of the current node, formatted
without a clamp. A held texture whose name matches, extension ignored and case ignored, is kept; otherwise it is closed and
the file loaded. Because the rect search needs a `WorldMapContinent` row, only continents that have one reach the load. A
node's map position is u = (highest y - y) / (highest x - lowest x) and v = (x - lowest x) / (highest y - lowest y), stored
in the slot. The route points use the same numerators with each axis over its own span; both agree when the spans are equal.

Addresses: `0x4dbb10`, `0x4dbb33`, `0x4dbb4b`, `0x4dbbb2`, `0x4db6d0`, `0x44ac70`, `0x449d90`, `0x4dc890`, `0x4dc3b0`, `0x4dc370`
Confidence: verified
Proof: one
Test: `ui::geo_4dc890`

`0x4dbb33` formats into a 0x104-byte buffer. `0x44ac70` compares the held texture's name at `+0xc` with the new name
through `0x449590` and `0x64a4c0`, ignoring the extension. The slot position is the stores `0x4db978` and `0x4db995`
(the divisors are `highest x - lowest x` for u, `highest y - lowest y` for v). `0x4dc890` divides u by the y span and v
by the x span. `0x4dc3b0` copies u and v out.

`ui::geo_4dc890` runs `0x4dc890` whole.

## Node types

`TaxiNodeGetType(slot)` returns one of four strings. A slot at or past the count gives `NONE`. The slot of the current
node gives `CURRENT`. A slot with no route gives `NONE`. Otherwise it tests the stored mask for the slot's node id: set
gives `REACHABLE`, clear gives `DISTANT`. The builder clears a bit only for a node it did not make a slot, and every slot's
bit is set, so `DISTANT` is not produced.

Addresses: `0x4dc4e0`, `0x4dc4e6`, `0x4dc504`, `0x4dc512`, `0x4dc523`, `0x4dc580`, `0x4dc545`, `0x4dc568`
Confidence: verified
Proof: one
Test: none

`0x4dc580` returns word (id-1) div 64 and bit (id-1) mod 64 for an id with a row. The stored mask has two writers, the copy
(`0x4db75b`, `0x4db768`) and the clear at `0x4db9fa`, which touches only a node that has no slot.

## Taxi Lua functions

`0x4dcf20` registers fourteen functions from a table of name and function pairs: `SetTaxiMap`, `NumTaxiNodes`,
`TaxiNodeName`, `TaxiNodePosition`, `TaxiNodeCost`, `TakeTaxiNode`, `CloseTaxiMap`, `TaxiNodeGetType`,
`TaxiNodeSetCurrent`, `TaxiGetSrcX`, `TaxiGetSrcY`, `TaxiGetDestX`, `TaxiGetDestY` and `GetNumRoutes`. Slots are 1-based.
`SetTaxiMap(texture)` assigns the held map texture to a texture region and does nothing before a map has loaded.
`TaxiNodeName(slot)` returns the node's localized name, `INVALID` for a slot out of range. `TaxiNodePosition(slot)`
returns u then v, and raises `Invalid taxi node slot` out of range. The six functions that take a slot raise their usage
text for a non-number.

Addresses: `0x4dcf20`, `0x84ada8`, `0x4dca50`, `0x4dca57`, `0x4dca8b`, `0x4dcaef`, `0x4dcb00`, `0x4dcb30`, `0x4dcb5f`, `0x4dc380`, `0x4dcb80`, `0x4dcbd4`, `0x4dcbef`, `0x4dc370`, `0x770300`, `0x4dcce0`, `0x4dccf0`
Confidence: verified
Proof: one
Test: none

The registrar loops the 14 entries (`0x4dcf37 cmp esi,0x70`) through `0x704120`. `SetTaxiMap` returns when the texture
cell is 0 (`0x4dca5c`), resolves its receiver as `ui/methods#finding-self` does with the texture type id, and stores the
handle through `0x770300`. The name is read at `+0x14 + locale * 4` of the row (`0x4dc397`, `0x4dc39d`). The usage texts
have the form `Usage: TaxiNodeName(slot)`; the one of `TaxiNodePosition` reads `Usage: TaxiNodeTaxiNodeLocation(slot)`,
a client fault. `CloseTaxiMap` runs `0x4dc0f0` and returns nothing.

## Closing the map

The close `0x4dc0f0` does nothing with no stored GUID. Otherwise it clears the interaction target, zeroes the GUID, frees
each slot's route array and signals `TAXIMAP_CLOSED` (307). Clearing the target (`0x4dc105`, `0x4931c0`) runs the dispatcher `0x493310` when the
master is still the target; it calls the close again while the GUID is still set, so a script close signals
`TAXIMAP_CLOSED` twice. The slot count, the mask and the texture stay. Six places call
it: the interaction clear when the stored target is the master, shutdown, the opening of another master, `CloseTaxiMap()`,
the activation reply 0, and the loss of the flight master NPC flag.

Addresses: `0x4dc0f0`, `0x4dc105`, `0x4dc140`, `0x4dc15f`, `0x493416`, `0x4db705`, `0x4db73a`, `0x4dcce0`, `0x5ed203`, `0x60b4e8`, `0x4db700`, `0x4db5b0`, `0x4911be`, `0x490ad0`, `0x4931c0`
Confidence: verified
Proof: one
Test: none

The close ends in a tail jump to the signal (`0x4dc15f jmp 0x703e50`). `0x4931c0` is the interaction clear, called with the
GUID cell and a clear flag of 0. The slot frees run per slot up to the stored count (`0x4dc130`). `0x4db700` frees the
state through `0x4db630` before the close, and `0x4db5b0` empties the path point map and ends in the same free.
