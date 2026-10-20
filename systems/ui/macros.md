# ui: macros

The macro system of `UIMacros.cpp`: the macro record and its order, the Lua functions that make, read and
change macros, how a macro runs, the spell a macro is bound to for the action bar, the name-to-spell
resolver it shares with `CastSpellByName`, the macro files, and the icon chooser's list.

Macros are client-side data: no message goes to the server. A macro runs only through `UseAction` on a macro
slot (`ui/actionuse#using-an-action`); the action bar reads a macro slot as `ui/actionbar` describes. The spell
book that names resolve in is `ui/spellbook`.

Roots: `0x4f15b0` (CreateMacro), `0x4f18b0` (EditMacro), `0x4f1850` (DeleteMacro), `0x4f1460` (run),
`0x4efe00` (bound spell), `0x4b4ab0` (CastSpellByName), `0x4f0090` (icon list), `0x4f0600` (load),
`0x4f0c60` (save)

## Macro records

A macro is a 0x56c-byte record. `+0` and `+0x18` hold its id, `+4` and `+0xc` the links of the hash table and
the list, `+0x1c` its scope (0 account, 1 character), `+0x20` its local flag, `+0x24` the name (0x40 bytes),
`+0x64` the icon basename (0x100), `+0x164` the body, `+0x564` the bound spell and `+0x568` the book selector.
A scope holds at most 18 macros. Ids count up from 1 in the account scope and from 0x1000001 in the
character scope. The body holds at most 256 characters; a longer text is cut. A record is found by id
through a hash table or by name through the list.

Addresses: `0x490bd0`, `0x4f1ca0`, `0x4f1010`, `0x4f1550`, `0x4f0f40`, `0x4f0ee0`, `0x4eff90`, `0x4effb0`, `0x807b8c`, `0xbdcc00`, `0xbdcd04`, `0xbdcbec`, `0xbdcbf8`, `0xbdcc44`, `0xbdcc54`, `0xbdcc5c`
Confidence: verified
Proof: one
Test: none

The table allocates a record through `0x4f1ca0` (0x56c bytes, name, icon, body, bound spell and selector cleared).
`0x4f1010(name, icon, body, local, scope)` builds one. A null or empty name returns 0. A scope already at 18
returns 0. The scope is whether the last argument is nonzero. Once 18 macros of that scope are not local, the
local argument is forced to 1. The id is the scope's counter in `0xbdcbec` plus 1. The counters start at 0 and
0x1000000 (`0x4eff90`). The worker stores the fields, cuts the body with `0x4f1550` (256 characters by UTF-8
length), derives the bound spell, adds 1 to the scope's count in `0xbdcc00` and, for a macro that is not local, to
its count in `0xbdcd04`. It sets the save flag `0xbdcbf8` and rebuilds the order. `0x4f0f40(id)` walks
a hash bucket of `0xbdcc54` under mask `0xbdcc5c`; `0x4f0ee0(name)` walks the list from `0xbdcc44` and compares
without case, so the first match in list order wins. `0x4effb0` frees every record and the icon list; interface shutdown calls
it after the save (`0x490bd0`).

## Macro order

Macros are addressed by position. `0xbdcc60` holds 36 ids, 18 per scope: positions 0 to 17 are account macros
and 18 to 35 character macros, each group sorted by name without case, unused entries 0. The Lua index of a
macro is its position plus 1, so the first character macro is index 19 however few account macros there are.
Any create, edit or delete rebuilds the table and signals `UPDATE_MACROS` (393).

Addresses: `0x4f0d30`, `0x4f0e10`, `0x4f0e40`, `0x4f0eb0`, `0x4f0ec0`, `0x4f0fa0`, `0x4f0fd0`, `0x4f0ff3`, `0x4f0ffa`, `0x4f17c1`, `0x84c988`, `0xbdcc60`
Confidence: verified
Proof: one
Test: none

`0x4f0d30` zeroes the 36 entries, collects each scope's records from the list, sorts them with the name compare
`0x4f0e10` (`SStrCmpI`), stores the ids and signals the event with no
arguments. `0x4f0e40(position)` returns the record or null for 36 and up. `0x4f0eb0(position)` returns the id
or 0. `0x4f0ec0(id)` returns the position or -1; the id 0 matches the first empty entry. `0x4f0fa0(name)` returns
the position of the first named record or -1. `0x4f0fd0(id, dest, size)` formats `"%s%s"` of the icon directory
string and the record's basename, and leaves an empty string only for an id with no record: a record with an
empty basename gives the directory alone (`0x4f0ff3` to `0x4f0ffa`), where `GetMacroInfo` answers nil for it.

## Macro functions

