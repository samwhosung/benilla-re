"""benilla-re for Binary Ninja: names, types and comments for WoW.exe 1.12.1.5875.

In the GUI, open WoW.exe and run this file (Plugins > Run Script...). Headless:

    python3 benilla_re.py path/to/WoW.exe [output.bndb]

It

  1. defines the structs of benilla-re.h as user types,
  2. creates every function of the symbol table that the view lacks, and
  3. names, types and comments every function and global of the table.

A row Binary Ninja refuses is printed and skipped; a summary ends the run.

On a database that already has names the default keeps them: an item whose name is not Binary
Ninja's own (sub_, data_ ...) keeps its name and its type, and our name is added to its comment.
Set OVERWRITE_NAMES below, or BENILLA_RE_OVERWRITE=1, to name, type and comment every row.

Each comment block starts with a `benilla-re: <name>` line. An item whose name is the one that
line records was named by an earlier run, so a later run, with a newer table, names and types it
again and replaces the block; a name you changed since is kept.

The data is read from beside this script: benilla-re.json, and benilla-re.h one folder up or
beside it. Keep the folders as the export wrote them (binja/ next to bundle/ and benilla-re.h), or
put the two files next to the script; BENILLA_RE_DIR names the script's folder when `__file__` is
not set. The repo is not needed.

Environment:
  BENILLA_RE_OVERWRITE=1  the same as OVERWRITE_NAMES = True.
  BENILLA_RE_FORCE=1  apply to an exe other than 1.12.1.5875 (the addresses will not match).
"""

import hashlib
import json
import os
import re
import sys

try:
    import binaryninja
    from binaryninja import Symbol, SymbolType
except ImportError:  # outside Binary Ninja, so that the helpers can be read and tested
    binaryninja = Symbol = SymbolType = None

# False: leave a name, a type and a comment that are already set. True: replace them with ours.
OVERWRITE_NAMES = os.environ.get("BENILLA_RE_OVERWRITE") == "1"

try:
    HERE = os.path.dirname(os.path.abspath(__file__))
except NameError:
    HERE = os.environ.get("BENILLA_RE_DIR", os.getcwd())

TYPES_BEGIN = "/* BRE-TYPES-BEGIN */"
TYPES_END = "/* BRE-TYPES-END */"

# What the header includes <stdint.h> for, for a parser that does not know the names.
PRELUDE = """\
typedef signed char int8_t;
typedef unsigned char uint8_t;
typedef short int16_t;
typedef unsigned short uint16_t;
typedef int int32_t;
typedef unsigned int uint32_t;
typedef long long int64_t;
typedef unsigned long long uint64_t;
typedef int intptr_t;
typedef unsigned int uintptr_t;
"""
PRELUDE_NAMES = {
    "int8_t", "uint8_t", "int16_t", "uint16_t", "int32_t", "uint32_t", "int64_t", "uint64_t",
    "intptr_t", "uintptr_t",
}

CONVENTIONS = {
    "BRE_CDECL": "__cdecl",
    "BRE_STDCALL": "__stdcall",
    "BRE_FASTCALL": "__fastcall",
    "BRE_THISCALL": "__thiscall",
}

DECLARATION = re.compile(r"^/\* (0x[0-9a-f]{6}) \*/ (.*)$")
TRAILING_COMMENT = re.compile(r"\s*/\*.*\*/\s*$")
MARKER = "benilla-re:"
DEFAULT_NAME = re.compile(r"^(?:j_)?(?:sub|data|loc|unknown)_[0-9a-fA-F]+$")
STRUCT_BLOCK = re.compile(r"(?m)^(?=/\* \d+ bytes: )")


def find(name, folders):
    """The first file called `name` in `folders` (relative to this script), or None."""
    for folder in folders:
        path = os.path.join(HERE, folder, name)
        if os.path.isfile(path):
            return path
    return None


def load_bundle(path):
    """The tables of benilla-re.json as {name: [row as a dict]}, and the whole document."""
    with open(path, encoding="utf-8") as handle:
        document = json.load(handle)
    tables = {}
    for name, table in document["tables"].items():
        columns = table["columns"]
        tables[name] = [dict(zip(columns, row)) for row in table["rows"]]
    return tables, document


