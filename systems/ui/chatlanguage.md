# ui: languages in chat

How a spoken language changes the text a character reads: the server sends plain text and a language id,
and the client rewrites each word it does not know into a word of the same length from `LanguageWords`.
The rewrite, the gate that decides whether a character understands a language, the record layouts and the
Lua functions that list languages.

The code is `ChatFrame.cpp`: the rewrite `0x49b560` with its word test `0x49b940` and case stamp
`0x49b8b0`, the call in the chat line function `0x49a870`, the language bindings `0x49fb30`, `0x49fbe0`
and `0x49fcd0`, and the word index built by the chat start-up `0x4982c0`.

Roots: `0x49b560`, `0x49b8b0`, `0x49b940`, `0x49fb30`, `0x49fbe0`, `0x49fcd0`, `0x49b970`

## When a line is rewritten

The chat line function `0x49a870` rewrites a line only when its language id is not 0 and the active
player exists. The id is forced to 0 for chat types 0x08, 0x0a, 0x0d and 0x5a and for a player whose
`PLAYER_FLAGS` has 0x8 (a game master), which also makes the line's language name empty. The rewrite
asks `0x5ec720(player, languageId, &skill)` for the skill and leaves the line alone from skill 300 up.
Below 300 each word is judged alone. A language of -1 never reaches it (it is the addon arm,
`chatevents#addon-messages`).

Addresses: `0x49a870`, `0x49a970`, `0x49a986`, `0x49a9cc`, `0x49a9db`, `0x49a9dd`, `0x49aa52`, `0x49aa56`, `0x49aa5a`, `0x49aa7c`, `0x49a9f0`, `0x49b560`, `0x49b56d`, `0x49b578`, `0x49b590`, `0x49b597`, `0x49b599`, `0x49b59f`, `0x5ec720`, `0x468460`
Confidence: verified
Proof: one
Test: none

`0x49b560(languageId, skill, src, dst, dstSize, keepAngle, keepPunct)` returns a verbatim copy
(`0x64a5a0`, `0x49b5a1`) for id 0 (`0x49b578`), no player (`0x49b597`, a failed `0x468460` lookup with mask
0x10) or skill 300 or more (`cmp esi,0x12c; jb` at `0x49b59f`). The caller checks the id again
(`0x49aa52`). The GM flag is `[[player+0xe68]+8]` bit 3 (`0x49a9cc` to `0x49a9db`), which stores 0
over the language argument (`0x49a9dd`).

## Which words are understood

