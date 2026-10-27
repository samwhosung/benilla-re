# crt: math routines

How the C runtime answers `frexp`, `fmod` and, where an operand is a NaN or an infinity, `pow`.
The Lua math library's `frexp`, `mod` and `pow` (`lua/libs`) call them, and so does the engine's
angle wrap (`fastmath`, `collision/kinematics`). `__CIfmod` and `__CIpow` are x87 intrinsics: the
dividend or base is the register below the top, the divisor or exponent the top, and the answer
is left in `st0`. `fmod` goes through a dispatcher that classes both operands and jumps through a
table; `pow` tests its operands itself and gives the infinite ones to a helper. Every routine runs
with the exceptions masked, and a domain error returns the indefinite and sets `errno` to
`EDOM` (33). The values of two ordinary finite operands come from `fprem`, `fyl2x` and `f2xm1`,
whose results differ by CPU (`docs/METHOD.md`); `pow` of two finite operands is not read here.

Roots: `0x7fd8db`, `0x73f90a`, `0x73f940`

## Math error flags

The error exits of these routines set `errno` and return the value they hold; none calls a
`_matherr` hook. `0x831ac8` holds 0x2694 and nothing writes it, which is what skips the hook;
`0x883918`, written only by the unreached `0x409a61`, stays 0, so the dispatcher's exit reports
every error. `0x88391c`, set at start-up from `0x40b425`, is the processor's FDIV erratum answer,
1 only on a flawed Pentium, so a current CPU runs `fprem`. `__87except` takes an error type: 1 marks
the masked invalid operation through `__handle_exc` and 7, a quiet NaN, becomes 1 and skips the
marking; `__set_errno` stores 33 for type 1 and 34 for 2 and 3.

Addresses: `0x831ac8`, `0x883918`, `0x88391c`, `0x409a61`, `0x409a6a`, `0x409a1b`, `0x40b425`, `0x747aa8`, `0x747ad9`, `0x747b57`, `0x40cc4b`, `0x40ceea`, `0x40cf37`, `0x40c85b`, `0x40c7a7`, `0x4104e6`, `0x74378a`, `0x743783`
Confidence: verified
Proof: one
Test: `crt::crt_frexp_difftest`, `crt::crt_fmod_difftest`, `crt::crt_pow_nonfinite_difftest`

The scan of every instruction and every dword of the image that names `0x831ac8` finds eight sites,
all reads: `0x40c7aa`, `0x40c806`, `0x40c8a1`, `0x40c945`, `0x7436d2`, `0x74370f`, `0x747b57` and the
loop bound `0x41048e` of `__dosmaperr`; the control is the same scan on `0x883b08`, which finds its
readers. A nonzero value sends `__except1` (`0x40c8a1`) and `__87except` (`0x747b57`) to
`__set_errno` and `__ctrlfp` instead of `_matherr`. `0x883918` has one write, `0x409a6a`, in a
function no call or stored address reaches; `0x74378a` reads it. `0x40b425` asks
`IsProcessorFeaturePresent(0)` and else runs a 4195835/3145727 division test; `0x73f916` reads the
answer. `0x747aa8` subtracts 1 per case from the type (`0x747abc`): type 1 pushes flag 8 for
`__handle_exc` (`0x40cc4b`), which with the invalid mask set marks the status and returns 1; type 7
stores 1 at `0x747ad9`. `0x40cf37` maps flag 8 to 1.

## Splitting a double

`_frexp(x, &e)` (`0x7fd8db`) answers m in [0.5, 1) with the sign of x and stores e with x = m·2^e.
A zero of either sign answers +0 and e 0. A subnormal is shifted left until bit 52 is set, e
starting at −1021 and falling one a step, so m and e are exact. An infinity stores e = −1 and
answers the indefinite, a quiet NaN answers itself, a signalling NaN answers itself with its quiet
bit set; each sets `errno` to 33. The routine runs under the control word 0x133f, masked with 64-bit
precision, and restores the caller's.

Addresses: `0x7fd8db`, `0x7fd8e6`, `0x7fd8fa`, `0x7fd908`, `0x7fd90f`, `0x7fd92f`, `0x7fd939`, `0x7fd945`, `0x7fd95d`, `0x7fd972`, `0x40c6e6`, `0x40c6f8`, `0x40c6fa`, `0x40c738`, `0x40c751`, `0x40c768`, `0x40c796`, `0x40c68c`, `0x40c5ff`, `0x40c7a7`, `0x40c85b`, `0x40cf81`, `0x82f4d8`
Confidence: verified
Proof: one
Test: `crt::crt_frexp_difftest`

`0x7fd8e6` calls `__ctrlfp(0x133f, 0xffff)`. `0x7fd8fa` tests the exponent field; for all ones
`0x7fd908` stores −1 in `*e` and `__sptype` (`0x40c68c`, called at `0x7fd90f`) classes x: 1 and 2
the infinities, 3 any NaN with bit 51 set, 4 the rest. Classes 1 and 2 load `0x82f4d8`, the
indefinite (`0x7fd939`), class 4 `x + 1.0` (`0x7fd945`), and call `__except1(8, 0x17, x, that, cw)`
(`0x7fd95d`), which returns it after `__set_errno`; class 3 returns `__handle_qnan1(0x17, x, cw)`
(`0x7fd92f`), x itself. A finite x goes to `__decomp` (`0x7fd972`): `fcomp 0.0; sahf; jne`
(`0x40c6f8`) sends a zero to `fldz` (`0x40c6fa`); a zero exponent field (`0x40c708`) loops
`shl` of the 64 bits with `dec esi` from `0xfffffc03` (`0x40c738` to `0x40c751`), restoring the
sign; `__set_exp` (`0x40c5ff`, called at `0x40c768`) writes the field 0x3fe, and a normal x stores
the field less 0x3fe (`0x40c796`).

