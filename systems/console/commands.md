# console: commands

The command registry and the commands that ship with it. A command is a name, a handler and a help
string in one category. The registry keeps the commands in a case-insensitive hash table and a list,
parses a typed line into a name and arguments, keeps the 32 latest lines, completes names and prints help.
Every system that offers a console command registers it here; the screen's executor runs it
(`screen#writing-and-executing`). The registry file is one object file with its own table object at `0xc4f848`.

Roots: `0x63f880`, `0x63f9b0`, `0x63f9c0`, `0x63f9e0`, `0x63fb40`, `0x63fc60`, `0x63fdb0`, `0x63fdc0`, `0x6400e0`, `0x640130`, `0x640140`, `0x640160`, `0x63cf70`, `0x6393b0`, `0x6397d0`, `0x639890`, `0x6395c0`, `0x639660`, `0x639700`, `0x638f40`, `0x638fd0`, `0x6390c0`, `0x639300`, `0x639100`, `0x639240`, `0x639250`

## Command records

A command is a `0x24`-byte record under the tag `CONSOLECOMMAND`: the hash of its name at `+0`, the bucket link at `+4`, the list
link at `+0xc`, the name pointer at `+0x14` (the caller's pointer, not a copy), the handler at `+0x18`, the help pointer at `+0x1c` (0
for none) and the category at `+0x20`. The table object is the hash table of `cvar/containers#hash-table-object`
at `0xc4f848`, with the vtable `0x80e268` and the list link offset `0xc`. It starts empty, with mask −1. Records are linked first in
their bucket and last in the list, so the list runs in registration order.

Addresses: `0x80e268`, `0xc4f848`, `0xc4f84c`, `0xc4f850`, `0xc4f854`, `0xc4f858`, `0xc4f85c`, `0xc4f860`, `0xc4f864`, `0xc4f868`, `0xc4f86c`, `0x8655c0`, `0x640260`, `0x6401e0`, `0x6403e0`, `0x640320`, `0x63f2e0`, `0x63f2f0`, `0x63f3b0`, `0x63f3c0`, `0x63f305`, `0x63f32d`, `0x63f38e`, `0x63f398`, `0x63f3a2`, `0x64027b`, `0x6402db`, `0x63fb24`, `0x63fb30`, `0x865494`
Confidence: verified
Proof: one
Test: none

The vtable holds `{0x6401e0 free, 0x640260 allocate, 0x6403e0 deleting destructor, 0x640320 clear}`. `0x640260` allocates `0x24` plus
the extra size under `.?AUCONSOLECOMMAND@@` (`0x64027b`, flag 8 for zero fill), clears the link words, and links the record after the bucket's terminator (`0x6402db`). The slot
function `0x63f2e0` calls `0x63f2f0`, which writes the placeholder offset `0xdddddddd` (`0x63f305`), the vtable (`0x63f32d`) and empties the table; it ends with offset `0xc` (`0x63f38e`),
the terminator linked to itself (`0x63f398`) and the mask −1 (`0x63f3a2`). `0x63f3b0` registers the exit destructor `0x63f3c0`, which unlinks the list and the buckets and frees nothing. `0x63fb24` to `0x63fb30` store the
registration's fields.

## Registering and removing

`ConsoleCommandRegister` (`0x63f9e0`; name in `ecx`, handler in `edx`, category and help on the stack) returns 0 for a name of 64 bytes or more or one already
registered (case-blind), and otherwise adds the command and returns 1. `ConsoleCommandUnregister` (`0x63fb40`) finds a name, unlinks it from both lists and
frees it. The first registration builds four buckets with mask 3; later ones grow the table when a bucket chain passes the limit of
`cvar/containers#growth`.

Addresses: `0x63f9e0`, `0x63fb40`, `0x63f9f7`, `0x63fa07`, `0x63fa4d`, `0x63fa52`, `0x63fa5b`, `0x63fa95`, `0x63faba`, `0x63fadb`, `0x63fb07`, `0x63fb16`, `0x63fc4f`, `0x640570`, `0x64b3f0`, `0x64a4c0`, `0x640780`, `0x6406c0`
Confidence: verified
Proof: one
Test: none

