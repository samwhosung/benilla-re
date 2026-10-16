# lua: the code generator

The code generator of the embedded Lua 5.0, `lcode.c` of the upstream source, from `0x700790` to
`0x701910`, called by the parser in `compiler`. An instruction is 32 bits: the opcode in bits 0 to
5, `C` in 6 to 14, `B` in 15 to 23, `A` in 24 to 31, or `Bx` in 6 to 23 with `sBx` = `Bx` minus
0x1ffff. The 35 opcodes are 5.0's, from `OP_MOVE` 0 to `OP_CLOSURE` 34, and `luaP_opmodes`
(`0x811de4`) holds 5.0's mode byte for each. An operand from 250 up names constant 250 less, and
register 255 is no register. The function state's fields are those `compiler` lists: the pc at
`+0x18`, the last jump target at `+0x1c`, the jumps pending to the next instruction at `+0x20`, the
first free register at `+0x24` and the constant count at `+0x28`. The bodies are the text of the
5.0 to 5.0.2 releases, which 5.0.3 changed, with one client change: constants carry the
execution-context word. The names are upstream's.

Roots: `0x700790`, `0x700810`, `0x700850`, `0x700860`, `0x7009e0`, `0x7009f0`, `0x700a50`, `0x700a80`, `0x700aa0`, `0x700b90`, `0x700bd0`, `0x700c20`, `0x700cf0`, `0x700fb0`, `0x700ff0`, `0x701010`, `0x7010b0`, `0x701170`, `0x7011d0`, `0x701380`, `0x701410`, `0x701430`, `0x701590`, `0x701610`, `0x701810`, `0x701820`, `0x7018e0`, `0x701910`

## Jump lists

A jump list is chained through the jumps' own offsets, -1 ending it. `luaK_jump` emits `OP_JMP`
carrying the jumps pending to here; `luaK_concat` appends a list; `luaK_patchtohere` adds a list to
the pending jumps, which the next instruction resolves. `luaK_patchlistaux` points each jump at a
target: a jump under a value test (`OP_TEST`) goes to the true or false target by the test's `C`
and gets that target's register, any other to the default target. `need_value` answers whether a
list holds a jump that is no value test of the given sense.

| function | name | decided by |
|---|---|---|
| `0x700810` | `luaK_jump` | pending jumps taken (`0x700818`, `0x700827`), `OP_JMP` with `sBx` -1 (`0x70082e`), `luaK_concat` (`0x70083c`) |
| `0x701320` | `luaK_condjump` | `luaK_codeABC` (`0x701334`), `luaK_jump` (`0x70133b`) |
| `0x700900` | `luaK_fixjump` | offset `dest - (pc + 1)` (`0x70090c`), above 0x1ffff `control structure too long` (`0x70091a`..`0x700929`), store (`0x700943`) |
| `0x700850` | `luaK_getlabel` | last jump target set to the pc (`0x700853`) |
| `0x700950` | `luaK_getjump` | `sBx` (`0x700958`..`0x700960`), -1 ends (`0x700965`), `pc + 1 + offset` (`0x70096d`) |
| `0x700980` | `getjumpcontrol` | pc at least 1 (`0x700980`), test bit of the previous opcode's mode (`0x700997`, `0x7009a0`) |
| `0x700e30` | `need_value` | `getjumpcontrol` (`0x700e46`), not `OP_TEST` or `C` not the sense gives 1 (`0x700e52`..`0x700e61`), `luaK_getjump` (`0x700e67`) |
| `0x7009b0` | `patchtestreg` | register 255 takes `B` (`0x7009b0`..`0x7009bd`), `A` set (`0x7009c5`..`0x7009d1`) |
| `0x700890` | `luaK_patchlistaux` | `ret 0x14` (`0x7008fd`), `OP_TEST` (`0x7008bd`), `C` (`0x7008c8`), true pair (`0x7008cf`, `0x7008d7`), false pair (`0x7008dd`, `0x7008e5`), `luaK_fixjump` (`0x7008ed`) |
| `0x700860` | `luaK_patchlist` | target at the pc gives `luaK_patchtohere` (`0x700866`, `0x70086b`), else one target three times (`0x700874`..`0x700881`) |
| `0x7009e0` | `luaK_patchtohere` | `luaK_getlabel` (`0x7009e0`), `luaK_concat` into the pending jumps (`0x7009e6`, `0x7009e9`) |
| `0x7009f0` | `luaK_concat` | empty second list (`0x7009f7`), empty first takes it (`0x700a02`, `0x700a09`), walk to the end (`0x700a14`..`0x700a2e`), `luaK_fixjump` (`0x700a35`) |

