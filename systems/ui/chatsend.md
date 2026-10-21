# ui: sending chat

What `SendChatMessage` and `SendAddonMessage` check and do before a line leaves the client, and what goes
on the wire: the chat type names, the group checks, the AFK and DND commands, the text steps and the
packet. The line a received message becomes is `chatevents`.

The code is `ChatFrame.cpp`: `SendChatMessage` `0x49f1e0` with the type lookup `0x49f7a0` and the language
lookup `0x49f8a0`, and `SendAddonMessage` `0x49f920`. The AFK pair is `CGPlayer_C::SetAFK` `0x5eb740` and
`ClearAFK` `0x5eb830` (`objectlayer/interaction#afk`).

Roots: `0x49f1e0`, `0x49f920`, `0x49f7a0`, `0x49f8a0`

## Arguments of SendChatMessage

`SendChatMessage(text [, type] [, language] [, target])` needs a local player (else it does nothing) and
a string or number text (else it raises `Usage: SendChatMessage(text [,type] [,language] [,targetPlayer])`).
The type is a name looked up without case in a 13-entry table (`SAY` 0, `PARTY` 1, `RAID` 2, `GUILD` 3,
`OFFICER` 4, `YELL` 5, `WHISPER` 6, `EMOTE` 8, `CHANNEL` 14, `AFK` 20, `DND` 21, `RAID_WARNING` 88,
`BATTLEGROUND` 92); an unknown name raises `Unknown chat type`; no type is SAY. An empty text sends nothing
unless the type is AFK or DND. The language is the race's default (`GetDefaultLanguage`) unless
named, an unknown name raising `Unknown language`.

Addresses: `0x49f1e0`, `0x49f208`, `0x49f219`, `0x49f223`, `0x49f230`, `0x49f237`, `0x49f27f`, `0x49f285`, `0x49f28d`, `0x49f29e`, `0x49f2a1`, `0x49f2aa`, `0x49f2d3`, `0x49f2e1`, `0x49f7a0`, `0x49f861`, `0x49f876`, `0x49f8a0`, `0x5ec890`
Confidence: verified
Proof: one
Test: `ui::glue_geom_49f1e0_percent_byte`

The type table is built on the stack by `0x49f7a0` (`0x49f7af` to `0x49f85a`), walked 13 times with
`0x64a4c0` (`cmp esi,0xd`, `0x49f876`). The empty-text test is `0x49f28d` to `0x49f2a1`. Argument 1 is checked
with `lua_isstring`, which accepts numbers. The default language is stored at `0x49f2b8` from
`0x5ec890`.

## Group and channel checks

A whisper needs a non-empty target (else it raises `Whisper message missing target player!`). Party needs a party member (else error message 0x48). Raid, raid
warning and battleground need a raid (error 0x19c), and raid warning also needs the player to be a
leader or assistant (error 0x4b). Raid from the raid leader goes out as type 0x57 (`RAID_LEADER`) and
battleground from the leader as 0x5d. A channel needs a target (else it raises `Channel send missing channel number`) that is the
number of a confirmed joined channel, whose name is sent; any other target sends nothing.

Addresses: `0x49f310`, `0x49f325`, `0x49f343`, `0x49f351`, `0x49f379`, `0x49f383`, `0x49f3a4`, `0x49f3bf`, `0x49f463`, `0x49f48b`, `0x49f4a9`, `0x49f4b2`, `0x49f4cd`, `0x49f4d9`, `0x49f4e0`, `0x49f4ea`, `0x496720`, `0x4bb080`, `0x49be50`, `0xbc6f48`, `0xbc6f4c`, `0xb713e0`, `0xbc75f8`, `0xbc75fc`
Confidence: verified
Proof: one
Test: none

The party test is the first party GUID `0xbc6f48`/`0xbc6f4c` (element 0 of a four-entry array) being non-zero. The
raid test is the raid member count `0xb713e0` (the global `GetNumRaidMembers` returns) being non-zero; the
leader tests compare the player's GUID with `0xbc75f8`/`0xbc75fc`. `0x4bb080(guid)` is the leader-or-assistant
test for raid warning (`0x49f4a9`). The channel arm parses the target with `SStrToInt` and `0x49be50`
(`0x49f4d9`, `0x49f4e0`); a null result returns at `0x49f4ea`. The messages are shown with `0x496720`.

