# ui: the auction house session

The client's auction house: its state, how a window opens and closes, the packets it sends and the list results it
reads. This is `AuctionHouse.cpp` (`0x4cc0f0` to `0x4d0480`). The Lua reads, the sort, the deposit and the sell
slot are `ui/auctionlists`; command results, notifications and message text are `ui/auctionmessages`. The
interaction ladder that sends the first request is `objectlayer/interaction#talking-to-an-npc`.

Roots: `0x4cc0f0`, `0x4cc280`, `0x4cc300`, `0x4cc420`, `0x4cc460`, `0x4cc7f0`, `0x4cca30`, `0x4ccc80`, `0x4cced0`, `0x4cd1f0`, `0x4cd480`, `0x4d03e0`

## Lua functions

`0x4d03e0` registers 23 functions from the pair table `0x8490a8` (name, then function) and `0x4d0410` removes them:
`CloseAuctionHouse`, `GetAuctionHouseDepositRate`, `CalculateAuctionDeposit`, `ClickAuctionSellItemButton`,
`GetAuctionSellItemInfo`, `StartAuction`, `QueryAuctionItems`, `GetOwnerAuctionItems`, `GetBidderAuctionItems`,
`GetNumAuctionItems`, `GetAuctionItemInfo`, `GetAuctionItemLink`, `GetAuctionItemTimeLeft`, `PlaceAuctionBid`,
`GetAuctionItemClasses`, `GetAuctionItemSubClasses`, `GetAuctionInvTypes`, `CanSendAuctionQuery`,
`SortAuctionItems`, `SetSelectedAuctionItem`, `GetSelectedAuctionItem`, `IsAuctionSortReversed` and `CancelAuction`.
`SetAuctionItem`, `SetAuctionSellItem` and `SetAuctionCompareItem` are tooltip methods (`ui/tooltip`). None of
the functions is called from native code; they run from Lua alone.

Addresses: `0x4d03e0`, `0x4d0410`, `0x8490a8`, `0x4ce160`, `0x4ce770`, `0x4ce980`, `0x4cec80`, `0x4cecd0`, `0x4cf610`, `0x4d0260`
Confidence: verified
Proof: one
Test: none

`0x4ce160` is `call 0x4cd5f0; xor eax,eax; ret`; it begins with a `call`, so it is found only through the table. A
whole-image census of `call` and `jmp` targets and of stored dwords finds, for each of `0x4ce770`, `0x4ce980`,
`0x4cf610`, `0x4d0260`, `0x4cec80` and `0x4cecd0`, no caller and exactly one stored address, its table entry.
`0x4d03e0` is called from `0x49035a` and `0x4d0410` from `0x490de9`.

## Session state

A reply opens the window and sets the state: the auctioneer GUID (`0xb725f8`) and the house id (`0xb72600`). Three
arrays of 50 record pointers hold the lists: browse `0xb72278`, owner `0xb72340` and bidder `0xb72470`, each with
a count and a total from the wire (`0xb7261c` and `0xb72620`, `0xb72624` and `0xb72628`, `0xb7262c` and
`0xb72630`) and a pending-lookups counter (`0xb72644`, `0xb72648`, `0xb7264c`). The 150 records are 0x48 bytes,
allocated once at start-up (`0x4cc0f0`); sorting permutes the pointers. Each list has a sort key stack (browse
`0xb725b0`, owner `0xb72420`, bidder `0xb72548`) of 8 pairs {column, reversed}. `0x4cc280` clears the counts, the
GUID, the house and the flags, and the sell slot.

Addresses: `0x4cc0f0`, `0x4cc280`, `0x4cc300`, `0x4cc195`, `0x4cc1ad`, `0x4cc1b8`, `0x4cc1c1`, `0x4cc238`, `0xb725f8`, `0xb72600`, `0xb72278`, `0xb72340`, `0xb72470`, `0xb7261c`, `0xb72620`, `0xb72624`, `0xb72628`, `0xb7262c`, `0xb72630`, `0xb72644`, `0xb72648`, `0xb7264c`, `0xb725b0`, `0xb72420`, `0xb72548`
Confidence: verified
Proof: one
Test: none

The record allocations are `0x6462e0(0x48, <AuctionHouse.cpp>, line, 0)`, three per loop pass for 50 slots
(`0x4cc1c1 cmp esi,0xc8`). The browse stack starts as columns 0 to 7 in order (`0x4cc246`); the owner and bidder
stacks as 5, 3, 2, 0, 6, 1, 4, 7 from the twin tables `0x849064` and `0x849084`; every reversed flag starts 0
(`0x4cc238` to `0x4cc274`). `0x4cc280` is called once, from `0x490a05`; `0x4cc0f0` once, from `0x48f5f9`.

## Auction records

A record is 0x48 bytes. The house id at +0 is not on the wire; the browse, owner and bidder results fill the rest in
the order of the packet. The time left arrives as a duration in milliseconds and is stored as an absolute deadline.