`0x63f9f7` compares the length with 0x40. `0x63fa4d` to `0x63fa5b` compare the name pointer and then the text with `SStrCmpI`; either match returns 0 at `0x63f9fc`. The first
registration (`0x63fa95` to `0x63faba`) sets mask 3, builds four buckets and gives each link offset 4 through `0x640570`. `0x63fadb` is the growth check
`0x640780`. The record is allocated through slot 1 (`0x63fb07`, `call [edx+4]`) and linked at the end of the list with mode 2 and destination 0 (`0x63fb16`, `0x6406c0`). `0x63fb40` unlinks both links
and frees the record through slot 0 (`0x63fc4f`).

## Parsing a line

`0x63f880` splits a line. The first word, up to 63 bytes ending at a space or the end, is copied to the static buffer `0xc4ee08`; spaces after it are skipped; the
pointer to the rest goes to the second out-parameter. It then returns the command with that name, or 0. A character is copied whole by its decoded width, and a
word of 64 bytes or more stops at the buffer limit. A tab does not end the word.

Addresses: `0x63f880`, `0xc4ee08`, `0x41aab0`, `0x63f89d`, `0x63f8a7`, `0x63f8af`, `0x63f8d6`, `0x63f8f7`, `0x63f90e`, `0x63f910`, `0x63f922`, `0x63f972`
Confidence: verified
Proof: one
Test: none

`0x41aab0` decodes one character and its width. The copy loop is `0x63f898` to `0x63f8cf`; it ends on −1, on a space (`0x63f8a7`) or when the width exceeds the room left (`0x63f8af`). `0x63f8d6` stores the NUL. The skip loop is `0x63f8f7`
to `0x63f90c` and `0x63f90e` stores the rest pointer. The lookup begins at `0x63f910` (a mask of −1 returns 0), hashes the buffer (`0x63f922`) and compares the hash and then the text with `SStrCmpI` (`0x63f972`).

## History

The history is a ring of 32 lines of `0x50` bytes at `0xc4ee48` and a write index `0xc518a0`. Adding a line copies at most 0x50 bytes into the slot at the index and advances it. Entry `i`
counts back from the newest: `ring + 0x50·((index − i − 1) & 0x1f)`. The input line walks it with Up and Down (`screen#input-line`).

Addresses: `0x641af0`, `0x63f9b0`, `0x63f9c0`, `0xc4ee48`, `0xc518a0`, `0x64a5a0`, `0x63ce3e`, `0x63ce59`, `0x641b04`
Confidence: verified
Proof: one
Test: none

`0x641af0` is `SStrCopy(ring + 0x50·index, text, 0x50)` (`0x641b04`) followed by `index = (index + 1) & 0x1f`. `0x63f9c0` computes the entry address; `0x63f9b0` returns 0x20, the ring size.
The executor adds a line at `0x63ce59` after it fails to match entry 0 (`0x63ce3e`).

## Completion

`0x63fc60` completes a command name. It takes the typed prefix in `ecx`, a state word in `edx` that holds the name last offered (0 at first) and a direction on the stack. With state 0 it starts at the first
registered command; otherwise it finds the command named by the state and steps to the next, or to the previous when the direction is nonzero. From there it walks the
list in that direction to the first name that begins with the prefix, case-blind, stores that name in the state and returns 1. It returns 0 at the end of the list or when the state's name
names no registered command; it does not wrap.

Addresses: `0x63fc60`, `0x63fc75`, `0x63fd05`, `0x63fd0d`, `0x63fd25`, `0x63fd2a`, `0x63fd48`, `0x63fd4e`, `0x63fd98`, `0xc4f854`
Confidence: verified
Proof: one
Test: none

`0x63fc75` loads `0xc4f854`, the terminator's first object. `0x63fc7f` onward find the state's command. `0x63fd05` tests the direction: nonzero follows the record's previous link (`0x63fd0d`), zero its next field (`0x63fd2a`). The compare is the name against the prefix over
the prefix length with `SStrCmpI` (`0x63fd4e`), and `0x63fd98` stores the hit.

## Help

`help` with no argument writes `Console help categories: `, then the nine category names joined by `, `, then `For more information
type 'help [command] or [category]'`, the last two in class 4. With a category name it writes `Commands registered for the category <name>:` in class 4 and the names of that category's commands
in registration order, eight to a line, or `NONE`. With a command name it writes `Help for command <name>:` in class 4 and `     <name> <help>`, with `No help yet` for a command that has none. An unknown word writes
nothing. The category names are `debug`, `graphics`, `console`, `combat`, `game`, `default`, `net`, `sound` and `gm` (`cvar/registry#flags-and-categories`).

