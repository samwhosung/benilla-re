# ui: the loot window

The loot window's data: the list the server sends, the 16-row array the window reads, the Lua functions over it, the
row widget, how a row is taken and when the window closes. This is `LootFrame.cpp` (`0x4c1880` to `0x4c3450`). The
wire parse of `SMSG_LOOT_RESPONSE` is `objectlayer/inventory#loot`; the group roll is `objectlayer/lootroll`; master
loot is `ui/masterloot`; the loot tooltips are `ui/tooltipenchant` and `ui/tooltipitem`.

Roots: `0x4c1cb0`, `0x4c1880`, `0x4c1820`, `0x4c2790`, `0x4c2c30`, `0x4c2c60`, `0x4c2e70`, `0x4c2ec0`, `0x4c3450`

## Loot state

The window keeps these cells: the loot source GUID `0xb71b48` (zero when no window is open), the money `0xb71ba0`
(0 for none, -1 after the coin row has been taken), the loot type `0xb71bbc`, a count of item records still being
fetched `0xb71b44`, the row waiting for a bind confirmation `0x847cec` (-1 for none), a flag that the auto-loot coin
message was shown `0xb71bc4`, and the row array `0xb71968`. A row of the array is 0x1c bytes, 16 of them.

| offset | field |
|---|---|
| +0x00 | 1 while the item's record is being fetched, else 0 |
| +0x04 | item entry, 0 for an empty row |
| +0x08 | display id |
| +0x0c | stack count |
| +0x10 | the fourth number of the item link |
| +0x14 | random property id |
| +0x18 | the wire slot, as a byte |

Addresses: `0xb71b48`, `0xb71ba0`, `0xb71bbc`, `0xb71b44`, `0x847cec`, `0xb71bc4`, `0xb71968`, `0x4c2740`, `0x4c1de7`, `0x4c1de9`, `0x4c1def`, `0x4c1df5`
Confidence: verified
Proof: one
Test: none

`0x4c1ddb` to `0x4c1de7` clear 0x70 dwords from `0xb71968` (16 rows of 0x1c). `0x4c1de9` stores the money, `0x4c1dd6` the type, `0x4c1def` zeroes the pending count and `0x4c1df5` stores -1
in the confirmation cell. The slot type of the wire (`objectlayer/inventory#loot`, byte +0x14 of its entry) stays in
that array and is read on demand by wire slot through `0x5ebce0`.

## Opening and closing a session

`0x4c1cb0(source, money, type)` ends the session in progress and starts the next. Ending: for a unit it asks `0x4c2a70`
whether anything is left and, if not, calls `0x600680` (the loot sparkle off); for a game object it calls `0x5f8bd0`;
for an item it unlocks it. It cancels every fetch still pending (`0x55c0c0`), zeroes the GUID, and signals `LOOT_CLOSED`
(269). Starting: it stores the GUID of the source object, clears the array, then copies the wire entries (below).
It asks the item cache for each copied item and counts the misses. With misses outstanding it returns, firing nothing.
With none, a player with the auto-loot byte (`+0x1d30`) set takes the auto-loot sweep instead; otherwise it signals
`LOOT_OPENED` (267).

Addresses: `0x4c1cb0`, `0x4c1cfc`, `0x4c1d07`, `0x4c1d1c`, `0x4c1d36`, `0x4c1d5a`, `0x4c1dac`, `0x4c1dc2`, `0x4c1e77`, `0x4c1ea6`, `0x4c1ec7`, `0x4c1ed3`, `0x4c1ee6`, `0x4c2a70`, `0x600680`, `0x5f8bd0`, `0x48f3a0`
Confidence: verified
Proof: one
Test: none

`0x4c1ce4` resolves the old source with mask 1; its type word is tested for unit (`shr 3`), game object (`shr 5`) and
item (`shr 1`). `0x4c1dac` calls `0x703e50` with `ecx` 0x10d. The new source GUID is stored at `0x4c1dc2` and `0x4c1dd0`. The
caller chain is `0x5eb900` to `0x48f3a0` to `0x4c1cb0`. `0x4c1ea6 jne` returns with a fetch outstanding; `0x4c1ee1` loads
0x10b.

## Copying the entries

