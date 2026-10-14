# objectlayer: dynamic objects

`CGDynamicObject_C`, the object type 6: the server's ground area of a spell, such as Blizzard or
Flamestrike. It has its own model, named by the spell's visual, a particle emitter and a looping
sound from the spell's kit, and a far-sight start for the caster's own area. The code is
DynamicObject_C.cpp, `0x5d5100` to `0x5d5f50`. The emitter's creation is in `spell`
(visuals).

Roots: `0x5d5140`, `0x5d5420`, `0x5d52f0`, `0x5d5890`, `0x5d51e0`

## Members and fields

A dynamic object's field block starts at field 6: the caster GUID `+0x0`, the bytes `+0x8`, the
spell id `+0xc`, the radius `+0x10`, the position `+0x14`, and the facing `+0x20`. The object adds
ten watcher lists of 12 bytes from `+0x118` and the members `+0x190` (animation flags), `+0x194`
(the scale, 1.0), `+0x198` (the emitter) and `+0x19c` (the sound).

Addresses: `0x5d5140`, `0x5d5196`, `0x5d51e0`, `0x5d5210`, `0x5d5700`, `0x5d5f10`, `0x5d5f40`, `0x5d5f50`, `0x5d56d0`
Confidence: verified
Proof: one
Test: none

`0x5d5140(fields, movement)` runs the base constructor, empties the ten lists, zeroes `+0x198` and
`+0x19c`, stores the vtable `0x80aa80` (`0x5d5196`), copies the movement record's position at
`+0x24` to field offset `0x14` and its facing at `+0x30` to `0x20`, clears bits 0 and 1 of `+0x190`
and sets `+0x194` to 1.0. Slot 5 (`0x5d5f10`) returns the position, slot 6 (`0x5d5f40`) the
facing, slot 7 (`0x5d5f50`) `+0x194`. The destructor (slot 0, `0x5d51e0`, body `0x5d5210`) stops the
area kit, empties the lists and runs the base destructor. `0x5d56d0` stops the sound with
`Sound::Stop2` and a 3.0 fade, and clears `+0x19c`.

## Creation

The create hook `0x5d5420` runs the base `Show`, then, if there is a model, sets the model event
callback `0x5d5540` and the done callback `0x5d5580`, both keyed by the object's GUID, and starts the
area kit (`0x5d55c0`). When the bytes field's low byte is 2, the caster is the active player and
the player's far-sight GUID is this object's, it starts far sight on the object.

Addresses: `0x5d5420`, `0x5d5439`, `0x5d546f`, `0x5d5495`, `0x5d549c`, `0x5d54a7`, `0x5d54b6`, `0x5d550e`, `0x5d5520`, `0x5d552d`, `0x5ee270`, `0x5d55c0`, `0x613af0`
Confidence: verified
Proof: one
Test: none

`0x5d546f` and `0x5d5495` set the callbacks. `0x5d549c` calls `0x5d55c0`. `0x5d54a7` compares the byte
at `+0x8` with 2 and `0x5d54b6` the caster with `ClntObjMgrGetActivePlayer`; `0x5d550e` reads the
far-sight GUID at `[player+0xe68]+0x830`, `0x5d5520` compares it, and `0x5d552d` calls
`CGPlayer_C::StartFarSight(player, object)`. The reactivation slot 2 (`0x5d5730`) runs
the base reactivation, makes the node if a model exists and starts the kit again; the
deactivation slot 1 (`0x5d5720`) runs the base deactivation and stops the kit.

## The model

Slot 10 (`0x5d5890`) names the object's model. The spell id selects the spell row, the spell's
`SpellVisual` row must have a nonzero field at `+0x2c`, and the `SpellVisualEffectName` row its
`+0x30` indexes gives the model path at `+0x8`. Each failure prints a message (`NOSPELLIDFOUND`,
`SPELLVISUALIDNOTFOUND`, `SPELLEFFECTNOAREAEFFECT`, `SPELLEFFECTIDNOTFOUND`, and
`NOOBJECTFILENAME|%d|Dynamic` from the slot) and gives no path.

Addresses: `0x5d5890`, `0x5d57c0`, `0x5d57e6`, `0x5d57fd`, `0x5d582f`, `0x5d5853`, `0x5d586e`, `0x6ec1e0`
Confidence: verified
Proof: one
Test: none

`0x5d57c0` bounds the spell id against the spell store, calls `GetSpellVisualRecord`
(`0x5d57fd`), tests `[visual+0x2c]` (`0x5d582f`) and bounds `[visual+0x30]` against the effect-name
store (`0x5d5853`); `0x5d5890` returns `[record+0x8]`.

## The loaded model

Slot 13 (`0x5d52f0`) runs when the model has loaded. It runs the base hook, then records in `+0x190`
whether the model has animation 0 (bit 0) and animation 158 (bit 1). Unless the bytes field is 1 or
2, it sets the scale `+0x194` to the radius field over the model's bounding-sphere radius, or over
the effect-name row's `+0x10` when the sphere is not above 0.001. A model with animation 158 and
none for 0 plays 158.

Addresses: `0x5d52f0`, `0x5d5312`, `0x5d5345`, `0x5d5365`, `0x5d5368`, `0x5d536c`, `0x5d5380`, `0x5d53a9`, `0x5d53cf`, `0x5d53d2`, `0x5d53d8`, `0x5d540a`, `0x711960`, `0x713680`, `0x801608`
Confidence: verified
Proof: one
Test: none

`0x5d5312` and `0x5d5345` call `HasAnimation` for 0 and 0x9e and store the bits. `0x5d5368` and
`0x5d536c` test the bytes field against 2 and 1, `0x5d5380` calls `GetBoundingSphere`, and the
radius at `[ebp-4]` divides the field when it is above the double at `0x801608`. `0x5d53a9` calls
`0x5d57c0` for the fallback record, divides `[field+0x10]` by `[record+0x10]` when that is above 0
(`0x5d53cf`), and `0x5d53d2` stores it. `0x5d53d8` to `0x5d540a` play 158 when only bit 1 is set.

## Model events and looping

The event callback `0x5d5540` resolves the GUID with mask 0x40 and calls `0x5d58c0`. The model
tag `$SND` plays the sound whose id is the event's argument at the object's position at full volume;
`$SHK` adds each of the three shakes of the camera-shake row the argument names. Any other tag prints a
message. The done callback `0x5d5580` acts on a natural end only: it restarts animation 158 when
the model has it, else animation 0.

Addresses: `0x5d5540`, `0x5d5580`, `0x5d5586`, `0x5d58c0`, `0x5d58e1`, `0x5d58e9`, `0x5d5924`, `0x5d592b`, `0x5d5947`, `0x5d5950`, `0x5d5999`, `0x5d59bb`, `0x5d5976`, `0x5d5993`, `0x458870`, `0x511d40`, `0x7121a0`, `0x4818f0`
Confidence: verified
Proof: one
Test: none

`0x5d58e1` compares the tag with 0x444e5324, `0x5d58e9` with 0x4b485324 (little-endian `$SND`
and `$SHK`). The shake arm loops three times, reading dwords from the row's `+0x4` and calling
`CameraAddShake(camera, id, scale)` (`0x5d592b`); the sound arm calls `PlaySoundAtArgs`
(`0x5d5947`). `0x5d5586` returns without acting unless the done callback's second argument is 0.
`0x5d5950` tests bit 1 of `+0x190`; with it set `0x5d5999` plays 158 (`0x5d59bb`), else animation 0
plays (`0x5d5976`, `0x5d5993`), each on the second model when there is one, else the body.
