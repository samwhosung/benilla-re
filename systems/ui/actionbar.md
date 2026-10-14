# ui: action bars

The action slots, the Lua `Action` functions the stock interface drives them with, and the engine
state behind each button: what a slot holds, which spell it stands for, whether it is usable, current
or in range, and which icon it shows.

The code is the start of `ActionBarFrame.cpp`, `0x4e4f50` to `0x4e6e10`, and 21 Lua functions in a
table at `0x84bd10`, which `0x4e7780` registers and `0x4e77b0` removes. Three tables hold the state:
the 120 slot words, the item counts and the usable flags. The spell tests live in `spell`
(`spell/checks`), the item walk in `objectlayer/inventory`. Placing, picking up and using actions,
the bonus bar, the toggles and the combo points are in `ui/actionuse`; the pet and form bars in
`ui/petbar`, the cursor in `ui/cursor`, macros in `ui/macros`.

Roots: `0x4e7780`, `0x4e77b0`, `0x4e4f50`, `0x4e4f90`, `0x4e4fc0`, `0x4e5b80`, `0x4e5bf0`, `0x4e5c00`, `0x4e5c60`, `0x4e5cc0`, `0x4e5d10`, `0x4e5d60`, `0x4e5db0`, `0x4e5de0`, `0x4e5e20`, `0x4e5e60`, `0x4e58c0`, `0x4e58d0`, `0x4e6c70`

## Action slots

The bar holds 120 slots, one dword each in `0xbc6980`. The top nibble is the kind: 0 a spell (the word
is the spell id), 0x40000000 a macro (the word with bit 30 cleared is the macro id), 0x80000000 an item
(the word with bit 31 cleared is the item entry). Any other nibble stands for nothing. Beside it sit
three tables of 120 dwords: item count or charges `0xbc6390`, usable flag `0xbc6b60`, no-power flag
`0xbc67a0`. `0x4e4f50` zeroes the four and the bonus offset `0xbc6e88`. Every Lua function turns its
slot into an integer with `__ftol` and subtracts 1; its core refuses a slot outside 0 to 119.

Addresses: `0x84bd10`, `0xbc6980`, `0xbc6390`, `0xbc6b60`, `0xbc67a0`, `0xbc6e88`, `0x4e4f50`, `0x4e6dd0`, `0x4e7050`, `0x4e70d0`, `0x4e7073`
Confidence: verified
Proof: one
Test: none

`0x4e4f50` is four `rep stos` of 0x78 dwords at `0x4e4f5d`, `0x4e4f69`, `0x4e4f75`, `0x4e4f81` and a
store to `0xbc6e88`; `0xbc6980 + 0x78·4` is `0xbc6b60`. `HasAction` (`0x4e70d0`) calls `0x4e6dd0`: a
macro slot whose id `0x4e5030` does not find answers 0, every other slot answers whether its word is
nonzero. `GetActionText` (`0x4e7050`) returns the name at `+0x24` of the macro record for a macro word,
else nil. It indexes `0xbc6980` at `0x4e7073` with no bound check, so a Lua slot outside 1 to 120
reads memory past the table; the other functions bound the slot (`0x4e5230`, `0x4e6c70`).

## Writing a slot

`0x4e5d60(slot, word, send, quiet)` stores a word, recounts it first if it is an item, and notifies
(`Slot changed`). `0x4e5db0(slot)` stores 0 and notifies with send set. The arrival of the active
player loads the 120 server words with send 0 and quiet 1, then signals `ACTIONBAR_SLOT_CHANGED`
(213) once with the argument 0.

Addresses: `0x4e5d60`, `0x4e5db0`, `0x4e5d71`, `0x4e5d90`, `0x5dea50`, `0x5deb00`, `0x5deb20`, `0x4e5ea0`
Confidence: verified
Proof: one
Test: none

`0x4e5d60`: `mov [esi*4+0xbc6980],edx` (`0x4e5d71`), the item test, `call 0x4e6d20` (`0x4e5d90`), then
the two stack arguments become the send and quiet flags of `0x4e58e0`. The arrival loop at `0x5deb00`
(inside `0x5dea50`) pushes 0 and 1; the signal at `0x5deb20` is `SignalEvent2(0xd5, "%d", 0)`.

## Replacing slots