The 16 wire entries are copied in order to the array, skipping each whose item entry is 0, so the rows are
packed and a row's position is not its wire slot. Each row takes the item entry, display id, count, link number and
random property from the wire entry's getters, and keeps the wire slot number in byte +0x18. A second pass asks the
item cache for each row's entry with the callback `0x4c2ac0` and sets +0x00 to 0 on a hit, or to 1 with one added to
the pending count on a miss.

Addresses: `0x4c1e07`, `0x4c1e20`, `0x4c1e2a`, `0x4c1e34`, `0x4c1e3e`, `0x4c1e46`, `0x4c1e49`, `0x4c1e60`, `0x4c1e80`, `0x4c1e8e`, `0x5ebc90`, `0x5ebca0`, `0x5ebcb0`, `0x5ebcc0`, `0x5ebcd0`, `0x5ebce0`
Confidence: verified
Proof: one
Test: none

The loop `0x4c1e07` to `0x4c1e50` runs the wire slot in `ebx` from 0 to 15 and moves the write cursor by 0x1c only when a row is
taken. `0x4c1e09` loads the item entry, `0x4c1e23`, `0x4c1e2d`, `0x4c1e37` and `0x4c1e41` the display id, count, link number and
random property, `0x4c1e49` stores the slot byte. The wire slot type is not copied.

## Late item records

`0x4c2ac0(entry, ok)` is the item-cache callback. A failed fetch does nothing. Otherwise it clears the pending flag
of every row with that entry and subtracts one from the pending count; at 0 it does what the end of `0x4c1cb0` does:
the auto-loot sweep, or `LOOT_OPENED`. So the window is not shown until every item record has arrived, and its one
`LOOT_OPENED` is raised on the last arrival. The stock window shows itself on that event and does not repaint on a
second one.

Addresses: `0x4c2ac0`, `0x4c2ac3`, `0x4c2ae2`, `0x4c2af6`, `0x4c2afc`, `0x4c2b29`, `0x4c2b37`, `0x55bdb0`
Confidence: verified
Proof: one
Test: none

`0x4c2ac3 mov al,[ebp+0xc]; test al,al; je` drops a failed arrival. `0x4c2ae2` clears the flag, `0x4c2af6 dec` and `0x4c2afc jne`
leave until the last, and `0x4c2b37` signals 267. The fetch completion `0x55bdb0` runs each pending callback with the success byte 1. The
callback address is pushed at `0x4c1e67`, `0x4c1d4f` and `0x4c1f47` only.

## Taking a row out

`0x4c1f00(wire slot)` removes the row that carries that wire slot: it cancels the row's fetch if one is pending,
zeroes the row's seven dwords and signals `LOOT_SLOT_CLEARED` (268) with the position in Lua terms (the row's array
position plus one, plus one more when there is money). The array is not packed again, so the later rows keep their
positions. When nothing is left (`0x4c2a70`) it closes the loot with release (`0x48f200` with 1, 0).
`0x4c2750` does the same for the coin row: it sets the money to -1, signals 268 with 1, and closes when nothing is left.

Addresses: `0x4c1f00`, `0x4c1f0f`, `0x4c1f24`, `0x4c1f47`, `0x4c1f59`, `0x4c1f60`, `0x4c1f77`, `0x4c1f7f`, `0x4c1f8d`, `0x4c2750`, `0x4c2765`, `0x4c276f`, `0x4c2785`, `0x4c2a70`
Confidence: verified
Proof: one
Test: none

`0x4c1f05` to `0x4c1f20` scan for a nonzero entry with the wire slot; `0x4c1f59 mov ecx,7; rep stos` zeroes the row;
`0x4c1f6a` and `0x4c1f6b` add the money and the one. `0x4c2a70` resolves the source (mask 1), returns 1 when the money
is above 0 or a row has an entry, else 0.

## Counting and guards

Every accessor of the window except the count (`0x4c22a0`, `0x4c22e0`, `0x4c2320`, `0x4c2360`, `0x4c23a0`,
`0x4c2440`, `0x4c2460`, `0x4c2550`, `0x4c2670`) takes a 0-based slot and applies the same guards: nothing is open, so the
answer is empty; with money not zero, slot 0 is the coin row and every other slot drops by one; a position of 16 or
more (unsigned) is empty. `0x4c2260`, the count, is the position of the last row with an entry plus one, plus one
when the money cell is nonzero, so a taken coin row (-1) still counts. The coin row has an entry only while the
money is above 0.

