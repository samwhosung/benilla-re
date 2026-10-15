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

# The addresses the header states, from the header itself.
STATED = {}
with open(os.path.join(OUT, "benilla-re.h"), encoding="utf-8") as handle:
    for line in handle:
        match = re.match(r"^/\* (0x[0-9a-f]{6}) \*/ ", line)
        if match:
            STATED[int(match.group(1), 16)] = line
STATED_FUNCTIONS = sum(1 for r in FUNCTIONS if int(r["address"], 16) in STATED)
STATED_GLOBALS = sum(1 for r in GLOBALS if int(r["address"], 16) in STATED)
expect(STATED_FUNCTIONS > 15000 and STATED_GLOBALS > 4000, "the header states few rows")


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


def ida():
    log = {
        "parse": [],
        "add_func": [],
        "names": {},
        "types": {},
        "func_cmt": 0,
        "cmt": 0,
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
        return 1

    idc.parse_decls = parse_decls
    idc.SetType = set_type
    idc.get_segm_start = lambda a: 0x400000 if 0x400000 <= a < 0xD06000 else BADADDR

    ida_name = types.ModuleType("ida_name")
    ida_name.SN_CHECK, ida_name.SN_NOWARN, ida_name.SN_NON_AUTO = 1, 2, 4

    def set_name(address, name, flags):
        # IDA's check refuses the characters of C++ names.
        if flags & ida_name.SN_CHECK and ("::" in name or "~" in name):
            log["refused_name"] += 1
            return False
        log["names"][address] = name
        return True

    ida_name.set_name = set_name

    ida_funcs = types.ModuleType("ida_funcs")
    ida_funcs.get_func = lambda a: a if a in funcs else None

    def add_func(start, end):
        log["add_func"].append((start, end))
        funcs.add(start)
        return True

    def set_func_cmt(function, text, repeatable):
        log["func_cmt"] += 1
        return True

    ida_funcs.add_func = add_func
    ida_funcs.set_func_cmt = set_func_cmt

    ida_bytes = types.ModuleType("ida_bytes")

    def set_cmt(address, text, repeatable):
        log["cmt"] += 1
        return True

    ida_bytes.set_cmt = set_cmt

    ida_nalt = types.ModuleType("ida_nalt")
    ida_nalt.retrieve_input_file_sha256 = lambda: bytes.fromhex(EXE["sha256"])
    ida_nalt.retrieve_input_file_size = lambda: EXE["size"]

    for module in (idc, ida_name, ida_funcs, ida_bytes, ida_nalt):
        sys.modules[module.__name__] = module
    script = load(os.path.join(OUT, "ida", "benilla_re.py"), "benilla_re_ida")

    first_typed = next(
        int(r["address"], 16) for r in FUNCTIONS if int(r["address"], 16) in STATED
    )
    refused_type["address"] = first_typed
    code, text = run(script, script.main)
    expect(code == 0, "IDA: main returned %r\n%s" % (code, text))
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
    expect(
        any(n == "CDataStore__PutInt8" for n in log["names"].values()),
        "IDA: a refused name is tried as the header spells it",
    )
    expect(
        len(log["types"]) == STATED_FUNCTIONS + STATED_GLOBALS - 1,
        "IDA: %d types for %d stated rows, one refused" % (len(log["types"]), STATED_FUNCTIONS + STATED_GLOBALS),
    )
    for declaration in log["types"].values():
        expect(declaration.rstrip().endswith(";") and "BRE_" not in declaration, "IDA: declaration %r" % declaration)
    expect(any("__thiscall " in d for d in log["types"].values()), "IDA: conventions are keywords")
    expect(log["func_cmt"] == len(FUNCTIONS) - 0, "IDA: every function is commented")
    expect(log["cmt"] == len(GLOBALS), "IDA: every global is commented")
    expect("type refused" in text and "benilla-re: done" in text, "IDA: the refusal and the summary print")
    print("IDA ok: %d functions, %d globals, %d types" % (len(FUNCTIONS), len(GLOBALS), len(log["types"])))

    # A database of another file is refused.
    ida_nalt.retrieve_input_file_size = lambda: 1
    code, text = run(script, script.main)
    expect(code == 1 and "SHA-256" in text, "IDA: another file is refused")


# ---- Binary Ninja ----


def binja():
    log = {"defined": [], "created": [], "symbols": [], "datavars": {}, "comments": 0}

    class Function:
        def __init__(self, address):
            self.start = address
            self.name = None
            self.comment = None
            self.type = None

        def set_user_type(self, value):
            self.type = value

    class View:
        parent_view = None
        length = EXE["size"]

        def __init__(self):
            self.functions = {}
            self.typed_together = 0

        def read(self, offset, length):
            return b"\0" * length

        def get_function_at(self, address):
            return self.functions.get(address)

        def create_user_function(self, address):
            log["created"].append(address)
            self.functions[address] = Function(address)
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
            log["defined"].append(name)

        def parse_type_string(self, declaration):
            if "lua_State" in declaration:
                raise SyntaxError("unknown type name 'lua_State'")
            return ("type", declaration), "name"

        def is_valid_offset(self, address):
            return 0x400000 <= address < 0xD06000

        def define_user_data_var(self, address, type_):
            log["datavars"][address] = type_

        def define_user_symbol(self, symbol):
            log["symbols"].append(symbol)

        def set_comment_at(self, address, text):
            log["comments"] += 1

    module = types.ModuleType("binaryninja")

    class SymbolType:
        DataSymbol = "data"

    class Symbol:
        def __init__(self, kind, address, name):
            self.kind, self.address, self.name = kind, address, name

    module.Symbol, module.SymbolType = Symbol, SymbolType
    sys.modules["binaryninja"] = module
    script = load(os.path.join(OUT, "binja", "benilla_re.py"), "benilla_re_binja")

    view = View()
    code, text = run(script, lambda: script.apply(view))
    expect(code == 1 and "SHA-256" in text, "Binja: another file is refused\n" + text)
    os.environ["BENILLA_RE_FORCE"] = "1"
    code, text = run(script, lambda: script.apply(view))
    expect(code == 0, "Binja: apply returned %r\n%s" % (code, text))
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
    expect(log["comments"] == len(GLOBALS), "Binja: every global is commented")
    expect("type refused" in text and "benilla-re: done" in text, "Binja: the refusal and the summary print")
    print("Binja ok: %d functions, %d globals, %d types" % (len(FUNCTIONS), len(GLOBALS), len(log["datavars"])))


ida()
binja()
