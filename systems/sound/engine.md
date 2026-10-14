# sound: the engine

The sound engine file wraps the FMOD 3 library (`fmod.dll`, imported by name): the device setup,
one `Sound` object per playing stream, the lists the frame pump walks, the volume categories and
the listener. The client never loads a sample or picks a channel: every sound is an FMOD stream
opened by file name and started on whatever channel FMOD gives it. FMOD's own behaviour is the
library's; these entries state the calls the client makes and their arguments. Positions here are
in the client's world axes; the engine converts them to FMOD's at each call
(`Positions and the listener`).

Roots: `0x7a4330`, `0x7a4a20`, `0x7a4850`, `0x7a4860`, `0x7a5450`, `0x7a5490`, `0x7a5620`, `0x7a5680`, `0x7a5700`, `0x7a5a10`, `0x7a5ac0`, `0x7a5730`, `0x7a57b0`, `0x7a57e0`, `0x7a5810`, `0x7a5870`, `0x7a5b10`, `0x7a5bf0`, `0x7a5ca0`, `0x7a5dc0`, `0x7a5e40`, `0x7a5e80`, `0x7a5eb0`, `0x7a5f50`, `0x7a5fa0`, `0x7a6390`, `0x7a6400`, `0x7a6480`, `0x7a6500`, `0x7a6570`, `0x7a53c0`, `0x7a47f0`, `0x7a4810`, `0x7a4830`

## Device initialization

`Sound::Initialize(getInt, getFloat, getString)` opens `Logs\Sound.log`, refuses a second call,
requires `FSOUND_GetVersion` to equal 3.75 (else returns 13), and reads its parameters through
the getters with these defaults. A value out of range returns 14 before FMOD is set up.

| parameter | default | range |
|---|---|---|
| SoundOutputSystem | −1 | −1 to 13 |
| SoundDriver | −1 → 0 | −1 to drivers − 1 |
| SoundMixer | −1 → 4 | −1 to 9 |
| SoundBufferSize | 0 (not set) | ≥ 0 |
| SoundMin / MaxHardwareChannels | −1 (not set) / 12 | ≥ −1 |
| SoundMixRate, SoundSoftwareChannels | 44100, 12 | |
| SoundInitFlags | 0x80, then `& ~0x200 | 2` | |
| SoundDistance / Doppler / RolloffFactor | 1.0, 1.0, 4.0 | |
| SoundMemoryCache | 1 | |

Addresses: `0x7a4330`, `0x7a48e0`, `0x7a48a0`, `0x7a43b3`, `0x7a4412`, `0x7a4452`, `0x7a448d`, `0x7a44a9`, `0x7a4a00`, `0x7a47b8`, `0x80c4ec`
Confidence: verified
Proof: one
Test: none

`0x7a48e0` stores the defaults, then calls the integer getter for the integer names and the float
getter for the three factors (`ret 8`: the string getter is passed and unused). The version test is
`fcom [0x80c4ec]` (3.75), `test ah,0x44`, `jnp` to the success path, so only equality passes. Each
range test jumps to `0x7a47b8` (`mov esi,0xe`). The mixer maps −1 to 4 at `0x7a44a9`, the driver −1
to 0 at `0x7a445f`.

## FMOD set-up sequence

In order the client calls `SetOutput`, `SetDriver`, `GetDriverCaps` (bit 0 clear sets the maximum
hardware channels to 0), `SetMixer`, `SetBufferSize` when non-zero, `SetHWND` with the window,
`SetMin`/`SetMaxHardwareChannels` when not −1, `SetMemorySystem(NULL, 0, alloc, realloc, free)` with
Storm wrappers tagged `"FMod"`, the file cache with the memory cache size, `File_SetCallbacks` with
the file cache's open, close, read, seek and tell, and `Init(rate, channels, flags)`. A failing call
logs `Error: %s(%i) returned %i` and returns FMOD's error code. On success it keeps the rate, logs
the output, driver, mixer and channel counts, registers the frame handler and the focus handler,
zeroes the 13 bus counters and sets the three 3D factors.

