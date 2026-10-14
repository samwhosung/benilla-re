# sound: ambience and reverb

The sounds of a place: the zone ambience loop, the world reverb, the loops near water, and the
loops placed doodads and objects register as emitters. Each lives in its own object file. All four
loop forever and are restarted, never queued.

Roots: `0x4609b0`, `0x460a40`, `0x460a60`, `0x460ab0`, `0x460ad0`, `0x460af0`, `0x460c20`, `0x460c30`, `0x460c40`, `0x460c50`, `0x45a700`, `0x45a830`, `0x45a8e0`, `0x45a900`, `0x462a40`, `0x462af0`, `0x462b40`, `0x462de0`, `0x462e10`, `0x461980`, `0x461990`, `0x461a20`, `0x461d80`, `0x461f80`, `0x462000`, `0x462120`

## Zone ambience choice

One ambience plays at a time. Its sound is, in order: the SoundEntries row named
`Ghost (DONOTRENAME)` while the player is a ghost; `Underwater (DONOTRENAME)` while the camera is
submerged; the weather message's sound when there is one and the area does not override it; else
the area's SoundAmbience day or night sound. The area change passes the SoundAmbience id with the
override flag: 1 from the zone text resolver, 0 from the zone change report.

Addresses: `0x4609b0`, `0x4609f8`, `0x460a19`, `0x460bd0`, `0x460bd9`, `0x460be8`, `0x460bee`, `0x460bf7`, `0x460c10`, `0x460c50`, `0x460c64`, `0x460c6a`, `0xb06d48`, `0xb06d4c`, `0xb06d20`, `0xb06d44`, `0xb05fbc`
Confidence: verified
Proof: one
Test: none

`0x4609b0` walks SoundEntries rows from the last and keeps the ids of the two names. `0x460bd0`
tests `[0x835a48] > 0`, then `[0x835a44] > 0`, then the weather sound with `[0xb06d44]` clear, and
reads row `+4 + 4·night`. `0x460c50` stores the row pointer (0 for an id out of range) and the
flag, then refreshes with a fade. The callers pass 1 at `0x67e7d4` (`UpdateIndoorArea`) and 0 at
`0x67e919` (`UpdateOutdoorArea`).

## Zone ambience changes

A refresh compares the chosen sound with the playing one and does nothing when they match. A
different sound fades the old loop out over 5.0 s and starts the new one silent, fading to the
definition's volume over 5.0 s. A submerged change swaps at once: the old loop is destroyed and the
new one starts at full volume. The ghost, day-night and weather changes refresh with the fade. With
EnableAmbience off nothing starts; turning it off destroys the loop and silences the water sounds,
turning it on starts the chosen sound at once.

Addresses: `0x460b00`, `0x460b15`, `0x460b2a`, `0x460b31`, `0x460b49`, `0x460b54`, `0x460b76`, `0x460b8c`, `0x460b9d`, `0x460baa`, `0x460bc0`, `0x460af0`, `0x460c20`, `0x460c30`, `0x460c40`, `0x460ca0`, `0x460cc0`, `0x460cd8`, `0x460cdd`, `0x836424`, `0xb06d3c`, `0xb06d40`
Confidence: verified
Proof: one
Test: none

`0x460b00(now)`: `cmp [0xb06d40],esi; je` keeps; `Sound::Stop` or `Stop2(5.0)`; the new loop is
`0x4588f0(id)` (a 2D loop with flag 0x8); without `now`, `SetVolume2(0)` then `0x7a57b0(5.0,
volume)`. `0x460af0` passes 1, the other three 0. `0x460ca0(off)` stores `[0x836424]`, calls
`0x462e40`, then destroys or refreshes with 1. `0x460c90` (is it off) is unreached.

## World reverb