## AFK and DND

A message of type AFK marks the player away through `0x5eb740`, or clears it through `0x5eb830(1)` when
already away and the text is empty; the text defaults to `DEFAULT_AFK_MESSAGE`. DND uses the player's
`PLAYER_FLAGS` bit 2 to decide, prints `MARKED_DND` (`You are now DND: %s.`) or `CLEARED_DND`, writes the
default text `DEFAULT_DND_MESSAGE` back into the message, and then falls into the ordinary send. Any other
type first clears AFK without force: when the AFK mirror `0xb6e5cc` is set and `autoClearAFK` is on,
`ClearAFK(0)` runs before the message.

Addresses: `0x49f3c7`, `0x49f3cd`, `0x49f3d6`, `0x49f3de`, `0x49f3f4`, `0x49f40d`, `0x49f41c`, `0x49f4f3`, `0x49f4fc`, `0x49f512`, `0x49f525`, `0x49f553`, `0x49f563`, `0x49f588`, `0x49f58d`, `0x49f592`, `0x5eb740`, `0x5eb830`, `0xb6e5cc`
Confidence: verified
Proof: one
Test: none

The AFK arm ends the function; both of its legs return 0 values after their own call (`0x49f525`,
`0x49f553`), so the AFK packet is the one `0x5eb740` or `0x5eb830` builds, not the generic one. The AFK
choice reads the mirror (`0x49f4fc`); DND reads `[[player+0xe68]+8]` bit 2 (`test byte [edx+8],4`, `0x49f3f0`).
The DND echo is the chat line function with type 10 (`0x49f588`). There is no DND mirror, so a second `/dnd` typed
before the server's flags update marks again.

## Text steps

The text goes through four steps before the packet. A dead or ghost player may send only whisper, party,
raid, raid leader, raid warning, guild, officer, channel, AFK, DND, battleground and battleground leader
(otherwise error 0xef). The tutorial flag 0x16 is set. The text is copied into a 256-byte buffer, expanding
`%N`, `%T`, `%n` and `%t` to the target's name (the next entries). A drunk player's text is run through the drunk
transform. Then the escape check runs, and a final gate decides whether the player may send at all.

Addresses: `0x49f592`, `0x49f597`, `0x49f5a3`, `0x49f5df`, `0x49f5f7`, `0x49f5fc`, `0x49f604`, `0x49f60f`, `0x49f614`, `0x49f61a`, `0x49f627`, `0x49f648`, `0x49f656`, `0x49f65b`, `0x49f661`, `0x49f6aa`, `0x49f6af`, `0x4b54c0`, `0x605f30`, `0x5083a0`, `0x508680`, `0x494a50`, `0x8029d0`
Confidence: verified
Proof: one
Test: `ui::glue4fd_508680_garble_noise_minus_one`, `ui::glue4fd_508680_garble_threshold_quarter`

The dead test is `0x605f30` (health at most 0, or ghost) and the allowed list is the compare chain `0x49f5a3`
to `0x49f5da`. `0x4b54c0(0x16)` sets tutorial flag 0x16. The expander is `0x5083a0(text, buf, 0x100)`, called
once, from `0x49f60f` (`target token expansion`); the buffer is `[ebp-0x138]` and the result is dropped. The drunk byte is
`[[player+0xe68]+0x1d]`, capped at 100 and multiplied by `0x8029d0` (0.01), and with it non-zero `0x508680` writes the
text the packet sends; its inner steps are not read here. The final gate is `0x494a50(6)` (`0x49f6af`).

## Target token expansion

