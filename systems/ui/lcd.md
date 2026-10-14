# ui: the keyboard LCD

The game's side of the keyboard LCD, `LCD.CPP` (`Source\Ui\UILCD`): it draws the in-world, notice and battlefield screens on a keyboard display through a small device wrapper
(`EZ_LCD.cpp`, `0x72c630` onwards). The screens are `LCDText` elements read from XML (`LcdInitialize` names the log `Logs\LCDXML.log`) and kept in a tree by id at `0xcf0638`; the wrapper object is `0xcf06e4`. This doc holds
the functions from `0x72a2f0` to `0x72c6b0`: the ready test, the element lookup, the screen changes, the status readouts, the notices, the battlefield screen and the tree and wrapper code they stand on. The device itself is the
operating system's and the Logitech library's, outside the exe's reading here.

Roots: `0x72a370`, `0x72a450`, `0x72a4a0`, `0x72a510`, `0x72a8d0`, `0x72b440`, `0x72b620`, `0x72b670`, `0x72b6c0`, `0x72b710`, `0x72b810`

## Ready test and element lookup

Every entry point first tests that the wrapper exists, that its device handle is not −1 (`0x72c850`) and that the cvar value at `0xc0e080` is 0 to 3 (`0x72a350`), and returns at once on failure.
`0x72a2f0(id)` walks the tree rooted at `0xcf0638` by the integer key and answers the element stored at `+0x1c` of the node with that key, or 0.

Addresses: `0x72a350`, `0x72a2f0`, `0x72c850`, `0x72dcc0`, `0x72a2fd`, `0x72a306`, `0x72a31b`, `0x72a342`, `0xc0e080`, `0xcf06e4`, `0xcf0638`
Confidence: verified
Proof: one
Test: none

`0x72a350` loads `0xc0e080` and answers 1 for a value `≥ 0` and `≤ 3` (`0x72a357`, `0x72a35c`). `0x72c850` adds 0xc to its object and jumps to `0x72dcc0`, which answers 1 when the dword at `+0x7c` of the result is not −1. The lookup compares the id with `[node]` (`0x72a322`) and goes to
`[node+4]` for a smaller id and `[node+8]` for a larger.

## Changing screen

`0x72b480(n)` stores `n` as the current screen `0x87405c` and the old one in `0x874060`, and tells the wrapper (`0x72d0d0`); it does nothing when `n` is already current. `0x72b810` shows screen 0, and `0x72b440` returns to the previous screen when the
current one is 2.

Addresses: `0x72b480`, `0x72b810`, `0x72b440`, `0x72b4a4`, `0x72b4af`, `0x72b45c`, `0x72d0d0`, `0x87405c`, `0x874060`
Confidence: verified
Proof: one
Test: none

`0x72b480` compares with `[0x87405c]` (`0x72b4a4`) and stores `0x874060` first (`0x72b4af`). `0x72b440` tests `[0x87405c] == 2` (`0x72b45c`) and is called by `Script::AcceptBattlefieldPort` (`0x4ab474`).

## Entering and leaving the world

`0x72a370` enters the world: it shows screen 1, sets the in-world flag `0xcf06ec`, caches the active player at `0xcf06e8` and, by the class byte `[[player+0x110]+0x79]` (1 to 9), sets a flag on the elements 0x14 and 0x2a through `0x72cf20`. `0x72a450` leaves it: it clears the flag,
shows screen 0, clears the battlefield flag `0xcf06f8` and the player, and updates the display (`0x72d060`). `0x72a4a0` shows the battlefield screen: it sets `0xcf06f8`, stores the time `[0xb6ebbc]` at `0xcf0704`, fills the screen (`0x72a5f0`) and shows screen 3.

