# spell: castgates

The caster validator the cast request runs last, rung by rung: cooldown, power, summons, control, stance, place and time.

`CheckCasterStateForSpell` (`0x6094f0`, `Unit_C.cpp`, owned by `objectlayer`) is the cast request's final
refusal check. The request (`cast`) runs the possession, equipment and range checks (`checks`) first,
then this validator, and only then fills the pending cast and sends. Each rung tests one condition on
the caster or the spell record, reports a reason through `0x6e1a00` (`failures`) and returns 0, so the
first failing rung is the only one shown. This topic lists the rungs in address order with the fields
each reads. Spell record offsets are the bytes'; their column names are inferred
(`checks`, Spell record columns), and `+0x2c` Stances, `+0x30` StancesNot, `+0x54`, `+0x58` and `+0x5c`
the spell, aura and channel interrupt flags, `+0x2a8` and `+0x2ac` the required faction and rank are
added here. A unit's `+0x110` points at its unit field block, as in `checks`.

Roots: none of its own; the validator is owned by `objectlayer` and called only by the cast request

## Validator entry and callers

`0x6094f0(caster, record, item)` is `__fastcall` with the caster in `ecx`, the record and the casting item
object on the stack, `ret 8`. It returns 1 when every rung passes. A null record sets last error 0x57
through `0x64e850` and returns 0 with no report. The cast request is the only caller: it calls it after
the range pre-check and before it fills the pending cast, and a 0 returns at once, so a refusal sends no
packet, starts no cast and writes no pending-cast field. The usability predicate that greys buttons never
calls it; a button stays lit while its spell would be refused here.

Addresses: `0x6094f0`, `0x609517`, `0x60952b`, `0x60a046`, `0x6e4f3b`, `0x6e4f42`, `0x6e506e`, `0x64e850`, `0x6e1a00`, `0x6e3d60`, `0x6e5a90`, `0x5d9258`, `0x5f35c0`
Confidence: verified
Proof: one
Test: none

A scan of the listing for `call 0x6094f0` and `jmp 0x6094f0` finds one site, `0x6e4f3b`, followed by
`test al,al; je 0x6e506e` (`xor al,al; ret 0xc`); the dword scan of the image for the address finds none
(control: the same scan finds `0x6e43a0` twice, as the address pushed for the inventory walk). The body
holds 40 calls of `0x6e1a00`, at least one for each rung below, and ends `mov al,1; ret 8` (`0x60a046`). The item
argument is the request's first stack argument, which its caller `0x6e5a90` forwards from `edx`: of the
15 call sites of `0x6e5a90`, 13 clear `edx` first and two pass an item object (`0x5d9258`, in the item
use, and `0x5f35c0`, in the game object use). The usability predicate `0x6e3d60` has no call of the
validator, and the address occurs nowhere as data.

## Cooldown rungs

With an item object, the validator asks the item's cooldown (`0x6e2ed0`, keyed by the item's entry) and
refuses a running one with reason 0x28, and a ready item skips the spell cooldown rung and the power rung.
Without one it asks the spell's cooldown (`0x6e2ea0`, player list) and refuses a running one with 0x3c.
Before that report, when the auto-repeat spell has a record whose `AttributesEx3` has bit 22 (0x400000),
it sends `CMSG_CANCEL_CAST` (0x12f) with the auto-repeat spell id and stops the auto-repeat (`0x6ea080`).
The cooldown queries include the global cooldown, so a global cooldown refuses a spell as 0x3c and an item
as 0x28.

Addresses: `0x609530`, `0x60953c`, `0x609543`, `0x609549`, `0x609553`, `0x609565`, `0x609569`, `0x60956e`, `0x609576`, `0x60959e`, `0x6095b8`, `0x6095e7`, `0x6095ec`, `0x609616`, `0x60961a`, `0x6e2ed0`, `0x6e2ea0`, `0x6e9fd0`, `0x6ea080`, `0x5ab630`, `0x418190`, `0x6e13e0`
Confidence: verified
Proof: one
Test: none

`0x609530` tests the item; `0x60953c` calls `0x6e2ed0` with the entry `[[item+8]+0xc]`, and a zero
result jumps to `0x6096b3`, past both the spell leg and the power rung. A nonzero one pushes 0, -1, -1 and
`dl` 0x28 (`0x609549` to `0x609553`). The spell leg calls `0x6e2ea0` with three null output pointers
(`0x609569`); `je 0x60962c` is the ready case. `0x6e9fd0` returns `0xceac30`; the id must lie from 0 to
the store's maximum (`0x609585`), the record be non-null and `test [rec+0x24],0x400000` set (`0x60959e`).
The packet is a data store that puts `0x12f` and the id (`0x6095b8` to
`0x6095d8`) and is sent at `0x6095e7`. `0x609616` loads 0x3c with the from-server byte 0.

