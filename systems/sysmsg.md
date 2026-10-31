# sysmsg

The system message printers: `SysMessage.cpp`, `0x44cb20` to `0x44cd70`. A caller reports a level
and a text; the client checks the level, formats the text into a stack buffer and drops it. What
the file keeps is a colour and duration table for the four levels, the display switch and level
window the console commands set, and a registration stub for the on-screen display. Static data
is at `0x835908` to `0x835930` and `0xb05d30`. The lower band `0x44c040` to `0x44cb20` is
`Texture.cpp`, not this file; the next object file starts at `0x44cd80`.

Roots: `0x44cbd0`, `0x44cb40`, `0x44cca0`, `0x44cd10`, `0x44cd40`, `0x44cc00`, `0x44cc20`, `0x44cc40`, `0x44cc80`, `0x44cd70`

## Printing a message

`0x44cbd0(level, channel, format, ...)` returns 0 after `SErrSetLastError(0x57)` when `format` is
null, and otherwise calls `0x44cb90` with a pointer to the variable arguments. `0x44cb90` writes
the text with `__vsnprintf` into a 256-byte stack buffer, sets its last byte to 0, and returns
`0x44cb20(text, level)`, which returns 1 when `text` is non-null and `level < 4` (signed), and
otherwise sets error `0x57` and returns 0. The buffer is not stored or shown, so a print does
nothing but the check. 35 call sites reach `0x44cbd0`; 31 reach `0x44cb20`.

Addresses: `0x44cbd0`, `0x44cb90`, `0x44cb20`
Confidence: verified
Proof: one
Test: none

`0x44cb20` takes the text in `ecx`, the level in `edx` and one stack argument it never reads
(`ret 4`); `0x44cb90` pushes the channel there, so the channel has no effect. The buffer is
`[ebp-0x100]` (`0x44cba1`) and the terminator store is `0x44cbc2`.

## Status object check

`0x44cb40(status, _)` returns 1 at once when `CStatus::IsEmpty` (`0x41a150`) of the status holds.
Otherwise it takes the status' error text with `CStatus::GetErrorStrAlloc` (`0x41a2c0`, argument
0), calls `0x44cb20(text, [status+0x10])`, frees the text with `SMemFree` (file tag `0x835934`,
line 0xae) and returns the check's result. Its second argument is pushed for the check and
never read. Its 10 callers include the cache-update timer `0x402a20`, which calls it with a
local status object.

Addresses: `0x44cb40`, `0x41a150`, `0x41a2c0`, `0x646430`, `0x402a20`
Confidence: verified
Proof: one
Test: none

The call order is `0x44cb46`, `0x44cb5c`, `0x44cb69`, `0x44cb7d`.

## Colour table

Four message levels have a colour and a duration, 8 bytes each from `0x835908`: bytes R, G, B, a
pad byte, then the duration as a float.

| level | R, G, B | duration |
|---|---|---|
| 0 | 255, 255, 255 | 10.0 |
| 1 | 255, 255, 127 | 15.0 |
| 2 | 255, 127, 127 | 15.0 |
| 3 | 127, 255, 255 | 20.0 |

`0x44cca0(level, &r, &g, &b)` stores each byte as `(float)byte · f32(1/255)` (`0x3b808081`,
at `0x8026c8`); the product is formed in double and rounded by the 32-bit store. The level is
not range-checked. `0x44cd00(level)` loads the duration; nothing calls it.

Addresses: `0x835908`, `0x44cca0`, `0x8026c8`, `0x44cd00`
Confidence: verified
Proof: one
Test: `sysmsg::color_lookup_difftest`

Each channel is `movzx byte [level*8+0x835908+k]`, `fild`, `fmul [0x8026c8]`, `fstp` (`0x44cca4`
to `0x44ccea`). The one caller is `0x409200`, the display callback (see below): it copies the text into a
slot of ten records of 0x10c bytes from `0x882e28`, multiplies each channel by 255.0
(`0x7ffe58`), converts with `__ftol`, stores B, G, R and 0xff at `+0x108` to `+0x10b`, and
advances the slot index modulo 10. The test runs `0x44cca0` whole for every byte value in each
record.

## Display switch and window

Four globals hold the console's choices: the switch `0x835928`, the window low `0xb05d30` and
high `0x83592c`, and the selector `0x835930`. `0x44cd10`, called once from `0x402ad0`, sets
them to 1, 0, 3 and −1. Setting the low bound raises the high to it when below; setting the high
lowers the low to it when above (`0x44cc20`, `0x44cc40`). `0x44cd40` clears the switch and
tail-calls `0x44cd60`, a `ret`; it is the tail jump of `DestroyClientSystems` (`0x4036c0`). The
console handlers `0x402f60` and `0x4030b0` call the window setters only for values 0 to 3
(`0x402f6e`, `0x402f73`, `0x4030be`, `0x4030c3`). The getters `0x44cc60`, `0x44cc70` and
`0x44cc90` have one caller each; `0x44cc10` and `0x44cd50` have none.

