# collision: overview

The movement physics of the client: the per-unit movement object, the per-frame mover, the movement
events and statuses the client applies and reports, gravity and jumping, and the sweep of a moving
body against terrain, models and liquid.

The system is four object files: Movement.cpp (`0x615730` through `0x61b110`), Movement_C.cpp
(`0x6307a0` through `0x631270`), the resolution layer (`0x631290` through `0x637a60`) and the shared
movement code (`0x7c4830` through `0x7c78b0`). Its state is one `CMovement` per unit, 352 bytes,
which a unit holds at `+0x9a8`, and the movement globals Movement_C.cpp keeps with the object
manager. The world queries it calls belong to other systems.

The docs split by topic: `mover` and `replay` (the per-frame drive and the event replay),
`events` and `eventapply` (the move-event queue), `status` and `handlers` (applying and reporting
a movement status), `kinematics` (position, facing and pitch from speed and time), `falling` and
`commands` (gravity, jumping and the movement commands), `resolve` and `airborne` (the swept world
query and the resolution loops), `walking` (the ground mode), `contact` (the sweep geometry) and
`world` (Movement_C.cpp). This doc holds the object files and their static initializers, which
the C runtime runs before `WinMain`.

Roots: `0x615730`, `0x615ac0`, `0x6185a0`, `0x6185c0`, `0x61abb0`, `0x61abe0`, `0x61ac10`, `0x61ac40`, `0x61ac70`, `0x61ad30`, `0x61ad60`, `0x61ad90`, `0x61ade0`, `0x61ae10`, `0x61ae40`, `0x61ae70`, `0x61aea0`, `0x61aed0`, `0x61af00`, `0x61af30`, `0x61af60`, `0x61af90`, `0x61afc0`, `0x61aff0`, `0x61b020`, `0x61b050`, `0x61b070`, `0x61b0a0`, `0x61b0d0`, `0x61b100`, `0x6307b0`, `0x630c20`, `0x630d20`, `0x630d40`, `0x630da0`, `0x630dd0`, `0x630e00`, `0x630e30`, `0x630e60`, `0x630e90`, `0x630ec0`, `0x630ef0`, `0x630f40`, `0x630f70`, `0x630fa0`, `0x630fd0`, `0x631000`, `0x631030`, `0x631060`, `0x631090`, `0x6310c0`, `0x6310f0`, `0x631120`, `0x631150`, `0x631180`, `0x6311b0`, `0x6311d0`, `0x631200`, `0x631230`, `0x631260`, `0x631290`, `0x6312b0`, `0x631340`, `0x6313d0`, `0x634360`, `0x634380`, `0x637550`, `0x637580`, `0x6375b0`, `0x6375e0`, `0x637610`, `0x637640`, `0x637670`, `0x6376e0`, `0x637730`, `0x637760`, `0x637790`, `0x6377c0`, `0x6377f0`, `0x637820`, `0x637850`, `0x637880`, `0x6378b0`, `0x6378e0`, `0x637910`, `0x637940`, `0x637970`, `0x6379a0`, `0x6379c0`, `0x6379f0`, `0x637a20`, `0x637a50`, `0x7c4830`, `0x7c5dc0`, `0x7c5e40`, `0x7c7430`, `0x7c7460`, `0x7c7490`, `0x7c74c0`, `0x7c74f0`, `0x7c7520`, `0x7c7550`, `0x7c7580`, `0x7c75b0`, `0x7c75e0`, `0x7c7610`, `0x7c7640`, `0x7c7670`, `0x7c76a0`, `0x7c76c0`, `0x7c76f0`, `0x7c7720`, `0x7c7750`, `0x7c7780`, `0x7c77b0`, `0x7c77e0`, `0x7c7810`, `0x7c7840`, `0x7c7870`, `0x7c78a0`

## Object files

collision owns four object files. Movement.cpp runs from `0x615730` through `0x61b110`,
Movement_C.cpp from `0x6307a0` through `0x631270`, the resolution layer from `0x631290` through
`0x637a60`, and the shared movement code from `0x7c4830` through `0x7c78b0`. The two files without a
source path are named here by what they hold. Each file's run of initializers opens with its +∞
(`systems/README`, Object files); the 2π and 1/2π just before the shared file's run are the geometry file's below it, and the +∞
just after opens the next file's.