`0x4e5e60(old, new)` runs when the server supercedes a spell. Each spell-kind slot whose spell is
`old` is cleared and set to `new`, each with a send, so it costs two packets; a macro slot that
resolved to `old` gets `new` in its record's cached spell (`+0x564`). `0x4e5e20(id)` clears every slot
whose resolved spell (`0x4e5a50`) is `id` and runs when a spell is unlearned. That is the spell slots, the macro slots whose
bound spell is `id` and the item slots whose on-use spell is `id`; each clear sends a word of 0. `0x4e5de0(id)` notifies, without sending, every
player-book slot for the spell, so the aura handlers repaint icons. `0x4e5cc0(id)` notifies, and `0x4e5d10(id)` clears, every slot holding a macro id.

Addresses: `0x4e5a50`, `0x4e5db0`, `0x4e5e60`, `0x4e5e93`, `0x4e5ea0`, `0x4e5ec4`, `0x5e6330`, `0x5e6356`, `0x4e5e20`, `0x5ea253`, `0x4e5de0`, `0x61237e`, `0x612440`, `0x4e5cc0`, `0x4e5d10`, `0x4f1329`, `0x4f1449`
Confidence: verified
Proof: one
Test: none

`0x4e5e60` clears at `0x4e5e93` and sets at `0x4e5ea0` with the pushes 0 and 1; the macro branch stores
`edi` at `0x4e5ec4`. Its one caller is the superceded-spell message handler `0x5e6330` (`0x5e6356`),
which reads the old id and then the new as two u16. `0x4e5e20` matches each of the 120 slots with `0x4e5a50` (`0x4e5e35`, `0x4e5e3a`) and clears a match with `0x4e5db0` (`0x4e5e40`), which stores 0 and sends it through `0x4e58e0`. It has one caller, spell unlearning
(`0x5ea253`); `0x4e5de0` has two, the aura handlers (`0x61237e`, `0x612440`); `0x4e5cc0` and
`0x4e5d10` are called from macro code (`0x4f1329`, `0x4f1449`).

## Slot changed

`0x4e58e0(slot, send, quiet)` recomputes the slot's usable and no-power flags with `0x4e5050` and
stores them. With `send` nonzero it sends `CMSG_SET_ACTION_BUTTON` (0x128): `u8 slot` (0-based) and
`u32 word`. With `quiet` zero it signals `ACTIONBAR_SLOT_CHANGED` (0xd5) with the format `"%d"` and the
1-based slot. A slot outside 0 to 119 does nothing. A drag between two occupied slots sends two
packets, one per slot.

Addresses: `0x4e58e0`, `0x4e5903`, `0x4e5922`, `0x4e5940`, `0x4e5949`, `0x4e5959`, `0x4e5964`, `0x4e5998`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

`0x4e5903 call 0x4e5050`, the stores to `0xbc6b60` and `0xbc67a0`, `0x4e591b je 0x4e5987` skipping the
send, `0x4e5922 push 0x128`, `Put32` (`0x418190`) at `0x4e5940`, `Put8` of the slot (`0x418070`) at
`0x4e5949`, `Put32` of the word at `0x4e5959`, `ClientServices::Send` at `0x4e5964`. The event call
at `0x4e5998` pushes `esi+1`, the format `"%d"` and 0xd5; `0x4e598a` skips it when quiet is set.

## Slot to spell

`0x4e5a50(slot, &book)` returns the spell a slot stands for and writes `book`. A spell slot returns its
word with `book` 0. A macro slot, when `0x4e5030` finds its id and `0x4f0f40` its record, returns the
record's cached spell (`+0x564`: 0, -1 or a spell id) and writes the record's book selector (`+0x568`:
0 the player's book, 1 the pet's). An item slot returns the first on-use spell of the item's template
and writes `book` 0. Anything else returns 0 and leaves `book` unwritten, a macro that is missing
included.

Addresses: `0x4e5a50`, `0x4e5ba0`, `0x4e5ae0`, `0x4e5030`, `0x4f0ec0`, `0x4f0f40`, `0x4e5a71`, `0x4e5abe`, `0x4e5bd8`, `0x55ba30`, `0xbdcc60`
Confidence: verified
Proof: one
Test: none

