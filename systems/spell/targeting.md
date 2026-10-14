# spell: targeting

The targeting cursor: the flag word, binding a clicked object or location, target validity and range, and the Lua `Spell*` targeting functions.

When a cast needs a target the player has not given, the client arms targeting: it keeps the
requirements in a 16-bit flag word and waits for a click. These functions sit in `Spell_C.cpp`.
The binder takes a clicked object, and two siblings a clicked location and trade slot 6. Each clears the word's bits it satisfies and records the target in the
pending cast record, whose caster, spell and target slots `cast` owns; an emptied word sends the
cast. The world frame (`worldframe`) asks the word's predicates which objects the mouse may pick,
draws the cursor from the read-only test and the location range test, and sizes and turns the
ground placement. The game UI binds from the selection click, the item clicks, the enchant
confirmations and the minimap ping, and FrameScript reaches five Lua functions through one table.
Every function here except one is reached from outside spell.

Roots: `0x6e5b40`, `0x6e60f0`, `0x6e6180`, `0x6e61a0`, `0x6e61e0`, `0x6e6200`, `0x6e6210`, `0x6e6230`, `0x6e6240`, `0x6e6250`, `0x6e6260`, `0x6e62d0`, `0x6e62e0`, `0x6e6320`, `0x6e6330`, `0x6e6350`, `0x6e6460`, `0x6e6810`, `0x6e6900`, `0x6e6970`, `0x6e6980`, `0x6e69d0`, `0x6e69e0`, and the Lua table entries `0x6e6cd0`, `0x6e6d00`, `0x6e6d90`, `0x6e6e30`, `0x6e6e80`

## The flag word

The word `0xcecac0` holds the target kinds the pending spell still needs; targeting is on while
it is nonzero. A bind tries the bits below in a fixed order; the first arm that binds clears its own bits and ORs
the bound kind into the 16-bit target mask at `+0x14` of the pending cast record `0xceac48`,
storing the target at `+0x18` (an object), `+0x20` (an item), `+0x30` or `+0x3c` (a position, three
floats). Bit 0x1 is never tested; 0x1000 occurs only in the mask; 0x2000 is bound by `0x6e5250` from a
pending name (`cast`), and otherwise read only by unreached code.

| bit | binds | mask bit, slot |
|---|---|---|
| 0x2 | a unit without unit-flag 0x100 | 0x2, `+0x18` |
| 0x4 | a unit passing `0x606d20` and `0x6066f0` | 0x2, `+0x18` |
| 0x8 | a unit passing `0x606c20` and `0x6066f0` | 0x2, `+0x18` |
| 0x10 | an item, or trade slot 6 | 0x10 or 0x1000, `+0x20` |
| 0x20 | a position, first | 0x20, `+0x30` |
| 0x40 | a position, when 0x20 is clear | 0x40, `+0x3c` |
| 0x80 | a unit passing `0x606980` | 0x2, `+0x18` |
| 0x100 | a unit passing `0x6066f0` | 0x2, `+0x18` |
| 0x200 | a dead player failing `0x6066f0`; a corpse failing `0x6067d0` | 0x2 or 0x200, `+0x18` |
| 0x400 | nothing: refuses a live unit; cleared by a dead unit's 0x2, 0x4, 0x8, 0x80 or 0x100 bind | none |
| 0x800, 0x4000 | a game object (either bit); 0x4000 also an item or trade slot 6 | 0x800, `+0x18` |
| 0x8000 | a dead player, or dead unit with `+0x214` set, passing `0x6066f0`; a corpse passing `0x6067d0` | 0x2 or 0x8000, `+0x18` |

Addresses: `0xcecac0`, `0xceac48`, `0x6e4955`, `0x6e5057`, `0x6e5098`, `0x6e525e`, `0x6e6096`, `0x6e6155`, `0x6e691c`, `0x6e72e0`
Confidence: verified
Proof: one
Test: none

