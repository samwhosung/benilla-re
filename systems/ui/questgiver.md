# ui: a quest giver's quest list

How the quest list of a quest giver is split into the quests the player can accept and the quests in
progress, from one wire field: the two pools of the quest frame, the opcode that fills them, what clicking a
row sends, and the same split on the gossip path.

The code is `QuestFrame.cpp`, `0x500af0` to `0x501d40`: the pools and their Lua functions; the quest list
handler `0x5dbb10` is in the player code. The gossip window is `gossip`.

Roots: `0x500e40`, `0x500e90`, `0x5012a0`, `0x501320`, `0x501a80`, `0x501aa0`, `0x501ca0`, `0x501ce0`

## The two pools

The quest frame keeps two arrays of 32 rows of 0x4c bytes: the available quests at `0xbdfe60` (count `0xbe0834`)
and the active quests at `0xbde690` (count `0xbe0838`). A row is the quest id at `+0x00`, the level at `+0x04` and the
title (64 bytes) at `+0x08`; an available row also has the one-click flag at `+0x48`, and the active row's
`+0x48` is written 0 and read by nothing. The append functions copy the title only when it is not null, so
rows are meaningful up to the count.

Addresses: `0xbdfe60`, `0xbe0834`, `0xbde690`, `0xbe0838`, `0x500e40`, `0x500e4d`, `0x500e56`, `0x500e5c`, `0x500e7b`, `0x500e81`, `0x500e90`, `0x500e9d`, `0x500ea6`, `0x500eac`, `0x500ec8`, `0x500ed2`, `0x500c41`, `0x500c76`, `0x500c7c`, `0x500c82`
Confidence: verified
Proof: one
Test: none

`0x500e40(id, title, level, oneClick)` stores at `base = count*0x4c` (`0x500e4d`, `0x500e56`, `0x500e7b`) and increments the count
(`0x500e81`); `0x500e90(id, title, level)` does the same for the active pool and stores the literal 0 at `+0x48`
(`0x500ec8`). Capacity is the reset loop's 32 rows (`mov edx,0x20`, `0x500c41`). Each append function has two call
sites, both in `0x5dbb10`.

## The quest list message

Opcode 0x185, handled by `0x5dbb10`: `u64 giverGuid` (latched to `0xc4d740`), a C string greeting (at most
0x200), `u32 emoteDelay`, `u32 emote`, `u8 rowCount` and the rows: `u32 questId`, `u32 icon`, `u32 level`, and a
C string title of at most 0x40. Both pools are cleared first. A row is active when its icon is 3 or 4 and
available for every other value; an available row's one-click flag is `icon == 0`. The handler ends by firing
`QUEST_GREETING` (id 300). The quest log is never consulted.

Addresses: `0x5dbb10`, `0x5dbb25`, `0x5dbb38`, `0x5dbb43`, `0x5dbb4e`, `0x5dbb59`, `0x5dbb66`, `0x5dbb73`, `0x5dbb85`, `0x5dbb99`, `0x5dbbfe`, `0x5dbc00`, `0x5dbc03`, `0x5dbc05`, `0x5dbc08`, `0x5dbc13`, `0x5dbc15`, `0x5dbc24`, `0x5dbc3d`, `0x5dbc42`, `0x500ee0`, `0x500bd0`, `0x5e5a02`, `0xc4d2b0`, `0xc4d340`, `0xc4d6b0`
Confidence: verified
Proof: one
Test: none

`rowCount` is never clamped to 32: the loop compares its counter with the zero-extended byte (`0x5dbc46` to `0x5dbc4d`), so more than 32 rows overrun the staging arrays at `0xc4d2b0`, `0xc4d340` and `0xc4d6b0`, the pools (`0x500e40` and `0x500e90` have no cap) and the 0x800-byte title buffer at `ebp-0xa14`; around row 40 the writes reach the saved frame, a client fault.

The row reads fill three 32-dword staging arrays, ids at `0xc4d2b0`, icons at `0xc4d340` and levels at
`0xc4d6b0`, each cleared with `rep stosd` and `ecx = 0x20`; the title goes to a stack buffer advanced by 0x40. The test is
`cmp ebx,3; je` and `cmp ebx,4; je` to the active append (`0x5dbc00` to `0x5dbc08`), then `test ebx,ebx; sete cl` for the one-click flag.
`0x500ee0` is `mov ecx,0x12c; jmp 0x703e50`. The reset `0x500bd0(0)` sets both counts to 0. An auto-completing quest the
player has not taken arrives with icon 4 and so lands in the active pool.

## Clicking a row

`SelectAvailableQuest(n)` and `SelectActiveQuest(n)` take a 1-based row. The available one sends
`CMSG_QUESTGIVER_QUERY_QUEST` (opcode 0x186) for a normal row and `CMSG_QUESTGIVER_COMPLETE_QUEST` (0x18a) for a
one-click row; the active one always sends 0x18a. Each sends the dialog's source guid and the quest id, then sets the
acted latch `0xbe0844`, and does nothing while it is set. The row bound is `cmp esi,count; ja`, so the index is decremented before the compare and the 1-based row count+1 is accepted, reading one row past the pool.

Addresses: `0x501ca0`, `0x501cc4`, `0x501ce0`, `0x501d04`, `0x5012a0`, `0x5012a8`, `0x5012b1`, `0x5012ba`, `0x5012db`, `0x5012e3`, `0x501320`, `0x5eab80`, `0x5eab89`, `0x5eaca0`, `0x5eaca9`, `0xbe0810`, `0xbe0844`
Confidence: verified
Proof: one
Test: none

The wrappers decrement the index (`0x501cc4`, `0x501d04`). `push 0x186` is at `0x5eab89` and `push 0x18a` at `0x5eaca9`. The
one-click flag has one reader, `0x5012db`.

## The Lua functions

`GetNumAvailableQuests` and `GetNumActiveQuests` return the counts. `GetAvailableTitle`, `GetActiveTitle`,
`GetAvailableLevel` and `GetActiveLevel` return a row's title or level. Neither the id nor the icon is
exposed, so the stock frame learns the split only from which pool a row sits in.

Addresses: `0x501a80`, `0x501aa0`, `0x501ac0`, `0x501b30`, `0x501ba0`, `0x501c20`
Confidence: verified
Proof: one
Test: none

Each getter bounds the index by its count and reads `idx*0x4c` from the pool: `+0x08` for a title and `+0x04` for a level.

## The split on gossip

The gossip message (`gossip`) keeps its quest rows in one array and splits them when asked, with the same
test: icon 3 or 4 is active. Four scanners count and pick available and active rows; each stops at a row with
quest id 0. `GetGossipAvailableQuests` and `GetGossipActiveQuests` push two values for each match, the title
and then the level.

Addresses: `0x4e2430`, `0x4e2444`, `0x4e244e`, `0x4e2460`, `0x4e2580`, `0x4e2594`, `0x4e259e`, `0x4e25b0`, `0x4e24b0`, `0x4e24e6`, `0x4e24ee`, `0x4e2600`, `0x4e2930`, `0x4e29b0`, `0x4e2455`
Confidence: verified
Proof: one
Test: none

`0x4e2430` and `0x4e2460` skip icons 3 and 4 (`cmp edx,3 je`, `cmp edx,4 je`); `0x4e2580` and `0x4e25b0` keep them. The
loop bound `cmp ecx,0x4180` is 32 rows of 0x20c bytes. The pools of this document are not touched on
the gossip path.
