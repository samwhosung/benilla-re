# gx: texture levels

The levels the Direct3D device creates and uploads for a texture. The device decides them from the
texture record's size and flags and the skip count `baseMip` leaves in the device
(`gx/texturesize`); `CGxDeviceD3d::CreateTexture` and `UploadTexture` both ask the one routine
(`gx/textures`, "Device texture upload"), so the created chain and the uploaded one agree.

Roots: `0x5a0880`

## Direct3D level range

`0x5a0880(tex, &width, &height, &first, &end)`, with the device in `ecx`, gives the levels the
Direct3D device creates and uploads: `first` to `end - 1`, the top one `width` by `height`. A
texture whose filter mode (flag bits 0 to 2) is 0 or 1, or whose flag bit 6 is set, with bit 5
clear has one level at full size: `first` 0, `end` 1. Any other has `floor(log2(max(width,
height))) + 1` levels; `first` is the device's skip count `[dev+0x31c]` held to the last level, both
sides shift right by it, and bit 5 makes `end` `first + 1`, else `end` is the level count. A texture
0 by 0 on the chain never returns.

Addresses: `0x5a0880`, `0x5a08ad`, `0x5a08cb`, `0x5a08f0`, `0x5a0902`, `0x5a0930`, `0x5a0951`, `0x5a0996`, `0x5a09c7`, `0x5a0a80`, `0x5a0aed`
Confidence: verified
Proof: one
Test: `gx::d3d_texture_levels_difftest`

The record's width, height and flags are `+0x24`, `+0x28` and `+0x3c`. `0x5a08ad` tests bit 5 for
a mode of 0 or 1, and bit 6 when the mode is neither (`0x5a08a8`); `0x5a08cb` takes the longer side
and `0x5a08f0` is the `shr`/`inc` loop. `0x5a0902` and `0x5a0903` hold the skip count to the count
less 1, unsigned. `0x5a0930` calls it (`0x5a0951`) and passes `end - first` as the level count of
`CreateTexture` (`0x5a09c7`), adding the auto-generate usage 0x400 for bit 6 (`0x5a0996`);
`0x5a0a80` calls it at `0x5a0aed` and locks level `n - first` for each `n` from `first` to `end`.
