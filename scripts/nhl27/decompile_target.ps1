param([Parameter(Mandatory=$true)][string]$Targets)
$ErrorActionPreference = 'Stop'
$researchPath = (Resolve-Path -LiteralPath 'local/nhl24-eboot').Path
$ghidraPath = (Resolve-Path -LiteralPath 'local/nhl24-eboot/tools/ghidra_12.1.4_PUBLIC').Path
$targetPath = (Resolve-Path -LiteralPath $Targets).Path
$scriptPath = (Resolve-Path -LiteralPath 'scripts/nhl24').Path
# Reuse the already imported NHL24 1.60 image; never analyze or rewrite the install.
& 'C:\Program Files\Java\jdk-21.0.11\bin\java.exe' '-Xmx8G' '-Djava.system.class.loader=ghidra.GhidraClassLoader' `
    "-Dapplication.settingsdir=$researchPath\ghidra-settings" "-Dapplication.cachedir=$researchPath\ghidra-cache" `
    -cp "$ghidraPath\Ghidra\Framework\Utility\lib\Utility.jar" ghidra.Ghidra ghidra.app.util.headless.AnalyzeHeadless `
    "$researchPath\ghidra-project" NHL24 -process image.elf -noanalysis -scriptPath $scriptPath `
    -postScript TargetDecompile.java $targetPath 'local/nhl27-eboot/animation-target-decompiled' `
    -log 'local/nhl27-eboot/target-vbr-ghidra.log'
if ($LASTEXITCODE -ne 0) { throw "Ghidra failed: $LASTEXITCODE" }
