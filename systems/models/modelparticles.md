# models: model particles

How an emitter with a model draws its particles: each 0x40-byte model particle carries an instance
of the model (`particles#model-particles`), and every frame `DrawModelParticles` (`0x7b4840`) hands
each one to `DrawModelParticle` (`0x7b4510`), which applies the twinkle, then places, tints and
shows the instance. The emitter and its other particles are `particles`; the gradients, the twinkle
of quad particles and the noise table are there too.

Roots: none

## Model particle twinkle

A model particle takes the twinkle (`particles#twinkle`) with one difference: its noise index is `((bits of
float(speed·age + 512.0)) >> 14) + (particle address >> 5)) & 127` with `speed·age` unclamped, where
a quad particle clamps it into [0, 255] first. With a visible fraction `+0x1b4` under 1, a value
above it hides the particle's instance and returns 0; a NaN fraction never hides. With a scale
range `+0x1c0` not 0, a NaN range included, the size is `(value·range + min)·size`.

Addresses: `0x7b4510`, `0x7b4529`, `0x7b4534`, `0x7b4547`, `0x7b4549`, `0x7b4568`, `0x7b457c`, `0x7b4590`, `0x7b4597`, `0x7b45f6`, `0x7b45f8`
Confidence: verified
Proof: one
Test: `models::lod_index_scale_difftest`, `models::alpha_distance_scale_difftest`

`0x7b4534` is `test ah,5; jnp` after `fcomp 1.0`, taken only below 1; `0x7b4547`, `test ah,0x44;
jnp` after `fcomp 0.0`, skips only on equal. `0x7b4549` to `0x7b4568` compute the index
with no `fcom` between the product and the `+ 512.0` (`0x8029cc`). `0x7b457c` repeats the first
test with `jp`; `0x7b4590` keeps the particle on C0 or C3 (`test ah,0x41; jne`), else `0x7b4597`
calls `0x710c50(0)`. `0x7b45f8` multiplies the noise by `+0x1c0`, adds `+0x1b8` and multiplies by
the gradient's size.

`lod_index_scale_difftest` runs `0x7b4549`–`0x7b4560`; `alpha_distance_scale_difftest` runs
`0x7b45f8`–`0x7b4611`.

## Model particle placement

A drawn model particle samples its gradient (`particles#gradients`, by the byte `+0xc`) at its age with the
emitter alpha `+0x1a8`, scales the size by `+0x264` under flag 0x200, builds the rotation of its
quaternion `+0x20`, scales it by the size and sets row 3 to the position. It multiplies that by the
emitter matrix `+0x1fc` under flag 0x100, else by the draw's frame when passed, and gives the
instance that base matrix, the diffuse `(b2, b1, b0)/255` and alpha `b3/255` of the colour bytes,
and its visible flag; it returns 1. The rotation, with `xx = 2x·x`, `wz = 2z·w` and so on:

| row | x | y | z |
|---|---|---|---|
| 0 | `1 − (zz + yy)` | `xy + wz` | `xz − wy` |
| 1 | `xy − wz` | `1 − (zz + xx)` | `yz + wx` |
| 2 | `xz + wy` | `yz − wx` | `1 − (yy + xx)` |

Addresses: `0x7b4510`, `0x7b45e0`, `0x7b4617`, `0x7b4628`, `0x7b4699`, `0x7b46ee`, `0x7b476d`, `0x7b4785`, `0x7b4799`, `0x7b47a4`, `0x7b47b0`, `0x7b47cd`, `0x7b47f8`, `0x7b4814`, `0x7b481e`, `0x8026c8`
Confidence: verified
Proof: one
Test: `models::flag200_scale_difftest`, `models::quat_billboard_matrix_difftest`, `models::color_byte_to_f32_difftest`

`0x7b45e0` calls `0x7b9b10` on `+0xbc + 0x60·byte`. Every product is rounded to a float except `yy`
and `yz`, which stay on the stack at 53 bits; `0x7b4699` is `d8 e9`, `fsubr st,st(1)`, so `yz − wx`.
`0x7b46ee` stores the nine floats with `0x5f8d20`, expanded to 4×4 with zero fourth entries and 1.0
in the last; `0x7b476d` is `0x7bdd00(size)`. `0x7b4785` tests flag 0x100, `0x7b4799` the frame, and
`0x7b47a4` is `0x7bce30`. The colour bytes are `fild`·1/255 (`0x8026c8`), each rounded once.

`flag200_scale_difftest` runs `0x7b461c`–`0x7b4628`, `quat_billboard_matrix_difftest`
`0x7b4628`–`0x7b46ee`, `color_byte_to_f32_difftest` `0x7b47ec`–`0x7b47f8`.