Addresses: `0x63fdc0`, `0x63fdb0`, `0x8653a0`, `0x63fe6b`, `0x63ff30`, `0x64005e`, `0x63fe80`, `0x63ff3a`, `0x63ffed`, `0x640008`, `0x640027`, `0x640088`, `0x64008f`
Confidence: verified
Proof: one
Test: none

The category table has nine entries of 0x18 bytes, an id then a name; the loop `0x63fe80` to `0x63fe9b` compares the word with each name by `SStrCmpI` and ends after nine. `0x63ff3a` compares the id with 9, which none of the nine has. The command loop is `0x63ffb0`
to `0x64001d`: it appends each name of the category to a 0x80-byte buffer with `, ` and writes the line after eight (`0x63ffed`); `0x640027` cuts the final comma. `0x64005e` is the command branch, `0x640088` reads the help
pointer and `0x64008f` falls back to the text `No help yet`. `0x63fdb0` loads `ecx` with the name `help` and jumps in with a command name.

## Built-in commands

`0x6400e0` registers four commands at start-up: `help` (category 2, with its help line), `quit` (category 5, which runs the
client quit `0x63cdf0`), `ver` (category 5, which writes `WoW [Release] Build 5875 (Sep 19 2006)`) and `setmap` (category 5, no help), whose
handler `0x63cf70` only returns 1. Handlers return 1. No command is named `run`.

Addresses: `0x6400e0`, `0x640130`, `0x640140`, `0x63cf70`, `0x63cdf0`, `0x402350`, `0x40236b`, `0x6400f1`, `0x640104`, `0x640117`, `0x64012a`
Confidence: verified
Proof: one
Test: none

`0x6400e0` calls `0x63f9e0` four times (`0x6400f1`, `0x640104`, `0x640117`, `0x64012a`) with the pushes `(help line, 2)` and `(0, 5)`; the names are `help`, `quit`, `ver` and `setmap`. `0x640130` calls
`0x63cdf0` and returns 1; `0x640140` writes the version text and returns 1. The only caller of `0x6400e0` is the start-up wiring at `0x40236b`. The literal names at the registration sites, the 13 console commands and the CVar names of `tables/cvars.tsv` include no `run`.

## Player command lifetime

`ClientInitializeGame` (`0x401570`) registers `port`, `worldport`, `setrawpos` (category 0), `showplayer` (1), `togglehelm` and `togglecloak` (4), none with a help line, and `ClientDestroyGame` (`0x401ee0`) removes them. It makes a 4-byte module object with the vtable `0x7ff9dc`, appends it to the holder list `0x8827c8` and calls slot 0, `RegisterPlayerCommands` (`0x4049e0`), after the object, movement and interface initializers and before the map opens. The game flag `0x882734` is set last. `ClientDestroyGame` returns at once while the flag is clear; otherwise it calls slot 1, `UnregisterPlayerCommands` (`0x405390`), of every listed module, which unregisters all six names, then frees the modules. Outside that span the names are not registered: each occurs only in those two functions. `whois` has the same lifetime (`0x5adc50`, `0x5ae2d0`).

Addresses: `0x401570`, `0x401ee0`, `0x4049e0`, `0x405390`, `0x7ff9dc`, `0x8827c8`, `0x882734`, `0x4016d2`, `0x4016db`, `0x40171d`, `0x401726`, `0x401734`, `0x4017cd`, `0x4017e5`, `0x401ef3`, `0x401f86`, `0x401f9a`, `0x401fb1`, `0x401fc6`, `0x5adc50`, `0x5adcfd`, `0x5ae2d0`, `0x5ae316`, `0x5ae31b`, `0x63f9e0`, `0x63fb40`, `0x46c236`
Confidence: verified
Proof: one
Test: none