Addresses: `0x7a4426`, `0x7a4463`, `0x7a4483`, `0x7a44b5`, `0x7a44df`, `0x7a4501`, `0x7a452c`, `0x7a4557`, `0x7a4585`, `0x7a45c6`, `0x7a45e4`, `0x7a45f5`, `0x7a462d`, `0x7a474e`, `0x7a475d`, `0x7a479d`, `0x7a479f`, `0x7a47b1`, `0x7a48a0`, `0x7a47f0`, `0x7a4810`, `0x7a4830`, `0x7a4850`, `0x7a4860`, `0x87cef0`, `0xcf553c`, `0xcf55e8`
Confidence: verified
Proof: one
Test: none

`0x7a48a0(ok, name, value)` returns 0 when `ok`, else logs and returns `FSOUND_GetError`. The
callbacks are `0x7a47f0`, `0x7a4810` and `0x7a4830` (`SMemAlloc`, `SMemReAlloc`, `SMemFree`);
`0x7a45c6` calls the os file cache `0x7a8fb0` and `0x7a45e4` passes `0x7a8770`, `0x7a8f00`,
`0x7a8ab0`, `0x7a8e70` and `0x7a8ee0`. `0x7a462d` stores the rate in `0x87cef0`. `0x7a4850` (frame
time, category 5) and `0x7a4860` (category 2) are registered at `0x7a474e`/`0x7a475d`; `0x7a479d`
zeroes 13 dwords at `0xcf553c`; `0xcf55e8` is set to 1. `setupapi.dll` is loaded for the call and
freed at the end. The client ignores the result (`interface#start-up`).

## The sound object

A `Sound` is a 0x90-byte record from a pool of 40-record blocks. It holds six list links (`+0x00`
every sound, `+0x08` fades, `+0x10` objects that move, `+0x18` panning, `+0x20` distance cutoff,
`+0x28` ended), the FMOD channel (`+0x30`, −1 when none) and stream (`+0x34`), the flags (`+0x38`),
the fade state (`+0x40` start, `+0x44` level, `+0x48` rate per ms), the position (`+0x50`) and
velocity (`+0x5c`), the object GUID it follows (`+0x68`), the cutoff distance (`+0x70`), the volume
(`+0x74`, 1.0), the distance gain (`+0x78`, 1.0), the frequency (`+0x7c`, 22050), the name hash
(`+0x80`), the bus (`+0x84`) and the pause reasons (`+0x88`).

Addresses: `0x7a4180`, `0x7a4200`, `0x7a51d0`, `0x7a3b10`, `0x7a3c00`, `0x7a3d00`, `0x7a3e00`, `0x7a3f00`, `0x7a4000`, `0x7a4100`, `0x7a4150`, `0x7a6970`, `0xcf5598`, `0xcf5604`, `0xcf5574`, `0xcf55dc`, `0xcf552c`, `0xcf55cc`, `0xcf5580`, `0xcf558c`
Confidence: verified
Proof: one
Test: none

The constructor `0x7a4180` zeroes the record, sets `+0x30` and `+0x80` to −1, `+0x74` and `+0x78` to
1.0, `+0x7c` to 0x5622 and counts the live records in `0xcf5604`. The six list heads are built by
`0x7a3b10` to `0x7a4000` with link offsets 0, 8, 0x10, 0x18, 0x20 and 0x28 at `0xcf5574`,
`0xcf55dc`, `0xcf552c`, `0xcf55cc`, `0xcf5580` and `0xcf558c`; `0x7a4100` makes the pool
(0x90 bytes, 40 per block) at `0xcf5598` and `0x7a4150` the lock `0xcf55b4`. `0x7a51d0(name, music)`
takes a record, links it last on the every-sound list and hashes the name with `SStrHash`. The
destructor `0x7a4200` stops it (`0x7a5950`) and unlinks all six links.

## Flags of a sound

The low byte of `+0x38` is the caller's: bit 0x1 asks for the duplicate check, 0x2 puts the sound
on the music category and exempts it from the sound-effects switch and the focus refusal, 0x4 keeps
the record when the stream ends, 0x8 is the ambience category and 0x10 no category. The engine
sets 0x10000 inside 0.9 of the cutoff, 0x20000 release when faded, 0x40000 position set, 0x80000
volume set, 0x100000 cut off, 0x200000 counted on its bus, 0x400000 velocity set, 0x2000000
frequency set, 0x4000000 looped, 0x8000000 3D, and one bit per list: 0x10000000 cutoff,
0x20000000 panning, 0x40000000 objects, 0x80000000 fades.

