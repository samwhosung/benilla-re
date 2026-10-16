# ui: the spam and profanity filters

What the `spamFilter` and `profanityFilter` options do to a line of chat and to the other texts the
profanity filter masks: the two predicates, the five word lists they match, the order they run in, and
the regular expression engine's two oddities. The chat line function that runs them is `chatevents`.

The code is `ChatFrame.cpp`: the masker `0x4a1a60` and the spam predicate `0x4a1ca0`, called from the chat
line function `0x49a870`; the word lists are built by the start-up code `0x6c91a0` and the spam list
message handler `0x49e6e0`.

Roots: `0x4a1a60`, `0x4a1ca0`, `0x49e6e0`

## The two options

`profanityFilter` and `spamFilter` are CVars registered by `0x402ba0` with default `1`. Each has a change
callback that converts the new value with `SStrToInt`, prints `Profanity filter enabled` or `disabled`
(`Spam filter enabled` or `disabled`) to the console and stores the number in a mirror word: `0x843600`
for the profanity filter and `0x843604` for the spam filter. The mirrors are 1 in the file. The
filters read the mirrors, never the CVar records.

Addresses: `0x402ba0`, `0x402e68`, `0x402e8e`, `0x403570`, `0x403594`, `0x4035b0`, `0x4035d4`, `0x843600`, `0x843604`, `0x49aa98`, `0x49aacc`, `0x4a1a66`, `0x5e725a`
Confidence: verified
Proof: one
Test: none

The callbacks differ only in strings and the mirror (`mov ds:0x843600,esi` at `0x403594`, `mov
ds:0x843604,esi` at `0x4035d4`). An image scan finds three references to `0x843600` (the writer, `0x49aa98` in the
chat line function and `0x4a1a66` in the masker itself) and two to `0x843604` (the writer and `0x49aacc`). The
masker therefore tests the option itself, so every one of its callers is gated. `0x5e725a` is a second, by-name
lookup of `profanityFilter` before the guild message of the day.

## Which lines are filtered

The spam filter is armed for every chat type except 91 (`FILTERED`, the type it produces). It is also off
for a game master viewer (`PLAYER_FLAGS` bit 3) and for a line whose tag is `GM`. The profanity filter masks a line only
when its type is one of fourteen: say, party, raid, guild, officer, yell, whisper, whisper reply, emote,
channel, AFK, DND, raid warning and battleground. A game master or a GM-tagged line is still masked.

Addresses: `0x49aa98`, `0x49aab0`, `0x49aabb`, `0x49aac2`, `0x49aac9`, `0x49aacc`, `0x49aad2`, `0x49aad4`, `0x49aad9`, `0x49aae8`, `0x49aaf1`, `0x49aaf7`, `0x49ab03`, `0x49ab06`, `0x49ab0c`, `0x49abb7`, `0x8046d8`
Confidence: verified
Proof: one
Test: none

`cmp edi,0x5b` then `jne 0x49aad6` skips the `xor esi,esi`, so the xor disarms the filter only for type 91. The tag
comparison `SStrCmp(tag, "GM", 0x7fffffff)` returning 0 clears `esi` (`0x49aaf1`). The 14-type table at
`0x8046d8` is walked to bound `0x38` (`0x49aabb`). All three exits of the spam arm (`0x49ab06`, `0x49ab0e`,
`0x49ab2a`) land on the profanity arm's entry `0x49abb7`, so the mask runs for a game master and with the
spam filter off; only `[ebp-8] == 0` skips it.

## The spam predicate

When armed, `0x4a1ca0(text, 0, out)` matches the text against the spam word lists. It does not change the
text. A line that matches is dropped, firing nothing, unless it is a whisper: a whisper with a zero report GUID
is dropped too, and one with a report GUID sends opcode 0x331 with that GUID and fires nothing. Every other type is dropped.

Addresses: `0x4a1ca0`, `0x49ab14`, `0x49ab23`, `0x49ab28`, `0x49ab30`, `0x49ab33`, `0x49ab39`, `0x49ab43`, `0x49ab50`, `0x49ab6e`, `0x49ab78`, `0x49ab87`, `0x49abb4`, `0x49afd7`, `0x418190`, `0x418370`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

