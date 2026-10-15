"""Runs the exported IDA and Binary Ninja scripts against stand-ins for their APIs.

    python3 -I scripts_test.py <export directory>

The stand-ins record the calls the scripts make and answer as the documented APIs do; they
prove that each script reads the export, walks every row and tolerates a refusal, not that IDA
or Binary Ninja accepts a given call. Exits non-zero on the first wrong result.
"""

import contextlib
import hashlib
import importlib.util
import io
import json
import os
import re
import sys
import types

OUT = os.path.abspath(sys.argv[1])
BADADDR = 0xFFFFFFFF


def fail(message):
    print("FAIL: " + message)
    sys.exit(1)


def expect(condition, message):
    if not condition:
        fail(message)


def rows(table):
    with open(os.path.join(OUT, "bundle", "symbols", table + ".tsv"), encoding="utf-8") as handle:
        lines = handle.read().split("\n")[:-1]
    columns = lines[0].split("\t")
    return [dict(zip(columns, line.split("\t"))) for line in lines[1:]]


FUNCTIONS = rows("functions")
GLOBALS = rows("globals")
with open(os.path.join(OUT, "bundle", "benilla-re.json"), encoding="utf-8") as handle:
    EXE = json.load(handle)["exe"]
# A rebased bundle names no file: the scripts apply it to any build and say what it is.
REBASED = bool(EXE.get("rebased"))
if REBASED:
    EXE = {"size": 1, "sha256": "00" * 32}

# The addresses the header states, from the header itself.
STATED = {}
with open(os.path.join(OUT, "benilla-re.h"), encoding="utf-8") as handle:
    for line in handle:
        match = re.match(r"^/\* (0x[0-9a-f]{6}) \*/ ", line)
        if match:
            STATED[int(match.group(1), 16)] = line
STATED_FUNCTIONS = sum(1 for r in FUNCTIONS if int(r["address"], 16) in STATED)
STATED_GLOBALS = sum(1 for r in GLOBALS if int(r["address"], 16) in STATED)
# The full tree's export has all its rows; an export of a subset (the rebased tables of a match)
# has fewer, and only the floors that follow from that hold.
FULL = len(FUNCTIONS) > 30000 and len(GLOBALS) > 4000
if FULL:
    expect(STATED_FUNCTIONS > 15000 and STATED_GLOBALS > 4000, "the header states few rows")
else:
    expect(
        STATED_FUNCTIONS > len(FUNCTIONS) // 5 and STATED_GLOBALS >= len(GLOBALS) // 2,
        "the header states few rows",
    )


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run(module, call):
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        code = call()
    return code, out.getvalue()


# ---- IDA ----


