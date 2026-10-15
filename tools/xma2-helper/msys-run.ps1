# Run one command inside the isolated MSYS2 toolchain with a process-local, minimal environment.
# Nothing here changes the machine or user PATH. Usage:
#   powershell -NoProfile -File tools\xma2-helper\msys-run.ps1 -Command "pacman -Q"
param(
    [Parameter(Mandatory = $true)][string]$Command,
    [string]$Root = $(if ($env:NHLMS_MSYS2_ROOT) { $env:NHLMS_MSYS2_ROOT } else { 'C:\nhlms-toolchain\msys64' }),
    [string]$Msystem = 'UCRT64'
)
$ErrorActionPreference = 'Stop'
$bash = Join-Path $Root 'usr\bin\bash.exe'
if (-not (Test-Path -LiteralPath $bash)) { throw "MSYS2 bash not found at $bash" }

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $bash
$psi.Arguments = '-lc "' + ($Command -replace '"', '\"') + '"'
$psi.UseShellExecute = $false
$psi.EnvironmentVariables.Clear()
foreach ($k in 'SystemRoot', 'SystemDrive', 'windir', 'TEMP', 'TMP', 'USERNAME', 'USERPROFILE', 'COMPUTERNAME', 'NUMBER_OF_PROCESSORS', 'PROCESSOR_ARCHITECTURE') {
    $v = [Environment]::GetEnvironmentVariable($k)
    if ($v) { $psi.EnvironmentVariables[$k] = $v }
}
$psi.EnvironmentVariables['PATH'] = "$env:SystemRoot\System32;$env:SystemRoot"
$psi.EnvironmentVariables['MSYSTEM'] = $Msystem
$psi.EnvironmentVariables['CHERE_INVOKING'] = '1'
$psi.EnvironmentVariables['MSYS2_PATH_TYPE'] = 'minimal'
$psi.EnvironmentVariables['HOME'] = (Join-Path $Root 'home\nhlms')
$psi.EnvironmentVariables['LC_ALL'] = 'C'
$p = [System.Diagnostics.Process]::Start($psi)
$p.WaitForExit()
exit $p.ExitCode