## Power rung

The validator refuses a spell the caster cannot pay for with reason 0x4d, for a spell without an item
object. The available amount is the caster's raw unit field: `UNIT_FIELD_POWER1` plus the power type
(`+0x7c`), or `UNIT_FIELD_HEALTH` for any negative type, unscaled by any modifier. The cost is
`0x6e31b0` (`records`). The comparison is signed, and equal passes. Reason 0x4d covers every power type;
its message is chosen by the type (`failures`).

Addresses: `0x60962c`, `0x609631`, `0x609646`, `0x60964c`, `0x609657`, `0x60965c`, `0x60965f`, `0x609661`, `0x60967d`, `0x609683`, `0x60968d`, `0x609692`, `0x609695`, `0x6e31b0`
Confidence: verified
Proof: one
Test: none

`0x609631 jl 0x60967d` sends every negative power type to the health arm (`[desc+0x40]` at `0x609683`), so
the `cmp eax,-2` at `0x609633` never equals and the health read at `0x609638` is dead. Otherwise the amount
is `[desc+eax*4+0x44]` (`0x60964c`), unbounded in the type. `0x609657` calls `0x6e31b0(id, 0)`; `0x60965f
jge` passes and `0x609661` reports. The other callers of `0x6e31b0` are the usability cache (`0x4e5201`), the usability predicate
(`0x6e3fdb`) and the tooltips (`0x507de3`, `0x52e8ad`).

## Learn, summon and pet rungs

For each of the three effects in order, the validator tests three things. A learn-spell effect (36) on a
player who knows its trigger spell (`0x60c740`) shows `ERR_SPELL_ALREADY_KNOWN_S` with that spell's name
and refuses. A summon or charm effect (28, 55, 56, or aura 6 or 2) refuses with reason 4 when the caster
holds a charm, else 5 when it holds a summon, unless `AttributesEx` has bit 0. An effect with implicit
target A or B 5 (the pet) refuses with 0x4c when the caster holds neither; a learn-spell effect on that pet
refuses with 0x63 when it cannot learn the spell, and shows `ERR_PET_SPELL_ALREADY_KNOWN_S` with the cast
spell's name when it knows it or a higher rank.

Addresses: `0x6096b3`, `0x6096c6`, `0x6096d4`, `0x6096e1`, `0x609854`, `0x609884`, `0x6098b7`, `0x6098ce`, `0x6098e8`, `0x609920`, `0x609930`, `0x60990a`, `0x609767`, `0x60979e`, `0x6097b1`, `0x6097c5`, `0x6097d9`, `0x60c740`, `0x60c840`, `0x60c8d0`, `0x496720`, `0x468460`
Confidence: verified
Proof: one
Test: none

`0x6096c6 cmp [Effect],0x24` and the player test `[[edi+8]+8] >> 4 & 1` gate the self check, whose
`0x60c740` argument is `[Effect+0xc0]`, the trigger spell at `+0x1b4`. The message ids 0x32 and 0x33 are
rows 50 and 51 of `tables/messages.tsv`. The summon test (`0x6096f6` to `0x609715`) compares the effect
with 0x1c, 0x38 and 0x37 and its aura (`+0x16c`) with 6 and 2; the guid it tests is the unit field block's
first pair (charm), else the pair at `+0x08` (summon), and `test [esi+0x1c],1` skips the refusal
(`0x609730`). `0x6098ce jne` reports 4 for a nonzero first pair, else 5. The pet rung compares `+0x148` and
`+0x154` of the effect with 5 (`0x60973e`, `0x609743`) and looks the pet up by that guid with mask 8.

## Control rung

A caster whose health is 0 or below skips this rung and the six crowd-control arms (`checks`, Crowd-control
use in the validator) and goes on to the stance rungs. For a living player whose `PLAYER_FIELD_BYTES`
byte 0 has bit 0x4, the rung refuses with reason 0x61, before the crowd-control arms.

Addresses: `0x609802`, `0x60980d`, `0x609813`, `0x609825`, `0x60982b`, `0x609832`, `0x60983e`, `0x609842`, `0x609945`, `0x609ba1`
Confidence: verified
Proof: one
Test: none