def ida_run(overwrite, preexisting, db=None, edit=None):
    """Runs the IDA script on a database where `preexisting` addresses already have a name and a
    type, or on `db`, what an earlier run left; `edit` changes the tables the script reads.
    Returns what the script did."""
    if db is None:
        db = {
            "names": {a: "mine_%x" % a for a in preexisting},
            "user": set(preexisting),
            "types": set(preexisting),
            "func_cmt": {},
            "cmt": {},
        }
    log = {
        "parse": [],
        "add_func": [],
        "names": {},
        "types": {},
        "func_cmt": [],
        "cmt": [],
        "refused_name": 0,
    }
    refused_type = {}
    funcs = set()

    idc = types.ModuleType("idc")
    idc.BADADDR = BADADDR

    def parse_decls(text, flags):
        log["parse"].append(text)
        return 0

    def set_type(address, declaration):
        if address == refused_type.get("address"):
            return 0
        log["types"][address] = declaration
        db["types"].add(address)
        return 1

    idc.parse_decls = parse_decls
    idc.SetType = set_type
    idc.get_type = lambda a: "int x;" if a in db["types"] else None
    idc.get_segm_start = lambda a: 0x400000 if 0x400000 <= a < 0xD06000 else BADADDR

    ida_name = types.ModuleType("ida_name")
    ida_name.SN_CHECK, ida_name.SN_NOWARN, ida_name.SN_NON_AUTO = 1, 2, 4

    def set_name(address, name, flags):
        # IDA's check refuses the characters of C++ names.
        if flags & ida_name.SN_CHECK and ("::" in name or "~" in name):
            log["refused_name"] += 1
            return False
        log["names"][address] = name
        db["names"][address] = name
        db["user"].add(address)
        return True

    ida_name.set_name = set_name
    ida_name.get_name = lambda a: db["names"].get(a, "sub_%x" % a)

    ida_funcs = types.ModuleType("ida_funcs")
    ida_funcs.get_func = lambda a: a if a in funcs else None

    def add_func(start, end):
        log["add_func"].append((start, end))
        funcs.add(start)
        return True

    def set_func_cmt(function, text, repeatable):
        log["func_cmt"].append((function, text, repeatable))
        db["func_cmt"][function] = text
        return True

    ida_funcs.add_func = add_func
    ida_funcs.get_func_cmt = lambda function, repeatable: db["func_cmt"].get(function)
    ida_funcs.set_func_cmt = set_func_cmt

    ida_bytes = types.ModuleType("ida_bytes")
    ida_bytes.get_flags = lambda a: a
    ida_bytes.has_user_name = lambda flags: flags in db["user"]
    # A name with no has_user_name is IDA's own; has_name is true for those too.
    ida_bytes.has_name = lambda flags: True

    def set_cmt(address, text, repeatable):
        log["cmt"].append((address, text, repeatable))
        db["cmt"][address] = text
        return True

    ida_bytes.set_cmt = set_cmt
    ida_bytes.get_cmt = lambda address, repeatable: db["cmt"].get(address)

    ida_nalt = types.ModuleType("ida_nalt")
    ida_nalt.retrieve_input_file_sha256 = lambda: bytes.fromhex(EXE["sha256"])
    ida_nalt.retrieve_input_file_size = lambda: EXE["size"]

    for module in (idc, ida_name, ida_funcs, ida_bytes, ida_nalt):
        sys.modules[module.__name__] = module
    script = load(os.path.join(OUT, "ida", "benilla_re.py"), "benilla_re_ida")
    script.OVERWRITE_NAMES = overwrite
    if edit is not None:
        original = script.load_bundle

        def edited(path):
            tables, document = original(path)
            edit(tables)
            return tables, document

        script.load_bundle = edited

    first_typed = next(
        int(r["address"], 16)
        for r in FUNCTIONS
        if int(r["address"], 16) in STATED and int(r["address"], 16) not in preexisting
    )
    refused_type["address"] = first_typed
    code, text = run(script, script.main)
    expect(code == 0, "IDA: main returned %r\n%s" % (code, text))
    log["script"], log["text"], log["ida_nalt"], log["db"] = script, text, ida_nalt, db
    return log


