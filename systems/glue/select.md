# glue: character select

The character list as the select screen keeps it, the selected character's model, and the
select screen's script functions.

The select screen copies the character list into a growable array of 0x120-byte
`CharacterSelectionDisplay` records (type tag `0x8386d4`) at `0xb4213c`. Beside it are the
selection `0x83856c`, the model frame `0xb42174`, the build counter `0xb42178` and the facing
`0xb4217c`. Selecting a record builds its model into the
model frame, a `CSimpleModelFFX` (`CSimpleModelFFX.cpp`, `0x46f510` to `0x46fad0`), the
glue screens' `ModelFFX` frame type. The select code, `0x471e30` to `0x473700`, passes no source
path. The table `0x838578` holds 11 script functions. The build drives the model, lighting and
screen-effect code of other systems, cited here, not derived.

Roots: `0x4724d0`, `0x472580`, `0x4725f0`, `0x472090`, `0x473700`, `0x4736c0`, `0x4736e0`, the bindings `0x4731c0`, `0x473230`, `0x473270`, `0x473280`, `0x4732a0`, `0x473470`, `0x4734c0`, `0x473520`, `0x473640`, `0x473650`, `0x473670`, the factory `0x46fa00`, the vtable slots `0x46fa60`, `0x46fa90`, and the helpers `0x471e30`, `0x471ec0`, `0x471f00`, `0x46f510`, `0x46f550`, `0x46f9e0`

## The roster

