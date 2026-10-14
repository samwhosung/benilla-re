# sound: sound entries and plays

`ISoundInterface.cpp` turns `SoundEntries.dbc` into sound definitions and builds the tables the
other files look sounds up in. The play functions of `SoundInterface.cpp` take a definition, pick a
file, and open it through the engine with the definition's volume, pitch, distances and reverb.

Roots: `0x45c060`, `0x45cc80`, `0x45cda0`, `0x45ce60`, `0x45cfe0`, `0x458850`, `0x458870`, `0x458890`, `0x4588d0`, `0x4588f0`, `0x458990`, `0x4589a0`, `0x458a40`, `0x458ab0`, `0x458830`, `0x458f70`, `0x458f90`, `0x459230`, `0x458be0`, `0x458c50`, `0x458bf0`, `0x45bb70`, `0x45cdf0`

## Sound definitions

A definition is a 0x98-byte record from a block pool: ten file slots of 12 bytes (`+0x00` name,
`+0x04` remaining weight, `+0x08` weight), the volume (`+0x78`), the flags (`+0x7c`), the minimum
distance (`+0x80`), the distance cutoff (`+0x84`), the remaining total (`+0x88`), the EAX definition
(`+0x8c`), the last file picked (`+0x90`) and the file count (`+0x94`). The loader makes one per
SoundEntries row from columns 24 to 28 and the file columns, and keeps it in an array indexed by
the row's id. A row with no files gets no definition, so its id plays nothing. `0x45cda0(id)`
returns the definition or null for an id past the array.

Addresses: `0x45c080`, `0x45c0a6`, `0x45c127`, `0x45c139`, `0x45c167`, `0x45c172`, `0x45c195`, `0x45bd90`, `0x45bdf0`, `0x45be00`, `0x45cda0`, `0x45b4b0`, `0xb06c38`, `0xb06214`, `0xb0621c`
Confidence: verified
Proof: one
Test: none

The constructor `0x45bd90` sets volume 1.0, minimum 27.777779, cutoff 83.333336 and −1 in `+0x8c`
and `+0x90`. `0x45c139` to `0x45c161` copy row `+0x64` (flags), `+0x60` (volume), `+0x68`, `+0x6c`
and `+0x70`. `0x45c172` returns a definition with `+0x94 == 0` to the pool (`0x45bdf0`, a jump to
`0x45be00`, clears the count). The array is sized to SoundEntries' largest id + 1 and zeroed (`0x45c0a6`, `0x45c0bb`).
`0x45b4b0` makes the pool (0x98 bytes, 0x1000 per block).

## File paths

Each file slot's name is `DirectoryBase`, a separator and `File[i]`, formatted by `SStrPrintf` with
`"%s%s%s"` into 260 bytes. The separator is `\` unless the directory is null, empty, or ends in `\`.
Nothing else is normalized. Slots are filled in order and stop at the first empty file name. A
weight below 1 counts as 1. Equal names share one copy in a string block, matched case-insensitively,
and the slot keeps a pointer into the block.

Addresses: `0x45be10`, `0x45be4b`, `0x45be5a`, `0x45be6e`, `0x45be7d`, `0x45be86`, `0x45beab`, `0x45bec6`, `0x45bf0d`, `0x45bf5a`, `0x45c1da`, `0x45c226`, `0x64a420`, `0x8362d8`, `0x833184`
Confidence: verified
Proof: one
Test: none

`0x45be5a` raises a weight below 1 (`jge`). `0x64a420` is `strrchr`: it keeps the last match
(`0x64a43c`). `0x45be7d` tests the byte after the last `\`. A name is hashed with `SStrHashHT` and
compared with `SStrCmpI`; a new one is appended to the block and its offset stored. After all rows
the block is copied to `0xb061d4` (`0x45c1da`) and every slot's offset rebased (`0x45c226`).

## Picking a file

`0x45bb70(def, sequential, out, index)` returns a file name. An index other than −1 takes
`min(index, count − 1)`. One file is always file 0. A sequential pick takes the file after the last.
Otherwise it draws from the client's shared generator and takes the first file whose running sum
of remaining weights exceeds `MulHigh32(total, r)`; that file's remaining weight goes to 0, and when
the total reaches 0 every weight is restored except that file's. The pick is stored as the last.

Addresses: `0x45bb70`, `0x45bb88`, `0x45bba1`, `0x45bbc3`, `0x45bcc8`, `0x45bce2`, `0x45bcfe`, `0x45bd06`, `0x45bd1a`, `0x45bd40`, `0x882664`
Confidence: verified
Proof: one
Test: none

The draw advances the state at `0x882664` inline with the steps of `random#state-advance`. The
sequential step is `(last + 1) mod count` by `idiv`; the zone music uses it (`music#zone-music-tracks`).
`0x45bd40(def, i)` refills: slot `i` gets 0, the others their weight, and the total their sum. There
is no lower bound on the index argument.