A word is understood when `SStrHash(word, 0, 0) % 300 < skill`; the hash is case-folded, so a word and
its capitalised forms share one fate, and the fraction understood is skill over 300 for a spread of
words. `SStrHash` starts `s1 = 0x7fed7fed`, `s2 = 0xeeeeeeee`; for each byte c (a to z raised, `/` to
`\`) it computes `s1 = (s1 + s2) ^ (T[c >> 4] - T[c & 15])` and `s2 = s2*33 + c + 3 + s1`, and returns `s1`
or 1 when 0. `T` is the 16-dword table `0x80e4e0`.

Addresses: `0x64af90`, `0x80e4e0`, `0x49b768`, `0x49b782`, `0x49b792`, `0x49b79c`, `0x49b7a1`, `0x49b7a3`, `0x49b7a6`, `0x49b7b7`, `0x64a760`
Confidence: verified
Proof: one
Test: none

The case-folding loop is `0x64afff` to `0x64b045` (`sub ecx,0x20` for 0x61 to 0x7a at `0x64b00e`, `/` to
0x5c at `0x64b016`); the sensitive loop `0x64afd0` to `0x64affb` is the same without them. The
word is copied with at most 0x100 bytes plus a terminator (`0x49b768`, `0x49b782`), hashed with case
sensitivity 0 and seed 0 (`0x49b787`, `0x49b789`), divided by 0x12c (`div ecx` at `0x49b7a1`) and the remainder
compared with the skill (`cmp edx,[ebp-0xc]; jae`). An understood word is appended unchanged
(`0x64a760`, `0x49b7b7`).

## Splitting a line into words

The line is read as UTF-8 (`0x41aab0`) and cut into runs of word characters and runs of anything else.
A code point is a word character when it is A to Z, a to z, 0xc0 to 0xdd except 0xd7, 0xdf, or
0xe0 to 0xff except 0xf7 (`0x6c9c60`), an ASCII digit (`0x6c9c30`), `'`, or above 0xff; the last test also admits the value that marks a
malformed byte. A run of other characters is written as one space on the chat path; with `keepPunct` it
is copied unchanged, and with `keepAngle` a span from `<` to `>` is copied unchanged.

Addresses: `0x41aab0`, `0x49b940`, `0x49b957`, `0x49b95c`, `0x6c9c60`, `0x6c9c30`, `0x49b5c1`, `0x49b5c9`, `0x49b603`, `0x49b687`, `0x49b6ee`, `0x82ee00`
Confidence: verified
Proof: one
Test: none

`0x49b940` is `isLetter || isDigit || cp == 0x27 || cp > 0xff` (`cmp esi,0x27`, `cmp esi,0xff; ja`). A leading,
trailing or inner run of separators therefore becomes one space on the chat path, and a line ending
in punctuation ends in a space. The space is the constant at `0x82ee00`, appended once per run through
a latch re-zeroed at `0x49b5c9` and `0x49b6ee`. The word length is in bytes.

## Choosing the substitute

A word that is not understood is replaced by a `LanguageWords` word of the same byte length in the same
language. The length key is the word's length capped at 18; when the language has no word of that
length the key falls by one until one exists, and a word with no match down to length 1 is dropped from
the line. The candidates of a key are in `LanguageWords` row order and the pick is the hash (computed
above) modulo their count, so the same word always gives the same substitute.

Addresses: `0x49b7c1`, `0x49b7c4`, `0x49b7c6`, `0x49b7cb`, `0x49b7ff`, `0x49b803`, `0x49b808`, `0x49b822`, `0x49b825`, `0x49b82b`, `0x49b87c`, `0x49b885`, `0x49b88b`, `0xb6e598`, `0xb6e5b4`, `0xb6e5bc`, `0x498322`, `0x4983a2`, `0x4983e9`, `0x497be0`
Confidence: verified
Proof: one
Test: none

The index is a chained hash table whose bucket is `mask & languageId` (the mask `0xb6e5bc`, the bucket
array `0xb6e5b4`, 12 bytes a bucket) and whose chain holds one node per language and length: a node has
the language at `+0x00`, the length at `+0x14`, a flag at `+0x18` that must be 0 and a growable array of
row pointers at `+0x1c` (count `+0x20`, data `+0x24`). The chain is walked for the key at
`0x49b7ff` and `0x49b803`; the retry repeats the walk with `edi - 1` (`0x49b82b`) until `edi == 1`
(`0x49b825`). The pick is `div [ecx+0x20]` of the hash (`0x49b885`) and `[+0x8]` of the row is the word. The start-up
`0x4982c0` fills it once from the rows in order, skipping empty words and keying on the byte length
of the word (`0x498322`, `0x4983a2`, `0x4983e9`). The 18 cap and the 0x100 copy cap are separate.

## Stamping the case

`0x49b8b0(dst, source, substitute, dstSize)` appends the substitute to the output one character at a time
for as long as the source word has characters: a character is upper-cased when the source character at
the same position is an upper-case letter and lower-cased otherwise. The output word is as long as the
shorter of the source and the substitute, within the remaining space, so a digit in the source gives a
lower-case letter and a word that fell to a shorter key is as long as that key.

Addresses: `0x49b8b0`, `0x49b8b5`, `0x49b8c6`, `0x49b8e3`, `0x49b8f1`, `0x49b901`, `0x49b90c`, `0x49b91b`, `0x49b926`, `0x73fbbf`, `0x41089b`, `0x40f92b`
Confidence: verified
Proof: one
Test: none

The loop first walks `dst` to its end with a budget of `dstSize - 1` (`0x49b8c6`). `_isupper` (`0x73fbbf`)
of the source character picks `_toupper` (`0x41089b`) or `tolower` (`0x40f92b`) for the substitute's character.
The loop ends at the source's terminator, the budget (`0x49b8e9`) or the substitute's last character
(`0x49b926`).

## LanguageWords and Languages

`LanguageWords` rows are 12 bytes: `+0x00` id, `+0x04` the language id, `+0x08` the word (a string
block offset made a pointer). `Languages` rows are 40 bytes: `+0x00` id, `+0x04` plus 4 times the locale
the name, `+0x24` name flags. Both loaders assert their field count (3 and 10) and record size (0x0c
and 0x28). The rewrite reads only `LanguageWords`; the language names, shown to Lua and in lists,
read only `Languages`.

Addresses: `0xc0db54`, `0xc0db40`, `0x548990`, `0x548a39`, `0x548a6e`, `0x57c900`, `0x57c97e`, `0x548bc0`, `0x548c69`, `0x548c9e`, `0x57ca50`, `0x540579`, `0x540583`
Confidence: verified
Proof: one
Test: none

The loader asserts at `0x548a39` (`cmp eax,3`), `0x548a6e` (`cmp eax,0xc`), `0x548c69` (10) and `0x548c9e` (0x28). `0x57c900`
reads three dwords and makes the third a pointer at `0x57c97e` to `0x57c984`. The index builder reads the language id from
`[row+4]` at `0x498322`. Shipped data has 13 languages (ids 1, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14 and 33) and
1481 words, with a word at every length up to each language's longest, so the drop path is never
taken against it.

## Knowing a language

A character understands a language through a spell. Adding a spell whose first effect is 39
(`SPELL_EFFECT_LANGUAGE`) stores the spell id at `[0xb700ac] + miscValue*4`, where the misc value is the
language id, and fires `LANGUAGE_LIST_CHANGED` (id 258) through the thunk `0x49b970`. `0x5ec720` maps a
language to the skill total of that spell's skill line for the active player only. Removing a
spell clears every table slot holding its id and fires no event. A language the character does not
know has skill 0.

Addresses: `0x4b25b0`, `0x4b2641`, `0x4b264a`, `0x4b2650`, `0x4b2656`, `0x4b2659`, `0x49b970`, `0xb700ac`, `0x5ec720`, `0x5ec733`, `0x5ec754`, `0x5ec762`, `0x5ec771`, `0x4b2c50`, `0x4b2d23`, `0x4b2d30`, `0x4b2d39`, `0x6de040`
Confidence: verified
Proof: one
Test: none

`0x4b2641` tests the Spell record's first effect (`cmp [esi+0xf4],0x27`) and `0x4b2656` is a plain store, so a
later spell for the same language overwrites an earlier one. `0x4b2d30` to `0x4b2d39` scan the whole table and
zero each slot equal to the removed id; they restore nothing. The thunk `0x49b970` is `mov ecx,0x102; jmp 0x703e50`
and has one caller. `0x5ec720` compares the player's GUID with the active player's first (`0x5ec733`
to `0x5ec749`), loads the spell (`0x5ec754`) and finds its ability row with the player's race and class
bytes (`0x5ec762` to `0x5ec771`; the skills are `ui/spellbook#adding-a-spell`, `objectlayer/playercombat#skills`).

## Language functions

`GetNumLaguages()` (the name has the typo) returns the number of `Languages` rows the active player
understands; `GetLanguageByIndex(n)` returns the name, in the client's locale, of the n-th such row
(1-based, in row order); `GetDefaultLanguage()` returns the name of the language whose id is the
`ChrRaces` row field at `+0x20` of the player's race: 7 (Common) for every Alliance race and 1 (Orcish) for
every Horde race in the shipped data.

Addresses: `0x49fb30`, `0x49fbe0`, `0x49fcd0`, `0x5ec890`, `0xc0db44`, `0xc0db48`, `0xc0e080`, `0xc0ded8`
Confidence: verified
Proof: one
Test: none

`0x49fb30` and `0x49fbe0` walk the rows at stride 0x28 and test each with `0x5ec720`; the pushes are
one number and one string (`Name_lang` at the locale index). `0x5ec890` reads the race byte
`[player+0x110]+0x78`, bounds it by the ChrRaces store and returns `row[+0x20]`; `0x49fcd0` looks that
id up in the Languages id map.

## What the rewrite reaches

The rewrite has three callers: every chat line (`0x49aa7c`, `keepAngle` and `keepPunct` 0, buffer 0x800),
an NPC greeting (`0x4e22ab`, `keepPunct` 1) and the text of readable items (`0x4e3856`, both 1, buffer
0x1f40). The text the chat line function builds is the one the event, the chat bubble and the chat
history use, so a script cannot read the plain text of a foreign-language line. The speaker's own words are
sent without being rewritten, and the speaker sees the server's echo like anyone else.

Addresses: `0x49aa7c`, `0x4e22ab`, `0x4e2010`, `0x4e3856`, `0x4e35b0`, `0x49ac74`, `0x49ac9a`, `0x49acd9`, `0x608ac0`, `0x49f1e0`
Confidence: verified
Proof: one
Test: none

An image census of `call` and `jmp` to `0x49b560` finds exactly those three sites (the same pattern finds
the three callers of `0x49b0b0`). `0x49a870` keeps the output in `[ebp-0xd0c]` and passes it to the history
(`0x49ac74`), the event (`0x49ac9a`) and the bubble (`0x49acd9`); the raw pointer is not read after
`0x49a9f6`. `SendChatMessage` sends the packet without calling `0x49a870` for real chat.
