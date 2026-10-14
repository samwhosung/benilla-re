# ui: dollar macros in server text

The server sends gossip greetings, quest texts, mail bodies, readable items and creature speech with
`$` macros in them, and the client expands them before display: names, race, class, a gender or rank
branch, a line break, and world state numbers. The expander for spell descriptions is `spelltokens`.

The code is `QuestTextParser.cpp`, `0x506f70` to `0x508a00`: the driver `0x506f70` and the token handler
`0x5070a0`. Both share the cursor global `0xbe0b80` with the spell expander, so neither is re-entrant.

Roots: `0x506f70`, `0x5070a0`

## The driver

`0x506f70(src, dst, size, &guid)` copies `src` to `dst` up to each `$`, expands the token after it through
`0x5070a0`, and returns whether every token was known. The subject of the tokens is resolved once: the
unit object of the GUID (mask 8) and, when there is none, its name cache record. A token the handler
refuses leaves a literal `$` in the output and clears the result, and scanning goes on. Copies are bounded
by the size, and truncation is silent.

Addresses: `0x506f70`, `0x506f8e`, `0x506fd9`, `0x506fe3`, `0x507043`, `0x507097`, `0x64a3c0`, `0x64a760`, `0x84f2d8`, `0xbe0b80`, `0x468460`, `0x55f080`, `0xc0e228`
Confidence: verified
Proof: one
Test: none

The scan loop is `SStrChr(src, '$')` (`dl = 0x24`) stored in the cursor, a literal copy by `SStrPack`
(which over-copies and then cuts at the exact length), the cursor stepped past the `$` and a call to the handler with
the unit and the record; a false result appends `$` (`0x84f2d8`). `SStrPack` is a bounded append. The result is
`flag == 0`; it says nothing of truncation.

## The tokens

After the `$`, digits (`0` to `9`, no sign, no limit) are read into a buffer and consumed. The letter that
follows, in either case, selects the token: `B` a line feed; `N` the subject's name; `R` the race name; `C` the
class name; `G` and `T` a branch `male:female;`; `W` and `E` a world state number from the digits. Any other
letter, or the end of the text, is refused. A letter's own case matters only for `R`, `C` and `T`, whose
lower-case forms give the name in lower case. Digits are used only by `W` and `E`.

Addresses: `0x5070a0`, `0x5070af`, `0x5070ec`, `0x5070f2`, `0x507104`, `0x50710b`, `0x507119`, `0x507132`, `0x5071a3`, `0x5071bb`, `0x50722f`, `0x507247`, `0x507400`, `0x50752f`, `0x507537`, `0x507544`, `0x507551`, `0x507575`, `0x507590`, `0x5075b4`, `0x64b480`, `0x741446`
Confidence: verified
Proof: one
Test: none

The dispatch is `c - 'B'` against 0x35, the byte table `0x5075b4` and nine jump entries `0x507590`; the arms are
`B 0x50752f`, `C 0x5071bb`, `E 0x507544`, `G 0x507400`, `N 0x50710b`, `R 0x507132`, `T 0x507247`, `W 0x507537`. `0x507119` is the
refusal (`xor al,al; ret 8`), `0x507575` the success tail, which steps the cursor over the letter. The digit buffer is a 20-byte stack
slot filled with `SStrCopy` of the digit count plus one, so a digit run of 12 or more overwrites the live size word `[ebp-8]` and one of 16 or more the destination word `[ebp-4]` (both reloaded after the copy, at `0x50734e` and `0x5074e3`); a run of 20 or more overwrites the saved frame. `B`
writes the single byte `0x0a`. The lower-casing is `_strlwr` (`0x741446`) over the whole string.

## Name, race and class

`$N` gives the unit's name (`0x609210`), or the first string of the cache record. `$R` and `$C` give the
ChrRaces name (`+0x44` plus the locale) or the ChrClasses name (`+0x14` plus the locale) of the subject's race
or class, for a unit the byte of `UNIT_FIELD_BYTES_0` and for a cached name the record's field. A subject
that is not a player gives its name instead. With neither a unit nor a record `$N`, `$R`, `$C`, `$G` and `$T` are
refused. An id outside its table reads a null row, which faults.

Addresses: `0x50710b`, `0x507132`, `0x50716b`, `0x507174`, `0x507187`, `0x5071b6`, `0x5071f7`, `0x507200`, `0xc0dee0`, `0xc0dee4`, `0xc0def4`, `0xc0def8`, `0xc0e080`, `0x609210`
Confidence: verified
Proof: one
Test: none

