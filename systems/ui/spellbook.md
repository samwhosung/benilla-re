# ui: the spell book

The spell book as the interface sees it: the lists of spell ids by slot, the tabs, what a learned spell
does to them, the line printed when a spell is learned, and the Lua functions that read them.

The code is `SpellBookFrame.cpp`, `0x4b24f0` to `0x4b4b70`, with the 19 Lua functions of the table at
0x846648 that `0x4b4b40` registers. The player's spells arrive through the learn and replace
messages (`objectlayer/playercombat`), which call into the book. The pet and form functions of the
same table are in `ui/petbar`.

Roots: `0x4b4b40`, `0x4b25b0`, `0x4b2c50`, `0x4b2f50`, `0x4b2fd0`, `0x4b31b0`, `0x4b31c0`, `0x4b3250`, `0x4b3cb0`, `0x4b3ce0`, `0x4b3e50`, `0x4b3fe0`, `0x4b40a0`, `0x4b4180`, `0x4b4240`, `0x4b42a0`, `0x4b42f0`, `0x4b4370`, `0x4b43e0`, `0x4b43f0`, `0x4b4410`, `0x4b44e0`

## Spell book lists

The player's book is the list `0xb700f0` of spell ids by slot, 1024 slots, its count at `0xb7116c`. The
pet's is `0xb6f098`, count `0xb71174`. Tabs are a list of 8-byte nodes `{tab key, spell count}`
(`0xb71160`, count `0xb71170`); the key is a SkillLine id and 0 is the General tab. `0xb710fc` is a
bitset of every known spell id. A Lua book function takes a 1-based slot and a book type; the
type selects the pet list only when it equals `pet` (without case), and any other value, `spell`
included, selects the player's.

Addresses: `0xb700f0`, `0xb6f098`, `0xb7116c`, `0xb71174`, `0xb71160`, `0xb71170`, `0xb710fc`, `0x4b3ec0`, `0x4b3ef8`, `0x4b3efd`, `0x4b3f0c`, `0x4b3f27`, `0x4b3f35`, `0x64a4c0`
Confidence: verified
Proof: one
Test: `ui::glue_4b0_4b3ec0_index_ftol`

`0x4b3ec0(L, &index, &book)` fails (returns 0) unless argument 1 is a number or numeric string
(`0x6f34d0`) and argument 2 is a string or number (`0x6f3510`); a missing book type fails. The index is
`trunc(arg1 - 1.0)`: the `fsub` of 1.0 (`0x4b3efd`) comes before the truncating `0x40a2b0`, so 0.5 gives
index 0. It must be 0 to 1023 (`0x4b3f0a`, `0x4b3f0c`). `0x4b3f27` compares argument 2 with `pet` using
`0x64a4c0` (without case) and `0x4b3f35` stores book 1. Each caller raises `Invalid spell slot in
<name>` on a 0 return.

## GetSpellName

`GetSpellName` (`0x4b3fe0`) returns two values for a slot whose spell has a record: the name and the
rank of the active locale, both strings, and the rank is the empty string for a spell without one,
never nil. Both are pointers into the Spell DBC string block, which the loader fills with the block
base plus the stored offset without a null test, so an offset of 0 gives the block's leading NUL. For a
slot whose spell id is 0, negative, past the table or without a record it returns nil, nil.

Addresses: `0x4b3fe0`, `0x4b4015`, `0x4b402b`, `0x4b4034`, `0x4b4054`, `0x4b405a`, `0x4b406d`, `0x4b4086`, `0x6f3890`, `0x583750`, `0x584a0e`, `0x584a1c`, `0x54ffd0`
Confidence: verified
Proof: one
Test: none

`0x4b4015` to `0x4b4034` read the list by book type; `0x4b405a` loads `Name[locale]` at `+0x1e0` and
`0x4b406d` the rank at `+0x204` from the locale index at 0xc0e080; both go through `0x6f3890`, which
pushes nil only for a null pointer. `0x4b4086` is the nil, nil exit. `SpellRec::Read` (`0x583750`) adds
the string block base to every string column unconditionally (`0x584a0e`, `0x584a1c` for the rank); its
base is the `edi` of `0x54ffd0` (`0x550116`, records plus count times 0x2b4). The bad-argument path
raises and does not return.