`0x609808` loads `[desc+0x40]`, the unit's health, and `0x60980d jle 0x609ba1` skips the block. `0x609813`
tests bit 4 of the type word, `0x609825` loads `[edi+0xe68]` (the player block) and `0x60982b test
[+0x1028],4` (field 1222, byte 0).

## Interrupt flag stance rungs

Four rungs share one shape: each refuses when a spell flag asks for a state the caster lacks, and each is
an OR of two independent arms. Arm A tests a bit of the spell's aura interrupt flags (`+0x58`); arm B tests
the same bit of the channel interrupt flags (`+0x5c`) when `AttributesEx` (`+0x1c`) has either bit of 0x44 (`and eax,0x44; je`). A pass of
arm A does not skip arm B.

| reason | bit | state required |
|---|---|---|
| 0x41 not unsheathed | 0x200 | weapon state `+0xd40` nonzero |
| 0x53 only mounted | 0x40 | `UNIT_FIELD_MOUNTDISPLAYID` above 0 |
| 0x58 only underwater | 0x100 | movement flag `SWIMMING` (0x200000) set |
| 0x50 only above water | 0x80 | movement flag `SWIMMING` clear |

The rungs run in the order 0x41, 0x53 and, after the mount, stand, daytime and nighttime rungs, 0x58 before 0x50.

Addresses: `0x609ba1`, `0x609ba4`, `0x609bd2`, `0x609bda`, `0x609bdf`, `0x609c05`, `0x609c36`, `0x609c3a`, `0x609c46`, `0x609d33`, `0x609d36`, `0x609d46`, `0x609d67`, `0x609d6f`, `0x609d7b`, `0x609da2`, `0x609dac`, `0x609db1`, `0x609db5`, `0x609dc2`, `0x609dcd`, `0x609dd1`
Confidence: verified
Proof: one
Test: none

The aura flags dword is loaded once at `0x609ba1` (`ecx`) and read again at `0x609c05` (`test cl,0x40`),
and at `0x609d33` (`eax`) and read again at `0x609da2` (`test al,al; jns`, bit 0x80); every path between
leaves the register intact. The sub-register tests decode from their ModRM bytes: `test ch,2` is bit 0x200
and `test ah,1` bit 0x100. `0x609bd2 and eax,0x44` leaves the masked `AttributesEx` in `eax`, which `0x609c36` and the
channel arms reuse (`0x609d6a` recomputes it into `ecx`). `0x609d39` hoists the mask 0x200000 into `edx`,
`0x609d75` overwrites it and `0x609d9d` restores it. The swim test reads the mover's flags
`[[edi+0x118]+0x40]`, as a dword at `0x609d46` and as `byte [+0x42] & 0x20` at `0x609d7b`.

## Mount and stand rungs

A mounted caster (`UNIT_FIELD_MOUNTDISPLAYID` above 0) refuses with reason 0x39 unless `Attributes` has
bit 24 (0x1000000). A caster that is not standing refuses with 0x3e unless `Attributes` has bit 27
(0x8000000): the test is the caster's stand state, vtable slot 41, nonzero. Mounted precedes stand, and
both follow the stance rungs 0x41 and 0x53.

Addresses: `0x609c6c`, `0x609c6f`, `0x609c7c`, `0x609c84`, `0x609c8c`, `0x609ca2`, `0x609ca7`, `0x609cad`, `0x609cbd`, `0x5ed570`, `0x60be50`
Confidence: verified
Proof: one
Test: none

`0x609c7c` loads `[desc+0x1fc]` and `0x609c84 jle` skips. `0x609cad` is `call [edx+0xa4]` on the caster's
vtable, slot 41, which is `CGPlayer_C::GetStandState` (`0x5ed570`) or `CGUnit_C::GetStandState`
(`0x60be50`). The player test is the stand state byte of `UNIT_FIELD_BYTES_1`, or the active player's local
state.

## Day and night rungs

A spell with `Attributes` bit 12 (0x1000) refuses with 0x51 when `0x60a050` returns 0, and one with bit 13
(0x2000) refuses with 0x54 when it returns nonzero. `0x60a050` decides whether the game clock `0xce8538`
lies between 5:30 and 21:00.

Addresses: `0x609cd3`, `0x609cd6`, `0x609cdb`, `0x609ce2`, `0x609cee`, `0x609d03`, `0x609d06`, `0x609d0b`, `0x609d12`, `0x609d1e`, `0x60a050`, `0x60a067`, `0x60a072`, `0x60a08b`, `0x60a0a8`, `0xce8538`, `0x642ae0`, `0x642710`
Confidence: verified
Proof: one
Test: none

