# spell: checks

The client's own refusals before a cast is sent: possession of totems and reagents, the equipped
item and its ammo, the target's range, lost control, and the crowd-control aura scans.

These functions sit in `Spell_C.cpp`, from `0x6e3d10` to `0x6e4920` and from `0x6e9c80` to
`0x6e9f60`, and own no state. They read the active player's object, the spell's `Spell.dbc` record,
the player-control flag `0xb4b3e4`, the selected target `0xb4e2d8`, and the cast globals that `cast`
and `targeting` write. Two kinds of caller use them. The usability predicate `0x6e3d60` greys
action-bar and form-bar buttons and reports nothing. The cast request `0x6e4b60` (`cast`) runs the
possession, equipment and range checks with their report flag set, so a refusal reaches the failure
reporter `0x6e1a00` (`failures`), which shows a UI error, and no packet is sent. The aura scanners
serve the caster validator `0x6094f0` (objectlayer) and the usability predicate. A unit's `+0x110`
points at its unit field block, a player's `+0xe68` at its player block and an item's `+0x114` at
its item block; the field names are the client's own descriptor name table's, whose indexes match
every offset read here. Record offsets are read from the bytes; their column names are inferred
(Spell record columns).

Roots: `0x6e3d10`, `0x6e3d20`, `0x6e3d30`, `0x6e3d40`, `0x6e3d50`, `0x6e3d60`, `0x6e4440`, `0x6e48a0`, `0x6e48e0`, `0x6e4900`, `0x6e4920`, `0x6e9c80`, `0x6e9e60`, `0x6e9ec0`, `0x6e9f20`, `0x6e9f40`, `0x6e9f60`, and the callback `0x6e43a0`, whose address `0x6e40e0` hands to the inventory walk

## Usability predicate

`0x6e3d60(rec, &noPower)` returns 1 when the active player can use the spell now, else 0, and
reports nothing. It clears `noPower`, returns 0 when there is no active player object, and returns
1 at once when the first effect is 47. Otherwise it applies these gates in order; the first that
fails returns 0:

| # | fails when |
|---|---|
| 1 | `0x605f30` is true and `Attributes & 0x800000` is clear |
| 2 | `0xb4b3e4` is zero and no scan exempts the spell (Lost control exemption) |
| 3 | `0x6e4000(player, rec, 0)` returns 0 |
| 4 | `0x6e40e0(player, rec, ammo 1, report 0)` returns 0 |
| 5 | `AttributesEx & 0x500000` and `PLAYER_FIELD_BYTES` byte 1 is 0 |
| 6 | the form test `0x612480` returns 0 |
| 7 | `Attributes & 0x20000` and `UNIT_FIELD_BYTES_1` byte 3 lacks 2 |
| 8 | `Attributes & 0x10000000` and `UNIT_FIELD_FLAGS & 0x80000` |
| 9 | `CasterAuraState` n is nonzero and bit n−1 of `UNIT_FIELD_AURASTATE` is clear |
| 10 | `TargetAuraState` n is nonzero and: no unit at `0xb4e2d8`, its bit n−1 clear, first implicit target 6 and `0x606980` false, or 21 and `0x6066f0` false |
| 11 | `Attributes & 0x2000000` and `0x6e2fa0(rec, 0)` is nonzero |
| 12 | the cost `0x6e31b0(id, 0)` exceeds the power (signed); sets `noPower` to 1 |

The power is `UNIT_FIELD_HEALTH` for power type −2, else `UNIT_FIELD_POWER1` plus the type.

Addresses: `0x6e3d60`, `0x6e3d73`, `0x6e3d99`, `0x6e3dad`, `0x6e3db6`, `0x6e3dbd`, `0x6e3eb2`, `0x6e3ec6`, `0x6e3ee3`, `0x6e3f01`, `0x6e3f58`, `0x6e3f8a`, `0x6e3fa2`, `0x6e3fb8`, `0x6e3dc3`, `0x6e3e58`, `0x6e3e6d`, `0x6e3e7a`, `0x6e3eaa`, `0x6e3ebf`, `0x6e3edc`, `0x6e3efc`, `0x6e3f1e`, `0x6e3f4e`, `0x6e3f83`, `0x6e3f9b`, `0x6e3fb1`, `0x6e3fba`, `0x6e3fe2`, `0x6e3feb`, `0x605f30`, `0x612480`, `0x606980`, `0x6066f0`, `0x6e2fa0`, `0x6e31b0`, `0xb4e2d8`
Confidence: verified
Proof: one
Test: none