Addresses: `0x4c2260`, `0x4c2277`, `0x4c2281`, `0x4c2290`, `0x4c229b`, `0x4c2440`, `0x4c2454`, `0x4c2456`, `0x4c22a0`, `0x4c22e0`, `0x4c2320`, `0x4c2360`, `0x4c23a0`, `0x4c2460`, `0x4c2550`, `0x4c2670`
Confidence: verified
Proof: one
Test: none

`0x4c2281 lea eax,[edx+1]` is rewritten for each row whose entry is above 0, so the last one wins. `0x4c229b inc eax`
follows `je` on a zero money cell. `0x4c2440` returns the money for slot 0 only when `jle` finds it above 0. The prologues are
`mov eax,[0xb71b48]; or eax,[0xb71b4c]`, then the money test, `test ecx,ecx; je` and `dec ecx`, then `cmp ecx,0x10; jae`.

## Loot Lua reads

`GetNumLootItems()` is the count above. `GetLootSlotInfo(slot)` (1-based, no range check) always returns four values:
icon path, name, stack count and quality. A closed window or a slot out of range gives a nil icon, an empty name, 0
and 0; a cleared row or an uncached item gives quality -1. The coin row gives the coin icon, the money text, 0 and 0.
An item row gives its icon (or `INV_Misc_QuestionMark`), its name with the random suffix
(`ui/auctionmessages#item-names-in-messages`), the count and the cache's quality. `LootSlotIsItem` and
`LootSlotIsCoin` return the number 1 or nil. A non-number raises `Usage: GetLootSlotInfo(slot)`.

Addresses: `0x4c2c30`, `0x4c2c60`, `0x4c2c88`, `0x4c2ca6`, `0x4c2cc0`, `0x4c2cd4`, `0x4c2cee`, `0x4c2d07`, `0x4c2d90`, `0x4c2e00`, `0x4c2435`, `0x4c24b7`, `0x4c252b`, `0x4c2559`, `0x55ba30`, `0x6f3890`, `0xb71840`
Confidence: verified
Proof: one
Test: none

`0x4c2d07 mov eax,4` is unconditional. `0x6f3890` pushes nil for a null pointer, so the icon producer's null returns
(`0x4c2470`, `0x4c2486`, `0x4c24a6`, `0x4c24b7`) become nil, while `0x4c2559 mov byte [ebx],0` empties the name before any guard.
The quality producer returns 0 for guards (including the coin row, `0x4c23de`) and -1 at `0x4c2435`, where the item cache
answers null for entry 0 as well as a miss. The icon is `0xb71840` built from the string-lookup directory and the
`ItemDisplayInfo` icon column (`+0x14`); the producers push signed numbers (`fild`).

## Item link and name

`GetLootSlotLink(slot)` pushes the link of the row, `|Hitem:<entry>:0:<random property>:<link number>|h[<name>]|h` with the
quality colour and `|r`, and pushes nothing on a miss or an uncached item. The enchantment id is always 0. The name
uses the same suffix join as the row text. The link number is the value of wire entry +0x0c.

Addresses: `0x4c2d20`, `0x4c2670`, `0x4c26ca`, `0x4c26fc`, `0x4c2717`, `0x52adb0`, `0x5d8b00`
Confidence: verified
Proof: one
Test: none

`0x4c26ca` asks the item cache for the entry, `0x4c2709` to `0x4c2715` push the name, the link number, the random
property and 0, then call `0x52adb0` with the quality, as in `ui/auctionlists#item-links-and-time-left`. The tooltip
side of a loot slot is `ui/tooltipenchant`.

## The row widget

`LootButton` is a frame type that `CreateFrame` and the XML loader can build (`0x495a30` is its factory, registered at
`0x4959ad`): a button of 0x4e0 bytes, one more dword (`+0x4dc`) than the base, holding a 0-based slot. It has one
method of its own, `SetSlot(index)`, which stores the 1-based index minus 1 and returns nothing
(`Usage: SetSlot(index)`). It overrides six virtuals of the button, of which only the click matters. Its script surface
is the button's, and no hover or drag code asks for the type.

Addresses: `0x495a30`, `0x4959ad`, `0x4c1880`, `0x4c193a`, `0x4c193b`, `0x4c1bb0`, `0x4c1be0`, `0x4c1820`, `0x8045c0`, `0x804594`, `0x6ee880`, `0x847ce4`
Confidence: verified
Proof: one
Test: none