## Book icons

`GetSpellTexture` (`0x4b3e50`) returns the SpellIcon path of the slot's spell through `0x4b3f50`, always
from `SpellIconID` (`+0x1d4`) and never from the active icon. A spell whose first effect is 78 or that
repeats with the ranged slot returns the attack or ranged weapon icon instead (`ui/actionbar#attack-and-ranged-icons`).
A slot of no spell, or an icon id outside the SpellIcon table, gives nil.

Addresses: `0x4b3e50`, `0x4b3f50`, `0x4b3f5d`, `0x4b3f61`, `0x4b3f6a`, `0x4b3f8a`, `0x4b3f99`, `0x4b3fae`, `0x4b3fcd`, `0x4e6870`, `0x4e6990`
Confidence: verified
Proof: one
Test: none

`0x4b3f50(index, book)`: `0x4b3f50 cmp ecx,0x400`, the list read, the id and record bounds, `0x4b3f8a cmp
[esi+0xf4],0x4e` and the tail jump to `0x4e6870`, the ranged test at `0x4b3f99` and `0x4b3f9f` and the call
`0x4e6990`, then `0x4b3fae` loads `+0x1d4` and `0x4b3fcd` the path at `+4` of the SpellIcon record.

## Adding a spell

`0x4b25b0(id, live, old)` adds a learned spell to the book. It rejects an id outside the Spell table and
sets the id's bit in the known set. It records a few spells by their first effect (effects 83, 84, 95
and 116 in four globals, 39 into a table by misc value, 33 into a list) and updates three more lists
(at 0xb71110, 0xb7114c and 0xb71104). Past that, a spell with `Attributes & 0x80` goes no further. A spell with
`Attributes & 0x20` (a trade skill spell) prints its learn line when `live` is set and goes no
further. Any other prints its line when `live` is set, and enters the book unless its `CastUI`
field (`+0xc`) is above 0.

Addresses: `0x4b25b0`, `0x4b25c6`, `0x4b25d2`, `0x4b25e5`, `0x4b2603`, `0x4b2605`, `0x4b2641`, `0x4b2665`, `0x4b2909`, `0x4b2911`, `0x4b2917`, `0x4b291e`, `0x4b29bf`, `0x4b29c4`, `0xb710fc`, `0xb71130`, `0xb700e4`, `0xb700e8`, `0xb700ac`, `0xb700b0`
Confidence: verified
Proof: one
Test: none

The three bounds tests at `0x4b25c6`, `0x4b25d2`, `0x4b25e5` leave through `0x4b2ba6`. `0x4b2603` sets the
bit in `0xb710fc`. `0x4b2605` and the three that follow compare `[rec+0xf4]` with 0x53, 0x54, 0x5f and 0x74
and store the id in four globals, among them `0xb71130`, `0xb700e4` and `0xb700e8`; `0x4b2641` (0x27) stores it at
`[0xb700ac] + misc*4` and calls `0x49b970`; `0x4b2665` (0x21) appends it to the list `0xb700b0`. All
five paths meet at `0x4b2909`, from which only the three gates remain: `0x4b2911 js` (bit 0x80),
`0x4b2917 test al,0x20` (the trade skill arm, which leaves at `0x4b2920` unless `live`), and
`0x4b29bf` to `0x4b29c4` (`CastUI`). The learn line is printed at `0x4b2956` to `0x4b29b7`, before
the `CastUI` test, so such a spell is announced but not booked.

## The learn line

With `live` set, a spell that is not a trade skill prints through `DisplayError` (`0x496720`) the message
55 `ERR_LEARN_SPELL_S` when `Attributes & 0x10` is clear and 56 `ERR_LEARN_ABILITY_S` when it is set, with
the spell's name and, when the rank string is not empty, `"%s (%s)"` of name and rank. A trade skill
spell prints message 57 `ERR_LEARN_RECIPE_S` with the name only. The three are catalogue rows of kind 0, so
the line is a chat line of the system type, not an error frame. The learn message passes `live` 1; the
login spell list, the replacement of a rank (`ReplaceSpellInBook` passes 1) and a spell that arrives
before the player object pass differently (`objectlayer/playercombat#spell-book-messages`).

