# crt: formatted input

How the C runtime's `sscanf` reads text. `_sscanf` wraps its string in a stack `FILE` and runs the
engine `__input`, which walks the format, reads bytes through `_inc`, pushes one back through
`__un_inc` and stores each conversion through the next argument pointer. The client's callers
include the mail invoice reader (`ui/mail`), the video settings (`ui/videoverbs`), the water LOD
setting (`terrain/world`) and the console variable callbacks that read a `%f`. Nothing calls
`_setlocale` (`net/who`), so `__mb_cur_max` is 1, the decimal point is `.` and no byte is a lead
byte: every branch below is the C locale's. The float conversions end in `_fassign`
(`crt/strtod#the-scanf-float-assignment`), which reads the collected text with `___strgtold12`.

Roots: `0x73fb5d`

## The string stream

`sscanf(s, fmt, ...)` (`0x73fb5d`) builds a stack `FILE` with `_ptr` and `_base` at `s`, `_cnt` its
length and `_flag` 0x49 (read, string, own buffer), and answers `__input(stream, fmt, args)`
(`0x744e21`). `_inc` (`0x74587d`) takes the next byte, unsigned, while `--_cnt` is not negative; past
the end `__filbuf` (`0x745946`) answers EOF, −1, and the count keeps falling. `__un_inc` (`0x745897`)
ignores EOF; `_ungetc_lk` (`0x741c1b`) keeps its step back and raises `_cnt` only when the byte there
is the one pushed, so a pushback never changes the string, and at the start with `_cnt` other than 0
it refuses. `__whiteout` (`0x7458ae`) counts and reads until a byte is not a space.

Addresses: `0x73fb5d`, `0x73fb66`, `0x73fb79`, `0x744e21`, `0x74587d`, `0x745946`, `0x745897`, `0x741c1b`, `0x7458ae`, `0x73fc64`
Confidence: verified
Proof: one
Test: `crt::crt_sscanf_fixed_difftest`, `crt::crt_sscanf_random_difftest`

`0x73fb66` stores 0x49 at `[ebp-0x14]`, the pointer at `[ebp-0x20]` and `[ebp-0x18]`, and `0x73fb79`
the length from `0x40d600` at `[ebp-0x1c]`. `0x74587d` is `dec [edx+4]; js` and then
`movzx eax,[ecx]; inc ecx`. `0x745946` returns −1 at `0x745a1d` as soon as the flag has 0x40
(`test al,0x40; jne`), before it touches the buffer. `0x741c1b` tests flag 0x1, compares `_ptr` with
`_base` and `_cnt` with 0, and for flag 0x40 does `dec [esi]; cmp [eax],bl; je`, else `inc eax`
to undo the step. `__whiteout` is `inc [esi]` and `0x74587d` in a loop on `_isspace` (`0x73fc64`).

## The format walk

`__input` keeps `n`, the conversions stored, `matched`, a byte counting those that matched even when
suppressed or `%n`, and `charcount`, the bytes read. A space in the format (9 to 13 or 0x20) reads
one byte, pushes back the first that is not a space and skips the format's next spaces; a byte other
than `%` must equal the next input byte, else it is pushed back and the scan ends; `%` starts a
conversion. The scan ends at the format's end, and when the last byte read was EOF unless `%n`
follows. It answers `n`, or −1 when that byte was EOF with `n` and `matched` 0. A format ending in
a space reads one more byte.

Addresses: `0x744e21`, `0x744e50`, `0x744e7b`, `0x744eb1`, `0x745793`, `0x7457d5`, `0x745802`, `0x745827`, `0x74582d`, `0x745788`
Confidence: verified
Proof: one
Test: `crt::crt_sscanf_fixed_difftest`, `crt::crt_sscanf_random_difftest`

