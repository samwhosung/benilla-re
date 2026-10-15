# The tables

Facts about `WoW.exe` 1.12.1.5875 that a program reads, each one a tab-separated file with a
header row, UTF-8, one row per fact, and a generator in `../tools/tables/`. Nothing here is edited
by hand: a wrong row is a wrong generator. There are two classes. A derived table is rebuilt from
your own copy of the exe. A captured table (below) records a run of the client and is committed as
generated.

```
tools/tables/regen.sh [DIR]  # rebuilds every table, into tables/ or DIR
python3 tools/tables/<name>.py -o tables/<name>.tsv
```

The generators are Python 3 with the standard library only. They read `WoW/WoW.exe` (they refuse
any other build, by size and SHA-256) and, for the ones that read instructions, `dumps/disasm.txt`,
which `regen.sh` writes with `../tools/disasm.sh` when it is missing. Each run twice gives
byte-identical output. Options on every generator: `--exe PATH`, `--disasm PATH`, `--tables DIR` (where a generator that
reads other tables finds them), `-o PATH`.

`cargo xtask vendored <benilla checkout>` compares the copies a consumer keeps with these files
(Vendored copies, below).

`cargo xtask check` checks each table's header against the columns this file gives it, and
`cargo xtask check --exe` regenerates every derived table into a scratch directory and fails on
any byte that differs from the committed one.

`signatures.tsv` is the exception to the generators: `cargo xtask signatures` writes it, from
the function rows, the exe and `dumps/disasm.txt`, and `cargo xtask check --exe` compares it.

## Conventions

- **Addresses** are `0x` and six lowercase hex digits (`0x703e50`), virtual addresses in the exe,
  as in `../symbols/README.md`. A cell that holds several (a conditional default) joins them
  with `|`.
- **Numbers** that are not addresses are decimal, except an opcode, which is `0x` and three hex
  digits, and a message type tag, `0x` and two.
- **`?`** in a cell means the generator could not resolve it to a constant. It never guesses; the
  reason is in `notes` where the table has one.
- **An empty cell** is a null: no help text, no callback, no format.
- Rows are sorted by the first column (`bindings.tsv`: by `name`, then `fn`).

## opcodes.tsv

Every network message the client registers a handler for (387 rows, 207 distinct handlers): the
handler table, the messages the client receives. The messages it sends are in `opcode_sends.tsv`
and `opcode_entries.tsv`.

| column | meaning |
|---|---|
| `opcode` | The message id, `0x000` to `0x33b` (the handler tables hold 828 entries). Sorted. |
| `name` | The message's name in vmangos's `Opcodes_1_12_1.h` (`src/game/Server/Protocol/`), from the committed input `../tools/tables/opcode_names.tsv`, which holds each entry's line in that file. The client holds no names. Empty where vmangos lists none. |
| `handler` | The function that runs when the message arrives. Several opcodes may share one. |
| `site` | The `call` that registers it. |
| `path` | `A`: through the wrapper `0x5ab650`; `B`: a direct call to `NetClient::SetMessageHandler 0x537a60`. |

## opcode_sends.tsv

Every place the client puts an opcode into an outgoing packet and sends it (394 rows, 310 distinct
opcodes). `../tools/tables/opcode_sends.py` opens with the contract: it reads the function that
holds each call to `ClientServices::Send 0x5ab630`, `NetClient::Send 0x5379a0` or the minigame's
`0x728640`, follows the opcode written first into the packet's store, and resolves an opcode passed
as an argument through its callers.

