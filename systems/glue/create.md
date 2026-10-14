# glue: character create

The create screen's state, its 24 Lua functions, and the random name generator.

The create code runs from `0x46ff60` to `0x471a70`, the name generator from `0x475110` to
`0x475680`; neither passes a source path. The first nine functions are `CSimpleModelFFX`'s
destructor and inherited methods its vtables hold. The rest keeps the create state in globals.
Each glue start rebuilds the race list; `ResetCharCustomize` draws a random race, sex and
appearance and builds that character into the frame's model; the other functions change one
part, query it, or send it to the server. Draws use the client's shared random state
`0x882664`. The name generator at `0xb420b8` counts the three-byte windows of one race's
`NameGen` names and walks them as a chain. The character object belongs to another system.

Roots: `0x470430`, `0x470640`, `0x471a20`, `0x471a50`, `0x4755f0`, `0x475110`, the 24 functions of the table 0x8380f8, `0x471330`, `0x4713a0`, `0x4713e0`, `0x4713f0`, `0x471450`, `0x471520`, `0x471560`, `0x4715d0`, `0x471620`, `0x471680`, `0x4716b0`, `0x4716e0`, `0x471740`, `0x471780`, `0x4717c0`, `0x471800`, `0x471810`, `0x471820`, `0x471890`, `0x471900`, `0x471910`, `0x471930`, `0x471980`, `0x4719e0`, the vtable slots `0x46ff60`, `0x46ff70`, `0x46ff80`, `0x46ff90`, `0x46ffa0`, `0x46ffd0`, `0x470030`, `0x470040`, `0x470080`, and the callbacks `0x470c90`, `0x470ce0`, `0x475420`

## Create state

The race list is a growable array of `ChrRaces` ids: capacity `0xb42058`, count `0xb4205c`,
data `0xb42060`. The class list holds `ChrClasses` record pointers: capacity `0xb4207c`, count
`0xb42080`, data `0xb42084`. `0xb42068` is the selected race index, −1 for none; `0x8380f0` the
class id, 1 in the file; `0xb42078` the create character; `0xb420f0` the model frame; `0xb420f4`
the facing in radians, a float. All but the class start zero. Until the first build the
character is null, and `GetNameForRace`, `GetSelectedSex`, `SetSelectedSex` and
`SetCharacterCreateFacing` read through it unchecked.

Addresses: `0xb42058`, `0xb4205c`, `0xb42060`, `0xb42068`, `0x8380f0`, `0xb42078`, `0xb4207c`, `0xb42080`, `0xb42084`, `0xb420f0`, `0xb420f4`, `0x470430`
Confidence: verified
Proof: one
Test: none

The file's data ends at 0x883000. Writers, by dword scan: `0x470430` zeroes the frame, facing
and race count (`0x47043b` to `0x470447`); `0x4707a0` stores the frame, `0x470a59` the facing,
`0x4705e1` the race count, `0x470a08`, `0x470a2c` and `0x470d5a` the index, `0x4709ed` and
`0x470f6a` the class, `0x470bcd` and `0x470652` the character; static initializers the rest. The
race array's free tag is `.H`, the class array's `0x8383d0`. `0x4713f0`, `0x4716b0`, `0x470e30`
and `0x470a50` index `[0xb42078]` with no test.

## Range draws

A draw in `[0, n)` advances the shared state `0x882664` exactly as `0x4531e0` does and returns
the high dword of `n` times the new accumulator. An `n` of 0 or less, signed, returns 0 and
leaves the state alone. It is seeded with 0 at load and with the clock in `InitializeGlobal`, and
many systems draw from it.

Addresses: `0x4702d0`, `0x4702db`, `0x470330`, `0x47039e`, `0x4703a9`, `0x882664`, `0x4531e0`, `0x455c70`
Confidence: verified
Proof: one
Test: none

