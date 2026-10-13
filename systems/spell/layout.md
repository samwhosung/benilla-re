# spell: layout

Where the system's code lies, what runs it at startup, and how its functions were found.

spell is the client's half of casting: the cast request and the client's own refusals, the
targeting cursor, the server's spell messages and the cast bar's events, cooldowns, values read
from spell records, and the visuals the client drives. Its code is one run, `0x6dd270` to
`0x6ed920`, of three object files: the spell and skill tables, `Spell_C.cpp` and
`SpellVisuals.cpp`; and the cast target code elsewhere, `0x7e4da0` to `0x7e4ed0`, which writes and
reads the cast target block. The run's files open their code with static initializers. Of the 509
functions in the run, 209 are trivial initializers and their thunks; the rest are documented by
topic in `tables`, `cooldowns`, `failures`, `records`, `checks`, `cast`, `targeting`, `handlers`
and `visuals`. The system's state lives in fixed globals of each file: the pending cast, the
in-flight spell, the targeting flag word, the spell manager with its cooldown history, the talent
modifier tables and the visuals' pools. The closure is not yet complete: none of its primitives has
a bit-exact test.

Roots: the static initializer thunks `0x6dd270`, `0x6dd290`, `0x6dd2b0`, `0x6dd500`, `0x6dd750`, `0x6de1f0`, `0x6de210`, `0x6e0890`, `0x6e08c0`, `0x6e08f0`, `0x6e0920`, `0x6e0950`, `0x6e0980`, `0x6e09b0`, `0x6e09e0`, `0x6e0a10`, `0x6e0a40`, `0x6e0a70`, `0x6e0aa0`, `0x6e0ad0`, `0x6e0b00`, `0x6e0b30`, `0x6e0b60`, `0x6e0b90`, `0x6e0bc0`, `0x6e0bf0`, `0x6e0c20`, `0x6e0c50`, `0x6e0c70`, `0x6e0ca0`, `0x6e0cd0`, `0x6e0d00`, `0x6e0d30`, `0x6e0d80`, `0x6e0da0`, `0x6e0dc0`, `0x6e0df0`, `0x6e1040`, `0x6e11d0`, `0x6ea700`, `0x6ea730`, `0x6ea760`, `0x6ea790`, `0x6ea880`, `0x6ea8b0`, `0x6ea8e0`, `0x6eaaa0`, `0x6eaad0`, `0x6eab00`, `0x6eab30`, `0x6eab60`, `0x6eab90`, `0x6eabc0`, `0x6eabf0`, `0x6eac20`, `0x6eac50`, `0x6eac80`, `0x6eacb0`, `0x6eace0`, `0x6ead10`, `0x6ead30`, `0x6ead60`, `0x6ead90`, `0x6eadc0`, `0x6eb3b0`, `0x6eb650`, `0x6eb670`, `0x6eb690`, `0x6eb6c0`, `0x6eb7b0`, `0x6eb800`, `0x6ebff0`, `0x6ec3e0`, `0x6ec420`, `0x6ed440`, `0x6ed470`, `0x6ed4a0`, `0x6ed4d0`, `0x6ed500`, `0x6ed530`, `0x6ed560`, `0x6ed590`, `0x6ed5c0`, `0x6ed5f0`, `0x6ed620`, `0x6ed650`, `0x6ed680`, `0x6ed6b0`, `0x6ed6e0`, `0x6ed710`, `0x6ed740`, `0x6ed770`, `0x6ed7a0`, `0x6ed7d0`, `0x6ed800`, `0x6ed820`, `0x6ed850`, `0x6ed880`, `0x6ed8b0`, `0x6ed8e0`, `0x7e4da0`, `0x7e4dc0`, `0x7e4de0`

## Object files

The run is bounded by the static initializer slots that point into it and by the source paths
its functions pass. The C runtime's initializer table points into the run at 101 consecutive
slots, `0x82cf10` to `0x82d0a0`, three blocks in link order, and nowhere else. Each block opens
with its file's copy of +∞, 2π and 1/2π.

