# ui: tutorials

The client has 50 numbered tutorial prompts. It keeps two bit sets, one that makes each prompt fire once and one that records what the player has acknowledged, and fills both from the server's flags at login. A prompt is a sound and the event `TUTORIAL_TRIGGER`; the text and the frames that show it are interface code. This doc is the two sets, the trigger and its timers, the acknowledgement and its packet, the four Lua functions and the sites that fire each number (`../../tables/tutorial_triggers.tsv`).

Roots: `0x4b5330`, `0x4b5380`, `0x4b5390`, `0x4b54c0`, `0x4b5700`, `0x4b5960`, `0x4b59b0`, `0x4b5a00`, `0x4b5a10`

## The two bit sets

The fire-once set holds the numbers whose prompt has fired or been acknowledged. The acknowledged set holds the numbers the player has dismissed. Each is a bit count (`0xb711b8`, `0xb711e4`) and a growable dword array (`0xb711bc`, `0xb711e8`: capacity, dword count, data pointer). Number `n` is bit `n & 31` of dword `n >> 5`. A count of 0 means the set was never received.
Bring-up zeroes the data and `SMSG_TUTORIAL_FLAGS` sizes and fills both sets.

Addresses: `0xb711b8`, `0xb711bc`, `0xb711c0`, `0xb711c4`, `0xb711d8`, `0xb711e4`, `0xb711e8`, `0xb711ec`, `0xb711f0`, `0x4b53b0`, `0x4b53c5`, `0x4b53de`, `0x4b54c5`, `0x4b54e2`, `0x4b5546`, `0x4b554b`, `0x4b5553`
Confidence: verified
Proof: one
Test: none

No function bounds-checks a number against the bit count. The fire function reads and writes dword `n >> 5` of the data pointer, and the acknowledge function does the same on both sets, so a number beyond the received size touches memory past the array. Every caller passes a number below 50 (`0x4b59d8` bounds the Lua function), so a server that sends 4 bytes or fewer makes numbers 32 to 49 touch memory past a one-dword array.

## The tutorial trigger

`0x4b5390(ecx = number, edx = delay)` does nothing when the fire-once count `0xb711b8` is 0 or the number's bit is already set. Otherwise it sets the bit and, for a delay of 0, plays the interface sound `TutorialPopup` and signals `TUTORIAL_TRIGGER` (event 384) with the format `%d` and the number plus one. The number the interface sees is therefore 1 to 50.
A delay above 0 queues the prompt (below). Because the bit is set first, each number fires once per set, and a number the server already marked never fires.

Addresses: `0x4b5390`, `0x4b5396`, `0x4b539b`, `0x4b53c8`, `0x4b53de`, `0x4b53e1`, `0x4b5497`, `0x4b549c`, `0x4b54a1`, `0x4b54ad`, `0x703f50`, `0x458030`
Confidence: verified
Proof: one
Test: none

The delay is compared unsigned. The event has two fire sites, `0x4b54ad` here and `0x4b5890` in the timer callback. 51 call sites pass a number; all but one pass delay 0. The fire-once set is the only gate: the acknowledged set is not tested, but acknowledging a number sets its fire-once bit too (below).

## The delayed trigger

A delay above 0 cancels the pending timer `ProcessTutorialTimers`, allocates a 16-byte node (links in the first 8 bytes, the number at `+0x8`, the deadline `now + delay` at `+0xc`, from `OsGetAsyncTimeMs`), links it into the list `0xb71178` and calls `0x4b57f0(now)`. That arms one game timer (`0x401520`) for the smallest time to a deadline, 0 for a deadline already past, and keeps its handle in `0xb711d8`.
The callback `0x4b5840` clears the handle, plays the sound and signals the event for every node whose deadline has come, frees each, and re-arms through `0x4b57f0`. Only one number is queued by the client: number 3, 10000 ms after a forward, backward or autorun press.

