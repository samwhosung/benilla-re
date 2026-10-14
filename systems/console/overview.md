# console: overview

The in-game console: a drop-down screen that shows the command output and an input line, the
registry of console commands behind it, and the display settings the client keeps in step with the
graphics device. The screen opens on the console key, and only when the client starts with
`-console`.

The docs split by topic: `screen` (the line list, the input line, keys and mouse, paint and
slide), `commands` (the registry, the built-in commands and the console's own) and `display` (the
display CVars, the device-format record and start-up) and `hardware` (the table of video hardware,
the match against it and the quality values it gives). This doc holds the object files, their
static objects and the shared header constants. The screen draws through `gx` and the font
strings; its CVars are registered through `cvar`.

Roots: `0x638500`, `0x638520`, `0x638540`, `0x638570`, `0x6385a0`, `0x6385d0`, `0x638600`, `0x638630`, `0x638660`, `0x638690`, `0x6386c0`, `0x6386f0`, `0x638720`, `0x638750`, `0x638780`, `0x6387b0`, `0x6387e0`, `0x638800`, `0x638830`, `0x638860`, `0x638890`, `0x6388c0`, `0x6388f0`, `0x638920`, `0x638960`, `0x638a50`, `0x638ad0`, `0x638b00`, `0x638b70`, `0x638be0`, `0x638c20`, `0x638c60`, `0x63f270`, `0x63f290`, `0x63f2b0`, `0x63f530`, `0x63f560`, `0x63f590`, `0x63f5c0`, `0x63f5f0`, `0x63f620`, `0x63f650`, `0x63f680`, `0x63f6b0`, `0x63f6e0`, `0x63f710`, `0x63f740`, `0x63f770`, `0x63f7a0`, `0x63f7c0`, `0x63f7f0`, `0x63f820`, `0x63f850`, `0x63f2e0`

## Object files

The console is two object files. `ConsoleClient.cpp` runs from `0x638500` through `0x63cfb0`: its
static initializers, the screen, the input, the display settings and the command executor.
`ConsoleVar.cpp` (`cvar`) follows it. The command registry runs after that, from `0x63f270`
through `0x640b40`, and passes no source path: it is placed here by its record type
`CONSOLECOMMAND`, by its run of static initializers and by its callers, the screen's executor and
every system that registers a command. The next file opens with its +∞ thunk `0x640c50`.

Addresses: `0x638500`, `0x63cfb0`, `0x63f270`, `0x640b40`, `0x8648c8`, `0x638eaf`, `0x63ccad`, `0x63cff0`, `0x640c50`, `0x827000`, `0x82b74c`, `0x82b7c8`, `0x82b7cc`, `0x82b81c`, `0x82b824`, `0x82b878`, `0x8655c0`
Confidence: verified
Proof: one
Test: none

`0x8648c8` is the path of `ConsoleClient.cpp`, passed at `0x638eaf` through `0x63ccad`; no function
of the registry range passes a path. The initializer table `0x827000` lists the screen's run at the
slots `0x82b74c` to `0x82b7c8`, `ConsoleVar.cpp`'s at `0x82b7cc` to `0x82b81c`, and the registry's at
`0x82b824` to `0x82b878`, each opening with its +∞ thunk (`0x638500`, `0x63cff0`, `0x63f270`) that reads
its own copy of the header constants. `0x63cfb0` ends the screen file where `0x63cff0` begins; the
registry's last function ends where `0x640c50` begins. Of the 252 functions in the two files, 248
are reached; the other four are listed in `Unreached code`.

## Shared header constants

Each file opens its run with the shared header's constants, each a 5-byte thunk and a body that stores
one float: +∞, 2π and 1/2π, the squares 10², 11.111111², 5.5555553², 10², 5², 10², 30², 120², 20², 10²,
0.5², 3² and 100², the float 5.0, then the squares 50², 100², 40², 1.5², and, in the screen file only,
10² and 0.5/10 after them.

