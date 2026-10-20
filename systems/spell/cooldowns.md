# spell: cooldowns

The spell history that answers whether a spell or item is ready, the packets that fill and clear
it, and the item-cooldown hash table.

The spell history is two lists of 48-byte `SPELLHISTORY` records (the type tag of their
allocations), the player's at `0xcecaec` and the pet's at `0xcecb04`. A record holds three
timers for one spell and item: the spell's own recovery, its category's recovery, and the global
cooldown, with a held flag that parks the first two until the server starts them. Records are
appended, never merged; a query takes the latest end over every record that applies. Outside
this topic, the cast send adds the global cooldown (`records`) and the spell-go handler adds the
player's and the pet's own recovery and runs the expiry sweep (`handlers`). The item-cooldown
table at `0xceca60` is a Storm hash table of `ITEMCOOLDOWNHASHNODE` records keyed by item; the
spell-go handler fills it while an item record loads. All of it sits in `Spell_C.cpp`. The rest
of the client reaches the five packet handlers, the cooldown setter `0x6e2c60` that the initial
spell lists use, the category adjustment `0x6e2b60`, the held-item test `0x6e2fc0` and the
shutdown clear `0x6e9ad0`.

Roots: `0x6e9460`, `0x6e95d0`, `0x6e9670`, `0x6e9730`, `0x6e2b60`, `0x6e2c60`, `0x6e2fc0`, `0x6e9ad0`, `0x6e0ed0`, `0x6e10a0`, `0x6e12b0`, the vtable slots `0x6ea500`, `0x6ea580`, `0x6ea7c0`, `0x6ea640`

## History record layout

Each of the two is 24 bytes: a record list (link offset 0, then its end link) and a free list of the
same shape. A record is a link at `+0x00`, then the spell id, item id, recovery start, recovery
time, category, category start, category time, a held byte at `+0x24`, the global-cooldown
category and the global-cooldown time at `+0x2c`. Times are milliseconds of the
`OsGetAsyncTimeMs` (`0x42c010`) clock.

Addresses: `0xcecaec`, `0xcecb04`, `0x6e12c0`, `0x6e139a`, `0x6e13d3`, `0x6e1367`
Confidence: verified
Proof: one
Test: none

`0x6e12c0` stores its ten stack arguments at `+0x08` through `+0x2c` (`0x6e139a` to `0x6e13d3`),
the eighth as a byte (`mov byte [esi+0x24],cl`). New records come from `SMemAlloc(0x30)` with the
tag `.?AUSPELLHISTORY@@` (`0x6e1367`). Every list walk starts at `+0x08` and follows the
record's `+0x04`; a value with bit 0 set or zero ends it. Every list argument in the image
is `0xcecaec + 0x18·n`, computed or constant, `n` 0 for the player and 1 for the pet. The meanings of the fields
follow from their writers in this doc and in `handlers`; the clock is that of `0x6e2c60` and
`0x6e13e0`.

## Adding a record

`0x6e12c0(list, spell, item, start, time, category, categoryStart, categoryTime, held,
gcdCategory, gcdTime)` adds nothing when the recovery time, the category time, the held byte and
the global-cooldown time are all zero. Otherwise it takes the free list's head, or allocates one,
links it at the record list's tail, and stores the ten values. It never looks for a record of the
same spell, so a spell can hold several records at once.

Addresses: `0x6e12c0`, `0x6e12c3`, `0x6e12e6`, `0x6e1354`, `0x6e1395`, `0x6ea450`
Confidence: verified
Proof: one
Test: none

