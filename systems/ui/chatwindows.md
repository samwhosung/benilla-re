# ui: chat windows and their settings file

The ten chat windows' settings: the record each keeps, the Lua functions that read and write it,
the message groups and channels a window shows, and the file that saves them. The start-up that
fills the records, the loader and the writer are `ChatFrame.cpp`, `0x498070` to `0x4a4xxx`; the
fifty Lua functions of the table at `0x843618` are registered by `0x4a1a10`. Chat colours are
`chatcolors`, the line and event path `chatevents`, channels `chatchannels`.

Roots: `0x4982c0`, `0x498a60`, `0x499a80`, `0x4a0ba0`, `0x4a0d20`, `0x4a0dc0`, `0x4a0e80`, `0x4a0f40`, `0x4a1000`, `0x4a1260`, `0x4a13f0`, `0x4a1470`, `0x4a14f0`, `0x4a15d0`, `0x4a1650`, `0x4a16b0`, `0x4a1730`, `0x4a1790`, `0x49fe40`, `0x49fef0`

## Window records

The windows are the ten 0x98-byte records at `0xb4fe50` (zero at load), indexed from 1 by Lua. Offsets:
`+0x00` name (32 bytes); `+0x20` one flag byte per message group (68); `+0x64` the channel-name array
header (count `+0x68`, slots `+0x6c`); `+0x74` the channel-id array header (count `+0x78`, slots
`+0x7c`); `+0x84` font size (int); `+0x88`, `+0x89`, `+0x8a`, `+0x8b` blue, green, red, alpha, one
packed dword `a<<24 | r<<16 | g<<8 | b`; `+0x8c` locked; `+0x90` docked; `+0x94` shown. A Lua index
outside 1 to 10 returns with no values and no error in every window function; only a non-number
argument raises the usage error.

Addresses: `0xb4fe50`, `0x4982c0`, `0x4984d7`, `0x4984de`, `0x4984e4`, `0x4984ee`, `0x4984f4`, `0x498502`, `0x4a2c10`, `0x4a0bdd`, `0x4a154e`, `0x4a1605`, `0x4a14a5`, `0x4a1689`, `0x4a16e9`, `0x4a1769`, `0x4a1429`
Confidence: verified
Proof: one
Test: none

Every site that forms a record pointer multiplies the index by 0x98 (`imul esi,esi,0x98`) after
`dec; cmp reg,0xa; jae`. The start-up `0x4982c0` runs for each of ten records (`0x4984c4` to `0x4985b6`): the
name byte, size, docked and shown are 0, locked is 1 (`0x4984e4`), and the colour dword comes from
the packer `0x4a2c10(0,0,0,0)` (`0x498502`), whose bytes store `(a<<24)|(r<<16)|(g<<8)|b`. The two
array constructors `0x4a2500` and `0x4a2030` are given 10, so each window starts with ten empty slots
(`0x498522` to `0x49853c` zero them); a null name slot is free everywhere.

## Window start-up state

Window 1 starts with groups 0 to 9 on and shown. Window 2 starts docked and shown with group i on
for i = 10 to 67 exactly when the group table's default-on flag is set (`chat_groups.tsv`); groups
0 to 9 stay off. Windows 3 to 10 start with every group off, hidden and undocked. The start-up fires
no event; the events come from the loader.

Addresses: `0x4982c0`, `0x498542`, `0x498547`, `0x498553`, `0x49855d`, `0x498574`, `0x498583`, `0x498598`, `0x4985a1`, `0xb4fe70`, `0x805fb0`
Confidence: verified
Proof: one
Test: none

Window 1: `mov edx,0x1010101` stored into `0xb4fe70`, `0xb4fe74` and the word `0xb4fe78` fills flags
0 to 9, a literal fill, and `0xb4fee4` (shown) becomes 1. Window 2 (`0x49856f`, `0x498574`) stores 1 into
shown and docked and loops `dl = [eax+0x805fb4]; setne` into `0xb4ff08 + i` for i from 10 with
`eax` from 0x78 in steps of 12 to 0x330. The `rep stos` of 17 dwords at `0x4985a1` clears the flags of
windows 3 to 10. The array is BSS, so every byte the start-up does not write is 0. The settings loader
zeroes all flags before it parses, so these defaults survive only when no file is read.

## Setting colour, alpha and size

