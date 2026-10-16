# lua: the compiler

The parser and lexer of the embedded Lua 5.0, `lparser.c` and `llex.c` of the upstream source,
from `0x6fc9b0` to `0x7002a0`. The code generator, `lcode.c`, follows them and is in `codegen`; the
loader of precompiled chunks sits between the two files and is in `chunks`. `close_func` passes the
path of `lparser.c` and the lines of the 5.0 source. The lexer state holds the current character
(`+0`), the line (`+4`), the line of the last token (`+8`), the token (`+0x10`, its value at
`+0x18`), the lookahead token (`+0x20`), the function state (`+0x30`), the Lua state (`+0x34`), the
stream (`+0x38`), the buffer (`+0x3c`), the chunk name (`+0x40`) and the nesting level (`+0x44`). A
function state is 0x5d8 bytes: the prototype, the constant table, the parent, the pc (`+0x18`), the
last jump target (`+0x1c`), the pending jumps (`+0x20`), the first free register (`+0x24`), four
counts (`+0x28` to `+0x34`), 32 upvalue descriptors of 20 bytes (`+0x38`) and 200 active-local
indices (`+0x2b8`). Tokens are 5.0's: 257 to 277 the reserved words, 278 a name, 287 the end.
The names are upstream's, and the bodies are the 5.0 release's text.

Roots: `0x6fc9b0`, `0x6ff400`, `0x6ff4d0`

## Parser tokens and checks

The parser reads tokens one ahead through `next`, which takes a held lookahead token before it asks
the lexer, and `lookahead`, which reads that token. `testnext` skips a token when it is the one
given, `check` requires it, and `check_match` requires a closing token and names the opening one
and its line when they differ. Names and string constants become expression descriptors of 20 bytes:
kind, index, auxiliary index, true list and false list.

| function | name | decided by |
|---|---|---|
| `0x6fca40` | `next` | lookahead test against 287 (`0x6fca49`), lookahead copied in (`0x6fca5c`), `luaX_lex` (`0x6fca7f`) |
| `0x6fd580` | `lookahead` | `luaX_lex` into the lookahead slot (`0x6fd586`, `0x6fd58b`) |
| `0x6fccf0` | `testnext` | token compare (`0x6fccf0`), `next` (`0x6fccf5`) |
| `0x6fd140` | `check` | `testnext` (`0x6fd146`), tail `jmp` to `error_expected` (`0x6fd155`) |
| `0x6fce90` | `check_match` | `testnext` (`0x6fce99`), same line gives `error_expected` (`0x6fcea8`, `0x6fceb0`), else the line format (`0x6fced2`) |
| `0x6fcef0` | `error_expected` | `` `%s' expected `` (`0x6fcefc`), tail `jmp` to `luaX_syntaxerror` (`0x6fcf0f`) |
| `0x6fd660` | `str_checkname` | name test (`0x6fd663`), `<name> expected` (`0x6fd66d`), `next` (`0x6fd67c`) |
| `0x6fd640` | `checkname` | `str_checkname` (`0x6fd646`), `codestring` (`0x6fd650`) |
| `0x6fd400` | `codestring` | `luaK_stringK` (`0x6fd40c`), kind 4 (`0x6fd412`) |
| `0x6fd3e0` | `init_exp` | both jump lists -1 (`0x6fd3e6`, `0x6fd3e9`), kind and index (`0x6fd3ef`, `0x6fd3f1`) |

Addresses: `0x6fca40`, `0x6fca49`, `0x6fca5c`, `0x6fca7f`, `0x6fd580`, `0x6fd586`, `0x6fd58b`, `0x6fccf0`, `0x6fccf5`, `0x6fd140`, `0x6fd146`, `0x6fd155`, `0x6fce90`, `0x6fce99`, `0x6fcea8`, `0x6fceb0`, `0x6fced2`, `0x6fcef0`, `0x6fcefc`, `0x6fcf0f`, `0x6fd660`, `0x6fd663`, `0x6fd66d`, `0x6fd67c`, `0x6fd640`, `0x6fd646`, `0x6fd650`, `0x6fd400`, `0x6fd40c`, `0x6fd412`, `0x6fd3e0`, `0x6fd3e6`, `0x6fd3e9`, `0x6fd3ef`, `0x6fd3f1`, `0x6fc9b0`, `0x6fca01`, `0x6fca0e`
Confidence: verified
Proof: one
Test: none

`luaY_parser` (`0x6fc9b0`) keeps the lexer state at `[ebp-0x48]` and the function state below it,
calls `luaX_setinput`, `open_func`, `next` (`0x6fca01`) and `chunk`, and raises `<eof> expected`
unless the token is 287 (`0x6fca0e`). The descriptor kinds are 5.0's `expkind`: 1 nil, 2 true, 3
false, 4 constant, 5 local, 6 upvalue, 7 global, 8 indexed, 9 jump, 10 relocatable, 11 in a
register, 12 call.

## Function states and blocks

`open_func` starts a function state with a new prototype and constant table, no pending jumps and a
stack of two registers; `close_func` appends the final return and trims the code, line, constant,
nested-prototype, local and upvalue vectors to their used sizes. `chunk` parses statements until
`block_follow` sees `else`, `elseif`, `end`, `until` or the end, resetting the free register after
each, and allows 200 nested levels. `body` compiles a function into a new prototype that
`pushclosure` adds to its parent.

| function | name | decided by |
|---|---|---|
| `0x6fca90` | `open_func` | `luaF_newproto` (`0x6fca9a`), pending jumps -1 (`0x6fcabd`), `luaH_new` (`0x6fcad0`), stack size 2 (`0x6fcaee`) |
| `0x6fcb00` | `close_func` | `removevars` 0 (`0x6fcb14`), `OP_RETURN` 0 1 0 (`0x6fcb26`), six `luaM_irealloc` (`0x6fcb48`..`0x6fcc26`) |
| `0x6fcc90` | `chunk` | level above 200 (`0x6fcc99`), `block_follow` (`0x6fccb3`), `statement` (`0x6fccbe`), `;` (`0x6fcccc`), free register reset (`0x6fccd9`) |
| `0x6fcd10` | `block_follow` | byte-table switch over 260 to 287 (`0x6fcd16`, `0x6fcd22`) |
| `0x6fcd60` | `statement` | byte-table switch over 258 to 277 (`0x6fcd6b`, `0x6fcd7f`), 1 after `return` and `break` (`0x6fce26`, `0x6fce35`), else `exprstat` (`0x6fce3e`) |
| `0x6fcf20` | `block` | `enterblock` 0 (`0x6fcf34`), `chunk` (`0x6fcf3b`), `leaveblock` (`0x6fcf42`) |
| `0x6fcf50` | `enterblock` | break list -1 (`0x6fcf56`), the block linked in (`0x6fcf72`) |
| `0x6fcf80` | `leaveblock` | `removevars` (`0x6fcf92`), `OP_CLOSE` for an upvalue (`0x6fcfad`), tail `jmp` to `luaK_patchtohere` (`0x6fcfbf`) |
| `0x6fd7c0` | `body` | `open_func` (`0x6fd7d6`), `self` (`0x6fd801`), `parlist` (`0x6fd808`), `chunk` (`0x6fd81b`), `close_func` (`0x6fd834`), `pushclosure` (`0x6fd842`) |
| `0x6fd980` | `pushclosure` | `luaM_growaux` (`0x6fd9af`), `OP_CLOSURE` (`0x6fd9d6`), `OP_MOVE` or `OP_GETUPVAL` per upvalue (`0x6fd9fe`..`0x6fda0f`) |
| `0x6fda30` | `parlist` | names (`0x6fda6b`), `...` (`0x6fda4d`), ``<name> or `...' expected`` (`0x6fda52`), `code_params` (`0x6fda85`, `0x6fda9d`) |

