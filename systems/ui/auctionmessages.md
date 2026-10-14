# ui: auction results and notices

The server's reply to an auction request, its three notifications, what each does to the cached lists, and where each
message is shown. This is the second half of `AuctionHouse.cpp`; the session, the records and the requests are
`ui/auction`, the Lua functions that read the lists are `ui/auctionlists`, and the message catalog all of these
use is `ui/inventory#failure-messages`.

Roots: `0x4cc460`, `0x4cced0`, `0x4cd1f0`, `0x4cd480`, `0x4cd190`, `0x4cdfe0`

## Command result

`SMSG_AUCTION_COMMAND_RESULT` (0x25b, `0x4cc460`) reads three u32: the auction id, the action (0 start, 1 cancel, 2
bid) and the result. A result of 1 reads a fourth u32, an inventory result; a successful bid (result 0, action 2) reads
a fifth, the new minimum increment. Result 5 reads a u64 bidder GUID and two u32, the bid and the increment. A
result above 13 and the results 2, 6, 8, 9, 11 and 12 show `ERR_AUCTION_DATABASE_ERROR` (id 0x172). The arms:
result 1 shows the error for the inventory result (`0x622630`), 3 `ERR_NOT_ENOUGH_MONEY` (0x25), 4 id 0x17, 7
`ERR_AUCTION_BID_INCREMENT` (0x174), 10 `ERR_AUCTION_BID_OWN` (0x173) and 13 `ERR_RESTRICTED_ACCOUNT` (0x1be).
Every arm returns the number 1 to the dispatcher.

Addresses: `0x4cc460`, `0x4cc479`, `0x4cc4ae`, `0x4cc4b7`, `0x4cc4be`, `0x4cc66b`, `0x4cc672`, `0x4cc775`, `0x4cc77c`, `0x4cc783`, `0x4cc78e`, `0x4cc792`, `0x4cc799`, `0x4cc79e`, `0x4cc7b4`, `0x496720`, `0x622630`
Confidence: verified
Proof: one
Test: none

The reads are `0x418e30` at `0x4cc479`, `0x4cc48f` and `0x4cc4a6`. `0x4cc4ae cmp eax,0xd; ja 0x4cc799` bounds the jump
table `0x4cc7b4`, whose 14 entries send 0 to `0x4cc4be`, 1 to `0x4cc783`, 3 to `0x4cc66b`, 4 to `0x4cc78e`, 5 to
`0x4cc672`, 7 to `0x4cc775`, 10 to `0x4cc77c`, 13 to `0x4cc792` and the rest to `0x4cc799`. Every arm ends at the shared
`0x496720(id)` (`0x4cc79e`) with no argument.

## Result 0: success

Action 0 (started) shows `ERR_AUCTION_STARTED` (0x178), sets the owner request flag and sends the owner list request
again at the saved page offset. Action 1 (cancelled) shows `ERR_AUCTION_REMOVED` (0x179) and removes the auction from
the lists. Action 2 (bid placed) shows `ERR_AUCTION_BID_PLACED` (0x17f), removes the auction from the outbid list, sets
the bidder request flag and sends that request at its saved offset. It then patches the browse row if the window is
open (current bid the amount last sent, the packet's increment, the local player as high bidder, the deadline at least
90 s out), signals the browse update when none is pending, and sends `CMSG_EMOTE` (0x102) with emote 3. Any other
action shows nothing.

Addresses: `0x4cc4c7`, `0x4cc4da`, `0x4cc4e0`, `0x4cc507`, `0x4cc517`, `0x4cc522`, `0x4cc528`, `0x4cc56b`, `0x4cc57b`, `0x4cc581`, `0x4cc59a`, `0x4cc5a2`, `0x4cc5c0`, `0x4cc5d0`, `0x4cc5dd`, `0x4cc60c`, `0x4cc640`, `0x4cc648`, `0x4cc658`, `0x4cdf30`, `0xb72658`
Confidence: verified
Proof: one
Test: none

