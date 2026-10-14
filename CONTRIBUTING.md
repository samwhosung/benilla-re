# Contributing

Corrections, names, findings and tests are all welcome. Read `METHOD.md` first: it says what
counts as evidence here. The formats are in `symbols/README.md` and `systems/README.md`.

## Setup

Bring your own client. Nothing from the game is in this repo, and nothing from it may be added:
no exe, no MPQ, no extracted file, no disassembly or decompiler dump (`dumps/` is ignored).

1. Copy your `WoW.exe` from a 1.12.1 (build 5875) install to `WoW/WoW.exe` at the repo root.
   `WoW/` is ignored by git.
2. Check it is the same build the findings were read from:

   | | |
   |---|---|
   | SHA-256 | `b4756d38ef207c02ed651f4952bd89a70b4857b73a33413339e1b285b28d2dc7` |
   | size | 4,775,986 bytes |

   `shasum -a 256 WoW/WoW.exe` on macOS and Linux, `certutil -hashfile WoW\WoW.exe SHA256` on
   Windows. A different hash is a different build, and the addresses here do not apply to it.
3. Read the code. Any disassembler works. The exe is a 32-bit PE with image base 0x400000, so
   the addresses in this repo are the addresses your tool shows. With LLVM's objdump:

   ```
   objdump -d -M intel --start-address=0x514ae0 --stop-address=0x514b35 WoW/WoW.exe
   ```

4. Write the disassembly the findings cite. Findings give instruction addresses, and the
   addresses are those of one listing: GNU objdump, Intel syntax, `.text` only, written by
   `tools/disasm.sh` to `dumps/disasm.txt`. A plain sweep of `.text` reads the jump tables inside
   code as instructions and can stay out of step after them, so the listing follows the real
   instruction stream: the sweep restarts at every function row and every jump-table entry, and
   leaves the tables' bytes out. The script refuses any other build of the exe, uses the first
   GNU objdump with the PE target it finds, runs `cargo xtask listing` (Rust, as for the check
   below), and prints the listing's size and SHA-256. The listing changes as rows are added, since
   each row restarts the sweep; the instruction addresses findings cite do not.

   ```
   tools/disasm.sh
   ```

5. Build the Ghidra project: the exe imported with the analysis options recorded in
   `tools/ghidra/SetOptions.java`, the functions analysis misses created from
   `symbols/functions.tsv` and every row's body made its size, and the symbol table applied
   (names, signatures, structs, labels and a plate comment per row). It takes about a minute.

   ```
   tools/ghidra/setup.sh
   ```

   The project is `dumps/ghidra/benilla-re`; open it in Ghidra 12.1 or later, or rerun
   `tools/ghidra/setup.sh --apply` after the table changes to apply it again to the project
   already built. The log, with every row it could not apply, is `dumps/ghidra/setup.log`.
   `tools/ghidra/decompile.sh 0x514ae0` decompiles functions from it without opening Ghidra.

`dumps/` is ignored by git and holds only what the scripts regenerate from your exe. Nothing in
it is ever committed.

### What the scripts need

The scripts are bash, and `tools/disasm.sh` also needs Rust (The check, below). Ghidra 12.x needs a
JDK 21 or later.

| | macOS (Homebrew) | Linux (Debian, Ubuntu) | Windows |
|---|---|---|---|
| objdump | `brew install mingw-w64` | `sudo apt install binutils` | in WSL, as Linux |
| Ghidra | `brew install ghidra` | release zip, below | in WSL, as Linux |
| JDK | `brew install openjdk@21` | `sudo apt install openjdk-21-jdk` | in WSL, as Linux |

- **macOS.** `/usr/bin/objdump` is LLVM's and is not used. `brew install mingw-w64` provides
  `i686-w64-mingw32-objdump`, which the script finds on the `PATH`. `brew install ghidra`
  is found by the script without further setup.
- **Linux.** Download the Ghidra release zip from
  `https://github.com/NationalSecurityAgency/ghidra/releases`, unzip it, and run
  `export GHIDRA_INSTALL_DIR=/path/to/ghidra_12.1_PUBLIC` (the folder that holds `support/`).
  The script needs a GNU objdump with the PE target: check `objdump -i | grep pei-i386`, and if
  it prints nothing, `sudo apt install binutils-mingw-w64-i686` and the script uses
  `i686-w64-mingw32-objdump`.
- **Windows.** Use WSL with a Linux distribution and follow the Linux column there, with the
  client copied into `WoW/` inside the WSL file system. Natively, steps 1 and 2 work as written
  (`certutil` for the hash), and with GNU objdump from MSYS2
  (`pacman -S mingw-w64-x86_64-binutils`) the disassembly is written by
  `cargo xtask listing <path to objdump.exe> dumps\disasm.txt`, the step `tools/disasm.sh` runs.

### The check

One command checks what the repo claims. It is Rust: install a toolchain with rustup, and
`rust-toolchain.toml` picks the stable one. Without the exe it runs anywhere, and CI runs it on
every pull request:

```
cargo xtask check
```

