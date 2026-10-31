# spell: failures

Cast failure reasons: the reporter that turns a reason into a message, a combat-log line and an
event, and the four failure packets.

Every refusal of the player's cast reaches one reporter in `Spell_C.cpp`, `0x6e1a00`: the client's
own pre-send checks in the unit code and in spell (`checks`, `cast`), and the `SMSG_CAST_RESULT`
handler (`handlers`). It takes a spell id, a reason byte, two argument words and a from-server flag.
It picks a message id of the UI message catalogue (`tables/messages.tsv`), builds the reason's text
from its `SPELL_FAILED_*` GlobalStrings key, fills the text from a DBC or the item cache, shows it,
writes a combat-log line, and fires `SPELLCAST_FAILED` or `SPELLCAST_INTERRUPTED`. Two item-cache
callbacks show a failure again once its item's name arrives. The same file handles
`SMSG_SPELL_FAILURE`, `SMSG_SPELL_FAILED_OTHER`, `SMSG_PET_CAST_FAILED` and
`SMSG_PET_TAME_FAILURE`; the pet's handler has its own message ids and never logs or fires an
event. The topic owns no variable: it reads the in-flight spell and the auto-repeat spell (`cast`), the
spell history (`cooldowns`) and the DBC stores. Its switch tables are `.text` data and its two
power tables are `.rdata`.

Roots: `0x6e1a00`, `0x6e6a20`, the cache callbacks `0x6e29b0`, `0x6e2a90`, and the packet handlers `0x6e8d80`, `0x6e8e40`, `0x6e8eb0`, `0x6e97e0`

## Reporter inputs and first effects

`0x6e1a00` is `fastcall(spell, reason)` with three stack arguments and `ret 0xc`: two words (−1
when there is none) and a byte that is nonzero only from `SMSG_CAST_RESULT`. It looks the spell up
in `Spell.dbc` by id. First, for every reason: when the spell is the trade-skill repeat spell,
`0x4fd790` clears that spell and fires `UPDATE_TRADESKILL_RECAST` (502). Then, when the record and
the active player exist, `0x458a50` takes the spell and the player as a unit and passes the id in
field 2 of the `Resistances.dbc` row named by the spell's school, field 1, to the sound call
`0x458870`. After the
reason's text is built, the from-server byte alone calls `0x6e1630` (`cooldowns`) with the spell.

Addresses: `0x6e1a00`, `0x6e1a26`, `0x6e1a3c`, `0x6e1a55`, `0x6e1a70`, `0x6e1a86`, `0x6e1aa6`, `0x6e1d7c`, `0x6e1d83`, `0x6e1d89`, `0x6e2257`, `0x6e73ac`, `0x4fd790`, `0x458a50`, `0x6e1630`, `0x468460`, `0x468550`, `0x458870`
Confidence: verified
Proof: one
Test: none

`ecx` goes to `[ebp-0xc]` and `dl` to `[ebp-0x4]`; the stack arguments are `[ebx+0x8]`, `[ebx+0xc]`
and the byte `[ebx+0x10]`; both exits are `ret 0xc` (`0x6e2225`, `0x6e2257`). `0x6e1a26` and
`0x6e1a2e` bound the id by the store's maximum, signed, before the id map read. `0x6e1a3c` compares
the spell with the dword `0x4fd790` tests, and `0x4fd790` zeroes it and fires 0x1f6 through
`0x703e50`. `0x6e1a70` asks `0x468460` for the `0x468550` GUID as a type 0x10 object; with it and the
record, `0x6e1a9c` gets it as a type 8 unit and `0x6e1aa6` calls `0x458a50`, which reads the record's
`+0x04`, the `Resistances.dbc` row of that id, and passes its `+0x08` with the result of the unit's slot 5 to `0x458870`
(`0x458aa6`).
Census of the 63 calls to `0x6e1a00` (`e8` scan): each pushes byte 0, through a zeroed register at
two of them, except `0x6e73ac`, which pushes 1. `0x6e1d7c` tests the byte; `0x6e1d83` pushes the spell and `0x6e1d89`
calls `0x6e1630`.

## Message id per reason

The message id starts at 0x2c (`ERR_SPELL_FAILED_S`). When the record exists, these reasons change
it; the rest keep 0x2c. Each id is a UI error message except 0x140, a UI info message.

