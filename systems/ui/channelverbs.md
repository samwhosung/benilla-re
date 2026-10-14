# ui: joining and leaving channels

The Lua verbs that join, leave and name a channel, the packets they send themselves, and the guild
recruitment mode that joins a capital's recruitment channel by itself. The slots they work on and the
ChatChannels rows are `chatchannels`.

The code is `ChatFrame.cpp`: the verbs `0x49ff00`, `0x4a0000`, `0x4a05e0`, `0x4a0040` and `0x4a0060`,
and the bodies `0x49eb70` (join), `0x49ee70` (leave) and `0x49ea90` (the cascade).

Roots: `0x49ff00`, `0x4a0000`, `0x4a05e0`, `0x4a0040`, `0x4a0060`

## JoinChannelByName

`JoinChannelByName(name, password, frameId)` needs a string or number name (anything else raises
`Usage: JoinChannelByName("name")`). A name with a space returns no values and sends nothing. The password
is optional and becomes `""` when absent or not a string; `frameId` is optional and is the 1-based
chat window (a value outside 1 to 10 only skips the window registration). The verb returns two values:
the ChatChannels id (a number) and the row's shortcut when the name is a shortcut, and the number 0 and nil
for any other channel. It returns no values when a shortcut matches while the zone text is nil or empty. It sends the join.

Addresses: `0x49ff00`, `0x49ff10`, `0x49ff24`, `0x49ff35`, `0x49ff4e`, `0x49ff57`, `0x49ff5e`, `0x49ff6e`, `0x49ff8a`, `0x49ff8b`, `0x49ff9e`, `0x49ffa3`, `0x49ffa8`, `0x49ffac`, `0x49ffbc`, `0x49ffcc`, `0x49ffdd`, `0x49ffe7`, `0x49fff5`, `0x64a3c0`, `0x6f3810`, `0x6f3890`, `0x6f37f0`
Confidence: verified
Proof: one
Test: none

`lua_isstring` (`0x6f3510`, true for a string or a number) gates argument 1 (`0x49ff10`); failing it
pushes the usage string (`0x49ffe7`) and raises. `0x64a3c0(name, ' ')` finds the space (`0x49ff57`); a hit
jumps to the zero-value exit `0x49fff5`. `frameId` is `__ftol(arg3) - 1` (`0x49ff8a`); a missing one stays
-1, which the body's unsigned test rejects. Both out-parameters are zeroed (`0x49ff8b`), the body is
called with flag 1 (`0x49ff9e`) and a false result gives no values (`0x49ffaa`). The two values are
`fild [ebp-4]` then a string or nil by whether the second out-parameter is null; both exits set
`eax = 2`. A custom channel keeps the zeroed id, so its first value is the number 0, not nil.

## The join body

The body `0x49eb70(name, password, frameIdx, flag, &shortcut, &id)` matches the name, ignoring case,
against the ChatChannels shortcuts; the first row wins. For a row it composes the wire name from the
row's pattern and the zone or city text (`chatchannels#composing-a-channel-name`), uses an empty
password, fails without sending when that text is nil or empty, and stores the row's shortcut and id.
Any other name goes verbatim with the caller's password. It then adds the shortcut or name to chat window
`frameIdx` unless present, finds or creates the slot with the id, and sends opcode 0x97 with the name and
password. A re-join sends again, and a full slot array still sends.

Addresses: `0x49eb70`, `0x49eb79`, `0x49eb9b`, `0x49ebe0`, `0x49ec0c`, `0x49ec13`, `0x49ec24`, `0x49ec27`, `0x49ec35`, `0x49ecb6`, `0x49ece3`, `0x49ece5`, `0x49ecea`, `0x49ecf3`, `0x49ecfa`, `0x49ed14`, `0x49ed19`, `0x49ed21`, `0x49ed3d`, `0x49ed47`, `0x49edde`, `0x49ede8`, `0x49edf3`, `0x49ee02`, `0x49ee1c`, `0x49ee42`, `0x49ee66`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

The scan compares the shortcut at `+0x30` plus the locale (`0x49ebfc` to `0x49ec0c`). `0x49ece5` and
`0x49ecea` are the only two predecessors of the failure exit `0x49ed47`; every other path ends with `al = 1`
(`0x49ee66`). `0x49ecf3` and `0x49ecfa` are the only stores through the out-parameters. `0x49ed21` stores the
empty password inside the matched branch. `0x49edde` sets the unread flag `0xb6e5c4`. The packet is
`push 0x97`, `0x418190`, `0x418430` for the name, `0x418430` for the password and `0x5ab630` (`0x49ee02` to
`0x49ee42`); `0x49b9c5` shows system message 0x199 when the array is full and `0x49edf3` ignores that result.