`0x4e5a6a` branches on a nonzero nibble; `0x4e5a71` writes 0 for a spell and `0x4e5abe` for an item
before `jmp 0x4e5ae0`; `0x4e5a93` tests the macro id and `0x4e5aa2` jumps to `0x4e5ba0`, which stores
`[rec+0x568]` at `0x4e5bd8` and returns `[rec+0x564]`. `0x4e5030` is true when `0x4f0ec0` finds the id
in the 36-entry macro id table `0xbdcc60`. `0x4e5ae0` fetches the template through `0x55ba30` on the
item cache at 0xc0e2a0 and scans its five spell slots: spell id at `+0x11c + 4i` nonzero and trigger
at `+0x130 + 4i` equal to 0 (`0x4e5b50`, `0x4e5b57`); the first match wins. An item with only equip or
chance-on-hit spells has no on-use spell.

## Usable and no power

`0x4e5050(slot, &noPower)` computes a slot's usable flag; `noPower` starts 0. With no active player the
answer is 0. An item word whose count (`0xbc6390`) is 0, or for which `0x6e2fc0` finds a held
cooldown record, answers 0. The slot's spell then decides: a spell, or an item's on-use spell, runs
`0x6e3d60` (`spell/checks#usability-predicate`) with `noPower`; a negative id, an id past the table or
a null record answers 0. A pet-book spell answers 0 unless `0x4bcf70` holds, then compares the cost
`0x6e31b0` with the pet's power (health for type -2): affordable is 1, else `noPower` 1 and 0.
`IsUsableAction` returns the cached pair as 1 or nil, then 1 or nil.

Addresses: `0x4e5050`, `0x4e5080`, `0x4e50ab`, `0x4e50d0`, `0x4e50d7`, `0x4e50ee`, `0x4e518b`, `0x4e51b2`, `0x4e5201`, `0x4e5210`, `0x4e521d`, `0x4e5221`, `0x4e5230`, `0x4e73d0`, `0x6e2fc0`, `0x6e3d60`, `0x6e31b0`, `0x4bcf70`
Confidence: verified
Proof: one
Test: none

`0x4e5080` is the no-player return. `0x4e50ab` and `0x4e50d7` are the item gates; `0x6e2fc0` tests
for a held (disabled-while-active) record only, so a running cooldown or the global cooldown does not
grey an item. `0x4e50ee` branches on the resolved id; `0x4e518b jl` takes the -1 of an unresolved
macro name to 0. `0x4e51b0 je 0x4e521d` sends book 0 to the spell test; `0x4e5201` is the pet cost
compare and `0x4e5210` the only store of 1 to `noPower` outside `0x6e3d60`. `0x4e5230` is the cached
read: `noPower` from `0xbc67a0`, usable from `0xbc6b60`, 0 for a bad slot. `0x4e73d0` pushes 1 or nil
twice.

## Slots with no spell

When the resolver returns 0, `0x4e5050` does not run the spell test. With `0xb4b3e4` (the player controls
the character) 0 the slot is unusable. An attack or ranged-repeat slot answers `not pacified`
(`UNIT_FIELD_FLAGS & 0x20000` clear). An item slot, and a macro slot whose id exists, answer 1, so a
macro without a cast line, or an item with no on-use spell, is never greyed. Any other slot answers
`0x4e55f0`, which is 0 for an empty slot or a macro whose id is gone.

Addresses: `0x4e50ee`, `0x4e50f4`, `0x4e50f9`, `0x4e5108`, `0x4e5113`, `0x4e5135`, `0x4e5149`, `0x4e5154`, `0x4e5170`, `0x4e55f0`, `0xb4b3e4`
Confidence: verified
Proof: one
Test: none

`0x4e50f4 mov eax,[0xb4b3e4]` and `0x4e50f9` test it; `0x4e5108` and `0x4e5113` call `0x4e59b0` and
`0x4e5a00`, each leading to `0x4e5170`, which loads `UNIT_FIELD_FLAGS` (`[unit+0x110]+0xa0`) and
returns `not eax; shr eax,0x11; and eax,1`. `0x4e5135 je 0x4e5164` returns 1 for an item word,
`0x4e5149` tests the macro id and `0x4e514e jne` returns 1. `0x4e5154 call 0x4e55f0` is the last
test.

## Refreshing the flags

`0x4e5c00` recomputes all 120 slots with `0x4e5050`, stores any pair that changed and signals
`ACTIONBAR_UPDATE_USABLE` (215), without arguments, once if any did. `0x4b31c0` runs it and then
signals `SPELL_UPDATE_USABLE` (263). Callers include the active player's unit-flags change when bits
0x01c20000 or the combat bit 0x80000 change, the second call coming before `PLAYER_REGEN_DISABLED`
(190) or `PLAYER_REGEN_ENABLED` (191) is signalled (`objectlayer/units#unit-flags-and-bytes`), the
combo-point byte change `0x5ddff0`, and cooldown and cast changes
(`spell/checks#usability-callers-and-events`). World entry (`0x490979`) runs `0x4e4f90`: the refresh,
then the bonus offset.