| reason | key suffix | id | message |
|---|---|---|---|
| 0x01 | `ALREADY_AT_FULL_HEALTH` | 0x18e | `ERR_SPELL_FAILED_ALREADY_AT_FULL_HEALTH` |
| 0x02 | `ALREADY_AT_FULL_POWER` | 0x18f | `ERR_SPELL_FAILED_ALREADY_AT_FULL_POWER_S` |
| 0x09 | `BAD_IMPLICIT_TARGETS` | 0xb8 | `ERR_GENERIC_NO_TARGET` |
| 0x18 | `EQUIPPED_ITEM` | 0xd4 | `ERR_SPELL_FAILED_EQUIPPED_ITEM` |
| 0x19 to 0x1b | `EQUIPPED_ITEM_CLASS…` | 0xd5 | `ERR_SPELL_FAILED_EQUIPPED_ITEM_CLASS_S` |
| 0x2f | `NEED_AMMO` | 0x133 | `ERR_NOAMMO_S` |
| 0x41 | `NOT_UNSHEATHED` | 0x140 | `ERR_SPELL_FAILED_NOTUNSHEATHED` |
| 0x56 | `ONLY_SHAPESHIFT` | 0xd6 | `ERR_SPELL_FAILED_SHAPESHIFT_FORM_S` |
| 0x59 | `OUT_OF_RANGE` | 0x131 | `ERR_SPELL_OUT_OF_RANGE` |
| 0x5c | `REAGENTS` | 0xd2 | `ERR_SPELL_FAILED_REAGENTS` |
| 0x78 | `TOTEMS` | 0xd1 | `ERR_SPELL_FAILED_TOTEMS` |
| 0x8e | `PLAY_TIME` | 0x1bf | `ERR_PLAY_TIME_EXCEEDED` |

Addresses: `0x6e1a45`, `0x6e1a7c`, `0x6e1aab`, `0x6e1b48`, `0x6e1b6c`, `0x6e1b7a`, `0x6e1b88`, `0x6e1b96`, `0x6e1bb1`, `0x6e1bc2`, `0x6e1bd3`, `0x6e1be4`, `0x6e1c10`, `0x6e1c21`, `0x6e1c32`, `0x6e1c43`, `0x6e1c58`, `0x6e1c5f`
Confidence: verified
Proof: one
Test: none

`0x6e1a45` stores 0x2c in `[ebp-0x8]`. A null record jumps from `0x6e1a7c` past the chain to
`0x6e1c64`. From `0x6e1aab` the chain compares `cl` with each reason and stores the id: 0x78 →
0xd1 (`0x6e1b6c`), 0x5c → 0xd2, 0x18 → 0xd4, 0x19, 0x1a and 0x1b → 0xd5 (`0x6e1c58`), 9 → 0xb8,
0x59 → 0x131, 0x2f → 0x133, 0x56 → 0xd6, 0x41 → 0x140, 1 → 0x18e, 2 → 0x18f, 0x8e → 0x1bf
(`0x6e1c43`); every store returns to `0x6e1b1b`, the last from `0x6e1c5f`. Keys and kinds are the catalogue's rows of those ids (`tables/messages.tsv`): kind 2,
the UI error message, for all but 320 (0x140), kind 1.

## Cooldown, item and target ids

Three reasons choose by the record. 0x3c `NOT_READY`: field 2 (`+0x08`) of 0xa or 0xb gives 0x2f
`ERR_FOOD_COOLDOWN`, 4 or 9 gives 0x2e `ERR_POTION_COOLDOWN`, else bit 4 of `Attributes` (`+0x18`)
gives 0x31 `ERR_ABILITY_COOLDOWN` and its absence 0x30 `ERR_SPELL_COOLDOWN`. When argument 1 is 0,
0x3c also asks the spell history `0x6e1690` about the spell and, on a nonzero answer, calls
`0x6e3050` (`cooldowns`). 0x28 `ITEM_NOT_READY`: field 2 of 4 or 9 gives 0x2e, else 0x2d
`ERR_ITEM_COOLDOWN`. 0x0a `BAD_TARGETS`: `Targets` (`+0x34`) bit 4 gives 0x13c
`ERR_INVALID_ITEM_TARGET`, else 0x2c.

Addresses: `0x6e1ab7`, `0x6e1ace`, `0x6e1ae1`, `0x6e1aea`, `0x6e1af1`, `0x6e1b02`, `0x6e1b13`, `0x6e1b4d`, `0x6e1bfa`, `0x6e1c04`, `0x6e1690`, `0x6e3050`
Confidence: verified
Proof: one
Test: none

At `0x6e1ab7` `[edi+0x8]` is compared with 0xa and 0xb (→ `0x6e1aea`, 0x2f) and with 4 and 9 (→
`0x6e1ae1`, 0x2e); otherwise `0x6e1ace` computes `(([edi+0x18] & 0x10) | 0x300) >> 4`, 0x30 or
0x31. `0x6e1af1` skips the history unless `[ebx+0x8]` is 0; `0x6e1b02` calls `0x6e1690(spell, 0)`
and `0x6e1b13` calls `0x6e3050` with the spell, 0, 0 and 1. `0x6e1b4d` reads the same field for
0x28 (0x2e or 0x2d). `0x6e1bfa` is `test byte [edi+0x34],0x10`, storing 0x13c at `0x6e1c04`. Field
names are the column positions `+0x18 / 4` = 6 and `+0x34 / 4` = 13 of `Spell.dbc`.

