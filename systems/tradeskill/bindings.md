# tradeskill: bindings

The Lua functions of the profession window (`TradeSkillFrame.cpp`). They read the list of `recipes`, set its filters and collapse state, and select a row. An index is the 1-based position in the sorted list. The functions that take an index test it
against the full row count and not the displayed count, which the sort keeps consistent because hidden rows sort last. DoTradeSkill and the repeat count are `cast`; the item tooltip lines are `itemstats`.

Roots: none, the function rows of this file belong to `ui`

## List and selection

`GetNumTradeSkills` (`0x4fd800`) returns the displayed count. `GetTradeSkillInfo` (`0x4fd820`) returns four values for an index. A recipe row gives the spell name, its difficulty word (`optimal`, `medium`, `easy` or `trivial`), the available count and nil. A header row gives the subclass name, `header`, 0, and 1 when the group is not collapsed or nil when it is.
`SelectTradeSkill` (`0x4fd9b0`) stores the row's spell id as the selection, or 0 for a header, and `GetTradeSkillSelectionIndex` (`0x4fd9f0`) returns the position of the selected spell plus 1, so 0 for none. `GetFirstTradeSkill` (`0x5001c0`) returns the position of the first displayed row with a spell id above 0, or 0.

Addresses: `0x4fd800`, `0x4fd820`, `0x4fd9b0`, `0x4fd9f0`, `0x5001c0`, `0x8080a0`, `0x846f64`, `0x4fd88a`, `0x4fd8d0`, `0x4fd8fc`, `0x4fd90a`, `0x4fd934`, `0x4fd95e`, `0x4fd995`, `0x4fd9a2`, `0x4fd9e7`, `0x4fd9f7`, `0x4fd9fc`, `0x5001d5`, `0x5001ef`, `0x5001f6`, `0x4fc9b0`, `0x4fc9e0`, `0x4fd6a0`
Confidence: verified
Proof: one
Test: none

An index out of range, a row with no spell record and a header with no subclass record give nil, nil, 0 and nil (`0x4fd8d0`). The difficulty words are the 32-byte entries at `0x8080a0`, indexed at `0x4fd995`, and `0x846f64` holds `header`. `0x4fd88a` handles a header row: it finds the ItemSubClass row of the group, takes the name at `+0x4c` or, when that is empty,
at `+0x28` (`0x4fd8fc` to `0x4fd90a`), and asks `0x4fd6a0` whether the group is collapsed (`0x4fd934`). `0x4fd95e` handles a recipe row and `0x4fd9a2` pushes the count. `0x4fd9e7` calls the selection function; `0x4fd9f7` calls the index finder and `0x4fd9fc` adds 1. `0x5001d5` to `0x5001ef` scan the displayed rows and `0x5001f6` pushes 0 when none qualifies.

## Row values

`GetTradeSkillCooldown` (`0x4fda20`) returns the seconds left of the recipe spell's cooldown, or nil when the index is bad, the row is a header or the spell is not cooling down: the start plus the duration minus the millisecond clock, scaled by 0.001. `GetTradeSkillNumMade` (`0x4fdc50`) returns the least and the greatest number of items the first effect makes, each at least 1, and 1 and 1 for a bad index.
`GetTradeSkillLine` (`0x4fdd40`) returns the open line's name, the player's rank in it and the maximum rank, each plus the permanent bonus when the base value is above 0; an unknown line gives `UNKNOWN`, 0 and 0, and a missing player gives the name, 0 and 0. `GetTradeSkillIcon` (`0x4fdae0`) is `ui/spellbook#recipe-icons`.

Addresses: `0x4fda20`, `0x4fda5d`, `0x4fda79`, `0x4fda8b`, `0x4fda94`, `0x4fdaad`, `0x4fdab5`, `0x4fdc50`, `0x4fdcd3`, `0x4fdcd8`, `0x4fdce0`, `0x4fdd13`, `0x4fdd40`, `0x4fdd83`, `0x4fddb6`, `0x4fddec`, `0x4fddf4`, `0x4fde5b`, `0x4fde6a`, `0x4fde92`, `0x4fdae0`, `0x6e2ea0`, `0x42c010`, `0x801360`, `0x6e3760`, `0x5ea3f0`
Confidence: verified
Proof: one
Test: none

