# The symbol table

Every known function, global, struct and struct field of `WoW.exe` 1.12.1.5875, one row each. A
row says what the thing is in a line and links to the finding that proves it. The detail lives
in the finding; the row is the index.

Four files, tab-separated, UTF-8, one header line, no comments:

- `functions.tsv`, sorted by `address`
- `globals.tsv`, sorted by `address`
- `structs.tsv`, sorted by `name`
- `fields.tsv`, sorted by `struct`, then `offset`

Sorting is plain byte order (`LC_ALL=C sort`), and every key below is written so that byte order
is numeric order. Two people adding rows in different places then merge without conflict.

## One row per address

An address has exactly one row, owned by one system. A function belongs to the system whose
object file holds it; other systems reach it through their closures (`../systems/README.md`).
Library code the client links is owned by its library: `crt` (the MSVC C runtime), `storm`
(Blizzard's Storm library), `lua` (Lua 5.0), `zlib` (zlib 1.2.2) and `d3dx` (Direct3D 9's D3DX
math routines and their dispatch). A correction edits the row in place. A row is never
duplicated to record a second opinion: a disagreement is settled at the bytes first
(`../docs/METHOD.md`).

## Systems without a doc

These systems own rows but have no doc under `../systems/` yet. A row's `system` is one of
these, a system with a doc, a library, or `unassigned`; the check holds the table to this list.

- `base`: Blizzard's base library files, `engine\Source\Base`: `CDataStore` (packet buffers),
  handles, `FileCache.cpp`, thread properties (`Prop.cpp`), `RCString.cpp`, `CDataRecycler.cpp`,
  `CDataAllocator.cpp` and `MsgBuffer.cpp`
- `client`: `Client.cpp`, the game's start-up and shutdown, and small client files: `Status.cpp`,
  async loads, the violence level, the guild emblem, HTTP fetches
- `objectalloc`: `ObjectAlloc.cpp`, the object heaps
- `os`: the engine's Windows layer outside `core`'s time and window files: CPU and system queries,
  file calls, `OsIME.cpp` and the IME initializer file after it, `OsGui.cpp`, `OsCall.cpp` (engine
  threads), `OsTcp.cpp`, `OsURLDownload.cpp`, `OsClipboard.cpp` and `OsISoundCache.cpp`

## Names

One convention, the client's own C++ style:

- **Methods.** `Class::Method` where the class is established: a vtable, an allocation or type
  tag, or consistent use of one object as `this`. The class name comes from a tag or string in
  the image (`HCAMERA` gives `CCamera`, `.PAVCBaseManaged@@` gives `CBaseManaged`,
  `.?AVCGObject_C@@` gives `CGObject_C`, suffix kept) or from the community database; it is never
  guessed. Only the class carries such a suffix; a method's own part does not, except that
  constructors and destructors are `Class::Class` and `Class::~Class`, and the deleting destructor a vtable holds is `Class::DeletingDestructor`.
- **Free functions.** A PascalCase name that says what the function does to its inputs:
  `RandomSeedFromString`, `NormalizeAngle`. This covers methods of a class the image does not
  name.
- **Static initializers.** `Init<Name>` for the body and `Init<Name>Thunk` for the thunk the
  initializer table points at; the thunk keeps its body's `<Name>`. A run of the table opens with
  its file's +∞ copy (`systems/README`, Object files), so the +∞ thunk belongs to the file it
  opens, not to the file before. `<Name>` takes the first form that fits:
  - `<File><Kind><n>` when the object file has a short name (`InitTerrainSquare305`,
    `InitPlayerFieldsSquare16`). `<Kind>` is the constant or object the body stores (`Copy` or
    `Infinity`, `Double`, `Quotient`, `Square`, `Product`, `Const`) and `<n>` counts the file's
    bodies of that kind in table order.
  - `<Kind><address>` when the file has no name yet: the body's address, six lowercase hex digits
    (`InitSquare4b8b30` at `0x4b8b30`).
  - A free name of what the body holds, when it holds one named constant or object
    (`InitLsTwoPi`, `InitSoundDefPool`).

  A body that only registers a destructor with `_atexit` is `<Name>AtExit`; the destructor is
  `<Name>Exit`, named for the body that built what it frees (`InitHashTable4c52d0Exit`).
- **Data.** `s_` and camelCase for a global (`s_particleDensity`); `Class::vftable` for the vtable
  of a named class.
- **Fields.** `m_` and camelCase (`m_pressTime`); `__vfptr` for a vtable pointer.
- **Structs.** The class name, by the rule for methods.
- **Library code** keeps the library's own names (`_srand`, `__ftol`, `SMemAlloc`,
  `math_randomseed`).