`SetChatWindowColor(index, r, g, b)` stores `int(255.0 * x)` of each argument into red `+0x8a`, green `+0x89`
and blue `+0x88`; `SetChatWindowAlpha(index, a)` stores `int(255.0 * a)` into `+0x8b`; neither clamps,
and the store keeps the low byte of the truncated integer, so alpha 2.0 stores 254, -1.0 stores 1 and
0.5 stores 127. `SetChatWindowSize(index, n)` stores the truncated integer into `+0x84` unless it is 0
or negative, which is discarded silently. Nothing else is written and no event or dirty flag follows.

Addresses: `0x4a14f0`, `0x4a15d0`, `0x4a1470`, `0x4a1569`, `0x4a157b`, `0x4a1598`, `0x4a15ae`, `0x4a1617`, `0x4a1628`, `0x4a14b7`, `0x4a14bc`, `0x4a14c6`, `0x40a2b0`, `0x806498`
Confidence: verified
Proof: one
Test: none

Each argument is read with `lua_tonumber` (`0x6f3620`), multiplied by the double 255 (`0x806498`,
`fmul` at `0x4a1569` and `0x4a1617`) and truncated by `0x40a2b0` (`fnstcw`, `or ah,0x0c`, `fistp`);
`mov byte` takes `al`. `0x4a14bc` is `test eax,eax; jle`, so size 0 or less skips the store at
`0x4a14c6`. No compare against 0 or 1 exists in any of the three bodies. Nothing reads the bytes
except the getter and the settings writer, which treat them as already 0 to 255.

## Locked, docked, shown and name

`SetChatWindowShown(index, v)` stores `+0x94` from the flag reader `0x6f1c10` (`ui/actionuse#reading-a-script-flag`) with default 1, so a call
with no second argument shows the window; `SetChatWindowLocked(index, v)` stores `+0x8c` the same way with
default 0. `SetChatWindowDocked(index, n)` stores `+0x90` as the truncated number, 0 when the argument is
not a number. `SetChatWindowName(index, s)` copies at most 32 bytes into `+0x00`, the empty string when
the argument is not a string. All return no values.

Addresses: `0x4a1730`, `0x4a1650`, `0x4a16b0`, `0x4a13f0`, `0x4a176e`, `0x4a1782`, `0x4a16a2`, `0x4a171a`, `0x4a1462`, `0x6f1c10`, `0x64a5a0`
Confidence: verified
Proof: one
Test: none

`0x4a176e` pushes the default 1 before the call to `0x6f1c10`; `SetChatWindowLocked` pushes 0. Only the
name and docked setters bypass the flag reader. The name copy is `0x64a5a0(dst, src, 0x20)`
(`0x4a1462`), so a name of 32 or more bytes is cut to 31 characters and a terminator.

## Reading a window

`GetChatWindowInfo(index)` returns nine values: name, font size, red, green, blue and alpha (each colour
byte times the float 1/255), shown, locked, docked. Shown and locked and docked are the raw dwords,
not booleans. The font size is read as an unsigned 32-bit value, so a negative stored size comes back
as the size plus 2^32.

Addresses: `0x4a0ba0`, `0x4a0bf3`, `0x4a0c00`, `0x4a0c20`, `0x4a0c32`, `0x4a0c40`, `0x4a0c60`, `0x4a0c80`, `0x8026c8`
Confidence: verified
Proof: one
Test: none

`mov eax,9` sets the count. The size is `mov [ebp-8],eax; mov dword [ebp-4],0; fild qword`, a 64-bit load
with a zero high dword (`0x4a0c00`). Each colour is `movzx`, `fild`, `fmul dword [0x8026c8]`, the float
nearest 1/255 (`0x3b808081`). The getter and the three colour setters give the byte order of the
record: the setter writes red to `+0x8a`, the getter reads red from there.

## Message groups of a window

A window shows a chat line when its flag byte for the line's group is set. The groups are 68 records of
12 bytes at `0x805fb0` (name pointer, default-on, version that added it; `chat_groups.tsv`), which
differ from the 94 colour types: `CREATURE` is a group only. `GetChatWindowMessages(index)` returns the
name of every set flag in table order; `AddChatWindowMessages(index, name)` and `RemoveChatWindowMessages`
set or clear the flag of the group whose name matches without case, and do nothing for an unknown name.
They return no values and fire no event.

Addresses: `0x805fb0`, `0x4a0d20`, `0x4a0d91`, `0x4a0e80`, `0x4a0ee7`, `0x4a0f0b`, `0x4a0f40`, `0x4a0f97`, `0x4a0fcb`
Confidence: verified
Proof: one
Test: none