`0x4702db` `test ecx,ecx; jg`. The inlined advance has the library's steps 4, 12, 24, 28, moduli
`0xbc`, `0xd4`, `0xec`, `0xf4`, table `0x802700` (`0x470330` on) and rotates, and stores both
dwords (`0x47039e`).
`0x4703a9` calls `0x455c70(n, acc)`, `mov eax,ecx; mul edx; mov eax,edx`. Of the 21 calls to
`0x44ea80`, two pass `0x882664`: one with 0 in a static initializer, `0x40280d` with
`0x42c010()`.

## The race list

At every glue start the list is rebuilt from `ChrRaces` in file order, Alliance races first,
then Horde. A row with flag bit 0 (`+0x04`) is skipped. For each `FactionGroup` row, in file
order, with a nonzero mask index whose bit the row's faction template (`+0x08`) holds in its
group mask (`+0x0c`), a case-insensitive match of the group's `InternalName` with `Alliance`
(first pass) or `Horde` (second) appends the race id, so a race matching two groups appears
twice. The shift takes the index mod 32.

Addresses: `0x470430`, `0x470483`, `0x470498`, `0x4704f8`, `0x470505`, `0x47051f`, `0x47057d`, `0x4705e9`, `0x83841c`, `0x838414`
Confidence: verified
Proof: one
Test: none

Stores, matched through their loaders: `0xc0ded8` ChrRaces
(stride `0x74`), `0xc0dd34` FactionTemplate, `0xc0dd5c` FactionGroup (stride `0x30`).
`0x470483` `test byte [edi+4],1`; `0x470498` bounds the template id; `0x4704f8` skips index 0;
`0x470505` `shl edx,cl`, then `test [tpl+0xc],edx`. `0x47051f` and `0x47057d` call `SStrCmpI`
on `+0x08` with `0x83841c` or `0x838414`; `0x4705e9` appends the row's `+0x00`, and the group
loop goes on.

## Class list for a race

The list holds, in `CharBaseInfo` file order, a `ChrClasses` record pointer for each row of the
race, except race 3 with class 8, which both passes skip; a class id past the store's maximum
or without a row gives a null pointer. The first pass counts: an unchanged count keeps the
array, 0 frees it, any other resizes it.

Addresses: `0x4706b0`, `0x4706e3`, `0x4706e8`, `0x47075b`, `0x470760`, `0x470781`, `0x470700`, `0x471a70`
Confidence: verified
Proof: one
Test: none

The race comes in `ecx`. `0xc0df8c` is the
CharBaseInfo store, records 8 bytes apart, race byte `+0`, class byte `+1`. Pass one: `cmp
edi,3; jne` (`0x4706e3`), `cmp byte [ecx+1],8; je` skip (`0x4706e8`); pass two repeats it at
`0x47075b` and `0x470760`. `0x470700` compares with `0xb42080`; 0 calls `0x471a70`, which frees
the data, and clears the cells. The id is looked up in the ChrClasses store `0xc0deec`,
else 0 (`0x470781`).

## Five customization draws

A random appearance is five draws in `[0, count)` for the race and sex, in this order: skin (`0x47a440`), hair style (`0x47a3e0`), hair colour (`0x47a400`, for the style
just drawn), facial hair (`0x47a4c0`), face (`0x47a4a0`). A random character first draws its
sex in `[0, 2)`, then a race list index. The names are the create request's, as the server reads
it. The counts index a grid by `2·race + sex`, so sex 2 reads the
next race's male counts.

Addresses: `0x4702a0`, `0x4703c0`, `0x4703ed`, `0x47a440`, `0x47a3e0`, `0x47a400`, `0x47a4c0`, `0x47a4a0`
Confidence: verified
Proof: one
Test: none

The record is a 0x16c-byte local: race `+0x00`, sex `+0x04`. `0x4702a0` draws 2 into `+0x04`,
then an index, stores `[0xb42060 + 4i]` and tail-jumps to `0x4703c0`, which calls each count
with race in `ecx`, sex in `edx`, and stores the draws at `+0x0c`, `+0x1c`, `+0x08`, `+0x18`,
`+0x14`. `0x4703ed` passes the `+0x1c` draw to `0x47a400`. Each count computes `lea
eax,[edx+ecx*2]`.

