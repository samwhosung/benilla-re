# ui: taxi routes, fares and the flight

How the taxi map picks a route to a node, what it costs, the requests the client sends to start a flight, the replies,
the route lines and the curve the flight follows. The map, its slots and the Lua functions are `ui/taxi`; the readers of
the messages are in `objectlayer/interaction#taxis`.

Roots: `0x4db340`, `0x4dc010`, `0x4dc3d0`, `0x4dc8d0`, `0x4dc5c0`, `0x4dc9d0`, `0x4dc170`

## Path tables

At game start `0x4db340` fills two tables. The path table holds, for a pair of node ids from 1 to 256, the `TaxiPath` row
that runs from the first node to the second. The rows are walked from the last to the first, so when two rows share a pair
the first in the file stays. The point runs come from `TaxiPathNode`: rows are 0x24 bytes, the path id at `+4` and the
point x, y and z at `+0x10`, `+0x14` and `+0x18`. Consecutive rows with one path id form a run, kept in a map by path id
as the first row and a count. `TaxiPathExists(from, to)` answers whether the table holds the pair.

Addresses: `0x4db340`, `0x4db36b`, `0x4db387`, `0x4db3e2`, `0x4db43b`, `0x4db580`, `0x4db58c`, `0x4dbad0`, `0x4dbad4`, `0x4dbae8`, `0x4dbaed`, `0x48f5c2`, `0x48f480`, `0x4db5b0`, `0x4911be`, `0xb73a60`, `0xc0d690`, `0xc0d6a4`, `0xc0d6c0`
Confidence: verified
Proof: one
Test: none

The loop `0x4db387` to `0x4db3e9` counts the row index down to 0, and each pass overwrites the pair's cell
(`0x4db3e2`), which leaves the first row. The cell address is the table origin 0x404 bytes below the array plus four times
`from * 256 + to`, so ids start at 1 and the bound is 256 (`0x4dbad4`, `0x4dbae8`). A run grows at `0x4db58c` when the
path id repeats and starts at `0x4db580`. `0x4db340` is called by the game UI setup `0x48f480` (`0x48f5c2`); `0x4db5b0`
tears the map down at shutdown (`0x4911be`).

## Route search

The seed `0x4dc010` gives the current node's slot (slot 0 when none matches) a fare of 0, one node, the route
`[current id]`, a length of 0 and the route flag, then relaxes every slot the current node has a direct path to. The relaxation `0x4dbce0(to, from, price)` adds the length of the path between the two slots to the
length at `from`. A slot keeps its first route; a later one replaces it only when strictly shorter. A replacement
copies `from`'s route plus the new id, adds `price` to the fare sum, takes `from`'s node count plus 1, sets the route flag
and relaxes every node `to` has a direct path to. Fares and hops do not count.

Addresses: `0x4dc010`, `0x4dc039`, `0x4dc06f`, `0x4dc082`, `0x4dc08a`, `0x4dc090`, `0x4dc0b1`, `0x4dc0c6`, `0x4dc0cf`, `0x4dbce0`, `0x4dbe01`, `0x4dbe06`, `0x4dbe20`, `0x4dbe2b`, `0x4dbe58`, `0x4dbe9e`, `0x4dbf24`, `0x4dbf3d`, `0x4dbbd0`, `0x4dbbf2`, `0x4dbc02`, `0x4dbc5d`, `0x4dbc68`, `0x4dbcbb`
Confidence: verified
Proof: one
Test: `ui::geo_4dbbd0`, `ui::glue_geom_4dbce0_cost_add`

The relaxation is depth first and ends because a replacement needs a strictly smaller length (`0x4dbe2b jp` rejects an
equal or unordered one). The length call `0x4dbe01` passes `to` first and `from` second, so the table read at `0x4dbbf2`
is the path from the new node back to the old one, while the price comes from the path in the other direction
(`0x4dc0c6`). The read has no null test: `0x4dbc02` loads the row's id. A pair with only the forward row faults the client.
The length sums the straight-line distance between consecutive points of the run (`0x4dbcbb fsqrt`); a path with no run
gives the constant 99999997952 (`0x4dbc5d`).

`ui::geo_4dbbd0` runs `0x4dbbd0` whole, with the table lookups and the hash probe seeded in `.bss` so that it reaches chosen points.

## Faction filter