The world reverb is a SoundProviderPreferences row: the zone code passes two ids, one used while
the camera is submerged and one otherwise. A missing row applies a fixed
default; a found row is copied into FMOD's reverb properties and applied only while SoundReverb is
on. Turning SoundReverb off applies the default. The default has environment 0, size 7.5, diffusion
1.0, room and room HF −10000, room LF 0, decay 1.0 s with ratios 1.0, reflections −2602 at 0.007 s,
reverb 200 at 0.011 s, echo and modulation time 0.25, air absorption −5.0, references 5000 and
250, and flags 0x33f.

Addresses: `0x45a700`, `0x45a720`, `0x45a732`, `0x45a73b`, `0x45a752`, `0x45a75b`, `0x45a76a`, `0x45a790`, `0x45a830`, `0x45a780`, `0x45a8e0`, `0x45a900`, `0xb06028`, `0xb0602c`
Confidence: verified
Proof: one
Test: none

`0x45a700(surface, underwater)` stores `0xb06028` and `0xb0602c`. `0x45a732` takes the underwater
id when `[0x835a44] > 0`; a negative id, one past the id map or a null row goes to `0x45a830`. The
copy `0x45a790` maps row `+0xc, +0x14, +0x18, +0x1c, +0x20, +0x40, +0x10, +0x24, +0x44, +0x28, +0x2c`
to the first eleven values, `+0x30, +0x34` to reverb and its delay, `+0x48` to `+0x54` to echo and
modulation, `+0x3c, +0x58, +0x5c, +0x38` to the next four and `+0x8` to the flags, zeroing the pans,
diffusion and density. Both paths end in `0x7a5fa0`. The callers are `0x67e7c5` and `0x67e90a`.

## Water sounds