Addresses: `0x7a530f`, `0x7a5555`, `0x7a532d`, `0x7a5b2b`, `0x7a5b4d`, `0x7a5dcc`, `0x7a5e4c`, `0x7a6742`, `0x7a6930`, `0x7a6960`, `0x7a5870`, `0x7a601c`, `0x7a60fe`, `0x7a61ce`, `0x7a629e`, `0x7a5a23`, `0x7a5095`
Confidence: verified
Proof: one
Test: `sound::channel_volume_mix`, `sound::source_3d_attrs`, `sound::volume_ramp_scale`

`0x7a530f` keeps `flags & 0xff`; `0x7a5555` adds 0x4000000 for a looped open and `0x7a532d`
0x8000000 for mode 0x1000. Each setter ORs its bit where the address shows. `0x7a5870` returns bit
0x100000, `0x7a6960` bit 0x10000. Bits 0x2, 0x8 and 0x10 choose the category in
`Volume of a sound`; 0x4 is read by the end and cutoff handling.
`channel_volume_mix` and `volume_ramp_scale` run `0x7a5dc0` whole; `source_3d_attrs` runs `0x7a5b10`
whole.

## Opening a stream

`Sound::Play(bus, name, mode, held, flags)` refuses, returning null, a non-music sound while the
sound-effects switch is off or while the window is inactive. It calls `FSOUND_Stream_Open(name,
mode, 0, 0)`; a null stream returns null. It makes the record, sets the end callback and, unless
`held`, starts it with `FSOUND_Stream_PlayEx(FSOUND_FREE, stream, NULL, FALSE)`; a failed start
frees the record and returns null. A started music sound is muted while the window is inactive. The
2D opener uses mode 0x82000, the 3D opener 0x1000; the looped openers use 0x82002 and 0x1002, call
`FSOUND_Stream_SetLoopCount(stream, count − 1)` (count 0 loops forever) and add flag 0x4000000.

Addresses: `0x7a5290`, `0x7a52a6`, `0x7a52bf`, `0x7a52e5`, `0x7a5344`, `0x7a5360`, `0x7a536f`, `0x7a5390`, `0x7a53af`, `0x7a5450`, `0x7a5490`, `0x7a54d0`, `0x7a5592`, `0x7a5620`, `0x7a5680`, `0x87cf00`, `0xcf55b0`, `0xcf5600`
Confidence: verified
Proof: one
Test: none

The tests at `0x7a529c` (flags & 2), `[0x87cf00] > 0` and `[0xcf55b0] | [0xcf5600]` zero return 0.
`0x7a5450` and `0x7a5490` run `Sound::DupeCheckFailed` first. `0x7a5620(bus, name, flags, count,
held)` and `0x7a5680` hand a count of 1 to the one-shot opener; otherwise they call `0x7a54d0` with
`count − 1` (`dec eax`), `0x7a5680` opening held and unpausing when its last byte is 0. With count 1
`0x7a5680` passes that byte as the flags and drops the caller's (`0x7a56a6`); every caller passes
count 0, so the dropped flags never matter.
`[0xcf5600]` is read five times and written nowhere: no instruction stores to it, and no dword in
the image holds its address outside the five readers' operands (`0x7a52c9`, `0x7a53a1`,
`0x7a5509`, `0x7a55f6`, `0x7a68ed`) (control: `0xcf55b0`, two stores in `0x7a4860`).

## Bus caps and duplicates

Every open names one of 13 buses. A bus at its cap refuses the open, as does a bus of 13 or more;
nothing is evicted. The caps are 2³¹−1, 1, 2, 2, 1, 1, 2, 2, 1, 6, 4, 1, 2 for buses 0 to 12. With
flag 0x1 the open also fails while a sound of the same file name hash is live and not fading, or is
cut off. A sound counts on its bus once its channel starts and stops counting when the channel
stops.

