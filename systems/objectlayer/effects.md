# objectlayer: effects

`CEffect`, the effect object a unit carries: a model hung at an attachment or planted at a point,
a sound, an animation-speed override, or a timer, created by the visual kits (`auras`), by the
unit's own hard-coded visuals (loot sparkle, breath, level up), by melee blood and by the fishing
line. The kits decide what to spawn and when it ends; this file holds the object, its set-up once
its model loads, its frame tick, and its stop and drain.

Effect_C.cpp runs from `0x61f470` to `0x6210e0`, its initializers excepted (`initializers`). An
effect is 0x90 bytes, `SMemAlloc` by its creator, reference-counted, and linked on its unit's list
`+0xb4` until it stops; then it moves to the dying list `0xc4dee4` until its particles have died.
The chain beams in an effect's slots `+0x30` to `+0x38` are the spell system's (`spell/visuals`).

Roots: `0x61f490`, `0x6210e0`, `0x61f5a0`, `0x61f640`, `0x61f680`, `0x61fa10`, `0x61fae0`, `0x61fc00`, `0x61fcf0`, `0x61fdd0`, `0x61fec0`, `0x61ffa0`, `0x620090`, `0x6201d0`, `0x620300`, `0x6203e0`, `0x620520`, `0x620580`

## The effect object

| offset | member |
|---|---|
| `+0x00`, `+0x04` | the model; the fishing pole's line model |
| `+0x08`, `+0x10` | the owner's and the target's GUIDs |
| `+0x18`, `+0x1c` | the spell id and the `SpellVisualKit` record |
| `+0x20`, `+0x24` | the `SpellVisualEffectName` record and the attachment, −1 for none |
| `+0x28`, `+0x2c` | the playing sound and the flags, below |
| `+0x30` to `+0x38` | three chain beams |
| `+0x48`, `+0x54`, `+0x58` | the position, the start time and the end time |
| `+0x60` to `+0x68` | three saved animation speeds |
| `+0x80` | the world object of a planted effect |
| `+0x84`, `+0x88`, `+0x8c` | the reference count (1 at construction) and the list link |

Flags: 0x1 the kit sound loops, 0x2 planted at `+0x48`, 0x4 animation-speed override, 0x10
camera shake, 0x20 no end time, 0x100 hidden, 0x200 scaled to its target, which no creator sets, 0x400 no kit sound,
0x800 set up, 0x4000 follows its owner.

Addresses: `0x61f490`, `0x6210e0`, `0x61f520`, `0x620360`, `0x6203a0`, `0xc4dee4`
Confidence: verified
Proof: one
Test: none

`0x61f490` zeroes the object, sets `+0x24` to −1 and `+0x84` to 1. `0x6210e0` decrements `+0x84`
and at zero runs `0x61f520` and frees the block. `0x61f520` closes the model, clears the line
model's draw callback and closes it, unlinks, releases the world object, stops the sound with a
0.15 s fade and tail-jumps to `StopEffectObjectChains`. `0x620360(head)` unlinks (`0x6203a0`) and
pushes at the head; `+0x88` holds the slot that points at the effect. A creator links the effect
and adds the list's reference, a model creator only once the model exists; the caller then
releases its own, so an effect whose model failed is freed at once.

## Hard-coded effects

At start-up `0x61f5b0` finds 14 `SpellVisualEffectName` rows by name: `HARDCODED Footstep Water
Run Spray`, `… Walk Spray`, `Breath Underwater`, `Breath Cold`, `Loot Art`, `Unit Level Up`,
`Mount Poof`, `Inebriated Bubbles`, `PetLoyalty Up Base`, `… Up Head`, `… Down Base`, `… Down
Head`, `Meeting Stone Join` and `Reputation`. `0x61fae0(kind, guid, done)` hangs kind's model on
the unit at the attachment of `0x80c968`, 19 when the model lacks it; the footstep kinds 0 and 1
have none and are skipped. `0x61fc00` plants one at a point; `0x61fa10` hangs the loot art at 19.

Addresses: `0x61f5a0`, `0x61f5b0`, `0x61f470`, `0x61fae0`, `0x61fc00`, `0x61fa10`, `0x8617b8`, `0xc4df30`, `0x80c968`
Confidence: verified
Proof: one
Test: none

