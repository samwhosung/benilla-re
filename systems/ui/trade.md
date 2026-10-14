# ui: trade requests and status messages

What the interface does with the trade statuses the server sends: the automatic answer to a trade request, the
messages each status prints, the window and accept-flag changes, and the Lua functions that start a trade. The wire
messages and the trade state are `objectlayer/trade`; the status handler `0x5d47c0` calls the dispatcher `0x4bf720`
once per status, after it has updated that state.

Roots: `0x4bf720`, `0x4bf4e0`, `0x4bf230`, `0x4bf280`, `0x4bfbd0`, `0x4bfcf0`, `0x4bfd70`, `0x4bfdc0`, `0x48a120`, `0x48aa60`, `0x48aa70`

## Answering a trade request

A request arrives as status 1, with the requester's GUID (u64) stored by `0x5d47c0`. The dispatcher answers at once,
with no step that asks the player. It takes the first row of the table that applies.

| # | test | answer |
|---|---|---|
| 1 | a trade this client asked for is pending | nothing sent |
| 2 | the requester is on the ignore list | `CMSG_IGNORE_TRADE` (0x119) |
| 3 | the requester's player object is not known | nothing sent |
| 4 | the requester is dead or a ghost | `CMSG_BUSY_TRADE` (0x118) |
| 5 | a cinematic is playing | busy |
| 6 | the player does not control the character | busy |
| 7 | an auctioneer window is open | busy |
| 8 | the `BlockTrades` CVar is nonzero | busy, then `ERR_TRADE_BLOCKED_S` with the requester's name |
| 9 | none of the above | `CMSG_BEGIN_TRADE` (0x117) |

Addresses: `0x4bf720`, `0x4bf736`, `0x4bf73b`, `0x4bf743`, `0x4bf750`, `0x4bf759`, `0x4bf779`, `0x4bf78a`, `0x4bf793`, `0x4bf79c`, `0x4bf7a5`, `0x4bf7b2`, `0x4bf7bc`, `0x4bf7c1`, `0x4bf7c6`, `0x4bf7cd`, `0x4bf7d6`, `0x4bf7e1`, `0x4bf7ee`, `0x5d3f60`, `0x5d3f70`, `0x5ae5a0`, `0x5d41c0`, `0x468460`, `0x605f30`, `0x5d4150`, `0x5d40e0`, `0x609210`, `0x63de30`, `0x496720`, `0xb4e310`, `0xb4b3e4`, `0xb725f8`
Confidence: verified
Proof: one
Test: none

The tests read the pending flag `0xc4bec8` (`0x5d3f70`), the ignore list through `0x5ae5a0`, the requester's player
object by GUID (`0x468460`, type mask 0x10), `0x605f30`, the cinematic flag `0xb4e310`, the control flag `0xb4b3e4`
and the auctioneer GUID `0xb725f8` (open while nonzero). The arm reads none of the dispatcher's arguments, stores
nothing and signals no event. `0x609210` names the requester from the name cache and may query the server on a miss.
A request is therefore also dropped while a trade this client started is open: the pending flag clears only on the
statuses of `objectlayer/trade#trade-status-message`.

## The request events

`TRADE_REQUEST` (285) is not signalled by any code, so a request never reaches the interface as an event. Status 3
signals `TRADE_REQUEST_CANCEL` (286), once, at `0x4bf832`. The message `ERR_TRADE_REQUEST_S` (186) has a catalog row and
no caller; the row for `ERR_INITIATE_TRADE_S` (185) is shown by the sender of a request, not by a status.

Addresses: `0x4bf82d`, `0x4bf832`, `0x703e50`, `0x703f50`, `0x496720`
Confidence: inferred
Proof: none
Test: none

Of the 336 calls and jumps to the signal helper `0x703e50`, 332 load an immediate id into `ecx` and none is 285; the other
four add a flag (0 or 1) to a base of 0x18d, 0xcb or 0x114 (`0x49211a`, `0x49495f`, `0x49630d`, `0x5aaf17`). Of the 149
sites of `0x703f50`, 144 push an immediate; the rest take the glue id 4, the message route (0xe0 or 0xe1), the chat
type table (ids 228 to 254, 358 and 441 to 474), the unit-event wrapper `0x515e50` (immediate ids and power-type
offsets) and the field reflex `0x5de4b0` (registered for 0x196, 0xbd, 0x95, 0xc9, 0x20b, 0x20c and 0x174). The
descriptor reflex `0x51bd50` passes a field index and is not traced. Of the 403 calls of the message display `0x496720`, 368 push an
immediate id and none is 186; the other 35 are not traced.

## BlockTrades