def ida():
    # A fresh database: every name is IDA's own, so every row is named, typed and commented.
    log = ida_run(overwrite=False, preexisting=set())
    script, text = log["script"], log["text"]
    expect(len(log["parse"]) == 1, "IDA: the structs are parsed once")
    parsed = log["parse"][0]
    for needle in ("typedef struct CCamera CCamera;", "#pragma pack(push, 1)", "struct CCamera {"):
        expect(needle in parsed, "IDA: the types lack %r" % needle)
    expect("extern" not in parsed and "BRE_" not in parsed, "IDA: only types are parsed")
    expect(len(log["add_func"]) == len(FUNCTIONS), "IDA: a function is made for each row")
    first = FUNCTIONS[0]
    expect(
        log["add_func"][0] == (int(first["address"], 16), int(first["address"], 16) + int(first["size"])),
        "IDA: a function gets the row's extent",
    )
    named = {int(r["address"], 16): r["name"] for r in FUNCTIONS + GLOBALS if r["name"]}
    expect(len(log["names"]) == len(named), "IDA: every named row is named")
    expect(log["refused_name"] > 100, "IDA: the stand-in refused some names")
    for address, name in named.items():
        got = log["names"][address]
        expect(got == name or got == script.c_name(name), "IDA: %#x named %r for %r" % (address, got, name))
    # The name the stand-in refuses is in the full tree; a subset of rows may lack it.
    expect(
        any(n == "CDataStore__PutInt8" for n in log["names"].values())
        or (not FULL and not any(r["name"] == "CDataStore::PutInt8" for r in FUNCTIONS)),
        "IDA: a refused name is tried as the header spells it",
    )
    expect(
        len(log["types"]) == STATED_FUNCTIONS + STATED_GLOBALS - 1,
        "IDA: %d types for %d stated rows, one refused" % (len(log["types"]), STATED_FUNCTIONS + STATED_GLOBALS),
    )
    for declaration in log["types"].values():
        expect(declaration.rstrip().endswith(";") and "BRE_" not in declaration, "IDA: declaration %r" % declaration)
    expect(any("__thiscall " in d for d in log["types"].values()), "IDA: conventions are keywords")
    expect(len(log["func_cmt"]) == len(FUNCTIONS), "IDA: every function is commented")
    expect(len(log["cmt"]) == len(GLOBALS), "IDA: every global is commented")
    expect(
        all(c[2] for c in log["func_cmt"] + log["cmt"]),
        "IDA: the default writes repeatable comments only",
    )
    expect("type refused" in text and "benilla-re: done" in text, "IDA: the refusal and the summary print")
    print("IDA ok: %d functions, %d globals, %d types" % (len(FUNCTIONS), len(GLOBALS), len(log["types"])))

    # A database that already names (and types) some items, as vmangos/idb does.
    mine = [int(r["address"], 16) for r in FUNCTIONS if r["name"]][:200]
    mine += [int(r["address"], 16) for r in GLOBALS if r["name"]][:50]
    old = set(mine)
    kept = ida_run(overwrite=False, preexisting=old)
    expect(not (old & set(kept["names"])), "IDA: a name already set is kept")
    expect(not (old & set(kept["types"])), "IDA: a type already set is kept")
    expect(
        len(kept["names"]) == len(named) - len(old),
        "IDA: every other row is named (%d)" % len(kept["names"]),
    )
    by_name = {int(r["address"], 16): r["name"] for r in FUNCTIONS + GLOBALS}
    ours = {c[0]: c[1] for c in kept["cmt"]}
    ours.update({c[0]: c[1] for c in kept["func_cmt"]})
    # ida_funcs.get_func returns the start here, so the comment's key is the function's address.
    for address in mine:
        expect(
            ours[address].startswith("benilla-re: %s\n" % by_name[address]),
            "IDA: %#x keeps its name and takes ours as a comment" % address,
        )
    expect(all(c[2] for c in kept["func_cmt"] + kept["cmt"]), "IDA: kept items get repeatable comments")
    print("IDA keeps %d names already set" % len(old))

    # A comment of the user's stays, ours is added after it.
    note = {mine[0]: "my note"}
    db = {
        "names": {a: "mine_%x" % a for a in old},
        "user": set(old),
        "types": set(old),
        "func_cmt": dict(note),
        "cmt": dict(note),
    }
    noted = ida_run(overwrite=False, preexisting=old, db=db)
    expect(
        db["func_cmt"].get(mine[0], db["cmt"].get(mine[0], "")).startswith("my note\n\nbenilla-re: "),
        "IDA: a comment of the user's is kept and ours follows it",
    )

    # A second run with a newer table: the names the first run applied are updated, a name that
    # was changed since is kept, and a comment holds one block.
    edited = set()

    def newer(tables):
        for table in tables.values():
            for row in table[:300]:
                if row.get("name") and "address" in row:
                    row["name"] += "_v2"
                    row["meaning"] += " (newer)"
                    edited.add(int(row["address"], 16))

    first = ida_run(overwrite=False, preexisting=set())
    db = first["db"]
    named_rows = [r for r in FUNCTIONS[:300] if r["name"]]
    target = int(named_rows[0]["address"], 16)
    changed = int(named_rows[1]["address"], 16)
    db["names"][changed] = "my_own_name"
    second = ida_run(overwrite=False, preexisting=set(), db=db, edit=newer)
    expect(len(edited) > (250 if FULL else 100), "IDA: the newer table changes rows")
    expect(changed not in second["names"], "IDA: a name changed since the last run is kept")
    for address in edited - {changed}:
        expect(second["names"][address].endswith("_v2"), "IDA: %#x updated" % address)
    block = db["func_cmt"][target]
    expect(block.count("benilla-re:") == 1 and "(newer)" in block, "IDA: a comment holds one block")
    third = ida_run(overwrite=False, preexisting=set(), db=db)
    expect(
        db["func_cmt"][target].count("benilla-re:") == 1,
        "IDA: a third run keeps one block",
    )
    print("IDA updates the names of an earlier run")

    # The overwrite flag names, types and comments everything.
    over = ida_run(overwrite=True, preexisting=old)
    expect(len(over["names"]) == len(named), "IDA: the overwrite flag renames every row")
    expect(not any(c[2] for c in over["func_cmt"] + over["cmt"]), "IDA: overwrite writes regular comments")
    expect(len(over["types"]) == STATED_FUNCTIONS + STATED_GLOBALS - 1, "IDA: overwrite retypes every row")

    # A database of another file is refused.
    over["ida_nalt"].retrieve_input_file_size = lambda: 1
    code, text = run(over["script"], over["script"].main)
    expect(
        REBASED or (code == 1 and "SHA-256" in text),
        "IDA: another file is refused",
    )