`GetNumMacros` returns the account count and the character count. `GetMacroInfo(index)` returns the name, the
icon path (the icon directory plus the basename; nil when there is no basename), the body and 1 or nil for the
local flag. A missing index returns four nils and a non-number raises a usage error. `GetMacroIndexByName(name)`
returns the position plus 1, or 0 for none. `PickupMacro(index)` puts the macro on the cursor
(`ui/cursor#payload-pictures-by-kind`); an unknown index does nothing. `DeleteMacro(index)` removes the macro, clears every
action slot that holds it and rebuilds the order. `DeleteMacro` does nothing when the kind 7 check fails (`ui/luastate#the-context-word`).

Addresses: `0x4f1710`, `0x4f1760`, `0x4f1b30`, `0x4f1ae0`, `0x4f1850`, `0x4f1340`, `0x4f1b90`, `0x4f1bb0`, `0x494f80`, `0x494a50`
Confidence: verified
Proof: one
Test: none

Every function converts the index to an integer (`0x40a2b0`) and subtracts 1. `0x4f1760` builds the results from the
record at `0x4f0e40`. `0x4f1850` and `0x4f1ae0` pass the position to `0x4f0eb0` first and do nothing on id 0.
`0x4f1340(id)` unlinks and frees the record, decrements the scope's counts, sets the save flag and clears each
slot that holds the id with `0x4e5d10` (a `CMSG_SET_ACTION_BUTTON` per slot, `ui/actionbar#replacing-slots`),
then rebuilds the order. `0x494f80` is the macro payload setter of the cursor. `0x4f1b90` registers the nine
functions when the interface starts and `0x4f1bb0` removes them at shutdown.

## Creating a macro

`CreateMacro(name, iconIndex, body, local, perCharacter)` needs a string name and a number icon index or it
raises a usage error. When the kind 7 check fails it returns nothing. A nil or empty name raises "no name
specified" and a full scope raises "already have 18 macros". The icon is entry `iconIndex - 1` of the icon list
(`Macro icons`), none when out of range. The body is the third argument when it is a string. `local` and
`perCharacter` are script flags (`ui/actionuse#reading-a-script-flag`). It returns the new index.

Addresses: `0x4f15b0`, `0x4f1010`, `0x4f0090`, `0x6f1c10`, `0x494a50`
Confidence: verified
Proof: one
Test: none

`0x4f15b0` tests argument 1 as a string and 2 as a number (`0x6f3510`, `0x6f34d0`), calls `0x494a50(7)`, then
`0x4f0090` if the icon list is empty and `0x4f1010` with the flags from `0x6f1c10`. A zero from the worker with
a name given is the full-scope error.

## Editing a macro

`EditMacro(index, name, iconIndex, body, local)` raises `Usage: EditMacro(index, name, icon, body, local)` unless argument 1 is a number or a numeric string (`0x6f34d0`), and does nothing without the kind 7 check. It maps the index to an
id and sets the fields that are given: a string name, a number icon index (an entry of the icon list), a string
body. It always replaces the local flag with the fifth argument, a script flag that is false when absent. The
bound spell is derived again and the macro's action slots are redrawn. It returns the macro's index; an index with no
macro also returns, with the position of the first empty entry plus 1.

Addresses: `0x4f18b0`, `0x4f1280`, `0x4f0eb0`, `0x4f0ec0`, `0x4e5cc0`, `0x4e5c00`
Confidence: verified
Proof: one
Test: none

`0x4f18b0` tests argument 1 with `0x6f34d0` (`0x4f18be`) and raises the usage text at 0x84cc44 (`0x4f18c7`), then maps the index with `0x4f0eb0` and calls `0x4f1280(id, name, icon, body, local)`, then returns the
position from `0x4f0ec0` plus 1. The worker copies each given field, stores the local flag, re-derives the bound
spell, sets the save flag, rebuilds the order, redraws each slot that holds the id with `0x4e5cc0` and refreshes
the usable flags with `0x4e5c00`.

## Running a macro

`0x4f1460(id)` finds the record by id and runs it; an unknown id runs nothing. The runner splits the body on
carriage returns and line feeds and signals `EXECUTE_CHAT_LINE` (392) once per nonempty line with the format
`%s` and the line as the argument. Quotes are not special. The interface's chat frame handles that event by
feeding the line to its edit box as if typed, so slash commands are the interface's and not the client's. Run by
name has no caller.

Addresses: `0x4f1460`, `0x4f14e0`, `0x4f14d0`, `0x64ae50`, `0x703f50`, `0x4e5ee0`
Confidence: verified
Proof: one
Test: none