| offset | field |
|---|---|
| +0x00 | house id, copied from `0xb72600` |
| +0x04 | auction id |
| +0x08 | item entry |
| +0x0c, +0x10, +0x14 | enchantment id, random property id, suffix factor |
| +0x18 | stack count |
| +0x1c | charges (read, used by nothing here) |
| +0x20 | owner GUID (u64) |
| +0x28, +0x2c, +0x30 | start bid, minimum bid increment, buyout |
| +0x34 | deadline: the tick when parsed plus the wire milliseconds |
| +0x38 | high bidder GUID (u64) |
| +0x40 | current bid |

Addresses: `0x4cc7f0`, `0x4cc86a`, `0x4cc880`, `0x4cc966`, `0x4cc975`, `0x42c010`
Confidence: verified
Proof: one
Test: none

The reader order is `0x418e30` for the u32 fields (`+4` to `+0x1c`), `0x4190b0` for the owner (`+0x20`), `0x418eb0`
for `+0x28`, `+0x2c`, `+0x30` and a time in a local, `0x4190b0` for the bidder (`+0x38`) and `0x418eb0` for `+0x40`
(`0x4cc89b` to `0x4cc997`). `0x4cc966` to `0x4cc975` adds the local to the tick sampled once before the loop
(`0x4cc865`), the same millisecond tick `0x42c010` that mail and the query throttle use.

## Browse results

`SMSG_AUCTION_LIST_RESULT` (0x25c) is `0x4cc7f0`: a u32 row count (into `0xb7261c`), the rows of the record layout
above, then a u32 total (`0xb72620`). The count is never compared with the 50 slots, so a count above 50 runs off the
browse pointer array `0xb72278` (200 bytes): rows 51 to 100 are written through the owner pointers at `0xb72340`, so they
overwrite the owner records. A previous request's unresolved item lookups are cancelled first. For each row the item record is
requested from the item cache; each miss adds one to `0xb72644`. After the total, the list is sorted and signalled
(`0x4cd8b0`) only when that counter is 0, else the last arriving lookup does it (`0x4cca10`).

Addresses: `0x4cc7f0`, `0x4cc806`, `0x4cc847`, `0x4cc859`, `0x4cc9ae`, `0x4cc9c8`, `0x4cc9dc`, `0x4cc9ce`, `0x4cc9f1`, `0x4cca10`, `0x4cd8b0`, `0x55ba30`, `0x55c0c0`
Confidence: verified
Proof: one
Test: none

`0x4cc806` to `0x4cc84c` run only when `0xb72644` is nonzero: they clear it and call `0x55c0c0` for each old row's entry
with the callback `0x4cca10`. The loop bound is the count alone (`0x4cc9ce` to `0x4cc9d6`). `0x4cc9c4` tests the
result of `0x55ba30(cache, entry, &out, callback, 0, 0)` and `0x4cc9c8` increments on a miss; `0x4cca10` decrements
and calls `0x4cd8b0` at 0 (`0x4cca21`).

## Owner and bidder results

`SMSG_AUCTION_OWNER_LIST_RESULT` (0x25d, `0x4cca30`) and `SMSG_AUCTION_BIDDER_LIST_RESULT` (0x265, `0x4ccc80`) read the
same rows into their own array, count, total, pending counter and sort tail (`0x4cdcd0`, `0x4cddc0`). Unlike the
browse result, each compares the count with the total after reading and, when they differ, sets the list's one-shot
request flag (`0xb7263c` owner, `0xb72640` bidder) to 1.

Addresses: `0x4cca30`, `0x4ccc2d`, `0x4ccc35`, `0x4ccc80`, `0x4ccc60`, `0x4cce85`, `0x4cceb0`, `0x4cdcd0`, `0x4cddc0`, `0xb7263c`, `0xb72640`, `0xb72648`, `0xb7264c`
Confidence: verified
Proof: one
Test: none

`0x4ccc2d cmp eax,[0xb72628]; je` jumps over the store `0x4ccc35 mov [0xb7263c],1`; the bidder handler repeats it at
`0x4cce85`. The arrival callbacks `0x4ccc60` and `0x4cceb0` count down like `0x4cca10` and call the owner and
bidder sort tails. The browse handler has no such comparison.

## Opening and closing

`MSG_AUCTION_HELLO` (0x255) from the server is `0x4cc420`: a u64 auctioneer GUID, then a u32 house id; later bytes are
not read. `0x4cd570` makes the auctioneer the interaction NPC with the walk-away range (`0xb72410`, 5.5555553 yards
squared), stores the GUID and house, zeroes the query throttle, sets both one-shot flags to 1 and clears all counts and
saved page offsets, clears the sell slot, and signals `AUCTION_HOUSE_SHOW` (421). `0x4cd5f0` does nothing with no
GUID; otherwise it signals `AUCTION_HOUSE_CLOSED` (422) first, ends the interaction, zeroes the GUID, house and both
flags, and clears the sell slot. Closing sends nothing and leaves the throttle as it was.

