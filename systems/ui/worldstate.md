# ui: world states in the interface

The server pushes numbered values, world states, that the client keeps in a table. `WorldStateUI.dbc` says which of them the always-visible zone and battleground readouts show, and two Lua functions hand a row and its live values to the interface. This doc is the row layout, the list builder and its filters, `GetNumWorldStateUI` and `GetWorldStateUIInfo`, the packets that fill the table,
the text expander and the scoreboard columns of the same table. The table itself, its getter and setter are `questtext#world-state-numbers`; the channel trigger is `chatchannels#defence-channel-state`.

Roots: `0x4c56e0`, `0x4c5650`, `0x4c5a40`, `0x4c5a70`, `0x508560`, `0x4abcd0`, `0x4abd00`

## The rows

A `WorldStateUI.dbc` row is 39 dwords (0x9c bytes; the loader `0x5539f0` asserts both sizes). The columns the client reads: `+0x0` id, `+0x4` map id (-1 for any), `+0x8` area id (0 for any), `+0xc` icon string, `+0x10` to `+0x30` the localised text (locale index picks the dword), `+0x34` to `+0x54` the tooltip,
`+0x5c` the state variable key, `+0x60` the type 0, 1 or 2, `+0x64` the dynamic icon, `+0x68` to `+0x88` the dynamic tooltip, `+0x8c` the extended UI name and `+0x90`, `+0x94`, `+0x98` three more keys. Column `+0x58` is stored by the loader and read by nothing.

Addresses: `0x5539f0`, `0x553a9e`, `0x553ad6`, `0x553b16`, `0x553b31`, `0x553bfa`, `0x553c19`, `0x588110`, `0x588657`, `0x588662`, `0x4c5725`, `0x4c5739`, `0x4c574c`, `0x4c576d`, `0x4c5aeb`, `0x4c5b17`, `0x4c5b3d`, `0x4c5b51`, `0x4c5b62`, `0x4c5b73`, `0x4c5b80`, `0x4c5b9e`, `0x4c5bbc`, `0xc0d564`, `0xc0d568`, `0xc0d56c`, `0xc0d570`
Confidence: verified
Proof: one
Test: none

The loader adds the string block base to every string offset without a zero test, so an empty column is a pointer to the empty string, never null. The id index is zero-filled, so the id 0 is null in it. The store is `uint32_t[5]`: rows, count, id index, highest id and the loaded buffer.

## The list builder

`0x4c56e0` rebuilds the always-visible list, an array of row ids at `0xb71e7c` (count `0xb71e78`), from the empty list. A row is taken when its map id is -1 or equals the map of the last `SMSG_INIT_WORLD_STATES` (`0xb71e84`), its area id is 0 or equals that message's zone (`0xb71ea8`) and its type is 0, or its type is 1 while the player is joined to the zone defence channel (`0xb6e5dc`).
Type 2 rows never enter; they are the scoreboard columns. The array grows in chunks of up to 0x40.

Addresses: `0x4c56e0`, `0x4c56ec`, `0x4c56f9`, `0x4c5725`, `0x4c572d`, `0x4c5740`, `0x4c574c`, `0x4c5753`, `0x4c575c`, `0x4c5767`, `0x4c57a8`, `0x4c57c8`, `0x4c57e4`, `0x4c57ea`, `0x4c56ce`, `0x49bdcf`, `0xb71e74`, `0xb71e78`, `0xb71e7c`, `0xb71e80`, `0xb71e84`, `0xb71ea8`, `0xb6e5dc`
Confidence: verified
Proof: one
Test: none

A failed test skips that row only. The builder has two callers: `0x4c5650`, on every INIT message, and the defence channel change. A zone change does not rebuild it: the list is for the map and zone the server last sent. Both set the filters first (`0x4c5650`) or use the last ones.

## The two functions

`GetNumWorldStateUI()` pushes the count `0xb71e78`. `GetWorldStateUIInfo(index)` raises `Usage: GetWorldStateUIInfo(index)` for a non-number and takes the 1-based, truncated index into the list. An index outside the list, or a row that is missing, pushes the single number 0 (not nil). Otherwise it pushes ten values:
the value of the row's key `+0x5c` (1.0 when the key is 0), the text with its world-state tokens expanded, the icon, the dynamic icon, the tooltip, the dynamic tooltip, the extended UI name and the values of the three extended keys.