Addresses: `0x615730`, `0x61b110`, `0x6307a0`, `0x631270`, `0x631290`, `0x637a60`, `0x7c4830`, `0x7c78b0`, `0x616860`, `0x6309b0`, `0x861418`, `0x8642e8`, `0x827000`, `0x613ad0`, `0x6151f0`, `0x61b130`, `0x7c47e0`, `0x7c47f0`, `0x7c4800`, `0x7c4810`, `0x7c78d0`, `0x7c78e0`
Confidence: verified
Proof: one
Test: none

`Movement.cpp` (`0x861418`) is pushed at `0x61686a` in `0x616860`, `Movement_C.cpp` (`0x8642e8`)
at `0x6309c0` in `0x6309b0`. Each file includes the shared header, so its run in `0x827000` opens
with its +∞ (`systems/README`, Object files). Movement.cpp's run is `0x82af2c` to `0x82afa0`: +∞
`0x615ac0`, 2π `0x6185a0` onward, and last `0x615730`, its lowest code. The run before opens at
`0x82aeb8` with Object_C.cpp's +∞ `0x613ad0`, copying 0x80c60c; its 2π `0x6151f0` doubles
0x80c610, the π 4 bytes on, which Object_C.cpp's code reads at 0x614be8, so `0x6151f0` to
`0x615710`, which ends at padding before `0x615730`, are Object_C.cpp's. LootRoll.cpp's +∞
`0x61b130` follows `0x61b110` at once. Movement_C.cpp's run is `0x82b5e0` to `0x82b654`, lowest
`0x6307b0`; `0x6307a0`, just below, is an empty `ret 8` whose one caller is the shared code
(`0x7c5d14`), so the file starts there. The resolution layer's run, `0x82b658` to `0x82b6d4`,
opens with `0x631290`, right after `0x631270`; the next function, `0x637a80`, pushes
`UnitCombatLogXP_C.cpp`. The shared file's run is `0x82dab4` to `0x82db20`. Before it, `0x7c47e0`
and `0x7c4800` (bodies `0x7c47f0`, `0x7c4810`, writing `0xcf5ca8` and `0xcf5ca4`) end the run
opened at `0x82daa8`; after it, `0x7c78d0` (body `0x7c78e0`, writing `0xcf5d30`) opens one.

## Shared header constants

Every one of the four files runs the same constant initializers from shared headers, once: +∞
copied from `0x7f800000`, 2π as π + π (6.2831855, `0x40c90fdb`) and 1/2π (0.15915494,
`0x3e22f983`); −21·(−1/36) = 0.58333331, that times −255 = −148.75,
18·(1/36) = 0.5, that times 256 = 128, 1/4.1666665 = 0.24000001, 10·10 and 0.5/10 = 0.05; and
10², 11.111111², 5.5555553², 10², 5², 10², 30², 120², 20², 10², 0.5², 3², 100², the plain
value 5.0, 50², 100², 40² and 1.5². Each result is rounded to a float and stored in a `.data` float
of the file. Movement.cpp, Movement_C.cpp and the resolution file also run a start-up count,
from `0x61ad90`, `0x630ef0` and `0x6376e0`; the shared file runs none.

Addresses: `0x6185b0`, `0x6185d0`, `0x61abc0`, `0x61abf0`, `0x61ad90`, `0x630ef0`, `0x6376e0`, `0x61ada0`, `0x61add0`, `0x61b060`, `0x615ad0`, `0x409aef`
Confidence: verified
Proof: one
Test: none

Each value is one body of four instructions: `fld; fadd st(0),st; fstp` for 2π (`0x6185b0`), `fld
1.0; fdiv; fstp` for the quotients (`0x6185d0`), `fld; fmul; fstp dword` for the products
(`0x61abc0`, `0x61abf0`), `mov eax,[c]; mov [g],eax` for +∞ (`0x615ad0`) and `mov dword
[g],0x40a00000` for 5.0 (`0x61b060`). 1/2π divides by the 2π its file stored first, and
0.58333331·(−255) multiplies the float stored just before, so both read rounded floats. The count is
one dword shared with other files: `0x61ada0` increments it and, when it becomes 1, tail-calls a
`ret`; the release `0x61add0`, registered with `_atexit` (`0x409aef`), decrements it and tail-calls
another `ret` at 0. The values were read by evaluating each body in
slot order from the image's constants.

## Movement.cpp initializers