The getter walks 68 entries (`0x4a0d80` to `0x4a0dab`), checking the stack for one slot and pushing
`[entry+0]` for each non-zero flag. The setters compare with `0x64a4c0(arg, name, 0x7fffffff)` and write
`[window*0x98 + i + 0xb4fe70]` as 1 (`0x4a0f0b`) or 0 (`0x4a0fcb`). Eight sites read the table: the
settings writer, the loader (the name test, the default-on test and the version test), the start-up and
these three verbs.

## Channels of a window

A window keeps channel names beside ids. `GetChatWindowChannels(index)` returns, for each non-null slot,
the name and then the id as a number. `AddChatWindowChannel(index, name)` returns one number: the
ChatChannels row id when `name` is a row's shortcut, 0 for any other channel. For a shortcut it stores
the row's own shortcut string, not the argument, and returns nothing while no zone text is set
(`GetRealZoneText` is nil or empty). A name already present is not stored twice. `RemoveChatWindowChannel`
takes a name or the number of a joined channel and clears the matching slot; it returns no values.

Addresses: `0x4a0dc0`, `0x4a0e44`, `0x4a0e4f`, `0x4a1000`, `0x4a10b2`, `0x4a10d0`, `0x4a10d9`, `0x4a10e5`, `0x4a1100`, `0x4a1144`, `0x4a120c`, `0x4a122b`, `0x4a1260`, `0x4a1315`, `0x4a1359`, `0x4a1378`, `0x4a13aa`, `0x49e880`, `0x49be50`, `0xb4b404`
Confidence: verified
Proof: one
Test: none

The add verb scans the ChatChannels rows comparing the argument with the shortcut column (`+0x30`
plus the locale index times 4, `0x4a10b2`); with no match the id stays 0 and the name is the argument.
On a match `0x4a10d0` tests the zone-text pointer `0xb4b404` and `0x4a10d9` leaves with no values if it is
null or empty; otherwise the id is the row's field 0 and `0x4a10e5` takes the row's shortcut. A free slot
(`0x4a120c`) or a doubled array takes the string copy and `0x4a122b` the id. The remove verb resolves
a numeric argument through `0x49e880` and `0x49be50`, which return the joined channel's name only
while that slot's number matches and its state is 0, and frees the matched name (`0x4a13aa`). The
stock chat frame treats the add verb's nil as "not yet" and 0 as a custom channel.

## Settings file writer

The writer `0x499a80` writes the settings to the account's cache file, `chat-cache.txt`, whole, with
a handle of -1 writing nothing. The header is `VERSION 2`, `ADDEDVERSION 2`, the guild recruitment
option line, `CHANNELS` with the joined custom channels, `END`, `ZONECHANNELS n`, `COLORS` with one
`name r g b` line per registry colour, and `END`. Then ten blocks: `WINDOW n`, `NAME s` only when the
name is not empty, `SIZE`, `COLOR r g b a`, `LOCKED` (0 or 1), `DOCKED` (raw), `SHOWN` (0 or 1), a blank
line, `MESSAGES` with each set group's name and `END`, `CHANNELS` with the window's custom channels and
`END`, `ZONECHANNELS mask` and `END`.

Addresses: `0x499a80`, `0x498a20`, `0x498a40`, `0xb6e5c4`, `0x42a5d0`, `0x499a97`, `0x499ad5`, `0x499b0e`, `0x499b22`, `0x499b90`, `0x499b96`, `0x499ba1`, `0x499c19`, `0x499c7a`, `0x499cd4`, `0x499cfe`, `0x499d6f`, `0x499dbf`, `0x499dfd`, `0x499e54`, `0x499e9f`, `0x499ed7`, `0x499f1c`, `0x499f6e`, `0x499f83`, `0x49a04e`, `0x49a084`, `0x49a093`, `0x49a133`, `0x49a19a`, `0x843608`, `0x843614`, `0xb4fe00`, `0xb4fe04`, `0xb6e5e0`, `0xb4e518`, `0xb4e500`, `0xb4e504`
Confidence: verified
Proof: one
Test: none

