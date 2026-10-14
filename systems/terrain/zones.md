# terrain: zones

How the client turns the player's position into a zone and a subzone: the area id under an entity,
the zone and subzone names, the audio of the area and the zone update sent to the server. The
code is the zone names file, `0x67e390` to `0x67ea90`, with the entity queries of `0x6703a0` to
`0x6705a0` and `0x69d830` to `0x69da00`. AreaTable rows are indexed by id; a row holds the id at
`+0x0`, the parent at `+0x8`, the flags at `+0x10`, the sound provider preferences at `+0x14` and
`+0x18`, the ambience at `+0x1c`, the zone music at `+0x20`, the intro sound at `+0x24` and the
name for the locale at `+0x2c + 4 × locale`. A WMOAreaTable row holds the id, WMO id, name set,
group id at `+0x0` to `+0xc`, the same five sound fields at `+0x10` to `+0x20`, the flags at `+0x24`,
the AreaTable id at `+0x28` and the name at `+0x2c + 4 × locale`.

Roots: `0x67e510`, `0x67e9c0`, `0x6703a0`, `0x6704e0`, `0x6704f0`, `0x6705a0`

## Area update

`0x67e510` runs on every tenth poll of the active mover with the map id (`ecx`) and the mover's
entity (`edx`). It does nothing for a null entity, an entity with no area id, or a zone id of 0.
The area row is the leaf; the zone row is the leaf's parent (`+0x8`), or the leaf itself when the
parent is 0, and then the subzone text is null. An entity that is indoors takes the indoor update,
any other the outdoor update; when that reports a change, the interface is told the zone id, the
zone text, the subzone text, the real zone text and the indoor bit, then the minimap text, and
the indoor bit is stored.

Addresses: `0x67e510`, `0x67e52d`, `0x67e546`, `0x67e56c`, `0x67e572`, `0x67e57d`, `0x67e58a`, `0x67e591`, `0x67e5c4`, `0x67e5dc`, `0x67e5fb`, `0x67e645`, `0x67e656`, `0x67e65b`, `0x494780`, `0x494970`, `0x6704a0`, `0x5db900`
Confidence: verified
Proof: one
Test: none

The leaf id must be 0 to the highest id (`0xc0e04c`, `jg` at `0x67e566`) and its row non-null (`0x67e56c` to
`0x67e577`). A parent above the highest id or below 0 sends `0x67e591` to `xor ebx,ebx; mov edx,[ebx]`, a read of
address 0, and a null row in range does the same: the client faults on such data. `0x67e5c4`
tests the indoor bit from `0x6704a0`: set calls `0x67e670` (`0x67e5dc`), clear calls `0x67e7f0`
(`0x67e5fb`); a 0 result ends the function. An empty text is passed as null. `0x67e645` calls `0x494780` with the zone id, the
zone text, the subzone text, the real zone text (the name before an indoor override) and the
indoor bit, `0x67e656` calls `0x494970` with the subzone text, else the zone text, and `0x67e65b`
stores the indoor bit in the indoor cache.

## Outdoor area change

`0x67e7f0` compares the zone id, the leaf id and the map id with the cache when the last update was
outdoors and returns 0 when all three match. Otherwise it stores them and applies the leaf's audio
fields, each from the zone row where the leaf's value is 0: reverb from `+0x14` and `+0x18`, music from
`+0x20` and `+0x24`, ambience from `+0x1c` with the override flag 0. It then sends the zone update
and returns 1.

Addresses: `0x67e7f0`, `0x67e809`, `0x67e82d`, `0x67e86b`, `0x67e90a`, `0x67e914`, `0x67e91b`, `0x67e925`, `0x45a700`, `0x461260`, `0x460c50`, `0x67e940`
Confidence: verified
Proof: one
Test: none

`ret 4`. `0x67e801` tests the indoor cache: when it is 0 and the three cached words match the
arguments the function returns 0 (`0x67e823`); a nonzero indoor cache skips the compare, so
coming back outside always applies. A field is the leaf's when it is −1 or nonzero, the zone row's
when it is 0 (`cmp eax,-1; je` then `cmp eax,0; je`, as at `0x67e86b`), and 0 with no rows.
`0x67e90a` calls `0x45a700` with `+0x14` and `+0x18`, `0x67e914` `0x461260` with `+0x20` and `+0x24`,
`0x67e91b` `0x460c50` with `+0x1c` and dl = 0, and `0x67e925` sends the zone id.

## Indoor area names

`0x67e670` takes the entity, the zone text slot, the subzone text slot and the zone id. It returns 0 without change
when the last update was indoors and the entity's group key (or the zone id when it has none)
equals the stored key. Otherwise it stores the key, replaces the zone text with the whole-WMO
name and clears the subzone when that name is non-empty and differs, ignoring case, from the
subzone, sets the subzone to the group's own name when that is non-empty, applies the audio of
the group's WMOAreaTable row, each field from the whole-WMO row where it is 0, with the
override flag 1, and returns 1.

Addresses: `0x67e670`, `0x67e688`, `0x67e698`, `0x67e6a0`, `0x67e6ac`, `0x67e6c4`, `0x67e6e4`, `0x67e6f0`, `0x67e70d`, `0x67e71e`, `0x67e7c5`, `0x67e7cf`, `0x67e7d8`, `0x6704e0`, `0x6704f0`, `0x6705a0`, `0x69d830`, `0x69d8f0`, `0x69d980`, `0x64a4c0`
Confidence: verified
Proof: one
Test: none

