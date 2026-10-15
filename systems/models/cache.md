# models: the cache

The model cache: one static object that owns every shared model, the model shaders, the helper
thread that animates half of a scene, and the release list that keeps an unused model in memory
for a while.

The cache is a static of M2Cache.cpp at `0xceefb0`, built and destroyed by the C runtime. The client
starts it once with a feature mask (`0x706dd0`, from `0x4029e0`) and destroys it at shutdown
(`0x706de0`). A scene is created through it (`0x706e10`) and a scene's model loads ask it for the
shared model of a file (`0x706a50`). The shared model itself is `shared`; the scene that asks is
`scene`.

Roots: `0x706d10`, `0x706dd0`, `0x706de0`, `0x706e00`, `0x706e10`

## Cache object

The cache is a 0x1038-byte static. Its constructor zeroes the initialized flag, the 1021 bucket
heads and the release list, and creates the two thread events; its destructor logs
`Model2: M2Destroy never called` when the flag is still set, then closes the thread and the events.

| offset | holds |
|---|---|
| `+0x000` | initialized flag |
| `+0x004` | granted feature mask |
| `+0x008` | scene sort and draw mode, stored by `0x706e00` |
| `+0x00c`, `+0x010` | release list: head, address of the tail link |
| `+0x014` | 1021 bucket heads of shared models |
| `+0x100c` to `+0x1028` | helper thread: handle, running, stop, start event, done event, thread context, job, argument |
| `+0x102c`, `+0x1030` | vertex shader array (1800 handles), pixel shader array (32 handles) |
| `+0x1034` | a 4-byte device object the texture loads read |

Addresses: `0xceefb0`, `0x706440`, `0x7064c0`, `0x706d90`, `0x706da0`, `0x706db0`, `0x706dc0`, `0x706e00`
Confidence: verified
Proof: one
Test: none

`0x706d90` calls `0x706da0`, which runs `0x706440` on `0xceefb0`, then jumps to `0x706db0`, which
registers `0x706dc0` (the destructor `0x7064c0`) with `_atexit`. The constructor zeroes `+0x0`,
`+0xc`, `+0x10` and 0x3fd dwords from `+0x14`. The destructor tests `+0x0` and pushes the string
`0x873070` before closing `+0x100c`, `+0x1018` and `+0x101c`. The other offsets are read from
their writers in `0x706510`, `0x706a50` and `0x706d10`.

## Feature mask

`M2Initialize` grants each requested feature bit the device supports and stores the result at
`+0x4`. Bit 4 (the helper thread) needs more than one processor; bit 8 (vertex shaders) a device
shader version of 1 or 2; bit 0x10 (pixel shaders) pixel shader version 4 and bit 8 granted; bit 1
a nonzero device field `+0xbc`; bit 2 a positive `+0xc0`, unless the vertex shader version is 2 and
bit 8 is granted; bit 0x20 is granted as requested. A second call logs
`Model2: M2Initialize called mode than once` and returns 1.

Addresses: `0x706dd0`, `0x706510`, `0x706541`, `0x7065ab`, `0x7065b9`, `0x70661c`, `0x706684`, `0x7066a0`, `0x7066af`, `0x7066ce`
Confidence: verified
Proof: one
Test: `models::noise_table_randf01_difftest`

`0x706dd0` moves the mask from `ecx` into `0x706510`'s argument. The device record comes from
`0x58a230`; its `+0x94` is compared with 1 and 2 at `0x7065ab` and `0x7065b9`, `+0x98` with 4 at
`0x70661c`, `+0xbc` and `+0xc0` tested at `0x706684` and `0x7066a0`. Each grant is `and` with the
requested mask and `or` into `+0x4`. The processor count is `0x429d20`'s result compared with 1 at
`0x706546`. With bit 4 granted the function starts `0x706d10` as a thread named `Model2`.

Of `0x706510`, `noise_table_randf01_difftest` runs `0x70680f`–`0x70684b`.

## Feature request

The start-up code `0x402840` registers the model system's console variables and builds the mask it passes to
`0x706dd0` from their values: bit 1 `M2UseZFill`, 2 `M2UseClipPlanes`, 4 `M2UseThreads`, 8 `M2UseShaders`, 0x10
`M2UsePixelShaders` and 0x20 `M2BatchDoodads`. All default to 1 but `M2UsePixelShaders`, which is 0, so
pixel shaders are never requested at the defaults. With vertex shaders granted `0x706510` allocates
0x708 four-byte slots at `+0x102c` and loads the file `shaders\vertex\Model2.bls` into them; with pixel shaders
granted 0x20 slots at `+0x1030` and `shaders\pixel\Model2.bls`. The programs are files of the install, not of the exe.

Addresses: `0x402880`, `0x4028a3`, `0x4028c6`, `0x4028e9`, `0x40290c`, `0x40292f`, `0x402952`, `0x402975`, `0x40297f`, `0x4029a2`, `0x4029b1`, `0x4029c1`, `0x4029d0`, `0x4029e0`, `0x706510`, `0x7065d3`, `0x7065e2`, `0x70660b`, `0x706612`, `0x70666e`, `0x706670`, `0x70667a`, `0x63db90`, `0x58b2e0`, `0x706dd0`
Confidence: verified
Proof: one
Test: none

Each `0x63db90` call registers one variable with its name, help and default string (the pushes before
the call); the result of each is stored by the `mov ds:G,eax` before the next call (`0x40289e`,
`0x4028c1`, `0x4028e4`, `0x402907`, `0x40292a`, `0x40294d`). `0x402984` onward read the record's value `+0x28`
of each in turn and `or` its bit into `ecx` (`0x40298d`, `0x40299f`, `0x4029ae`, `0x4029be`, `0x4029cd`, `0x4029dd`)
before the call at `0x4029e0`. The vertex slots are `0x1c20` bytes at `0x7065e2` (`0x70660b` pushes the path,
count `0x708`), the pixel ones `0x80` bytes (`0x70666e`, `0x706670`, count `0x20`). The strings are at `0x8730ac`
and `0x873090`.

