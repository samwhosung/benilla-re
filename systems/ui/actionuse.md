# ui: placing, picking up and using actions

What the stock interface's drop, drag and click on an action button do in the client, and the three
small action-bar functions beside them: the bonus-bar offset, the four bar toggles and the combo
points. The slot words and the tests a button reads are in `ui/actionbar`; the cursor payloads these
functions take and give are in `ui/cursor`.

Roots: `0x4e5ee0`, `0x4e6130`, `0x4e62e0`, `0x4e7140`, `0x4e71d0`, `0x4e7240`, `0x4e7620`, `0x4e7660`, `0x4e76e0`, `0x51a190`, `0x6f1c10`

## Placing an action

`PlaceAction` (`0x4e7240`) runs `0x4e62e0(slot)`. It returns with nothing done unless the slot is 0 to 119
and the context check `0x494a50` for kind 9 passes. It reads three candidates: the spell on the
cursor (mode 3), the item entry (the mode 7 payload, else the entry of the mode 1 item) and the macro
(mode 8). The first of spell, item, macro that is set decides. A cursor holding anything else (money,
pet action, vendor row, ammo, stabled pet) matches none: nothing happens and the payload stays on
the cursor.

Addresses: `0x4e62e0`, `0x4e6316`, `0x4e6323`, `0x4e6334`, `0x4e6341`, `0x4e6357`, `0x4e6384`, `0x4e66fd`, `0x4e685d`, `0x494a50`, `0x494e10`, `0x494c60`, `0x495170`, `0x4e7240`
Confidence: verified
Proof: one
Test: none

`0x4e6316 call 0x494a50` with `ecx` 9. `0x4e6323` loads the spell (`0x494e10`), `0x4e6334` the mode 7
payload (`0x495170`), `0x4e6341` the macro (`[0xb4e2fc]`), `0x4e6357` the mode 1 item's entry
through `0x468460` (item type mask). `0x4e6384 jle` falls to the item test, `0x4e66fd` is the macro
test, and `0x4e685d` is the bare return every refusal takes.

## Placement refusals

A spell with `Attributes & 0x40` (passive) is refused with the error message 158 and stays on the
cursor. An item is refused silently when its template is unknown, or when it has no on-use spell
(spell id nonzero, trigger 0, in any of five slots) and an inventory type of 0. A spell whose record
is missing is refused. When the incoming id equals the slot's current id (the slot's resolved spell
for a spell, the entry for an item, the macro id for a macro) the cursor is cleared and nothing else
happens.

Addresses: `0x4e63a7`, `0x4e63ad`, `0x4e63cd`, `0x4e6567`, `0x4e6571`, `0x4e6595`, `0x4e65cf`, `0x4e672f`, `0x496720`, `0x495190`
Confidence: verified
Proof: one
Test: none

`0x4e63a7 test [rec+0x18],0x40`, then `push 0x9e` and `0x496720` at `0x4e63ad` and a plain return.
The item filter is `0x4e6571` to `0x4e6598`: the loop over `[rec+0x11c + 4i]` and `[rec+0x130 + 4i]`,
then `cmp [rec+0x2c],0` at `0x4e6595`; `0x4e6567` rejects a missing template. `0x4e63cd`, `0x4e65cf`
and `0x4e672f` are the same-id compares, each followed by the clear `0x495190(1, 1)`.

## Placing into a slot

The slot's old content goes onto the cursor first: an item as a mode 7 payload (its entry, display
id and the slot), a spell as mode 3, a macro as mode 8, an empty slot just clears the cursor. The slot then
stores the new word (`entry | 0x80000000`, the spell id, `macroId | 0x40000000`); an item slot
refreshes its count; and `0x4e58e0(slot, 1, 0)` sends the slot and signals its change.

Addresses: `0x4e6455`, `0x4e6499`, `0x4e64e7`, `0x4e66e7`, `0x4e66e9`, `0x4e6848`, `0x4e684f`, `0x4e644b`, `0x4e67aa`, `0x4950f0`, `0x494d20`, `0x494f80`, `0x4e58e0`
Confidence: verified
Proof: one
Test: none