`0x744e50` tests the format byte against the class table's 8 bit and `0x744e7b` decrements
`charcount` before `__whiteout` (`0x744e84`) and `__un_inc` (`0x744e8c`). `0x744eb1` compares with
`%` (0x25); `0x745793` reads and compares, `0x745802` pushes back on a mismatch. `0x7457d5` is
`cmp [ebp-0x14],-1`; with EOF it needs `%` at `0x7457db` and `n` at `0x7457e3` to go on.
`0x745827` ends at `0x74583e` unless EOF; `0x74582d` returns `n`, or −1 when `[ebp-0x15]`, which
`0x745788` raises per conversion, is 0. `[ebp-0x14]` is written only by a read, so a format of no
bytes (`0x744e40`) and a `%n` first leave it as the stack had it; a format ending in a space
reads one more byte (`0x745793`) and a mismatch pushes it back.

## Conversion flags

After `%`, bytes up to the type are flags. Digits build the width, a 0 among them marking it set:
`%0d` fails. `*` suppresses the store. The size byte starts at 1: `l` and `L` add 1 and `h` takes 1,
so `hh` makes it −1. `l` and `w` ask for wide output, `h` narrow. `I64` selects 64 bits; `F` and `N` are skipped; any other byte is the type. The type ORed with 0x20 picks the
conversion: `D` acts as `d`, and `[` and `{` both open a set. `%c`, `%s` and `%[`
read their first byte at once, `%n` none, the rest skip spaces first. A conversion not suppressed takes the next argument pointer.

Addresses: `0x744eba`, `0x744ee8`, `0x744f2a`, `0x744f4a`, `0x744f99`, `0x744fb8`, `0x744fd2`, `0x74500b`, `0x7452ac`, `0x7457f7`
Confidence: verified
Proof: one
Test: `crt::crt_sscanf_fixed_difftest`, `crt::crt_sscanf_random_difftest`

`0x744ee8` reads the next format byte; `0x744f2a` compares 0x4e, 0x2a, 0x46, 0x49, 0x4c, then `h`
(0x68), `l` (0x6c) and `w` (0x77), and `0x744f7c` ends the flags for the rest. `0x744f4a` needs
`6` and `4` after the `I`. `0x744fb8` sets the wide-output byte from `S` or `C` when no `h`, `l`
or `w` came. `0x744fd8` ORs 0x20; `0x744fdb` to `0x745007` pick `0x74587d` or `__whiteout`.
`0x74500b` tests the width set and 0, a set width of 0 ending the scan after that read through `0x7457f7`, which pushes the byte back. The
default case `0x7452ac` compares the raw type byte with the input byte, and on a match undoes the
argument (`0x7452c5`) and the `matched` count.

## Integer conversions

`d`, `i`, `u`, `o`, `x`, `X` and `p` read an optional sign and then digits, in 32 bits that wrap, or
64 with `I64`. `d`, `u` and `i` take decimal digits, `o` the digits below 8, `x` and `p` hex digits.
`i` and `x` also read `0x` or `0X` after the sign; `i` followed by a 0 then not an `x` reads octal.
`p` has no prefix and stores 32 bits. The sign takes one of the width, a `0` and `x` prefix none.
A `-` negates in the width of the store. With no digit the conversion fails. The store is eight
bytes for `I64`, else four unless the size byte is 0, then two.

Addresses: `0x745534`, `0x74506a`, `0x7452f9`, `0x74531b`, `0x745569`, `0x745573`, `0x745682`, `0x7456d0`, `0x745708`, `0x74573e`, `0x745746`, `0x745762`, `0x745778`, `0x745846`, `0x73ff20`, `0x414e60`
Confidence: verified
Proof: one
Test: `crt::crt_sscanf_fixed_difftest`, `crt::crt_sscanf_random_difftest`