## Remainder dispatch

`__CIfmod` (`0x73f90a`) loads the record `0x875ea0` and enters `__cintrindisp2` (`0x7436c0`), which
sets the control word 0x133f, classes the divisor and the dividend with `fxam` as normal (a double
subnormal included), zero, NaN or infinity, and jumps to the table entry `0x875eb0 + 16·dividend +
4·divisor`. The remainder of two normal operands is `fprem`'s, looped until C2 clears, with the
dividend's sign. The other answers:

| dividend \ divisor | normal | zero | NaN | infinity |
|---|---|---|---|---|
| normal | remainder | indefinite | the NaN | indefinite |
| zero | +0 | indefinite | the NaN | indefinite |
| NaN | the NaN | the NaN | a NaN | the NaN |
| infinity | indefinite | indefinite | the NaN | indefinite |

The indefinite sets `errno` to 33, a NaN does as well; of two NaNs the larger significand answers.

Addresses: `0x73f90a`, `0x7436c0`, `0x743997`, `0x7439c4`, `0x743a21`, `0x875ea0`, `0x875eb0`, `0x8761ec`, `0x73f914`, `0x73f91f`, `0x743a34`, `0x743a38`, `0x743ad6`, `0x743a6d`, `0x743a6f`, `0x743a97`, `0x74378a`, `0x8761d0`
Confidence: verified
Proof: one
Test: `crt::crt_fmod_difftest`

`0x73f90a` is `mov edx,0x875ea0; jmp 0x7436c0`. `0x743997` loads 0x133f (`0x7439ae`) unless the
record's function code is 5 (`0x743997`). `fxam` runs for each operand (`0x7439c4`, `0x7439e2`);
`rol` moves C3, C0, C1, C2 into the low nibble that `xlat` maps through `0x8761ec` to 0, 4, 8 or 12
for normal, zero or denormal, NaN or empty, infinity. `0x743a21` is `jmp [edx+eax+0x10]`. The 16
dwords are `0x73f914` (normal, normal), `0x743a34` (zero over normal), `0x743a6d` and `0x743a6f`
(a NaN), `0x743a97` (two NaNs), and `0x743ad6` for the rest. `0x743a34` pops both and `fldz`
(`0x743a38`); `0x743ad6` pops both and loads the tbyte `0x8761d0`, then sets the error type 1;
the NaN handlers test the quiet bit (`0x743a7b`) and `faddp`. `0x73f91f` is the `fprem`; the zero
remainder it leaves has the dividend's sign, which the emulator's `fprem` does not give.
`0x74378a` hands the type to `__87except`.

## Non-finite powers

`__CIpow` (`0x73f940`) stores the base and the exponent as doubles and enters its second entry,
`0x73f962`, which no row holds. When either is a NaN the answer is that NaN, quiet, from `faddp` of
the two (of two NaNs the larger significand), with `errno` 33. When either is an infinity,
`__powhlp` (`0x744c8c`) answers by the table below, `y` being the exponent; a 1 from it sets
`errno` to 33, the answer kept.

| base | y = +inf | y = −inf | other y |
|---|---|---|---|
| \|x\| > 1 | +inf | +0 | |
| \|x\| < 1, zero | +0 | +inf | |
| \|x\| = 1 | indefinite, 1 | indefinite, 1 | |
| +inf | | | +inf for y > 0, 1 for 0, +0 for y < 0 |
| −inf | | | by the whole-number kind of y: odd gives −inf and −0, else +inf and +0; y = 0 gives 1 |

Addresses: `0x73f940`, `0x73f950`, `0x744c8c`, `0x744cdc`, `0x744d33`, `0x744d49`, `0x744db0`, `0x744dbc`, `0x7458d2`, `0x40a1e6`, `0x82f4d0`, `0x82f4d8`, `0x82f4f0`
Confidence: verified
Proof: one
Test: `crt::crt_pow_nonfinite_difftest`

`0x73f940` is `fxch st(1); fstp [esp]; fst [esp+8]` and `call 0x73f962` (`0x73f950`). The second
entry splits on the exponent field of y, then of x, builds an infinity or NaN for the x87 by hand
(`0x744b45`), and for a NaN reaches `faddp` (second entry `+0xaa`), for an infinity pushes both
doubles and calls `__powhlp(x, y, &out)`. `__powhlp` compares the high and low dwords with
`0x7ff00000` and `0xfff00000` (`0x744cb6`, `0x744cef`); `0x744cdc` loads the indefinite and
returns 1, `0x744d33` and `0x744d49` the infinity `0x82f4d0` and zero, `0x744db0` 1.0 and `0x82f4f0`
is −0. `__d_inttype` (`0x744dbc`) answers 0 for a subnormal or a value `__frnd` (`0x7458d2`) changes,
else 2 when half of it is whole too, else 1; `0x40a1e6` supplies the class.
