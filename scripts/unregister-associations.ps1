[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$progId = "SafeArc.Archive"
$extensions = @(
    ".zip", ".zipx", ".7z", ".rar", ".lzh", ".lha", ".tar", ".gz", ".tgz",
    ".bz2", ".tbz", ".tbz2", ".xz", ".txz", ".zst", ".tzst", ".z", ".cab"
)

foreach ($extension in $extensions) {
    $openWith = "HKCU:\Software\Classes\$extension\OpenWithProgids"
    if (Test-Path -LiteralPath $openWith) {
        Remove-ItemProperty -LiteralPath $openWith -Name $progId -ErrorAction SilentlyContinue
    }
}

Remove-Item -LiteralPath "HKCU:\Software\Classes\$progId" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath "HKCU:\Software\Classes\Applications\safearc-shell.exe" `
    -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath "HKCU:\Software\SafeArc" -Recurse -Force -ErrorAction SilentlyContinue
Remove-ItemProperty -LiteralPath "HKCU:\Software\RegisteredApplications" `
    -Name "SafeArc" -ErrorAction SilentlyContinue

Write-Host "SafeArc registration was removed for the current user."
Write-Host "A Windows UserChoice entry, if already selected, may remain until changed in Default Apps."