`0x61f5b0`, reached through the thunk `0x61f5a0` from 0x402ad0, zeroes the 14 slots `0xc4df30`
and stores each row's id whose name `+0x04` equals an entry of `0x8617b8` (`SStrCmp`). `0x61f470`
returns the record of a slot. `0x80c968` holds 37, 37, 17, 17, 19, 19, 19, 17, then 19; 37 skips
(`0x61fb23`). `0x61fae0` sets owner and target to the GUID and flags 0; `0x61fc00(kind, x, y, z,
guid, flags, done)` sets `+0x48` and flags `| 0x2`. `0x61fa10(unit, done)` takes slot 4 with flags
0x8 and frees the effect itself when the model fails. `0x61fae0` first stops the unit's effect of
the same record at the same attachment (`0x6208e0`); each creates the model in the unit model's
scene, sets the done callback `done` and the event callback `0x61f6f0`, and links on `+0xb4`.

## Adding an effect

The kits create effects through six entries. `CEffect::AddEffect` (`0x61fdd0`) hangs a model at
an attachment, replacing one of the same name there; `0x61fcf0` plants one at a point with a start
time. `0x61fec0` makes a sound at a point or on the unit, `0x61ffa0` and `0x620090` make bare
effects that hold a spell and a kit, and `0x6201d0` overrides the unit's animation speed until its
end time, saving the old speeds.

Addresses: `0x61fdd0`, `0x61fe14`, `0x61fcf0`, `0x61fec0`, `0x61ffa0`, `0x620090`, `0x6201d0`, `0x6208e0`
Confidence: verified
Proof: one
Test: none

`0x61fdd0(attachment, spell, kit, record, owner, target, flags, done)` links on the target unit; an
attachment of −1 prints `CEffect::AddEffect called with ATTACH_NONE` (`0x61fe14`) and goes on.
`0x61fcf0(x, y, z, start, spell, kit, record, guid, flags, done)` stores `+0x54` and flags
`| 0x2`. `0x61fec0(sound, end, spell, point, unit)` plays through 0x4589a0 at the point, or at the
unit attached to its GUID through 0x7a57e0, flags `| 0x800`. `0x61ffa0(spell, kit, guid, flags)`
and `0x620090(end, spell, kit, guid)` set flags 0x800 and an end time. `0x6201d0(end, speed, spell,
kit, guid)` saves and sets key bone −1 of the mount `+0xdc` into `+0x60`, key bone 4 of the model
when present into `+0x64` and its key bone −1 into `+0x68` (1.0 for an unloaded model) and sets
flag 0x4. `0x6208e0(unit, record, attachment)` stops the unit's effects with the same record id at
the same attachment, unless the attachment is −1.

## No cap on effects

Nothing bounds how many effects exist. Each creator resolves its unit, creates the model and
links the effect with no count, distance or visibility test; the one thinning is the replace of an
effect with the same record at the same attachment. How many particles an effect's emitters keep
is the models' (`models/particles`), and the `spellEffectLevel` option writes the particle
density (`camera`) rather than refusing effects.

Addresses: `0x61fdd0`, `0x61fcf0`, `0x61fec0`, `0x61ffa0`, `0x620090`, `0x6201d0`, `0x620360`, `0x6208e0`
Confidence: verified
Proof: one
Test: none

The creators' only calls are `ClntObjMgrObjectPtr`, `StopUnitEffectsAt` (`0x6208e0`, from
`0x61fdd0` and `0x61fcf0`), the scene and model calls, the sound calls of `0x61fec0`, the key-bone
speed calls of `0x6201d0` and `CEffect::Link` (`0x620360`), which only unlinks and pushes at the
head.

## Setting up once loaded

An effect with a model is set up once, on the first frame its model is loaded. A planted effect
gets a world object at its point, turned by its owner's facing and scaled by its owner's scale,
and shows from its start time.
An attached one must find its attachment on its target's model; flag 0x200 would scale it to the
target's size over its own, but no creator sets it. Then the kit's camera shake plays at the effect, and the kit's sound
plays once at the effect or, with flag 0x1, loops on the object.

Addresses: `0x620be0`, `0x620c05`, `0x620c35`, `0x620c86`, `0x620df5`, `0x620e16`, `0x620e43`, `0x620f5f`, `0x62102e`, `0x621039`, `0x620a90`, `0x620930`, `0x6209f0`
Confidence: verified
Proof: one
Test: `objectlayer::diff_vec3_mag2`, `objectlayer::f2_attach_scale_div_difftest`

