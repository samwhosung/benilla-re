# storm: compressed loose files

A loose file whose name ends `.bz`, in any case, enters the loose-file index under its name without
the extension as kind 1 (`mount`, "Loose-file index"). `SFile::OpenEx` opens such a hit as a 24-byte
header and a zlib stream; the name is not bzip2's. The open file is a 0x60-byte object, and
`SFile::Read`, `SFile::SetFilePointer` and `SFile::GetFileSize` dispatch on its kind at `+0x00`: kind 1
reads inflate the stream through a 4 KiB input buffer, a seek forward reads and discards, a seek back
starts the stream again, and the size is the header's. The code is `SFile2.cpp`'s; the inflater is the
zlib 1.2.2 the client links.

Roots: `0x6477c0`, `0x648460`, `0x648a60`, `0x6487f0`, `0x648730`

## The file object

`0x647290(kind)` builds the object: the kind at `+0x00`, the lock at `+0x24`, and 0 in every field but
`+0x14`, `+0x20` and `+0x44` to `+0x50`, which keep what the allocation held. A kind-1
open fills the `FILE *` (`+0x04`), the path and name copies (`+0x0c`, `+0x10`), the inflated size
(`+0x14`), the input buffer (`+0x18`), the zlib stream (`+0x1c`), the position (`+0x20`) and 16 header
bytes (`+0x44`, flagged at `+0x54`). Other kinds use `+0x08`, `+0x3c` and `+0x40`; `+0x58` and
`+0x5c` serve overlapped reads. `0x6472d0` ends the inflate and frees the stream, the copies and
the buffer, and deletes the lock; it does not close the `FILE *`, which only `SFile::Close` does first.

Addresses: `0x647290`, `0x6472a1`, `0x647299`, `0x647817`, `0x647a45`, `0x647ab2`, `0x648674`, `0x647617`, `0x64749f`, `0x655210`, `0x647b10`, `0x647b2a`, `0x6472d0`, `0x6472da`, `0x648730`, `0x648743`, `0x648790`, `0x648f60`
Confidence: verified
Proof: two
Test: none

`0x6472a1` stores the argument and `0x6472a5` to `0x6472c3` zero `+0x04` to `+0x10`, `+0x18`, `+0x1c`,
`+0x3c`, `+0x40` and `+0x54` to `+0x5c`; `0x647299` constructs the lock. `SFile::OpenEx` allocates it with
flags 0, so unzeroed (`0x647817`). `0x6472da` is `inflateEnd(+0x1c)`, then four `SMemFree`
and the lock's destructor as a tail jump. `0x648743` is `fclose(+0x04)` before `0x648790` calls the
destructor. `0x647a45` stores kind 2's block at `+0x08`, whose `+0x04` the archive open fills; `0x647a7e` and `0x647ab2`
pass `+0x3c` to the archive file open; kind 4 keeps an `SZip.cpp` handle at `+0x40`. `SFile::Close` closes
all three. `SFile::Load` sets `+0x58` for an overlapped read
(`0x648674`), which `0x647350` tests to close the file; `0x647617` and `0x64749f` count `+0x5c` up and down.
For kinds 2 and 3 opened with flag 0x10000, `0x655210` fills `+0x44` and the open reads it (`0x647b10` to
`0x647b2a`), setting `+0x54` only when it is not all zero. A kind-1 file's `+0x44` is read only by `0x648f60`,
which `tools/callers.py` finds no call or stored address of, with `0x4e6d20` and `0x4e5b80` as controls.

## Opening a .bz file

For kind 1, `SFile::OpenEx` opens the full path with mode `rb`; a null `FILE *` frees the object and returns
0. It copies the path, cut at its last `.` when the rest is `.bz` in any case, and the name, then reads 24
bytes without checking the count. Unless the dword at header offset 4 is `BZ00` it frees the object and
returns 0, leaving the file open. Otherwise it keeps the size and the 16 bytes, sets the position to 0,
allocates the 0x1000-byte input and the 0x38-byte stream, calls `inflateInit_(stream, "1.1.3", 0x38)`
without testing it, and returns 3.

