# storm: the archive format

How the client reads an MPQ archive: the original version-0 format, from the header to the bytes of a
file. The code is Storm's, linked into the exe (`SFile.cpp` and `SFile2.cpp`); no import supplies it.
`mount.md` has the order the archives are opened in and how a name is resolved across them.

Roots: `0x655670`, `0x6477c0`, `0x648460`, `0x648620`, `0x6549a0`

## Header

`0x655bf0` tries offsets of the archive in steps of `0x200`, reading `0x1000` bytes at each, and takes the first
that passes four tests: the magic `0x1a51504d`, a header size of at least `0x20`, and at most `0x10000` hash
entries and `0x10000` block entries. The 32-byte header is the magic, header size, archive size, a 16-bit version
(not tested here), a 16-bit sector shift (low byte used; sector size `0x200 << shift`), then the offsets of the hash
and block tables and their entry counts, relative to the header. A short read or no room for `0x20` bytes fails
with `0x8510006c`.

Addresses: `0x655bf0`, `0x655670`, `0x655690`
Confidence: verified
Proof: one
Test: none

The copy is `rep movs` of 8 dwords into the record at `+0x14c` (`0x655c7a`), the tests are `0x655c7f`,
`0x655c87`, `0x655c9b` and `0x655c9f`, the step is `0x200` (`0x655cbf`; `0x1000` when the open flags have bit 4, with
bit 3 ending the search at the first miss: `test cl,8` at `0x655caa`). The sector size is `0x200 << [rec+0x15a]`
(`0x655d0a`). `0x655670` calls `0x655690`, which allocates the 0x2a0-byte record zero-filled and then calls
`0x655bf0`.

## Hash and block tables

The hash table, `count × 16` bytes, is decrypted with the type 3 hash of `(hash table)` and the block table
with that of `(block table)`. A hash entry is name hash A, name hash B, a 16-bit locale, a 16-bit platform whose high byte is cleared at load, and
a 32-bit block index; `0xffffffff` ends a probe and `0xfffffffe` is a deleted slot. A block entry is the file offset,
packed size, unpacked size and flags. The client expands each to `0x2c` bytes, adds the header's position to every
nonzero offset, and sets bit 0 of the cleared hash byte of each block whose flags carry `0x02000000`.

Addresses: `0x655bf0`, `0x656130`, `0x651d90`, `0x655fde`
Confidence: verified
Proof: one
Test: none

`0x655e15` and `0x655f05` call `DecryptBlock` on the two tables with the keys from the inline hash loops at
`0x655dc0` and `0x655eb0`, whose strings are at `0x866844` and `0x866834`. `0x655e2e` stores 0 in byte `+0xb` of each hash
entry; `0x655fc8` tests `0x2000000` in each expanded block's flags and `0x655fde` sets that byte's bit 0 on the
hash entry `0x655f83` to `0x655fb2` records for the block. `0x656130` writes the `0x2c`-byte entries from the last
down. A block index at or above the block count (`0x655f97`) fails the open with `0x85100083`.

## Crypt table

`table` below is Storm's crypt table, `0x500` dwords built at the first use: from the seed 1,048,577, twice per entry
`seed = (seed·125 + 3) mod 2,796,203`; the entry is the first value's low 16 bits shifted up, OR the second's, which
is the next seed. Five groups of 256 entries, `0x1400` bytes. The fill is entry-major: for `i` = 0 to 255, for `g` = 0 to 4, `table[g·256 + i]` takes the next pair, so one seed run feeds the five groups' entry `i` in turn.

Addresses: `0x654f60`, `0xc53fbc`
Confidence: verified
Proof: one
Test: none

`0x654f85` seeds the generator; `0x654faa` to `0x654fb7` multiplies by `0x7d` (125), adds 3 and divides by the modulus
(`div`); `0x654fdd` to `0x654fe8` packs the two values; `0x654fbe` steps the group by `0x400` bytes (the store `0x654feb` is `[edi + table − 0x400]`, five times per entry, `0x654fa3` counting with `ebx` = 5; the outer index `[ebp-4]` runs 0 to `0x3fc` by 4, `0x654ff7` to `0x654fff`), and the allocation
is `0x1400` bytes (`0x654f80`).

## Name hash