Before a relaxation the destination's `TaxiNodes` row is tested against the player's group. The group is the mask at `+0xc`
of the faction template row whose id is the player's race. Two ids at `+0x38` and `+0x3c` of the node row take part. A
player whose group is exactly 3 skips a node that names 0x8b0 or 0xdf6 and names neither 0xefd nor 0x21d; a player whose
group is exactly 5 skips a node that names 0xefd or 0x21d and names neither 0x8b0 nor 0xdf6. Any other group, a player
that cannot be found and a race without a row skip nothing. A skipped node gets no route, so its type is `NONE`.

Addresses: `0x4dbd0e`, `0x4dbd27`, `0x4dbd2d`, `0x4dbd39`, `0x4dbd45`, `0x4dbd56`, `0x4dbd59`, `0x4dbd5e`, `0x4dbd61`, `0x4dbd68`, `0x4dbd6f`, `0x4dbd72`, `0x4dbd78`, `0x4dbd82`, `0x4dbd89`, `0x4dbd90`, `0x4dbd93`, `0x4dbd99`, `0x4dbda1`, `0x4dbda3`, `0x4dbdab`, `0x4dbdb2`, `0x4dbdb9`, `0x4dbdbc`, `0x4dbdc2`, `0x4dbdca`, `0x4dbdcc`, `0x4dbdd1`, `0x4dbdd3`, `0x4dbdda`, `0x4dbddd`, `0x4dbde3`, `0x4dbde5`, `0x4dbdeb`, `0x4dbdf1`
Confidence: verified
Proof: one
Test: none

The race is the low byte of the unit bytes word (`0x4dbd2d movzx [..+0x78]`) and the group word is read at `0x4dbd56`;
the tests are exact compares with 3 (`0x4dbd59`) and 5 (`0x4dbda3`). A pass leaves at `0x4dbdf1`, a skip at `0x4dbffc`. The filter acts only on the node being reached, so a node behind a skipped one is reached only by another
road.

## Fares

A slot's fare is its price times one minus the flight master's discount, as a float, then converted to an integer with
`fistp`. The price is the `TaxiPath` price from the current node when a direct path exists, otherwise the route's fare
sum. The discount comes from the flight master for the player (`0x612b80`). With no active player, no unit for the master's
GUID or neither source, the fare is 0. `TaxiNodeCost(slot)` raises `Invalid taxi node slot` past the count.

Addresses: `0x4dc3d0`, `0x4dc3f1`, `0x4dc41c`, `0x4dc44f`, `0x4dc45d`, `0x4dc462`, `0x4dc47b`, `0x4dc480`, `0x4dc488`, `0x4dc495`, `0x4dc4ad`, `0x4dc4bd`, `0x4dc4ca`, `0x612b80`, `0x4dcc20`, `0x4dcc63`, `0x4dcc67`, `0x4dcc7c`
Confidence: verified
Proof: one
Test: `ui::glue_4dc_4dc3d0_fade_step`

The discount is left on the FPU stack by `0x612b80`; `fsubr` with the constant 1.0 (`0x4dc462`) leaves the factor, `fild`
loads the price (`0x4dc47b` for a direct path, `0x4dc4ad` for the sum) and the product is stored as a float
(`0x4dc480`) before `fistp` (`0x4dc488`). The direct test is a table read (`0x4dc44f`); without it the route flag byte
(`0x4dc495`) decides. A slot with no direct path that was never routed carries the start fare 999999, so its fare is that figure less the
discount.

## Taking a node

`TakeTaxiNode(slot)` asks the server to start the flight. It does nothing for a slot past the count. A mounted player
(mount display above 0) shows message 0xb2. Taking the current node shows 0xaa. When a path joins the current node
to the slot's node it sends `CMSG_ACTIVATETAXI` (0x1ad): the 8-byte GUID of the flight master, the u32 id of the current
node, the u32 id of the target. Otherwise, with a route, it sends `CMSG_ACTIVATETAXIEXPRESS` (0x312): the GUID, the u32 fare
sum (before any discount), the u32 node count, then each node id as a u32, current node first. With no route it shows 0xab.
`CMSG_TAXIQUERYAVAILABLENODES` (0x1ac) is the GUID alone.

