# cvar: config

The registry's file: how it starts, the directory it keeps its files in, how `Config.wtf` is read and written,
and how it shuts down. The file holds one line `SET <name> "<value>"` for each setting that differs from its
default; reading it is running those lines as console commands.

Roots: `0x63d380`, `0x63d440`, `0x63d820`, `0x63d980`, `0x63d910`, `0x63daf0`, `0x63e1a0`

## Start-up

`Initialize(file)` runs once at start-up, before any setting is registered, with `Config.wtf`. It keeps the file name, makes the
directory `WTF` beside the executable and creates every missing parent, registers the four registry commands, and reads the file. The
start-up wiring then reads `RunOnce.wtf` the same way; the file is deleted at shutdown, and `realmlist.wtf` is read by the same routine
when the realm list address is first asked for, and again on each ask whose argument (ecx) is nonzero (`0x5ab689`).

Addresses: `0x63d380`, `0x63d440`, `0x63d395`, `0x63d39b`, `0x63d3bd`, `0x63d3cb`, `0x63d3e1`, `0x63d3f7`, `0x63d40d`, `0x63d423`, `0x63d42e`, `0x648890`, `0x42a960`, `0x42a760`, `0x402375`, `0x40237f`, `0x403be7`, `0x5ab6b5`, `0x5ab689`, `0xc4edd4`
Confidence: verified
Proof: one
Test: none

`0x63d395` stores the file name at `0xc4edd4`. `0x63d39b` gets the base path (`0x648890`) and `0x63d3bd` formats `%s%s\` with `WTF`; `0x63d3cb` creates the
directories. `0x63d440` looks for each `\` after the first character, copies the prefix, creates it with `0x42a960`, and when that fails accepts a directory that exists
(`0x42a760` attributes with bit 0x10) and otherwise stops. The commands are registered with `0x63f9e0` at `0x63d3e1`, `0x63d3f7`, `0x63d40d` and `0x63d423`, all in category 5.
`0x63d42e` reads the file. The caller `0x402375` passes `Config.wtf`, `0x40237f` reads `RunOnce.wtf` and `0x403be7` deletes it; `0x5ab6b5` reads `realmlist.wtf`.

## Reading the file

`LoadFile(name)` loads the whole file by name and, when that fails, `WTF\<name>`; with neither it does nothing. A UTF-8 byte order mark is
skipped. It splits the text at CR and LF; a line keeps its first 0x7f bytes and the rest is dropped. It runs each line that begins with `SET ` (case-blind) through the
console's executor with the history flag off. The `set` command keeps 0x40 bytes of the name and 0x100 of the value, and a quote inside a value ends it. Because the executor runs `set`, an unknown name becomes a script-hidden record (category 5) holding the
file's value as its value and reset text; a later registration keeps that value (`registry#registering-a-setting`).

Addresses: `0x63d820`, `0x63d83d`, `0x63d858`, `0x63d877`, `0x63d895`, `0x63d8a9`, `0x63d8c7`, `0x64ae50`, `0x64af4e`, `0x63d8da`, `0x63d8eb`, `0x63d900`, `0x648620`, `0x648700`, `0x63ce00`, `0x63d51d`, `0x63d539`, `0x80e1d4`, `0x64aebf`, `0x64af1b`
Confidence: verified
Proof: two
Test: none

`0x63d83d` calls the file loader `0x648620`; `0x63d858` formats `WTF\%s` for the second try (`0x63d877`). `0x63d895` to `0x63d8a9` compare the first three bytes with
`ef bb bf` and step past them. `0x63d8c7` tokenizes with the delimiters CR LF into a 0x80-byte buffer, and the tokenizer `0x64ae50` stops storing at `0x64af4e` and skips to the delimiter; `0x63d8da` compares four bytes with `SET `
using the case-blind compare; `0x63d8eb` calls the executor `0x63ce00` with a zero history flag. `0x63d900` frees the file (`0x648700`).
The `set` handler tokenizes the name at `0x63d51d` and the value at `0x63d539` with the delimiters at `0x80e1d4` (space, comma, semicolon, tab, quote, CR, LF); the tokenizer opens a quoted span at `0x64aebf` and closes it at the next quote (`0x64af1b`).

## Writing the file