Addresses: `0x700810`, `0x700818`, `0x700827`, `0x70082e`, `0x70083c`, `0x701320`, `0x701334`, `0x70133b`, `0x700900`, `0x70090c`, `0x70091a`, `0x700929`, `0x700943`, `0x700850`, `0x700853`, `0x700950`, `0x700958`, `0x700960`, `0x700965`, `0x70096d`, `0x700980`, `0x700997`, `0x7009a0`, `0x700e30`, `0x700e46`, `0x700e52`, `0x700e61`, `0x700e67`, `0x7009b0`, `0x7009bd`, `0x7009c5`, `0x7009d1`, `0x700890`, `0x7008fd`, `0x7008bd`, `0x7008c8`, `0x7008cf`, `0x7008d7`, `0x7008dd`, `0x7008e5`, `0x7008ed`, `0x700860`, `0x700866`, `0x70086b`, `0x700874`, `0x700881`, `0x7009e0`, `0x7009e6`, `0x7009e9`, `0x7009f0`, `0x7009f7`, `0x700a02`, `0x700a09`, `0x700a14`, `0x700a2e`, `0x700a35`, `0x811de4`
Confidence: verified
Proof: one
Test: none

`luaK_patchlistaux` takes seven arguments, the list and a true target, true register, false target,
false register and default target (`ret 0x14` with two in registers), the 5.0 form; 5.0.3 has five.
`need_value` compares only the opcode and `C` (`0x700e52`, `0x700e61`), where 5.0.3 also requires
`A` to be 255. `luaP_opmodes` (`0x811de4`) holds 5.0's 35 mode bytes, from `0x24` for `OP_MOVE`
to `0x21` for `OP_CLOSURE`, and bit 7 marks a test (`0x7009a0`).

## Registers

`luaK_reserveregs` takes registers above the first free one, raising the function's stack size
through `luaK_checkstack`, which refuses 250 or more with `function or expression too complex`.
`freereg` gives back a register that holds no local and is below 250, and `freeexp` the register of
an expression held in one. `luaK_nil` emits `OP_LOADNIL`, or widens the previous instruction when it
is an `OP_LOADNIL` that no jump targets and whose range meets the new one.

| function | name | decided by |
|---|---|---|
| `0x700a50` | `luaK_checkstack` | free register plus n against the stack size (`0x700a56`..`0x700a5f`), 250 (`0x700a63`..`0x700a73`), store (`0x700a7a`) |
| `0x700a80` | `luaK_reserveregs` | `luaK_checkstack` (`0x700a86`), free register raised (`0x700a91`) |
| `0x700cd0` | `freereg` | at least the local count and below 250 (`0x700cd0`, `0x700cd5`), lowered (`0x700cdd`) |
| `0x700d30` | `freeexp` | kind 11 (`0x700d30`), tail `jmp` to `freereg` (`0x700d38`) |
| `0x700790` | `luaK_nil` | no jump target here (`0x700798`), previous `OP_LOADNIL` (`0x7007ae`), range test (`0x7007c0`..`0x7007c9`), `B` widened (`0x7007da`..`0x7007e7`), else `OP_LOADNIL` (`0x700800`) |

Addresses: `0x700a50`, `0x700a56`, `0x700a5f`, `0x700a63`, `0x700a73`, `0x700a7a`, `0x700a80`, `0x700a86`, `0x700a91`, `0x700cd0`, `0x700cd5`, `0x700cdd`, `0x700d30`, `0x700d38`, `0x700790`, `0x700798`, `0x7007ae`, `0x7007c0`, `0x7007c9`, `0x7007da`, `0x7007e7`, `0x700800`, `0x700a59`, `0x7007b2`
Confidence: verified
Proof: one
Test: none