Addresses: `0x7a66a0`, `0x7a66b5`, `0x7a6730`, `0x7a6760`, `0x87ce60`, `0xcf553c`
Confidence: verified
Proof: one
Test: none

`cmp ecx,0xd; jl` then `cmp [ecx*4+0xcf553c],[ecx*4+0x87ce60]; jl` (`0x7a66b5`); the table has no
other reference. The duplicate walk compares `SStrHash(name)` with `+0x80` and fails when the match
has bit 31 clear (`jns`) or `0x7a5870` true. `0x7a6730` increments under bit 0x200000 at the channel
start (`0x7a6893`); `0x7a6760` decrements, not below 0, in `Sound::Suspend` and `Sound::Stop3`.

## Pause reasons

`+0x88` holds four pause reasons: 0 held, 1 sound effects off, 2 suspended, 3 cut off.
`0x7a65a0(reason, set)` changes one; when the any-set state changes it pauses or resumes the
channel with `FSOUND_SetPaused`, or, with no channel and nothing set, starts the stream with
`FSOUND_Stream_PlayEx(FSOUND_FREE, stream, NULL, TRUE)`, applies the stored state and unpauses. It
returns whether reason 0 is clear, and 0 when a start fails. `Sound::SetPaused(b)` is reason 0.
`Sound::Suspend` stops the channel and stream, keeps the flags in `+0x3c`, leaves the object and
panning lists and sets reason 2; `Sound::Resume` restores them and clears it.

Addresses: `0x7a65a0`, `0x7a5ac0`, `0x7a5880`, `0x7a5900`, `0x7a6720`, `0x7a6860`, `0x7a6620`, `0x7a663c`
Confidence: verified
Proof: one
Test: `sound::channel_set_volume`, `sound::channel_volume_mix`, `sound::source_3d_attrs`, `sound::volume_ramp_scale`

`0x7a65a0` rejects a reason outside 0 to 3. `0x7a6860(this, silent)` applies the stored position
and velocity, counts the bus, sets the stored frequency, then either `FSOUND_SetVolume(ch, 0)` when
`silent`, a mute for a non-music sound with the switch off, or `Sound::UpdateVolume`; it mutes again
while the window is inactive and restarts a fade's clock. `Sound::IsSuspended` is bit 2 of `+0x88`.
`channel_set_volume`, `channel_volume_mix`, `source_3d_attrs` and `volume_ramp_scale` run `0x7a6720`
whole.

## Volume of a sound

`Sound::SetVolume2(v)` stores `v` at `+0x74` and, with a channel, calls `FSOUND_SetVolume(ch,
ftol(c · v · g · 255.0))` with `g` the distance gain `+0x78` and `c` the MusicVolume factor for flag
0x2, else AmbienceVolume for 0x8, else none for 0x10, else SoundVolume. The product is in the x87
register and `__ftol` truncates. `Sound::SetVolume(level)` passes `level · (1/255)`.
`Sound::UpdateVolume` reapplies `+0x74`, or level 0 for a sound on the fade list. `0x7a5d70` reads
a level back as the low byte of the float `v · 255 + 512.5`: it rounds, wraps once `v · 255 ≥ 255.5`
(`v ≥ 1.00196`), and holds only while `v · 255 + 512.5 < 1024`.

Addresses: `0x7a5dc0`, `0x7a5ded`, `0x7a5dfc`, `0x7a5e0b`, `0x7a5e1d`, `0x7a5e26`, `0x7a5da0`, `0x7a59e0`, `0x7a5d70`, `0x87cef4`, `0x87cef8`, `0x87cefc`, `0x7ffe58`
Confidence: verified
Proof: one
Test: `sound::channel_volume_curve`, `sound::channel_volume_mix`, `sound::channel_volume_rate_stream`, `sound::volume_ramp_scale`

`test al,2` loads `[0x87cef8]`, `test al,8` `[0x87cefc]`, `test al,0x10` skips the factor, else
`[0x87cef4]`; then `fmul [esi+0x78]`, `fmul [0x7ffe58]` and `__ftol` (`0x7a5e1d` to `0x7a5e26`). The
categories' file values (1.0, 0.5, 0.5) are replaced at start-up by the CVar handlers. `0x7a5d70`
adds `0x81d72c` (512.5) and returns `(bits >> 14) & 0xff`.
`channel_volume_curve` and `channel_volume_rate_stream` run `0x7a5d70` whole; `channel_volume_mix`
runs `0x7a5dc0` whole; `volume_ramp_scale` runs `0x7a5da0` and `0x7a5dc0` whole.

