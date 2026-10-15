# benilla-re

The reverse engineering of the World of Warcraft 1.12.1 client (`WoW.exe`, build 5875), read at
its bytes and cited by address. It is the reference behind
[benilla](https://github.com/samwhosung/benilla), a from-scratch 1.12.1 client in Rust, and it is
written for anyone else who needs to know what the client does: another client, a server, a
client mod, a format reader.

Bring your own client. Nothing here comes from a WoW install. The tools read `WoW/WoW.exe` from
your own copy of 1.12.1.5875 and refuse any other build.

## What is here

- **`symbols/`**: one row per function, global, struct and field read so far, with its name,
  owning system, signature, a one-line meaning, how sure that is (`verified` at the bytes, or
  `inferred`), and the finding that derives it.
- **`systems/`**: what each system does, one entry per mechanism, each with the addresses that
  decide it and its derivation. Written so far:
  [core](systems/core) (main loop, events, window input, clock),
  [net](systems/net) (the wire, dispatch, session, logon),
  [collision](systems/collision) (movement physics),
  [terrain](systems/terrain) (the world map, chunks, liquid, queries, cull and LOD),
  [models](systems/models) (M2 and WMO, the scene, rendering, animation, lights, particles),
  [lighting](systems/lighting) (the day-night lights, fog, sky, clouds, weather and shadows),
  [objectlayer](systems/objectlayer) (the update packet, objects, fields, reaction, units),
  [spell](systems/spell), [glue](systems/glue) (the login and character screens),
  [ui](systems/ui) (the frame and layout core),
  [sound](systems/sound) (the FMOD engine, sound entries, music, ambience and object sounds),
  [camera](systems/camera.md), [input](systems/input.md) and [random](systems/random.md).
  The rest of the client is open work in the issue tracker.
- **`tables/`**: facts a program reads (opcode handlers, events and their fire sites, CVars,
  Lua bindings, the message catalogue, DBC stores and layouts), each rebuilt from your exe by a
  generator in `tools/tables/`.
- **`crates/`**: reimplementations of the client's computations in Rust, a crate per system, each
  tested bit for bit by running the exe's own code beside it in an emulator (`crates/difftest`).
  Ported so far: `core`, `camera`, `random`, `screencoord`, `fastmath`, `gametime`, `sysmsg` and
  `minigame`; the rest follow ([#4](https://github.com/samwhosung/benilla-re/issues/4)).
- **`tools/`**: the listing the findings cite (`tools/disasm.sh`), a Ghidra project with this
  repo's names, signatures and types applied (`tools/ghidra/setup.sh`, and
  `tools/ghidra/decompile.sh` to decompile from it), the table generators, and the instruments
  below. Each opens with what it does and how far its answer goes.
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

## How to trust it

[METHOD.md](METHOD.md) says what counts as evidence: the binary decides, every claim states its
confidence and proof, and a contradiction is settled at the bytes. `cargo xtask check` holds the
repo to it: the formats, every address a doc cites having a row, every row's finding naming its
address, and every row of a written system having a class; with `--exe`, every function's size
against its body in the listing, every table against its generator, and every test a row or an
entry names against the exe. It does not walk a system's closure, and most of the bit-exact tests
that complete one are not ported yet ([#4](https://github.com/samwhosung/benilla-re/issues/4)).

## Contributing

Names, findings, corrections and checks are welcome: [CONTRIBUTING.md](CONTRIBUTING.md) covers
the setup and how a change is judged.

## License

MIT OR Apache-2.0, at your option. The tests link the Unicorn emulator (`unicorn-engine`,
GPL-2.0) as a test-only dependency, through `crates/difftest`; the repo's own code is MIT OR
Apache-2.0.
