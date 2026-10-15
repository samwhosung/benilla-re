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

Import it and run both scripts in one headless pass, from this folder:

```
analyzeHeadless <project folder> <project name> -import <path to WoW.exe> \
    -processor x86:LE:32:default -cspec windows \
    -scriptPath . \
    -postScript CreateFunctions.java symbols \
    -postScript ApplySymbols.java symbols
```

On a program already imported and analysed, run the two scripts alone with
`-process WoW.exe -noanalysis` in place of `-import ...`. The scripts take the tables' folder as
their one argument, so they run headless; the Script Manager passes none.

Pass `keep` after the folder (`-postScript ApplySymbols.java symbols keep`) on a program that
already has names of its own: an item whose name is not Ghidra's default keeps its name and its
type, and takes ours as a repeatable comment. Without it every row is applied.

## What it prints

One `ApplySymbols: FAIL` line for each row it could not apply, then a verification of every
name against the program and a failure count. A signature fails when it names a type
`symbols/structs.tsv` does not define or is written for the `__userpurge` convention; the row keeps
its name and comment. A row whose address is no function in your analysis is skipped.
