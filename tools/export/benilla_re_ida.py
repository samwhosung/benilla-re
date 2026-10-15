"""benilla-re for IDA 9: names, types and comments for WoW.exe 1.12.1.5875.

Run it in an IDA database of that exe (File > Script file...). It

  1. loads the structs of benilla-re.h into the local types,
  2. creates every function of the symbol table that the database lacks, and
  3. names, types and comments every function and global of the table.

A row IDA refuses is printed and skipped; a summary ends the run.

The data is read from beside this script: benilla-re.json, and benilla-re.h one folder up or
beside it. Keep the folders as the export wrote them (ida/ next to bundle/ and benilla-re.h), or
put the two files next to the script. The repo is not needed.

Environment:
  BENILLA_RE_DIR      the script's folder, when `__file__` is not set.
  BENILLA_RE_FORCE=1  apply to an exe other than 1.12.1.5875 (the addresses will not match).
"""

import json
import os
import re

try:
    import ida_bytes
    import ida_funcs
    import ida_name
    import ida_nalt
    import idc
except ImportError:  # outside IDA, so that the helpers can be read and tested
    ida_bytes = ida_funcs = ida_name = ida_nalt = idc = None

try:
    HERE = os.path.dirname(os.path.abspath(__file__))
except NameError:
    HERE = os.environ.get("BENILLA_RE_DIR", os.getcwd())
TYPES_BEGIN = "/* BRE-TYPES-BEGIN */"
TYPES_END = "/* BRE-TYPES-END */"

# What the header includes <stdint.h> for, spelled for IDA's parser.
PRELUDE = """\
typedef signed char int8_t;
typedef unsigned char uint8_t;
typedef short int16_t;
typedef unsigned short uint16_t;
typedef int int32_t;
typedef unsigned int uint32_t;
typedef long long int64_t;
typedef unsigned long long uint64_t;
"""

CONVENTIONS = {
    "BRE_CDECL": "__cdecl",
    "BRE_STDCALL": "__stdcall",
    "BRE_FASTCALL": "__fastcall",
    "BRE_THISCALL": "__thiscall",
}

DECLARATION = re.compile(r"^/\* (0x[0-9a-f]{6}) \*/ (.*)$")
TRAILING_COMMENT = re.compile(r"\s*/\*.*\*/\s*$")


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
    {address: C declaration for IDA}."""
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    begin = text.index(TYPES_BEGIN) + len(TYPES_BEGIN)
    types = text[begin : text.index(TYPES_END)]
    declarations = {}
    for line in text[text.index(TYPES_END) :].splitlines():
        match = DECLARATION.match(line)
        if match is None:
            continue
        declaration = TRAILING_COMMENT.sub("", match.group(2))
        if declaration.startswith("extern "):
            declaration = declaration[len("extern ") :]
        for macro, keyword in CONVENTIONS.items():
            declaration = declaration.replace(macro, keyword)
        declarations[int(match.group(1), 16)] = declaration
    return types, declarations


def c_name(name):
    """The name the header gives a row: `::` written `__`, a destructor written `dtor`."""
    parts = name.split("::")
    if parts[-1].startswith("~"):
        parts[-1] = "dtor"
    return "__".join(parts)


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


def check_exe(document):
    """True when the database holds the exe the table describes."""
    want = document["exe"]
    try:
        have = ida_nalt.retrieve_input_file_sha256()
        size = ida_nalt.retrieve_input_file_size()
    except Exception as error:  # an IDA without these calls
        print("benilla-re: cannot read the input file's identity (%s); going on" % error)
        return True
    have = have.hex() if isinstance(have, (bytes, bytearray)) else have
    if have == want["sha256"] and size == want["size"]:
        return True
    print(
        "benilla-re: this database is of a file of %s bytes, SHA-256 %s;\n"
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


def set_name(address, name):
    """Names an address, as the table spells the name or, when IDA refuses that, as the header."""
    flags = ida_name.SN_CHECK | ida_name.SN_NOWARN | ida_name.SN_NON_AUTO
    if ida_name.set_name(address, name, flags):
        return "as is"
    if ida_name.set_name(address, c_name(name), flags):
        return "as in the header"
    return None


def apply_types(types):
    errors = idc.parse_decls(PRELUDE + types, 0)
    print("benilla-re: structs loaded, %d errors" % errors)
    return errors


def apply_functions(rows, declarations, summary):
    for number, row in enumerate(rows):
        if number and number % 5000 == 0:
            print("benilla-re: %d of %d functions" % (number, len(rows)))
        address = int(row["address"], 16)
        name = row["name"]
        function = ida_funcs.get_func(address)
        if function is None:
            if ida_funcs.add_func(address, address + int(row["size"])):
                summary.add("functions created")
                function = ida_funcs.get_func(address)
            else:
                summary.failed(row["address"], name, "no function could be made")
                continue
        if name:
            how = set_name(address, name)
            if how is None:
                summary.failed(row["address"], name, "name refused")
            else:
                summary.add("functions named " + how)
        declaration = declarations.get(address)
        if declaration is None:
            if row["signature"]:
                summary.add("signatures the header does not state")
        elif idc.SetType(address, declaration):
            summary.add("functions typed")
        else:
            summary.failed(row["address"], name, "type refused: " + declaration)
        if function is not None:
            ida_funcs.set_func_cmt(function, comment_of(row), False)


def apply_globals(rows, declarations, summary):
    for row in rows:
        address = int(row["address"], 16)
        name = row["name"]
        if idc.get_segm_start(address) == idc.BADADDR:
            summary.failed(row["address"], name, "not in the database")
            continue
        if name:
            how = set_name(address, name)
            if how is None:
                summary.failed(row["address"], name, "name refused")
            else:
                summary.add("globals named " + how)
        declaration = declarations.get(address)
        if declaration is None:
            summary.add("globals the header does not state")
        elif idc.SetType(address, declaration):
            summary.add("globals typed")
        else:
            summary.failed(row["address"], name, "type refused: " + declaration)
        ida_bytes.set_cmt(address, comment_of(row), False)


def main():
    bundle = find("benilla-re.json", [".", os.path.join("..", "bundle")])
    header = find("benilla-re.h", [".", ".."])
    if bundle is None or header is None:
        print(
            "benilla-re: benilla-re.json and benilla-re.h are not beside this script "
            "(%s); see the top of the script" % HERE
        )
        return 1
    tables, document = load_bundle(bundle)
    if not check_exe(document):
        print("benilla-re: stopped; BENILLA_RE_FORCE=1 applies it anyway")
        return 1
    print("benilla-re: table of commit %s" % document["commit"])
    types, declarations = read_header(header)
    summary = Summary()
    summary.add("struct parse errors", apply_types(types))
    apply_functions(tables["symbols/functions"], declarations, summary)
    apply_globals(tables["symbols/globals"], declarations, summary)
    print("benilla-re: done")
    for key in sorted(summary.counts):
        print("  %6d  %s" % (summary.counts[key], key))
    return 0


if __name__ == "__main__":
    main()