The factory allocates 0x4e0 bytes, calls the button constructor `0x7786a0`, zeroes `+0x4dc` (`0x495a56`) and stores the
vtables `0x8045c0` (`0x495a60`) and `0x804594` (`0x495a66`). The method table `0x847ce4` holds one entry,
`SetSlot` -> `0x4c1880`, installed by `0x4c1bb0`; `0x4c193a dec eax` and `0x4c193b mov [edi+0x4dc],eax` store the slot.
Slot 37 of the primary vtable (offset `0x94`), the click, is `0x4c1820` where the button's is `0x779540`.

## The row click

`0x4c1820(button code, scripted)` is the click. A scripted click (`Button:Click()`) does nothing at all, not even the
`<OnClick>` script. A real click first runs the button's own click, which fires the `<OnClick>` script, then returns if
Shift, Control or Alt is down, and otherwise takes the row (`0x4c2790(slot, 0)`). The button code is not examined, so
a left and a right release both take, and a quick second click takes a second row.

Addresses: `0x4c1820`, `0x4c182b`, `0x4c1833`, `0x4c183a`, `0x4c1848`, `0x4c1856`, `0x4c185f`, `0x4c1867`, `0x779540`, `0x779285`, `0x7793a9`, `0x41f8f0`, `0x7827b6`
Confidence: verified
Proof: one
Test: none

`0x4c182b jne` skips the body for a nonzero second argument. The mouse dispatchers `0x779285` and `0x7793a9` call
`[vtable+0x94]` with 0, the `Button:Click` binding with 1 (`0x7827b6`). `0x41f8f0` is the modifier test: `ecx` 0, 1 and 2 for
Shift, Control and Alt (`0x4c183a`, `0x4c1848`, `0x4c1856`), the same calls as the key state Lua functions
(`0x488590`, `0x4885c0`, `0x488600`).

## Taking a slot

`0x4c2790(slot, confirm)` takes a row. A click (`confirm` 0) on slot 0 with money above 0 takes the coin row: it plays
`LOOTWINDOWCOINSOUND`, sends `CMSG_LOOT_MONEY` (0x15e), signals 268 with 1, sets the money to -1 and closes when
nothing is left. Otherwise the money-excluded index picks a row, and its wire loot type decides: 1 shows
`ERR_LOOT_ROLL_PENDING` (0x19a) and sends nothing; 2 signals `OPEN_MASTER_LOOT_LIST` (503) and sends nothing; any other
type, for a bind-on-pickup item (bonding 1) of quality 2 or more, signals `LOOT_BIND_CONFIRM` (287) with the index plus
one, stores the index in `0x847cec` and sends nothing; else it plays the item sound and sends
`CMSG_AUTOSTORE_LOOT_ITEM` (0x108) with the wire slot byte.

Addresses: `0x4c2790`, `0x4c283b`, `0x4c2847`, `0x4c2858`, `0x4c2885`, `0x4c28a9`, `0x4c28b3`, `0x4c28cb`, `0x4c28d1`, `0x4c28f2`, `0x4c28fb`, `0x4c2901`, `0x4c2917`, `0x4c2921`, `0x4c2936`, `0x5ea8c0`, `0x5e1ad0`, `0x457ff0`
Confidence: verified
Proof: one
Test: none

`0x4c28a9` fetches the type, `0x4c28ae cmp eax,1` and `0x4c28c4 cmp eax,2`, the latter ending in the tail `jmp 0x703e50` at
`0x4c28d1`. Types 0, 3 and 4 share the default arm from `0x4c28d6`, so a locked or owner row is sent and refused by the
server. `0x5e1ad0` writes `push 0x108` and the slot byte read from `[esi+0xb71980]`. With `confirm` set (`LootSlot` after
the confirmation) the row must equal `0x847cec`; it then sends and resets the cell, reading the row before its bound
check (`0x4c27d1`, `0x4c27df`).

## Loot functions

`LootSlot(index)` (1-based) calls `0x4c2790(index - 1, 1)`, the confirmation form, which acts only on the pending row; no
Lua function takes a row directly, the take lives in the row widget's click. `CloseLoot([flag])` closes the loot with
release; a true argument also shows `ERR_LOOT_NO_UI`. `IsFishingLoot()` returns 1 when the stored loot type is 3, else
nil. `SetLootPortrait(texture)` bakes the loot source unit's portrait into a texture (`0x524f60`) and returns 1, or
clears the texture and returns nil when the source is not a unit.