`0x67e688` calls `0x6704e0` for the whole-WMO name, `0x67e698` calls `0x6704f0` for the group name and
the key (the key overwrites the zone-id argument when the group's `+0x7c` is nonzero). `0x67e6a0`
and `0x67e6ac` are the skip: the indoor cache above 0 and the key equal. `0x67e6e4` compares with
`SStrCmpI` and a limit of 0x7fffffff; equal skips `0x67e6f0`, which stores the name and clears the
subzone. `0x67e70d` stores a non-empty group name. `0x67e71e` calls `0x6705a0` for the two rows; the
audio calls are `0x67e7c5`, `0x67e7cf` and `0x67e7d8`, the last with dl = 1.

## Entity area names

`0x69d830` writes a name for an entity in a map object group: the whole-WMO row's name, or when
that is empty the name of its AreaTable row, or when it has none the name of the terrain area
under the entity; the default is the empty string. `0x69d8f0` writes the group's own WMOAreaTable
name (empty when the row is missing), cached in the group node at `+0x8c`, and the group key
`+0x7c` when nonzero. `0x69d980` writes the group's row and the whole-WMO row. Each returns 1 only
when `0x69d4f0` finds a group. `0x6704e0`, `0x6704f0` and `0x6705a0` call them for a non-null
entity; `0x6705a0` also needs the entity indoors.

Addresses: `0x69d830`, `0x69d8f0`, `0x69d980`, `0x69d4f0`, `0x7ccde0`, `0x7ccd30`, `0x69b120`, `0x69d85f`, `0x69d87e`, `0x69d89a`, `0x69d8a4`, `0x69d93e`, `0x69d9c7`, `0x69d9e5`
Confidence: verified
Proof: one
Test: none

The key of a lookup is (WMO id `[[model + 0x120] + 0x20]`, name set `[map object + 0x128]`, group
id): `0x7ccde0` searches the table with a binary search and returns the row, `0x7ccd30` returns
the locale name or the empty string. `0x69d87e` and `0x69d9e5` pass −1 for the whole-WMO row;
`0x69d93e` and `0x69d9c7` pass the group's id, `[group record + 0x14c]`. `0x69d89a` and
`0x69d8a4` take the AreaTable id `+0x28` when the name is empty, and `0x69d8a4` falls back to `0x69b120`.

## Entity area id

`0x6703a0` returns 1 and writes an area id for an entity, or returns 0. With the map-is-one-object
flag set it is the map record's field at `+0x4c`, failing when that is 0. The record is the
map table `0xc0daa8` entry for the loaded map id, checked against the highest id (`0x6703ba`, `0x6703bc`); out
of range, or null, the read is of address 0x4c. Otherwise a null entity fails. The entity's first node decides: a group def (flags `+0x8` bit
0x10) gives its WMOAreaTable row's AreaTable id when that is nonzero; a def with flags bit 0x4 gives
its `+0x16c` field; anything else, and a zero id, gives the terrain area id `0x69b120` at the
entity position `+0x24`.

Addresses: `0x6703a0`, `0x6703a4`, `0x6703d1`, `0x6703de`, `0x6703fb`, `0x67042b`, `0x670437`, `0x670453`, `0x670458`, `0x67043e`, `0x69b120`, `0x6704a0`, `0x6703ba`, `0x6703bc`, `0x670400`, `0x670405`, `0x670409`, `0x67040b`, `0x67040d`, `0xc0daa8`, `0xc0daac`, `0x86a2cc`
Confidence: verified
Proof: one
Test: none

`0x6703a4` tests the flag; `0x6703d1` loads `[record + 0x4c]` from the table at map id (the
map record base is 0 for an id out of range, reading address `0x4c`). `0x6703fb` tests bit 0x10 of the
def's flags; `0x67042b` looks up with group id −1; `0x670437` takes `+0x28` and tests it;
`0x670453` tests bit 0x4 and `0x670458` loads `+0x16c`. For a group def, `0x670400` loads the def's `+0x20`;
when that is 0 or has bit 0 set (`0x670405`, `0x670409`), `0x67040b` zeroes the pointer and `0x67040d` reads address 8.

## Breath flag

`0x67e9c0` returns whether the entity's area lets a unit puff breath: bit 0 of the area's flags,
or of the parent's flags when the area's own bit 1 is clear and the area has a parent in range.
Indoors the area is the AreaTable row of the group's WMOAreaTable row, else of the whole-WMO row;
outdoors it is the entity's area id from `0x6703a0`. It returns 0 when no row is found.

Addresses: `0x67e9c0`, `0x67e9c9`, `0x67e9e9`, `0x67ea0a`, `0x67ea23`, `0x67ea68`, `0x67ea7a`, `0x67ea88`, `0x6703a0`, `0x6705a0`
Confidence: verified
Proof: one
Test: none

`0x67e9c9` tests indoors; `0x67e9e9` calls `0x6705a0`; `0x67ea0a` and `0x67ea23` read `+0x28` of the
two rows in turn, each range-checked against the table count and non-null. `0x67ea68` loads the parent row
(`+0x8`, in range or 0); `0x67ea7a` loads `+0x10`; `test al,2` keeps the area's flags when set, else the parent's when
there is one; `0x67ea88` ands with 1.

## Zone update message

`0x67e940` sends `CMSG_ZONEUPDATE` (0x1f4) with the zone id as a 32-bit word when the zone id is
nonzero, and frees the store.

Addresses: `0x67e940`, `0x67e950`, `0x67e977`, `0x67e986`, `0x418190`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

`0x67e950` pushes 0x1f4 to `0x418190` (PutUInt32); `0x67e977` appends the zone id; `0x67e986`
sends with `0x5ab630`. The store is a stack `CDataStore`; its destroy slot runs only when a buffer
was allocated.
