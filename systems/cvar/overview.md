# cvar: overview

The client's settings registry. A setting is a named record with a text value, its integer and float forms, a default, a
reset value, a latched value, flags, a category, a help line and a change callback. The registry file finds, creates and sets
them, gives each one a console command, and reads and writes `Config.wtf`. A second file in the system registers the 31 video
options and holds their callbacks.

The docs split by topic: `registry` (the record, registering, setting, lookup and the registry's commands), `config` (start-up,
the file, shutdown), `containers` (the hash table both registries are built from), `options` (the video options) and `defaults` (the defaults built at the call). This doc holds the object
files, their static objects and the shared header constants. Every subsystem registers its settings here, and the console
registers each setting's command (`console/commands`). The table of all registration sites is `tables/cvars.tsv`.

Roots: `0x63cff0`, `0x63d010`, `0x63d0e0`, `0x63e680`, `0x63e6a0`, `0x63ef20`, `0x63ef50`, `0x63ef80`, `0x63efb0`, `0x63efe0`, `0x63f010`, `0x63f040`, `0x63f070`, `0x63f0a0`, `0x63f0d0`, `0x63f100`, `0x63f130`, `0x63f160`, `0x63f190`, `0x63f1b0`, `0x63f1e0`, `0x63f210`, `0x63f240`

## Object files

The registry is `ConsoleVar.cpp`, from `0x63cff0` to the end of `0x63f250`. Its path is `0x865150`, passed at `0x63d297` and `0x63dfa3`; the
command registry follows from `0x63f270`. The options file runs from `0x688400` to `0x689a50` and passes no path: it is placed by its run
of static initializers in the table `0x827000` (slots `0x82bdf4` to `0x82be64`) and by its one registration function.

Addresses: `0x63cff0`, `0x63f250`, `0x63f270`, `0x865150`, `0x63d297`, `0x63dfa3`, `0x827000`, `0x82b7cc`, `0x82b820`, `0x82bdf4`, `0x82be64`, `0x688400`, `0x689a50`
Confidence: verified
Proof: one
Test: none

The table `0x827000` lists the registry's run at the slots `0x82b7cc` to `0x82b820`: 21 header slots, then the table's own initializer `0x63d010`.
The options file's run is the slots `0x82bdf4` to `0x82be64`. Each run opens with a +∞ thunk (`0x63cff0`, `0x688400`) and the next file opens with its own (`0x63f270`, `0x689a50`).
The options file holds callbacks that terrain's docs describe: the rows `0x688d40`, `0x688d90`, `0x689110`, `0x6891f0` and `0x689250` keep terrain as their owner.

## Shared header constants

Each file opens its run with the shared header's constants (`models/overview#shared-header-constants`): a 5-byte thunk and a body that stores
one float each. The registry file has 21: +∞, 2π, 1/2π, the squares 10², 11.111111², 5.5555553², 10², 5², 10², 30², 120², 20², 10², 0.5², 3²
and 100², the float 5.0, then 50², 100², 40² and 1.5². The options file has 28: the same, then four products, a second 1/2π, 10² and 0.5/10.

Addresses: `0x63cff0`, `0x63d000`, `0x63e680`, `0x63e690`, `0x63e6a0`, `0x63e6b0`, `0x63ef20`, `0x63ef30`, `0x63ef50`, `0x63ef60`, `0x63ef80`, `0x63ef90`, `0x63efb0`, `0x63efc0`, `0x63efe0`, `0x63eff0`, `0x63f010`, `0x63f020`, `0x63f040`, `0x63f050`, `0x63f070`, `0x63f080`, `0x63f0a0`, `0x63f0b0`, `0x63f0d0`, `0x63f0e0`, `0x63f100`, `0x63f110`, `0x63f130`, `0x63f140`, `0x63f160`, `0x63f170`, `0x63f190`, `0x63f1a0`, `0x63f1b0`, `0x63f1c0`, `0x63f1e0`, `0x63f1f0`, `0x63f210`, `0x63f220`, `0x63f240`, `0x63f250`, `0x688400`, `0x688410`, `0x6889c0`, `0x6889d0`, `0x6889e0`, `0x6889f0`, `0x6895b0`, `0x6895c0`, `0x6895e0`, `0x6895f0`, `0x689610`, `0x689620`, `0x689640`, `0x689650`, `0x689670`, `0x689680`, `0x6896a0`, `0x6896b0`, `0x6896d0`, `0x6896e0`, `0x689700`, `0x689710`, `0x689730`, `0x689740`, `0x689760`, `0x689770`, `0x689790`, `0x6897a0`, `0x6897c0`, `0x6897d0`, `0x6897f0`, `0x689800`, `0x689820`, `0x689830`, `0x689840`, `0x689850`, `0x689870`, `0x689880`, `0x6898a0`, `0x6898b0`, `0x6898d0`, `0x6898e0`, `0x689900`, `0x689910`, `0x689930`, `0x689940`, `0x689960`, `0x689970`, `0x689990`, `0x6899a0`, `0x6899c0`, `0x6899d0`, `0x6899f0`, `0x689a00`, `0x689a20`, `0x689a30`
Confidence: verified
Proof: one
Test: none

The bodies are `mov eax,[c]; mov [g],eax` for +∞, `fld; fadd st(0),st; fstp` for 2π, `fld 1.0; fdiv; fstp` for 1/2π and 0.5/10, `fld; fmul; fstp` for the squares and the
products, and a plain store for 5.0. The four products are -21 · -0.027777778, that product · -255, 18 · 0.027777778 and that product · 256. The products, the second 1/2π, 10² and 0.5/10 are in the options file's run only. In the registry file, 2π and 1/2π sit between the code (`0x63e680`, `0x63e6a0`) and the squares follow its last function (`0x63ef20`).

## Static objects

The registry builds its table object with a slot function, `call body; jmp register`, and registers `0x63d0f0` as its exit destructor with `_atexit`. The body `0x63d020` leaves
the table empty (`containers#hash-table-object`) and the destructor unlinks every record and frees the bucket vector. The options file builds a use count: its slot function `0x688420`
calls `0x688430`, which adds 1 to `0xc2a3c4` and runs `0x5b71c0` on the first use, and registers `0x688460`, which subtracts 1 and runs `0x5b71d0` at the last.

Addresses: `0x63d010`, `0x63d020`, `0x63d0e0`, `0x63d0f0`, `0x63d0e5`, `0x688420`, `0x688430`, `0x688450`, `0x688460`, `0x688440`, `0x688468`, `0x5b71c0`, `0x5b71d0`, `0xc2a3c4`, `0x409aef`
Confidence: verified
Proof: one
Test: none

`0x63d0e5` calls `_atexit` (`0x409aef`) with `0x63d0f0` pushed at `0x63d0e0`; the destructor is `0x63d0f0`. `0x688440` and `0x688468` are the tail jumps into the shared object's construct and destruct routines.
