# The tables

One table and a second with the same columns, for the check's control.

## demo.tsv

| column | meaning |
|---|---|
| `id` | The index. |
| `name`, `name_address` | The name and its string. |

## copy.tsv

A copy of the demo rows. Columns as in `demo.tsv`.

## bindings.tsv

The shape of every Lua binding.

| column | meaning |
|---|---|
| `name` | The Lua name. |
| `fn` | The C function. |
| `pair` | The registrar pair. |
| `table` | The registrar table. |
| `table_kind` | `global`, `widget`, `baselib` or `stdlib`. |
| `argc` | The largest stack index read. |
| `argc_conf` | `exact` or `lower-bound`. |
| `arity` | The number of values returned. |
| `arity_conf` | `exact`, `set` or `unknown`. |
| `kinds` | The kinds of the returned values. |
| `kinds_conf` | `agree`, `unverified` or `unknown`. |
| `notes` | Flags, `;`-separated. |

## events.tsv

The event names.

| column | meaning |
|---|---|
| `id` | The slot index. |
| `name` | The event name. |
| `name_address` | The name string. |
| `store` | The instruction that fills the slot. |

## event_sites.tsv

Every call to the dispatchers.

| column | meaning |
|---|---|
| `site` | The call. |
| `dispatcher` | `SignalEvent` or `SignalEvent2`. |
| `event_ids` | The ids that reach the site. |
| `event_names` | The names, in the same order. |
| `arg_format` | The format string. |
| `space` | `glue` or `game`. |

## cvars.tsv

Every registration.

| column | meaning |
|---|---|
| `site` | The call. |
| `name` | The name. |
| `name_address` | The string. |
| `default` | The default value. |
| `default_address` | Its address. |
| `flags` | The flags. |
| `callback` | The change callback. |
| `category` | The category. |
| `script_hidden` | 0 for readable from Lua. |
| `callback_arg` | The callback's argument. |
| `record` | The global that gets the record. |
| `help` | The help text. |
| `notes` | Notes. |

## opcodes.tsv

Every registered message.

| column | meaning |
|---|---|
| `opcode` | The message id. |
| `handler` | The function. |
| `site` | The registering call. |
| `path` | `A` or `B`. |