The stack size is the prototype's byte at `+0x47` (`0x700a59`, `0x700a7a`), so the limit test
`cmp ebx,0xfa; jl` (`0x700a63`) stops at 250. `luaK_nil` reads the previous `A` as the byte at
`+3` of the instruction (`0x7007b2`) and joins when `A <= from <= B + 1`.

## Constants and the context word

`addk` looks a constant up in the function's constant table, a Lua table from value to index, and
returns the index it holds; a new constant is appended to the prototype's constants, at most 0x3ffff
(`constant table overflow`), and its index stored in the table. The client stamps constants with
the execution-context word (`ui/luastate`, The context word): `luaK_stringK`, `luaK_numberK` and
`nil_constant` build the value with the current word `s_scriptContext`, and `addk` copies all 16
bytes into the prototype, makes a nonzero word current while `s_scriptContextDepth` is nonzero
(`test ecx,ecx; je`, `0x700b54`), and
gives the table's index entry the current word. A chunk's constants carry the word current when they
were first added.

| function | name | decided by |
|---|---|---|
| `0x700ad0` | `addk` | `luaH_get` (`0x700adc`), number gives `__ftol` (`0x700ae1`..`0x700ae9`), `luaM_growaux` (`0x700b18`), 16-byte copy (`0x700b30`..`0x700b43`), word made current (`0x700b46`..`0x700b58`), `luaH_set` with the word (`0x700b64`..`0x700b7b`) |
| `0x700aa0` | `luaK_stringK` | tag 4 (`0x700ab5`), word (`0x700aa6`, `0x700abc`), `addk` (`0x700abf`) |
| `0x700b90` | `luaK_numberK` | tag 3 (`0x700bae`), word (`0x700b99`, `0x700ba4`), `addk` (`0x700bb8`) |
| `0x701070` | `nil_constant` | nil value and the constant table as its key, each with the word (`0x701076`..`0x701099`), `addk` (`0x70109c`) |

Addresses: `0x700ad0`, `0x700adc`, `0x700ae1`, `0x700ae9`, `0x700b18`, `0x700b30`, `0x700b43`, `0x700b46`, `0x700b58`, `0x700b64`, `0x700b7b`, `0x700aa0`, `0x700ab5`, `0x700aa6`, `0x700abc`, `0x700abf`, `0x700b90`, `0x700bae`, `0x700b99`, `0x700ba4`, `0x700bb8`, `0x701070`, `0x701076`, `0x701099`, `0x70109c`, `0xceeac0`, `0x700b4e`, `0x700b69`, `0xceeac4`, `0x700b54`
Confidence: verified
Proof: one
Test: none

Upstream 5.0 copies the constant with `setobj2n`, the tag and the value, and leaves the word bytes
of each value unset. Here the value is built at `[ebp-0x10]` with the tag, then `[0xceeac0]` at
`+4` and the string or number at `+8`, and `addk` copies four dwords from it (`0x700b30`..`0x700b43`),
then tests `[v+4]` and `[0xceeac4]` and stores `[v+4]` into `0xceeac0` when both are nonzero
(`0x700b4e`..`0x700b58`). The index entry gets tag 3, `[0xceeac0]` at `+4` and the count by `fild`
(`0x700b69`..`0x700b7b`). `nil_constant` keys nil by the constant table itself, tag 5, since nil is
no table key.

## Discharging expressions

`luaK_dischargevars` turns a variable into a value: a local is already in its register, an upvalue,
global or field gets its load instruction, and a call is cut to one result by `luaK_setcallreturns`.
`discharge2reg` puts a value without jumps into a given register: `OP_LOADNIL`, `OP_LOADBOOL`,
`OP_LOADK`, the target register of a relocatable instruction or `OP_MOVE`. `luaK_exp2reg` adds the
jump lists, loading false and true through `code_label` when some jump yields no value. `luaK_exp2RK`
answers a constant operand when the index fits in 9 bits after the 250 bias.

