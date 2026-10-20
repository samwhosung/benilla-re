# spell: records

The values the client computes from a `Spell.dbc` record: level, cost, cast time, range, duration, effect amounts, and the talent modifiers on them.

Each getter takes a spell id or a `SpellRec` (`Spell_C.cpp`), reads the record from the `Spell.dbc`
store and its side tables (`SpellCastTimes`, `SpellDuration`, `SpellRange`), and scales by a level
the caster's vtable gives. The talent modifiers live in two `int32_t[64][29]` tables, flat and
percent, indexed by a bit of the record's 64-bit family mask and an operation code; the server
fills them with `SMSG_SET_FLAT_SPELL_MODIFIER` (`0x266`) and `SMSG_SET_PCT_SPELL_MODIFIER`
(`0x267`), and they apply only to records of the player's class family. The effect amounts run
on a software single-precision arithmetic (`cmath`) that truncates where IEEE rounds. The
tooltips, the cast checks, the action bars, the trainer and the lock checks reach these getters;
the cast send reaches the global-cooldown start, and the item and spell cooldown queries sit
over the spell histories (`cooldowns`).

Roots: `0x6e2de0`, `0x6e2ea0`, `0x6e2ed0`, `0x6e2f70`, `0x6e3130`, `0x6e31b0`, `0x6e3300`, `0x6e3340`, `0x6e3480`, `0x6e3760`, `0x6e3800`, `0x6e6af0`, `0x6e6bf0`, `0x6e6ca0`, `0x6e9950`, `0x6ea000`

## Record lookups

Every getter finds a record the same way: an id below 0 or above the store's `+0x0c` bound, or a
null pointer in the store's `+0x08` table at that id, is no record. The stores are `Spell.dbc`
`0xc0d780`, `SpellRange.dbc` `0xc0d794`, `SpellDuration.dbc` `0xc0d820`, `SpellCastTimes.dbc`
`0xc0d870` and `ChrClasses.dbc` `0xc0deec`.

Addresses: `0xc0d780`, `0xc0d794`, `0xc0d820`, `0xc0d870`, `0xc0deec`
Confidence: verified
Proof: one
Test: none

The pattern is `test; jl`, `cmp reg,[store+0xc]; jg`, `mov [store+8]`, `mov [eax+id*4]`, `test; je`
(`0x6e31bf` to `0x6e31d3`). The first four stores are rows of `tables/dbc_stores.tsv`.
`0xc0deec` is not: its loader `0x542360`, called in the same load sequence, takes its path from
a getter returning `DBFilesClient\ChrClasses.dbc` instead of a pushed string.

## Caster level

`0x6e3130(spellId, pet)` returns the level a formula scales by: with `pet` zero, the active player's
unit; otherwise the unit whose guid sits in the player's unit fields at `+0x00`, or at `+0x08`
when that is zero. It calls the unit's vtable slot 42 with the id and divides the result by 5,
unsigned and truncating; no unit gives 0. A creature's slot, `0x60cd80`, returns its level at unit
field `+0x70` times 5, lowered to the record's `+0x6c` times 5 when that is positive, and 0 for a
negative result or no record. The player's slot, `0x5ea690`, returns a skill value instead, with
the same cap.

Addresses: `0x6e3130`, `0x6e318d`, `0x6e3195`, `0x60cd80`, `0x5ea690`, `0x5fad50`, `0x5dd2e0`, `0x468460`, `0x468550`
Confidence: verified
Proof: one
Test: none

`0x6e3130` looks the unit up through `ClntObjMgrObjectPtr` (`0x468460`) with type mask 8, which
tests the mask against the object's type field, `[[obj+8]+8]`. `0x6e318d` is `call [edx+0xa8]`;
`0x6e3195` is `mul 0xcccccccd`, `shr edx,2`. The object factory builds type 3 with `0x5fad50` and
type 4 with `0x5dd2e0`; slot 42 of the vtables they store, at `0x5fb12b` and `0x5dd473`, is
`0x60cd80` and `0x5ea690`. `0x5ea690` passes the race and class bytes (`+0x78`, `+0x79`) and the id `0x5ea5d0`
returns to `0x6de040`, and with a result returns the skill value of its `+0x04`, capped as above
and floored at 0; no result gives 0. That the value is the skill of the spell's line is inferred
from the callees, which belong to `tables` and `objectlayer`.