## Player unit cleanup

When the active player exists, the reporter first, if the spell equals its unit's `+0xc8c`, calls
`0x60d040(spell, 1, 1)`, `0x614150(spell, 0)` and `0x5fd9e0(−1)` on the unit. Then, whatever
`+0xc8c` holds, reason 0x2f calls `0x4b5390(0x2c, 0)` and `0x60f530`; 0x30, 0x31, 0x59, 0x76 and 0x7c call `0x60f530`
alone; other reasons call neither. When the spell is the auto-repeat spell, reason 0x17
`DONT_REPORT` returns at once and every other reason calls `0x6ea080` (`cast`). After this,
reason 0x17 returns in every case: no text, message, log line or event.

Addresses: `0x6e1c67`, `0x6e1c7f`, `0x6e1c8e`, `0x6e1c98`, `0x6e1ca1`, `0x6e1cb5`, `0x6e1cbc`, `0x6e1cca`, `0x6e1cd1`, `0x6e1cd9`, `0x6e1ce4`, `0x6e1cea`, `0x6e1cfa`, `0x6e225c`, `0x6e2268`, `0x614150`, `0x5fd9e0`, `0x4b5390`, `0x60f530`, `0x6ea080`, `0xceac30`
Confidence: verified
Proof: one
Test: none

`0x6e1c67` requires the player and bit 4 of its type word (`[[esi+0x8]+0x8]`), the bit
`0x468460` already demanded. `0x6e1c7f` compares `[esi+0xc8c]`, and `0x6e1c85 jne 0x6e1ca9` skips only the three calls: the
switch and the auto-repeat test follow on both paths. The switch subtracts 0x2f, bounds
by 0x4d and indexes the byte table `0x6e2268` (78 bytes) into the three-entry table `0x6e225c`:
0x2f → `0x6e1cc3`, the five reasons above → `0x6e1ccf`, every other byte → `0x6e1cd9`.
`0x4b5390` tests and sets bit 0x2c of a bit set (`0x4b53c5`, `0x4b53de`). `0x6e1cd9` compares
which the auto-repeat start writes before `START_AUTOREPEAT_SPELL` (`0x6e594c`) and
its stop clears before `STOP_AUTOREPEAT_SPELL` (`0x6ea16a`); `0x6e1ce4` and `0x6e1cfa` jump to the
epilogue `0x6e224f` on 0x17.

## Reason key switch

`0x6e23e0` maps a reason byte to its GlobalStrings key. Reason r from 0 to 0x90 returns the string
r places below the top of a run of 146 strings, each padded to 4 bytes, that
descends from `0x870b64` (0, `SPELL_FAILED_AFFECTING_COMBAT`) to `0x86fab8` (0x90,
`SPELL_FAILED_MIN_SKILL`); any larger byte returns the run's lowest string, `0x86faa0`,
`SPELL_FAILED_UNKNOWN`. Every key is `SPELL_FAILED_` and a name; the 146 are distinct. Keys this
topic relies on: 0x17 `DONT_REPORT`, 0x23 `INTERRUPTED`, 0x24 `INTERRUPTED_COMBAT`, 0x3c
`NOT_READY`, 0x4d `NO_POWER`, 0x61 `SPELL_IN_PROGRESS`, 0x84 `PROSPECT_NEED_MORE`.

Addresses: `0x6e23e0`, `0x6e23e4`, `0x6e23ef`, `0x6e23f6`, `0x6e275c`, `0x6e2764`, `0x86faa0`, `0x86fab8`, `0x870b64`
Confidence: verified
Proof: one
Test: none

`xor eax,eax; mov al,cl; cmp eax,0x90; ja 0x6e275c`, then `jmp [eax*4+0x6e2764]`. All 145 table
dwords were read: entry r is `0x6e23f6 + 6r`, a `mov eax,imm32; ret` pair, and the immediates fall
strictly with r. The strings tile `0x86faa0` through `0x870b64` with no gap, so the k-th string from the
bottom is the key of reason 0x91 − k. benilla's key table matches all 146.

## Reason text and pet variant

The reason's text is its key's GlobalStrings value. For a spell whose first effect (field 61,
`+0xf4`) is 0x39, or is 0x24 with field 82 (`+0x148`) equal to 5, the reporter first looks up
the key with `_PET` appended (a 64-byte buffer) and keeps that text when it is not empty; else, and
for every other spell, it looks up the key itself. The text goes to a 1024-byte buffer; a key with
no string value gives an empty text. With no record the plain key is used.