## Volume and pitch of a play

`0x458c60(def, m)` returns the play volume: the definition's volume, plus `n · 0.01` with
`n = MulHigh32(31, r) − 15` when flag 0x800 is set, times `m`, clamped to [0, 1] with NaN to 0.
`0x458da0(def, sound, m)` sets that volume and, when flag 0x400 is set, the frequency
`trunc(22050 · (85 + MulHigh32(31, r)) / 100)` Hz, absolute rather than relative to the file's rate.
Both draws advance the client's shared generator.

Addresses: `0x458c60`, `0x458c69`, `0x458d45`, `0x458d4a`, `0x458d58`, `0x458d62`, `0x458d6d`, `0x458da0`, `0x458dc1`, `0x458e95`, `0x458e9a`, `0x458ea6`, `0x835a70`, `0x8029d0`
Confidence: verified
Proof: one
Test: none

`0x458d50` loads `n` with `fild`, `fmul [0x8029d0]` (0.01), `fadd [def+0x78]`, then `fmul m`; the
clamp tests `fcom 0` with `test ah,0x41` and `fcom 1.0` with `test ah,5`. The frequency is `imul`
by `[0x835a70]` (22050, a `.data` value nothing writes) and division by 100 through `0x51eb851f`.

## Playing by id

`InternalPlaySound(keep, noCategory, bus, id, index, music)` looks the definition up, picks a file,
and opens it held with `Sound::Play2D` and flags: the definition's duplicate bit (flag 0x20 of the
row), 0x4 for `keep`, 0x10 for `noCategory`, 0x2 for `music`. It then sets the play volume and pitch
and unpauses; a failed start destroys the sound. It returns the sound or null.
`SndInterfacePlaySound(id)` is bus 0 with nothing set and returns whether it played. `0x458a40(id,
noCategory)` keeps the sound; the cinematic camera's narration passes `noCategory` 1.

Addresses: `0x45ce60`, `0x45ce8c`, `0x45ce95`, `0x45cebd`, `0x45ced2`, `0x45cedc`, `0x45cee8`, `0x458850`, `0x458a40`, `0x458f40`
Confidence: verified
Proof: one
Test: none

`ret 0x10`: `cl` keep, `dl` no category, then bus, id, index and music on the stack. `0x458f40` is
`(flags >> 5) & 1`. `0x458850` passes `(0, 0, 0, id, −1, 0)` and turns the pointer into 0 or 1.

## Playing at a position

`0x45cf00(culled, keep, noCategory, bus, id, pos, index, m)` refuses a position at or beyond the
definition's cutoff from the listener, setting `*culled`. Otherwise it opens the file with
`Sound::Play3D`, sets the play volume, the position, the cutoff and the distances (the definition's
minimum, maximum 100000.0), unpauses, and sets the channel reverb. `0x458890` and `0x45cfe0`, the
same function in two files, return 1 when it plays or is culled and 0 when it fails.
`0x458870(id, pos, index, m)` is bus 0; `0x4588d0` bus 2; `0x458f90` and `0x459230` keep the sound.

Addresses: `0x45cf00`, `0x45cf2e`, `0x45cf3b`, `0x45cf79`, `0x45cf95`, `0x45cfa4`, `0x45cfd0`, `0x45cdf0`, `0x45ce3e`, `0x458ed0`, `0x458efb`, `0x458890`, `0x45cfe0`, `0x458870`, `0x4588d0`, `0x458f90`, `0x459230`
Confidence: verified
Proof: one
Test: none

`0x45cdf0(pos, c)` is `d² < c·c` against the listener (`fcompp`, `test ah,0x41`, `jne` false), so a
NaN is refused. `0x458ed0` calls `Sound::SetPosition(pos, NULL)`, `SetCutoffDistanceSquared(+0x84)`
and `0x7a5e80(+0x80, 100000.0)`. `0x458f90(id, index, pos)` and `0x459230(id, pos, index)` differ
in argument order; both pass keep 1.

## Looped plays

`0x458900(id, ambience, noCategory)` opens a definition's file as a 2D loop on bus 0, with flag 0x8
for `ambience` and 0x10 for `noCategory`, sets the volume and pitch, and unpauses. `0x4588f0` is the
ambience form; `0x458990` is the cinematic sequence's sound. `0x4589a0(id, pos)` opens a 3D loop at a
position. `0x458ab0(object, id, seconds, index)` opens a 3D loop at the object, fades it in from 0 to
level 255 over `seconds`, and makes it follow the object; the fade ends at volume 1.0 whatever the
definition says. The dynamic object code passes 2.0 s.