## Noise table

Initialization fills the 128 floats at `0xcf58f0` with uniform values in [0, 1): it seeds a
generator state from `(rand() << 16) | (rand() & 0xffff)`, advances it 128 times, and for each
32-bit output `u` stores `asfloat((u & 0x7fffff) | 0x3f800000) − 1.0`. The particle registry fills
the same table the same way when it is created first.

Addresses: `0xcf58f0`, `0x706510`, `0x7ae010`, `0x7ae13b`, `0x7ae161`
Confidence: verified
Proof: one
Test: `models::noise_table_randf01_difftest`

Both fills inline the generator advance of `0x4531e0` on a stack state seeded by `0x44ea80`: four
table reads from `0x802700` with rotations, `xor` and an add into the first state word. The value
is `and eax,0x7fffff; or eax,0x3f800000` (`0x7ae13b`), loaded as a float and reduced by
`fsub 1.0` before `fstp [eax+0xcf58f0]` (`0x7ae161`), for byte offsets 0 to 0x1fc. The particle
code reads the table through `0x7b1cc0`.

Of `0x7ae010`, `noise_table_randf01_difftest` runs `0x7ae13b`–`0x7ae177`.

## Shared model lookup

`0x706a50` returns the shared model of a file, loading it on first use. `.mdl` and `.mdx` (without
case) become `.m2`; any other extension fails, and so does a file that does not open. The key is
the name the opened file reports, up to its last dot: `h = h·19 + tolower(c)` over its bytes in
32-bit arithmetic, each byte sign-extended first, so a byte of 0x80 or above adds its negative
value, which the C-locale `tolower` returns unchanged; the bucket is `h mod 1021`, unsigned. A bucket's chain is sorted by hash, then by name without case; a match adds a reference
and closes the file, a miss inserts a new 0x164-byte shared model and starts its load.

Addresses: `0x706a50`, `0x706b35`, `0x706b68`, `0x706b71`, `0x706b8a`, `0x6487b0`, `0x71d2c0`, `0x71d430`, `0x71d4e0`, `0x73f700`, `0x40f92b`
Confidence: verified
Proof: one
Test: none

The extension is found with `strrchr` (`0x73f700`) and compared with `SStrCmpI`. `0x6487b0`
copies the file's name into a 0x104-byte buffer (`0x706b35`) and the last dot is cut. The hash loop
loads each byte with `movsx` (`0x706b68`), calls `tolower` (`0x40f92b`), whose C-locale path adds
0x20 only for 0x41 to 0x5a, and computes `imul ebx,ebx,0x13; add ebx,eax` (`0x706b71`). The bucket
is `div` by 0x3fd (`0x706b8a`). The chain
walk continues while the node's hash `+0x12c` is lower, or equal with `SStrCmpI(node+0x20, name)`
negative. A new node is allocated with line 0x19d, constructed by `0x71d2c0`, loaded by
`0x71d4e0(file, loose)` and given the name at `+0x20` and the hash at `+0x12c`.

## Release and eviction

A shared model whose count drops to zero stays cached on the release list, stamped with the
time. Setting a scene's clock evicts from the head of that list every model released 30000 ms or
more ago, stopping at the first younger one. A reference taken before eviction takes the model off
the list.

Addresses: `0x706870`, `0x707490`, `0x71d480`, `0x71d430`
Confidence: verified
Proof: one
Test: none

`0x71d480` stores `OsGetAsyncTimeMs()` at `+0x1c` and appends the node through the tail link
`+0x10` of the cache. `0x706870` compares `now − node+0x1c` (unsigned) with 30000 and stops on
`jb`; each evicted node closes its read request, its file, runs `0x71d330` and is freed.
`0x71d430` unlinks a node whose count is zero before incrementing it. `0x707490` calls `0x706870`
after storing the clock.

## Helper thread

With bit 4 granted the cache runs one helper thread. A job is a function and an argument stored on
the cache, then the start event; the thread runs `job(argument)` with the argument in `ecx`, sets
the done event and waits again. Destroying the cache sets the stop flag, wakes the thread and waits
for its handle.

Addresses: `0x706cd0`, `0x706d00`, `0x706d10`, `0x706900`
Confidence: verified
Proof: one
Test: none

`0x706cd0` stores `+0x1024` and `+0x1028` and sets `+0x1018`; `0x706d00` waits on `+0x101c`.
`0x706d10` sets its thread context from `+0x1020`, sets `+0x1010`, and loops on `+0x1014`.
`0x706900` stores 1 at `+0x1014`, sets `+0x1018` and waits on `+0x100c`, then destroys every shared
model on the release list and in the buckets and releases the 1800 vertex and 32 pixel shaders.

## Scene creation and teardown

`0x706e10` allocates a 0x1d6c-byte scene and constructs it on the cache. `M2Destroy` destroys the
cache's contents, then the particle and the ribbon emitter registries.

Addresses: `0x706e10`, `0x706ff0`, `0x706de0`, `0x706900`, `0x7ae190`, `0x7aecd0`
Confidence: verified
Proof: one
Test: none

`0x706e10` calls `SMemAlloc(0x1d6c)` with line 0x221 and `0x706ff0(0xceefb0)`. `0x706de0` calls
`0x706900` and `0x7ae190`, then jumps to `0x7aecd0`.