Addresses: `0x4e5c00`, `0x4e5c15`, `0x4e5c53`, `0x4b31c0`, `0x4b31c5`, `0x5ff4e0`, `0x5ff50e`, `0x5ff51b`, `0x5ff533`, `0x5ddff0`, `0x4e4f90`, `0x490979`
Confidence: verified
Proof: one
Test: none

`0x4e5c10` to `0x4e5c46` loop `esi` to 0x78, compare `eax` and the out value with `0xbc6b60[esi]` and
`0xbc67a0[esi]` (`0x4e5c1a`, `0x4e5c26`), store and set `edi`; `0x4e5c4e` loads 0xd7 and `0x4e5c53`
signals. `0x4b31c5 call 0x4e5c00`. `0x5ff4e0` tests `esi` (the changed bits) against 0x1c20000
(`0x5ff506`) and 0x80000 (`0x5ff513`), calls `0x4b31c0` (`0x5ff50e`, `0x5ff51b`) and signals 190 or 191
(`0x5ff533`) by the new bit.

## Action bar events

The action bar signals these events. All carry no arguments except `ACTIONBAR_SLOT_CHANGED`, which
carries the 1-based slot, 0 for all.

| id | event | signalled by |
|---|---|---|
| 210, 211 | `ACTIONBAR_SHOWGRID`, `ACTIONBAR_HIDEGRID` | `0x4e58c0`, `0x4e58d0` |
| 212 | `ACTIONBAR_PAGE_CHANGED` | `ChangeActionBarPage` (`0x4e7650`), which changes nothing else |
| 213 | `ACTIONBAR_SLOT_CHANGED` | `0x4e58e0` |
| 214 | `ACTIONBAR_UPDATE_STATE` | `0x4e5bf0`; `0x4b3250` adds 261 `CURRENT_SPELL_CAST_CHANGED` |
| 215 | `ACTIONBAR_UPDATE_USABLE` | `0x4e5c00` |
| 216 | `ACTIONBAR_UPDATE_COOLDOWN` | `0x4e5c60`; `0x4b31b0` adds 262 `SPELL_UPDATE_COOLDOWN` |
| 217 | `UPDATE_BONUS_ACTIONBAR` | `0x4e4fc0` |

Addresses: `0x4e58c0`, `0x4e58d0`, `0x4e7650`, `0x4e5bf0`, `0x4b3250`, `0x4e5c60`, `0x4b31b0`, `0x4e4fc0`, `0x703e50`
Confidence: verified
Proof: one
Test: none

`0x4e58c0`, `0x4e58d0`, `0x4e5bf0`, `0x4e5c60` and `0x4e7650` are a `mov ecx,id` and a jump or call to
`0x703e50`; `0x4e7650` returns 0 values. `0x4b3250` calls `0x4e5bf0` then jumps with 0x105; `0x4b31b0`
calls `0x4e5c60` then jumps with 0x106.

## Item slot counts

`0x4e6d20(entry)` counts an item for a slot: 0 without an active player; otherwise it requests the
template (callback `0x4e5b80`) and, when the template's charges (`+0x144`) are neither 0 nor -1, sums the
absolute spell charges of the entry's items with `0x622180`, else the stack counts with `0x622130`. The
walk uses mask 0, which widens to equipment, bag slots, backpack and keyring, entering bags and
leaving out the bank (`objectlayer/inventory`), so a worn item counts. `0x4e6c70(slot)` is
`GetActionCount`: the cached `0xbc6390` for an item slot, else 0.

Addresses: `0x4e6d20`, `0x4e6c70`, `0x4e6e70`, `0x4e6d81`, `0x4e6d87`, `0x4e6d8c`, `0x4e6d9c`, `0x4e6db6`, `0x622130`, `0x622180`
Confidence: verified
Proof: one
Test: none

`0x4e6d81 mov eax,[eax+0x144]`, the -1 and 0 compares at `0x4e6d87` and `0x4e6d8c`, the two walks at
`0x4e6d9c` and `0x4e6db6` with the vtable call `[edx+0x10]` returning the inventory at
`player+0x1d38` and the stack arguments entry and 0. `0x4e6e70` pushes the number from `0x4e6c70`.