The only caller of `0x4f1460` is the macro arm of `UseAction` (`0x4e5ee0`). `0x4f14e0` calls the tokenizer
`0x64ae50(&cursor, buf, 0x400, "\r\n", 0)` until the cursor ends and signals through `0x703f50`. The tokenizer
treats `"` as a quote only when the delimiter set holds it, and this set does not. `0x4f14d0` (by name) has no
caller.

## The bound spell

A macro has a bound spell, `+0x564`, which the action bar uses for its icon, range, cooldown and usable
state (`ui/actionbar#slots-with-no-spell`). It is a spell id, 0 for none or -1 for an alias that did not resolve.
`0x4efe00` derives it from the body, line by line, and stops at the first line that matches a cast line. No
line gives 0. The resolver writes `+0x568` on a match: 0 for the player's book, 1 for the pet's. The derivation never
clears it. A failed alias shows the slot as unusable. A macro with a failed `CastSpellByName` name or no
cast line shows as usable.

Addresses: `0x4efe00`, `0x4b3bc0`
Confidence: verified
Proof: one
Test: none

Each line is copied into a 256-byte buffer. The routine ends in `0x4eff14`, which stores 0, or in one of the two
arms below, which store the result.

## Cast lines

Arm A: the line starts with the value of a global `SLASH_CAST<n>`, for n from 1 while the global is a nonempty
string, compared without case and followed by a space. The rest of the line after that space, unchanged, is the
spell name. A name that resolves gives its spell id and one that does not gives -1. Arm B, tried on a line only
when no alias matches it: the line contains `CastSpellByName(` (case-sensitive) and a quote. The text from the
quote to the next quote, or to the line's end, is the name, and the resolver's answer is stored, 0 when it fails.

Addresses: `0x4efe00`, `0x703bf0`, `0x64a4c0`, `0x64b4f0`, `0x4b3bc0`
Confidence: verified
Proof: one
Test: none

`SLASH_CAST%d` is formatted per n and read as a script global with `0x703bf0`; a nil or empty value ends the
loop. The prefix compare uses `0x64a4c0` with the alias length and the next byte must be 0x20. A hit calls
`0x4b3bc0(name, &rec[0x568])` and replaces a 0 result with -1. Arm B searches the line with `0x64b4f0`, ends the
name in place at the closing quote and stores the resolver's result without the -1 rule. A line with
`CastSpellByName(` and no quote falls through to the next line.

## Name to spell

`0x4b3950(name, &book)` splits `Name(Rank N)`: the name before the last `(` and the text between that `(` and
the next `)`. It calls `0x4b3a10(name, rank, &book)`, which searches the player's book and then the pet's, each
from its last slot down, for the first spell whose name equals the name without case and, when rank text was
given, whose rank equals it. Both lists are in book order, so a bare name resolves to the highest rank in the
book. A match writes book 0 or 1 and returns the slot; no match returns -1 and leaves the selector. `0x4b3bc0`
turns the slot and book into a spell id, 0 on a miss or a slot of 0x400 and up.

Addresses: `0x4b3950`, `0x4b3985`, `0x4b398e`, `0x64a450`, `0x4b3a10`, `0x4b3bc0`
Confidence: verified
Proof: one
Test: none

`0x4b3950` finds the last `(` with `0x64a450` (`0x4b3985`), which keeps the latest match as it walks to the
end, and ends the name there (`0x4b398e`); the rank text is walked by UTF-8 character to the `)`.
`0x4b3a10` loads the entry at the count and decrements, so the walk is from the last entry; it compares the
record's `Name` (`+0x1e0` plus the locale) and `Rank` (`+0x204`) strings (`ui/spellbook#getspellname`). The only
writes of the selector are on a match. `0x4b3bc0` reads the pet list for a nonzero book and the player list otherwise.

## CastSpellByName

`CastSpellByName(name[, onSelf])` resolves the name as above and casts the slot as
`ui/spellbook#casting-a-book-spell` does, on the player when `onSelf` is true and on the current target
otherwise. An unknown name does nothing.

Addresses: `0x4b4ab0`, `0x4b3300`
Confidence: verified
Proof: one
Test: none

`0x4b4ab0` calls the resolver, then `0x4b3300` with the slot and the book it wrote.

## When the bound spell is derived

The bound spell is derived when a macro is created or edited, once after the macro files load, once after the
player's own spell list is installed at login, and for the macros whose spell is -1 or from the pet book after
each pet spell message. A superceded-spell message rewrites a cached spell in place
(`ui/actionbar#replacing-slots`). Nothing derives it per use, so a macro whose `CastSpellByName` name failed
keeps 0 until the next edit or load, and a failed alias is retried at the next pet spell message.

Addresses: `0x4f0ca0`, `0x4f0ce0`, `0x4f1010`, `0x4f1280`, `0x4f0600`, `0x5dea50`, `0x4bdae0`
Confidence: verified
Proof: one
Test: none

