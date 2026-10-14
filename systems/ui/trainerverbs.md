# ui: trainer window getters and filters

The Lua functions of `ClassTrainerFrame.cpp` that open and close the window, describe a service row (cost, level, skill,
icon, description, kind) and read or move the filters. The state, the records, the selection and the first group of getters
are `ui/trainer`; which rows show and in what order is `ui/trainerlist`. Every getter here takes a 1-based row, decrements it
and fetches the record with `0x4d89b0`, which answers null at or past the total length, so a missing row gives the
nil, 0 or empty answers named below. The spell store (`0xc0d788`) and the skill line store (`0xc0d92c`) are read by id with
a signed range test.

Roots: `0x4d8cf0`, `0x4d8ea0`, `0x4d8ed0`, `0x4d8f50`, `0x4d9160`, `0x4d92f0`, `0x4d9410`, `0x4d9930`, `0x4d9b40`, `0x4da030`

## Opening and the type tests

`OpenTrainer()` asks the server for the trainer list with the player's own GUID, through `TalkToTrainer`; it does nothing
without a player. `CloseTrainer()` closes the window with a zero GUID. `IsTradeskillTrainer()` returns 1 when the trainer type is
2, else nil. `IsTalentTrainer()` returns 1 when no trainer is open or the type is 1 and nil otherwise, so it answers 1 with the
window closed. `GetTrainerGreetingText()` returns the greeting buffer (0xb737b8).

Addresses: `0x4d8cf0`, `0x4d8d3c`, `0x4d8d50`, `0x4d8d76`, `0x4d8ea0`, `0x4d8ed0`, `0x4d8ed5`, `0x4d8edb`, `0x4d8ef1`, `0x4d8f40`, `0x5df680`, `0x4d7440`
Confidence: verified
Proof: one
Test: none

`0x4d8d3c` calls `TalkToTrainer` with a pointer to the first two descriptor words of the player, its GUID. `0x4d8d76` calls the open-or-close routine with a zero GUID and type 0. `0x4d8ed5` ors the two GUID words and `je` at `0x4d8edb` jumps to the push of 1.0 at `0x4d8ef1` when both are zero.

## Cost and level

`GetTrainerServiceCost(row)` returns three numbers: the money cost (record `+0x08`) and the two point costs (`+0x0c`, `+0x10`),
all 0 for a missing row. `GetTrainerServiceLevelReq(row)` returns the required level, the byte at `+0x14`, or 0. A
non-number raises the usage text with the function's own name.

Addresses: `0x4d92f0`, `0x4d9330`, `0x4d9339`, `0x4d93a0`, `0x4d93e5`, `0x4d93ee`
Confidence: verified
Proof: one
Test: none

`0x4d9339` to `0x4d9348` load the three cost fields and `0x4d9350` zeroes them for a null record. `0x4d93ee` loads the level byte.

## Skill requirement

`GetTrainerServiceSkillReq(row)` returns the localized skill line name, the required value and 1 when the player meets it, else
nil. The line is record `+0x18` and the value `+0x1c`; both must be nonzero and the line in the store. The player's rank is found
by scanning the 128 skill slots (12 bytes each at `+0x848` of the field block) for the line and adding the slot's bonus half to
its rank. A row without the requirement or no player gives nil, 0, 1; an empty name gives nil, 0 and the flag.
`GetTrainerServiceStepReq(row)` returns nil and the number 1 for every row, because the spell id at record `+0x20` is always 0.

Addresses: `0x4d9410`, `0x4d9495`, `0x4d94a0`, `0x4d94d6`, `0x4d950b`, `0x4d9517`, `0x4d9526`, `0x4d9558`, `0x4d9570`, `0x4d957c`, `0x4d95bc`, `0x4d95d9`, `0x4d9930`, `0x4d99b3`, `0x4d99b8`, `0x4d9afc`
Confidence: verified
Proof: one
Test: none