A raw-dword census of `0xcecac0` and `0xcecac1` finds 64 and 15 references, all in spell's code,
and none of the two bytes above it; the control is the known read at `0x6e48a2`. The writers are `AbortCast`
(`0x6e4955`, zero), the cast request (`0x6e5057`, `0x6e5098`, zero), `0x6e5250` (`0x6e525e`, a
16-bit store, then byte edits to `0x6e531b`), the binder's clears and its restore (`0x6e6096`), the
location bind (`0x6e6155`), the trade-slot bind (`0x6e691c`) and the system start (`0x6e72e0`). The
census covers direct references only: the word lies `+0x60` into the object whose vtable
the store at `0x6e0ed4` installs (`cooldowns`), whose methods were not walked for a store there. The table's rows are the binders' arms, below; every store
to the word is 16-bit or a byte edit, and the predicates read the dword but mask the low half.

## Flag word predicates

Eleven live predicates return 1 when the flag word, read whole, shares a bit with a mask; two
return a single bit. `0x6e61a0` also requires that the pending spell lack `AttributesEx` 0x80000
(record `+0x1c`); it reads the record without a null check, so an invalid spell id faults.

| function | test |
|---|---|
| `0x6e6180` | word & 0x878e |
| `0x6e61a0` | word & 0x850e, and record `+0x1c` & 0x80000 clear |
| `0x6e61e0` | word & 0x040c |
| `0x6e6200` | word & 0x0500 |
| `0x6e6210` | word & 0x0480 |
| `0x6e6230` | word & 0x8600 |
| `0x6e6240` | bit 15 (0x8000), as 0 or 1 |
| `0x6e6250` | bit 9 (0x200), as 0 or 1 |
| `0x6e62d0` | word & 0x4800 |
| `0x6e6320` | word & 0x60 |
| `0x6e6330` | word & 0x4010 |

Addresses: `0x6e6180`, `0x6e61a0`, `0x6e61cd`, `0x6e61e0`, `0x6e6200`, `0x6e6210`, `0x6e6230`, `0x6e6240`, `0x6e6250`, `0x6e62d0`, `0x6e6320`, `0x6e6330`, `0x481050`, `0x7cba30`, `0x4c7300`, `0x4f9b30`
Confidence: verified
Proof: one
Test: none

Each is `xor al,al; test <word>,<mask>; je; mov al,1; ret`; `0x6e6240` is `movzx` of the word,
`shr 15`, and `0x6e6250` `shr 9; and 1`. `0x6e61a0` sets `ecx` to 0 for an id outside the store
(`0x6e61cd`) and reads `[ecx+0x1c]`. The world frame's pick-mask builder `0x481050`, while
targeting, ORs 3 for `0x6e6320`, 4 for `0x6e62d0`, 0x18 for `0x6e6180` and then 0x20, 0x10000,
0x20000 and 0x40000 for `0x6e61a0`, `0x6e61e0`, `0x6e6200`, `0x6e6210`, then 0x200040 or 0x100000
by `0x6e6230`, and in every case 0x20040 and 0x40040 for `0x6e6240` and `0x6e6250`. `0x7cba30`, a
vtable slot, returns 0 while targeting a position with no unit bit. The item clicks
`0x4c7300` and the Lua `PickupContainerItem` `0x4f9b30` pass the item to `0x495d60` while
`0x6e6330` holds.

## Binding a clicked object

`0x6e5b40` binds the object in `ecx` to the flag word and returns 1 when it bound. Each exit
below returns 0 with no write and no message, in this order; the unit arms then run in the order
0x8, 0x4, 0x100, 0x80, 0x2, 0x8000, 0x200, and the first to bind wins. Items, game objects and
corpses take the arms of the flag word's table; a corpse is tried for 0x8000 and then 0x200, each
requiring corpse flag 0x1 clear. A miss of every arm is silent too.

| exit | when |
|---|---|
| `0x6e5b5e`, `0x6e5b67` | the object is null; the word is 0 |
| `0x6e5b93` | the caster guid at `+0x08` resolves to no unit |
| `0x6e5ba5` | `0x494a50(2)` returns 0 |
| `0x6e5bd2` | the spell id at `+0x10` has no record |
| `0x6e5bf7` | the object is the caster and `AttributesEx` has 0x80000 |
| `0x6e5c1d` | a unit with unit-flag 0x10000 |
| `0x6e5c53` | a unit outside a nonzero `TargetCreatureType` mask |
| `0x6e5c8b` | a dead unit (health ≤ 0) with dynamic flag 0x20 clear, no 0x8600 in the word, `AttributesEx2` bit 0 clear |
| `0x6e5ca2` | a live unit while the word has 0x400 |