Addresses: `0x835928`, `0xb05d30`, `0x83592c`, `0x835930`, `0x44cd10`, `0x44cc00`, `0x44cc10`, `0x44cc20`, `0x44cc40`, `0x44cc60`, `0x44cc70`, `0x44cc80`, `0x44cc90`, `0x44cd40`, `0x44cd50`, `0x44cd60`
Confidence: verified
Proof: one
Test: none

Census: the `e8` and `e9` targets and the operand mentions of each address. Nothing reads the
switch except `0x44cc10`, so it gates nothing in this file.

## Display callback registration

`0x44cd70(callback)` is a `ret`, so it keeps nothing. `0x409040` passes it `0x409200` when its
enable word is set (`0x4090e8`), and `0x409100` passes it 0 (`0x409112`). Nothing calls or
jumps to `0x409200`, and its address occurs once in the image, as that immediate (`0x4090e4`),
so it never runs, nor the colour lookup it alone calls: no colour or duration reaches the
screen.

Addresses: `0x44cd70`, `0x409040`, `0x409100`, `0x409200`, `0x4090e8`, `0x409112`
Confidence: verified
Proof: one
Test: none

Both call sites load `ecx` just before the call (`0x4090e3`, `0x409109`).

## Error display settings

`ClientRegisterConsoleCommands` registers `Errors` with the callback `0x402ee0` and `ShowErrors` with
`0x402f20`. Each reads the new value with `SStrToInt`. For `Errors`, a non-zero value calls `0x409170`
and prints `Error display enabled`, zero calls `0x4091a0` and prints `Error display disabled`; for
`ShowErrors`, non-zero calls `0x4091c0` with `Error display shown`, zero `0x4091e0` with
`Error display hidden`. Both print through `ConsoleWrite` in class 0, then pass the integer to
`SysMsgSetEnabled` and return 1, so either setting sets the display switch. `0x403110` prints the
filter: `Now filtering: all messages` when `SysMsgGetSelected` is −1, else `Now filtering: ` and a
name for each set bit: 0x1 `general`, 0x2 `world`, 0x4 `ui`, 0x8 `animation`, 0x10 `models`,
0x20 `objects`, 0x80 `movement`. Bit 0x40 prints nothing.

Addresses: `0x402ee0`, `0x402f20`, `0x403110`, `0x402c86`, `0x402ca9`, `0x402f12`, `0x402f52`, `0x40311e`, `0x4031cd`, `0x409170`, `0x4091a0`, `0x4091c0`, `0x4091e0`
Confidence: verified
Proof: one
Test: none

The pushes `0x402c86` and `0x402ca9` pass the callbacks to the registration; a callback reads the
new text at `[ebp+8]` and returns with `mov al,1; ret 8`. `0x409170` sets a byte flag when it is
clear and two fields of its file are set; `0x4091a0` clears it; `0x4091c0` clears a second flag and
`0x4091e0` sets it. `0x403110` packs the names into an 80-byte buffer with `SStrPack` (limit 0x50),
testing `bl` against 1 to 0x20 and its sign (`0x4031cd`, `jns`) for 0x80; nothing tests 0x40. It
prints with `ConsolePrintf`. Its one caller is `0x4030f0`. The two callbacks are named for their
settings, `ErrorsCallback` and `ShowErrorsCallback`, as `GammaCallback` is.

## Error level settings

`ErrorLevelMin` (default `1`) and `ErrorLevelMax` (default `3`) are registered with the callbacks `0x402f60` and
`0x4030b0`. Each reads the new text with `SStrToInt` and takes it only for a value of 0 to 3. `0x402f60` then calls
`SysMsgSetMinDisplayLevel`, which raises the high level to the value when it is below; `0x4030b0` calls
`SysMsgSetMaxDisplayLevel`, which lowers the low level to it when it is above. Both then call `0x402fa0` and return 1.
Any other value is refused: the callback writes `%i is not valid, valid values are 0 - %i` with the value and 3 in
console class 0 and returns 0, which vetoes the new text.

Addresses: `0x402f60`, `0x4030b0`, `0x402ccc`, `0x402cef`, `0x402f6e`, `0x402f73`, `0x402f77`, `0x402f7c`, `0x402f87`, `0x402f91`, `0x402f99`, `0x4030c7`, `0x4030cc`, `0x4030d7`, `0x4030e1`, `0x63cd00`, `0x64ac60`, `0x44cc20`, `0x44cc40`
Confidence: verified
Proof: one
Test: none

`0x402ccc` and `0x402cef` push the callbacks to the registration. `0x402f66` pushes the text to `0x64ac60`; `0x402f6e`
and `0x402f73` bound the signed result (`jl`, `cmp eax,4; jge`); `0x402f77` calls `0x44cc20`, `0x402f7c` the printer and
`0x402f81` returns `al` 1 with `ret 8`. The refusal pushes 3, the value, 0 and the format (`0x402f87` to `0x402f91`), calls
`ConsoleWriteA` (`0x63cd00`, class 0), drops 16 bytes and returns `al` 0 (`0x402f99`). `0x4030b0` is the same with
`0x44cc40` (`0x4030c7`) and the printer call at `0x4030cc`. The pair rule lives in the setters, not the callbacks.

