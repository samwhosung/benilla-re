# Method

How a finding in this repo is made trustworthy. Everything here serves one test: someone who has
never seen the exe can implement the mechanism from the finding alone and match the client bit
for bit.

## The binary decides

The authority on what the client does is `WoW.exe` 1.12.1.5875 itself. A derivation is a path
from the bytes to the claim: addresses, opcodes and constants read from the disassembly or the
raw image, or the exe's own code executed in the emulator. Everything else is a lead or
corroboration, never a derivation: decompiler output, a name (ours, the community's or a
string's), a wiki, a server emulator, another client. A capture of the running client is an
observation: it can refute a reading or choose between two, but the bytes still supply the
mechanism.

Some instructions have no single exact answer: `rsqrtss` and `rcpss` approximate, and the x87
transcendentals (`fsin`, `fcos`, `fsincos`, `fptan`, `f2xm1`) differ by CPU. For these a
finding gives the full-precision value the instruction approximates, and a test compares them
with the host's math library structurally rather than bit for bit. Everything else is
deterministic, and the standard for it is exact.

Some code picks its implementation at run time: D3DX's math routines jump through slots filled by
CPU feature (x87, 3DNow or SSE versions), and the client itself chooses SSE paths by CPUID. A
finding over such code names the selector and each version, says which one a current SSE CPU
runs, and derives that one; a version it did not read is inferred.

## Confidence and proof

Every claim, in a row or an entry, carries a confidence and a proof, and names its tests.

- **verified**: read at the bytes, with the derivation recorded in a finding. A behaviour claim
  is verified only when the path from the bytes to the behaviour is complete; a verified byte
  fact does not verify a behaviour inferred from it.
- **inferred**: anything short of that. A name without a derivation, a callee named but not
  read, a purpose guessed from a caller. An inferred claim is a lead for the next reader.

Proof says how strongly a verified claim is held:

- **one**: one reader's derivation. The default for a detail local to one function.
- **two**: two independent readings, reconciled. Required where a wrong answer spreads: what goes
  on the wire, what is saved to disk, maths many systems build on (time, coordinates, random
  numbers, matrices), the frame and event core, and a system's roots and boundaries. Until the
  second reading lands, such a claim stays at `one`.

A test is recorded on its own line, never as a proof: a row's `test` column and an entry's `Test`
line name the bit-exact tests that run the exe's own code beside a reimplementation and pass, and
the entry says whether a test runs the whole function or a slice of it. A test proves the
transcription, not the prose: a direction, a unit, an order or a meaning the code cannot express
still rests on reading, so a test raises neither the confidence nor the proof.

Two readings are independent only when neither reader saw the other's work, its conclusions or
any note derived from it, and each re-derives every load-bearing step from the bytes. The second
reader gets the addresses and the question, never the answer, and ideally enters from the other
end: from a caller where the first started at the callee. A reviewer who checks a derivation
after reading it corroborates it; that is valuable, but it is not a second reading.

## A contradiction is settled at the bytes

When two readings, or a reading and a published finding, disagree, the question is settled by
finding the error: the instruction the losing side misread, quoted beside the correct reading. It
is never settled by counting readers or by deferring to the older note. If the error cannot be
located, a third reading is made. An observation that contradicts a complete reading is
recorded as unexplained, not fitted with a fudge factor. The winning entry is rewritten in place;
the commit message says what was wrong and where.

## Completeness

A system is understood when its closure is (`systems/README.md`): every function its roots reach
has a row with a class, every struct field its functions touch has a row, every branch is
accounted for, and every primitive has a bit-exact test. No system meets this yet: tests cover a
few primitives, and the rest are open work
([#87](https://github.com/samwhosung/benilla-re/issues/87)).

The closure is only as good as its roots, and a walk of direct calls misses code. Look for:

- functions without a frame prologue, which a prologue scan does not find;
- vtable slots, found by reading the vtable the constructor stores, not by offset arithmetic;
- registered handlers: any code address pushed or stored as an immediate, or held in a table;
- static initializers, which only the C runtime's initializer table reaches.

A census starts from true function starts. A function starts at a `call` target, a stored
address, or the first instruction after padding, unless the code above reaches it another way:
an address that the instruction before it falls through to, or that a conditional jump or a
jump-table entry of the function above lands on, is that function's code and never a start.
Executed `nop`s fall through, a case label is a stored address, and an exit block can sit after
`ret` and padding. An unconditional `jmp` to the first instruction after padding is a tail call to
a new function, as in the static-initializer thunks. Read the function above before deciding where
the next one starts.

The reliable census for a function is two scans with a control: every `e8`/`e9` whose target is
the function, and every little-endian occurrence of its address anywhere in the image. A function
with neither is unreached; say so in the doc rather than leaving it to look owned. Code that is
part of the function above is reached whenever that function is.

## Checklist: reading the bytes

- Bound a function by walking to its final `ret` followed by padding. A `ret` inside is a block
  end, not the function's end; padding can be zero or two bytes.
- Use virtual addresses everywhere. An RVA reads like a valid citation and decodes as unrelated
  code.
- Decide a predicate's polarity from the block that produces each result, not from the `jcc`
  alone, and decode every jump target before calling it a skip, a bail or a loop.
- After `fcomp`/`fnstsw ax`, `ah` holds C0 (`0x01`), C2 (`0x04`) and C3 (`0x40`); `jp`/`jnp`
  test the parity of the masked bits; an unordered compare sets all three. State what a NaN does
  on every float test.
- Settle the operand order of `fsubr`, `fdivr`, `fsubp` and `fdivp` from the opcode bytes against
  the Intel manual; disassemblers disagree on these mnemonics.
- `fst` stores without popping: the live `st0` keeps its full precision while the stored copy is
  rounded. Name, for each later use, whether it reads the register or the store.
- A return value is quoted through the `ret`; compilers put the final arithmetic after the
  epilogue pops.
- Read the calling convention from `ret N` and from which registers the callee reads before
  writing them.
- A `.data` value is a default until a writer census says otherwise; part of `.data` is
  zero-filled at load and has no file bytes at all.
- A field is named by the code that writes it, and its type by the instruction that loads it.
- When two paths converge on one store, the arriving jump decides what is stored.

## Checklist: proof by absence

A claim that something never happens is a census, and a census earns its zero.

- Every census carries a control: the same pattern, run the same way, finding a known positive.
  A zero without a control is not a result.
- Search every addressing form: `[reg+disp]`, absolute `moffs` loads and stores, `lea`, `push`,
  `rep movs`, `fst` and `fstp`, and a pointer computed into a register and stored through.
  A disassembler's spacing and sign conventions differ; write patterns that tolerate both.
- Count callers through both `call` and `jmp`, and through stored addresses.
- Count them only for a true start (Completeness). An address reached by fall-through from the
  instruction before it is the function above's code, with that function's callers; a census
  that treats it as a function with no callers declares live code unreached.
- Resolve every indirect call on the path (`call [reg+disp]`, `call [mem]`) to its table and the
  code that fills it before claiming it cannot reach something.
- A pointer through a writable slot is not resolved by its file value: find every writer.
- A census over a shared function has one line per call site.
- A scan that filters its candidates prints what it rejected and why.

## Checklist: writing a finding

- State the inputs the claim holds for. A finding about one case names it.
- A formula names its constants by value and its operations in the order the code does them,
  with the precision of each step.
- Effects in sequence state their order and what the first does to the second's inputs.
- A table states every column for every row, or says which were not derived.
- Name a function by what it does to its inputs, never by the scenario that first led to it.
- A correction re-derives every other use of the misread idiom in the same function.
- A consequence that depends on a value the finding did not settle says so.
- Never publish an instruction to implementers over a gap the finding itself names.

## Checklist: observations

- A proposed observation states what is seen if the claim holds, what is seen if it does not,
  and whether the difference exceeds the observer's noise.
- An observation retracts a byte-derived mechanism only after the mechanism's predicted size at
  that configuration is computed.
- A capture through a translation layer is read through the layer: identify it first.

## Checklist: operands, tables and calls

- Read a constant by its bytes at its address; a string near small integers is usually a source
  path and line number.
- Read a name-to-function table from its pairs, in the field order the loop that reads it uses.
- A vtable slot belongs to the table the constructor stores; a virtual call acts on the object in
  `ecx` at the call.
- A switch table is read from its raw dwords, and checked against a caller whose use of the case
  is known.
- A call's return-value store is the first store of `eax` after the call, which the compiler may
  schedule among the next call's pushes.

## The harness

A bit-exact test runs the exe's own code in the Unicorn emulator beside the reimplementation. The
harness is `crates/difftest`, and each system's crate under `crates/` holds a reimplementation and
its tests (`CONTRIBUTING.md`, Adding a test). The x87 setting below holds for every finding, the
emulator's limits after it bound what a test can show, and the rules after them are the ones each
test follows.

The client runs the x87 at 53-bit precision, rounding to nearest, every exception masked: the
control word `0x023F`. `WinMain` (`0x404100`) starts by calling `__clearfp` (`0x409bf2`) and
then `__control87(0x9001F, 0xFFFFFFFF)` (`0x409c06`, at `0x40410c`): `0x409d0e` turns the five
exception bits into the mask bits `0x3D` (`0x409d1b` to `0x409d38`), adds `0x2` for the denormal
bit (`0x409d42`) and `0x200` for 53-bit precision (`0x409d4d`, `0x409d83`), leaves rounding at
nearest, and `0x409c33` loads the result with `fldcw`. `0x5995c0` creates the
Direct3D 9 device with the flags `0x22` or `0x52` (`0x599614` to `0x59961c`), both of which
include `D3DCREATE_FPU_PRESERVE`, so the device never lowers the precision. Every finding and test
assumes this control word unless it says otherwise.

The emulator is the QEMU x86 core of `unicorn-engine` 2.1.5, and its x87 is not the client's at
every instruction. `f2xm1`, `fyl2x`, `fyl2xp1`, `fsin`, `fcos`, `fsincos`, `fptan`, `fpatan`,
`fprem` and `fprem1` convert their operands to the host's `double`, call its C library and
convert back, so a result carries 53 significant bits where the x87 rounds at 64, and the two
remainders are computed in `double`, not exactly. Their edges differ too: `fsin`, `fcos`,
`fsincos` and `fptan` reduce an operand that rounds to ±2^63 as a `double`, which the x87 reports
out of range, and
give ±∞ back unchanged with C2 set where the x87 returns the indefinite. `fyl2x` of 0 or less
and `fyl2xp1` of −1 or less, or of a NaN, set C2 and pop nothing, and `fyl2xp1` outside
`|x| < 1 − √2/2`, where the x87's result is undefined, gives the host's logarithm. `fprem` and
`fprem1` return the host's positive NaN for an infinite or NaN operand, a zero divisor, or
exponents so far apart that their scale `2^(gap − N)` overflows a double, as `fmod(5, 5e-320)`
does, and with exponents 53 or more apart their partial remainder is not the x87's. Beyond these,
a signalling NaN loaded from memory or stored to it stays signalling where the x87 quiets it, and
a read of an empty register returns what the register last held: there is no stack fault and no
indefinite. A test whose compared values pass through one of these shows the code around the
instruction, not the client's result there, and keeps its inputs off the other differences.
`BENILLA_RE_X87_CENSUS` names a file to which every test appends a line for each address of the
ten that it runs: its name as a row's `test` column gives it, the address and the mnemonic, and
`edge` on a second line once it runs one on an operand where the answers differ beyond rounding.
A test that runs one names it by address in its file's header and says what its comparison rests
on; a test with an `edge` line compares no value that passes through it there.

- Each test reads the reader's own `WoW.exe` and refuses any other build by its hash.
- The emulator pins the x87 control word to `0x027F` and `MXCSR` at entry and checks them after (`0x027F`
  differs from the client's `0x023F` only in reserved bit 6, which changes no result),
  starts with an empty x87 stack and cleared XMM registers, and stops a runaway by instruction
  count, not by time. An unmapped access names its address, the instruction and the kind of access.
- A stub stands in for a call the test does not run; it is identified by the callee's body and
  shown not to change the result.
- Inputs span every regime the code distinguishes: both signs, zero, each threshold exactly and a
  step either side, NaN and the infinities where floats are read. A threshold gets a
  deterministic probe, not only fuzz.
- A test runs the whole function where it can. A test of a slice says which instructions it
  covers.
- A test that cannot run reports itself skipped, and a skip is never a pass. A test finds the exe
  at `BENILLA_RE_WOW_EXE`, else at `WoW/WoW.exe`. With `BENILLA_RE_REQUIRE_EXE` set (to `1`), a
  test that needs the exe and cannot find it panics instead of skipping. `cargo xtask check --exe`
  sets it and requires every test the table and the docs name to appear as passed, so a test
  that skipped, was ignored or did not run fails the check.
- A file from a run of the real client, from the install or from a third party is never
  committed. A test that needs one reads it from a `BENILLA_RE_*` variable and skips without it,
  even with `BENILLA_RE_REQUIRE_EXE` set, so no row or entry names such a test.
- A tool that shells out checks its exit code and carries a control on every run, so a broken
  invocation fails loudly instead of returning an empty answer.