Addresses: `0x72a370`, `0x72a450`, `0x72a4a0`, `0x72a3d3`, `0x72a3e0`, `0x72a475`, `0x72a4c1`, `0x72d060`, `0x72cf20`, `0x72a438`, `0x72a444`, `0xcf06e8`, `0xcf06ec`, `0xcf06f8`, `0xcf0704`
Confidence: verified
Proof: one
Test: none

`0x72a370` reads the player through `0x468550` and `0x468460` with the type mask 0x10 and the jump table `0x72a438` indexed by the byte table `0x72a444`. `0x72a370` is called by `EnterWorldUI` (`0x490a1e`) and `0x72a450` by `LeaveWorldUI` (`0x490b3e`).

## The per-frame update

`UpdateKeyboardLcd` (`0x72a8d0`) runs while the LCD is ready and the in-world flag is set. It writes the player's values to the text elements with the ids 3, 5, 7, 9, 0xb and 0xd (each `%d`), the element 0xf and the elements 0x11 and 0x13
from three further formats, the readouts of `0x72b150`, `0x72b0b0`, `0x72b230` and `0x72b270`, and then settles the screen: when the notice deadline `0xcf0708` has passed it returns to the previous screen, and while the wait-time
deadline `0xcf06f0` is running it keeps the wait screen and writes the minutes and seconds. At its end it calls `0x72a5f0` when the battlefield flag `0xcf06f8` is set and `0x72a510`.

Addresses: `0x72a8d0`, `0x72a901`, `0x72a947`, `0x72a98b`, `0x72aa6b`, `0x72aaa8`, `0x72ab5b`, `0x72ac9d`, `0x72acf2`, `0x72acfd`, `0x72ad5e`, `0x72adbf`, `0x72adea`, `0x72ae0d`, `0x72ae12`, `0x72af71`, `0x72af76`, `0x72b150`, `0x72b0b0`, `0x72b230`, `0x72b270`, `0x72a5f0`, `0x72a510`, `0x72cb10`, `0xcf0708`, `0xcf06f0`, `0x42c010`
Confidence: inferred
Proof: none
Test: none

The element ids and the call order are read; what each value is (the fields of the player behind `0xcf06e8`) is not. `0x72a510` runs last: it keeps screen 5 while element 2 is shown, else picks screen 3, 2 or 1 from the elements 1 and 0 and the notice flag.

## Status readouts

`0x72b150` answers the element with id 0x14 or 0x2a by the class byte of the player: the jump table at `0x72b210` indexed by `0x72b21c` selects one of them, and a player without a class entry gives 0. `0x72b0b0` answers, for the active player only, the count of the class's item through
`CountInventoryItemsByEntry(player + 0x1d38, entry, 0)` (`0x622130`), the entry being `0x1879` for class 9 and the dword at `[[player+0xe68]+0x102c]` otherwise, or 0 when that entry is 0. `0x72b230` and `0x72b270` forward to `CountFreeSlots` (`0x622910`) and `GetDurabilityPercent` (`0x6229d0`) of the same list.

Addresses: `0x72b150`, `0x72b195`, `0x72b19c`, `0x72b0b0`, `0x72b0db`, `0x72b0ea`, `0x72b118`, `0x72b126`, `0x72b13a`, `0x72b230`, `0x72b25b`, `0x72b270`, `0x72b29b`, `0x622130`, `0x622910`, `0x6229d0`
Confidence: verified
Proof: one
Test: none

`0x72b0b0` compares the player's GUID `[player+8]` with the one `0x468550` returns (`0x72b0ea`, `0x72b0ee`) before it reads `+0xe68`; for another unit it answers 0 for the entry. `0x72b230` and `0x72b270` are guarded tail jumps, `lea ecx,[player+0x1d38]; jmp` (`0x72b25b`, `0x72b29b`).

## Notice ring

