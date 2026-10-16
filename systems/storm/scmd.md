# storm: command-line switches

Storm's `SCmd.cpp` keeps the switches a program registers and the values the command line gives
them, as nodes on two lists. The client registers two tables: fifteen base switches from
`0x419c70` and its own seventeen from `0x403ba0` (`console/screen`, Console key). Code reads a
switch by its id through `SCmdGetNum`, `SCmdGetBool` and `SCmdGetString`, and the client's own
wrappers `CmdLineGetBool` and `CmdLineGetString` sit beside its registration.

Roots: `0x64f200`, `0x64f220`, `0x64f270`, `0x64fda0`, `0x64fdc0`

## Reading a switch by id

`SCmdGetNum(id)` walks the list at `0xc53f24`, then the one at `0xc53f30`, and returns the
value word `+0x3c` of the first node whose id `+0xc` equals `id`, or 0 when none does.
`SCmdGetBool(id)` is `SCmdGetNum(id) != 0`. `SCmdGetString(id, buffer, size)` raises error 0x57
and returns 0 for a null buffer or a zero size, writing 0 to `buffer[0]` first whenever the buffer
is not null; otherwise it finds the node the same way, copies the string `+0x3c` points to with
`SStrCopy(buffer, string, size)` when it is not null, and returns 1, or returns 0 with the buffer
empty when no node has the id.

Addresses: `0x64f200`, `0x64f220`, `0x64f270`, `0x64f286`, `0x64f2c8`, `0x64f2e0`, `0x64f2ea`, `0xc53f24`, `0xc53f30`
Confidence: verified
Proof: one
Test: none

`0x64f220` and `0x64f270` share one loop: `ecx` counts the two heads (`cmp ecx,1; jle`), a head's
first node is its `+0x8`, a node with bit 0 set or 0 ends the list, and `+0x4` is the next node;
`0x64f2c8` compares the id. `0x64f281` stores the 0 before the size test at `0x64f284`, and `0x64f286`
pushes 0x57 for `SErrSetLastError`. `0x64f2e0` loads `+0x3c`, `0x64f2ea` copies, and the found path
returns 1 whether or not a string was there. `0x64f200` turns the number into 0 or 1 with
`neg; sbb; neg`. The registrar `0x64fe20` puts a switch with either flag bit of
0x3000000 on `0xc53f24` and any other on `0xc53f30` (`0x64fe5b`, `0x64fead`). The names `SCmdGetNum` and `SCmdGetBool` are the community's; `0x64f270` sits beside
them, shares their lookup and is the string reader `CmdLineGetString` wraps as `CmdLineGetBool`
wraps `SCmdGetBool`, so it takes the same `SCmdGet` form.

## The client's switch readers

`CmdLineGetBool(id)` is `SCmdGetBool(id)`. `CmdLineGetString(id)` reads the switch into the one
260-byte buffer `0x883c90` with `SCmdGetString` and returns that buffer, so a switch the command
line did not give reads as an empty string, and each call overwrites the previous answer.
`0x419c70` registers the fifteen base switches of the table `0x801088` and then parses the process
command line through `0x64fda0`. The base ids are 0 `d3d`, 1 `datadir`, 2 `nolagfix`, 3 `loadfile`, 4 `gametype`,
5 `opengl`, 6 `swtnl`, 7 `timedemo`, 8 `rez`, 9 `depth`, 10 `detail`, 11 `sound`, 12 `fullscreen`,
13 `22050`, 14 `nowarnings`; 1, 3, 4 and 8 to 11 carry the flag 0x20000 and take a string.

Addresses: `0x419c10`, `0x419c20`, `0x419c70`, `0x883c90`, `0x801088`, `0x64fda0`, `0x64fdc0`, `0x403ba0`
Confidence: verified
Proof: one
Test: none

`0x419c20` pushes 0x104, `0x883c90` and the id in `ecx`, calls `0x64f270`, and returns `0x883c90`
whatever it answered. `0x419c70` passes the table and the count 0xf to `0x64fdc0`, the registrar
`0x403ba0` uses for the client's table, and each 16-byte record holds the flags, the id, the name
pointer and a zero. `0x64fda0(a, b)` passes `GetCommandLineA()`, 1, `a` and `b` (here 0 and 0) to
`0x64f3c0`. The callers seen read switch 1 in `0x402210` and switch 0x26, `gxoverride`, in
`ConsoleDeviceInitialize` (`0x63a6b5`).

## Switch name and number parsing

`0x64fac0(text, node, minimum)` walks a switch list from `node` and returns the node whose name is
the longest prefix of `text`, of at least `minimum` and at most `strlen(text)` characters, or 0.
A node with flag bit 0x100 compares case-sensitively (`strncmp`), any other ignoring case
(`_strnicmp`); of two names of equal length the first in the list wins. `0x64f940(node, text,
&length)` converts `text` to the node's value `+0x3c` with `strtol` when the node's flag bit 0 is
set and `strtoul` otherwise, both in base 0, stores the number of characters the conversion used in
`length`, and copies the low `min(size, 4)` bytes of the value to the node's output `+0x2c` when it
has one, `size` being `+0x30`.

Addresses: `0x64fac0`, `0x64fae6`, `0x64faea`, `0x64faf2`, `0x64fb19`, `0x64f940`, `0x64f94c`, `0x64f977`, `0x64f986`, `0x64f99a`
Confidence: verified
Proof: one
Test: none

`0x64fac0` starts its bound at `minimum − 1` and skips a node unless its length `+0x20` is above the
bound (`0x64fae6`, `jle`) and not above the text's length (`0x64faea`, `jg`); `0x64faf2` tests the
flag, and a match raises the bound to that length (`0x64fb19`), so a later node must be strictly
longer. `0x64f94c` tests bit 0; the end pointer starts at 0, so `0x64f986` stores `end − text`, and the
`SStrLen` fallback runs only for a null end pointer, which the C runtime does not leave. `0x64f99a`
tests the output pointer and the copy is `rep movsd` then `rep movsb` of the clamped size.
