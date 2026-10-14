# objectlayer: auras

How a unit shows the spells on it: the aura fields and the slots the client keeps beside them,
the visual kits a spell plays on the unit and their character procedures, the effects, tints and
fades those leave, channels, and the missiles a unit launches. The fields are the server's:
`UNIT_FIELD_AURA` (fields 47 to 94, one spell id per slot), `UNIT_FIELD_AURAFLAGS` (95 to 100, a
nibble per slot) and `UNIT_FIELD_AURAAPPLICATIONS` (113 to 124, a byte per slot). A slot holds an
aura when its spell id is nonzero and its nibble has a bit of 0xe.

Roots: `0x604d00`, `0x5ff350`, `0x60edf0`, `0x60d7c0`, `0x60a3d0`

## Aura changes

The watcher `0x604d00` runs when any of fields 47 to 100 changes. It compares the 48 slots with
the copy of the old values: a slot whose old aura is gone or replaced is removed first, through
`0x612320`, then a slot with a new aura is added through `0x6123f0`. Each also posts the combat
log line, slots 32 to 47 as harmful. A rising applications byte posts the stack line.

Addresses: `0x604d00`, `0x604d79`, `0x604dc9`, `0x604e73`, `0x612320`, `0x6123f0`, `0x604ea0`, `0x612450`
Confidence: verified
Proof: one
Test: none

`0x604d00` walks `[+0x110]+0xa4` to `+0x164` against the old block, taking the nibble at
`+0x164 + slot/2`, shifted by 4 for an odd slot, and `and 0xe` (`0x604d79`); a slot stays when its
spell and presence match. `0x612320(slot, spell)` lets the active player undo a far-sight aura
(effect aura 76, 0x50d320 on the active camera), calls `0x4e5de0(spell)`, clears the slot
(`0x5ff290`), logs `0x62b480(unit, spell, harmful, 0)` and refreshes the transform spell
(`0x5ff0c0`). `0x6123f0` does the reverse with `0x5ff350` and logs with 1. `0x604ea0` takes the
byte's offset less `0x1ac`; `0x612450` logs `0x62b800` when the new count beats the old.

## Showing an aura's slot

`0x5ff350(slot)` records the slot's spell at `+0xb58 + 4·slot` and plays its visual. A slot keeps
the spell already there when that one's priority is higher. A spell that needs an aura vision
the active player lacks is not shown. A far-sight aura moves the active player's camera. The
state kit of the spell's visual plays as kind 2 unless the spell's missile is still in flight.
`0x5ff290` stops it and clears the slot.

Addresses: `0x5ff350`, `0x5ff360`, `0x5ff3ba`, `0x5ff3cc`, `0x5ff4c6`, `0x5ff290`, `0x5ff298`, `0x5ff130`, `0x612cd0`, `0x5ff0c0`, `0x5ff100`
Confidence: verified
Proof: one
Test: none

The slot array is 48 dwords, `+0xb58` to `+0xc17`, but both functions bound the slot with
`cmp 0x30; ja` (`0x5ff360`, `0x5ff298`), so slot 48 passes: it reads `AURAFLAGS[0]` as a spell
and writes `+0xc18`, the breath timer. No caller passes 48; every loop stops at 47. Priority is
`+0x1dc` of the spell records (`0x5ff3ba`); vision is `+0x2b0`, a bit of the player block's
`+0x10c1` (`0x5ff3cc`). `0x5ff130` re-shows every nonzero slot; `0x612cd0(mask)` re-shows or
clears the slots whose vision is in `mask` and signals `UNIT_AURA`. `0x5ff0c0` keeps at `+0xd54`
a spell that `0x5ff100` finds applying aura 36 (shapeshift) or 56 (transform).

## Playing a visual kit

`0x60edf0(spell, kit, kind, position)` plays a `SpellVisualKit` row on the unit. The kind picks
the effect's lifetime: 0 one shot, 1 impact, 2 a state that holds, 3 a state that lasts the
spell's duration, 4 a precast loop. The kit's animation plays on the unit; each effect slot spawns
an effect at its attachment; the world effect is placed at the position; then the character
procedures, the sound and the camera shake run.

Addresses: `0x60edf0`, `0x60f510`, `0x60f4f8`, `0x60ee77`, `0x60ef3d`, `0x60f011`, `0x60f106`, `0x60f35c`, `0x60f3c5`, `0x60f4bc`, `0x60f4e6`, `0x60ed40`, `0x60ed00`, `0x5ff170`, `0x5ff1d0`, `0x5ff270`, `0x5fbf50`
Confidence: verified
Proof: one
Test: none