- **Community names** from `vmangos/idb` (1.12.1.5875) are used verbatim, decorations included
  (`_WinMain@16`), once checked at the bytes, with `idb` in `name_from`. A community name the
  bytes contradict is not used.

A name is unique among functions and globals, and a field's name is unique within its struct.
The exports (`cargo xtask export`, and the Ghidra apply `../tools/ghidra/setup.sh`) leave a
function with no name at the tool's own `FUN_00514ae0`.

## Common columns

| column | values |
|---|---|
| `address` | `0x` and exactly six lowercase hex digits: `0x514ae0`. The virtual address in the exe, never a file offset or an RVA. |
| `name` | As above. Empty only when the meaning is unknown. |
| `name_from` | `idb` for a community name; empty for a name given here. |
| `system` | The owning system: a lowercase word naming a doc under `../systems/`, a library, or a system without a doc (above). `unassigned` when the row's object file is not yet identified, or its owner is not; ownership is never guessed. |
| `meaning` | One line starting with a verb: what the thing does or holds. |
| `confidence` | `verified`: read at the bytes, the derivation recorded in `finding`. `inferred`: a lead, not settled. |
| `proof` | For `verified`: `one` (one reading) or `two` (two independent readings reconciled). For `inferred`: `none`. A test is not a proof; it has its own column. |
| `finding` | Where the derivation is: a doc under `../systems/` without `.md`, `#`, and the entry's anchor, for example `random#state-advance`; `METHOD#the-harness` for the facts the harness rests on. The entry mentions the row's address. |

Confidence and proof are separate on purpose: every row states both, so `verified` is never a
default and never says how strongly. The full rules are in `../docs/METHOD.md`.

An anchor is the GitHub slug of the entry's heading: lowercase, spaces to hyphens, punctuation
dropped (`## State advance` is `#state-advance`). Renaming a heading means changing every row
that links to it in the same commit.

A row's finding may live in another system's doc: a library function whose behaviour matters to
one system (the C runtime's `rand` is explained in `random`), or a function of a system with no
doc yet whose meaning a finding elsewhere derives.

## functions.tsv

`address  size  name  name_from  system  class  signature  meaning  confidence  proof  finding  test`

| column | values |
|---|---|
| `size` | Decimal bytes from the entry to the end of the body's last instruction, by the rule below. A cited instruction address belongs to the function whose `[address, address + size)` holds it. |
| `class` | `primitive`, `orchestration`, `host`, `boundary` or `unreached`. Required on every row of a system that has a doc; empty only for a row owned by a system not yet written. |
| `signature` | The C prototype without the name, in the grammar below: `uint32_t __fastcall (uint32_t x)`. Empty when not derived. |
| `test` | The bit-exact tests that run this function against the exe, whole or a slice, `crate::test_fn` separated by `, `: a top-level test function of a file under `crates/<crate>/tests/` (`../docs/CONTRIBUTING.md`, Adding a test). Empty when there is none. |

A function row's address is a function start (`../docs/METHOD.md`, Completeness). Its body is every
instruction reached from that entry by falling through (a `call` falls through), by the targets of
`jcc` and `jmp`, and by the entries of a jump table it indexes. A path ends at `ret`, at an
indirect `jmp` that is not a table jump, at a tail call: a `jmp` to an address with a row of its
own, which the target of every tail call needs, and at a `call` to a function that never returns
(`_exit`, `__exit`, `__amsg_exit`, `longjmp`, `ExitProcess`, or one whose body reaches no return)
where padding or another function follows the call. A `call` to the next instruction pushes its own
address and enters nothing. A jump table's entries run from the address its `jmp` names; where
hand-written code biases its index, from the first dword after it that is an entry, and where it
negates its index (`neg ecx; jmp [ecx*4+base]`), down from that address; its cases may follow it.
Code reached only through a stored address, such as an exception filter or handler in a scope
table, is not in the body. `size` runs from the entry to the end of the body's last instruction,
and nothing in `[address, address + size)` is other than body, padding, the function's jump-table
data and its own exception code: code reached only through a stored address that points into that
range (a filter, a handler, a catch funclet, or the continuation one returns, as in
`mov eax,<continuation>; ret`), with the code it reaches. Exception code past the end of the body
stays outside the size. Padding is `int3` and the no-op fillers compilers align with, where no
branch lands: `nop` of any length, `xchg ax,ax`, `mov edi,edi`, and `lea` of a register to itself
plus zero (`lea ecx,[ecx+0x0]`).
`cargo xtask check --exe` recomputes every size from `dumps/disasm.txt`, which restarts the decode
at every row, every `call` target and every table's end.

Thirty-three starts, all the C runtime's, can hold no row, since no size satisfies this rule
(`cargo xtask census` lists them, and its test holds the list):

- Another entry lies inside the extent, because hand-written routines share tails and branches:
  `0x40b115`, `0x40bccf`, `0x40beee`, `0x40d510`, `0x73f959`, `0x7405b4`, `0x741774`, `0x741834`,
  `0x744890`, `0x744944`, `0x744949`, `0x744aa0`, `0x744bd5`, and the entries the run-time
  library's packed records store, `0x746a60`, `0x746a79` (`cosh`) and `0x746ab2` (`tanh`).
- Code above falls through into the entry: the `__finally` blocks a function both calls and enters
  from its unwind code (`0x40b9ff`, `0x40bb4d`, `0x40bc0d`, `0x40bc88`, `0x40bd95`, `0x40da4e`,
  `0x40dad7`, `0x7470ad`), and the x87 routines' second entries (`0x73f962`, `0x7405bd`,
  `0x74177d`, `0x74183d`, `0x74378a`, `0x743a3d`, `0x743ae9`, `0x7449d1`).
