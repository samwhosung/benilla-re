# storm: UTF-8 to UTF-16

Storm's conversion of the client's UTF-8 text to the UTF-16 the wide Windows calls take, and the
count that sizes a buffer for it. File, folder and registry names go through the conversion, as do
the name validator and the error box; the count serves the error box and the guild rank name
checks. Neither validates: a lead byte's entry in the table `0x80e780` says how many bytes follow
(0 for `0x00` to `0xbf`, then 1 to 5 from `0xc0`, `0xe0`, `0xf0`, `0xf8` and `0xfc`), and those
bytes are folded in whatever they hold.

Roots: `0x65b350`, `0x65b250`

## Converting UTF-8 to UTF-16

`SUniConvertUTF8to16(dst, cap, src, len, &written, &read)` folds a sequence of `n + 1` bytes as
`v = (v << 6) + byte` modulo 2^32, less the `n`th dword of `0x80e764`. A `v` up to 0xffff is one unit, surrogates and overlong forms too, and 0 ends
the run with result 0; above 0x10ffff it is 0xfffd; between, the pair `0xd800 + ((v − 0x10000) >> 10)`,
`0xdc00 + (v & 0x3ff)`. What does not fit in `cap` returns 1 unstored; a source that runs out returns
−1, a sequence past its end −(n + 1). `written` counts units with the NUL, `read` bytes without
its sequence; a null source or a `len` of 0 returns −1 and sets neither.

Addresses: `0x65b350`, `0x65b365`, `0x65b376`, `0x65b3b3`, `0x65b3c2`, `0x65b3d1`, `0x65b415`, `0x65b41f`, `0x65b42b`, `0x65b43b`, `0x65b445`, `0x65b451`, `0x65b45b`, `0x65b473`, `0x65b48c`, `0x65b492`, `0x65b49e`, `0x65b4a5`, `0x65b4b6`, `0x65b4c4`, `0x65b4d2`, `0x80e780`, `0x80e764`
Confidence: verified
Proof: one
Test: `storm::utf8_to_utf16_whole`

`0x65b365` and `0x65b370` refuse a null source and a zero length (`0x65b4d2`); `0x65b376` turns 0x7fffffff into the end
0xffffffff. `0x65b3b3` reads the lead's entry and `0x65b3c2` compares `lead + n` with the end; the jump table
`0x65b4dc` (`0x65b3d1`) enters the `shl ecx,6; add` chain `n` steps from its end, each byte read by `movzx`, and
`0x65b415` subtracts the dword of `0x80e764`: 0, 0x3080, 0xe2080, 0x3c82080, 0xfa082080, 0x82082080. `0x65b41f` compares the write address with `dst + 2·cap`; `0x65b42b`
takes a value up to 0xffff, `0x65b43b` ends on a stored 0, `0x65b445` stores 0xfffd, and `0x65b451` tests room for the
second unit; `0x65b45b` and `0x65b473` form the pair. `0x65b492`, `0x65b49e` and `0x65b4a5` set −1, −1 − n and 1.
`0x65b4b6` and `0x65b4c4` store the counts. With no end, a lead byte just before the NUL folds the NUL in as a trailing
byte and the run goes on past the string: `c3 00` stores U+0040. The test runs the function whole over a corpus of valid, overlong,
surrogate, five- and six-byte, stray and truncated forms and a fuzz, bounded and unbounded, into destinations of
0 to 4 units and one more than the bytes.

## Counting UTF-16 units

`0x65b250(src, len, &read)` counts, sequence by sequence and the NUL included, what the conversion
would store, with the same table, the same bounds and the same results for a source that runs out,
but folds each byte sign-extended. A lead from `0xc0` is then negative, so a sequence of two to four
bytes (lead `0xc0` to `0xf7`) folds above 0x10ffff, whatever follows the lead, and counts 1, a four-byte
form included, which the conversion stores as two units; for valid UTF-8 the count is the code points
plus one. `read` takes the bytes
consumed, the NUL's included, and 0 for a null or empty source.

Addresses: `0x65b250`, `0x65b258`, `0x65b263`, `0x65b28c`, `0x65b299`, `0x65b2a2`, `0x65b2b0`, `0x65b2da`, `0x65b2e1`, `0x65b2e9`, `0x65b2f0`, `0x65b2fb`, `0x65b303`, `0x65b308`, `0x65b318`, `0x65b326`, `0x435f1a`, `0x435f40`, `0x4d2127`, `0x4d212c`
Confidence: verified
Proof: one
Test: `storm::utf8_to_utf16_len_whole`

The chain at `0x65b2a9` to `0x65b2d9`, entered through `0x65b334` (`0x65b2a2`), reads every byte with `movsx`
(`0x65b2b0`), where the conversion uses `movzx`. `0x65b2e1` counts 1 up to 0xffff (`0x65b2e9`, ending on 0), `0x65b2f0`
1 above 0x10ffff and `0x65b2fb` 2 between. The error box `0x435f00` sizes a stack buffer with the count (`0x435f1a`) and
converts into that many units (`0x435f40`): a text with a character above U+FFFF fills it with no NUL, and the
conversion's 1 is not tested. `GuildControlSaveRank` takes the count less one (`0x4d212c`) and refuses a name outside 1
to 16. The test runs the function whole on the conversion's corpus, bounded and unbounded.