`BlockTrades` is a CVar registered at interface start with category 5, default `"0"`, no flags and no callback. Only the
request arm reads it, by name through `CVar::Lookup`, as the integer at record `+0x28`, and it does not test the
lookup for null. Any nonzero value refuses every request that reaches step 8 of the table above. The Lua accessors
`GetCVar` and `SetCVar` reach the same record.

Addresses: `0x48fdef`, `0x48fdf4`, `0x63db90`, `0x63de30`, `0x4bf7c6`, `0x4bf7cb`
Confidence: verified
Proof: one
Test: none

The registration pushes, last argument first: flags 0, the default `"0"` (`0x48fde7`), callback 0, category 5
(`0x48fde4`, stored at record `+0x18`, `0x63dd42`), a zero byte argument and userdata 0, with the help text null
(`0x48fded xor edx,edx`). The name is used twice in the image: here and at the read `0x4bf7bc`. The record layout,
`+0x24` as float and `+0x28` as integer, is `uicamera/cvars`.

## Status messages

The dispatcher prints one message for these statuses. A status above 22, and the statuses 1 (apart from step 8 above),
2, 4, 7, 9 and 13, print nothing. The route is how `0x496720` shows the line (`ui/inventory#failure-messages`).

| status | id and key | route |
|---|---|---|
| 0, 5 | 0x3e `ERR_PLAYER_BUSY_S` with the partner's name | chat frame |
| 14 | 0x13d `ERR_IGNORING_YOU_S` with the partner's name | chat frame |
| 3 | 0xbe `ERR_TRADE_CANCELLED` | information |
| 8 | 0xbf `ERR_TRADE_COMPLETE` | information |
| 6 | 0xb8 `ERR_GENERIC_NO_TARGET` | error |
| 10 | 0xbd `ERR_TRADE_TOO_FAR` | error |
| 11 | 0xff `ERR_PLAYER_WRONG_FACTION` | error |
| 15 to 21 | 0x191 `ERR_GENERIC_STUNNED`, 0x192 `ERR_TARGET_STUNNED`, 0x7e `ERR_PLAYER_DEAD`, 0xbc `ERR_TRADE_TARGET_DEAD`, 0x19d `ERR_LOGGING_OUT`, 0x19e `ERR_TARGET_LOGGING_OUT`, 0x1be `ERR_RESTRICTED_ACCOUNT` | error |
| 22 | 0x1ca `ERR_TRADE_WRONG_REALM` | error |

Addresses: `0x4bf847`, `0x4bf871`, `0x4bf8a6`, `0x4bf8b8`, `0x4bf8ca`, `0x4bf8dc`, `0x4bf928`, `0x4bf971`, `0x4bf983`, `0x4bf995`, `0x4bf9a4`, `0x4bf9b6`, `0x4bf9c8`, `0x4bf9da`, `0x4bf9ec`, `0x484ca0`
Confidence: verified
Proof: one
Test: none

The catalog row of each id is built by the static initializer `0x484ca0`, whose registers hold constants for the whole
run of rows: `ecx` 0, `eax` 2 and `edx` 1 after `0x486235`. The route word of 0x3e and 0x13d is stored from `ecx`, of
0xbe and 0xbf from `edx`, and of 0xb8, 0xbc, 0xbd, 0x7e and 0xff from `eax`; the rest use the immediate 2. Row 0xb8
stores 0x2d in its `+0xc` word, a vocal sound id, where the others store `ebx` (0x44, the named-cue value).
`0x4bf8dc` and `0x4bf928` name the partner and are the only arms that push a second argument.

## Names in refusal messages

Statuses 0, 5 and 14 print only while this client has a request of its own pending (`0xc4bec8`) and the GUID in
`0xc4bed0` still names a player the object manager knows. Either failure prints nothing and does not retry. The name is
the live unit's, so it is the player this client asked to trade with. Only `0x5d3fb0` sets the pending flag (`0x5d4021`), so on a
player who only received a request these arms print nothing.

Addresses: `0x4bf8dc`, `0x4bf8e1`, `0x4bf8e3`, `0x4bf8ee`, `0x4bf904`, `0x4bf906`, `0x4bf910`, `0x4bf91b`, `0x4bf928`, `0x4bf92d`, `0x4bf92f`, `0x4bf93a`, `0x4bf950`, `0x4bf95c`, `0x4bf964`, `0x5d3f60`, `0x5d3f70`, `0x5d3fb0`, `0x5d4021`, `0x5d4923`
Confidence: verified
Proof: one
Test: none

`0x5d3fb0` stores the target's GUID in `0xc4bed0` and sets the pending flag (`0x5d4010` to `0x5d4021`, in
`objectlayer/trade#initiating-a-trade`); the dispatcher runs before the status handler clears either (`0x5d4923`, then
`0x5d4931`). The guards are `0x4bf92d test eax,eax; je 0x4bfa02` and `0x4bf950 test eax,eax; je 0x4bfa02`, the bare
return; the arms differ only in id and in the line argument pushed first (`0x13c` and `0x145`).

