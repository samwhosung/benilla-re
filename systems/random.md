# random

The client's random-number library and the C runtime's `rand`.

The library is one object file, the functions from `0x44ea80` through `0x4531e0`. Its state is
two dwords, seeded by `0x44ea80` and advanced by `0x4531e0`, which reads the 256-byte table
`0x802700`. In this build only the seed, the advance, the two unit-vector functions and three
static initializers are reached. Everything else in the object file (deviates, shuffles, a
keystream, table hashes, gradient noise) is unreached, with one entry below. The advance is also
inlined 23 times in other systems.

The C runtime's `rand` (`0x7400e5`) is a separate generator, seeded from the clock at startup.
Lua's `math.random` draws from it, and several systems seed the library's state from it.

Roots: `0x44ea80`, `0x4531e0`, `0x44fd00`, `0x44fe20`, and the static initializers `0x44eae0`,
`0x453190`, `0x4531b0`.

## State and seeding

The state is two dwords: an accumulator, and four one-byte lane indices. `0x44ea80` sets the
accumulator to the seed and each lane to four times the seed modulo a prime: byte 3 is
`4·(s mod 47)`, byte 2 `4·(s mod 53)`, byte 1 `4·(s mod 59)`, byte 0 `4·(s mod 61)`.

Addresses: `0x44ea80`
Confidence: verified
Proof: two
Test: `random::srand_seed_state_init`

Four unsigned `div` by `0x2f`, `0x35`, `0x3b`, `0x3d` (`0x44ea92` to `0x44eac1`); the remainders are
packed high byte first, the whole dword shifted left by 2 (`0x44eac8`) and stored at `+4`
(`0x44eacb`); the seed is stored at `+0` (`0x44ea9b`). `ret 4`: `this` in `ecx`, the seed on the
stack. No lane can exceed `0xf0`, so the shift loses nothing. The state's class name,
`NTempest::CRndSeed`, is the community database's name for `0x44ea80`. The test runs `0x44ea80`
whole.

## State advance

`0x4531e0` moves each lane down by a fixed step, adding the lane's modulus back when the result
is negative; reads four dwords of the table at the new byte offsets; XORs them after rotating
three of them; adds the result to the accumulator; stores both dwords and returns the new
accumulator. With `T(n)` the dword at byte offset `n` of `0x802700`:

| lane | byte | step | modulus | rotate left |
|---|---|---|---|---|
| 3 | 3 | 4 | `0xbc` | 1 |
| 2 | 2 | 12 | `0xd4` | 2 |
| 1 | 1 | 24 | `0xec` | 3 |
| 0 | 0 | 28 | `0xf4` | 0 |

`acc += rotl(T(n3),1) ^ rotl(T(n2),2) ^ rotl(T(n1),3) ^ T(n0)`, all modulo 2³².

Addresses: `0x4531e0`, `0x802700`
Confidence: verified
Proof: two
Test: `random::crandom_uint32_advance`

The steps are `sub` then a signed `test`/`jge` that skips the `add` of the modulus (`0x453215`,
`0x453225`, `0x45323a`, `0x453259`): the modulus is added only when the subtraction goes negative,
so a lane above its modulus counts down to it first. The rotates are `shr`/`shl` pairs (31/1 at
`0x45324d`, 29/3 at `0x45326c`, 30/2 at `0x453276`). The lanes are repacked byte 3 first
(`0x45328e` to `0x4532a6`) and stored at `+4`, the sum at `+0`; `eax` returns it.

From a seeded state every lane is a multiple of 4, so each is a counter over dwords: steps 1, 3,
6 and 7 modulo the primes 47, 53, 59 and 61. The lanes repeat together every
47·53·59·61 = 8,965,109 advances.

`0x4531e0` has 83 call sites outside the library. A further 23 copies are inlined in other systems:
a scan for the immediate `0x802700` outside the library finds 92 operands in groups of four, each
group with the lane moduli and the byte-indexed form `[reg+0x802700]`. The control is the library
itself, where the same scan finds the advance and the hashes. The hashes' `[4*reg+0x802700]` form
occurs nowhere outside it. The test runs `0x4531e0` whole.

## Unit vectors

`0x44fd00` advances a state once, takes the accumulator's low 23 bits as the mantissa of a float
`m` in `[1, 2)`, and writes `(cos a, sin a)` as floats, where `a = (m − 1)·2π` rounded to a
float. `0x44fe20` advances twice: the first value gives `z = m − 2` when the accumulator is
non-negative as an `int32` and `z = 2 − m` when it is negative; the second gives the angle `a` as
above. It writes `(cos a · r, sin a · r, z)` with `r = √(1 − z²)` stored as a float. Both take
the output pointer in `ecx` and the state in `edx`.

Addresses: `0x44fd00`, `0x44fe20`
Confidence: verified
Proof: one
Test: `random::rand_44fd00_gradient`, `random::rand_unit_vector_3d_diff`

Each inlines the advance of `0x4531e0` (same steps, moduli and rotates), stores the state back,
and calls nothing. The angle: mantissa `and 0x7fffff`, `or 0x3f800000`, `fsub` 1.0, `fmul` 2π
(`0x40c90fdb`), `fstp dword` (`0x44fdc8` to `0x44fdf5`), then `fsincos`, first store the cosine
(`0x44fe01` to `0x44fe14`). In `0x44fe20`, `z` comes from the sign test at `0x44fef6` and the
radius from `fld z; fmul z; fsubr 1.0; fsqrt` (`0x450002` to `0x450010`). `z` is never 0 and
can be −1 or 1. `rand_44fd00_gradient` runs only `0x44fdc8` to `0x44fe17`, so `0x44fd00`'s
inlined advance has no test of its own; `rand_unit_vector_3d_diff` runs `0x44fe20` whole. Each
function has one caller outside the library.