`SaveConfig` writes the file only while the dirty byte is set, and clears it first. It opens `WTF\<name>` for writing and replaces the file (`CreateFile` with `CREATE_ALWAYS`, share mode 0; `0x63d9dd`, `0x63d9df`), so any line of the old file that is not a `SET` line is lost. For each record
in list order whose bit 0 is set, it picks the latched text, else the value, else the default; it skips a record with no text or whose text
equals its default case-blind, and writes `SET <name> "<text>"` and a line feed for the rest. It returns 1 on success, and 0 when the file will not
open or a write stores nothing.

Addresses: `0x63d980`, `0x63d98e`, `0x63d9ad`, `0x63d9b6`, `0x63d9ce`, `0x63d9dd`, `0x63d9df`, `0x63d9eb`, `0x63da10`, `0x63da21`, `0x63da27`, `0x63da2e`, `0x63da35`, `0x63da4a`, `0x63da63`, `0x64a865`, `0x63d582`, `0x63da69`, `0x63da92`, `0x63da9a`, `0x63dac3`, `0x63dad3`, `0x42a460`, `0x42a5d0`, `0x42a4d0`, `0x46b6f6`, `0x490c50`, `0xc4edd8`
Confidence: verified
Proof: two
Test: none

`0x63d98e` returns 1 at once when `0xc4edd8` is clear; `0x63d9ad` clears it. `0x63d9b6` and `0x63d9ce` build `WTF\` plus the name; `0x63d9eb` opens it through `0x42a460` with write access.
The loop `0x63da10` follows the list. `0x63da21` tests bit 0; `0x63da27`, `0x63da2e` and `0x63da35` choose the latched text, the value, then the default; `0x63da4a` compares with the
default. `0x63da69` formats `SET %s "%s"` plus a line feed and `0x63da92` writes it with `0x42a5d0`; a write of 0 bytes (`0x63da9a`) closes the file (`0x63dac3`)
and returns 0, and the end of the list returns 1 (`0x63dad3`). The callers are the interface shutdown `0x490c50`, the world entry `0x46b6f6` and the registry's shutdown.
A name read from `Config.wtf` gets its record when its line is read (`0x63d582`), before the game registers its settings. The saved file keeps the order of the old file and appends the settings that were not in it. The line is formatted into 0x104 bytes (`0x63da63`), so a saved line longer than 0x7f bytes is cut when read back, and the formatter keeps 259 characters (`0x64a865`): a longer line loses its closing quote and line feed, runs into the next record's line, and that record is lost on the next read.

## Shutdown

The client's shutdown deletes `RunOnce.wtf`, runs the registry's shutdown and clears the command registry. The registry's shutdown saves
the file, unregisters its four commands, empties the all-records list and frees every record through the table's slot 0. A `DeleteConfigFile` that finds the name deletes it,
and otherwise tries `WTF\<name>`; it returns 1 when it deleted a file.

Addresses: `0x63daf0`, `0x63d910`, `0x63dafa`, `0x63db04`, `0x63db0e`, `0x63db18`, `0x63db1d`, `0x63db3c`, `0x63db6f`, `0x63d91c`, `0x63d927`, `0x63d948`, `0x63d95f`, `0x42a4e0`, `0x42a910`, `0x403bec`, `0x403bf1`, `0x640160`
Confidence: verified
Proof: one
Test: none

`0x63daf0` calls `SaveConfig`, then unregisters `set`, `cvar_reset`, `cvar_default` and `cvarlist` with `0x63fb40` at `0x63dafa` to `0x63db18`,
zeroes the chain counter (`0x63db1d`), unlinks each record from the all-records list (`0x63db3c`) and frees each remaining record from the buckets
through the table's slot 0 (`0x63db6f`). The client calls it at `0x403bec` and then clears the command table with `0x640160` (`0x403bf1`). `0x63d910` tests the name with `0x42a4e0`, deletes
with `0x42a910` (`0x63d927`), and otherwise formats `WTF\%s` (`0x63d948`) and repeats (`0x63d95f`).

## Dump to file

`DumpToFile(file)` writes `Name: <name> Info: <help> Current Value: <value>` and CR LF for each record, then `CVar Dump written successfully`,
or an error line in class 3 when the file cannot be made or written. Nothing calls it and no table holds its address.

Addresses: `0x63e1a0`, `0x63e1bd`, `0x63e24f`
Confidence: verified
Proof: one
Test: none

`0x63e1bd` opens the file (`0x42a460`) and `0x63e24f` writes each line with `0x42a5d0`. The census over every byte of `.text` and every dword of the image finds no call, jump or stored address for `0x63e1a0` (`unreached`);
the same scan finds the two callers of `0x63cdf0` as its control.
