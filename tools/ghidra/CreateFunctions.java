// Post-script (after analysis): creates every function of symbols/functions.tsv that analysis did
// not make, at its address, and makes each one's body the row's (symbols/README.md): it removes
// any function analysis started inside a row's extent, which docs/METHOD.md (Completeness) rules out,
// and follows the jump tables analysis left unresolved, as `cargo xtask check --exe` does. Prints one line
// per row that could not be made a function and one per repair, and checks each function's size
// against the row's. Argument: the symbols directory.
//
// @category benilla

import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.RefType;
import ghidra.program.model.symbol.SourceType;

import java.io.BufferedReader;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

public class CreateFunctions extends GhidraScript {

    private int repairs = 0;

    // The table of a `jmp [reg*4 + table]`: its displacement.
    private Long tableOf(Instruction i) {
        for (Object o : i.getOpObjects(0)) {
            if (o instanceof Scalar && ((Scalar) o).getUnsignedValue() >= 0x401000L) {
                return ((Scalar) o).getUnsignedValue();
            }
        }
        return null;
    }

    private void repair(Function f, long entry, long size, Set<Long> rows) throws Exception {
        Address first = toAddr(entry), last = toAddr(entry + size - 1);
        boolean changed = false;
        List<Function> inside = new ArrayList<>();
        if (size > 1) {
            for (Function g : currentProgram.getFunctionManager().getFunctions(new AddressSet(first.add(1), last), true)) {
                if (!rows.contains(g.getEntryPoint().getOffset())) inside.add(g);
            }
        }
        for (Function g : inside) {
            println("CreateFunctions: removed " + g.getEntryPoint() + ", a start analysis made inside "
                    + f.getName() + " at " + first);
            removeFunction(g);
            changed = true;
        }
        List<Instruction> jumps = new ArrayList<>();
        for (Instruction i : currentProgram.getListing().getInstructions(new AddressSet(first, last), true)) {
            if (i.getFlowType().isJump() && i.getFlowType().isComputed() && i.getFlows().length == 0) jumps.add(i);
        }
        for (Instruction i : jumps) {
            Long table = tableOf(i);
            if (table == null) continue;
            int k = 0;
            for (; ; k++) {
                long v = getInt(toAddr(table + 4L * k)) & 0xffffffffL;
                if (v < entry || v >= table) break;
                Address target = toAddr(v);
                if (getInstructionAt(target) == null) disassemble(target);
                i.addOperandReference(0, target, RefType.COMPUTED_JUMP, SourceType.USER_DEFINED);
            }
            if (k == 0) continue;
            println("CreateFunctions: followed the " + k + "-entry jump table at " + toAddr(table)
                    + " from " + i.getAddress() + " in " + f.getName());
            changed = true;
        }
        if (changed) {
            CreateFunctionCmd.fixupFunctionBody(currentProgram, f, monitor);
            repairs++;
        }
    }

    @Override
    public void run() throws Exception {
        String dir = getScriptArgs()[0];
        List<String[]> rows = new ArrayList<>();
        try (BufferedReader r = Files.newBufferedReader(Paths.get(dir, "functions.tsv"))) {
            r.readLine();
            String line;
            while ((line = r.readLine()) != null) {
                rows.add(line.split("\t", -1));
            }
        }
        Set<Long> starts = new HashSet<>();
        for (String[] c : rows) starts.add(Long.parseLong(c[0].substring(2), 16));
        println("CreateFunctions: functions after analysis: "
                + currentProgram.getFunctionManager().getFunctionCount());
        int total = rows.size(), found = 0, created = 0, failed = 0, sizeDiffers = 0, split = 0;
        // First every row's start becomes a function, then the bodies are repaired and checked, so
        // that a function never meets a start that is not yet one.
        List<String[]> made = new ArrayList<>();
        for (String[] c : rows) {
            Address a = toAddr(Long.parseLong(c[0].substring(2), 16));
            Function f = getFunctionAt(a);
            if (f != null) {
                found++;
                made.add(c);
                continue;
            }
            // Analysis can make one function over two rows, when the code of the first jumps to the
            // second: the first function is made again after the second, and stops at it.
            Function inside = getFunctionContaining(a);
            Address earlier = null;
            if (inside != null) {
                earlier = inside.getEntryPoint();
                removeFunction(inside);
            }
            if (getInstructionAt(a) == null) {
                // Data or nothing in the way: clear it, then disassemble.
                if (getDataAt(a) != null) {
                    clearListing(a);
                }
                disassemble(a);
            }
            f = createFunction(a, null);
            if (f == null) {
                println("CreateFunctions: FAIL " + c[0] + " " + c[2] + ": createFunction returned null");
                failed++;
            } else {
                created++;
                made.add(c);
            }
            if (earlier != null) {
                if (createFunction(earlier, null) == null) {
                    println("CreateFunctions: FAIL " + earlier + ": not made again after the split at " + a);
                    failed++;
                } else {
                    println("CreateFunctions: split " + earlier + " at " + a);
                    split++;
                }
            }
        }
        for (String[] c : made) {
            Address a = toAddr(Long.parseLong(c[0].substring(2), 16));
            long size = Long.parseLong(c[1]);
            Function f = getFunctionAt(a);
            repair(f, a.getOffset(), size, starts);
            // The size of the row runs to the end of the last instruction.
            long end = f.getBody().getMaxAddress().getOffset() - f.getEntryPoint().getOffset() + 1;
            if (f.getBody().getNumAddressRanges() > 1 || end != size) {
                println("CreateFunctions: size " + c[0] + " " + c[2] + ": row " + size + ", ghidra "
                        + f.getBody().getNumAddresses() + " in " + f.getBody().getNumAddressRanges() + " range(s)");
                sizeDiffers++;
            }
        }
        println("CreateFunctions: rows=" + total + " already_functions=" + found + " created=" + created
                + " split=" + split + " failed=" + failed + " repaired=" + repairs + " size_differs=" + sizeDiffers);
        println("CreateFunctions: functions in the program now: "
                + currentProgram.getFunctionManager().getFunctionCount());
    }
}
