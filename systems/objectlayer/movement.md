# objectlayer: movement

The movement records on the wire: the movement status every movement packet carries, and the
movement block of an object update, which is a status, six speeds and an optional spline.

The codec lives in the object file of the base object's shared field code, `0x47e600` to
`0x47f2e0`, with the heading helper `0x47f220`. The status reader `0x47eba0` and writer `0x47ecb0`
serve the movement packets the physics handles (`collision`); the block reader `0x47f050` and its
skip `0x47ede0` serve the update packet (`update`). The structure they fill is the one the
movement object keeps per unit: the status from `+0x00`, the speeds from `+0x50` and the spline
from `+0x68`. Bit names given below for the flags are the server's; the bit values are the
client's.

Roots: `0x47eba0`, `0x47ecb0`, `0x47f050`, `0x47ede0`, `0x47f220`

## Movement status

A status is, in order: u32 flags, u32 time, four floats position x, y, z and facing; with flag
0x2000000 (on transport) a full u64 transport GUID and four floats, the offset x, y, z and
facing on it; with 0x200000 (swimming) a float pitch; a u32 fall time; with `0x2000` (falling)
four floats of the jump; with 0x4000000 (spline elevation) a float. The reader zeroes the transport GUID and the
pitch when absent and leaves the falling and elevation fields as they were.

| offset | field |
|---|---|
| `+0x00` | time |
| `+0x08` | transport GUID |
| `+0x10` | flags |
| `+0x14` | transport offset, four floats |
| `+0x24` | position, four floats |
| `+0x34` | pitch |
| `+0x38` | fall time |
| `+0x3c` | jump, four floats |
| `+0x4c` | spline elevation |

Addresses: `0x47eba0`, `0x47ebab`, `0x47ebb3`, `0x47ebe6`, `0x47ebf5`, `0x47ec2c`, `0x47ec34`, `0x47ec4a`, `0x47ec53`, `0x47ec5b`, `0x47ec8e`, `0x47ecb0`, `0x4190b0`, `0x418fb0`
Confidence: verified
Proof: one
Test: `objectlayer::movement_block_encode_difftest`

`0x47eba0(ecx store, edx status)` reads the flags into `+0x10` (`0x47ebab`), the time into `+0x00`
(`0x47ebb3`), then floats into `+0x24` to `+0x30`. `test [esi+0x10],0x2000000` (`0x47ebe6`) gates
`0x4190b0`, 8 bytes, into `+0x08` and floats into `+0x14` to `+0x20`; the other branch stores 0 at
`+0x08` and `+0x0c` (`0x47ec2c`). `0x47ec34` gates the pitch, `0x47ec4a` storing 0 without it;
`0x47ec53` reads the fall time; `test ah,0x20` (`0x47ec5b`) gates `+0x3c` to `+0x48`; `0x47ec8e`
gates `+0x4c`. It returns the store. `0x47ecb0` writes the same fields in the same order under the
same tests.

## Movement block

An update's movement block is a status, six floats of speeds into `+0x50` to `+0x64`, and, with
flag 0x400000 (spline), a spline: u32 spline flags; with 0x10000 three floats, a point; with
0x20000 a u64 GUID; with 0x40000 a float, an angle; an i32 elapsed time, a u32 duration, a
u32 id, a u32 node count, that many nodes of three floats, and three floats, the destination. The
client stores the spline's start as the clock now minus the elapsed time.

| offset | field |
|---|---|
| `+0x50` | six speeds |
| `+0x68` | spline target GUID |
| `+0x70` | spline point |
| `+0x7c` | spline angle |
| `+0x80` | spline flags |
| `+0x84` | spline start time |
| `+0x8c` | duration |
| `+0x90` | spline id |
| `+0x94` | nodes |
| `+0xc0` | destination |

Addresses: `0x47f050`, `0x47f05c`, `0x47f0a3`, `0x47f0ba`, `0x47f0bf`, `0x47f0e8`, `0x47f0f9`, `0x47f10d`, `0x47f11a`, `0x47f128`, `0x47f148`, `0x47f161`, `0x47f16e`, `0x47f179`, `0x453360`, `0x42c010`, `0x418e30`
Confidence: verified
Proof: one
Test: `objectlayer::movement_block_decode_difftest`