`0x6e12c3` to `0x6e12e0` test `+0x14`, `+0x20`, the byte `+0x24` and `+0x2c` of the frame and
jump to the `ret 0x28` when all are zero. `0x6e12e6` reads the free list's head `[list+0x14]`;
an untagged one is unlinked and relinked before the record end link (`0x6e1354` to `0x6e1362`:
`[esi] = [list+4]`, the old last's next and `[list+4]` become the record). Otherwise
`0x6e1395` links the new record with `0x6ea450(record, 2, 0)`, which links before the end link,
the tail. No instruction compares a record's spell.

## Reading a cooldown

`0x6e13e0(list, spell, item, &duration, &start, &enable)` sets `enable` to 1, then returns 0 for
a spell id outside `Spell.dbc` and for a spell whose `+0xf4` is 0x4e or 0x2f. Otherwise it walks
every record with `best = now` and three legs per record. A leg passes when its end minus `best`
is not negative (signed 32-bit); it then writes `duration` and `start` and sets `best` to its end,
and the two held legs set `enable` to `held == 0` while `enable` is still nonzero. It returns
`best != now`.

| leg | applies when | end | duration, start written |
|---|---|---|---|
| spell | same spell and item, and time or held nonzero | time + (held ? now + 1 : start) | held ? 1 : time; held ? now : start |
| category | category nonzero and the spell's, or its `SpellCategory` flags `& 2` | catTime + (held ? now : catStart) | catTime; held ? now : catStart |
| global | GCD category equals the spell's `+0x274`, GCD time nonzero | `+0x10` start + gcdTime | gcdTime; `+0x10` start |

Addresses: `0x6e13e0`, `0x6e1406`, `0x6e1439`, `0x6e1442`, `0x6e1457`, `0x6e14c5`, `0x6e14f7`, `0x6e1563`, `0x6e1567`, `0x6e1588`, `0x6e15cc`, `0x6e15df`, `0x6e160e`, `0x55ba30`
Confidence: verified
Proof: one
Test: none

The spell's category is its record's `+0x8`; with an item (`0x6e1457`), `0x55ba30` fetches the
item record and each of its five spell slots `+0x11c` that names the spell and has a positive
`+0x16c` replaces the category, the last one winning. `0x6e14c5` matches `+0x08` and `+0x0c`; the
held spell leg uses `lea esi,[now+1]`. `0x6e14f7`, `0x6e1588` and `0x6e15df` are `sub; js` against
`best`. The category leg reads the `SpellCategory` record for the record's category and tests
`[rec+4] & 2` (`0x6e1567`) after a null test (`0x6e1563`). The global leg (`0x6e15cc`) never reads the spell or item and never
writes `enable`. `0x6e160e` is `setne al` on `best` against `now`. The output pointers are written
only by passing legs, so a caller sees its own initial values when none passes.

## Held records

`0x6e1690(list, spell, item)` returns 1 when a record is held and either matches the spell and
item, or has the spell's category (the item's, as in `Reading a cooldown`) and a nonzero category
time; otherwise 0. Unlike the reading's category leg, a category of 0 matches records of category
0. `0x6e2fc0(item)` fetches the item record for the player, takes the first of its five spell
slots with a positive spell id and a zero `+0x130`, and returns `0x6e1690` on the player's list
for that spell and item; with no record or no such slot it returns 0.

Addresses: `0x6e1690`, `0x6e173f`, `0x6e1756`, `0x6e175c`, `0x6e1768`, `0x6e2fc0`, `0x6e2ffb`, `0x6e3017`, `0x6e303e`
Confidence: verified
Proof: one
Test: none