`0x4efe00` has four callers: the create worker, the edit worker, `0x4f0ca0` (every record) and `0x4f0ce0`. The load
ends in `0x4f0ca0`, as does the player's spell list install (`0x5dea50`), and the pet spell message calls
`0x4f0ce0`, which derives only for records whose `+0x564` is below 0 or whose `+0x568` is nonzero.

## Macro files

The account macros are written under `WTF\Account\<account>\` and the character macros in a folder two
levels below it, each scope in two files: `macros-local.txt` for macros whose local flag is set and
`macros-cache.txt` for the rest. A file holds one block per macro: `MACRO <id> "<name>" <icon>`, one line per
body line, and `END`, each ended by a carriage return and line feed. The save runs when the save flag
`0xbdcbf8` is set.

Addresses: `0x4f0aa0`, `0x4f0b61`, `0x4f0c60`, `0x5afed0`, `0x5af6b0`, `0x64a7f0`, `0x84cb60`, `0xbdcbf8`
Confidence: verified
Proof: one
Test: none

`0x4f0c60` clears the save flag and calls `0x5afed0` for file kinds 4 (account) and 5 (character) with the
callback `0x4f0aa0`; the callback gets the local file's handle in `ecx` and the cache file's in `edx`, and a macro
goes to the first when `+0x20` is set. The block is built with the format `MACRO %d "%s" %s` plus CRLF (`0x84cb60`, pushed
at `0x4f0b61` with the id `[rec+0x18]`, the name `+0x24` and the icon `+0x64`), each
body line from the tokenizer with CRLF, then `END` with CRLF.

## Loading macro files

The reader takes a header only as `MACRO` in any case, a nonzero id and a name. It skips a scope that already has 18
macros, adds the scope's base to an id below it, skips an id that exists, and ends a block at a line `END`
(any case). Each body line is followed by `\n`, the last too, so a loaded body ends in `\n`; the body is cut to 256 characters. A loaded macro always has the
local flag clear, so a macro read from `macros-local.txt` is written back to `macros-cache.txt`. The load ends by
rebuilding the order and deriving every bound spell.

Addresses: `0x4f0600`, `0x4f0a70`, `0x5afe50`, `0x4f0d30`, `0x4f0ca0`, `0x4f095c`, `0x4f1550`
Confidence: verified
Proof: one
Test: none

`0x4f0a70` registers the reader `0x4f0600` for the same two kinds through `0x5afe50`, which runs it at once or
when the account data is ready. The reader tokenizes lines on `\r\n`, the header on a space, then on space, `"`, CR
and LF (quote-aware), keeps the largest id seen as the scope's last id, and copies the name (0x40) and the icon
(0x100). Its only store to `+0x20` is 0 (`0x4f095c`). The header compare at `0x4f06d7` is a case-insensitive string compare; `0x4f0a15` appends each kept line and `0x4f0a2b` a `\n` (0x835144) to the body buffer.

## Macro icons

`GetNumMacroIcons` builds the list on first use and returns its size. `GetMacroIconInfo(index)` returns the
icon directory plus the stored basename for a 1-based index and the empty string, not nil, for an index outside
the list; it never builds. The list is the sorted, deduplicated (without case) union of the extension-stripped
names of: every `(listfile)` line of the patch archives and `interface.MPQ` that starts with `Interface\Icons\`
and whose name starts with `Ability_` or `Spell_` and ends `.blp` or `.tga`; the loose files of the base
path's `Interface\Icons\` with the same name rule (a `.bz` suffix removed first); and the loose `.blp` and
`.tga` files of a working-directory `Interface\Icons\`, any name. The spell icon table is not a source.

Addresses: `0x4f0090`, `0x4f0220`, `0x4f0350`, `0x4f04f0`, `0x4f05e0`, `0x4f19f0`, `0x4f1a30`, `0x401470`, `0x42ad10`, `0xbdcc1c`, `0xbdcc20`
Confidence: verified
Proof: one
Test: none

`0x4f0090` reads the sources in turn: `0x401470(6, prefix, 0x4f0220)` walks the archives; `0x4f0220` skips the
prefix, tests `Ability_` and `Spell_`, finds the last dot, accepts `.blp` and `.tga` and stores a copy cut at the
dot. The directories are enumerated with `0x42ad10` and the callbacks `0x4f0350` (name test, `.bz` unwrap) and
`0x4f04f0` (no name test). The list `0xbdcc20` of `0xbdcc1c` entries is sorted with `0x4f05e0` (`SStrCmpI`) and
adjacent equal entries are removed from the end. `0x4f1a30` subtracts 1 and compares as unsigned, so 0, negative
and oversize indices give the empty string. The count depends on the install.