Blank lines: `VERSION %d\n\n`, `ADDEDVERSION %d\n\n`, `%s %s\n\n` (option line), `ZONECHANNELS %d\n\n` and `END\n\n` each end in one, as does `SHOWN`. Each line is `_snprintf` into a 0x100 buffer and written by `0x42a5d0`, which calls `WriteFile`. The
registration `0x498a40` installs `0x499a80` as the saver of cache-file kind 7 and clears `0xb6e5c4`, which only the channel join and leave set and no instruction reads; the window setters mark nothing and the file is rewritten whole. The header option line is
the string at `0x843614` followed by `AUTO` when `0x843608` is 1, else `STANDARD`. The header `CHANNELS`
lists the joined-channel slots (array `0xb4fe04`, count `0xb4fe00`, stride 0xa0) with a number, state 0
and ChatChannels id 0 (`0x499b90`, `0x499b96`, `0x499ba1`). The `COLORS` lines cover the 94-entry
registry (`0x499cd4`, bound 0x189a) and then the extra channel colours, skipping empty names. `COLOR`
pushes alpha, blue, green, red so the line reads red first. The window's `ZONECHANNELS` is the global
mask `0xb6e5e0` and the bits of the window's own channel ids, `1 << (id-1)` for each non-zero id
(`0x49a04e`); a window's `CHANNELS` line is its own stored name when a joined slot of the same
composed name is joined or suspended (`0x49a093`).

## Settings file reader

The loader `0x498a60` parses the whole file line by line, keywords matched without case. First it
empties the deferred-join list and clears every window's 68 flag bytes. Line 1 is `VERSION n`; for n of
2 or more, line 2 may be `ADDEDVERSION m`. At top level the keywords are `CHANNELS` (header list),
`COLORS`, `WINDOW n`, `ZONECHANNELS n` (sets the global mask) and the guild recruitment option line
(`STANDARD` clears the latch, any other word sets it). A line that matches nothing is ignored; the
`VERSION` value is stored nowhere.

Addresses: `0x498a60`, `0x498a78`, `0x498a89`, `0x498aaf`, `0x498b8d`, `0x498bc4`, `0x498c0b`, `0x498c30`, `0x498cf4`, `0x498d13`, `0x498d17`, `0x498d83`, `0x498e35`, `0x498eb4`, `0x498ebc`, `0x499a64`, `0x49ea70`, `0x64ae50`, `0x64ac60`, `0x64a4c0`, `0xb4fdd4`, `0xb4fdd8`, `0xb4fddc`, `0xb6e5e0`
Confidence: verified
Proof: one
Test: none

Reads go through the tokenizer `0x64ae50` with the delimiters carriage return and line feed for lines and
space, carriage return and line feed for words, so blank lines never reach the state machine and a
token over 127 bytes is cut. The `WINDOW` test `0x498d13` is `dec eax; cmp eax,0xa; ja`, which accepts
1 to 11, where the Lua setters reject 11 with `jae`; window 11 addresses the record after the array.
`ZONECHANNELS` overwrites `0xb6e5e0` (`0x498d83`). The option line stores 1 in `0x843608` through
`0x49ea70` for every word but `STANDARD`, `AUTO` and a missing word alike (`0x498e35`). `END` leaves a
window's `CHANNELS` or `MESSAGES` block for the window, and the window for the top level
(`0x498ebc`, `0x498eb4`). The jump table `0x499a64` picks the data-line state.

## Settings file lines

In the header `CHANNELS` block each line's first word joins the deferred-join list `0xb4fdd4`. In `COLORS`,
a registry colour name, or `CHANNELn` within the extra count, takes three words as red, green and blue.
Inside a window the first word is `NAME` (the rest of the line, 31 characters at most), `SIZE`, `COLOR`
(red, green, blue, alpha, as bytes), `LOCKED`, `DOCKED`, `SHOWN` (raw integers), `ZONECHANNELS`,
`CHANNELS` or `MESSAGES`. A window `ZONECHANNELS` adds each ChatChannels row whose bit `1 << (id-1)` is set
to the window and leaves the global mask alone. A `MESSAGES` line sets its group's flag.

Addresses: `0x498f11`, `0x498fa9`, `0x4990ce`, `0x499510`, `0x49962d`, `0xb4fdd8`, `0xb4fddc`, `0x4990f7`, `0x499157`, `0x4991b9`, `0x499290`, `0x4992df`, `0x49932c`, `0x499332`, `0x4993c6`, `0x49940c`, `0x4994a2`, `0x4995d4`, `0x499664`, `0xb4e500`, `0xb4e504`, `0xb4fe70`
Confidence: verified
Proof: one
Test: none