| function | name | decided by |
|---|---|---|
| `0x700bd0` | `luaK_setcallreturns` | call (`0x700bd3`), `C` = n + 1 (`0x700beb`..`0x700c00`), one result to its register (`0x700c08`..`0x700c18`) |
| `0x700c20` | `luaK_dischargevars` | switch on kind 5 to 12 (`0x700c25`..`0x700c30`), local (`0x700c38`), `OP_GETUPVAL` (`0x700c42`), `OP_GETGLOBAL` (`0x700c56`), field frees and `OP_GETTABLE` (`0x700c67`..`0x700c8c`), call (`0x700ca3`) |
| `0x700eb0` | `discharge2reg` | `luaK_dischargevars` (`0x700eba`), switch on kind 1 to 11 (`0x700ec1`..`0x700ed4`), nil (`0x700ee4`), booleans (`0x700efe`..`0x700f07`), constant (`0x700f1d`), relocatable (`0x700f45`..`0x700f52`), register (`0x700f6a`..`0x700f76`) |
| `0x701350` | `discharge2anyreg` | in a register (`0x701354`), `luaK_reserveregs` (`0x701360`), `discharge2reg` (`0x70136e`) |
| `0x700e90` | `code_label` | `luaK_getlabel` (`0x700e93`), `OP_LOADBOOL` (`0x700ea6`) |
| `0x700d40` | `luaK_exp2reg` | `discharge2reg` (`0x700d50`), jump kind (`0x700d55`..`0x700d63`), `need_value` (`0x700d7f`, `0x700d8e`), `code_label` (`0x700db6`, `0x700dc6`), two `luaK_patchlistaux` (`0x700df1`, `0x700e0d`) |
| `0x700cf0` | `luaK_exp2nextreg` | `luaK_dischargevars` (`0x700cf6`), `freeexp` (`0x700cff`), `luaK_reserveregs` (`0x700d0b`), `luaK_exp2reg` (`0x700d19`) |
| `0x700fb0` | `luaK_exp2anyreg` | register without jumps (`0x700fbb`..`0x700fc9`), not a local (`0x700fcb`, `0x700fd5`), else `luaK_exp2nextreg` (`0x700fe4`) |
| `0x700ff0` | `luaK_exp2val` | jumps (`0x700ff3`), `luaK_exp2anyreg` (`0x700ff8`) or `luaK_dischargevars` (`0x700ffd`) |
| `0x701010` | `luaK_exp2RK` | `luaK_exp2val` (`0x701016`), nil (`0x701037`..`0x701057`), constant (`0x701025`..`0x70102d`), else `luaK_exp2anyreg` (`0x701064`) |

Addresses: `0x700bd0`, `0x700bd3`, `0x700beb`, `0x700c00`, `0x700c08`, `0x700c18`, `0x700c20`, `0x700c25`, `0x700c30`, `0x700c38`, `0x700c42`, `0x700c56`, `0x700c67`, `0x700c8c`, `0x700ca3`, `0x700eb0`, `0x700eba`, `0x700ec1`, `0x700ed4`, `0x700ee4`, `0x700efe`, `0x700f07`, `0x700f1d`, `0x700f45`, `0x700f52`, `0x700f6a`, `0x700f76`, `0x701350`, `0x701354`, `0x701360`, `0x70136e`, `0x700e90`, `0x700e93`, `0x700ea6`, `0x700d40`, `0x700d50`, `0x700d55`, `0x700d63`, `0x700d7f`, `0x700d8e`, `0x700db6`, `0x700dc6`, `0x700df1`, `0x700e0d`, `0x700cf0`, `0x700cf6`, `0x700cff`, `0x700d0b`, `0x700d19`, `0x700fb0`, `0x700fbb`, `0x700fc9`, `0x700fcb`, `0x700fd5`, `0x700fe4`, `0x700ff0`, `0x700ff3`, `0x700ff8`, `0x700ffd`, `0x701010`, `0x701016`, `0x701037`, `0x701057`, `0x701025`, `0x70102d`, `0x701064`, `0x700de2`, `0x700dff`, `0x701028`
Confidence: verified
Proof: one
Test: none

`luaK_exp2reg` passes the false list with the false load, no register, the final pc, the target
register and the false load (`0x700de2`..`0x700df1`), and the true list with the final pc, the
register, the true load, no register and the true load (`0x700dff`..`0x700e0d`), as 5.0 does.
`luaK_exp2RK` tests `index + 250 <= 511` (`0x701028`, `0x70102d`).

