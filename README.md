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

## What is here

- **`symbols/`**: one row per function, global, struct and field: its name, owning system,
  signature, a one-line meaning, how sure that is (`verified` at the bytes, or `inferred`) and the
  finding that derives it. Every function start `cargo xtask census` finds in `.text` has a row.
- **`systems/`**: what each system does, one entry per mechanism, each with the addresses that
  decide it and its derivation; [systems/README.md](systems/README.md) lists them and the doc
  format.
- **`tables/`**: facts a program reads (opcode handlers, events and their fire sites, CVars,
  Lua bindings, the message catalogue, DBC stores and layouts, direct calls), each rebuilt from
  your exe by a generator in `tools/tables/`.
- **`crates/`**: reimplementations of the client's computations in Rust, a crate per system, each
  tested bit for bit by running the exe's own code beside it in an emulator (`crates/difftest`).
- **`tools/`**: the listing the findings cite (`tools/disasm.sh`), a Ghidra project with this
  repo's names, signatures and types applied (`tools/ghidra/setup.sh`, and
  `tools/ghidra/decompile.sh` to decompile from it), the table generators, the scripts
  `cargo xtask export` packages, and the instruments below. Each opens with what it does and how
  far its answer goes.
  - `tools/callers.py`: every call, jump and stored address that names an address, with controls.
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
  `<site>/d/<doc path without .md>.html` a doc (`d/systems/random.html`, `d/METHOD.html`) and
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
    (`op/0x03a.html`); `op/index.html` lists them. A `name` column in `tables/opcodes.tsv`, once
    the table has one, shows on the page, the index and in the search.
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

- `benilla-re.h`: a C header of every struct at its table offsets (packed, with explicit
  `_pad` members for the gaps and `__vfptr` where a vtable exists), the prototypes whose types it
  can state (behind `BENILLA_RE_PROTOTYPES`, as no function can be linked), and the globals. It
  compiles as C and as C++. `::` is written `__`. For a call from injected code, each function
  has `BRE_ADDR_<name>` (its address) and `BRE_FN_<name>` (a pointer type with its calling
  convention, MSVC keywords under `_MSC_VER` and attributes for clang and gcc on i386), a
  `__thiscall` one also `BRE_FNX_<name>`, the `__fastcall` form with a dead `edx`; each global has
  its `BRE_ADDR_<name>`. Compile with `-DBENILLA_RE_LAYOUT_CHECK` to assert every size and
  offset. A row it cannot state is left out and counted in the export's output.
- `ghidra/`: `ApplySymbols.java` and `CreateFunctions.java` with the tables they read, and a
  README for running them on your own program.
- `ida/benilla_re.py` for IDA 9 and `binja/benilla_re.py` for Binary Ninja: each loads the
  header's types, then creates, names, types and comments every function and global. On a
  database that already has names (vmangos/idb) they keep a name that is not the tool's own
  default and add ours as a comment; `OVERWRITE_NAMES` at the top of the script replaces instead.
  They read `bundle/benilla-re.json` and `benilla-re.h` from beside them and need no checkout.
- `x64dbg/WoW.exe.dd32`: an x64dbg database of labels and comments for module `wow.exe`, to
  copy into x64dbg's `db` folder.
- `bundle/`: `symbols/` and `tables/` as TSV, and `benilla-re.json`: `schema` (1), `exe` (name,
  version, `size`, `sha256`), `commit`, and `tables`, a map from a table's path
  (`symbols/functions`) to its `columns` and `rows`, every cell a string as in the file.

## How to trust it

[METHOD.md](METHOD.md) says what counts as evidence: the binary decides, every claim states its
confidence and proof, and a contradiction is settled at the bytes. `cargo xtask check` holds the
repo to it: the formats, every address a doc cites having a row, every row's finding naming its
address, and every row of a written system having a class; with `--exe`, every function's size
against its body in the listing, every table against its generator, and every test a row or an
entry names against the exe. Each row says how far it goes: `inferred` is a lead, `verified` with
proof `one` is one reading of the bytes, proof `two` is two independent readings reconciled, and a
`test` runs the client's own code against the reimplementation. The site's front page counts each.

## Contributing

Names, findings, corrections and checks are welcome: [CONTRIBUTING.md](CONTRIBUTING.md) covers
the setup and how a change is judged, and [AGENTS.md](AGENTS.md) is the short version for a
coding agent working here.

## License

MIT OR Apache-2.0, at your option, for everything here: the code, the symbol table, the docs and
the tables. The tests link the Unicorn emulator (`unicorn-engine`, GPL-2.0) as a test-only
dependency, through `crates/difftest`. Names from the community database `vmangos/idb` carry
`idb` in their row's `name_from`. To cite the repo, see [CITATION.cff](CITATION.cff).