Addresses: `0x6e5b40`, `0x6066f0`, `0x606980`, `0x606c20`, `0x606d20`, `0x6e5b5e`, `0x6e5b67`, `0x6e5b93`, `0x6e5ba5`, `0x6e5bd2`, `0x6e5bf7`, `0x6e5c1d`, `0x6e5c53`, `0x6e5c8b`, `0x6e5ca2`, `0x6e5d89`, `0x6e5ddb`, `0x6e5e33`, `0x6e5e7f`, `0x6e5f35`, `0x6e5f70`, `0x6e5fc7`, `0x6e600c`, `0x6e6024`, `0x468460`, `0x605570`
Confidence: verified
Proof: one
Test: none

The caster is `0x468460` with type mask 8 on `+0x08` (`0x6e5b93`); `0x6e5bf7` compares guids and
tests record `+0x1c`. Unit fields are read through the object's `+0x110`: health `+0x40` signed
(`jg`), flags `+0xa0`, dynamic flags `+0x224`, and `+0x214` for the dead 0x8000 arm, which takes a
player (type bit 4) or a nonzero `+0x214`. `0x6e5c53` tests `1 << (type − 1)` from `0x605570`
against record `+0x38`. A dead unit's bind also clears 0x400 (`0x6e5d89`, `0x6e5ddb`), on the 0x2
arm all of 0x8600 (`0x6e5e33`); the 0x8000 and 0x200 unit arms do not. A dead 0x8000 candidate that
fails `0x6066f0` returns 0 (`0x6e5e7f`) without trying 0x200. The type split reads the
descriptor's type mask: bit 3 unit, 1 item (`0x6e5f35` clears 0x4010, stores the guid at `+0x20`),
5 game object (`0x6e5f70` clears 0x4800; no lock test), 7 corpse (field `+0x74` bit 0,
`0x6e5fc7`, `0x6e600c`); any other type returns 0. With no arm bound, `0x6e6024` returns 0.

## The bind's range test

After a bind, when the mask has any of 0x8202 and `+0x18` holds this object, `0x6e47b0` tests the
caster's range to it: a unit or corpse always, an item never (its guid goes to `+0x20`), a game
object only when an earlier bind left a unit or corpse bit in the mask.
Out of range, it calls `0x6e1a00` with the spell id and reason 0x59 (too far) or 0x76 (too
close), then stores back the word and mask read at entry, zeroes `+0x18` and returns 0. In range, or untested, an empty
word fires events 214 and 261 through `0x4b3250` and sends the cast through `0x6e54f0` with `dl` 0;
a word with bits left returns 1 with neither.

Addresses: `0x6e602c`, `0x6e6063`, `0x6e6089`, `0x6e6096`, `0x6e609d`, `0x6e60a4`, `0x6e60c1`, `0x6e60d7`, `0x6e47b0`, `0x6e1a00`, `0x6e54f0`, `0x4b3250`, `0x4e5bf0`
Confidence: verified
Proof: one
Test: none

`0x6e602c` is `test dword [mask],0x8202`, then two guid compares. `0x6e6063` calls `0x6e47b0`
with the caster, the object, the spell id and a local out byte. `0x6e47b0` returns 0 with the byte
clear when `distSq < min²`, and 0 with it set when `distSq > max²` (`checks`).
`0x6e606c` picks `dl` 0x59 (byte set) or 0x76 and `ecx` the record's id; `0x6e6089` calls `0x6e1a00` with −1, −1, 0 pushed. The
restore follows the call: `0x6e6096` stores the 16-bit word saved at entry, `0x6e609d` the mask,
`0x6e60a4` zeroes the guid. So whatever `0x6e1a00` does to the word, targeting is armed again with
the pre-click word (`failures` and `cast` say what it aborts). `0x6e60c1` compares the 16-bit word with 0;
`0x4b3250` calls `0x4e5bf0`, which signals 214 (`ACTIONBAR_UPDATE_STATE`), and signals 261
(`CURRENT_SPELL_CAST_CHANGED`); `0x6e60d7` calls `0x6e54f0(0xceac48, 0)`.