## The guild recruitment reset on join

The join body sets the guild recruitment mode to 0 when its flag is non-zero and the matched row has flag
0x20000. The Lua verb passes flag 1, so `JoinChannelByName("GuildRecruitment")` turns the mode off. The
cascade passes 0 and does not.

Addresses: `0x49ed19`, `0x49ed1f`, `0x49ed2f`, `0x49ed35`, `0x49ed3d`, `0x49ea70`, `0x49eb47`, `0x49ff9e`, `0x843608`
Confidence: verified
Proof: one
Test: none

`mov al,[ebp+0xc]; test al,al; je 0x49ec24` (`0x49ed19` to `0x49ed1f`) skips the reset when the flag is 0, and
the row test is `test [row+4],0x20000` before `call 0x49ea70` with `ecx = 0` (`0x49ed3d`). The callers push 1
(`0x49ff9e`) and 0 (`0x49eb47`).

## LeaveChannelByName

`LeaveChannelByName(name)` needs a string or number (else it raises `Usage: LeaveChannelByName("name")`),
returns no values and sends `CMSG_LEAVE_CHANNEL`, opcode 0x98 with one C string. If the name parses as
a non-zero integer n, the key is the name of joined slot n and anything that slot cannot supply is a
complete no-op. Otherwise the key is the argument as typed, so `"0"` and `""` are names. The wire name is
the key, except that a key matching a shortcut becomes the row's pattern filled with the plain zone text,
or the verb does nothing when that text is nil or empty. No slot lookup or state test precedes the send for a name.

Addresses: `0x4a0000`, `0x4a0008`, `0x4a0011`, `0x4a001f`, `0x4a0023`, `0x4a002c`, `0x49ee70`, `0x49ee7c`, `0x49ee7f`, `0x49ee88`, `0x49eea0`, `0x49eeac`, `0x49ef94`, `0x49efca`, `0x49efd2`, `0x49efe8`, `0x49eff6`, `0x49eff9`, `0x49f087`, `0x49f098`, `0x49f0b2`, `0x49f0c1`, `0x49f0cc`, `0x49e880`, `0x49be50`, `0x64ac60`, `0x6f3510`
Confidence: verified
Proof: one
Test: none

`0x49e880` parses with `0x64ac60` (`SStrToInt`: an optional minus, then digits, 0 when the first character
after the sign is not a digit) and, for a non-zero n, tail-jumps to `0x49be50`; for 0 it returns the string. A
null key leaves at once (`0x49ee88` to `0x49f129`). The matched branch loads the zone text into `ebx`
(`0x49eeac`), tests it for nil and empty (`0x49efca`, `0x49efd2`, both to the bare exit) and calls
`0x64a7f0(buf, 0x100, pattern, zoneText)` (`0x49efe8` to `0x49eff9`); no area or city row is read there. The
body runs for a caller flag of 1 from the verb and 0 from the cascade, and its third argument,
which would skip only the packet, is 0 at both call sites. In a capital `Trade` therefore leaves
`Trade - <city>`, which is not the joined `Trade - City`.

## Leave side effects

After the wire name is fixed, a flag-1 call whose key equals the composed name of the guild recruitment
row sets the recruitment mode to 0. Every chat window loses the first entry equal to the key, where the key is the
matched row's shortcut or the key as typed. After the send the slot named by the wire name is looked
up in any state and, if its id is not 0, bit id-1 of the zone channel mask is cleared. The slot itself
and its state stay until a server notice drops them. The unread flag `0xb6e5c4` is set.

Addresses: `0x49eee1`, `0x49eeec`, `0x49eefb`, `0x49ef0c`, `0x49ef2a`, `0x49ef32`, `0x49ef4c`, `0x49ef6f`, `0x49ef84`, `0x49ef8f`, `0x49f001`, `0x49f010`, `0x49f036`, `0x49f048`, `0x49f064`, `0x49f0ee`, `0x49f0f4`, `0x49f0fd`, `0x49f10a`, `0x49f11a`, `0x49f11f`, `0x49be90`, `0x646430`
Confidence: verified
Proof: one
Test: none

The reset needs five things (`0x49eee1`, `0x49eeec`, `0x49eefb`, `0x49ef4e`, `0x49ef84`): the flag, row flag
0x20000, a non-empty zone text, a non-empty substitution and `0x64a4c0(composed, key) == 0`. So the bare
word `GuildRecruitment` resets nothing. The strip loops ten windows from `0xb4feb8`
(`0x49f001`), frees the name with `SMemFree`, zeroes the name and id cells and leaves the count. A
window entry is the shortcut for a shortcut argument, so a number leaves it. The mask clear is `0x49f0f4` to
`0x49f11a`. Nothing here frees a slot or fires an event.

