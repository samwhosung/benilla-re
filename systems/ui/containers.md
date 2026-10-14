# ui: list, table and array helpers

The ui object files instantiate the client's list, hash table and array templates for their own types, and the compiler repeats each template function once for every instance. The rows here are byte clones, once their addresses are masked (`tools/clones.py`), of rows read in other systems (three clusters: in other ui docs), so each does what its clone does with its own lists, tables and arrays. 550 rows in ten groups. The types they serve are named by the tag strings the free calls pass, such as `.?AUSAVEDVARIABLE@@`.

Roots: `0x4846c0`, `0x495b70`, `0x484690`, `0x497710`, `0x496be0`, `0x4969e0`

## Links

Intrusive links are two dwords in the node; the end of a list is a link marked by bit 0. 139 rows are the link operations:

| Function | Rows | Does |
|---|---|---|
| Unlink | 47 | Unlinks a link from its list and clears it |
| NextLink | 33 | Returns the address a link names, resolving negative offsets through the list's offset |
| InitEndLink | 22 | Points a list's end link at itself, marked by bit 0 |
| LinkOrEnd | 20 | Returns a node's link, or the list's end link for a null node |
| Relink | 13 | Unlinks a node and links it again into a list by a placement argument |
| LinkNode | 2 | Unlinks a node and links it after another for where 1, else before it; a null neighbour means the head |
| EndOrNode | 2 | Returns its argument, or the list's end link for 0 |

Addresses: `0x484220`, `0x484690`, `0x4846c0`, `0x484700`, `0x496e60`, `0x496ea0`, `0x496ec0`, `0x497210`, `0x4974b0`, `0x4a3090`, `0x4a3150`, `0x4a3800`, `0x4a3a60`, `0x4a3d40`, `0x4a3d70`, `0x4a40a0`, `0x4a40d0`, `0x4a4200`, `0x4a4220`, `0x4a4250`, `0x4a4270`, `0x4a4330`, `0x4a44e0`, `0x4a4500`, `0x4a4800`, `0x4a4dd0`, `0x4a5380`, `0x4b1960`, `0x4b1990`, `0x4b1de0`, `0x4b5ab0`, `0x4b5af0`, `0x4b5eb0`, `0x4b88c0`, `0x4b8900`, `0x4b9200`, `0x4b9230`, `0x4b9260`, `0x4b9280`, `0x4b9340`, `0x4b9360`, `0x4b98a0`, `0x4b9b20`, `0x4c3c10`, `0x4c3c50`, `0x4c3c80`, `0x4c6410`, `0x4c6450`, `0x4c6470`, `0x4c6580`, `0x4c6880`, `0x4dda10`, `0x4dda50`, `0x4dda70`, `0x4ddc70`, `0x4ddf70`, `0x4f1f00`, `0x4f2400`, `0x4f2430`, `0x4f2720`, `0x520a60`, `0x520aa0`, `0x520ec0`, `0x5212b0`, `0x521360`, `0x521390`, `0x521450`, `0x521470`, `0x521580`, `0x521740`, `0x521760`, `0x5219d0`, `0x521eb0`, `0x521ec0`, `0x522160`, `0x5225f0`, `0x526010`, `0x526a20`, `0x526a50`, `0x526a90`, `0x527280`, `0x5272b0`, `0x527320`, `0x527360`, `0x5273d0`, `0x527410`, `0x527430`, `0x527810`, `0x527960`, `0x527c60`, `0x527db0`, `0x528440`, `0x6efb30`, `0x6efd80`, `0x6efdc0`, `0x6f01d0`, `0x6f0210`, `0x6f0230`, `0x6f02f0`, `0x6f0310`, `0x6f07e0`, `0x6f0930`, `0x6f0c50`, `0x6f0da0`, `0x7055e0`, `0x705a00`, `0x705d10`, `0x72b950`, `0x72bca0`, `0x72bce0`, `0x72c130`, `0x72d6d0`, `0x72d710`, `0x72d750`, `0x72d790`, `0x72e5e0`, `0x72e620`, `0x72e660`, `0x72f290`, `0x7669a0`, `0x7669d0`, `0x766a60`, `0x76c320`, `0x76c360`, `0x76c3a0`, `0x76c3c0`, `0x76c3e0`, `0x773240`, `0x773370`, `0x7845a0`, `0x7845d0`, `0x784730`, `0x784750`, `0x784c90`, `0x784de0`, `0x789020`, `0x789190`, `0x78b140`, `0x78b180`
Confidence: verified
Proof: one
Test: none