## Talent modifier tables

`HandleSetSpellModifierPacket` (`0x6e9950`) reads a byte `b`, a byte `op` and an `int32_t v`, and
stores `v` at index `b·29 + op` of the percent table `0xcecb30` for opcode `0x267`, of the flat
table `0xcead60` for any other, replacing what was there; then it calls `0x4d7d40` and returns 1.
Neither byte is checked: an index past 1855 writes beyond the table, up to `255·29 + 255`. A short
packet leaves the unread fields as they were on the stack. `0x6e6ca0(class)` copies the
`ChrClasses` record's `+0x3c` to the class family `0xcecaac` and leaves it unchanged for no record.

Addresses: `0x6e9950`, `0x6e9989`, `0x6e9998`, `0x6e99b4`, `0xcead60`, `0xcecb30`, `0xcecaac`, `0x6e6ca0`, `0x6e7150`, `0x418cb0`, `0x418e30`, `0x4d7d40`
Confidence: verified
Proof: one
Test: none

`0x6e7150` registers `0x6e9950` for both opcodes (`tables/opcodes.tsv`), zeroes `0x740` dwords
from each table and stores 0 to `0xcecaac`. The reads are `CDataStore::GetUInt8` twice and
`GetInt32`, which on underflow write nothing. `0x6e9989` is `cmp edi,0x267; jne`;
`0x6e9993` and `0x6e99af` are `imul eax,eax,0x1d; add eax,ecx`. The first byte lands in the high
byte of the time argument's slot (`lea eax,[ebp+0xf]`). A scan of the image for each of `0xcecaac`, `0xcead60` and
`0xcecb30` as a little-endian dword finds three sites: its writer (`0x6e6ca0` or `0x6e9950`),
`0x6e7150` and the reader `0x6e6b30`; control: the same scan finds `0x6e9950` at `0x6e7246` and
`0x6e7256`.

## Modifier sums

`0x6e6b30(spell, op, &flat, &pct)` returns 0 with `flat = 0` and `pct = 100` when the record's
`+0x280` is 0 or differs from `0xcecaac`, or its `+0x24` has bit 29 set. Otherwise it sums, for
each bit `i` of the 64-bit mask at `+0x284` (low dword first), cell `i·29 + op` of the flat table
into `flat` and of the percent table into `pct`, in 32-bit wrapping adds. Both sums zero returns 0
with `pct` left 0. Else `pct = pct + 100`, set to 0 when negative, and it returns 1. `op` is not
checked.

Addresses: `0x6e6b30`, `0x6e6b46`, `0x6e6b52`, `0x6e6b83`, `0x6e6ba8`, `0x6e6bb4`
Confidence: verified
Proof: one
Test: none

`0x6e6b46` compares `+0x280` with `[0xcecaac]`; `0x6e6b52` tests `0x20000000`. The loop at
`0x6e6b72` tests `[edi+(i>>5)*4+0x284]` with `1 << (i&31)` and steps `edx` by `0x74`, 64 times. The
clamp is `add ebx,0x64`, `sets dl`, `dec edx`, `and` (`0x6e6bb4` to `0x6e6bc5`). The early exits
return `al = 0`, which every applier tests before touching its value.

## Modifier appliers