Addresses: `0x6fca90`, `0x6fca9a`, `0x6fcabd`, `0x6fcad0`, `0x6fcaee`, `0x6fcb00`, `0x6fcb14`, `0x6fcb26`, `0x6fcb48`, `0x6fcc26`, `0x6fcc90`, `0x6fcc99`, `0x6fccb3`, `0x6fccbe`, `0x6fcccc`, `0x6fccd9`, `0x6fcd10`, `0x6fcd16`, `0x6fcd22`, `0x6fcd60`, `0x6fcd6b`, `0x6fcd7f`, `0x6fce26`, `0x6fce35`, `0x6fce3e`, `0x6fcf20`, `0x6fcf34`, `0x6fcf3b`, `0x6fcf42`, `0x6fcf50`, `0x6fcf56`, `0x6fcf72`, `0x6fcf80`, `0x6fcf92`, `0x6fcfad`, `0x6fcfbf`, `0x6fd7c0`, `0x6fd7d6`, `0x6fd801`, `0x6fd808`, `0x6fd81b`, `0x6fd834`, `0x6fd842`, `0x6fd980`, `0x6fd9af`, `0x6fd9d6`, `0x6fd9fe`, `0x6fda0f`, `0x6fda30`, `0x6fda6b`, `0x6fda4d`, `0x6fda52`, `0x6fda85`, `0x6fda9d`, `0x6fcb34`, `0x6fcc0c`, `0x6fda01`, `0x6fda0c`
Confidence: verified
Proof: one
Test: none

`close_func` passes `lparser.c` with the lines 348, 350, 352, 354, 356 and 358, those of the 5.0
source (`0x6fcb34`, `0x6fcc0c`). The statement switch sends 258 `break`, 259 `do`, 264 `for`, 265
`function`, 266 `if`, 268 `local`, 272 `repeat`, 273 `return` and 277 `while` to their rules.
`pushclosure` emits `OP_MOVE` for an upvalue whose descriptor is a local and `OP_GETUPVAL` for the
others (`sete`, `dec`, `and 4` at `0x6fda01`..`0x6fda0c`). The limits are 5.0's: 200 levels, 200
locals, 32 upvalues, 100 parameters.

## Local variables and upvalues

A local is declared by `new_localvar`, which records it in the prototype through
`luaI_registerlocalvar`, and enters scope through `adjustlocalvars`, which stores its first
instruction; `removevars` stores its last. A name resolves through `singlevaraux`: an active local
of the current function first, else the enclosing functions' locals and upvalues, which become an
upvalue here, else a global named by a string constant. `markupval` flags the block of a local that
an inner function captures, so that the block closes it.