Addresses: `0x6e1b1b`, `0x6e1b2f`, `0x6e1b3c`, `0x6e1d00`, `0x6e1d0e`, `0x6e1d22`, `0x6e1d35`, `0x6e1d47`, `0x6e1d54`, `0x6e1d59`, `0x6e1d77`, `0x703bf0`, `0x64a7f0`, `0x64a5a0`
Confidence: verified
Proof: one
Test: none

`0x6e1b1b` compares `[edi+0xf4]` with 0x39 and 0x24, `0x6e1b2f` compares `[edi+0x148]` with 5,
and `0x6e1b3c` sets the flag `[ebp-0x10]`, zeroed at `0x6e1a4c`. `0x6e1d22` formats `"%s_PET"`
with the key into `[ebp-0x88]` (size 0x40), `0x6e1d35` calls `FrameScript_GetText`, `0x6e1d47`
copies into `[ebp-0x488]` (0x400), and `0x6e1d54` keeps it when the first byte is nonzero.
`0x6e1d59` to `0x6e1d77` do the plain key. `0x703bf0` starts its result at an empty string
(`0x703c02`) and replaces it only on a successful lookup. 1.12's GlobalStrings has
`SPELL_FAILED_LEVEL_REQUIREMENT_PET` and `SPELL_FAILED_SPELL_UNAVAILABLE_PET`.

## Record name arms

A second switch fills the text's format from a record named by the arguments; the text is the
format and the result goes to a second 1024-byte buffer. 0x19 to 0x1b: field 2 of the `ItemSubClassMask.dbc`
row equal to (argument 1, argument 2), else field 10 of `0x6e2380`'s `ItemSubClass.dbc` row. 0x31: `0x6e2380(6, 1 << argument 1)`. 0x5d: `AreaTable.dbc` field 11 by argument 1. 0x5e:
`SpellFocusObject.dbc` field 1. 0x8d: `SpellMechanic.dbc` field 1. 0x90: `SkillLine.dbc` field 3,
or `UNKNOWN` with no row, formatted with argument 2 after it. A name is read in the locale's column.
Any other missing row leaves the format unfilled.

Addresses: `0x6e1d8e`, `0x6e1d9a`, `0x6e1da9`, `0x6e1db0`, `0x6e1db7`, `0x6e1de0`, `0x6e1dfe`, `0x6e1e54`, `0x6e1e5c`, `0x6e1e5e`, `0x6e1e63`, `0x6e1e6a`, `0x6e1f10`, `0x6e1f38`, `0x6e1f62`, `0x6e1fad`, `0x6e2190`, `0x6e21ca`, `0x6e21d2`, `0x6e21d8`, `0x6e22b8`, `0x6e22ec`, `0x6e2380`
Confidence: verified
Proof: one
Test: none

`0x6e1d9a` points `edi`, the text, at the first buffer. `0x6e1d8e` subtracts 2, bounds by 0x8e and indexes the byte table `0x6e22ec` (143 bytes) into the
13 entries of `0x6e22b8`; entry 12, `0x6e21d8`, is every unlisted reason. `0x6e1db7` walks the
48-byte records, matching `+0x00` and `+0x04` against the arguments (`0x6e1de0`), name `+0x08`;
`0x6e1dfe` falls back to `0x6e2380`, name `+0x28`. `0x6e1e5c` is `shl edx,cl`, so the shift
counts modulo 32; `0x6e1e5e` loads class 6 and `0x6e1e6a` takes a null row to `0x6e21d8`. The id-indexed arms bound the id from 0 to the store's maximum and take the name
at `+0x2c` (`0x6e1fad`), `+0x04` (`0x6e1f62`, `0x6e2190`) or `+0x0c` (`0x6e1f10`, `UNKNOWN` at
`0x6e1f38`). Each pushes the name, the 0x400 size and both buffers to `SStrPrintf` (`0x6e21ca`),
and `0x6e21d2` makes the second buffer the text; a miss goes to `0x6e21d8`, which keeps the first.
0x90 pushes argument 2, then the name. Fields are offsets over 4 in the in-memory records.

## Item, power and form arms

0x56: the non-empty names (field 2) of the `SpellShapeshiftForm.dbc` rows whose ordinal bit is
set in `Stances` (`+0x2c`), joined by `", "`. 0x5c, 0x78: the `+0x08` name of argument 1's item
cache record; 0x84: the same after argument 2, in the locale's column. 0x02: `MANA`, `RAGE`,
`FOCUS`, `ENERGY` or `HAPPINESS` for the player unit's power byte, 0 to 4. 0x4d: id 0x123 for a
spell power type of −2, else `0x8118dc`[type], with an empty text. 0x2f, 0x30: clear bit 9 of the
player's `+0xd58`. Nothing is shown, logged or fired when 0x56 finds no record or name, the item
cache misses (its callback shows it later), or 0x02 finds another byte or an empty noun.

