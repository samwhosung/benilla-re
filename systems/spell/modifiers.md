# spell: modifiers

Which of the 29 spell modifier operations the client reads, and where.

The server sets talent and aura modifiers with `SMSG_SET_FLAT_SPELL_MODIFIER` and
`SMSG_SET_PCT_SPELL_MODIFIER`; the handler stores any `(bit, op)` pair in two `int32_t[64][29]`
tables, and one reader sums them for a spell and an operation (`records`, Talent modifier tables and
Modifier sums). The reader is called only by three appliers: the integer one, the single-float one
and the software-float one. This topic lists every use of the appliers, since a client that stores
the tables and applies the wrong operation at a site changes costs, cooldowns, ranges and tooltips.

Roots: `0x6e6af0`, `0x6e6bf0`, `0x6e6c30`

## Modifier ops read

Every use of the tables goes through one of the three appliers, 35 call sites in all, each pushing the
operation as a literal. The client reads 20 of the 29 operations. The table lists them by what they
modify, with the applier's kind and each call site.

| op | modifies | applier and call sites |
|---|---|---|
| 14 | power cost | integer `0x6e32f0` |
| 10 | cast time | integer `0x6e33be`, `0x5ee1fb` |
| 11 | cooldown and category recovery | integer `0x6e2c50`, `0x6e2d40`, `0x6e846f`, `0x6e85c5`, `0x6e85d1`, `0x6e9523`, `0x52eaf4` |
| 21 | global cooldown | integer `0x6e2e33`, `0x6e9569` |
| 1 | duration | integer `0x6ea06d`, `0x51a459` |
| 5 | maximum range | float `0x6e3746`, `0x507d9a` |
| 6 | area radius | float `0x6e643c`, `0x6e6450`; integer `0x507cad` |
| 8, 0, 22, 2, 12, 23, 24, 3 | effect amount | software `0x6e3932`, `0x6e393e`, `0x6e395b`, `0x6e3966`, `0x6e3972`, `0x6e399d`, `0x6e39ad`, `0x6e39bd`, `0x6e39cd`, `0x6e39dd`, `0x6e39e9` |
| 4, 17, 18, 19, 27 | tooltip tokens | integer `0x507ec2`, `0x507e0d`, `0x507ea2`, `0x507e5c`; float `0x507f94` |

Addresses: `0x6e6af0`, `0x6e6bf0`, `0x6e6c30`, `0x6e6b30`, `0x6e32f0`, `0x6e33be`, `0x5ee1fb`, `0x6e2c50`, `0x6e2d40`, `0x6e846f`, `0x6e85c5`, `0x6e85d1`, `0x6e9523`, `0x52eaf4`, `0x6e2e33`, `0x6e9569`, `0x6ea06d`, `0x51a459`, `0x6e3746`, `0x507d9a`, `0x6e643c`, `0x6e6450`, `0x507cad`, `0x6e3932`, `0x6e393e`, `0x6e395b`, `0x6e3966`, `0x6e3972`, `0x6e399d`, `0x6e39ad`, `0x6e39bd`, `0x6e39cd`, `0x6e39dd`, `0x6e39e9`, `0x507ec2`, `0x507e0d`, `0x507ea2`, `0x507e5c`, `0x507f94`, `0x6e9950`
Confidence: verified
Proof: one
Test: none

A scan of the listing for `call 0x6e6af0`, `0x6e6bf0` and `0x6e6c30` finds the 35 sites (none is a `jmp`),
and a scan of the image for each address as a dword finds none; the control is the same dword scan
finding the handler `0x6e9950` twice, at its two registrations. `0x6e6b30` has three callers, one in
each applier (`0x6e6b02`, `0x6e6c02`, `0x6e6c44`). The operation is the last `push` before each call;
the effect-amount arms of `0x6e3800` each push their operation and share the call at `0x6e39e9`
(`records`, Effect amount modifiers and rounding). The tooltip sites are in the spell text expander
`0x507710`: the radius token reads the `SpellRadius.dbc` row's `+0x04` through `0x40a2b0` and applies
op 6 as an integer (`0x507c9b`), the range token reads the `SpellRange.dbc` row's `+0x08` (`0x507d8d`),
and tokens 17, 19 and 27 read the per-effect dwords at `+0x190`, `+0x178` and `+0x184` of the spell
record, token 18 the dword at `+0x64` and token 4 the dword at `+0x68`.

## Modifier ops never read

Operations 7, 9, 13, 15, 16, 20, 25, 26 and 28 have no reader. The handler stores a value for any
operation byte, and a value for these nine sits in its table cell unread; the cell index is still
`bit·29 + op`, so a client that stores the server's modifiers keeps the 29-wide stride for all of them.

Addresses: `0x6e9950`, `0x6e6b30`, `0x6e6af0`, `0x6e6bf0`, `0x6e6c30`
Confidence: verified
Proof: one
Test: none

The reader takes its operation only from the three appliers (above), and every one of their 35 call
sites pushes a literal, so the set of operations that can reach the tables is the set of literals in
the previous entry: `{0, 1, 2, 3, 4, 5, 6, 8, 10, 11, 12, 14, 17, 18, 19, 21, 22, 23, 24, 27}`. The
literals are an exhaustive census of the listing, not of the server's names for the numbers.