## Item records arriving

The template requests carry the callback `0x4e5b80`. When a record arrives with a nonzero ready flag
it runs `0x4e5c70(entry)`, which recounts the entry and, for each item slot of that entry, stores the
count and notifies with no send and the event on, so every such slot repaints and asks for the icon
again. Container updates and item deactivation call `0x4e5c70` too.

Addresses: `0x4e5b80`, `0x4e5b86`, `0x4e5c70`, `0x4e5c75`, `0x4e5caa`, `0x4e5cb1`, `0x4f8e3c`, `0x4f8e5e`, `0x4f8f5a`, `0x4f9153`, `0x4f9175`, `0x5d85ce`, `0x5d8798`
Confidence: verified
Proof: one
Test: none

`0x4e5b86` tests the ready flag and `0x4e5b8a` calls `0x4e5c70`. `0x4e5c75` calls `0x4e6d20`; `0x4e5caa`
stores the count; `0x4e5cb1` calls `0x4e58e0` with `edx` 0 and the quiet argument 0. The callers of
`0x4e5c70` are the container frame code (`0x4f8e3c`, `0x4f8e5e`, `0x4f8f5a`, `0x4f9153`, `0x4f9175`),
the item record arrival (`0x5d85ce`) and item deactivation (`0x5d8798`).

## Checked state

`IsCurrentAction` pushes 1 or nil from `0x4e53a0`. An item slot is current while the item of the
in-flight item cast has the slot's entry. An attack slot is current while the player's attack target
(`+0xc48`) is set. Any other slot needs a spell with book flag 0 and is current when its spell is the
in-flight spell or the spell awaiting a target; or, for a spell whose first effect is 47, when the
open craft or trade-skill window names it; or, for a spell with a shapeshift aura effect (aura 36),
when that effect's misc value equals the player's form byte.

Addresses: `0x4e53a0`, `0x4e53d4`, `0x4e5467`, `0x4e54c1`, `0x4e54d0`, `0x4e54fe`, `0x4e5522`, `0x4e5541`, `0x4e5560`, `0x4e558b`, `0x6e3d10`, `0x6e48e0`, `0x6e3d20`, `0x6de040`, `0xbdcfb8`, `0xbde040`, `0x4e72f0`
Confidence: verified
Proof: one
Test: none

`0x4e53d4 call 0x6e3d20` returns the in-flight item guid; `0x4e53fc` to `0x4e5428` compare entries.
`0x4e5467 lea ecx,[eax+0xc48]` then `0x47bf60`. `0x4e54b9` requires book 0; `0x4e54c1` and `0x4e54d0`
compare `0x6e3d10` and `0x6e48e0` with the record's id. `0x4e54fe cmp [esi+0xf4],0x2f` opens the
trade-skill arm: the first effect's misc value (`[esi+0x1a8]`), if nonzero, is compared with the open
craft profession `0xbdcfb8` (`0x4e5541`); if zero, the skill line of `0x6de040(race, class, id)` is
compared with `0xbde040` (`0x4e5522`). `0x4e5560` scans the three `+0x16c` aura names for 0x24 and
`0x4e558b` compares the byte at `[unit+0x110]+0x212`. The icon test `0x4e55f0` reads auras instead, so
a form that another spell grants checks a button without changing its icon.

## Repeat and attack tests

`IsAutoRepeatAction` (`0x4e7360`) pushes 1 or nil from `0x4e55b0`: true when the slot's book-0 spell is
the auto-repeat spell (`0x6e9fd0`). `IsAttackAction` (`0x4e7280`) uses `0x4e59b0`: true when the spell's
first effect is 78; the test reads the effect, not the spell id 6603. `0x4e5a00` is the ranged
repeat test the usable and icon code use: `Attributes & 2` and `AttributesEx2 & 0x20`.

Addresses: `0x4e55b0`, `0x4e55d3`, `0x4e7360`, `0x4e7280`, `0x4e59b0`, `0x4e59de`, `0x4e5a00`, `0x6e9fd0`
Confidence: verified
Proof: one
Test: none

`0x4e55d3 call 0x6e9fd0`, then `sub eax,esi; neg eax; sbb eax,eax; inc eax` is the equality test.
`0x4e59de cmp [eax+0xf4],0x4e` ends the attack test. `0x4e5a00` reads the record's `+0x18` and
`+0x20`.