The hops: items `0x4950f0(entry, display, slot, 7)` (`0x4e644b`, `0x4e67aa`), spells `0x494d20`, macros
`0x494f80`. The stores are `0x4e6455`, `0x4e6499`, `0x4e64e7`, `0x4e66e7` and `0x4e6848`; `0x4e66e9`
recounts. The tail `0x4e684f`/`0x4e6851` is `0x4e58e0` with `edx` 1. A drag between two slots sends
two packets: the pickup clears and sends the source slot, the place sends the target.

## Picking up an action

`PickupAction` (`0x4e71d0`) runs `0x4e6130(slot)`. A slot outside 0 to 119 does nothing. While the cursor
already holds a spell, an item, a macro or a mode 7 payload, it places instead (`0x4e62e0`). An empty
slot does nothing. Otherwise it puts the slot's content on the cursor and clears the slot with
`0x4e5db0`, which stores 0 and sends it: an item as mode 7 (the entry, the template's display id or
0 when the template is unknown, and the slot), a spell as mode 3, a macro as mode 8. A word of no kind
or a macro that is gone just clears the slot.

Addresses: `0x4e6130`, `0x4e615d`, `0x4e6184`, `0x4e6215`, `0x4e621c`, `0x4e6254`, `0x4e625b`, `0x4e6272`, `0x4e629d`, `0x4e62a4`, `0x4e62c9`, `0x4e71d0`, `0x4e5db0`
Confidence: verified
Proof: one
Test: none

`0x4e615d` to `0x4e6194` are the four cursor tests (`0x494e10`, `0x494c60`, `[0xb4e2fc]`, mode 7 with
`0x495170`), and `0x4e62c9` the tail call to `0x4e62e0`. The item arm calls `0x55ba30`, takes
`[rec+0x18]` at `0x4e61e9` (0 on a miss at `0x4e61ee`) and calls `0x4950f0` at `0x4e6215` with
`push esi`, `push 7`. `0x4e6254` and `0x4e629d` are the spell and macro pickups; every arm ends in
`0x4e5db0` (`0x4e621c`, `0x4e625b`, `0x4e62a4`).

## Using an action

`UseAction` (`0x4e7140`) takes the slot, a cursor flag and a self flag and runs
`0x4e5ee0(slot, cursorFlag, selfFlag)`. A bad slot or an empty one does nothing. With the
cursor flag set and a spell, item, macro or mode 7 payload on the cursor it places instead. Then by
kind. A macro runs `0x4f1460(macro id)` (`ui/macros`). A spell `s`: if `0x4e55f0` holds (its active
icon is showing) it cancels the aura with `0x6e7040(s)`; else if `s` is the auto-repeat spell it stops
it (`0x6ea080`); else if `s` is the spell awaiting a target it stops targeting (`0x6e4900`); else it
casts `s` through `0x6e5a90`, with the active player as the target when the self flag is set and none
otherwise.

Addresses: `0x4e7140`, `0x4e5ee0`, `0x4e5f12`, `0x4e5f40`, `0x4e6098`, `0x4e60b6`, `0x4e60c1`, `0x4e60da`, `0x4e60f3`, `0x4e6119`, `0x6e5a90`, `0x6e7040`, `0x6ea080`, `0x4e6900`, `0x6e4900`, `0x6e9fd0`, `0x6e48e0`, `0x4f1460`
Confidence: verified
Proof: one
Test: none

The binding reads argument 3 into the stack slot and argument 2 into `edx` (`0x4e716f`, `0x4e717e`).
`0x4e5f14` to `0x4e5f3e` are the cursor tests, `0x4e5f42` the tail call to placement. The macro arm
calls `0x4f1460` at `0x4e6098`. The spell arm: `0x4e60b6` the active test, `0x4e60c1` the aura cancel,
`0x4e60da` the auto-repeat stop after comparing with `0x6e9fd0`, `0x4e60f3` the targeting stop after
`0x6e48e0`, `0x4e6119` the cast. The cooldown of the cast starts from the cast machinery, not here
(`spell/go#own-cooldown-start`).

