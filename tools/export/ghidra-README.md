# benilla-re for Ghidra

Two Ghidra scripts and the tables they read, to name and type your own copy of `WoW.exe`
1.12.1.5875. The scripts are the ones `tools/ghidra/setup.sh` runs; this folder needs no checkout.

- `CreateFunctions.java` creates the functions `symbols/functions.tsv` lists that analysis missed,
  and gives each the row's body.
- `ApplySymbols.java` applies the table: struct types, function names (`Class::Method` as
  namespaces), signatures where a row has one, labels and types for globals, and a plate comment
  per row with its meaning and confidence.
- `symbols/` holds the four tables the scripts read.

## Run it

You need Ghidra 12.1 or later and a JDK 21 or later. Check your exe first: it must be 4,775,986
bytes with SHA-256 `b4756d38ef207c02ed651f4952bd89a70b4857b73a33413339e1b285b28d2dc7`. Any other
build has other addresses.

Import and analyse in one headless run, then run the two scripts in a second run with analysis
off, from this folder. Give `-scriptPath` and the tables' folder as absolute paths: `analyzeHeadless`
starts in its own folder, so a relative path is not found. Analysis after a script removes code at
a few row starts, which is why the scripts get their own run.

```
analyzeHeadless <project folder> <project name> -import <path to WoW.exe> \
    -processor x86:LE:32:default -cspec windows

analyzeHeadless <project folder> <project name> -process WoW.exe -noanalysis \
    -scriptPath "$PWD" \
    -postScript CreateFunctions.java "$PWD/symbols" \
    -postScript ApplySymbols.java "$PWD/symbols"
```

The second command can run again on the same project. The scripts take the tables' folder as
their one argument, so they run headless; the Script Manager passes none.

Pass `keep` after the folder (`-postScript ApplySymbols.java "$PWD/symbols" keep`) on a program that
already has names of its own: an item whose name somebody set (user-defined or imported, not
Ghidra's default, analysis or AI) keeps its name and its type, and takes ours as a block of its
repeatable comment; the rest of the comment stays. Every name the script applies leaves a
`benilla-re: <name>` line in that comment, and with `keep` a later run, with a newer table, names
again the items whose name is still the one that line records. Without `keep` every row is
applied.

## What it prints

`CreateFunctions` prints one line per function it splits off: where analysis made one function
over two rows because the first jumps to the second, the second's start becomes a function and
the first ends at it. It prints a `size` line for each function whose body differs from its row's
size, and a total.

`ApplySymbols` prints one `FAIL [reason]` line for each row it could not apply, then a summary:
the rows, the names and signatures found in the program, and the failures by reason. A type the
tables name but never define (`lua_State`, `CDataStore`) becomes an empty struct, so pointers to
it apply; the C builtins, the stdint types, `bool`, `...` and the SDK's `RECT`, `POINT`, `SIZE` and
`MSG` are known. A row keeps its name and comment when its signature fails. Two reasons are
defects of the table: `signature outside the grammar` (a name, a comment or a register note in
the prototype) and `__thiscall without this` (a `__thiscall` row whose first parameter is not
`this`); the summary marks them. A `__thiscall` row with no parameters applies as it is.
