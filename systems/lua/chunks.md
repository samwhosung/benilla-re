# lua: precompiled chunks

The writer and reader of precompiled chunks of the embedded Lua 5.0, `ldump.c` and `lundump.c` of
the upstream source, and the code checker of `ldebug.c` they use. `ldump.c` lies from `0x6fb600` through
`DumpHeader` (`0x6fb940`), before the debug interface, and `lundump.c` from `0x6fec00` to `0x6ff3f0`, between the
parser and the lexer (`compiler`); the code checker is from `0x6fc000` to `0x6fc430`. The loaders
pass the path of `lundump.c` and the lines of the 5.0 source to `luaM_irealloc`. A load state holds
the Lua state, the stream, the buffer, a byte-swap flag (`+0xc`) and the chunk's name (`+0x10`); a
dump state the Lua state, the writer and its data. No chunk the client loads reaches the reader
(`ui/luastate`, How a chunk reaches the compiler), so the checker runs only for `getobjname`. The
names are upstream's.

Roots: `0x6fb600`, `0x6fc010`, `0x6fec00`

## Loading a precompiled chunk

`luaU_undump` names the chunk from its stream name less a leading `@` or `=`, or `binary string`
when it begins with the escape byte, and loads it. The header must hold the signature `1b 4c 75 61`,
version 0x50, a byte-order byte that sets the swap flag when it differs from 1, the sizes 4, 4, 4,
6, 8, 9, 9 and 8, and a number whose integer part is 31415926. Each function then gives its source,
line, four bytes, lines, locals, upvalue names, constants and code, and must pass the code checker.
The constants take the current execution-context word.

| function | name | decided by |
|---|---|---|
| `0x6fec50` | `LoadChunk` | `LoadHeader` (`0x6fec53`), tail `jmp` to `LoadFunction` with no parent source (`0x6fec5a`, `0x6fec5d`) |
| `0x6ff210` | `LoadHeader` | `LoadSignature` (`0x6ff214`), version 0x50 (`0x6ff223`, `0x6ff25b`), `luaU_endianness` (`0x6ff29d`), eight `TestSize` (`0x6ff2b8`..`0x6ff32f`), `__ftol` of the number (`0x6ff33b`, `0x6ff340`) |
| `0x6ff360` | `LoadSignature` | signature bytes (`0x6ff360`, `0x6ff36b`), compare (`0x6ff374`..`0x6ff37c`), `bad signature in %s` (`0x6ff396`) |
| `0x6ff3b0` | `TestSize` | one byte (`0x6ff3b9`), compare (`0x6ff3c1`), `virtual machine mismatch` (`0x6ff3d1`) |
| `0x6ff3f0` | `luaU_endianness` | `mov eax,1; ret` (`0x6ff3f0`) |
| `0x6fec70` | `LoadFunction` | `luaF_newproto` (`0x6fec79`), source or the parent's (`0x6fec82`..`0x6fec8e`), four bytes (`0x6fec9d`..`0x6fecbb`), the five vectors (`0x6fecc7`..`0x6feceb`), `luaG_checkcode` (`0x6fecf2`), `bad code in %s` (`0x6fed01`) |
| `0x6fefe0` | `LoadLines` | count (`0x6fefe7`), `luaM_irealloc` line 136 (`0x6fefee`, `0x6ff006`), `LoadVector` (`0x6ff018`) |
| `0x6fef60` | `LoadLocals` | count (`0x6fef6b`), line 123 (`0x6fef74`, `0x6fef89`), name, first and last instruction (`0x6fefa2`..`0x6fefc5`) |
| `0x6ff030` | `LoadUpvalues` | count 0 or the upvalue count (`0x6ff045`..`0x6ff05f`), line 148 (`0x6ff069`, `0x6ff07f`), names (`0x6ff092`) |
| `0x6ff0b0` | `LoadConstants` | count (`0x6ff0bd`), line 159 (`0x6ff0c4`, `0x6ff0d8`), type byte (`0x6ff0ff`), nil, number and string with the word (`0x6ff12e`..`0x6ff16f`), prototypes line 182 (`0x6ff18f`, `0x6ff1a9`), `LoadFunction` (`0x6ff1c5`) |
| `0x6feea0` | `LoadCode` | count (`0x6feea7`), line 114 (`0x6feeae`, `0x6feec3`), `LoadVector` (`0x6feed5`) |