The roster is a growable array: capacity `0xb4213c`, count `0xb42140`, records `0xb42144`,
chunk `0xb42148`. Appending (`0x472300`, net's callback per character) grows it when count + 1
exceeds the capacity, builds the record at the old count with `+0x110` and `+0x114` zero and
`+0x118` = 1, copies net's 0x110-byte record over its first 0x110 bytes, raises the count, and
hands the name to the interface's add-on state (`0x51ebe0`). With no chunk set, a request of 8 or
more sets the chunk to 8 and a smaller one uses the largest power of two not above it; the
capacity is the request rounded up to a multiple of the chunk.

| offset | field |
|---|---|
| `+0x00`, `+0x08` | guid (8 bytes), name (48) |
| `+0x3c`, `+0x40` | zone id, guild id |
| `+0x50`, `+0xa0` | per slot 0 to 19: item display id, inventory type |
| `+0xf0`, `+0xf4`, `+0xf8` | pet display id, level, family |
| `+0xfc` | flags: 0x400 drops slot 0, 0x800 slot 14, 0x2000 ghost, 0x4000 rename |
| `+0x100` to `+0x108` | bytes: race, class, sex, skin, face, hair style, hair colour, facial hair, level |
| `+0x110`, `+0x114` | the character's component, the pet's model |
| `+0x118` | 1 until built; no glue code reads it |

Addresses: `0x472300`, `0x472315`, `0x472359`, `0x4723b8`, `0x4723c0`, `0xb4213c`, `0xb42140`, `0xb42144`, `0xb42148`
Confidence: verified
Proof: one
Test: none

`0x5aabe0` calls the callback with `ecx` at each 0x110-byte record (`0x5aac0b`). `0x472315`
compares count + 1 with the capacity; `0x472323` `cmp esi,8`, the chunk store at `0x472346`,
the lowest-bit loop `0x472331` to `0x472338` and `div` at `0x472359` size the request;
`0x47236b` calls `0x473ad0`, which reallocates in 0x120-byte records tagged `0x8386d4`.
`0x47238f` to `0x4723a4` construct, `rep movs` of 0x44 dwords copies (`0x4723b8`), and `0x4723c0`
jumps to `0x51ebe0` with the name. Net's reader `0x5b42a0` stores the fields at these offsets.
A census of `+0x118]` in glue finds the stores at `0x46b54a`, `0x4723a4` and `0x472faa` only.
The flags' meanings are their uses below; the names of `+0x102` to `+0x108` and of
`+0xa0` are the protocol's, not derived here.

## Roster load

When the list arrives, `0x4724d0` empties the interface's add-on state (`0x51f0b0` with null),
destroys every record, zeroes the count and appends net's records. With characters, it sets
`0x837e9c` to 0, selects the integer of the CVar `lastCharacterIndex` (`0x472740` clamps it) and
fires `CHARACTER_LIST_UPDATE` (7). With none, it selects −1, which stores 0 and fires
`UPDATE_SELECTED_CHARACTER` with 1, and detaches scene attachments 0 and 1; then, while
`0x837e9c` holds −1 (no realm chosen this session), it asks for the realm list with
`0x46b8d0(1)` and fires nothing, else it fires `CHARACTER_LIST_UPDATE`.

Addresses: `0x4724d0`, `0x47252e`, `0x47253d`, `0x47254b`, `0x472550`, `0x472556`, `0x837e9c`
Confidence: verified
Proof: one
Test: none

`0x4724d0` starts `xor ecx,ecx; call 0x51f0b0`, runs `0x473700` on each record, zeroes the
count (`0x47250c`) and enumerates with `0x472300` (`0x47251d`). A count of 0 calls
`0x472740(-1)` (`0x47252e`), `0x4722c0` and `0x4722e0`; `0x47253d` `cmp [0x837e9c],-1; jne`
reaches the event, else `0x47254b` jumps to `0x46b8d0` with `ecx` 1. Otherwise `0x472550`
loads the CVar record `0x882674` and its integer `+0x28`, `0x472556` stores 0 in `0x837e9c`,
and `0x472563` selects. `0x472568` tail-jumps to `FrameScript_SignalEvent` with 7.

## Roster teardown

Three functions empty the roster: each runs the record destructor `0x473700` on every record,
which releases the pet model `+0x114` (`0x7103a0`, leaving the pointer) and destroys the
component `+0x110` (`0x476000`, zeroing it), and zeroes the count. `0x4723d0`
(`GetCharacterListUpdate`) then detaches scene attachments 0 and 1 and tail-calls `0x46b880`.
`0x472580` detaches both, selects −1 (`UPDATE_SELECTED_CHARACTER` with 1), calls `0x76cec0(0)`
on the model frame when one is set, and fires `CHARACTER_LIST_UPDATE`. `0x472090`, from
`0x46ac90`, also frees the storage and zeroes the array, and detaches nothing. The detaches,
`0x4722c0` and `0x4722e0`, call `0x7130a0` with 0 and 1 on the frame's scene model `+0x318`.

Addresses: `0x473700`, `0x4723d0`, `0x472580`, `0x4725c6`, `0x4725d7`, `0x472090`, `0x4722c0`, `0x4722e0`
Confidence: verified
Proof: one
Test: none

`0x473700`: `+0x114` to `0x7103a0` with no store after; `+0x110` to `0x476000`, then
`mov [esi+0x110],0`. `0x4723ff` and `0x4725af` zero the count; `0x472413` jumps to `0x46b880`.
In `0x472580`, `or ecx,-1; call 0x472740` (`0x4725c6`); `0x4725d7` pushes 0 for `0x76cec0` on
`[0xb42174]`; `0x4725e1` jumps to `FrameScript_SignalEvent` with 7. `0x4720d2` frees
`[0xb42144]` with the tag `0x8386d4` and `0x4720d7` to `0x4720eb` zero the three cells.
`0x4722c0` and `0x4722e0` test `[0xb42174]` and `+0x318`; `0x7130a0` detaches each attached
model whose `+0x1d0` equals the argument (`0x7130b2`).

## Selecting a character

`0x472740(index)` stores the index in `0x83856c` when 0 ≤ index < count, else 0; runs the
model build `0x472950`; and fires `UPDATE_SELECTED_CHARACTER` (8) with the stored value plus 1,
formatted `%d`. The cell holds −1 in the image and `0x472754` is its only writer, so it is never
negative after the first select, and an empty roster stores 0 and fires 1.

Addresses: `0x472740`, `0x47274a`, `0x472754`, `0x47275e`, `0x47276c`, `0x83856c`
Confidence: verified
Proof: one
Test: none

`xor eax,eax; test ecx,ecx; setl al; dec eax; and eax,ecx` makes a negative index 0;
`0x47274a` `cmp eax,[0xb42140]; jl` keeps a smaller one (signed), else `xor eax,eax`.
`0x47275e` re-reads the cell after the build and adds 1; `0x47276c` calls `0x703f50(8, "%d",
value)`. The cell's file bytes are `ff ff ff ff`. Census: the image holds 31 little-endian
occurrences of `0x83856c`, each an operand of a glue instruction; the one store is
`mov ds:0x83856c,eax` at `0x472754`, which the census finds (its control).

## Select model build

`0x472950` acts only when 0 ≤ selection < count. It detaches scene attachments 0 and 1 and
zeroes the facing `0xb4217c` on every such select. A record with a component goes on to the glow
pass (Glow pass and attachment). Otherwise `0x5ec480(race, sex)` gives a model row; with none, or
an empty path `+0x08`, the build ends. In order it then stores `0x475fb0`'s result at `+0x110`;
passes the path to `0x707350` on the scene `0x76cfc0` returns; passes a description of the record
to `0x476b90`; passes `0x473090` to `0x711bb0` and `0x472150` to `0x7134b0`; and calls
`0x7121a0(-1, 0, -1, 0, 1.0, 1, 1)`. What these callees do is not read here.

Addresses: `0x472950`, `0x472971`, `0x472a61`, `0x472a6d`, `0x472a8a`, `0x472b61`, `0x472bda`, `0x473090`
Confidence: verified
Proof: one
Test: none

`0x472971` exits on `jge` count, `0x472965` on `jl` 0; `0x472a61` stores 0 before `0x472a6d`
tests `+0x110`. `0x5ec480(race, sex)` returns a `CreatureModelData` row; `0x472a8a` and
`0x472a93` leave on a null row or path. The description: `+0x00` race, `+0x04` sex, `+0x08` hair
colour, `+0x0c` skin, `+0x14` face, `+0x18` facial hair, `+0x1c` hair style, `+0x20` the model
(`0x472b42`), other words zero, and at `+0x12c` the 16 words 1, 101, …, 601, 702, 801, …, 1501.
The calls are at `0x472aac`, `0x472b30`, `0x472b37`, `0x472b61`, `0x472b87`, `0x472bac` and
`0x472bda`. `0x473090(model,
sequence, a, b, …)`, `ret 0x14`, does nothing when `b` is nonzero; for sequence −1 or 26 it
repeats the starting call; for `a` = 15 nothing; otherwise `0x7121a0(sequence, -1, -1, 0, 1.0,
1, 1)`.

## Equipment and the guild tabard

For slots 0 to 19 with a nonzero display id, the build skips slot 17, slot 0 under flag 0x400
and slot 14 under 0x800. Slots 15 and 16 with an `ItemDisplayInfo` row go to `0x47a0c0` on the
component's model with (slot, 0, 0, type = 14, 0); the rest go to `0x478cb0(slot, display id)`.
Slot 18 whose row has bit 0 of `+0x24` asks the guild cache (`0x6d6d20`) for guild `+0x40`'s
emblem with the callback `0x472420`; five values found at once go to `0x47a610`.
`0x472420(guild, …, found)` ignores guild 0 and `found` 0; otherwise, for each record of that
guild, it reads the cache again and hands the values to `0x47a610` on the component of the
record selected then.

Addresses: `0x472c01`, `0x472c15`, `0x472c25`, `0x472c84`, `0x472ca4`, `0x472cde`, `0x472420`, `0x4724a4`
Confidence: verified
Proof: one
Test: none

`0x472bf8` skips id 0, `0x472c01` `cmp esi,0x11`, `0x472c15` `test edi,0x400`, `0x472c25`
`test edi,0x800`. `0x472c4f` and `0x472c54` route 15 and 16;
`0x472c84` `sete` on `cmp edx,0xe`, the slot's `+0xa0`. `0x472c9b` `cmp esi,0x12`, `0x472ca4`
`test byte [edi+0x24],1`. `0x472cde` calls `0x6d6d20(guild, 0x472420, guid, 0, five
outputs)`; on `al` set, `0x472d13`. In `0x472420`, `ret 8`, the guild is `ecx` and `found` the
byte at `[ebp+0xc]`; `0x472455` compares each record's `+0x40`, and `0x4724a4` loads `+0x110`
of record `[0x83856c]`. `0x6d6d20` asks the guild cache through `0x560e30`, which on a miss
keeps the callback and sends the cache's query, opcode 0x54, through `ClientServices::Send`
(`0x5ab630`).

## Ghost kit

A record with flag 0x2000 gets `SpellVisualKit` row 989 on the character's model (`0x4727f0`),
when the store reaches id 989 and the row exists. Nine effect columns go to `0x472780` with an
attachment: `+0x0c` 20, `+0x10` 34, `+0x14` 19, `+0x18` 21, `+0x1c` 22, `+0x20` 17, `+0x24` 23,
`+0x28` 24, `+0x2c` 25. Then four (kind, value) pairs, kinds at `+0x3c` and values at `+0x4c`:
kind 1 passes three colour channels to `0x710cf0`, kind 14 a clamped value to `0x710cb0`.
`0x472780` skips a zero from `0x712cb0`, a negative or out-of-range effect id and a missing row;
otherwise it passes the row's path to `0x707350` and a non-null result, with the model and the
attachment, to `0x712f70`, then to `0x7103a0`.

Addresses: `0x4727f0`, `0x4727f6`, `0x47280f`, `0x472780`, `0x472795`, `0x472799`, `0x4728af`, `0x4728d3`, `0x4728ec`
Confidence: verified
Proof: one
Test: `glue::char_customize_clamp_difftest`, `glue::char_customize_color_difftest`

`0x4727f6` compares the store's largest id with 0x3dd (`jl`); `0x47280f` reads index entry
0xf74/4 = 989. The calls from `0x472823` push 0x14, 0x22, 0x13, 0x15, 0x16, 0x11, 0x17, 0x18,
0x19 with `edx` from `+0x0c` to `+0x2c`. In `0x472780`, `0x472795` leaves when `0x712cb0`
returns 0 and `0x472799` on `jl`. Tint: `c = __ftol(value)`; bytes 2, 1, 0 of `c` (`0x4728f8`),
each `fild`, `fmul` by the f32 1/255 (`0x3b808081`) and `fstp` to f32, are red, green and blue for
`0x710cf0` (`0x472939`). Alpha: `fcomp` 0.0, `test ah,5; jp` at `0x4728af` stores 0 only for a
value below 0 (C0 alone); `fcomp` 1.0, `test ah,0x41; jne` at `0x4728d3` keeps any value not
above 1, NaN included, else 1.0; then `0x710cb0`.
`char_customize_clamp_difftest` runs `0x4728a4` to `0x47293e`; `char_customize_color_difftest` runs
`0x4728ec` to `0x472939`.

## The pet

A nonzero pet display id `+0xf0` with a `CreatureDisplayInfo` row whose `CreatureModelData` row
exists passes the old `+0x114` to `0x7103a0` and stores there what `0x707350` returns for the
model row's path. The pet model's scale is `s = f32(model +0x10 × display +0x10)`, or, when the family
`+0xf8` has a `CreatureFamily` row, `f32(min + (max − min) · t)` with `t = clamp(level −
minLevel, 0, range) / range` and `range = maxLevel − minLevel`, from the row's `+0x04`, `+0x08`,
`+0x0c` and `+0x10` and the level `+0xf4`; a zero range gives NaN. Its model then gets the two
callbacks, the starting call, `0x4797b0(model, display row, model row)` and `0x710620` with
diag(s, s, s, 1).

Addresses: `0x472d6b`, `0x472d9c`, `0x472e4f`, `0x472dc6`, `0x472e05`, `0x472e6e`, `0x472e87`, `0x472f94`
Confidence: verified
Proof: one
Test: `glue::charcreate_blend_frame_interp_difftest`, `glue::charcreate_blend_scale_product_difftest`

`0x472d6b` `jbe` skips id 0; `0x472d7d` and `0x472d9c` read the two rows, `0x472e20` releases,
`0x472e4f` stores the new model; `0x472dc0` skips the rest without one. `0x472dc6` `fld
[edi+0x10]`, `fmul [ebx+0x10]`, `fstp` f32. `0x472e05` `jl` makes a level below the minimum 0
(signed); `0x472e6e` `jle` keeps steps not above the range. `fild` steps, `fidiv` range,
`fld max; fsub min; fmulp; fadd min` at 53-bit precision, and one `fstp` to f32 at `0x472e87`.
`0x472f09` calls `0x4797b0(model, display row, model row)`; `0x472f15` to `0x472f85` store the
matrix, s at entries 0, 5 and 10, 1.0 at 15, zero elsewhere, for `0x710620` (`0x472f94`).
`charcreate_blend_frame_interp_difftest` runs `0x472e73` to `0x472e8a`;
`charcreate_blend_scale_product_difftest` runs `0x472dc6` to `0x472dd7`.

## Glow pass and attachment

A select that passes the gate and finds a model row passes the `ffxDeath` effect (`0xb41468`)
for a ghost, else `ffxGlow` (`0xb414c4`), to `0x6cde60`, and writes 0.15 (`0x838570`) or 0.40
(`0x838574`) to `+0x110` of the pointer `0x6d48b0` returns. A first build before that clears
`+0x118` and raises the build counter `0xb42178`. With the frame's scene model `+0x318`, it passes
the character's model, `+0x318` and attachment 0 to `0x712f70`, calls `0x4730e0(0)`, passes a pet
model with attachment 1 the same way, and calls `0x710450(1, 1)` on `+0x318`. What the callees
do is not read here.

Addresses: `0x472fb2`, `0x472fd9`, `0x472fe9`, `0x473002`, `0x473039`, `0x47306e`, `0x838570`, `0x838574`, `0xb41468`, `0xb414c4`
Confidence: verified
Proof: one
Test: none

`0x472fb2` `inc [0xb42178]` is the counter's only writer; the image holds two occurrences, the
other the read at `0x4730e6`. `0x472fba` calls `0x6d48b0` for the light state; `0x472fd9`
`test dh,0x20`. `0x472fde` passes `[0xb41468]` to `0x6cde60`, and `0x472fe9` stores `[0x838570]` (`0x3e19999a`); `0x472ff7` and `0x473002` do the same
with `[0xb414c4]` and `[0x838574]` (`0x3ecccccd`). The effects are built at `0x46a723` by
`0x6cc130`, which registers the CVar `ffxGlow`, and at `0x46a752` by `0x6cc690` (`ffxDeath`).
`0x47301b` jumps to `0x473073` without a scene model; `0x473039` attaches with 0 and `0x47306e`
with 1.

## Ghost light callback

`0x472150` is the light callback of the select scene and of the selected character's and pet's
models. It does nothing unless the selected record is on the roster with flag 0x2000. Then it
adds one light to the model's lighting: from `LightParams` row 3, `LightIntBand` rows 37 and 38
(`18 · 3 − 17 + k`) at time 0, band 38 into the light's `+0x30` and band 37 into `+0x3c` as red,
green and blue over 255; direction (0, 0, −1); enabled. `SetCharSelectBackground` (`0x473230`)
passes its file to `0x472110`, which, with a model frame set and a non-empty name, loads it
through the frame's slot 37 and puts `0x472150` on the new scene model in place of the frame's
`0x76d680`.

Addresses: `0x472150`, `0x472194`, `0x4721d3`, `0x4721ed`, `0x472280`, `0x472110`, `0x472123`, `0x472144`
Confidence: verified
Proof: one
Test: none

`0x472166` and `0x472172` bound the selection; `0x472194` `test ch,0x20`. `0x4721d3`
compares the `LightParams` store's largest id with 3 (`jl`); `0x4721e2` loads entry 3.
`0x6d6ad0` indexes `LightIntBand` with `edx + 18·id − 17`, called with 0 (`0x4721ed`) and 1
(`0x472204`); `0x6d62e0` evaluates each at `edx` 0. Band 1's colour goes to `[ebp-0x60]` and
band 0's to `[ebp-0x54]`, the light being `[ebp-0x90]`. `0x472280` stores −1.0 as z of the
direction, `0x472294` enables, `0x4722a2` adds the light (`0x71bf90`). `0x472123` skips an empty
name, `0x472128` calls `+0x94`, `0x472144` calls `0x7134b0`. Slot 37, `0x76cce0`, loads the file
and hands the model to slot 36, `0x76cd30`, which stores it at `+0x318` and installs `0x76d680`
the same way.

## Forced rename applier

`0x4725f0(name, guid)`, on the rename reply, finds the guid's record, or does nothing.
It moves `WTF\Account\<account>\<realm>\<old>` to the same path with the new name, then
`WTF\Account\<account>\<old>` likewise (`0x42a820`, `MoveFileW`), the account from `0xb41db0`
and the realm from `GetRealmName`. It drops the old name's add-on state (`0x51f0b0`), loads the
new name's (`0x51ebe0`), copies the name into `+0x08` (at most 48 bytes), clears flag 0x4000 and
fires `CHARACTER_LIST_UPDATE`.

Addresses: `0x4725f0`, `0x472613`, `0x47269a`, `0x4726f0`, `0x472718`, `0x472722`, `0xb41db0`
Confidence: verified
Proof: one
Test: none

`0x472613` to `0x472620` compare `+0x00` and `+0x04` with the two stack words (`ret 8`); `ecx`
is the new name. `SStrPrintf` with `0x838708` (`WTF\Account\%s\%s\%s`) builds both paths and
`0x47269a` calls `0x42a820(old, new)`; `0x8386f4` (`WTF\Account\%s\%s`) builds the second pair
for `0x4726f0`. `0x42a820` converts both to UTF-16 and calls `MoveFileW`, bound on first use.
`0x4726ff` passes the old name, `0x472706` the new, `0x472718` calls `SStrCopy` with 0x30,
`0x472722` `and [+0xfc],0xffffbfff`, `0x472732` fires 7. The caller, `0x46b4eb`, is in the
handler of opcode 0x2c8, `0x46b440`.

## Script functions

`0x4736c0` registers the 11 `{name, function}` pairs of `0x838578`; `0x4736e0` removes them. An
index is truncated (`__ftol`), less 1; a first argument of the wrong type raises the usage
string.

| name | function | arguments | returns |
|---|---|---|---|
| `SetCharSelectModelFrame` | `0x4731c0` | frame name | none |
| `SetCharSelectBackground` | `0x473230` | file | none (Ghost light callback) |
| `GetCharacterListUpdate` | `0x473270` | none | none; `0x4723d0` |
| `GetNumCharacters` | `0x473280` | none | the count |
| `GetCharacterInfo` | `0x4732a0` | index | eight (Character info) |
| `SelectCharacter` | `0x473470` | index | none; −1 outside [−1, count), then `0x472740` |
| `DeleteCharacter` | `0x4734c0` | index | none; a record's guid to `0x46b2e0` |
| `RenameCharacter` | `0x473520` | index, name | none, 1 or nil |
| `UpdateSelectionCustomizationScene` | `0x473640` | none | none |
| `GetCharacterSelectFacing` | `0x473650` | none | degrees (Facing in degrees) |
| `SetCharacterSelectFacing` | `0x473670` | degrees | the top of the stack |

`RenameCharacter` returns nothing unless the name is a non-empty string, the index is on the
roster and the flags hold 0x4000; the old name again, case folded, fires `FORCE_RENAME_CHARACTER`
(23) with `CHAR_CREATE_NAME_IN_USE`; else `0x46b340(name, guid)` gives 1 when nonzero, nil when
zero. `UpdateSelectionCustomizationScene` calls `0x477860(1)` on the selected component.
`SetCharSelectModelFrame` stores a frame of the model type with that name in `0xb42174`
(`0x472100`); nothing clears the cell.

Addresses: `0x4736c0`, `0x838578`, `0x473210`, `0x472100`, `0xb42174`, `0x4734a5`, `0x4734b2`, `0x4735ab`, `0x4735bd`, `0x47360d`, `0x473625`
Confidence: verified
Proof: one
Test: none

`0x4736c0` calls `0x704120(name, function)` per pair; `0x4736e0` calls `0x704160(name)`. Each check is `lua_isnumber` or `lua_isstring`, with `luaL_error` and the usage
string `0x838720`, `0x83874c`, `0x838778`, `0x838798`, `0x8387b8`, `0x8387f0` or `0x838814`. `0x4734a5` `cmp eax,-1; jl` and `0x4734b2` `or eax,-1`; `0x4734f5`
`js`, then `0x473516` passes `+0x00`, `+0x04`. In `0x473520`, `0x47355c` and `0x473574` leave on
a bad name, `0x4735ab` `test ch,0x40`, `0x4735bd` `SStrCmpI` with `0x7fffffff`, `0x4735d2` fires
with `0x8387d8`; `0x473602` calls `0x46b340`, `0x47360d` pushes 1.0 and `0x473625` nil, each
returning 1. `0x473190` tests `[0x83856c] < count` unsigned and `+0x110` (`0x4731b6`).
`0x473210` calls `0x76c760(name, type id)` and `0x47321b` calls `0x472100`, the one store among
the image's 12 occurrences of `0xb42174`.

## Character info

`GetCharacterInfo(index)` returns, for a record on the roster: the name; the race's `ChrRaces`
name in the client's locale, or the empty string with no row; the class's `ChrClasses` name
likewise; the level; the zone's `AreaTable` name, or nil; the race's file string (`ChrRaces`
`+0x3c`), or the empty string; the sex; 1 for a ghost (flag 0x2000), else nil. Off the roster:
nil, nil, nil, 0, nil, nil, 0, nil.

Addresses: `0x4732a0`, `0x4732dc`, `0x473328`, `0x473336`, `0x4733b6`, `0x4733ce`, `0x4733ff`, `0x47341e`
Confidence: verified
Proof: one
Test: none

`0x4732dc` and `0x4732e8` bound it. Pushes: `+0x08`; the `ChrRaces`
row's `+0x44 + 4·locale` (`0x473328`, the client's locale index), else an empty string
(`0x473336`); the `ChrClasses` row's `+0x14 + 4·locale` (`0x473362`), else empty (`0x47336e`);
byte `+0x108`; the `AreaTable` row of `+0x3c`, its `+0x2c + 4·locale` (`0x4733b6`), else nil
(`0x4733c5`); the race row's `+0x3c` (`0x4733ce`), else empty; byte `+0x102`; `0x4733ff`
`test ah,0x20` pushes 1.0 or nil. Every path returns 8; `0x47341e` pushes the off-roster tuple.

