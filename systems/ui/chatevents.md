# ui: chat messages and their events

How a line of chat reaches the interface: the server's chat message, the checks and queues before a
line shows, the chat line function every chat source calls, and the `CHAT_MSG_*` event it fires with
ten arguments. The window settings are `chatwindows`, channels `chatchannels`, the language rewrite
`chatlanguage`, and sending `chatsend`.

The code is `ChatFrame.cpp`: the chat message handler `0x49d560`, the chat line function `0x49a870`, the
event helper `0x49b0b0` and the line queue `0x49cae0`.

Roots: `0x49d560`, `0x49a870`, `0x49b0b0`, `0x49cae0`, `0x49ccc0`, `0x49d230`, `0x49dbe0`

## The chat message

Opcode 0x96, handled by `0x49d560`: `u8 type`, `u32 language`, then by type. Types 0, 1, 5, 11 and 12
(say, party, yell, monster say, monster yell) first carry a GUID. Types 11, 12, 13, 26, 89 and 90 then carry
`u32 length` and that many bytes of speaker name. Type 14 (channel) carries a channel name (a C string of up
to 128 bytes) and a `u32` rank. Every type ends with a GUID, `u32 length`, that many bytes of text and
`u8 tag`.

Addresses: `0x49d560`, `0x49d5af`, `0x49d5ba`, `0x49d5dc`, `0x49d5e4`, `0x49d610`, `0x49d6a3`, `0x49d6b1`, `0x49d6bc`, `0x49d6c7`, `0x49d6d2`, `0x49d6e1`, `0x49d6f2`, `0x49d671`, `0x49d67e`, `0x49d961`, `0x49da3d`, `0x498630`, `0x4190b0`, `0x418e30`, `0x418eb0`, `0x4191b0`, `0x419ac0`, `0x418cb0`
Confidence: verified
Proof: one
Test: none

The type byte is read at `0x49d5af` and the language at `0x49d5ba`; the five types of the first GUID are
tested at `0x49d5c2` to `0x49d5d4` (`GetGuid`, `0x49d5dc`). The tests `0x49d5e4` to `0x49d60e` send
types 11, 12, 90, 13 and 26 to `0x49da3d` (length, name, GUID, length, text, tag) and 89 falls into `0x49d610`, the
same reads. Types 82, 83 and 84 go to `0x49d961`, the default reads with their own checks; 14 reads the name
with limit 0x80 (`0x49d6b1`) and a `u32` (`0x49d6bc`). The text is not copied: `GetDataInSitu` (`0x419ac0`) returns a pointer into the packet, advancing past the
length. The tag byte is read at `0x49d6f2`. Type 89 reads its fields at `0x49d610` to `0x49d65e` and, if its sender is not ignored (`0x49d671`), becomes type 6, a whisper (`0x49d67e`). The handler is registered by `0x498630` to `0x49863a`.

## The chat tag

The tag byte becomes a token for the interface: 1 is `AFK`, 2 is `DND`, 3 is `GM` and anything else is
empty. A line from an ignored player is dropped unless its tag is 3 (GM) on the default path; the monster path (`0x49daa5`) and type 89 (`0x49d678`) drop an ignored sender whatever the tag. An ignored whisper instead sends the
ignored reply to the server.

Addresses: `0x49d6f7`, `0x49d6fe`, `0x49d70b`, `0x49d718`, `0x49d72d`, `0x49d736`, `0x49d73c`, `0x49d740`, `0x49d74e`, `0x49d4b0`, `0x5ae5a0`, `0xc28168`
Confidence: verified
Proof: one
Test: none

The mapping stores the string pointers for `DND`, `AFK` and `GM`
in `[ebp-0x1c]`, which starts at the empty string (`0x49d5a5`). The ignore test `0x5ae5a0(list, guid)`
(`0xc28168` is the friend list) jumps to the bare return `0x49dbd0` unless the tag is 3
(`0x49d736`); a whisper (type 6) calls `0x49d4b0` instead, `SendIgnoredResponse`, and returns.

## Before a line is shown

For a speaker with a GUID the handler asks the name cache for the speaker's name (`0x55f080`). If the GUID is
non-zero and the name is not yet cached, the line is queued with its fields and the handler returns; the
name query's callback shows it later. The speaker's talk or laugh gesture is chosen next (for say, yell
and party); then the sender name is built, and the line is queued instead of shown while the display latch
`0x8435fc` is set. Otherwise it goes to the chat line function.