`0x4fda79` rejects a header row (`jle`), `0x4fda8b` asks for the cooldown with the player history and `0x4fda94` reads the clock. `0x4fdaa1` subtracts it from the sum of the two values; the quantity is stored as an unsigned 64-bit integer (`fild qword` at `0x4fdaad`) and multiplied by the float at `0x801360` (`0x4fdab5`). `0x4fdcd3` calls the effect amount `0x6e3760` with effect 0 and level 0 (`spell/records#effect-amount-formula`), and `0x4fdcd8` and
`0x4fdce0` raise the two results to 1. `0x4fdd13` is the fallback pair. `0x4fdd83` pushes the name, `0x4fddb6` finds the skill slot and `0x4fddec` and `0x4fde5b` read the rank and maximum words, to which `0x4fddf4` and `0x4fde6a` add the permanent bonus word `+0x852`. `0x4fde92` pushes `UNKNOWN`. A player with no slot for the open line gives the slot -1, which `0x4fddec` and `0x4fde5b` do not test, so the words are read below the first slot.

## Reagents

`GetTradeSkillNumReagents` (`0x4ff510`) returns the number of the spell's eight reagent slots (`+0xa8`) that hold an item, 0 for a bad index. `GetTradeSkillReagentInfo` (`0x4ff5c0`) takes an index and the 1-based number of a filled slot and returns four values: the item's name, its icon path, the count the recipe needs (`+0xc8`) and the count the player has. A slot number
that matches no filled slot gives four nils. A reagent whose template is missing gives nil, nil, then the two counts, and the cache request carries the callback `0x4fdc30`. The icon path is the icon directory prefix (`0x53f810` with 3), a backslash when the prefix is not empty, and the display icon name of the item.

Addresses: `0x4ff510`, `0x4ff583`, `0x4ff590`, `0x4ff5c0`, `0x4ff66a`, `0x4ff672`, `0x4ff67a`, `0x4ff68b`, `0x4ff6b5`, `0x4ff6e2`, `0x4ff6f0`, `0x4ff6fd`, `0x4ff708`, `0x4ff71e`, `0x4ff73d`, `0x4ff763`, `0x4ff7a2`, `0x4ff7b5`, `0x4ff754`, `0x4ff7c8`, `0x4fdc30`, `0x53f810`, `0x5d88b0`, `0x622130`, `0x833184`, `0x882748`, `0x8362d8`
Confidence: verified
Proof: one
Test: none

`0x4ff590` counts the nonzero item words. `0x4ff672` to `0x4ff680` count filled slots until the counter equals the requested number, and `0x4ff68b` pushes four nils when none does. `0x4ff6e2` requests the template, with the requester spell id and the callback; `0x4ff6f0` pushes the name and `0x4ff6fd` fetches the prefix. `0x4ff708` and `0x4ff71e` choose the backslash `0x833184` or the empty string `0x882748`
and fetch the icon, and `0x4ff73d` formats `%s%s%s`. `0x4ff754` is the missing-template path. `0x4ff763` pushes the needed count and `0x4ff7a2` the inventory count; `0x4ff7c8` pushes 0 when the player is missing.

## Item links

`GetTradeSkillItemLink` (`0x4ff410`) returns the link of the item the recipe creates, and no value when the index is bad, the row is a header or the template is missing. `GetTradeSkillReagentItemLink` (`0x4ff800`) takes an index and a filled-slot number and returns the link of that reagent, or a single nil. The link is the colour of the item's quality, `|Hitem:` and the item id, three zeros, the plain item name in brackets,
and the closing codes. The quality colour comes from the table `0x854124` of seven entries; a quality of 7 or more, or a negative one, uses white. The template requests carry no callback, so a missing template sends nothing.