| function | name | decided by |
|---|---|---|
| `0x6fd8e0` | `new_localvar` | 200 `local variables` (`0x6fd903`), `luaI_registerlocalvar` (`0x6fd90c`), the active index (`0x6fd917`) |
| `0x6fd930` | `luaI_registerlocalvar` | `luaM_growaux` of 12-byte records (`0x6fd958`), the name (`0x6fd969`), the count (`0x6fd973`) |
| `0x6fd8b0` | `new_localvarstr` | `luaS_newlstr` (`0x6fd8cb`), `new_localvar` (`0x6fd8d4`) |
| `0x6fd850` | `create_local` | `new_localvarstr` (`0x6fd855`), tail `jmp` to `adjustlocalvars` 1 (`0x6fd862`) |
| `0x6fd870` | `adjustlocalvars` | count raised (`0x6fd87b`), first instruction stored (`0x6fd899`) |
| `0x6fcc50` | `removevars` | count lowered (`0x6fcc66`), last instruction stored (`0x6fcc76`) |
| `0x6fdab0` | `code_params` | `adjustlocalvars` (`0x6fdaba`), 100 `parameters` (`0x6fdacb`), counts (`0x6fdad5`, `0x6fdadf`), `arg` (`0x6fdaeb`), `luaK_reserveregs` (`0x6fdaf5`) |
| `0x6fdea0` | `singlevar` | `str_checkname` (`0x6fdeaa`), `singlevaraux` (`0x6fdebb`) |
| `0x6fded0` | `singlevaraux` | no function: global, register 255 (`0x6fdee1`), `searchvar` (`0x6fdefb`), `markupval` (`0x6fdf1f`), the parent (`0x6fdf36`), `luaK_stringK` (`0x6fdf4b`), `indexupvalue` (`0x6fdf5f`) |
| `0x6fdf80` | `indexupvalue` | descriptor match (`0x6fdfb3`, `0x6fdfba`), 32 `upvalues` (`0x6fdfde`), `luaM_growaux` (`0x6fe004`), descriptor copy (`0x6fe02d`) |
| `0x6fe050` | `searchvar` | innermost first (`0x6fe053`), name compare (`0x6fe069`), -1 (`0x6fe076`) |
| `0x6fe080` | `markupval` | blocks above the level skipped (`0x6fe087`), flag set (`0x6fe093`) |

Addresses: `0x6fd8e0`, `0x6fd903`, `0x6fd90c`, `0x6fd917`, `0x6fd930`, `0x6fd958`, `0x6fd969`, `0x6fd973`, `0x6fd8b0`, `0x6fd8cb`, `0x6fd8d4`, `0x6fd850`, `0x6fd855`, `0x6fd862`, `0x6fd870`, `0x6fd87b`, `0x6fd899`, `0x6fcc50`, `0x6fcc66`, `0x6fcc76`, `0x6fdab0`, `0x6fdaba`, `0x6fdacb`, `0x6fdad5`, `0x6fdadf`, `0x6fdaeb`, `0x6fdaf5`, `0x6fdea0`, `0x6fdeaa`, `0x6fdebb`, `0x6fded0`, `0x6fdee1`, `0x6fdefb`, `0x6fdf1f`, `0x6fdf36`, `0x6fdf4b`, `0x6fdf5f`, `0x6fdf80`, `0x6fdfb3`, `0x6fdfba`, `0x6fdfde`, `0x6fe004`, `0x6fe02d`, `0x6fe050`, `0x6fe053`, `0x6fe069`, `0x6fe076`, `0x6fe080`, `0x6fe087`, `0x6fe093`, `0x6fcc69`, `0x6fd88f`, `0x6fdf40`
Confidence: verified
Proof: one
Test: none

The active-local indices sit at `+0x2b8` of the function state (`0x6fcc69`, `0x6fd88f`) and the
records are the prototype's 12-byte name, first and last instruction. `singlevaraux` stores the
string-constant index only when called with `base` set (`0x6fdf40`), as upstream does. A parameter
list with `...` declares `arg` after the parameters and sets the vararg byte at prototype `+0x46`.

## Expressions

`expr` parses `subexpr` with the limit -1. `subexpr` takes a unary operator (`not` or `-`) with
priority 8, else `simpleexp`, then folds binary operators whose left priority is above its limit,
reading the right side at the operator's right priority. The priorities are 5.0's: `^` 10 and 9,
`*` `/` 7, `+` `-` 6, `..` 5 and 4, comparisons 3, `and` 2, `or` 1, so `^` and `..` are right
associative. There is no `%` operator. `primaryexp` takes a name or parenthesized expression and
its suffixes.

