# ui: frame draw layers and sizing

The two frame methods that switch a draw layer and start a size from the cursor. The other methods that read or set one field of a frame
(type, title region, id, toplevel, movable, resizable, user-placed, clamped, drag buttons, keyboard) are the contracts of
the frame method table; `strata` has the level and strata, `visibility` show and hide, `mouse` the mouse switches, `backdrop` the backdrop methods and
`frames` the child and region lists.

Every one of these methods starts with the check of `methods`, "Finding self": a table whose element 0 is a frame, else an error.

Roots: `0x7755b0`, `0x775680`, `0x776830`

## Draw layers

`EnableDrawLayer(name)` and `DisableDrawLayer(name)` switch one of the five layers on or off. A layer name that is
missing, not a string or not recognised, ignoring case, means ARTWORK. A switch changes what the frame's regions draw at once:
the frame marks its own batches dirty when it draws itself, otherwise the level that holds it.

Addresses: `0x7755b0`, `0x775640`, `0x775647`, `0x775662`, `0x77566d`, `0x775680`, `0x77573d`, `0x76a710`, `0x76a717`, `0x76a722`, `0x76a730`, `0x76a742`, `0x76a990`, `0x76a999`, `0x76a9a2`, `0x76a9c6`, `0x6f18b0`, `0x811a80`
Confidence: verified
Proof: one
Test: none

The methods start the layer at 2 (`0x775640`), replace it when `lua_isstring` holds and `0x6f18b0` finds the name (`0x775647`, `0x775662`) and call
`0x76a710` (`0x77566d`) or `0x76a730` (`0x77573d`). These store 1 or 0 at `+0x198 + 4·layer` (`0x76a717`) and call `0x76a990` (`0x76a722`, `0x76a742`),
which tests bit 0x2000 of `+0xb4` (`0x76a999`) for the frame's own batches (`0x76b530`, `0x76a9a2`) and otherwise, with a top and a visible
frame, marks the level (`0x764a60`, `0x76a9c6`).

## Starting a size

`StartSizing([point])` starts a size of the frame from the cursor, with the anchor point named by the argument, default `BOTTOMRIGHT`. It raises
`Frame %s is not resizable` unless bit 0x200 of `+0xb4` is set, and does nothing while the top already moves or sizes a frame. The move
that `StartMoving` starts is the same operation with the point 4.

Addresses: `0x776830`, `0x7768bb`, `0x7768da`, `0x77690c`, `0x776913`, `0x776934`, `0x77693f`, `0x776981`, `0x7652b0`, `0x6f1840`
Confidence: verified
Proof: one
Test: none

`0x776830` tests the resizable bit (`0x7768bb`) and raises its message (`0x7768da`); the point starts at 8 (`0x776913`) and is replaced by `0x6f1840` of argument 2 when it is a string
(`0x776934`). It then reads the top's moving-frame word at `+0xcfc` (`0x77693f`) and, when it is clear, calls `0x7652b0(frame, 3, x, y, point)` (`0x776981`) with the cursor converted by `0x41ad80`.
`0x7652b0` is the start of a move or size described in `input`.