A name hashes to a value of type 0 to 3. The state is two dwords, `0x7fed7fed` and `0xeeeeeeee`. For each byte `c` of
the name, upper-cased by `toupper` (`0x41089b`) with `/` turned into `\`: `first = table[type·0x100 + c] ^ (first +
second)`, then `second = c + second·0x21 + first + 3`. The value is `first`. Type 0 picks the slot (`& (count − 1)`),
types 1 and 2 are name hashes A and B, type 3 is a key.

Addresses: `0x6530d0`, `0x41089b`, `0xc53fbc`, `0x883b08`
Confidence: verified
Proof: one
Test: none

The hash loop at `0x654ba0` and its copies at `0x654bf0`, `0x655dc0` and `0x656828` read `[table + c·4 + type·0x400]`; the
uppercasing is `call 0x41089b` then `cmp eax,0x2f` (`0x654bb6`). While the word `0x883b08` is 0 (`0x41089e`),
`0x41089b` subtracts `0x20` from `a` to `z` (`0x4108aa` to `0x4108b4`) and returns every other value; its 21 operands in
the image are all reads. Nothing else is applied to the name: no leading or trailing separator is removed, doubled
separators stay, and a byte of `0x80` or above is not folded.

## Hash lookup

The probe starts at slot `hash0 & (count − 1)` and walks forward with wrap, to the first free slot or back to its
start. A slot matches when its two name hashes equal A and B and its block index is not `0xfffffffe`. A slot whose
locale and platform equal the requested ones (both 0 here) ends the probe at once. Otherwise a slot with locale 0 or
the requested locale, and platform 0 or the requested one, becomes the candidate, and a later such slot replaces
it. The result is the exact match, else the last candidate, else a miss.

Addresses: `0x6549a0`, `0x866520`, `0x866522`, `0x657420`, `0x657480`
Confidence: verified
Proof: one
Test: none

`0x654c55` starts at `(count − 1) & hash0`; `0x654c72` to `0x654c85` compare A, B and the deleted index; the exact
test is at `0x654c87` to `0x654c97`; the candidate logic at `0x654c99` to `0x654cb7`; the stop at a free slot `0x654cd2`.
The requested locale is the word at `0x866520` and the platform the byte at `0x866522`, both 0 in the file; their
only writers, `0x657420` and `0x657480`, are unreached.

## Block flags and sectors

Flags: `0x100` imploded, `0x200` compressed, `0x10000` encrypted, `0x20000` key adjusted by position, `0x1000000`
one unit, `0x2000000` delete marker, `0x80000000` exists. A one-unit file is one sector; any other is cut into
sectors of the sector size. A compressed or imploded file starts with `count + 1` offsets (dwords) relative to
the file; sector `i` lies between `table[i]` and `table[i + 1]`. A sector whose packed size is not below its
unpacked size is stored as it is; otherwise `0x200` runs the codec dispatch and `0x100` the explode.

Addresses: `0x651450`, `0x656690`, `0x661a80`
Confidence: verified
Proof: one
Test: none

`0x656690` counts the sectors (`0x6568a2` to `0x6568ad`; one for `0x1000000`, `0x65688b`) and stores the file's key
at `+0x160`, the count at `+0x16c`. `0x651450` reads the table at `0x651549` when the flags have any of `0xff00` and
the file is not one unit, checks that its entries do not decrease (`0x6515b3`) and decrypts it with the key − 1
(`0x65158f` to `0x65159a`). The count is `⌈unpacked size / sector size⌉`. The raw copy is `cmp esi,edi; jbe` (`0x651982`, `0x651987`), the codec call `0x6519f4`,
and the test `and ecx,0xff00; cmp ecx,0x200` at `0x651993` to `0x651999`.

## Decryption

A block is decrypted dword by dword. The running value starts as `0xeeeeeeee`; for each dword the key stream is
added `table[0x400 + (key & 0xff)]`, the dword is XORed with `(running + key)`, `running = running·0x21 + plain + 3`,
and `key = ((~key << 0x15) + 0x11111111) | (key >> 0x0b)`. A file's key is the type 3 hash of its base name:
the name after the last `:` and then after the last `\`, never `/`. With `0x20000` the key becomes
`(key + offset − header position) ^ unpacked size`. The sector table uses the key − 1, and sector `i` the key + `i`.

Addresses: `0x651aa0`, `0x656690`
Confidence: verified
Proof: one
Test: none

`0x651ac5` adds the table dword; `0x651ace` to `0x651ad8` update the running value; `0x651ade` to `0x651af1` the key.
`0x6567fa` and `0x65680a` find the last `:` and `\` with `0x64a420`; `0x65687b` to `0x656888` adjust the key; `0x651892` adds
the sector index (`add edx,edi`) before `0x65189c`.

## Codecs

A compressed sector's first byte is a mask of codecs. `0x661a80` accepts only the bits `0x01` (`0x661350`), `0x02`
(`0x660800`), `0x10` (`0x660c10`), `0x20` (`0x660ac0`), `0x40` (`0x660f60`) and `0x80` (`0x6611e0`); any other bit fails
the call. It applies the codecs of the set bits in the order `0x10`, `0x02`, `0x01`, `0x20`, `0x80`, `0x40`. Equal
packed and unpacked sizes copy the bytes.

Addresses: `0x661a80`, `0x80fa1c`
Confidence: verified
Proof: one
Test: none

The table at `0x80fa1c` holds six `{mask, function}` pairs in the order `0x40, 0x80, 0x20, 0x01, 0x02, 0x10`. The functions are, by the names of the format, ADPCM mono, ADPCM stereo, sparse, Huffman, zlib and one unnamed here. The bit test
walks it from `0x80fa44` down to `0x80fa1c` (`0x661af8` to `0x661b13`) and fails if a bit is left (`0x661b1d`); the
application loop starts at `0x80fa48` and steps by `−8` to `0x80fa20` (`0x661b64`, `0x661bd0` to `0x661bd3`), calling each
function through `call [edi]` (`0x661bc2`). The copy case is `cmp ebx,eax` at `0x661abd`.

## Freeing a loaded file

`SFile::Unload(buffer)` (`0x648700`) frees a buffer with `SMemFree`, tagged with `SFile2.cpp` and
line 1262, and returns 1. Callers pass it the buffer `SFile::Load` returns, as `0x6f1200` does;
the census finds 27 call sites and no stored address.

Addresses: `0x648700`, `0x648713`, `0x648620`, `0x646430`, `0x6f1238`, `0x6f1254`
Confidence: verified
Proof: one
Test: none

`0x648700` pushes 0, `0x4ee` and the path `E:\build\buildWoW\Storm\Source\SFile2.cpp`, then the
buffer, and calls `SMemFree` (`0x648713`); `ret 4`. In `0x6f1200` the buffer `SFile::Load` wrote at
`0x6f1238` is passed to it at `0x6f1254`. The name pairs it with `SFile::Load`, a community name,
whose class it takes.