| function | name | decided by |
|---|---|---|
| `0x6fd160` | `expr` | `push -1`, `subexpr` (`0x6fd162`) |
| `0x6fd170` | `subexpr` | level above 200 (`0x6fd180`), `getunopr` (`0x6fd19f`), priority 8 (`0x6fd1b2`), `simpleexp` (`0x6fd1cc`), `getbinopr` (`0x6fd1d4`), `priority` (`0x6fd1e5`, `0x6fd204`), `luaK_infix` (`0x6fd1ff`), `luaK_posfix` (`0x6fd226`) |
| `0x6fe0a0` | `getunopr` | `-` gives 0 (`0x6fe0a2`), `not` 1, else 2 (`0x6fe0a7`..`0x6fe0b0`) |
| `0x6fe0c0` | `getbinopr` | byte-table switch from `*` (`0x6fe0c0`..`0x6fe0d1`), else 14 (`0x6fe129`) |
| `0x6fd240` | `simpleexp` | byte-table switch from `{` (`0x6fd246`..`0x6fd25e`), number (`0x6fd270`), string (`0x6fd293`), constructor (`0x6fd2ec`), `body` (`0x6fd302`), else `primaryexp` (`0x6fd310`) |
| `0x6fde40` | `prefixexp` | `(` (`0x6fde46`), name `singlevar` (`0x6fde65`), else `unexpected symbol` (`0x6fde56`) |
| `0x6fdb00` | `primaryexp` | `prefixexp` (`0x6fdb10`), suffix switch (`0x6fdb22`), `.` (`0x6fdb34`), `[` (`0x6fdb3f`..`0x6fdb56`), `:` (`0x6fdb69`, `0x6fdb76`), call (`0x6fdb81`, `0x6fdb8a`) |
| `0x6fdcb0` | `luaY_field` | `luaK_exp2anyreg` (`0x6fdcc2`), `checkname` (`0x6fdcd3`), `luaK_indexed` (`0x6fdce0`) |
| `0x6fd690` | `luaY_index` | `expr` (`0x6fd69f`), `luaK_exp2val` (`0x6fd6a9`), `]` (`0x6fd6b7`) |
| `0x6fdcf0` | `funcargs` | `(` (`0x6fdcfd`), ambiguous syntax (`0x6fdd48`), `explist1` (`0x6fdd74`), constructor (`0x6fdd41`), string (`0x6fdd30`), `OP_CALL` (`0x6fddc7`), `luaK_fixline` (`0x6fddde`) |
| `0x6fddf0` | `explist1` | `expr` (`0x6fddfc`), `,` (`0x6fde08`), `luaK_exp2nextreg` (`0x6fde16`), the count (`0x6fde37`) |

Addresses: `0x6fd160`, `0x6fd162`, `0x6fd170`, `0x6fd180`, `0x6fd19f`, `0x6fd1b2`, `0x6fd1cc`, `0x6fd1d4`, `0x6fd1e5`, `0x6fd204`, `0x6fd1ff`, `0x6fd226`, `0x6fe0a0`, `0x6fe0a2`, `0x6fe0a7`, `0x6fe0b0`, `0x6fe0c0`, `0x6fe0d1`, `0x6fe129`, `0x6fd240`, `0x6fd246`, `0x6fd25e`, `0x6fd270`, `0x6fd293`, `0x6fd2ec`, `0x6fd302`, `0x6fd310`, `0x6fde40`, `0x6fde46`, `0x6fde65`, `0x6fde56`, `0x6fdb00`, `0x6fdb10`, `0x6fdb22`, `0x6fdb34`, `0x6fdb3f`, `0x6fdb56`, `0x6fdb69`, `0x6fdb76`, `0x6fdb81`, `0x6fdb8a`, `0x6fdcb0`, `0x6fdcc2`, `0x6fdcd3`, `0x6fdce0`, `0x6fd690`, `0x6fd69f`, `0x6fd6a9`, `0x6fd6b7`, `0x6fdcf0`, `0x6fdcfd`, `0x6fdd48`, `0x6fdd74`, `0x6fdd41`, `0x6fdd30`, `0x6fddc7`, `0x6fddde`, `0x6fddf0`, `0x6fddfc`, `0x6fde08`, `0x6fde16`, `0x6fde37`, `0x811d4c`
Confidence: verified
Proof: one
Test: none

`priority` (`0x811d4c`) holds the 14 left and right pairs 6 6, 6 6, 7 7, 7 7, 10 9, 5 4, six of 3 3,
2 2 and 1 1. The binary switch maps `+` `-` `*` `/` `^` to 0 to 4, `..` (279) to 5, `~=` (284) to
6, `==` (281) to 7, `<` `<=` `>` `>=` to 8 to 11, `and` to 12 and `or` to 13. `funcargs` raises
`ambiguous syntax (function call x new statement)` when the `(` is on a later line than the last
token, compares `[esi+4]` with `[esi+8]` (`0x6fdd48`), and leaves one result with `C` 2. `prefixexp`
has no case for the `%` upvalue syntax, which 5.0 builds only with `LUA_COMPATUPSYNTAX`.

## Table constructors

`constructor` emits `OP_NEWTABLE`, then reads fields separated by `,` or `;`: a name followed by
`=` or a bracketed key is a keyed field, compiled at once into `OP_SETTABLE`; anything else is a
positional item. Positional items wait in registers and are stored by `OP_SETLIST` every 32 items,
and the last group by `OP_SETLISTO` when its last item is a call, which then keeps all its results.
The table's `B` operand gets the array size through `luaO_int2fb` and `C` the log2 of the keyed
count plus 1.