## Opening and closing the window

`0x4bf4e0(done, partner GUID)` is the one place the window changes. A nonzero GUID makes the partner the interaction
object, stores the GUID, clears the seven offer slots, the accept flags and the money cells, offers what the cursor holds
when the trade was started by a drop, and signals `TRADE_SHOW` (0x13f). A zero GUID with a partner
stored stores `done`, ends the interaction, clears the partner and signals `TRADE_CLOSED` (0x140). It then sends
`CMSG_CANCEL_TRADE` (0x11c) whether or not a partner was stored. When the partner is still the interaction target the close runs twice (below). Statuses 3, 8 and 12 close, and status 8 passes
`done` 1; 2 opens. `CloseTrade()` first signals `PLAYER_TRADE_MONEY` (0x146) when `done` is 0, then closes.

Addresses: `0x4bf4e0`, `0x4bf4ef`, `0x4bf4fd`, `0x4bf50a`, `0x4bf522`, `0x4bf527`, `0x4bf533`, `0x4bf53e`, `0x4bf549`, `0x4bf5ba`, `0x4bf5f1`, `0x4bf644`, `0x4bf705`, `0x4bf7f8`, `0x4bf801`, `0x4bf842`, `0x4bf862`, `0x4bf89a`, `0x4bfdc0`, `0x4bfdc5`, `0x4bfdce`, `0x4bfdd9`, `0x4931c0`, `0x4930d0`, `0x5d3f80`, `0x5d4310`, `0xb71728`, `0xb71748`, `0xb716d0`
Confidence: verified
Proof: one
Test: none

The double close: Ending the interaction (`0x4bf50a`, `0x4931c0`)
runs the dispatcher `0x493310` when the partner is still the interaction target; it zeroes the target and calls
`0x4bf4e0(0, 0)` again while the partner GUID is still set, so the close runs twice: `TRADE_CLOSED` fires twice,
`CMSG_CANCEL_TRADE` is sent twice, and the inner call stores `done` 0 (`0x4bf4fd`, `ecx` 0 from `0x493363`) over the
outer value, so the `done` 1 of status 8 never survives.

The partner pair `0xb71728` is a different pair from `objectlayer/trade`'s `0xc4bed0`. The statuses that clear the
network state but not the window (0, 5, 6, 10, 11, 13 and 14 to 21) leave the window on screen until a script closes it.
`0x4bf7f8` passes the GUID from `0x5d3f60` with `ecx` 0. The clears at `0x4bf558` to `0x4bf5ef` zero three dword arrays of seven, the byte array at the start of the offer block and two GUID
arrays of seven, one at `0xb716d0`. `0x4bfcf0` unlocks each item
GUID that is nonzero without zeroing it; it runs before the close in statuses 3, 8 and 12.

## Accept flags

Two flags hold the accept state: the player's (`0xb71730`) and the partner's (`0xb71734`). `0x4bf230(flag)` and
`0x4bf280(flag)` set them and signal `TRADE_ACCEPT_UPDATE` (0x142) with both flags, player first, only when the flag
changes. Status 4 sets the partner's flag; status 9 clears it. Status 7 clears the player's, then the partner's, so it
can signal twice. A change of the player's flag also sends `CMSG_ACCEPT_TRADE` (0x11a) or `CMSG_UNACCEPT_TRADE`
(0x11b); `AcceptTrade()` and `CancelTradeAccept()` set it, and a new partner offer clears both flags.

Addresses: `0x4bf230`, `0x4bf233`, `0x4bf238`, `0x4bf241`, `0x4bf260`, `0x4bf26d`, `0x4bf272`, `0x4bf280`, `0x4bf286`, `0x4bf299`, `0x4bf80b`, `0x4bf81a`, `0x4bf821`, `0x4bf81c`, `0x4bf823`, `0x4bf810`, `0x4c06e0`, `0x4c06e5`, `0x4c06f0`, `0x4c06f2`, `0x4bf2b0`, `0x4bf2b6`, `0x4bf2bd`, `0x494a50`, `0x703f50`, `0xb71730`, `0xb71734`
Confidence: verified
Proof: one
Test: none

