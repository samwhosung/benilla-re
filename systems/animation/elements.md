# animation: element loops

After the bone loop, the animate pass runs one loop over each family of animated records in the shared model
header and samples their tracks into the instance's state arrays: colours, transparency, texture frames,
texture transforms, lights, cameras, ribbons, particle emitters and attachment bytes. Each family has a
header count and array, a record stride, a fixed set of tracks at fixed record offsets, and a state of the
shape `tracks` gives each value. This doc maps the families and states; what the values mean to the
draw is `models/lights`, `models/ribbons`, `models/emission` and `models/render`. The code is
`0x715966` through `0x7185e1` of `CM2Model::Animate`.

Roots: none

## Element families

The loops run in the order of the table. Every track is sampled by the block of `tracks#sampling-block` and
only when its key count exceeds the first-update gate, except in the texture transform loop and for a light's
visibility byte, which sample a track whenever its count is not 0. The clock bone is the bone state whose time, slot, blend clock and weight the
block reads.

| family | header count, stride | tracks, by record offset | clock bone |
|---|---|---|---|
| colours | `+0x54`, 0x38 | `+0x00` vector, `+0x1c` 16-bit | bone 0 |
| transparency | `+0x64`, 0x1c | `+0x00` 16-bit | bone 0 |
| texture frames | `+0x6c`, 0x1c | `+0x00` word | bone 0 |
| texture transforms | `+0x74`, 0x54 | `+0x00` vector, `+0x1c` quaternion, `+0x38` vector | bone 0 |
| lights | `+0x11c`, 0xd4 | `+0x10` vector, `+0x2c` scalar, `+0x48` vector, `+0x64` scalar, `+0xb8` byte | record word `+0x2` |
| cameras | `+0x124`, 0x7c | `+0x10` vector cubic, `+0x38` vector cubic, `+0x60` scalar cubic | bone 0 |
| ribbons | `+0x134`, 0xdc | `+0x24` vector, `+0x40` 16-bit, `+0x5c` and `+0x78` scalars, `+0xa4` word, `+0xc0` byte | record word `+0x4` |
| particles | `+0x13c`, 0x1f8 | ten scalars from `+0x34` in steps of 0x1c, byte `+0x1dc` | record word `+0x14` |
| attachments | `+0x104`, 0x30 | `+0x14` byte | record word `+0x4` |

Addresses: `0x715966`, `0x715983`, `0x715c87`, `0x715ca0`, `0x715e46`, `0x715e60`, `0x715f25`, `0x7163bc`, `0x7163e0`, `0x716ad9`, `0x716b00`, `0x71763e`, `0x717660`, `0x717d6a`, `0x717d90`, `0x7185e3`, `0x718602`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

The sites of each track, with the bone that supplies the clock: colours `0x7159c6` and `0x715b21` (bone 0 through
`[+0x90]`), texture transforms `0x71607f`, `0x71623d` and the quaternion `0x715ff9`, lights `0x71641b`
(`movzx esi,word [edx+2]; imul esi,esi,0x118`), ribbons `0x717691` (`word [edx+4]`), particles `0x717dca`
(`word [esi+0x14]`), attachments `0x718624` (`word [eax+4]`). The 58 calls of `0x713d50` split as 4 in the bone loop,
4 colours, 2 transparency, 2 frames, 4 texture transforms, 10 lights, 6 cameras, 12 ribbons and 14 particles
(the byte and six scalars, each with a blend search; the other four scalars use `0x71af20`, the attachment
byte `0x71ae90` and the rotation `0x713ea0`). The ten particle scalars are
at `+0x34`, `+0x50`, `+0x6c`, `+0x88`, `+0xa4`, `+0xc0`, `+0xdc`, `+0xf8`, `+0x114` and `+0x130`.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Element states

Each family's state record holds one track state per track, at fixed offsets: a vector state is 0x30 bytes
with its value at `+0xc`, a scalar, 16-bit, word or byte state 0x20 bytes with its value at `+0xc`, a
quaternion state 0x38 bytes. The records are 0x50 (colours), 0x20 (transparency, texture frames), 0x98
(texture transforms, with a 64-byte matrix per record in the second array), 0x170 (lights), 0x84 (cameras), 0xd0
(ribbons), 0x16c (particles) and 0x20 (attachment bytes).

| family | track states, by state offset | after the tracks |
|---|---|---|
| colours | vector `+0x00`, 16-bit `+0x30` | |
| texture transforms | translation `+0x00`, rotation `+0x30`, scale `+0x68` | matrix |
| lights | vector `+0x00`, scalar `+0x30`, vector `+0x50`, scalar `+0x80`, byte `+0xe0` | enabled `+0x100`, light `+0x104` |
| cameras | vector `+0x00`, vector `+0x30`, scalar `+0x60` | camera `+0x80` |
| ribbons | vector `+0x00`, 16-bit `+0x30`, scalars `+0x50`, `+0x70`, word `+0x90`, byte `+0xb0` | |
| particles | ten scalars `+0x00` to `+0x120` in steps of 0x20, byte `+0x140` | flags `+0x160`, `+0x164` |