## Facing in degrees

The facing is an f32 in radians at `0xb4217c`. `GetCharacterSelectFacing` returns it times the
f32 57.29578 (`0x803a78`, `0x42652ee1`) as a double. `SetCharacterSelectFacing(degrees)` passes
`f32(degrees × 0.017453292)`, the f32 `0x3c8efa35`, the product at 53-bit precision, to
`0x4730e0`, and returns 1 with nothing pushed: the caller gets its own last argument. `0x4730e0`
does nothing until the build counter `0xb42178` is nonzero; then it stores the facing and, with a
roster and a selected component model, sets that model's transform to a rotation by the facing
about (0, 0, 1), with no translation and scale 1 (`0x710650`). It never turns the pet.

Addresses: `0x473650`, `0x473670`, `0x47369f`, `0x4736ae`, `0x4730e0`, `0x4730ed`, `0x473179`, `0xb4217c`, `0x803a78`
Confidence: verified
Proof: one
Test: `glue::lua_deg2rad_difftest`, `glue::lua_rad2deg_difftest`

`0x473650` `fld dword [0xb4217c]`, `fmul dword [0x803a78]`, `fstp qword` for `lua_pushnumber`. `0x47369f` multiplies the double from `lua_tonumber` by the pooled f32 and
`fstp`s an f32 as `0x4730e0`'s argument; `0x4736ae` `mov eax,1`. `0x4730ed` `je` on a zero
counter, `0x4730f6` stores, `0x473102` `jbe` on count 0. `0x473179` calls `0x710650(model,
&(0,0,0), facing, &(0,0,1), &(1,1,1))` on `+0x110`'s `+0x38`. The image holds three occurrences
of `0xb4217c`: the stores `0x472a61` and `0x4730f6`, and this read.
`lua_deg2rad_difftest` runs `0x47369a` to `0x4736a9`; `lua_rad2deg_difftest` runs `0x473650` to
`0x473662`.