`0x4a1ca0` has one call site, `0x49ab23`, with `dl = 0` (`0x49ab1b`) and a stack argument that receives
the matched substring (an uninitialised buffer nothing reads again). `cmp edi,6; jne 0x49afd7` drops every
non-whisper type to the bare epilogue (`0x49afd7`). For a whisper the report GUID at `[ebp+0x28]` and `[ebp+0x2c]`
(`0x49ab39`, `0x49ab3c`) is tested (`0x49ab43`); the packet is `push 0x331` (`0x49ab50`), `0x418190`, the GUID by
`0x418370` and the send `0x5ab630`. A match writes out `{start, end}` byte offsets, the end being past the match.

## Masking

`0x4a1a60(text, useCache)` replaces each match of each profanity pattern with the next character of
`!@#$%^&*`, one byte for one byte of the match. The next-character index `0xb6e5f0` is shared by the whole
process: it starts at 0, advances after every replaced byte and wraps at 8, so the mask continues from
one match, line and caller to the next. All patterns of the list are applied in list order, not in text
order, and every occurrence of each is masked.

Addresses: `0x4a1a60`, `0x4a1a80`, `0x4a1b40`, `0x4a1b55`, `0x4a1b5a`, `0x4a1b70`, `0x4a1b76`, `0x4a1b80`, `0x4a1b86`, `0x4a1b8f`, `0x4a1ba6`, `0x4a1bc4`, `0x806458`, `0xb6e5f0`, `0xce8940`, `0xce8944`
Confidence: verified
Proof: one
Test: none

The loop over patterns steps 0x10 bytes (`0x4a1bc4`) from the list base `0xce8944` for the count `0xce8940`. For
each match `0x71fc70(pattern, subject, 1, &out, 0)` fills `out` (`0x4a1b55`), the length is `end - start`
(`0x4a1b6a`), the byte at `0x806458 + index` is stored (`0x4a1b86`) and the index advances and wraps (`0x4a1b8f`);
the search resumes at the advanced cursor (`0x4a1bb8`). The "something was masked" latch is set before
the length test (`0x4a1b70`) and the function returns it. A zero-length match writes nothing. The chat
line function discards the result and runs the mask after the spam arm, so a dropped line never advances the index.

## The mask cache

With `useCache` set the masker keeps each masked string in a hash table keyed by the
case-folded hash of the original, and on a later call with an equal string copies the stored result back
without matching, so the mask index does not advance. Seven of its thirteen callers pass the flag.

Addresses: `0x4a1a8e`, `0x4a1a9b`, `0x4a1aa9`, `0x4a1aaa`, `0x4a1ac1`, `0x4a1ad4`, `0x4a1b14`, `0x4a1bd4`, `0x4a1bed`, `0x4a1c01`, `0x4a4950`, `0x4a4b90`, `0x64b3f0`
Confidence: inferred
Proof: none
Test: none

The cache head is -1 until the first caching call; `0x4a1a8e` sends every later call to the lookup
at `0x4a1c01`. A call with the flag and no cache entry builds one (`0x4a1ac1` to `0x4a1b1a`) and stores
the masked text with a latch byte (`0x4a1bdd`) after the match loop. The lookup arm itself is not read here.
The spam predicate has a cache of its own that its single caller never reaches.

## Where the mask applies

The masker has thirteen call sites. Chat is one. The others are the mail list subjects
(`0x4ad5b2`), the text of an inbox mail (`0x4af203`, `GetInboxText`), the guild rank name returned by
`GetGuildInfo` (`0x4c9402`), the guild record's name fields (`0x4d09df`, `0x4d09ec`), the guild
roster's message of the day and info text (`0x4d0b71`, `0x4d0b8e`), `GetGuildRosterInfo` (`0x4d12fd`),
`GuildControlGetRankName` (`0x4d1f62`), the text of a book or letter (`0x4e3672`, then `ITEM_TEXT_READY`),
the guild event's message of the day (`0x5e7277`) and the outgoing message of `SendChatMessage`
(`0x508891`). The seven guild and roster sites use the cache.

Addresses: `0x49abc6`, `0x4ad5b2`, `0x4af203`, `0x4c9402`, `0x4d09df`, `0x4d09ec`, `0x4d0b71`, `0x4d0b8e`, `0x4d12fd`, `0x4d1f62`, `0x4e3672`, `0x508891`, `0x5e7277`, `0x5e728d`, `0x49f656`
Confidence: verified
Proof: one
Test: none

An image census of `call` and `jmp` finds those thirteen sites and no stored address of `0x4a1a60`. The
sites that push `dl = 1` are `0x4c9402`, `0x4d09df`, `0x4d09ec`, `0x4d0b71`, `0x4d0b8e`, `0x4d12fd` and `0x4d1f62`.
The guild message of the day, after the by-name option test (`0x5e725a`), masks (`0x5e7277`) and fires `GUILD_MOTD`
(`0x11c`, `0x5e728d`). The send path calls the masker through `0x508680` from `0x49f656`.

