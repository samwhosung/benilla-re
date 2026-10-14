# ui: the cursor payload

What the cursor holds while the player drags: the payload kinds and their globals, how a payload is set and
dropped, the item lock, the picture that follows the pointer, and the Lua functions that read or drop a payload.

This is `GameUI.cpp` and `Cursor.cpp`. The picture the cursor shows over the world (Point, Cast, Attack and the
rest) is `ui/worldcursor`. The senders that move items are `objectlayer/inventory`; the money payload is
`ui/cursormoney`, the vendor payload and cursors are `ui/merchantcursor`. Action bar, spell book, macro and pet
bar pick-ups put their payloads on the cursor as `ui/actionuse`, `ui/spellbook`, `ui/macros` and `ui/petbar` say.

Roots: `0x494b60` (item), `0x494cc0` (money), `0x494d20` (spell), `0x494e20` (pet action), `0x494f80` (macro),
`0x495010` (stabled pet), `0x4950f0` (preview), `0x495190` (clear), `0x495300` (destroy confirm)

## Payload kinds

`s_cursorType` (`0xb4d900`) says what the cursor holds; 0 is nothing. Each kind owns its own globals.

| kind | holds | globals |
|---|---|---|
| 1 | an item | GUID `0xb4e248`, source container GUID `0xb4b3c8`, source slot `0xb4b400`, split count `0xb4b40c` |
| 2 | money | copper `0xb4e2f0` |
| 3 | a spell | spell id `0xb4e2f4` |
| 4 | a pet action | the action word `0xb4e2f8` |
| 5, 7, 9 | a preview | entry `0xb4b41c`, display id `0xb4d8ec`, index `0xb4b420` |
| 8 | a macro | macro id `0xb4e2fc` |
| 10 | a stabled pet | stable index `0xb4e300` |

Kind 5 is a vendor row (the index is the row), kind 7 an action bar item, kind 9 ammo. Kind 6 has no writer.
The preview entry `0xb4b41c` is nonzero exactly for kinds 5, 7 and 9, and `GetCursorAmmo` (`0x495170`) returns
it. `0x495180` returns the entry and index. `0xb4da24` is a latch, 1 when the grab showed a bar grid.

Addresses: `0xb4d900`, `0xb4e248`, `0xb4b3c8`, `0xb4b400`, `0xb4b40c`, `0xb4e2f0`, `0xb4e2f4`, `0xb4e2f8`, `0xb4b41c`, `0xb4d8ec`, `0xb4b420`, `0xb4e2fc`, `0xb4e300`, `0xb4da24`, `0x495170`, `0x495180`
Confidence: verified
Proof: one
Test: none