Addresses: `0x6fec50`, `0x6fec53`, `0x6fec5a`, `0x6fec5d`, `0x6ff210`, `0x6ff214`, `0x6ff223`, `0x6ff25b`, `0x6ff29d`, `0x6ff2b8`, `0x6ff32f`, `0x6ff33b`, `0x6ff340`, `0x6ff360`, `0x6ff36b`, `0x6ff374`, `0x6ff37c`, `0x6ff396`, `0x6ff3b0`, `0x6ff3b9`, `0x6ff3c1`, `0x6ff3d1`, `0x6ff3f0`, `0x6fec70`, `0x6fec79`, `0x6fec82`, `0x6fec8e`, `0x6fec9d`, `0x6fecbb`, `0x6fecc7`, `0x6feceb`, `0x6fecf2`, `0x6fed01`, `0x6fefe0`, `0x6fefe7`, `0x6fefee`, `0x6ff006`, `0x6ff018`, `0x6fef60`, `0x6fef6b`, `0x6fef74`, `0x6fef89`, `0x6fefa2`, `0x6fefc5`, `0x6ff030`, `0x6ff045`, `0x6ff05f`, `0x6ff069`, `0x6ff07f`, `0x6ff092`, `0x6ff0b0`, `0x6ff0bd`, `0x6ff0c4`, `0x6ff0d8`, `0x6ff0ff`, `0x6ff12e`, `0x6ff16f`, `0x6ff18f`, `0x6ff1a9`, `0x6ff1c5`, `0x6feea0`, `0x6feea7`, `0x6feeae`, `0x6feec3`, `0x6feed5`, `0x6fec00`, `0x6fec0c`, `0x6fec14`, `0x6ff134`, `0x6ff14f`, `0x6ff16a`, `0x6fec10`
Confidence: verified
Proof: one
Test: none

`luaU_undump` (`0x6fec00`) compares the name's first byte with `@`, `=` and the signature's first
byte (`0x6fec0c`..`0x6fec14`). `LoadHeader` raises `too new` above 0x50 and `too old` below it,
pushing the version as `v / 16` and `v % 16`, and compares `__ftol` of the number with 0x1df5e76.
`LoadConstants` stores tag 0, 3 or 4 and `[0xceeac0]` at `+4` of each constant (`0x6ff134`,
`0x6ff14f`, `0x6ff16a`) where upstream sets the tag and value only, and raises `bad constant type
(%d) in %s` for another type byte.

## Reading the stream

The reader takes bytes through the stream of the input module (`runtime`, Input streams) and raises
`unexpected end of file in %s` when it ends. A block is read as it is, or byte by byte from its last
byte when the swap flag is set; a vector swaps each element. An integer must not be negative (`bad
integer in %s`). A string is a size, 0 for none, then that many bytes ending in a zero, which the
string drops.

| function | name | decided by |
|---|---|---|
| `0x6fed20` | `ezgetc` | next byte or `luaZ_fill` (`0x6fed26`..`0x6fed44`), end gives `unexpectedEOZ` (`0x6fed4b`..`0x6fed52`) |
| `0x6fed60` | `unexpectedEOZ` | the message (`0x6fed66`), `luaG_runerror` (`0x6fed6c`) |
| `0x6fee10` | `ezread` | `luaZ_read` (`0x6fee1d`), bytes left give `unexpectedEOZ` (`0x6fee24`, `0x6fee28`) |
| `0x6fedc0` | `LoadBlock` | swap flag (`0x6fedc7`), bytes stored from the last (`0x6fede0`..`0x6fedeb`), else `ezread` (`0x6fedf7`) |
| `0x6feee0` | `LoadVector` | swap flag (`0x6feee4`), each element reversed (`0x6fef10`..`0x6fef33`), else `ezread` of the product (`0x6fef3e`..`0x6fef46`) |
| `0x6fed80` | `LoadInt` | `LoadBlock` 4 (`0x6fed8c`), negative (`0x6fed94`), `bad integer in %s` (`0x6fed9e`) |
| `0x6fee80` | `LoadSize` | `LoadBlock` 4 (`0x6fee89`) |
| `0x6ff1f0` | `LoadNumber` | `LoadBlock` 8 (`0x6ff1fb`), `fld` (`0x6ff200`) |
| `0x6fee40` | `LoadString` | `LoadSize` (`0x6fee44`), 0 gives null (`0x6fee4b`), `luaZ_openspace` (`0x6fee59`), `ezread` (`0x6fee65`), `luaS_newlstr` of size less 1 (`0x6fee6c`, `0x6fee70`) |

