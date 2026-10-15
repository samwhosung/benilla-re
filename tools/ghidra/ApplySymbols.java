// Post-script (after CreateFunctions): applies the symbol table to the program.
//   structs.tsv, fields.tsv  struct types, in the category of their namespace
//   functions.tsv            names (Class::Method as namespaces), signatures where given, and a
//                            plate comment with the meaning, the confidence and the finding
//   globals.tsv              labels, their types, and a plate comment
// Arguments: the symbols directory, and optionally `keep`. With `keep`, an item whose name somebody
// set (source user-defined or imported, not default, analysis or AI) keeps its name and its type,
// and takes ours as a block of its repeatable comment; the rest of the comment stays.
// Every name this script applies leaves a `benilla-re: <name>` line in the repeatable comment. An
// item whose name is still the one that line records was named by an earlier run, so `keep` names
// it again from the newer table.
// Prints one FAIL line per row it could not apply, with its reason in brackets, and, last, a
// summary: the rows, the names and signatures in the program, and the failures by reason. The
// reasons "signature outside the grammar" and "__thiscall without this" are defects of the table.
//
// @category benilla

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.data.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;

import java.io.BufferedReader;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class ApplySymbols extends GhidraScript {

    private static final Pattern FUNC_POINTER =
            Pattern.compile("^(.+?)\\s*\\((?:__\\w+\\s*)?(\\*+)(?:\\[(\\d+)\\])?(\\w*)\\)\\s*\\(.*\\)$");
    private static final Pattern ARRAY = Pattern.compile("^(.*?)\\s*\\[(\\d+)\\]$");
    private static final Pattern SIGNATURE = Pattern.compile("^(.+?)\\s*\\b(__\\w+)\\s+\\((.*)\\)$");
    private static final Pattern PARAM = Pattern.compile("^(.*?)([A-Za-z_]\\w*)$");

    private DataTypeManager dtm;
    private SymbolTable symtab;
    private final Map<String, Structure> structs = new TreeMap<>();
    private int failures = 0;
    private final Map<String, Integer> reasons = new TreeMap<>();
    private final Map<String, DataType> opaque = new TreeMap<>();
    private int thiscallBare = 0;
    private final Set<Long> applied = new HashSet<>();
    private boolean keep = false;
    private final Set<Long> kept = new HashSet<>();

    // Reasons that are defects of the table, outside what the importer can apply (issue 81).
    private static final Set<String> TABLE_DEFECT = new HashSet<>(Arrays.asList(
            "signature outside the grammar", "__thiscall without this"));

    private void fail(String reason, String what) {
        failures++;
        reasons.merge(reason, 1, Integer::sum);
        println("ApplySymbols: FAIL [" + reason + "] " + what);
    }

    private List<String[]> read(String dir, String file) throws Exception {
        List<String[]> rows = new ArrayList<>();
        try (BufferedReader r = Files.newBufferedReader(Paths.get(dir, file))) {
            r.readLine();
            String line;
            while ((line = r.readLine()) != null) {
                rows.add(line.split("\t", -1));
            }
        }
        return rows;
    }

    // ---- types ----

    private static final Pattern HANDLE_NAME = Pattern.compile("^(H[A-Z0-9]+|\\w+_CFunction)$");
    private static final Pattern IDENT = Pattern.compile("^[A-Za-z_]\\w*(::[A-Za-z_]\\w*)*$");

    // The C types of the table that are not structs: the builtins, the stdint family and the
    // names the header's includes define.
    private DataType builtin(String name) {
        switch (name) {
            case "void": return VoidDataType.dataType;
            case "bool": return BooleanDataType.dataType;
            case "char": return CharDataType.dataType;
            case "signed char": case "int8_t": return SignedByteDataType.dataType;
            case "unsigned char": case "uint8_t": return ByteDataType.dataType;
            case "short": case "int16_t": return ShortDataType.dataType;
            case "unsigned short": case "uint16_t": return UnsignedShortDataType.dataType;
            case "int": case "int32_t": case "long": case "intptr_t": case "ptrdiff_t":
                return IntegerDataType.dataType;
            case "unsigned": case "unsigned int": case "uint32_t": case "unsigned long":
            case "uintptr_t": case "size_t":
                return UnsignedIntegerDataType.dataType;
            case "long long": case "int64_t": return LongLongDataType.dataType;
            case "unsigned long long": case "uint64_t": return UnsignedLongLongDataType.dataType;
            case "wchar_t": return UnsignedShortDataType.dataType;
            case "float": return FloatDataType.dataType;
            case "double": return DoubleDataType.dataType;
            case "va_list": return pointerTo(VoidDataType.dataType);
            default:
                // A handle or a callback typedef of the SDK or the client: a pointer.
                return HANDLE_NAME.matcher(name).matches() ? pointerTo(VoidDataType.dataType) : null;
        }
    }

    private static CategoryPath categoryOf(String name) {
        List<String> q = qualifier(name);
        return q.isEmpty() ? CategoryPath.ROOT : new CategoryPath(CategoryPath.ROOT, q);
    }

    // A type the table names but does not define, as an empty struct, so that pointers to it
    // work. Null when the name is no type name.
    private DataType opaqueType(String name) {
        if (!IDENT.matcher(name).matches()) return null;
        DataType dt = opaque.get(name);
        if (dt == null) {
            StructureDataType s = new StructureDataType(categoryOf(name), leaf(name), 0, dtm);
            s.setDescription("opaque: named by the table, not defined");
            dt = dtm.addDataType(s, DataTypeConflictHandler.KEEP_HANDLER);
            opaque.put(name, dt);
        }
        return dt;
    }

    private DataType baseType(String name, boolean viaPointer) {
        name = name.replaceFirst("^(struct|union|enum)\\s+", "");
        DataType dt = builtin(name);
        if (dt == null) dt = structs.get(name);
        if (dt == null && viaPointer) dt = opaqueType(name);
        return dt;
    }

    private DataType pointerTo(DataType dt) {
        return new PointerDataType(dt, dtm);
    }

    private DataType type(String t) {
        return type(t, false);
    }

    // The type of a declaration's type text. `viaPointer` lets a name the table never defines
    // stand as an opaque struct, which only a pointer to it can use.
    private DataType type(String t, boolean viaPointer) {
        t = t.trim().replaceFirst("^const\\s+", "").replaceAll("\\s+", " ");
        Matcher fp = FUNC_POINTER.matcher(t);
        if (fp.matches()) {
            // A pointer to a function is a pointer to void here; the signature is not modelled.
            DataType dt = pointerTo(VoidDataType.dataType);
            for (int i = 1; i < fp.group(2).length(); i++) {
                dt = pointerTo(dt);
            }
            return fp.group(3) != null ? new ArrayDataType(dt, Integer.parseInt(fp.group(3)), dt.getLength()) : dt;
        }
        Matcher ar = ARRAY.matcher(t);
        if (ar.matches()) {
            DataType el = type(ar.group(1), viaPointer);
            return el == null ? null : new ArrayDataType(el, Integer.parseInt(ar.group(2)), el.getLength());
        }
        if (t.endsWith("*")) {
            DataType to = type(t.substring(0, t.length() - 1), true);
            return to == null ? null : pointerTo(to);
        }
        t = t.replaceFirst("\\s+const$", "");
        return baseType(t.trim(), viaPointer);
    }

    // ---- names ----

    // The namespace for the segments, the last one a class.
    private Namespace namespace(List<String> segments) throws Exception {
        Namespace cur = currentProgram.getGlobalNamespace();
        for (int i = 0; i < segments.size(); i++) {
            Namespace next = symtab.getNamespace(segments.get(i), cur);
            if (next == null) {
                next = i == segments.size() - 1
                        ? symtab.createClass(cur, segments.get(i), SourceType.USER_DEFINED)
                        : symtab.createNameSpace(cur, segments.get(i), SourceType.USER_DEFINED);
            }
            cur = next;
        }
        return cur;
    }

    private static List<String> qualifier(String name) {
        List<String> parts = new ArrayList<>(Arrays.asList(name.split("::")));
        parts.remove(parts.size() - 1);
        return parts;
    }

    private static String leaf(String name) {
        int i = name.lastIndexOf("::");
        return i < 0 ? name : name.substring(i + 2);
    }

    private static String findingPath(String finding) {
        if (finding.isEmpty()) return "";
        int h = finding.indexOf('#');
        String doc = h < 0 ? finding : finding.substring(0, h);
        String rest = h < 0 ? "" : finding.substring(h);
        return (doc.equals("METHOD") ? "docs/METHOD.md" : "systems/" + doc + ".md") + rest;
    }

    private static final String MARK = "benilla-re:";

    // A comment as {the text outside our block, the name our block records or null}. Our block runs
    // from its `benilla-re:` line to the next empty line.
    private static String[] splitComment(String existing) {
        if (existing == null) existing = "";
        String[] lines = existing.split("\n", -1);
        for (int start = 0; start < lines.length; start++) {
            if (!lines[start].startsWith(MARK)) continue;
            int end = start;
            while (end < lines.length && !lines[end].isEmpty()) end++;
            List<String> rest = new ArrayList<>(Arrays.asList(lines).subList(0, start));
            rest.addAll(Arrays.asList(lines).subList(end, lines.length));
            return new String[] {
                String.join("\n", rest).replaceAll("^\n+|\n+$", ""),
                lines[start].substring(MARK.length()).trim()
            };
        }
        return new String[] {existing.replaceAll("^\n+|\n+$", ""), null};
    }

    // Whether an earlier run named the item: its name is still the one the run recorded.
    private static boolean named(String[] parts, String current) {
        // An import's thunk keeps the library's namespace, so a name without one matches the leaf.
        return parts[1] != null && !parts[1].isEmpty()
                && (parts[1].equals(current) || !parts[1].contains("::") && parts[1].equals(leaf(current)));
    }

    // Whether `keep` leaves the item's name and type alone.
    private boolean keeps(SourceType source, String[] parts, String current) {
        return keep && (source == SourceType.USER_DEFINED || source == SourceType.IMPORTED)
                && !named(parts, current);
    }

    private String repeatable(Address a) {
        return currentProgram.getListing().getComment(CommentType.REPEATABLE, a);
    }

    // Writes our block after the text of the repeatable comment that is not ours.
    private void writeBlock(Address a, String[] parts, String name, String text) {
        String block = MARK + " " + (name.isEmpty() ? "(no name)" : name) + (text.isEmpty() ? "" : "\n" + text);
        currentProgram.getListing().setComment(a, CommentType.REPEATABLE,
                parts[0].isEmpty() ? block : parts[0] + "\n\n" + block);
    }

    private String comment(String[] c, int meaning, int confidence, int proof, int finding) {
        StringBuilder sb = new StringBuilder(c[meaning]);
        sb.append("\n").append(c[confidence]).append(", proof ").append(c[proof]);
        if (!c[finding].isEmpty()) sb.append("\nFinding: ").append(findingPath(c[finding]));
        return sb.toString();
    }

    // ---- structs ----

    private void applyStructs(String dir) throws Exception {
        List<String[]> srows = read(dir, "structs.tsv");
        List<String[]> frows = read(dir, "fields.tsv");
        // Shells first, so that fields can point at any struct.
        for (String[] c : srows) {
            List<String> q = qualifier(c[0]);
            CategoryPath cat = q.isEmpty() ? CategoryPath.ROOT : new CategoryPath(CategoryPath.ROOT, q);
            StructureDataType s = new StructureDataType(cat, leaf(c[0]), Integer.parseInt(c[1]), dtm);
            s.setDescription(c[4]);
            structs.put(c[0], (Structure) dtm.addDataType(s, DataTypeConflictHandler.REPLACE_HANDLER));
        }
        sdkStructs();
        int fields = 0;
        for (String[] c : frows) {
            Structure s = structs.get(c[0]);
            if (s == null) { fail("field without struct", "field " + c[0] + "::" + c[4]); continue; }
            DataType dt = type(c[3]);
            if (dt == null) { fail("unknown type", "field " + c[0] + "::" + c[4] + ": " + c[3]); continue; }
            int off = Integer.decode(c[1]);
            int size = Integer.parseInt(c[2]);
            if (dt.getLength() != size) { fail("field size", "field " + c[0] + "::" + c[4] + ": " + c[3] + " is " + dt.getLength() + " bytes, row says " + size); continue; }
            s.replaceAtOffset(off, dt, size, c[4], c[6]);
            fields++;
        }
        println("ApplySymbols: structs=" + structs.size() + " fields=" + fields);
    }

    // The SDK structs the table passes by value, unless the table defines them.
    private void sdkStructs() {
        String[][] defs = {
            {"RECT", "int left", "int top", "int right", "int bottom"},
            {"POINT", "int x", "int y"},
            {"SIZE", "int cx", "int cy"},
            {"MSG", "void * hwnd", "unsigned int message", "unsigned int wParam", "int lParam",
                "unsigned int time", "POINT pt"},
        };
        for (String[] d : defs) {
            if (structs.containsKey(d[0])) continue;
            StructureDataType s = new StructureDataType(CategoryPath.ROOT, d[0], 0, dtm);
            for (int i = 1; i < d.length; i++) {
                int sp = d[i].lastIndexOf(' ');
                s.add(type(d[i].substring(0, sp)), d[i].substring(sp + 1), null);
            }
            structs.put(d[0], (Structure) dtm.addDataType(s, DataTypeConflictHandler.REPLACE_HANDLER));
        }
    }

    // ---- functions ----

    private List<String> splitParams(String s) {
        List<String> out = new ArrayList<>();
        int depth = 0, start = 0;
        for (int i = 0; i < s.length(); i++) {
            char ch = s.charAt(i);
            if (ch == '(') depth++;
            else if (ch == ')') depth--;
            else if (ch == ',' && depth == 0) { out.add(s.substring(start, i).trim()); start = i + 1; }
        }
        if (!s.substring(start).trim().isEmpty()) out.add(s.substring(start).trim());
        return out;
    }

    // The parts of a signature: return type, convention, parameters. Null outside the grammar.
    private String[] parseSignature(String sig) {
        if (sig.contains("/*") || sig.contains("@")) return null;
        Matcher m = SIGNATURE.matcher(sig);
        if (!m.matches()) return null;
        return new String[] {m.group(1), m.group(2), m.group(3)};
    }

    private static final Pattern THIS = Pattern.compile("(^|[\\s*])this$");

    // Returns null when applied, else {reason, detail}.
    private String[] applySignature(Function f, String sig, String rowName) throws Exception {
        String[] parts = parseSignature(sig);
        if (parts == null) return new String[] {"signature outside the grammar", sig};
        DataType ret = type(parts[0]);
        if (ret == null) return new String[] {"unknown type", "return type " + parts[0]};
        String conv = parts[1];
        List<Variable> params = new ArrayList<>();
        List<String> ps = splitParams(parts[2]);
        if (ps.size() == 1 && ps.get(0).equals("void")) ps.clear();
        boolean varargs = !ps.isEmpty() && ps.get(ps.size() - 1).equals("...");
        if (varargs) ps.remove(ps.size() - 1);
        if (conv.equals("__thiscall")) {
            if (ps.isEmpty()) {
                thiscallBare++;
            } else if (THIS.matcher(ps.get(0)).find()) {
                ps.remove(0); // `this` is the convention's own parameter
            } else {
                return new String[] {"__thiscall without this", sig};
            }
        }
        int n = 0;
        for (String p : ps) {
            String name, ty;
            Matcher fp = FUNC_POINTER.matcher(p);
            Matcher pm = PARAM.matcher(p);
            if (fp.matches()) {
                name = fp.group(4);
                ty = fp.group(1) + " (" + fp.group(2) + ")(void)";
            } else if (pm.matches() && !pm.group(1).trim().isEmpty()
                    && (pm.group(1).endsWith(" ") || pm.group(1).endsWith("*"))) {
                ty = pm.group(1);
                name = pm.group(2);
            } else {
                ty = p;
                name = "param_" + (n + 1);
            }
            // A parameter declared as an array is a pointer to its element.
            Matcher arr = Pattern.compile("^(.*?)(\\w+)\\s*\\[\\d*\\]$").matcher(p);
            if (!fp.matches() && arr.matches()) {
                ty = arr.group(1).trim() + " *";
                name = arr.group(2);
            }
            DataType dt = type(ty);
            if (dt == null || dt.getLength() == 0) return new String[] {"unknown type", "parameter type " + ty};
            if (name.isEmpty()) name = "param_" + (n + 1);
            params.add(new ParameterImpl(name, dt, currentProgram));
            n++;
        }
        // A function of one jump is a thunk, and a thunk takes its target's convention and
        // parameters; the row's signature is the function's own. An import's thunk stays one.
        if (f.isThunk() && !f.getThunkedFunction(true).isExternal()) {
            List<String> q = qualifier(rowName);
            Namespace own = q.isEmpty() ? currentProgram.getGlobalNamespace() : namespace(q);
            f.setThunkedFunction(null);
            f.setParentNamespace(own);
            if (!rowName.isEmpty()) f.setName(leaf(rowName), SourceType.USER_DEFINED);
        }
        f.setCallingConvention(conv);
        f.updateFunction(conv, new ReturnParameterImpl(ret, currentProgram),
                params, Function.FunctionUpdateType.DYNAMIC_STORAGE_FORMAL_PARAMS, true, SourceType.USER_DEFINED);
        f.setVarArgs(varargs);
        return null;
    }

    private int applyFunctions(String dir) throws Exception {
        int named = 0, sigs = 0;
        for (String[] c : read(dir, "functions.tsv")) {
            Address a = toAddr(Long.parseLong(c[0].substring(2), 16));
            Function f = getFunctionAt(a);
            if (f == null) { fail("no function", "function " + c[0] + " " + c[2]); continue; }
            try {
                String[] parts = splitComment(repeatable(a));
                if (keeps(f.getSymbol().getSource(), parts, f.getName(true))) {
                    kept.add(a.getOffset());
                    println("ApplySymbols: kept " + c[0] + " " + f.getName(true) + " (row " + c[2] + ")");
                    writeBlock(a, parts, c[2], comment(c, 7, 8, 9, 10));
                    continue;
                }
                if (!c[2].isEmpty()) {
                    List<String> q = qualifier(c[2]);
                    if (!q.isEmpty()) f.setParentNamespace(namespace(q));
                    f.setName(leaf(c[2]), SourceType.USER_DEFINED);
                    named++;
                }
                if (!c[6].isEmpty()) {
                    String[] why = applySignature(f, c[6], c[2]);
                    if (why != null) fail(why[0], "signature " + c[0] + " " + c[2] + ": " + why[1]); else { sigs++; applied.add(a.getOffset()); }
                }
                setPlateComment(a, comment(c, 7, 8, 9, 10));
                if (!c[2].isEmpty()) writeBlock(a, parts, c[2], "");
            } catch (Exception e) {
                fail("exception", "function " + c[0] + " " + c[2] + ": " + e);
            }
        }
        println("ApplySymbols: functions named=" + named + " signatures=" + sigs);
        return named;
    }

    // ---- globals ----

    private void applyGlobals(String dir) throws Exception {
        int named = 0, typed = 0;
        for (String[] c : read(dir, "globals.tsv")) {
            Address a = toAddr(Long.parseLong(c[0].substring(2), 16));
            try {
                if (!currentProgram.getMemory().contains(a)) { fail("global not in memory", "global " + c[0] + " " + c[2]); continue; }
                Symbol existing = symtab.getPrimarySymbol(a);
                String[] parts = splitComment(repeatable(a));
                if (existing != null && keeps(existing.getSource(), parts, existing.getName(true))) {
                    kept.add(a.getOffset());
                    println("ApplySymbols: kept " + c[0] + " " + existing.getName(true) + " (row " + c[2] + ")");
                    writeBlock(a, parts, c[2], comment(c, 5, 6, 7, 8));
                    continue;
                }
                DataType dt = type(c[1]);
                if (dt == null) { fail("unknown type", "global " + c[0] + " " + c[2] + ": " + c[1]); continue; }
                if (getInstructionContaining(a) != null) { fail("global over code", "global " + c[0] + " " + c[2]); continue; }
                if (!c[2].isEmpty()) {
                    List<String> q = qualifier(c[2]);
                    Symbol s = symtab.createLabel(a, leaf(c[2]),
                            q.isEmpty() ? currentProgram.getGlobalNamespace() : namespace(q), SourceType.USER_DEFINED);
                    s.setPrimary();
                    named++;
                }
                clearListing(a, a.add(dt.getLength() - 1));
                createData(a, dt);
                typed++;
                setPlateComment(a, comment(c, 5, 6, 7, 8));
                if (!c[2].isEmpty()) writeBlock(a, parts, c[2], "");
            } catch (Exception e) {
                fail("exception", "global " + c[0] + " " + c[2] + ": " + e);
            }
        }
        println("ApplySymbols: globals named=" + named + " typed=" + typed);
    }

    // ---- verification ----

    // The number of parameters Ghidra shows for an applied signature: `this` is the convention's
    // own, and `...` is no parameter.
    private int expectedParameters(String sig) {
        String[] parts = parseSignature(sig);
        List<String> ps = splitParams(parts[2]);
        if (ps.size() == 1 && ps.get(0).equals("void")) ps.clear();
        if (!ps.isEmpty() && ps.get(ps.size() - 1).equals("...")) ps.remove(ps.size() - 1);
        int n = ps.size();
        // `this` is a parameter of the program's function, listed or not.
        if (parts[1].equals("__thiscall")) n = n > 0 ? n : 1;
        return n;
    }

    private void verify(String dir) throws Exception {
        int fRows = 0, fNamed = 0, fOk = 0, sRows = 0, sOk = 0, gRows = 0, gNamed = 0, gOk = 0;
        for (String[] c : read(dir, "functions.tsv")) {
            fRows++;
            long off = Long.parseLong(c[0].substring(2), 16);
            Function f = getFunctionAt(toAddr(off));
            if (!c[2].isEmpty() && !kept.contains(off)) {
                fNamed++;
                if (f != null && (f.getName(true).equals(c[2]) || f.isThunk() && f.getName().equals(c[2]))) fOk++;
                else println("ApplySymbols: MISMATCH function " + c[0] + ": row " + c[2] + ", program " + (f == null ? "none" : f.getName(true)));
            }
            if (!c[6].isEmpty() && applied.contains(off)) {
                sRows++;
                if (f != null && f.getParameterCount() == expectedParameters(c[6])
                        && f.getCallingConventionName().equals(parseSignature(c[6])[1])) sOk++;
                else println("ApplySymbols: MISMATCH signature " + c[0] + ": row " + c[6] + ", program "
                        + (f == null ? "none" : f.getPrototypeString(true, true) + " [" + f.getCallingConventionName() + ", "
                        + f.getParameterCount() + " parameters]"));
            }
        }
        for (String[] c : read(dir, "globals.tsv")) {
            gRows++;
            if (c[2].isEmpty() || kept.contains(Long.parseLong(c[0].substring(2), 16))) continue;
            gNamed++;
            Symbol s = symtab.getPrimarySymbol(toAddr(Long.parseLong(c[0].substring(2), 16)));
            if (s != null && s.getName(true).equals(c[2])) gOk++;
            else println("ApplySymbols: MISMATCH global " + c[0] + ": row " + c[2] + ", program " + (s == null ? "none" : s.getName(true)));
        }
        int sigRows = 0, defects = 0;
        for (String[] c : read(dir, "functions.tsv")) if (!c[6].isEmpty()) sigRows++;
        println("ApplySymbols: summary");
        println("ApplySymbols:   function rows " + fRows + ", with a signature " + sigRows + ", global rows " + gRows);
        println("ApplySymbols:   kept " + kept.size() + " items with names of their own");
        println("ApplySymbols:   function names in the program " + fOk + "/" + fNamed);
        println("ApplySymbols:   signatures applied " + applied.size() + ", in the program " + sOk + "/" + sRows
                + " (parameter count and convention)");
        println("ApplySymbols:   global names in the program " + gOk + "/" + gNamed);
        println("ApplySymbols:   __thiscall rows without parameters, applied as they are " + thiscallBare);
        println("ApplySymbols:   opaque types made " + opaque.size());
        println("ApplySymbols:   failures " + failures);
        for (Map.Entry<String, Integer> e : reasons.entrySet()) {
            boolean defect = TABLE_DEFECT.contains(e.getKey());
            if (defect) defects += e.getValue();
            println("ApplySymbols:     " + e.getValue() + " " + e.getKey() + (defect ? " (table defect)" : ""));
        }
        println("ApplySymbols:   failures that are table defects " + defects + ", others " + (failures - defects));
    }

    @Override
    public void run() throws Exception {
        String dir = getScriptArgs()[0];
        keep = getScriptArgs().length > 1 && getScriptArgs()[1].equals("keep");
        if (keep) println("ApplySymbols: keep");
        dtm = currentProgram.getDataTypeManager();
        symtab = currentProgram.getSymbolTable();
        applyStructs(dir);
        applyFunctions(dir);
        applyGlobals(dir);
        verify(dir);
    }
}