Addresses: `0x4b2926`, `0x4b2934`, `0x4b2936`, `0x4b2952`, `0x4b2963`, `0x4b2971`, `0x4b2982`, `0x4b29b3`, `0x4b29b7`, `0x496720`, `0x4b2f50`, `0x5e9c20`, `0x5e9c62`
Confidence: verified
Proof: one
Test: none

`0x4b294a and eax,0x10` keeps the ability bit; `0x4b2952` tests `live`; `0x4b2963 cmp byte [eax],0`
tests the rank string and `0x4b2971 push 0x8468b0` formats `"%s (%s)"` into a 0x200-byte buffer, else
`0x64a5a0` copies the name. `0x4b29b3 add eax,0x37` makes the message 55 or 56 and `0x4b29b7` shows it.
The recipe arm pushes 0x39 at `0x4b2934`. `0x4b2f50` calls `0x4b2c50(old, 1)` and then `0x4b25b0(new, 1, old)`.

## Entering the book

A spell that passes the gates takes its tab: `0x4b29ca` to `0x4b2a21` compute the key, the loop at
`0x4b2a40` finds the tab with that key and increments its count, and a new tab is allocated as a node
`{key, 1}` and appended. The id is appended to the book at `0x4b2b3f`. With `live` set the lists are
re-sorted (`0x4b2fd0`), and `LEARNED_SPELL_IN_TAB` (510) is signalled with the 1-based index of the
spell's tab, and flag 0x27 is set through `0x4b5390`.

Addresses: `0x4b29ca`, `0x4b2a40`, `0x4b2a4e`, `0x4b2b12`, `0x4b2b2a`, `0x4b2b3f`, `0x4b2b5a`, `0x4b2b92`, `0x4b2b9c`, `0x4b2fd0`, `0x4b5390`, `0x703f50`, `0x6462e0`
Confidence: verified
Proof: one
Test: none

`0x4b2a40` to `0x4b2a4c` search the tab list for the key; a hit increments `[node+4]` (`0x4b2a5a`), a miss
falls to the allocation at `0x4b2af8` (8 bytes, source line 0x15b) and `0x4b2b2a` stores count 1.
`0x4b2b3f` is the append to `0xb700f0` and `0x4b2b47` stores the new count. `0x4b2b5a call 0x4b2fd0`
with `ecx` 1 and `edx` 0, `0x4b2b92` is `SignalEvent2(0x1fe, "%d", tab + 1)`, and `0x4b2b9c` calls
`0x4b5390` with 0x27.

## Tabs

A spell's tab key is the `+4` of the player's `SkillRaceClassInfo` row (`0x6ddf90`) for the skill line
of the spell's ability (`0x60c7c0`); it is 0 when any step fails or the row's byte at `+0x10` is
negative. `0x4b24f0(id)` returns the position of that key in the tab list, or the tab count when absent.
`0x4b2580(tab)` counts the book spells of the tabs before it. `GetNumSpellTabs` (`0x4b3cb0`) returns the
tab count.

Addresses: `0x4b24f0`, `0x4b2580`, `0x4b3cb0`, `0x60c7c0`, `0x6ddf90`, `0x4b2524`, `0x4b2536`, `0x4b2544`, `0x4b2563`
Confidence: verified
Proof: one
Test: none

`0x4b24f0` fetches the active player (`0x468460`), calls `0x60c7c0` (`0x4b251b`) and `0x6ddf90` with the
race and class bytes at `[unit+0x110]+0x78` and `+0x79` (`0x4b2536`); `0x4b2542 test cl,cl; js` takes a
negative `+0x10` to key 0 (`0x4b2549`); `0x4b2563` compares each node's key and returns the index, or the
count (`0x4b256c`). `0x4b2580` sums `[node+4]` from `tab - 1` down to 0 and returns 0 for tab 0 or a tab at
or past the count.