## Conditions and operators

`luaK_goiftrue` makes an expression jump when false and `luaK_goiffalse` when true: a constant needs
no jump or always jumps, a comparison inverts or keeps its jump, and anything else gets `OP_TEST`
through `jumponcond`, which folds a preceding `OP_NOT`. `luaK_infix` prepares the left operand and
`luaK_posfix` combines: `and` and `or` merge jump lists, `..` chains `OP_CONCAT` into one
instruction, and `codebinop` emits arithmetic or a comparison and jump, swapping the operands of `>`
and `>=`. `luaK_prefix` folds `-` of a number constant.

| function | name | decided by |
|---|---|---|
| `0x7010b0` | `luaK_storevar` | switch on kind 5 to 8 (`0x7010bd`..`0x7010cb`), local (`0x7010e6`), `OP_SETUPVAL` (`0x701101`), `OP_SETGLOBAL` (`0x70111d`), `OP_SETTABLE` (`0x70114b`), `freeexp` (`0x701154`) |
| `0x701170` | `luaK_self` | `luaK_exp2anyreg` (`0x70117a`), two registers (`0x701190`), `OP_SELF` (`0x7011ac`) |
| `0x701410` | `luaK_indexed` | `luaK_exp2RK` (`0x701419`), kind 8 (`0x701421`) |
| `0x7011d0` | `luaK_goiftrue` | switch on kind 2 to 9 (`0x7011dd`..`0x7011e5`), `luaK_jump` (`0x7011ff`), `invertjump` (`0x701216`), `jumponcond` 0 (`0x701232`), false list (`0x7011f5`) |
| `0x701380` | `luaK_goiffalse` | switch on kind 1 to 9 (`0x70138d`..`0x70139a`), `luaK_jump` (`0x7013b4`), `jumponcond` 1 (`0x7013de`), true list (`0x7013aa`) |
| `0x701270` | `invertjump` | `getjumpcontrol` (`0x701273`), `A` inverted (`0x701278`..`0x701291`) |
| `0x7012a0` | `jumponcond` | `OP_NOT` (`0x7012be`), pc lowered (`0x7012d9`), `discharge2anyreg` (`0x7012e4`), `OP_TEST` with `A` 255 (`0x7012fc`, `0x701301`), `luaK_condjump` (`0x701306`) |
| `0x7014c0` | `codenot` | switch on kind 1 to 11 (`0x7014cb`..`0x7014da`), constants (`0x7014e8`, `0x7014fd`), `invertjump` (`0x70150f`), `OP_NOT` (`0x701544`), lists swapped (`0x701552`..`0x70155c`) |
| `0x701430` | `luaK_prefix` | minus (`0x701433`), number constant `fchs` and `luaK_numberK` (`0x701445`..`0x70146a`), `OP_UNM` (`0x701497`), else `codenot` (`0x7014ae`) |
| `0x701590` | `luaK_infix` | `..` (`0x701596`, `0x7015f9`), `and` (`0x7015da`..`0x7015e9`), `or` (`0x7015b9`..`0x7015c8`), else `luaK_exp2RK` (`0x7015a9`) |
| `0x701610` | `luaK_posfix` | `and` (`0x70166c`..`0x70169c`), `or` (`0x7016a6`..`0x7016d6`), `OP_CONCAT` chained (`0x7016fd`..`0x70172b`) or new (`0x70176d`), else `codebinop` (`0x70165e`) |
| `0x701790` | `codebinop` | arithmetic `OP_ADD` plus the operator (`0x701796`..`0x7017ab`), swap (`0x7017be`..`0x7017d3`), `~=` sense 0 (`0x7017d9`..`0x7017de`), `ops` (`0x7017e9`), `luaK_condjump` (`0x7017f0`) |

