# image: object files and roots

The client's image readers: `tga.cpp` (the TGA record, its decoder and its writer), the mip
geometry arithmetic after it, a file with no source path that holds the pixel format conversion
matrix, the DXT decoders and the box filter, `blp.cpp` (the BLP2 reader) and the palette
converters that end the run. The texture service calls the readers; the graphics layer chooses
the pixel format and uploads. Topic docs: `tga`, `mips`, `blp` and `convert`.

The code lies from `0x5a35e0` to `0x5a9dc0`. Slots `0x82a61c` to `0x82a650` of the initializer
table hold the run, 14 initializers.

Roots: `0x5a3600`, `0x5a3660`, `0x5a3a30`, `0x5a3fc0`, `0x5a4070`, `0x5a41b0`, `0x5a41f0`, `0x5a4250`, `0x5a43c0`, `0x5a4810`, `0x5a4af0`, `0x5a4b80`, `0x5a4bb0`, `0x5a4c30`, `0x5a4f60`, `0x5a62c0`, `0x5a7aa0`, `0x5a7b40`, `0x5a7f00`, `0x5a8100`, `0x5a8220`, `0x5a8240`, `0x5a83b0`, `0x5a8430`, `0x5a35e0`, `0x5a4c10`, `0x5a4ef0`, `0x5a4f10`, `0x5a4f30`, `0x5a7880`, `0x5a78a0`, `0x5a78c0`, `0x5a78f0`, `0x5a7990`, `0x5a7a40`, `0x5a80e0`, `0x5a9db0`, `0x5a9dd0`

## Object files

Four stretches of the run belong to different files. `tga.cpp` is `0x5a3600` to `0x5a4810`: its
allocations cite `E:\build\buildWoW\ENGINE\Source\Images\tga.cpp` and its single initializer is
`0x5a35e0`. The mip geometry `0x5a49b0` to `0x5a4bb0` takes the caller's file and line for its
allocation and cites no path. The next file opens its run with `0x5a4c10` and holds the box
filter, the conversion matrix and the DXT decoders to `0x5a79f0`; it cites no path. `blp.cpp`
(`E:\build\buildWoW\engine\Source\BLPFile\blp.cpp`) runs from `0x5a7880`, with its two statics
`0x5a7990` and `0x5a7a40`, to the palette converters, and the run's last initializers
`0x5a9db0` and `0x5a9dd0`.

Addresses: `0x827000`, `0x82a61c`, `0x82a650`, `0x5a35e0`, `0x5a4c10`, `0x5a7880`, `0x5a7990`, `0x5a7a40`, `0x5a80e0`, `0x5a9db0`, `0x5a9dd0`, `0x5a3600`, `0x5a4810`, `0x5a49b0`, `0x5a4bb0`, `0x5a79f0`
Confidence: verified
Proof: one
Test: none

The path strings sit at `0x85c824` (`tga.cpp`) and `0x85c9d0` (`blp.cpp`); every code reference to
them lies in `0x5a360f` to `0x5a47e7` and in `0x5a7ab8` to `0x5a8485`. Slots `0x82a61c` to
`0x82a62c` name `0x5a35e0` and the four initializers of the second file, `0x82a630` to `0x82a63c` the
four of `blp.cpp` (+∞, 2π, 1/2π and the interpolation table), `0x82a640` its late +∞ copy, and
`0x82a644` to `0x82a650` the 2π and 1/2π thunks `0x5a9db0` and `0x5a9dd0` with the two statics. The
next slot, `0x82a654`, names code of another system.

## Initializers and statics

Fourteen initializers run before the first frame. Eleven copy +∞ or store 2π and 1/2π, one builds
the DXT interpolation table, and two set up the status sink and the read buffer of the BLP reader.

Addresses: `0x82a61c`, `0x5a35f0`, `0x5a4c20`, `0x5a4f00`, `0x5a4f20`, `0x5a4f40`, `0x5a7890`, `0x5a78b0`, `0x5a78d0`, `0x5a7900`, `0x5a7990`, `0x5a79a0`, `0x5a79f0`, `0x5a7a00`, `0x5a7a40`, `0x5a7a50`, `0x5a7a70`, `0x5a7a80`, `0x5a80f0`, `0x5a9dc0`, `0x5a9de0`, `0xc0f548`, `0xc0f54c`, `0xc0f550`, `0xc0f554`, `0xc0fa70`, `0xc0fa78`, `0xc0fa7c`, `0xc0fc00`, `0xc0fc08`, `0xc15c50`, `0xc21cc8`, `0xc15c54`, `0xc15c40`, `0x5a35e0`, `0x5a4c10`, `0x5a4ef0`, `0x5a4f10`, `0x5a4f30`, `0x5a7880`, `0x5a78a0`, `0x5a78c0`, `0x5a78f0`, `0x5a79e0`, `0x5a7a10`, `0x5a80e0`, `0x5a9db0`, `0x5a9dd0`, `0x85c824`, `0x85c9d0`
Confidence: verified
Proof: one
Test: none

Each thunk jumps to its body. The bodies copy a +∞ from rodata into `0xc0f548`, `0xc0f54c`,
`0xc0f550`, `0xc0fa78` or `0xc0fc08`; store 2π (`fld π; fadd st,st`) into `0xc0fa70`, `0xc0fc00`
and `0xc21cc8`; and store 1/2π into `0xc0f554`, `0xc0fa7c` and `0xc15c50`. No other code touches the eleven
globals except these bodies reading the 2π globals. `0x5a7900` builds the interpolation table (see `convert`). `0x5a79a0` makes the global
status sink `0xc15c54` (vtable at `0x80a2d8`, list head at `0xc15c5c`) and `0x5a79f0` registers its
destructor `0x5a7a00` with `_atexit`. `0x5a7a50` zeroes the four words of the shared read buffer
`0xc15c40` to `0xc15c4c`; `0x5a7a70` registers `0x5a7a80`, which frees its buffer `0xc15c48`.

## Unreached functions

These have no call, jump or stored address outside unreached code: the TGA copy constructor and
the run-length encoder, the in-memory BLP builder and its mip copy, the BLP setters and size
queries, the plain and ordered-dither palette converters and a TGA check.

Addresses: `0x5a4220`, `0x5a4350`, `0x5a41d0`, `0x5a4710`, `0x5a4570`, `0x5a44f0`, `0x5a7c20`, `0x5a7c50`, `0x5a7cc0`, `0x5a7ad0`, `0x5a7e40`, `0x5a7ea0`, `0x5a8230`, `0x5a8720`, `0x5a8750`, `0x5a8c00`, `0x5a8910`, `0x5a8a40`, `0x5a8b80`, `0x5a8ea0`, `0x5a9030`, `0x5a91d0`
Confidence: verified
Proof: one
Test: none

Census: every `e8` and `e9` whose target is the function, and every little-endian occurrence of
its address in any section; then the same without the sites inside unreached code, to a
fixed point. `0x5a4710` reaches `0x5a4570`, which reaches `0x5a44f0`; `0x5a4350` is the only
caller of `0x5a41d0`; `0x5a7c50` calls `0x5a7c20`, `0x5a7cc0`, which calls `0x5a7e40` and
`0x5a7ad0`; `0x5a8c00` calls the three plain converters. Controls: `0x5a3660` (4 call sites),
`0x5a4af0` (5), `0x5a8ce0` (two live callers) and `0x5a5780`, whose address `0x5a4fc5` stores. The ordered family `0x5a8ea0`, `0x5a9030`, `0x5a91d0` has no callers at all.