| function | name | decided by |
|---|---|---|
| `0x6fd430` | `constructor` | `OP_NEWTABLE` (`0x6fd453`), `{` (`0x6fd493`), `closelistfield` (`0x6fd4b6`), `[` and name with `=` ahead (`0x6fd4be`..`0x6fd4db`), `}` (`0x6fd51f`), `lastlistfield` (`0x6fd529`), `luaO_int2fb` (`0x6fd53c`), `luaO_log2` (`0x6fd563`) |
| `0x6fd590` | `recfield` | name count limit (`0x6fd5bc`), `checkname` (`0x6fd5cd`) or `luaY_index` (`0x6fd5d7`), `=` (`0x6fd5e3`), `OP_SETTABLE` (`0x6fd620`), free register restored (`0x6fd629`) |
| `0x6fd790` | `listfield` | `expr` (`0x6fd796`), limit 0x3ffff (`0x6fd7aa`), counts (`0x6fd7b8`, `0x6fd7bb`) |
| `0x6fd6c0` | `closelistfield` | no item (`0x6fd6c3`), `luaK_exp2nextreg` (`0x6fd6cb`), 32 pending (`0x6fd6d0`), `OP_SETLIST` (`0x6fd6ef`) |
| `0x6fd710` | `lastlistfield` | none pending (`0x6fd716`), call (`0x6fd71f`), `luaK_setcallreturns` -1 (`0x6fd726`), `OP_SETLISTO` (`0x6fd73e`), `OP_SETLIST` (`0x6fd770`) |

Addresses: `0x6fd430`, `0x6fd453`, `0x6fd493`, `0x6fd4b6`, `0x6fd4be`, `0x6fd4db`, `0x6fd51f`, `0x6fd529`, `0x6fd53c`, `0x6fd563`, `0x6fd590`, `0x6fd5bc`, `0x6fd5cd`, `0x6fd5d7`, `0x6fd5e3`, `0x6fd620`, `0x6fd629`, `0x6fd790`, `0x6fd796`, `0x6fd7aa`, `0x6fd7b8`, `0x6fd7bb`, `0x6fd6c0`, `0x6fd6c3`, `0x6fd6cb`, `0x6fd6d0`, `0x6fd6ef`, `0x6fd710`, `0x6fd716`, `0x6fd71f`, `0x6fd726`, `0x6fd73e`, `0x6fd770`, `0x6fd4a7`
Confidence: verified
Proof: one
Test: none

The constructor's state is the pending item at `[ebp-0x2c]`, the table descriptor, the keyed count,
the positional count and the pending count (`[ebp-0x18]`..`[ebp-0xc]`). `recfield` limits the keyed
count to `0x7ffffffd` and `listfield` the positional count to `0x3ffff`, both with `items in a
constructor`. A leading `;` is skipped before each field (`0x6fd4a7`).

## Loops and conditionals