## Building the character

A build needs a model path for the race and sex. It destroys the old character, makes one, loads the path into the frame's scene, sets the model's two callbacks and an animation,
sets the character up from the record, applies the stored facing, rebuilds the class list,
dresses it, bakes it and passes it with the frame's model at `+0x318` to `0x712f70`. With no
path nothing changes; a failed load leaves the new character without a model.

Addresses: `0x470b70`, `0x470b80`, `0x470b8e`, `0x470bb7`, `0x470bc2`, `0x470bdf`, `0x470be9`, `0x470c2e`, `0x470c33`, `0x470c59`, `0x470c79`, `0x470c90`, `0x470ce0`
Confidence: inferred
Proof: none
Test: none

Most callees are not read. `0x5ec480(race, sex)` returns a
record whose `+0x08` path must be non-empty (`0x470b80` to `0x470b8e`). `0x476000` takes the old
character (`0x470bb7`), `0x475fb0` makes the new one (`0x470bc2`), `0x707350` loads the path into the
frame's scene (`0x470bdf`) and the record's `+0x20`; null returns
(`0x470be9`). `0x7121a0` gets −1, 0, −1, 0, 1.0, 1, 1. Then `0x476b90(record)` (`0x470c2e`),
`0x470a50` (`0x470c33`), `0x4706b0`, `0x470800`, `0x477860(1)` (`0x470c59`), `0x712f70`
(`0x470c79`). The callers' `dl` is never read (`0x470b73`). `0x470bfa` stores
`0x470c90` at the model's `+0x70`; it re-issues the animation call only when its second stack
argument is 0 (`0x470c98`), and the one `call [reg+0x70]` found, `0x7122c1`, pushes 1 there.
`0x470c09` stores `0x470ce0` at `+0x3bc`; it stores 1 at `+0x1b0` of `edx`.

## Reset and selection

`ResetCharCustomize` needs a frame: it calls `0x7130a0(0)` on the frame's model, draws a random
record, sets class 1, builds, stores the list index of the character's race (−1 if absent), and
the name generator learns the character's race and sex. Race index `i` off the list or already
selected does nothing; else `i` is stored, the record drawn for that race with the character's
sex, class index 0 of the old list selected, the character built, and the generator learns race
`i` with the character's sex. Sex `s` of 3 or more, or the character's, does nothing; else it
draws and builds for the character's race, and the generator learns the selected race with `s`.

Addresses: `0x470900`, `0x470913`, `0x470922`, `0x4709ed`, `0x470a08`, `0x470a3e`, `0x470cf0`, `0x470cff`, `0x470d0b`, `0x470df4`, `0x470e30`, `0x470e3c`, `0x470e4e`, `0x470f45`
Confidence: verified
Proof: one
Test: none

`0x470922` calls `0x7130a0`. Each builder stores 16
dwords at the record's `+0x12c`: 1, 101, 201, 301, 401, 501, 601, 702, then 801 by 100 to 1501.
`0x4709ed` stores 1, `0x470a08` −1; `0x470a3e` calls `0x475480(race, sex)`. `0x470cff` `jae` and `0x470d0b` `je` gate the race; `0x470df4` calls `0x470f50(0)` before
the build. `0x470e3c` `cmp esi,3; jae` and `0x470e4e` `je` gate the sex; `0x470f45` passes
`[0xb42060 + 4·[0xb42068]]` and `s`, whether or not the build ran. Indices compare unsigned.

## Class and starting outfit

Class index `i` on the list gives entry `i`'s id; a new id is stored and the character dressed
again: slots 0 to 11 unequipped, `0x47a310` called three times, and each slot of the first
`CharStartOutfit` row matching race, class and sex whose display id is above 0 equipped with its
inventory type and that id.

