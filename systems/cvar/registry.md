# cvar: registry

The CVar registry: one record per named setting, found by name, with a text value, its integer and
float forms, a default, a reset value, a latched value, flags, a category, a help line and a change
callback. This doc holds the record, how a setting is registered, found and set, and the commands that
come with every setting. The config file is `config`, the table the records live in is `containers`,
and the video options are `options`.

Roots: `0x63db90`, `0x63de30`, `0x63dec0`, `0x63df50`, `0x63e0b0`, `0x63dff0`, `0x63e010`, `0x63e030`, `0x63e060`, `0x63dde0`, `0x63d500`, `0x63d590`, `0x63d640`, `0x63d6f0`, `0x63d280`, `0x63d260`

## CVar record

A record is `0xc4` bytes, allocated under the tag `CVar`.

| offset | holds |
|---|---|
| `+0` | hash of the name |
| `+4`, `+0xc` | bucket link, all-records link |
| `+0x14` | the name, a duplicate of the registered text |
| `+0x18` | category id |
| `+0x1c` | flags |
| `+0x20` | value text |
| `+0x24`, `+0x28` | the value as a float and as an integer |
| `+0x2c` | change count |
| `+0x30`, `+0x34` | default text, reset text |
| `+0x38` | latched text |
| `+0x3c`, `+0xbc`, `+0xc0` | help line (0x80 bytes), callback, callback argument |

Addresses: `0x865388`, `0x63e2e0`, `0x63d260`, `0x63dd20`, `0x63dd69`, `0x63dd72`, `0x63e11e`, `0x63e124`, `0x63e127`, `0x63e130`, `0x63dc3d`, `0x63dc43`, `0x63dd4e`, `0x63dd57`
Confidence: verified
Proof: one
Test: none

`0x63e2e0` allocates `0xc4` bytes under `0x865388` and `0x63d260` zeroes the link words `+4` to `+0x14`. `0x63dd20` duplicates the name
into `+0x14`. The help line is copied with `SStrCopy` at `0x63dd69` or zero-filled (`0x63dd72`, 0x20 dwords) when the registration
has none. `0x63e11e` duplicates the value and `0x63e124` stores it, `0x63e127` stores the integer from `SStrToInt` and `0x63e130` the float from `SStrToFloat`. The
callback and its argument are stored at `0x63dc3d` and `0x63dc43` for an existing record and at `0x63dd4e` and `0x63dd57` for a new one.

## Flags and categories

Bit 0 of the flags is set on every record `Register` creates: the config file keeps only records with it, and so
every changed setting is saved. Bit 1 latches the value: a set is kept in `+0x38` until `Update` applies it. Bit 2
makes the record read-only. Bit 31 marks the record as visible to scripts, and `Register` sets it when its
script-hidden byte is 0. The category is an index into the table `0x8653a0` of nine 0x18-byte entries, an id
then a name: 0 debug, 1 graphics, 2 console, 3 combat, 4 game, 5 default, 6 net, 7 sound, 8 gm.

Addresses: `0x63ddaa`, `0x63ddb4`, `0x63dc78`, `0x63da21`, `0x63df3c`, `0x63df8e`, `0x63e0ba`, `0x63e0bf`, `0x63e030`, `0x63e060`, `0x48f579`, `0x491252`, `0x83de4c`, `0x8653a0`, `0x488b76`, `0x488bd1`, `0x488c47`, `0x488d21`, `0x63d56b`
Confidence: verified
Proof: one
Test: none

`0x63ddaa` ORs 1 into the flags argument, and `0x63ddb4` sets bit 31 when the byte argument is 0. `0x63da21` tests bit 0 only when saving. Bit 31 is
read by `0x63dec0` at `0x63df3c`; its four callers are the Lua functions `RegisterCVar`, `GetCVar`, `SetCVar` and
`GetCVarDefault` (`0x488b76`, `0x488bd1`, `0x488c47`, `0x488d21`). The only registration with the byte argument 1 is the `set` command
(`0x63d56b`). `0x63df8e` tests bit 1 and `0x63e0ba` to `0x63e0bf` test bit 2. `0x63e030` sets or clears bit 2; the game interface
calls it for the three names at `0x83de4c`, `realmList`, `realmName` and `scriptMemory` (`0x48f579` on entering, `0x491252` on leaving).

