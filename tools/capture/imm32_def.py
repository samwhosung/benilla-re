#!/usr/bin/env python3
"""Writes the module-definition file that makes a rig DLL a complete stand-in for imm32.dll.

The rig is loaded in place of imm32.dll, so it has to export every function imm32.dll exports:
the client imports a few, and the system's user32 loads others on demand, and a missing export
ends the process. Each export is forwarded to the real imm32, copied beside the rig as
imm32_orig.dll, so the IME behaves as it would without the rig.

Usage: imm32_def.py <imm32.dll> <output.def>

Reads the export directory of the given 32-bit imm32.dll and writes one forwarder per named
export, keeping its ordinal. An export with no name is listed on stderr and left out.
"""

import struct
import sys

CLIENT_IMPORTS = {
    "ImmAssociateContext",
    "ImmAssociateContextEx",
    "ImmGetCandidateListA",
    "ImmGetCompositionStringA",
    "ImmGetContext",
    "ImmGetConversionStatus",
    "ImmNotifyIME",
    "ImmReleaseContext",
    "ImmSetConversionStatus",
}


def exports(data):
    """The (ordinal, name) pairs of a PE32 image's export directory; name is None if unnamed."""
    if data[:2] != b"MZ":
        raise ValueError("not an MZ image")
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe : pe + 4] != b"PE\0\0":
        raise ValueError("no PE signature")
    machine, sections, _, _, _, optional_size, _ = struct.unpack_from("<HHIIIHH", data, pe + 4)
    if machine != 0x14C:
        raise ValueError(f"machine 0x{machine:x} is not i386")
    optional = pe + 24
    if struct.unpack_from("<H", data, optional)[0] != 0x10B:
        raise ValueError("not a PE32 optional header")
    export_rva, export_size = struct.unpack_from("<II", data, optional + 96)
    if export_size == 0:
        return []
    table = optional + optional_size
    spans = []
    for i in range(sections):
        size, rva, raw_size, raw = struct.unpack_from("<IIII", data, table + 40 * i + 8)
        spans.append((rva, max(size, raw_size), raw))

    def offset(rva):
        for start, size, raw in spans:
            if start <= rva < start + size:
                return rva - start + raw
        raise ValueError(f"RVA 0x{rva:x} is in no section")

    def cstring(rva):
        at = offset(rva)
        return data[at : data.index(b"\0", at)].decode("ascii")

    d = offset(export_rva)
    base, functions, names, functions_rva, names_rva, ordinals_rva = struct.unpack_from(
        "<IIIIII", data, d + 16
    )
    named = {}
    for i in range(names):
        name_rva = struct.unpack_from("<I", data, offset(names_rva) + 4 * i)[0]
        index = struct.unpack_from("<H", data, offset(ordinals_rva) + 2 * i)[0]
        named[index] = cstring(name_rva)
    result = []
    for index in range(functions):
        if struct.unpack_from("<I", data, offset(functions_rva) + 4 * index)[0] == 0:
            continue
        result.append((base + index, named.get(index)))
    return result


def main(argv):
    if len(argv) != 3:
        sys.stderr.write(__doc__)
        return 2
    with open(argv[1], "rb") as f:
        data = f.read()
    found = exports(data)
    lines = ["LIBRARY imm32", "EXPORTS"]
    for ordinal, name in found:
        if name is None:
            sys.stderr.write(f"imm32_def: ordinal {ordinal} has no name; not forwarded\n")
            continue
        lines.append(f"{name} = imm32_orig.{name} @{ordinal}")
    count = len(lines) - 2
    # A control: the nine functions WoW.exe imports from IMM32.dll (its import table) must all be
    # there, or this is not the file the client needs.
    missing = sorted(CLIENT_IMPORTS - {name for _, name in found})
    if missing:
        sys.stderr.write(f"imm32_def: {argv[1]} lacks {', '.join(missing)}; not imm32.dll\n")
        return 1
    with open(argv[2], "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"imm32_def: {count} exports forwarded from {argv[1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