## Category volumes

The Sound, Music and Ambience volume setters clamp to [0, 1] (NaN becomes 1), store the factor and
reapply every channel whose `flags & 0xa` is 0, 2 or 8. The master volume clamps the same way and
calls `FSOUND_SetSFXMasterVolume` once with the rounded level. Turning sound effects off pauses every
looped or ambience non-music sound under reason 1 and destroys the other non-music sounds, keeping
records with flag 0x4 but stopping their streams; turning them on clears reason 1.

Addresses: `0x7a6390`, `0x7a6400`, `0x7a6480`, `0x7a6500`, `0x7a6560`, `0x7a6570`, `0x7a67a0`, `0x7a6660`, `0x87cf00`
Confidence: verified
Proof: one
Test: `sound::sfx_master_volume`

Each setter's three compares send v < 0 to 0 and v ≥ 1 or NaN to 1 (`fcomp`, `test ah,0x41/5`).
`0x7a6660(key)` walks the every-sound list. `0x7a6570(off)` stores `[0x87cf00] = (off != 0)` when it
changes and tail-calls `0x7a67a0` with `[0x87cf00] == 1`; `0x7a67a0` skips flag 0x2, applies
reason 1 to `flags & 0x4000008`, and when turning off stops (`0x7a5950`, flag 0x4) or destroys the
rest. The CVar handler passes `off` as the CVar being 0 (`interface#cvar-handlers`).
`sfx_master_volume` runs `0x7a6500` whole.

## Fades

`0x7a5730(seconds, level)` fades a sound in from silence to `level` (0 to 255): with a stream and
`seconds` ≥ 0.1 it stores the target, a rate of `level / (seconds · 1000)` per ms and the start time,
and puts it on the fade list; below 0.1 it only reapplies the stored volume. `0x7a57b0(seconds, v)`
passes `ftol(v · 255)`. `0x7a5a50(seconds)` fades out from the current level at
`level / (seconds · −1000)`, or, below 0.1 s or without a channel, stops the stream at once.
`Sound::Stop2(seconds)` fades out and releases the record at silence, or releases it at once.
`Sound::Stop` destroys the record a slot points at and clears the slot.

Addresses: `0x7a5730`, `0x7a575b`, `0x7a5777`, `0x7a57b0`, `0x7a5a50`, `0x7a5a67`, `0x7a5a85`, `0x7a5a10`, `0x7a5a23`, `0x7a5700`, `0x7a5fc0`, `0x7a6030`, `0x8022cc`, `0x801448`, `0x801620`
Confidence: verified
Proof: one
Test: `sound::channel_set_volume`, `sound::channel_volume_rate_stream`, `sound::volume_to_byte`

The threshold is `fcomp [0x801620]` (0.1) with `test ah,5` (`0x7a575b`, `0x7a5a67`): a NaN takes
the fade path. The rates multiply by `[0x8022cc]` (1000.0) and `[0x801448]` (−1000.0) and are stored
as floats at `+0x48`. `0x7a5fc0` links the fade list and sets bit 31; `0x7a6030` sets the volume to
the stored level `+0x44`, unlinks and clears it. `0x7a5a10` ORs 0x20000 when `0x7a5a50` returns 1.
`channel_set_volume` runs `0x7a5730` whole; `channel_volume_rate_stream` runs `0x7a5a50` whole;
`volume_to_byte` runs `0x7a57b0` whole.

## The fade pump

Each frame, for a fading sound with a channel, level = ftol((now − start) · rate) is added to the
start level for a fade out, or taken as is for a fade in. A fade out at or below 0 ends: the sound
leaves the fade list, then is released (flag 0x20000), suspended (cut off), stopped with its record
kept (flag 0x4), or released. A fade in stops at its target. Otherwise the level is applied with
`Sound::SetVolume`. A fading sound without a channel is released if flagged 0x20000 and left alone
otherwise.

