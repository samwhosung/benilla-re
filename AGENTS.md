# benilla-re

The reverse engineering of the World of Warcraft 1.12.1 client (`WoW.exe`, build 5875), read at
its bytes. The binary is the spec: a claim here is what the exe does, derived from its bytes, with
its confidence stated, never what it probably does.

Read, in this order:

1. `docs/METHOD.md`: what counts as evidence, confidence and proof, and the checklists for reading the
   bytes. Binding for every change.
2. `docs/CONTRIBUTING.md`: the setup (your own exe at `WoW/WoW.exe`, the listing, the checks) and how
   a name, a finding or a test is added.
3. `symbols/README.md` and `systems/README.md`: the formats of a row and of a finding, the naming
   rule, and how a function's system is proved.

To look something up, grep before you read the bytes: `symbols/functions.tsv` (a row per function:
name, system, signature, meaning, confidence, the finding behind it), `symbols/globals.tsv`,
`tables/` (opcodes, events, CVars, Lua bindings, messages, calls) and the docs under `systems/`.
`cargo xtask census --why <address>` says what reaches an address; `tools/callers.py` lists its
callers.

The rules an agent most often breaks here:

- **The bytes decide.** A name, a decompiler's output, a doc in this repo, a server emulator or a
  wiki is a lead. Where a row and the bytes disagree, the row is wrong: correct it in place.
- **Say how sure you are.** `verified` needs a derivation recorded in a finding; anything less is
  `inferred`. A callee you did not read is stated as a call and its arguments, not as what it
  does. Before "only", "every", "never" or "no caller", run the search, with a control that finds
  a known positive.
- **Read the instruction, not the mnemonic you expect.** The x87 compare flags and `jp`, the
  operand order of `fsubr` and `fdivr` from the opcode bytes, `fst` against `fstp`, `ret N`,
  padding and false starts, and the listing going out of step after a jump table
  (`docs/METHOD.md`, "Checklist: reading the bytes").
- **Never commit anything from the install.** Not the exe, not data files, not interface code.
  Names and facts read from them are fine; their contents are not.
- **Run the checks.** `cargo xtask check` must report 0 problems, and `cargo xtask check --exe`
  too when you change a row's size, a table or a test. A pull request runs `check` in CI.

The rest of the root: `crates/` the reimplementations and their bit-exact tests, `tools/` the
generators and instruments, `xtask/` the checks, the site and the exports.

@docs/METHOD.md
