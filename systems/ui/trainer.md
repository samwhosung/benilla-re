# ui: the trainer window

The trainer list the server sends, the records it becomes, the Lua functions that read them, the selection and the
purchase. This is `ClassTrainerFrame.cpp` (`0x4d7260` to `0x4da860`). Which rows show, in what order and in which state
is `ui/trainerlist`; the packet is read by `objectlayer/units` (`0x5dca60`).

Roots: `0x4d7440`, `0x4d7560`, `0x4d74f0`, `0x4d7520`, `0x4d89b0`, `0x4d8cf0`, `0x4d8d50`, `0x4d8dc0`, `0x4d8e60`, `0x4d9600`, `0x4d96e0`, `0x4da210`

## Trainer state

The window keeps its state in a few cells.

| cell | holds |
|---|---|
| `0xb73a00` | the open trainer's GUID (u64), 0 for none |
| `0xb73a08` | the trainer type: 0 class, 1 talent or mount, 2 tradeskill, 3 pet |
| `0xb73a0c` | the selection, a spell id |
| `0xb737b0`, `0xb73a10` | the array of pointers to 0x38-byte service records, and its total length |
| `0xb739c0`, `0xb73a14` | the array of group headers and its length |
| `0xb73a18` | the visible count, what `GetNumTrainerServices` returns |
| `0xb73a1c`, `0xb73a20`, `0xb73a24` | the state filter, the skill-line filter and the expanded mask |

Addresses: `0x4d7440`, `0x4d73e0`, `0x4d7400`, `0x4d74e9`, `0x4d7463`, `0x4d7486`, `0x4d8d50`, `0x4d8cf0`, `0xb73a00`, `0xb73a08`, `0xb73a0c`, `0xb737b0`, `0xb73a10`, `0xb739c0`, `0xb73a14`, `0xb73a18`, `0xb73a1c`, `0xb73a20`, `0xb73a24`
Confidence: verified
Proof: one
Test: none

`0x4d7440(guid, type)` opens or closes the window: the same GUID again returns at once (`0x4d74e9`); a zero GUID signals
`TRAINER_CLOSED` (0x137), ends the interaction and zeroes the GUID, header count and total length but not the selection
(`0x4d7463` to `0x4d7486`); another GUID becomes the interaction target and is stored with the type. A new GUID first
raises tutorial 0xc when it is the player's own GUID and 0xd for any other (`0x4d7492` to `0x4d74ae`, `0x4b54c0`), and
the own GUID sets no interaction target (`0x4d74c3`). The close signals before it ends the interaction
(`0x4d7468`, then `0x4931c0` at `0x4d7474`); when the NPC is still the interaction target that call runs the
dispatcher `0x493310`, which zeroes the target and re-enters the close with a zero GUID (`0x493434` to `0x49343f`) while the trainer GUID `0xb73a00` is still stored, so a script close
signals `TRAINER_CLOSED` twice. `0x4d73e0` is the
world-exit reset of the GUID, selection and total.

## Service records

A service record is 0x38 bytes. The server's fields are copied in; the window adds the group key, the state it derives
and the visible flag.

| offset | field |
|---|---|
| +0x00 | the taught spell's id as sent (a group header holds -1) |
| +0x04 | the group key |
| +0x08 | money cost, copper |
| +0x0c, +0x10 | the two character-point costs |
| +0x14 | required level (u8) |
| +0x18, +0x1c | required skill line and required skill value |
| +0x20 | always 0 |
| +0x24, +0x28, +0x2c | required ability spell ids; 0 for unused |
| +0x30 | state (u8): 0 available, 1 unavailable, 2 used, 3 only for a type 1 trainer |
| +0x34 | visible flag |

Addresses: `0x4d7560`, `0x4d76cd`, `0x4d76de`, `0x4d76f9`, `0x4d7713`, `0x4d7725`, `0x4d7737`, `0x4d7749`, `0x4d7764`, `0x4d777e`, `0x4d780c`, `0x4d84c6`, `0x4d8ba0`, `0x4d89b0`
Confidence: verified
Proof: one
Test: none

The builder's nine stack arguments are the wire arrays, one entry per record. `0x4d89b0(i)` returns record `i` or null at the
total length, unsigned. The state names come from `0x4d8ba0`: `available` for 0, `used` for 2, `unavailable` for the rest,
`header` for a header row.

