// Post-script, run on the project tools/ghidra/setup.sh builds: decompiles the functions at the
// given addresses, in order, into one C file, with this repo's names, signatures and types applied.
// An address inside a function decompiles the function that holds it. Decompiler output is a lead
// for reading the bytes, never a derivation (docs/METHOD.md, The binary decides). Arguments: the output
// file, then one or more addresses (0x hex).
//
// @category benilla

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.HashSet;
import java.util.Set;

public class Decompile extends GhidraScript {

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 2) {
            printerr("Decompile: usage: Decompile.java <output file> <address> [<address> ...]");
            return;
        }
        FunctionManager functions = currentProgram.getFunctionManager();
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        File out = new File(args[0]);
        if (out.getParentFile() != null) {
            out.getParentFile().mkdirs();
        }
        Set<Long> done = new HashSet<>();
        int failed = 0;
        try (PrintWriter w = new PrintWriter(new FileWriter(out))) {
            for (int i = 1; i < args.length; i++) {
                for (String text : args[i].split(",")) {
                    long address = Long.decode(text.trim());
                    Function f = functions.getFunctionContaining(toAddr(address));
                    if (f == null) {
                        w.println("// " + text.trim() + ": no function holds this address");
                        failed++;
                        continue;
                    }
                    long entry = f.getEntryPoint().getOffset();
                    if (!done.add(entry)) {
                        continue;
                    }
                    w.printf("// %s at 0x%06x, %d bytes%n", f.getName(), entry,
                        f.getBody().getNumAddresses());
                    DecompileResults r = decompiler.decompileFunction(f, 120, monitor);
                    if (r != null && r.decompileCompleted() && r.getDecompiledFunction() != null) {
                        w.println(r.getDecompiledFunction().getC());
                    } else {
                        w.println("// the decompiler failed: "
                            + (r == null ? "no result" : r.getErrorMessage()));
                        failed++;
                    }
                }
            }
        }
        decompiler.dispose();
        println("Decompile: " + done.size() + " functions, " + failed + " failed, into " + args[0]);
    }
}