## Registering a setting

`Register(name, help, flags, default, callback, category, scriptHidden, callbackArg)` returns the record. A name that exists keeps its record: the
flags are ORed, the callback and argument are replaced, and the callback is called with the current value. Then the default text is offered with
`Set(apply 0)`, which fills only the empty reset and default slots, so a value read from the config file survives. A new name gets a duplicated name, the help text and
the callback, and `Set(default, apply 1)` runs before the flags exist, so a latch cannot defer it; the callback sees the default, and a false result leaves the value
empty. The record then gets its flags and a console command of its own name.

Addresses: `0x63db90`, `0x63dc58`, `0x63dc6c`, `0x63dc8b`, `0x63dca0`, `0x63dcb4`, `0x63dce1`, `0x63dcf0`, `0x63dd9f`, `0x63ddaf`, `0x63ddc9`, `0x63dde0`, `0x63f9e0`, `0x64b3f0`, `0x63e7b0`, `0x63e9f0`
Confidence: verified
Proof: one
Test: none

`0x63dc58` calls the callback as `fastcall(record, old value, new value, argument)`, the record in ecx and the current text in edx (`0x63dc51`) and again as the pushed new value, and ignores its result. `0x63dc6c` calls `0x63df50` with apply 0 and
the two slot flags set from whether `+0x34` and `+0x30` are empty. The new path hashes at `0x63dc8b`, builds the buckets at `0x63dca0` the first time, grows at `0x63dcb4`, allocates with the table's slot 1 at
`0x63dce1` and links the record at the end of the all-records list (`0x63dcf0`, mode 2). `0x63dd9f` calls `Set` with apply 1, and with
the reset flag for a nonzero byte argument and the default flag otherwise; `0x63ddaf` stores the flags after it. `0x63ddc9` registers the command `0x63dde0` under the
name, with the registration's category and help. A script-hidden byte of 0 fills the default slot, 1 the reset slot. An existing record also gets bit 31 for a byte of 0 (`0x63dc78`).

## Setting a value

`Set(record, text, apply, setReset, setDefault, force)` always returns 1. With apply 0 it is `InternalSet`. With apply 1 it first calls the callback as
`fastcall(record, old value, text, argument)`; a false result drops the set. Then it counts the change. A latched record keeps the text in `+0x38` and marks the config dirty; any other
runs `InternalSet`. That does nothing to a read-only record or a null text. With apply it stores the text, with its integer and float, only when `SStrCmpI`
finds it different from the value, so a change of case alone is ignored. It fills an empty reset or default slot when asked, and marks the config dirty when
something changed and force is set.

Addresses: `0x63df50`, `0x63e0b0`, `0x63df6b`, `0x63df7d`, `0x63df83`, `0x63df90`, `0x63dfb9`, `0x63dfc4`, `0x63dfe1`, `0x63e0ec`, `0x63e158`, `0x63e179`, `0x63e190`, `0x42a320`, `0xc4edd8`, `0x64ac60`, `0x64aaa0`, `0x63dff0`, `0x63e010`, `0x63e060`, `0x63e083`, `0x63e098`
Confidence: verified
Proof: one
Test: none