Addresses: `0x4b53e7`, `0x4b53f1`, `0x4b5406`, `0x4b540f`, `0x4b5418`, `0x4b548b`, `0x4b57f0`, `0x4b5819`, `0x4b5834`, `0x4b5839`, `0x4b5840`, `0x4b5843`, `0x4b5853`, `0x4b5877`, `0x4b5890`, `0x4b58cf`, `0x401520`, `0x401550`, `0x42c010`, `0xb71178`, `0xb7117c`, `0xb71180`, `0xb711d8`, `0x514a97`
Confidence: verified
Proof: one
Test: none

The callback tests neither set: the fire-once bit was set when the node was queued, and an acknowledgement removes the node. One timer handle serves the list, so each new delayed node kills and re-arms it. The list is walked from `[0xb71180]` along the link at `+0x4`.

## Acknowledging a tutorial

`0x4b54c0(ecx = number)` returns when the number's bit is set in the acknowledged set. Otherwise it removes the first queued timer node for that number and frees it, sets the bit in both sets and sends `CMSG_TUTORIAL_FLAG` (0xFE): a `u32` opcode then a `u32` number, 8 bytes.
The number on the wire is the zero-based one. The set fire-once bit makes `0x4b5390` ignore the number from then on.

Addresses: `0x4b54c0`, `0x4b54e2`, `0x4b54eb`, `0x4b54f2`, `0x4b550e`, `0x4b5519`, `0x4b5534`, `0x4b5546`, `0x4b554b`, `0x4b5553`, `0x4b555e`, `0x4b5578`, `0x4b5584`, `0x4b558f`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

