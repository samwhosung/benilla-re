# ui: reputation messages and requests

The five server messages that fill the reputation list, the three requests the client sends, what each does to the
slots, and the four places that signal `UPDATE_FACTION`. The list is `ui/reputation`; the Lua functions that call the
requests are `ui/reputationlua`.

Roots: `0x4d5290`, `0x4d5640`, `0x4d56b0`, `0x4d5710`, `0x4d5760`, `0x4d59a0`, `0x4d5fd0`, `0x4d60f0`

## Faction messages

`0x4d5290` registers five handlers and `0x4d5360` removes them. The slot in each message is the faction's
`reputationListID`. No message checks a slot or a count against the 64 slots.

| opcode | message | body |
|---|---|---|
| 0x122 | `SMSG_INITIALIZE_FACTIONS` | u32 count, then per entry a u8 flags and a u32 standing; the slot is the entry's position |
| 0x123 | `SMSG_SET_FACTION_VISIBLE` | u32 slot |
| 0x124 | `SMSG_SET_FACTION_STANDING` | u32 count, then per entry a u32 slot and a u32 standing |
| 0x313 | `SMSG_SET_FACTION_ATWAR` | u32 slot, u8 flags |
| 0x2a5 | `SMSG_SET_FORCED_REACTIONS` | u32 count, then per entry a u32 faction and a u32 rank |
| 0x125 | `CMSG_SET_FACTION_ATWAR` | u32 slot, u8 at war |
| 0x317 | `CMSG_SET_FACTION_INACTIVE` | u32 slot, u8 inactive |
| 0x318 | `CMSG_SET_WATCHED_FACTION` | u32 slot, -1 for none |

Addresses: `0x4d5290`, `0x4d529c`, `0x4d52ad`, `0x4d52be`, `0x4d52cf`, `0x4d52e0`, `0x4d5360`, `0x4d5381`, `0x4d53a9`, `0x4d5640`, `0x4d56b0`, `0x4d5710`, `0x4d5760`, `0x4d59a0`, `0xb73690`
Confidence: verified
Proof: one
Test: none

Each registration is `push 0; mov edx,<handler>; mov ecx,<opcode>; call 0x5ab650` (`0x4d529c` to `0x4d52e0`) and each removal
`mov ecx,<opcode>; call 0x5ab670` (`0x4d5381` to `0x4d53a9`). The readers are `GetUInt32` (`0x418e30` and `0x418eb0`) and
`GetUInt8`. The forced reaction list `0x4d59a0` stores the count, grows the array when the count exceeds its capacity,
reads the pairs in place and then refreshes the active player's reaction (`0x4d5a5e`). The pairs are read by
`objectlayer/reaction#the-active-players-reputation`.

## Initialize factions

The initialize message reads the count and, for each index from 0, one byte of flags and one standing. It stores the flags
and the standing into slot `index` unchecked, so a count above 64 writes past the slot array. Then it rebuilds the list and
signals `UPDATE_FACTION`. The signal is the second one of the message, because the rebuild ends in the sorter, which
signals too. A count of 0 or less reads nothing and still rebuilds.

Addresses: `0x4d5640`, `0x4d5651`, `0x4d565d`, `0x4d5666`, `0x4d5670`, `0x4d567b`, `0x4d5687`, `0x4d5692`, `0x4d5694`, `0x4d5699`, `0x4d569e`, `0x4d5fc0`, `0x4d6330`, `0x4d5430`
Confidence: verified
Proof: one
Test: none

The loop test is `jle 0x4d5694` for the count (`0x4d565d`) and `jl` at the bottom (`0x4d5692`), both signed. The standing store uses
log argument 0 (`0x4d5683 push 0`), so initialization logs nothing.

## Standing changes

Each entry of the standing message stores the standing, logging it only for the first entry and only when the hidden
bit is clear. A slot neither visible nor hidden becomes visible and joins the list. A new rank below 2 sets the at-war bit;
a rank that rose clears it. A changed rank, unless hidden, logs the standing change (not for a row just added) and notes a
rise. After the entries the message re-sorts if a flag changed, rebuilds the quest list and refreshes the player's reaction
if a rank changed, adds hard-coded effect 13 to the player after a rise, raises tutorial 0xf for a player of level 10 or
more, and signals `UPDATE_FACTION`.

Addresses: `0x4d5760`, `0x4d5795`, `0x4d57b7`, `0x4d57c2`, `0x4d57e3`, `0x4d5804`, `0x4d5815`, `0x4d5826`, `0x4d5833`, `0x4d584c`, `0x4d585f`, `0x4d5864`, `0x4d587c`, `0x4d5880`, `0x4d5896`, `0x4d589b`, `0x4d58c8`, `0x4d58f3`, `0x4d5905`, `0x4d5926`, `0x4d5931`, `0x4d5957`, `0x4d595e`, `0x4d5975`, `0x4d597a`, `0x4d597f`, `0x4d5984`, `0x62c670`, `0x62c5f0`, `0x4de510`, `0x606e20`, `0x61f490`, `0x61fae0`, `0x6210e0`, `0x4b5390`
Confidence: verified
Proof: one
Test: none