| file | first function | last function | slots |
|---|---|---|---|
| the spell and skill tables | `0x6dd270` | `0x6e0d70` | `0x82cf10` to `0x82cf94` |
| `Spell_C.cpp` | `0x6e0d80` | `0x6eb540` | `0x82cf98` to `0x82d014` |
| `SpellVisuals.cpp` | `0x6eb650` | `0x6ed920` | `0x82d018` to `0x82d0a0` |
| the cast target run and code (one file or two; see below) | `0x7e4da0` | `0x7e4ed0` | `0x82dfa8` to `0x82dfb0` |

Addresses: `0x827000`, `0x82cf10`, `0x82cf94`, `0x82cf98`, `0x82d014`, `0x82d018`, `0x82d0a0`, `0x6dd270`, `0x6e0d70`, `0x6e0d80`, `0x6eb540`, `0x6eb650`, `0x6ed920`, `0x6e1a92`, `0x6e9919`, `0x6ec0e7`, `0x6ec4dc`, `0x7e4da0`, `0x7e4e10`, `0x7e4ed0`, `0x6e5863`, `0x6e77f8`, `0x6e7c76`
Confidence: verified
Proof: one
Test: none

A scan of the table from `0x827000` for dwords pointing into the run finds exactly the 101 slots.
The slot before, `0x82cf0c`, holds an initializer of the geometry file below the run, and the slot
after, `0x82d0a4`, one of `FrameXML.cpp` above it, whose globals follow the visuals' own. The path
`Magic\MagicClient\Spell_C.cpp` is pushed at 44 sites from `0x6e1a92` to `0x6e9919`,
`SpellVisuals.cpp` at `0x6ec0e7`, `0x6ec4b6` and `0x6ec4dc`. The tables file passes no path; its
code alone uses the type tags of its three lists. Each file's +∞ and π constants sit with that
file's other constants in `.rdata`, after those of the geometry file below, whose own initializers
store theirs and whose globals nothing in the run reads. The cast target code lies between the
FreeType library and the sky file's +∞: a run of three slots that opens with its +∞ `0x7e4da0`,
then the block's writer `0x7e4e10` and its reader `0x7e4ed0`. The two follow the run's last
initializer, so they are that file's or a file with no run (`systems/README`, Object files); either
way nothing else lies there, and their only callers are `Spell_C.cpp`'s: `0x6e5863` in the cast
request, `0x6e77f8` and `0x6e7c76` in the spell start and spell go handlers.

## Function starts

The run holds 509 function starts, and every instruction in it lies in exactly one body. A start
is a call target (234), an address stored as a dword in data (125) or pushed or stored by the
run's own code (47), a jump target from outside the run (3), or the first instruction after
padding. Fifteen switch tables sit inside the run, after the code of the function that indexes
them.

Addresses: `0x6dd270`, `0x6ed920`, `0x6e1cbc`, `0x6e23ef`
Confidence: verified
Proof: one
Test: none

The listing restarts after each switch table: a `jmp
[reg*4+table]` gives the table, read while its dwords point back into the function, and a
`movzx` from a byte table before it gives the index table, bounded by the `cmp` before that. The
bodies follow the rule of `symbols/README.md` over that listing. A candidate start that lies
inside another body was dropped when its only evidence was a dword: each such dword was a
coincidence in data or an operand. `0x6e1cbc` and `0x6e23ef` are two of the table jumps.

## Starts the listing hides

Three function starts lie where the shared disassembly listing, decoding linearly through a jump
table, is out of step: `0x6e3b80`, `0x6e47b0` and `0x6e9460`. Each is a `call` target, and a
disassembly that starts at the address decodes a normal prologue there.

Addresses: `0x6e3b80`, `0x6e47b0`, `0x6e9460`, `0x6e3ab8`, `0x6e3ad0`, `0x6e4748`, `0x6e4770`, `0x6e9394`, `0x6e93d0`
Confidence: verified
Proof: one
Test: none

