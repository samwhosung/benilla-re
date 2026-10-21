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
`0xb725b0`, owner `0xb72420`, bidder `0xb72548`) of 8 pairs {column, reversed}.

Addresses: `0x4cc0f0`, `0x4cc280`, `0x4cc300`, `0x4cc195`, `0x4cc1ad`, `0x4cc1b8`, `0x4cc1c1`, `0x4cc238`, `0xb725f8`, `0xb72600`, `0xb72278`, `0xb72340`, `0xb72470`, `0xb7261c`, `0xb72620`, `0xb72624`, `0xb72628`, `0xb7262c`, `0xb72630`, `0xb72644`, `0xb72648`, `0xb7264c`, `0xb725b0`, `0xb72420`, `0xb72548`
Confidence: verified
Proof: one
Test: `ui::outbid_list_start_up_difftest`

The record allocations are `0x6462e0(0x48, <AuctionHouse.cpp>, line, 0)`, three per loop pass for 50 slots
(`0x4cc1c1 cmp esi,0xc8`). The browse stack starts as columns 0 to 7 in order (`0x4cc246`); the owner and bidder
stacks as 5, 3, 2, 0, 6, 1, 4, 7 from the twin tables `0x849064` and `0x849084`; every reversed flag starts 0
(`0x4cc238` to `0x4cc274`). `0x4cc280` is called once, from `0x490a05`; `0x4cc0f0` once, from `0x48f5f9`. The outbid list's part, `0x4cc1c9` to
`0x4cc234`, runs in `ui::outbid_list_start_up_difftest`.

## The session reset

`0x4cc280` clears the sell slot, then zeroes the auctioneer GUID and the house id, the three counts and totals, the query
throttle `0xb72638`, the two request flags `0xb7263c` and `0xb72640` and the three pending counters, and stores -1 in the
three selection cells `0x849058`, `0x84905c` and `0x849060`. It stores nothing else: the outbid count `0xb72634`, the
records, the sort key stacks and the saved page offsets `0xb72650` and `0xb72654` stay as they were. It has one call
site, `0x490a05` in `0x4908c0`.

Addresses: `0x4cc280`, `0x4cc285`, `0x4cc287`, `0x4cc28c`, `0x4cc291`, `0x4cc296`, `0x4cc2af`, `0x4cc2b4`, `0x4cc2b9`, `0x4cc2be`, `0x4cc2c3`, `0x4cc2cd`, `0x4cc2d2`, `0x4cc2d5`, `0x4cc2da`, `0x4cc2df`, `0x4cd640`, `0x490a05`, `0x4908c0`, `0xb72650`, `0xb72654`, `0xb725f8`, `0xb72600`, `0xb7261c`, `0xb72630`, `0xb72634`, `0xb72638`, `0xb7263c`, `0xb72640`, `0xb72644`, `0xb72648`, `0xb7264c`, `0x849058`, `0x84905c`, `0x849060`
Confidence: verified
Proof: one
Test: none

`0x4cc280 call 0x4cd640` clears the sell slot. Then `eax` is 0 (`0x4cc285`) and is stored to `0xb725f8` and `0xb725fc` (the
GUID, `0x4cc287`, `0x4cc28c`), `0xb72600` (`0x4cc291`), the six counts and totals `0xb7261c` to `0xb72630` (`0x4cc296` to
`0x4cc2af`), the throttle (`0x4cc2b4`), the two request flags (`0x4cc2b9`, `0x4cc2be`) and the pending counters `0xb72644`,
`0xb72648` and `0xb7264c` (`0x4cc2c3` to `0x4cc2cd`). `0x4cc2d2 or eax,-1` makes -1, stored at `0x4cc2d5`, `0x4cc2da` and
`0x4cc2df`. The body ends at `ret`, `0x4cc2e4`, with no other store. The call sites of `0x4cc280` in the image are
`0x490a05` alone, inside `0x4908c0`, and the address is stored nowhere (an image scan for the dword and for `e8`/`e9` targets finds that one
call; control: `0x4cc0f0` finds `0x48f5f9`).

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

