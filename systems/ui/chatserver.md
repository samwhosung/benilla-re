# ui: the server's chat broadcasts

Four server messages that become a chat line without a chat message: the zone under attack notice, the
defence message, the server message and the chat restriction. None fires an event of its own name; each
ends in the chat line function as a `CHAT_MSG_CHANNEL` or `CHAT_MSG_SYSTEM` line (`chatevents`).

The first three handlers are in `ChatFrame.cpp`, registered by the chat start-up (`chatwindows#chat-shutdown`);
the fourth is an arm of the player-services dispatcher `0x5e38c0`, registered at `0x5e36c7`.

Roots: `0x49dcc0`, `0x49de30`, `0x49df80`

## Zone under attack

Opcode 0x254, `0x49dcc0`: `u32 areaId`. The handler returns at once for an id that is negative, past the
AreaTable maximum or without a row. Otherwise it formats the global `ZONE_UNDER_ATTACK` with the area's name
in the client's locale into an 0x800-byte buffer, replaces the id by its parent area when it has one, and
shows the text as a `CHANNEL` line (chat type 14) on every defence channel it applies to
(`The defence channels`). It always returns 1.

Addresses: `0x49dcc0`, `0x49dcd2`, `0x49dcda`, `0x49dce2`, `0x49dcf4`, `0x49dd05`, `0x49dd0f`, `0x49dd14`, `0x49dd26`, `0x49dd2b`, `0x49dd35`, `0x49de23`, `0x498650`, `0x49865a`, `0x703bf0`, `0x64a7f0`, `0xc0e04c`, `0xc0e048`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

The area name is `[row+4*locale+0x2c]` (`0x49dd05`). It is pushed as the formatter's argument
(`0x49dd09`) before `0x703bf0("ZONE_UNDER_ATTACK", -1, 0)` (`0x49dd0f`, `0x49dd14`; `ret 4` eats the second push), and
`0x64a7f0(buf, 0x800, text, name)` formats it. `0x49dd2b` loads `ParentAreaID` at `+0x08`; a non-zero one
replaces the id (`0x49dd35`). The string `ZONE_UNDER_ATTACK` has one reference in the image, `0x49dd10`, and no
event of that name is in the event table, so the stock chat frame's handler for it never runs.

## Defence message

Opcode 0x33b, `0x49de30`: `u32 zoneId`, `u32 length`, `length` bytes of text. The text is not copied: it is a
pointer into the packet, and no terminator is looked for. The zone id is replaced by its parent when the
area has a row and a parent, and is otherwise kept as sent. It is shown like the zone notice on the
defence channels. A length past the packet leaves a null pointer, which the chat line function refuses.

Addresses: `0x49de30`, `0x49de42`, `0x49de4d`, `0x49de5c`, `0x49de66`, `0x49de61`, `0x49de87`, `0x49de8a`, `0x49df04`, `0x49df4c`, `0x419ac0`, `0x49a881`, `0x49a88b`, `0x498660`, `0x49866a`
Confidence: verified
Proof: one
Test: none

`GetDataInSitu` (`0x419ac0`) returns the buffer pointer and advances the cursor, and writes 0 to its output
when the length overruns (`0x419acc`); the handler checks nothing and passes it on (`0x49de66`). The chat line
function then takes its null-text exit (`0x49a881`, `0x49a88b`). Unlike the zone notice, the bounds tests at `0x49de61` to
`0x49de87` fall through when the id is bad instead of leaving.

## The defence channels

Both notices walk the joined channels and show the text on each slot that has a number, is confirmed
(state 0), whose ChatChannels row has flag 0x10000, and, when the row also has flag 0x2, whose zone is the
player's current zone (`0xb6e5d0`, the parent-resolved zone of the last zone walk). The shipped rows with
0x10000 are LocalDefense (zone-scoped) and WorldDefense (not). The line is the chat line function with type
14, no sender, language 0 and the slot's stored name as the channel, so the channel is always found and its
number shows. While the display latch is set the line is queued instead.

Addresses: `0x49dd38`, `0x49dd50`, `0x49dd5a`, `0x49dd69`, `0x49dd78`, `0x49dd84`, `0x49dd91`, `0x49dd94`, `0x49dd9c`, `0x49dda4`, `0x49ddac`, `0x49ddca`, `0x49dddd`, `0x49ddf9`, `0x49def4`, `0x49defc`, `0x49df45`, `0xb6e5d0`, `0xc0df1c`, `0xc0df20`
Confidence: verified
Proof: one
Test: none