`ecx` is the record (`edi`), `edx` the out pointer, cleared at `0x6e3d73`; the player is
`0x468460(0x10, 0x468550())`, null → `0x6e3ff1`, `xor al,al`. `0x6e3d99 cmp [edi+0xf4],0x2f` → 1.
Gates 3 and 4 push `0`, and `0` then `1`: `0x6e40e0`'s first stack argument is the ammo flag, its
second the report flag. Gate 5 compares the caster's GUID with `0x468550()`, always equal here, and
reads `[+0xe68]+0x1029`. `0x612480` writes a reason into the local `[ebp-0x10]`, which nothing
reads. Gate 7 is `test byte [+0x213],2`, gate 8 `shr eax,0x13` on `+0xa0`. Gates 9 and 10 build
`1 << (n−1)` (`0x6e3f1c`, `0x6e3f67`) against `+0x1dc`; the target is `0x468460(8, [0xb4e2d8])`
and `[edi+0x148]` picks `0x606980` (6) or `0x6066f0` (0x15). Gate 12: `[edi+0x7c] == −2` reads
`+0x40`, else `[+0x44 + 4·type]`, so −1 also reads the health; `0x6e3fe0 cmp eax,esi; jle` → 1,
else `0x6e3feb` stores 1 and returns 0.

## Lost control exemption

While the control flag `0xb4b3e4` is zero, gate 2 of `0x6e3d60` passes only when a scan exempts
the spell: `UNIT_FIELD_FLAGS & 0x800000` with `0x6e9f20`, `& 0x400000` with `0x6e9f40`, or a
nonzero `UNIT_FIELD_CHARMEDBY` with `0x6e9f60` (Crowd-control aura scans). With none, every spell
greys for as long as control is lost. The flag is zero at load. `0x4958e0` stores its argument and
signals `PLAYER_CONTROL_GAINED` (197) for nonzero or `PLAYER_CONTROL_LOST` (196) for zero, both
without arguments, then tail-jumps to the button refresh `0x4b31c0`; two other stores write 1.

Addresses: `0xb4b3e4`, `0x6e3dc3`, `0x6e3df1`, `0x6e3e1a`, `0x6e3e3e`, `0x6e3e47`, `0x4958e0`, `0x4958f7`, `0x49590b`
Confidence: verified
Proof: one
Test: none

`0x6e3dc3 mov eax,[0xb4b3e4]; jne 0x6e3e52` skips the gate. `[ebp-4]` starts 0; bit 23
(`shr ecx,0x17` on `+0xa0`) with `0x6e9f20(player, rec, NULL)` nonzero sets it, bit 22 with
`0x6e9f40` likewise; `+0x10|+0x14` nonzero with `0x6e9f60` nonzero jumps to `0x6e3e52`; then
`0x6e3e47` returns 0 on `[ebp-4] == 0`. `0x4958e0` returns when `ecx` equals the flag, stores it,
and calls `0x703e50` with 0xc5 (`0x4958f7`) or with 0xc4 after cancelling targeting (`0x49590b`);
`tables/event_sites.tsv` lists both sites. Writer census: the address occurs 24 times in the image,
each an instruction operand in `dumps/disasm.txt`; three are stores, `0x4958ea` and two
`mov dword,1`. Control: the same count for `0xceca88` gives 14, its three stores among them.

## Usability callers and events

The bars cache `0x6e3d60`'s answers. `0x4e5050(slot, &noPower)` computes an action slot's pair and
runs `0x6e3d60` for a spell. `0x4e5c00` recomputes all 120 slots into two caches and signals
`ACTIONBAR_UPDATE_USABLE` (215), without arguments, when any slot changed; Lua `IsUsableAction`
reads the caches. `0x4b31c0` runs `0x4e5c00`, retests the reactive spells with `0x6e3d60` and
signals `SPELL_UPDATE_USABLE` (263); it recomputes no form-bar entry (`ui/reactivespells#usable-refresh`).
Lua `GetShapeshiftFormInfo` (`0x4b45c0`) calls `0x6e3d60` on each query, for the castable value of a
form whose active value is unset.

Addresses: `0x4e5050`, `0x4e5221`, `0x4e5c00`, `0x4e5c15`, `0x4e5c53`, `0x4b31c0`, `0x4b31d7`, `0x4b31fd`, `0x4b3246`, `0x4b45c0`, `0x4b479c`
Confidence: verified
Proof: one
Test: none

