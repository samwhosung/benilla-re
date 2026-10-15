# warden: module decoding and loading

How a module blob becomes a running image: decrypted, checked against a fixed public key, inflated, mapped,
relocated, linked and protected, and how it is unloaded. The blob's RC4 layer, its SHA-1 and RSA check and its
zlib stream are the exe's Storm and zlib routines (`0x66ec20`, `0x66ecc0`, `0x66cdd0`, `0x66ce40`, `0x66cf10`,
`0x734810`); this page states what the client feeds them and what it takes from them. The loader `0x7a79d0` is
called from one place only.

Roots: `0x6ca360`

## Decoding a module

`LoadWardenModule(blob, length, key)` returns a 12-byte module object or null. A length under `0x108` returns
null at once. Otherwise it copies the blob, decrypts the copy whole with RC4 under the 16-byte key, checks the
signature (next entry), reads the first dword of the decrypted copy as the decompressed size, inflates the zlib
stream that follows it into a buffer of that size, and gives the buffer to the loader. It frees the copy and
the buffer and returns the object; any failure frees what it holds and returns null. Both callers pass
`ecx` the blob, `edx` its length and the key on the stack. The loader is given the length and never reads it.

Addresses: `0x6ca360`, `0x6ca36e`, `0x6ca393`, `0x6ca3c8`, `0x6ca3e2`, `0x6ca462`, `0x6ca498`, `0x6ca4cf`, `0x6ca525`, `0x6ca53d`, `0x6ca541`, `0x6ca557`, `0x6ca55e`, `0x6ca56d`, `0x6ca5b1`, `0x66ec20`, `0x66ecc0`, `0x734810`, `0x6ca30e`, `0x6ca7f4`
Confidence: verified
Proof: one
Test: `warden::decoder_real_signature`, `warden::decoder_after_the_signature`, `warden::decoder_loader_failure`

`0x6ca36e` compares the length with `0x108`. `0x6ca393` allocates the copy (line `0x16a`). `0x66ec20` is
the RC4 key schedule (key, 16 bytes, to a 0x102-byte state in the frame) and `0x66ecc0` runs the stream over the whole copy,
in place (`0x6ca3e2`). `0x6ca498` allocates the output, `[ebx]` bytes; `0x6ca4cf` is zlib's `uncompress`
(`0x734810`, `inflateInit`, `inflate` with finish, `inflateEnd`) over `copy+4` for `length-4` bytes: a
stream shorter than the declared size succeeds with the length it has, a longer one fails. `0x6ca541` calls the
loader with the buffer; `0x6ca53d` pushes the length too, which the loader ignores. A 0 return frees the buffer (`0x6ca557`), unloads the object (`0x6ca55e`) and frees it (`0x6ca56d`). The test runs the
whole function with the allocator, the loader, `SStrUpper` and zlib's allocator stubbed and Storm's RC4, SHA-1,
RSA and zlib running.

## The signature check

The decrypted copy must end with 260 bytes: the four bytes `NGIS` and a 256-byte signature. SHA-1 of the copy less
those 260 bytes, followed by the upper-cased name `MAIEV.MOD`, is signed with RSA: the signature to the power
65537 modulo the 256-byte modulus at `0x811330` (both little-endian) must equal the digest, then 235 bytes `0xbb`,
then `0x0b`. A failure frees the copy and returns null. The name is built at run time: `M`, then each byte of
the table `0x811320` after its first times `0xb9` modulo 256, until a product is 0.

Addresses: `0x6ca3e7`, `0x6ca3f0`, `0x6ca401`, `0x6ca407`, `0x6ca416`, `0x6ca427`, `0x6ca43c`, `0x6ca450`, `0x6ca462`, `0x6ca478`, `0x811320`, `0x811330`, `0x811430`, `0x66cdd0`, `0x66ce40`, `0x66cf10`, `0x64b460`
Confidence: verified
Proof: two
Test: `warden::signature_name_loop`, `warden::decoder_real_signature`

`0x6ca3e7` stores `M` and `0x6ca3f0` to `0x6ca401` multiply (`imul cl` with `cl = 0xb9`) and store; the table's bytes
after the first are `e9 31 0d 26 9e 55 67 84`, which give `aiev.mod`. `0x6ca407` upper-cases the string
(`SStrUpper`). `0x66cdd0` starts a signature object (key size `0x100`, tail 4) that holds back the last `0x104`
bytes of what it is fed, so the three updates, the copy less 260 bytes (`0x6ca427`), the name (`0x6ca43c`) and
the last 260 bytes (`0x6ca450`), hash the first two. `0x66cf10` requires `NGIS` (`0x5349474e`) at the tail, runs
the public operation and compares 256 bytes against the digest filled with `0xbb` ending `0x0b`. For the built-in
module the digest is `2fc54335c8e2866a5ac35c32de17293bf5ef9816`; the exponent at `0x811430` is `0x10001`.

## The module image