The five state arms are the jump table entries. `NAME` tokenizes straight into the record with size
0x20 (`0x4990f7`); `COLOR` stores four integers at `+0x8a`, `+0x89`, `+0x88`, `+0x8b` (`0x4991b9` and the
three after it). `LOCKED`, `DOCKED` and `SHOWN` are plain dword stores (`0x499290`, `0x4992df`,
`0x49932c`). The `ZONECHANNELS` arm walks the ChatChannels rows (`ui/chatchannels#chatchannels-rows`), taking the shortcut at
`+0x30` plus the locale, skips a name the window holds, fills the first null slot or doubles both
arrays and appends, and stores the row id in the id array. A `MESSAGES` word compares against the 68
group names (`0x499631` to `0x499651`) and stores 1 at `0xb4fe70 + window*0x98 + i`.

## Settings file backfill

At the end of the file, when `ADDEDVERSION` is below 2, every group that is default-on and added in a
later version is switched on again: window 1 for groups 0 to 9, window 2 for groups 10 to 67. With the
shipped table this enables `COMBAT_FACTION_CHANGE` (added in version 1) and `MONEY` (version 2) in window
2 for a file with no `ADDEDVERSION` line or version 0, `MONEY` only for version 1, and nothing from 2. Then
the loader sets the chat-ready flag `0xb6e5c8` and runs the channel refresh `0x49a210`.

Addresses: `0x49967c`, `0x49967f`, `0x499688`, `0x499692`, `0x49969a`, `0x4996a6`, `0x499a18`, `0x499a22`, `0x49a210`, `0xb6e5c8`
Confidence: verified
Proof: one
Test: none

`0x49969a cmp ecx,0xa; sbb esi,esi; inc esi` is 0 for a group below 10 and 1 for the rest, times 0x98.
A file with no `MESSAGES` lines leaves every window silent because the flags were cleared before
the parse.

## Defaults without a file

When the file is missing, empty or unreadable (a handle of -1, a size of 0, or a failed read) the
loader keeps the start-up window state and visits every ChatChannels row: a row with flag bit 1
adds `1 << (id-1)` to the global zone mask `0xb6e5e0` and its shortcut and id to window 1's channel
arrays. The shipped table has the flag on rows 1, 2 and 22, so the fresh mask is 0x200003. It then
raises the same events and sets the same flag as after a parse.

Addresses: `0x4997ad`, `0x4997e8`, `0x4997fc`, `0x499817`, `0x4998e6`, `0x499934`, `0x499a16`, `0xb6e5e0`
Confidence: verified
Proof: one
Test: none

The three early exits are the handle test `0x498a78`, the size test `0x498a89` (`GetFileSize` returns
0) and the read test `0x498aaf`. The row test is `test byte [eax+4],1` at `0x4997e8`. A stock file in
the wild carries 0x1200003, whose bit 24 is `GuildRecruitment`, set by the recruitment option rather
than by the flag.

## Start-up events

After a parse or the defaults, the loader fires `UPDATE_CHAT_WINDOWS` (id 395, no arguments) and then
`UPDATE_CHAT_COLOR` (id 226, format `%s%f%f%f`) for each of the 94 registry colours and each extra
channel colour, passing the name and red, green and blue as the byte times 1/255. The same colour
event is fired by `ChangeChatColor` alone. The window start-up fires none.

Addresses: `0x4996b9`, `0x4996be`, `0x4996c3`, `0x4996ea`, `0x4997a6`, `0x499934`, `0x703e50`, `0x703f50`, `0x4a098b`
Confidence: verified
Proof: one
Test: none

`mov ecx,0x18b; call 0x703e50` and a loop over the registry from `0xb4e518` with count 0x5e (no name test) calling
`0x703f50` with id 0xe2. The extra entries follow in the same loop form. The six sites are the two paths'
pairs of calls to each dispatcher plus the extra-entry loops.

## Chat logging