def read_header(path):
    """The header's type declarations as one text, and its prototypes and globals as
    {address: C declaration, without the closing semicolon}."""
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    begin = text.index(TYPES_BEGIN) + len(TYPES_BEGIN)
    types = text[begin : text.index(TYPES_END)]
    declarations = {}
    for line in text[text.index(TYPES_END) :].splitlines():
        match = DECLARATION.match(line)
        if match is None:
            continue
        declaration = TRAILING_COMMENT.sub("", match.group(2)).rstrip(";")
        if declaration.startswith("extern "):
            declaration = declaration[len("extern ") :]
        for macro, keyword in CONVENTIONS.items():
            declaration = declaration.replace(macro, keyword)
        declarations[int(match.group(1), 16)] = declaration
    return types, declarations


def finding_path(finding):
    if not finding:
        return ""
    doc, _, anchor = finding.partition("#")
    path = "METHOD.md" if doc == "METHOD" else "systems/" + doc + ".md"
    return path + ("#" + anchor if anchor else "")


def comment_of(row):
    text = "%s\n%s, proof %s" % (row["meaning"], row["confidence"], row["proof"])
    if row["finding"]:
        text += "\nFinding: " + finding_path(row["finding"])
    return text


def check_exe(bv, document):
    """True when the view holds the exe the table describes."""
    want = document["exe"]
    try:
        raw = bv.parent_view or bv
        size = raw.length
        have = hashlib.sha256(raw.read(0, size)).hexdigest()
    except Exception as error:  # a view with no raw bytes to read
        print("benilla-re: cannot read the file's identity (%s); going on" % error)
        return True
    if have == want["sha256"] and size == want["size"]:
        return True
    print(
        "benilla-re: this view is of a file of %s bytes, SHA-256 %s;\n"
        "  the table describes %s %s, %s bytes, SHA-256 %s"
        % (size, have, want["name"], want["version"], want["size"], want["sha256"])
    )
    return os.environ.get("BENILLA_RE_FORCE") == "1"


class Summary:
    def __init__(self):
        self.counts = {}

    def add(self, key, number=1):
        self.counts[key] = self.counts.get(key, 0) + number

    def failed(self, address, name, what):
        self.add("failed: " + what.split(":")[0])
        print("benilla-re: %s %s: %s" % (address, name, what))


def define(bv, result, summary):
    for name, type_ in result.types.items():
        if str(name) in PRELUDE_NAMES:
            continue
        bv.define_user_type(name, type_)
        summary.add("types defined")


def apply_types(bv, types, summary):
    """Defines the header's structs: all at once, else one struct at a time."""
    for prelude in ("", PRELUDE):
        try:
            define(bv, bv.parse_types_from_string(prelude + types), summary)
            return
        except SyntaxError as error:
            last = error
    print("benilla-re: the structs do not parse together (%s); trying them one by one" % last)
    blocks = STRUCT_BLOCK.split(types)
    forward = PRELUDE + blocks[0].replace("#pragma pack(push, 1)", "")
    for block in blocks[1:]:
        text = forward + "#pragma pack(push, 1)\n" + block.split("#pragma pack(pop)")[0]
        try:
            define(bv, bv.parse_types_from_string(text), summary)
        except SyntaxError as error:
            summary.failed("-", block.splitlines()[1] if "\n" in block else block, str(error))


def parse(bv, declaration):
    type_, _name = bv.parse_type_string(declaration)
    return type_


def made_by_analysis(symbol):
    """Whether a symbol is Binary Ninja's own: sub_401000, data_401000 ..."""
    if symbol is None:
        return True
    return bool(getattr(symbol, "auto", False)) or bool(DEFAULT_NAME.match(symbol.name))


def split_comment(existing):
    """A comment as (the text outside our block, the name our block records or None). Our block
    runs from its `benilla-re:` line to the next empty line."""
    lines = (existing or "").split("\n")
    for start, line in enumerate(lines):
        if line.startswith(MARKER):
            end = start
            while end < len(lines) and lines[end] != "":
                end += 1
            others = "\n".join(lines[:start] + lines[end:]).strip("\n")
            return others, line[len(MARKER) :].strip()
    return (existing or "").strip("\n"), None


def comment_for(row, others):
    """The comment to set: our block, after the text of the item's own that is kept."""
    block = "%s %s\n%s" % (MARKER, row["name"], comment_of(row))
    return "%s\n\n%s" % (others, block) if others and not OVERWRITE_NAMES else block