| column | meaning |
|---|---|
| `opcode` | The message id, or `?` where the opcode is computed at run time or not read. Sorted, `?` last. |
| `name` | The name, as in `opcodes.tsv`. |
| `site` | The send call; for a note `through-<address>`, the call to that function that passes the opcode. |
| `function` | The function row that holds the site. |
| `put` | The instruction that supplies the opcode; empty for `?`. |
| `send` | `A` for `ClientServices::Send`, `B` for the direct send. |
| `notes` | `;`-separated: `via-register`, `through-<address>`, `cache-object` (the opcode is the one a cache's constructor passes), `hand-resolved`, `opcode-not-constant`, `store-not-built-here`, `unreached` (the function is unreached). |

## opcode_entries.tsv

For each handler registration and each send, the entries of `../systems/` that cover the code (781 rows).
`../tools/tables/opcode_entries.py` opens with the contract.

| column | meaning |
|---|---|
| `opcode` | As in `opcode_sends.tsv`. Sorted, `?` last, then `role`, then `site`. |
| `name` | The name. |
| `role` | `handler` for a row of `opcodes.tsv`, `send` for a row of `opcode_sends.tsv`. |
| `site` | The registration call or the send call. |
| `function` | The handler, or the function that holds the send. |
| `entries` | The covering entries as `<doc>#<anchor>`, `\|`-separated, the doc relative to `systems/` without `.md`; empty for `unread`. |
| `basis` | `opcode`: the listed entries cover the function and name the opcode; `function`: they cover the function and none names the opcode; `unread`: no entry covers the function. |

## spell_failed.tsv

The cast failure reasons (146 rows): the reason byte the client reports for a refused or failed cast
and the GlobalStrings key it shows. `../tools/tables/spell_failed.py` opens with the contract.

| column | meaning |
|---|---|
| `reason` | The reason byte, `0x00` to `0x91`. `0x91` is the key any byte above `0x90` shows. Sorted. |
| `key` | The GlobalStrings key, `SPELL_FAILED_` and a name. |
| `address` | The key's string. |

## messages.tsv

The game UI's message catalogue, the 465-record table at `0xb4b498` that `0x496720` indexes.

| column | meaning |
|---|---|
| `id` | The index, `0` to `464`. |
| `address` | The record, 0x14 bytes. |
| `key` | The GlobalStrings key, looked up as Lua `_G[key]`. |
| `kind` | `0` chat composer `0x49a870`; `1` UI info message; `2` UI error message; `3` console sink `0x63cd00`. |
| `sound` | The sound cue name, `NONE` for silence. |
| `type_tag` | `0x44` plays the cue; any other value is an error-speech id. |
| `chat_type` | The chat type handed to the composer; read for `kind` 0 only. |

## events.tsv

The C-to-Lua event names, indexed by the id that `FrameScript_SignalEvent` is given (419 rows).
The name array at `0xbe1198` has 549 slots; the other slots are null. Two ids may share a name.

| column | meaning |
|---|---|
| `id` | The slot index. |
| `name` | The event name. |
| `name_address` | The name string. |
| `store` | The instruction that fills the slot at startup. |

## glue_events.tsv

The 26 events of the glue screens (login, realm and character screens). While they run, the
dispatch table is rebuilt from the array at `0xb41e70`, so the same id names a different event
than in `events.tsv`. Columns as in `events.tsv`.

## event_sites.tsv

Every call or tail jump to `FrameScript_SignalEvent 0x703e50` or `FrameScript_SignalEvent2 0x703f50` (485
sites, 149 of them `SignalEvent2`).

| column | meaning |
|---|---|
| `site` | The `call` or `jmp`. |
| `dispatcher` | `SignalEvent` (id in `ecx`, no arguments) or `SignalEvent2` (id and a printf-style format on the stack, then the arguments). |
| `event_ids` | Every id that reaches the site along any path, ascending, `|`-separated; `?` where the id is a parameter or a table lookup. |
| `event_names` | The names, in the same order; empty where an id has no name. |
| `arg_format` | The format string `SignalEvent2` is given: a declaration, not a count of what the caller pushed. Empty for `SignalEvent`. |
| `space` | `glue` for a site in glue's object files (`0x46a260` to `0x475b50`), named from `glue_events.tsv`; otherwise `game`, named from `events.tsv`. |

An event with no row here may still be fired: the unit-field events reach the dispatcher through
a wrapper, `0x515e50`, which is the `?` row at `0x515e7a`.

## chat_types.tsv

The 94 chat types, in the order the client indexes them (94 rows). A line of chat carries a type
byte; the type picks the colour and the event.

| column | meaning |
|---|---|
| `index` | The type byte, `0` to `93`. |
| `name` | The type's name, without the `CHAT_MSG_` prefix. |
| `rgb` | The default colour, six lowercase hex digits. |
| `event_id` | The event id fired for a line of this type. |
| `event` | The event's name in `events.tsv`; empty where the id has no name. |

## chat_groups.tsv

The 68 message groups a chat window can show (68 rows). A window keeps one flag byte per group.

| column | meaning |
|---|---|
| `index` | The group's position, `0` to `67`, which is the flag byte's position. |
| `name` | The group's name. |
| `default_on` | `1` when the group is on in the second chat window of a new character. |
| `added_in` | The settings-file version that added the group: `0`, `1` or `2`. |

## channel_notices.tsv

The 32 notices of the channel notify message, opcode `0x99` (32 rows). The message is a u8 notice,
the channel name and the notice's own fields.

| column | meaning |
|---|---|
| `notice` | The notice byte, `0` to `31`; a larger byte shows an empty system line. |
| `arm` | The code that handles it. |
| `token` | The text that becomes the event's first argument; `a|b` where the arm picks one by the channel's state. Empty for none. |
| `chat_type` | The chat type of the line: `CHANNEL_JOIN`, `CHANNEL_LEAVE`, `CHANNEL_NOTICE` or `CHANNEL_NOTICE_USER`. Empty where the arm shows no line. |
| `reads` | The fields the arm reads after the channel name, in order: `guid`, `u8`, `u32`, `cstring`. |

## cvars.tsv

Every `CVar::Register 0x63db90` registration (381 rows). A call site that registers several
times, in a loop or from a table, has one row per registration, each with the site. The generator
runs such a function's integer code with the tables read from the image, and keeps the static
row where the run cannot go on.

| column | meaning |
|---|---|
| `site` | The `call`. |
| `name`, `name_address` | The variable's name and the string's address. |
| `default`, `default_address` | The default value, a string, and its address. A default chosen by a condition lists both arms, `A|B`. |
| `flags` | The flags argument. |
| `callback` | The change callback; empty for none. |
| `category` | The category argument. |
| `script_hidden` | The fifth stack argument: 0 for a variable Lua can read, 1 for one it cannot. `CVar::Register` sets flag bit 31 when it is 0, and the script lookup `0x63dec0` finds only records with that bit. |
| `callback_arg` | The argument passed to the callback. |
| `record` | The global that receives the returned record; empty if the result is not stored, `|`-joined if two stores follow. |
| `help` | The help text; empty for no help pointer and for a pointer to the empty string (the `CombatLogRange` rows point at 0x882748, `""`). |
| `notes` | `<field>-computed` (not a constant at the site), `<field>-not-a-string`, `default-conditional`, `record-ambiguous`, `looped` (one of several registrations at the site), `name-built` (the name is made in a buffer, so `name_address` is empty). |

## dbc_stores.tsv

The client's database stores, one per DBC file (149 rows). A store is 0x14 bytes: records `+0x00`,
count `+0x04`, id map `+0x08`, largest id `+0x0c` (the largest first-column value, -1 until loaded),
loaded flag `+0x10`. The id map is indexed by the first column of each record.

| column | meaning |
|---|---|
| `store` | The store's address. |
| `loader` | The function the load sequence calls with the store as `this`; `StaticDBLoadAll` for the five stores it loads inline. |
| `load_site` | The `call` in that sequence, or for an inline load the store to its loaded flag. |
| `file` | The `DBFilesClient\...` path the loader reads. |

## dbc_schemas.tsv

The record layout of each DBC type, read from the type's `Read` function (151 rows).

| column | meaning |
|---|---|
| `file` | The DBC file name without `.dbc`; the record type is `<file>Rec`. |
| `field_count` | The number of four-byte fields (a byte-packed column counts one). |
| `record_size` | The record size in bytes. |
| `string_cols` | The field indices that are string-block offsets, comma-separated. A run of eight is a localized string, one per locale. |
| `fixups` | The number of string offsets `Read` rebases on the string block; equals the length of `string_cols`. |

## bindings.tsv

The shape of every Lua binding (1722 rows). `../tools/tables/bindings.py` opens with the full
contract; in brief:

| column | meaning |
|---|---|
| `name` | The Lua name. Not a key: some names are registered more than once. |
| `fn` | The C function. |
| `pair` | The `(name, function)` pair in the registrar's table. |
| `table` | The registrar table. |
| `table_kind` | `global`, `widget` (method tables), `baselib` or `stdlib`. |
| `argc` | The largest constant Lua stack index read: a floor, never a gate. |
| `argc_conf` | `exact` or `lower-bound`. |
| `arity` | The number of values returned: one number, a `|`-separated set, or `?`. |
| `arity_conf` | `exact`, `set` or `unknown`. Sound. |
| `kinds` | The kinds of the returned values, one alternative per path: `(number,string?)`. `string?` is a string or nil. |
| `kinds_conf` | `agree` (independently cross-checked at every return), `unverified` or `unknown`. |
| `notes` | `stack-leak`, `variadic`, `name-not-unique`, `delegated-push`, `iteration-cap`, `cfg-truncated`, `indirect-branch`; `;`-separated. |

## unit_gates.tsv

How each of the 83 unit functions checks its arguments (83 rows). `../tools/tables/unit_gates.py`
opens with the contract. A function without a check takes a missing or nil argument without an
error.

| column | meaning |
|---|---|
| `name` | The Lua name, in table order. |
| `fn` | The C function. |
| `checked` | The checked arguments, `,`-separated: `1:string` for `lua_isstring` (a number passes) and `1:number` for `lua_isnumber`, by Lua stack index. Empty for none. The table does not tell a probe from a check: `GetPVPRankInfo`'s `2:string` is a probe (`0x51a97c`, `0x51a998`) whose result is tested, and it never raises. |
| `usage` | Every `Usage:` string the function pushes for its error, `\|`-separated. Empty for none. |

## tutorial_triggers.tsv

Every place the client fires or acknowledges a tutorial (58 rows). `../tools/tables/tutorial_triggers.py`
opens with the contract. A tutorial has a number from 0 to 49; the interface sees it plus one.

| column | meaning |
|---|---|
| `site` | The call or tail jump. |
| `fn` | The function that holds it. |
| `verb` | `fire` for `0x4b5390` (set the fire-once bit and signal `TUTORIAL_TRIGGER`), `ack` for `0x4b54c0` (set both bits and tell the server). |
| `tutorials` | The numbers that reach the site, `\|`-separated; `?` where the walk cannot name one (a table read or a parameter). |
| `delay_ms` | For `fire`, the delay that reaches the site: 0 fires now; empty for `ack`. |

## calls.tsv

The direct calls and tail jumps out of each function (23,354 rows). `../tools/tables/calls.py`
opens with the contract. A function with no callee has no row; the callers of a function are the
rows whose `callees` hold it. A call through a register or a memory slot, and a function reached
only by a stored address, are not in it. The address site generates its caller and callee lists
from it.

| column | meaning |
|---|---|
| `function` | The function row. Sorted. |
| `callees` | The other function rows its extent calls, jumps or branches to by a bare address, once each, in address order, `\|`-separated. |

## event_args.tsv

The arguments each event hands to Lua (394 rows, one per event name in `events.tsv`).
`../tools/tables/event_args.py` opens with the contract. A shape is a printf-style format: `%s`
a string, `%d` and `%f` numbers; `()` is no values.

| column | meaning |
|---|---|
| `name` | The event name. Sorted. |
| `ids` | Every id of the name that a producer reaches or `events.tsv` holds, ascending, `\|`-separated. |
| `producers` | `signal` (a `SignalEvent` site), `signal2` (a `SignalEvent2` site, its format is the shape), `unit-field-bridge` (the watch on each named id below `0xb6`, shape `%s`) and `token-fanout` (a call to `0x515e50` with a literal id, shape `%s`); sorted, `\|`-separated. |
| `arg_formats` | Each distinct shape of the producers, sorted, `\|`-separated; `?` where no producer reaches the name. An event with two shapes carries both; which one fires depends on the transition. |
| `conf` | `exact`: every producer's shape is known. `advisory`: a site's format declares more values than its caller pushes, which hands Lua whatever lies above the last push. `none`: no producer reaches the name here, which is not a claim it has none. |
| `note` | For `advisory`, each site and the counts it declares and passes; joined with `; `. |

A site that `event_sites.tsv` gives several ids contributes its shape to each named id. A site in
the glue space is left out: its ids name other events.

## verb_events.tsv

The events a Lua verb fires (50 rows). `../tools/tables/verb_events.py` opens with the contract: a
verb is a `bindings.tsv` row outside the glue object files and not a getter, and a site belongs
to the function row that holds it. A floor, not a census.

| column | meaning |
|---|---|
| `verb` | The Lua name. |
| `event` | The event name. Rows are sorted by `verb`, then `event`. |
| `shape` | `body`: a signal site in the verb's own function. `helper`: a signal site in a function that only verbs call (at most 8 of them). `traced`: reached through a shared dispatcher, read by hand. |
| `via` | The helper or dispatcher; `-` for `body`. |
| `sites` | The signal sites, ascending, `\|`-separated. |

## glue_surface.tsv

Every Lua function in the glue screens' state before GlueXML loads (195 rows: 76 base, 14 shared,
105 glue). `../tools/tables/glue_surface.py` opens with the contract: the registrars, the library
openers and the compat chunk are read from the listing and the image.