`0x4016d2` allocates 4 bytes under `Client.cpp` line `0x891` and `0x4016db` stores `0x7ff9dc`, whose two dwords are `0x4049e0` and `0x405390`. `0x40171d` appends the module to the holder's eight slots and `0x401726` is `call [eax]`. `0x401734` onward register six packet handlers, `0x4017cd` opens the map and `0x4017e5` sets the flag; a scan of every `e8` and `e9` in `.text` finds one caller of `0x401570`, `0x46c236`, and four of `0x401ee0` (`0x403646`, `0x46c5f8`, `0x5aad8d`, `0x5aaeca`); the same scan finds 333 of `0x42c010`, as many as the listing has `call` and `jmp` lines to it. `0x401ef3` tests the flag. The loop from `0x401f77` calls `[eax+4]` at `0x401f86`, frees each module (`0x401f9a`) and the holder (`0x401fb1`). `0x405390` calls `ConsoleCommandUnregister` once per name and tail-jumps for the last. Each name's address occurs twice in the image, in `0x4049e0` and `0x405390`, the vtable once (`0x4016dd`), and `0x4049e0` and `0x405390` occur only as its two dwords, called by no `e8` or `e9`. `0x5adc50` registers `whois` (`0x5adcfd`) when the friend list is new; `0x5ae2d0` runs at `0x401fc6` and, when the list exists, loads its name at `0x5ae316` and unregisters it at `0x5ae31b`.

## Show player command

`0x4049e0` registers `showplayer` with the handler `0x405220`, category 1 and no help line. With
the world frame present (`s_worldFrame` non-null) the handler toggles the active player's model
flag through `ToggleShowOwnModel` and prints `player visible` when the flag is now set and
`player hidden` when it is now clear, through `ConsoleWrite` in class 0. Without a world frame it
does nothing. It returns 1 either way and reads neither argument.

Addresses: `0x4049e0`, `0x404a1d`, `0x404a27`, `0x405220`, `0x405229`, `0x40522e`, `0x5ec8c0`, `0xb4b2bc`
Confidence: verified
Proof: one
Test: none

`0x404a1d` loads `edx` with the handler and `0x404a27` calls `ConsoleCommandRegister` with `ecx` the
name and the pushes 1 and 0. `0x405220` tests `[0xb4b2bc]` and skips to `mov eax,1; ret` on zero.
`0x5ec8c0` writes `sete` of the old flag into the flag and returns it in `eax`; `0x40522e` tests that
value to pick the text.

## Teleport commands

`port` and `setrawpos` need the active player and return 1 without one. When the arguments begin
with a digit or `-`, both read `x y [z] [facing]`, split at tabs, line breaks, `"`, spaces and
commas (`0x82ef90`, the set of `worldport` plus a comma): `x` and `y` are required (without them the
handler returns 0), `z` defaults to 0 and `facing`, in degrees, to the player's own. The position
must pass `0x404d10`, else `Coordinates out of range` is printed. `port` then sends opcode 0xc6 and
`setrawpos` 0xe1, each with the four floats, facing in radians. Other text makes `port` send opcode
0x9 with the whole argument string and makes `setrawpos` return 0.

Addresses: `0x404a60`, `0x404aab`, `0x404ab8`, `0x404ac8`, `0x404aef`, `0x404b31`, `0x404b7d`, `0x404bca`, `0x404c11`, `0x404c3a`, `0x404c46`, `0x404cdc`, `0x405000`, `0x405048`, `0x405055`, `0x40506d`, `0x405157`, `0x4051e4`, `0x404d10`, `0x82ef90`
Confidence: verified
Proof: one
Test: none

`0x404aab` and `0x405048` leave with 1 when `ClntObjMgrObjectPtr` finds no player. `0x404a69` and
`0x405006` copy `0x82ef90` (`\t\r\n" ,`) to the stack for `SStrTokenize`. `0x404ab8` tests the
first byte with `IsAsciiDigit` and `0x404ac8` against `-`; otherwise `0x404aef` starts opcode 0x9,
`PutCString` adds the text and `0x404b31` sends. `0x404b7d` returns 0 on a missing `x`.
A missing `z` stores 0.0 (`0x404c11`), a given facing is scaled by π/180 (`0x404c3a`) and a missing one
comes from the player's slot `+0x18` (`0x404c46`). `0x405000` repeats the parse; it returns 0 at
`0x40506d` and starts opcode 0xe1 at `0x405157`.

## World port command