`0x609cd6 test ah,0x10` and `0x609d06 test ah,0x20` are bits 12 and 13. The two rungs differ only in
the branch after the call: `0x609ce2 jne` passes the day rung and `0x609d12 je` passes the night rung.
`0x60a050` encodes the times 21:00 (`push 0x15`) and 5:30 (`push 5`, `edx` 0x1e) through `0x642ae0`, decodes
both with `0x642c30` and returns `0x642710` on the clock with the two times (`0x60a0a8`). The comparison
function is in `base`; both rungs reload `edi` afterwards.

## Moving rung

A spell whose `InterruptFlags` (`+0x54`) has bit 0 refuses with 0x2e when the caster's movement flags
have any of forward, backward, strafe left, strafe right or falling (0x200f). It does not apply to a spell
with `AttributesEx2` bit 5 (0x20, auto-repeat). It applies when the spell has a cast time, or its aura
flags or its channel flags have bit 3 or 4 (0x18): an instant spell without those bits passes. The cast time
is `0x6e3340` with the pet flag set when `UNIT_FIELD_PETNUMBER` is nonzero. Turning and pitching in place,
swimming and the 0x4000 flag do not count.

Addresses: `0x609de3`, `0x609de9`, `0x609def`, `0x609df8`, `0x609dfe`, `0x609e04`, `0x609e16`, `0x609e1b`, `0x609e1f`, `0x609e22`, `0x609e28`, `0x609e35`, `0x609e37`, `0x6e3340`
Confidence: verified
Proof: one
Test: none

`0x609def test [edx+0x40],0x200f` reads the mover flags in the layout the movement packets use: 0x1
forward, 0x2 backward, 0x4 and 0x8 strafe, 0x2000 falling (`CMovement::StartFalling2`, `0x7c61f0`, sets it). `0x609e04` loads
`[desc+0x214]`, the pet number; `0x609e16` calls `0x6e3340(id, petflag, 0)` and `0x609e1d` refuses on a
nonzero time. `0x609e22` is `mov al,0x18`, tested against `[esi+0x58]` (`0x609e24`) and `[esi+0x5c]`
(`0x609e28`). The moving rung follows the water rungs and precedes the form rung.

## Form and stealth rungs

After the moving rung the validator calls the form check `0x612480` with an out dword; a 0 result refuses
with the reason it stored, 0x3d or 0x56. A spell with `Attributes` bit 17 (0x20000) refuses with 0x57 when
the caster's `UNIT_FIELD_BYTES_1` byte 3 lacks bit 0x2 (stealth). A spell with `Attributes` bit 28
(0x10000000) refuses with reason 0, `AFFECTING_COMBAT`, when `UNIT_FIELD_FLAGS` has bit 19.

Addresses: `0x609e49`, `0x609e50`, `0x609e59`, `0x609e64`, `0x609e76`, `0x609e79`, `0x609e89`, `0x609e9c`, `0x609eae`, `0x609ebb`, `0x609ec1`, `0x609ed2`, `0x612480`
Confidence: verified
Proof: one
Test: none

`0x609e59` reads the reason from `[ebp-0x18]` into `dl`. `0x609e89 test byte [ecx+0x213],2` is byte 3 of
`UNIT_FIELD_BYTES_1` bit 1. `0x609ec1 shr eax,0x13` on `[desc+0xa0]` tests bit 19; `0x609ed0 xor dl,dl`
loads reason 0.

## Aura state and reputation rungs

A spell whose `CasterAuraState` (`+0x40`) is n above 0 refuses with 0x12 when bit n−1 of
`UNIT_FIELD_AURASTATE` is clear. A spell whose required faction (`+0x2a8`) is nonzero refuses with 0x8f
when the player's standing with that faction (`0x4d6370`) is below the minimum of the rank in `+0x2ac`:
`s_reputationRankMinimums` (`0x80928c`), −42000, −6000, −3000, 0, 3000, 9000, 21000, 42000, then 43000.
The rank index is not bounded.

Addresses: `0x609ee4`, `0x609eeb`, `0x609efb`, `0x609f0d`, `0x609f1f`, `0x609f25`, `0x609f29`, `0x609f34`, `0x609f45`, `0x609f47`, `0x4d6370`, `0x80928c`
Confidence: verified
Proof: one
Test: none

`0x609ee4` loads `[esi+0x40]` and `0x609eeb` forms `1 << (n-1)` against `[desc+0x1dc]`. `0x609f1f` loads
`[esi+0x2a8]`; `0x609f29` calls `0x4d6370` and `0x609f34 cmp eax,[ecx*4+0x80928c]` with `ecx` from
`[esi+0x2ac]`; `jge` passes.