## The ModelFFX frame

`CSimpleModelFFX` is the frame type `ModelFFX`: a 988-byte `CSimpleModel` made by the factory
`0x46fa00`, whose constructor `0x46fa30` runs the base constructor and stores the vtables
`0x803960` and, at `+0x24`, `0x803934`. It replaces the base's primary slots 0, 12 and 17. Slot
12, which the frame's show path calls, writes 0.30 (`0x8380b4`) to the light state's `+0x110`,
makes `ffxGlow` current and runs the base slot 12. Slot 17 is the base's layer registration with
`0x46fad0` in place of the model draw `0x76d240` for layer 2: the model draws between the current
effect's begin (`0x6cd890`) and end (`0x6cda70`).

Addresses: `0x46fa00`, `0x46fa30`, `0x46fa60`, `0x46fa68`, `0x46fa90`, `0x46fad0`, `0x803960`, `0x803934`, `0x8380b4`
Confidence: verified
Proof: one
Test: none

`0x46a4f4` registers the name `0x836f20` (`ModelFFX`) with `0x46fa00`, which calls
`SMemAlloc(0x3dc, CSimpleModelFFX.cpp, 23)` (`0x46fa11`) and `0x46fa30(parent)`. After
`CSimpleModel::CSimpleModel` (`0x76c8e0`), `0x46fa3f` and `0x46fa45` store the vtables, which
differ from the base constructor's two at primary slots 0, 12 and 17 and secondary slot 1. `0x76ae10` sets a frame's `+0xd4` from 0 to 1 and calls slot 12
(`+0x30`). `0x46fa68` stores `[0x8380b4]` (`0x3e99999a`) into `0x6d48b0()`'s `+0x110`, then
`0x6cde60` takes `[0xb414c4]` and `0x46fa82` jumps to `0x76b260`, the base's slot 12. The base's
slot 17, `0x76d160`, is `0x46fa90` instruction for instruction but for the callback: `0x76b3a0`,
then, with a scene model and layer 2, `0x773110(callback, this)` (`0x46fabc`). `0x46fad0` calls
`0x6cd890`, `0x76d240` and jumps to `0x6cda70`.