The shapes are the clones of `0x4237a0` (core), `0x420980` (core), `0x422f90` (core), `0x459ac0` (sound), `0x45e770` (sound), `0x67d740` (lighting), `0x422d80` (core), each a verified row read in its own system. A row's own list is the one its caller names; the function reads only the link it is given, so the rows differ only in the addresses they mask.

## Lists

90 rows build, clear and re-offset lists. A list head holds the link offset, the head link and the end link:

| Function | Rows | Does |
|---|---|---|
| ClearList | 23 | Unlinks every node of a list, then its head |
| ClearBucket | 19 | Unlinks every node of a bucket list |
| FreeBucketNodes | 14 | Unlinks and frees the nodes of a bucket list |
| InitUnsetList | 13 | Makes an empty list with the unset link offset 0xdddddddd |
| InitList | 10 | Makes an empty list: link offset 0 and an end link marked by bit 0 |
| SetLinkOffset | 7 | Sets a list's link offset, unlinking every node first when it differs |
| Clear4 | 2 | Zeroes four dwords at this and returns this |
| Clear2 | 2 | Zeroes two dwords at this and returns this |

Addresses: `0x481780`, `0x496be0`, `0x496c70`, `0x497430`, `0x497460`, `0x4a30d0`, `0x4a36e0`, `0x4a3940`, `0x4a3ba0`, `0x4a4720`, `0x4a4750`, `0x4a4ab0`, `0x4a4b60`, `0x4a4d80`, `0x4a4f20`, `0x4a52a0`, `0x4a52d0`, `0x4a5720`, `0x4b8cd0`, `0x4b8d90`, `0x4b9070`, `0x4b9100`, `0x4b9580`, `0x4b9630`, `0x4b9850`, `0x4b9aa0`, `0x4b9ad0`, `0x4c3750`, `0x4c5f50`, `0x4c6530`, `0x4c67a0`, `0x4c67d0`, `0x4dd710`, `0x4ddb30`, `0x4dde90`, `0x4ddec0`, `0x4ef7a0`, `0x4f1f40`, `0x4f2640`, `0x4f2670`, `0x4f29b0`, `0x520680`, `0x520690`, `0x520d30`, `0x520dc0`, `0x521000`, `0x521090`, `0x521190`, `0x521530`, `0x521e80`, `0x5220e0`, `0x522110`, `0x5222d0`, `0x522380`, `0x5225a0`, `0x5227e0`, `0x522810`, `0x522be0`, `0x522c10`, `0x526c50`, `0x526ef0`, `0x527160`, `0x5272d0`, `0x527380`, `0x5274f0`, `0x527730`, `0x527760`, `0x527b80`, `0x527bb0`, `0x528220`, `0x528250`, `0x6ef9f0`, `0x6efba0`, `0x6efe00`, `0x6efe90`, `0x6efff0`, `0x6f0080`, `0x6f0730`, `0x6f0790`, `0x6f0ba0`, `0x6f0c00`, `0x705970`, `0x705a70`, `0x705c70`, `0x728ff0`, `0x72b990`, `0x784610`, `0x784970`, `0x784a20`, `0x784c40`
Confidence: verified
Proof: one
Test: none