## Static initializers

Three static initializers of the library run at startup, from slots `0x827530`, `0x827534` and
`0x827538` of the C runtime's initializer table: `0x44eaf0` copies +∞ into `0xb05d68`,
`0x4531a0` stores 2π in `0xb05d70` as `π + π` in floats (`0x40c90fdb`), and `0x4531c0` stores
`1 / 0xb05d70` in `0xb05d6c` (`0x3e22f983`). Each is reached through a thunk, `0x44eae0`,
`0x453190` and `0x4531b0`, a `jmp` padded to 16 bytes.

Addresses: `0x44eae0`, `0x44eaf0`, `0x453190`, `0x4531a0`, `0x4531b0`, `0x4531c0`, `0x827000`
Confidence: verified
Proof: one
Test: none

The table is the 7,180 pointers from `0x827000`, run in ascending order by `__initterm`
(`0x40a404`), which `__cinit` (`0x40a2e0`) calls at `0x40a304`. Only the unreached `0x452ff0`
reads `0xb05d6c` and `0xb05d70`, and nothing reads `0xb05d68`: each has one or two references in
the image, all inside the library.

## The C runtime rand

`rand` (`0x7400e5`) updates a per-thread dword as `s = s·214013 + 2531011` and returns
`(s >> 16) & 0x7fff`. `srand` (`0x7400d8`) stores its argument there. The main thread's state is
seeded at startup, before `WinMain`, with the client clock `0x42c010`; again with the clock
whenever the player enters a PIN at login; and whenever Lua code calls `math.randomseed`. So the
sequence differs on every run. `rand` has 15 call
sites; one is Lua's `math.random` (`0x7fb310`).

Addresses: `0x7400e5`, `0x7400d8`, `0x40acbb`, `0x7fb310`, `0x7fb450`, `0x822d10`, `0x5d1c70`, `0x5b8000`, `0x5b3120`, `0x5ab5a0`, `0x46b0c0`, `0x46d1c0`
Confidence: verified
Proof: one
Test: `random::msvc_rand_lcg_difftest`

Both find the per-thread data through `__getptd` (`0x40acbb`) and use its `+0x14`; `imul`
214013, `add` 2531011 (`0x7400ed`, `0x7400f3`). The Lua math library's table `mathlib`
(`0x822c60`) pairs `random` with `0x7fb310` and `randomseed` with `0x7fb450` (`0x822d10`,
`0x822d18`); `0x7fb310` calls `rand` at `0x7fb319`. The test runs `0x7400ed` to `0x740106`.

`srand` has two call sites: `0x7fb460`, in `0x7fb450`, which converts its first argument with
`__ftol`; and `0x5d1c8b`, in `0x5d1c70`. `0x5d1c70` has two callers. One is `0x5b800b`, in the
static initializer `0x5b8000` (table slot `0x82a9ac`, through the thunk `0x5b7ff0`), which passes
`0x42c010()` as the seed. Static initializers run before `WinMain`: in `_WinMainCRTStartup`
(`0x4098c0`), `__cinit` runs at `0x409965` and `WinMain` is called at `0x40999b`. The other is
`0x5b3163`, in `0x5b3120`, which also passes `0x42c010()`. The glue script function
`PINEntered` (`0x46d1c0`) passes the digits to `0x46b0c0`, whose last instruction, at
`0x46b0e0`, jumps to `0x5ab5a0`; that calls `0x5b3120` when its global object pointer is set.

## Unreached functions

Every other function of the library is unreached: nothing outside it calls, jumps to or stores
the address of any of them, and the reached functions call none of them. Their rows, of class
`unreached`, keep a one-line meaning each, marked `inferred` because no derivation of it is kept.
The data only they use, the Gaussian spares `0xb05d74` and `0xb05d78` and the gradient table
`0x802800`, have rows too.

Addresses: `0x44eb00`, `0x44eb20`, `0x44eb50`, `0x44ec60`, `0x44ec70`, `0x44ed90`, `0x44ef70`, `0x44f090`, `0x44f270`, `0x44f3a0`, `0x44f580`, `0x44f680`, `0x44f790`, `0x44f8a0`, `0x44f9b0`, `0x44fa70`, `0x44fb30`, `0x44fb50`, `0x44fb70`, `0x44fba0`, `0x44fbd0`, `0x44fc00`, `0x44fc30`, `0x44fc60`, `0x44fc90`, `0x44fcc0`, `0x450050`, `0x450090`, `0x4500d0`, `0x4500f0`, `0x450100`, `0x450230`, `0x450240`, `0x450370`, `0x450380`, `0x4504b0`, `0x4505e0`, `0x450710`, `0x450830`, `0x450860`, `0x4508e0`, `0x4509f0`, `0x450b90`, `0x450dc0`, `0x450ee0`, `0x451230`, `0x4516b0`, `0x451730`, `0x4518d0`, `0x451d00`, `0x452190`, `0x452380`, `0x452640`, `0x452960`, `0x452ff0`
Confidence: verified
Proof: one
Test: none

The census is two scans over the whole image: every `e8`/`e9` whose target is one of the
library's functions, and every occurrence of a function's address as a dword in any section.
Outside the library, the first finds calls only to the four reached functions and the second
finds nothing; the control is the 18 calls to `0x4516b0` from inside the library. `0x44ea80`
and `0x4531e0` make no call, `0x44fd00` and `0x44fe20` inline the advance and call nothing, and
the static initializers call nothing.