The kind switch is `0x60f4f8`. Kinds 0 and 1 end the effect when its model ends (`0x5fbf50`);
2 holds with 158 Hold and ends with 159 Decay once the effect's `+0x58` time passes (`0x5ff170`,
`0x5ff1d0`, `0x5ff270`); 3 takes `SpellDuration` base plus per-level times the level over the
spell's, capped, as `+0x58` (`0x60ef3d`); 3 and 4 loop through `0x60ed00`. Kit fields and
attachments: `+0xc` head 20, `+0x10` chest 34, `+0x14` base 19, `+0x18` and `+0x1c` hands 21,
22, `+0x20` breath 17, `+0x24`, `+0x28` weapons 23, 24, `+0x2c` 25, through `0x60ed40`; `+0x30`
at the position (`0x60f106`); `+0x34` sound (`0x60f4bc`); `+0x38` shake (`0x60f4e6`). A
ranged spell (Attributes 0x2) draws the ranged weapon. Outside kind 2 the animation `+0x8` plays through
`0x5fe2f0` (`0x60f3c5`), as a wound for 8 to 10 (`0x60f510`).

## The state kit's animation

A kit played as a state (kind 2) does not play its animation. It only checks whether the unit
already plays it and, if not, picks the base animation again. The stun animation is therefore
named by the stun spells' state kits and never played by them.

Addresses: `0x60edf0`, `0x60f361`, `0x60f366`, `0x60f383`, `0x60f38b`, `0x60f390`, `0x60f399`, `0x60f3c5`, `0x5fdb50`, `0x5fd9e0`, `0x5ff4c6`, `0x612b31`, `0x61da65`, `0x61dced`, `0x612990`, `0x5fd2e0`, `0x5fd31f`, `0x612a05`, `0x5fda20`, `0x5fd951`, `0x5fda03`, `0x6129e4`, `0x6129e9`
Confidence: verified
Proof: one
Test: none

A kit animation below 0 skips the block (`0x60f366`). For kind 2 (`0x60f383`) the code takes the
animation armed on the unit (`0x5fdb50`, the upper-body key bone first), returns when it equals
the kit's (`0x60f390`) and else calls `0x5fd9e0(-1)` (`0x60f399`); it never reaches `0x60f3c5`,
the call that plays a kit's animation for the other kinds. A precast or channel kit's animation reaches `0x5fe2f0` another way:
`0x5fd9e0` calls the selector `0x5fd8b0` (`0x5fda03`), which calls the resolver `0x5fd2e0` (`0x5fd951`); that calls `0x612990` (`0x5fd31f`), which returns the animation `+0x8` of a kit row (`0x612a05`), and `0x5fda20`, inside `0x5fd9e0`, plays it through `0x5fe2f0`. The four calls with kind 2 are `0x5ff4c6`,
`0x612b31`, `0x61da65` and `0x61dced`; the first and last read the visual's state kit, `0x612b31` its channel
kit. The kit row of `0x612990` comes from the table `0xc0d74c` by the spell's visual: its field `+0x4` when the unit holds a precast spell `+0xc8c`, else its field `+0x14` for the spell in `+0x228` of the field block (`0x6129e4`, `0x6129e9`).

## Stopping a spell's kits

`0x614150(spell, force)` stops the effects of a unit that a spell's kits made. A state effect lives
until `force` is 1: the cancels of a cast leave it, and the removal of the aura takes it.

Addresses: `0x614150`, `0x614167`, `0x614179`, `0x614181`, `0x61419c`, `0x6141c0`, `0x6141d8`, `0x60eecd`, `0x5ff2d2`, `0x60d22b`, `0x60d2a6`, `0x60e16d`, `0x60e1b3`, `0x6e7fe8`, `0x6e7ffb`, `0x6e8de4`, `0x6e8e99`, `0x6203e0`
Confidence: verified
Proof: one
Test: none

The function walks the unit's effect list `+0xb4` by the link `+0x8c`. It takes an effect whose
`+0x18` equals the spell, skips one with flag 0x1000 unless `force` is set (`0x614179`) and always
skips one with flag 0x20 (`0x614181`). A taken effect whose model is loaded and has animation 159
plays it at once and stops when it is done (`0x6141c0`, callback `0x6141f0`); any other is stopped
(`0x6141d8`). Kind 2 sets flag 0x1000 (`0x60eecd`). Force 1 comes from the aura removal `0x5ff290`
(`0x5ff2d2`), `0x60d1c0` (`0x60d22b`, `0x60d2a6`) and `0x60e16d`, `0x60e1b3`; force 0 from the spell
go and failed messages (`0x6e7fe8`, `0x6e7ffb`, `0x6e8de4`, `0x6e8e99`) and `0x6e1c98`.