Case 7 is `xor ecx,ecx; call 0x4bf230` and then runs into case 9's entry `0x4bf821`, which is `xor ecx,ecx; call
0x4bf280`. `0x4bf230` calls `0x494a50` with tag 5 first (`0x4bf233`); that gate returns 1 for tag 5 on every path, so
it never blocks. The format is `%d%d`; `0x4bf230` pushes the partner's flag then its own, so the
arguments arrive player first. Both functions return early when the flag already has the value.

## A refused offer slot

Status 22 reads a trailing byte, prints `ERR_TRADE_WRONG_REALM` and calls `0x4bfbd0(byte)`. The byte 0xff zeroes the
offered money, signals `PLAYER_TRADE_MONEY` (0x146) and `PLAYER_MONEY` (0xc8). Any other byte is a slot: its item is
unlocked, its record zeroed and `TRADE_PLAYER_ITEM_CHANGED` (0x144) is signalled with the 1-based slot. The window and
network state stay as they are. The byte is not bounded, so a value from 7 to 0xfe writes past the arrays.

Addresses: `0x4bf9ec`, `0x4bf9f6`, `0x4bf9fd`, `0x4bfbd0`, `0x4bfbd3`, `0x4bfbe0`, `0x4bfbea`, `0x4bfbf5`, `0x4bfc0a`, `0x4bfc11`, `0x4bfc2d`, `0x4bfc3f`, `0x495420`, `0xb71740`, `0xb716d0`
Confidence: verified
Proof: one
Test: none

`0x4bfbd0` is also called for the slots of the extended status message (`0x5d4ad3`, `0x5d4b33`). Only the low byte of the
status's trailing dword is used (`0x4bf9f6 movzx ecx,byte ptr [ebp+0x10]`). The only bound is the compare with 0xff
(`0x4bfbd3`).

## Inventory refusals

Status 12 (the window closes) prints a message chosen by the inventory result and a wire byte: `0x4bfd70` maps result
4 (bag full), 0x11 (too many) and 0x1c (reagent) itself and passes every other result to the inventory-result table
`0x622630`, 61 distinct message ids in all. The id 0x1d1 shows nothing. For the three mapped results the byte picks
between a line about the player and one about the partner.

| result | byte 0 | byte 1 |
|---|---|---|
| 4 | 0xc1 `ERR_TRADE_TARGET_BAG_FULL` | 0xc0 `ERR_TRADE_BAG_FULL` |
| 0x11 | 0xc2 `ERR_TRADE_MAX_COUNT_EXCEEDED` | 0xc3 `ERR_TRADE_TARGET_MAX_COUNT_EXCEEDED` |
| 0x1c | 0xd3 `ERR_SPELL_FAILED_REAGENTS_GENERIC` | 0x1d1 (nothing) |

Addresses: `0x4bf87e`, `0x4bf883`, `0x4bf888`, `0x4bf88d`, `0x4bfd70`, `0x4bfd73`, `0x4bfd7f`, `0x4bfd84`, `0x4bfd97`, `0x4bfda4`, `0x622630`, `0x496720`
Confidence: verified
Proof: one
Test: none

The byte is the second field of the status, parsed as 0 or 1 by `0x5d490a`. The sense of the byte is opposite for
results 4 and 0x11: `0x4bfda4` yields the partner's line for 0 and `0x4bfd97` the player's line for 0. A message
that takes an argument, such as the level message of inventory result 1 (id 2), gets none from this arm and reads
the word after the id on the stack, which is the status handler's flag 1.

## Starting a trade

`InitiateTrade("unit")` raises `Usage: InitiateTrade("unit")` for a non-string. A token that names no unit does
nothing. If the active player's own object is dead or a ghost it shows `ERR_PLAYER_DEAD` and sends nothing;
otherwise it calls `0x5d3fb0` with mode 0. Nothing tests the target. `BeginTrade()` and `CancelTrade()` take no
arguments and send `0x117` and `0x11c`. The other starters of `0x5d3fb0` are `DropItemOnUnit` and the world-click
drop, with mode 1. `0x117` has two callers (the script function and step 9 above), `0x118` two (steps 4 to 8), `0x119`
one (step 2).

Addresses: `0x48a120`, `0x48a12e`, `0x48a14b`, `0x48a16e`, `0x48a177`, `0x48a186`, `0x48a18a`, `0x48a19a`, `0x48aa60`, `0x48aa70`, `0x48da62`, `0x49278e`, `0x5d3fb0`, `0x5d40e0`, `0x5d4150`, `0x5d41c0`, `0x5d4310`, `0x490be4`, `0x4bf220`, `0x4bf527`
Confidence: verified
Proof: one
Test: none

`0x48a16e` asks `0x605f30` about the player found from the active player's GUID (`0x48a152`), not about the target,
which `0x515970` resolves and passes on untouched. The callers of `0x5d4310` (`0x11c`) are the script function,
logout (`0x490be4`), the leave-world thunk (`0x4bf220`) and the close path (`0x4bf527`). `0x5d4230` and `0x5d42a0` are
reached only through the accept flags.