`whilestat` compiles the condition first, copies its code out (at most 100 instructions,
else `` `while' condition too complex ``), compiles the body and puts the condition's code after it, so
the loop runs one jump per pass. `repeatstat` jumps back while the `until` condition is false.
`fornum` makes three hidden locals, `(for limit)` and `(for step)` after the variable, with a step of
1 when none is given, and subtracts the step from the start before the loop. `forlist` makes
`(for generator)` and `(for state)` and emits `OP_TFORPREP`. `ifstat` chains `elseif` and `else`.

| function | name | decided by |
|---|---|---|
| `0x6fcfd0` | `whilestat` | `luaK_jump` (`0x6fcfee`), constant made true (`0x6fd00a`), `luaK_goiffalse` (`0x6fd022`), 100 (`0x6fd03b`), copy (`0x6fd06d`), `luaK_code` loop (`0x6fd0f1`), `luaK_patchlist` (`0x6fd126`) |
| `0x6fe260` | `repeatstat` | `luaK_getlabel` (`0x6fe272`), `block` (`0x6fe28f`), `until` (`0x6fe2a1`), `cond` (`0x6fe2ab`), `luaK_patchlist` of the false list (`0x6fe2b9`) |
| `0x6fe2d0` | `cond` | `expr` (`0x6fe2d6`), nil made false (`0x6fe2db`), `luaK_goiftrue` (`0x6fe2eb`), tail `jmp` to `luaK_patchtohere` (`0x6fe2f8`) |
| `0x6fe300` | `forstat` | `str_checkname` (`0x6fe325`), `=` to `fornum` (`0x6fe352`), `,` or `in` to `forlist` (`0x6fe35b`), `` `=' or `in' expected `` (`0x6fe341`) |
| `0x6fe380` | `fornum` | three locals (`0x6fe390`..`0x6fe3ac`), three `exp1` (`0x6fe3bf`, `0x6fe3d2`, `0x6fe3e9`), step 1.0 (`0x6fe3f0`..`0x6fe40a`), `OP_SUB` (`0x6fe42c`), `forbody` (`0x6fe444`) |
| `0x6fe450` | `exp1` | `expr` (`0x6fe45d`), kind kept (`0x6fe465`), `luaK_exp2nextreg` (`0x6fe46b`) |
| `0x6fe480` | `forbody` | `do` (`0x6fe49f`), `block` (`0x6fe4ba`), `OP_FORLOOP` (`0x6fe4e0`) or `OP_TFORLOOP` (`0x6fe4f8`), `luaK_fixline` (`0x6fe504`), `luaK_patchlist` (`0x6fe520`) |
| `0x6fe540` | `forlist` | two locals (`0x6fe55e`, `0x6fe56c`), names (`0x6fe5a1`), `in` (`0x6fe5bd`), `adjust_assign` (`0x6fe5db`), `luaK_checkstack` 3 (`0x6fe5e7`), `OP_TFORPREP` (`0x6fe5fc`) |
| `0x6fe6a0` | `ifstat` | `test_then_block` (`0x6fe6ba`, `0x6fe6e9`), `elseif` (`0x6fe6bf`), `else` (`0x6fe6f7`), `end` (`0x6fe752`) |
| `0x6fe760` | `test_then_block` | `next` (`0x6fe766`), `cond` (`0x6fe76f`), `then` (`0x6fe77b`), tail `jmp` to `block` (`0x6fe784`) |

Addresses: `0x6fcfd0`, `0x6fcfee`, `0x6fd00a`, `0x6fd022`, `0x6fd03b`, `0x6fd06d`, `0x6fd0f1`, `0x6fd126`, `0x6fe260`, `0x6fe272`, `0x6fe28f`, `0x6fe2a1`, `0x6fe2ab`, `0x6fe2b9`, `0x6fe2d0`, `0x6fe2d6`, `0x6fe2db`, `0x6fe2eb`, `0x6fe2f8`, `0x6fe300`, `0x6fe325`, `0x6fe352`, `0x6fe35b`, `0x6fe341`, `0x6fe380`, `0x6fe390`, `0x6fe3ac`, `0x6fe3bf`, `0x6fe3d2`, `0x6fe3e9`, `0x6fe3f0`, `0x6fe40a`, `0x6fe42c`, `0x6fe444`, `0x6fe450`, `0x6fe45d`, `0x6fe465`, `0x6fe46b`, `0x6fe480`, `0x6fe49f`, `0x6fe4ba`, `0x6fe4e0`, `0x6fe4f8`, `0x6fe504`, `0x6fe520`, `0x6fe540`, `0x6fe55e`, `0x6fe56c`, `0x6fe5a1`, `0x6fe5bd`, `0x6fe5db`, `0x6fe5e7`, `0x6fe5fc`, `0x6fe6a0`, `0x6fe6ba`, `0x6fe6e9`, `0x6fe6bf`, `0x6fe6f7`, `0x6fe752`, `0x6fe760`, `0x6fe766`, `0x6fe76f`, `0x6fe77b`, `0x6fe784`, `0x6fe4ea`, `0x6fe4d3`, `0x6fe5ef`, `0x6fe3f5`
Confidence: verified
Proof: one
Test: none

The generic loop's `OP_TFORLOOP` takes `C` = names minus 3 (`0x6fe4ea`), and the numeric loop's
`OP_FORLOOP` and `OP_TFORPREP` take `sBx` -1, written as `Bx` 0x1fffe (`0x6fe4d3`, `0x6fe5ef`). The
default step is the double 1.0 pushed as `0x3ff00000` and 0 (`0x6fe3f0`, `0x6fe3f5`). A `while`
condition that names a constant becomes `true` (kind 4 to 2), and a `repeat` or `if` condition that
is `nil` becomes `false` (kind 1 to 3).

## Statements and assignments

`exprstat` compiles a call as a statement with no results, else an assignment. `assignment` gathers
the targets, which must be locals, upvalues, globals or fields (`syntax error` otherwise), and
`check_conflict` copies a local that an earlier field target indexes by before it is overwritten.
`adjust_assign` pads with nil or drops values, letting a final call supply the difference. `retstat`
turns `return f(...)` into a tail call. `breakstat` closes upvalues of the blocks it leaves and
raises `no loop to break` outside a loop.

| function | name | decided by |
|---|---|---|
| `0x6fe920` | `exprstat` | `primaryexp` (`0x6fe930`), call with no results (`0x6fe935`..`0x6fe942`), `assignment` (`0x6fe95b`) |
| `0x6fe970` | `assignment` | kinds 5 to 8 (`0x6fe983`..`0x6fe996`), `,` (`0x6fe9a2`), `check_conflict` (`0x6fe9c6`), `=` (`0x6fe9e1`), `adjust_assign` (`0x6fea02`), `luaK_storevar` (`0x6fea36`, `0x6fea58`) |
| `0x6fea70` | `check_conflict` | field targets (`0x6fea85`), table and key compared (`0x6fea8e`, `0x6fea9e`), `OP_MOVE` (`0x6feac1`), `luaK_reserveregs` (`0x6feacd`) |
| `0x6fe620` | `adjust_assign` | difference (`0x6fe62a`), call (`0x6fe62d`, `0x6fe65f`), `luaK_exp2nextreg` (`0x6fe673`), `luaK_nil` (`0x6fe68d`) |
| `0x6fe800` | `localstat` | names (`0x6fe81f`), `=` (`0x6fe83b`), `explist1` (`0x6fe849`), `adjust_assign` (`0x6fe862`), `adjustlocalvars` (`0x6fe86b`) |
| `0x6fe790` | `localfunc` | `new_localvar` (`0x6fe7a4`), free register raised in place (`0x6fe7ac`..`0x6fe7b2`), `adjustlocalvars` (`0x6fe7ca`), `body` (`0x6fe7da`), `luaK_storevar` (`0x6fe7e9`) |
| `0x6fe880` | `funcstat` | `funcname` (`0x6fe896`), `body` (`0x6fe8a2`), `luaK_storevar` (`0x6fe8b1`), `luaK_fixline` (`0x6fe8bb`) |
| `0x6fe8d0` | `funcname` | `singlevar` (`0x6fe8d9`), `.` fields (`0x6fe8f4`), `:` with `self` (`0x6fe8fe`..`0x6fe90d`) |
| `0x6feae0` | `retstat` | `block_follow` or `;` (`0x6feaf5`, `0x6feafe`), `explist1` (`0x6feb08`), `OP_TAILCALL` (`0x6feb33`, `0x6feb36`), `OP_RETURN` (`0x6feb69`) |
| `0x6feb80` | `breakstat` | breakable block walk (`0x6feba0`..`0x6febb0`), `no loop to break` (`0x6febb2`), `OP_CLOSE` (`0x6febd4`), `luaK_concat` (`0x6febe6`) |

Addresses: `0x6fe920`, `0x6fe930`, `0x6fe935`, `0x6fe942`, `0x6fe95b`, `0x6fe970`, `0x6fe983`, `0x6fe996`, `0x6fe9a2`, `0x6fe9c6`, `0x6fe9e1`, `0x6fea02`, `0x6fea36`, `0x6fea58`, `0x6fea70`, `0x6fea85`, `0x6fea8e`, `0x6fea9e`, `0x6feac1`, `0x6feacd`, `0x6fe620`, `0x6fe62a`, `0x6fe62d`, `0x6fe65f`, `0x6fe673`, `0x6fe68d`, `0x6fe800`, `0x6fe81f`, `0x6fe83b`, `0x6fe849`, `0x6fe862`, `0x6fe86b`, `0x6fe790`, `0x6fe7a4`, `0x6fe7ac`, `0x6fe7b2`, `0x6fe7ca`, `0x6fe7da`, `0x6fe7e9`, `0x6fe880`, `0x6fe896`, `0x6fe8a2`, `0x6fe8b1`, `0x6fe8bb`, `0x6fe8d0`, `0x6fe8d9`, `0x6fe8f4`, `0x6fe8fe`, `0x6fe90d`, `0x6feae0`, `0x6feaf5`, `0x6feafe`, `0x6feb08`, `0x6feb33`, `0x6feb36`, `0x6feb69`, `0x6feb80`, `0x6feba0`, `0x6febb0`, `0x6febb2`, `0x6febd4`, `0x6febe6`, `0x6fe7af`
Confidence: verified
Proof: one
Test: none

`localfunc` is the 5.0 release text: it raises the free register in place (`lea`/`mov`,
`0x6fe7af`, `0x6fe7b2`) where 5.0.1 and later call `luaK_reserveregs`, so the function's stack size does not
grow for the new local, and it does not set the local's first instruction again after the store. `retstat`
sets the opcode of the call to 26 with `and 0xffffffda; or 0x1a`, which leaves the other fields.

## Lexer errors and limits

`luaX_init` interns the 21 reserved words as fixed strings that carry their index plus 1, which the
lexer turns into a token. A syntax error reads ``<chunk>:<line>: <message> near `<token>'``: the
token is the name's text, the buffer for a number or string, or the token's text, and the chunk is
shortened by `luaO_chunkid` to 80 bytes. Counts are limited by `luaX_checklimit` with `too many %s
(limit=%d)`, lines to `0x7ffffffd` per chunk.

| function | name | decided by |
|---|---|---|
| `0x6ff400` | `luaX_init` | 21 words (`0x6ff42b`), `token2string` (`0x6ff407`), `luaS_newlstr` (`0x6ff41d`), fixed bit (`0x6ff422`), index plus 1 (`0x6ff42e`) |
| `0x6ff440` | `luaX_checklimit` | compare (`0x6ff446`), the format (`0x6ff455`), `luaX_syntaxerror` (`0x6ff467`) |
| `0x6ff480` | `luaX_errorline` | `luaO_chunkid` 80 (`0x6ff498`), `s_textSDSNearS` (`0x87217c`, `0x6ff4aa`), `luaD_throw` 3 (`0x6ff4bf`) |
| `0x6ff530` | `luaX_error` | the current line (`0x6ff533`), `luaX_errorline` (`0x6ff53b`) |
| `0x6ff4d0` | `luaX_syntaxerror` | name (`0x6ff515`), number or string (`0x6ff4e1`..`0x6ff4f4`), `luaX_token2str` (`0x6ff503`) |
| `0x6fffd0` | `luaX_lexerror` | end gives `luaX_token2str` (`0x6fffdf`, `0x6fffe5`), else the buffer (`0x6ffffa`) |
| `0x6ff550` | `luaX_token2str` | below 257 `%c` (`0x6ff550`..`0x6ff562`), else `token2string` (`0x6ff56b`) |
| `0x6ffc20` | `inclinenumber` | next character (`0x6ffc23`..`0x6ffc40`), line count (`0x6ffc57`), `lines in a chunk` (`0x6ffc5a`) |

Addresses: `0x6ff400`, `0x6ff42b`, `0x6ff407`, `0x6ff41d`, `0x6ff422`, `0x6ff42e`, `0x6ff440`, `0x6ff446`, `0x6ff455`, `0x6ff467`, `0x6ff480`, `0x6ff498`, `0x6ff4aa`, `0x87217c`, `0x6ff4bf`, `0x6ff530`, `0x6ff533`, `0x6ff53b`, `0x6ff4d0`, `0x6ff515`, `0x6ff4e1`, `0x6ff4f4`, `0x6ff503`, `0x6fffd0`, `0x6fffdf`, `0x6fffe5`, `0x6ffffa`, `0x6ff550`, `0x6ff562`, `0x6ff56b`, `0x6ffc20`, `0x6ffc23`, `0x6ffc40`, `0x6ffc57`, `0x6ffc5a`, `0x811d68`
Confidence: verified
Proof: one
Test: none

`token2string` (`0x811d68`) holds the 31 token texts from `and` to `<eof>`, and `luaX_token2str`
reads the entry of token minus 257 (`0x6ff56b`). The fixed bit is
`0x10` of the mark byte at `+5` and the index the byte at `+6`. `luaX_errorline` throws status 3,
the syntax error.

## Reading tokens

`luaX_lex` skips newlines, blanks and comments: `--` to the end of the line, or `--[[` to the
matching `]]`. It returns `[[`-strings and quoted strings as 286, numbers as 285, names as 278 or
their reserved token, `..` 279, `...` 280, `==` 281, `>=` 282, `<=` 283, `~=` 284, the end as 287,
and any other character as itself, raising `invalid control char` for a control character. Long
brackets nest, and a newline right after the opening is dropped. A quoted string decodes `\a \b \f
\n \r \t \v`, an escaped newline and up to three decimal digits, at most 255.

| function | name | decided by |
|---|---|---|
| `0x6ff610` | `luaX_lex` | byte-table switch on the character plus 1 below 0x80 (`0x6ff61c`..`0x6ff62c`), comments (`0x6ff65e`..`0x6ff6bf`), blanks (`0x6ff709`), digits (`0x6ffa48`..`0x6ffa77`), names (`0x6ffa85`..`0x6ffb5f`), control characters (`0x6ffabb`..`0x6ffafd`) |
| `0x6ffc70` | `readname` | buffer room (`0x6ffc7c`, `0x6ffc96`..`0x6ffca4`), `isalnum` or `_` (`0x6ffcd8`..`0x6ffd07`), terminating zero (`0x6ffd0e`) |
| `0x6ffd20` | `read_numeral` | leading `.` (`0x6ffd4f`), digits (`0x6ffd54`), `..` after the point (`0x6ffdca`..`0x6ffe4c`), exponent (`0x6ffec9`..`0x6fff0b`), `luaO_str2d` (`0x6fffa0`), `malformed number` (`0x6fffa9`) |
| `0x700010` | `read_long_string` | first newline (`0x70006c`), switch (`0x7000a5`), end of stream (`0x7000b3`..`0x7000d5`), nesting (`0x70011b`, `0x700196`), comment buffer reset (`0x7001b5`), `luaS_newlstr` (`0x700211`) |
| `0x7002a0` | `read_string` | end of stream or newline (`0x70031d`, `0x700326`), escape switch (`0x70039c`), digits (`0x7005af`..`0x700612`), above 255 (`0x700614`..`0x700639`), `luaS_newlstr` (`0x7006d2`) |

Addresses: `0x6ff610`, `0x6ff61c`, `0x6ff62c`, `0x6ff65e`, `0x6ff6bf`, `0x6ff709`, `0x6ffa48`, `0x6ffa77`, `0x6ffa85`, `0x6ffb5f`, `0x6ffabb`, `0x6ffafd`, `0x6ffc70`, `0x6ffc7c`, `0x6ffc96`, `0x6ffca4`, `0x6ffcd8`, `0x6ffd07`, `0x6ffd0e`, `0x6ffd20`, `0x6ffd4f`, `0x6ffd54`, `0x6ffdca`, `0x6ffe4c`, `0x6ffec9`, `0x6fff0b`, `0x6fffa0`, `0x6fffa9`, `0x700010`, `0x70006c`, `0x7000a5`, `0x7000b3`, `0x7000d5`, `0x70011b`, `0x700196`, `0x7001b5`, `0x700211`, `0x7002a0`, `0x70031d`, `0x700326`, `0x70039c`, `0x7005af`, `0x700612`, `0x700614`, `0x700639`, `0x7006d2`, `0x6ff61f`, `0x70020a`, `0x70020e`, `0x7006cd`, `0x7006d1`
Confidence: verified
Proof: one
Test: none

The character tests are the C runtime's `_pctype` masks: 8 blank, 4 digit, 0x103 letter, 0x107
letter or digit, 0x20 control, through `__isctype` when `__mb_cur_max` is above 1. A character from
0x7f up takes the default case (`0x6ff61f`), where those masks decide whether it is a blank, a
digit, a letter or a token of its own. `read_numeral` raises `ambiguous syntax (decimal
point x string concatenation)` for `1..2`. A long string keeps the text between its brackets
(`0x70020a`, `0x70020e`), a quoted string the text between its delimiters (`0x7006cd`, `0x7006d1`).
