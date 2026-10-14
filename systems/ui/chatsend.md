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
Test: none

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
`%t` (and `%n`) to the target's name. A drunk player's text is run through the drunk transform. Then the
escape check runs, and a final gate decides whether the player may send at all.

Addresses: `0x49f592`, `0x49f597`, `0x49f5a3`, `0x49f5df`, `0x49f5f7`, `0x49f5fc`, `0x49f604`, `0x49f60f`, `0x49f614`, `0x49f61a`, `0x49f627`, `0x49f648`, `0x49f656`, `0x49f65b`, `0x49f661`, `0x49f6aa`, `0x49f6af`, `0x4b54c0`, `0x605f30`, `0x5083a0`, `0x508680`, `0x494a50`, `0x8029d0`
Confidence: verified
Proof: one
Test: none

The dead test is `0x605f30` (health at most 0, or ghost) and the allowed list is the compare chain `0x49f5a3`
to `0x49f5da`. `0x4b54c0(0x16)` sets tutorial flag 0x16. `0x5083a0(text, buf, 0x100)` is the `%` expander of
`QuestTextParser.cpp`: it finds each `%`, and for `T`, `t`, `N` or `n` appends the name of the object whose GUID is
in `0xb4e2d8` (the target), or the global `TARGET_TOKEN_NOT_FOUND`; any other letter keeps the `%`. The drunk byte is
`[[player+0xe68]+0x1d]`, capped at 100 and multiplied by `0x8029d0` (0.01), and with it non-zero `0x508680` writes the
text the packet sends; its inner steps are not read here. The final gate is `0x494a50(6)` (`0x49f6af`).

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