| column | meaning |
|---|---|
| `name` | The global. Sorted; the key. |
| `scope` | `base` (`0x7039e0`, which both states run), `shared` (a registrar the in-game state also calls) or `glue`. |
| `registrar` | The function that installs the row. |
| `fn`, `table_va`, `arity` | As in `bindings.tsv`. A compat alias carries the library binding it names; a Lua closure has `fn` `lua`, an empty `table_va` and arity `?`; `__pow` has no table. |
| `world_fn` | A glue row's in-game namesake, a different binding, `\|`-joined; empty for none. |
| `note` | `shape-differs: ...` where both shapes are exact and the argument count or the return arity differs from the namesake's. |

## Captured tables

A captured table is not derived from the exe. It records a run of the real client, so it cannot be
rebuilt from a copy of the exe. It is committed, `cargo xtask check` checks its header, and
`cargo xtask check --exe` does not regenerate it. Its section here states the capture, what is
dropped, and the generator that turns the capture into the table. The capture and the files it is
attributed against are never committed; the generator takes them by path from `BENILLA_RE_*`
variables or options.

## lua_globals.tsv

The global names the 1.12 interface defines (21,555 rows): the engine's functions, which
`bindings.tsv` also covers, and the names the shipped UI (FrameXML and the twelve `Blizzard_*`
addons) defines. It is a capture of the running client's `_G` in the world, unioned with every
global the interface files define. The generator is `../tools/tables/lua_globals.py`; its header
carries the attribution rules.