Addresses: `0x7a4c20`, `0x7a4c5c`, `0x7a4c9b`, `0x7a4ca9`, `0x7a4cb8`, `0x7a4cbd`, `0x7a4d18`, `0x7a4d2a`, `0x7a4d38`
Confidence: verified
Proof: one
Test: `sound::ramp_step`

The elapsed time is the 32-bit difference of `OsGetAsyncTimeMs` readings, zero-extended and loaded
with `fild qword`, times the float rate, truncated by `__ftol`. A rate below 0 (`test ah,5` falls
through) is a fade out (`add esi,[ebx+0x44]`, `jg` keeps fading); a rate above 0 clamps at
`cmp esi,[ebx+0x44]; jl`. The next link is read before a sound is released.
`ramp_step` runs `0x7a4c20` whole.

## Positions and the listener

The engine maps a world vector (x, y, z) to FMOD as (−y, z, x). `Sound::SetPosition(pos, vel)`
stores both for a 3D sound and, with a live channel, calls `FSOUND_3D_SetAttributes(ch, pos, vel or
NULL)`. `0x7a5eb0(pos, vel, forward, up)` calls `FSOUND_3D_Listener_SetAttributes` with all four
mapped and a null velocity passed as NULL; `0x7a5f50` reads the listener position back as (z, −x, y)
of FMOD's. `0x7a5e80(min, max)` sets `FSOUND_Sample_SetMinMaxDistance` on the stream's sample for a
3D sound, `0x7a5e40` the frequency, `0x7a5bf0(props)` `FSOUND_Reverb_SetChannelProperties` for a
3D sound with a channel and non-null properties, and `0x7a5fa0(props)` `FSOUND_Reverb_SetProperties`.

Addresses: `0x7a5b10`, `0x7a5b82`, `0x7a5bc8`, `0x7a5eb0`, `0x7a5f1f`, `0x7a5f50`, `0x7a5e80`, `0x7a5e40`, `0x7a5bf0`, `0x7a5bf6`, `0x7a5fa0`
Confidence: verified
Proof: one
Test: `sound::env_sound_audible`, `sound::first_slot_farther_than_query`, `sound::listener_3d_attrs`, `sound::listener_get_attrs`, `sound::max_distance_cull`, `sound::sound_audible_distance`, `sound::source_3d_attrs`

`0x7a5b82` loads `pos.y`, `fchs`, and stores it first, then `pos.z` and `pos.x`; the velocity and the
listener's vectors follow the same pattern. `0x7a5f1f` turns a null velocity into a null argument
(`neg; sbb; and`). `0x7a5f50` stores `−f.x` at `+4`, `f.z` at `+0` and `f.y` at `+8`. The ui's
world frame feeds the listener (`0x483218`).
`env_sound_audible`, `first_slot_farther_than_query`, `listener_get_attrs`, `max_distance_cull` and
`sound_audible_distance` run `0x7a5f50` whole; `listener_3d_attrs` runs `0x7a5eb0` whole;
`source_3d_attrs` runs `0x7a5b10` whole.

## Sounds that follow objects

`Sound::Set3DUpdateHandle(guid)` puts a 3D sound on the object list with a GUID, or takes it off
for GUID 0. Each frame, when a position callback is installed, the pump asks it for the position of
each listed sound's GUID and, on success, stores it and calls `FSOUND_3D_SetAttributes` with no
velocity. The interface installs and removes the callback (`entries#playing-at-a-position`).

Addresses: `0x7a57e0`, `0x7a4d50`, `0x7a4da4`, `0x7a60a0`, `0x7a6110`, `0xcf55fc`, `0x7a4dae`
Confidence: verified
Proof: one
Test: `sound::sibling_remap_flip`

`0x7a57e0` tests flag 0x8000000, stores `+0x68` and `+0x6c` and calls `0x7a60a0` or `0x7a6110`.
`0x7a4d50` returns at once when `[0xcf55fc]` is null and skips sounds without a channel; the call
`call [0xcf55fc]` (`0x7a4da4`) takes the out vector in `ecx` and the GUID on the stack.
`sibling_remap_flip` runs `0x7a4dae` to `0x7a4ddf`.

## Distance cutoff

