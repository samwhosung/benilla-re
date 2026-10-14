# The tables

Facts about `WoW.exe` 1.12.1.5875 that a program reads, each one a tab-separated file with a
header row, UTF-8, one row per fact, and a generator in `../tools/tables/` that rebuilds it from
your own copy of the exe. Nothing here is edited by hand: a wrong row is a wrong generator.

```
tools/tables/regen.sh [DIR]  # rebuilds every table, into tables/ or DIR
python3 tools/tables/<name>.py -o tables/<name>.tsv
```

The generators are Python 3 with the standard library only. They read `WoW/WoW.exe` (they refuse
any other build, by size and SHA-256) and, for the ones that read instructions, `dumps/disasm.txt`,
which `regen.sh` writes with `../tools/disasm.sh` when it is missing. Each run twice gives
byte-identical output. Options on every generator: `--exe PATH`, `--disasm PATH`, `-o PATH`.

`cargo xtask check` checks each table's header against the columns this file gives it, and
`cargo xtask check --exe` regenerates every table into a scratch directory and fails on any byte
that differs from the committed one.

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

Every network message the client registers a handler for (387 rows, 207 distinct handlers).

| column | meaning |
|---|---|
| `opcode` | The message id, `0x000` to `0x33b` (the handler tables hold 828 entries). Sorted. |
| `handler` | The function that runs when the message arrives. Several opcodes may share one. |
| `site` | The `call` that registers it. |
| `path` | `A`: through the wrapper `0x5ab650`; `B`: a direct call to `NetClient::SetMessageHandler 0x537a60`. |

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

Every `CVar::Register 0x63db90` call (214 rows).

| column | meaning |
|---|---|
| `site` | The `call`. |
| `name`, `name_address` | The variable's name and the string's address. |
| `default`, `default_address` | The default value, a string, and its address. A default chosen by a condition lists both arms, `A|B`. |
| `flags` | The flags argument. |
| `callback` | The change callback; empty for none. |
| `category` | The category argument. |
| `script_visible` | Whether Lua may read it. |
| `callback_arg` | The argument passed to the callback. |
| `record` | The global that receives the returned record; empty if the result is not stored, `|`-joined if two stores follow. |
| `help` | The help text; empty for none. |
| `notes` | `<field>-computed` (not a constant at the site), `<field>-not-a-string`, `default-conditional`, `record-ambiguous`. |

## dbc_stores.tsv

The client's database stores, one per DBC file (149 rows). A store is 0x14 bytes: records `+0x00`,
count `+0x04`, id map `+0x08`, max row `+0x0c` (row count minus one, -1 until loaded), loaded flag
`+0x10`. The id map is indexed by row, not by the id column.

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
