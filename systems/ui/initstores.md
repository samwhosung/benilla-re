# ui: stored constants, tables and object constructors

The bodies of the ui static initializers that are not float arithmetic: constants copied or stored, globals zeroed, tables filled and empty containers built. Each ends in `ret`. A body reached by a call thunk (`ui/initthunks`) also has an exit callback (`ui/initexits`). The float bodies are `ui/initfloats`.

Roots: `0x480380`, `0x484590`, `0x4805c0`, `0x484970`, `0x497820`, `0x4ea260`

## Infinity copies

116 bodies copy the dword 0x7f800000, +inf as a float, from a constant of the data section into a global: `mov eax,[c]; mov [g],eax; ret`, 11 bytes. Their thunks are the roots of `ui/initthunks`.

Addresses: `0x480390`, `0x484820`, `0x497730`, `0x4a5a00`, `0x4a9aa0`, `0x4ac890`, `0x4b05f0`, `0x4b1ea0`, `0x4b5140`, `0x4b5f60`, `0x4ba0a0`, `0x4bc5e0`, `0x4bf0d0`, `0x4c0d00`, `0x4c1720`, `0x4c3d10`, `0x4c4bd0`, `0x4c52b0`, `0x4c6c90`, `0x4c9ed0`, `0x4caa70`, `0x4cc070`, `0x4d0930`, `0x4d29c0`, `0x4d4700`, `0x4d51a0`, `0x4d7250`, `0x4daf80`, `0x4de380`, `0x4e1ee0`, `0x4e3000`, `0x4e4050`, `0x4e4f40`, `0x4e7be0`, `0x4ea090`, `0x4efb20`, `0x4f2b80`, `0x4f3da0`, `0x4f4f90`, `0x4f5810`, `0x4f5df0`, `0x4f8450`, `0x4f8cb0`, `0x4fac40`, `0x4fc660`, `0x5009f0`, `0x502770`, `0x5041c0`, `0x505580`, `0x506760`, `0x506f60`, `0x508f50`, `0x515890`, `0x51c2f0`, `0x5236b0`, `0x524280`, `0x528d90`, `0x6ee270`, `0x6f10e0`, `0x6f19f0`, `0x6f2ca0`, `0x7019a0`, `0x705e40`, `0x7298a0`, `0x72cb00`, `0x72da90`, `0x72dfc0`, `0x72e3b0`, `0x72ea50`, `0x72f750`, `0x72fdb0`, `0x72ffa0`, `0x730290`, `0x7305e0`, `0x730b00`, `0x764170`, `0x767250`, `0x769370`, `0x76c7f0`, `0x76c8d0`, `0x76d940`, `0x76fd90`, `0x773810`, `0x778690`, `0x7799a0`, `0x77e8c0`, `0x77fab0`, `0x77fee0`, `0x7830c0`, `0x784140`, `0x785000`, `0x785900`, `0x786cd0`, `0x787660`, `0x789740`, `0x789dc0`, `0x78b380`, `0x78c270`, `0x78d9f0`, `0x78de80`, `0x78f3e0`, `0x7906d0`, `0x791520`, `0x791cf0`, `0x794440`, `0x795ca0`, `0x797080`, `0x799be0`, `0x79b170`, `0x79f3a0`, `0x7a1800`, `0x7a2f80`, `0x7a3160`, `0x7a3230`, `0x7cb100`, `0x7cc330`
Confidence: verified
Proof: one
Test: none

The 116 bodies read 115 different constants, every one 0x7f800000. `0x480390` is the first.

## Immediate stores

78 bodies store one immediate dword in a global: `mov dword [g],imm; ret`. 58 store 0x40a00000, 5.0 as a float; the other 20 store a dword whose top byte is 0xff. The distinct values and their counts:

| Dword | Bodies |
|---|---|
| 0xff000000 | 1 |
| 0xff0000ff | 1 |
| 0xff002e5a | 1 |
| 0xff00ff00 | 2 |
| 0xff40c040 | 1 |
| 0xff808080 | 2 |
| 0xffff0000 | 2 |
| 0xffff2020 | 2 |
| 0xffff8040 | 1 |
| 0xffffd200 | 1 |
| 0xffffff00 | 3 |
| 0xffffff97 | 1 |

The rest: 0xffffffff (2).