## Growable array helpers

`0x471ec0(n)` returns a growable array's chunk: 64 for n ≥ 64, also stored at `+0x0c`; else the
largest power of two not above n, or 1 for 0. `0x471f00(n, chunk)` returns n rounded up to a
multiple of the chunk, unsigned. Nine call sites in other systems grow their arrays with them.
`0x471e30` and `0x46f550` set a pointer array's capacity to n: reallocate, or allocate, copy
min(n, count) and free. `0x46f510` frees a `RealmCategory` array's storage, and the array when
bit 0 of its argument is set. `0x46f9e0` builds the 16-byte base, vtable `0x803820` and three
zero words, of the login listener object.

Addresses: `0x471ec0`, `0x471ece`, `0x471f00`, `0x471f0e`, `0x471e30`, `0x46f550`, `0x46f510`, `0x46f9e0`, `0x803820`
Confidence: verified
Proof: one
Test: none

`0x471ec9` `cmp eax,0x40; jae` to the store of 0x40 at `[ecx+0xc]`; `0x471ece` `lea ecx,[eax-1];
and ecx,eax` repeats until one bit is left, then `cmp eax,1; jae` returns it or 1. `0x471f0e`
`div ecx`; a remainder adds `chunk − remainder`; `ret 8`, `ecx` unread. Each other call site
takes the chunk from `0x471ec0` when the chunk word is zero, rounds with `0x471f00` and
reallocates, as `0x472300` does inline. `0x471e30` stores n at `+0x00`, `SMemReAlloc`s `+0x08` to 4n bytes tagged
`0x8383d0` (`0x471e52`), and on null `SMemAlloc`s (`0x471e67`), copies and frees; `0x46f550` is
the same tagged `0x837fb4`. `0x46f52c` `test byte [ebp+8],1`; the only caller, `0x46e35f`,
passes 0. `0x46e561` calls `0x46f9e0` on 0x14 bytes and then stores `0x803808`.