`0x6e173f` and `0x6e1744` match `+0x08` and `+0x0c`; whatever the time at `+0x14`, both paths
reach `0x6e1756`, `cmp byte [eax+0x24],0; jne` to the `mov eax,1`. `0x6e175c` compares `+0x18`
with the category without a zero test (the item's from `0x55ba30` at `0x6e16f6`), then requires `+0x20` and the held byte (`0x6e1768`).
`0x6e2fc0` calls `0x55ba30` with the player's guid and the callback `0x6e2f70` (`0x6e2ffb`), scans
`+0x11c`/`+0x130` (`0x6e3010` to `0x6e301a`) and calls `0x6e1690` with `0xcecaec`, the item id
pushed at `0x6e3037` (`0x6e303e`). One caller outside spell calls `0x6e2fc0`; inside, the cast-failure path and
`0x6e2fa0` (`records`) call `0x6e1690`.

## Starting or removing a spell

`0x6e3050(spell, time, pet, remove)` calls `0x6e1790` on the list. For each record of the spell:
with `remove`, it moves the record to the free list; without it, a held record gets
`start = categoryStart = time` and a cleared held byte, while an unheld record is left alone.
Then it fires the cooldown and usable events and, for the pet's list, `PET_BAR_UPDATE_COOLDOWN`,
else `BAG_UPDATE_COOLDOWN` (`Cooldown events fired`). The global leg reads the start, so a
started record's global cooldown runs again from `time`.

Addresses: `0x6e3050`, `0x6e1790`, `0x6e17cf`, `0x6e17e3`, `0x6e183f`, `0x6e185a`, `0x6e3071`, `0x6e3076`, `0x6e307e`
Confidence: verified
Proof: one
Test: none

`0x6e3050` passes `remove != 0` as a byte (`setne al`) with the list `0xcecaec + 0x18·pet`.
In `0x6e1790`, `0x6e17cf` tests the held byte: unheld and no `remove` skips; held and no `remove`
jumps to `0x6e185a`, which stores `time` at `+0x10` and `+0x1c` and 0 at `+0x24`; the rest unlink
the record and link it before the free list's end link (`0x6e183f` to `0x6e1850`). `0x6e3071`
calls `0x4b31b0`, `0x6e3076` calls `0x4b31c0`, and `0x6e307e` picks `0x4bce90` (`0x6e3080`) or
`0x4f93d0`.
The packet handler and the cast-result and cast-failure paths call it (`handlers`, `failures`).

## Clearing the global cooldown

`0x6e1630(list, spell)` zeroes the global-cooldown category and time of every record of the
spell. When one matched, it runs the expiry sweep at the current time and fires the cooldown
events. The cast-failure path calls it on the player's list for the failed spell (`failures`).

Addresses: `0x6e1630`, `0x6e1658`, `0x6e1670`, `0x6e1678`, `0x6e167d`
Confidence: verified
Proof: one
Test: none

`0x6e1658` matches `+0x08`; `0x6e1660` and `0x6e1663` store 0 at `+0x28` and `+0x2c` and set `dl`.
With `dl` set, `0x6e1670` calls `OsGetAsyncTimeMs`, `0x6e1678` calls `0x6e1910(list, now)` and
`0x6e167d` calls `0x4b31b0`; `ret 4`.

## Expiry sweep and whole lists

`0x6e1910(list, now)` moves to the free list every unheld record whose recovery has ended (time
zero, or `now − (start + time)` not negative) and whose category recovery has ended likewise. It
never reads the global-cooldown fields, so a record with only a global cooldown goes at the first
sweep. `0x6e1880(list)` moves every record to the free list. `0x6e9700(pet)` empties the chosen list
and fires the cooldown events, then `PET_BAR_UPDATE_COOLDOWN` or `BAG_UPDATE_COOLDOWN`.

Addresses: `0x6e1910`, `0x6e1946`, `0x6e195d`, `0x6e1973`, `0x6e19d1`, `0x6e1880`, `0x6e18ea`, `0x6e9700`, `0x6e970d`, `0x6e971c`, `0x6e9721`
Confidence: verified
Proof: one
Test: none

`0x6e1946` skips a held record. With `+0x14` nonzero, `0x6e195d` computes `now − ([+0x10] +
[+0x14])` and `js` keeps the record; `0x6e1973` does the same with `+0x1c` and `+0x20`. The record is
then unlinked and linked before the free end link (`0x6e19d1` to `0x6e19e2`). `0x6e1880` repeats the
unlink and relink (`0x6e18ea`) while the head is untagged. `0x6e9700` computes `0xcecaec +
0x18·pet`, calls `0x6e1880` (`0x6e970d`) and `0x4b31b0` (`0x6e9712`), then tail-jumps to `0x4bce90` for a
nonzero `pet` (`0x6e971c`), else `0x4f93d0` (`0x6e9721`).

## Category recovery adjustment

`0x6e2b60(&value, spellRecord, skip)` adjusts a category recovery for the active player; with no
player it does nothing. Unless `skip`: when the spell's category has `SpellCategory` flags `& 1`
and the player's inventory slot 15 holds an item whose record is loaded, `value =
(record[+0x110] · value) / 1000`; then, when the spell's `+0x18` has `0x2` set and its `+0x20` lacks
`0x20000`, it adds the player's descriptor dword `+0x1e8`. In all cases with a player it then applies
spell modifier 11 (`0x6e6af0`).

Addresses: `0x6e2b60`, `0x6e2b96`, `0x6e2baa`, `0x6e2bcf`, `0x6e2be0`, `0x6e2c0d`, `0x6e2c1d`, `0x6e2c2c`, `0x6e2c41`, `0x6e2c47`, `0x6e2c50`, `0x6228a0`
Confidence: verified
Proof: one
Test: none

`0x468460` with type mask 0x10 on `0x468550`'s guid (`0x6e2b96`) gives the player; a null one
jumps to the epilogue. `0x6e2baa` skips to `0x6e2c49` for a nonzero `skip`. `0x6e2bcf` tests
`[catRec+4] & 1`. The player's slot 4 returns its inventory, `0x6228a0(15)` the item object, and
the item's descriptor `+0x0c` keys `0x55ba30`. `0x6e2c0d` loads `+0x110` and `0x6e2c13` is `imul ecx,[esi]` (the low 32 bits),
then `imul 0x10624dd3`, `sar edx,6` and the sign-bit add: signed division by 1000, truncated
toward zero. `0x6e2c41` loads `[[player+0x110]+0x1e8]` and `0x6e2c47` adds it. `0x6e2c50` calls `0x6e6af0` with 0xb. The
`+0x1e8` dword is the unit's ranged attack time and slot 15 the main hand by the community's
update-field and slot tables, which the bytes do not name.

## Cooldowns from remaining times

`0x6e2c60(spell, item, category, recovery, categoryRecovery, held, pet)` records cooldowns given as
remaining milliseconds. A positive `recovery` gives `time = mod11(max(base, recovery))` and `start
= now − time + recovery`, the base being the item slot's `+0x158` when nonnegative, else the
spell's `+0x4c`. A positive `categoryRecovery` gives `catTime = max(c, categoryRecovery)` and
`catStart = now − catTime + categoryRecovery`, `c` being the slot's `+0x180` when nonnegative,
else `0x6e2b60(spell +0x50, skip = pet)`. It adds the record with no global cooldown and fires no
event.

Addresses: `0x6e2c60`, `0x6e2c81`, `0x6e2cf4`, `0x6e2d1d`, `0x6e2d31`, `0x6e2d40`, `0x6e2d4e`, `0x6e2d60`, `0x6e2d80`, `0x6e2d8b`, `0x6e2d94`, `0x6e2dcc`
Confidence: verified
Proof: one
Test: none

`now` is `OsGetAsyncTimeMs` (`0x6e2c81`). The item slot is the first of five `+0x11c` entries equal
to the spell (`0x6e2cf4`); none found drops the item record. `0x6e2d1d` and `0x6e2d60` are `jle`
on the arguments. `0x6e2d31` keeps the larger (`cmp; jg`, signed) before `0x6e2d40` applies
modifier 11; `0x6e2d4e` is `now − time + recovery`. On the category side the modifier, inside
`0x6e2b60` (`0x6e2d80`), precedes the maximum (`0x6e2d8b`), and an item value skips both;
`0x6e2d94` forms the start. `0x6e2dcc` passes the argument `category`, `held` and zeros for the
global cooldown to `0x6e12c0` on `0xcecaec + 0x18·pet`. The sixth stack argument is never read
(`ret 0x18`). Its three callers are outside spell: two read packet entries and pass bit 31 of the
category recovery dword as `held`, clearing it from the value; one passes `held` 1 and the spell's
`+0x4c` as the recovery.

## Spell cooldown packet

`SMSG_SPELL_COOLDOWN` (`0x134`, `0x6e9460`) is a 64-bit guid, then pairs of a 32-bit spell id and
a 32-bit time to the end. The player's guid picks the player's list, the pet bar's guid the pet's;
any other guid ends the handler. For each pair with a valid spell, the record is: start and
category start the packet's time; time the packet's value, or modifier 11 of `+0x4c` when it is
0; category `+0x8`; category time `0x6e2b60(value ? 0 : +0x50, skip = pet)`; held from bit 25 of
`+0x18`; and, unless held, the global-cooldown category `+0x274` and modifier 21 of `+0x278`.

Addresses: `0x6e9460`, `0x6e9471`, `0x6e947e`, `0x6e948d`, `0x6e94ae`, `0x6e94c6`, `0x6e94d1`, `0x6e9504`, `0x6e9516`, `0x6e9544`, `0x6e9549`, `0x6e9562`, `0x6e9598`, `0x6e95b0`, `0x6e95b9`, `0xb714a0`
Confidence: verified
Proof: one
Test: none

`0x6e9471` reads the guid (`0x4190b0`, eight bytes); `0x6e947e` compares it with `0x468550`'s,
`0x6e948d` with `0xb714a0`. The loop runs while the store's slot 5 (`0x6e94ae`) returns 0, reading
`0x418e30` and `0x418fb0`, four bytes each. `0x6e9504` is `shr ebx,0x19; and bl,1`. A nonzero value
is stored as the time (`0x6e9511`); a zero one takes `+0x4c` through `0x6e6af0(0xb)` (`0x6e9516`).
`0x6e9544` passes `edi`, the list index, as `skip`. A held spell pushes 0 and 0 (`0x6e9549`), else
`+0x274` and `+0x278` (`0x6e9553`) through modifier 0x15 (`0x6e9562`). `0x6e9598` adds the record with item 0;
`0x6e95b0` calls `0x4b31b0`, and `0x6e95b9` `0x4bce90` for the pet. It returns 1 (`ret 8`). The
message names in this doc are the server's names for the opcodes.

## Item cooldown packet

`SMSG_ITEM_COOLDOWN` (`0x0b0`, `0x6e95d0`) is a 64-bit item guid and a 32-bit spell id. For a valid
spell whose item object exists, it adds to the player's list a record of the spell and the item's
entry with start the packet's time, a recovery of 30,000 ms and nothing else. It always fires the
cooldown events and `BAG_UPDATE_COOLDOWN`.

Addresses: `0x6e95d0`, `0x6e95e0`, `0x6e95eb`, `0x6e9623`, `0x6e962f`, `0x6e9641`, `0x6e9651`, `0x6e9656`, `0x6e965b`
Confidence: verified
Proof: one
Test: none

`0x6e95e0` reads the guid, `0x6e95eb` the spell id over the store argument. `0x6e9623` is
`0x468460` with type mask 2 (an item) on the guid; `0x6e962f` reads the object's descriptor
`+0x0c`, its entry. The pushes from `0x6e9635` are six zeros, `0x7530` (`0x6e9641`), the packet's
time, the entry and the spell, to `0x6e12c0` on `0xcecaec` (`0x6e9651`). `0x6e9656` and `0x6e965b`
call `0x4b31b0` and `0x4f93d0` on every path.

## Cooldown event, clear and cheat

`0x6e9670` handles `SMSG_COOLDOWN_EVENT` (`0x135`) and `SMSG_CLEAR_COOLDOWN` (`0x1de`): a 32-bit
spell id, then a 64-bit guid. The pet bar's guid picks the pet's list and every other guid the
player's. It calls `0x6e3050(spell, packet time, list, opcode == 0x1de)`: the event starts held
records, the clear removes every record of the spell. `SMSG_COOLDOWN_CHEAT` (`0x1e1`, `0x6e9730`)
is a 64-bit guid; the player's guid empties the player's list and the pet bar's the pet's
through `0x6e9700`, and any other does nothing. Each returns 1.

Addresses: `0x6e9670`, `0x6e9683`, `0x6e968e`, `0x6e9696`, `0x6e96b2`, `0x6e96ce`, `0x6e96df`, `0x6e9730`, `0x6e974a`, `0x6e9757`, `0x6e9767`, `0x6e977c`, `0x6e7150`
Confidence: verified
Proof: one
Test: none

`0x6e9683` reads `0x418e30` into the store argument's slot, `0x6e968e` the guid. `0x6e9696`
compares with `0xb714a0` only. `0x6e96b2` tests the opcode for `0x1e1` and calls `0x6e9700`, but
`Spell_C::SystemInitialize` (`0x6e7150`) registers `0x6e9670` for `0x135` and `0x1de` only and
`0x6e9730` for `0x1e1` (`tables/opcodes.tsv`), so that branch never runs. `0x6e96ce` is `cmp
edi,0x1de; sete dl`, pushed as `remove`, and `0x6e96df` calls `0x6e3050`. `0x6e974a` compares with
the player's guid and `0x6e9757` calls `0x6e9700(0)`; `0x6e9767` compares with `0xb714a0` and
`0x6e977c` calls `0x6e9700(1)`.

## Cooldown events fired

The cooldown and usable events take no arguments, except the `COMBAT_TEXT_UPDATE` of `0x4b31c0`, which
carries `SPELL_ACTIVE` and a spell name. Each path fires, in order:

| path | events (id) |
|---|---|
| `0x4b31b0` | `ACTIONBAR_UPDATE_COOLDOWN` (216), `SPELL_UPDATE_COOLDOWN` (262) |
| `0x4b31c0` | `ACTIONBAR_UPDATE_USABLE` (215) when an action's usable state changed, `COMBAT_TEXT_UPDATE` (542) `SPELL_ACTIVE` for each reactive spell that became usable, then `SPELL_UPDATE_USABLE` (263) |
| `0x4bce90` | `PET_BAR_UPDATE_COOLDOWN` (354) |
| `0x4f93d0` | `BAG_UPDATE_COOLDOWN` (330) |
| spell cooldown packet | `0x4b31b0`, then `0x4bce90` for the pet |
| item cooldown packet | `0x4b31b0`, `0x4f93d0` |
| event and clear | `0x4b31b0`, `0x4b31c0`, then `0x4bce90` for the pet, else `0x4f93d0` |
| cheat | `0x4b31b0`, then `0x4bce90` for the pet, else `0x4f93d0` |
| global cooldown cleared | `0x4b31b0` |

Addresses: `0x4b31b0`, `0x4e5c60`, `0x4b31c0`, `0x4b3226`, `0x4b3246`, `0x4bce90`, `0x4f93d0`, `0x703e50`
Confidence: verified
Proof: one
Test: none

`0x4b31b0` calls `0x4e5c60`, which jumps to `FrameScript_SignalEvent` (`0x703e50`) with `ecx =
0xd8`, and then jumps there with `0x106`. `0x4b31c0` refreshes the action slots' usable cache,
which signals `0xd7` only on a change, retests each reactive spell and raises `0x21e` with `SPELL_ACTIVE`
for one that became usable (`0x4b3226`, `ui/reactivespells#usable-refresh`), and signals `0x107`
at `0x4b3246`. `0x4bce90` and `0x4f93d0` jump there with `0x162` and `0x14a`. The
names are `tables/events.tsv`'s. `0x6e2c60`, `0x6e1910` and `0x6e1880` fire nothing.

## Spell id by name

`0x6e30a0(name)` returns the id of the first `Spell.dbc` record, in the store's record order, for
which `0x64a4c0(localized name, name, 0x7fffffff)` returns 0. With none it passes `Unknown Spell
'%s'` and the name to `0x63cd00` and returns −1. Its one caller is `0x6e5af8`, in the unreached
`0x6e5ad0` (`cast`), so nothing live reaches it.

Addresses: `0x6e30a0`, `0x6e30e1`, `0x6e30ef`, `0x6e3111`, `0x6e3123`, `0x64a4c0`, `0x63cd00`, `0x6e5ad0`, `0x6e5af8`
Confidence: verified
Proof: one
Test: none

The loop runs `i` from 0 below the store's record count with a stride of `0x2b4`, the record size.
`0x6e30e1` loads the string pointer at `+0x1e0 + 4·locale`, field 120 onward, the localized name.
`0x6e30ef` calls `0x64a4c0(recordName, name, 0x7fffffff)`, and a zero result returns `[rec]`
(`0x6e3123`). `0x6e3111` calls `0x63cd00` with the format, 0 and the name. `0x64a4c0` passes its
three arguments to the C runtime's `__strnicmp` by the community's names, so the match ignores
case; the console write is `0x63cd00`'s community name.

## Item cooldown table

`0xceca60` is a hash table with the vtable `0x811904`: a list of every node (link offset `0x0c`),
a counter, an array of 12-byte slot lists (count `+0x18`, data `+0x1c`), and a mask at `+0x24`, −1
with no slots. A node's slot is its key, `+0x00`, AND the mask; its slot link is at
`+0x04`. Slot 0 of the vtable unlinks a node from both lists and frees it; slot 1 allocates a node
of 0x24 bytes plus extra at a slot list's head; slot 2 destroys the table, freeing it on flag bit
0; slot 3 unlinks every node and frees the slot array. `0x6e9ad0` frees every node and empties
both history lists.

Addresses: `0xceca60`, `0x811904`, `0x6e0e00`, `0x6e0ed0`, `0x6ea500`, `0x6ea580`, `0x6ea7c0`, `0x6ea640`, `0x6e9ad0`, `0x6e9b2d`, `0x6e9b48`, `0x6e87ef`, `0x6e99e0`
Confidence: verified
Proof: one
Test: none

`0x6e0e00` stores the vtable, the end link and offset `0x0c`, zeros, and −1 at `+0x24`; it and
`0x6e0ed0` and `0x6ea7c0` are the image's three stores of `0x811904`, whose four dwords are
`0x6ea500`, `0x6ea580`, `0x6ea7c0` and `0x6ea640`. `0x6ea500` unlinks `+0x0c` and `+0x04` and calls
`SMemFree` with the tag `.?AUITEMCOOLDOWNHASHNODE@@`; `0x6ea580` calls `SMemAlloc(extra + 0x24, …,
flags | 8)`. `0x6e9ad0` zeroes `+0x10`, unlinks the node list, calls slot 0 on each slot list's head
until empty (`0x6e9b2d`), and calls `0x6e1880` on `0xcecaec` and `0xcecb04` (`0x6e9b48`); the
spell shutdown `0x6e99e0` (`handlers`) jumps to it. Slot 1 is called at `0x6e87ef` (`handlers`).
`0x6e0ed0`, the exit destructor, unlinks every node without freeing it, destroys the slot lists and
frees their array with the list type's tag.

## Item cooldown table internals

The table's lists are Storm's: a link is a previous-link pointer and a next-node pointer, bit 0
marking the end link. A resize to `n` slots (`0x6eaf90`) moves every node to a scratch list, sets
the array's count to `n`, resets every slot list to link offset 4, sets the mask to `n − 1` and
relinks each node at its slot's tail. Growing the array rounds its capacity up to a multiple of a
quantum: the stored one, or for a size from 21 up the stored 21, else the largest power of two not
above the size; then it reallocates in place, or allocates, rebuilds the lists empty and frees the
old array.

Addresses: `0x6eaf90`, `0x6eb0fe`, `0x6eb119`, `0x6eb137`, `0x6eb270`, `0x6eb297`, `0x6eb2ca`, `0x6eb400`, `0x6eb4af`, `0x6eb540`, `0x6eb599`, `0x6eb180`, `0x6eb1b0`, `0x6eb260`, `0x6ea910`, `0x6ea950`, `0x6ea980`, `0x6eaa10`, `0x6eadf0`, `0x6eae30`, `0x6eae50`, `0x6eaf10`, `0x6eaf60`
Confidence: verified
Proof: one
Test: none

In `0x6eaf90` the scratch list takes offset 4, the old slots' nodes are appended, `0x6eb400` and
`0x6eb180`/`0x6eb1b0` grow or shrink the array, `0x6eb0fe` stores 4, `0x6eb119` stores `n − 1`, and
`0x6eb137` indexes `[node] & mask`. `0x6eb297` keeps 21 at `+0x0c`; the power of two is the last
nonzero `x & (x − 1)` (`0x6eb29e` to `0x6eb2ac`); `0x6eb2ca` is the `div`. `0x6eb4af` and `0x6eb599`
call `SMemReAlloc` with flag 0x10. `0x6eb180` builds a slot list with offset `0xdddddddd`,
`0x6eb260` resets an end link, `0x6eb1b0` destroys one and frees it on flag bit 0. `0x6ea910` and
`0x6eadf0` unlink one link; `0x6ea950` and `0x6eaf60` return the next link, deriving the offset for
a negative one; `0x6eae30` returns a node's link; `0x6eae50` links a node after (1) or before (2)
another, the end link for none; `0x6eaf10` and `0x6ea980` empty a list, the second also unlinking
its end link; `0x6eaa10` destroys the slot array.

## Static initializers

Six functions run from the C runtime's initializer and exit tables: `0x6e0e00` builds the item
table and `0x6e0ed0` destroys it at exit; `0x6e1050` builds both history lists empty and `0x6e10a0`
frees every record of both at exit; `0x6e11e0` zeroes the cast-request globals (`cast`) and sets
one to −1; `0x6e12b0`, registered at exit, is a bare `ret`.

Addresses: `0x6e0e00`, `0x6e0ed0`, `0x6e1050`, `0x6e1060`, `0x6e10a0`, `0x6e10d2`, `0x6e11e0`, `0x6e127f`, `0x6e12b0`
Confidence: verified
Proof: one
Test: none

The thunks that call or register them belong to `layout`. `0x6e1050` loops twice from `0xcecafc`
with a stride of `0x18` (`0x6e1060`), storing offset 0 and a self end link marked by bit 0 for both
lists. `0x6e10a0` walks the pet's pair, then the player's, freeing each record of both lists with
`SMemFree` and the `SPELLHISTORY` tag (`0x6e10d2`), then unlinks the end links. `0x6e11e0` stores
zero to 26 dwords and two words and `0xffffffff` at `0x6e127f`.