The clones are `0x423530` (core), `0x45a520` (sound), `0x45a1e0` (sound), `0x45a1b0` (sound), `0x422d50` (core), `0x45e300` (sound), `0x422f80` (core), `0x408a30` (loadingscreen).

## Hash tables

123 rows are the functions of the hash table template: the bucket array is a vector of 12-byte list heads, the main list threads every node, and the mask is the bucket count less one:

| Function | Rows | Does |
|---|---|---|
| EmptyTable | 19 | Empties the table, frees its buckets and sets the mask to -1 |
| ReallocBuckets | 18 | Reallocates a bucket array, moving each bucket list |
| DestroyTable | 17 | Destroys a table: empties it, frees its buckets and frees it when bit 0 of the argument is set |
| FreeBuckets | 17 | Empties a table's bucket lists and frees the bucket array |
| ResizeBuckets | 14 | Sets a bucket array's count, rounding the capacity to its quantum below 21 elements |
| GrowBuckets | 10 | Grows a bucket array by a count, rounding the capacity to its quantum |
| GrowOnLongChain | 8 | Doubles a table's buckets and rehashes when the run counter passes 13 on a chain walk, below mask 0x1fff |
| RehashTable | 7 | Moves a hash table's nodes into a resized bucket array |
| ClearTable | 7 | Unlinks every node of a table and, unless its argument is set, frees each |
| InitTable | 4 | Builds an empty hash table with mask -1 |
| ShouldGrow | 2 | Adds a bucket's chain walk to the run counter and grows the table when it passes 13 |

Addresses: `0x4969e0`, `0x496b20`, `0x496ce0`, `0x497240`, `0x4974c0`, `0x497600`, `0x4a2730`, `0x4a2940`, `0x4a2b50`, `0x4a2db0`, `0x4a2e70`, `0x4a2f30`, `0x4a3460`, `0x4a3770`, `0x4a3840`, `0x4a39d0`, `0x4a3aa0`, `0x4a3c30`, `0x4a4530`, `0x4a4810`, `0x4a4b90`, `0x4a4de0`, `0x4a4f70`, `0x4a50b0`, `0x4a5390`, `0x4a54d0`, `0x4a55e0`, `0x4a57d0`, `0x4a58e0`, `0x4b8490`, `0x4b8680`, `0x4b8740`, `0x4b8800`, `0x4b89f0`, `0x4b8ee0`, `0x4b8f70`, `0x4b9170`, `0x4b9660`, `0x4b98b0`, `0x4b9b30`, `0x4b9c70`, `0x4b9e10`, `0x4b9f20`, `0x4c35d0`, `0x4c3690`, `0x4c37e0`, `0x4c5dd0`, `0x4c5e90`, `0x4c5fe0`, `0x4c65b0`, `0x4c6890`, `0x4c69d0`, `0x4c6b10`, `0x4dd0f0`, `0x4dd1d0`, `0x4dd7a0`, `0x4ddca0`, `0x4ddf80`, `0x4de0c0`, `0x4de200`, `0x4f1d80`, `0x4f1e40`, `0x4f1fd0`, `0x4f2450`, `0x4f2730`, `0x4f2870`, `0x4f2a00`, `0x51fdd0`, `0x51ffe0`, `0x520490`, `0x5206a0`, `0x520760`, `0x520820`, `0x5208e0`, `0x5209a0`, `0x520ae0`, `0x520e30`, `0x520f00`, `0x521100`, `0x521220`, `0x521c90`, `0x521ef0`, `0x5223b0`, `0x5228c0`, `0x522cc0`, `0x522e00`, `0x522f40`, `0x523080`, `0x5231c0`, `0x5232d0`, `0x523440`, `0x5261c0`, `0x5263c0`, `0x5265b0`, `0x526670`, `0x526730`, `0x5267f0`, `0x526ce0`, `0x526f80`, `0x5271f0`, `0x527540`, `0x527820`, `0x527990`, `0x527c70`, `0x527dc0`, `0x527f00`, `0x528470`, `0x528580`, `0x528690`, `0x5287d0`, `0x6ef190`, `0x6ef5b0`, `0x6ef670`, `0x6ef900`, `0x6f03d0`, `0x6f0460`, `0x6f0eb0`, `0x6f0fc0`, `0x7843d0`, `0x7844e0`, `0x7846a0`, `0x784a50`, `0x784e70`
Confidence: verified
Proof: one
Test: none