Addresses: `0x49d761`, `0x49d77f`, `0x49d793`, `0x49d797`, `0x49d7ba`, `0x49d7d0`, `0x49d8b3`, `0x49db5c`, `0x49db62`, `0x49db8c`, `0x49dbcb`, `0x55f080`, `0x49cae0`, `0x49ccc0`, `0x8435fc`, `0xc0e228`
Confidence: verified
Proof: one
Test: none

The name cache `0xc0e228` is called with `0x49ccc0` as the callback (`0x49d77f`); a zero return for a non-zero
GUID (`0x49d793`, `0x49d797`) queues the line through `0x49cae0` (`0x49d7ba`) and returns. The latch test
is `cmp [0x8435fc],ebx; je 0x49db9f` (`0x49db5c`); a set latch queues through `0x49cae0` (`0x49db8c`). The
call is `0x49dbcb`. The addon test of the language is downstream of all of these.

## The display latch

`0x8435fc` holds 1 from start-up until the world-enter code has run, and again while the interface is
torn down for a world change; with it set, chat lines and the two defence broadcasts of `chatserver` are
queued, and the world-enter code sets it to 0 and runs that queue. A text emote received under the latch is
queued apart and never shown (`chatemote#the-pending-list`).

Addresses: `0x8435fc`, `0x498a10`, `0x490a9e`, `0x4989c0`, `0x4989fd`, `0x498a03`, `0x490974`, `0x49d230`, `0x49ccc0`, `0x49ce3c`, `0x49d165`, `0x49d239`, `0x49dc7f`, `0x49577a`
Confidence: verified
Proof: one
Test: none

The data word is 1 in the file. `0x498a10` stores 1 and has one caller, `0x490a9e`, in the world-leave
teardown; `0x4989fd` stores 0 in `0x4989c0` (one caller, `0x490974`, after `PLAYER_LOGIN` and
`PLAYER_ENTERING_WORLD`) which then jumps to the drain `0x49d230`. Eight reads: `0x49577a`, `0x49ce3c`,
`0x49d165`, `0x49d239`, `0x49db5c`, `0x49dc7f`, and the two defence handlers. `0x4989c0` also stores the
AFK mirror (`0xb6e5cc`) from `PLAYER_FLAGS` bit 2.

## The chat line function

`0x49a870(text, type, sender, language, channel, name2, tag, bubbleGuid, rank, reportGuid)` (ret 0x28) shows
one line. A null text sets last error 0x57 and returns. A language of -1 takes the addon arm. Otherwise it
forces the language to 0 for types 8, 10, 13 and 90 and replaces an empty text of type 8 with the global
`CHAT_EMOTE_UNKNOWN`; rewrites the text by language (`chatlanguage`); decorates the channel name; applies
the filters (`chatfilter`); replaces the sender by a PvP rank title; stores the line in the history; fires
the event; tells the keyboard display about a whisper; shows a bubble; and writes the log line.

Addresses: `0x49a870`, `0x49a881`, `0x49a88b`, `0x49a89b`, `0x49a89e`, `0x49a970`, `0x49a98e`, `0x49a99d`, `0x49a9f6`, `0x49aa17`, `0x49aa27`, `0x49aa48`, `0x49aa86`, `0x49ab23`, `0x49abbe`, `0x49abdb`, `0x49ac74`, `0x49ac9a`, `0x49ac9f`, `0x49aca7`, `0x49acd9`, `0x49acde`, `0x72b620`, `0x703bf0`
Confidence: verified
Proof: one
Test: none

`0x49a89e cmp edi,-1; jne 0x49a970` splits the addon arm. The channel name is looked up (`0x49aa27`) and
becomes `%d. %s` with the slot number when found, else is copied (`0x49aa86`). The text goes through
`0x49a750` (history), `0x49b0b0` (event, `0x49ac9a`), `0x72b620(sender)` for type 6 (`0x49aca7`) and, when the
speaker is a unit the client has, `0x608ac0(type, text)` (`0x49acd9`). Fifty call sites call `0x49a870`.

## The ten event arguments

`0x49b0b0(text, type, sender, language, channel, name2, tag, record)` fires the event of the line's type
with `%s%s%s%s%s%s%d%d%s%d`: the text, the sender, the language's name, the channel name with its number, the
second name of a two-name notice, the tag token, the channel's ChatChannels id, its slot number, its stored
name and its instance number; the last four are zero, zero, empty and zero without a joined channel.
A type of 94 or more fires nothing; the test is `cmp edx,0x5e; jge`, which is signed, so a negative type is not rejected (callers pass small positive types). The event of a type is `tables/chat_types.tsv`; type 89 has an id with
no name.