## Tab info

`GetSpellTabInfo` (`0x4b3ce0`) raises unless its argument is a number and returns name, icon path, offset
and spell count. An index below 1, past the count or with a null node gives nil, nil, 0, 0. The General
tab gives the global string `GENERAL` and `Interface\Icons\Ability_Kick`; another tab its SkillLine
name and SpellIcon path, nil where a record is missing.

Addresses: `0x4b3ce0`, `0x4b3d0e`, `0x4b3d1a`, `0x4b3d26`, `0x4b3d29`, `0x4b3d3c`, `0x4b3d47`, `0x4b3d96`, `0x4b3dbd`, `0x4b3dc4`, `0x4b3dcd`, `0x4b3e12`, `0x4b3e37`, `0x703bf0`
Confidence: verified
Proof: one
Test: none

`0x4b3d1a` truncates the argument and `0x4b3d26 dec ebx` decrements it afterwards; the unsigned compare
at `0x4b3d29` rejects 0, negatives and indices past the count, so 0.5 truncates to 0 and is rejected.
`0x4b3d47 jne` splits General from a keyed tab; the General name is `0x703bf0("GENERAL", -1, 0)`
(`0x4b3d52`), the icon the literal at `0x4b3d60`. A keyed tab reads `SkillLine[key]` (`0x4b3d78`), its
name at `+0xc` of the locale (`0x4b3d8b`) and its icon id at `+0x54` (`0x4b3d96`); an id outside the table
gives two nils (`0x4b3dbd`), a bad icon one (`0x4b3dc4`). Offset and count come from `0x4b2580`
(`0x4b3dcd`) and `[node+4]`, each pushed as a number. The failing exit `0x4b3e12` pushes nil, nil, 0, 0
and returns 4 (`0x4b3e37`).

## Book order

`0x4b2fd0(sortBook, sortPet)` sorts the tab list with `0x4b3040` and the book with `0x4b30c0` when
`sortBook` is nonzero, sorts the pet book with `0x4b30c0` when `sortPet` is nonzero, and always signals
`SPELLS_CHANGED` (260). Tabs: General (key 0) first, then by the localized SkillLine name with
`SStrCmp`. Spells: by their tab's position in the tab list, then by name (`+0x1e0`), then by rank, then
by the rank string (`+0x204`). The rank is the first run of digits in the rank string, or the spell level
(`+0x74`) when it has none; two nonzero ranks compare numerically and end the comparison, and the rank
strings are compared only when either rank is 0. `UpdateSpells` (`0x4b43e0`) calls `0x4b2fd0(0, 0)`, so it
signals only.

Addresses: `0x4b2fd0`, `0x4b3040`, `0x4b30c0`, `0x4b3c30`, `0x4b43e0`, `0x73f727`, `0x64a480`, `0x4b3051`, `0x4b305e`, `0x4b310c`, `0x4b3143`, `0x4b3164`, `0x4b3179`
Confidence: verified
Proof: one
Test: `ui::sortcmp_4b30c0`

`0x4b2fe2` and `0x4b2ff6` push the comparators for `qsort` (`0x73f727`) of 4-byte elements; the pet sort is
at `0x4b3015`; `0x4b302a` loads 0x104 and tail-jumps to the signal. `0x4b3053` returns -1 for a first
key of 0 and `0x4b305e` +1 for a second key of 0. `0x4b310c` and `0x4b3115` are the two tab positions,
`0x4b3143` the name compare, `0x4b3164 cmp eax,ebx` the numeric ranks (`0x4b3166 jbe`, `0x4b3171 sbb`),
`0x4b3179` the rank strings. `0x4b3c30` loops over the rank string with the digit test `0x6c9c30`
(`n = 10n + digit`) and falls back to `[rec+0x74]` at `0x4b3c94`.