The function reads the acknowledged set's data pointer `0xb711f0` without a null test, so an acknowledgement before the first `SMSG_TUTORIAL_FLAGS` dereferences a small address: a client fault. It removes one node only, the first with that number; the fire-once bit lets a number be queued once, so that is all.
The seven callers: `0x514a88` (number 1: a press of forward, backward or autorun), `0x514aa9` (number 2: a mouse button press, `input#what-a-press-latches`), `0x4d74ae` (12 when the GUID that `0x4d7440` opens the trainer window for is the active player's, else 13), `0x5e949c` (17, `InviteByName`), `0x5ae67c` (21, `AddFriend`), `0x49f5fc` (22, every chat message that passes the dead-player gate, `chatsend`) and `0x4b59dc` (`FlagTutorial`).

## The Lua functions

`TutorialsEnabled()` pushes 1 when any byte of the acknowledged set differs from 0xFF, else nil; it pushes nil for an empty set. `FlagTutorial(n)` takes a number (`Usage: FlagTutorial("tutorial")` otherwise), truncates it and acknowledges `n - 1`; a result below 0 or above 49 does nothing and no error.
`ClearTutorials()` cancels every queued node, fills both sets with ones and sends the empty message 0xFF. `ResetTutorials()` fills both sets with zeros, leaves the queue and sends the empty message 0x100. None of the four pushes a value but the first.

Addresses: `0x4b5960`, `0x4b5973`, `0x4b5979`, `0x4b5984`, `0x4b5990`, `0x4b59b0`, `0x4b59b8`, `0x4b59cd`, `0x4b59d2`, `0x4b59d8`, `0x4b59dc`, `0x4b5a00`, `0x4b5a10`, `0x4b55c0`, `0x4b5660`, `0x4b58e0`, `0x4b5a20`, `0x4b5a40`, `0x490337`, `0x490dc6`, `0x846b04`
Confidence: verified
Proof: one
Test: none

`TutorialsEnabled` walks `[0xb711ec] * 4` bytes of the data. The byte that holds bits 48 to 55 gets only bits 48 and 49 from acknowledgements, so acknowledging all 50 numbers never fills it and the function still pushes 1; only `ClearTutorials`, or a server set of all ones, makes it nil. The four name and function pairs sit at `0x846b04`; `0x4b5a20` registers them (its one caller is `0x490337`) and `0x4b5a40` removes them (`0x490dc6`).
`ClearTutorials` and `ResetTutorials` change the data only when its count is nonzero, then send whether or not it was.

## The server's flags

`SMSG_TUTORIAL_FLAGS` (0xFD, handler `0x4b5700`) is the remaining bytes of the packet, any number of them. The handler sets both bit counts to 8 times the length, resizes both arrays to the length in dwords and copies the bytes into both. The two sets therefore start equal: a number the server has stored is already fired and acknowledged. The handler returns 1.

Addresses: `0x4b5700`, `0x4b5706`, `0x4b5716`, `0x4b5723`, `0x4b572a`, `0x4b5731`, `0x4b5767`, `0x4b576d`, `0x4b578c`, `0x4b57c8`, `0x4b57d5`, `0x4b57e0`, `0x419ac0`, `0x429340`, `0x4b536f`, `0x5ab650`
Confidence: verified
Proof: one
Test: none

The length is the packet's end minus its read position. The dword count is `((8 * length - 1) >> 5) + 1`, so a length of 0 makes it 0x8000000: a client fault for an empty packet. A length that is not a multiple of 4 copies the whole bytes and leaves the rest of the last dword as it was. The handler is registered by the bring-up (`0x4b536f`).

## Bring-up and teardown

`0x4b5330`, called by `CGGameUI::InitializeGame` (`0x48f5e0`), zeroes the data of both sets when they exist and registers the handler for 0xFD. `0x4b5380`, called by `CGGameUI::ShutdownGame` (`0x4911e6`), cancels the timer, frees every queued node (`0x4b58e0`) and removes the handler. It does not free the arrays or clear their counts; the next bring-up zeroes the data.

Addresses: `0x4b5330`, `0x4b534a`, `0x4b5363`, `0x4b536a`, `0x4b536f`, `0x4b5380`, `0x4b5385`, `0x4b538a`, `0x4b58e0`, `0x48f5e0`, `0x4911e6`, `0x5ab670`
Confidence: verified
Proof: one
Test: none

`0x4b5380` ends in a jump to `ClearMessageHandler` with `ecx = 0xfd`. The Lua functions are registered and removed elsewhere (see the Lua functions).

## Where tutorials fire

`../../tables/tutorial_triggers.tsv` lists every site by function and number. The numbers are zero-based. A movement press and a mouse press acknowledge 1 and 2, and 3 follows the movement press by 10 s. A hit the active player lands fires 5 (`0x629b37`), a unit becoming lootable 6 (`0x60049b`), the spell book 39 (`0x4b2ba1`), a quest accept of type 1 number 40 (`0x5dde7d`, `0x5ddf14`), the meeting stone status 46 (`0x4ca363`) and the battleground queue statuses 1 and 2 numbers 47 and 48 (`0x4aaa93`, `0x4aa91d`). The world frame's idle pass fires 1 once 90 s have passed since the input control's start stamp, while 1 has not fired (`0x483000`).

Addresses: `0x629b37`, `0x60049b`, `0x4ca363`, `0x4aa91d`, `0x4aaa93`, `0x5dde7d`, `0x5ddf14`, `0x5de542`, `0x5e38c0`, `0x5e4092`, `0x5e40a6`, `0x5e40b2`, `0x5e40c6`, `0x5e40da`, `0x5e40e6`, `0x5e40fa`, `0x5e410e`, `0x4b2ba1`, `0x4908c0`, `0x490a48`, `0x490a51`, `0x482fce`, `0x482ff0`, `0x483000`, `0x484670`, `0x42c010`, `0x5143e0`
Confidence: verified
Proof: one
Test: none

Every world entry fires 41 and then 0 (`0x490a48`, `0x490a51`). The level message `0x5e38c0` fires 22 at level 3 or above, 13 and 2 at 4 or above, 38 at 5 or above, 21 and 37 at 7 or above, 32 at 8 or above and 12 at 10 or above. Each test is `cmp eax,N; jl` (`0x5e4086` to `0x5e4105`), so a jump of several levels fires every lower number too. One site (`0x5de542`) takes its number from a table. `unitinfo`, `meetingstone`, `battlefield`, `questtitle`, `spellbook` and `worldenter` give the condition at each of their sites.

The idle pass `0x482ea0` reads the clock (`0x42c010`) on entry; after the idle legs `0x482fce` splits 1 into its dword and bit with `0x484670` and tests the fire-once set at `0xb711c4`, and when the bit is clear `0x482ff0` subtracts the input control's stamp (`0x5143e0`, its `+0`) and 90000 (`sub 0x15f90; js`) before `0x483000` calls `0x4b5390(1, 0)`.