Three appliers take `(spell, &value, op)` and change the value only when `0x6e6b30` returns 1.
The integer one sets `value = (value + flat)·pct / 100`, 32-bit wrapping, the division signed and
truncating. The float one sets `value = (float)(((double)flat + value)·pct·0.01f)`: each step at
53-bit precision in that order, `0.01f` being `0x3c23d70a`, one rounding to single at the store.
The software one sets `value = ((value ⊕ F(flat)) ⊗ F(pct)) ⊗ 0.01f` in the arithmetic of
Software floats, where `F` is its int-to-float.

Addresses: `0x6e6af0`, `0x6e6b10`, `0x6e6b14`, `0x6e6bf0`, `0x6e6c0b`, `0x6e6c30`, `0xcf0b0c`
Confidence: verified
Proof: one
Test: `spell::spell_mod_float_difftest`, `spell::spell_range_for_caster_difftest`

`0x6e6af0`: `add ecx,[ebp+8]`, `imul ecx,[ebp-4]` (`0x6e6b10`), then `0x51eb851f`, `sar edx,5` plus
the sign bit (`0x6e6b14`). `0x6e6bf0`: `fild flat`, `fadd [value]`, `fimul pct`, `fmul [0x8029d0]`,
`fstp` (`0x6e6c0b`). `0x6e6c30` pushes `0xcf0b0c` first, then makes `F(pct)` and `F(flat)` with
`0x761260`, and calls `0x760e20(value, F(flat))`, `0x760be0(sum, F(pct))` and `0x760be0(product,
0.01)`. `0xcf0b0c` holds `0x3c23d70a`, stored by the static initializer `0x7626e0`; it has no other
writer. A NaN value passes through the float applier as NaN. `spell_mod_float_difftest` runs a slice
of the float applier, `0x6e6c0b` to `0x6e6c1b`. `spell_range_for_caster_difftest` calls the float
applier whole, but with no modifier, so it returns at the early exit. The integer and software
appliers have no test.

## Software floats

Five routines compute on single-precision bit patterns with integer instructions. They have no
case for infinity or NaN, and none checks exponent overflow.

| routine | result |
|---|---|
| `0x761260` int to float | 0 for 0; else sign, exponent from the top set bit, the magnitude shifted to 24 bits, low bits dropped |
| `0x760be0` multiply | with either fraction 0: 0 if either exponent is 0, else the exponent sum with the other fraction; else the high word of the two left-aligned 24-bit significands, truncated; either way 0 when the exponent sum less the bias, before any carry, is below 1 |
| `0x760e20` add | the other operand when an exponent is 0; the larger when exponents differ by 23 or more; else two's-complement significands with a guard bit, the smaller `sar`-aligned, summed, renormalized by shifts, truncated; a zero sum is +0 |
| `0x760fb0` floor | exact for finite inputs; under 1 in magnitude, −1.0 for a negative nonzero value, else +0.0 |
| `0x761040` ceil | exact for finite inputs; under 1 in magnitude, +0.0 for a negative nonzero value, else 1.0, zero included |

Addresses: `0x761260`, `0x760be0`, `0x760c3f`, `0x760e20`, `0x760e80`, `0x760ea7`, `0x760fb0`, `0x761040`, `0x76105d`, `0x761160`, `0xcf0a3c`, `0xcf0a40`, `0xcf0b64`, `0xcf0b2c`, `0x7625e0`, `0x762e20`, `0x762520`, `0x762be0`
Confidence: verified
Proof: one
Test: none

`0x760c3f` forms `expA + expB − 0x3f800000`; the mask `~((e − 0x800000) >> 31)` zeroes underflow.
The add's bounds are `cmp ecx,0xb800000; jl` (`0x760e80`) and `cmp ecx,0xf4800000; jg`
(`0x760ea7`); the result exponent is the larger plus `(8 − lz − 1)·2^23`. Floor and ceil keep the
top `e + 9` bits, adding `~mask` to the fraction first when rounding away from zero; `0x76105d`
tests the sign mask against `x + x`, so ±0 reads 1.0. Their constants are `F(0)`, `F(1)` and
`F(−1)`, made by `0x7625e0`, `0x762e20` and `0x762520`. `0x761160` is `floor(x ⊕ 0.5)`, the 0.5
being `0xcf0b2c`, stored as `0x3f000000` by `0x762be0`. Each global has only its initializer as writer.