Addresses: `0x4ff410`, `0x4ff4a0`, `0x4ff4b9`, `0x4ff4d2`, `0x4ff4e9`, `0x4ff4f3`, `0x4ff800`, `0x4ff8c7`, `0x4ff8da`, `0x4ff8e3`, `0x4ff8ed`, `0x4ff902`, `0x4ff917`, `0x4ff93c`, `0x4ff946`, `0x52adb0`, `0x52ad90`, `0x854124`, `0x8549c8`, `0x5d8b00`, `0x55ba30`, `0x4ff489`, `0x4ff483`
Confidence: verified
Proof: one
Test: none

`0x4ff4b9` requests the template with a zero requester and no callback, and `0x4ff4d2` writes the name with property 0. `0x4ff4e9` calls the link builder `0x52adb0` with the item id, the quality, three zero arguments and the name; its format is `0x8549c8`, `%s|Hitem:%d:%d:%d:%d|h[%s]|h%s`, and `0x52ad90` indexes the colour table (`jb` is unsigned). The reagent link has the same body: `0x4ff8da` requests, `0x4ff917` onward writes the name and
`0x4ff93c` builds. When `0x4ff8e3` finds no template the loop runs on and ends at `0x4ff902` with a nil. `0x4ff483` rejects a header row.

## Tools

`GetTradeSkillTools` (`0x4ff980`) returns a flat list of pairs, a name and a flag, for what the recipe needs near the player: first the spell focus object of the spell (`+0x3c`) with the flag 1 as a constant, then each of the two totem item slots (`+0xa0`, `+0xa4`) whose item has a template, with the flag 1 when the player has the item and nil when not. A missing focus, an empty slot or a missing template adds
no pair, so the count is 0 to 3 pairs. The template requests carry the callback `0x4fdc30`. It returns twice the pair count.

Addresses: `0x4ff980`, `0x4ffa51`, `0x4ffa62`, `0x4ffa76`, `0x4ffa8b`, `0x4ffa9c`, `0x4ffaa3`, `0x4ffade`, `0x4ffaf5`, `0x4ffb10`, `0x4ffb1b`, `0x4ffb29`, `0x4ffb47`, `0x4ffb51`, `0x622270`, `0x4ffa0c`, `0x4ffa1f`
Confidence: verified
Proof: one
Test: none

`0x4ffa51` loads the focus id and `0x4ffa62` finds the focus row in the table at `0xc0d800`; `0x4ffa76` to `0x4ffa9c` push its localized name (`+4` plus 4 for the locale) and the number 1 as an immediate (`push 0x3ff00000`, `0x4ffa8b`). `0x4ffaa3` starts the totem loop at `+0xa0`. `0x4ffade` requests the item with the callback; `0x4ffb10` asks the inventory for the item
(`0x622270`), `0x4ffb1b` pushes 1 and `0x4ffb29` nil. `0x4ffb51` doubles the pair count. `0x4ffa0c` and `0x4ffa1f` leave when the row or the player is missing.

## Subclasses and slots

`GetTradeSkillSubClasses` (`0x4ffb60`) pushes the localized subclass name of every header in header order, from the ItemSubClass row of the group (the second name when the first is empty), and returns the header count. `GetTradeSkillInvSlots` (`0x4ffc20`) pushes, for each set bit of the accumulated slot mask `0xbde058`, from bit 0 to bit 23, the global string named at `0x84dd70`, and returns how many it pushed.
Neither follows the filters.

Addresses: `0x4ffb60`, `0x4ffb87`, `0x4ffbd7`, `0x4ffbdd`, `0x4ffbea`, `0x4ffbf1`, `0x4ffbff`, `0x4ffc20`, `0x4ffc24`, `0x4ffc41`, `0x4ffc45`, `0x4ffc51`, `0x4ffc5a`, `0x4ffc66`, `0x84dd70`, `0x703bf0`, `0xc0db90`
Confidence: verified
Proof: one
Test: none

`0x4ffbd7` handles a header whose subclass row exists: `0x4ffbdd` picks `+0x4c`, `0x4ffbea` falls back to `+0x28`, and `0x4ffbf1` pushes it. `0x4ffbff` reloads the header count as the result even when a header found no row. `0x4ffc24` loads the mask, `0x4ffc45` reads the key for the bit, `0x4ffc51` fetches the string and `0x4ffc5a` pushes it; `0x4ffc66` returns the count.

## Filter functions