The five gates are `cmp [slot],0` (`0x49dd50`), `cmp [slot+0x9c],0` (`0x49dd5a`), the id range and row test
(`0x49dd69` to `0x49dd8f`), `test [row+4],0x10000` (`0x49dd94`) and, for 0x2, `cmp zone,[0xb6e5d0]`
(`0x49dda4`, `0x49def4` in the defence handler). A custom channel has id 0 and no row 0 exists, so it never
passes. The live call is `edx = 0xe` with the other arguments 0 (`0x49ddf9`, `0x49df45`); these two sites
are the only code that passes the literal type 14. The queued call is `0x49cae0` with `ecx = 0xe` and a language
field of 1 (`0x49dddd`).

## Server message

Opcode 0x291, `0x49df80`: `u32 type`, then a C string of at most 0x400 bytes. The type indexes
`ServerMessages` rows directly; the row's text for the client's locale is a format. With a row and a
non-empty string the line is the format filled with the string; with a row and an empty string it is
the row's text; with no row it is `[<type>]: <string>`. The line is a system line (chat type 10).

Addresses: `0x49df80`, `0x49df93`, `0x49dfa6`, `0x49dfd8`, `0x49dff5`, `0x49dffa`, `0x49e010`, `0x49e012`, `0x49e030`, `0x49e033`, `0x49e052`, `0xc0d988`, `0xc0d994`, `0x54c5c0`, `0x54c615`, `0x54c669`, `0x54c69e`, `0x540691`, `0x498670`, `0x49867a`
Confidence: verified
Proof: one
Test: none

The store `0xc0d988` is loaded by `0x54c5c0` (10 fields, 40-byte records, asserted at `0x54c669` and
`0x54c69e`) with the id map at `+0x08` and the maximum at `+0x0c`; the text is field `1 + locale`. The tests
at `0x49dfa6` onward are `jl`, `cmp [0xc0d994]`, `jg` and the null row. The three formats are
`SStrPrintf(buf, 0x400, text, string)`, `SStrCopy(buf, text, 0x400)` and `SStrPrintf(buf, 0x400, "[%d]: %s",
type, string)`. This handler does not test the display latch. The shipped table has five rows (shutdown and
restart countdowns, custom text, and two cancellations).

## Chat restricted

Opcode 0x2fd, an arm of `0x5e38c0` (`0x5e4a09`): no body is read. It shows message id 0x1c3 through the
message catalog function `0x496720`, which is the global `ERR_CHAT_RESTRICTED` as a system line with no
sound.

Addresses: `0x5e38c0`, `0x5e36c7`, `0x5e38dc`, `0x5e4156`, `0x5e46a1`, `0x5e49b4`, `0x5e49c3`, `0x5e49ca`, `0x5e4a09`, `0x5e4a0e`, `0x5e4ae8`, `0x496720`, `0x4882ec`
Confidence: verified
Proof: one
Test: none

The arm is `push 0x1c3; call 0x496720; mov eax,1; ret 8`; it reads nothing from `[ebx+0xc]`. Its dispatch
is the cascade to `lea eax,[edx-0x2eb]`, the byte table at `0x5e4e20` (index 0x12 gives 2) and the five-entry
jump table at `0x5e4e0c`. Row 451 of `tables/messages.tsv` (set at `0x4882ec`) holds the key, kind 0
(chat composer), sound `NONE` and chat type 10. The catalogue lookup shows nothing for a row without a key.

## The text lookup

`0x703bf0(key, ordinal, gender)` returns the Lua global named by the key as a string, and the empty string
when the global is absent or not a string; it never returns null and never raises. For ordinal -1 and gender
0, the case every handler above uses, it is a plain read of the global table; otherwise it calls the
global function named `GetText` (the string at `0x872cc8`) with the key, gender and ordinal.

Addresses: `0x703bf0`, `0x703c02`, `0x703c12`, `0x703c17`, `0x703cd9`, `0x704350`, `0x704390`, `0x872cc8`
Confidence: verified
Proof: one
Test: none

`0x703c02` stores the empty string `0x882748` as the result before anything; `0x704350` pushes the key and reads
the global table (index -10001, `LUA_GLOBALSINDEX`), and `lua_isstring` gates the result store at `0x704390`.
Nothing in the exe writes these globals; they come from the interface's `GlobalStrings` file, so a client
that has not loaded it shows blank lines.