Addresses: `0x4c2e70`, `0x4c2ea4`, `0x4c2eac`, `0x4c2ec0`, `0x4c2ee0`, `0x4c2ee5`, `0x4c2b40`, `0x4c2bd1`, `0x4c2bf0`, `0x4c2c0d`, `0x48f200`, `0x4c2740`, `0x4c3450`, `0x847cf0`
Confidence: verified
Proof: one
Test: none

`0x4c2ea4 dec eax` then `edx` 1 is the confirmation call. `0x4c2ec0` passes the truth of its first argument as the
last argument of `0x48f200(1, 0)`. `0x4c2ee5 cmp eax,3` tests the type read by `0x4c2740`. `0x4c2bd1` resolves the
source with mask 8. The loot functions are 16 pairs from `0x847cf0` registered by `0x4c3450`: `SetLootPortrait`,
`GetNumLootItems`, `GetLootSlotInfo`, `GetLootSlotLink`, `LootSlotIsItem`, `LootSlotIsCoin`, `LootSlot`, `CloseLoot`,
`IsFishingLoot`, the two master loot functions (`ui/masterloot`) and four roll functions.

## Loot refusals

`SMSG_LOOT_RESPONSE` with loot type 0 is a refusal: a u8 code follows. Codes 4 to 14 show an error through a jump table,
other codes the `ERR_LOOT_DIDNT_KILL` message. All are error-kind messages (red text), and the codes 4 to 7 also speak the
error line of the player's voice.

| code | error | code | error |
|---|---|---|---|
| 4 | `ERR_LOOT_TOO_FAR` | 10 | `ERR_LOOT_PLAYER_NOT_FOUND` |
| 5 | `ERR_LOOT_BAD_FACING` | 11 | `ERR_PLAY_TIME_EXCEEDED` |
| 6 | `ERR_LOOT_LOCKED` | 12 | `ERR_LOOT_MASTER_INV_FULL` |
| 7, others | `ERR_LOOT_DIDNT_KILL` | 13 | `ERR_LOOT_MASTER_UNIQUE_ITEM` |
| 8 | `ERR_LOOT_NOTSTANDING` | 14 | `ERR_LOOT_MASTER_OTHER` |
| 9 | `ERR_LOOT_STUNNED` | | |

Addresses: `0x5eb900`, `0x5eba11`, `0x5eba17`, `0x5ebc64`, `0x5ebabd`, `0x5ebac2`, `0x496720`
Confidence: verified
Proof: one
Test: none

`0x5eba0b add eax,-4; cmp eax,0xa; ja 0x5ebabd` bounds the table `0x5ebc64`; `0x5ebabd` pushes 0x83 and is also the arm for
code 7. Codes 10 and 12 to 14 call `0x496720` and return; every other code, with those outside 4 to 14, goes on to the
shared release (`0x5ebac2`), which clears the player's loot target, unlocks the item and recomputes the animation
(`objectlayer/inventory#loot`). The speech lines belong to the message rows 0x81 to 0x84.

## Loot tooltip

`GameTooltip:SetLootItem(slot)` (`0x533470`) takes the 1-based slot as a number; anything else raises
`Invalid loot slot in SetInventoryItem`, the text of another function. It returns nothing. An item entry of 0 or less
(the coin row, an empty row, a closed window) returns without a tooltip. Otherwise it fills the item instance block
of the tooltip from the row: the seven enchantments, cooldown and creator fields 0, the charge field -1, the link number
and random property from the row. It builds the item tooltip with the loot source GUID, a zero instance GUID and the block
flag set, so the builder takes its suffix path and does not print the random enchantment placeholder
(`ui/tooltipenchant`).

Addresses: `0x533470`, `0x533541`, `0x533551`, `0x533556`, `0x53355b`, `0x533573`, `0x53359c`, `0x5335af`, `0x5335b7`, `0x5335ba`, `0x5335c5`, `0x5335d0`, `0x5335d9`, `0x5335df`, `0x5335ef`, `0x4c22a0`
Confidence: verified
Proof: one
Test: `ui::luabind_533470_set_loot_item`