## Binding a clicked location

`0x6e60f0` binds the position at `ecx` (three floats). With a zero word it returns 0 at once.
Otherwise it stores the position at `+0x30` for word bit 0x20, or else at `+0x3c` for 0x40, ORs
that bit into the mask and clears it from the word. It then fires events 214 and 261 whether or
not it bound, and when it bound and the word is now 0 it sends the cast. It applies no range test:
the cursor shows the range, the click does not enforce it.

Addresses: `0x6e60f0`, `0x6e6107`, `0x6e6130`, `0x6e6155`, `0x6e615d`, `0x6e6177`, `0x492580`, `0x4eeca0`
Confidence: verified
Proof: one
Test: none

`0x6e6107` to `0x6e6120` copy three dwords to `+0x30` and OR 0x20; `0x6e6130` does the same at
`+0x3c` with 0x40; `0x6e6155` stores the 16-bit word less that bit. `0x6e615d` calls `0x4b3250`
before the test of `bl`; `0x6e6177` calls `0x6e54f0(0xceac48, 0)` when `bl` is set and the word is 0.
Callers: the world click handler `0x492580` passes its click record (position first) when the
record's `+0xc` is 1; the Lua `PingLocation` `0x4eeca0`, while `0x6e6320` holds, passes the
minimap point with its height from a vertical ray between ±100000 plus 8.0.

## Binding the trade slot

`0x6e6900` binds trade slot 6 (item guid {6, 0}) when the word has 0x10 or 0x4000 and the trade
global `0xb7150c` is nonzero: it ORs 0x1000 into the mask, clears 0x4010, stores the item guid
{6, 0} at `+0x20`, and fires events 214 and 261. When the word is then 0 it sends the cast with
`dl` 1 and calls `AbortCast(0, 0, 0x17)`. It returns 1 when it bound, else 0.

Addresses: `0x6e6900`, `0x6e6915`, `0x6e6925`, `0x6e694f`, `0x6e695a`, `0xb7150c`, `0x48d330`, `0x496170`
Confidence: verified
Proof: one
Test: none

`0x6e6915` is `or byte [mask+1],0x10`; `0x6e6925` stores 6 and 0. `0x6e694f` calls `0x6e54f0`
with `dl` 1; `0x6e695a` calls `0x6e4940` with `cl` 0, `dl` 0, 0x17 pushed. The Lua
`ReplaceTradeEnchant` `0x48d330` calls it directly. `0x496170`, with the same trade global set and
a spell record for the in-flight id, tail-jumps to it unless an effect 0x35 would replace an
enchantment the slot already has, when it signals event 404 (`TRADE_REPLACE_ENCHANT`) instead.

## Testing an object without binding

`0x6e6460` answers whether the object with the guid at `ecx` would bind, writing nothing. It runs
the binder's gates and arms with three differences: it has no zero-word and no `0x494a50` gate,
its game-object arm also requires `0x5f8260`, and every passing arm, items and game objects
included, ends in a range test when `dl` is nonzero. That test returns 1 exactly when
`min² ≤ d ≤ max²`, where `d` is the squared distance rounded to a float; a NaN anywhere fails.
Both callers pass `dl` 1: the world hover, which shows cursor 2 (`Cast`) on 1 and 0x16
(`UnableCast`) on 0, and `SpellCanTargetUnit`.

Addresses: `0x6e6460`, `0x6e6544`, `0x6e668c`, `0x6e66e7`, `0x6e670f`, `0x6e67c8`, `0x6e67d1`, `0x6e67e4`, `0x6e67f4`, `0x5f8260`, `0x6e3480`, `0x4828d0`
Confidence: verified
Proof: one
Test: `spell::spell_objs_dist_sq_difftest`