Addresses: `0x715b0f`, `0x715b21`, `0x716551`, `0x716649`, `0x716814`, `0x71692f`, `0x71644c`, `0x716f76`, `0x7173cc`, `0x7176c2`, `0x7177b1`, `0x717933`, `0x717ab8`, `0x717baf`, `0x717cc2`, `0x717dfb`, `0x7185ba`, `0x717e94`, `0x717eb0`, `0x717ec9`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

A state's first triple is the `{k, k+1, fraction}` of the search. The out pointer of each call gives the
offsets: `esi+0x30` at `0x715b0f` for the colours' 16-bit track, `edi+0x30` (`0x716551`), `edi` (`0x716649`),
`edi+0x80` (`0x716814`), `esi = edi+0x50` (`0x71692f`) and `edi+0xe0` (`0x71644c`) for lights, `edi`, `edi+0x30` (`0x716f76`)
and `edi+0x60` (`0x7173cc`) for cameras, `esi+0xb0` (`0x7176c2`), `esi` (`0x7177b1`), `esi+0x30` (`0x717933`), `esi+0x50`,
`esi+0x70` and `esi+0x90` for ribbons, and `edi+0x140` (`0x717dfb`) plus the scalars' `edi+0x20·i` for particles.
The light object is embedded at `state+0x104` and the camera object is the pointer at `state+0x80`.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Light loop

A light record is sampled in two steps. When the state's enabled word `+0x100` is nonzero and the record's
visibility track has keys, the pass samples the visibility byte into `+0xec`; this track's count is tested
against 0, not the gate. The other tracks are sampled when the enabled word and the byte are both nonzero,
and on the pass where the first-update gate is 0; with the gate at 1 a disabled or invisible light is skipped. The intensity is sampled before its colour. When the ambient colour is sampled the
light's ambient colour becomes `f32((intensity · alpha) · colour[i])` per component, `alpha` the instance's
`+0x19c`, and the diffuse colour likewise from the diffuse intensity and colour.

Addresses: `0x7163fc`, `0x716404`, `0x71640d`, `0x716415`, `0x7164f2`, `0x7164fa`, `0x716504`, `0x716506`, `0x71650e`, `0x71678a`, `0x716793`, `0x7167bd`, `0x7167ce`, `0x716a67`, `0x716a73`, `0x716aa6`, `0x716aa9`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

`0x7163fc` loads the enabled word; `0x716404 je 0x716506` and `0x716415 je 0x7164f2` (no visibility keys)
reach the test at `0x7164f2`: enabled, then the byte `[edi+0xec]`, and both nonzero continue at `0x716514`.
Otherwise `0x716506` reads `[ebx+0x8c]` and `0x71650e jne 0x716aa9` skips the record. `0x71678a` is
`fld [edi+0x3c]; fmul [ebx+0x19c]`, the product kept at 53 bits, then `fmul` by each colour component with an
`fstp dword` spill and the stores to `[edi+0x134]` to `[edi+0x13c]`; `0x716a67` does the same into
`[edi+0x140]` onward from `[edi+0x8c]`. The stores sit in the colour tracks' branches.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Particle states

For each particle record the pass samples the enabled byte into the state `+0x14c`, then the ten scalars, and sets
two flags. `state+0x160` is 1 when the enabled byte and the instance's `+0x50` are both nonzero, else 0.
`state+0x164` is 1 when `+0x160` is 1 or the emitter has live particles, else 0, and the instance's `+0x3d8`
is the OR of the `+0x164` values, cleared before the loop. A record whose enabled byte is 0 skips its scalars
once the first-update gate is 1. The emitter list is `[+0x3d4]`, an array of emitter pointers.

Addresses: `0x717d6f`, `0x717dca`, `0x717dfb`, `0x717e94`, `0x717e9e`, `0x717eb0`, `0x717eb8`, `0x717ebb`, `0x717ecf`, `0x717ed5`, `0x717ee7`, `0x7b5f60`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

`0x717e94` loads the byte and `0x717e9e` the instance flag; `0x717eb0` stores the first flag and `0x717ebb`
calls `CParticleEmitter2::HasLiveParticles` on `[[+0x3d4] + 4i]` only when it is 0. `0x717ee7 jne 0x7185b1` skips the
scalars for a disabled record once `+0x8c` is nonzero. The byte track's blend leg stores the second byte
at `+0x15c` and does not blend.
`animation::m2_animate_difftest` runs `0x714260` whole.