The race id is `[unit+0x110]+0x78` or `[record+0x138]`, the class id `+0x79` or `[record+0x140]`, each bounds-checked against
the id map and maximum of its store (`0xc0dee0`/`0xc0dee4`, `0xc0def4`/`0xc0def8`); the name is copied into 0x20 bytes.
A unit without the player bit takes `0x609210` instead (`0x50716b`, `0x5071f7`). The bounds failure leaves `eax = 0`
and the next load reads near address 0.

## Gender and rank branches

`$G a:b;` gives `a` for a subject of gender 0 and `b` for any other; `$T a:b;` gives the player's rank title
(the global `PVP_RANK_<rank>_<side>`) when it exists and otherwise the same branch. Leading spaces of a branch
are dropped and trailing ones trimmed. Without a `:` and a following `;` nothing is emitted and the text after the
letter stays, so the marker vanishes and the branch text remains. A gender of 2 is read again for the
active player.

Addresses: `0x507400`, `0x50752d`, `0x507247`, `0x50725d`, `0x5072d1`, `0x5073fb`, `0x5efe00`, `0x5ed5b0`, `0x5fa6d0`, `0x703bf0`, `0x8445b4`
Confidence: verified
Proof: one
Test: none

The gender is `[unit+0x110]+0x7a` or `[record+0x13c]`; a value of 2 asks `0x5fa6d0` whether the unit is the active
player and then `0x5ed5b0` for its gender, else is treated as 1. The test is `g == 0` for the first branch. The rank is
`[unit+0xe68]+0x1f` and the side `0x5efe00`; the key is formatted `PVP_RANK_%d_%d` (`0x8445b4`) and looked up with
`0x703bf0`. The branch text is never lower-cased, and the copy writes at the old length plus the branch length
without checking the size again.

## World state numbers

`$<n>W` and `$<n>E` give the world state value of key `n` and of `-n` as decimal text; a missing value is 0. With
no digits the key is read from an uninitialised buffer. The table is the one `SMSG_INIT_WORLD_STATES` (opcode
0x2c2) and `SMSG_UPDATE_WORLD_STATE` (0x2c3) fill.

Addresses: `0x507537`, `0x507544`, `0x507551`, `0x50754f`, `0x4c5810`, `0x4c5870`, `0x48f690`, `0x48f9c4`, `0x48f9f1`, `0x48f515`, `0x48f52f`, `0xb71ec8`, `0xb71ed0`, `0x4c5382`, `0x835154`
Confidence: verified
Proof: one
Test: none

The digits go through `SStrToInt`; `E` negates the key (`neg ecx`, `0x50754f`); the value is formatted with `%d` into
16 bytes. `0x4c5810` is an open hash of three dwords a bucket, bucket `mask & key`, the value at node `+0x18`; the mask `0xb71ed0`
is -1 while empty (`0x4c5382`) and a miss returns 0. The setter `0x4c5870` has two call sites, both in the packet
handler `0x48f690` (`0x48f9c4`, `0x48f9f1`), registered for both opcodes at `0x48f515` to `0x48f52f`.

## Where it runs

Every other caller passes the active player's GUID as the subject: the gossip greeting, quest log texts, the quest
frame, the class trainer greeting, item text and mail text. The chat handler uses it on creature speech and
battleground announcements, with the subject the GUID read from the message: the target GUID after the name for
monster types, the only GUID for battleground types. A refused expansion drops the line when the GUID is 0 or
its name is known, and otherwise queues the raw text until the name arrives; the pending line is expanded
again and shown as it is if that fails.

Addresses: `0x49d9c4`, `0x49dabd`, `0x49cf70`, `0x49d3f8`, `0x49d9c9`, `0x49dac2`, `0x49db1e`, `0x49da2a`, `0x49db31`, `0x49cf82`, `0x49cf36`, `0x49d3c3`, `0x49d5e4`, `0x49d687`, `0x49ccc0`, `0x4af1f6`, `0x4af261`, `0x4d7b69`, `0x4dff89`, `0x4dffae`, `0x4e2274`, `0x4e3666`, `0x4e3744`, `0x500cc9`, `0x500e04`
Confidence: verified
Proof: one
Test: none

There are 14 call sites. The chat sites test the type set with flat compare chains: monster types 11, 12, 13, 26 and
the battleground types 82, 83, 84 in the queued paths, and 90 (`RAID_BOSS_EMOTE`) as well in the wire path. Type 89
never calls the expander and becomes a whisper. The queue stores the same GUID that the expander used.