Addresses: `0x470f50`, `0x470f60`, `0x470f6f`, `0x470800`, `0x470817`, `0x470898`, `0x4708a4`, `0x4708b3`, `0x4708d4`, `0x4708e1`
Confidence: verified
Proof: one
Test: none

`0x470f50` `cmp ecx,[0xb42080]; jae` returns; `0x470f60` reads the record's `+0x00` with no
null test; `0x470f6f` tail-jumps to `0x470800`, whose `0x470817` calls `0x478960(i)`.
`0xc0df50` is the CharStartOutfit store, stride `0x98`; `0x470898`, `0x4708a4` and `0x4708b3`
compare bytes `+4`, `+5`, `+6`. The display ids start at `+0x38`; `jle` skips 0 or less
(`0x4708d4`); `0x4708e1` calls `0x479010` with the dword `0x30` further on, then the id.

## Cycling a customization

`CycleCharCustomization(index, delta)` moves customization `index` by `delta`. A delta of 0 or
an index outside 1 to 5 does nothing. The current value plus the delta, as a signed 32-bit sum,
becomes `count − 1` when negative, then 0 when at or above the count, and goes to the setter.
Hair colour counts the current style's colours.

| index | field | count | setter |
|---|---|---|---|
| 1 skin | `+0x24` | `0x47a440` | `0x478790(v, 1)` |
| 2 face | `+0x2c` | `0x47a4a0` | `0x4785b0(v, 1)` |
| 3 hair style | `+0x34` | `0x47a3e0` | `0x4784c0(v)` |
| 4 hair colour | `+0x20` | `0x47a400` | `0x4783f0(v, 1)` |
| 5 facial hair | `+0x30` | `0x47a4c0` | `0x478660(v, 1)` |

Addresses: `0x470f80`, `0x470f88`, `0x470f8e`, `0x470fa2`, `0x47106c`, `0x470fb9`, `0x470fc2`, `0x471019`
Confidence: verified
Proof: one
Test: none

`0x471890` passes `index − 1` in `ecx`, the delta in `edx`.
`0x470f88` returns for 0; `0x470f8e` `cmp edi,4; ja`; `0x470fa2` jumps through `0x47106c`:
`0x470fa9`, `0x470fcf`, `0x470ff5`, `0x471019`, `0x471043`. Each case: `add edx,esi; jns`, else
`lea edx,[eax-1]` (`0x470fb9`); `cmp edx,eax; jl`, else `xor edx,edx` (`0x470fc2`). `0x471019`
pushes `+0x34` for `0x47a400`.

## Randomizing and counts

`RandomizeCharCustomization` draws five values, sets them with no build (skin, hair colour, hair
style, face, facial hair; flag 1 only for the last) and dresses the character again. `HasCharCustomization(index)` returns 1 when that
customization's count is above 1, unsigned, else nil, and nil for an index outside 1 to 5.

Addresses: `0x471080`, `0x47114f`, `0x471162`, `0x4711ae`, `0x4711b3`, `0x471270`, `0x471273`, `0x4712e0`, `0x47185c`
Confidence: verified
Proof: one
Test: none

`0x471080` copies `+0x18` and `+0x1c` into its record and calls `0x4703c0` (`0x47114f`); the
setters run from `0x471162` to `0x4711ae`, then `0x470800` (`0x4711b3`). `0x471273` `cmp ecx,4;
ja` returns 0; the table `0x4712e0` reads `0x47127c`, `0x47128e`, `0x4712a0`, `0x4712b2`,
`0x4712cb`, the cycle's five counts. `0x47185c` `cmp eax,1; jbe` pushes nil, else 1.0.

## Facing in degrees

`GetCharacterCreateFacing` returns the stored float times 57.29578 as a double.
`SetCharacterCreateFacing(degrees)` multiplies by 0.017453292, rounds to a float, stores it with
no wrap, clamp or NaN test, passes it, when the character has a model, to the model's transform
setter with the origin, axis `(0, 0, 1)` and scale `(1, 1, 1)`, and returns its last argument.