Each request is a packet whose first field is the auctioneer GUID (u64, not packed); strings carry their NUL. The owner
and bidder requests save the list offset they send in `0xb72650` and `0xb72654`; a started auction sends the owner
request again at its offset, and a placed bid the bidder request at its.

| opcode | sent by | body after the GUID |
|---|---|---|
| 0x256 | `StartAuction` | item GUID u64, minimum bid u32, buyout u32, duration u32 |
| 0x257 | `CancelAuction` | auction id u32 |
| 0x258 | `QueryAuctionItems` | list offset u32, text, level min u8, level max u8, inventory type u32, class u32, subclass u32, quality u32, usable u8 |
| 0x259 | `GetOwnerAuctionItems` | list offset u32 |
| 0x25a | `PlaceAuctionBid` | auction id u32, bid u32 |
| 0x264 | `GetBidderAuctionItems` | list offset u32, count u32, then count auction ids u32 |

Addresses: `0x4ce925`, `0x4d03a6`, `0x4cec3d`, `0x4cd6e0`, `0x4cf915`, `0x4cd7fd`, `0x4cd680`, `0x4cd720`, `0x4cd7f2`, `0x4cd808`, `0x417e10`, `0x5ab630`, `0x4cd6ef`, `0x4cd81a`, `0x4cd5d2`, `0x4cd5d7`, `0x4cc4d1`, `0x4cc51c`, `0xb72650`, `0xb72654`
Confidence: verified
Proof: one
Test: none

The writers are `0x418190` and `0x4181f0` (u32), `0x418070` (u8), `0x418370` (u64) and `0x418430` (string); the
send is `0x5ab630`. `0x4cd720` writes the count as 0, then appends the id of every outbid-list pair whose house is
the open one, patches the count with `0x417e10` (`0x4cd7f2`), sends, and calls `0x4cdf90` once (`0x4cd808`). The list
offset is the page times 50 (`imul reg,reg,0x32`: `0x4ceb44`, `0x4cecbf`, `0x4ced0f`). No sort order is sent. The calls
`0x4cd680` and `0x4cd720` send only while their one-shot flag is set and clear it afterwards; the offset they were
called with is stored after the send, in `0xb72650` (`0x4cd6ef`) and `0xb72654` (`0x4cd81a`). The hello zeroes both
(`0x4cd5d2`, `0x4cd5d7`). The started arm reads `0xb72650` (`0x4cc4d1`) and the bid arm `0xb72654` (`0x4cc51c`) for
the call. An image scan for each dword finds those three sites and no other; the control `0xb72658` finds its two.

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

The list is a pointer array `0xb72594` with `0xb72590` slots (eight after start-up), each pointing at a {house id,
auction id} pair, and a count `0xb72634` of the pairs in use. A bidder notification for an outbid auction appends its
pair (`0x4cde50`). Appending to a full list doubles the slots and writes the pair in the first new slot without raising
the count, so the pair is lost: the next append takes the same slot. A successful bid and a bidder request remove
pairs (next entry).

Addresses: `0x4cde50`, `0x4cde62`, `0x4cde90`, `0x4cde9b`, `0x4cded7`, `0x4cdee1`, `0x4cdf0d`, `0x4cdf1c`, `0x4cdf23`, `0x4cc1c9`, `0x4cc1d0`, `0x4cc207`, `0x4cc213`, `0x4cc413`, `0x4cdf57`, `0x4cdfae`, `0x4ccff3`, `0x4d07d0`, `0x4d0810`, `0x4d0830`, `0x4cd7e6`, `0x4cdf4d`, `0x4cdf7f`, `0x4cdfd4`, `0xb72594`, `0xb72590`, `0xb72634`, `0xb7258c`, `0xb72598`
Confidence: verified
Proof: one
Test: `ui::outbid_list_start_up_difftest`, `ui::outbid_list_later_start_up_difftest`, `ui::outbid_list_doubling_difftest`, `ui::outbid_list_sequence_difftest`

