// Post-script (after CreateFunctions): applies the symbol table to the program.
//   structs.tsv, fields.tsv  struct types, in the category of their namespace
//   functions.tsv            names (Class::Method as namespaces), signatures where given, and a
//                            plate comment with the meaning, the confidence and the finding
//   globals.tsv              labels, their types, and a plate comment
// Arguments: the symbols directory, and optionally `keep`. With `keep`, an item whose name is not
// Ghidra's own default keeps its name and its type, and takes ours as a repeatable comment.
// Prints one FAIL line per row it could not apply and, last, a verification that every row's name
// is the name in the program.
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
            Pattern.compile("^(.+?)\\s*\\((\\*+)(?:\\[(\\d+)\\])?(\\w*)\\)\\s*\\(.*\\)$");
    private static final Pattern ARRAY = Pattern.compile("^(.*?)\\s*\\[(\\d+)\\]$");
    private static final Pattern SIGNATURE = Pattern.compile("^(.+?)\\s*\\b(__\\w+)\\s+\\((.*)\\)$");
    private static final Pattern PARAM = Pattern.compile("^(.*?)([A-Za-z_]\\w*)$");

    private DataTypeManager dtm;
    private SymbolTable symtab;
    private final Map<String, Structure> structs = new TreeMap<>();
    private int failures = 0;
    private boolean keep = false;
    private final Set<Long> kept = new HashSet<>();

    private void fail(String what) {
        failures++;
        println("ApplySymbols: FAIL " + what);
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

    private DataType baseType(String name) {
        switch (name) {
            case "void": return VoidDataType.dataType;
            case "char": return CharDataType.dataType;
            case "int": case "int32_t": return IntegerDataType.dataType;
            case "unsigned int": case "uint32_t": return UnsignedIntegerDataType.dataType;
            case "int8_t": return SignedByteDataType.dataType;
            case "uint8_t": return ByteDataType.dataType;
            case "int16_t": case "short": return ShortDataType.dataType;
            case "uint16_t": case "unsigned short": return UnsignedShortDataType.dataType;
            case "float": return FloatDataType.dataType;
            case "double": return DoubleDataType.dataType;
            default: return structs.get(name);
        }
    }

    private DataType pointerTo(DataType dt) {
        return new PointerDataType(dt, dtm);
    }

    private DataType type(String t) {
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
            DataType el = type(ar.group(1));
            return el == null ? null : new ArrayDataType(el, Integer.parseInt(ar.group(2)), el.getLength());
        }
        if (t.endsWith("*")) {
            DataType to = type(t.substring(0, t.length() - 1));
            return to == null ? null : pointerTo(to);
        }
        return baseType(t.trim());
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
        return (doc.equals("METHOD") ? "METHOD.md" : "systems/" + doc + ".md") + rest;
    }

    // An item that keeps its name: ours goes in a repeatable comment.
    private void commentKept(Address a, String name, String text) {
        currentProgram.getListing().setComment(a, CommentType.REPEATABLE,
                "benilla-re: " + (name.isEmpty() ? "(no name)" : name) + "\n" + text);
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
        int fields = 0;
        for (String[] c : frows) {
            Structure s = structs.get(c[0]);
            if (s == null) { fail("field " + c[0] + "::" + c[4] + ": no struct"); continue; }
            DataType dt = type(c[3]);
            if (dt == null) { fail("field " + c[0] + "::" + c[4] + ": unknown type " + c[3]); continue; }
            int off = Integer.decode(c[1]);
            int size = Integer.parseInt(c[2]);
            if (dt.getLength() != size) { fail("field " + c[0] + "::" + c[4] + ": " + c[3] + " is " + dt.getLength() + " bytes, row says " + size); continue; }
            s.replaceAtOffset(off, dt, size, c[4], c[6]);
            fields++;
        }
        println("ApplySymbols: structs=" + structs.size() + " fields=" + fields);
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

    // Returns null when applied, else the reason.
    private String applySignature(Function f, String sig, boolean hasClass) throws Exception {
        Matcher m = SIGNATURE.matcher(sig);
        if (!m.matches()) return "cannot parse signature " + sig;
        DataType ret = type(m.group(1));
        if (ret == null) return "unknown return type " + m.group(1);
        String conv = m.group(2);
        List<Variable> params = new ArrayList<>();
        List<String> ps = splitParams(m.group(3));
        if (ps.size() == 1 && ps.get(0).equals("void")) ps.clear();
        if (conv.equals("__thiscall") && !ps.isEmpty()) {
            ps.remove(0); // `this` is the convention's own parameter
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
            DataType dt = type(ty);
            if (dt == null) return "unknown parameter type " + ty;
            params.add(new ParameterImpl(name, dt, currentProgram));
            n++;
        }
        f.setCallingConvention(conv);
        f.updateFunction(conv, new ReturnParameterImpl(ret, currentProgram),
                params, Function.FunctionUpdateType.DYNAMIC_STORAGE_FORMAL_PARAMS, true, SourceType.USER_DEFINED);
        return null;
    }

    private int applyFunctions(String dir) throws Exception {
        int named = 0, sigs = 0;
        for (String[] c : read(dir, "functions.tsv")) {
            Address a = toAddr(Long.parseLong(c[0].substring(2), 16));
            Function f = getFunctionAt(a);
            if (f == null) { fail("function " + c[0] + " " + c[2] + ": no function"); continue; }
            try {
                if (keep && f.getSymbol().getSource() != SourceType.DEFAULT) {
                    kept.add(a.getOffset());
                    commentKept(a, c[2], comment(c, 7, 8, 9, 10));
                    continue;
                }
                if (!c[2].isEmpty()) {
                    List<String> q = qualifier(c[2]);
                    if (!q.isEmpty()) f.setParentNamespace(namespace(q));
                    f.setName(leaf(c[2]), SourceType.USER_DEFINED);
                    named++;
                }
                if (!c[6].isEmpty()) {
                    String why = applySignature(f, c[6], !qualifier(c[2]).isEmpty());
                    if (why != null) fail("signature " + c[0] + " " + c[2] + ": " + why); else sigs++;
                }
                setPlateComment(a, comment(c, 7, 8, 9, 10));
            } catch (Exception e) {
                fail("function " + c[0] + " " + c[2] + ": " + e);
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
                if (!currentProgram.getMemory().contains(a)) { fail("global " + c[0] + " " + c[2] + ": not in memory"); continue; }
                Symbol existing = symtab.getPrimarySymbol(a);
                if (keep && existing != null && existing.getSource() != SourceType.DEFAULT) {
                    kept.add(a.getOffset());
                    commentKept(a, c[2], comment(c, 5, 6, 7, 8));
                    continue;
                }
                DataType dt = type(c[1]);
                if (dt == null) { fail("global " + c[0] + " " + c[2] + ": unknown type " + c[1]); continue; }
                if (getInstructionContaining(a) != null) { fail("global " + c[0] + " " + c[2] + ": code at the address"); continue; }
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
            } catch (Exception e) {
                fail("global " + c[0] + " " + c[2] + ": " + e);
            }
        }
        println("ApplySymbols: globals named=" + named + " typed=" + typed);
    }

    // ---- verification ----

    private void verify(String dir) throws Exception {
        int fOk = 0, fRows = 0, fNamed = 0, gOk = 0, gRows = 0, gNamed = 0;
        for (String[] c : read(dir, "functions.tsv")) {
            fRows++;
            if (c[2].isEmpty() || kept.contains(Long.parseLong(c[0].substring(2), 16))) continue;
            fNamed++;
            Function f = getFunctionAt(toAddr(Long.parseLong(c[0].substring(2), 16)));
            if (f != null && f.getName(true).equals(c[2])) fOk++;
            else println("ApplySymbols: MISMATCH function " + c[0] + ": row " + c[2] + ", program " + (f == null ? "none" : f.getName(true)));
        }
        int sOk = 0, sRows = 0, shown = 0;
        for (String[] c : read(dir, "functions.tsv")) {
            if (c[6].isEmpty() || kept.contains(Long.parseLong(c[0].substring(2), 16))) continue;
            sRows++;
            Function f = getFunctionAt(toAddr(Long.parseLong(c[0].substring(2), 16)));
            Matcher m = SIGNATURE.matcher(c[6]);
            List<String> ps = splitParams(m.matches() ? m.group(3) : "");
            int expected = (ps.size() == 1 && ps.get(0).equals("void")) ? 0 : ps.size();
            if (f != null && f.getParameterCount() == expected
                    && f.getCallingConventionName().equals(m.group(2))) sOk++;
            else println("ApplySymbols: MISMATCH signature " + c[0] + ": row " + c[6] + ", program "
                    + (f == null ? "none" : f.getPrototypeString(true, false)));
            if (c[2].contains("::") && shown++ < 3) println("ApplySymbols: sample " + f.getPrototypeString(true, false));
        }
        println("ApplySymbols: verified signatures " + sOk + "/" + sRows);
        for (String[] c : read(dir, "globals.tsv")) {
            gRows++;
            if (c[2].isEmpty() || kept.contains(Long.parseLong(c[0].substring(2), 16))) continue;
            gNamed++;
            Symbol s = symtab.getPrimarySymbol(toAddr(Long.parseLong(c[0].substring(2), 16)));
            if (s != null && s.getName(true).equals(c[2])) gOk++;
            else println("ApplySymbols: MISMATCH global " + c[0] + ": row " + c[2] + ", program " + (s == null ? "none" : s.getName(true)));
        }
        println("ApplySymbols: kept " + kept.size() + " items with names of their own");
        println("ApplySymbols: verified functions " + fOk + "/" + fNamed + " named rows (" + fRows
                + " rows), globals " + gOk + "/" + gNamed + " named rows (" + gRows + " rows), failures " + failures);
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