Addresses: `0x471910`, `0x471919`, `0x471930`, `0x47195f`, `0x471966`, `0x47196e`, `0x470a50`, `0x470a59`, `0x470abc`
Confidence: verified
Proof: one
Test: `glue::lua_deg2rad_difftest`, `glue::lua_rad2deg_difftest`

`0x471919` `fmul dword` 57.29578 (`0x42652ee1`) on the stored float, then `fstp qword`: a product
of two floats is exact at 53 bits. `0x47195f` multiplies `lua_tonumber`'s double by
0.017453292 (`0x3c8efa35`), rounding to 53 bits, and `0x471966` `fstp dword`. `0x470a59`
stores before any test; `0x470abc` calls `0x710650` on the character's `+0x38` with those three
vectors and the angle. `0x47196e` returns 1 with nothing pushed.
`lua_deg2rad_difftest` runs `0x47195a` to `0x471969`; `lua_rad2deg_difftest` runs `0x471910` to
`0x471922`.

## Frame and background

`SetCharCustomizeFrame(name)` stores the frame of that name when one answers to the model type
id, else keeps the old one. `SetCharCustomizeBackground(file)` needs a frame and a non-empty
name: it calls the frame's set-model slot, then passes the character's model and the frame's
model at `+0x318`, when both exist, to `0x712f70`. `UpdateCustomizationScene` calls
`0x477860(1)` on the character and `0x710450(1, 1)` on the frame's model.

Addresses: `0x471330`, `0x471353`, `0x471380`, `0x4707a0`, `0x4713a0`, `0x4707b0`, `0x4707c8`, `0x4707ed`, `0x471300`, `0x471308`, `0x471320`
Confidence: verified
Proof: one
Test: none

`0x471353` takes the id from `0xcf0c5c`, assigning one on first use; `0x76c760(name, id)` finds
the frame (`0x471380`) and a hit goes to `0x4707a0`. `0x4707b0` tests the frame, the pointer and
the first byte, then `call [edx+0x94]` (`0x4707c8`). `0x4707ed` is the `0x712f70` call; it
reads the character's `+0x38` without testing the character. `0x471300` calls `0x477860`
(`0x471308`) and `0x710450` when `+0x318` is set (`0x471320`).

## Query functions

These read no argument. Strings are the locale's column; a null string pushes nil.

| function | address | returns |
|---|---|---|
| `GetNameForRace` | `0x4713f0` | character's race: name, `+0x3c`; nil, nil |
| `GetFactionForRace` | `0x471450` | first matching group: name, `InternalName`; nil, nil |
| `GetAvailableRaces` | `0x471520` | name, `+0x3c` per listed race |
| `GetClassesForRace` | `0x471560` | name, `+0x38` per listed class |
| `GetHairCustomization` | `0x4715d0` | selected race's `+0x70`, else `NORMAL` |
| `GetFacialHairCustomization` | `0x471620` | selected race's `+0x68 + 4·sex`, else `NORMAL` |
| `GetSelectedRace` | `0x471680` | selected index + 1 |
| `GetSelectedSex` | `0x4716b0` | character's sex + 1 |
| `GetSelectedClass` | `0x4716e0` | class name, `+0x38`; nil, nil |
| `GetCharacterCreateFacing` | `0x471910` | facing in degrees |
| `GetRandomName` | `0x471980` | a name |
| `HasCharCustomization` | `0x471820` | 1 or nil |

Addresses: `0x470ad0`, `0x470af0`, `0x470b20`, `0x470b50`, `0x4714b0`, `0x4714cd`, `0x47160a`, `0x471696`, `0x838484`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

