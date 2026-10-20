// Targeted offline Ghidra analysis. Arguments: address-list file, output directory.
// @category NHL24.Research
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import java.nio.file.*;

public class TargetDecompile extends GhidraScript {
    public void run() throws Exception {
        String[] args = getScriptArgs();
        Path out = Paths.get(args[1]);
        Files.createDirectories(out);
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        try {
            for (String line : Files.readAllLines(Paths.get(args[0]))) {
                line = line.trim();
                if (line.isEmpty() || line.startsWith("#")) continue;
                Address address = toAddr(Long.parseUnsignedLong(line.replace("0x", ""), 16));
                disassemble(address);
                Function f = getFunctionAt(address);
                if (f == null) f = createFunction(address, "nhl_" + address.toString());
                if (f == null) { println("Cannot create " + address); continue; }
                DecompileResults result = decompiler.decompileFunction(f, 120, monitor);
                String code = result.decompileCompleted() ? result.getDecompiledFunction().getC() : result.getErrorMessage();
                Files.writeString(out.resolve(address.toString() + ".c"), code);
                println("NHL_DECOMPILED " + address + " " + result.decompileCompleted());
            }
        } finally { decompiler.dispose(); }
    }
}