`0x63df56` tests apply, `0x63df67` the callback, `0x63df6b` checks the pointer with `0x42a320` (`CheckFunctionPointer`) and `0x63df7d` calls it with the old text in edx (`0x63df76`), the new text and the argument pushed. `0x63df83` drops the set on a zero result. `0x63df90` bumps `+0x2c`;
a latched record duplicates the text at `0x63dfb9` and stores the dirty byte `0xc4edd8` at `0x63dfc4`. `0x63e0ec` is the case-blind compare. The reset slot
is filled at `0x63e158` when `+0x34` is empty, the default slot at `0x63e179`, and the dirty byte at `0x63e190`. `Reset` (`0x63dff0`) and `Default` (`0x63e010`)
call `InternalSet` straight with the reset or default text, falling back to the other, so they run no callback. `Update` (`0x63e060`) applies a
latched text through `InternalSet` (`0x63e083`) and frees it (`0x63e098`). Every caller outside `Register` and the per-record command `0x63dde0` passes `(1, 0, 0, 1)` for the last four arguments; `0x63de23` pushes apply 1, reset 1, default 0, force 1.

## Lookup

`Lookup(name)` returns the record or 0. `0x63dec0` is the same lookup that returns a record only when bit 31 is set; the Lua
functions use it, so a script sees only settings registered script-visible. A name made by the console `set` command has the byte
argument 1 and is invisible to scripts.

Addresses: `0x63de30`, `0x63dec0`, `0x63de79`, `0x63de87`, `0x63df09`, `0x63df17`, `0x63df3c`
Confidence: verified
Proof: one
Test: none

Both hash with `SStrHashHT`, index the bucket with `hash & mask`, and compare the stored hash (`0x63de79`, `0x63df09`) then the name case-blind (`0x63de87`, `0x63df17`). A mask of −1 returns 0.
`0x63df3c` loads the flags and `jns` returns 0 when bit 31 is clear.

## Commands of the registry

Every record has a command of its own name (handler `0x63dde0`). With no argument it writes `CVar "<name>" is "<value>"`; with one it
runs `Set(argument, 1, 1, 0, 1)`, so a typed value also fills an empty reset slot. Four commands in category 5 come with the registry: `set <name> <value>`
sets an existing record or creates a script-hidden one; `cvar_reset` and `cvar_default` restore one record, or all in list order after
writing `Resetting all cvars` or `Restoring all cvars`, and write `No such cvar "<name>"` in class 3 for an unknown name; `cvarlist` writes
every record as `  "<name>" is "<value>"` with ` (default "<d>")` or ` (reset "<r>")` appended when they differ.

Addresses: `0x63dde0`, `0x63d500`, `0x63d590`, `0x63d640`, `0x63d6f0`, `0x63d541`, `0x63d55b`, `0x63d582`, `0x63d5cd`, `0x63d5e0`, `0x63d690`, `0x63de23`, `0x63de08`, `0x64ae50`, `0x64a480`, `0x63d7e8`
Confidence: verified
Proof: one
Test: none

The handlers tokenize with `0x64ae50` using the separators space, comma, semicolon, tab, quote, CR and LF: `set` uses a 0x40-byte buffer for the name and a 0x100-byte one for the value. `0x63d541` looks the name up;
`0x63d55b` sets it, and `0x63d582` registers a new record with category 5, no callback, byte argument 1 and the value as the default.
`0x63d5cd` writes the unknown-name error with `0x63cd00`. `0x63d5e0` resets, `0x63d690` restores the default. `0x63de08` writes the value through `0x63cd50`
and `0x63de23` sets with reset 1. `cvarlist` compares with `0x64a480` and writes each line with `0x63cb50` (`0x63d7e8`).

## Record teardown

A record's destructor `0x63d280` removes the command of the record's name (`0x63fb40`), frees the value, default, reset and latched
strings and then the name, and unlinks the bucket and list links.

Addresses: `0x63d280`, `0x63fb40`, `0x63d287`, `0x63d29d`, `0x63d2b2`, `0x63d2c8`, `0x63d2de`, `0x63d2f8`
Confidence: verified
Proof: one
Test: none

`0x63d287` calls `0x63fb40` with the name. The four string frees (`0x63d29d`, `0x63d2b2`, `0x63d2c8`, `0x63d2de`) pass the source lines 0x51 to 0x54 of
`ConsoleVar.cpp`, and the name's free (`0x63d2f8`) passes line 0x8b3 of the Storm template header.