`0x4d9495` and `0x4d94a0` test the two fields for zero, `0x4d94d6` to `0x4d9524` scan the slots by line id (`cmp eax,0x600` ends it), `0x4d9558` loads the rank and `0x4d9563` the bonus, and `0x4d9570` compares the sum with the value. The default flag of 1 (`0x4d945a`) survives every early exit, so they push 1.0 at `0x4d95d9`. `0x4d99b3` loads `+0x20` and `0x4d99b8` jumps to `0x4d9afc` when it is 0; that pushes a nil, then `0x4d9b09` pushes the double with high word `0x3ff00000` and low word the spell id 0, which is 1.0. The caller zeroes the source array (`0x5dcc11`), so the id is always 0; `edi`, 1 from `0x4d997a`, differs from it at `0x4d9b03`.

## Skill line name

`GetTrainerServiceSkillLine(row)` returns the localized name of the skill line of the row's ability, found through the player's
race and class and the skill line ability table: through the spell a learn effect (0x24 or 0x39) teaches, otherwise through the
sent spell itself. It returns nil without a player or a match. A row with no learn effect and no ability row of its own falls
through to a lookup that indexes the spell's trigger-spell array at -1, which reads the third effect's misc value (`+0x1b0`).

Addresses: `0x4d9160`, `0x4d91dc`, `0x4d91e6`, `0x4d91eb`, `0x4d91fb`, `0x4d9238`, `0x4d925f`, `0x4d9261`, `0x4d9292`, `0x4d92bb`, `0x6de040`
Confidence: verified
Proof: one
Test: none

`0x4d91b4` presets the effect index to -1 and `0x4d91fb` replaces it with the first effect found. `0x4d9238` calls `FindSkillLineAbility` (spell, race, class) on the sent spell when the index is -1 and `0x4d925f` takes the name when that finds a line. `0x4d9261` reads `[spell + index * 4 + 0x1b4]`, the trigger spell, which with -1 is the dword before the array, `EffectMiscValue[2]` at `+0x1b0`. `0x4d92bb` reads the name at `+0xc` by locale.

## Icon

`GetTrainerServiceIcon(row)` returns an icon path. For a type 2 trainer whose sent spell teaches another by a learn effect
and whose taught spell creates an item, it is that item's inventory icon (nil while the item record is not cached); in
every other case it is the sent spell's icon from the icon store.

Addresses: `0x4d8f50`, `0x4d8fe2`, `0x4d9008`, `0x4d9050`, `0x4d906a`, `0x4d9096`, `0x4d90fa`, `0x4d9119`
Confidence: verified
Proof: one
Test: none

`0x4d8fe2` tests trainer type 2, `0x4d9050` follows the trigger spell and `0x4d906a` loads the created item at `+0x19c`; `0x4d9096` looks the item up and `0x4d90fa` formats `%s%s%s`. `0x4d9008` is the spell icon path (`+0x1d4`).

## Description

`GetTrainerServiceDescription(row)` returns the expanded description of the sent spell. For a type 1 trainer's used row, or a
sent spell without a description, it first tries the spell named by the learn effect: its description, else the description of
the item that spell creates when it has attribute 0x20 and a create-item effect (0x18). It then falls back to the sent spell's
description and gives nil when that is empty too.

Addresses: `0x4d9b40`, `0x4d9c1a`, `0x4d9c2c`, `0x4d9c32`, `0x4d9c6f`, `0x4d9ca3`, `0x4d9cd0`, `0x4d9cd6`, `0x4d9d10`, `0x4d9d62`, `0x4d9d6f`, `0x4d9d90`, `0x5075f0`
Confidence: verified
Proof: one
Test: none

`0x4d9c1a` and `0x4d9c2c` test type 1 and state 2, `0x4d9c32` the sent description at `+0x228`, `0x4d9ca3` the taught one, `0x4d9cd0` and `0x4d9cd6` the attribute bit 0x20 at `+0x18` and effect 0x18 (create item), `0x4d9d10` the item description at `+0x198`, and `0x4d9d6f` `ExpandSpellText`.

## Row kind tests

`IsTrainerServiceSkillStep(row)` returns 1 when the sent spell has an effect 0x2c (skill step), else nil.
`IsTrainerServiceLearnSpell(row)` returns 1 and nil when a learn effect (0x24) teaches a missing spell or one without attribute
0x20, 1 and 1 for a learn pet spell effect (0x39), else nil and nil. `IsTrainerServiceTradeSkill(row)` returns 1 when a learn
effect teaches a spell with attribute 0x20, else nil. `GetTrainerServiceStepIncrease(row)` returns, for a row with a skill step
effect that is not in state 2, `INCREASE_POTENTIAL` formatted with the skill line name and five times the effect's base points
less those of the spell at record `+0x20` (0 in effect); nil when that is 0 or a link is missing.