Addresses: `0x4845a0`, `0x48cf70`, `0x4a2410`, `0x4a95e0`, `0x4ac750`, `0x4b0260`, `0x4b1ca0`, `0x4b4e80`, `0x4b5de0`, `0x4b8e10`, `0x4bc4a0`, `0x4bef00`, `0x4c0bc0`, `0x4c15e0`, `0x4c3b40`, `0x4c4a90`, `0x4c5170`, `0x4c6340`, `0x4c9d90`, `0x4ca930`, `0x4cbf30`, `0x4d0700`, `0x4d2880`, `0x4d4580`, `0x4d5060`, `0x4d7040`, `0x4daae0`, `0x4dd640`, `0x4e1da0`, `0x4e2ec0`, `0x4e3f10`, `0x4e4e00`, `0x4e7aa0`, `0x4e9f50`, `0x4ef6d0`, `0x4f2330`, `0x4f3c60`, `0x4f4d70`, `0x4f56d0`, `0x4f5cb0`, `0x4f81d0`, `0x4f8b70`, `0x4fab00`, `0x4fc520`, `0x500640`, `0x502630`, `0x504070`, `0x505430`, `0x506610`, `0x506780`, `0x506e10`, `0x508e10`, `0x51c0c0`, `0x521ac0`, `0x524050`, `0x528b60`, `0x528e50`, `0x528e90`, `0x528ed0`, `0x528f10`, `0x528f50`, `0x528f90`, `0x528fd0`, `0x529010`, `0x529050`, `0x529090`, `0x5290d0`, `0x529110`, `0x529150`, `0x529190`, `0x536bb0`, `0x72c3e0`, `0x7cb120`, `0x7cb160`, `0x7cb1a0`, `0x7cb1e0`, `0x7cb220`, `0x7cc1e0`
Confidence: verified
Proof: one
Test: none

`0x506780` is the first of the 20 others. The destinations are globals of the file.

## Zero fills

67 bodies clear globals. 57 clear 2 to 4 consecutive dwords by stores of zero; 10 clear a table by a loop or by `rep stos`, from 40 bytes up to 4096. When a zero fill has an exit callback, the callback frees what the cleared globals hold (`ui/initexits`).

Addresses: `0x4805d0`, `0x484be0`, `0x484c40`, `0x48bb70`, `0x48bbd0`, `0x497750`, `0x497a10`, `0x497a70`, `0x497b20`, `0x497b80`, `0x4a5b10`, `0x4a5bc0`, `0x4a5c30`, `0x4a5ca0`, `0x4a9ac0`, `0x4a9b30`, `0x4a9b90`, `0x4a9bf0`, `0x4ac8b0`, `0x4ac920`, `0x4b0900`, `0x4b1ec0`, `0x4b1f30`, `0x4b1fa0`, `0x4b2010`, `0x4b2080`, `0x4b20f0`, `0x4b2160`, `0x4b5160`, `0x4b51d0`, `0x4bc600`, `0x4bc630`, `0x4c5530`, `0x4c6cb0`, `0x4cc090`, `0x4d0950`, `0x4d29e0`, `0x4d2a50`, `0x4d51c0`, `0x4d5230`, `0x4d7270`, `0x4d72e0`, `0x4db210`, `0x4db270`, `0x4db2d0`, `0x4e3110`, `0x4e7c00`, `0x4ea1f0`, `0x4ea690`, `0x4ea6f0`, `0x4ea740`, `0x4ea910`, `0x4ea970`, `0x4ea9f0`, `0x4efda0`, `0x4f2ba0`, `0x4f3dc0`, `0x4f5e10`, `0x4f5e80`, `0x4fc680`, `0x4fc6f0`, `0x5067c0`, `0x506820`, `0x508f70`, `0x51c5b0`, `0x7019c0`, `0x728f10`
Confidence: verified
Proof: one
Test: none

`0x4805d0` is `xor eax,eax` and stores of `eax`. The loop shape is `mov eax,G; mov ecx,N; xor edx,edx`, stores of `edx` through `eax`, `add eax,step`, `dec ecx; jne`; `0x497a70` is the first.

## Constant tables

8 bodies store a table or a matrix of constants:

| Body | Stores |
|---|---|
| `0x4804f0` | Stores the 4 × 4 identity matrix as 16 floats from 0xb4b2e8 |
| `0x4ea270` | Stores 24 floats from 0xbc7810 as eight triples: (0, 0, 0.5), (0.5, 0, 0), (0.5, 0, 1), (0.5, 1, 0), (0.5, 0.5, 1), (1, 1, 1), (0, 0.5, 0.5), (1, 0, 1) |
| `0x4ea4d0` | Stores 38 floats from 0xbc8310: 1.0 at the word indices 6, 11, 16, 21, 22, 27, 32 and 37, 0.0 at the others |
| `0x4ea790` | Stores the four corners (±h, ±h, 0) of a square as 12 floats from 0xbc81f8, h being the float at 0xbc762c |
| `0x4ea850` | Stores the four corners (±h, ±h, 0) of a square as 12 floats from 0xbc8230, h being the float at 0xbc7630 |
| `0x506880` | Stores the 4 × 4 identity matrix as 16 floats from 0xbe0a50 |
| `0x5291d0` | Stores 7 colours from 0xc0d3c8 in item quality order: 0xff9d9d9d, 0xffffffff, 0xff1eff00, 0xff0070dd, 0xffa335ee, 0xffff8000, 0xffe6cc80 |
| `0x76fab0` | Stores the dwords 0, 0 and the float 1.0 at 0xcf4cf4, 0xcf4cf8 and 0xcf4cfc |

