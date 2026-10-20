param(
    [string]$Targets = 'local/nhl27-eboot/targets.txt',
    [string]$ResearchRoot = 'local/nhl27-eboot',
    [string]$Java = 'C:\Program Files\Java\jdk-21.0.11\bin\java.exe',
    [string]$Ghidra = 'local/nhl24-eboot/tools/ghidra_12.1.4_PUBLIC'
)
$ErrorActionPreference = 'Stop'
$researchPath = (Resolve-Path -LiteralPath $ResearchRoot).Path
$ghidraPath = (Resolve-Path -LiteralPath $Ghidra).Path
$targetPath = (Resolve-Path -LiteralPath $Targets).Path
$scriptPath = (Resolve-Path -LiteralPath 'scripts/nhl24').Path
# Call the bundled launcher class directly: avoid modifying user Java preferences.
& $Java '-Xmx8G' '-Djava.system.class.loader=ghidra.GhidraClassLoader' `
    "-Dapplication.settingsdir=$researchPath\ghidra-settings" `
    "-Dapplication.cachedir=$researchPath\ghidra-cache" `
    -cp "$ghidraPath\Ghidra\Framework\Utility\lib\Utility.jar" `
    ghidra.Ghidra ghidra.app.util.headless.AnalyzeHeadless `
    "$researchPath\ghidra-project" NHL27 -process analysis.elf -noanalysis `
    -scriptPath $scriptPath -postScript TargetDecompile.java $targetPath `
    "$researchPath\decompiled" -log "$researchPath\ghidra.log"
if ($LASTEXITCODE -ne 0) { throw "Ghidra failed: $LASTEXITCODE" }