A decompressed module is a header, a section table and a stream of runs. Header values are offsets from the
image base. At `+0x28` each section has 12 bytes: an address, a size and a Win32 protection. The runs follow
the table. A run is a 16-bit length; runs alternate between copied (the bytes follow) and skipped (zeros),
starting with a copy at the first section's address and ending when the address reaches the image size.

| offset | field |
|---|---|
| `+0x00` | image size, under `0x80000000` |
| `+0x04` | not read |
| `+0x08` | relocation stream |
| `+0x0c` | relocation count |
| `+0x10` | export table |
| `+0x14` | export count |
| `+0x18` | ordinal of the first export |
| `+0x1c` | import table |
| `+0x20` | import count |
| `+0x24` | section count |

Addresses: `0x7a79d0`, `0x7a7a15`, `0x7a7add`, `0x7a7af3`, `0x7a7afc`, `0x7a7b18`, `0x7a7b32`, `0x7a7b7d`
Confidence: verified
Proof: two
Test: `warden::loader_whole`

`0x7a7a15` compares the first dword with `0x80000000`. `0x7a7add` copies ten dwords of header. `0x7a7af3` puts the
run stream after `section count * 12` bytes of table; `0x7a7afc` takes the first run's address from the table's
first dword. `0x7a7b18` is the loop head (the address against `base + size`), `0x7a7b32` tests the copy flag and
`0x7a7b7d` toggles it. Unpacking the built-in module's runs from its bytes consumes the stream exactly (6584 of
6584 bytes).

## Relocations

A relocation entry whose first byte is below `0x80` is two bytes, big-endian, added to the previous offset (the
first adds to 0). One of `0x80` or more is four bytes, big-endian, taken whole with the top bit kept. Each entry adds
the load base to the dword at base plus offset, modulo 2^32.

Addresses: `0x7a79d0`, `0x7a7bc1`, `0x7a7bc5`, `0x7a7be7`, `0x7a7c05`
Confidence: verified
Proof: two
Test: `warden::loader_whole`, `warden::loader_relocation_forms`

`0x7a7bc1` (`jns`) separates the two forms; the four-byte form builds `b0<<24 | b1<<16 | b2<<8 | b3` with `b0`
unmasked (`0x7a7bc5` to `0x7a7be0`), and `0x7a7c05` adds the base to the dword at `base + offset`. The short form
adds `b0<<8 | b1` to the running offset (`0x7a7be7` to `0x7a7bf6`), so a first byte of `0x7f` is a short entry
and `0x80` the first long one. A long entry's offset is `0x80000000` or more, so no long entry names a dword of
the image: the exe is not large-address-aware (PE characteristics `0x10f`, without `0x20`), and by Windows' rule a
base then lies below `0x80000000`. A base plus offset below `2^32` is an address at or above `0x80000000`, kernel
space, where the write faults (the loader's handler catches it, next entry); at `2^32` or more it wraps to an address
below the image, in the process's own memory, where the write succeeds. The tests run short entries with a first
byte `0x7f` and a long entry that wraps to mapped memory, against the exe's code with the expected values written in
the test.

## Imports and exports

An import is eight bytes: an offset from the base to a library name, and an offset to a slot list. A slot is 0 to
end, a value with the top bit an ordinal, otherwise an offset from the base to a name. An export is a dword offset,
indexed from the first ordinal.

Addresses: `0x7a79d0`, `0x7a7c25`, `0x7a7c3b`, `0x7a7c60`, `0x7a7c6e`, `0x7a7c70`, `0x7a7c77`, `0x7a7c84`, `0x7a7c86`, `0x7a7c97`, `0x7a7cb7`
Confidence: verified
Proof: one
Test: `warden::loader_whole`, `warden::loader_library_fails_after_some_load`

`0x7a7c25` is the import loop head; `0x7a7c3b` adds the base to the library name's offset and `0x7a7c60` to the
slot list's. `0x7a7c6e` is the slot loop: zero ends, a set sign bit goes to the ordinal arm (`0x7a7c70`, mask
`0x7fffffff`), and any other value is added to the base (`0x7a7c84` to `0x7a7c86`) for a name. The test builds
modules of random sizes, sections, runs, relocations, imports and exports and runs the loader against this crate's.

The call at `0x7a7c77` goes through the delay-load slot of `GetProcAddress` (slot 0x88249c, whose stub at `0x415611`
pushes `kernel32.dll` and `GetProcAddress`). `0x7a7c97` adds 1 to the module object's library count after each
library's slots are filled, so a library that fails leaves the count of those loaded before it, which the unloader
frees (the test fails the first, second and third in turn).

## Loading an image

`0x7a79d0(image, length)` allocates `size` bytes (`VirtualAlloc`, commit, read-write), copies the header, unpacks the
runs, relocates, resolves the imports, protects each section and returns 1. It never reads `length`. A size of
`0x80000000` or more builds `Improper header received: [ ` and the first 32 bytes as `%02X ` and calls
`SErrDisplayAppFatal`, which ends the client. A null allocation or a library that does not load returns 0 after the
unloader runs.