Bit-exact coverage: `ui::sortcmp_4b30c0` covers `0x4b30c0` to `0x4b30e2`, `0x4b30e6` to `0x4b30f6`, `0x4b30fa` to `0x4b31a3` of `0x4b30c0`.
`UpdateSpells` is two `xor`s, the call to `0x4b2fd0` (`0x4b43e4`), `xor eax,eax` and `ret` at `0x4b43eb`: it reads no argument, returns no
value and does nothing else; its end, `0x4b43ec`, is the first of four padding bytes before `0x4b43f0`.

## Who signals SPELLS_CHANGED

`0x4b2fd0` is the only function that signals `SPELLS_CHANGED` (260), and seven direct calls are all that reach it. Each passes the book flag and the pet flag. A learned spell (`0x4b2b5a`) passes 1, 0 and a removed one (`0x4b2f3f`) 1, 0, the removal only when the id was in the book. `UpdateSpells` (`0x4b43e4`) passes 0, 0 and `SMSG_PET_SPELLS` (`0x4bdadb`) 0, 1.
The active player's arrival (`0x5deac2`) passes 1, 0 after it replays the pending initial spells. Two calls pass 0, 0 for the active player: a changed main hand or ranged visible item (`0x5deeed`) and a level change (`0x5edb7b`).

Addresses: `0x4b2fd0`, `0x4b302f`, `0x4b2b5a`, `0x4b2f2f`, `0x4b2f38`, `0x4b2f3f`, `0x4b43e4`, `0x4bdadb`, `0x5dea50`, `0x5deac2`, `0x5dee30`, `0x5dee91`, `0x5deeb1`, `0x5deeed`, `0x5edb60`, `0x5edb6f`, `0x5edb7b`, `0x60cdd0`, `0x60ce4b`
Confidence: verified
Proof: one
Test: none

No jump table, vtable or data word holds the address of `0x4b2fd0`, so the seven are the whole list. `0x4b2c50` leaves its search index at 0x400 when the id is not in the book, and `0x4b2f2f` compares it with 0x400 and skips the call. `0x5dee30` takes the slot as `(field offset - 0x118) / 48`; slots 15 and 17 reach `0x5deeed` (17 after `UNIT_RANGED_ATTACK_POWER`) and slot 16 does not.
Both active-player paths compare the object's GUID with `0x468550` first (`0x5dee91`, `0x5edb6f`); `0x60cdd0` reaches `0x5edb60` only for an object whose type mask has bit 0x10 (`0x60ce4b`). A call that moves entries sorts first, so the signal never precedes the order; the three 0, 0 calls leave the order alone.

## Checked spells

`IsCurrentCast` (`0x4b4370`) pushes 1 or nil from `0x4b3600(slot, book)`, a smaller test than the action
bar's. A slot of no spell is false. A spell whose first effect is 47 is true when its misc value
(`+0x1a8`), if nonzero, equals the open craft profession, else when the open trade-skill line equals the
line of its skill ability (taken from the open recipe record if there is one). Any spell with a
shapeshift aura effect (aura 36) is true when that effect's misc value equals the player's form byte.
There are no casting, targeting, item or attack arms.

Addresses: `0x4b4370`, `0x4b3600`, `0x4b3664`, `0x4b3677`, `0x4b367c`, `0x4b3698`, `0x4b36a8`, `0x4b36c3`, `0x4b36d8`, `0x4b43ab`, `0x60c7c0`, `0xbdcfb8`, `0xbde040`
Confidence: verified
Proof: one
Test: none

`0x4b3600` is called only from `0x4b43ab`. It reads the list and record as above, fetches the active
player once, and tests `[rec+0xf4]` against 0x2f at `0x4b3664`. `0x4b3677 call 0x60c7c0` and the compare
with `[result+4]`; `0x4b3698` compares `0xbdcfb8`. The loop at `0x4b36b0` scans the three aura
names at `+0x16c` for 0x24; `0x4b36d8` reads the form byte at `[player+0x110]+0x212`.

## Book spell functions