Each frame, while MapWaterSounds is on, the camera is not submerged and the player is in the world,
the terrain reports the nearest liquid within 9.0 yards in twelve slots, one per liquid class (0
to 3) and speed (0 to 2). Class 1 is ignored indoors. The classes are served in order and at most
two play; each plays the SoundWaterType sound of its lowest speed present as a 3D loop, fading in over 5.0 s to level 255 (volume 1.0, not the definition's), and fading out
over 5.0 s when its class goes away, changes speed, or is past the first two.

Addresses: `0x462b40`, `0x462b50`, `0x462b6d`, `0x462ba1`, `0x462bdb`, `0x462c26`, `0x462c32`, `0x462c58`, `0x462da0`, `0x462daa`, `0x462db5`, `0x4627c0`, `0x4627ef`, `0x46280a`, `0x462830`, `0x4628ce`, `0x4628fd`, `0x46293f`, `0x462780`, `0x4627a4`, `0x462a40`, `0x462a7f`, `0x462aa3`, `0xb2306c`, `0xb230b8`, `0x803560`, `0x80355c`
Confidence: verified
Proof: one
Test: none

`FindNearestLiquid` gets the player's world node, 9.0 (`0x41100000`) and the two twelve-slot
arrays; slot `i` is class `i & 3` and speed `(i >> 2) & 3`. `0x462a40` fills group `type` speed
`(col2 >> 2) & 3` with the SoundEntries row of column 3. `0x462da0` stops a group once two play. The
open is `Sound::Play3DLooped(0, file, 8, 0, 1)` with `SetVolume2(0)` and `0x7a5730(5.0, 255)`.

## Water sound placement

A water loop is placed at the player plus an offset that moves toward the liquid's reported offset
by at most 1/6 yard per frame, snapping when within 1/6. A liquid closer than 5.8926 yards puts the
target straight above the player at that height instead. Going under water destroys the loops at
once; coming back up starts them at full volume without the fade, once.

Addresses: `0x462960`, `0x4629a1`, `0x4629f6`, `0x462c65`, `0x462ca6`, `0x462cb5`, `0x462d4a`, `0x462e10`, `0x462e2e`, `0x462e37`, `0xb2326c`, `0x803568`, `0x7ff9d0`, `0x803550`
Confidence: verified
Proof: one
Test: none

The snap test is `d² ≤ 1/36` (`0x7ff9d0`, `test ah,0x41; jp`); the step is `min(√d², [0x803568])`
along the unit offset. The radius is `√(2 · 4.1666665²)`, computed once (`0x462c65`). `0x462e10`
clears all four groups at once on submerging and sets bit 0x2 of `0xb2326c`; surfacing with 0x2
set adds bit 0x1, which skips `SetVolume2(0)` and the fade until the next frame clears it.

## Water sound switches

`MapWaterSounds` (default 1, category 5) destroys the four loops when set to 0. EnableAmbience off
destroys them and keeps them from starting until it is on again. World teardown destroys them and
unregisters the frame handler.

Addresses: `0x462af0`, `0x462b10`, `0x462e40`, `0x462e4c`, `0x462881`, `0x462de0`, `0x8364e0`
Confidence: verified
Proof: one
Test: none

`0x462af0` calls `0x462b10(1)` for a value of 0. `0x462e40(off)` stores `[0x8364e0]`; `0x462881`
skips starting while it is above 0. `0x462de0` sets it to −1 and clears.

## Emitters

A placed doodad's or object's looping sound registers its position with `0x461d80(id, pos, flag)`
into one of 32 entries, one per SoundEntries id, each holding up to 256 positions. It returns the
handle `((entry + 1) << 16) | slot`, or 0 for a NaN position, id 0, no free entry or no slot, and
refuses an id whose open failed before. A full entry reuses the first slot farther from the
listener than the new position. `0x462000(handle, pos)` moves a position, `0x461f80(handle)` frees
it, the last one freeing the entry and fading its sound over 3.0 s, and `0x462120` returns a
handle's id.

Addresses: `0x461d80`, `0x461d99`, `0x461def`, `0x461e40`, `0x461e60`, `0x461e93`, `0x461ebd`, `0x461f1e`, `0x461a60`, `0x461af3`, `0x462000`, `0x4620c2`, `0x461f80`, `0x461d20`, `0x461fc0`, `0x461fdc`, `0x462120`, `0xb06dd8`
Confidence: verified
Proof: one
Test: none

An entry is 0xe10 bytes: 256 positions, a used and a moved byte per slot, the id (`+0xe00`,
negated after a failed open), the sound (`+0xe04`) and the slot it plays at (`+0xe08`). `0x461fc0`
checks the entry index but its slot test only runs for a negative slot (`0x461fdc`), so any slot
below 0x10000 passes. `0x462000` marks the slot used and moved when the position differs.

## Emitter pump

Each frame the pump walks the 32 entries in index order. Once four are sounding it fades every
later entry's sound over 3.0 s. Otherwise it serves the entry: its nearest used position to the
listener is chosen; a silent entry opens its sound as a 3D loop there (an open that fails disables
the id); a sounding entry moves to the nearest position when that changes. An entry counts as
sounding while its sound plays and is not cut off by distance.

Addresses: `0x461990`, `0x4619c0`, `0x4619cf`, `0x4619ef`, `0x461b40`, `0x461b61`, `0x461b9d`, `0x461bd7`, `0x461bfd`, `0x461c33`, `0x461c3a`, `0x461c58`, `0x461c71`, `0x461c7c`, `0x461ca0`, `0x461cf6`, `0x461980`, `0x461a20`
Confidence: verified
Proof: one
Test: none

`cmp edi,4; jl` (`0x4619c0`); `edi` grows only when `0x461b40` returns true at `0x461c85`. The open
is `Sound::Play3DLooped(0, file, 0, 0, 1)` with the volume, `0x458ed0` at the slot and the channel
reverb. `0x461ca0` keeps the first strict minimum. A cutoff of 0 is never cut
(`engine#distance-cutoff`). `0x461980` registers the pump; `0x461a20` unregisters it and destroys
every entry's sound.