`Sound::SetCutoffDistanceSquared(c)` stores the distance `c` (not its square) for a 3D sound: 0
removes the sound from the cutoff list; otherwise it joins it, and a looped sound at or beyond `c`
of the listener, or at a NaN distance, is paused under reason 3 and flagged cut off. Each frame, a
listed sound strictly beyond `c` is skipped if cut off, suspended if looped, left playing if
flagged 0x4, and released otherwise. In range its distance gain is `1 − min(max(d − 0.9c, 0), k) / k` with
`k = c · 0.10000002`: recomputed every frame between `0.9c` and `c`, and once on entering `0.9c`,
which leaving re-arms. A cut-off sound coming back resumes, restarting its stream.

Addresses: `0x7a5ca0`, `0x7a5cca`, `0x7a5d14`, `0x7a506e`, `0x7a5d1a`, `0x7a5d32`, `0x7a5000`, `0x7a5071`, `0x7a507e`, `0x7a508e`, `0x7a50ac`, `0x7a50d2`, `0x7a50e6`, `0x7a50f2`, `0x7a517d`, `0x7a5102`, `0x7a510e`, `0x7a515d`, `0x7a516f`, `0x7a5196`, `0x7a6240`, `0x7a62b0`, `0x804588`, `0x81d728`
Confidence: verified
Proof: one
Test: `sound::channel_dist_update`, `sound::max_distance_cull`

`0x7a5cca` `jnp` takes `c == 0` only. The pump stores `d²` as a float and compares it with `c·c` in
the register (`0x7a506e`, `fld d²; fcompp`, `test ah,0x41; jne`): only `d² > c²` is out of range.
The setter keeps `d²` in the register (`0x7a5d14`, `fld c; fmul c; fcompp`, `test ah,0x41; je`):
only `c² > d²` is in range, so equality and NaN count as out. The `0.9c` test at `0x7a50f2` computes the
near state, which `SetNearCutoff` (`0x7a6930`) stores at `0x7a517d`; `0x7a6960` skips the gain
while it is set. `0x804588` is 0.9 and `0x81d728` 0.10000002. `de f1` at `0x7a515d` is `fdivrp`
computing `v / k`; the store at `0x7a516f` happens only when the value differs from `+0x78`, which
sets the reapply flag. A cutoff of 0 is never listed, so its sound is never cut.
`channel_dist_update` runs `0x7a5000` whole; `max_distance_cull` runs `0x7a5ca0` whole.

## Panning list

`Sound::ProcessPanningList` places a listed sound at unit distance from the listener: the
horizontal unit direction toward its position, rotated by its pan × 90° about that direction
crossed with straight up. Nothing live fills the list: the pan setter `0x7a5c20` is unreached, and `Sound::Resume` re-adds only a sound
that was on the list before.

Addresses: `0x7a4e00`, `0x7a5c20`, `0x7a6170`, `0x7a61e0`, `0x7a5900`, `0x7a592b`, `0x808acc`
Confidence: verified
Proof: one
Test: `sound::channel_2d_position_synth`

The angle is clamp(`+0x4c`, 0, 1) · `[0x808acc]` (π/2); the direction is skipped when both
horizontal offsets are under 0.001. The rotation axis is the direction crossed with (0, 0, 1),
built by `0x7bbab0` and applied by `0x7bb0e0`. `0x7a61ce` in `0x7a6170` is the only store of flag
0x20000000; `0x7a6170` is called from `0x7a5c20` and from `0x7a5936` under that saved flag.
`channel_2d_position_synth` runs `0x7a4e00` whole.

## The frame pump

The frame-time handler runs `Sound::Update`: the ended list, the fade list, the object list, then
the listener position, the panning list, the cutoff list and `FSOUND_Update`. FMOD's end callback
puts a stream's sound on the ended list under the engine lock; the pump stops a flag-0x4 sound's
stream and keeps its record (its owner sees it stopped) and releases any other.

Addresses: `0x7a4850`, `0x7a4ad0`, `0x7a4b21`, `0x7a4b4a`, `0x7a53c0`, `0x7a6920`, `0x7a4b60`, `0x7a4bcf`, `0x7a5810`, `0x7a5950`, `0x7a4b26`
Confidence: verified
Proof: one
Test: `sound::pump_listener_flip`