Addresses: `0x4c5a40`, `0x4c5a70`, `0x4c5a82`, `0x4c5a8b`, `0x4c5aad`, `0x4c5ab8`, `0x4c5abb`, `0x4c5b05`, `0x4c5ace`, `0x4c5af2`, `0x4c5b09`, `0x4c5b2b`, `0x4c5be5`, `0x4c5bdb`, `0x40a2b0`
Confidence: verified
Proof: one
Test: none

Only the text is expanded; the tooltips and the other strings are pushed as stored. Every string is a Lua string, empty where the column is empty. A stored value above 2^31 - 1 reads as negative (`fild` of a signed dword). An out-of-range index looks up row id 0, which the id index holds as null.

## The text expander

`0x508560(source, destination, size)` copies the source and replaces each `%<digits>w` or `%<digits>W` with the world state value of that key as decimal text; nothing else is a token. Any other character after a `%` keeps a literal `%`, flags an error and is not consumed; the function returns whether no error occurred. A null source returns 0 and writes nothing.
`GetWorldStateUIInfo` passes a 260-byte buffer and a size of 0x100.

Addresses: `0x508560`, `0x508566`, `0x508586`, `0x5085c9`, `0x5085ef`, `0x5085f3`, `0x5085fb`, `0x508610`, `0x508617`, `0x50861d`, `0x508628`, `0x50863e`, `0x508660`, `0x4c5810`, `0xbe0b80`, `0x835154`
Confidence: verified
Proof: one
Test: none

It is not the quest text expander (`0x5070a0`); they share only the getter. The scan cursor `0xbe0b80` is a global, so the function is not re-entrant. With no digits the key is 0, a real lookup.

## The packets

`SMSG_INIT_WORLD_STATES` (0x2c2) is `u32 mapId`, `u32 zoneId`, `u16 count` and `count` pairs of `u32 id, u32 value`. It first clears the table (`0x4c5650(mapId, zoneId)`: every entry is freed, the bucket array stays, and the builder filters are set), then stores each pair with `0x4c5870`, a count of 0 skipping the loop.
`SMSG_UPDATE_WORLD_STATE` (0x2c3) is one `u32 id, u32 value`. Both end with the world map rebuild `0x4a67a0` and the event `UPDATE_WORLD_STATES` (526).

Addresses: `0x48f690`, `0x48f933`, `0x48f971`, `0x48f990`, `0x48f9a6`, `0x48f9c4`, `0x48f9d2`, `0x48f9f1`, `0x48f9f6`, `0x48fa0d`, `0x48fa12`, `0x4c5650`, `0x4c565b`, `0x4c56c2`, `0x4c5870`, `0x4c5a23`, `0x6dabc0`, `0x6d9a90`, `0xb71ec8`, `0xb71ed0`
Confidence: verified
Proof: one
Test: none

The count is a 16-bit word and the loop has no terminator test. The setter overwrites an existing key and never deletes, so a value of 0 stays a node. After each store `0x6dabc0` finds the first minimap landmark candidate with a nonzero position and the same world state id (`+0x70`) and redraws it (`minimapblips`). The list is rebuilt on INIT only, the event fires on both.

## The scoreboard columns

The type 2 rows are the columns of the battleground score board. The status handler for 0x2d4 (`0x4aa850`) builds them inline into `0xb6e6ec` (count `0xb6ebd8`): rows of the current battleground map or any map, of type 2, as the first contiguous run: after the first accepted row the first row that does not match ends the scan.
`GetNumBattlefieldStats()` pushes the count and `GetBattlefieldStatInfo(index)` pushes three strings: the raw text, the icon and the raw tooltip; a bad index pushes three nils.

Addresses: `0x4aa850`, `0x4aa9c3`, `0x4aa9d1`, `0x4aa9dd`, `0x4aa9fe`, `0x4aaa44`, `0x4abcd0`, `0x4abd00`, `0x4abd41`, `0x4abd50`, `0x4abd65`, `0x4abd76`, `0x4abd80`, `0x4abd91`, `0x4abd98`, `0x4a9c55`, `0xb6e6ec`, `0xb6ebd8`
Confidence: verified
Proof: one
Test: none

The text is not expanded here, unlike `GetWorldStateUIInfo`, and a bad index gives three nils where that function gives the number 0. `battlefield#scores-and-stats` has the per-player values.