The object is `0x468460` with type mask 1 on the guid, the caster with mask 8. The item arm
(`0x6e66e7`) and the game-object arm (`0x6e670f`, `0x5f8260(object, 0xceac48, spell)`) fall into
`0x6e668c` like the unit arms. Positions come from slot 5 (`+0x14`) of each object; with `dx =
caster.x − object.x` and so on, at 53-bit precision, `d = (dz² + dx²) + dy²`, stored to a float
(`0x6e67c8`). `0x6e67d1` calls `0x6e3480(caster, spell, &min, &max, object)`. Then `min·min` `fcomp d`
with `test ah,0x41; jp` fails when greater or unordered (`0x6e67e4`), and `max·max` `fcomp d` with
`test ah,1; jne` fails when less or unordered (`0x6e67f4`). The hover `0x4828d0` calls it with the
hovered guid and passes 2 or 0x16 to `0x523c20`. `spell_objs_dist_sq_difftest` runs a slice,
`0x6e6797` to `0x6e67d1`, the squared distance. The arms and the range compare are not run.

## Location range test

`0x6e6810` returns whether a position at `ecx` is within the pending spell's range of the caster.
It returns 0 when the word has neither 0x20 nor 0x40 or the caster is gone, 1 when `dl` is 0, and
otherwise 1 exactly when `min² ≤ d ≤ max²` with `d = (dx² + dz²) + dy²`, caster minus position,
rounded to a float; a NaN fails. The range comes from `0x6e3480` with no target object.

Addresses: `0x6e6810`, `0x6e681d`, `0x6e6872`, `0x6e68a7`, `0x6e68b0`, `0x6e68c3`, `0x6e68d3`
Confidence: verified
Proof: one
Test: `spell::spell_target_in_range_difftest`

`0x6e681d` calls `0x6e6320` and returns its zero. The caster's position is its slot 5 (`0x6e6872`);
the three differences are `fsub` of the position from the caster. The squares are formed in the order
x, z, y and summed `(dx² + dz²) + dy²`, stored to a float at `0x6e68a7`. `0x6e68b0` calls `0x6e3480`
with the target pushed as 0 and `&min` last. The compares at `0x6e68c3` (`test ah,0x41; jp`) and
`0x6e68d3` (`test ah,1; jne`) are those of `0x6e6460`. `spell_target_in_range_difftest` runs
`0x6e6810` whole with the flag word set for a location, the caster lookup stubbed and `0x6e3480`
replaced by a patch that writes the window.

## Object pick filters

The world frame's per-object pick filter `0x480780`, while targeting, admits a game object only
when `0x6e62e0` holds and a corpse only when `0x6e6260` holds. `0x6e62e0` requires word 0x4800, a
game-object type bit and `0x5f8260`. `0x6e6260` requires a live caster and a corpse without flag
0x1: with word bit 0x8000 it returns whether `0x6067d0` holds and never tries 0x200; otherwise,
with 0x200, it returns whether `0x6067d0` fails.

Addresses: `0x6e62e0`, `0x6e6260`, `0x6e6291`, `0x6e62ad`, `0x480780`, `0x6067d0`
Confidence: verified
Proof: one
Test: none

`0x480780` switches on the type mask less 9 through the byte table `0x480850` into four cases:
unit (9) and player (0x19) need pick bits 0x8 and 0x10, game object (0x21) bit 4 then `0x6e62e0`,
corpse (0x81) bit 0x40 then `0x6e6260`; outside targeting the last two pass. In `0x6e6260`,
`0x6e6291` (`test ah,ah; jns`) branches on 0x8000, and a failed test there reaches the 0 exit
without the 0x200 test at `0x6e62ad`. The binder and `0x6e6460` try both.

## Area reticle radius

`0x6e6350` returns the pending spell's area radius: for each of the record's first two effect
radius indexes (`+0x160`, `+0x164`), the `SpellRadius.dbc` row's radius plus the caster's level
times its per-level radius, or 0 for an index with no row. The first is rounded to a float; the
larger wins, the second on a tie or NaN; modifier 6 is applied through `0x6e6bf0`. The spell
record is read with no null check. The third effect
and the row's maximum are not read. With no caster it returns 0. The reticle `0x4820f0` draws
`min(radius, 20)`, 20 for NaN.