## Range

`IsActionInRange` (`0x4e7550`) pushes nil, 0 or 1 from `0x4e56f0`. Nil: no player, a bad slot, a
pet-book spell, a missing record, or an unlisted target type. 0: `0x6e4440` fails
(`spell/checks#range-pre-check`, no report). 1: the first effect's implicit target is 6, 21, 23, 25, 26,
35, 45, 57 or 61 and a target is selected. `ActionHasRange` (`0x4e75d0`) is true when the spell's `|min|`
or `|max|` range from `0x6e3480` exceeds 2.384e-07.

Addresses: `0x4e56f0`, `0x4e7550`, `0x4e75d0`, `0x4e5810`, `0x4e5880`, `0x4e5765`, `0x4e5770`, `0x4e5780`, `0x4e579c`, `0x6e3480`, `0x6e4440`
Confidence: verified
Proof: one
Test: none

`0x4e5765 call 0x6e4440` with three zero arguments; a 0 result stores 0 and returns 1 (`0x4e5770`).
`0x4e5780` loads the first implicit target `[rec+0x148]`, `0x4e5795` jumps through a 2-case table
at 0x4e57c4, and `0x4e579c` requires `0xb4e2d8` or `0xb4e2dc` nonzero. `0x4e5880` to `0x4e58a3` are two
`fabs` compares with the constant at 0x8029d4.

## Cooldown and flags

`GetActionCooldown` (`0x4e6ed0`) returns three numbers: start and duration times 0.001, then the
enable flag unscaled. An item slot reads `0x6e2ed0` for its entry and any other slot `0x6e2ea0` for
its spell (`spell/records#cooldown-queries`); an empty slot gives zeros. `IsConsumableAction` (`0x4e5250`)
is true for an item of inventory type 24 or 25, or with an on-use spell whose charges are negative.
`IsEquippedAction` (`0x4e5310`) is true for an item whose entry the equipment slots hold. `GetActionAutocast`
(`0x4e6f90`) returns two values, both nil unless the spell comes from the pet book, then bits 31 and 30
of the pet's spell flags as 1 or nil.

Addresses: `0x4e6ed0`, `0x4e6ca0`, `0x4e6f26`, `0x4e6f45`, `0x4e5250`, `0x4e52b9`, `0x4e5310`, `0x4e5380`, `0x4e6f90`, `0x4e6fd1`, `0x4bd160`, `0x6e2ed0`, `0x6e2ea0`, `0x622270`, `0x4e7470`, `0x4e74e0`
Confidence: verified
Proof: one
Test: none

The cooldown binding multiplies by the double 0.001 at `0x4e6f26` and `0x4e6f45` and pushes the third
value without it. `0x4e52b9` loads `[rec+0x2c]` and compares 0x18 and 0x19; the charge test reads
`[rec+0x144 + 4i]` of each on-use spell. `0x4e5380 call 0x622270` pushes the entry and mode 1
(equipment). `0x4e6fd1` passes the spell id to `0x4bd160`. `0x4e7470` and `0x4e74e0` push 1 or nil.

## Action icon

`GetActionTexture` (`0x4e6e10`) pushes the string of `0x4e6a50(slot)` or nil. In order: an empty slot or
a slot outside 0 to 119 gives nil; an attack slot gives `0x4e6870`; a ranged-repeat slot gives `0x4e6990`
when that returns a nonempty path; an item slot gives `Interface\Icons\` plus the display icon of its
template, and `INV_Misc_QuestionMark` while the template is unknown, so it never gives nil
(`objectlayer/items#inventory-icons`); a spell slot gives the SpellIcon path of its `ActiveIconID`
(`+0x1d8`) when `0x4e55f0` holds, else of its `SpellIconID` (`+0x1d4`); an existing macro gives its own
icon from `0x4f0fd0`.

Addresses: `0x4e6e10`, `0x4e6a50`, `0x4e6ab2`, `0x4e6acb`, `0x4e6add`, `0x4e6b07`, `0x4e6b24`, `0x4e6b2d`, `0x4e6b4b`, `0x4e6b6f`, `0x4e6bbd`, `0x4e6bc6`, `0x4e6bce`, `0x4e6c44`, `0x4f0fd0`, `0x5d88b0`, `0x53f810`, `0xbc6d58`, `0xbc6d4c`, `0xbc6e5c`
Confidence: verified
Proof: one
Test: none