Addresses: `0x4cc420`, `0x4cc43b`, `0x4cd570`, `0x4cd57f`, `0x4cd5a3`, `0x4cd5e1`, `0x4cd5f0`, `0x4cd5fd`, `0x4cd609`, `0x4cd62e`, `0x4930d0`, `0x4931c0`, `0x4d04f0`, `0xb72410`, `0x4cc2f0`, `0x4934d7`
Confidence: verified
Proof: one
Test: none

`0x4d04f0` is a static initializer: `fld [0x806fe8]; fmul [0x806fe8]; fstp [0xb72410]`, with `0x806fe8` the float
5.5555553. `0x4930d0` copies it into the live gate that drops the interaction when the player walks away, which calls
`0x4cd5f0` as slot 12 of its cascade (`0x4934d7`). The other callers of `0x4cd5f0` are the thunk `0x4cc2f0`, reached
from the reset `0x4908c0`, and `0x4ce160`. Neither `0x4cd5f0` nor the interaction clear it calls builds a packet.

## Requests the client sends

Each request is a packet whose first field is the auctioneer GUID (u64, not packed); strings carry their NUL.

| opcode | sent by | body after the GUID |
|---|---|---|
| 0x256 | `StartAuction` | item GUID u64, minimum bid u32, buyout u32, duration u32 |
| 0x257 | `CancelAuction` | auction id u32 |
| 0x258 | `QueryAuctionItems` | list offset u32, text, level min u8, level max u8, inventory type u32, class u32, subclass u32, quality u32, usable u8 |
| 0x259 | `GetOwnerAuctionItems` | list offset u32 |
| 0x25a | `PlaceAuctionBid` | auction id u32, bid u32 |
| 0x264 | `GetBidderAuctionItems` | list offset u32, count u32, then count auction ids u32 |

Addresses: `0x4ce925`, `0x4d03a6`, `0x4cec3d`, `0x4cd6e0`, `0x4cf915`, `0x4cd7fd`, `0x4cd680`, `0x4cd720`, `0x4cd7f2`, `0x417e10`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

The writers are `0x418190` and `0x4181f0` (u32), `0x418070` (u8), `0x418370` (u64) and `0x418430` (string); the
send is `0x5ab630`. `0x4cd720` writes the count as 0, then appends the id of every outbid-list entry whose house is
the open one, and patches the count with `0x417e10` (`0x4cd7f2`). The list offset is the page times 50 (`imul
reg,reg,0x32`: `0x4ceb44`, `0x4cecbf`, `0x4ced0f`). No sort order is sent. The calls `0x4cd680` and `0x4cd720` send
only while their one-shot flag is set and clear it afterwards, saving the offset (`0xb72650`, `0xb72654`).

## The query throttle

`CanSendAuctionQuery()` returns the number 1 when `0xb72638` is 0 or the tick has reached it, else nil, and signals
nothing. `QueryAuctionItems` repeats the test and silently returns when it fails, or with no auctioneer. After
sending it sets `0xb72638` to the tick plus 5000 ms. Open sets it to 0; close leaves it. The owner and bidder
requests are not throttled; they are gated by the one-shot flags instead, which are set by open, by the owner and
bidder results when the count differs from the total, and by a successful start or bid (`ui/auctionmessages`),
and cleared by close and by the send.

Addresses: `0x4cfbb0`, `0x4cec80`, `0x4cecd0`, `0x4cfbc1`, `0x4ce9a2`, `0x4ce9b0`, `0x4cec42`, `0x4cec4c`, `0x4cec90`, `0x4cece0`, `0x4cd686`, `0x4cd6e9`, `0x4cd726`, `0x4cd814`, `0xb72638`, `0xb7263c`, `0xb72640`
Confidence: verified
Proof: one
Test: none

`0x4cfbb0 mov eax,[0xb72638]; test; je` allows a never-armed cell, then `0x42c010`, `sub`, `setns`. `0x4cec42` to
`0x4cec4c` is `call 0x42c010; add eax,0x1388; mov [0xb72638],eax`. The cell is written at `0x4cc2b4` (reset), `0x4cd5a3`
(open) and `0x4cec4c`. The binding gates are `0x4cec90` and `0x4cece0`, and the sender gates `0x4cd686` and `0x4cd726`
re-read the flag.

## Outbid list

Up to eight {house id, auction id} pairs (growing by doubling) are kept in `0xb72594` with the count `0xb72634`. A
bidder notification for an outbid auction appends its pair (`0x4cde50`), a successful bid removes the auction's pair
(`0x4cdf30`), and the bidder request sends the open house's auction ids and then removes that house's pairs
(`0x4cdf90`).

Addresses: `0x4cde50`, `0x4cdf30`, `0x4cdf90`, `0x4cc1c9`, `0x4cc213`, `0x4cd793`, `0x4cd802`, `0x4ccff3`, `0x4cc517`, `0xb72594`, `0xb72590`, `0xb72634`
Confidence: verified
Proof: one
Test: none

`0x4cc213` allocates eight 8-byte pairs at start-up, `0x4cde62 cmp eax,ebx; jae` finds the list full and
`0x4cde9b lea edi,[ebx+ebx]` doubles the capacity. `0x4cd793` loads the count for the request and `0x4cd7b6` compares
the house with `0xb72600`.