## The word lists

The filters match compiled regular expressions held in five arrays of 16-byte entries, each array a
`{capacity, count, base}` triple. The spam filter reads the `SpamMessages` list and then the list pushed by
the server; the masker reads the `ChatProfanity` list; the character name validator reads `NamesReserved`,
`ChatProfanity` and `NamesProfanity`. The four file lists are loaded once at start-up from the DBC files
of those names; the server list is empty until the server sends opcode 0x332. A row that does not
compile is skipped, so a list's count is the number of rows that compiled.

Addresses: `0x6c91a0`, `0x6c9228`, `0x6c9309`, `0x6c93e9`, `0x6c94c8`, `0x6c94cf`, `0x6c94e5`, `0x6c96c0`, `0x6c997e`, `0x71fba0`, `0x402b7f`, `0x49e767`, `0xce8918`, `0xce8924`, `0xce8930`, `0xce893c`, `0xce8948`
Confidence: verified
Proof: one
Test: none

The triples are `0xce8918` (server), `0xce8924` (SpamMessages), `0xce8930` (NamesReserved), `0xce893c`
(ChatProfanity) and `0xce8948` (NamesProfanity). The builders are `0x49e767`, `0x6c94c8`, `0x6c93e9`, `0x6c9309` and
`0x6c9228`, each calling the compiler `0x71fba0`; on failure the slot is not advanced (`0x6c94e5`), and the
warning each file builder formats, such as `Warning: invalid chat profanity filter expression`, goes to
the empty print `0x4420b0` (`0x6c923d`), so nothing reports it. The
loader runs from the start-up step at `0x402b7f`. The names `DBFilesClient\SpamMessages.dbc`,
`ChatProfanity.dbc`, `NamesProfanity.dbc` and `NamesReserved.dbc` are literal, so a locale archive
overrides them by the archive priority and not by a composed name.

## The server's spam list

Opcode 0x332, `0x49e6e0`: `u32 count`, then `count` C strings. Each string is compiled with the
case-insensitive flag into the server list; one that fails to compile is reported in the console and
skipped. A count of 0 empties the list. The strings are read with no length limit into a 0x204-byte stack
buffer.

Addresses: `0x49e6e0`, `0x49e6f3`, `0x49e751`, `0x49e767`, `0x49e77d`, `0x49e793`, `0x49e7a8`, `0x49e7e8`, `0x4986ea`, `0x4191b0`, `0x4420b0`, `0xce891c`, `0xce8920`
Confidence: verified
Proof: one
Test: none

`0x49e6f3` reads the count. The loop `0x49e742` to `0x49e791` reads each string by `GetCString` with the
limit `0x7fffffff` (`0x49e742`) into `[ebp-0x204]` and compiles it into slot `ebx*0x10` of the base `0xce8920`
(`0x49e767`); a nonzero compile result prints through `0x4420b0` (`0x49e77d`). The surviving count is stored at
`0xce891c`.

## Regular expression options

Every pattern is compiled case-insensitively, as UTF-8 with no validity check, and every match is run
with the options `0x2000` (no check) and one match per call. In patterns `\<` and `\>` are both word boundary
assertions, the same as `\b` and neither directional. Case folding is for ASCII only, and a code point
of 256 or above is not a word character.

Addresses: `0x71fba0`, `0x71fba7`, `0x71fbb6`, `0x71fbbb`, `0x71fbcf`, `0x71fc70`, `0x71fc8b`, `0x720250`, `0x7202f3`, `0x721460`, `0x812330`, `0x72509d`
Confidence: verified
Proof: one
Test: none

`0x71fba0` builds the options from the caller's flag byte (`0x2800`, plus 1 for caseless, plus 2 for multi-line,
`0x71fbb6`, `0x71fbbb`) and calls `pcre_compile` (`0x720250`) with the default tables; all five call sites
push 1. `0x71fc70` builds `0x2000` (`0x71fc8b`); its twelve call sites push 1 and 0. The escape table at `0x812330`
is 75 words indexed from `'0'`: `\<` (index 12), `\>` (index 14) and `\b` (index 50) are all -4, which the
matcher runs as opcode 4 (`0x72509d`). `0x7202f3` rejects any option outside `0x7a7f`.