The jump table `0x6e3ab8` with its byte table `0x6e3ad0` ends 2 bytes before `0x6e3b80`;
`0x6e4748` with `0x6e4770` ends 1 byte before `0x6e47b0`; `0x6e9394` with `0x6e93d0` ends 2 bytes
before `0x6e9460`. The listing decodes the table bytes as instructions and joins the real stream after
each start. Decoding from each address gives `push ebp; mov ebp,esp` and a body ending in `ret`.

## Static initializers

Each file runs a header block of identical initializers: its copies of +∞, 2π and 1/2π, 25
float stores computed from constants (four products, a quotient, a square, a quotient, thirteen
squares, the constant 5.0 and four squares) and a counter of the files that use the network
library. 182 of the 209 trivial initializers are those blocks with their thunks; the other 27 are
the thunks of the 12 initializers that build state, the functions that register exit functions,
and two that store zero. The globals the header blocks write, the shared counter aside, are read
only by the next initializer of the same block.

Addresses: `0x6dd270`, `0x6dd280`, `0x6dd290`, `0x6dd2a0`, `0x6dd2b0`, `0x6dd380`, `0x6dd500`, `0x6dd5d0`, `0x6dd750`, `0x6dd830`, `0x6de1f0`, `0x6de200`, `0x6de210`, `0x6de220`, `0x6e0860`, `0x6e0890`, `0x6e08a0`, `0x6e08c0`, `0x6e08d0`, `0x6e08f0`, `0x6e0900`, `0x6e0920`, `0x6e0930`, `0x6e0950`, `0x6e0960`, `0x6e0980`, `0x6e0990`, `0x6e09b0`, `0x6e09c0`, `0x6e09e0`, `0x6e09f0`, `0x6e0a10`, `0x6e0a20`, `0x6e0a40`, `0x6e0a50`, `0x6e0a70`, `0x6e0a80`, `0x6e0aa0`, `0x6e0ab0`, `0x6e0ad0`, `0x6e0ae0`, `0x6e0b00`, `0x6e0b10`, `0x6e0b30`, `0x6e0b40`, `0x6e0b60`, `0x6e0b70`, `0x6e0b90`, `0x6e0ba0`, `0x6e0bc0`, `0x6e0bd0`, `0x6e0bf0`, `0x6e0c00`, `0x6e0c20`, `0x6e0c30`, `0x6e0c50`, `0x6e0c60`, `0x6e0c70`, `0x6e0c80`, `0x6e0ca0`, `0x6e0cb0`, `0x6e0cd0`, `0x6e0ce0`, `0x6e0d00`, `0x6e0d10`, `0x6e0d30`, `0x6e0d40`, `0x6e0d60`, `0x6e0d70`, `0x6e0d80`, `0x6e0d90`, `0x6e0da0`, `0x6e0db0`, `0x6e0dc0`, `0x6e0dd0`, `0x6e0df0`, `0x6e0ec0`, `0x6e1040`, `0x6e1090`, `0x6e11d0`, `0x6e12a0`, `0x6ea700`, `0x6ea710`, `0x6ea730`, `0x6ea740`, `0x6ea760`, `0x6ea770`, `0x6ea790`, `0x6ea7a0`, `0x6ea880`, `0x6ea890`, `0x6ea8b0`, `0x6ea8c0`, `0x6ea8e0`, `0x6ea8f0`, `0x6eaaa0`, `0x6eaab0`, `0x6eaad0`, `0x6eaae0`, `0x6eab00`, `0x6eab10`, `0x6eab30`, `0x6eab40`, `0x6eab60`, `0x6eab70`, `0x6eab90`, `0x6eaba0`, `0x6eabc0`, `0x6eabd0`, `0x6eabf0`, `0x6eac00`, `0x6eac20`, `0x6eac30`, `0x6eac50`, `0x6eac60`, `0x6eac80`, `0x6eac90`, `0x6eacb0`, `0x6eacc0`, `0x6eace0`, `0x6eacf0`, `0x6ead10`, `0x6ead20`, `0x6ead30`, `0x6ead40`, `0x6ead60`, `0x6ead70`, `0x6ead90`, `0x6eada0`, `0x6eadc0`, `0x6eadd0`, `0x6eb3b0`, `0x6eb3c0`, `0x6eb3e0`, `0x6eb3f0`, `0x6eb650`, `0x6eb660`, `0x6eb670`, `0x6eb680`, `0x6eb690`, `0x6eb6a0`, `0x6eb6c0`, `0x6eb700`, `0x6eb7b0`, `0x6eb7e0`, `0x6eb800`, `0x6eb820`, `0x6ebff0`, `0x6ec030`, `0x6ec3e0`, `0x6ec3f0`, `0x6ec400`, `0x6ec420`, `0x6ec430`, `0x6ec440`, `0x6ed440`, `0x6ed450`, `0x6ed470`, `0x6ed480`, `0x6ed4a0`, `0x6ed4b0`, `0x6ed4d0`, `0x6ed4e0`, `0x6ed500`, `0x6ed510`, `0x6ed530`, `0x6ed540`, `0x6ed560`, `0x6ed570`, `0x6ed590`, `0x6ed5a0`, `0x6ed5c0`, `0x6ed5d0`, `0x6ed5f0`, `0x6ed600`, `0x6ed620`, `0x6ed630`, `0x6ed650`, `0x6ed660`, `0x6ed680`, `0x6ed690`, `0x6ed6b0`, `0x6ed6c0`, `0x6ed6e0`, `0x6ed6f0`, `0x6ed710`, `0x6ed720`, `0x6ed740`, `0x6ed750`, `0x6ed770`, `0x6ed780`, `0x6ed7a0`, `0x6ed7b0`, `0x6ed7d0`, `0x6ed7e0`, `0x6ed800`, `0x6ed810`, `0x6ed820`, `0x6ed830`, `0x6ed850`, `0x6ed860`, `0x6ed880`, `0x6ed890`, `0x6ed8b0`, `0x6ed8c0`, `0x6ed8e0`, `0x6ed8f0`, `0x6ed910`, `0x6ed920`, `0x7e4da0`, `0x7e4db0`, `0x7e4dc0`, `0x7e4dd0`, `0x7e4de0`, `0x7e4df0`
Confidence: verified
Proof: one
Test: none