`0x4e5221 call 0x6e3d60` with the record in `ecx` and the caller's out pointer in `edx`. `0x4e5c00`
loops `esi` to 0x78 (`0x4e5c43`), compares both results with the cached pair, sets a changed flag,
and calls `0x703e50(0xd7)` at `0x4e5c53`. `0x4b31c5 call 0x4e5c00`; the loop `0x4b31d7` to `0x4b323d`
walks the reactive list and `0x4b31fd` calls `0x6e3d60` on each entry's record; no instruction of
`0x4b31c0` reads the form list. `0x4b3246` signals 0x107. `0x4b479c call 0x6e3d60` returns 1.0 when
nonzero. `tables/event_sites.tsv` lists `0x4e5c53` and `0x4b3246`.

## Totems and reagents

`0x6e4000(caster, rec, report)` returns 1 when the caster is not the active player. Otherwise it
tests the two totems, then the eight reagents, and returns 0 at the first miss: a nonzero totem
(`+0xa0`, `+0xa4`) with no carried item of that entry is reason 0x78 `SPELL_FAILED_TOTEMS`; a
nonzero reagent (`+0xa8` on) whose carried count is below its count (`+0xc8` on, signed) is 0x5c
`SPELL_FAILED_REAGENTS`. With `report` set it first calls `0x6e1a00(id, reason, entry, −1, 0)`.
Carried means found by the walk `0x622420` with mask 0: slots 0 to 0x26 and 0x51 to 0x70 and bag
contents, never the bank, skipping an item with `ITEM_FIELD_FLAGS & 0x10`, or with durability 0
of a nonzero maximum unless its flags have 8.

Addresses: `0x6e4000`, `0x6e400e`, `0x6e4032`, `0x6e405f`, `0x6e4064`, `0x6e4097`, `0x6e40c1`, `0x5fa6d0`, `0x622270`, `0x622130`, `0x622420`, `0x5dd2e0`
Confidence: verified
Proof: one
Test: none

`0x6e400e call 0x5fa6d0`, which compares the object's GUID with `0x468550()`; false → `0x6e40d1`,
1. Totems: `0x622270(entry, 0)` on `caster+0x1d38`, zero → `0x6e407d`; reagents: `0x622130(entry,
0)`, `cmp eax,[esi+0x20]; jl` (`0x6e4064`). `0x622270` stops the walk at the first item whose
`[+8]+0xc` (`OBJECT_FIELD_ENTRY`) matches; `0x622130`'s callback adds `ITEM_FIELD_STACK_COUNT`
(`+0x20`) per match and never stops. `0x622420` widens a zero mask to 0x47 when the inventory's
byte `+0x10` is set, and the player setup `0x5dd2e0` sets it (`0x5dd44d`) with 0x71 slots for the
active player; 0x47 admits 0 to 0x12, 0x13 to 0x16, 0x17 to 0x26 and 0x51 to 0x70. Bags descend
through slot 4 of the item unless mask bit 0x10; the skips are `0x6224f7` to `0x622513`, after
a third that skips an item for which 0x614ea0 is nonzero (`0x6224e1`). The
reports load `dl` 0x78 (`0x6e4095`) and 0x5c (`0x6e40bf`) and push the entry, −1 and 0.

## Equipped item and ammo

`0x6e40e0(caster, rec, ammo, report)` returns 1 for a caster not the active player,
`Targets & 0x10`, `EquippedItemClass < 0` or a zero subclass mask. The slot mask is
0x8000 for `AttributesEx3 & 0x400`, else 0x10000 for `& 0x1000000`, else all. No item from
`0x5f0c50` is reason 0x1a for 0x400, else 0x1b for 0x1000000, else 0x19; with both bits a second
search of slot 16 alone fails as 0x1b. Then, with `ammo` set, 1 is returned for an
`ItemSubClass.dbc` match with flags 0x10, and for an item without a stats record or with ammo type
0; else a carried projectile (class 6) of that subclass whose entry is `PLAYER_AMMO_ID` must be
found through `0x6e43a0`, or 0x2f `SPELL_FAILED_NEED_AMMO`.

Addresses: `0x6e40e0`, `0x6e4110`, `0x6e4120`, `0x6e4130`, `0x6e4136`, `0x6e4153`, `0x6e415b`, `0x6e417d`, `0x6e41b0`, `0x6e41c0`, `0x6e41e2`, `0x6e41f6`, `0x6e4264`, `0x6e429c`, `0x6e42d5`, `0x6e4331`, `0x6e434e`, `0x6e4373`, `0x6e43a0`, `0x5f0c50`, `0x5d9f30`, `0x5d9f90`
Confidence: verified
Proof: one
Test: none