Addresses: `0x6e1ff8`, `0x6e202d`, `0x6e2058`, `0x6e2071`, `0x6e208b`, `0x6e20c6`, `0x6e1e7f`, `0x6e1ea4`, `0x6e1eab`, `0x6e1ecd`, `0x6e1efc`, `0x6e211f`, `0x6e2131`, `0x6e2176`, `0x6e20eb`, `0x6e20f6`, `0x6e20fd`, `0x6e2109`, `0x6e2113`, `0x6e1e3d`, `0x6e1e45`, `0x8118dc`, `0x55ba30`
Confidence: verified
Proof: one
Test: none

`0x6e1ff8` exits on a null record; the loop tests bit i of `[rec+0x2c]` (`0x6e202d`), takes record i
(stride 0x38) and skips a null or empty name (`0x6e2058`); the first is copied (`0x6e2071`), each
later one appended after `", "` (`0x6e208b`), and `0x6e20c6` exits when none was. `0x6e1ea4` calls
`DBCache::GetRecord` with argument 1, the callback `0x6e29b0` and the reason; a null result exits
at `0x6e1eab` (`0x6e1efc` for 0x84, whose name is `+0x08` + 4·locale). `0x6e211f` reads
`[[esi+0x110]+0x7b]` with no null test of the player in `esi`, exits on 5 or more (`0x6e2131`) or an empty text (`0x6e2176`). `0x6e20eb`
reads `[rec+0x7c]` with no null test and no bound, compares −2 (`0x6e20f6`, 0x123), else
`0x6e2109`; both zero the second buffer (`0x6e20fd`, `0x6e2113`). `0x6e1e3d` skips a null player and
`0x6e1e45` is `and [esi+0xd58],0xfffffdff`. `0x8118dc` holds 0x11f, 0x120, 0x121, 0x122 and 0x168
(`ERR_OUT_OF_MANA` to `ERR_OUT_OF_ENERGY`, `ERR_NOT_HAPPY_ENOUGH`).

## Display, log and events

`CGGameUI::DisplayError` gets the message id and the text, the argument of the id's format. The combat-log
formatter `0x62c360` gets the player, the spell and the text, or, when the text is empty, the text
just displayed. If the spell is the in-flight spell `0xceca88`, `0x6e4940` (`cast`) runs with 1, the
from-server byte's negation and the reason, firing `SPELLCAST_INTERRUPTED` (340) for 0x23 and 0x24
and `SPELLCAST_FAILED` (339) for the rest when its caster is the player, and the reporter returns.
Otherwise, with no spell in flight and a reason other than 0x61, it fires the same choice itself; no
event carries arguments.

Addresses: `0x6e21d8`, `0x6e21dd`, `0x6e21e2`, `0x6e21ee`, `0x6e21fc`, `0x6e2201`, `0x6e2218`, `0x6e2228`, `0x6e2234`, `0x6e2238`, `0x6e224a`, `0x6e4940`, `0x6e4a24`, `0x6e4a50`, `0x6e3d30`, `0x496720`, `0x4968a0`, `0x62c360`, `0x703e50`, `0xceca88`, `0xb4da40`
Confidence: verified
Proof: one
Test: none

`0x6e21dd` calls `0x496720(id, text)`, which formats the id's string with the text and copies the
result to `0xb4da40`. `0x6e21e2` pushes the text unless its first byte is 0, else `0x4968a0`'s
`0xb4da40`; `0x6e21fc` calls `0x62c360` with `ecx` the player, `edx` the spell. `0x6e2201` compares
the spell with `0xceca88`; `0x6e2218` passes `cl` 1, `dl` `sete` of the byte, and the reason, then
returns. In `0x6e4940` with `cl` 1, `0x6e4a24` picks 0x154 for 0x23 or 0x24 and 0x153 otherwise,
after its caster GUID matched the active player's (`0x6e4a11`). `0x6e2228` returns on `0x6e3d30`
nonzero (`0xceca88` set), `0x6e2234` on 0x61; `0x6e2238` chooses 0x154 for 0x23 and 0x24, and
`0x6e224a` calls `0x703e50` with the id alone.

## Item cache redisplay

`0x6e29b0` is the item-cache callback of reasons 0x5c and 0x78. It looks the item record up again
without a callback, formats the reason's key text with its `+0x08` name, or `UNKNOWN`, and shows
it under 0xd2 for 0x5c, 0xd1 for 0x78 and 0x2c otherwise. `0x6e2a90` does the same for 0x84 with
the count it was given, the count first, under 0x2c, and 0 with `UNKNOWN` on a miss. Neither
tries the `_PET` key, logs, fires an event or touches the cast.

Addresses: `0x6e29b0`, `0x6e29da`, `0x6e29fe`, `0x6e2a27`, `0x6e2a49`, `0x6e2a71`, `0x6e2a90`, `0x6e2af2`, `0x6e2b18`, `0x6e2b43`, `0x55ba30`
Confidence: verified
Proof: one
Test: none