A notice is kept in five 0x50-byte records at the data pointer `0xcf0644`: a 0x40-byte text at `+0x8` and, at `+0x4c`, its kind. `0x72b4d0(text, kind)` shifts the records down one place, stores the text and the kind in the first and redraws (`0x72b550`).
`0x72b550` formats each record into an element: the text goes through the template of its kind (`0xcf06b8` for kind 1, `0xcf0678` for 2, `0xcf0650` for 3, an empty one otherwise) and `0x72cb10(id, text)` sets the record's element.

Addresses: `0x72b4d0`, `0x72b4f8`, `0x72b50c`, `0x72b53d`, `0x72b550`, `0x72b5ca`, `0x72b605`, `0x72cb10`, `0xcf063c`, `0xcf0644`, `0xcf06b8`, `0xcf0678`, `0xcf0650`
Confidence: verified
Proof: one
Test: none

`0x72b4d0` copies the records back to front in `0x50`-byte steps from `0xf0` (`0x72b4f3` to `0x72b523`) with `SStrCopy` of 0x40 bytes and their kind dword, then stores the new text (`0x72b532`) and the kind at `+0x4c` (`0x72b53d`). `0x72b550` loops `esi` to 0x140 in steps of 0x50 (`0x72b60d`).

## Notice kinds

`0x72b670` stores an empty notice of kind 2 and `0x72b6c0(text)` one of kind 3, the auction bidder notice; `0x72b620` stores a whisper as kind 1. Each shows screen 5, sets the notice deadline `0xcf0708` to the current time plus `[0x874064]`, 30,000 ms, and clears `0x874068`.

Addresses: `0x72b670`, `0x72b696`, `0x72b6aa`, `0x72b6b0`, `0x72b6c0`, `0x72b6e6`, `0x72b620`, `0x72b646`, `0x874064`, `0x874068`, `0xcf0708`
Confidence: verified
Proof: one
Test: none

`0x72b670` is called by `CGWorldFrame::OnRender` (`0x482f1f`) after a text lookup and `0x49a870`; `0x72b6c0` by `HandleAuctionBidderNotificationPacket` (`0x4ccfe8`).

## Battlefield status

`0x72b710` handles one battlefield status message (`HandleBattlefieldStatusPacket`, `0x4aab12`). It reads a slot below 3 and a dword into the slot's record at `0xb6e9d0 + slot · 0x20`, a byte, the map's name from the array at `0xc0daa8`, a status and a time. Status 2 calls `0x72b2b0` and status 3 `0x72a4a0`.
`0x72b2b0(minutes, name, deadline)` writes the name, stores the deadline in `0xcf06f0`, shows screen 2 and sets element 0x16 to the remaining minutes and seconds with `MINUTES_ABBR` and `SECONDS_ABBR`.

Addresses: `0x72b710`, `0x72b742`, `0x72b75c`, `0x72b7c0`, `0x72b7f1`, `0x72b7fd`, `0x72b2b0`, `0x72b31b`, `0x72b3ad`, `0x72b413`, `0x874468`, `0x874458`, `0xcf06f0`, `0xb6e9d0`
Confidence: inferred
Proof: none
Test: none

The reads and the call order are verified; the meaning of the packet fields is not. The calls of `0x72b2b0` and `0x72a4a0` follow the compares with 2 and 3 (`0x72b7db`, `0x72b7f6`).

## Battlefield scores

`0x72a5f0` fills the battlefield screen from the player and the scoreboard `0xb6e880` (count `0xb6ebc0`): it sorts a copy with `qsort` and the comparator `0x72a5e0` (the second argument's first dword less the first's) and writes values to the elements 0x18, 0x1a, 0x1c, 0x1e, 0x20 and 0x21 with `%d` and `%d / %d`.

Addresses: `0x72a5f0`, `0x72a629`, `0x72a6d7`, `0x72a7c1`, `0x72a7dc`, `0x72a5e0`, `0x72a7ad`, `0x72a8c9`, `0x73f727`, `0xb6e880`, `0xb6ebc0`
Confidence: inferred
Proof: none
Test: none