Addresses: `0x4804f0`, `0x4ea270`, `0x4ea4d0`, `0x4ea790`, `0x4ea850`, `0x506880`, `0x5291d0`, `0x76fab0`
Confidence: verified
Proof: one
Test: none

The two corner tables copy a half size from a float that only `0x4edcc1` and `0x4edcd3` write at run time; at start-up it is 0, so the bodies store zeros. The matrices are 1.0 on the diagonal of 16 consecutive floats.

## Hash tables

35 bodies build an empty hash table in a global: link offset 0xc, a list head linked to itself, an end link with bit 0 set, zero fields and 0xffffffff in the last. The exit callback unlinks the entries and frees an array of 12-byte bucket lists (`ui/initexits`).

Addresses: `0x484980`, `0x497be0`, `0x497e30`, `0x498080`, `0x4c1960`, `0x4c52d0`, `0x4dafa0`, `0x4ef0a0`, `0x4efb40`, `0x503950`, `0x504e50`, `0x505fe0`, `0x51c360`, `0x524480`, `0x5246d0`, `0x524920`, `0x536650`, `0x76f5f0`, `0x778310`, `0x782a10`, `0x783620`, `0x78da10`, `0x78ee60`, `0x78fe50`, `0x790c30`, `0x791970`, `0x793f40`, `0x795920`, `0x796d00`, `0x799860`, `0x79a350`, `0x79c3a0`, `0x79eba0`, `0x7a0e80`, `0x7a2c20`
Confidence: verified
Proof: one
Test: none

`0x484980` opens with a loop that unlinks entries through a header still holding 0xdddddddd, then stores the fields. The bodies differ in their globals and in a pointer stored before the list: 23 of them store 0x806cb8.

## List heads

12 bodies build an empty list head: the link offset at -4, the head pointing at itself and the end link, the head's address with bit 0 set. Link offsets: 0x0 (10), 0x8 (1), 0x6c (1). `0x4b5f80` builds the list inside a CStatus, storing 0x806908 in the first dword; its callback runs `CStatus::~CStatus`.

Addresses: `0x497830`, `0x497920`, `0x4b0700`, `0x4b0800`, `0x4b5240`, `0x4b5f80`, `0x508ff0`, `0x5090e0`, `0x51c610`, `0x5242a0`, `0x524390`, `0x7670b0`
Confidence: verified
Proof: one
Test: none

`0x497830` is `mov eax,G; or eax,1; mov [G-4],0; mov [G],G; mov [G+4],eax; ret`.

## Object constructors

10 bodies run a function on a global or build one. Two set up a block pool through `InitBlockPool`, two call a function on each record of a table, five run a constructor or a sizing call on a global, and one stores the fields of a static object. All but `0x4ea250` have an exit callback.

| Body | Does |
|---|---|
| `0x4e4070` | Calls 0x4e4490 on each of 48 records of 16 bytes from 0xbc6040 |
| `0x4ea250` | Stores the result of 0x454a10 for 0x40 in a global |
| `0x4ea430` | Calls 0x4ead10 on each of 512 records of 164 bytes from 0xbc83a8 |
| `0x6ed940` | Runs 0x6ed980 on the object at 0xcee9e8 |
| `0x6eda30` | Runs 0x6ef190 on the object at 0xceea00 |
| `0x6eda70` | Runs 0x6ef670 on the object at 0xcee9bc |
| `0x728fb0` | Runs 0x72b8d0 on the object at 0xcf062c |
| `0x779440` | Builds a static object at 0xcf4d90: the vtable 0x81c898, then 0, -1 and 0 at 0xcf4d94, 0xcf4d98 and 0xcf4d9c |
| `0x76f9f0` | Sets the block pool at 0xcf4ce0 to elements of 300 bytes, 873 to a block, through InitBlockPool |
| `0x76fa50` | Sets the block pool at 0xcf4d10 to elements of 304 bytes, 862 to a block, through InitBlockPool |

Addresses: `0x4e4070`, `0x4ea250`, `0x4ea430`, `0x6ed940`, `0x6eda30`, `0x6eda70`, `0x728fb0`, `0x779440`, `0x76f9f0`, `0x76fa50`
Confidence: verified
Proof: one
Test: none

The pool bodies push the elements per block and then the element size before the call to `0x760390`. The record loops step a pointer by the record size and call the function on each record.

## Empty body

One slot's body is a lone `ret`.

Addresses: `0x4db1f0`
Confidence: verified
Proof: one
Test: none

`0x4db1f0` is reached by a jump thunk and does nothing.