def named_by_us(recorded, symbol):
    """Whether an earlier run named the item: its name is still the one the run recorded."""
    return bool(recorded) and symbol is not None and symbol.name == recorded


def apply_functions(bv, rows, declarations, summary):
    for number, row in enumerate(rows):
        if number and number % 5000 == 0:
            print("benilla-re: %d of %d functions" % (number, len(rows)))
        address = int(row["address"], 16)
        name = row["name"]
        try:
            function = bv.get_function_at(address)
            if function is None:
                function = bv.create_user_function(address)
                if function is None:
                    summary.failed(row["address"], name, "no function could be made")
                    continue
                summary.add("functions created")
            others, recorded = split_comment(function.comment)
            ours = named_by_us(recorded, function.symbol)
            keep = not OVERWRITE_NAMES and not made_by_analysis(function.symbol) and not ours
            if keep:
                summary.add("functions kept as they are")
            elif name:
                function.name = name
                summary.add("functions named")
            declaration = declarations.get(address)
            if declaration is None:
                if row["signature"]:
                    summary.add("signatures the header does not state")
            elif not keep:
                try:
                    function.set_user_type(parse(bv, declaration))
                    summary.add("functions typed")
                except SyntaxError as error:
                    summary.failed(row["address"], name, "type refused: %s: %s" % (declaration, error))
            function.comment = comment_for(row, others)
        except Exception as error:
            summary.failed(row["address"], name, "error: %s" % error)


def apply_globals(bv, rows, declarations, summary):
    for row in rows:
        address = int(row["address"], 16)
        name = row["name"]
        try:
            if not bv.is_valid_offset(address):
                summary.failed(row["address"], name, "not in the view")
                continue
            symbol = bv.get_symbol_at(address)
            others, recorded = split_comment(bv.get_comment_at(address))
            ours = named_by_us(recorded, symbol)
            keep = not OVERWRITE_NAMES and not made_by_analysis(symbol) and not ours
            declaration = declarations.get(address)
            if keep:
                summary.add("globals kept as they are")
            elif declaration is None:
                summary.add("globals the header does not state")
            else:
                try:
                    bv.define_user_data_var(address, parse(bv, declaration))
                    summary.add("globals typed")
                except SyntaxError as error:
                    summary.failed(row["address"], name, "type refused: %s: %s" % (declaration, error))
            if name and not keep:
                bv.define_user_symbol(Symbol(SymbolType.DataSymbol, address, name))
                summary.add("globals named")
            bv.set_comment_at(address, comment_for(row, others))
        except Exception as error:
            summary.failed(row["address"], name, "error: %s" % error)


def apply(bv):
    """Applies the table to a view of WoW.exe; 0 when it ran."""
    bundle = find("benilla-re.json", [".", os.path.join("..", "bundle")])
    header = find("benilla-re.h", [".", ".."])
    if bundle is None or header is None:
        print(
            "benilla-re: benilla-re.json and benilla-re.h are not beside this script "
            "(%s); see the top of the script" % HERE
        )
        return 1
    tables, document = load_bundle(bundle)
    if not check_exe(bv, document):
        print("benilla-re: stopped; BENILLA_RE_FORCE=1 applies it anyway")
        return 1
    print(
        "benilla-re: table of commit %s; %s"
        % (
            document["commit"],
            "replacing names, types and comments"
            if OVERWRITE_NAMES
            else "keeping names, types and comments already set",
        )
    )
    types, declarations = read_header(header)
    summary = Summary()
    apply_types(bv, types, summary)
    apply_functions(bv, tables["symbols/functions"], declarations, summary)
    apply_globals(bv, tables["symbols/globals"], declarations, summary)
    print("benilla-re: done")
    for key in sorted(summary.counts):
        print("  %6d  %s" % (summary.counts[key], key))
    return 0


def main(argv):
    if "bv" in globals():  # the GUI's Run Script
        return apply(globals()["bv"])
    if len(argv) < 2:
        print(__doc__)
        return 2
    bv = binaryninja.load(argv[1])
    try:
        code = apply(bv)
        if code == 0 and len(argv) > 2:
            bv.update_analysis_and_wait()
            bv.file.create_database(argv[2])
        return code
    finally:
        bv.file.close()


if __name__ == "__main__":
    status = main(sys.argv)
    if "bv" not in globals():  # headless; in the GUI a SystemExit would end the console
        sys.exit(status)