Addresses: `0x4dc8d0`, `0x4dc8dc`, `0x4dc903`, `0x4dc909`, `0x4dc911`, `0x4dc913`, `0x4dc93d`, `0x4dc952`, `0x4dc95b`, `0x4dc965`, `0x4dc96f`, `0x4dc98e`, `0x4dc999`, `0x4dc9be`, `0x4dcca0`, `0x4dccd7`, `0x5ed0a0`, `0x5ed0a9`, `0x5ed0db`, `0x5ed0fe`, `0x5ed130`, `0x5ed13b`, `0x5ed16d`, `0x5ed179`, `0x5ed185`, `0x5ed19a`, `0x5ed1aa`, `0x5ed020`, `0x5ed029`, `0x5ed05d`, `0x5ed068`
Confidence: verified
Proof: one
Test: none

The mount test is `cmp [..+0x1fc]` through `jle` (`0x4dc909`, `0x4dc911`), so only a positive value refuses. The no-path
byte at `0x4dc95b` and the route pointer at `0x4dc965` must both be set to send the express request. The packet writers
are `PutGuid` and `PutUInt32`; the opcode is the constructor argument (`0x5ed0a9`, `0x5ed13b`, `0x5ed029`). The single
request does not depend on the computed route, and a refusal by the client sends nothing.

## Activation replies

`SMSG_ACTIVATETAXIREPLY` (0x1ae) is a u32 code. Code 0 closes the map. Codes 1 to 12 show a message by the table below.
A code of 13 or more shows nothing. The message ids index the client messages `ERR_TAXI...`.

| code | message | code | message |
|---|---|---|---|
| 1 | 0xac, unspecified server error | 7 | 0xb1, player busy |
| 2 | 0xab, no such path | 8 | 0xb2, player already mounted |
| 3 | 0xad, not enough money | 9 | 0xb3, player shapeshifted |
| 4 | 0xae, too far away | 10 | 0xb4, player moving |
| 5 | 0xaf, no vendor nearby | 11 | 0xaa, same node |
| 6 | 0xb0, not visited | 12 | 0xb6, not standing |

Addresses: `0x5ed1e0`, `0x5ed1e6`, `0x5ed1eb`, `0x5ed1ef`, `0x5ed1f7`, `0x5ed203`, `0x85fedc`, `0x5e3ef9`, `0x5e3f2c`, `0x5e3f37`, `0x496720`
Confidence: verified
Proof: one
Test: none

The dispatcher reads the code with `GetUInt32` (`0x5e3f2c`) after finding the active player. `0x5ed1e6 cmp eax,0xd; jae`
drops codes of 13 and more, and a code of 0 is the `je` at `0x5ed1eb`. The table entry for a code is read at `0x5ed1ef` and
shown through `DisplayError` (`0x5ed1f7`). Entry 0 holds 0x1d1, which is never read.

## Route lines

The line builder `0x4dc5c0(slot)` makes a list of segments for a slot, each 0x18 bytes: the source u, v and 0, then the
destination u, v and 0. A slot with a direct path gets the segment current node to node. A slot with a route gets one
segment per consecutive pair of its route. A slot with no route, or past the count, builds nothing. `TaxiGetSrcX`, `TaxiGetSrcY`, `TaxiGetDestX` and `TaxiGetDestY` take a 1-based slot and a
1-based segment and return the coordinate, 0 out of range. `GetNumRoutes(slot)` returns 1 for a direct path, otherwise the
route's node count less 1, and 0 for a bad slot. `TaxiNodeSetCurrent(slot)` rebuilds the lines and returns its argument.

Addresses: `0x4dc5c0`, `0x4dc5d5`, `0x4dc5ea`, `0x4dc600`, `0x4dc631`, `0x4dc643`, `0x4dc67e`, `0x4dc6d7`, `0x4dc738`, `0x4dc776`, `0x4dc7a2`, `0x4dc812`, `0x4dc890`, `0x4dc9d0`, `0x4dc9fc`, `0x4dca0c`, `0x4dca1d`, `0x4dca2a`, `0x4dca32`, `0x4dca3a`, `0x4dc170`, `0x4dc18b`, `0x4dc191`, `0x4dcd40`, `0x4dcd77`, `0x4dcd90`, `0x4dcde0`, `0x4dce30`, `0x4dce80`, `0x4dced0`, `0x4db2d0`, `0xbb3a68`, `0xb73a4c`, `0x84ada4`
Confidence: verified
Proof: one
Test: none

