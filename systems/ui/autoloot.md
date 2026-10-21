# ui: auto-loot

The sweep that takes a loot window's contents for a player who held Shift at the click, what the window holds
after it, and what happens when the server refuses the stores. The sweep is in `LootFrame.cpp` (`0x4c1fa0`); the
byte that starts it belongs to the player (`objectlayer/interaction#the-auto-loot-byte`); the window is `ui/loot`.

Roots: `0x4c1fa0`

## Auto-loot

`0x4c1fa0(flag)` sweeps the open window. It returns at once while item records are pending, and with no active player only signals `LOOT_OPENED`. Otherwise, in order: it sends `CMSG_LOOT_MONEY` when the money cell is not 0 (−1 too), and for money above 0 plays the coin sound and, outside a party and a raid, prints the money line, once per source; it sends each row of loot type 0 as a click does (item sound, `CMSG_AUTOSTORE_LOOT_ITEM`), except a bind-on-pickup row of quality 2 or more, of which the first raises `LOOT_BIND_CONFIRM`; other types are left; it clears the player's auto-loot byte; then it closes the loot with release when `LootHasContents` finds nothing, else signals `LOOT_OPENED` if `flag` is 0.

Addresses: `0x4c1fa0`, `0x4c1fb3`, `0x4c1fd9`, `0x4c1fe0`, `0x4c1ff4`, `0x4c1ffc`, `0x4c2008`, `0x4c2015`, `0x4c2020`, `0x4c2030`, `0x4c203d`, `0x4c2170`, `0x4c2196`, `0x4c21b3`, `0x4c21bc`, `0x4c21c2`, `0x4c21d4`, `0x4c21dc`, `0x4c21f8`, `0x4c21ff`, `0x4c2219`, `0x4c221e`, `0x4c2228`, `0x4c2237`, `0x4c2240`, `0x4c224f`, `0x4c2a70`, `0x5ea8c0`, `0x5e1ad0`, `0x5ebce0`, `0x5df460`, `0x48f200`, `0x48f264`, `0x4c1ed3`, `0x4c2b29`, `0x5df480`, `0xb71b44`, `0xb71bc4`, `0x847cec`
Confidence: verified
Proof: one
Test: `ui::auto_loot_sweep_difftest`

`0x4c1fb3 jne` returns on `0xb71b44`; `0x4c1fd9 jne` leaves the no-player arm (`0x4c1fe0` signals 267). `0x4c1ff4 je` skips a zero money cell, not −1; `0x4c1ffc` calls `0x5ea8c0`, which sends only while the player's loot target `+0x1d28` is set. `0x4c2008 jne` (`0xb71bc4`) and `0x4c2015 jle` gate the sound (`0x4c2020`) and the line, which `0x4c2030` (a party member, `0xbc6f48`) and `0x4c203d` (a raid, `0xb713e0`) skip (`0x4c2170` prints). `0x4c2196 jne` skips rows whose wire type (`0x5ebce0`) is not 0; `0x4c21b3` and `0x4c21bc` test bonding 1 and quality 2 or more; `0x4c21c2` keeps one confirmation (`0x4c21d4` signals 287, `0x4c21dc` stores the position in `0x847cec`); `0x4c21ff` calls `0x5e1ad0` with the wire slot byte of `0x4c21f8`, not the array position. `0x4c2219` calls `0x5df460` with 0, `0x4c221e` stores 1 in `0xb71bc4`, `0x4c2228` calls `0x4c2a70`; `0x4c2237 jne` returns for a nonzero flag, `0x4c2240` signals 267, and `0x4c224f` is `0x48f200` with release 1. Callers: `0x4c1ed3`, `0x4c2b29` and `0x5df480` pass flag 0; `0x48f264` passes the close's own flag for a loot type 4 window with contents. The test runs the whole function with `0x468550`, `0x5ebce0`, `0x5df460` and `0x4c2a70` run for real.

## Window state after the sweep

The sweep's sends change no row and not the money cell: a row goes when its `SMSG_LOOT_REMOVED` arrives (`0x5ebeb0` to `0x4c1f00`) and the coin row when `SMSG_LOOT_CLEAR_MONEY` does (`0x5ec030` to `0x4c2750`), both after the sweep has returned. `LootHasContents` counts as left a source that still resolves with money above 0 or any occupied row, sent or not. So a sweep that found money or a row ends by signalling `LOOT_OPENED`, before any reply; only an empty window or a source gone from the object manager closes with release. Each cleared row then signals `LOOT_SLOT_CLEARED`; the last closes the loot with release.

Addresses: `0x4c2a70`, `0x4c2a8c`, `0x4c2a93`, `0x4c2a9c`, `0x4c2aa6`, `0x4c2aae`, `0x5ebeb0`, `0x5ec030`, `0x4c1f00`, `0x4c1f77`, `0x4c1f7f`, `0x4c1f8d`, `0x4c2750`, `0x4c2765`, `0x4c276f`, `0x4c2777`, `0x4c2785`, `0xb71968`, `0xb71ba0`
Confidence: verified
Proof: one
Test: `ui::auto_loot_sweep_difftest`

