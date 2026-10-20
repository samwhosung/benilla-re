# sound: the interface

`SoundInterface.cpp` is the client's sound front end: its start-up and the world's, the CVars and
their handlers, the four Lua sound functions, the interface sounds by name, the item, error-speech
and emote sounds, the day-night flag and the state the other files read. The sound-definition
plays it calls are in `entries`, the object and combat sounds in `objects`.

Roots: `0x456fe0`, `0x4571b0`, `0x457770`, `0x457cc0`, `0x4576c0`, `0x4573e0`, `0x457400`, `0x457470`, `0x457490`, `0x4574d0`, `0x457500`, `0x457550`, `0x457570`, `0x457590`, `0x4578c0`, `0x4586d0`, `0x458720`, `0x458770`, `0x458780`, `0x4587f0`, `0x458810`, `0x458030`, `0x457fb0`, `0x457ff0`, `0x4580f0`, `0x458250`, `0x4592b0`, `0x459270`, `0x458f50`, `0x458f60`, `0x458650`, `0x458680`, `0x4586b0`

## Start-up

`SndInterfaceInitialize` registers the sound CVars and, unless the command-line switch with id
0x1a is set, initializes the engine with the three CVar getters, applies MasterVolume again (its
handler runs before FMOD is up), loads the sound definitions and their tables, builds the footstep
cache and resolves the named sounds. With the switch set nothing after the CVars runs: no device
and no definitions, so every later play finds no definition and does nothing. The engine's result
is not checked.

Addresses: `0x456fe0`, `0x456fea`, `0x457002`, `0x45701c`, `0x457021`, `0x457026`, `0x457035`, `0x457550`, `0x457570`, `0x457590`, `0x45ad80`, `0x45a8f0`, `0x419c10`
Confidence: verified
Proof: one
Test: none

`0x456fea` calls `0x419c10` with 0x1a and `jne` returns. `0x457002` calls `Sound::Initialize` with
`0x457550` (the CVar's integer `+0x28`), `0x457570` (its float `+0x24`) and `0x457590` (its string
`+0x20`); each returns false for an unknown name. Then `0x7a6500` with MasterVolume's float,
`0x45c060`, `0x457040`, the empty `0x45ad80` and `0x45a8f0`, and a tail jump to `0x4575b0`.

## Sound CVars

The CVars, with their defaults (category 7). The device CVars carry flag 2.

| CVar | default | handler |
|---|---|---|
| SoundBufferSize | "50" for processor class 1 (`console/hardware#processor-class`), else "100" | |
| SoundOutputSystem, SoundDriver, SoundMixer, SoundMinHardwareChannels | "-1" | |
| SoundMaxHardwareChannels, SoundSoftwareChannels | "12" | |
| SoundMixRate, SoundInitFlags, SoundMemoryCache, SoundRolloffFactor | "44100", "128", "4", "4" | |
| MusicVolume, SoundVolume, MasterVolume | "0.4", "1.0", "1.0" | set the factors |
| MasterSoundEffects, EnableMusic, SoundReverb | "1" | yes |
| EmoteSounds | "1" | |
| EnableGroupSpeech, EnableErrorSpeech, SoundListenerAtCharacter | "1" (world) | |
| SoundZoneMusicNoDelay | "0" (world) | |
| AmbienceVolume, EnableAmbience | "0.6", "1" (world) | yes |

Addresses: `0x4571b0`, `0x457520`, `0x457840`, `0x460a60`, `0x835dd4`, `0x835d38`, `0x835c6c`, `0x835c08`, `0x82e92c`, `0x82e748`, `0x82e570`, `0x8364a8`
Confidence: verified
Proof: one
Test: none

`0x4571b0` calls `CVar::Register` 18 times; the default is the second stack argument, the handler
the third, the flags the first. SoundRolloffFactor and SoundMemoryCache share `0x835c6c` ("4").
`0x457840` registers the world four and tail-calls `0x460a60` for the ambience pair. The engine
also asks for SoundDistanceFactor and SoundDopplerFactor, whose names only `0x7a48e0` references:
no code registers them, so the getter fails and the defaults stand unless a record of that name
exists (`engine#device-initialization`).