## Power cost

`0x6e31b0(spellId, pet)` returns −1 without a record or caster. Else, with `L` the level from slot 42
over 5: `c = (L − base)·perLevel + cost` from `+0x70`, `+0x84`, `+0x80`; if `+0x270` is nonzero,
`c += P·pct/100` with `P` from `0x612c50` for the power type `+0x7c` (base mana for 0, base health for
−2, 1000 for 1, 100 for 2 and 3, else 0). Then `c += M[s]` and
`c = rint((float)((1.0 + K[s])·c))` for the school `s` at `+0x04`, `M` and `K` the unit's
`int32_t` at `+0x29c` and `float` at `+0x2b8`; `c ≤ 0` gives 0; then the integer applier with op 14,
unclamped.

Addresses: `0x6e31b0`, `0x6e3254`, `0x6e3290`, `0x6e32ad`, `0x6e32c0`, `0x6e32d6`, `0x6e32e0`, `0x6e32f0`, `0x612c50`, `0x6e3300`
Confidence: verified
Proof: one
Test: `spell::spell_power_cost_difftest`

The player is found with type mask `0x10`; with `pet`, the pet as in Caster level. The percentage term
is `imul`, then `0x51eb851f`, `sar 5` (`0x6e3290`). `0x6e32c0` to `0x6e32d6`: `fld K`, `fadd 1.0`,
`fimul c`, `fstp` to single, `fld`, `fistp` at round-to-nearest-even. A NaN `K` stores `0x80000000`,
which the clamp (`setle; dec; and`, `0x6e32e0`) makes 0. `0x6e3300` returns `(L − base)·[+0x8c] +
[+0x88]`, the power per second, with no modifier; −1 without a record. `spell_power_cost_difftest`
runs a slice, `0x6e32c0` to `0x6e32d9`. The lookups, the percentage term, the clamp and the applier
are not run.

## Cast time

`0x6e3340(spellId, pet, allowNegative)` returns 0 without a record or cast-time row. Else
`t = max((L − base)·perLevel + base, minimum)` from the `SpellCastTimes` record named by `+0x48`,
then the integer applier with op 10. Unless the record's `+0x18` has a bit of `0x30`, the caster
unit exists and `t > 0`: with `m` the unit's float at `+0x22c`, if `|m − 1| ≥ 2^-22`, or `m` is
NaN, `t = rint((float)(m·t))`. Finally, unless `allowNegative`, `t ≤ 0` gives 0.

Addresses: `0x6e3340`, `0x6e33b0`, `0x6e33be`, `0x6e33c3`, `0x6e3424`, `0x6e343a`, `0x6e3445`, `0x6e3456`, `0x6e3470`
Confidence: verified
Proof: one
Test: `spell::spell_cast_time_difftest`

The row's `+0x04`, `+0x08`, `+0x0c` are base, per level and minimum; `cmp eax,esi; jge` keeps the
larger (`0x6e33b0`). `0x6e3432` to `0x6e3445`: `fsub 1.0`, `fabs`, `fcomp 2^-22` (`0x34800000`), `test
ah,5; jnp` skips the multiply when C0 alone is set, so a NaN, which sets C0 and C2, multiplies.
`fimul` runs at 53 bits, `fstp` rounds to single, `fistp` (`0x6e3456`) rounds to even; a NaN stores
`0x80000000`. `t ≤ 0` jumps past the multiply (`0x6e3424`). The clamp is at `0x6e3468` to `0x6e3470`.
`spell_cast_time_difftest` runs a slice, `0x6e3426` to `0x6e3461`, for a positive time. The row
arithmetic, the applier and the clamp are not run.

## Duration