`GetSpellCooldown` (`0x4b40a0`) returns start and duration times 0.001 and the enable flag, from
`0x6e2ea0` for the slot's spell. `GetSpellAutocast` (`0x4b4180`) returns two values, nil unless the slot
is a pet-book spell, then bits 31 and 30 of its pet spell flags (`0x4bd160`) as 1 or nil.
`ToggleSpellAutocast` (`0x4b4240`) toggles a pet-book spell through `0x4bccb0`. `IsSpellPassive`
(`0x4b44e0`) pushes 1 for `Attributes & 0x40`, else nil. `PlayerHasSpells` (`0x4b43f0`) always pushes 1.
`HasPetSpells` (`0x4b4410`) pushes nil, nil for an empty pet book, else the pet spell count and the
pet token of the player's class record (`+0x10`), or `PET` without a player.

Addresses: `0x4b40a0`, `0x4b4180`, `0x4b4240`, `0x4b44e0`, `0x4b43f0`, `0x4b4410`, `0x4b4490`, `0x4b4291`, `0x4bccb0`, `0x4bd160`, `0x6e2ea0`
Confidence: verified
Proof: one
Test: `ui::luabind_4b40a0_get_spell_cooldown`

`0x4b4125` and `0x4b4144` multiply by 0.001; the third value is pushed unscaled (`0x4b4166`). `0x4b41da` calls
`0x4bd160` for a pet-book spell and `0x4b41e7`, `0x4b420c` shift bits 31 and 30. `0x4b4291` calls
`0x4bccb0` for a pet-book spell with a nonzero id. `0x4b4553 test [rec+0x18],0x40`. `0x4b4416` reads
the pet count; a class id past the class table makes `HasPetSpells` read address 0x10 (`0x4b4490`).

Bit-exact coverage: `ui::luabind_4b40a0_get_spell_cooldown` covers `0x4b411d` to `0x4b412e`, `0x4b413c` to `0x4b414d` of `0x4b40a0`.

## Picking up a book spell

`PickupSpell` (`0x4b42a0`) runs `0x4b3260(slot, book)`. It clears the cursor first. If the cursor held the
same spell it plays `igSpellBookSpellIconDrop`; if it held another spell, or the slot holds no spell,
it stops there. Otherwise the slot's spell goes onto the cursor, as a spell (`0x494d20`) or, for the
pet book, as a pet action (`0x494e20` of the pet spell record from `0x4bd160`), with the sound
`igSpellBookSpellIconPickup`.

Addresses: `0x4b42a0`, `0x4b3260`, `0x4b3285`, `0x4b3293`, `0x4b3298`, `0x4b32aa`, `0x4b32b6`, `0x4b32bd`, `0x4b32ca`, `0x4b32e5`, `0x4b32f5`, `0x494d20`, `0x494e20`, `0x494e10`, `0x495190`, `0x4bd160`
Confidence: verified
Proof: one
Test: none

`0x4b3285 call 0x494e10` reads the cursor spell and `0x4b3293 call 0x495190` clears the cursor with both
flags 1. `0x4b3298 cmp esi,edi` is the same-spell test (the drop sound at `0x4b32e5`); `0x4b32aa jg`
leaves when the cursor held another spell; `0x4b32b6` takes the pet-book branch; `0x4b32ca` and
`0x4b32bd` are the two pickups; `0x4b32f5` plays the pickup sound.

## Casting a book spell

`CastSpell` (`0x4b42f0`) runs `0x4b3300(slot, book, target)`; the target is the current target, or the
player when the self flag is set. A spell on the cursor goes to `0x4b3260` instead. The slot's spell is
cast directly when its first effect is 47. If its aura toggle is up (`0x4b36f0`, the active icon test on
the player or pet) the aura is cancelled (`0x6e7040`; `CMSG_PET_CANCEL_AURA` 619 for the pet). If it
is the spell awaiting a target, targeting stops. A spell with a shapeshift aura whose form is the
current one cancels the form, or does nothing when the form record's flag `+0x2c & 2` is set. Else
it casts: `0x6e5a90`, or sends `CMSG_PET_ACTION` for the pet book.