## List arrival

`SMSG_TRAINER_LIST` reads a u64 GUID, a u32 trainer type and a u32 count, then per entry: the spell id u32, the state u8,
the cost u32, the two point costs i32, the required level u8, the required skill line and value u32, and three
required ability ids i32. The handler builds records from every entry; a packet whose GUID is not the current target
builds nothing. The builder sets the filters, creates the group headers, drops rows whose group key is 0, finalizes the list
and selects row 0. It then signals `TRAINER_SHOW` (0x135).

Addresses: `0x5dca60`, `0x5dcba9`, `0x5dcbb6`, `0x5dcc11`, `0x5dcc85`, `0x5dccb5`, `0x5dcce7`, `0x4d7560`, `0x4d75d9`, `0x4d75e5`, `0x4d75ef`, `0x4d7807`, `0x4d79e6`, `0x4d7b35`, `0x4d7b3b`, `0x4d7b42`, `0x4d7b6e`
Confidence: verified
Proof: one
Test: none

The handler fills eleven arrays in one loop with no skip; the twelfth, behind record `+0x20`, is zeroed. The builder resets the
skill-line filter and the expanded mask to all ones for every packet and the state filter as `ui/trainerlist#filters`
says. A key of exactly 0 drops the row: the slot is reused and the total is the wire count less the dropped
(`0x4d7807` to `0x4d79e6`). The tail calls the finalizer (`0x4d7b3b`) and then the selection with row 0 (`0x4d7b42`).

## Group keys

The group key of a row says which header it sits under. At trainer type 2 it is 1 when the taught spell has a skill-step
effect (effect 0x2c) and 2 otherwise, so a row is never dropped. At type 1 a row in state 2 gets -1, the pre-made known
group. Every other row gets the skill line of the ability it teaches, found through the taught spell's learn effect (0x24,
or 0x39 through the pet) and the skill-line ability table by race and class; a row with no such line has key 0 and is
dropped. Header labels: type 2 resolves `TRADESKILL_SERVICE_STEP` and `TRADESKILL_SERVICE_LEARN`, group -1
`KNOWN_TALENTS_HEADER`, the rest the skill line's display name.

Addresses: `0x4d7c60`, `0x4d7786`, `0x4d7791`, `0x4d77b6`, `0x4d77c0`, `0x4d77d0`, `0x4d77da`, `0x4d77e2`, `0x4d77e8`, `0x4d77f1`, `0x4d7d12`, `0x4d8aa0`, `0x4d8af3`, `0x4d8b11`, `0x4d8b1f`, `0x60c920`, `0x703bf0`
Confidence: verified
Proof: one
Test: none

At type 2 the key defaults to the type value 2 (`0x4d7794`) and becomes 1 on an effect 0x2c (`0x4d77c0`); the taught spell's
skill line plays no part. `0x4d7c60` returns 0 for an id below 0 or above the highest, a missing record, no player, a pet
leg without a pet, or no ability row. The header labels are keys passed to the global string lookup `0x703bf0`, so Lua
receives the localized text.

## Selection

The selection is the spell id of the chosen record. `SelectTrainerService(index)` truncates the index, subtracts 1 and
stores that record's id; an index below 1 or at the total length or above clears the selection. A hidden record can be
selected. `GetTrainerSelectionIndex()` searches the whole array for the stored id and returns its 1-based position, or
0 when the id is 0, the array is empty or the id is absent. A trained row leaves the visible list but stays in the array,
so the index it returns is then above `GetNumTrainerServices()`. A group header holds -1, so selecting one reads back the
first header.

Addresses: `0x4d8e60`, `0x4d8e94`, `0x4d8e95`, `0x4d74f0`, `0x4d74f6`, `0x4d750d`, `0x4d8f10`, `0x4d8f1c`, `0x4d7520`, `0x4d7527`, `0x4d752b`, `0x4d7543`, `0x4d754c`, `0x4d7551`, `0x4d7b42`, `0x4d73ec`
Confidence: verified
Proof: one
Test: none