It checks the symbol table and every Markdown file against the formats in `symbols/README.md`
and `systems/README.md` (columns, sorting, names, classes, confidence and proof, the text limits,
every `finding` resolving to an entry that mentions its row's address, every cited address
having a row, every row's system known) and each table's header against `tables/README.md`. It prints one line per
problem, with its file and line, then a summary, and fails on any problem.

With your exe, it checks the rest:

```
cargo xtask check --exe
```

It refuses any exe but 1.12.1.5875, at `WoW/WoW.exe` or at the path in `BENILLA_RE_WOW_EXE`.
It writes `dumps/disasm.txt` with `tools/disasm.sh` when it is missing, or when a row starts where
it has no instruction (a row newer than the listing), and checks that every function row starts
a function and has its body's size. It regenerates every table into a
scratch directory and requires each byte for byte. It runs
`cargo test --workspace --profile exetest` with `BENILLA_RE_REQUIRE_EXE=1`, and requires every test that a row's `test` or an entry's `Test` line
names to appear as passed. It needs GNU objdump (above), Python 3, bash for `tools/disasm.sh`, a
POSIX `sh` for `tools/tables/regen.sh`, and Rust; not Ghidra or a JDK.

The tests build the Unicorn emulator from source, which needs CMake and a C compiler:
`brew install cmake` on macOS, `sudo apt install cmake build-essential` on Linux. Without the exe
`cargo test --workspace` builds and runs them, and each test that needs the exe skips with one
line; CI runs it that way.

A transcription can pass in a debug build and fail in an optimized one: the optimizer may fold a
float compare so that `-0.0` or NaN comes out differently. `exetest` (in `Cargo.toml`) inherits
`release` and keeps `overflow-checks` and `debug-assertions` on, so one run is optimized and still
catches overflow. To run the tests the way the check does:

```
BENILLA_RE_REQUIRE_EXE=1 cargo test --workspace --profile exetest
```

`cargo xtask cited <path to a benilla checkout>` lists every client address the benilla tree
cites that has neither a row nor a mention in a doc here: a report, not a check.

## Adding a name

1. Find the function's row in `symbols/functions.tsv`, or add one in address order. Check that
   the address starts a function and measure its size by the rules in `symbols/README.md`.
2. Read the function at the bytes, enough to state its meaning in one line, and record the
   derivation in its system's doc: a new entry, or a line in the entry that covers the mechanism.
3. Fill `name`, `meaning`, `confidence`, `proof` and `finding`. A name you give follows the
   client's C++ style (`symbols/README.md`): `Class::Method` where the class is established,
   otherwise a PascalCase name that says what the function does, not where you met it.
4. A name from `vmangos/idb` is checked at the bytes before use, keeps its spelling, and gets
   `idb` in `name_from`. A community name that the bytes contradict is not used.

One row per address. If the address already has a row in another system, the owner stays; your
finding can live in your system's doc and the row can link there.

## Adding a finding

1. Pick the system doc, or start one from the shape in `systems/README.md`.
2. Write the entry: the claim, the addresses that decide it, the confidence, the proof, the test,
   and the derivation from the bytes.
3. Give every address the entry cites a row, and point each row's `finding` at the entry.
4. If the claim is one that spreads (the wire, the disk, shared maths, the frame and event core,
   a system's roots), it needs a second independent reading before its proof is `two`. Ask for
   one in the pull request; the second reader works from the addresses and the question alone.
5. If the finding contradicts an existing entry, find the instruction one of them misread, and
   rewrite the wrong entry. Say what was wrong and where in the commit message.

## Adding a test

A system's reimplementation is a crate, `crates/<system>`, named after the system's doc
(`objectlayer`, package `benilla-re-objectlayer`), with `publish = false` and the workspace's
license, version and edition. Its tests live in `crates/<system>/tests/`, each a top-level
function of its file. A test runs the exe's own code through
`benilla-re-difftest`, a dev-dependency, beside the crate's Rust reimplementation, and compares the
results bit for bit. A function whose row is `unreached` gets neither.

- Test the whole function where possible; a slice test names the instructions it covers, and the
  entry's derivation says which.
- Cover every regime the code distinguishes, with a deterministic probe at each threshold.
- When the test passes against the exe, add it to the row's `test` column and the entry's `Test`
  line as `crate::test_fn`: the crate's directory name and the function's name. That name only
  reaches a test that is a top-level function of a file in `crates/<crate>/tests/`, never one
  inside a module. The row's confidence and proof stay as the reading left them (`METHOD.md`,
  Confidence and proof).
- `Wow::load()` finds the exe at `BENILLA_RE_WOW_EXE`, else at `WoW/WoW.exe`, and refuses any other
  build by its SHA-256. Without it a test prints one skip line and returns, unless
  `BENILLA_RE_REQUIRE_EXE` is set (to `1`): then it panics. `cargo xtask check --exe` sets it, so
  a skip there fails the check and is never a pass. `data_dir()` finds the install the same way,
  at `BENILLA_RE_WOW_DATA`, else `WoW/Data`.
- A file from a client run or a third party is read with `fixture("BENILLA_RE_…")`, which skips
  without it even under `BENILLA_RE_REQUIRE_EXE`; no row or entry names such a test.

The emulator is `unicorn-engine` (GPL-2.0), used as a test dependency only; nothing that ships
links it.

## Review

Every change lands by pull request, and a reviewer checks:

- that `cargo xtask check` passes, as CI runs it: the formats and the text limits hold, every
  row's `finding` resolves to an entry that mentions its address, and every cited address
  resolves to a row;
- that `cargo xtask check --exe` passes on the reviewer's own exe: every function row starts a
  function and has its body's size, the tables regenerate byte for byte, and every named test
  passes;
- each new or changed claim against the bytes, by reading the cited instructions;
- that `verified` rows have a derivation, `inferred` rows say `none` for proof, and `two` is
  claimed only with a second reader who worked cold;
- that a correction rewrites the wrong entry instead of adding a second one beside it.

Keep a pull request to one system or one mechanism. Quote a few instructions where they decide a
point; never paste a listing.

## License

MIT OR Apache-2.0, at your option. By contributing you agree that your contribution is licensed
the same way.
