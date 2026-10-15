# benilla-re

The reverse engineering of the World of Warcraft 1.12.1 client (`WoW.exe`, build 5875), read at
its bytes and cited by address. It is the reference behind
[benilla](https://github.com/samwhosung/benilla), a from-scratch 1.12.1 client in Rust, and it is
written for anyone else who needs to know what the client does: another client, a server, a
client mod, a format reader.

Browse it at **[samwhosung.github.io/benilla-re](https://samwhosung.github.io/benilla-re/)**: a
page per function, global and struct, per system, per Lua function, event, CVar and opcode, with
search, and `?0x5e6853` to jump to whatever holds an address.

Bring your own client. Nothing here comes from a WoW install. The symbol table, the docs, the
tables, the site and the exports are plain files that need no exe; only the tests, the table
generators and the listing read `WoW/WoW.exe` from your own copy of 1.12.1.5875, and they refuse
any other build.

## Start here

- **Patching or injecting into the client** (a DLL, a loader, a fix): `cargo xtask export` writes
  `benilla-re.h`, every struct with its layout asserted and every function's address with a typed
  pointer in its calling convention, and an x64dbg database ([below](#start-from-the-names)). Look
  an address up on the site, or grep `symbols/functions.tsv`.
- **Reversing it in Ghidra, IDA, Binary Ninja or x64dbg**: the same export has an import for each,
  which keeps the names your database already has ([below](#start-from-the-names)).
- **Writing a server or an emulator**: the [opcodes](tables/opcodes.tsv) and their handlers, what
  the client reads from each message and checks before it sends one, in
  [net](systems/net), [objectlayer](systems/objectlayer), [spell](systems/spell) and
  [collision](systems/collision) (movement); [warden](systems/warden) for the anticheat channel.
- **Writing addons**: every Lua function with its argument count ([bindings](tables/bindings.tsv)),
  every event with where it fires ([events](tables/events.tsv),
  [event_sites](tables/event_sites.tsv)), every CVar with its default and help text
  ([cvars](tables/cvars.tsv)); on the site under Lua, events and CVars.
- **Reading the file formats** (M2, WMO, ADT, DBC, MPQ): what the client itself reads, in
  [models](systems/models), [terrain](systems/terrain), [dbc](systems/dbc.md) and
  [storm](systems/storm), and the record layouts in [dbc_schemas](tables/dbc_schemas.tsv).
- **Writing another client**: the [systems](systems/README.md) index, and the crates, which
  reimplement the client's computations and are tested bit for bit against its own code.

Slash commands and most of what the interface does (`/who`, the chat frame, the spellbook) are
written in the interface code, FrameXML and GlueXML in the player's own install, which calls the
Lua API the exe serves. A question about `/who` starts in that code and comes here for the engine
side: the Lua function in [bindings](tables/bindings.tsv), the opcode it sends, the event it
fires.

## What is here

- **`symbols/`**: one row per function, global, struct and field: its name, owning system,
  signature, a one-line meaning, how sure that is (`verified` at the bytes, or `inferred`) and the
  finding that derives it. Every function start `cargo xtask census` finds in `.text` has a row.
- **`systems/`**: what each system does, one entry per mechanism, each with the addresses that
  decide it and its derivation; [systems/README.md](systems/README.md) lists them and the doc
  format.
- **`tables/`**: facts a program reads (opcode handlers, send sites and the entries that cover
  them, events and their fire sites, CVars, Lua bindings, the message catalogue, DBC stores and
  layouts, direct calls), each rebuilt from your exe by a generator in `tools/tables/`.
- **`crates/`**: reimplementations of the client's computations in Rust, a crate per system, each
  tested bit for bit by running the exe's own code beside it in an emulator (`crates/difftest`).
- **`tools/`**: the listing the findings cite (`tools/disasm.sh`), a Ghidra project with this
  repo's names, signatures and types applied (`tools/ghidra/setup.sh`, and
  `tools/ghidra/decompile.sh` to decompile from it), the table generators, the scripts
  `cargo xtask export` packages, and the instruments below. Each opens with what it does and how
  far its answer goes.
  - `tools/callers.py`: every call, jump and stored address that names an address, with controls.
  - `tools/conventions.py`: the calling convention of every row with a signature, from the
    registers its body reads and what each caller loads before the call.
  - `tools/closure.py`: a system's closure walked from its roots, and what in it lacks a class,
    a reading or a test.
  - `tools/clones.py`: functions whose bytes match once the addresses in them are masked.
  - `tools/class-tags.py`: the compiler's decorated type names and the code that uses each.
  - `tools/coverage.py`: which instructions of each function the bit-exact tests ran.
  - `tools/fst-live.py`: where a float is stored with `fst` and then used unrounded.
  - `tools/event-args.py`: the `SignalEvent2` sites whose format reads more than the caller
    pushed.
  - `tools/idb-names.py`: the community names of `vmangos/idb`, for `cargo xtask census --fill`.
  - `tools/capture/`: what the running client measures where the emulator cannot reach: an addon
    (`BenillaReCapture`: glyph widths, `math.random` draws) for the tests that read such a file,
    and rigs inside the client's process that record the player's movement and a function's
    calls.

## The address site

`cargo xtask site <out dir>` writes a static website from `symbols/`, `systems/`, the spec docs
and `tables/`; it needs no exe, empties the folder first (and refuses one that does not hold a
previous site), and the same input writes the same bytes. `.github/workflows/pages.yml`
builds it on every push to main and deploys it to GitHub Pages. It is plain HTML with one
stylesheet and one script, and it reads on a phone and in a dark theme.

Addresses are written `0x` and lowercase hex, and every page URL is built from the address alone:

- `<site>/?0x5e6850` goes to the row that holds the address: a row's own address, an instruction
  inside a function (`start <= a < start + size`) or a byte inside a global's extent. An
  instruction lands on `<site>/a/5e6850.html#0x5e6853`, the function's page with the
  instruction named. This is the link for any address a comment cites.
- `<site>/a/<six lowercase hex digits>.html` is the page of the row that starts there, for
  example `a/5e6850.html`. Use it when the address is known to be a row's start.
- `<site>/t/<struct>.html` is a struct (`::` written `--`), `<site>/s/<system>.html` a system,
  `<site>/d/<doc path without .md>.html` a doc (`d/systems/random.html`, `d/docs/METHOD.html`) and
  `<site>/tb/<table>.html` a table (one over 5,000 rows shows its first 200 and links the file).
  A system with many rows continues on `<site>/s/<system>/2.html`, `3.html` and so on.
- For a name, not an address:
  - `<site>/lua/<name>.html` is a Lua function, from `tables/bindings.tsv` (`lua/GetItemInfo.html`);
    `lua/index.html` lists them by registrar table. A name registered in several tables is one
    page. Two names that differ only by case share no file: the one with a capital writes it
    `~` and the lowercase letter (`Lower` is `lua/~lower.html`).
  - `<site>/event/<NAME>.html` is an event with its fire sites (`event/UNIT_HEALTH.html`);
    `event/index.html` lists them.
  - `<site>/cvar/<name>.html` is a CVar with its default, flags, help text and callback
    (`cvar/M2UseZFill.html`); `cvar/index.html` lists them.
  - `<site>/op/<0x and three lowercase hex digits>.html` is an opcode with its handler
    (`op/0x03a.html`); `op/index.html` lists them. The `name` column of `tables/opcodes.tsv` shows
    on the page, the index and in the search. An opcode the client sends and registers no handler
    for (`tables/opcode_sends.tsv`) has a page too, and every opcode page lists its send sites and
    the entries that cover it (`tables/opcode_entries.tsv`).
  - Any other byte in a name is written `%XX`.

An address page has the row's name, signature or type, meaning, confidence and proof, the claim
of the finding it links to, its callers and callees, and its test. A name page has the table's
row, and the doc entries that cite its address. A system page has its docs'
overviews, the coverage counts (named, verified, proof two, tested), its closure and its rows. The
search box finds names and addresses from `index.json`, loaded in the browser.

Callers and callees come from `tables/calls.tsv`, which `cargo xtask check --exe` rebuilds and
compares with the committed file: the direct calls and jumps by a bare address. A function reached
through a register, a slot or a stored address shows none for that path. The table also leaves out
a function's calls to itself, calls and jumps to code that has no row, and jumps into the middle
of another row.

## Start from the names

`cargo xtask export <out dir>` writes the symbol table as files to start a session from, so that
a function is `CDataStore::PutInt8` and not `FUN_00514ae0`. It needs no exe, and the same tree
writes the same bytes.

- `README.md`: where to start for each tool, the build the files fit (version, size, SHA-256),
  the commit, and how a `finding` cell maps to a file and to the site.
- `benilla-re.h`: a C header of every struct at its table offsets (packed, with explicit
  `_pad` members for the gaps and `__vfptr` where a vtable exists), and the declarations of the
  functions and globals whose types it can state. It compiles as C and as C++. `::` is written
  `__`. For code that runs in the client's process, each function has `BRE_ADDR_<name>` (its
  address) and `BRE_FN_<name>` (a pointer type with its calling convention, MSVC keywords under
  `_MSC_VER` and attributes for clang and gcc on i386), a `__thiscall` one also `BRE_FNX_<name>`,
  the `__fastcall` form with a dead `edx`; each global has `BRE_ADDR_<name>` and
  `BRE_G_<name>`, the global as an lvalue of its type. The top comment shows a call and a global
  read. The plain declarations (`extern` globals and prototypes) are behind
  `-DBENILLA_RE_PROTOTYPES`, as nothing in `WoW.exe` can be linked. Compile with
  `-DBENILLA_RE_LAYOUT_CHECK` to assert every size and offset. A row it cannot state is left out
  and counted in the export's output.
- `ghidra/`: `ApplySymbols.java` and `CreateFunctions.java` with the tables they read, and a
  README for running them on your own program.
- `ida/benilla_re.py` for IDA 9 and `binja/benilla_re.py` for Binary Ninja: each loads the
  header's types, then creates, names, types and comments every function and global. On a
  database that already has names (vmangos/idb) they keep a name that is not the tool's own
  default and add ours as a comment; `OVERWRITE_NAMES` at the top of the script replaces instead.
  They read `bundle/benilla-re.json` and `benilla-re.h` from beside them and need no checkout.
- `x64dbg/WoW.exe.dd32`: an x64dbg database of labels and comments for module `wow.exe`, to
  copy into x64dbg's `db` folder.
- `bundle/`: `symbols/` and `tables/` as TSV, each with its `README.md` (what every column
  means), and `benilla-re.json`: `schema` (1), `exe` (name, version, `size`, `sha256`), `commit`,
  `repo` and `site` (the repository's and the site's URLs), and `tables`, a map from a table's
  path (`symbols/functions`) to its `columns` and `rows`, every cell a string as in the file.
  `repo` and `site` are additions to schema 1; a reader that ignores unknown keys is unaffected.
- `docs/` (`docs/METHOD.md`, `docs/CONTRIBUTING.md`) and `systems/` (every system doc): the Markdown a
  `finding` cell names, so that `net/friends#who-query-tags` is the file
  `systems/net/friends.md` at its heading, and on the site `d/systems/net/friends.html`.
- With `--symbols <dir>`, a folder rebased onto another build (Other builds), the export has the
  folder's `symbols/` only: the other tables (opcodes, cvars, bindings, the call tables) hold
  5875's addresses and are left out, `exe` has `size` and `sha256` null and `rebased` true, and
  the header says the addresses are matches, not proof. The scripts apply such a bundle to any
  build and say so.
- `bundle/signature.tsv`: one line per function row, `address`, `name`, `pattern`, `unique`: the
  masked bytes below. The row's other columns are in `tables/signatures.tsv`.

## Other builds

Every row is keyed to 1.12.1.5875, but a function's bytes are mostly the same in a modified
1.12 client, in 1.12.2 and 1.12.3, and in 1.6 to 1.11. `tables/signatures.tsv` gives each
function row a pattern: the body's bytes, hex and spaced, with `??` for each byte that depends
on where the code sits. Wildcards cover the rel32 of a `call`, `jmp` or conditional jump that
leaves the function (a rel8 that leaves it too, with or without a prefix), and every four-byte
operand that holds an address in the image (a global, a string, a vtable, a jump table and its
entries, of any length). The 5875 image has no `.reloc` section, so the decoded
instructions in the listing supply them (`dumps/disasm.txt`, which your clone lacks until
`tools/disasm.sh` writes it from your exe). A pattern is cut to the shortest prefix, at least
16 bytes, that occurs once in the executable sections; a body that is shorter, or whose whole
body occurs more than once, keeps all of it. `unique` says which. A pattern has at least one
fixed byte. The table also holds each row's `size`, its `calls`, and a `body`: the masked bytes
through the last call site, when the pattern stops before it.

```
cargo xtask match <exe> <out>
```

reads any PE, searches its executable sections for every pattern, and writes `<out>/match.tsv`
(`address` in 5875, `name`, `found`: the address in that build, `missing`, `ambiguous` or
`conflict`; and `via`: `pattern`, `call` or `order`), `<out>/conflicts.tsv` (why each conflict is
one) and `<out>/symbols/`, the tables with each found function at its new address and the others
left out. Structs and fields are copied; globals have no pattern, so that table is empty.
`cargo xtask export <dir> --symbols <out>/symbols` writes the header, the scripts' inputs and the
database from it. It needs no 5875 exe, only the checkout.

A pattern alone leaves most rows ambiguous, so the matcher builds on what it has found, in steps:

1. **Pattern**: the row's pattern occurs once, and the row is `unique`. A row that is not unique
   in 5875 is never found by pattern, however few places it matches in the build: its twin may
   be what is left.
2. **Calls**: a found function whose whole `body` matches where it was found has, at the offsets
   its 5875 body has them, a `call` or `jmp` whose 5875 target is row X. X is at the address
   that instruction goes to, when X's pattern matches there and has at least 6 fixed bytes.
   Repeated until nothing new is found.
3. **Order**: the found rows whose addresses rise with 5875's bound the rows between them (a
   found row off that rising run moved, and bounds nothing). A row with at least 6 fixed bytes
   and exactly one address its pattern matches between its bounds, outside the extent (5875's
   `size`) of every found row, is found there, and so are several rows between two bounds that
   have exactly one way to take rising addresses. Rows in one gap with equal patterns are never
   placed by order. Alternates with step 2 until nothing new is found.

`match.tsv`'s `via` says how much a placement is worth:

- `pattern`: the row's own bytes, unique in 5875 and found once in the build. The strongest, and
  still bytes and position: a changed body can keep a prefix that matches.
- `call`: a found function's call goes there and the row's pattern matches there. Weaker: it
  trusts the found function's call offsets and the row's bytes at one address. A short pattern
  or a rewritten caller can send it to another function.
- `order`: the row's neighbours bound it and its pattern matches once between them. Weakest: it
  trusts that the build keeps 5875's order and sizes. A deleted function can give its name to
  another function that looks like it, and a pattern of 6 fixed bytes is little evidence.

A row with several candidates left stays `ambiguous`. Two rows claiming one address, a row claimed
at two addresses, a claimed address that its pattern refutes, and two rows in one gap that fall
out of order are `conflict`: reported and left out, with what was found from them, not resolved.

On 5875 itself 14,383 of the 37,967 rows have a unique pattern and the matcher finds all 14,383
at their own address. The other 23,584 are indistinguishable by their bytes (the `jmp` thunks,
the static initializers, the one-line accessors). After each step:

| step | found | of which new | ambiguous | conflict | at a wrong address |
|---|---|---|---|---|---|
| pattern | 14,383 | 14,383 | 23,584 | 0 | 0 |
| calls | 17,254 | 2,871 | 20,713 | 0 | 0 |
| order | 21,243 | 3,989 (3,986 by order, 3 by calls) | 16,724 | 0 | 0 |

Nine synthetic builds of 5875 test it (`xtask/src/builds.py`, run by the matcher's tests; the
fast test makes three, `cargo test -p xtask -- --ignored` all nine): the same bytes shifted by
an odd amount, with odd padding, with 60 neighbour pairs swapped, with 12 functions duplicated,
with 200 calls sent into the middle of a function, with 200 thunk calls sent to another thunk,
with 300 functions deleted, with 100 callees given a new prologue, and unchanged. Each is
checked against where every function is in it:

| build | right | wrong | conflict | ambiguous |
|---|---|---|---|---|
| unchanged | 21,243 | 0 | 0 | 16,724 |
| shifted | 21,243 | 0 | 0 | 16,724 |
| padded | 21,243 | 0 | 0 | 16,724 |
| 60 pairs swapped | 21,238 | 0 | 2 | 16,727 |
| 12 duplicated | 21,242 | 0 | 0 | 16,725 |
| 200 calls into a function | 21,114 | 0 | 126 | 16,727 |
| 200 thunk calls swapped | 21,243 | 0 | 0 | 16,724 |
| 300 deleted | 21,128 | 0 | 33 | 16,806 |
| 100 new prologues | 21,214 | 0 | 28 | 16,725 |

These builds change what a compiler would not, and no real second build has been run through
the matcher yet. A found row is a match of bytes and position, not a proof that the function is
the same: a changed body can still match, a call can move with a changed caller, and a row's
`size` is 5875's. Take `call` and `order` rows as leads to check, and `pattern` rows as likely.

## How to trust it

[METHOD.md](docs/METHOD.md) says what counts as evidence: the binary decides, every claim states its
confidence and proof, and a contradiction is settled at the bytes. `cargo xtask check` holds the
repo to it: the formats, every address a doc cites having a row, every row's finding naming its
address, and every row of a written system having a class; with `--exe`, every function's size
against its body in the listing, every table against its generator, and every test a row or an
entry names against the exe. Each row says how far it goes: `inferred` is a lead, `verified` with
proof `one` is one reading of the bytes, proof `two` is two independent readings reconciled, and a
`test` runs the client's own code against the reimplementation. The site's front page counts each.

## Contributing

Names, findings, corrections and checks are welcome: [CONTRIBUTING.md](docs/CONTRIBUTING.md) covers
the setup and how a change is judged, and [AGENTS.md](AGENTS.md) is the short version for a
coding agent working here. A gap or a wrong finding is an issue, and needs no exe
([CONTRIBUTING.md](docs/CONTRIBUTING.md), "Report a gap or a wrong finding").

## License

MIT OR Apache-2.0, at your option, for everything here: the code, the symbol table, the docs and
the tables. The tests link the Unicorn emulator (`unicorn-engine`, GPL-2.0) as a test-only
dependency, through `crates/difftest`. Names from the community database `vmangos/idb` carry
`idb` in their row's `name_from`. To cite the repo, see [CITATION.cff](CITATION.cff).