`worldport <map> [x [y [z [facing]]]]` (`0x404dc0`) does nothing and returns 1 without an active player. It splits its arguments at tabs, line breaks, `"` and spaces (`0x82ecc8`, no comma) into tokens of at most 63 bytes. With an empty first token (none, or `""`) it prints `Usage: worldport <continentID> [x y z] [facing]` in class 4 and returns 0. The map is the first token read by `SStrToUnsigned`: text not starting with a digit gives 0, reading stops at the first non-digit, and digits wrap at 32 bits. A map negative as a signed integer, above the highest `Map.dbc` id or without a row prints `Bad world number: ` and the signed number in class 3, and returns 0.

Addresses: `0x404dc0`, `0x404dec`, `0x404e07`, `0x404e29`, `0x404e2e`, `0x404e3d`, `0x404e4e`, `0x404ef1`, `0x404ef9`, `0x404f04`, `0x404f0e`, `0x82ecc8`, `0x82efb0`, `0x82ef98`, `0x64ae50`, `0x64ade0`, `0x63cb50`, `0x63cd00`, `0xc0daa8`, `0xc0daac`, `0x63ce9a`, `0x53bfe0`, `0x53c001`, `0x829d68`
Confidence: verified
Proof: one
Test: `console::worldport_whole_difftest`

`0x404dec` is `je` on the null from `ClntObjMgrObjectPtr`. `0x404e07` loads `0x82ecc8` (a dword and a word, `\t\r\n" `) into the stack copy that every tokenize call takes with the 0x40-byte buffer and a null quote pointer (`0x404e29`). `0x404e2e` tests the buffer's first byte: zero prints `0x82efb0` with `ConsoleWrite` and `edx` 4 (`0x404e3d`). `0x404e4e` reads the map into `esi`. `0x404ef1` is `jl` for a negative `esi` (signed), `0x404ef9` is `jg` against the highest id `[0xc0daac]`, and `0x404f04` tests the entry of the id map `[0xc0daa8]` for null; `0x404f0e` calls `ConsoleWriteA` with `0x82ef98`, class 3 and `esi`. The initializer `0x53bfe0`, listed in the initializer table at `0x829d68`, runs `0x53bff0`, which stores -1 in `[0xc0daac]` at `0x53c001`, so every map fails until `Map.dbc` loads. `0x63ce9a` calls the handler and returns without reading `eax`. The test runs `0x404dc0` whole (see the message entry).

## World port message

After the map passes, `worldport` sends opcode 8 with `OsGetAsyncTimeMs`, the map as a u32, then `x`, `y`, `z` and the facing as floats, and returns 1. Each of the next four tokens that exists replaces its default, read by `SStrToFloat` (text that is not a number gives 0): the defaults are the active player's position, vtable slot 5, and its facing in radians, slot 6. A given facing is multiplied by π/180, so it is in radians on the wire. No range or finite test applies, unlike `port`'s `0x404d10`. vmangos reads the same order (`src/game/Server/Packets/Misc.cpp`, `WorldTeleport::ReadFromWorldPacket`).

Addresses: `0x404dc0`, `0x404dfa`, `0x404e01`, `0x404e04`, `0x404e77`, `0x404e9c`, `0x404ec1`, `0x404ee6`, `0x404eec`, `0x404f1f`, `0x404f45`, `0x404f4e`, `0x404f57`, `0x404f66`, `0x404fc8`, `0x404ff0`, `0x42c010`, `0x418190`, `0x4182b0`, `0x4181f0`, `0x4183d0`, `0x5379a0`, `0x64aaa0`, `0x7ffaac`, `0x8827c8`, `0x80af78`, `0x5f1f10`, `0x5f1f30`
Confidence: verified
Proof: one
Test: `console::worldport_whole_difftest`

`0x404dfa` calls slot 5 with the pointer to the floats `[ebp-0x1c]` to `[ebp-0x14]` and `0x404e01` slot 6, stored at `[ebp-8]` (`0x404e04`); the player's vtable `0x80af78` holds `0x5f1f10` and `0x5f1f30` there. The tokens after the map are `SStrToFloat` stored as floats at `0x404e77`, `0x404e9c` and `0x404ec1`; the fifth is multiplied by the float at `0x7ffaac` at `0x404ee6` and the 53-bit product stored as a float at `0x404eec`. An empty token skips its store. `0x404f1f` reads the clock; the store takes `PutUInt32` of 8 (`0x404f45`), the clock, the map and four `PutFloat` calls (`0x404f66` onward); `0x404fc8` sends through the holder `0x8827c8`, made when null as `port` does, and `0x404ff0` returns 1. The test runs `0x404dc0` whole over 9,000 argument lines, with the real tokenizer, number readers and data store, and stubs for the player, the console, the clock, the allocator and the send; it compares the return, the stubs' calls in order and the sent bytes.