`0x6ea000(spell, pet, unmodified)` returns 0 for a null record or no `SpellDuration` row at `+0x78`.
Else `d = (L − base)·perLevel + duration` from the row's `+0x08` and `+0x04`, replaced by the row's
`+0x0c` unless below it; then, unless `unmodified`, the integer applier with op 1. The level term
is not floored and nothing is clamped.

Addresses: `0x6ea000`, `0x6ea046`, `0x6ea055`, `0x6ea064`
Confidence: verified
Proof: one
Test: none

`0x6ea03e` passes `movzx edx,dl` as the pet flag to `0x6e3130`. `0x6ea046` subtracts `+0x70`;
`0x6ea055` is `cmp eax,esi; jl`. `0x6ea064` tests the stack byte and skips the `push 1` call.

## Range

`GetMinMaxRange(caster, spellId, &min, &max, target)` zeroes both and returns for a null caster or
no record, or with `max = 100.0` for `+0x18 & 0x404`. `target` is a unit when its type
has bit 3. With `R(u)` a unit's float at unit field `+0x1f0`: a `SpellRange` row (`+0x90`) with flag
bit 0 sets `min = 0` and `max = S` if `S = R(t) + R(caster) + 4/3 > 5.0`, else 5.0, `t` being the
unit target, else the unit whose guid is at caster `+0xc48`, else the caster. Another row copies
its `+0x04` and `+0x08`; a target of type bit 3 or 7 adds `R(caster) + R(unit target, else
caster)` to `max`, and to `min` unless `min == 0`.

Addresses: `0x6e3480`, `0x6e34e1`, `0x6e34fb`, `0x6e3504`, `0x6e3545`, `0x6e3552`, `0x6e35a3`, `0x6e35bf`, `0x6e35ea`, `0x6e35ee`, `0x6e3605`, `0x6e3633`, `0x6e363e`, `0x47bf60`
Confidence: verified
Proof: one
Test: `spell::spell_minmax_range_difftest`

`0x6e34e1` to `0x6e34f9` keep the target in `esi` when `[[t+8]+8] >> 3 & 1`. `0x6e3504` loads
`[0x8118d4]`, 100.0. The melee sum runs at 53 bits with `4/3` as `0x3faaaaab`; `fcom 5.0`, `test
ah,0x41; je` (`0x6e35bf`) keeps the sum only when greater, so a NaN gives 5.0. `0x6e3552` fetches the
`+0xc48` guid with `0x47bf60` and looks it up with mask 8 when no unit was passed. `0x6e35ee` tests
bits 3 and 7 of the passed target; `0x6e3605` reads the second reach from the unit in `esi`, or the
caster's again. `max + P` and `P + min` each round once to single. `0x6e3633` compares `min` with 0.0:
`test ah,0x44; jnp` skips only on equal, so a NaN `min` takes `P`. `spell_minmax_range_difftest` runs
`0x6e3480` whole with its op 5 applier call overwritten, over the next-swing case, a ranged row and a
melee row with a unit target.

## Range bonuses and modifier

After either arm, with a unit in `esi`: when the movement flags at `[unit+0x118]+0x40` of both the
caster and that unit have a bit of `0x200d`, `0x5fc350` returns 0 for both, and the row has flag
bit 0 or the unit's type has bit 4, `max += 8/3`. Then, when `+0x18` has bit 1 and the caster's type
has bit 4, and the caster has more than 17 inventory slots whose slot 17 holds a cached item,
`max = (float)((W·0.01f)·max)` with `W` the item record's float at `+0x118`. Last, the float
applier with op 5 on `max`. A missing range row skips to the item step with both still 0.

Addresses: `0x6e3648`, `0x6e3658`, `0x6e366f`, `0x6e3683`, `0x6e369c`, `0x811914`, `0x6e36aa`, `0x6e36c1`, `0x6e3720`, `0x6e3729`, `0x6e3746`, `0x5fc350`, `0x55ba30`
Confidence: verified
Proof: one
Test: `objectlayer::diff_cast_time_elapsed`, `spell::spell_minmax_range_arms_difftest`