## The kit sound

A visual kit's sound, kit `+0x34`, plays once per kit play, at a position and for everyone near,
without a test of the active player. Each effect the kit makes copies the caller's flag word, and
`PlayUnitSpellVisualKit` ORs 0x400, "no kit sound", into its own word after it makes one, so only
the first effect plays the sound, in its one-time arm pass. A kit that made no effect plays the
sound at the end of `0x60edf0` instead.

Addresses: `0x60edf0`, `0x60f106`, `0x60f119`, `0x60f434`, `0x60f444`, `0x60f464`, `0x60f48f`, `0x60f4bc`, `0x61fec0`, `0x620be0`, `0x620c05`, `0x620c10`, `0x6210a1`, `0x6210b9`, `0x6210c9`, `0x4589a0`, `0x458870`
Confidence: verified
Proof: one
Test: none

`0x60f106` makes an effect with `PlantEffect` and `0x60f119` sets 0x400 in the word at `[ebp-0xc]`.
`0x620c05` sets 0x800 on an effect and `0x620c10` skips its sound when 0x400 is set or the kit's
`+0x34` is 0 or −1; the pass runs once. `0x6210a1` starts a looping sound at the effect's
position and keeps the handle at `+0x28`; `0x6210b9` plays a one-shot there (`0x458870` with −1 and
1.0). The end of `0x60edf0` tests 0x400 (`0x60f434`) and the kit's `+0x34` (`0x60f43d`): a looping
sound (`0x60f444`) makes an effect for it (`0x60f464`, `0x61fec0`), else a one-shot plays
(`0x60f4bc`).

## Kit character procedures

`0x60d7c0(spell, kit, time)` runs the kit's four procedures, kinds at `+0x3c` and four float
parameters each from `+0x4c`. Kinds 0 and 12 start a chain beam, 1 tints the model, 6 tints the
screen, 8 colours the weapon trail, 11 starts an effect, 13 pulses a colour, 14 sets a
translucency, 15 fades the unit; the rest do nothing. Tints and pulses skip a harmful spell on a
creature whose type flags have 0x40.

Addresses: `0x60d7c0`, `0x60dbfc`, `0x60dc20`, `0x60d80a`, `0x60d840`, `0x60d8df`, `0x60d972`, `0x60d9e8`, `0x60da55`, `0x60da79`, `0x60db7e`, `0x60caa0`, `0x60caf0`, `0x60cb50`, `0x60cbf0`, `0x60cd30`, `0x60d120`, `0x60d150`, `0x60d180`, `0x5ff2f0`, `0x5ff320`
Confidence: verified
Proof: one
Test: `objectlayer::recompute_render_alpha_difftest`, `objectlayer::color_fade_t_difftest`, `objectlayer::fade_model_rgb_difftest`, `objectlayer::u5_duration_field_difftest`

The switch is `0x60dc20` into `0x60dbfc`. Kind 8 (`0x60d80a`) stores `p0 | p3 << 24` at `+0xd1c`
and `p2` at `+0xd20`. Kind 1 (`0x60d840`) replaces the spell's effect on the list `+0xce0` with
one of colour `p0 | 0xff000000`; kind 13 (`0x60d8df`) calls `0x60caa0(colour, p1·1000, p2·1000)`;
kind 14 (`0x60d972`) puts an alpha `p0` effect on `+0xb50` when 0 ≤ `p0` < 1 and calls
`0x60d180`; kind 15 (`0x60d9e8`) calls `0x60caf0(slot 27's alpha · p0, p1, 500, p3, p2)`; kind
6 calls `0x6ec910`; kinds 0 and 12 call `0x6ecb90` with the channel target or the chain targets
`+0xd4c`. `0x60cb50` sets the model tint from `0x60cbf0`'s pulse or the first `+0xce0` colour.
`0x60cbf0` holds the colour, then blends it out over `+0xd00` ms. `0x60cd30` ends a fade.
`0x60d120`, `0x60d150`, `0x5ff2f0`, `0x5ff320` release the alpha and tint effects. Slices: color_fade_t_difftest `0x60cc42`-`0x60cc65`; u5_duration_field_difftest `0x60d9e8`-`0x60d9ff`.

## Channels

