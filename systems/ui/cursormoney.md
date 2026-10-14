# ui: the money cursor

The money payload (kind 2 of `ui/cursor`): the setter, the four Lua functions that read or change it, why
the purse is never debited, the coin icon that follows the pointer, and the trade functions that take the
money off the cursor.

Roots: `0x494cc0` (set), `0x48abc0` (PickupPlayerMoney), `0x48aba0` (DropCursorMoney), `0x48d4e0` (GetCoinIcon),
`0x4c0790` (PickupTradeMoney), `0x4c07f0` (AddTradeMoney)

## The money payload

`0x494cc0(amount)` puts money on the cursor, or takes it off for 0. It does nothing while the player has no
control, and nothing for 0 unless the cursor holds money. Otherwise it clears the cursor with `ClearCursor(1, 1)`,
which signals `CURSOR_UPDATE` while the cursor is already empty. For a nonzero amount it then stores the amount, sets
the kind to 2, plays `LOOTWINDOWCOINSOUND`, sets the coin picture, and signals `PLAYER_MONEY`. No second
`CURSOR_UPDATE` follows, so a handler of that event sees no money on the cursor. Holding money and picking up
more first runs the money arm of the clear, so the event order is the sound, `PLAYER_MONEY`, `CURSOR_UPDATE`, then
the sound and `PLAYER_MONEY` of the new payload.

Addresses: `0x494cc0`, `0x495190`, `0x5239d0`, `0xb4e2f0`, `0xb4b3e4`
Confidence: verified
Proof: one
Test: none

`0x494cc5` tests control. `0x494cd0` is the clear-only test of the kind for 2 when the amount is 0. `0x494cde`
calls `ClearCursor(1, 1)`. `0x494ce5` ends on a zero amount. `0x494cee` stores the amount, `0x494cf4` the kind, `0x494cfe`
the sound, `0x494d05` the picture and `0x494d10` tail-calls the event with 0xc8. The money arm of the clear
(`ui/cursor#clearing-the-cursor`) never sends a packet.

## Money functions

`GetCursorMoney()` pushes the amount as an unsigned number and always returns 1 value. `CursorHasMoney()` pushes
the number 1 when the kind is 2, nil otherwise. `DropCursorMoney()` does nothing at all when the cursor holds no
money; otherwise it sets 0, so the money arm runs. `PickupPlayerMoney(amount)` raises a usage error for a
non-number (a numeric string is accepted). The amount is truncated toward zero. It does nothing for 0, without an
active player, or for an amount above the purse compared unsigned, so a negative amount is refused by the
purse test. The purse is the player's coinage field. Otherwise it calls the setter. It returns no values.

Addresses: `0x48ab70`, `0x489630`, `0x48aba0`, `0x48abc0`, `0x40a2b0`, `0x6f34d0`
Confidence: verified
Proof: one
Test: none

`0x48ab7e` zeroes the high dword before the load, so the amount is unsigned. `0x48ac23 jbe` is the zero test and `0x48ac51 ja`
the unsigned purse test; `0x48ac3b` substitutes a purse of 0 when the object found is not the active player.
`0x48ac3f` reads the coinage field at `[[player+0xe68]+0xf70]`. `0x48ac53` calls the setter.

## The purse stays

Picking money up does not change the player's coinage field, and dropping it gives nothing back. The cursor
amount is a display figure only: the interface's backpack figure is the coinage minus the cursor money minus
the trade offer. The server's next field update is the only thing that changes coinage, and its watcher
`0x5ddf30` plays the coin sound, signals `PLAYER_MONEY` and trims a trade offer that now exceeds the purse
(`objectlayer/playerfields#inventory-and-money-changes`, `objectlayer/trade`).

Addresses: `0x5ddf30`, `0x494cc0`, `0x495190`
Confidence: inferred
Proof: none
Test: none

No instruction writes the coinage field at its displacement outside the object constructor. The generic
field-update store writes by index, so it is not visible to a displacement search.

## Coin icons

`GetCoinIcon(amount)` raises a usage error for a non-number and returns `Interface\Icons\` plus a name chosen by
the truncated amount, compared signed: below 10 `INV_Misc_Coin_05`; below 100 `INV_Misc_Coin_06`; below 1000
`INV_Misc_Coin_03`; below 10000 `INV_Misc_Coin_04`; below 100000 `INV_Misc_Coin_01`; else `INV_Misc_Coin_02`.
A negative amount gives `INV_Misc_Coin_05`. The money cursor's picture is the same icon.

Addresses: `0x48d4e0`, `0x6c62d0`, `0x53f810`, `0x5239d0`
Confidence: verified
Proof: one
Test: none

`0x6c62d0(amount, buffer, size)` formats the `StringLookups` row 3 text, a separator and the leaf; the compares are
`0x6c6307`, `0x6c6320`, `0x6c6339`, `0x6c6355` and `0x6c6371`. `0x48d4e0` passes the buffer `0xb4d920` of 0x104 bytes
and pushes it as a string.

## Trade money from the cursor

`AddTradeMoney()` takes no argument. With money on the cursor it tries to add that amount to the trade offer
(`objectlayer/trade`); when the offer sender accepts, it clears the cursor with `ClearCursor(1, 0)`, so no
`PLAYER_MONEY` fires. When the sender refuses, the money stays. `PickupTradeMoney(amount)` raises a usage error for
a non-number. It does nothing unless the truncated amount is above 0 and at most the offer, compared signed.
Then it lowers the offer by the amount and puts the amount on the cursor, which signals `PLAYER_MONEY`; the
setter refuses without control, after the offer is lowered.
`SetTradeMoney(amount)` raises a usage error for a non-number and sends the truncated amount as the new offer.

Addresses: `0x4c07f0`, `0x4c0790`, `0x4c0820`, `0x5d4500`, `0x5d45f0`, `0x5d4680`, `0x5d4760`
Confidence: verified
Proof: one
Test: none

`0x4c07f6` tests the cursor amount, `0x4c07fa` calls the add sender with it, `0x4c0805` clears with `ecx` 1 and `edx` 0.
`0x4c07c6 jle` and `0x4c07d1 jg` are the signed tests in `0x4c0790`; `0x4c07ca` reads the offer, `0x4c07d5` lowers it and
`0x4c07dc` calls the setter. `0x4c0856` calls the absolute sender.