Addresses: `0x6e6350`, `0x6e63c7`, `0x6e63f0`, `0x6e641f`, `0x6e6426`, `0x6e643c`, `0x6e6450`, `0x4820f0`
Confidence: verified
Proof: one
Test: `spell::spell_range_for_caster_difftest`

`0x6e63c7` loads row `+4`, `fild` the caster's `+0x110` field `+0x70` and multiplies by row `+8`;
`fadd st,st(1)` makes `level·perLevel + radius`, stored as a float at `0x6e63f0`. The second sum stays
at 53-bit precision (`0x6e641f`). `0x6e6426` is `fld first; fcomp second; test ah,0x41; jne`: less,
equal or unordered keeps the second, rounded by `fstp`. Both paths call `0x6e6bf0(record, &value, 6)`.
The rows come from the `SpellRadius.dbc` store's id map, bounded by its max row. `0x4820f0` compares
the radius with 20.0 (`test ah,5; jp`) and uses it only when below; out of range by `0x6e6810` it
draws 0. `spell_range_for_caster_difftest` runs `0x6e6350` whole over a record with no modifier, so
the final modifier leaves the value unchanged and the modified case is not run.

## The targeting cursor

While targeting, the world frame's cursor update `0x481790` shows cursor 2 (`Cast`) over a valid
target or an in-range position and 0x16 (`UnableCast`) otherwise, nothing picked included. Over a position, `0x4820f0` stores the point, asks
`0x6e6810` with `dl` 1, and on 1 sets the reticle radius and cursor 2, on 0 radius 0 and cursor
0x16. Over an object, `0x4828d0` asks `0x6e6460`. Outside a position target `0x4820f0` restores
the base cursor through `0x523d30`. Names are the strings the cursor table holds at these
indexes; `AbortCast` returns both the base and the shown cursor to 1 (`Point`).

Addresses: `0x481790`, `0x48185e`, `0x4820f0`, `0x48214b`, `0x4821ac`, `0x4821e1`, `0x4828d0`, `0x482910`, `0x523c20`, `0x523d30`, `0x853b88`
Confidence: verified
Proof: one
Test: none

`0x481790` branches on the pick result: a position to `0x4820f0`, an object to `0x4828d0`, else,
while targeting, 0x16 (`0x48185e`). `0x48214b` calls `0x6e6810` after `0x6e48a0` and `0x6e6320`; `0x4821ac` is `0x523c20` with 2 or
0x16, which stores the shown cursor and refreshes it when it changes. `0x4821e1` is `0x523d30`, a
tail call of `0x523c20` with the base cursor. `0x482910` calls `0x6e6460(&guid, 1)`. The pointer
table `0x853b88` holds `Point` at 1, `Cast` at 2 and `UnableCast` at 0x16 of its 0x29 names.
Entering targeting sets the base to 2 and the shown cursor to 0x16 (`cast`, `0x6e4b60`).

## Placement model and rotation

A spell that places a game object keeps a preview model `0xceca8c`, a rotatable flag `0xceca90`
and a facing `0xcecad4`. `0x6e6970` and `0x6e69d0` return the model and the flag. `0x6e6980`
returns the facing: when not rotatable it reads the caster's facing, stores it and returns it at
full precision; otherwise, or with no caster, the stored value. `0x6e69e0` adds π/2 as a float
and, when the unrounded sum is at least 2π (as a float), stores the rounded sum less 2π. `0x492c20`, on a world input whose `+0x14` is 4 while targeting, rotates a rotatable placement
and otherwise stops targeting.

Addresses: `0xceca8c`, `0xceca90`, `0xcecad4`, `0x6e6970`, `0x6e6980`, `0x6e69b2`, `0x6e69d0`, `0x6e69e0`, `0x6e69ec`, `0x6e69f2`, `0x6e6a0b`, `0x492c20`, `0x4837b0`
Confidence: verified
Proof: one
Test: `spell::spell_reticle_angle_difftest`