- An instruction no path reaches lies inside the extent: `0x741b02`.

Classes:

- `primitive`: computes a value or state a reimplementation must reproduce bit for bit. Any
  arithmetic, bit manipulation or table lookup on values, beyond loading, storing, comparing and
  computing addresses, makes a function primitive, however thin. Every primitive in a closure
  needs a bit-exact `test` before its system is complete; most have none yet
  ([#87](https://github.com/samwhosung/benilla-re/issues/87)).
- `orchestration`: calls other functions and moves values (loads, stores, compares, branches) and
  computes nothing of its own. Its contract is its call order and arguments, which its finding
  states.
- `host`: talks to the operating system, the heap, files, threads, the GPU or the sound device. A
  reimplementation replaces it rather than reproducing it.
- `boundary`: library code whose documented behaviour is enough (`strlen`, `malloc`, `__ftol`).
  Closures stop at it. Library code whose exact output matters, like `rand`, is `primitive`.
- `unreached`: nothing live reaches it. No `call` or `jmp` in the image targets it and no section
  stores its address, except other unreached code, and no live code falls through into it. The
  row keeps its one-line meaning; it owes no test and its `test` is empty, and a port drops its
  transcription.

A class on a row with no `finding` is the classifier's, from `cargo xtask census`; a reading
replaces it.

### The signature grammar

A signature is `RET CONV (PARAMS)` and nothing else: the return type, one convention, the
parameter list. Tools read it (the C header and the Ghidra, IDA and Binary Ninja imports), so
the check holds every signature to this form.

- `RET` is a type with no name. `CONV` is `__cdecl`, `__stdcall`, `__fastcall` or `__thiscall`,
  and no other word: a register is never written in the signature.
- `PARAMS` is `void`, nothing, or parameters separated by `, `. A parameter is a type with an
  optional name, unique in the signature. A function pointer parameter has no convention of
  its own: `int (*fn)(lua_State *)`; a callee that takes a different convention is `void *`.
- `...` ends the list and only `__cdecl` takes it.
- No `/* */` comment, no `@`, no name before the parameter list.
- Every type is built in (`void`, the C integer and float words), one of the fixed-width and
  size types `bool`, `int8_t` to `int64_t`, `uint8_t` to `uint64_t`, `intptr_t`, `uintptr_t`,
  `size_t`, `ptrdiff_t`, `wchar_t` and `va_list`, a row of `structs.tsv`, or a name behind a
  pointer, which the header declares opaque. A type by value, the return included, is built in or
  a struct row: a Windows handle is `void *`, and a `RECT` passed by value is its four `int32_t`.
- `__thiscall` lists `T *this` first: the object in `ecx`. The rest are the stack arguments.
  `T` is the type of the object `ecx` points to: the class when the method's name gives one,
  else the struct or built-in type the body treats it as (`EvtContext *this`, `float *this`), else
  `void`. A name's class does not give way to a type the body shows (`uint64_t *this` for
  `CGGameUI::Target`, whose `ecx` is a guid pointer). A function that takes an integer in `ecx` is `__fastcall`. Only
  `__thiscall` names a parameter `this`.
- `__fastcall` takes its first two parameters of at most four bytes that are not `float` in `ecx`
  and `edx`, and the rest on the stack. A register the body never reads still has its parameter
  (`uint32_t unused`; `unused1` and `unused2` when both are), since the next parameter does not
  move into it. A register parameter with no known name is `argN`, where N counts every
  parameter of the signature from 1, `this` included.
- Every parameter takes four bytes on the stack, rounded up: eight for `uint64_t`, `int64_t` and
  `double`, a struct's size rounded up to four.

`cargo xtask check --exe` adds the stack: the sum of the bytes the signature puts on the stack is
the `N` of every `ret N` the body reaches in its own extent, and a `__cdecl` body returns with
`ret`. A struct returned by value through a hidden pointer (any size but 1, 2, 4 and 8) adds four
bytes to `__stdcall` and `__thiscall`. A body that ends in a tail jump, as a thunk does, takes the
`ret N` of the row it jumps to; a body that never returns is not compared.

The convention is decided at the bytes on both sides of the call, by `tools/conventions.py`: the
registers the body reads before it writes them, and what every direct caller loads into `ecx` and
`edx` before the call. A function the client reaches through a pointer has the type of the
registration, whatever its body reads: `int __fastcall (lua_State *L)` for a Lua C function,
`__fastcall (const char *name, const char *args)` for a console command, and `__fastcall (void
*param, uint32_t opcode, uint32_t time, CDataStore *msg)` for a message handler. Another function
that no direct call reaches and whose body reads neither register keeps the registers of its older
signature. Which test it applies, and how far its answer goes, is in the tool's
header.

## globals.tsv

`address  type  name  name_from  system  meaning  confidence  proof  finding`

| column | values |
|---|---|
| `type` | A C type: `float`, `uint32_t[64]`, `char[8]`, `void *`, a struct from `structs.tsv`, or a pointer to one. Its size is the size of the type. |

A `.data` global that code writes is a variable, even when its file bytes hold a sensible value:
the meaning names its default and its finding names its writers. A `.rdata` value is a constant.
Pooled literals (`1.0f` and the like) get a row only when a finding cites their address.

## structs.tsv

`name  size  vtable  system  meaning  confidence  proof  finding`

| column | values |
|---|---|
| `name` | The class name, by the naming rule. |
| `size` | Decimal bytes, from the allocation or the constructor. |
| `vtable` | The address of the vtable the constructor finally stores, a row of `globals.tsv`; empty for a struct without one. |

## fields.tsv

`struct  offset  size  type  name  system  meaning  confidence  proof  finding`

| column | values |
|---|---|
| `struct` | A row of `structs.tsv`. |
| `offset` | `0x` and exactly four lowercase hex digits from the start of the struct: `0x0014`. |
| `size` | Decimal bytes; `offset + size` stays within the struct. |
| `type` | As in `globals.tsv`. |

A field's type comes from the instruction that loads it (`fld dword` is a `float`), and its
meaning from its writers, never from one reader's use of it. A sub-object of a class the image
does not name is written as its named base followed by its own fields.

## Rules the check enforces

`cargo xtask check` enforces these, and `cargo xtask check --exe` the body of every function row.

| rule | value | why |
|---|---|---|
| columns | the headers above, in order, every row complete | parallel edits must merge |
| sorting | byte order of the keys above | parallel edits must merge |
| `meaning` | 120 characters, one line, verb first, no final period | a cell is a gloss; the finding holds the detail |
| `name` | 48 characters, the naming rule | applied to Ghidra as an identifier, and to the exports |
| `signature` | 200 characters, and the grammar above | one prototype, no commentary |
| banned in `meaning` | `?`, `TODO`, `probably`, `maybe`, `seems`, `likely` | a guess is `inferred`, said in the confidence column |
| `verified` | needs `one` or `two`, and a `finding` | provenance is the point of the label |
| `inferred` | needs `none`; a `finding` is allowed | a finding can name what it did not settle |
| `finding` | resolves to an entry that mentions the row's address | the row and its derivation stay linked |
| `class` | set on every row of a system with a doc | a written system's closure is classified |
| `system` | a system with a doc, a library, `unassigned`, or one of Systems without a doc, which lists no system that has a doc | every row has a known owner |
| `unreached` | `test` empty | nothing runs the code |
| addresses | unique across functions and globals | one row per address |
| addresses in `meaning` | each resolves to a row | a citation without a row cannot be looked up |
| bodies | every function row starts a function and has its body's size (`--exe`) | a cited address belongs to the row whose extent holds it |
| stack | the bytes a signature puts on the stack are the `ret N` of its body (`--exe`) | a caller cannot tell a missing parameter from a wrong convention |