The kind is written at one site per setter: `0x494bfe` (1), `0x494cf4` (2), `0x494dbb` (3), `0x494f12` (4),
`0x494fbd` (8), `0x4950ae` (10) and `0x495132` (5, 7 or 9, from the setter's mode argument). Every
setter's callers pass 5, 7 or 9 only. `0x492b50` tests the entry, money, spell, pet, macro and stable-pet globals
in turn, so it answers whether any payload is held.

## Setting a payload

Every setter first drops the old payload with `ClearCursor(x, 1)`, then stores its globals and the kind, sets
the picture and plays a grab sound: `INTERFACESOUND_CURSORGRABOBJECT` for a spell, pet action, macro or
stabled pet, `LOOTWINDOWCOINSOUND` for money, the item's own sound for an item or preview. The item, money and
pet action setters refuse while the player has no control (`s_playerHasControl`, `0xb4b3e4`); the item setter
also runs when the player's unit flags have bit 20 set. The preview setter needs control except for kind 7.
The spell, macro and stabled pet setters have no control test.

Addresses: `0x494b60`, `0x494cc0`, `0x494d20`, `0x494e20`, `0x494f80`, `0x495010`, `0x4950f0`, `0xb4b3e4`
Confidence: verified
Proof: one
Test: none

`0x494b8d` is the item setter's control test and `0x494ba3` its unit flags bit-20 test (`shr ecx,0x14`). `0x494cc5`
and `0x494e26` are the money and pet action control tests. `0x4950f3` is the preview setter's, and `0x495106`
accepts mode 7 without control. `0x494d20` and `0x494f80` open with a table lookup or a path test and no control test.

## Item and preview setters

`0x494b60(slot, unlock, guid, containerGuid, count)` is the item setter. A zero GUID leaves the cursor empty.
It shows the action bar grid (`ACTIONBAR_SHOWGRID`) and sets the latch when `0x5d8c00` or `0x5d9ff0` accepts
the item, and signals `CURSOR_UPDATE` last. The preview setter `0x4950f0(entry, displayId, index, mode)`
stores its three words, shows the grid for kind 7 only, sets the picture from the display id, plays the item
sound for the display id and signals `CURSOR_UPDATE`.

Addresses: `0x494b60`, `0x4950f0`, `0x4e58c0`, `0x5d8c00`, `0x5d9ff0`, `0x523b30`, `0x457ff0`
Confidence: verified
Proof: one
Test: none

`0x494c04` sets the item picture and `0x494c3b` shows the grid. `0x495149` sets the preview picture,
`0x495150` plays its sound, `0x495159` signals the event and `0x49513a` shows the grid for mode 7.

## Payload pictures by kind

The spell setter `0x494d20(spell)` takes the picture from the spell: the attack icon (`0x4e6870`) when its first
effect is 0x4e, the ranged icon (`0x4e6990`) when its attributes carry the ranged flags, else the name of its
spell icon record (`ui/actionbar#attack-and-ranged-icons`). It does nothing for a spell with no such name. The macro
setter `0x494f80(id)` needs the macro's icon path from `0x4f0fd0` to be nonempty (`ui/macros#macro-order`). The
stabled pet setter `0x495010(index)` reads a stable index, where a negative one means the summoned pet, and needs a
creature family row with an icon. The pet action setter `0x494e20(word)` uses the spell's icon for the five spell
kinds and the global named `PET_<name>_TEXTURE` for the two command kinds.

Addresses: `0x494d20`, `0x494df0`, `0x494e20`, `0x494f80`, `0x495010`, `0x4e6870`, `0x4e6990`, `0x4f0fd0`, `0x4bce50`, `0x4bce60`
Confidence: verified
Proof: one
Test: none

`0x494df0` drops the cursor when it holds a spell, which the unlearn path uses. `0x494d51` tests the effect for 0x4e, `0x494d5d` and `0x494d63` the attribute bits, `0x494d79` the fallback.
`0x494e39` takes the kind from the word's top byte; `0x494e49` jumps through the table at 0x494f40, where entries 0 to 4
go to the spell arm `0x494e50`, entry 5 to `0x494eaa` and entry 6 to `0x494ec5`, which format `PET_%s_TEXTURE`
(`0x4bce50`, `0x4bce60` give the names) and read it as a script global.

## Clearing the cursor

`0x495190(unlock, signalMoney)` drops whatever the cursor holds, by kind. It always clears the pending gift-wrap
register, restores the shown picture (`0x523bf0`), then runs the kind's arm. Kind 1 releases the item lock when
`unlock` is nonzero and a GUID is held, plays the item put-down sound, and zeroes the GUID, container GUID and
source slot. Kind 2 zeroes the money, plays `LOOTWINDOWCOINSOUND`, and signals `PLAYER_MONEY` when
`signalMoney` is nonzero. Kinds 5, 6, 7 and 9 play the put-down sound for the display id and zero the preview
globals; kinds 3, 4, 8 and 10 zero their word. Kinds 3 to 10 then play `INTERFACESOUND_CURSORDROPOBJECT`;
kinds 1 and 2 do not.

Addresses: `0x495190`, `0x5edf10`, `0x5eded0`, `0x523bf0`, `0x495420`, `0x457fb0`, `0x457ff0`, `0x458030`
Confidence: verified
Proof: one
Test: none

`0x4951bb` is the jump through the ten-entry table at 0x4952d4 indexed by the kind minus 1; the arms are `0x4951c2` (1),
`0x49522f` (2), `0x49524f` (3), `0x495257` (4), `0x49525f` (5, 6, 7 and 9), `0x495283` (8) and `0x49528b` (10). The
kinds 3 to 10 reach `0x495291`, which plays the drop sound by name. Kind 2 jumps over it
(`0x49524d`), and kind 1 jumps to `0x49529b` (`0x49522d`).

## Clearing the tail

Every kind ends the same way. If the grid latch `0xb4da24` is set the clear hides the bar grid, with
`PET_BAR_HIDEGRID` for kind 4 and `ACTIONBAR_HIDEGRID` otherwise, and resets the latch. It zeroes the kind and
signals `CURSOR_UPDATE`. That signal fires on every call, including one on an empty cursor. The split count
`0xb4b40c` is not cleared.

Addresses: `0x49529b`, `0x4bce80`, `0x4e58d0`, `0xb4da24`
Confidence: verified
Proof: one
Test: none

`0x4952a3` tests the kind for 4, `0x4952ac` calls the pet bar hide, `0x4952b3` the action bar hide. The kind is
zeroed at `0x4952bf` and event 0x124 signalled at `0x4952c6`.

## The item lock

Bit 0 of the item's word at `+0x314` marks an operation pending. `0x4953e0(guid)` sets it and signals
`ITEM_LOCK_CHANGED` only when the item resolves. `0x495420(guid)` clears it and signals the event for any nonzero
GUID, whether or not the item resolves. A locked item cannot be picked up from a bag. The server's reply clears it: the
inventory slot field update (`objectlayer/playerfields#inventory-and-money-changes`) and a failed inventory
change message (`objectlayer/inventory`) clear the items they name. A cancelled drag releases its item through
the clear's `unlock` argument; a swap clears with `unlock` 0, so the held item stays locked until the server
answers. `0x495460` clears every item the active player holds, bags included, and signals once; the player's
resurrection calls it.

Addresses: `0x4953e0`, `0x495420`, `0x495460`, `0x5fef70`
Confidence: verified
Proof: one
Test: none

`0x4953e0` does `or [item+0x314],1` at `0x495409` after the lookup (type mask 2) and signals event 0xbc at `0x495415`;
`0x495420` signals at `0x495450` outside the null test of the lookup. `0x495460` walks the lists the player's virtual at `+0x10`
returns and clears bit 0 of each item it finds (`0x4954f8`, `0x495556`). Its one caller is the resurrection routine
`0x5fef70` (`0x5fefae`), when the unit is the player.

## Dragging picture

While a payload with a picture is held, the pointer shows that picture: the icon scaled to 32 by 32, laid
over the `Item` cursor where that image is opaque. `0x523a70(guid)` and `0x523b30(displayId)` (kinds 1, 5, 7
and 9), `0x5239d0(amount)` (money) and `0x523a30(path)` (spell, pet action, macro, stabled pet) each load a
texture, store the picture state in `0xbe2c48` (1, 2 or 3; the blit tests only for nonzero) and the texture
handle in `0xbe2c60`, and call `0x523840`.

`0x523840` accepts only a 64 by 64 texture. It averages each 2 by 2 block into one pixel, forces the alpha to
0xff, writes 32 by 32 pixels into `0xbe1c28`, and frees the texture.

Addresses: `0x523820`, `0x523840`, `0x5239d0`, `0x523a30`, `0x523a70`, `0x523b30`, `0xbe2c48`, `0xbe2c60`, `0xbe1c28`
Confidence: verified
Proof: one
Test: none

An item or preview icon is `Interface\Icons\` plus its basename. `0x5238a4` and `0x5238ac` test both texture sizes against 0x40. The box filter at `0x5238e1` sums four texels per
channel with a rounding term of 2. `0x523820` frees the handle when the payload is cleared. `0x523a70` builds the
path from `StringLookups` row 3 (`0x53f810`) and `0x5d8af0` with the format `%s%s%s`.

## Sending the picture

`0x523790` sends one image to the device. With no payload picture it sends the current mode's image from the
cursor array `0xbe2d80`, 0x1000 bytes per mode. With a payload and a live texture handle it sends the `Item` image
alone. With a payload and no handle it sends, per pixel, the `Item` image where its alpha byte is nonzero, else
the icon buffer. No digit or stack count is drawn. The `Item` image is slot 18 of the array, `0xbf4d80`.

Addresses: `0x523790`, `0xbe2d80`, `0xbf4d80`, `0x58b5b0`, `0x58b5c0`
Confidence: verified
Proof: one
Test: none

`0x5237a2` indexes the array by the mode shifted left 12. `0x5237c3` to `0x5237d6` is the live-handle copy.
`0x5237e5` is the per-pixel test against the alpha mask 0xff000000.

## DELETE_ITEM_CONFIRM

`0x495300` runs when an item payload is released over the world. It does nothing unless the held item exists
and its owner is the active player. If the item template's flags have bit 0x20 set it shows error 0x152 and
returns. Otherwise it signals `DELETE_ITEM_CONFIRM` with the item's name and quality (`"%s%d"`). It never clears
the cursor.

`DeleteCursorItem` sends the destroy. With a held item it finds the inventory slot of the source container
(`0x5e13b0`) and refuses an item with the same flag bit (error 0x152). Otherwise it sends opcode 0x111 with
that slot, the source slot and the split count (two bytes and a dword), clears the cursor and locks the item.

Addresses: `0x495300`, `0x4897b0`, `0x5da2a0`, `0x496720`, `0x55ba30`, `0x5e13b0`, `0x418190`, `0x418070`, `0x4181f0`
Confidence: verified
Proof: one
Test: none

`0x495346` reads the owner pair at `[item+0x114]` and compares it with the active player's GUID
(`0x495359`, `0x49535d`). `0x49536e` calls the template flags getter at the `+0x110` vtable's slot 0 (`0x5da2a0`), and `0x495370 test al,0x20`
tests the flag. `0x4953a5` looks the template up in the item cache, `0x4953b6` pushes the format `%s%d`, `0x4953c0`
signals event 0x123. `DeleteCursorItem` (`0x4897b0`) does the same test (`0x489834`, `0x489836`), then
`0x489854` pushes the opcode 0x111; `0x48986b`, `0x48987b` and `0x48988f` add bag, slot and count.

## Cursor functions

`ClearCursor()` runs `ClearCursor(1, 1)` and returns 1 result without pushing one. `CursorHasItem()` returns
the number 1 for kinds 1 and 9 and nil otherwise; `CursorHasSpell()` for kind 3; `CursorHasMoney()` for
kind 2. None returns a boolean. `AutoEquipCursorItem()` calls the player's equip-from-cursor routine with 0
(`objectlayer/inventory#bind-on-equip-confirmation`).

`EquipCursorItem(slot)` needs a number (usage error otherwise) and an item on the cursor. A slot argument of 0
or 256 auto-equips. A slot holding the cursor item only clears the cursor. If `CheckItemForSlot` accepts the
slot it sends a swap from the item's origin and locks the displaced item. If it refuses and the item came from
an equipment slot below 19, it clears the cursor; otherwise it auto-equips with 1.

Addresses: `0x4895b0`, `0x4895d0`, `0x489600`, `0x489630`, `0x48a040`, `0x489660`, `0x494c60`, `0x494c80`, `0x5e1480`, `0x5ea720`, `0x5e0c40`
Confidence: verified
Proof: one
Test: none

`0x4895b0` ends in `mov eax,1` after the clear, so the Lua result is whatever is on the stack top.
`0x4895d0` tests 1 at `0x4895d5` and 9 at `0x4895da`. `0x494c60` returns the item GUID only for kind 1, `0x494c80`
returns the GUID, container GUID and source slot without a kind test. `0x4896e3` and `0x4896ef` send 255 and -1
to auto-equip; `0x489781` compares the source slot with 0x13. The swap `0x489757` takes the cursor GUID, the
container GUID and source slot, the player's GUID and the slot.

## Dropping an item on a unit

`DropItemOnUnit(unit)` needs a string or number argument (usage error `DropItemOnUnit("unit")` otherwise) and
returns nothing on every path. It does nothing unless an item is on the cursor and the unit token names a unit.
For another player who can cooperate with the active player (same faction group) it first handles trade: the open
trade partner gets the item in a trade slot, and with no trade open the call asks to trade (opcode 0x116). The
payload stays. Then, for the player's pet (the charm, else the summon), it tries to feed it
(`spell/cast#cast-entries`) and, when that returns true, clears the cursor with `ClearCursor(1, 1)`. A refusal
leaves the cursor alone. There is no distance test.

Addresses: `0x48d960`, `0x494c60`, `0x515940`, `0x606ba0`, `0x5da2c0`, `0x4bfc50`, `0x5d3fb0`, `0x6ea1e0`, `0xb71728`, `0xb714a0`
Confidence: verified
Proof: one
Test: none

`0x48d975` raises the usage error; `0x48d98f` reads the item GUID and `0x48d9aa` stops without an item. `0x48d9be` resolves the
unit and `0x48d9db` requires a unit type. `0x48d9e4` tests the player type bit; `0x48d9f5` excludes the active player and
`0x48da1f` calls `CanCooperate`. `0x48da36` compares with the trade partner pair `0xb71728`: equal goes to
`0x4bfc50` (`0x48da50`), a zero pair to `0x5d3fb0` with 1 and the GUID (`0x48da62`). `0x48da6c` and `0x48da79` compare with the
pet GUID pair `0xb714a0`, `0x48da86` calls the feed routine and `0x48da96` clears.