The clones are `0x459430` (sound), `0x45a570` (sound), `0x459530` (sound), `0x459940` (sound), `0x45a2a0` (sound), `0x45a3e0` (sound), `0x63e9f0` (cvar), `0x459630` (sound), `0x459fc0` (sound), `0x45d150` (sound). The table in `0x4969e0` empties its main list, then each bucket list, frees the bucket array and sets the mask to -1; the growth function `0x497240` refuses at a mask of 0x1fff or more, then keeps a counter at `+0x10` that persists between calls: each call takes 3 off it (to 0 at the least, `0x497257` to `0x497267`), each node walked adds 1 (`0x4972a4`), and a count above 13 (`0x4972a7`) doubles the buckets and returns 1. A table can therefore grow on a short chain. `ShouldGrow` (`0x6f03d0`, `0x6f0460`, the same bytes once addresses are masked) does the same with the call to `0x6f04f0` for the doubling.

## Hash table nodes

12 rows add, allocate, delete and free the nodes of a hash table's lists:

| Function | Rows | Does |
|---|---|---|
| AddNode | 4 | Adds a node for a hash to a table, growing the buckets after a long chain walk |
| DeleteNode | 3 | Unlinks a node from its lists and frees it |
| FreeNode | 3 | Frees a node, its buffer and its links |
| NewNode | 2 | Allocates a zeroed node and links it into a bucket list |

Addresses: `0x4968b0`, `0x4a3560`, `0x4c34a0`, `0x4c3510`, `0x4c5c90`, `0x4dcfb0`, `0x4f1c20`, `0x51fca0`, `0x51fd10`, `0x5215c0`, `0x526ad0`, `0x526d70`
Confidence: verified
Proof: one
Test: none

The clones are `0x459730` (sound), `0x45d020` (sound), `0x5f1060` (objectlayer), `0x45d090` (sound). A node is linked into the bucket list its hash selects and into the main list; the free functions unlink both links.

## Arrays

64 rows size and round the arrays whose capacity is a multiple of a quantum:

| Function | Rows | Does |
|---|---|---|
| SetArrayCapacity | 17 | Sets an array's capacity exactly, keeping the smaller of capacity and count when it must move |
| RoundUp | 17 | Rounds a size up to a multiple of a quantum |
| GrowArrayBy | 7 | Raises an array's capacity for count + add, rounding on request, and leaves the count |
| Quantum21 | 6 | Returns the grow quantum for a count: the largest power of two not above it below 21, else 21 |
| ResizeArray | 5 | Resizes an array of 8-byte entries, copying the kept ones when it must move |
| Quantum64 | 5 | Returns the grow quantum for a count: the largest power of two not above it below 64, else 64 |
| SetArrayCount | 2 | Sets an array's count, growing its capacity to a quantum multiple when it must |
| ReserveArray | 2 | Grows an array to hold more entries, rounding up to its quantum on request |
| GrowChunk | 2 | Returns the growth step for a count |
| Quantum8 | 1 | Returns the grow quantum for a count: the largest power of two not above it below 8, else 8 |