`0x5083a0(text, buf, size)` copies the C string `text` into `buf` and replaces `%N`, `%T`, `%n` and `%t`, and no
other letter, with the target's name. After any other byte, including a second `%` and the end of the text, the `%`
stays and the scan goes on with the next byte, so `%%t` gives `%` and the name and `100%` is unchanged. Every
append is cut at `size - 1` bytes. It returns 1 when every `%` was followed by one of the four letters, else 0; a
null `text` returns 0 and writes nothing. `SendChatMessage` drops the result.

Addresses: `0x5083a0`, `0x5083bf`, `0x5083d7`, `0x5083dc`, `0x508403`, `0x508411`, `0x508430`, `0x508433`, `0x508443`, `0x508524`, `0x50852c`, `0x5084c3`, `0x5084d0`, `0x5084e6`, `0x5084f0`, `0x508502`, `0x50851b`, `0x49f60f`, `0x49f614`, `0x64a3c0`, `0x64a6f0`, `0x64a760`, `0xbe0b80`, `0x84f3a0`
Confidence: verified
Proof: one
Test: `ui::expand_target_tokens_whole`

`0x5083bf` returns 0 for a null `text` before any store; `0x5083d7` stores the NUL at `buf[0]`. `SStrChr`
(`0x64a3c0`, `dl` = `%`) finds each `%` (`0x5083dc`, `0x5084f0`) and `0xbe0b80` holds the scan's place, which the
file's other expanders share; it ends 0. The byte after the `%` is sign-extended and reduced by `N` (`0x508430`);
an unsigned compare with 0x26 (`0x508433`) sends the rest to `0x5084d0`, and the others index the byte table
`0x50852c` (39 bytes, 0 at the offsets of `N`, `T`, `n` and `t`, else 1) into the dword table `0x508524`
(`0x50844a` for 0, `0x5084d0` for 1). `0x5084d0` appends `%` (`0x84f3a0`), sets the flag `[ebp-4]` and leaves
the scan on the next byte, so a `%` there opens the next token. The result is `sete` of the flag (`0x50851b`);
`0x49f614` writes `ecx` and `0x49f61a` overwrites `al`, so nothing reads it. `SStrLen` and `SStrPack` bound every append (`storm/strings#string-length-and-append`).
The test runs `0x5083a0` whole over every text of five symbols to length six against five sizes, a fuzz and
the vectors above, with the four lookups of the next entry stubbed.

## Target token name

Each token looks the name up anew from the target GUID, `0xb4e2d8` (low dword) and `0xb4e2dc`. A GUID of 0 gives the
text of the global `TARGET_TOKEN_NOT_FOUND`. Otherwise the object manager is asked for a unit with that GUID
(`0x468460`, mask 8) and one found is named by `GetUnitName` (`0x609210`), whose miss text is `UNKNOWNOBJECT`
(`objectlayer/unitnames#unit-names`). With no unit, the name cache `0xc0e228` is asked (`0x55f080`) with no
callback, so it sends no query, and a record found gives its name, at the record's start; a miss gives
`TARGET_TOKEN_NOT_FOUND`. That global is appended as the script text lookup returns it, which is empty when the
global is not a string.

Addresses: `0x50844a`, `0x508450`, `0x508457`, `0x50845c`, `0x50846f`, `0x508481`, `0x508488`, `0x50849f`, `0x5084a4`, `0x5084ae`, `0x5084bc`, `0x5084c1`, `0x468460`, `0x609210`, `0x609324`, `0x55f080`, `0x55f125`, `0x55f241`, `0x703bf0`, `0xb4e2d8`, `0xc0e228`, `0x84f3a4`
Confidence: verified
Proof: one
Test: `ui::expand_target_tokens_whole`

`0x50845c` tests `[0xb4e2d8] | [0xb4e2dc]` and jumps to `0x5084ae`. `0x50846f` is `0x468460` with `ecx` = 8 and the GUID;
a null result goes to `0x508488`, which calls `0x55f080` (`0x50849f`) with `ecx` = `0xc0e228`, the GUID, the owner
cell (zeroed at `0x50848b`) and three zeros for callback, data and flag, and a null result there to `0x5084ae`. `0x508481` is `0x609210` with
its `realm` argument 0. `0x5084bc` is `0x703bf0` with the key `0x84f3a4`, `edx` = -1 and 0, as `0x609324` calls it
for `UNKNOWNOBJECT`; with a null callback `0x55f080` returns null for a miss (`0x55f125`) and for a pending
record (`0x55f241`). The test stubs the four lookups, runs `0x5083a0` whole and checks their registers,
arguments and order for each token, with the name each returns appended.