## CVar handlers

MusicVolume, SoundVolume, MasterVolume and AmbienceVolume parse a float and call the engine's
setter. SoundVolume is ignored while a server object sound holds the duck (`objects#the-duck`).
EnableMusic pauses the zone music with the inverse of its value, then starts the glue music with an
empty name when on or stops it at once when off. SoundReverb stores its byte and, when off, applies
the default reverb. MasterSoundEffects passes `value == 0` to the engine. EnableAmbience passes
`value == 0` to the ambience switch.

Addresses: `0x4573e0`, `0x457400`, `0x457430`, `0x457470`, `0x457490`, `0x4574a1`, `0x4574b3`, `0x4574c0`, `0x4574d0`, `0x4574e3`, `0x457500`, `0x460ab0`, `0x460ad0`, `0x835a4c`
Confidence: verified
Proof: one
Test: none

Each returns 1 (`mov al,1`). `0x4574a1` `sete cl` before `0x4603b0`; then `0x45aeb0` with the
empty string `0x882748` or `0x45b050(0)`. `0x4574e3` stores the byte at `0x835a4c` and calls
`0x45a830` when it is 0. `0x457500` and `0x460ab0` compute `neg; sbb al,al; inc al`.

## World start and teardown

`SndInterfaceWorldInitialize` registers the world CVars and, unless switch 0x1a is set, starts the
zone music, finds the ghost and underwater ambiences, builds the water and doodad emitters,
registers the frame handler, sizes the zone-intro times, builds the emote sounds, and sets the
night flag and the ambience. `0x457cc0` unregisters the frame handler and stops the zone music,
the water sounds, the ambience, the doodad emitters and the zone intro, frees the emote tables and
releases the server object sounds. `0x4576c0`, at exit, resets the reverb and the submerged and
ghost states, stops the glue and credits music, unloads the definitions and shuts the engine.

Addresses: `0x457770`, `0x45778d`, `0x4577a1`, `0x4577cb`, `0x457829`, `0x457cc0`, `0x457d60`, `0x4576c0`, `0x45775d`
Confidence: verified
Proof: one
Test: none

The calls are `0x45ffc0`, `0x4609b0`, `0x462a40`, `0x461980`, `EventRegister(5, 0x4578c0)`,
`0x4611a0`, `0x457ae0` and `0x460c30`; teardown `0x460370`, `0x462de0`, `0x460a40`, `0x461a20`,
`0x461240`, `0x457d60` and the slot walk; exit `0x45a8e0`, `0x45ad90`, `0x45cc80`, the footstep
cache, −1 into `0x835a48` and `0x835a44`, and the tail jump to `0x7a4a20`.

## Day and night

The frame handler recomputes the night flag each frame: day while the game clock lies in the range
`WowTime::InRange` gets for 05:30 to 21:00 (any date), night otherwise. A change refreshes the
ambience with a fade. The handler also ends a finished zone intro and runs the duck.

Addresses: `0x4578c0`, `0x4578ca`, `0x4578e5`, `0x457909`, `0x457926`, `0x457931`, `0x45793f`, `0x457947`, `0xb05f18`
Confidence: verified
Proof: one
Test: none

`WowTime::WowEncodeTime` packs minute (`edx`) and hour (first stack argument), the other fields −1:
(0, 21) and (30, 5). `InRange(clock 0xce8538, 05:30, 21:00)` true clears `esi`; `0xb05f18` holds
1 at night and indexes the day and night columns of the music and ambience rows. `InRange` takes 05:30 up
to, not including, 21:00 (`gametime/overview#time-range-test`).

## Lua sound functions