`0x745534` is the sign for `d`, `u`, `o` and `p`, where `0x745530` sets the size to 1;
`0x74506a` is `x`, and `i` first sets the type to `d` (`0x745067`). `0x7452f9` and `0x745547`
decrement the width and, at 0 with a width set, mark the scan done. `0x74531b` to `0x745364` read
the prefix: `x` or `X` sets the type to `x` (`0x74534d`), another byte sets the started count
(`0x745342`) and for `x` pushes the byte back (`0x74535a`) and takes `0` again, for `i` makes the
type `o`. The 32-bit loop shifts by 4 (`0x7456fa`), 3 (`0x7456c4`) or multiplies by 10
(`0x7456c9`), and adds `ch − 0x30` (`0x74570f`), `__hextodec` (`0x745846`) having turned a letter
into its value plus 0x30. The 64-bit loop uses `__allshl` and `__allmul` (`0x7455b8`, `0x745619`).
`0x7456c2` stops `o` at 8.

## Float conversions

`e`, `f` and `g` copy the text into a stack buffer and store it through `_fassign`. The width is
clamped to 349 if unset or larger. The sign takes one of the width and is stored for `-` only.
Digits, a `.` and digits follow, each byte taking one of the width; only after a digit may an `e` or
`E` follow, stored as `e`, then a sign, stored for `-`, then digits. The `e` and the sign are
consumed whether or not a digit follows; only the byte after them is pushed back. A scan with no
digit fails. The store is a float when the size byte is 1, else a double.

Addresses: `0x74507f`, `0x7450a0`, `0x7450c9`, `0x7450d0`, `0x745115`, `0x745183`, `0x74518d`, `0x7451c7`, `0x7451e5`, `0x74521b`, `0x74523b`, `0x745251`, `0x82f400`, `0x40b50e`
Confidence: verified
Proof: one
Test: `crt::crt_sscanf_fixed_difftest`, `crt::crt_sscanf_random_difftest`

`0x7450c9` stores 0x15d. The loops test `mov eax,[width]; dec [width]; test eax,eax` before
each byte (`0x7450f7`, `0x745120`, `0x745165`, `0x74519e`), so a width of 0 stops them. `0x7451c7`
takes one width for the exponent's sign and, at 0, sets the width to 0 and leaves the sign as the
byte to push back. `0x745241` to `0x745251` push the buffer, `size − 1` and the argument and call
the slot, whose start-up value `0x409a29` stores is `_fassign` (`0x40b50e`) (`crt/printf#the-conversion-slots`).

## Character sets

`%c`, `%s` and `%[` build a 256-bit set and copy bytes while the set accepts them. `%c` reads
`width` bytes (1 if unset) and accepts every byte, `%s` every byte but 9 to 13 and 0x20. `%[...]`
lists bytes and ranges `a-b`, in either order; a leading `^` complements the set, a `]` first in
it belongs to it, and a `-` first or last is a byte. The width counts bytes. Narrow output stores
the bytes; wide output stores each as an unsigned 16-bit unit. `%s` and `%[` end the output with a
NUL unit; `%c` does not. No byte taken fails. A set the format leaves open reads on past the
format's NUL until a `]`.

Addresses: `0x74525f`, `0x74537e`, `0x7452d0`, `0x745391`, `0x7453b8`, `0x74541d`, `0x745446`, `0x7454b6`, `0x7454ea`, `0x7454f6`, `0x8763c0`, `0x8763c8`, `0x410090`, `0x747b78`
Confidence: verified
Proof: one
Test: `crt::crt_sscanf_fixed_difftest`, `crt::crt_sscanf_random_difftest`

`0x8763c0` holds the set text ` \t-\r]` and `0x8763c8` the empty one `]`; both run with the
complement flag set (`0x74538d`). `0x745391` clears 32 bytes with `_memset` (`0x410090`). `0x7453b8`
reads set bytes until `]` (`0x7453ba`), taking `-` as a range only when a byte came before it and
no `]` follows (`0x7453c3` to `0x7453cc`). The scan at `0x745446` stops at the width (`0x745454`), at
EOF or at a byte with `(bitmap ^ complement) & bit` clear, then pushes it back (`0x7454ea`).
A wide store calls `_mbtowc` (`0x747b78`) with `__mb_cur_max`, which in this locale gives the byte
as the unit (`0x7454b6` to `0x7454d0`).