Addresses: `0x458900`, `0x458929`, `0x458933`, `0x45894c`, `0x4588f0`, `0x458990`, `0x4589a0`, `0x4589d9`, `0x458a00`, `0x458ab0`, `0x458b1e`, `0x458b2e`, `0x458b47`
Confidence: verified
Proof: one
Test: none

All three open with a loop count of 0 (forever) and held. `0x458b1e` calls `0x7a5730(seconds,
255)` before the volume is set at `0x458b2e`; the fade pump then drives the level from 0 to 255 and
leaves `+0x74` at 1.0 (`engine#fades`). `0x458b47` is `SndInterfaceAssociateSoundWithObject`.

## Sounds that follow objects

`SndInterfaceAssociateSoundWithObject(sound, object)` places a sound at the object and gives the
engine its GUID; a null object clears the GUID. The unit sound code installs `0x458bf0` as the
engine's position callback through `0x458be0` and removes it through `0x458c50`; the callback finds
the object by GUID and returns its position.

Addresses: `0x458b90`, `0x458ba8`, `0x458bb3`, `0x458bc4`, `0x458be0`, `0x458bf0`, `0x458c11`, `0x458c24`, `0x458c50`, `0xcf55fc`
Confidence: verified
Proof: one
Test: none

`0x458b90` calls the object's `vtable+0x14` for the position, `Sound::SetPosition(pos, NULL)`, then
`Sound::Set3DUpdateHandle` with the GUID at `[object+8]`. `0x458bf0` calls
`ClntObjMgrObjectPtr(guid, mask 1)` and `vtable+0x14`, and returns whether the object exists. The
callers are `0x623118` and `0x6230c5` (`objectlayer`).

## Channel reverb

`0x45c650` builds one 18-dword reverb record per SoundSamplePreferences row, indexed by id, and
marks it valid. `0x458f10(def, sound)` passes the record named by the definition's EAX column to
the engine when SoundReverb is on. An EAX value with no valid record passes null, and the engine
then makes no call. The record's flags are 7 and its rolloff factor 0.

Addresses: `0x45c650`, `0x45c6bf`, `0x45c710`, `0x45cacc`, `0x45cad3`, `0x45cae6`, `0x45cdc0`, `0x458f10`, `0x458f13`, `0xb061c0`
Confidence: verified
Proof: one
Test: none

`0x45c710` copies the row's 16 values, converting three floats with `__ftol`, and clamps them: the
first and third to [−10000, 1000], the integer levels to [−10000, 0], the ratios to [0, 1] or
[0, 10]. `0x45cdc0(i)` returns record `+4` when `0 ≤ i <` count and the valid byte is set. The
record layout is FMOD's channel reverb properties; the record array grows through `0x45e8c0`.

## Interface sound names

`0x45c2c0` enters every SoundEntries row of type 2 into a name table, keyed case-insensitively by
the row's name, with the row's id as the value; a later row of the same name replaces the id.
`0x458030` looks names up here (`interface#interface-sounds-by-name`).

Addresses: `0x45c2c0`, `0x45c306`, `0x45c31d`, `0x45c35e`, `0x45c3e9`, `0x45c464`, `0xb06bfc`
Confidence: verified
Proof: one
Test: none

`0x45c306` tests row `+4` (type) against 2. The table at `0xb06bfc` is a hash table of 0x1c-byte
nodes (`overview#hash-tables`); a found node gets the id at `0x45c464`, a new one is made
by `0x45ee10` and keeps a copy of the name.

## Loading and unloading

`0x45c060` unloads, loads the definitions, then builds the name table, the sheathe sounds, the
reverb records, the swing sounds and the weapon impacts. `0x45cc80` returns every definition to the
pool, frees the definition array, the sheathe and name tables, the reverb records and the string
block, and frees the pool's blocks.

Addresses: `0x45c060`, `0x45c2b0`, `0x45caf0`, `0x45cc80`
Confidence: verified
Proof: one
Test: none

`0x45c060` calls `0x45cc80`, `0x45c080`, `0x45c2b0` (`0x45c2c0`, `0x45c490`, `0x45c650`) and
tail-jumps to `0x45caf0` (`0x45cb00`, `0x45cb60`).

## Definition queries

`0x458830(id, out)` stores bit 0x200 of a definition's flags and returns whether it exists;
`0x458f70(id)` returns its file count, or 0.

Addresses: `0x458830`, `0x458843`, `0x458f70`
Confidence: verified
Proof: one
Test: none

`0x458843` is `shr eax,9; and al,1`. The callers of `0x458830` are `0x5f4051` and `0x60ee43`.