## GetChannelName

`GetChannelName(x)` returns three values: the slot's number, its name and its instance number. A number
(or numeric string) selects the slot by number among confirmed slots; a string selects the first slot by name in any state;
anything else raises the usage error. With no slot it returns 0, nil, 0.

Addresses: `0x4a05e0`, `0x4a05ed`, `0x4a0609`, `0x4a0610`, `0x4a0627`, `0x4a062c`, `0x4a0632`, `0x4a0654`, `0x4a0666`, `0x4a0689`, `0x4a0693`, `0x49bf30`, `0x49be90`
Confidence: verified
Proof: one
Test: none

`lua_isnumber` on argument 1 (`0x4a05ed`) sends it through `0x49bf30` after `__ftol`; otherwise
`lua_isstring` sends it through `0x49be90` (`0x4a0627`); neither raises (`0x4a0693`). The pushes are
`[slot]` (`0x4a0632`), `slot+4` or null (`0x4a0654`) and `[slot+0x98]` (`0x4a0666`); `mov eax,3` at `0x4a0689`.

## Guild recruitment mode

`GetGuildRecruitmentMode()` returns the number in `0x843608`, 1 at start and 0 or 1 after any write.
`SetGuildRecruitmentMode(mode)` truncates a number (a numeric string works) and raises for anything
that is not 0 or 1 after truncation (`SetGuildRecruitmentMode: invalid mode`), or `Usage:` for a non-number.
It stores the mode, runs the cascade when the mode is 1, then fires `UPDATE_CHAT_WINDOWS`, and returns
nothing. Mode 0 changes the value only and leaves any joined channel joined.

Addresses: `0x4a0040`, `0x4a0060`, `0x4a006d`, `0x4a008f`, `0x4a0094`, `0x4a0098`, `0x4a009f`, `0x4a00a4`, `0x4a00a9`, `0x4a00b2`, `0x49ea70`, `0x49ea73`, `0x49ea79`, `0x49ea7b`, `0x843608`, `0x498e0a`, `0x498e35`, `0x499b0e`, `0x49ea96`
Confidence: verified
Proof: one
Test: none

`0x49ea70` is `cmp ecx,1; mov [0x843608],ecx; jne ret; jmp 0x49ea90`: the store is unconditional and the
cascade runs only for 1. The value is written there and nowhere else. The settings loader calls it with
0 for the word `STANDARD` and 1 for any other word. The getter loads it with `fild` and pushes a number.
The stock options window passes 1 or 0. The saver writes `AUTO` for exactly 1 and `STANDARD` otherwise,
and it does so at chat teardown, not at the call.

## Guild recruitment cascade

With the mode at 1, `0x49ea90` acts on the recruitment channel at once. It does nothing without an
active player. It composes the channel names once (`0x49f140`). With no zone row it arms the retry
flag `0xb6e5e4` and returns. For a player in a guild (`PLAYER_GUILDID` nonzero) it sends a leave of
`GuildRecruitment - City` and fires `UPDATE_CHAT_WINDOWS`. For one without a guild, in an area whose
flags include 0x100, it joins `GuildRecruitment` through the join body (frame 0, flag 0) and fires the
event; outside a capital it arms the retry flag. Neither arm checks for an existing membership.

Addresses: `0x49ea90`, `0x49ea96`, `0x49eaa9`, `0x49eac9`, `0x49ead2`, `0x49ead7`, `0x49eaf5`, `0x49eb01`, `0x49eb0c`, `0x49eb10`, `0x49eb17`, `0x49eb1c`, `0x49eb21`, `0x49eb2e`, `0x49eb33`, `0x49eb3f`, `0x49eb55`, `0x49eb5a`, `0x49eb5f`, `0x49f140`, `0xb50440`, `0xb504c0`, `0xb6e5ec`, `0x5dec1e`, `0x5e279a`, `0x49a6b2`
Confidence: verified
Proof: one
Test: none

The callers are the zone walk's tail (`0x49a6b2`), the latch write of 1 (`0x49ea7b`), the player creation
(`0x5dec1e`) and the guild id watcher (`0x5e279a`). `[[player+0xe68]+0xc]` is read at `0x49eb01`.
The retry arming stores 1 at `0xb6e5e4` (`0x49eaf5`, `0x49eb33`). The composer `0x49f140` runs once
(`0xb6e5ec`): it needs the city row, finds the first ChatChannels row with flag 0x20000, copies its shortcut
into `0xb50440` and formats its pattern with the city row's name into `0xb504c0`. The leave in the
cascade is given the composed name, so its window strip, keyed on the full name, misses the shortcut entry.
