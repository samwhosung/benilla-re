# animation: attachment space

The space in which a model instance's matrices are built. Every bone matrix, every attachment matrix and every transposed constant a
draw uploads is relative to the camera, not to the map: the world position the unit's model carries is cancelled once, at the root
instance, by the scene's view matrix, and a child instance starts from its parent's bone matrix. The device is left with identity
view and world transforms for the whole model pass. This doc holds the three facts that make that so; the arithmetic of each is
`models/scene` and `kernel`.

Roots: none

## Origin by caller

The scene's collect pass (`0x707680`) subtracts an origin `p` from the translation row of the view matrix it reads from the device. The
world render passes the camera position, `[[ui+0x65b8]+8]`, so a world model's root matrix is relative to the camera. The interface
callers pass a zero vector, since a model frame's camera is its own origin. Twelve call sites exist.

Addresses: `0x707680`, `0x48368a`, `0x46c4c6`, `0x50596a`, `0x50c7ad`, `0x50f0df`, `0x52557a`, `0x525640`, `0x6d4b4f`, `0x6d4c1a`, `0x76ce39`, `0x76d619`, `0x7e619b`, `0x58b0b0`, `0x482b11`
Confidence: verified
Proof: one
Test: none

`0x48367e mov eax,[edi+0x65b8]; add eax,8; push eax` precedes the call, and `0x482b11` copies the same three dwords into the frame's
camera field. Nine sites build a vec3 of three zero dwords on their frame and push it (`0x46c4b1`, `0x50594e`, `0x50c791`, `0x50f0c3`,
`0x52555c`, `0x6d4b3a`, `0x6d4c05`, `0x76ce1d`, `0x7e6186`). `0x525640` and `0x76d619` push a vector filled elsewhere in their frames.
`0x707690` and `0x7076c6` read the device's matrix through `0x58b0b0`, which copies 16 dwords of the stack entry `[dev+0x1ac0]`.

## Eye-space palette

A root instance's world matrix `+0xfc` is its base matrix `+0xbc` times the scene matrix `+0x9c`, so its translation row is the unit's
map position less the camera position, carried through the view rotation. Every bone matrix of the instance is a bone's local
matrix times its parent's matrix, rooted at `+0xfc`, so the palette `+0x94` is in camera-relative space and no step of the chain
holds a map-scale value. The squared length of `+0xfc`'s translation, stored in `+0x84`, is the distance the scene sorts by.

Addresses: `0x71439b`, `0x7143e2`, `0x714943`, `0x714953`, `0x7151ba`, `0x7151c4`, `0x7143a0`, `0x70dd7f`, `0x707772`, `0x7077e0`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

`0x714389` to `0x71439b` pass the parent matrix and `+0xbc` to the product; a root's parent matrix is `+0x9c` of its scene (`0x714037`
`add eax,0x9c`, `0x7077f7 push edi`). The constructor writes the identity into `+0xbc` (`0x70dd7f`, `0x70dd85`, `0x70dd8b`). A bone with
no parent reads `&+0xfc` (`0x714943`), any other `palette + parent·0x40` (`0x714953`); the keyless leg copies the parent matrix (`0x7151c4`).
The scene walk skips every instance with a parent (`0x707772` `mov eax,[esi+0x1cc]; test; jne`), so children are reached only
through the parent's attached-children step.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Identity device transforms

The list draw (`0x70b360`) saves the device's view matrix, resets the per-stage transform slots 0 to 8, then sets the view and the
slot-8 transform to the static identity at `0xcf03e8`, and restores the view at its end. The vertex program gets the bone
palette and the projection as constants. The identity is written once by a startup initialiser, `0x706f20`.

Addresses: `0x70b360`, `0x70b3c3`, `0x70b3d2`, `0x70b3dd`, `0x70b3e2`, `0x70b3e7`, `0x70b3f1`, `0x70b705`, `0x70b715`, `0x706f20`, `0x58b050`, `0x58b240`, `0x5a1450`, `0x70caf4`, `0x70cb15`, `0x70cb1f`
Confidence: verified
Proof: one
Test: none

`0x70b3dd mov ecx,0xcf03e8; call 0x58b050` goes through the device slot `+0x74` to `0x5a1450`, which stores the matrix in the `[dev+0x1ac0]`
stack and calls the driver's transform setter with state 2 (`0x5a1469 push 2`, `call [eax+0xb0]`). `0x58b240` stores the matrix at
`dev + 0xfd0 + 8·0x118`. The loop `0x70b3d0` to `0x70b3db` calls `0x58b200` for 0 to 8 and `0x70b705` calls `0x58b220` for the same range. The
five other instructions that name `0xcf03e8` sit in `0x706f20` and the particle bracket (`0x70caf4`, `0x70cb15`, `0x70cb1f`).