Movement.cpp's static initializers store the shared constants, run the start-up count, and make
the empty free list of move-event nodes, whose teardown at exit unlinks any
node still on it.

Addresses: `0x615730`, `0x615740`, `0x615770`, `0x615780`, `0x615ac0`, `0x615ad0`, `0x6185a0`, `0x6185b0`, `0x6185c0`, `0x6185d0`, `0x61abb0`, `0x61abc0`, `0x61abe0`, `0x61abf0`, `0x61ac10`, `0x61ac20`, `0x61ac40`, `0x61ac50`, `0x61ac70`, `0x61ac80`, `0x61ad30`, `0x61ad40`, `0x61ad60`, `0x61ad70`, `0x61ad90`, `0x61ada0`, `0x61adc0`, `0x61add0`, `0x61ade0`, `0x61adf0`, `0x61ae10`, `0x61ae20`, `0x61ae40`, `0x61ae50`, `0x61ae70`, `0x61ae80`, `0x61aea0`, `0x61aeb0`, `0x61aed0`, `0x61aee0`, `0x61af00`, `0x61af10`, `0x61af30`, `0x61af40`, `0x61af60`, `0x61af70`, `0x61af90`, `0x61afa0`, `0x61afc0`, `0x61afd0`, `0x61aff0`, `0x61b000`, `0x61b020`, `0x61b030`, `0x61b050`, `0x61b060`, `0x61b070`, `0x61b080`, `0x61b0a0`, `0x61b0b0`, `0x61b0d0`, `0x61b0e0`, `0x61b100`, `0x61b110`
Confidence: verified
Proof: one
Test: none

Each slot holds a 5-byte thunk that jumps 16 bytes on to its body, or a `call` to a body followed by
a `jmp` to a three-instruction registrar that passes a teardown to `_atexit`. The list head is
`0xc4db88`, the free list the event allocator `0x6175d0` takes nodes from: `0x615740` stores 0 there (the link offset), the head's own address at `0xc4db8c` and
that address with bit 0 set at `0xc4db90`, the empty-list marker. `0x615780` walks the head's links,
unlinking each node until it meets a link with bit 0 set, then empties the head.

## Movement_C.cpp initializers

Movement_C.cpp's static initializers store the shared constants, run the start-up count, and make
one empty intrusive list with link offset 0x208.

Addresses: `0x6307b0`, `0x6307c0`, `0x6307f0`, `0x630c20`, `0x630c30`, `0x630d20`, `0x630d30`, `0x630d40`, `0x630d50`, `0x630da0`, `0x630db0`, `0x630dd0`, `0x630de0`, `0x630e00`, `0x630e10`, `0x630e30`, `0x630e40`, `0x630e60`, `0x630e70`, `0x630e90`, `0x630ea0`, `0x630ec0`, `0x630ed0`, `0x630ef0`, `0x630f00`, `0x630f20`, `0x630f30`, `0x630f40`, `0x630f50`, `0x630f70`, `0x630f80`, `0x630fa0`, `0x630fb0`, `0x630fd0`, `0x630fe0`, `0x631000`, `0x631010`, `0x631030`, `0x631040`, `0x631060`, `0x631070`, `0x631090`, `0x6310a0`, `0x6310c0`, `0x6310d0`, `0x6310f0`, `0x631100`, `0x631120`, `0x631130`, `0x631150`, `0x631160`, `0x631180`, `0x631190`, `0x6311b0`, `0x6311c0`, `0x6311d0`, `0x6311e0`, `0x631200`, `0x631210`, `0x631230`, `0x631240`, `0x631260`, `0x631270`
Confidence: verified
Proof: one
Test: none

`0x6307c0` stores 0x208 at `0xc4e4b4`, the address `0xc4e4b8` there and that address with bit 0 set
at `0xc4e4bc`. `0x6307f0` registers `0x630800`, the list's teardown, with `_atexit`. The thunks and
bodies follow the pattern of the shared header constants.

## Resolution file initializers

The resolution layer's static initializers store the shared constants, from its +∞ `0x631290`,
run the start-up count, and set up three statics the world query fills: two pairs of four-dword records whose fourth
dwords start at 0x100, each released at exit, and a box of six floats starting at 0.