## Place and taming rungs

A spell with `Attributes` bit 14 (0x4000) or 15 (0x8000) tests the caster's world node (`0x612cc0`). Bit 14 refuses with 0x52 when the test returns nonzero. Bit 15 refuses when it returns 0: with 0x4b when
one of the three effects applies an aura of type 78 (`Effect` 6), else 0x55. A spell with `AttributesEx2`
bit 16 (0x10000) refuses with 0x17 when the caster is the active player and `PLAYER_FIELD_BYTES` byte 0 has
bit 0, after showing the taming failure for reason 2 (`0x6e6a20`).

Addresses: `0x609f59`, `0x609f63`, `0x609f6f`, `0x609f7c`, `0x609f90`, `0x609f94`, `0x609fa1`, `0x609fa6`, `0x609fb4`, `0x609fd0`, `0x609fd7`, `0x609fe0`, `0x609ffb`, `0x60a00b`, `0x60a018`, `0x60a01e`, `0x60a02b`, `0x612cc0`, `0x6e6a20`
Confidence: verified
Proof: one
Test: none

`0x612cc0` is `mov ecx,[ecx+0xe0]; jmp 0x6706f0`, the world node test. `0x609f5c test ah,0xc0` selects
bits 14 and 15; `0x609f6d je` is the zero-result case. The scan at `0x609f94` compares `[esi+0x16c+4i]` with
0x4e and `[esi+0xf4+4i]` with 6 for i in 0 to 2, and `0x609fd0 mov dl,0x4b` joins the 0x55 report at
`0x609fb6`. `0x609fe0` takes the object's type word bit 4 and compares its guid with `0x468550`; `0x60a018
test al,1` is `PLAYER_FIELD_BYTES` byte 0, and `0x60a01e` is `call 0x6e6a20` with `cl` 2.

## Rung order

The rungs run in this order, each a report followed by a return of 0: item cooldown 0x28; spell cooldown
0x3c; power 0x4d; learn, summon and pet (0x32, 0x33, 4, 5, 0x4c, 0x63); for a living caster 0x61 and the
control arms (0x14, 0x64, 0x60, 0x5a, 0x1e, 0x16, each with 0x8d for a named mechanic); not unsheathed
0x41; only mounted 0x53; mounted 0x39; not standing 0x3e; daytime 0x51; nighttime 0x54; underwater 0x58;
above water 0x50; moving 0x2e; form 0x3d or 0x56; stealth 0x57; combat 0; caster aura state 0x12;
reputation 0x8f; indoors 0x52 and outdoors 0x55 or 0x4b; taming 0x17.

Addresses: `0x609549`, `0x609616`, `0x609661`, `0x6096a1`, `0x6096e8`, `0x609730`, `0x609761`, `0x6097b8`, `0x6097cc`, `0x6098d6`, `0x609842`, `0x609e37`, `0x609ed2`, `0x609f47`, `0x60a02d`
Confidence: verified
Proof: one
Test: none

The rungs run in the order above, which is not the order of their report calls in the listing. The three-effect
loop (`0x6096b3` to `0x6097fc`) runs before the health test, but its reports sit after the 0x61 call
(`0x609842`): `0x6096e8`, `0x609730`, `0x609761`, `0x6097b8` and `0x6097cc` jump out of the loop to `0x609854`
(0x32), `0x6098b7` (4 or 5), `0x6098e8` (0x4c), `0x609904` (0x63) and `0x609920` (0x33). Power reports at
`0x609667` and, for a health cost, `0x6096a1`. The other reports read `dl` before the call: `0x60954f` 0x28,
`0x609616` 0x3c, `0x60983e` 0x61, the six crowd-control arms from `0x609989`, `0x609bb9` 0x41, `0x609c20` 0x53,
`0x609c8c` 0x39, `0x609cbd` 0x3e, `0x609cea` 0x51, `0x609d1a` 0x54, `0x609d51` 0x58, `0x609dcd` 0x50, `0x609e35`
0x2e, `0x609e9a` 0x57, `0x609ed0` 0, `0x609f0b` 0x12, `0x609f45` 0x8f, `0x609f7c` 0x52, `0x609fb4` 0x55,
`0x60a02b` 0x17. From the 0x61 rung on, each rung is reached by falling through from the one above, and `0x6098ce`
reuses the water rung's call at `0x609dd1` for reason 4.
