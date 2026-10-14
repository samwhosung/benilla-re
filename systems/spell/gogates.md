# spell: gogates

What the spell go handler needs from the caster, the visual row and the lists before each of its legs runs.

The handler for `SMSG_SPELL_GO` (`go`) looks the caster up by guid and then runs legs that need the caster, legs
that need only the hit and miss lists, and legs that need the caster to be the player. A lookup that fails
does not stop the handler; it skips the legs that need the caster. This topic lists which leg needs what, so
a cast by something that is not a unit still plays its impacts.

Roots: `0x6e7a70`

## Early exits and the caster lookup

The handler returns early on three conditions only, all on the spell record: an id below 0, above the
store's maximum, or with no record. They are checked before the caster is looked up. The caster is the
object of the second guid, looked up as a unit (mask 8); a null result keeps the pointer null and skips the
cast-state arm, never returns. The miss and reflect legs look their own guids up, with mask 8, and skip
when the lookup fails.

Addresses: `0x6e7cd5`, `0x6e7cd7`, `0x6e7cdd`, `0x6e7ce3`, `0x6e7cf2`, `0x6e7cf7`, `0x6e8647`, `0x6e7dbd`, `0x6e7e3a`, `0x6e7f7a`, `0x6e7f8b`, `0x6e7f94`, `0x6e7f97`, `0x6e8012`, `0x468460`
Confidence: verified
Proof: one
Test: none

`0x6e7cd7 jl`, `0x6e7ce3 jg` and `0x6e7cf7 je` jump to the end `0x6e8647`; no later test of the caster
does. `0x6e7f8b` is `call 0x468460` with `ecx` 8 on the second guid; `0x6e7f94` stores the result at
`[ebp-0x30]` before `0x6e7f97 je 0x6e8012`, a forward skip over the arm. The other lookups are `0x6e7dbd` (the
miss target) and `0x6e7e3a` (the reflect target), each followed by a short forward `je` over their own
calls. The 1090 calls of `0x468460` each load `ecx` with a single-bit mask: 0x10 (538), 8 (310), 1 (108), 2 (84),
0x20 (32), 4 (11), 0x80 (5) and 0x40 (2).

## Legs by gate

The cast-state arm, the caster's cast kit, the dest-point effect, the whole missile branch (a speed above 0,
or NaN) and the pet cooldown tail need the caster. The cast kit and the dest effect also need the visual's
kit or point. The instant impact of a speed of 0 or below runs on the hit list whatever the caster, and so
do the miss lines; the reflect impact receives the caster and tolerates null. The open-lock leg and the
player's cooldown tail need the caster to be the active player. A spell with no visual row plays nothing of
the visual legs, impacts included.

Addresses: `0x6e7f99`, `0x6e801d`, `0x6e801f`, `0x6e8035`, `0x6e803f`, `0x6e8041`, `0x6e82a7`, `0x6e8064`, `0x6e8066`, `0x6e8082`, `0x6e8148`, `0x6e8159`, `0x6e815c`, `0x6e816b`, `0x6e8192`, `0x6e819b`, `0x6e8219`, `0x6e821b`, `0x6e81ac`, `0x6e81b8`, `0x6e81c3`, `0x6e8577`, `0x6e857a`, `0x6e8bf0`, `0x6e8cb0`, `0x6e8cde`, `0x6e8a50`, `0x60d450`, `0x6ec1e0`
Confidence: verified
Proof: one
Test: none

`0x6e801f je 0x6e8035` picks the visual resolver: the caster's (`0x60d450`, which merges the equipped ranged
weapon's row) or the spell's own (`0x6ec1e0`); `0x6e8041 je 0x6e82a7` is the missing-row path, which prints
and jumps to `0x6e81ac`, past the speed router. `0x6e8066` and `0x6e8082` test the caster before the cast kit and
the dest effect. The speed router (`0x6e8148` to `0x6e815c`) is reached with the caster possibly null;
`0x6e816b` calls the instant impact `0x6e8bf0` with the record and the visual, and no caster. `0x6e8192`
passes the caster to the reflect impact, whose `0x6e8cde test edi,edi` skips a null one. `0x6e8219 test esi,esi;
je 0x6e81ac` skips the missile branch, a call on the caster. `0x6e81b8` and `0x6e81c3` compare the
second guid with the active player's; a mismatch jumps to `0x6e8577`, past the open-lock leg and the cooldown
tail, and `0x6e857a` null-guards the pet leg.

## Game object caster

A cast whose caster is a game object plays as a cast with no caster. The first guid is looked up with mask 1,
which matches any object; if the object has the unit bit or the game object bit, its guid is the source of the
miss lines, else the second guid is. The caster slot is looked up with mask 8 on the second guid; a game object
has no unit bit, so it is null, and the cast plays its impact legs but no cast kit, no dest effect and no
missile.

Addresses: `0x6e7d0e`, `0x6e7d13`, `0x6e7d18`, `0x6e7d24`, `0x6e7d2c`, `0x6e7d33`, `0x6e7d40`, `0x6e7f8b`, `0x6e83da`, `0x6e83e0`, `0x6e8402`, `0x47bf60`, `0x6131a0`, `0x62bab0`
Confidence: verified
Proof: one
Test: none

`0x6e7d0e mov ecx,1`, `0x6e7d13 call 0x468460` on the first guid, `0x6e7d24` tests bit 5 of the object's
type word and `0x6e7d2c` bit 3; `0x6e7d33` keeps the first guid as the source and `0x6e7d40` the second. The
type word of a game object is `0x21` (bits 0 and 5, `objectlayer`, Object types), so `0x6e7f8b` finds nothing
for it. The player tail dereferences the caster with no test (`0x6e83da lea ecx,[esi+0xc48]`, `0x6e8402
call 0x6131a0`) but is reached only when the second guid equals the active player's guid, and the active
player has a unit; `0x468550` answers 0 when there is no object manager.