# ---- Binary Ninja ----


def binja_run(overwrite, preexisting, view=None, edit=None):
    """Runs the Binary Ninja script on a view where `preexisting` addresses already have a name of
    their own and a comment, or on `view`, what an earlier run left; `edit` changes the tables the
    script reads. Returns what the script did."""
    log = {"created": [], "symbols": [], "datavars": {}, "comments": {}, "text": ""}

    class Symbol:
        def __init__(self, kind, address, name):
            self.kind, self.address, self.name, self.auto = kind, address, name, False

    class Function:
        def __init__(self, address, named):
            self.start = address
            self.symbol = Symbol("function", address, "mine_%x" % address) if named else Symbol("function", address, "sub_%x" % address)
            self.comment = "old comment" if named else ""
            self.type = None

        @property
        def name(self):
            return self.symbol.name

        @name.setter
        def name(self, value):
            self.symbol = Symbol("function", self.start, value)

        def set_user_type(self, value):
            self.type = value

    class View:
        parent_view = None
        length = EXE["size"]

        def __init__(self):
            self.functions = {}
            self.symbols = {}
            self.comments = {a: "old comment" for a in preexisting}
            # The view's analysis found the functions that `preexisting` names.
            for address in preexisting:
                if address in FUNCTION_ADDRESSES:
                    self.functions[address] = Function(address, True)

        def read(self, offset, length):
            return b"\0" * length

        def get_function_at(self, address):
            return self.functions.get(address)

        def create_user_function(self, address):
            log["created"].append(address)
            self.functions[address] = Function(address, False)
            return self.functions[address]

        def parse_types_from_string(self, text):
            # The documented parser knows the stdint names; the stand-in does not, so that the
            # script's second attempt, with the names defined, is the one that parses.
            if "typedef unsigned int uint32_t;" not in text:
                raise SyntaxError("unknown type name 'uint32_t'")
            names = re.findall(r"^typedef struct (\w+) \1;$", text, re.M)
            result = types.SimpleNamespace()
            result.types = {n: ("type", n) for n in names + ["uint32_t", "int8_t"]}
            return result

        def define_user_type(self, name, type_):
            log.setdefault("defined", []).append(name)

        def parse_type_string(self, declaration):
            if "lua_State" in declaration:
                raise SyntaxError("unknown type name 'lua_State'")
            return ("type", declaration), "name"

        def is_valid_offset(self, address):
            return 0x400000 <= address < 0xD06000

        def get_symbol_at(self, address):
            if address in self.symbols:
                return self.symbols[address]
            if address in preexisting:
                return Symbol("data", address, "mine_%x" % address)
            return None

        def get_comment_at(self, address):
            return self.comments.get(address, "")

        def define_user_data_var(self, address, type_):
            log["datavars"][address] = type_

        def define_user_symbol(self, symbol):
            log["symbols"].append(symbol)
            self.symbols[symbol.address] = symbol

        def set_comment_at(self, address, text):
            log["comments"][address] = text
            self.comments[address] = text

    module = types.ModuleType("binaryninja")

    class SymbolType:
        DataSymbol = "data"

    module.Symbol, module.SymbolType = Symbol, SymbolType
    sys.modules["binaryninja"] = module
    script = load(os.path.join(OUT, "binja", "benilla_re.py"), "benilla_re_binja")
    script.OVERWRITE_NAMES = overwrite
    if edit is not None:
        original = script.load_bundle

        def edited(path):
            tables, document = original(path)
            edit(tables)
            return tables, document

        script.load_bundle = edited

    view = view or View()
    os.environ.pop("BENILLA_RE_FORCE", None)
    code, text = run(script, lambda: script.apply(view))
    expect(
        REBASED or (code == 1 and "SHA-256" in text),
        "Binja: another file is refused\n" + text,
    )
    os.environ["BENILLA_RE_FORCE"] = "1"
    code, text = run(script, lambda: script.apply(view))
    expect(code == 0, "Binja: apply returned %r\n%s" % (code, text))
    log["view"], log["text"], log["defined"] = view, text, log.get("defined", [])
    return log