Both are `fastcall(item, guid)` with the parameter and a found flag on the stack, `ret 8`: the item
cache's callback calls load the id into `ecx` and the request's GUID address into `edx`, and push
the flag, then the parameter. `0x6e29da` takes the key of the byte
parameter, `0x6e29fe` picks the id, `0x6e2a27` is `GetRecord(item, &zero, 0, 0, 0)`, `0x6e2a49`
pushes `UNKNOWN`, `0x6e2a71` is `DisplayError`. `0x6e2a90` fixes reason 0x84 (`mov cl,0x84`),
pushes the name then the parameter (`0x6e2b01`), or `UNKNOWN` then 0 (`0x6e2b18`), and shows 0x2c
(`0x6e2b43`).

## Subclass by mask

`0x6e2380(class, mask)` returns the first `ItemSubClass.dbc` row, in row order, whose class field
equals `class` and whose subclass field's bit is set in `mask`, the bit taken modulo 32; else null.

Addresses: `0x6e2380`, `0x6e2389`, `0x6e23b2`, `0x6e23be`, `0x6e23c3`, `0x6e23cf`
Confidence: verified
Proof: one
Test: none

`0x6e2389` loads the row count and walks 0x74-byte records from the store's base, `cmp [eax],ecx`
at `0x6e23b2`, `shl edx,cl` with `cl` the row's `+0x04` (`0x6e23be`) and `test ecx,edx`
(`0x6e23c3`); `0x6e23cf` returns 0. The only callers are `0x6e1a00` (`0x6e1dfe`, `0x6e1e63`) and
`0x6e8eb0` (`0x6e90cb`).

## Spell failure packets

`SMSG_SPELL_FAILURE` (0x133) carries a `u64` caster GUID, a `u32` spell id and a `u8` reason. For
a known unit, `0x6e8d80` calls `0x60d040(spell, 1, 1)` and `0x614150(spell, 0)`, passes the spell and
the unit to `0x458a50`, calls `0x6ea080` (`cast`) when the unit is the active player and the
reason is 0x2f, 0x30 or 0x31, then `0x60fc50` and, when `0x611590(1)` holds, `0x60fb60(0, 1)`. It
shows no message and fires `STOP_AUTOREPEAT_SPELL` on the `0x6ea080` leg, and other events and sends through its callees (`What the failure handler signals`). `SMSG_SPELL_FAILED_OTHER` (0x2a6), a `u64` GUID and a `u32`
spell id, makes `0x6e8e40` call only the first two. Both return 1. The names are the server's.

Addresses: `0x6e8d80`, `0x6e8d90`, `0x6e8d9b`, `0x6e8da6`, `0x6e8dc2`, `0x6e8dd7`, `0x6e8de4`, `0x6e8dee`, `0x6e8df5`, `0x6e8e01`, `0x6e8e0d`, `0x6e8e14`, `0x6e8e1d`, `0x6e8e2c`, `0xceac30`, `0x6e8e40`, `0x6e8e50`, `0x6e8e5b`, `0x6e8e8c`, `0x6e8e99`, `0x6e718f`, `0x6e727f`, `0x4190b0`, `0x418e30`, `0x418cb0`, `0x5fa6d0`, `0x60fc50`, `0x611590`, `0x60fb60`
Confidence: verified
Proof: one
Test: none

`Spell_C::SystemInitialize` registers them at `0x6e718f` and `0x6e727f`. The reads are
`0x4190b0` (8 bytes), `0x418e30` (4) and `0x418cb0` (1), in that order (`0x6e8d90` to `0x6e8da6`;
`0x6e8e50`, `0x6e8e5b`). `0x6e8dc2` looks the GUID up as a type 8 object. `0x6e8df5` calls
`0x5fa6d0`, which compares the unit's GUID with the active player's; `0x6e8e01` to `0x6e8e0b` test
the reason. Each ends `mov eax,1; ret 8`. A short packet leaves the read's local unwritten.

## What the failure handler signals

The ammo leg of `SMSG_SPELL_FAILURE` sends CMSG_CANCEL_AUTO_REPEAT_SPELL and fires `CURRENT_SPELL_CAST_CHANGED` then `STOP_AUTOREPEAT_SPELL`. For the active player the precast clear can also close a loot window: it fires `LOOT_CLOSED`, sends CMSG_LOOT_RELEASE and, by what is open, `ITEM_LOCK_CHANGED`, `PLAYER_TARGET_CHANGED`, a chat line or a loot-open event.