## Expander buffer overrun

The expander does not append the text before a `%` alone: it appends the whole rest of the text, cut at `size - 1`,
then stores a NUL at `buf[length so far + length of the run before the %]` (`0x50841e`), with no bound. Inside the
buffer that ends the string after the run. From offset 256 it stores one zero byte past `SendChatMessage`'s 256-byte
buffer `[ebp-0x138]`, into its frame, at the offset the sum names. Names lengthen the text, so a short text of `%t`
tokens fills the buffer, and then any run of 9 bytes or more before a `%` reaches offset 264.

| offset | slot | a zero byte there |
|---|---|---|
| 256 to 263 | `[ebp-0x38]`, `[ebp-0x34]` | nothing: not read after the call |
| 264 | `lua_State` pointer `[ebp-0x30]` | the escape error raises on a changed state (`0x49f760`) |
| 268 | language `[ebp-0x2c]` | the packet's language loses its low byte (`0x49f6f2`) |
| 272 to 295 | packet buffer `[ebp-0x28]` to `[ebp-0x11]` | nothing: written after the call (`0x49f6c8`) |
| 296 | target pointer `[ebp-0x10]` | a whisper or channel target pointer changes (`0x49f70b`) |
| 300 to 307 | `[ebp-0xc]`, `[ebp-8]` | nothing: not read after the call |
| 308 | chat type `[ebp-4]` | a whisper goes out as say, with no target (`0x49f6e6`, `0x49f6fe`) |
| 312 | the caller's saved `ebp` | every exit restores a changed `ebp` |
| 316 | return address copy `[ebp+4]` | nothing: not read |
| 320 to 331 | saved `ebx`, then the return address | at 320 or 324, and 324 or 328: `ebx` or the return changes |

Addresses: `0x5083a0`, `0x508403`, `0x508411`, `0x50841e`, `0x49f1e3`, `0x49f1e9`, `0x49f604`, `0x49f609`, `0x49f60f`, `0x49f6c8`, `0x49f6e6`, `0x49f6f2`, `0x49f6fe`, `0x49f70b`, `0x49f751`, `0x49f760`, `0x49f775`, `0x49f78c`, `0x64a760`
Confidence: verified
Proof: one
Test: `ui::expand_target_tokens_whole`

`0x508403` is `SStrLen` of the buffer and `0x508411` the append of the rest of the text from the run's start; `0x50841e`
stores 0 at `[ecx+esi]` with `ecx` the sum of the two. The buffer is `lea edx,[ebp-0x138]` with 0x100 pushed
(`0x49f609`, `0x49f604`). The prologue (`0x49f1e3` to `0x49f1e9`) aligns `esp` to 8, which puts `ebp` 8 or 12 bytes
below `ebx`, hence the two readings of the last row; the exits after the call, `0x49f751`, `0x49f775` and `0x49f78c`,
pop `ebp` and `ebx` and return through `ebx`. The test compares the whole 0x800-byte region after each call, with
runs of 1 to 140 bytes before a `%t` at buffer lengths 0, 5, 35, 165 and 255, and of 240 to 300 at length 0, so a
store at every offset from 256 to 395 is checked against the model; the frame is not emulated.

## The escape check

Text sent by `SendChatMessage` and `SendAddonMessage` is checked for `|` escapes. A `|` must be followed
by `|` (a literal pipe) or by `c`; any other follow-up, including a lone trailing `|` and a bare `|r`, raises
`Invalid escape code in chat message`. A `|c` skips to the first `|r` after it, case-sensitively, and nothing
between is examined; with no closer the text is cut at the `|c` and still sent, without an error.