`0x6e69b2` calls the caster's slot 6 (`+0x18`); `fst` stores it and returns `st0`. `0x6e69e0` is `fld
facing; fadd 0x3fc90fdb; fst facing` (`0x6e69ec`), then `fcomp 0x40c90fdb; test ah,1; jne`
(`0x6e69f2`): less or unordered keeps it; otherwise `fld facing; fsub 2π; fstp` (`0x6e6a0b`) from the
rounded store. From 0 the presses give π/2, π, 0x4096cbe4, 0x40c90fdb (2π itself), then 0x3fc90fdc.
`0x492c20` calls `0x6e69e0` when `0x6e69d0` is set and `0x6e4900` otherwise. The reticle `0x4820f0`
moves the model to the point with `0x6e6980` and `0x6e69d0`; `0x4837b0` draws it. Writers: the cast
request sets the flag for effect 0x51 and the facing to 0 (`0x6e50c8`, `0x6e511d`, `0x6e50cf`).
`spell_reticle_angle_difftest` runs `0x6e69e0` whole.

## Lua targeting functions

Five Lua functions sit in the `luaL_reg` table `0x86f950`, which `0x6e6ef0` registers and
`0x6e6f10` unregisters. `SpellIsTargeting` returns 1 or nil. `SpellCanTargetUnit("unit")` returns
1 when `0x6e6460` passes for the token's guid with range, else nil; it has no other gate.
`SpellTargetUnit("unit")` returns nothing: silent unless `0x494a50(2)` passes and targeting is on;
a token naming no unit raises reason 0x59 through `0x6e1a00`; otherwise it binds through `0x6e5b10`.
`SpellStopTargeting` stops targeting through `0x6e4900` and returns 1 if it was on, else nil.
`SpellStopCasting` cancels auto-repeat, else aborts an in-flight cast with 0x1c, returning 1, else
nil. A missing string raises the usage error.

Addresses: `0x86f950`, `0x6e6cd0`, `0x6e6d00`, `0x6e6d90`, `0x6e6e12`, `0x6e6e30`, `0x6e6e80`, `0x6e6ec3`, `0x515940`, `0x515970`, `0x6f3510`, `0x6f3810`, `0x6f37f0`, `0x6f4940`
Confidence: verified
Proof: one
Test: none

The table's five pairs are `SpellIsTargeting` `0x6e6cd0`, `SpellCanTargetUnit` `0x6e6d00`,
`SpellTargetUnit` `0x6e6d90`, `SpellStopTargeting` `0x6e6e30`, `SpellStopCasting` `0x6e6e80`;
`0x6e6ef0` walks 0x28 bytes of it. Results are `lua_pushnumber` of 1.0 or `lua_pushnil`, and
`eax` 1. A failed `lua_isstring` on argument 1 calls `luaL_error` with `Usage:
SpellCanTargetUnit("unit")` or `Usage: SpellTargetUnit("unit")`. Tokens go through `0x515970`,
which raises `Unknown unit name: %s` for a token it does not parse, and `0x515940`, which resolves
the guid as a unit. `0x6e6e12` passes `[+0x10]` and 0x59 to `0x6e1a00`. `SpellStopTargeting`,
`SpellTargetUnit` and `SpellStopCasting` return at once when `0x494a50(2)` fails, the first and
last with nil. `SpellStopCasting` calls `0x6ea080` when `0x6e9fd0` is nonzero, else, when
`0x6e3d30` holds, `0x6e4940(0, 1, 0x1c)` (`0x6e6ec3`).

## Unreached functions

`0x6e68f0`, which returns bit 13 (0x2000) of the flag word, is unreached.

Addresses: `0x6e68f0`
Confidence: verified
Proof: one
Test: none

A whole-image scan for `e8`/`e9` rel32 targets and little-endian dwords finds none for
`0x6e68f0`; the instruction before it is `ret` at `0x6e68e9` and padding. The same scan finds the
two calls of `0x6e69d0` (`0x4821bc`, `0x492c2f`) as its control. The dword `0x6e6900` occurs in `.data`
inside the text `\0in\0` of a keyword list, not as a reference.