Of the 101 slots, 85 hold a five-byte `jmp` to a body at the next 16-byte boundary, 15 a `call`
to a body followed by a `jmp` to a function that registers an exit function with `_atexit`
(`0x409aef`), and one, `0x6e0830`, the body itself. A typical pair is `0x6e0b30`, `jmp 0x6e0b40`, and `0x6e0b40`, `fld [0x8117f0]; fmul
[0x8117f0]; fstp [0xceab64]; ret`. The header block is the same sequence of constants in all three
files (`0x6e0890` to `0x6e0d10`, `0x6ea700` to `0x6eadd0`, `0x6ed440` to `0x6ed8c0`). The counter
`0x6e0d40` increments `0xc2a3c4` and jumps to `0x5b71c0` when it becomes 1; `0x6e0d70` decrements it
and jumps to `0x5b71d0` at 0 (`net/layout#shared-helpers`). `0x6ec3f0` and `0x6ec430` store 0 in
two globals of `visuals`. A scan of the listing for each written global's
address finds only its writer and, for a quotient or product of a global, the next initializer; the
counter `0xc2a3c4` is every file's.
The 24 initializers that build state are documented with that state (`tables`, `cooldowns`,
`visuals`). The cast target run holds only the first three of the header block: `0x7e4da0`, `0x7e4dc0`
and `0x7e4de0`, with the bodies `0x7e4db0`, `0x7e4dd0` and `0x7e4df0`.