Addresses: `0x49b0b0`, `0x49b0b6`, `0x49b0bf`, `0x49b0c5`, `0x49b0f4`, `0x49b10c`, `0x49b113`, `0x49b115`, `0x49b11b`, `0x49b11e`, `0x49b12f`, `0x49b146`, `0x49b162`, `0x49b16e`, `0x49b189`, `0x49b18c`, `0x49b193`, `0x49b199`, `0x8062e0`, `0x703f50`, `0x49b1b0`, `0x49b1ce`, `0x49b495`
Confidence: verified
Proof: one
Test: none

`cmp edx,0x5e` bounds the type; the id is `[0x8062e0 + 4*type]`. A null string is replaced by the empty
string (`0x49b14a` to `0x49b16e`). The language name is `Languages` `Name_lang[locale]` for an id in range with
a row, else empty (`0x49b0c5` to `0x49b0f8`), so language 0 gives empty. Arguments 7 to 10 are
`[chan+0x94]`, `[chan+0]`, `chan+4` and `[chan+0x98]` (`0x49b11e`, `0x49b113`, `0x49b11b`, `0x49b115`), all
defaults for a null record (`0x49b12f`). The helper has three callers: `0x49a870`, a wrapper
`0x49b1b0` with six zero arguments (used by combat text) and the text emote composer (`0x49b495`).

## Event ids by chat type

The id table at `0x8062e0` has 94 dwords. Types 0 to 12 map to ids 228 to 240 (`CHAT_MSG_SAY` to
`CHAT_MSG_MONSTER_YELL`) and type 13, `CHAT_MSG_MONSTER_EMOTE`, to 242; 14 to 19 are the channel family,
243 to 248; type 26 (`MONSTER_WHISPER`) is 241. The colour and the event name of every type are
`tables/chat_types.tsv`.

Addresses: `0x8062e0`, `0x49b18c`, `0x804710`
Confidence: verified
Proof: one
Test: none

The table is read from the image at `0x8062e0`; 94 entries are in range of the helper's test. The ids that
the table names are those of `tables/events.tsv`. Chat type 89 maps to id 540, which has no event name,
and the chat message handler turns the type into 6 before a line can reach the event.

## Addon messages

A line with language -1 is an addon message. The addon arm fires `CHAT_MSG_ADDON` (id 227) with `%s%s%s%s`:
the text up to its first tab (the prefix), the rest after it (the message), the distribution and the sender.
With no tab the whole text is the prefix and the message is empty. The distribution is `PARTY` for type 1,
`RAID` for 2, `GUILD` for 3 and `BATTLEGROUND` for 92, and `UNKNOWN` for every other type. The arm shows no
line, bubble or sound.

Addresses: `0x49a89e`, `0x49a8a9`, `0x49a8c4`, `0x49a8d0`, `0x49a8d5`, `0x49a8dd`, `0x49a8df`, `0x49a8e7`, `0x49a8e9`, `0x49a8f4`, `0x49a902`, `0x49a90a`, `0x49a911`, `0x49a939`, `0x49a955`, `0x49a95f`, `0x49afe0`, `0x49aff4`
Confidence: verified
Proof: one
Test: none

Every branch of the arm targets below `0x49a970`; its only call is `0x703f50` with id 0xe3 (`0x49a95f`). The
split is a loop from `ebp-0x150c` that, at the first tab with the destination still in the prefix buffer,
ends the prefix (`0x49a8e9`) and moves to the message buffer at `ebp-0x1f0c`; a later tab fails the lower
bound (`0x49a8dd`) and is copied. The prefix buffer ends at `ebp-0xd0c` (`0x49a8e7`), after which tabs are
copied too. The distribution is `remap[type-1]` (92 bytes at `0x49aff4`) into the five-entry jump table
`0x49afe0`; a type of 0 or past 92 is `UNKNOWN`. The arm is not gated by the type.

## Whisper notice on the LCD

A whisper line (type 6) calls `0x72b620(sender)`, a function of the keyboard LCD module that, when the
module is present, shows the sender and arms a timer. Nothing else marks a whisper in the chat line
function.

Addresses: `0x49ac9f`, `0x49aca3`, `0x49aca7`, `0x72b620`, `0x72b64b`, `0x72b650`, `0x72b660`
Confidence: inferred
Proof: none
Test: none