`SetTradeSkillSubClassFilter` (`0x4ffc70`) takes an index, an on flag and an optional exclusive flag. An index of 0 or below turns every subclass on. Otherwise the index is the 1-based header position: a bit is cleared by an on flag of 0, set alone by a nonzero exclusive flag, and added otherwise. `GetTradeSkillSubClassFilter` (`0x4ffd90`) returns 1 or nil for a header, and for an index of 0 or below 1 when every header is on.
`SetTradeSkillInvSlotFilter` (`0x4ffe60`) and `GetTradeSkillInvSlotFilter` (`0x4fffd0`) do the same for the slots; the index counts only the set bits of `0xbde058`, ascending. Each setter ends in a mask setter of `recipes` and so raises `TRADE_SKILL_UPDATE`.

Addresses: `0x4ffc70`, `0x4ffc7a`, `0x4ffcab`, `0x4ffcb0`, `0x4ffcbb`, `0x4ffcc3`, `0x4ffce7`, `0x4ffd12`, `0x4ffd25`, `0x4ffd4c`, `0x4ffd60`, `0x4ffd78`, `0x4ffd90`, `0x4ffdcd`, `0x4ffde0`, `0x4ffe0e`, `0x4ffe2a`, `0x4ffe60`, `0x4ffea3`, `0x4ffec0`, `0x4ffed9`, `0x4fff1c`, `0x4fff5d`, `0x4fff9b`, `0x4fffb6`, `0x4fffd0`, `0x500018`, `0x50006e`, `0x4fd710`, `0x4fd730`, `0x84dd60`, `0x84dd64`, `0x6f3510`, `0x6f34d0`
Confidence: verified
Proof: one
Test: none

A header position past the last header raises `Bad sub class in SetTradeSkillSubClassFilter` (`0x4ffcc3`), and a missing on flag raises `Missing on//off parameter in SetTradeSkillSubClassFilter` (`0x4ffce7`). The slot functions raise `Bad inv slot in SetTradeSkillInvSlotFilter` (`0x4ffed9`), `Missing on//off parameter in SetTradeSkillInvSlotFilter` (`0x4fff1c`), `Bad sub class in GetTradeSkillSubClassFilter` (`0x4ffe0e`) and
`Bad inv type in GetTradeSkillInvSlotFilter` (`0x50006e`). `0x4ffc7a` tests the index with `lua_isstring`, and `0x4ffcab` branches on the sign of index - 1; `0x4ffcb0` sets every bit. `0x4ffd12` to `0x4ffd25` clear the bit, `0x4ffd4c` to `0x4ffd60` set the lone bit and `0x4ffd78` adds it. The getter loops the headers from `0x4ffde0` and tests the bit at `0x4ffe2a`. The slot setter walks the 24 bits from `0x4ffec0` and calls `0x4fd730`
(`0x4fff5d`, `0x4fff9b`, `0x4fffb6`); the slot getter loops from `0x500018`.

## Collapse functions

`CollapseTradeSkillSubClass` (`0x5000c0`) and `ExpandTradeSkillSubClass` (`0x500140`) take the index of a header row of the list. An index of 0 or below collapses or expands every group by setting the collapse mask to 0 or to all ones. Otherwise the row must be a header row, and its group's bit is cleared or set. A row that is not a header raises `Bad sub class in CollapseTradeSkillSubClass`
or `Bad skill line in ExpandTradeSkillSubClass`. Both end in the mask setter `0x4fd750`.

Addresses: `0x5000c0`, `0x5000f5`, `0x5000f9`, `0x500104`, `0x50010b`, `0x500133`, `0x500140`, `0x500177`, `0x50017a`, `0x500185`, `0x5001b2`, `0x4fd650`, `0x4fd750`
Confidence: verified
Proof: one
Test: none

`0x5000f5` branches on the sign of the index; `0x5000f9` stores mask 0 and `0x50017a` mask all ones. `0x500104` and `0x500185` call the header-position function `0x4fd650`, which returns -1 for a row that is not a header; the following `jge` at `0x50010b` and `0x50018c` raise the errors. `0x500133` clears the bit and `0x5001b2` sets it.