| offset | bytes | content |
|---|---|---|
| 0 | 4 | the inflated size, little-endian; `SFile::GetFileSize` answers it |
| 4 | 4 | `BZ00` |
| 8 | 16 | kept at `+0x44`; no live code reads them for a `.bz` file |
| 24 | to the end | a zlib stream: a 2-byte header, deflate data, an Adler-32 |

Addresses: `0x6477c0`, `0x64783d`, `0x647894`, `0x6478a4`, `0x6478b5`, `0x6478e6`, `0x64791d`, `0x647935`, `0x64794e`, `0x647964`, `0x647966`, `0x647993`, `0x64799e`, `0x6479ba`, `0x6479cd`, `0x6479d0`, `0x6479e7`, `0x647a1d`, `0x647a24`, `0x647db0`
Confidence: verified
Proof: two
Test: `storm::bz_open_header_difftest`

`0x64783d` jumps to `0x647894` for kind 1; `0x6478a4` calls `0x647db0(path, "rb")` and `0x6478b5` tests the
result. `0x6478e6` and the next call copy the path and name with `SStrDupA`; `0x64791d` finds the last `.`,
`0x647935` compares it with `.bz` (`SStrCmpI`, length 0x7fffffff) and `0x64793e` writes 0 there.
`0x64794e` is `fread(header, 1, 0x18, file)`; `0x647964` compares `[header+4]` with the dword `BZ00`, and
`0x647966` calls the destructor and `SMemFree`. `0x647993` stores offset 0 at `+0x14`, `0x64799e` on
offsets 8 to 23 at `+0x44`, `0x6479ba` 1 at `+0x54` and `0x6479cd` 0 at `+0x20`; `0x6479d0` and `0x6479e7`
allocate, the stream's `avail_in`, `zalloc`, `zfree` and `opaque` are zeroed, and `0x647a1d` calls
`0x730e80`. The test runs `0x647940` to `0x647a22` and to `0x647966`.

## Reading a .bz file

`0x6476e0(file, out, size, &read)` sets the stream's output to `out` and `size`, then loops: with no input
left it points the input at the buffer and `fread`s up to 0x1000 bytes into it, storing the count when
it is above 0; it calls `inflate(stream, Z_SYNC_FLUSH)` (`0x730ea0`, "The zlib it links"). A full output stores `size`, adds it to the
position and returns 1. Otherwise `Z_STREAM_END` stores the bytes produced, adds them and returns 1 when
there are any, else 0; `Z_OK` loops; any other result returns 0 with `read` and the position untouched.
So a stream that ends early, or damage the output reaches, fails the read in which it shows.

Addresses: `0x6476e0`, `0x6476ed`, `0x6476fe`, `0x647703`, `0x647716`, `0x647725`, `0x647730`, `0x64773b`, `0x64773f`, `0x647744`, `0x647757`, `0x647759`, `0x647779`, `0x64778a`, `0x647795`, `0x648460`, `0x648505`, `0x647610`, `0x647350`, `0x6473e7`
Confidence: verified
Proof: two
Test: `storm::bz_read_difftest`, `storm::bz_damaged_difftest`

`0x6476ed` stores `avail_out` and `next_out`; `0x6476fe` tests `avail_in`, `0x647703` stores `next_in`,
`0x647716` is the `fread` and `0x647725` stores `avail_in` after `jle`. `0x647730` calls `0x730ea0` with `edx`
2; `0x64773b` tests `avail_out` first, then `0x64773f` compares 1 and `0x647744` 0. `0x647757` and
`0x647759` store the count and add it; `0x647779` and `0x64778a` do the same with `size - avail_out`, and
`0x647795` is `sbb`/`neg` of `0 < count`. `SFile::Read` calls it for kind 1 at `0x648505` under the
object's lock; an overlapped read goes to the queue `0x647610`, whose worker `0x647350` seeks to the
request's offset when it is nonzero, reads (`0x6473e7`) and seeks back. The tests run `0x6476e0` whole.

## Size and seeking