Addresses: `0x4b42f0`, `0x4b4326`, `0x4b432d`, `0x4b3300`, `0x4b3310`, `0x4b337c`, `0x4b338d`, `0x4b33aa`, `0x4b33ec`, `0x4b3464`, `0x4b3466`, `0x4b3474`, `0x4b347d`, `0x4b3497`, `0x4b35cf`, `0x4b35da`, `0x4b34ce`, `0x4b3510`, `0x4b35ee`, `0x4b36f0`, `0x6e5a90`, `0x6e7040`, `0x6e48e0`, `0x6e4900`
Confidence: verified
Proof: one
Test: none

`0x4b4326` and `0x4b432d` load the current target and, when argument 3 is true, the player (`0x4b433c`
calls `0x6f1c10`). `0x4b3310` loads the cursor spell. `0x4b337c cmp [rec+0xf4],0x2f` then `0x4b338d`
casts. `0x4b33aa call 0x4b36f0`; a true result goes to `0x4b3464` (cancel, player book) or builds
`push 0x26b` at `0x4b33ec` (pet book). `0x4b3474` compares the targeting spell, `0x4b347d` stops it.
`0x4b3497` scans the three aura names for 0x24; `0x4b35cf test [form+0x2c],2` and the cancel at
`0x4b35da`. `0x4b34ce push 0x175` opens `CMSG_PET_ACTION`: pet guid, `spell & 0xffff | 0x1000000` (`0x4b3510`),
target guid. `0x4b35ee` is the player cast. `0x4b36f0` is the book twin of `ui/actionbar#active-icon-test`.

## Trainer icons

`GetTrainerServiceIcon` (`0x4d8f50`) takes the wire spell of the service. When the trainer type
`0xb73a08` is 2 and one of the spell's three effects is 36 or 57, it takes that effect's trigger spell
(`+0x1b4`); if the trigger spell's first item type (`+0x19c`) is nonzero it shows the display icon of
that item's template, nil while the template is requested. In every other case it shows the wire
spell's own `SpellIconID`, never the trigger spell's.

Addresses: `0x4d8f50`, `0x4d8fe2`, `0x4d8ff5`, `0x4d8ffa`, `0x4d9050`, `0x4d906a`, `0x4d9096`, `0x4d90a4`, `0x4d9008`, `0x4d89b0`, `0x4d9140`, `0xb73a08`, `0x4d7d40`, `0x55ba30`
Confidence: verified
Proof: one
Test: none

`0x4d8fe2 cmp [0xb73a08],2` gates the effect scan (`0x4d8ff5`, `0x4d8ffa` compare 0x24 and 0x39); the
register holding the wire record is never reassigned before the fallback at `0x4d9008`. `0x4d9050` loads
`[rec+4i+0x1b4]` and `0x4d906a` the item type `+0x19c` of the trigger record; `0x4d9096` requests the
template with the callback `0x4d9140`, and a miss pushes nil (`0x4d90a4`). The callback refreshes the
trainer list (`0x4d7d40`), which signals `TRAINER_UPDATE`. The trainer type is the dword the trainer list
message stores (`0x4d7440`); `0x4d89b0` returns the service record.

## Recipe icons

`GetTradeSkillIcon` (`0x4fdae0`) reads `EffectItemType[0]` (`+0x19c`) of the recipe spell without testing its
effect and shows that item's icon; it gives nil when the type is 0 or the template is unknown, and never
shows the spell's own icon. `GetCraftIcon` (`0x4f7160`) shows the recipe spell's own `SpellIconID` and
never reads the item type. The windows differ: the trade skill window shows what a recipe makes, the
craft window the spell.

Addresses: `0x4fdae0`, `0x4fdb82`, `0x4fdba8`, `0x4fdc15`, `0x4fdc30`, `0x4f7160`, `0x4f71c7`, `0x4f71d9`
Confidence: verified
Proof: one
Test: none

`0x4fdb82` reads `+0x19c` of the recipe record with no effect test, `0x4fdba8` requests the template with
the callback `0x4fdc30`, and `0x4fdc15` is the shared nil exit. `0x4f71c7` reads `+0x1d4` of the record
and `0x4f71d9` maps it through the SpellIcon table.