Addresses: `0x6ea080`, `0x6ea086`, `0x6ea0c6`, `0x6ea140`, `0x6ea160`, `0x6ea170`, `0x6e4b4e`, `0x4b3250`, `0x4b3255`, `0x60d040`, `0x60f530`, `0x60f53d`, `0x60fb60`, `0x60fb76`, `0x60fc72`, `0x6e8e1d`, `0x60d083`, `0x60d0c3`, `0x860adc`, `0x6204aa`, `0xceac30`
Confidence: verified
Proof: one
Test: none

With an auto-repeat spell set (`0xceac30`, `0x6ea086`), `0x6ea080` sends CMSG_CANCEL_AUTO_REPEAT_SPELL (`0x6ea0c6`, opcode 0x26d). When that spell is also the current cast (`0x6ea154`), `0x6ea160` calls `0x6e4ad0(0, 0)`, which restores the previous current spell and jumps to `0x4b3250` (`0x6e4b4e`): `ACTIONBAR_UPDATE_STATE`, then `CURRENT_SPELL_CAST_CHANGED` (`0x4b3255`). `STOP_AUTOREPEAT_SPELL` (event 436, `0x6ea170`) follows.

`0x60fb60` fires `AUTOFOLLOW_END` only in mode 3 (`0x60fb76`). Its calls here run in mode 0 or 1: `0x6e8e1d`, `0x60d083`, `0x60d0c3`, the mask 1 at `0x860adc` that `0x60fc50` tests, and `0x60f530`, which calls `0x60f8f0` at `0x60f53d` for mode 0 (reached from `0x60fc72` and `0x6ea140`). The effect stop `0x614150` runs `0x6203e0`, which can refresh the unit's animation through `0x5fd9e0` (`0x6204aa`); that refresh's callees are not read for signals.

## A failure closes the loot window

For the active player, the precast clear of `SMSG_SPELL_FAILURE` closes an open loot window: it fires `LOOT_CLOSED` and sends CMSG_LOOT_RELEASE, or while moving does nothing for a corpse's loot or an item.

Addresses: `0x60d040`, `0x60d049`, `0x60d0fd`, `0x60d10b`, `0x48f200`, `0x48f24a`, `0x48f25a`, `0x48f264`, `0x48f2b5`, `0x48f2bd`, `0x48f311`, `0x48f33d`, `0x48f369`, `0x4c1d36`, `0x4c1d9b`, `0x4c1fa0`, `0x4c2170`, `0x4c21d4`, `0x4c2240`
Confidence: verified
Proof: one
Test: none

`0x60d040` reaches `0x48f200` for the active player when the spell is the unit's precast spell (`0x60d049`, `0x60d10b`), with the moving flag `[unit+0x118]+0x40 & 0x200f` (`0x60d0fd`) in `dl` and 0 pushed. With no loot source held (`0xb71b48`) it returns. Otherwise, in order:
- A type 4 loot window with contents (`0x48f24a`, `0x4c2a70`): while moving nothing happens (`0x48f25a`); else it auto-loots through `0x4c1fa0` (`0x48f264`) and returns. That can show a chat line (`0x4c2170`), fire `LOOT_BIND_CONFIRM` (`0x4c21d4`) and, with rows left, `LOOT_OPENED` (`0x4c2240`).
- Any other window: an item source while moving returns (`0x48f2b5`); an item source otherwise is unlocked, `ITEM_LOCK_CHANGED` (`0x48f2bd`). CMSG_LOOT_RELEASE (0x15f) is sent (`0x48f311`). `0x4c1cb0` (`0x48f33d`) unlocks an item source again (`0x4c1d36`), clears the guid and fires `LOOT_CLOSED` (`0x4c1d9b` loads 0x10d, the call is `0x4c1dac`). A dead unit source then goes through `0x493910` (`0x48f369`): `PLAYER_TARGET_CHANGED` and CMSG_SET_SELECTION when it is the target.

## Pet cast failure packet

`SMSG_PET_CAST_FAILED` (0x138) carries a `u32` spell id, a `u8` that is never read, and a `u8`
reason, then, by reason, two `u32` for 0x19 to 0x1b (item class, subclass mask) or one `u32` id for
0x5d, 0x5e and 0x8d. `0x6e8eb0` needs the spell's record, else shows nothing, and uses the reason
byte whatever the second byte holds. It shows one message through `CGGameUI::DisplayError`: no
combat-log line, no event, no sound, no `_PET` key, and no change to the cast. Its argument
words start at 0 and stay 0 when the packet is short.

Addresses: `0x6e8eb0`, `0x6e8ec5`, `0x6e8ed0`, `0x6e8edb`, `0x6e8f04`, `0x6e8f11`, `0x6e8f1c`, `0x6e8f23`, `0x6e9053`, `0x6e9056`, `0x6e9064`, `0x6e9161`, `0x6e9164`, `0x6e91b9`, `0x6e92e2`, `0x6e719f`
Confidence: verified
Proof: one
Test: none