`0x6e4153`: `and 0x1000000; neg; sbb; and 0x10001; dec`. Reports push (class, subclass mask, 0) or
(ammo type, −1, 0). The reason is `shr edx,0x17; and dl,2;
or dl,0x19` unless `0x400` (`0x6e4176 mov dl,0x1a`). `0x5f0c50` scans slots 0 to 0x16 in the mask,
skipping broken items, matching `0x5d9f30(item) == rec+0xe8` and bit `0x5d9f90(item)` of
`rec+0xec`; with `UNIT_FIELD_FLAGS & 0x200000` it drops slot 15, or else 16, when the unit's slot
38 (`+0x98`) returns a record whose first byte is 2 for that hand.
`0x6e41f6` tests the ammo flag. The `ItemSubClass.dbc` rows (the store at 0xc0db90, 0x74 bytes) compare class
and subclass; `0x6e4264 test byte [esi+0x10],0x10`. The ammo type is the item stats record's
`+0x114`; `PLAYER_AMMO_ID` is `[+0xe68]+0x102c`, zero → `0x6e4362`, 0x2f. `0x6e4331` walks with
the context {type, ammo id, 0, 0}. `0x6e43a0` accepts class 6 of subclass `[ctx]` whose entry is
`[ctx+4]`; its zero-id and bag branches need context values this caller never passes.

## Range pre-check

`0x6e4440(caster, rec, guid, report)` returns 0 only when a relation admits the target and the
range compare fails. The target is `guid`'s object, `0xb4e2d8`'s for 0; none returns 1. A mask
starts from `Targets`'s low word; each effect's first implicit target edits it: 1 clears 0x400, 5
clears 0x8000, 6 and 53 set 0x80, 21 and 45 set 0x100, 23 sets 0x800, 25 and 63 set 2, 26 sets
0x4000, 35 sets 8, 57 and 61 set 4. Mask 0x4800 admits a game object; a corpse without
`CORPSE_FIELD_FLAGS & 1` is admitted by 0x8000 when `0x6067d0` holds, by 0x200 when it fails.
Units: next entry. The report is 0x59 `SPELL_FAILED_OUT_OF_RANGE` beyond the maximum, else 0x76
`SPELL_FAILED_TOO_CLOSE`, with (−1, −1, 0).

Addresses: `0x6e4440`, `0x6e445d`, `0x6e4488`, `0x6e44a6`, `0x6e4748`, `0x6e4770`, `0x6e44f9`, `0x6e469a`, `0x6e46b2`, `0x6e46d6`, `0x6e4710`, `0x6e4731`, `0x6e4735`, `0x6067d0`, `0xb4e2d8`
Confidence: verified
Proof: one
Test: none

`0x6e445d`: a zero GUID loads `0xb4e2d8`/`0xb4e2dc`; `0x468460(1, ...)`. `0x6e4488 mov bx,[edi+0x34]`;
the loop runs `edx` 3 times over `+0x148`, `+0x14c`, `+0x150`: `dec eax; cmp eax,0x3e; ja`, then
`movzx eax,[eax+0x6e4770]` and `jmp [eax*4+0x6e4748]`. The tables' raw bytes: ten cases, the
tenth the loop's continue; the index bytes map the values above, every other value to case 9.
The type is `[[esi+8]+8]`: bit 3 unit, else bit 5 (`0x6e469a`, `test bh,0x48`), else bit 7
(`0x6e46ab`), else 1. `0x6e4710 call 0x6e47b0(caster, target, id, &far)`; `0x6e4731 mov dl,0x59`,
`jne`, else `0x6e4735 mov dl,0x76`. Lua `IsActionInRange` reaches it through `0x4e56f0` with GUID
0 and no report, answering 0 for a 0 and 1 for a pass when the first implicit target is 6, 21, 23,
25, 26, 35, 45, 57 or 61 and a target is selected.

## Range pre-check for units