The sweep (`0x4c1fa0` to `0x4c2257`) stores only `0x847cec` and `0xb71bc4`, and no callee writes a row or the money cell: `0x5df460` stores the byte, `0x48f200` ends the session and the signals run the interface's Lua, which has no function that takes a row (`ui/loot#loot-functions`). `0x4c2a8c` resolves the source with mask 1 and `0x4c2a93 je` answers 0 for none; `0x4c2a9c jle` tests the money cell above 0 and `0x4c2aa6` to `0x4c2ab8` the 16 entries for one nonzero. `0x4c1f77` signals 268, `0x4c1f7f` asks again and `0x4c1f8d` closes with release when it answers 0; `0x4c2750` does the same for the coin row. The test reads the 16 rows and the money cell back after each sweep and finds them unchanged.

## Auto-loot with full bags

Refused stores leave the window open. A server that cannot fit the items the sweep asked for answers each `CMSG_AUTOSTORE_LOOT_ITEM` with `SMSG_INVENTORY_CHANGE_FAILURE` and no item GUID (vmangos does: `LootHandler.cpp:234`, `Player.cpp:11700-11701`). Each shows the error for its result (50, `ERR_INV_FULL`) and signals no loot event: the tail that can signal `LOOT_OPENED` needs the auto-loot byte, which the sweep cleared. So the player sees `LOOT_OPENED` once, from the sweep, which shows the stock window; the coin row, if taken, clears by `LOOT_SLOT_CLEARED` (1); the refused rows stay. Only a closer ends the window: `CloseLoot`, a movement start, `SMSG_LOOT_RELEASE_RESPONSE`, another loot opening, or the source going away.

Addresses: `0x5e38c0`, `0x5e3a4b`, `0x5e3acf`, `0x5e3ad8`, `0x5e3b0d`, `0x5e3b15`, `0x5e3b20`, `0x4c2219`, `0x622630`, `0x496720`, `0x4c1f00`, `0x4c2750`, `0x60e990`, `0x4c2ec0`, `0x5ec090`, `0x4c1cb0`, `0x5d8660`, `0x493974`
Confidence: verified
Proof: one
Test: none

Every nonzero result of `SMSG_INVENTORY_CHANGE_FAILURE` reaches `0x5e3ad8` (`objectlayer/playermessages#inventory-change-failures`): with both item GUIDs zero `0x5e3a4b` and `0x5e3acf` skip their releases, and `0x5e3b0d` loads the byte the sweep cleared at `0x4c2219`, so `0x5e3b15 je` leaves without `0x5e3b20`. `0x622630` maps result 50 to `ERR_INV_FULL` (`objectlayer/inventory#inventory-results`). Nothing in this path closes the window. The closers are `CloseLoot` (`0x4c2ec0`, `ui/loot#loot-functions`), the movement-start guard `0x60e990` (`ui/loot#no-walk-away-leash`), the release response `0x5ec090`, which clears the target and ends the loot (`objectlayer/inventory#loot`), a new source in `0x4c1cb0` (`ui/loot#opening-and-closing-a-session`), an item source's deactivation `0x5d8660` (`objectlayer/items#item-slots`) and the target release `0x493974` (`ui/targeting#releasing-the-old-target`).

## Looting with a window open

A Shift-click on a second lootable while a window is open sweeps the first and loots nothing else. The click's `SetAutoLoot(1)` runs the sweep on the open window at once, since the loot target is held, and the sweep clears the byte. `LootUnit` and `LootCorpse` then stop at `CanLootNow`, which refuses while any loot target is held, the same object included. Only when the sweep finds nothing left and closes the window, clearing the target, do they go on, and the second loot then opens with the byte at 0: no auto-loot.

Addresses: `0x5d6c68`, `0x60bfe4`, `0x5d6c78`, `0x60bff7`, `0x5df460`, `0x5df470`, `0x5df480`, `0x5df130`, `0x5df151`, `0x5df2a0`, `0x5df2c1`, `0x5ec110`, `0x5ec1e7`, `0x5ec1f4`, `0x5ec1f6`, `0x4c2219`, `0x4c224f`, `0x48f2c9`
Confidence: verified
Proof: one
Test: none

`0x5d6c68` and `0x60bfe4` call `0x5df460` with 1 before `0x5d6c78` (`0x5df130`) and `0x60bff7` (`0x5df2a0`). `0x5df470` finds `+0x1d28` set and `0x5df480` runs the sweep, which clears the byte at `0x4c2219`. `0x5df151` and `0x5df2c1` call `0x5ec110` first. It compares the held target with the object (`0x5ec1e7`), returns 0 for the same one (`0x5ec1f4 je 0x5ec1ca`) and for any other nonzero target (`0x5ec1f6 or eax,edx; jne 0x5ec1ca`); the walk-up and `CMSG_LOOT` come after it. A sweep that finds nothing closes the loot (`0x4c224f`), and `0x48f2c9` zeroes the target before `0x5df130` or `0x5df2a0` asks.