## Position range test

`0x404d10(x, y, z, margin)` returns 1 when `x`, `y` and `z` are finite and, with H = 17066.666
(`0x46855555`) and S = 34133.332, `margin ≤ H − x < S − margin` and `margin ≤ H − y < S − margin`;
otherwise 0. `z` is only tested for being finite. Both callers pass `z` and `margin` as 0, so `x` and
`y` must each lie in (−17066.666, 17066.666]. `H − x` is rounded to a float before its tests; `H − y`
is not.

Addresses: `0x404d10`, `0x404d1c`, `0x404d4a`, `0x404d6c`, `0x404d6f`, `0x404d85`, `0x404d92`, `0x404d9f`, `0x7ffab0`, `0x7ffab4`, `0x404bc6`
Confidence: verified
Proof: one
Test: none

`_finite` is called on each of the three (`0x404d1c` to `0x404d4a`). Both differences are
`fsub H; fchs`; the one for `x` is stored as a float at `0x404d6c` and the one for `y` stays on the
stack. `0x404d6f` fails `y` below the margin (`C0`), `0x404d85` compares it with `S − margin`, stored as a
float, and passes only on `C0`; `0x404d92` and `0x404d9f` do the same for the stored `x`. `0x404bc6`
pushes 0, 0, `y` and `x`.

## Helm and cloak commands

`togglehelm` sends opcode 0x2b9 and `togglecloak` 0x2ba, each with no body, whatever the arguments
and whether or not a player exists, and returns 1. The client's flag is not consulted, unlike the
interface's helm and cloak buttons (`objectlayer/inventory`, Helm and cloak).

Addresses: `0x405250`, `0x405259`, `0x4052b2`, `0x4052f0`, `0x4052f9`, `0x405352`, `0x8827c8`
Confidence: verified
Proof: one
Test: none

`0x405259` and `0x4052f9` push the opcode for `PutUInt32` and nothing else is written; `0x4052b2` and
`0x405352` call `NetClient::Send` on the connection at `+0x24` of the holder `0x8827c8`, which each creates
with a 0x28-byte allocation when it is null. That allocation passes the flags 0, so it is not zeroed: only `+0x20`
is set, and `+0x24` is written only at `0x4016cf`, in `ClientInitializeGame`.

## Screen commands

The screen registers 13 commands in category 2. `fontcolor`, `bgcolor` and `highlightcolor` set colours (`screen#colour-classes`). `fontsize <n>` sets the font size to
`n · 0.001` clamped to 0.01 to 0.05 and keeps the panel height. `font <name>` loads `Fonts\<name>.ttf`. `consolelines <n>` sets the line count to `n` (0 is ignored, a negative number is accepted) and the height to
`n · size`. `clear` frees every line. `proportionaltext` toggles the proportional style bit `0x10`. `spacing <n>` sets the character spacing to `n`, or 0 with no argument. `settings` prints the size, the font file and the line count.
`default` restores the colours, 0.02, `Fonts\ARIALN.ttf` and 10 lines. `closeconsole` closes the screen and `repeat <n> <command>` queues a command.

Addresses: `0x80e148`, `0x864578`, `0x8645ac`, `0x639660`, `0x639700`, `0x6395c0`, `0x638f40`, `0x638fd0`, `0x6390c0`, `0x639300`, `0x639100`, `0x639240`, `0x639250`, `0x6396a4`, `0x6396c8`, `0x6396eb`, `0x63920e`, `0x63922f`, `0x63b570`, `0x63b585`, `0x639666`, `0x639672`, `0x63968b`, `0x639607`, `0x639612`, `0x639621`, `0x639633`, `0x6391ca`, `0x864560`, `0x864574`, `0xc4e848`, `0xc4ec34`, `0xc4ec50`
Confidence: verified
Proof: one
Test: `console::font_size_set_difftest`, `console::font_metrics_reset_difftest`, `console::line_spacing_set_difftest`