`0x4cc207` stores 8 slots. The count is stored at `0x4cc1d0`, `0x4cc413`,
`0x4cde90`, `0x4cdf57` and `0x4cdfae` only (14 image sites for the dword; the control `0xb72658` finds
its two), and `0x4cde90` raises it from below the slot count, so a full list has a count equal to its slots.
`0x4cde62 cmp eax,ebx; jae 0x4cde9a` finds that, and `0x4cde9b` doubles. `0x4cdea8 jbe` skips the array's growth when
`0xb7258c` holds the doubled count; else `0x4d0810` rounds the count up to a quantum (the cap `0xb72598` if set, else
`0x4d07d0`'s: the largest power of two not above the count, here the count itself, 64 from 64 up, which sets the cap)
and `0x4d0830` raises the array. The teardown (`0x4cc401` to `0x4cc413`) leaves the cap, so a later start-up
(`0x4cc1e1`) sizes the array to 64 and the doublings to 16, 32 and 64 skip the growth. `0x4cded7` stores the slot count, `0x4cdf0d` and `0x4cdf1c` store the pair in slot `ebx`, and `0x4cdf23` returns with no
count store. Of 17 image sites for `0xb72594`, only the sender and the removals read a
pair, below the count (`0x4cd7e6`, `0x4cdf4d`, `0x4cdf7f`, `0x4cdfd4`); `0x4d0830` moves pointers (`[esi+8]`).
Tests run `0x4cde50` whole and `0x4cc1c9` to `0x4cc234` and `0x4cc3bb` to `0x4cc419`.

## Outbid removals

`0x4cdf30` removes the first pair with an auction id, a successful bid's (`0x4cc517`). `0x4cdf90` removes the first
pair with a house id and stops. Each lowers the count and copies the later pairs down a slot, leaving the old last
slot's pair in place. The bidder request sends the auction id of every pair of the open house, then calls `0x4cdf90`
once (`0x4cd808`): a house with n pairs sends all n and drops one, so it takes n requests to clear.

Addresses: `0x4cdf30`, `0x4cdf90`, `0x4cdf53`, `0x4cdf54`, `0x4cdf81`, `0x4cdf85`, `0x4cdfa7`, `0x4cdfab`, `0x4cdfb4`, `0x4cdfb6`, `0x4cdfd6`, `0x4cdfd8`, `0x4cc517`, `0x4cd793`, `0x4cd7b6`, `0x4cd7e8`, `0x4cd802`, `0x4cd808`, `0xb72594`, `0xb72634`, `0xb72600`
Confidence: verified
Proof: one
Test: `ui::outbid_list_remove_house_difftest`, `ui::outbid_list_remove_auction_difftest`, `ui::outbid_list_sequence_difftest`

`0x4cdfa7 cmp [edi],ecx; jne 0x4cdfd8` finds the first house match, `0x4cdfab dec esi` lowers the count, and `0x4cdfb6`
to `0x4cdfd6` copy each later pair down with the scan's own index `eax`, which ends at the new count; so `0x4cdfd8 inc
eax; cmp eax,esi; jb 0x4cdfa4` ends the scan. A match in the last slot arrives by `jae 0x4cdfd8` (`0x4cdfb4`) with
`eax` at the new count and ends it the same way. `0x4cdf30` has its own copy loop (`0x4cdf54` to `0x4cdf81`), which returns
at `0x4cdf85`; with no match it returns at `0x4cdf53`. The sender's loop closes at `0x4cd7e8 jb 0x4cd7a4` and tests the house against `0xb72600` (`0x4cd7b6`);
`0x4cd802` and `0x4cd808` call once after it. The tests run both removals whole, with a match in the first, a middle
and the last slot, and a pair past the count that the key would match.
