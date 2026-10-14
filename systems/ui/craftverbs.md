# ui: craft window verbs

The Lua functions of `CraftInfo.cpp` that open and close the craft window (Beast Training, Disguise, Enscribe, Enchanting),
select a row and read its name, icon, link, description, reagents and focus, collapse its groups and cast it. The rows and
their order are `ui/craft`; the profession window with the same shape is `ui/tradeskill`. Every function here takes a
1-based row of the window list, decrements it and compares it unsigned with the row count, so a row past the end gives
the nil or empty answer named below.

Roots: `0x4f6000`, `0x4f6c10`, `0x4f6cb0`, `0x4f70f0`, `0x4f7160`, `0x4f7210`, `0x4f72a0`, `0x4f74d0`, `0x4f78b0`, `0x4f7a90`, `0x4f7d10`

## Window state

The window keeps its craft type (0xbdcfb8: 0 closed, 1 Beast Training, 2 Disguise, 3 Enscribe or Enchanting), the selected
spell id (0xbdcfbc), the row array (`0xbdcf78`, count `0xbdcfc0`), the group header array (0xbdcf88, count 0xbdcfc4) and the
number of visible rows (0xbdcfc8, what `GetNumCrafts` returns). `0x4f6000(type)` toggles the window: the open type closes it
and signals CRAFT_CLOSE (361); any other type is stored, the selection and the collapse mask are reset (all ones), the list is
built and CRAFT_SHOW (359) is signalled. `CloseCraft()` stores type 0 and signals CRAFT_CLOSE. A rebuild signals CRAFT_UPDATE (360).

Addresses: `0x4f6000`, `0x4f6006`, `0x4f6022`, `0x4f602c`, `0x4f6036`, `0x4f603b`, `0x4f5fe0`, `0x4f6c00`, `0x4f6e70`, `0x4f6be0`, `0xbdcf78`, `0xbdcfb8`, `0xbdcfc0`
Confidence: verified
Proof: one
Test: none

`0x4f6022` clears the selection and `0x4f602c` sets the mask to all ones. `0x4f5fe0` stores 0 and signals 0x169; `0x4f603b` signals 0x167. `0x4f6be0` stores a new mask, re-sorts and signals 0x168.

## Selection and names

`SelectCraft(row)` stores the spell id of the row as the selection, or 0 for a row past the end or a header. `GetCraftSelectionIndex()`
returns the 1-based position of the selected spell among the rows, or 0 when nothing is selected. `GetCraftName()` returns the
name of the spell in the player's effect slot for the open craft type, nil with no player or spell. `GetCraftButtonToken()`
returns the button token of the type. `GetCraftDisplaySkillLine()` works for type 3 only: it takes the first spell of the
player's enchanting tab, finds its skill line and returns the line's name, the player's rank in it and its maximum, else nil, 0, 0.

Addresses: `0x4f70f0`, `0x4f7127`, `0x4f6050`, `0x4f6064`, `0x4f7130`, `0x4f6080`, `0x4f6c10`, `0x4f6c3b`, `0x4f6c90`, `0x4f6cb0`, `0x4f6ce3`, `0x4f6cf4`, `0x4f6d14`, `0x4f6d52`, `0x4f6d99`, `0x4f6e08`, `0x5ea360`, `0x5ea330`, `0x60c7c0`, `0x5ea3f0`
Confidence: verified
Proof: one
Test: none

`0x4f6064` stores the id at the row and `0x4f606b` stores 0 when the id is not positive. `0x4f6c3b` asks `GetEffectSlotSpell` for the craft type. `0x4f6ce3` requires type 3, `0x4f6cf4` gets the tab spells, `0x4f6d14` finds the skill line ability and `0x4f6d52` the player's skill slot; the rank and maximum are the slot's halfwords read at `0x4f6d99` and `0x4f6e08`.

## Icon, skill line and link

`GetCraftIcon(row)` returns the icon path of the row's spell, nil for a header, an unknown spell or an icon outside the store; its
usage text names `GetTradeSkillIcon`. `GetCraftSkillLine(row)` returns the localized name of the skill line in the row's group key.
`GetCraftItemLink(row)` returns `|cffffffff|Henchant:<id>|h[<name>]|h|r` for a spell whose cast-UI field is 3 and otherwise
the item link of the first of its three effects that creates an item (effect 0x18), with the item's quality colour; with no such
effect or an uncached item it returns no value.