Addresses: `0x638500`, `0x638510`, `0x638520`, `0x638530`, `0x638540`, `0x638550`, `0x638570`, `0x638580`, `0x6385a0`, `0x6385b0`, `0x6385d0`, `0x6385e0`, `0x638600`, `0x638610`, `0x638630`, `0x638640`, `0x638660`, `0x638670`, `0x638690`, `0x6386a0`, `0x6386c0`, `0x6386d0`, `0x6386f0`, `0x638700`, `0x638720`, `0x638730`, `0x638750`, `0x638760`, `0x638780`, `0x638790`, `0x6387b0`, `0x6387c0`, `0x6387e0`, `0x6387f0`, `0x638800`, `0x638810`, `0x638830`, `0x638840`, `0x638860`, `0x638870`, `0x638890`, `0x6388a0`, `0x6388c0`, `0x6388d0`, `0x6388f0`, `0x638900`, `0x63f270`, `0x63f280`, `0x63f290`, `0x63f2a0`, `0x63f2b0`, `0x63f2c0`, `0x63f530`, `0x63f540`, `0x63f560`, `0x63f570`, `0x63f590`, `0x63f5a0`, `0x63f5c0`, `0x63f5d0`, `0x63f5f0`, `0x63f600`, `0x63f620`, `0x63f630`, `0x63f650`, `0x63f660`, `0x63f680`, `0x63f690`, `0x63f6b0`, `0x63f6c0`, `0x63f6e0`, `0x63f6f0`, `0x63f710`, `0x63f720`, `0x63f740`, `0x63f750`, `0x63f770`, `0x63f780`, `0x63f7a0`, `0x63f7b0`, `0x63f7c0`, `0x63f7d0`, `0x63f7f0`, `0x63f800`, `0x63f820`, `0x63f830`, `0x63f850`, `0x63f860`
Confidence: verified
Proof: one
Test: none

The bodies are `mov eax,[c]; mov [g],eax` for +∞, `fld; fadd st(0),st; fstp` for 2π, `fld 1.0; fdiv; fstp`
for 1/2π and 0.5/10, `fld; fmul; fstp` for the squares and `mov [g],0x40a00000` for 5.0. The same set
runs in every object file that includes the header (`models/overview#shared-header-constants`). The screen
file's run has 23 slots of them, the registry's 21.

## Static objects

The screen file builds eight statics with its own slots, each a slot function that calls a body and registers an
exit destructor with `_atexit`: the console's lock (`0xc4e64c`), the line list (empty, with its sentinel), the
colour table, the two fixed device-format records (640 × 480 and 800 × 600), the working record and the last-good
record (constructed by `0x5898d0`) and the device-mode list header; a ninth slot is a bare thunk that stores
the panel height, `line count · font size`. The registry builds its command table object. The destructors free
the lines and the mode-list buffer; the others delete the lock or the table, or do nothing.

Addresses: `0x638920`, `0x638930`, `0x638940`, `0x638950`, `0x638960`, `0x638970`, `0x6389a0`, `0x6389b0`, `0x638a50`, `0x638a60`, `0x638ab0`, `0x638ac0`, `0x638ad0`, `0x638ae0`, `0x638b00`, `0x638b10`, `0x638b50`, `0x638b60`, `0x638b70`, `0x638b80`, `0x638bc0`, `0x638bd0`, `0x638be0`, `0x638bf0`, `0x638c00`, `0x638c10`, `0x638c20`, `0x638c30`, `0x638c40`, `0x638c50`, `0x638c60`, `0x638c70`, `0x638c90`, `0x638ca0`, `0x63f2e0`, `0x63f2f0`, `0x63f3b0`, `0x63f3c0`, `0xc4e64c`, `0xc4e9a8`, `0xc4ea9c`
Confidence: verified
Proof: one
Test: none

A slot function is `call body; jmp register`, and the register routine is `push dtor; call _atexit` (`0x638940`).
`0x638930` loads `ecx = 0xc4e64c` and jumps to `SCritSect::SCritSect` (`0x657990`); its destructor jumps to
`0x6579a0`. `0x638970` stores the link offset 0, the sentinel `0xc4e9a8` linked to itself and the head `0xc4e9ac` as the
sentinel with bit 0 set; `0x6389b0` frees every line. `0x638a60` stores the nine colours. `0x638ae0` stores
`[0x864560] · [0x86455c]` into `0xc4ec10`. `0x638b10` and `0x638b80` call `0x589900` for the two fixed records, and `0x638bf0`
and `0x638c30` jump to `0x5898d0` with `ecx` the working and last-good records. `0x638c70` clears four dwords at
`0xc4ea9c`, whose destructor `0x638ca0` frees the buffer under the tag at `0x84298c`. `0x63f2f0` builds the command
table (`commands#command-records`).

## Unreached code

Four functions have no live caller: the formatted write with a va_list `0x63cda0`, two empty stubs `0x63cf50` and
`0x63cf60`, and the setter of the executor's fallback handler `0x6400d0`. No `call` or `jmp` reaches them and no
dword of the image holds their address.

Addresses: `0x63cda0`, `0x63cf50`, `0x63cf60`, `0x6400d0`, `0xc518a4`
Confidence: verified
Proof: one
Test: none

The scan covers every byte of `.text` for `e8`/`e9` and `0f 8x` rel32 targets and every dword of the image. As a control
it finds the two callers of `0x63cdf0` (`0x63c9a0`, `0x640130`) and the 27 of `0x63cb00`. `0x63cda0` formats
`ecx` with the va_list in `edx` and writes it (`0x63cdbb` to `0x63cdd9`); `0x6400d0` stores `ecx` in `0xc518a4`.
Everything else in both files is reached.