`0x620c05` sets 0x800. `0x620c35` takes the target, else the owner. `0x620c86`: flag 0x2 places
the model by `0x620a90(owner)`, `Translate(+0x48)·Rotate(facing)·Scale(scale)`, from vtable `+0x18` and
`+0x1c`, with a scale at or below 0 taken as 1, creates the world object through `CWorld::CreateLightObjectA` with the callback
0x481510 and sets animated and visible to `now > +0x54`. `0x620df5`: flag 0x10 and kit `+0x38` above
0 call `PlaySpellEffectCameraShakes`. A missing attachment prints 0x861ac4 (`0x62107a`). `0x620e43`:
the size is animation 0's radius from `GetSequenceInfo`, else height `+0x1c`·`+0xbc`, else the
bounding-sphere radius; the effect's own size uses its radius, then 1.0 (`0x620f5f`). The flags reach `+0x2c`
from the creators' arguments and constants; no constant with bit 0x200 appears in the creators or
in their callers 0x60edf0, 0x60ed40, 0x60d7c0, 0x625010, 0x5fbf70 and 0x6e7a70. `0x62102e`:
the sound is kit `+0x34`, skipped for flag 0x400. `0x620930` checks each matrix for NaN entries and
an empty first row, and `0x6209f0` formats `*** trouble in facing` into a buffer it then drops. Slices: diff_vec3_mag2 `0x62096e`-`0x62099b`; f2_attach_scale_div_difftest `0x620f21`-`0x620fe7`.

## The frame tick

The unit's update runs each effect on its list: it sets up an effect not yet set up, makes one with
flag 0x4000 follow its owner when the owner has moved, restores the animation speeds and stops an
override at its end time, and stops any other effect at its end time unless it has flag 0x20. A
planted effect's world-object callback also shows the model when its start time passes.

Addresses: `0x620580`, `0x620592`, `0x6205a5`, `0x62066c`, `0x62075c`, `0x620834`, `0x620884`, `0x620520`, `0x608046`
Confidence: verified
Proof: one
Test: `objectlayer::f2_effect_track_reresolve_difftest`

`CGUnit_C::Update` calls `0x620580` for each effect on `+0xb4` (`0x608046`). `0x620592 test ah,9`
runs `0x620be0` when neither 0x100 nor 0x800 is set. `0x6205a5`: 0x4000 rebuilds the matrix at the
owner's position and applies it with new bounds only when its translation moved by more than 0.001
squared (`0x62066c`), else restores `+0x48`. `0x62075c`: 0x4 with `now − end > 0` restores each
nonzero saved speed and calls `CGUnit_C::UpdateBaseAnimation(-1)`. `0x620834` handles `+0x5c`,
which only the unreached `0x620100` sets. `0x620520` (from 0x481510) makes the model animated and
visible unless 0x100 once `now − start > 0`, clears the start, and stops the effect at its end. Slices: f2_effect_track_reresolve_difftest `0x620641`-`0x620676`.

## Stopping and draining

`0x6203e0` stops an effect: it moves it to the dying list, hides it, restores an animation-speed
override, stops its sound with a 0.15 s fade, drops its model callbacks and stops its chains. A
hidden model's emitters stop emitting, and each world frame `0x61f680` releases every dying effect
whose model is gone or has no emitter still emitting or holding a live particle, so a stopped
effect's particles finish their lives first. Shutdown releases every dying effect.

Addresses: `0x6203e0`, `0x6203ee`, `0x6203f9`, `0x620500`, `0x61f680`, `0x61f640`, `0xc4dec8`, `0x7135c0`, `0x7b5f60`, `0x717ecf`
Confidence: verified
Proof: one
Test: none

`0x6203ee` sets 0x900 after linking on `0xc4dee4`. The test is `CM2Model::HasLiveParticles`,
`+0x3d8`, which `CM2Model::Animate` rebuilds per emitter from its emitting state `+0x160` or
`CParticleEmitter2::HasLiveParticles` (`0x717ecf`), a live count `+0x64` here or in a child. `0x6203f9`: flag 0x4 restores the speeds of a
live owner and refreshes its animation. The callbacks are cleared, the model hidden;
`StopEffectObjectChains` (`0x6208b0`) then stops the three chain beams of
`+0x30` to `+0x38` (`spell/visuals`) and clears the slots. `0x620500` adds a reference only when the
effect is not linked. Between the link and it, `0x5fd9e0` (`0x6204aa`) reaches by direct calls
the releases of `0x5ff320` (list `+0xce0`), `0x60d150` (list `+0xb50`), `0x60d7c0`, `0x60ed40`,
`0x60edf0` and `0x612630` (`+0xb4c`), and `0x614120`, which unlinks the unit's `+0xb4` list, but
neither `0x61f680` nor `0x61f640`, the only walks of the dying list. Releasing a neighbour there
only moves this effect's `+0x88` to another nonzero slot, so it stays linked and the increment
does not run. `0x61f680` (from the world frame,
0x48324e) also walks `0xc4dec8`, releasing effects past `+0x54`; nothing links an effect there: the
address appears in the image only at `0x61f661`, `0x61f677` and `0x61f683`, all loads, against
`0xc4dee4`'s push at `0x6203e3`. `0x61f640` releases both lists.