The helper `0x4d74f0` compares with the total length unsigned (`0x4d74f0`), clears the cell otherwise (`0x4d74f8`) or
stores `[record+0]` (`0x4d750d`). The search `0x4d7520` scans to the total, not the visible count. Four references touch
the cell: the reset (`0x4d73ec`), the two writes of `0x4d74f0` and the search; only the list builder and the reset
write it outside Lua. A non-number raises `Usage: SelectTrainerService(index)`.

## Index law

Every trainer getter accepts any index up to the total length, not the visible count. `GetTrainerServiceInfo(i)` returns
the name, the rank text, the state name and whether the row is expanded; past the total it returns `nil, nil, nil, 1`. The
other getters (`GetTrainerServiceIcon`, `...Cost`, `...LevelReq`, `...SkillReq`, `...NumAbilityReq`, `...AbilityReq`,
`...StepReq`, `...Description`, `IsTrainerServiceSkillStep`, `...LearnSpell`, `...TradeSkill`, `...StepIncrease`) return nil
or 0 past it. `BuyTrainerService(index)` sends the purchase of one row (opcode 0x1b2: the trainer GUID and the row's spell id) when the
row exists and its state is 0; any other state is refused silently. An index of 0 or less buys every visible row.

Addresses: `0x4d89b0`, `0x4d8dc0`, `0x4d8aa0`, `0x4d8b50`, `0x4d8ba0`, `0x4d8c30`, `0x4d8be0`, `0x4d8d90`, `0x4d8d96`, `0x4da210`, `0x4da23f`, `0x4da245`, `0x4da247`, `0x4da252`, `0x4d89d0`, `0x4d89e5`, `0x4d8a70`
Confidence: verified
Proof: one
Test: none

The compare is unsigned, so 0 and negative indices behave as past the end. Only `GetNumTrainerServices` and the buy-all
loop read the visible count (`0x4d8d96`, `0x4d8a70`). `GetTrainerServiceInfo` always returns four values: the siblings
share the gate, a null string pushes nil, and the fourth gives 1.0 past the end. Buying branches on a signed test
(`0x4da245 jns`); the single-row path `0x4d89d0` refuses a record whose state byte is not 0 (`0x4d89e5`) and never reads the visible flag.

## Names and ranks

A row's name is the localized name of the sent spell and its rank text the sent spell's rank text; neither hops to the
spell the sent spell teaches. The icon getter, which does hop for a type 2 trainer, is a separate resolver.

Addresses: `0x4d8aa0`, `0x4d8acc`, `0x4d8ad8`, `0x4d8b50`, `0x4d8b63`, `0x4d8b84`, `0x4d8f50`
Confidence: verified
Proof: one
Test: none

`0x4d8aa0` reads `[Spell record + 0x1e0 + locale*4]` for the id at `row+0`, and `0x4d8b50` `+0x204` of the same record, an empty
string for a header. Neither reads the trigger-spell word (`+0x1b4`).

## Required abilities

`GetTrainerServiceNumAbilityReq(i)` counts the positive entries of the row's three required ability ids.
`GetTrainerServiceAbilityReq(i, k)` returns for entry k a text and a flag: the spell's name and rank as `Name (Rank)`, or
the name alone when the rank is empty, and whether the player (or the pet for a service that teaches a pet spell) knows
that ability or a higher rank. The flag defaults to true when the spell, the unit or the record cannot be found.

Addresses: `0x4d9600`, `0x4d963d`, `0x4d964b`, `0x4d9653`, `0x4d96e0`, `0x4d9731`, `0x4d975f`, `0x4d979c`, `0x4d97a5`, `0x4d97d4`, `0x4d97ed`, `0x4d981c`, `0x4d9821`, `0x4d9848`, `0x4d9860`, `0x4d9892`, `0x4d98a5`, `0x4d98ae`, `0x60c740`, `0x60c8d0`
Confidence: verified
Proof: one
Test: none

The text is `%s (%s)` of the localized name at `+0x1e0` and rank at `+0x204`; the flag starts as 1 and is cleared at one
site (`0x4d98ae`) when neither `0x60c740` (known) nor `0x60c8d0` (a higher rank known) holds. The unit is the player unless
the service's own spell has effect 0x39, then the unit named by the player's summon field (`+0x110` view). Both indices
are 1-based and the second must be below 3.