The notify argument of the standing store is 1 only when the entry index is 0 and bit 2 is clear (`0x4d57d2` to `0x4d57e8`).
The visible test is `test al,5` (`0x4d5815`): neither the visible nor the hidden bit set. The war rules are `cmp esi,2; jge`
(`0x4d5833`) and `cmp esi,[ebp-0x10]; jle` (`0x4d583f`), signed. The standing log is `0x62c670(faction, rank)` (`0x4d5896`), skipped
when the adder reported a new row. The war-change latch at `0x4d587c` is unreachable from the flag leg: `0x4d586a` re-reads the
byte just stored, so the xor with it is 0 and `test dl,2; je` always skips; only a rank change sets that latch (`0x4d58ac`).
The tutorial call is `0x4b5390(0xf, 0)` (`0x4d5975`, `0x4d597a`) after the level test `cmp [ecx+0x70],0xa`.

## Visible and at-war messages

The visible message adds the slot's row (notifying) when its visible bit is clear, sets the bit and re-sorts; with the
bit set it does nothing and signals nothing. The at-war message copies bit 1 of its flags into the slot's at-war bit and
signals `UPDATE_FACTION`; it does not sort and does not read the other bits of the wire byte.

Addresses: `0x4d5710`, `0x4d5728`, `0x4d5731`, `0x4d5738`, `0x4d5745`, `0x4d574a`, `0x4d56b0`, `0x4d56db`, `0x4d56e4`, `0x4d56e9`, `0x4d56ec`, `0x4d56f1`, `0x4d56f6`
Confidence: verified
Proof: one
Test: none

The visible handler tests bit 0 (`0x4d572e`), calls the adder with notify 1 (`0x4d5738`), stores the flag with bit 0 set
(`0x4d5745`) and calls the sorter (`0x4d574a`), which signals. The at-war handler tests `al,2` (`0x4d56d9`) against the
wire flags and sets or clears bit 1 of the stored byte (`0x4d56e4`, `0x4d56e9`).

## Faction requests

`SetFactionAtWar` refuses without a message when asked for peace with the standing below -3000. For a faction in no slot
it does nothing. In combat (unit flags bit 19) it shows `ERR_NOT_IN_COMBAT` and sends nothing. Otherwise it sets the slot's
at-war bit at once, except that a peace-forced slot is cleared even when war is asked, and sends `CMSG_SET_FACTION_ATWAR`
with the slot and the requested state. `SetFactionInactiveFlag` sets or clears the inactive bit at once, sends
`CMSG_SET_FACTION_INACTIVE` and re-sorts. `SendWatchedFaction` sends `CMSG_SET_WATCHED_FACTION` when the slot differs from the
player's field, and writes no field.

Addresses: `0x4d5fd0`, `0x4d5fd9`, `0x4d5fe3`, `0x4d5fe8`, `0x4d5ff5`, `0x4d6023`, `0x4d6032`, `0x4d6040`, `0x4d6045`, `0x4d6065`, `0x4d606a`, `0x4d606f`, `0x4d6074`, `0x4d6079`, `0x4d6094`, `0x4d60a0`, `0x4d60b3`, `0x4d60be`, `0x4d60f0`, `0x4d6122`, `0x4d6126`, `0x4d6131`, `0x4d6158`, `0x4d616b`, `0x4d6176`, `0x4d617b`, `0x4d6240`, `0x4d627e`, `0x4d62a5`, `0x4d62b4`, `0x496720`
Confidence: verified
Proof: one
Test: none

The floor test runs only when the requested state is peace (`0x4d5fd9 cmp dl,bl` then `0x4d5fe8 cmp eax,0xfffff448; jl`). The combat
test reads the unit flags word (`0x4d6032`) and bit 19 (`0x4d6038 shr ecx,0x13`); the message id is 0x1a1 (`0x4d6040`). The local
bit is set when war is asked and bit 4 is clear (`0x4d6065`, `0x4d606a`), otherwise cleared (`0x4d606f`). The packets are the
opcode (`PutUInt32`), the slot, and a byte from `setne` (`0x4d60b3`, `0x4d6164`).

## Faction events

`UPDATE_FACTION` (372) is signalled from four places and no other: the end of the initialize message after the rebuild,
the at-war message, the end of the standing message, and the sorter, which every rebuild, visible message, collapse,
expand, inactive change and flag-changing standing message reaches. The player's watched-faction field has a field
watcher that signals it too (`objectlayer/playerfields#watchers-on-every-player`).

Addresses: `0x4d5699`, `0x4d56f1`, `0x4d5984`, `0x4d5dab`, `0x703e50`, `0x51b437`
Confidence: verified
Proof: one
Test: none

Each site loads 0x174 into `ecx` and calls `0x703e50`. The event name is stored in the table at index 0x174 by the
static initializer at `0x51b437`.