`0x4e6a7e` to `0x4e6a9c` caches the icon directory (`0x53f810(3)`) in `0xbc6d4c` once, behind bit 0 of
`0xbc6e5c`. `0x4e6ab2` and `0x4e6acb` call the attack and ranged tests. The item arm reads
`[template+0x18]` (`0x4e6b2d`), maps it with `0x5d88b0` (`0x4e6b4b`) and formats `"%s%s%s"` (prefix,
backslash unless the prefix is empty, leaf) into `0xbc6d58` (`0x4e6b6f`). The spell arm picks `+0x1d8`
at `0x4e6bc6` or `+0x1d4` at `0x4e6bce`. The macro arm calls `0x4f0fd0` (`0x4e6c44`) with a buffer of 0x104.

## Active icon test

`0x4e55f0(slot)` is true when the slot's spell has a nonzero `ActiveIconID` and its own id sits in one
of the 48 aura slots of the unit with bit 0 of that slot's flag nibble set. The unit is the active
player, or the pet when the slot's book flag is nonzero. The aura add and remove handlers repaint
the slots (`0x4e5de0`), so a toggle spell shows its active icon while its aura is on. The spellbook
never shows the active icon.

Addresses: `0x4e55f0`, `0x4e5636`, `0x4e563c`, `0x4e5648`, `0x4e564d`, `0x4e5690`, `0x4e56ad`, `0x4b3f50`
Confidence: verified
Proof: one
Test: none

`0x4e563c cmp [edi+0x1d8],esi` rejects a zero active icon. `0x4e5648` tests the book flag: nonzero
loads the pet guid (`0xb714a0`), zero the active player (`0x468550`). The unit's fields are
`[unit+0x110]`: the 48 aura ids from `+0xa4` (loop at `0x4e5690` to 0x30) and the flag nibbles at
`+0x164 + i/2` (`0x4e56ad`, shifted by `4·(i&1)` and tested with 1). `0x4b3f50`, the spellbook icon,
never reads `+0x1d8`.

## Attack and ranged icons

A spell whose first effect is 78 shows `0x4e6870` instead of its own icon, on the action bar
(`0x4e6ab2`), in the spellbook (`0x4b3f8a`) and on the cursor (`0x494d56`). The helper returns
nil without a player. A nonzero form byte (`[unit+0x110]+0x212`) with a `SpellShapeshiftForm` record
whose icon field (`+0x34`) names a SpellIcon gives that icon. Otherwise a disarmed player
(`UNIT_FIELD_FLAGS & 0x200000`) whose main hand holds a weapon gives `Interface\Buttons\Spell-Reset`.
Otherwise the main-hand item (equipment slot 15) gives its inventory icon, or `Spell-Reset` with no
item. A ranged-repeat spell shows `0x4e6990`: the ranged item (slot 17) icon, or nil when there is none
or its subclass is 16, sending the caller to the spell's own icon.

Addresses: `0x4e6870`, `0x4e689a`, `0x4e68bc`, `0x4e68df`, `0x4e6900`, `0x4e6902`, `0x4e6917`, `0x4e6980`, `0x4e6990`, `0x4e69c9`, `0x4e69f3`, `0x4b3f8a`, `0x4b3fa5`, `0x4b3faa`, `0x494d20`, `0x494d74`, `0x494d56`, `0x494d69`, `0x5d8af0`, `0x5d9f90`, `0xbc6698`, `0xbc6578`
Confidence: verified
Proof: one
Test: none

`0x4e68a1` skips the form leg for form 0; `0x4e68bc` loads the form record's `+0x34`. `0x4e68df` tests
the flag, `0x4e68f3` calls the unit's weapon accessor (vtable `+0x98`) with `(0, 1)` and
`0x4e68fd cmp byte [eax],2` jumps to the `Spell-Reset` return at `0x4e6980`. The weapon leg reads the
8-byte guid at `+0x78` of the equipment array (slot 15) at `0x4e6917`, the ranged leg at `+0x88` (slot
17) at `0x4e69c9`. The cursor setter `0x494d20` uses the same helpers and falls back to the spell's icon
when a string is empty (`0x494d74`), as the action bar does for the ranged one (`0x4e6add`); the
spellbook accepts any nonzero result (`0x4b3faa`). The helpers write `0xbc6698` and `0xbc6578`.
