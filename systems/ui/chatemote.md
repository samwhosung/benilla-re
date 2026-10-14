# ui: received text emotes

The server's text emote message becomes one chat event line: "X waves at you.", "X waves." or "You wave at Y."
The sentence is a template of the `EmotesText` and `EmotesTextData` tables, picked by who performs the emote,
who it targets and the performer's gender. Sending an emote and its animation are `objectlayer/interaction#text-emotes`;
the voice a received emote plays is `objectlayer/sounds#death-thud-and-emote-sounds`.

Roots: `0x49dbe0`

## The message

Opcode 0x105 is handled by `0x49dbe0` (registered at `0x498645`). The body is `u64 performerGuid`,
`u32 textId` (an `EmotesText` id), `u32 emoteNum`, `u32 nameLength` and the target's name, a pointer into
the packet with no terminator check. The handler always returns 1. It drops the message when the friend list `0xc28168`
holds the performer as ignored (`0x5ae5a0`). Otherwise it asks the name cache for the performer: a cached record
goes to the composer `0x49b200(&guid, textId, name, emoteNum)` unless the display latch `0x8435fc` is set; an
uncached one is queued with `0x49d0d0` as the cache callback.

Addresses: `0x49dbe0`, `0x498645`, `0x4190b0`, `0x418e30`, `0x418eb0`, `0x419ac0`, `0x5ae5a0`, `0x49dc3d`, `0x49dc59`, `0x49dc6e`, `0x49dc7f`, `0x49dc95`, `0x49dcae`, `0x55f080`, `0xc0e228`, `0xc28168`
Confidence: verified
Proof: one
Test: none

`0x4190b0` reads the GUID, `0x418e30` the text id and the number, `0x418eb0` the length and `0x419ac0` the name
with that length (`0x49dc20`). The ignore test is `0x49dc36` and its true branch `0x49dc3d` jumps to the return.
The cache query at `0x49dc59` passes the performer GUID, a pointer to it and the callback; a zero return calls
`0x49cc00(&guid, textId, name, 0, emoteNum)` (`0x49dc6e`); a record with the latch set calls it with held flag 1
(`0x49dc95`); else the composer runs (`0x49dcae`). The composer is called only here and from the callback.

## The pending list

`0x49cc00` appends a 0x20-byte node to a list: `+8` the performer GUID, `+0x10` the text id, `+0x14` a copy of
the target name (`SStrDupA`), `+0x18` the held flag, `+0x1c` the number. When the performer's name arrives the
cache calls `0x49d0d0` with the GUID: each node with flag 0 and that GUID is looked up again. A missing record
frees the node and shows nothing. A record with the latch set marks the node held (flag 1). A record without the
latch runs the composer with the node's fields and frees the node. A held node is never shown: nothing else
composes from the list (the exit routine `0x497960` is not a composer); this callback skips flag 1.

Addresses: `0x49cc00`, `0x49cc17`, `0x49cc99`, `0x49d0d0`, `0x49d126`, `0x49d165`, `0x49d176`, `0x49d18f`, `0x49d1db`, `0x497920`, `0x497960`, `0xb4fe0c`, `0x8435fc`
Confidence: verified
Proof: one
Test: none

The latch is 1 from start-up until the world-enter code has run (`chatevents#the-display-latch`), so an emote
received before then is lost while a chat line received then is kept. The list is a static initialised at
`0x497920` (`0xb4fe10` points at itself, `0xb4fe14` is the empty mark) and unlinked at exit by `0x497960`.
Census: `0xb4fe10` and `0xb4fe14` are named by the initialiser, the exit routine, `0x49cc00` and `0x49d0d0` only;
`0x49d230`, the drain of queued chat lines, does not name them.

## The sentence column

The composer returns without a line when the target name pointer is null (last error 0x57), the active
player object is missing, the performer has no name record, or the text id is negative, past the
`EmotesText` maximum or without a row. The row holds the sixteen `EmotesTextData` ids `EmoteText[0..15]` at
`+0xc`; the column is `gender | untargeted | performerIsMe | targetIsMe`.

| bit | value | set when |
|---|---|---|
| 0 | 1 | the performer is not the active player and the target name equals the player's own name (`SStrCmp`, case-sensitive) |
| 1 | 2 | the performer GUID is the active player's; the target compare is skipped |
| 2 | 4 | the target name is empty |
| 3 | 8 | the performer record's `+0x13c` is 1 |

Addresses: `0x49b200`, `0x49b21f`, `0x49b22f`, `0x49b246`, `0x49b263`, `0x49b282`, `0x49b28c`, `0x49b29a`, `0x49b2a0`, `0x49b2b0`, `0x49b2cf`, `0x49b2db`, `0x49b2f3`, `0x49b2fd`, `0x49b306`, `0x49b311`, `0x49b316`, `0x49b31c`, `0x49b332`, `0xc0dd98`, `0xc0dda0`, `0xc0dda4`, `0x544930`, `0x5449de`, `0x544a16`
Confidence: verified
Proof: one
Test: none