`PlaySound(name)`, `PlayMusic(name)`, `PlaySoundFile(file)` and `StopMusic()` are registered for the
glue and the game UI. `PlaySound` plays an interface sound by name. `PlayMusic` and `StopMusic` are
the Lua music slot (`music#lua-music`). `PlaySoundFile` opens the file on bus 3 (cap 2) with no
duplicate check and no definition, started at once at volume 1.0 on the Sound category, and
returns 1 or nil. A non-string, non-number argument raises the usage error.

Addresses: `0x4586d0`, `0x458720`, `0x458770`, `0x458780`, `0x4587c1`, `0x4587f0`, `0x458810`, `0x835a50`, `0x835f60`, `0x835f7c`, `0x835f98`
Confidence: verified
Proof: one
Test: none

`0x4587f0` registers the four `{name, function}` pairs at `0x835a50` through
`FrameScriptRegisterFunction`; it is called at `0x46abce` and `0x4902a6`. `0x4587c1` is
`Sound::Play2D(3, file, 0, 0)`. `StopMusic` is `0x460450(NULL)` and reads no argument.

## Interface sounds by name

`0x458030(name)` plays the interface sound of that name: nothing for a null or empty name, while
the suppression count is above 0, or while MasterSoundEffects is 0. The name is looked up,
case-insensitively, among SoundEntries rows of type 2 (`entries#interface-sound-names`) and the
row's id is played with `0x458850`. The suppression count is raised by `0x458f50` and lowered by
`0x458f60`; the UI load and the enter and leave world paths bracket themselves with the pair, so a
`PlaySound` during them is dropped.

Addresses: `0x458030`, `0x458046`, `0x458053`, `0x458075`, `0x4580b7`, `0x4580e2`, `0x458f50`, `0x458f60`, `0xb05fa0`
Confidence: verified
Proof: one
Test: none

`0x458046` reads `0xb05fa0` before the CVar lookup at `0x458053`. The hash is `SStrHashHT`; the
compare `SStrCmpI`. `0x458f50` is `inc dword [0xb05fa0]`, `0x458f60` the `dec`; their callers are
`0x48fbfa`, `0x4908d5`, `0x490a94`, `0x490bd1` and `0x49016d`, `0x490a56`, `0x490b64`, `0x490cd9`.

## Item sounds