`0x7a4850` returns 1. `0x7a53c0` (`ret 0x10`) links its fourth argument on the list at
`0xcf558c` between `SCritSect::Enter` and `Leave`, calls the empty `0x7a6920` and returns 0. `0x7a4b60` tests `[edi+0x38]` bit
0x4 (`0x7a4bcf`). `0x7a5810` reports a suspended sound as playing, a sound without a channel as
stopped, else `FSOUND_IsPlaying`. `Sound::Stop3` clears the end callback, frees the channel
(`FSOUND_SetReserved(ch, FALSE)`, `FSOUND_StopSound`), stops and closes the stream and leaves every
list but the every-sound list.
`pump_listener_flip` runs `0x7a4b26` to `0x7a4b3d`.

## Window focus

The window activation event mutes every channel with `FSOUND_SetMute(FSOUND_ALL, TRUE)` when the
window becomes inactive and unmutes them when it becomes active. While it is inactive the engine
refuses to open any non-music sound and mutes a music sound it starts. The active flag starts at 0,
so plays before the first activation event are refused.

Addresses: `0x7a4860`, `0x7a486e`, `0x7a4882`, `0x7a52bf`, `0x7a53a9`, `0x7a68e3`, `0xcf55b0`
Confidence: verified
Proof: one
Test: none

`0x7a4860` reads the payload's first dword, stores it or 0 in `[0xcf55b0]` and passes −3
(`FSOUND_ALL`) with 0 or 1. `0xcf55b0` lies in the zero-filled part of `.data`; its only writers are
`0x7a486e` and `0x7a4882`.

## Shutdown

`0x7a4a20` releases every sound; if the device is up it unregisters both handlers, calls
`FSOUND_StopSound(FSOUND_ALL)` and `FSOUND_Close`, shuts the file cache, closes the log, clears the
device flag, sets the sound-effects state to −1 and frees the pool's blocks.

Addresses: `0x7a4a20`, `0x7a4a84`, `0x7a4a89`, `0x7a4abd`
Confidence: verified
Proof: one
Test: none

The walk takes each next link before releasing. `0x7a4a8e` calls the os cache's `0x7a93b0`.

## When a sound fails

Nothing in the engine retries or reports a failed play after start-up. A failed open or start
returns null and the caller's slot stays empty; a refused open (bus cap, duplicate, sound effects
off, window inactive) looks the same. When device set-up fails the error goes to the log, the
client carries on, and every later open fails because FMOD is not initialized.

Addresses: `0x7a52ec`, `0x7a536d`, `0x7a552c`, `0x7a55c4`, `0x7a6628`, `0x7a5450`, `0x7a5490`
Confidence: verified
Proof: one
Test: none

The null returns are `0x7a52ec`/`0x7a552c` (no stream) and `0x7a536d`/`0x7a55c4` (no channel, the
record freed). `0x7a6628` leaves a held sound without a channel and returns 0. That an open fails
when FMOD is not initialized is FMOD's behaviour, not read here.

## Unreached engine code

Fifteen engine functions have no live caller: `0x7a5840` (is it fading out), `0x7a5ae0` and
`0x7a5af0` (stream length and seek), `0x7a5c20` (pan), `0x7a5fb0` (read the reverb), `0x7a6310`
(FMOD's CPU use), `0x7a6320` to `0x7a6380` (output, driver and mixer counts and names, and an empty
function), `0x7a6790` (the mix rate) and `0x7a6830` (set flag 0x800000).

Addresses: `0x7a5840`, `0x7a5ae0`, `0x7a5af0`, `0x7a5c20`, `0x7a5fb0`, `0x7a6310`, `0x7a6320`, `0x7a6330`, `0x7a6340`, `0x7a6350`, `0x7a6360`, `0x7a6370`, `0x7a6380`, `0x7a6790`, `0x7a6830`, `0x87ce94`, `0x87cec8`
Confidence: verified
Proof: one
Test: none

`0x7a6330` and `0x7a6370` index the name tables `0x87ce94` (13 entries, for a count of 14) and
`0x87cec8` without a bound check. The census is `overview#unreached-code`'s.