The context is 0 (other performer, other target), 1 (other performer, target is the player), 2 (player
performs), 4 (other performer, no target) or 6 (player performs, no target): bit 0 needs a nonempty target and
the performer test jumps over it. The columns reached are those five, each plus 8 for a gender-1 record. The player's name is `GetUnitName` of the active player (`0x49b2f3`). The
store `0xc0dd98` holds the rows and the row count, then the id map (`0xc0dda0`) and the maximum id (`0xc0dda4`,
-1 empty); the loader `0x544930` accepts 19 fields and a 76-byte record: id, name, the `Emotes` id and the
sixteen text ids.

## The template fallback

A column's text id must be in range `0..max`, have a row in the `EmotesTextData` store `0xc0ddc0` and a nonempty
string for the client's locale (`[row + 4 + 4*locale]`, locale `0xc0e080`). Four tries, in order: the full column;
the column without the gender bit; the context with bit 0 cleared and bit 2 set, with the gender bit; the same
context without gender. When the last fails no line is shown. The formatting arguments do not change
with the fallback: they come from the original performer and target tests.

Addresses: `0x49b332`, `0x49b33a`, `0x49b34f`, `0x49b35e`, `0x49b38a`, `0x49b3c5`, `0x49b3f4`, `0x49b4bd`, `0xc0ddc0`, `0xc0ddc8`, `0xc0ddcc`, `0xc0e080`, `0x5444d0`, `0x544579`, `0x5445ae`
Confidence: verified
Proof: one
Test: none

`0x49b38a` computes `(context & ~1) | 4`, a self-or-other form with no target. `0x49b3c5` repeats the lookup for
that value without the gender bit and `0x49b3f4` rejects an empty string, jumping to `0x49b4bd`. The
`EmotesTextData` loader `0x5444d0` accepts 10 fields and a 40-byte record: the id, eight locale strings and a
flags dword that nothing here reads.

## Filling the sentence

The template is formatted with `SStrPrintf` into a 0x100-byte buffer; the arguments follow the actor, not the
template. A performer other than the player with a target other than the player gets the performer's name
and the target name; a target that is the player gets the performer's name alone; a performer that is the
player gets the target name alone, also when the target is the player's own name. The line is shown by
`0x49b0b0(buf, 9, performerName, 0, 0, 0, 0, 0)`, which fires `CHAT_MSG_TEXT_EMOTE` with the sentence as `arg1`;
with chat logging on it is also written to the chat log as `%s`. The voice call follows with or without a line.

Addresses: `0x49b3fa`, `0x49b412`, `0x49b42b`, `0x49b444`, `0x49b463`, `0x49b495`, `0x49b49a`, `0x49b4b5`, `0x49b2bd`, `0x49b2c0`, `0x49b2e0`, `0x49b30b`, `0x609341`, `0x49b4bd`, `0x49b4d6`, `0x49b4e9`, `0x64a7f0`, `0x65ace0`, `0x468460`, `0x623c80`, `0xb4fdb4`, `0xb50540`
Confidence: verified
Proof: one
Test: none

The flags at `[ebp-0x10]` (performer is not the player) and `[ebp-8]` (target is not the player) are set to 1 at
`0x49b2bd` and `0x49b2c0`; `[ebp-0x10]` is cleared at `0x49b2e0` and `[ebp-8]` only at `0x49b30b`, which the
performer-is-player path skips, so the site with no arguments (`0x49b463`) is never reached. The performer's
name is the record pointer itself: `GetUnitName` returns the record as the name string (`0x609341`). `0x49b4bd` looks up the unit object of the
performer GUID with type mask 8 and, when found, calls `0x623c80(textId, emoteNum)` on it. The composer's early returns
skip that call.

## DoEmote

`DoEmote(token [, target])` raises `Usage: DoEmote("emote"[, "target"])` for a non-string token. It searches the `EmotesText` rows from the last to the first for a name equal to the token without regard
to case; an unknown token does nothing. The target is the current target unless the second argument is a
nonempty string that does not begin with `%`: then it is the best player for that name by `ui/targeting#matching-a-name` (party members first, then the raid, then player objects, by name or shared prefix), and no match leaves a zero target.
It calls `SendTextEmote` for the active player with the row and the target and makes no check of its own on the player's state.

Addresses: `0x49fd30`, `0x49fd5c`, `0x49fd9b`, `0x49fdb4`, `0x49fdc5`, `0x49fdd4`, `0x49fde9`, `0x49fe16`, `0x49fe24`, `0x5ef560`, `0x493aa0`
Confidence: verified
Proof: one
Test: none

`0x49fd5c` loads the row count and the loop at `0x49fd72` steps back through the 0x4c-byte rows (`0x49fd6f imul edi,edi,0x4c`); `0x49fd9b` compares the token with the name at `+4`. `0x49fdb4` loads the target GUID, `0x49fdd4` compares the first character of the second argument with `%` and `0x49fde9` calls `ClosestObjectMatch` with kind mask 0x10, mode 0, exact flag 0 and the largest float as the distance; `0x49fe24` pushes the usage text for a non-string token. `0x49fe16` calls `SendTextEmote`.