Addresses: `0x7a79d0`, `0x7a7a0a`, `0x7a7ab2`, `0x7a7ac1`, `0x7a7c44`, `0x7a7c77`, `0x7a7c8a`, `0x7a7c97`, `0x7a7cf6`, `0x7a7cfc`, `0x7a7d11`, `0x7a7d28`, `0x7a7d33`, `0x7a7d4b`, `0x64d0e0`, `0x64cfa0`, `0x64d008`, `0x64cc80`, `0x73fec4`
Confidence: verified
Proof: one
Test: `warden::loader_whole`, `warden::loader_library_missing`, `warden::loader_library_fails_after_some_load`, `warden::loader_alloc_failed`, `warden::loader_header_too_large`

Each library is `LoadLibraryA`; each slot is `GetProcAddress`, stored as returned, with no null check. After the
imports each section is `VirtualProtect`ed, and one with an execute bit has its instruction cache flushed. The loader
then calls `VirtualFree` from the relocation table's offset to the end of the image with `MEM_RELEASE` and a nonzero
size, which Windows rejects.

`0x7a7a0a` loads the image pointer, the only argument it reads. `0x7a7ac1` calls `VirtualAlloc(0, size, 0x1000, 4)`.
`0x7a7ab2` leaves the header error for the exit `0x7a7d4b`, which unloads and returns 0 if the dialog returns; the
dialog `0x64d080` is not read, and `0x64cfa0` then calls `0x64cc80` with `ecx = 1` (`0x64d008`), which pushes it and
calls `ExitProcess` through its import. `0x7a7cf6` is `VirtualProtect(base+address, size, protection, &old)`;
`0x7a7cfc` tests the low byte of the protection against `0xf0`, and `0x7a7d11` is the flush. `0x7a7d28` pushes
`0x8000` (`MEM_RELEASE`; `MEM_DECOMMIT` is `0x4000`), then the image size less the relocation offset, and `0x7a7d33`
calls `VirtualFree` at `base + offset`, only when the offset is below the size. That `VirtualFree` rejects
`MEM_RELEASE` with a size is the API's documented rule, not read from the exe.

## Faults while loading

The client makes no bounds check, and a fault does not end it. A run, a relocation offset, an import name or a section
can reach past the blob or the image. The loader runs inside an exception frame whose filter handles every exception;
a fault ends in the unloader and a return of 0. An access that does not fault succeeds without a sign: the image is
`VirtualAlloc`ed in whole pages, and a run copied past `size` lands in the rest of the last page.

Addresses: `0x7a79d3`, `0x7a79e6`, `0x7a79fc`, `0x7a7b18`, `0x7a7c05`, `0x7a7d39`, `0x7a7d42`, `0x7a7d48`, `0x7a7d4b`, `0x7a7d5c`, `0x81d760`, `0x40b140`
Confidence: verified
Proof: one
Test: `warden::loader_run_past_the_image_end`, `warden::loader_relocation_forms`

The frame is opened at `0x7a79d3` to `0x7a79e6` (`-1`, the scope table `0x81d760`, the handler `0x40b140`, `fs:0`).
The table is `{-1, 0x7a7d42, 0x7a7d48}`; the filter at `0x7a7d42` is `mov eax,1; ret`, so every exception is handled.
The handler at `0x7a7d48` restores `esp` and goes to `0x7a7d4b`, the exit of every failure: `[ebp-0x20]`, 1 from
`0x7a79fc`, is cleared only at `0x7a7d39`, the end of a load, so a failure runs the unloader (`0x7a7d5c`) and returns
0. `0x7a7b18` tests the address against the end only between runs, so a run is copied whole. A relocation that wraps
writes to the process (`relocations`). The tests run the copy past `size` and the wrap; a fault is not run, as the
emulator has no exception handling.

## Unloading and exports

`0x7a7d80` frees a module's libraries, last loaded first (`GetModuleHandleA` of each import name, then
`FreeLibrary`), then calls `VirtualFree(base, size, MEM_RELEASE)`, which Windows rejects for a nonzero size, so
the image stays mapped. It zeroes the base and size. `0x7a7de0` returns the address of export `n`:
the image base plus the table's dword `n` minus the first ordinal, or 0 when that index is not below the export count or
the base is 0.

Addresses: `0x7a7d80`, `0x7a7dac`, `0x7a7daf`, `0x7a7dc9`, `0x7a7de0`, `0x7a7dfa`, `0x7a7e05`
Confidence: verified
Proof: one
Test: `warden::loader_whole`

`0x7a7d97` loads the import slot of `GetModuleHandleA` and `0x7a7daf` calls `FreeLibrary` through its slot in a
loop that decrements the library count; `0x7a7dc9` calls `VirtualFree` through its slot with `0x8000`.
The rejection is the API's documented rule for `MEM_RELEASE` with a size, not read from the exe. `0x7a7dfa` compares
the index with the export count (`jb`, unsigned), and `0x7a7e05` indexes the table by 4. The loader's own failure
path calls the unloader (`0x7a7d5c`).