Addresses: `0x631290`, `0x6312a0`, `0x6312b0`, `0x6312c0`, `0x6312f0`, `0x631300`, `0x631340`, `0x631350`, `0x631380`, `0x631390`, `0x6313d0`, `0x6313e0`, `0x631420`, `0x631430`, `0x634360`, `0x634370`, `0x634380`, `0x634390`, `0x637550`, `0x637560`, `0x637580`, `0x637590`, `0x6375b0`, `0x6375c0`, `0x6375e0`, `0x6375f0`, `0x637610`, `0x637620`, `0x637640`, `0x637650`, `0x637670`, `0x637680`, `0x6376e0`, `0x6376f0`, `0x637710`, `0x637720`, `0x637730`, `0x637740`, `0x637760`, `0x637770`, `0x637790`, `0x6377a0`, `0x6377c0`, `0x6377d0`, `0x6377f0`, `0x637800`, `0x637820`, `0x637830`, `0x637850`, `0x637860`, `0x637880`, `0x637890`, `0x6378b0`, `0x6378c0`, `0x6378e0`, `0x6378f0`, `0x637910`, `0x637920`, `0x637940`, `0x637950`, `0x637970`, `0x637980`, `0x6379a0`, `0x6379b0`, `0x6379c0`, `0x6379d0`, `0x6379f0`, `0x637a00`, `0x637a20`, `0x637a30`, `0x637a50`, `0x637a60`
Confidence: verified
Proof: one
Test: none

`0x6312c0` zeroes `0xc4e52c` to `0xc4e548` except `0xc4e538` and `0xc4e548`, which get 0x100;
`0x631350` does the same for `0xc4e564` to `0xc4e580`. Their releases `0x631300` and `0x631390` pass
the third dword of each record, when nonzero, to `SMemFree` (`0x646430`) with the allocation tags
`.?AUCFacet@NTempest@@` and `._K`, the type names of a `NTempest::CFacet` and an `unsigned __int64`.
They are the facet lists the world query fills: `0x6721b0` reads `+0x04` as the count, steps
52 bytes through the buffer at `+0x08` and zeroes `+0x04` and `+0x14` (`0x6721c7` to `0x6721e3`), and
the collision code reads 8-byte ids through `+0x18`. `0x6313e0` zeroes
`0xc4e5a0` to `0xc4e5b4`; its teardown `0x631430` is a bare `ret`.

## Shared file initializers

The shared movement file's static initializers store the shared constants. Of the floats they
write, only the 2π, `0xcf5d00`, is read by the movement code, which wraps angles with it.

Addresses: `0x7c4830`, `0x7c4840`, `0x7c5dc0`, `0x7c5dd0`, `0x7c5e40`, `0x7c5e50`, `0x7c7430`, `0x7c7440`, `0x7c7460`, `0x7c7470`, `0x7c7490`, `0x7c74a0`, `0x7c74c0`, `0x7c74d0`, `0x7c74f0`, `0x7c7500`, `0x7c7520`, `0x7c7530`, `0x7c7550`, `0x7c7560`, `0x7c7580`, `0x7c7590`, `0x7c75b0`, `0x7c75c0`, `0x7c75e0`, `0x7c75f0`, `0x7c7610`, `0x7c7620`, `0x7c7640`, `0x7c7650`, `0x7c7670`, `0x7c7680`, `0x7c76a0`, `0x7c76b0`, `0x7c76c0`, `0x7c76d0`, `0x7c76f0`, `0x7c7700`, `0x7c7720`, `0x7c7730`, `0x7c7750`, `0x7c7760`, `0x7c7780`, `0x7c7790`, `0x7c77b0`, `0x7c77c0`, `0x7c77e0`, `0x7c77f0`, `0x7c7810`, `0x7c7820`, `0x7c7840`, `0x7c7850`, `0x7c7870`, `0x7c7880`, `0x7c78a0`, `0x7c78b0`
Confidence: verified
Proof: one
Test: none

`0x7c4840` copies +∞ into `0xcf5cc0`, `0x7c5dd0` stores 2π into `0xcf5d00` and `0x7c5e50` 1/2π
into `0xcf5cd0`. The squares run from `0x7c7440` and the −21·(−1/36) group from `0x7c7790`. A scan
of the listing for the three addresses as operands finds `0xcf5d00` in 21 `fcom`, `fcomp`, `fsub`,
`fadd` and `fld` operands from `0x7c4b00` to `0x7c6f14`, besides its two initializers, and the
other two only in their one store each.
