# sound: music

Six music slots, each one sound object: the zone music `0xb06cc4`, the zone intro `0xb06dc8`, the
server's music `0xb06cd0`, the Lua music `0xb06ccc`, the glue music `0xb06038` and the credits
music `0xb06098`. All carry flag 0x2, so they take the MusicVolume factor and ignore the
sound-effects switch, and none is 3D. The zone music, its schedule, the server and Lua slots and
the pause are one file; the zone intro, and the glue and credits music, are two more.

Roots: `0x45ffc0`, `0x460040`, `0x4602e0`, `0x460370`, `0x4603b0`, `0x460410`, `0x460420`, `0x460450`, `0x460520`, `0x461120`, `0x461170`, `0x4611a0`, `0x461240`, `0x461260`, `0x461480`, `0x4614a0`, `0x45aeb0`, `0x45b000`, `0x45b050`, `0x45ad90`, `0x45ae00`

## Zone music selection

`0x4602e0(id)` makes ZoneMusic row `id` the area's music when it has a day or night sound, else
none. Unless a Lua or server track plays, a playing zone track of another row, or any track when
there is no row, fades out over 4.0 s, and the schedule is set to start at once; the same row keeps
playing. The ghost's music is the row named `GhostMusic (DONOTRENAME)`, chosen instead while the
player is a ghost.

Addresses: `0x4602e0`, `0x4602f8`, `0x460306`, `0x460317`, `0x460338`, `0x46033c`, `0x460346`, `0x460352`, `0x4602c0`, `0x45ffc0`, `0x46000c`, `0xb06cc0`, `0xb06cbc`, `0xb06c6c`, `0x836400`
Confidence: verified
Proof: one
Test: none

`0x4602f8` tests row `+0x18` and `+0x1c`. The keep test is `cmp [0xb06cbc],eax; je` at `0x46033c`;
`0x460346` stores −1 in `0x836400` before the stream test, and `0x460352` is `Stop2(4.0)`.
`0x4602c0` returns `0xb06c6c` when `[0x835a48] == 1`. `0x45ffc0` registers the pump, sets the
schedule to −1 and finds the ghost row by name (`0x46000c`).

## The zone music pump

Each frame, unless music is paused or a Lua track plays: a finished server track is destroyed and
the next zone track is scheduled; a finished zone track is destroyed and the next is scheduled by
the silence rule; and with no zone track, one starts when the schedule is −1 or the frame's clock
has passed it.

Addresses: `0x460040`, `0x460045`, `0x460055`, `0x460067`, `0x460079`, `0x460088`, `0x4600ad`, `0x4600b6`, `0x4600ca`, `0x4600e9`, `0x4600ee`, `0x4600f3`
Confidence: verified
Proof: one
Test: none

The tests are `[0xb06cc8]` and `[0xb06ccc]`; a live server track (`0x7a5810`) returns. After a
server track the delay is `0x460100(30000, 60000)` ms. The clock is the frame-time payload's `+4`
(`core/loop#frame-time`), compared unsigned with `jbe` (`0x4600ee`).

## Zone music tracks

A zone track is the day or night sound of the area's row (or the ghost row): the definition's next
file in order, opened as a one-shot on bus 0 with flags 0x6 (music, kept), held, at the
definition's volume with no variation or pitch, then unpaused.

Addresses: `0x460240`, `0x460252`, `0x460269`, `0x460276`, `0x46028a`, `0x460297`, `0x46029d`, `0x4602ad`
Confidence: verified
Proof: one
Test: none

`0x460252` reads row `+0x18 + 4·night`. The pick is `0x45bb70(def, 1, 0, −1)`, the sequential mode
(`entries#picking-a-file`), so a row's files play in turn from file 0 for the life of the process. The open
is `Sound::Play2D(0, file, 6, 1)`, then `Sound::SetVolume2(def+0x78)`.

## Silence between tracks

