# System docs

One doc per system of the client: `systems/<system>.md`. A system that outgrows one doc splits by
topic into `systems/<system>/<topic>.md`, and its rows link to the topic doc. A doc is a
reference: it says what the client does, where, and how that was read from the bytes. History is
in git; open work is in the issue tracker.

## Shape of a doc

```
# <system>

<Overview>

Roots: `0x……`, `0x……`, …

## <Entry title>
…
```

### Overview

What the system does and where it lives in the exe: its object files as address ranges, the
structs and globals it owns, and which of its functions the rest of the client actually reaches.
A system owns the functions of its object files (`../symbols/README.md`). At most 200 words, no
addresses beyond what a reader needs to find the code.

### Object files

Most object files include a shared maths header whose static initializers store +∞, 2π, 1/2π and
a block of constants. The C runtime's initializer table, `0x827000`, lists each file's initializers
as one contiguous run in link order, and a file that includes the header opens its run with its +∞
copy: the table's first slot, `0x827004`, is one, and its last three, from `0x82e024`, are +∞, 2π
and 1/2π. A run gives the file's initializers, not its extent. The file's code can start before
its +∞ thunk, at a lower initializer of the same run or at a function with no initializer, and
code past its last initializer can belong to a file with no run. A file without the header or
without initializers has no run: it is bounded by the source paths its code passes and by its
callers. One stretch of code between two runs can hold several files, as WardenClient.cpp
(0x6c9f8e) and FFXEffects.cpp (0x6cbed2) share the stretch after slot `0x82cd78`.

A row's system is its object file's, and a row stays `unassigned` until its file is proved. Four
proofs settle it:

- **A listed span.** A system's doc lists its object files by the addresses of their first and last
  function, with the bytes that bound them: the functions between are that system's. A span whose
  end the doc leaves open covers its first function only. A span that holds a row of another
  system is not used.
- **A source path.** A function that passes a `.cpp` or `.c` path (`__FILE__` in an assertion or an
  allocation) is in that file; the path's directory gives the system. A header's path names no
  file, since headers are inlined anywhere.
- **An initializer run.** The functions of one run are one file's: its +∞ thunk and initializer and
  the functions that read the file's own copies of the header's constants. A file proved only by
  its run takes the one system that the evidence inside its stretch agrees on, and never a system
  with a doc, which lists its own object files.
- **A stretch.** Code is contiguous per object file, so the functions between the first and the
  last that a path or a run proves to be one file's are that file's. A stretch is not used when it
  overlaps another file's, or holds a row of another system.

Two functions of one system with an unproved function between them settle nothing: the stretch
can hold another system's file. The library code the client links is owned by its library
(`../symbols/README.md`, "One row per address"): the community database names the C runtime's
functions, and a function that passes a `DBFilesClient\` file name is a DBC record file's.
`cargo xtask census --assign` applies the rule to every row that has no system.

### Roots

The functions through which anything outside the system enters it:

- functions called or jumped to from code another system owns;
- functions whose address is stored anywhere in the image: vtable slots, registration tables,
  callbacks passed as arguments;
- static initializers, which the C runtime calls through its initializer table.

The closure of a system is every function reachable from its roots by calls, tail jumps, vtable
slots and stored addresses, stopping at functions another system owns. Every function in the
closure has a row with a `class` (`../symbols/README.md`), and every `primitive` in it has a
bit-exact test. That is the definition of a complete system, and no system meets it yet: tests
cover a few primitives, and the rest are open work
([#87](https://github.com/samwhosung/benilla-re/issues/87)). A
function the system owns that nothing live reaches is of class `unreached`:
it keeps a row with its one-line meaning and owes no test. The doc gives the unreached functions
one short entry that lists their addresses and the census that shows nothing reaches them.

### Entries

One entry per mechanism: a `##` heading (the title, at most six words, which is also the
anchor rows link to), then this shape:

```
## State advance

<Claim: what the client does, in one to three sentences, with the inputs it holds for.>

Addresses: `0x4531e0`, `0x802700`
Confidence: verified
Proof: two
Test: none

<Derivation: the path from the bytes to the claim, in a few lines.>
```

- **Claim.** The mechanism in plain words, precise enough to reimplement: formulas with their
  constants, every branch that changes the result, the order of effects, what is stored where.
  It states the input space it holds for; a claim about one case names it.
- **Addresses.** Every address that decides the claim: the functions, the globals, and the
  instruction addresses of deciding branches and stores. Each one resolves to a row: an exact
  address, or an instruction inside a function row's extent.
- **Confidence** and **Proof** take the values of the table columns: `verified` with `one` or
  `two`; `inferred` with `none`. The entry states its weakest load-bearing step; a row
  linking here may carry less, never more.
- **Test.** The bit-exact tests that run the entry's code against the exe, `crate::test_fn`
  separated by `, `, or `none`. When a test covers only part of a function, the derivation says
  which instructions.
- **Derivation.** Instruction addresses, opcodes and constants, in the order that proves the
  claim. Quote at most a few instructions; never paste a listing. A census states its pattern,
  where it searched, its count and its control (`../docs/METHOD.md`).

An entry that settles a configuration another entry decided reconciles both in the same change.
A correction rewrites the entry; the wrong version stays only in git.

## Addresses in prose

An address is written `0x` and six lowercase hex digits, in backticks: `` `0x514ae0` ``. Struct
fields are written `+0x14` with the struct named nearby. Every address in a system doc resolves
to a row of the table. A pooled constant is cited by value (`2π as f32, 0x40c90fdb`) unless its address
matters. A constant that would read as an address, six hex digits after `0x`, is written in
decimal or without backticks. A range is written by its first and last function, never by an end
address that belongs to no row. A function's end, and the padding bytes after it, are written in
backticks as any address is: the check accepts an address that lies less than 16 bytes past the
end of the nearest row before it.

## Rules the check enforces

`cargo xtask check` enforces these. They apply to every
doc under `systems/` except this one.

| rule | value | why |
|---|---|---|
| overview | 200 words | a reader orients on one screen |
| entry title | 6 words | it is an anchor and must stay stable |
| claim | 120 words, a table not counted | a longer claim is two mechanisms |
| table in a claim | 12 rows | a longer table belongs in the symbol table |
| derivation | 200 words | a derivation cites bytes; a longer one is a pasted dump |
| quoted instructions | 8 lines per block | findings cite the bytes, they do not reproduce the exe |
| one doc | 4,000 words | past that, split by topic |
| hedges | `probably`, `likely`, `seems`, `appears to`, `roughly`, `maybe`, `should be`, `TODO`, `FIXME` | a guess is `inferred`, said once in the Confidence line |
| shouting | `NOTE:`, `IMPORTANT`, `WARNING`, `CORRECTED`, `VERIFIED`, `INFERRED` | emphasis does not make a claim truer |
| history | `previously`, `used to`, `no longer`, `originally`, `corrected`, `was wrong` | the doc states the current fact; git holds the past |
| first person | `we`, `our`, `I` | the claim is about the client, not the reader |
| entry shape | the four lines `Addresses`, `Confidence`, `Proof`, `Test` with valid values | the table and the docs share one vocabulary |
| addresses | every address in a system doc resolves to a row; one in a spec doc (`docs/METHOD.md`, `docs/CONTRIBUTING.md`, the READMEs) lies inside the image | a citation without a row cannot be looked up; a spec doc's addresses are examples and pointers |
| anchors | no two headings in one doc share an anchor | a link must reach one entry |