## Using an item slot

An item whose template has inventory type 0, or whose entry the equipment slots (walk mode 1) already
hold, is used: the walk finds an item of the entry (mode bit 0x20 when the template's charges are
neither 0 nor -1, which skips copies with 0 charges) and `0x5d8d00` uses it
(`objectlayer/items#using-an-item`). A template of nonzero inventory type with no copy equipped is
searched for in equipment, bags and keyring (`0x6222b0`) and the first copy is auto-equipped with
`0x5e1840`, one `CMSG_AUTOEQUIP_ITEM`. An unequipped trinket therefore equips on a click and a
worn one is used. No active player, a missing template or no copy does nothing.

Addresses: `0x4e5f73`, `0x4e5fd2`, `0x4e5fdd`, `0x4e5fe7`, `0x4e5ff0`, `0x4e5ff7`, `0x4e600c`, `0x4e602a`, `0x4e603a`, `0x4e604f`, `0x4e605f`, `0x4e607b`, `0x5e1840`, `0x5d8d00`, `0x622270`, `0x6222b0`
Confidence: verified
Proof: one
Test: none

`0x4e5fd2` fetches the template; `0x4e5fdd` loads the inventory type and `0x4e5fe2 je` takes the use
arm for 0. `0x4e5fe7` pushes mode 1 for `0x622270` (`0x4e5ff0`) and `0x4e5ff7 jne` takes the use arm
when a copy is equipped. `0x4e600c call 0x6222b0` with mode 0, then `0x4e602a call 0x5e1840` with the
container guid and slot. The use arm builds the flag 0x20 at `0x4e604f` and calls `0x622270`
(`0x4e605f`), then `0x5d8d00` at `0x4e607b`.

## Reading a script flag

`0x6f1c10(L, index, default)` reads a Lua argument as a flag. A missing argument, a light userdata
and a table, function, userdata or thread give the default. Nil gives 0. A boolean gives itself. A
number is truncated toward zero and gives 1 when nonzero, so 0.6 is 0. A string gives 0 when its first
character is `0`, `F`, `f`, `N` or `n`, and 1 when it is `1` to `9`, `T`, `t`, `Y` or `y`. Any other
string is compared without case with `off` and `disabled`, giving 0, with `on` and `enabled`, giving 1,
and otherwise gives the default. The string `"0"` is therefore false although every Lua 5.0 string is
true.

Addresses: `0x6f1c10`, `0x6f1c19`, `0x6f1c27`, `0x6f1c2e`, `0x6f1c46`, `0x6f1c60`, `0x6f1c72`, `0x6f1c81`, `0x6f3400`, `0x6f3660`, `0x6f3620`, `0x6f3690`, `0x64a4c0`
Confidence: verified
Proof: one
Test: none

`0x6f1c19` calls `lua_type` (`0x6f3400`); `0x6f1c1e cmp eax,4; ja` sends a missing argument to the
default at `0x6f1cdc`, the others through the five-dword table at 0x6f1ce8: nil to the zero return,
boolean to `0x6f3660`, light userdata to the default, number to `0x6f3620` and the truncating
`0x40a2b0`, string to `0x6f1c51`. The first byte, minus 0x30, indexes a 0x4a-byte table at 0x6f1d08
(arms 0 false, 1 true, 2 the keyword compares of `0x6f1c81` onward with `0x64a4c0` and length
0x7fffffff); a byte outside `0` to `y`, and the empty string, take arm 2.

## Bonus bar offset

`GetBonusBarOffset` (`0x4e7620`) pushes the number in `0xbc6e88`. `0x4e4fc0` sets it: 0, then the
`SpellShapeshiftForm` record of the player's form byte (`[unit+0x110]+0x212`), if the form is within
the table and the record exists, gives its second field (`+4`); then it signals
`UPDATE_BONUS_ACTIONBAR` (217). The mapping from form to bar is data in the record; the client does
not compute it. A form change (`0x5de010`) and world entry (`0x4e4f90`) run it.