For a unit, `0x6e4440` returns 1 without a compare when `UNIT_FIELD_FLAGS & 0x10000`; when
`TargetCreatureType` is nonzero and `0x605570`'s type is 0 or outside it; when the unit is alive
and the mask has 0x400; and when it is dead and none of `UNIT_DYNAMIC_FLAGS & 0x20`, mask 0x8600 or
`AttributesEx2 & 1` holds. Otherwise the first relation that holds runs the compare: mask 8 with
`0x606c20` and `0x6066f0`; 4 with `0x606d20` and `0x6066f0`; 0x100 with `0x6066f0`; 0x80 with
`0x606980`; 2 with `UNIT_FIELD_FLAGS & 0x100` clear; 0x8000 on a dead player or pet with
`0x6066f0`, which returns 1 when it fails; 0x200 on a dead player without `0x6066f0`. Else 1.

Addresses: `0x6e4511`, `0x6e4524`, `0x6e4556`, `0x6e4583`, `0x6e458c`, `0x6e45b2`, `0x6e45d8`, `0x6e45f1`, `0x6e4609`, `0x6e4626`, `0x6e466b`, `0x605570`, `0x606c20`, `0x606d20`, `0x6066f0`, `0x606980`
Confidence: verified
Proof: one
Test: none

`0x6e4517 test byte [eax+0xa2],1` → 1. `0x6e452d call 0x605570`, then `1 << (type−1)` against
`[edi+0x38]`. `0x6e4556`: health `+0x40` `jg` → `test bh,4`; otherwise `+0x224 >> 5 & 1`,
`test bh,0x86`, `test byte [edi+0x20],1`, all zero → 1. Each relation `jne 0x6e4701` (or
`0x6e4704`) into the compare. `0x6e4626`: 0x8000, health ≤ 0, `[[esi+8]+8] >> 4 & 1` or
`UNIT_FIELD_PETNUMBER` (`+0x214`) nonzero, then `0x6066f0` true → compare. `0x6e466b`: 0x200,
health ≤ 0, a player, `0x6066f0` false → compare. The meanings of `0x605570`, `0x606c20` and
`0x606d20` (creature type, party, raid) are inferred.

## Range compare

`0x6e47b0(caster, target, id, &far)` clears `far`, gets `(min, max)` from `0x6e3480`, and computes
`d = caster position − target position` per axis and `s = (dx·dx + dz·dz) + dy·dy` at 53-bit
precision from the float positions. `s < min·min` returns 0 with `far` 0; `s > max·max` sets `far`
to 1 and returns 0; else 1. Equality passes both bounds. A NaN in either compare passes it, so a
NaN distance or bound is in range. An implementation in `f64` reproduces it exactly.

Addresses: `0x6e47b0`, `0x6e47c4`, `0x6e47ca`, `0x6e47d5`, `0x6e47e2`, `0x6e47ef`, `0x6e47f2`, `0x6e4814`, `0x6e4822`, `0x6e4825`, `0x6e4840`, `0x6e4849`, `0x6e3480`
Confidence: verified
Proof: one
Test: `spell::spell_range_check_difftest`

`0x6e47d5 call 0x6e3480` with `edx` the id and the stack `&[ebp+0xc]`, `&[ebp+8]`, target; the first
compare uses `[ebp+0xc]`, and both callers map `far` 0 to 0x76, so it is the minimum. The positions
are the three floats slot 5 (`call [edx+0x14]`) of each object writes to a local vector; the name is
inferred from this use. `d8 26` and `d8 66 xx` are `fsub m32`, `st0 = caster − target`. `de cb` `fmulp
st(3)` squares x, `d8 c9` and `de c3` add `dz²`, `d8 ca` and `de c3` add `dy²`. `fcompp` of `s`
against `min²`, `test ah,5`, `jp` continues on C0 and C2 equal: clear (s ≥ min²) or both set
(unordered). The second, `test ah,0x41; jne`, returns 1 on less, equal or unordered. Both products of
two floats are exact in 53 bits. `spell_range_check_difftest` runs `0x6e47b0` whole, with `0x6e3480`
replaced by a patch that writes the chosen window and both position calls stubbed.

## Crowd-control aura scans

`0x6e9ca0(unit, castRec, auraType, &mechanic)` reads the 48 `UNIT_FIELD_AURA` slots and, for each
effect of each aura record whose aura type is `auraType`, asks `0x6e9d70(castRec, auraRec, effect)`.
The first refusal stores the effect's mechanic (`+0x13c`), else the aura's (`+0x14`) when nonzero,
and returns 0. It returns 1 when such an effect exists and every one is exempt, else 0. The
wrappers clear `mechanic` and try aura types in order, returning 1 on the first 1: `0x6e9c80` 12;
`0x6e9e60` 27, 12, 60; `0x6e9ec0` 25, 12, 60; `0x6e9f20` 7; `0x6e9f40` 5; `0x6e9f60` 6, 177, 2.