Addresses: `0x4d9dd0`, `0x4d9e33`, `0x4d9e4f`, `0x4d9e70`, `0x4d9ed9`, `0x4d9efd`, `0x4d9f03`, `0x4d9f28`, `0x4d9f46`, `0x4d9f70`, `0x4d9fe0`, `0x4d9ffa`, `0x4da019`, `0x4da030`, `0x4da0c0`, `0x4da0eb`, `0x4da13d`, `0x4da15b`, `0x4da190`, `0x4da1c9`, `0x6e3760`, `0x703bf0`
Confidence: verified
Proof: one
Test: none

`0x4d9efd` tests attribute 0x20 of the taught spell and `je` at `0x4d9f01` goes to the success block `0x4d9f28`, so a learn-spell answer is 1, nil for an unflagged spell. `0x4d9ffa` jumps to the success push when it is set. `0x4da0eb` refuses state 2. `0x4da13d` reads the previous step's points (`0x6e3760`), `0x4da15b` subtracts, and `0x4da1b1` multiplies by five for `SStrPrintf`.

## Filter and group getters

`GetTrainerServiceTypeFilter("type")` returns 1 when the state bit of `available`, `unavailable` or `used` is set, else nil;
`all` or another name raises `Bad service type in GetTrainerServiceTypeFilter`. `GetTrainerSkillLineFilter(n)` returns 1 when
group n - 1 is shown (for n below 1, when every group is) and raises `Bad skill line in GetTrainerSkillLineFilter` past the
count. `GetTrainerSkillLines()` pushes the localized skill line name stored at each group's key, nil for a key without one, and
returns the group count. `CollapseTrainerSkillLine(n)` and `ExpandTrainerSkillLine(n)` clear or set the expanded bit of the group
whose header is row n - 1 of the full array, or the whole mask for n below 1; a non-header row raises `Bad skill line in ...`.

Addresses: `0x4da510`, `0x4da541`, `0x4da569`, `0x4da590`, `0x4da5cd`, `0x4da606`, `0x4da60e`, `0x4da660`, `0x4da683`, `0x4da6bf`, `0x4da6c0`, `0x4da6f9`, `0x4da704`, `0x4da70d`, `0x4da733`, `0x4da740`, `0x4da77a`, `0x4da785`, `0x4da78e`, `0x4d8be0`, `0x4d8cd0`, `0x4da390`
Confidence: verified
Proof: one
Test: none

`0x4da546` rejects the mapper answer 6. `0x4da5cd` branches on the decremented number: a negative one scans every group bit (`0x4da5e0`). `0x4da606` bounds a group number by the group count. `0x4d8be0` returns the group ordinal of a header row, or -1 for a service row or an index at or past the total. `0x4da6f9` stores a zero mask and `0x4da77a` an all-ones mask.

## Setting the skill line filter

`SetTrainerSkillLineFilter(n [, on, exclusive])` sets the filter to all groups for n below 1. Otherwise group n - 1 must be
below the group count (`Bad skill line in SetTrainerSkillLineFilter`) and `on` must be a number (`Missing on//off parameter in
SetTrainerSkillLineFilter`): zero clears the group's bit, a nonzero value with a nonzero `exclusive` makes it the only bit, and any other nonzero value adds it.
Each change runs the finalizer and signals TRAINER_UPDATE through the store routine.

Addresses: `0x4da3f0`, `0x4da42b`, `0x4da430`, `0x4da43b`, `0x4da457`, `0x4da492`, `0x4da4a5`, `0x4da4d1`, `0x4da4e0`, `0x4da4f8`, `0x4d8cb0`
Confidence: verified
Proof: one
Test: none

`0x4da430` stores an all-ones mask. `0x4da49d` to `0x4da4a5` clear the bit, `0x4da4d7` to `0x4da4e0` store the single bit and `0x4da4ed` to `0x4da4f8` or it in.