Addresses: `0x4f7160`, `0x4f71c7`, `0x4f7210`, `0x4f725c`, `0x4f72a0`, `0x4f7330`, `0x4f7347`, `0x4f7386`, `0x4f73d1`, `0x4f73e4`, `0x5d8b00`, `0x52adb0`
Confidence: verified
Proof: one
Test: none

`0x4f71c7` loads the spell icon id at `+0x1d4` and `0x4f725c` the group key at row `+0x04`. `0x4f7330` tests the cast-UI field at `+0xc` for 3. `0x4f7386` scans the three effects for 0x18 and loads the item at `+0x19c`.

## Description

`GetCraftDescription(row)` returns the expanded description of the row's spell, with the pet flag for craft type 1. A spell
without a description gives that of the spell a learn effect (0x24 or 0x39) teaches, and nil when there is none.

Addresses: `0x4f7a90`, `0x4f7b21`, `0x4f7b3d`, `0x4f7b53`, `0x4f7b82`, `0x4f7bc1`, `0x4f7be6`, `0x5075f0`
Confidence: verified
Proof: one
Test: none

`0x4f7b21` reads the description at `+0x228` by locale, `0x4f7b3d` sets the pet flag for craft type 1, `0x4f7b82` to `0x4f7bc1` follow the learn effects and `0x4f7be6` expands the taught spell's description, with the pet flag for effect 0x39.

## Reagents and focus

`GetCraftNumReagents(row)` counts the nonzero reagent item ids of the spell, the eight ids at `+0xa8`; 0 for a missing row.
`GetCraftReagentInfo(row, k)` takes the k-th nonzero reagent and returns the item name, the icon path, the count the recipe needs
(`+0xc8`) and how many the player carries, or four nils for a k that matches nothing; with an uncached item the name and icon are nil but the counts remain.
`GetCraftReagentItemLink(row, k)` returns the item link of that reagent, nil for none. `GetCraftSpellFocus(row)` returns pairs: first the name of the spell's focus object (`+0x3c`) with 1 whenever it is set,
then for each of the two tool items (`+0xa0`) its name and 1 when the player carries it, else nil.

Addresses: `0x4f7420`, `0x4f74d0`, `0x4f757c`, `0x4f7582`, `0x4f75c5`, `0x4f7673`, `0x4f76b2`, `0x4f7730`, `0x4f7827`, `0x4f78b0`, `0x4f7976`, `0x4f79a6`, `0x4f79c9`, `0x4f7a3c`, `0x622130`, `0x622270`, `0x55ba30`
Confidence: verified
Proof: one
Test: none

`0x4f757c` to `0x4f7596` scan the eight reagent slots and stop at the k-th nonzero one. `0x4f7673` loads the needed count and `0x4f76b2` counts the player's. `0x4f7976` loads the focus object id and `0x4f79a6` its name at `+0x4` by locale. `0x4f79c9` starts at the tool ids and `0x4f7a3c` looks for the item in the player's inventory.

## Groups and casting

`CollapseCraftSkillLine(n)` and `ExpandCraftSkillLine(n)` clear or set the bit of the group whose header is row n - 1, or the whole
mask for n below 1; a row that is not a header raises `Bad skill line in CollapseCraftSkillLine` or
`Bad skill line in ExpandCraftSkillLine`. `DoCraft(row)` casts the row's spell for the active player with no target, and does
nothing for a row past the end.

Addresses: `0x4f7c10`, `0x4f7c45`, `0x4f7c54`, `0x4f7c5d`, `0x4f7c6f`, `0x4f7c90`, `0x4f6b30`, `0x4f6b80`, `0x4f7d10`, `0x4f7d64`, `0x6e5a90`
Confidence: verified
Proof: one
Test: none

`0x4f7c47` stores a zero mask for a number below 1 and `0x4f7c54` maps the row to its group ordinal (-1 for a non-header). `0x4f6b80` is the same mapping returning whether the group is expanded. `0x4f7d64` calls `CastSpellAsActivePlayer` with the spell id and no target.