Names are `ChrRaces` `+0x44`, `ChrClasses` `+0x14`, `FactionGroup` `+0x0c`, plus four times the
locale index `0xc0e080`. `0x470ad0` gives the race id at an index, 0 at or past the count; the
hair queries apply it to `0xb42068`. `0x470b20` and `0x470af0` give a listed race's name and
`+0x3c`, `0x470b50` a class pointer. `0x4714b0` to `0x4714cd` take the first group with a
nonzero index whose bit the template holds. `0x47160a` pushes `0x838484`; `0x471696` `fild
qword` of the index plus one over a zero high dword.

## Command functions

These push nothing but `SetCharacterCreateFacing`. A number must pass `lua_isnumber`, else
the usage string is raised, and is truncated by `__ftol`; an index is decreased by one. A string
must pass `lua_isstring`.

| function | address | arguments | body |
|---|---|---|---|
| `SetCharCustomizeFrame` | `0x471330` | name | `0x4707a0` |
| `SetCharCustomizeBackground` | `0x4713a0` | file | `0x4707b0` |
| `ResetCharCustomize` | `0x4713e0` | none | `0x470900` |
| `SetSelectedRace` | `0x471740` | index | `0x470cf0` |
| `SetSelectedSex` | `0x471780` | index | `0x470e30` |
| `SetSelectedClass` | `0x4717c0` | index | `0x470f50` |
| `UpdateCustomizationBackground` | `0x471800` | none | `0x470cf0` on the selected index, refused |
| `UpdateCustomizationScene` | `0x471810` | none | `0x471300` |
| `CycleCharCustomization` | `0x471890` | index, delta | `0x470f80` |
| `RandomizeCharCustomization` | `0x471900` | none | `0x471080` |
| `SetCharacterCreateFacing` | `0x471930` | degrees | `0x470a50` |
| `CreateCharacter` | `0x4719e0` | name | `0x4711d0`, no usage error |

Addresses: `0x8380f8`, `0x838428`, `0x838454`, `0x83848c`, `0x8384ac`, `0x8384cc`, `0x8384ec`, `0x838510`, `0x83853c`, `0x471774`
Confidence: verified
Proof: one
Test: none

`0x8380f8` holds the 24 `{name, function}` pairs in address order. The usage strings are
`0x838428`, `0x838454`, `0x83848c`, `0x8384ac`, `0x8384cc`, `0x8384ec`, `0x838510` and
`0x83853c`, in table order. `0x471774` `dec eax`, so index 0 becomes `0xffffffff` and fails
every unsigned gate. All return 0 but `0x471930`.

## The create request

`CreateCharacter(name)` reads its argument with `lua_tostring`. A null name fails as
`CHAR_NAME_NO_NAME` unchecked; otherwise the name check runs. A failure raises
`OPEN_STATUS_DIALOG` with `OKAY` and the code's text and sends nothing. A pass copies the name
into 48 bytes and nine bytes after it: race, class, sex, then the character's `+0x24`, `+0x2c`,
`+0x34`, `+0x20`, `+0x30` and 0, which glue's create call sends in that order as opcode `0x36`.

Addresses: `0x4719e0`, `0x4711d0`, `0x4711db`, `0x4711e2`, `0x47120c`, `0x471219`, `0x471260`, `0x471264`
Confidence: verified
Proof: one
Test: none

`0x4719e0` keeps the second of two `lua_tostring` calls. `0x4711db`
loads 0x45 for a null pointer; `0x4711e2` calls `CheckCharacterName`, and 0x52 passes.
`0x47120c` is `SignalEvent2(3, "%s%s", "OKAY", text)`. `0x471219` is `SStrCopy(buf, name,
0x30)`; `ebp−0xc` to `ebp−0x4` take `+0x18`, `0x8380f0`, `+0x1c`, the five fields and 0
(`0x471260`). `0x471264` calls `0x46b290`, which reaches `0x5aac50`.

## Name rows for a race

The generator learns a run of `NameGen` rows: from the first, in file order, whose race and sex
match, through every following row while the race alone matches, so a run can hold the other
sex's names. Its name list is emptied first; with no matching row it stays empty and the
statistics stay as they were.