The capture: sha256 `ce7f68773860bfaff6e9b8300c250ccc8f69ae107df4d459469b466179046d75`, a
saved-variables file written on 2026-06-20 by an addon that walks `_G` once the player is in the
world. Its `meta` records client build 1.12.1.5875 and locale `enUS`; it holds 19,572 globals.
The capture does not record which load-on-demand windows were open; a name only the interface
files define is a `lod` row. A later write of the same
file (sha256 `4ba3f3c33ebf37c95e14e56daf6ba5cc4d9eccec939047f1e1be524e1341c14d`) holds the
same 19,572 names and yields the same table.

Three of its globals are the addon's own, so the instrument and not the client, and are dropped
with `--drop`: its saved variable (the one top-level table of the capture file), its holder
table (a global the addon keeps its state in), and its slash-command token
(`SLASH_<token>1`). The generator stops if a dropped name is not in the capture. 19,569 names
remain.

Regenerate:

```
BENILLA_RE_GLOBALS_CAPTURE=<capture file> \
BENILLA_RE_INTERFACE_DIRS=<framexml folder>:<addons folder> \
python3 tools/tables/lua_globals.py --drop <saved variable> --drop <holder table> \
    --drop <slash token> -o tables/lua_globals.tsv
```

The folders hold the extracted `enUS` interface: the FrameXML folder with its `FrameXML.toc`, and
a folder of the twelve `Blizzard_*` addons, each with its `.toc`. Reading follows the `.toc` and
the XML references from each addon's root, as the client loads them (219 files); a file on disk
that nothing references is not read and is listed on standard error, so an extra file in an
extraction cannot add a row (`Bindings.xml` is one such file in FrameXML: its names are key-binding
commands, not globals). A referenced file that is missing stops the run. Every name the
interface files define that the capture lacks is a row with type `lod` (1,986 rows).