FUNCTION_ADDRESSES = {int(r["address"], 16) for r in FUNCTIONS}


def binja():
    log = binja_run(overwrite=False, preexisting=set())
    view, text = log["view"], log["text"]
    expect("CCamera" in log["defined"] and "uint32_t" not in log["defined"], "Binja: the structs, not the prelude, are defined")
    expect(len(log["created"]) == len(FUNCTIONS), "Binja: a function is made for each row")
    named = {int(r["address"], 16): r["name"] for r in FUNCTIONS if r["name"]}
    for address, name in named.items():
        expect(view.functions[address].name == name, "Binja: %#x named" % address)
    typed = sum(1 for f in view.functions.values() if f.type is not None)
    refused = sum(1 for r in FUNCTIONS if int(r["address"], 16) in STATED and "lua_State" in STATED[int(r["address"], 16)])
    expect(typed == STATED_FUNCTIONS - refused, "Binja: %d typed, %d stated, %d refused" % (typed, STATED_FUNCTIONS, refused))
    for function in view.functions.values():
        expect(function.comment, "Binja: every function is commented")
        if function.type is not None:
            expect("BRE_" not in function.type[1] and not function.type[1].endswith(";"), "Binja: declaration %r" % (function.type,))
    expect(len(log["symbols"]) == sum(1 for r in GLOBALS if r["name"]), "Binja: every named global gets a symbol")
    expect(len(log["datavars"]) == STATED_GLOBALS, "Binja: every stated global is typed")
    expect(len(log["comments"]) == len(GLOBALS), "Binja: every global is commented")
    expect("type refused" in text and "benilla-re: done" in text, "Binja: the refusal and the summary print")
    print("Binja ok: %d functions, %d globals, %d types" % (len(FUNCTIONS), len(GLOBALS), len(log["datavars"])))

    # A view whose analysis or user already named some items keeps them and adds our name.
    mine = [int(r["address"], 16) for r in FUNCTIONS if r["name"]][:200]
    mine += [int(r["address"], 16) for r in GLOBALS if r["name"]][:50]
    old = set(mine)
    kept = binja_run(overwrite=False, preexisting=old)
    for address in mine:
        if address in FUNCTION_ADDRESSES:
            f = kept["view"].functions[address]
            expect(f.name == "mine_%x" % address and f.type is None, "Binja: %#x keeps its name and type" % address)
            expect(f.comment.startswith("old comment\n\nbenilla-re: "), "Binja: %#x adds our name to its comment" % address)
        else:
            expect(address not in kept["datavars"] and all(s.address != address for s in kept["symbols"]), "Binja: global %#x kept" % address)
            expect(kept["comments"][address].startswith("old comment\n\nbenilla-re: "), "Binja: global %#x comment" % address)
    # A second run with a newer table: the names the first run applied are updated, a name that
    # was changed since is kept, and a comment holds one block.
    edited = set()

    def newer(tables):
        for table in tables.values():
            for row in table[:300]:
                if row.get("name") and "address" in row:
                    row["name"] += "_v2"
                    row["meaning"] += " (newer)"
                    edited.add(int(row["address"], 16))

    first = binja_run(overwrite=False, preexisting=set())
    named_rows = [r for r in FUNCTIONS[:300] if r["name"]]
    target = int(named_rows[0]["address"], 16)
    changed = int(named_rows[1]["address"], 16)
    first["view"].functions[changed].name = "my_own_name"
    second = binja_run(overwrite=False, preexisting=set(), view=first["view"], edit=newer)
    functions = second["view"].functions
    for address in edited & FUNCTION_ADDRESSES:
        if address != changed:
            expect(functions[address].name.endswith("_v2"), "Binja: %#x updated" % address)
            expect(functions[address].comment.count("benilla-re:") == 1, "Binja: one block at %#x" % address)
    expect(functions[changed].name == "my_own_name", "Binja: a name changed since the last run is kept")
    for address in edited - FUNCTION_ADDRESSES:
        expect(second["view"].symbols[address].name.endswith("_v2"), "Binja: global %#x updated" % address)
    print("Binja updates the names of an earlier run")

    over = binja_run(overwrite=True, preexisting=old)
    expect(all(over["view"].functions[a].name == named[a] for a in named), "Binja: overwrite renames every function")
    print("Binja keeps %d items already named" % len(old))


ida()
binja()


# A headless run ends the process with the status; the GUI's run does not raise SystemExit.
def binja_exit():
    path = os.path.join(OUT, "binja", "benilla_re.py")
    for gui in (False, True):
        module = types.ModuleType("binaryninja")
        module.Symbol = module.SymbolType = None
        view = types.SimpleNamespace(
            parent_view=None,
            length=1,
            read=lambda a, b: b"x",
            file=types.SimpleNamespace(close=lambda: None),
        )
        module.load = lambda path: view
        sys.modules["binaryninja"] = module
        sys.argv = ["benilla_re.py", "WoW.exe"]
        namespace = {"__name__": "__main__", "__file__": path, "__doc__": ""}
        if gui:
            namespace["bv"] = view
        os.environ.pop("BENILLA_RE_FORCE", None)
        out = io.StringIO()
        raised = False
        try:
            with contextlib.redirect_stdout(out):
                exec(compile(open(path, encoding="utf-8").read(), path, "exec"), namespace)
        except SystemExit:
            raised = True
        expect(raised != gui, "Binja: SystemExit %r when gui is %r" % (raised, gui))
    print("Binja exits only when headless")


if not REBASED:  # the stand-in view is refused as another file, which a rebased bundle never does
    binja_exit()