`SndInterfacePlayItemSound(kind, item)` plays a live item's sound of kind 0, 1 or 2. Only the
wrapped-item rows label a kind; the callers pass 0 when an item goes onto the cursor and 1 when it
leaves it. A wrapped item (flag 0x8 in its fields' `+0x3c`) plays the row named `(DONOTRENAME)WrappedItemDrop`
for kind 1 and `(DONOTRENAME)WrappedItemPickup` otherwise; any other item plays column `kind + 1` of its cached ItemGroupSounds
row, or nothing without one. `0x457ff0(kind, displayId)` reaches the same row through
ItemDisplayInfo column 11.

Addresses: `0x457fb0`, `0x457fb6`, `0x457fbc`, `0x457fc6`, `0x457ff0`, `0x458008`, `0x458024`, `0xb05fa4`, `0xb05fa8`
Confidence: verified
Proof: one
Test: none

`0x457fb6` tests `[[item+0x114]+0x3c] & 8`; `0x457fbc` reads the row at `[item+0x340]` and
`0x457fc6` `[row+4·kind+4]`. `0x458008` reads ItemDisplayInfo `+0x2c` and checks it against the
ItemGroupSounds id map. All three plays tail-jump to `0x458850`.

## Error speech

`0x4580f0(race, gender)`, run when the active player arrives, fills a table of VocalUISounds rows
for that race and gender: per error 0 to 67 a normal sound, a fed-up sound and the fed-up sound's
file count. `0x458250(gender, error)` plays it on bus 1 (cap 1) when MasterSoundEffects and
EnableErrorSpeech are on. Repeats of one error play the normal sound; after four that started it
switches to the fed-up sound, one of its files per repeat in order, and back once they run out. Each
call also tries the normal sound, which the bus cap refuses while a fed-up line plays. A different
error starts over.

Addresses: `0x4580f0`, `0x458109`, `0x45815f`, `0x458176`, `0x458250`, `0x45829c`, `0x4582ad`, `0x4582f0`, `0x458307`, `0x45833d`, `0x458359`, `0xb06240`, `0xb06570`, `0xb05ee4`, `0xb05f78`, `0x835a40`
Confidence: verified
Proof: one
Test: none

The builder clears 0x264 dwords from `0xb06240` and stores rows `+0xc`/`+0x14` for gender 0 at
`0xb06240 + 12·error` and `+0x10`/`+0x18` for gender 1 at `0xb06570`, with `SOUNDDEFINITION +0x94`.
`0x458307` plays the fed-up sound with the counter as its file index, `0x45833d` the normal sound
with −1, both through `InternalPlaySound(bus 1)`. The bus cap refuses the second of the two while
the first plays. The caller is `CGGameUI::DisplayError` (`0x49677c`).

## Emote sounds

`0x457ae0` builds, per EmotesText id, a 20-entry table of EmotesTextSound sounds by race (0 to 9)
and gender (0, 1). `0x459270(emote, race, gender)` returns the sound or 0. `0x4592b0` returns a
random file index of that sound's definition, or −1, without playing it.

Addresses: `0x457ae0`, `0x457b78`, `0x457c4f`, `0x457c7f`, `0x457c97`, `0x459270`, `0x4592b0`, `0xb05ebc`, `0xb05ef0`
Confidence: verified
Proof: one
Test: none

The pointer array at `0xb05ebc` takes EmotesText's max row + 1 entries. The record array at
`0xb05ef0` grows with a granularity equal to the number of runs of equal emote ids (`0x457b78`),
counted first, so its first growth holds every record and the stored pointers stay valid. Rows with
race ≥ 10 or gender ≥ 2 are skipped (`0x457c7f`, `0x457c87`).

## Named sounds

`0x4575b0` scans SoundEntries rows for six fixed names and keeps the id of the last row with each:
`(DONOTRENAME)AbsorbGetHit` (`0x835eac`), `(DONOTRENAME)ShieldWoodImpact` (`0x835e8c`),
`(DONOTRENAME)WrappedItemPickup` (`0x835e6c`), `(DONOTRENAME)WrappedItemDrop` (`0x835e4c`),
`(DONOTRENAME)Combat Miss 1H` (`0x835e30`) and `(DONOTRENAME)Combat Miss 2H` (`0x835e14`).

Addresses: `0x4575b0`, `0xb05fb4`, `0xb05fb8`, `0xb05fa8`, `0xb05fa4`, `0xb05fb0`, `0xb05fac`, `0x835eac`, `0x835e8c`, `0x835e6c`, `0x835e4c`, `0x835e30`, `0x835e14`
Confidence: verified
Proof: one
Test: none

The compares are `SStrCmp` on the row's name `+0x8` with the length 0x7fffffff, case-sensitive and
whole-string, so a bare `ShieldWoodImpact` matches none; a match stores the row's id.

## State the other files read

`0x458650(submerged)` and `0x458680(ghost)` store a boolean at `0x835a44` and `0x835a48` when it
changes. A submerged change swaps the ambience at once, tells the water sounds and reselects the
reverb; a ghost change restarts the zone music within 3 s and refreshes the ambience with a fade.
`0x4586b0(sound)` keeps the weather message's sound and refreshes the ambience when it changes; the
world load and unload pass 0.

Addresses: `0x458650`, `0x458666`, `0x45866b`, `0x458670`, `0x458680`, `0x458696`, `0x45869b`, `0x4586b0`, `0x835a44`, `0x835a48`, `0xb05fbc`
Confidence: verified
Proof: one
Test: none

The calls are `0x460af0`, `0x462e10` and `0x45a900`; `0x460420` and `0x460c20`; `0x460c40`. The
callers are the liquid test `0x511aa9`, the player flags watcher `0x5dea3f` (bit 0x10 of the flags
at `[player+0xe68]+8`), and `0x401598`, `0x401c12` and the weather message `0x48faad`.