Addresses: `0x6fed20`, `0x6fed26`, `0x6fed44`, `0x6fed4b`, `0x6fed52`, `0x6fed60`, `0x6fed66`, `0x6fed6c`, `0x6fee10`, `0x6fee1d`, `0x6fee24`, `0x6fee28`, `0x6fedc0`, `0x6fedc7`, `0x6fede0`, `0x6fedeb`, `0x6fedf7`, `0x6feee0`, `0x6feee4`, `0x6fef10`, `0x6fef33`, `0x6fef3e`, `0x6fef46`, `0x6fed80`, `0x6fed8c`, `0x6fed94`, `0x6fed9e`, `0x6fee80`, `0x6fee89`, `0x6ff1f0`, `0x6ff1fb`, `0x6ff200`, `0x6fee40`, `0x6fee44`, `0x6fee4b`, `0x6fee59`, `0x6fee65`, `0x6fee6c`, `0x6fee70`
Confidence: verified
Proof: one
Test: none

`ezgetc` reads the stream's count at `+0` and pointer at `+4` as `zgetc` does, and tests the result
against -1 (`0x6fed4b`). `LoadBlock` and `LoadVector` read the swap flag at `+0xc` of the load state.

## Dumping a function

`luaU_dump` writes a function as a precompiled chunk through a writer function, called with the Lua
state, a block, its size and the writer's data. The header is the signature, version 0x50, the
byte order 1, the sizes 4, 4, 4, 6, 8, 9, 9 and 8, and the double 31415926.535897933. A function
writes its source (none when it is the parent's), line, four bytes, lines, locals, upvalue names,
constants (a type byte, then a number or string) and code.

| function | name | decided by |
|---|---|---|
| `0x6fb600` | `luaU_dump` | the dump state (`0x6fb609`..`0x6fb618`), `DumpHeader` (`0x6fb61b`), `DumpFunction` (`0x6fb628`) |
| `0x6fb940` | `DumpHeader` | signature (`0x6fb944`..`0x6fb94e`), version (`0x6fb955`), `luaU_endianness` (`0x6fb95f`), sizes (`0x6fb96f`..`0x6fb9c3`), the number (`0x6fb9cd`..`0x6fb9d9`) |
| `0x6fb640` | `DumpFunction` | source or none (`0x6fb646`..`0x6fb659`), line (`0x6fb663`), four bytes (`0x6fb66e`..`0x6fb68f`), the five parts (`0x6fb698`..`0x6fb6bc`) |
| `0x6fb810` | `DumpLines` | count (`0x6fb819`), block of 4 each (`0x6fb825`, `0x6fb828`) |
| `0x6fb7b0` | `DumpLocals` | count (`0x6fb7c3`), name and two integers each (`0x6fb7dc`..`0x6fb7f8`) |
| `0x6fb830` | `DumpUpvalues` | count (`0x6fb841`), names (`0x6fb85b`) |
| `0x6fb870` | `DumpConstants` | count (`0x6fb882`), type byte (`0x6fb8a2`), number (`0x6fb8c8`), string (`0x6fb8b6`), prototypes (`0x6fb8e7`, `0x6fb8ff`) |
| `0x6fb790` | `DumpCode` | count (`0x6fb799`), block of 4 each (`0x6fb7a5`, `0x6fb7a8`) |
| `0x6fb730` | `DumpString` | none gives size 0 (`0x6fb730`..`0x6fb73d`, `0x6fb759`), size with the zero (`0x6fb743`), the text (`0x6fb750`) |
| `0x6fb6f0` | `DumpBlock` | the writer through `[D+4]` (`0x6fb6f7`..`0x6fb700`) |
| `0x6fb6d0` | `DumpByte` | one byte (`0x6fb6d4`..`0x6fb6e0`) |
| `0x6fb710` | `DumpInt` | four bytes (`0x6fb714`..`0x6fb720`) |

Addresses: `0x6fb600`, `0x6fb609`, `0x6fb618`, `0x6fb61b`, `0x6fb628`, `0x6fb940`, `0x6fb944`, `0x6fb94e`, `0x6fb955`, `0x6fb95f`, `0x6fb96f`, `0x6fb9c3`, `0x6fb9cd`, `0x6fb9d9`, `0x6fb640`, `0x6fb646`, `0x6fb659`, `0x6fb663`, `0x6fb66e`, `0x6fb68f`, `0x6fb698`, `0x6fb6bc`, `0x6fb810`, `0x6fb819`, `0x6fb825`, `0x6fb828`, `0x6fb7b0`, `0x6fb7c3`, `0x6fb7dc`, `0x6fb7f8`, `0x6fb830`, `0x6fb841`, `0x6fb85b`, `0x6fb870`, `0x6fb882`, `0x6fb8a2`, `0x6fb8c8`, `0x6fb8b6`, `0x6fb8e7`, `0x6fb8ff`, `0x6fb790`, `0x6fb799`, `0x6fb7a5`, `0x6fb7a8`, `0x6fb730`, `0x6fb73d`, `0x6fb759`, `0x6fb743`, `0x6fb750`, `0x6fb6f0`, `0x6fb6f7`, `0x6fb700`, `0x6fb6d0`, `0x6fb6d4`, `0x6fb6e0`, `0x6fb710`, `0x6fb714`, `0x6fb720`, `0x6fb770`, `0x6fb920`, `0x6fb9d2`, `0x6fb6fc`, `0x6fb6fe`
Confidence: verified
Proof: one
Test: none

`DumpSize` (`0x6fb770`) has the same body as `DumpInt`, and `DumpString` calls it for the size;
`DumpNumber` (`0x6fb920`) writes the eight bytes of its stack argument. The header's number is
pushed as `0x417df5e7` and `0x689309b6` (`0x6fb9cd`, `0x6fb9d2`), the double of 31415926.535897933.
The writer is called with the Lua state in `ecx` and the block in `edx` (`0x6fb6fc`, `0x6fb6fe`).

## The code checker

`luaG_symbexec` walks a prototype's instructions up to a pc and returns the last one before it that
set a given register, refusing malformed code with 0: a register at or above the stack size, an
operand that is no register or constant, a constant index past the constants, a test not followed by
`OP_JMP`, a jump out of the code, a wrong instruction after an open call, a closure's upvalue
instructions other than `OP_MOVE` or `OP_GETUPVAL`. `precheck` requires a stack of at most 250, as
many lines as instructions or none, and a final `OP_RETURN`. `luaG_checkcode` runs it over the whole
code with no register.

| function | name | decided by |
|---|---|---|
| `0x6fc000` | `luaG_checkcode` | size of code and register 255 (`0x6fc000`, `0x6fc003`), `luaG_symbexec` (`0x6fc008`) |
| `0x6fc010` | `luaG_symbexec` | `precheck` (`0x6fc027`), `A` (`0x6fc069`), mode (`0x6fc075`..`0x6fc09e`), operands (`0x6fc0d6`..`0x6fc114`), sets `A` (`0x6fc12d`..`0x6fc139`), test then jump (`0x6fc13f`..`0x6fc15c`), opcode switch (`0x6fc16a`..`0x6fc17d`), result (`0x6fc362`, `0x6fc365`) |
| `0x6fc3d0` | `precheck` | 250 (`0x6fc3d0`), line count (`0x6fc3d9`..`0x6fc3e5`), `OP_RETURN` (`0x6fc3ed`..`0x6fc3f9`) |
| `0x6fc400` | `checkopenop` | call, tail call or return with `B` 0 (`0x6fc40c`..`0x6fc428`), `OP_SETLISTO` (`0x6fc416`) |
| `0x6fc430` | `checkRK` | below the stack size (`0x6fc434`), or 250 up and below the constants (`0x6fc438`..`0x6fc44b`) |

Addresses: `0x6fc000`, `0x6fc003`, `0x6fc008`, `0x6fc010`, `0x6fc027`, `0x6fc069`, `0x6fc075`, `0x6fc09e`, `0x6fc0d6`, `0x6fc114`, `0x6fc12d`, `0x6fc139`, `0x6fc13f`, `0x6fc15c`, `0x6fc16a`, `0x6fc17d`, `0x6fc362`, `0x6fc365`, `0x6fc3d0`, `0x6fc3d9`, `0x6fc3e5`, `0x6fc3ed`, `0x6fc3f9`, `0x6fc400`, `0x6fc40c`, `0x6fc428`, `0x6fc416`, `0x6fc430`, `0x6fc434`, `0x6fc438`, `0x6fc44b`, `0x6fc25d`, `0x6fc281`, `0x6fc508`
Confidence: verified
Proof: one
Test: none

The opcode switch handles `OP_LOADBOOL`, `OP_LOADNIL`, the upvalue, global and self instructions,
`OP_CONCAT`, `OP_JMP`, the two loops, calls, `OP_RETURN`, `OP_SETLIST` and `OP_CLOSURE`, as
upstream's; with a register given, a forward jump that does not pass the pc is taken (`0x6fc25d`..
`0x6fc281`). `getobjname` (`runtime`) passes a pc and a register (`0x6fc508`) to name a value.