`UNIT_CHANNEL_SPELL` (field 144) and `UNIT_CHANNEL_OBJECT` (fields 20 and 21) drive a channel's
visual. A new channel sets bit 0x10000, keeps the active player facing the channel object through
auto-move mode 1 for a spell with AttributesEx 0x4000, and plays the visual's channel kit as kind
2, or only its procedures when the precast animation already plays. A fishing channel at a
fishing node draws the pole and casts the bobber.

Addresses: `0x612a30`, `0x612990`, `0x60d1c0`, `0x60d2f0`, `0x604b70`, `0x604bb0`, `0x605710`, `0x605780`, `0x612630`, `0x612650`
Confidence: verified
Proof: one
Test: none

`0x612990` returns the kit animation of the precast spell `+0xc8c` (visual `+0x4`) or of the
channel (visual `+0x14`). `0x60d1c0(old)` stops the old spell's kits on the unit and its channel
object, its tints and fades, the spell effect, and clears the aura slots holding it; with no
channel left it ends a mode 1 auto-move, else it starts the new visual. `0x60d2f0` handles the
object: at a game object of type 17 it draws the weapon, and with a fishing pole (subclass 20)
calls `0x612650` with the bobber GUID, which creates the spell effect `+0xb4c` once while a
model hangs at attachment 1. `0x605710(list, spell)` stores a channel
object a spell with AttributesEx 0x4 names, fills the chain array `+0xd44` from the list's end
skipping the unit (`0x605780`), and refreshes `0x60d1c0`. `0x612630` releases `+0xb4c`.

## Precast and spell visual choice

`0x60d450(spell, out)` builds the 64-byte visual a unit shows for a spell: the spell's
`SpellVisual` record, with a ranged spell's empty kits, missile and sound filled from the ranged
weapon's own visual. `0x60d040(spell, end, stop)` clears the precast spell `+0xc8c` and its bit
0x400 and, when told, ends a tracking auto-move, the chain count and the spell effect, and calls
0x48f200 for the active player.

Addresses: `0x60d450`, `0x60d040`, `0x60d020`, `0x60fc50`, `0x60d670`
Confidence: verified
Proof: one
Test: `objectlayer::anim_timer_rate_difftest`

`0x60d450` calls `0x6ec1e0`, then for Attributes 0x2 reads slot 40 of hand 2, its
`ItemDisplayInfo` `+0x28` and that `SpellVisual`, copying its `+0x4`, `+0x8`, `+0xc`, `+0x18`
(as 1 with `+0x1c`), `+0x20`, `+0x24`, `+0x28` and `+0x38` into zero fields. `0x60d040` acts
only when `+0xc8c` holds the spell; with `stop` it ends a tracking channel's auto-move.
`0x60d670` (slot 45) is a cast time: the `SpellCastTimes` base plus per-level times (slot 42's
level over 5 less the spell level `+0x70`), at least the minimum; times `UNIT_MOD_CAST_SPEED`
(field 145) unless Attributes has 0x30 or it is 1; 0x7fffffff for Attributes 0x2; 0 if not
positive. Slices: anim_timer_rate_difftest `0x60d70b`-`0x60d738`.

## Missiles

`0x60a3d0` launches a spell missile from the unit: its model is the visual's missile, the ammo's
model, or `Spells\ErrorCube.mdx`. It records caster, target, spell, speed and hit, aims at the
target's attachment or the target point, copies the hit list and reasons, and links the missile
on the unit's list `+0xac`. The cast hand's model event or the missile's own timing releases it.

Addresses: `0x60a3d0`, `0x60a478`, `0x60a526`, `0x60c940`, `0x60c9b0`, `0x60ba30`, `0x60f530`
Confidence: verified
Proof: one
Test: none

The missile is `0x61cc70`'s object: `+0x18` spell, `+0x1c` the spell's `+0x94` speed, `+0x38` bit 1
for a hit and bit 4 for a ranged spell from a bow, crossbow or thrown weapon, `+0x3c` the
attachment from `0x860a18` by the visual's `+0x24` (`0x60a526`), `+0x5c` the GUIDs and `+0x6c`
the reasons. A hidden rifle shot sets the model's alpha to 0. `0x60c9b0` releases from the
`$CSL`, `$CSR` or `$CST` event position, else the unit's. `0x60ba30` swaps the ammo model
`+0xd28` for a display id `+0xd2c`, nocked at attachment 35 while a bow is drawn. `0x60f530`
clears the drawn bit 0x4000, detaches the ammo and settles the ranged weapon model.