Each slot has a 16-byte array record: capacity `+0`, count `+4`, buffer `+8`. A segment's x is its first float and y the
second (`+0`, `+4` source; `+0xc`, `+0x10` destination). The builder runs for every slot when a map opens. The segment
count call is `0x4dc170`: a slot whose no-path byte is 0 answers 1, otherwise its node count less 1 (`0x4dc191`), so a
slot never routed answers 999998 and the current node 0. Its u and v use the points' own spans (`ui/taxi#map-art-and-positions`).

## New flight path

`SMSG_NEW_TAXI_PATH` (0x1af) has an empty body. The client shows message 0xf2 (`ERR_NEWTAXIPATH`) and clears the gossip
target. The message is routed as an information line (`UI_INFO_MESSAGE`) and plays the interface sound `TaxiNodeDiscovered`
by name, because its tag word is 0x44. Clearing the gossip target does nothing when none is set; otherwise it ends the
interaction, zeroes the GUID cell and signals `GOSSIP_CLOSED` (299). The interaction clear (`0x4e1fc5`, `0x4931c0`) runs the
dispatcher `0x493310`, which calls the close `0x4e1fa0` again while the GUID is still set, so the event fires twice
when the NPC is still the interaction target.

Addresses: `0x5e3e50`, `0x5e3e55`, `0x5e3e68`, `0x496720`, `0x49673d`, `0x496784`, `0x49679a`, `0x4967ab`, `0x496810`, `0x486d17`, `0x486d21`, `0x486d27`, `0x486d31`, `0x486235`, `0x484ca6`, `0x4e1fa0`, `0x4e1fb0`, `0x4e1fbc`, `0x4e1fc5`, `0x4e1fd6`, `0x4e1fe9`, `0x4930d0`, `0x458030`, `0xbc3f58`
Confidence: verified
Proof: one
Test: none

The descriptor of message 0xf2 is written by constants: the key (`0x486d17`), route 1 (`0x486d21`, from
`0x486235`), the sound name (`0x486d27`) and the tag 0x44 (`0x486d31`, from `0x484ca6`). `0x496720` tests the tag
(`0x49673d`) and, when it is 0x44 and the sound name is not `NONE` (`0x49679a`), plays the name (`0x4967ab`); any other tag
plays the vocal line of that id instead. Route 1 goes through `0x4945b0` as `UI_INFO_MESSAGE`
(`ui/inventory#failure-messages`). `0x4e1fa0` returns when the argument equals the stored GUID (`0x4e1fb0`), clears on a zero
argument (`0x4e1fbc`, `0x4e1fc5`, `0x4e1fd6`) and sets the target with the range cell otherwise (`0x4e1fe9`).

## Flight curve

A spline installed with flag bit 9 (0x200, flying) takes curve mode 1; any other spline takes mode 0. Mode 0 places a
point on the straight line between control points `i + 1` and `i + 2` of the segment. Mode 1 is a Catmull-Rom cubic: the
four control points `i` to `i + 3` are weighted by four polynomials in t, so a taxi flight follows a smooth curve. The
weights are the rows `-0.5t^3 + t^2 - 0.5t`, `1.5t^3 - 2.5t^2 + 1`, `-1.5t^3 + 2t^2 + 0.5t` and `0.5t^3 - 0.5t^2`.
The facing and the bank of the follower are `collision/kinematics#spline-follower`.

Addresses: `0x7c6a50`, `0x7c6a8f`, `0x7c6a94`, `0x7c6a9d`, `0x4541b0`, `0x4541b6`, `0x4541bb`, `0x454238`, `0x453580`, `0x4535c4`, `0x453620`, `0x453fa0`, `0xb05e10`
Confidence: verified
Proof: one
Test: none

`0x7c6a8f test dh,2` stores 1 or 0 at `+0x54` of the spline, which is `+0x28` of the curve at `+0x2c`. `0x4541b6` reads that
word: zero takes the linear path (`0x4541bd` to `0x454221`), anything else calls `0x453580` with the basis at `0xb05e10`
(`0x454231`). The basis is written by a static initializer (`0x453fa0`) as 16 floats. `0x453620` evaluates a degree-3
polynomial by Horner's rule (`0x453630`).