Addresses: `0x475480`, `0x47548b`, `0x4754c9`, `0x4754d1`, `0x4754da`, `0x4754f3`, `0x4755b0`, `0x4755f0`, `0x475534`, `0x475548`
Confidence: verified
Proof: one
Test: none

`0xc0da78` is the NameGen store, stride `0x10`: name `+0x04`, race `+0x08`, sex `+0x0c`.
`0x47548b` calls the free `0x4755b0`. `0x4754c9` and `0x4754d1` test race and sex until
a match; then `0x4754da` tests the race alone and leaves on a mismatch. `0x4754f3` returns with
no match. `0x4755f0(n)` sets the capacity to `n` and reallocates to `4n` bytes, else allocates,
copies and frees; it has a second caller outside glue. `0x475534` stores each name pointer;
`0x475548` calls `0x4752e0`.

## Name statistics

Each name becomes `_` and the name in 64 bytes, lower-cased; every window of up to three bytes
starting at 0 to length − 2 is a 4-byte key, the last window two bytes. A key seen again counts
one more, a new one is appended with count 1, and `qsort` then sorts by key. The array is sized
for 10,000 records; the append has no bound.

Addresses: `0x4752e0`, `0x475317`, `0x475345`, `0x475361`, `0x4753a8`, `0x4753fb`, `0x475420`, `0x4755d0`, `0x475680`
Confidence: verified
Proof: one
Test: none

Records are `DictionaryRecord` (tag `0x8383fc`): key `+0x00`, count `+0x04`. `0x4752f5` frees through
`0x4755d0`; `0x475317` calls `0x475680(10000, 1)`, which takes a chunk of 32 and rounds up, so
10,016 fit. `0x475345` is `SStrPrintf(tmp, 0x40, "_%s", name)`, `0x475351` the C runtime's
`_strlwr`. `0x475361` loops below `length − 1`; `0x475383` is `SStrCopy(key, tmp + k, 4)`.
`0x4753a8` compares whole keys with `SStrCmp`; `0x47540a` counts a hit. `0x4753fb` is
`qsort(data, n, 8, 0x475420)`, comparing keys with `SStrCmp`; the generator's `+0x1c` counts.

## Generating a name

A name grows in a zeroed buffer of `size` bytes. While it is empty the candidates are the keys
beginning with `_`, after that those whose first two bytes are its last two. With `total` their
summed counts and `r` one advance of `0x882664`, `x` is the high dword of `total · r`; the first
candidate in key order whose count exceeds what remains of `x` wins. An empty buffer takes the
key; otherwise the key's third byte is appended, and a two-byte key ends the name, as do no
candidate and `size − 1` bytes. The first byte is then removed and the next upper-cased.

Addresses: `0x475130`, `0x475183`, `0x47519c`, `0x4751ab`, `0x4751b8`, `0x4751c2`, `0x4751e2`, `0x47521b`, `0x475246`, `0x4752a9`, `0x4752be`, `0x4752ca`
Confidence: verified
Proof: one
Test: none

`0x475149` zeroes the buffer. `0x475183` `cmp byte [edx],0x5f`; `0x47521b` is `SStrCmp(key, buf
+ len − 2, 2)`. Each scan keeps the lowest and highest match and the count sum, stopping at a
mismatch once the highest exceeds the lowest (`0x47519c`, `0x475246`); `0x4751ab` ends when the
highest is below the lowest, though with no records it reads record 0. `0x4751b8` calls
`0x4531e0` on `0x882664`, `0x4751c2` `0x455c70(total, r)`; `0x4751e2` `cmp eax,edx; jl` takes
the key, else subtracts. `0x47527b` copies, `0x47528f` tests byte 2, `0x47529d` appends with
`SStrPack`, `0x4752a9` loops below `size − 1`. `0x4752be` is `memcpy(buf, buf + 1, size − 1)`,
`0x4752ca` `toupper` of the sign-extended byte.