Registered at `0x6e719f`. Reads: `0x418e30` into `[ebp-0x10]` (`0x6e8ec5`), `0x418cb0` into
`[ebp-0x9]` (`0x6e8ed0`), which nothing else touches, and `0x418cb0` into `[ebp+0xf]` (`0x6e8edb`).
`0x6e8f04` exits on a null record. The arms store 0 before each extra read: `0x6e9053`, then
`0x6e9056` and `0x6e9064` for class and mask; `0x6e9161`/`0x6e9164` (0x5e), `0x6e91b9` (0x5d),
`0x6e92e2` (0x8d). Every exit is `mov eax,1; ret 8`; the only calls out are the reads, `0x6e23e0`,
`0x703bf0`, `SStrCopy`, `SStrPrintf`, `0x6e2380` and `0x496720`.

## Pet failure messages

Reasons above 0x8d and those the table leaves show their key text under 0x2c. Fixed ids: 0x00 →
0x14c, 0x13 → 0x150, 0x32 → 0x151, 0x33 → 0x14e, 0x59 → 0x14d, 0x5f → 0x14b, 0x65 → 0x14f (the
`ERR_PET_SPELL_*` rows 331 to 337). 0x3c: 0x31 or 0x30 by `Attributes` bit 4. 0x4d: 0x123 for power
type −2, else `0x8118f0`[type]. 0x56: the first non-empty form name under 0xd6. 0x19 to 0x1b, 0x5d,
0x5e, 0x8d: the name from the words, as the player's arms find it, under 0x2c. A missing row or
name shows nothing.

Addresses: `0x6e8f2a`, `0x6e8f4f`, `0x6e8f6a`, `0x6e8f8d`, `0x6e8fab`, `0x6e8fc6`, `0x6e8fe1`, `0x6e8ffc`, `0x6e9017`, `0x6e9032`, `0x6e904d`, `0x6e90d2`, `0x6e911a`, `0x6e915b`, `0x6e91b0`, `0x6e921a`, `0x6e9282`, `0x6e928c`, `0x6e92be`, `0x6e92d9`, `0x6e936a`, `0x6e9385`, `0x6e9394`, `0x6e93d0`, `0x8118f0`
Confidence: verified
Proof: one
Test: none

`0x6e8f1c` reads the byte table `0x6e93d0` (142 bytes) and jumps through the 15 entries of
`0x6e9394`; entry 14, `0x6e936a`, is the default. Each fixed arm pushes only the id. `0x6e8f2a`
computes `((attr & 0x10) | 0x300) >> 4`; `0x6e8f8d` reads `0x8118f0`, whose five words equal
`0x8118dc`'s. `0x6e904d` matches `ItemSubClassMask.dbc` (`0x6e911a`) or calls `0x6e2380`, and
`0x6e90d2` exits on null. `0x6e9282` leaves the form loop at the first name, `0x6e928c` returns with
none, `0x6e92be` shows 0xd6. The id arms jump to `0x6e9385`, the plain return, on a bad id or a null
row. `0x6e936a` shows `GetText(key)` under 0x2c.

## Pet tame failure

`SMSG_PET_TAME_FAILURE` (0x173) is one `u8` reason. `0x6e97e0` reads it and calls `0x6e6a20`, which
shows `ERR_TAME_FAILED` (0xee) with the GlobalStrings text of a `PETTAME_` key: 1
`INVALIDCREATURE`, 2 `TOOMANY`, 3 `CREATUREALREADYOWNED`, 4 `NOTTAMEABLE`, 5
`ANOTHERSUMMONACTIVE`, 6 `UNITSCANTTAME`, 7 `NOPETAVAILABLE`, 8 `INTERNALERROR`, 9
`TOOHIGHLEVEL`, 10 `DEAD`, 11 `NOTDEAD`, any other byte `UNKNOWNERROR`. The client's own check
calls `0x6e6a20` with 2 and then reports reason 0x17 to `0x6e1a00`.

Addresses: `0x6e97e0`, `0x6e97ea`, `0x6e97f2`, `0x6e6a20`, `0x6e6a2d`, `0x6e6a33`, `0x6e6a87`, `0x6e6a91`, `0x6e6ab4`, `0x6e6ac0`, `0x6e71ff`
Confidence: verified
Proof: one
Test: none

Registered at `0x6e71ff`; `0x6e97ea` reads one byte and `0x6e97f2` passes it in `cl`, then `mov
eax,1; ret 8`. `0x6e6a2d` decrements, bounds by 0xa and jumps through the 11 entries of `0x6e6ac0`,
each loading one key; `0x6e6a87` is the default. `0x6e6a91` calls `0x703bf0`, and `0x6e6ab4` calls
`0x496720(0xee, text)`. The census of `0x6e6a20` has two calls: `0x6e97f2` and the unit code's tame
check, which passes `cl` 2 and then calls `0x6e1a00` with reason 0x17.
