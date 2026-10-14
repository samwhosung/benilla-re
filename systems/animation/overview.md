# animation: overview

How a model instance is animated: the clocks of its bones, the sampling of key tracks, the bone matrices, the billboards,
the animated elements of a model, the calls that start a sequence, and the object-layer code that chooses what a unit plays.
The system has no object file and no function of its own. The evaluator is one function of `models`' M2Model.cpp, and the
choosing of sequences belongs to the unit code of `objectlayer`, so every row it touches is owned there and every doc says
`Roots: none`.

The docs split by topic. `kernel` is the pass as a whole, `clock` the bone clocks, `pose` the bone matrices, `billboards`
the facing stage, `tracks` the sampling of a track, `elements` the loops over colours, lights, cameras, ribbons and emitters,
`attachspace` the space the matrices are built in, `playback` the call that starts a sequence, `units` the unit tracks
that call it, and `hosts` the models no unit controls. `models/animation` states the same code in short, and
`objectlayer/unitanim` holds the unit's side of the sequence tables.

Roots: none

## Evaluator location

`CM2Model::Animate` is `0x714260`, 17703 bytes inside M2Model.cpp, which `models` owns (`models/overview#object-files`). The
function is thiscall with four stack arguments. Its callers are the scene's animate pass, the ensure-animated step, its own
child walk and the model particles; sequences are started by `0x7121a0` and the code of `playback`.

Addresses: `0x714260`, `0x70dc60`, `0x71b070`, `0x7121a0`, `0x5fe2f0`
Confidence: verified
Proof: one
Test: none

M2Model.cpp spans `0x70dc60` to `0x71b070` by the initializer run of `models/overview`. The unit entry `0x5fe2f0` is
`PlayAnimation` of the object layer; its track play, `units#playing-a-track`, starts a unit's sequences.

## Pass order

One pass runs in a fixed order: the gates and the world matrix, the bone loop, the elements, the attached children. Each bone
gets its clock, then its matrix, then the billboard stage. The element loops follow, then the children, then the frame stamp.

Addresses: `0x714260`, `0x7143ee`, `0x714530`, `0x714d0f`, `0x7151f9`, `0x715966`, `0x7185e3`, `0x718775`
Confidence: verified
Proof: one
Test: none

The bone loop runs from `0x7143ee` (pause slide) through the clock legs `0x714530`, the matrix steps from `0x714d0f` and the billboard
stage `0x7151f9`. The element loops start at `0x715966`, the children at `0x7185e3`, and the exit that stores the scene frame is `0x718775`.