`SFile::GetFileSize` answers kind 1 with the header's size and leaves the high word unwritten.
`SetFilePointer`'s `&high` is neither read nor written for kind 1.
`SFile::SetFilePointer(file, distance, &high, method)` for kind 1: method 0 below the position ends the
inflate, starts it again, empties the input, seeks the file to offset 24 and skips `distance` bytes, and
at or above it skips `distance - position`; method 1 skips `distance`; any other method moves nothing. A
skip of 0 or less does nothing. Skipping reads through a 4 KiB stack buffer, at most 0x1000 bytes a
read; a failed read returns 0, else the function returns the position.

Addresses: `0x6487f0`, `0x64883f`, `0x648a60`, `0x648ad9`, `0x648ae6`, `0x648aff`, `0x648b06`, `0x648b23`, `0x648b2f`, `0x648b37`, `0x648b3c`, `0x648b40`, `0x648b45`, `0x648b4b`, `0x648b64`, `0x648b6b`, `0x648b72`
Confidence: verified
Proof: two
Test: `storm::bz_seek_difftest`, `storm::bz_damaged_difftest`, `storm::bz_read_difftest`

The size table at `0x648874` sends kind 1 to `0x64883f`, `mov eax,[eax+0x14]`. `0x648a60`'s table sends kind
1 to `0x648ad9`: method 0 (`sub eax,edi; je`) to `0x648af9`, 1 to `0x648b40`, others to `0x648ae6`, which
returns `+0x20`. `0x648aff` compares `distance` with the position, signed (`jge` to `0x648b3c`, `sub`).
The restart is `0x648b06` (`inflateEnd`), `avail_in` 0, `next_in` the buffer, `0x648b23` (`inflateInit_`) and
`0x648b2f` (`fseek(file, 0x18, SEEK_SET)`); `0x648b37` zeroes the position. `0x648b45` (`jle`) skips
nothing; `0x648b4b` caps each read at 0x1000, `0x648b64` reads, `0x648b6b` fails on 0 and `0x648b72` loops.
The tests run both functions whole on kind-1 objects.

## The zlib it links

`0x730e80` is zlib's `inflateInit_`: it passes window bits 15 to `0x730d60`, `inflateInit2_`. That checks
the version's first character against `1.2.2` and the stream size against 0x38, defaults the allocator
to `0x7348f0` and `0x734910`, allocates a 0x1ba8-byte state and, for 15 bits, sets the wrapper to 1 and the window to 2^15: a zlib
header and an Adler-32 check. `0x732320` is `inflateEnd`. `0x730ea0` opens as zlib 1.2.2's `inflate`
does; the rest of its 4,816 bytes is not read here, and the tests run it on every stream they use. Unlike
later zlib it does not require a dynamic block to code end-of-block, so such a block decodes on.

Addresses: `0x730e80`, `0x730e88`, `0x731994`, `0x7319c9`, `0x730d60`, `0x730d79`, `0x730d85`, `0x730da7`, `0x730db6`, `0x730dc0`, `0x730deb`, `0x730dfc`, `0x732320`, `0x732347`, `0x730ea0`, `0x730ee0`
Confidence: verified
Proof: one
Test: none

`0x730e88` is `mov edx,0xf`. `0x730d79` compares the version's first byte with the image's `1.2.2`; the
client passes `1.1.3`, which passes. `0x730d85` is `cmp [ebp+0xc],0x38`; `0x730da7` and `0x730db6` store
the defaults; `0x730dc0` pushes 0x1ba8. `0x730deb` to `0x730df7` compute `(bits >> 4) + 1` into the state's
`+0x8`, and `0x730dfc` bounds the bits to 8 to 15. `0x732347` frees the state through `zfree`. `0x730ea6` to
`0x730eda` reject a null stream, state or `next_out`, or a null `next_in` with input; `0x730ee0` turns mode
11 into 12, inflate's `TYPE` into `TYPEDO`. After the code lengths `0x731994` tests only for mode 27 (bad) and
`0x7319c9` builds the length table; the image holds no `missing end-of-block` message. A current zlib, and
zlib-rs, fail that block at its header; `storm::bz_missing_eob_divergence` shows the client decoding it.