`0x72b620` is entered with the sender when `[ebp-4] == 6` (`0x49ac9f`). It tests the module object and two
calls (`0x72c850`, `0x72a350`), then calls `0x72b4d0(sender, 1)` and `0x72b480(5)` and stores a deadline from the
clock (`0x72b660`). What those callees draw is not read here.

## Pending lines

A queued line is a 0xc8-byte record allocated under the tag `__AUPENDINGCHAT__` and linked into a list
at `0xb4fdc4`: the text copy at `+0x04`, the type at `+0x08`, two GUIDs, the sender name copy, the
language, the channel name (128 bytes), the tag (5 bytes) and the rank. The queue function takes the
type in `ecx`, the text in `edx` and ten more arguments (ret 0x2c). Drains show the records with the
same chat line function, the language unchanged.

Addresses: `0x49cae0`, `0x49cbfa`, `0x49cc00`, `0x49ccc0`, `0x49cfb7`, `0x49cfc1`, `0x49d230`, `0xb4fdc0`, `0xb4fdc4`, `0x64a620`
Confidence: verified
Proof: one
Test: none

`0x49cae0` ends at `0x49cbfa` (`ret 0x2c`); `0x49cc00`, a different function, queues text emotes and ends
at `0x49ccb3`. `0x49ccc0` is the name query callback and the per-record drain; it re-checks the latch
(`0x49ce3c`), reads the stored language from `+0x20` and calls `0x49a870` (`0x49cfb7`, `0x49cfc1`). The bulk drain
`0x49d230` runs at world entry.

## The history ring

The chat line function keeps the last 60 lines of some types in a ring of 0x800-byte slots at
`0xb50580` with a timestamp array at `0xb4e3f8`. A line is kept for types 10 to 13 and 15 to 91 and
is written as `Type: [%d], Channel: [%s], Player Name: [%s], Text: [%s]`. Types 0 to 9, 14 and 92 are not kept.

Addresses: `0x49a750`, `0x49a6c0`, `0x49a761`, `0x49a7cf`, `0x49a7d9`, `0x49a7e4`, `0x49a7ef`, `0x49a810`, `0x49a830`, `0x49a850`, `0xb50580`, `0xb4e3f8`, `0xb6e5d4`, `0xb6e5d8`, `0x49a6dc`, `0x49a6e4`
Confidence: verified
Proof: one
Test: none

`0x49a6c0(type)` is a 93-entry byte table at `0x49a6e4` through a two-entry jump table at `0x49a6dc`
(keep or skip); 10 to 13 and 15 to 91 are 1. The first call clears 60 dwords and 60 slot heads
(`0x49a761` to `0x49a790`); the write index `0xb6e5d4` wraps at 60 (`0x49a7ef`). `0x49a810`, `0x49a830`
and `0x49a850` read a line's presence, address and time.

## The chat log line

With chat logging on (`0xb4fdb4`), the chat line function ends by writing one line to the chat log
writer `0xb50540`. The format depends on the type: 17 (`CHANNEL_LIST`) uses the global `CHAT_CHANNEL_LIST_GET`
with the decorated channel name; 18 and 19 use `CHAT_<token>_NOTICE` for the notice token; 22
(`IGNORED`) and 91 (`FILTERED`) use `CHAT_IGNORED` and `CHAT_FILTERED` with the sender; every other type
uses `CHAT_<TYPE>_GET` with the type's name, the channel as `[channel] ` with the zone suffix after
` - ` replaced by `] `, the language name in brackets when it differs from the default language, and the
text with its escapes removed.

Addresses: `0x49acde`, `0x49aceb`, `0x49acee`, `0x49acf7`, `0x49acfe`, `0x49ad05`, `0x49ad2d`, `0x49ad56`, `0x49adbe`, `0x49ae07`, `0x49ae36`, `0x49aeb5`, `0x49b050`, `0x49b064`, `0x65ace0`, `0xb4fdb4`, `0xb50540`
Confidence: inferred
Proof: none
Test: none

`lea eax,[edi-0x11]; cmp eax,0x4a; ja 0x49ae07` takes a type from 17 to 91 through the 75-byte table
`0x49b064` into the five arms of `0x49b050`; the other types take the default arm. The format keys are
the strings the arms push (`CHAT_IGNORED`, `CHAT_FILTERED`, `CHAT_CHANNEL_LIST_GET`, `CHAT_%s_NOTICE`,
`CHAT_%s_GET`). The default arm's use of the language name and of the text cleaning loop (`0x49af2d` to
`0x49afa7`) is read at the instructions but not followed through its callees.