`0x4cc4c3 cmp eax,ebx` tests action 0, `0x4cc4f3 cmp eax,2` action 2 and `0x4cc640 cmp eax,edi` (edi is 1) action 1. The
started arm re-arms `0xb7263c` with `edi` = 1 and calls `0x4cd680` with the saved offset `0xb72650`, then returns with
no patch and no event. The bid arm re-arms `0xb72640`, calls `0x4cd720` with `0xb72654`, scans the browse list for the
id (`0x4cc558`), copies `0xb72658` (the amount sent by the last bid, `0x4cf91e`) to `+0x40`, the packet's increment to
`+0x2c`, the player's GUID to `+0x38` and raises `+0x34` to now + 90000 (`0x15f90`) if smaller. The emote packet is `push
0x102` and `push 3` (`0x4cc5dd`, `0x4cc5fc`), sent at `0x4cc60c`. No other query follows a cancel.

## Result 5: higher bid

Result 5 always ends by showing `ERR_AUCTION_HIGHER_BID` (0x175), but first, with the window open, it updates the
browse row and the bidder row whose auction id matches: the current bid, the increment and the high bidder come from
the packet. Each row's event is signalled when that list has no lookups pending. If the bidder list has no such
auction, the message is the same. With no window open nothing is patched.

Addresses: `0x4cc672`, `0x4cc693`, `0x4cc69f`, `0x4cc6c9`, `0x4cc6fe`, `0x4cc703`, `0x4cc72f`, `0x4cc75c`, `0x4cc769`, `0x4cc76e`
Confidence: verified
Proof: one
Test: none

`0x4cc678`, `0x4cc683` and `0x4cc68e` read the u64, the bid and the increment. `0x4cc69f je 0x4cc76e` skips the patches
without a stored auctioneer; both patch blocks store `+0x40`, `+0x2c`, `+0x38` and `+0x3c` and neither touches the
deadline. Each block gates its event on its own pending counter (`0x4cc6f6` browse, `0x4cc75c` bidder), and a bidder row
miss jumps to the same push of 0x175 (`0x4cc728`).

## Bidder notification

`SMSG_AUCTION_BIDDER_NOTIFICATION` (0x25e, `0x4cced0`) reads: house id u32, auction id u32, bidder GUID u64, bid u32,
minimum increment u32, item entry u32, random property u32. A bid of 0 means the player won: it shows
`ERR_AUCTION_WON_S` (0x17b) with the item's name and removes the auction from the lists. Any other bid means the
player was outbid: it shows `ERR_AUCTION_OUTBID_S` (0x17a), adds the auction to the outbid list, and, when the house
is the open one, patches the bidder row and the browse row (bid, increment, high bidder, deadline at least 90 s out)
and signals their update events. A house id that differs from the open window's patches and signals nothing.

Addresses: `0x4cced0`, `0x4ccef7`, `0x4ccfc1`, `0x4ccfd5`, `0x4ccfe8`, `0x4ccff3`, `0x4ccffe`, `0x4cd001`, `0x4cd033`, `0x4cd09e`, `0x4cd143`, `0x4cd163`, `0x4cd173`, `0x4cde50`, `0x4cdfe0`, `0x72b6c0`
Confidence: verified
Proof: one
Test: none

`0x4ccfc1 cmp [ebp-8],edi` with `edi` 0 is the test for a win, and the outbid arm alone calls `0x4cde50`. `0x4ccffe cmp
edx,[ebp-0x10]; jne 0x4cd178` is the house gate, after the message and the outbid-list append. The patch block
`0x4cd033` to `0x4cd08d` stores `+0x40`, `+0x2c`, `+0x38`, `+0x3c` and `+0x34`; the bidder event fires at `0x4cd09e`
when `0xb7264c` is 0, the browse event at `0x4cd143` when `0xb72644` is 0. `0x72b6c0` is called with the item
name on the outbid arm and its effect is not derived.

## Owner notification

`SMSG_AUCTION_OWNER_NOTIFICATION` (0x25f, `0x4cd1f0`) reads: auction id u32, bid u32, increment u32, bidder GUID u64,
item entry u32, random property u32; there is no house id. A nonzero bidder GUID means someone bid: the owner row
and the browse row are patched (bid, increment, bidder, deadline) and their events signalled, with no message, and
only while a window is open. A zero bidder GUID ends the auction: it shows `ERR_AUCTION_SOLD_S` (0x17c) when the bid
is nonzero and `ERR_AUCTION_EXPIRED_S` (0x17d) when it is 0, then removes the auction from the lists.

Addresses: `0x4cd1f0`, `0x4cd217`, `0x4cd259`, `0x4cd26b`, `0x4cd296`, `0x4cd301`, `0x4cd3a8`, `0x4cd3bd`, `0x4cd3f8`, `0x4cd43e`, `0x4cd44a`, `0x4cd458`, `0x4cd468`
Confidence: verified
Proof: one
Test: none

`0x4cd256 or eax,[ebp-0x14]` and `0x4cd259 je 0x4cd3bd` split the legs. `0x4cd26b je 0x4cd46d` is the no-window exit of the
patch leg, which holds no `call 0x496720`. The message leg picks 0x17c or 0x17d at `0x4cd43e cmp [ebp-8],esi`, and the
deferred form computes the same with `neg ecx; sbb ecx,ecx; add ecx,0x17d` (`0x4cd3f8`). The patches are at
`0x4cd296` to `0x4cd2c0` (owner row) and the browse row, with events at `0x4cd301` and `0x4cd3a8`.

## Removed notification

`SMSG_AUCTION_REMOVED_NOTIFICATION` (0x28d, `0x4cd480`) reads the auction id, the item entry and the random
property, each a u32, shows `ERR_AUCTION_REMOVED_S` (0x17e) with the item's name, and removes the auction from all
three lists. It patches nothing else and issues no request.

Addresses: `0x4cd480`, `0x4cd4a6`, `0x4cd4ae`, `0x4cd4b9`, `0x4cd4fb`, `0x4cd540`, `0x4cd550`, `0x4cdfe0`
Confidence: verified
Proof: one
Test: none

`0x4cd540 push 0x17e` is its only message, and the deferred form hard-codes the same id (`0x4cd4fb mov [eax+4],0x17e`).
`0x4cd550` calls `0x4cdfe0` after either form.

## Item names in messages

The name in a won, outbid, sold, expired or removed message comes from `0x5d8b00(buffer, 0x100, entry, random
property)`: the plain item name, or the name with the random property suffix through the `ITEM_SUFFIX_TEMPLATE` string,
never a link. Each handler first asks the item cache for the entry. When the record is missing it queues an 8-byte
record {random property, message id} with the callback `0x4cd190`, and the message is shown when the record arrives.
A failed lookup frees the record and shows nothing.

Addresses: `0x5d8b00`, `0x4cd190`, `0x4cd199`, `0x4cd1b3`, `0x4cd1c3`, `0x55ba30`, `0x4ccf56`, `0x4ccf6e`, `0x4cd3d5`, `0x4cd3eb`
Confidence: verified
Proof: one
Test: none

`0x4cd199 mov al,[ebp+0xc]; test al,al; je 0x4cd1cb` skips the print on a failed lookup and goes on to free the record
(`0x4cd1d9`). `0x4cd1b3` calls `0x5d8b00` with the item entry in `ecx` and the stored random property, and `0x4cd1c3`
calls `0x496720` with the stored message id and the name. The handlers allocate the record at `0x4ccf6e` and
`0x4cd3eb` (and `0x4cd4ef` for the removed notice) and queue it with `0x55ba30`.

## Removing an auction

`0x4cdfe0(auction id)` removes the auction from the browse, owner and bidder lists, one after the other, by moving
the later records down; a list is signalled (424, 425 or 426) only when it held the auction and none of its item
lookups is pending. Called by a cancelled result, a win, a sold or expired notice, a removed notice, and by
`GetAuctionItemTimeLeft` for a row more than 10000 ms past its deadline.

Addresses: `0x4cdfe0`, `0x4cdfec`, `0x4cdffb`, `0x4ce035`, `0x4ce03e`, `0x4ce04b`, `0x4ce05a`, `0x4ce095`, `0x4ce09e`, `0x4ce0ab`, `0x4ce0ba`, `0x4ce0f5`, `0x4ce101`, `0x4cc658`, `0x4cd173`, `0x4cd468`, `0x4cd550`, `0x4cf574`
Confidence: verified
Proof: one
Test: none

The function is three consecutive copies of one scan: count test (`0x4cdfec`), id compare (`0x4cdffb`), record move,
count decrement, then the pending test and the event (`0x4ce035`, `0x4ce03e` for 424). Every exit of a copy falls into
the next one's setup (`0x4ce043`, `0x4ce0a3`), so the three events are decided independently. The function ends
at the tail jump `0x4ce101` with no padding before it.

## List update events

The three list events fire at sixteen sites, all in this file. The sort tails `0x4cd8b0`, `0x4cdcd0` and `0x4cddc0`
fire without a test. The name-cache stubs `0x4cf290`, `0x4cf2b0` and `0x4cf2d0` fire when their byte argument
is nonzero. The other ten (command results 0 and 5, the three notifications, `0x4cdfe0`) fire only when their list's
count is above 0, the auction id is in that list and the list's pending counter is 0. An event held back by the
pending counter is raised by the last item arrival through the sort tail.

Addresses: `0x4cc5d0`, `0x4cc703`, `0x4cc769`, `0x4cd09e`, `0x4cd143`, `0x4cd301`, `0x4cd3a8`, `0x4cd8cf`, `0x4cdcef`, `0x4cdddf`, `0x4ce03e`, `0x4ce09e`, `0x4ce101`, `0x4cf29f`, `0x4cf2bf`, `0x4cf2df`
Confidence: verified
Proof: one
Test: none

The ids are 424 browse, 425 owner and 426 bidder (`mov ecx,0x1a8` to `0x1aa`). The sort tails are
`mov eax,[count]; push comparator; push 4; push count; push base; call 0x73f727; mov ecx,id; jmp 0x703e50`. In the other
ten, the shape is a count test, a scan with a compare on `[record+4]`, the patch, the pending test and the fire.
The census of `mov ecx,0x1a8` to `0x1aa` over instruction starts finds these sixteen and two register-opcode
decoys (`0x5e3325`, `0x5e90ec`).

## Message routes

The messages 0x16c to 0x17f are rows of the catalog read by `0x496720`; the first twelve, 0x16c to 0x177, are errors
(kind 2: `UI_ERROR_MESSAGE`) and the last eight, 0x178 to 0x17f, chat lines (kind 0, chat type 10, `CHAT_MSG_SYSTEM`
with the text and empty remaining fields). The ids count down the key list in `.rdata` order. `ERR_AUCTION_QUEST_ITEM` and
`ERR_AUCTION_BOUND_ITEM` carry the voice id 0x3b and play the error speech of the player's race and gender; the
others have none. All twenty carry the sound name `NONE`.

Addresses: `0x496720`, `0x484ca0`, `0x488410`, `0x487c1e`, `0x487cff`, `0x487d13`, `0x49a870`, `0x49b0b0`, `0x458250`
Confidence: verified
Proof: one
Test: none

The catalog is written once by the static initializer at `0x484ca0` (entered through the trampoline `0x484c90`) with the
row setter `0x488410` (`ret 0x14`: key, kind, sound name, voice id, chat type). The initializer's register preamble
sets the default kind 2, voice 0x44, chat type 10 and sound `NONE`. The groups for 0x16c to 0x17f run from `0x487c1e`;
`0x487cff push 2` is the last error and `0x487d13 push 0` the first chat line. The chat route `0x49a870` passes chat
type 10 on to `0x49b0b0`, which maps it to event 238 (`CHAT_MSG_SYSTEM`) through its chat type table; its colour is the interface's.