The three tables of 13 words at `0x864578` (names), `0x8645ac` (help) and `0x80e148` (handlers) are read together at `0x63b570` to `0x63b585`. `fontsize` is `atof(n) · 0.001` (`0x639666`), clamped by `fcom` against 0.01 and 0.05
(`0x639672`, `0x63968b`) and stored at `0x86455c`; it recreates the font and recomputes the height as the line count times `size` (`0x6396c8` to `0x6396eb`). `consolelines` reads one `%f` and ignores 0 (`0x639607`, `0x639612`); it stores the count as a float and `size · n` (`0x639621`, `0x639633`).
`default` stores the size `0x3ca3d70a` (`0x6391ca`), the count 10.0 and the height `size · 10.0` (`0x63920e` to `0x63922f`). `closeconsole` clears the open flag and returns 0; `repeat` splits at the first space or
tab and takes the count with `SStrToUnsigned`. The tests run `0x639660` to `0x6396a4` and `0x6396c8` to `0x6396eb`, `0x63920e` to `0x63922f`, and `0x6395c0` whole.

## Table twins

The registry's table is the hash table of `cvar/containers`, and each function of the CVar file has a twin here with the same code. The two compile slightly differently where noted.

| this file | CVar file | does |
|---|---|---|
| `0x6401e0` | `0x63e2b0` | frees a record (slot 0) |
| `0x640260` | `0x63e2e0` | allocates and links a record (slot 1) |
| `0x6403e0` | `0x63e460` | deleting destructor (slot 2) |
| `0x640320` | `0x63e3a0` | clears the table (slot 3) |
| `0x6404a0` | `0x63e520` | unlinks a link |
| `0x6404e0` | `0x63e560` | empties a bucket and its terminator |
| `0x6405e0` | `0x63e5f0` | frees the bucket vector |
| `0x640670` | `0x63ed80` | resolves the next link's back slot |
| `0x6406a0` | `0x63e6d0` | returns an object's link |
| `0x6406c0` | `0x63e6f0` | links a node |

Addresses: `0x6401e0`, `0x640260`, `0x6403e0`, `0x640320`, `0x6404a0`, `0x6404e0`, `0x6405e0`, `0x640670`, `0x6406a0`, `0x6406c0`, `0x63e2b0`, `0x63e2e0`, `0x63e460`, `0x63e3a0`, `0x63e520`, `0x63e560`, `0x63e5f0`, `0x63ed80`, `0x63e6d0`, `0x63e6f0`
Confidence: verified
Proof: one
Test: none

Each pair compares equal instruction by instruction with addresses and branch targets masked; the pairs above match except the allocator, which constructs its record inline instead of calling `0x63d260`, and the free routine, which
unlinks the two links inline instead of calling a destructor. `0x640260` clears the link words `+4`, `+8`, `+0xc`, `+0x10`, `+0x14` and `+0x1c`.

## Growth twins

The rest of the template: the rehash check `0x640780` (twin of `0x63e9f0`), the list constructor `0x640970` (`0x63e9c0`), the terminator reset `0x6409f0` (`0x63ec30`), the chain drain `0x6409a0` (`0x63ebe0`), the count setter `0x640a00` (`0x63ec40`) and the reallocation
`0x640b40` (`0x63ee10`). `0x640a00` holds a register differently from `0x63ec40` and is otherwise the same. `0x640570` sets a list's link offset: it unlinks every object, empties the terminator and stores the offset. `0x640160` empties the registry at shutdown: it unlinks every record from the list and frees each one still in a bucket through slot 0.

Addresses: `0x640780`, `0x640970`, `0x6409f0`, `0x6409a0`, `0x640a00`, `0x640b40`, `0x640570`, `0x640160`, `0x63e9f0`, `0x63e9c0`, `0x63ec30`, `0x63ebe0`, `0x63ec40`, `0x63ee10`, `0x403bf1`, `0x6401bd`, `0x640576`, `0x6405cb`, `0x6405d5`
Confidence: verified
Proof: one
Test: none

`0x640570` returns when the offset is unchanged (`0x640576`), unlinks each object through the old offset, makes the terminator empty (`0x6405cb`) and stores the new offset (`0x6405d5`). `0x640160` is called at `0x403bf1` after the CVar registry's shutdown; its slot-0 call is `0x6401bd`.