`LoggingChat` and `LoggingCombat` set a per-session flag from their argument and return 1 or nil.
With no argument they only read. With an argument they store the flag read by `0x6f1c10` with default
1, and on the first enable create `Logs\` and open `Logs\WoWChatLog.txt` or `Logs\WoWCombatLog.txt`
for appending; a failed open stores 0 again. The return value is the number 1 or nil, never a
boolean. Nothing prints on a change.

Addresses: `0x49fe40`, `0x49fef0`, `0x49fe50`, `0x49fe60`, `0x49fe70`, `0x49fe79`, `0x49feab`, `0x49febf`, `0x49fed4`, `0x42a960`, `0x65a0c0`, `0x65a1c0`, `0xb4fdb4`, `0xb50540`, `0x84360c`, `0x843610`, `0x498700`
Confidence: verified
Proof: one
Test: none

The two entries load `edx` with 0 and 1 and jump to the shared `0x49fe50`. `lua_gettop <= 0` skips the
write. The flags are `0xb4fdb4[index]` and the writer objects `0xb50540[index]`; a null object with a
non-zero flag calls `0x42a960` (directory creation) and `0x65a0c0(path, 4, &object)`, where bit 2 opens
with append and seeks to the end. The paths are the pointers at `0x84360c` and `0x843610`. The
chat teardown `0x498700` closes the objects and clears the flags; the settings writer never reads them.

## Enumerating server channels

`EnumerateServerChannels()` returns the shortcut of each ChatChannels row that applies where the
player stands, in row order, or the name column when the shortcut is empty. It returns nothing when
the current zone is negative, past the AreaTable maximum or has no row. A row with flag 0x10 is
listed only in a zone whose AreaTable flags include 0x8 (a capital city). The 1.12.1 table holds six
rows: General, Trade (city), LocalDefense, WorldDefense, LookingForGroup, GuildRecruitment (city).

Addresses: `0x4a1790`, `0x4a17a3`, `0x4a17b1`, `0x4a17b9`, `0x4a17c9`, `0x4a17f9`, `0x4a17ff`, `0x4a1818`, `0xb4e314`
Confidence: verified
Proof: one
Test: none

The function reserves one stack slot per row, reads the zone `0xb4e314` and tests it against
`ds:0xc0e04c`, the AreaTable maximum, then against the table slot. The names are the row shortcuts and
never the composed `name - zone` text. The count of rows in the stock table (six, with flags and
shortcuts) is data of the install, not of the exe.

## Chat script functions

`0x4a1a10` registers the 50 name and function pairs of `0x843618` with `FrameScriptRegisterFunction`
(`0x704120`), eight bytes a pair, 0x190 bytes in all. They are `SendChatMessage`, `SendAddonMessage`, the
language functions, `DoEmote`, the logging pair, the channel functions (join, leave, list, owner,
password, moderation, `GetChannelName`), the colour functions, the window functions of this doc,
`EnumerateServerChannels`, the guild recruitment pair and the raid instance functions. `0x4a1a40`
unregisters them.

Addresses: `0x4a1a10`, `0x4a1a40`, `0x4a1a13`, `0x4a1a1f`, `0x4a1a2d`, `0x843618`, `0x704120`
Confidence: verified
Proof: one
Test: none

`mov edx,[esi+0x84361c]; mov ecx,[esi+0x843618]; call 0x704120; add esi,8; cmp esi,0x190`: the pair is
the name pointer then the function. The name strings lie in the reverse order of the pairs, so a name
is read from its pointer and never from its neighbour. `tables/bindings.tsv` lists every pair.

## Chat shutdown

`0x498700` closes both log writers and zeroes their flags, empties the colour, channel and window
arrays (each window's names freed through `SMemFree`, the counts reset), frees the deferred-join list
and unregisters the 14 network messages the chat code handles: 0x99, 0x9b, 0x96, 0x105, 0x254,
0x33b, 0x291, 0x2cc, 0x2fa, 0x31e, 0x31f, 0x320, 0x32b and 0x332. The messages are registered by the
start-up `0x4982c0`.

Addresses: `0x498700`, `0x498710`, `0x49871b`, `0x498726`, `0x4987c8`, `0x49883c`, `0x4988a8`, `0x5ab670`, `0x4982c0`, `0x5ab650`
Confidence: verified
Proof: one
Test: none

The loop at `0x498710` closes `0xb50540[i]` through `0x659ec0` and stores 0 to it and to `0xb4fdb4[i]`
for two entries. The window loop from `0xb4feb8` runs ten times (count in `[ebp-4]`, step 0x98). The
unregistration is one `call 0x5ab670` per message with `ecx` its opcode (`0x4988bf` to `0x498941`); the
start-up registers the same fourteen with `0x5ab650`, the sites in `tables/opcodes.tsv` from `0x49861a`
to `0x4986ea`.