`0x6e3658` tests `0x200d` on both units; `0x6e366f` and `0x6e367a` call `0x5fc350`, which returns 0
when twice the unit's float at `+0xa30` is below the current speed `0x7c4c90` gives (`fcompp`, `test
ah,1`), `CMovement::GetCurrentSpeed` (`collision/kinematics`). `0x6e369c` adds `0x402aaaab`
(`0x811914`) at 53 bits and stores single. `0x6e36c1` is `cmp [edi+0x1d38],0x11; ja`, the guid at
`[[edi+0x1d3c]+0x88]`; the item is found with mask 2, its record through `0x55ba30` with the entry at
object field `+0x0c`. `0x6e3729`: `fld W`, `fmul 0.01f`, `fmul max`. Out-of-range row indices jump
straight to `0x6e36aa`. `diff_cast_time_elapsed` runs `0x5fc350` on the path where `0x7c4c90` returns
its constant. `spell_minmax_range_arms_difftest` runs two slices of `0x6e3480`, `0x6e3697` to
`0x6e36a4` (the 8/3 add) and `0x6e3729` to `0x6e373c` (the item scale). The flag tests, the item
lookup and the op 5 applier are not run.

## Effect classes

`0x6e3b80(effect, aura, &integral)` sorts an effect by its code and aura. It returns 1, with
`integral = 1`, for effects 2, 9, 17, 31, 58, 121, and for effects 6, 27, 35, 119, 128, 129 with
aura 3, 15, 43, 53 or 89. It returns 0 with `integral = 1` for effects 8, 10, 30, 62, 67, 75, and for
the aura effects with aura 8, 20, 21, 24, 62, 63, 64 or 162. Anything else returns 0 with
`integral = 0`, including effects outside 2 to 129.

Addresses: `0x6e3b80`, `0x6e3b99`, `0x6e3bb1`, `0x6e3bcc`, `0x6e3bdc`, `0x6e3c5c`, `0x6e3c68`
Confidence: verified
Proof: one
Test: none

`0x6e3b99` jumps through the four-entry table `0x6e3bcc` by the byte table `0x6e3bdc` at
`effect − 2`, 128 entries; `0x6e3bb1` through the three-entry `0x6e3c5c` by `0x6e3c68` at
`aura − 3`, 160 entries. The targets are `0x6e3bb8` (flag 1, return 1), `0x6e3bc2` (flag 1, return
0) and `0x6e3bc5` (return 0). Both byte tables were read from the image, every entry.

## Effect amount formula

`GetEffectPoints(spell, i, &min, &max, level, pet, unmodified)` zeroes both and returns for a null
record. `n = level`, or the caster level for the record's id when 0; `n −= base` when `base > 0`;
`n < 0` gives 0. With the effect's integers `perLevel` `+0x118`, `basePoints` `+0x130`, `dice`
`+0x10c`, `sides` `+0x100` and float `real` `+0x124`, each at `+4i`:
`min = F(perLevel·n + basePoints + dice) ⊕ (real ⊗ F(n))` and
`max = F(sides·(perLevel·n + dice) + basePoints) ⊕ (real ⊗ F(n))`, the integers 32-bit wrapping,
the rest software floats.

Addresses: `0x6e3800`, `0x6e3839`, `0x6e384d`, `0x6e3854`, `0x6e3863`, `0x6e3889`, `0x6e38b4`, `0x6e38bb`, `0x6e3915`
Confidence: verified
Proof: one
Test: `spell::spell_effect_amount_difftest`

`0x6e3839` classifies the effect first (`+0xf4`, `+0x16c`). `0x6e384d` calls `0x6e3130` with `[esi]`
and the `pet` argument. `0x6e3854` to `0x6e3861`: `jle` skips the subtraction, `jge` the floor.
`0x6e3863` to `0x6e387d` form the minimum's integer; `0x6e38bb` to `0x6e38d9` the maximum's. Each side
converts with `0x761260`, multiplies `real` (copied through `fld`/`fstp`, `0x6e3889`: bitwise, except
that a signalling NaN comes out quiet, bit 22 set) by `F(n)` with `0x760be0`, and adds integer first,
product second with `0x760e20` (`0x6e38b4`, `0x6e3915`). `spell_effect_amount_difftest` runs three
slices: the level delta (`0x6e3854` to `0x6e3863`), the minimum's integer (`0x6e3863` to `0x6e3882`)
and the maximum's (`0x6e38bb` to `0x6e38e3`). The software-float steps and the modifiers are not run.

## Effect amount modifiers and rounding

Unless `unmodified`, the software applier runs on `min`, then `max`, for op 8; then, for an effect
`0x6e3b80` returns 1 for, op 22 when the aura is 3, else op 0; then the aura's op: 2 for auras 10,
103, 183; 12 for 31, 32, 33, 58, 129, 130, 171, 172; 24 for 65; 3 for 99; 23 for 138. Then each of
`min` and `max` is rounded to 1/128: a nonzero exponent gains 7, `r = floor(x ⊕ 0.5)`, and the
result is `r` with 7 taken from its exponent, or +0 when that takes the exponent below 1. When
`integral`, `min = floor(min)` and `max = ceil(max)`.

Addresses: `0x6e392b`, `0x6e394a`, `0x6e397e`, `0x6e3988`, `0x6e3ab8`, `0x6e3ad0`, `0x6e39ee`, `0x6e3a0e`, `0x6e3a15`, `0x6e3a67`
Confidence: verified
Proof: one
Test: none

`0x6e397e` takes `aura − 10`; above `0xad` it skips. `0x6e3988` indexes the byte table `0x6e3ad0`,
174 entries, into the six-entry jump table `0x6e3ab8`, read from the image: byte 0 pushes 2,
1 pushes 12, 2 pushes 24, 3 pushes 3, 4 pushes 23, 5 is the exit. `0x6e39ee`: `and 0x7f800000`,
`neg`, `sbb`, `and 0x3800000`, `add`. `0x6e3a0e` calls `0x761160`; `0x6e3a15` to `0x6e3a27` add
`0xfc800000` under the mask `~(((r − 0x4000000) ^ r) >> 31)`. `0x6e3a67` tests `integral`; floor
and ceil are `0x760fb0` and `0x761040`, copied out through `fld`/`fstp`.

## Effect amount as integers

`0x6e3760` calls `GetEffectPoints` with its own arguments and converts each bound `x` to
`rint(2x − 0.5) >> 1` when `x ≥ 0` and `rint(2x + 0.5) >> 1` otherwise, an arithmetic shift, at
round-to-nearest-even. That is the floor for `x ≥ 0`, and nearest with ties toward zero for
`x < 0`. A NaN takes the second form and gives `0xc0000000`; so does a value past the `int32_t`
range.

Addresses: `0x6e3760`, `0x6e3777`, `0x6e377f`, `0x6e3791`, `0x6e3797`, `0x6e37b9`, `0x86f94c`
Confidence: verified
Proof: one
Test: `spell::spell_minmax_radius_difftest`

The pushes at `0x6e3763` to `0x6e3776` pass the caller's `level`, `pet` and `unmodified`, then the
addresses of its own `pet` and `unmodified` slots as `&max` and `&min`, so the bounds come back in
those slots. `0x6e377f` is `fcomp 0.0`, `test ah,1`: C0, set by less or unordered, takes the `+0.5`
form. `fadd st,st` doubles exactly; `0x86f94c` is 0.5, a `.data` float that only these four
instructions reference (dword scan of the image, control `0x6e2f70` found at its two pushes). `fistp`
(`0x6e3797`) then `sar eax,1` (`0x6e37b9`). `spell_minmax_radius_difftest` runs `0x6e3760` whole with
its call of `0x6e3800` overwritten, so the bounds enter as the two floats.

## Global cooldown start

`0x6e2de0(spellId, pet)` does nothing without a record, or when its `+0x274` and `+0x278` are both
0, or when `+0x278` after the integer applier with op 21 is 0. Otherwise it adds an entry to the
player's history (`pet` 0) or the pet's: the id, item 0, the clock as both start times, 0 for both
cooldowns, Attributes bit 25 as the held flag, `+0x274` as category and the modified time. Then
it calls `0x4b31b0`, `0x4b31c0` when `+0x18` has bit 25, and `0x4bce90` when `pet` is set.

Addresses: `0x6e2de0`, `0x6e2e0f`, `0x6e2e21`, `0x6e2e2c`, `0x6e2e3d`, `0x6e2e72`, `0x6e2e77`, `0x6e2e85`, `0x6e2e8e`, `0xcecaec`, `0x6e12c0`, `0x42c010`, `0x4b31b0`, `0x4b31c0`, `0x4bce90`, `0x4bd1d0`
Confidence: verified
Proof: one
Test: none

The history is `0xcecaec + 24·pet` (`0x6e2e67`). The ten pushes before `0x6e2e72` are, from the
first argument, id, 0, time, 0, 0, time, 0, `([+0x18] >> 25) & 1`, `[+0x274]`, the modified time;
`0x6e12c0` stores them at `+0x08` to `+0x2c` of a new entry (`cooldowns`). `0x4b31b0` signals
`ACTIONBAR_UPDATE_COOLDOWN` (216) then `SPELL_UPDATE_COOLDOWN` (262); `0x4b31c0` refreshes the
action bar flags, retests the reactive spells with `0x6e3d60` and signals `SPELL_UPDATE_USABLE` (263); `0x4bce90` signals
`PET_BAR_UPDATE_COOLDOWN` (354) (`tables/event_sites.tsv`). Callers: the cast send at `0x6e58fb`,
in `0x6e54f0`, with `pet` 0, and the pet bar's press, `0x4bd1d0`, at `0x4bd36e` with 1.

## Cooldown queries

`0x6e2ea0(spellId, pet, &duration, &start, &enabled)` returns `0x6e13e0` on the selected history
for the id and item 0. `0x6e2ed0(itemId, &duration, &start, &enabled)` fetches the item record
from the item cache, naming `0x6e2f70` as the callback for a record not yet cached, and queries
the player's history for the first of its five spells with an id above 0 and trigger 0, with the
item id; it returns 0 otherwise. `0x6e2f70`, given a found record, re-queries with null outputs
and, on a running cooldown, calls `0x4b31b0` and `0x4f93d0`. `0x6e2fa0(spell, pet)` returns
`0x6e1690` for the record's id and item 0.

Addresses: `0x6e2ea0`, `0x6e2ed0`, `0x6e2efb`, `0x6e2f20`, `0x6e2f58`, `0x6e2f70`, `0x6e2f80`, `0x6e2fa0`, `0x6e13e0`, `0x6e1690`, `0x4f93d0`
Confidence: verified
Proof: one
Test: none

`0x6e2ea0` pushes its three stack arguments, 0 and the id for `0x6e13e0`, `ret 0xc`. `0x6e2efb`
pushes the callback with argument 0 and flag 1 to `0x55ba30`. `0x6e2f20` walks `+0x11c` (spell ids)
and `+0x130` (triggers), `jle` then `je`. `0x6e2f58` queries `0xcecaec`. `0x6e2f70` tests its second
stack byte, then calls `0x6e2ed0` with `edx` and two pushed zeros, `ret 8`; the cache passes the
record's id in `ecx`. `0x4f93d0` signals `BAG_UPDATE_COOLDOWN` (330). `0x6e2fa0` loads `[ecx]`.
The meanings of the history routines are `cooldowns`'.