## Random name acceptance

`GetRandomName` generates names into 14 bytes until one passes the name check and is 4 to 10
bytes long, and returns it; it has no other exit. Glue's teardown frees and nulls the
generator's arrays but keeps its record count, so a name drawn before the generator learns again
reads records through a null pointer.

Addresses: `0x471980`, `0x471990`, `0x4711c0`, `0x4719ab`, `0x4719b3`, `0x4719b5`, `0x4719bd`, `0x470640`
Confidence: verified
Proof: one
Test: none

`0x471990` `mov edx,0xe`; `0x4711c0` calls `0x475130(buf, 14)` on `0xb420b8`. `0x4719ab` calls
`CheckCharacterName`; `0x4719b3` retries unless 0x52, `0x4719b5` below 4, `0x4719bd` above 10,
unsigned. `0x470640` zeroes `+0x00` to `+0x14`; only `0x4752e0` writes `+0x1c`.

## ModelFFX vtables and stubs

`CSimpleModelFFX` (988 bytes) has two vtables, `0x803960` at `+0x00` and `0x803934` at `+0x24`.
Besides its destructor and a thunk, they hold seven inherited methods here: three of the model
class, answering type queries as `Model`, and four stubs shared with dozens of frame vtables.
The addresses 0x803938, 0x803958 and 0x803970 are slots inside them, not vtables.

| function | table, slot | does |
|---|---|---|
| `0x470040` | `0x803960`, 0 | stores both vtables, calls `0x76ca50`, frees when bit 0 is set |
| `0x470080` | `0x803934`, 1 | subtracts `0x24` from `this`, jumps to `0x470040` |
| `0x46ff70` | `0x803960`, 1 | returns `this+0x98` |
| `0x46ffa0` | `0x803960`, 4 | 1 for one of three type ids |
| `0x46ffd0` | `0x803960`, 6 | 1 for `Model`, `Frame` or `Region`, any case |
| `0x470030` | `0x803960`, 7 | returns `Model` |
| `0x46ff80`, `0x46ff90` | `0x803960`, 21, 22 | return 0, pop one argument |
| `0x46ff60` | `0x803934`, 9 | returns 0 |

Addresses: `0x46ff60`, `0x46ff70`, `0x46ff80`, `0x46ff90`, `0x46ffa0`, `0x46ffd0`, `0x470030`, `0x470040`, `0x470080`, `0x803960`, `0x803934`
Confidence: verified
Proof: one
Test: none

The constructor `0x46fa30` calls `0x76c8e0` and stores both tables; glue registers the factory
`0x46fa00`, which allocates `0x3dc` bytes, as `ModelFFX`. A dword scan
finds `0x803960` and `0x803934` twice each and the three slot addresses nowhere. `0x76c8e0`'s
primary table holds `0x46ffa0`, `0x46ffd0` and `0x470030` at the same slots. The ids are
three globals, the first `0xcf0c5c`; the names are read through three data pointers whose file
values point to the strings. The stubs have 35, 33, 26 and 26 stored copies and no `call` or `jmp`.

## Registration and teardown

Glue registers the 24 functions of `0x8380f8` as Lua globals in table order and removes them by
name at its teardown, which also destroys the character and frees the name generator's arrays.
A static initializer zeroes the generator's first seven dwords at load.

Addresses: `0x471a20`, `0x471a50`, `0x470640`, `0x470652`, `0x475110`, `0xb420b8`
Confidence: verified
Proof: one
Test: none

`0x471a20` passes each pair to `0x704120` for `esi` below `0xc0` by 8; `0x471a50` each name to
`0x704160`. `0x470640` calls `0x476000` on the character, stores 0 (`0x470652`), frees the
generator's `+0x08` and `+0x14` and zeroes `+0x00` to `+0x14`; the lists, the frame and the
facing stay. `0x475110` zeroes `+0x00` to `+0x18` of `ecx`, `0xb420b8` from a static
initializer's tail jump.