Addresses: `0x7010b0`, `0x7010bd`, `0x7010cb`, `0x7010e6`, `0x701101`, `0x70111d`, `0x70114b`, `0x701154`, `0x701170`, `0x70117a`, `0x701190`, `0x7011ac`, `0x701410`, `0x701419`, `0x701421`, `0x7011d0`, `0x7011dd`, `0x7011e5`, `0x7011ff`, `0x701216`, `0x701232`, `0x7011f5`, `0x701380`, `0x70138d`, `0x70139a`, `0x7013b4`, `0x7013de`, `0x7013aa`, `0x701270`, `0x701273`, `0x701278`, `0x701291`, `0x7012a0`, `0x7012be`, `0x7012d9`, `0x7012e4`, `0x7012fc`, `0x701301`, `0x701306`, `0x7014c0`, `0x7014cb`, `0x7014da`, `0x7014e8`, `0x7014fd`, `0x70150f`, `0x701544`, `0x701552`, `0x70155c`, `0x701430`, `0x701433`, `0x701445`, `0x70146a`, `0x701497`, `0x7014ae`, `0x701590`, `0x701596`, `0x7015f9`, `0x7015da`, `0x7015e9`, `0x7015b9`, `0x7015c8`, `0x7015a9`, `0x701610`, `0x70166c`, `0x70169c`, `0x7016a6`, `0x7016d6`, `0x7016fd`, `0x70172b`, `0x70176d`, `0x70165e`, `0x701790`, `0x701796`, `0x7017ab`, `0x7017be`, `0x7017d3`, `0x7017d9`, `0x7017de`, `0x7017e9`, `0x7017f0`, `0x811e08`
Confidence: verified
Proof: one
Test: none

`jumponcond` emits `OP_TEST` with `A` 255 in both cases (`0x701301`), folding `not x` into a test of
`x`'s register with the opposite sense; 5.0.3 passes the register as `A` there, and its `codenot`
also clears the registers of value tests, a step this `codenot` lacks. `ops` (`0x811e08`) holds
`OP_EQ`, `OP_EQ`, `OP_LT`, `OP_LE`, `OP_LT`, `OP_LE` for `~=` to `>=`, read as `[op*4+base]` with
`base` 24 bytes below it.

## Emitting instructions

`luaK_code` resolves the jumps pending to the new instruction through `luaK_dischargejpc`, appends it
and the line to the prototype's code and line vectors, growing each by `luaM_growaux` (`code size
overflow`), and returns its index. `luaK_codeABC` and `luaK_codeABx` pack the operands and take the
line of the last token read; `luaK_fixline` sets the line of the last instruction.

| function | name | decided by |
|---|---|---|
| `0x701820` | `luaK_code` | `luaK_dischargejpc` (`0x70182c`), two `luaM_growaux` (`0x701852`, `0x701884`), stores (`0x701860`, `0x701895`), pc raised (`0x70189f`) |
| `0x7018b0` | `luaK_dischargejpc` | `luaK_patchlistaux` with the pc for every target (`0x7018b3`..`0x7018c6`), pending jumps -1 (`0x7018cb`) |
| `0x7018e0` | `luaK_codeABC` | `((A << 9 | B) << 9 | C) << 6 | op` (`0x7018f0`..`0x701903`), the lexer's last line (`0x7018e5`, `0x7018e8`), `luaK_code` (`0x701905`) |
| `0x701910` | `luaK_codeABx` | `(A << 18 | Bx) << 6 | op` (`0x701920`..`0x70192b`), last line (`0x701918`), `luaK_code` (`0x70192d`) |
| `0x701810` | `luaK_fixline` | line of `pc - 1` (`0x701818`) |

Addresses: `0x701820`, `0x70182c`, `0x701852`, `0x701884`, `0x701860`, `0x701895`, `0x70189f`, `0x7018b0`, `0x7018b3`, `0x7018c6`, `0x7018cb`, `0x7018e0`, `0x7018f0`, `0x701903`, `0x7018e5`, `0x7018e8`, `0x701905`, `0x701910`, `0x701920`, `0x70192b`, `0x701918`, `0x70192d`, `0x701810`, `0x701818`, `0x70184a`, `0x70187c`
Confidence: verified
Proof: one
Test: none

`luaK_code` grows the vectors with the limit `0x7ffffffd` (`0x70184a`, `0x70187c`). The line comes
from the lexer state's `+8`, the line of the last token consumed (`0x7018e8`).