Addresses: `0x49f661`, `0x49f665`, `0x49f670`, `0x49f6a8`, `0x49f758`, `0x49f760`, `0x49f9bb`, `0x49f9c8`, `0x49f9d0`, `0x49f9d7`, `0x49f9dc`, `0x49f9e4`, `0x49f9f4`, `0x49fa04`, `0x49fa6a`, `0x49fa6f`, `0x64a3f0`, `0x64b4f0`
Confidence: verified
Proof: one
Test: none

The scan is duplicated in the two functions (`0x49f661` to `0x49f6a8` and `0x49f9bb` to `0x49fa04`). It finds a `|` with `0x64a3f0`, which does
not match the terminator; `||` resumes after both pipes (`add esi,2`, `0x49f9d7`), so an odd run of pipes ends
in the error; `|c` searches for `|r` with the case-sensitive `0x64b4f0` (`0x49f9e4`) and resumes at the closer plus 2
(`0x49f9f4`). A missing closer stores 0 over the `|` (`0x49fa6a`) and continues. The buffer is 256 bytes in
`SendChatMessage` and, in `SendAddonMessage`, the composed `prefix\tmessage` (2048 bytes).

## The chat packet

The packet is opcode 0x95 (`CMSG_MESSAGECHAT`) as a u32, the type as u32, the language as u32, a C string of
the target for whisper and channel types only, and the C string of the text. The AFK commands build their
own: type 20, language 0 and the message (an empty string to clear).

Addresses: `0x49f6c3`, `0x49f6d7`, `0x49f6e1`, `0x49f6ed`, `0x49f6f9`, `0x49f701`, `0x49f712`, `0x49f71b`, `0x49f72a`, `0x418190`, `0x4181f0`, `0x418430`, `0x5ab630`, `0x5eb7da`, `0x5eb7e4`, `0x5eb7ed`, `0x5eb7f6`, `0x5eb8a5`, `0x5eb8c5`
Confidence: verified
Proof: one
Test: none

`0x418190` and `0x4181f0` are byte-identical `PutUInt32` bodies and `0x418430` writes a C string with its
terminator. The target is written for types 6 and 14 (`cmp eax,6`, `cmp eax,0xe`, `0x49f701`, `0x49f706`). The
packet is sent by `0x5ab630`, which drops it unless the world connection is in state 6. The language
field of the AFK packets is 0 (`0x5eb7ed`, `0x5eb8b8`).

## SendAddonMessage

`SendAddonMessage(prefix, message [, type])` sends nothing without a local player. With both strings empty it
raises `Usage: SendAddonMessage("prefix", "message" [,"type"])`. The text is `prefix`, a tab and `message`
(2047 characters at most); after the escape check the type must be `PARTY`, `RAID`, `GUILD` or `BATTLEGROUND` (no
type is `PARTY`), else it raises `Unknown addon chat type`. `RAID` with no raid becomes `PARTY`, and `PARTY`
with nobody in a party sends nothing without an error. The packet is opcode 0x95, the type, the language
0xffffffff and the text.

Addresses: `0x49f920`, `0x49f92c`, `0x49f94b`, `0x49f97f`, `0x49f984`, `0x49f9a2`, `0x49f9ad`, `0x49fa06`, `0x49fa0b`, `0x49fa1c`, `0x49fa3f`, `0x49fa53`, `0x49fa8b`, `0x49fa9f`, `0x49faab`, `0x49fab9`, `0x49facf`, `0x49fad8`, `0x49fae1`, `0x49faf0`, `0x49faff`, `0x49fb21`
Confidence: verified
Proof: one
Test: none

The compose is `_snprintf(buf, 0x800, "%s\t%s", prefix, message)` (`0x49f9a2` to `0x49f9b3`); a tab inside the prefix is
not removed. The whitelist compares the resolved type with 1, 2, 3 and 0x5c (`0x49fa3f` to `0x49fa4e`) and raises
at `0x49fa53`; a third argument that is not a string or number skips the whitelist and keeps PARTY
(`0x49fa1c`). The raid downgrade is `0x49fa8b`. The language is `or ebx,-1` (`0x49fab9`). No recipient is sent,
which is why an addon message cannot be a whisper.
