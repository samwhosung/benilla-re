# benilla-re export

The names, types and addresses of the World of Warcraft 1.12.1 client (`WoW.exe`, build 5875),
read from its bytes, as files to start a session from. Nothing here comes from a WoW install; you
bring your own `WoW.exe`.

{{BUILD}}

## Where to start

| You are | Open first | Then |
|---|---|---|
| Writing injected code (a DLL, a loader, a patch) | `benilla-re.h` | Its top comment has a call and a global read. |
| Using IDA 9 | `ida/benilla_re.py` | Steps below. |
| Using Binary Ninja | `binja/benilla_re.py` | Steps below. |
| Using Ghidra 12.1 or later | `ghidra/README.md` | Headless; the scripts and tables are in `ghidra/`. |
| Using x64dbg | `x64dbg/WoW.exe.dd32` | Steps below. |
| Writing a server, a tool or a reader | `bundle/` | `bundle/benilla-re.json` is every table in one file. |
| Wanting to know why a name is what it is | `systems/` | One doc per system, with the derivation from the bytes. |

Check your exe against the build above first: any other build has other addresses.

- **IDA.** Open your `WoW.exe` in IDA 9 and let analysis finish. Run `ida/benilla_re.py` with
  File > Script file. Keep the folders as they are here: the script reads `bundle/benilla-re.json`
  and `benilla-re.h` from beside it. It loads the header's structs, creates the functions the
  database lacks, and names, types and comments every function and global. A name already in the
  database is kept and ours is added as a comment; `OVERWRITE_NAMES` at the top of the script
  replaces instead.
- **Binary Ninja.** Open your `WoW.exe`, then Plugins > Run Script and pick
  `binja/benilla_re.py`; or run `python3 binja/benilla_re.py path/to/WoW.exe out.bndb` headless.
  It reads the same two files and behaves as the IDA script does.
- **x64dbg.** Copy `x64dbg/WoW.exe.dd32` into the `db` folder of x64dbg, beside `x32dbg.exe`
  (the `x32` folder of the release), then debug your `WoW.exe`: x32dbg loads the database of the
  module `wow.exe` when the module loads. Or use File > Import database in x32dbg. It holds labels
  and comments.
- **Ghidra.** `ghidra/README.md` has the two `analyzeHeadless` runs. The program is
  `<ghidra install>/support/analyzeHeadless`; it is not on the `PATH`.

## What each folder is

- `benilla-re.h`: a C and C++ header. Every struct sits at its table offsets, packed. Every
  function has `BRE_ADDR_<name>` and a pointer type `BRE_FN_<name>` in its calling convention
  (`BRE_FNX_<name>` for a `__thiscall` function, as a `__fastcall`). Every global has
  `BRE_ADDR_<name>` and `BRE_G_<name>`, the global itself as an lvalue. `::` is written `__`.
  `-DBENILLA_RE_PROTOTYPES` adds plain declarations, which cannot be linked; the types, the
  addresses and `BRE_G_` need no define. `-DBENILLA_RE_LAYOUT_CHECK` asserts every size and offset.
- `ida/`, `binja/`, `ghidra/`, `x64dbg/`: the import for each tool, above.
- `bundle/`: the data, as files a program reads.
  - `benilla-re.json`: `schema` (1), `exe` (name, version, `size`, `sha256`), `commit`, `repo`,
    `site`, and `tables`, a map from a table's path (`symbols/functions`) to its `columns` and
    `rows`. Every cell is a string, as in the TSV file.
  - `symbols/`: one TSV per kind (`functions`, `globals`, `structs`, `fields`), and `README.md`,
    which says what every column means. A function row has `address`, `size`, `name`, `signature`,
    `meaning`, `confidence` (`verified` or `inferred`), `proof` (`one`, `two` or `none`) and
    `finding`, the doc entry that derives it.
  - `tables/`: facts a program reads (opcodes, events, CVars, Lua bindings, messages, DBC layouts,
    calls), one TSV each, and `README.md`, which says what every column means. For the network:
    `opcodes.tsv` is the handler table, the messages the client receives; the messages it sends
    are in `opcode_sends.tsv` (the call that sends and the instruction that supplies the opcode),
    and `opcode_entries.tsv` joins both to the docs that cover them.
  - `signature.tsv`: one row per function row, with the columns `address`, `name`, `pattern` and
    `unique`. The pattern is the function's bytes in hex, spaced, with `??` for each byte that
    depends on where the code sits; it is the shortest prefix, at least 16 bytes, that occurs once
    in the executable sections of 5875, or the whole body when it is shorter or not unique.
    `unique` is `yes` when the pattern occurs once and `no` when the whole body occurs more than
    once. It finds a function in another build by its bytes. A row with no name is `FUN_` and its
    address in eight hex digits. `size`, `body` and `calls` of the same rows are in
    `tables/signatures.tsv`.
- `docs/`: `METHOD.md` (what counts as evidence, and what `confidence` and `proof` mean) and
  `CONTRIBUTING.md`.
- `systems/`: one Markdown doc per system, an entry per mechanism with the addresses that decide
  it and its derivation.

## From a `finding` to its doc

A function row's `finding` cell, and an `entries` cell of `opcode_entries.tsv`, name a doc and an
anchor: `net/who#who-query-tags`.

- In this export it is the file `systems/net/who.md`, at the heading `who-query-tags`. A
  finding of `METHOD#...` is `docs/METHOD.md`.
- On the site it is `{{SITE}}d/systems/net/who.html#who-query-tags`.

An address looks up on the site as `{{SITE}}?0x5e6853`, which goes to the row that holds it. A
doc's link to another file of the repository (`symbols/README.md`, `tables/README.md`) is here
`bundle/symbols/README.md` and `bundle/tables/README.md`; the others (`tools/`, `crates/`) are in
the repository.

## Links

- The repository: {{REPO}}
- The site: {{SITE}}
- Report a gap or a wrong finding: {{REPO}}/issues. Without an exe, name the address or the
  behaviour, what you expected and where it shows.
- `docs/CONTRIBUTING.md` and `docs/METHOD.md` in this export say how a change is judged.

This export is generated by `cargo xtask export` from the repository at the commit above; the same
commit writes the same bytes. Licensed MIT OR Apache-2.0.