Addresses: `0x6e9ca0`, `0x6e9cc9`, `0x6e9cf7`, `0x6e9d06`, `0x6e9d41`, `0x6e9d58`, `0x6e9d2f`, `0x6e9c80`, `0x6e9e60`, `0x6e9ec0`, `0x6e9f20`, `0x6e9f40`, `0x6e9f60`
Confidence: verified
Proof: one
Test: none

`ecx` from `0xa4` to `0x164` step 4 over `[unit+0x110]`; ids of 0, negative or above the store's maximum
and null records are skipped. `[edi+0x16c+4j]` against `[ebp+8]` sets `[ebp-1]` before the call;
`je 0x6e9d3a` on 0 writes `[edi+0x13c+4j]` or `[edi+0x14]` through a non-null pointer and returns
`xor al,al`, `ret 8`. The wrappers pass `ecx` and `edx` through and push `&mechanic` and the type
(`push 0xc` at `0x6e9c91`, `push 0xb1` at `0x6e9f8a`). The aura type names (stun, silence, pacify,
fear, confuse, charm, possess) are inferred.

## Crowd-control use in the validator

The caster validator `0x6094f0` (objectlayer) runs six scans after its power and in-progress tests,
skipping all of them for a caster with health 0 or below. A failed scan refuses with 0x8d
`SPELL_FAILED_PREVENTED_BY_MECHANIC` and the mechanic as the first argument when one was stored,
else with the arm's reason; both report. The arms, in order: `UNIT_FIELD_CHARMEDBY` set and not
the active player, `0x6e9f60`, 0x14 `CHARMED`; `UNIT_FIELD_FLAGS & 0x40000`, `0x6e9c80`, 0x64
`STUNNED`; `PreventionType` 1 and `& 0x2000`, `0x6e9e60`, 0x60 `SILENCED`; `PreventionType` 2 and
`& 0x20000`, `0x6e9ec0`, 0x5a `PACIFIED`; `& 0x800000`, `0x6e9f20`, 0x1e `FLEEING`; `& 0x400000`,
`0x6e9f40`, 0x16 `CONFUSED`.

Addresses: `0x6094f0`, `0x60980d`, `0x609972`, `0x6099d0`, `0x609a36`, `0x609a9d`, `0x609afd`, `0x609b5d`
Confidence: verified
Proof: one
Test: none

`0x60980d jle 0x609ba1` on `+0x40`. Each arm: the flag test, `lea edx,[ebp-8]; push`, the scan,
`jne` past; else `push 0; push −1`, then `push eax; mov dl,0x8d` when `[ebp-8]` is nonzero, or
`mov dl,<reason>; push −1`, and `call 0x6e1a00`. `PreventionType` is `[esi+0x294]`. The charm arm
compares `+0x10`/`+0x14` with `0x468550()` (`0x60995e`). The cast request reaches `0x6094f0` at
`0x6e4f3b`, after the range pre-check.

## Immunity matcher

`0x6e9d70(castRec, auraRec, j)` returns 1 when the spell being cast grants an immunity that covers
effect `j` of the aura. It returns 0 when `castRec` is null, lacks `AttributesEx & 0x8000`, or
`auraRec` has `Attributes & 0x20000000`. Otherwise, for each of the cast's three effects that is
6 (apply aura), with `m` its misc value, the effect's aura type decides: 38 matches when `m` equals
the aura's aura type for effect `j`; 39 when bit `School & 31` of `m` is set and the aura lacks
`AttributesEx2 & 0x4000000`; 41 when `m` equals the aura's `Dispel`; 77 when `m` equals its
`Mechanic` or effect `j`'s mechanic. Any match returns 1; other types never match.

Addresses: `0x6e9d70`, `0x6e9d78`, `0x6e9d7e`, `0x6e9d81`, `0x6e9d83`, `0x6e9d89`, `0x6e9da0`, `0x6e9dac`, `0x6e9dbb`, `0x6e9e18`, `0x6e9e2c`, `0x6e9dc2`, `0x6e9dcd`, `0x6e9ded`, `0x6e9df4`, `0x6e9e02`
Confidence: verified
Proof: one
Test: none