`0x47f05c` calls `0x47eba0`; six `0x419130` reads fill `+0x50` to `+0x64`. `0x47f0a3` tests
0x400000 in the status flags; `esi` becomes `+0x68`. `0x47f0ba` reads the spline flags into
`+0x80`; `0x47f0bf` gates three floats into `+0x70`, `0x47f0e8` the GUID into `+0x68`, `0x47f0f9`
a float into `+0x7c`. `0x47f10d` reads the clock and `0x47f11a` a signed dword, and `sub` stores
the difference at `+0x84` (`0x47f128`). Two u32s go to `+0x8c` and `+0x90`; `0x47f148` reads the
count, `0x47f161` takes `12·count` bytes and `0x47f16e` copies them as nodes into `+0x94`; the
last three floats go to `+0xc0` (`0x47f179`).

## Skipping a movement block

The skip reads only the flag words and advances past the rest: past the status by its flags,
six speeds, and with a spline its flags, 12, 8 and 4 bytes for its three optional fields, 12 for
the times and id, then the node count and `12·(count + 1)` bytes.

Addresses: `0x47ede0`, `0x47edea`, `0x47edf9`, `0x47edfe`, `0x47ee11`, `0x47ee48`, `0x47ee5e`, `0x47eb10`, `0x47eb2d`, `0x47eb6a`, `0x419ac0`
Confidence: verified
Proof: one
Test: `objectlayer::movement_block_skip_difftest`, `objectlayer::movement_info_size_difftest`

`0x47eb10` reads the status flags and advances by `0x14`, or `0x2c` with 0x2000000, plus 4 with
0x200000, plus 4 for the fall time, plus `0x10` with `0x2000`, plus 4 with 0x4000000
(`0x47eb2d` to `0x47eb6a`), and returns the flags. `0x47ede0` calls it (`0x47edea`), skips `0x18`
(`0x47edf9`), and with 0x400000 (`0x47edfe`) reads the spline flags (`0x47ee11`), adds `0xc`,
8 and 4 for bits 0x10000, 0x20000 and 0x40000, plus `0xc`, and skips that (`0x47ee48`);
then it reads the count and skips `(3·count + 3)·4` (`0x47ee5e`). `0x419ac0` advances the cursor
by its length.

## Heading between two points

`0x47f220(a, b)` returns the heading from `a` to `b` in the x-y plane. With `dx = b.x − a.x`
rounded to a float and `dy = b.y − a.y` kept at the client's 53-bit precision: when `|dx|` is
below 2^−22 it returns 1.5π for `dy < 0` and 0.5π otherwise; else when `|dy|` is below 2^−22 it
returns π for `b.x < a.x` and 0 otherwise; else `atan2(dy, dx)`, in (−π, π]. A NaN `dx` or `dy`
takes the else branches. The products 1.5π and 0.5π use π as the float `0x40490fdb`.

Addresses: `0x47f220`, `0x47f22a`, `0x47f238`, `0x47f243`, `0x47f245`, `0x47f272`, `0x47f287`, `0x47f2a7`
Confidence: verified
Proof: one
Test: `ui::geo_47f220`

`0x47f22a` stores `dx` as a float; `dy` stays in `st0`. `fcomp` of `|dx|` with `0x34800000`
(2^−22) at `0x47f238`, then `test ah,0x5; jp` (`0x47f243`): less gives C0 alone, odd parity, no
jump; greater, equal or unordered jump. `0x47f245` compares `dy` with 0 the same way: less
returns `1.5·π`, else `0.5·π`. `0x47f272` repeats the test on `|dy|`; less compares `b.x` with
`a.x` (`0x47f287`), less returning π, else 0. Otherwise `fld dx; fpatan` (`0x47f2a7`), `st1/st0`
with `st1 = dy`. The result returns in `st0`; `ecx` is `a` and `edx` is `b`.

`ui::geo_47f220` runs `0x47f220` whole and compares its `st0` result.