`0x533541` subtracts 1.0 and `0x533547` truncates. `0x533551` calls the entry getter and `0x53355b jle` leaves. The two
`rep stos` of seven dwords (`0x533573`, `0x53357d`) clear the block from `tooltip+0x3d0`; `0x53359c` stores -1 at `+0x428`;
`0x5335b7` and `0x5335bf` store the row's link number (`+0x420`) and random property (`+0x424`). The builder arguments are the
entry, `&0xb71b48`, `&{0, 0}`, 0, 0, 1 for the block flag (`0x5335c5 push 1`) and zeros.

Bit-exact coverage: `ui::luabind_533470_set_loot_item` covers `0x53353c` to `0x533547` of `0x533470`.

## Auto-loot

`0x4c1fa0(flag)` is the auto-loot sweep, run when a session ends fetching and the player's auto-loot byte is set (it
follows the Shift key at interaction, `objectlayer/interaction#looting`). Without a player it signals `LOOT_OPENED`.
It sends the coin request when there is money, plays the coin sound, and prints the money line for a player in no
group. It then walks the rows: a type-0 bind-on-pickup item of quality 2 or more raises the confirmation for the first
such row only; any other type-0 row is sent as in a click. It clears the auto-loot byte; if loot remains and `flag` is
0 it signals `LOOT_OPENED`, and if nothing remains it closes the loot with release.

Addresses: `0x4c1fa0`, `0x4c1fb3`, `0x4c1ff4`, `0x4c1ffc`, `0x4c2020`, `0x4c2170`, `0x4c2180`, `0x4c2196`, `0x4c21c2`, `0x4c21d4`, `0x4c21ff`, `0x4c2219`, `0x4c221e`, `0x4c2240`, `0x4c224f`, `0x5df460`, `0xb71bc4`
Confidence: verified
Proof: one
Test: none

`0x4c1fb3 jne` returns while fetches are pending. `0x4c1ffc` sends the coin request, and `0x4c2001` to `0x4c2020` play the sound
only when `0xb71bc4` is 0. `0x4c2196 test eax,eax; jne` selects loot type 0; `0x4c21c2 test ebx,ebx` keeps one
confirmation. `0x4c2219` calls `0x5df460(0)`, which clears the byte, and `0x4c221e` stores 1 in `0xb71bc4`.

## No walk-away leash

Nothing arms the interaction range for loot: the loot GUID is never stored in the interaction cell `0xb4e2d0`, so the
per-frame range check `0x493230` never measures a loot source. The loot GUID is the 14th entry of the interaction
drop-all `0x493310`, whose close routine is a single `ret` (`0x4c3f40`). The call of `0x4930d0` at `0x4c3ec0` belongs to
the lottery kiosk window, whose open `0x4c3eb0` has no caller. What closes loot on movement is the movement-start guard
`0x60e990`: a keyboard move, strafe, turn or pitch start, or a jump, calls `0x48f200(1, 1)` and the loot closes with
release, unless the source is an item (a lockbox) or a disenchant window with items left.

Addresses: `0x493230`, `0x493310`, `0x49350e`, `0x4c3f40`, `0x4c3eb0`, `0x4c3ec0`, `0x4930d0`, `0x60e990`, `0x60e9dd`, `0x48f200`, `0x48f2ab`, `0x48f2b5`, `0x48f2da`, `0x48f311`, `0x48f33d`, `0x4c3e70`
Confidence: verified
Proof: one
Test: none

The writers of `0xb4e2d0` are `0x493114` (in `0x4930d0`) and `0x49334b` (in `0x493310`) only. The calls of `0x4930d0` are at
`0x4acd1c`, `0x4bf53e`, `0x4c3ec0`, `0x4cade2`, `0x4cd57f`, `0x4d74ce`, `0x4dba5c`, `0x4e1fe9`, `0x4e349a`, `0x4f4fc0`, `0x4f5882`, `0x4f8507`,
`0x4fad40` and `0x500d74`. `0x4c3eb0` pushes 0x334, the lottery opcode, and has no `call`, `jmp` or stored address. In `0x48f200` a
nonzero release byte sends `CMSG_LOOT_RELEASE` (0x15f) at `0x48f311`; `0x48f2ab` to `0x48f2b5` leave an item source
alone, and `0x48f33d` calls `0x4c1cb0(0, 0, 0)`.