`0x6e9d81 test ah,ah; jns` is bit 15 of `[ecx+0x1c]`; `0x6e9d89 test [edx+0x18],0x20000000`.
`eax` walks `castRec+0x1a8` (misc values); `[eax−0xb4]` is the effect, `[eax−0x3c]` the aura type.
`0x6e9dac add ecx,−0x26; cmp 0x27; ja`, `movzx [ecx+0x6e9e2c]`, `jmp [ecx*4+0x6e9e18]`: the index
bytes give case 0 to 38, 1 to 39, 2 to 41, 3 to 77, 4 (next effect) elsewhere. Case 0 compares
with `[edx+edi*4+0x16c]`, case 1 tests `[edx+0x20] & 0x4000000` and `shl ebx,cl` by `[edx+4]`,
case 2 `[edx+0x10]`, case 3 `[edx+0x14]` then `[edx+edi*4+0x13c]`.

## How the refusals surface

Every reported refusal here goes to `0x6e1a00(id, reason, a1, a2, a3)` (`failures`), which shows a
UI error from the message catalogue and, when no cast is in flight, signals `SPELLCAST_FAILED`
(339). The messages these reasons select:

| reason | message |
|---|---|
| 0x78, 0x5c | 209 `ERR_SPELL_FAILED_TOTEMS`, 210 `ERR_SPELL_FAILED_REAGENTS`, naming the item `a1` |
| 0x19 to 0x1b | 213 `ERR_SPELL_FAILED_EQUIPPED_ITEM_CLASS_S` |
| 0x2f | 307 `ERR_NOAMMO_S` |
| 0x59 | 305 `ERR_SPELL_OUT_OF_RANGE` |
| 0x8d | 44 `ERR_SPELL_FAILED_S`, the reason's text naming the mechanic `a1` |
| 0x76 and the arm reasons | 44 `ERR_SPELL_FAILED_S` with the reason's text |

The cast request `0x6e4b60` runs `0x6e4000` (`0x6e4ded`), `0x6e40e0` with ammo and report
(`0x6e4e03`), then `0x6e4440` with its target GUID (`0x6e4f26`) and `0x6094f0` (`0x6e4f3b`); a
0 from any of them returns before the send.

Addresses: `0x6e1a00`, `0x6e1b6c`, `0x6e1b7a`, `0x6e1b96`, `0x6e1bc2`, `0x6e1bd3`, `0x6e21dd`, `0x6e223c`, `0x6e23e0`, `0x6e4b60`, `0x6e4ded`, `0x6e4e03`, `0x6e4f26`, `0x6e4f3b`
Confidence: verified
Proof: one
Test: none

`0x6e1a45` defaults the message to 0x2c; the compares at `0x6e1b6c` (0x78 → 0xd1), `0x6e1b7a`
(0x5c → 0xd2), `0x6e1b96` (0x19 to 0x1b → 0xd5), `0x6e1bc2` (0x59 → 0x131) and `0x6e1bd3` (0x2f →
0x133) set it; `tables/messages.tsv` names the ids. The text is the localized string of the key
`0x6e23e0` returns for the reason; 0x8d formats it with the `SpellMechanic` name of `a1`
(`0x6e2190`). `0x6e21dd call 0x496720` shows it. When the spell is not
`0xceca88` and no cast is in flight, `0x6e223c` signals 0x153 for reasons other than 0x23, 0x24 and
0x61. Each call site in `0x6e4b60` is followed by `test al,al; je 0x6e506e`.

## Cast-state accessors

Five accessors read the cast globals that `cast` writes. `0x6e3d10` returns the in-flight spell id
`0xceca88`; `0x6e3d30` returns whether it is nonzero. `0x6e3d20` returns the address of
`0xcecab0`, the GUID of the item the in-flight cast came from, 0 for none. `0x6e3d40` returns the
address of the 256-byte pending-cast block `0xceac48`, and `0x6e3d50` the address of its `+0x18`.

Addresses: `0x6e3d10`, `0x6e3d20`, `0x6e3d30`, `0x6e3d40`, `0x6e3d50`, `0xceca88`, `0xcecab0`, `0xceac48`, `0x6e4ad0`
Confidence: verified
Proof: one
Test: none

`0x6e3d10 mov eax,[0xceca88]; ret`; `0x6e3d30 xor eax,eax; test ecx,ecx; setne al`; `0x6e3d20`,
`0x6e3d40` and `0x6e3d50` are `mov eax,imm32; ret`. `0x6e4ad0(id, item)`, for a nonzero `id` other
than the current one, stores it at `0xceca88` and the item's GUID `[[item+8]]` at `0xcecab0` (0 for
a null item); the cast request
passes its item argument (`0x6e5024 mov edx,esi`). The cast request clears `0xceac48` as 0x40
dwords (`0x6e4f80 rep stos`).