## Error level status line

`0x402fa0` reads the window's low and high levels (`0x44cc60`, `0x44cc70`) and writes one line to the console in class
0, whose colour is `ffffffff` until a `fontcolor` command changes it (`console/screen#colour-classes`):

| low, high | line |
|---|---|
| 0, 3 | `Displaying all system messages` |
| equal | `Displaying only ` and the level's name |
| low below high | `Displaying `, the low level's name, ` through ` and the high level's name |

The names are 0 `informational messages`, 1 `warnings`, 2 `errors` and 3 `fatal errors`. A high level of 0 can meet a
low level of 0 only in the equal case, so the tail never names it.

Addresses: `0x402fa0`, `0x402fba`, `0x402fc6`, `0x402fd1`, `0x402fdc`, `0x402ff3`, `0x402ffd`, `0x40303e`, `0x403043`, `0x403083`, `0x403088`, `0x63cb50`, `0x64a5a0`, `0x64a760`, `0x44cc60`, `0x44cc70`
Confidence: verified
Proof: one
Test: none

`esi` is the low level and `edi` the high. `0x402fba` takes 0 and 3 to the all-messages text, written through `ConsoleWrite`
with `edx` 0 (`0x402fc6`). Otherwise `SStrCopy` puts `Displaying ` in an 80-byte buffer (`0x402fd1` to `0x402fdc`) and equal
levels `SStrPack` `only ` (`0x402ff3`). A low level up to 3 jumps through the table `0x403094` (`0x402ffd`) to its name, packed at
`0x40303e`. `0x403043` compares the levels; unequal ones test the high level and pack ` through warnings`, ` through errors`
or ` through fatal errors`. `0x403083` writes the buffer through `ConsoleWrite` with `edx` 0 (`0x403088`).

## Error filter setting

`ErrorFilter`, default `all`, is a list of filter names separated by tabs, line breaks, double
quotes or spaces, compared ignoring case. Starting from an empty mask, `general`, `world`, `ui`,
`animation`, `models`, `objects` and `movement` set the bits 0x1, 0x2, 0x4, 0x8, 0x10, 0x20 and 0x80,
`all` sets every bit, and `except` makes each later name clear its bit, `all` then giving 0. The mask goes to `SysMsgSetSelected` and the filter is printed. A word that is
none of these prints `Unknown filter %s` and three help lines, leaves the selection unchanged and
vetoes the new value. Only `all` sets 0x40: the word picked for the letter `s` is `objects`,
which a word starting with `s` never equals.

Addresses: `0x4030f0`, `0x403200`, `0x403231`, `0x403257`, `0x40327c`, `0x4032ce`, `0x4033de`, `0x403423`, `0x403433`, `0x402d12`
Confidence: verified
Proof: one
Test: none

`0x402d12` registers `0x4030f0` with the default text `all`. The callback returns 0 when `0x403200`
does and otherwise calls `0x403110` and returns 1. `0x403231` tokenises with `SStrTokenize`;
`0x403257` switches on the first character minus `A` through a byte table after the function, upper
and lower case sharing a case. `all` computes `(except != 0) − 1` (`0x40327c`); `except` sets
its flag (`0x4032ce`); the `s` case (`0x4033de`) compares `objects` and sets 0x40 on a match. The end of
the list calls `SysMsgSetSelected` (`0x403423`); the fault path starts at `0x403433` and returns 0.

## Display layer

`0x409040` builds the on-screen layer once: a font batch, the font of stock text block 1 and that
block's height, and a screen layer over the whole screen, `(0, 0)` to `(1, 1)`, at priority 10.0 with
the paint function `0x4092c0`. It stops at the first of these that fails, and a height of exactly 0
also stops it. With the layer made it passes `0x409200` to `0x44cd70` and sets its flag `0x8838f4`.
`0x409100`, called from `DestroyClientSystems`, clears the flag, passes 0 to `0x44cd70`, destroys
the batch and ten strings, and closes the layer's handle.

Addresses: `0x409040`, `0x409057`, `0x40906e`, `0x409088`, `0x40909e`, `0x4090d5`, `0x8838f4`, `0x409100`, `0x409121`, `0x409147`, `0x40915d`, `0x442b50`, `0x442bd0`, `0x4092c0`
Confidence: verified
Proof: one
Test: none

`0x409057` calls `GxuFontCreateBatch` and `0x40906e` `0x442b50(1)`, whose font `TextBlockGetFontPtr`
takes; `0x409088` stores `0x442bd0(1)`. `0x40909e` is `jnp` after `test ah,0x44`, so
equality (C3 alone) leaves and NaN (C3 and C2) goes on. `0x4090d5` calls `ScrnLayerCreate` with the
rectangle, 10.0 (`0x41200000`), flags 0, the paint function and its out slot. `0x409121`
calls `GxuFontDestroyBatch`, `0x409147` `GxuFontDestroyString` on each of ten 0x10c-byte string slots, and
`0x40915d` jumps to `HandleClose`.