The reads and the writes are verified; the meaning of the scoreboard entries is not. `0x72a5e0` is `mov eax,[arg2]; sub eax,[arg1]` (`0x72a5eb`), pushed as the comparator at `0x72a7ad`.

## Unreached LCD code

`0x72a4e0` would restore the previous screen: it clears the notice flag and calls `0x72b480` with `0x874060`. `0x72b840(index)` would forward a screen and an index to `0x72d3b0`. `0x72c5f0` formats its arguments into a 0x400-byte buffer and passes it to a function it takes from a pointer table. Nothing calls any of the three and no section stores their addresses.

Addresses: `0x72a4e0`, `0x72b840`, `0x72c5f0`, `0x72b86c`, `0x72c60d`
Confidence: verified
Proof: one
Test: none

The census is every `call` and `jmp` to each address in the listing and every little-endian occurrence of each in each section of the image: none of either. The same scans find the callers of `0x72a450` (`0x490b3e`) and of `0x72b4d0` (`0x72b646`, `0x72b696`, `0x72b6e6`), the controls.

## The element tree

The element tree at `0xcf0638` is an AVL tree of nodes with the key at `+0x0`, the children at `+0x4` and `+0x8` and the balance at `+0xc`. `0x72b9a0(key, slot)` inserts a node, recursing to a child slot, rebalancing on the way back and answering whether the height grew; `0x72bd20` erases one,
and `0x72c030(slot, side)` and `0x72c0a0(slot, side)` are its single and double rotations. `0x72bb40(n)` resizes the array of 0x50-byte records at `0xcf063c` (the notice ring and its neighbours) to `n`, reallocating with `SMemReAlloc` under the tag `0x874074`.
`0x72b8d0` initialises an object pool (block size 0x10, an empty list) and `0x72b8f0` takes a record from it, clearing its fields when the pool is not flagged.

Addresses: `0x72b9a0`, `0x72bb40`, `0x72bd20`, `0x72c030`, `0x72c0a0`, `0x72b8d0`, `0x72b8f0`, `0x72b9fd`, `0x72ba32`, `0x72bbc6`, `0x72bbdf`, `0x72b8e1`, `0x72b908`, `0x72c130`, `0x72bce0`, `0x874074`, `0xcf0638`, `0xcf063c`
Confidence: inferred
Proof: none
Test: none

The shape is read from the child, balance and rotation stores (`0x72c06b`, `0x72c0cf`); the exact balance rules of the rotations and of `0x72bd20` (781 bytes, recursive at `0x72bd9d` and `0x72be05`) are not derived.

## The device wrapper

`0x72c630(title, type, flags)` constructs the wrapper object: it builds the base at `+0xc` (`0x72d810`), makes an empty list at `+0xe0`, stores the type at `+0x0`, the flags at `+0x4`, the title at `+0x8` (`World of Warcraft` when the title is empty), −1 in `+0xe8` and `+0xec`, 1 in `+0xb0` and the object's own address in `+0x80`.
`0x72c6b0` is its destructor, called by `LcdShutdown` (`0x72a2c2`); `0x72c510` is the destructor of the object with the vtable `0x8037f4`, which writes out and closes a log handle at `+0x14` and runs `CStatus::~CStatus`, and is reached only through an exception handler table.

Addresses: `0x72c630`, `0x72c639`, `0x72c673`, `0x72c685`, `0x72c695`, `0x72c6b0`, `0x72a2c2`, `0x72c510`, `0x72c52c`, `0x72c560`, `0x72c576`, `0x72c584`, `0x72d810`, `0x82e42c`
Confidence: inferred
Proof: none
Test: none

The stores of `0x72c630` are read; what the wrapper's fields are for is not. `0x72c510` is called from `0x7feb3d` (an unwind funclet) and its body names `SLogWrite` and `SLogClose` as its callees; the owner of the log it closes is not read here.