Addresses: `0x4e7620`, `0x4e4fc0`, `0x4e4fc5`, `0x4e4fef`, `0x4e4ffa`, `0x4e500f`, `0x4e5012`, `0x4e501d`, `0x5de010`, `0x5de01b`, `0xbc6e88`, `0xc0d774`, `0xc0d778`
Confidence: verified
Proof: one
Test: none

`0x4e4fc5` stores 0; `0x4e4fef` loads the form byte; `0x4e4ffa cmp eax,[0xc0d778]` bounds it by the
store's maximum id; `0x4e5002` loads the id table `0xc0d774`; `0x4e500f mov edx,[eax+4]` and `0x4e5012`
store the offset; `0x4e501d` jumps to `0x703e50` with 0xd9. `0x5de01b` is the call in the form-change
handler. The store at 0xc0d76c is the `SpellShapeshiftForm` DBC (`tables/dbc_stores.tsv`).

## Bar toggles

`SetActionBarToggles` (`0x4e76e0`) reads Lua arguments 1 to 4 with `0x6f1c10` (default false) into bits
0 to 3 of one byte and sends `CMSG_SET_ACTIONBAR_TOGGLES`: `u32 703`, `u8 byte`. A fifth argument is
never read. It changes no local state and adds no check of its own. `GetActionBarToggles` (`0x4e7660`)
returns four values, 1 or nil for bits 0 to 3 of the active player's `PLAYER_FIELD_BYTES` byte 2
(`[player+0xe68]+0x102a`), and four nils without a player, so a missing player and a zero byte look
alike. Nothing in the client writes that byte; only the server's field update does, and no event
follows.

Addresses: `0x4e76e0`, `0x4e76fa`, `0x4e7709`, `0x4e771d`, `0x4e7737`, `0x4e7743`, `0x4e774e`, `0x4e7660`, `0x4e768c`, `0x4e76b9`, `0x4e76cd`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

`0x4e76f1` is the loop head: `edi` is the argument index 1 to 4 and the shift is `edi - 1`
(`0x4e7703`); the accumulator at `[ebp-4]` can only hold 0 to 15. `0x4e771d push 0x2bf`, `Put32`
(`0x4e7737`), `Put8` of the byte (`0x4e7743`), `Send` (`0x4e774e`). The one read of `+0x102a` in
the image is `0x4e768c`. `0x4e76cd` returns 4. The field is index 1222 of the player's descriptor,
which only the active player's allocation holds, so the binding resolves the player itself.

## Combo points

`GetComboPoints` (`0x51a190`) pushes one number, never nil. It is 0 unless the active player is a rogue
(class 4) or a druid (class 11) and the player's combo target (`[player+0xe68]+0x838`) equals the
selected target (`0xb4e2d8`); then it is the combo-point byte (`[player+0xe68]+0x1029`, byte 1 of
`PLAYER_FIELD_BYTES`). The button usable test reads the same byte class-blind and target-blind
(`spell/checks#usability-predicate`, gate 5), and a change of the byte signals `PLAYER_COMBO_POINTS`
(202) (`objectlayer/playerfields#combo-points-form-and-far-sight`).

Addresses: `0x51a190`, `0x51a1c9`, `0x51a1cc`, `0x51a1d0`, `0x51a1f9`, `0x51a205`, `0x51a20d`, `0x51a234`, `0x51a261`
Confidence: verified
Proof: one
Test: none

`0x51a1c9` loads the class byte (`[unit+0x110]+0x79`), compared with 4 and 0xb at `0x51a1cc` and
`0x51a1d0`. The pair at `+0x838` and `+0x83c` is compared with the target guid at `0x51a205` and
`0x51a20d`. `0x51a234` loads the count; every failing path reaches `0x51a261`, which pushes 0.0.
The two self-identity compares (`0x51a1e5`, `0x51a222`) compare the player with the active player.