| column | meaning |
|---|---|
| `name` | The global. Sorted. |
| `type` | The Lua type in the capture (`function`, `table`, `string`, ...), or `lod` for a name only the interface files define. |
| `origin` | `lua` (Lua 5.0's own runtime), `engine` (the client provides it), `framexml` (the interface defines it, by a Lua or XML definition or as a `$parent` child of one). |

## Vendored copies

A consumer that keeps copies of these tables pins a commit of this repo and keeps each copy byte
for byte equal to the table at that commit. `cargo xtask vendored <checkout>` checks a checkout of
such a consumer: its `reference/PIN` holds the commit as 40 hex digits (a branch or tag name moves
and is refused), and each file under its `reference/` whose name is a table here is compared with
`git show <commit>:tables/<name>`. It lists each file as `same`, `differs` or `missing` (the pinned
commit has no table of that name), and says whether the pin is behind `origin/main` (else `main`).
With `--strict` it exits nonzero when any file differs or is missing, and when no file matched a
table. A checkout with no `reference/PIN` is reported as having none, and the command exits
cleanly. `.gitattributes` marks `tables/*.tsv` and `symbols/*.tsv` as `-text`, so a checkout never
rewrites their line endings; a copy must not either.

## signatures.tsv

One masked byte pattern for each function row (37,967 rows). Written by `cargo xtask signatures`;
`cargo xtask match` finds the rows in another build with it (`../README.md`, Other builds).

| column | meaning |
|---|---|
| `address` | The function row. Sorted. |
| `unique` | `yes` when the pattern occurs once in the executable sections of 5875; `no` when the whole body occurs more than once. |
| `pattern` | Hex bytes separated by spaces, `??` for a wildcard: the rel32 of a branch that leaves the function (a rel8 too), every four-byte operand that is an address in the image and every jump-table entry. At least one fixed byte. The shortest prefix of the body that is unique, at least 16 bytes; the whole body when it is shorter or not unique. |
| `size` | The row's size in 5875, decimal. |
| `body` | The pattern format, the whole masked body through the end of the last `calls` instruction; empty when `pattern` already covers it. |
| `calls` | The direct `call`, `jmp` and conditional jump with a rel32 that go from this body to another function row: each as the instruction's offset in the body and the target's 5875 address, both hex without `0x`, as `offset:target`, space-separated. Empty when none. |