## Targeting-state predicates

The targeting word `0xcecac0` (`targeting`) is nonzero while a spell awaits a target. `0x6e48a0`
returns whether it is. `0x6e48e0` returns the pending spell id (block `+0x10`) while it is, else 0.
`0x6e48b0` returns 1 while targeting for a caster other than the active player (block `+0x08`).
`0x6e4900` and `0x6e4920` cancel targeting through `0x6e4940(cl 1, dl 1, reason 0x1c)` when
`0x6e48a0`, or `0x6e48b0`, holds; the cancel, for the active player's own pending cast, signals
`SPELLCAST_FAILED` (339).

Addresses: `0x6e48a0`, `0x6e48b0`, `0x6e48e0`, `0x6e4900`, `0x6e4907`, `0x6e4920`, `0xcecac0`, `0x6e4940`, `0x6e4a50`
Confidence: verified
Proof: one
Test: none

`0x6e48a2 cmp word [0xcecac0],ax; setne al`. `0x6e48e0`: `neg ax; sbb eax,eax; and eax,ecx`, `ecx`
the block's `+0x10`.
`0x6e48b0` compares block `+0x08`/`+0x0c` with `0x468550()`; the cast request stores the caster's
GUID there (`0x6e4fb1`, `edi` the caster from `[ebp-8]`). `0x6e4907 je` skips the cancel;
`mov dl,1; push 0x1c; mov cl,dl`. In `0x6e4940`, a caster GUID equal to the active player's
signals `0x152 + 1 + (reason is 0x23 or 0x24)` for `cl` nonzero (`0x6e4a50`).

## Chain fill predicate

`0x6e4870(rec, flags)` returns 1 for `AttributesEx2 & 0x40000000`; else 0 for
`AttributesEx & 0x400` or `AttributesEx2 & 0x10000000`; else 1 when `flags & 1` is clear or
`flags & 0x1000` is set; else 0. Its one caller, the handler `0x6e7a70` (`handlers`), passes the
first value it reads from the packet, into a zeroed dword, as `flags`; the result gates its chain-target fill at
`0x6e800d` and is an argument of the caster call before it.

Addresses: `0x6e4870`, `0x6e4878`, `0x6e4881`, `0x6e4888`, `0x6e488d`, `0x6e4892`, `0x6e7a70`, `0x6e7fa1`, `0x6e8000`, `0x6e800d`
Confidence: verified
Proof: one
Test: none

`[ecx+0x20] & 0x40000000` → `0x6e4897` (1); `[ecx+0x1c] & 0x400` → `0x6e4894` (0); `& 0x10000000`
→ 0; `test dl,1; je` → 1; `test dh,0x10; jne` → 1; else 0. `0x6e7a9d` zeroes `[ebp-0x10]` and
`0x6e7aa4` reads into it; `0x6e7f9c mov edx,[ebp-0x10]`, `0x6e7fa1 call`, `0x6e7fb7` stores `al` in
`[ebp-0xc]`, pushed for the call at `0x6e7fc5`; `0x6e8000 test al,al; je 0x6e8012` skips
`0x6e800d`. What the packet's dword holds is the handler's to say.

## Spell record columns

The checks read these `Spell.dbc` record offsets; the names are the conventional 1.12 layout's,
column = offset / 4. `+0x14` Mechanic, `+0x18` Attributes, `+0x1c` AttributesEx, `+0x20`
AttributesEx2, `+0x24` AttributesEx3, `+0x34` Targets, `+0x38` TargetCreatureType, `+0x40`
CasterAuraState, `+0x44` TargetAuraState, `+0x7c` PowerType, `+0xa0` Totem, `+0xa8` Reagent,
`+0xc8` ReagentCount, `+0xe8` EquippedItemClass, `+0xec` EquippedItemSubClassMask, `+0xf4` Effect,
`+0x13c` EffectMechanic, `+0x148` EffectImplicitTargetA, `+0x16c` EffectApplyAuraName, `+0x1a8`
EffectMiscValue, `+0x294` PreventionType; `+0x4` School, `+0x10` Dispel.

Addresses: `0x6e3d60`, `0x6e4000`, `0x6e40e0`, `0x6e4440`, `0x6e9ca0`, `0x6e9d70`
Confidence: inferred
Proof: none
Test: none

The offsets are the bytes' (each entry above); the client holds no column names. The reason codes
chosen at the totem, reagent and equipped-item tests agree with these names.