Addresses: `0x496d70`, `0x496f80`, `0x497080`, `0x4970a0`, `0x497130`, `0x4a2030`, `0x4a2500`, `0x4a3280`, `0x4a3db0`, `0x4a3df0`, `0x4a4110`, `0x4a4150`, `0x4a4170`, `0x4a5770`, `0x4a57b0`, `0x4b0330`, `0x4b03c0`, `0x4b04b0`, `0x4b04f0`, `0x4b4f50`, `0x4b5020`, `0x4b5040`, `0x4b9db0`, `0x4b9df0`, `0x4befd0`, `0x4d07d0`, `0x4d0810`, `0x4d0830`, `0x4d24c0`, `0x4d4190`, `0x4d4220`, `0x4d6ce0`, `0x4d7150`, `0x4dabb0`, `0x4daca0`, `0x4dad90`, `0x4dae20`, `0x4dae60`, `0x4dae80`, `0x4dd830`, `0x4ddbc0`, `0x4ddbe0`, `0x4ef9c0`, `0x4f3900`, `0x4f4e40`, `0x4f7e90`, `0x4f82a0`, `0x4f8330`, `0x4f8350`, `0x503d10`, `0x5213c0`, `0x521c50`, `0x5233e0`, `0x523420`, `0x526930`, `0x536c80`, `0x784e10`, `0x784e50`, `0x7867d0`, `0x7867f0`, `0x786880`, `0x7868c0`, `0x7891a0`, `0x7891e0`
Confidence: verified
Proof: one
Test: none

The clones are `0x423030` (core), `0x423050` (core), `0x422b50` (core), `0x469d60` (objectlayer), `0x5f24e0` (objectlayer), `0x422ff0` (core), `0x475680` (glue), `0x422850` (core), `0x6c8eb0` (playername), `0x45e880` (sound). `0x496f80` rounds with a divide and remainder; the quantum functions test the count against 0x15 or 0x40, clear the lowest set bit until one bit is left, and store the cap at this+0xc for larger counts.

## Deleting destructors

55 rows are deleting destructors: they destroy an object, then free it when bit 0 of the argument is set. The call to `SMemFree` passes the tag "delete".

| Function | Rows | Does |
|---|---|---|
| DeletingDestructor | 45 | Destroys the object through a destructor and frees it when bit 0 of the argument is set |
| FreeArrayAndSelf | 7 | Frees an array's data and the array when bit 0 of the argument is set |
| RestoreBase | 2 | Restores the base vtable and frees the object when bit 0 of the argument is set |
| DeleteRecord | 1 | Unlinks a record and frees it when bit 0 of the argument is set |

Addresses: `0x495b70`, `0x4a2c40`, `0x4a2d70`, `0x4aff50`, `0x4b6020`, `0x4d4650`, `0x4dd290`, `0x4dd320`, `0x4ee0a0`, `0x4f38c0`, `0x502890`, `0x5042e0`, `0x505840`, `0x529450`, `0x536d10`, `0x6ed9f0`, `0x6ef9c0`, `0x6efb70`, `0x6f0760`, `0x6f0bd0`, `0x705a40`, `0x72d860`, `0x72dda0`, `0x72e050`, `0x72e780`, `0x72f3f0`, `0x72fce0`, `0x72fe30`, `0x730030`, `0x7303a0`, `0x730910`, `0x76ca20`, `0x772ec0`, `0x778820`, `0x779620`, `0x779900`, `0x779dc0`, `0x77e550`, `0x77e690`, `0x77f6c0`, `0x782e90`, `0x783af0`, `0x785110`, `0x785800`, `0x786680`, `0x786b50`, `0x7879b0`, `0x788e60`, `0x789520`, `0x789f70`, `0x78b390`, `0x78c230`, `0x7a3290`, `0x7a35b0`, `0x7cc380`
Confidence: verified
Proof: one
Test: none

The clones are `0x4482b0` (gx), `0x4594f0` (sound), `0x426190` (core), `0x423670` (core). The layout thunks of `Layout thunks` jump to the destructors of the first group; `0x4a2d70` and its clones first free a block at this+8.

## Block frees

20 rows free one block:

| Function | Rows | Does |
|---|---|---|
| FreeField | 11 | Frees the block that a field points at when it is not null |
| FreeRecord | 4 | Clears the record its argument points at and frees it |
| FreeGlobal | 3 | Frees the block that a global points at when it is not null |
| FreeGlobalArray | 2 | Frees the block that a global points at and zeroes the three dwords of its array header |

Addresses: `0x496b00`, `0x4a24e0`, `0x4a3cc0`, `0x4a9f00`, `0x4aff30`, `0x4b4b90`, `0x4d4170`, `0x4dd1b0`, `0x4f3eb0`, `0x503520`, `0x5038f0`, `0x5200a0`, `0x520340`, `0x5260c0`, `0x536910`, `0x6ef250`, `0x6ef730`, `0x701bb0`, `0x7842e0`, `0x78b790`
Confidence: verified
Proof: one
Test: none

The clones are `0x4229c0` (core), `0x63e2b0` (cvar), `0x420b40` (core) and, read in `ui/gameui`, `0x4e31f0`. The first group tests a pointer field and frees it with the tag of its element type; the second clears a record through a call and frees the record.

## Layout thunks

29 rows are 8-byte thunks of the layout vtables: `sub ecx,0x24; jmp target`. Each is stored in a layout vtable in the data section, in most the slot after the entry `0x76b580`, and reaches the object, whose layout part is at +0x24: 27 of the 29 jump to its deleting destructor.

| Function | Rows | Does |
|---|---|---|
| LayoutThunk | 29 | Moves this back by 0x24 to the object and jumps to its deleting destructor |

Addresses: `0x497710`, `0x4b1e20`, `0x4efb00`, `0x504140`, `0x505500`, `0x5066e0`, `0x536ea0`, `0x76c850`, `0x76d8c0`, `0x7733a0`, `0x7733c0`, `0x7798f0`, `0x77e490`, `0x77e5e0`, `0x77fe60`, `0x7834b0`, `0x784f80`, `0x7855d0`, `0x7869d0`, `0x7875e0`, `0x789310`, `0x7893d0`, `0x789d40`, `0x78b1b0`, `0x78c000`, `0x78d700`, `0x7a39f0`, `0x7a3a00`, `0x7cc2b0`
Confidence: verified
Proof: one
Test: none

The clones are `0x470080` (glue). The targets are the destructors of `Deleting destructors`; `ui/frames` (Vtables and the two parts) describes the layout part.

## Type tests

13 rows compare their argument with three type-id globals and answer 1 or 0:

| Function | Rows | Does |
|---|---|---|
| IsTypeId | 11 | Answers 1 for the three type ids of its class and 0 for any other |
| IsTypeId2 | 2 | Answers 1 for two type ids of its class and 0 for any other |

Addresses: `0x4ee000`, `0x5293b0`, `0x773400`, `0x773550`, `0x779d20`, `0x782df0`, `0x785760`, `0x786ab0`, `0x787910`, `0x789480`, `0x789ed0`, `0x78b2d0`, `0x78c190`
Confidence: verified
Proof: one
Test: none

The clones are `0x46ffa0` (glue) and, read in `ui/taxiframe`, `0x484780`. `0x4ee000` compares with the id of its own class and with the frame and region type ids (`ui/frames`).

## Method lookups

5 rows look a method up in a class's method hash with `FrameScriptPushMethod` and fall back to the lookup of the base class, `0x778590` for the frame classes and `0x7a2ea0` for Texture and FontString:

| Function | Rows | Does |
|---|---|---|
| MethodLookup | 5 | Pushes a method from its class hash, else through the base lookup |

Addresses: `0x5368d0`, `0x78dc90`, `0x796f80`, `0x79c620`, `0x79ee20`
Confidence: verified
Proof: one
Test: none

The clones are `0x4c1be0`, read in `ui/loot`. The hash tables are those registered by `ui/scriptapi` (Method table unregistrars).