## Model events

An effect's model events reach `0x61f6f0`: `$SND` plays the event's sound at its position and
`$HIT` plays the wound animation of the effect's target. Both need the target to be a unit.

Addresses: `0x61f6f0`, `0x61f716`, `0x61f744`
Confidence: verified
Proof: one
Test: none

The target `+0x10` resolves with mask 8 first (`0x61f716`). The tags compare as dwords, 0x444e5324
and 0x54494824. `0x61f744` calls `PlaySoundAtArgs(sound, position, -1, 1.0)` with the event's
second and third arguments; `$HIT` calls `CGUnit_C::PlayWoundAnimation(0)` for a unit (type bit 3).

## The fishing line

`0x620300(unit, bobber)` takes the model hanging at the unit model's attachment 1, the fishing
pole, and draws through it a line from its `$CCH` event to the bobber: 65 points from the event to a
point half the bobber's height above it, sagging by `0.5·sin(πt)` yards at `t` along it.

Addresses: `0x620300`, `0x61f780`, `0x61f7ea`, `0x61f82e`, `0x61f90e`, `0x61f941`, `0x61f991`, `0x61f9ce`
Confidence: verified
Proof: one
Test: `objectlayer::effect_beam_trail_difftest`, `objectlayer::effect_beam_trail_special_values_difftest`

`0x620300` stores `CM2Model::FindChild(1)` with a reference at `+0x04`, sets its draw callback to
`0x61f780` with the effect, and stores the owner and bobber GUIDs. `0x61f780` needs the model to
have the event 0x48434324 and the bobber to resolve; `0x61f7ea` adds `height·+0xbc·0.5` to the
bobber's z. `0x61f82e`: the colour channels from `+0x5c`, `+0x58` and `+0x54` of the record the
model passes are 0 at or below 0, 255 at or above 1, else `ftol(c·255 + 0.5)`, alpha 255; a
device with caps `+0x1c` of 1 swaps the byte order (`0x61f90e`). `0x61f941` steps `i/64` (0.015625)
for `i` from 0 to 64 and `0x61f991` adds `sin(t·π)·(−0.5)` to z. The 65 vertices, 0x10 bytes each
with no texture coordinates, go to a locked buffer; `0x61f9ce` turns render state 0xe off through
`GxRsSet`, and 0x58a810 draws them as 64 joined segments, primitive type 2. Slices: effect_beam_trail_difftest, effect_beam_trail_special_values_difftest `0x61f7ea`-`0x61f9ab`.

## Effect model draw state

An effect model draws like any model in the scene's lists. Of the effect object file, only the
fishing line draw (`0x61f780`) calls the render state setter, the state save or a draw; no other
effect function sets a state, a bias or a draw. The model lists begin each pass with the depth
function (id 0x11), the depth bias (id 0), lighting and fog all 0 (`0x70b360`), and the per-batch
material sets cull, depth test and depth write from the material flags alone (`models/render`). No
function of the effect file calls a terrain query, so a base-attached effect rides its unit's
origin and nothing in the effect code lifts it off a slope.

Addresses: `0x61f780`, `0x61f9c2`, `0x61f9ce`, `0x61f9fb`, `0x70b360`, `0x70b3f6`, `0x70b406`, `0x707350`, `0x671380`, `0x6717d0`, `0x620cf3`, `0x620741`, `0x620dca`, `0x6d7330`
Confidence: verified
Proof: one
Test: none

The census is every call of `0x589e60`, `0x589f40`, `0x589f50`, `0x58a810` and `0x58a830` between
`0x61f470` and `0x6210e0`: three sites, all in `0x61f780`. The callees of that range include no
terrain query. Its five model creators call `0x707350` (`CreateModelFromPath`); `0x671380` and
`0x6717d0` there create a planted effect's map entity (`0x671380` takes one from the pool `0x6a0930`; no model is pushed) and set its bounds (`0x620cf3`,
`0x620741`, `0x620dca`). The calls that set the depth bias (id 0) with a literal id, other than
the list set-up's 0 (`0x70b406`), are in the console caret, the foam, the footprints, the query
cells and the decal projector `0x6d7480`. `0x6d7330` has five callers (`0x48395a`, `0x5d7105`, `0x608f28`,
`0x611575`, `0x6d8047`), none an effect, spell or missile function.

## Unreached effect code

`0x620010` and `0x620100` link a bare effect with times; `0x620100` also sets a unit's `+0x94` and
`+0xa0`. Nothing calls either (`overview`, Unreached functions).

Addresses: `0x620010`, `0x620100`
Confidence: verified
Proof: one
Test: none

Read at the bytes; the census is the overview's.