When a zone track ends the next starts at once with SoundZoneMusicNoDelay on, 6 s later when no row
is recorded (never, since the ended track's row still is), and otherwise after `0x460100(min, max)` ms with the row's day or night silence
interval. `0x460100(lo, hi)` returns `hi` when `hi ≤ lo`, else `lo + MulHigh32(hi − lo, r)` with
`r` the first value of a generator seeded with 0 on each call: always 0x16be8aa9, so the delay is
always `lo + 0.0888·(hi − lo)` and never random. After a server track that is 32665 ms.

Addresses: `0x4601f0`, `0x460205`, `0x460213`, `0x46021e`, `0x460100`, `0x460110`, `0x46011e`, `0x4601de`, `0x44ea80`
Confidence: verified
Proof: one
Test: none

`0x46021e` passes row `+0x8 + 4·night` (min) and `+0x10 + 4·night` (max). `0x46011e` calls
`NTempest::CRndSeed::SetSeed(0)` on a stack state, advances it once inline (`random#state-advance`)
and calls `MulHigh32`; from seed 0 the lanes become 0xb8, 0xc8, 0xd4, 0xd8 and the sum is
0x16be8aa9 for this exe's table `0x802700`.

## Zone intro music

`0x461260(music, intro)` is the area change: it records the area's ZoneMusic id, tries the
ZoneIntroMusicTable row, and selects the zone music only when no intro plays. An intro plays its
sound once on bus 0 unless music is paused, the player is a ghost, or its minimum delay (minutes,
`+0x10`) since it last played has not passed. A playing intro stays when the new row's priority
(`+0x0c`) is below its own; otherwise it fades over 3.0 s. When the intro ends the frame handler
hands back to the zone music.

Addresses: `0x461260`, `0x46126d`, `0x461290`, `0x4612c9`, `0x4612d6`, `0x461310`, `0x461336`, `0x461356`, `0x461375`, `0x4613b0`, `0x4613d3`, `0x4613d9`, `0x461440`, `0x461465`, `0x461170`, `0x4611a0`, `0x461480`, `0x461240`, `0x4614a0`, `0xb06d9c`, `0x8364b0`, `0xb06dc8`
Confidence: verified
Proof: one
Test: none

`0x461440` allows a row id below the time array's count whose time is 0 or `≤ now − 60000·delay`.
`0x461310` tests bit 0x10 of the flags at `[player+0xe68]+8` and stops the intro. The open is
`Sound::Play2D(0, file, 6, 1)` with `0x458da0`. `0x461170` destroys a finished intro and reselects
`[0xb06dcc]`. `0x4612d6` refuses every intro while the byte `0xb06d9c` is set, but its only writer,
`0x4611a0`, stores 0.

## Lua music

`PlayMusic(name)` and `StopMusic()` both run `0x460450(name)`. It fades the server track over 3.0 s
and the Lua track over 4.0 s, empties both slots, and schedules the zone music 6 s later. With a
name it opens the file as an endless loop on bus 0 with flag 0x2 at volume 1.0; if that works it
fades the zone track over 4.0 s and starts the loop unless music is paused. A file that does not
open leaves the zone track playing. While the Lua slot holds a sound the zone pump does nothing.

Addresses: `0x460450`, `0x46045d`, `0x46047b`, `0x46048f`, `0x4604a0`, `0x4604b1`, `0x4604bd`, `0x4604c6`, `0x4604de`, `0x4604f2`, `0x460503`, `0xb06ccc`
Confidence: verified
Proof: one
Test: none

`0x4604a0` returns for a null or empty name. `0x4604b1` is `Sound::Play2DLooped(0, name, 2, 0, 1)`,
held; with music paused (`0x4604f2`) the loop stays held until `0x4603b0` unpauses it. The loop
never ends, so only `0x460450` or a pause touches the slot.

## Server music

The server's play-music message (opcode 0x277) passes a SoundEntries id to `0x460520`. Unless a zone
intro, a Lua track or the music pause is in the way, it plays the id with `InternalPlaySound` on
bus 0, kept, as music; on success it fades the previous server track and the zone track over 3.0 s
and keeps the new one. `StopMusic` fades it out.

Addresses: `0x460520`, `0x460523`, `0x460549`, `0x46055e`, `0x460578`, `0xb06cd0`, `0x48f849`
Confidence: verified
Proof: one
Test: none

`0x460523` calls `0x4614a0` (an intro is held). The play is `InternalPlaySound(1, 0, 0, id, −1, 1)`.
The caller is the `0x277` arm of `CGGameUI::HandlePacket` (`0x48f849`).

## Pausing music

`SndInterfacePauseZoneMusic(pause)` stores the pause flag. Pausing destroys a server track; then a
Lua track is paused or resumed in place; without one, pausing ends the zone intro and destroys the
zone track, and resuming schedules the zone music 3 s later. EnableMusic, the in-engine cinematic's
start and end, and leaving the world call it. A ghost change destroys the zone track and schedules
the music 3 s later unless a Lua or server track plays.

Addresses: `0x4603b0`, `0x4603ba`, `0x4603c5`, `0x4603e1`, `0x4603eb`, `0x4603f5`, `0x4603ff`, `0x460410`, `0x460420`, `0x460441`, `0xb06cc8`
Confidence: verified
Proof: one
Test: none

`0x4603ba` is the only writer of `0xb06cc8`; `0x460410` reads it. Callers: `0x4574a4` (CVar),
`0x48ed83` with 1 and `0x48f147` with 0 (cinematic), `0x490ba7` with 0. `0x460420` is called by
`0x458680`.

## Glue music

`SndInterfaceSetGlueMusic(name)` first fades the credits music over 1.0 s. With the CVar
`enablemusic` off it does nothing more. A null name, or a glue screen not set up or suspended, stops
the glue music over the 3.0 s default and forgets the name. An empty name restarts the remembered
name, or reapplies the volume to the playing loop. A new name is remembered (260 bytes), the old
loop stops over 3.0 s, and the file opens as an endless loop on bus 0 with flag 0x2 at volume 0.8.
`StopGlueMusic(seconds)` fades it out and empties the slot.

Addresses: `0x45aeb0`, `0x45aeb5`, `0x45aeba`, `0x45aed7`, `0x45aedf`, `0x45aef9`, `0x45af21`, `0x45af33`, `0x45af68`, `0x45af85`, `0x45af9b`, `0x45afaa`, `0x45afb8`, `0x45afe0`, `0x45b050`, `0x803248`, `0xb060a0`, `0xb41d90`, `0xb41d94`, `0xb06038`
Confidence: verified
Proof: one
Test: none

The glue state is `[0xb41d90]` set up and `[0xb41d94]` suspended. The same name with a live loop
returns at `0x45af83`. The opens are `Sound::Play2DLooped(0, name, 2, 0, 1)` and the volume
`0x3f4ccccd` (0.8). `[0x803248]` is 3.0. The Lua `StopGlueMusic` passes 1.0 and the EnableMusic
handler 0. The client's world entry passes null.

## Credits music

`0x45b000` stops the glue music over 3.0 s and the credits music over 1.0 s, then, with the CVar
`enablemusic` on, starts the seven-track credits list at track 0 and registers its frame handler,
which starts the next track (index + 1, modulo 7) when one ends. The tracks are the seven city
intro files from `0x836028`, opened on bus 0 with flags 0x6 at volume 1.0.

Addresses: `0x45b000`, `0x45b032`, `0x45b04a`, `0x45ae00`, `0x45ae0a`, `0x45ae25`, `0x45ae50`, `0x45ae59`, `0x45ae6e`, `0x45adb0`, `0x45ad90`, `0x836024`, `0x836028`, `0xb06098`
Confidence: verified
Proof: one
Test: none

`0x45ae50(n)` stores `n mod 7` and opens `[0x836028 + 4·(n mod 7)]` with `Sound::Play2D(0, file, 6,
1)`. `0x45adb0(now)` destroys the track and unregisters when `now`, else fades it over 1.0 s.
`0x45ad90` stops both at once and forgets the glue name, at exit.

## Music teardown

At world teardown `0x460370` unregisters the zone pump, forgets the area's row and destroys the zone,
Lua and server tracks; `0x461240` destroys the intro and forgets the area's id.

Addresses: `0x460370`, `0x461240`
Confidence: verified
Proof: one
Test: none

`0x460370` calls `Sound::Stop` on `0xb06cc4`, `0xb06ccc` and `0xb06cd0`.
