[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SourceDirectory,

    [string]$DestinationDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
[char[]]$PathSeparators = @([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
if ([string]::IsNullOrWhiteSpace($DestinationDirectory)) {
    $DestinationDirectory = Join-Path $ProjectRoot "backend\libarchive"
}

function Get-FullPath([string]$Path) {
    return [System.IO.Path]::GetFullPath($Path)
}

function Test-IsInside([string]$Child, [string]$Parent) {
    $parentWithSeparator = $Parent.TrimEnd($PathSeparators) + [System.IO.Path]::DirectorySeparatorChar
    return $Child.StartsWith($parentWithSeparator, [System.StringComparison]::OrdinalIgnoreCase)
}

function Assert-SafeRelativeName([string]$RelativePath) {
    if ([string]::IsNullOrWhiteSpace($RelativePath)) {
        throw "Empty backend path is not allowed."
    }
    if ($RelativePath.Contains("`t") -or $RelativePath.Contains("`r") -or $RelativePath.Contains("`n")) {
        throw "Backend paths must not contain tabs or newlines: $RelativePath"
    }
}

$sourceItem = Get-Item -LiteralPath $SourceDirectory -Force -ErrorAction Stop
if (-not $sourceItem.PSIsContainer) {
    throw "SourceDirectory is not a directory: $SourceDirectory"
}
if (($sourceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "SourceDirectory itself must not be a reparse point: $SourceDirectory"
}

$SourceDirectory = (Resolve-Path -LiteralPath $SourceDirectory).Path
$DestinationDirectory = Get-FullPath $DestinationDirectory
$resolvedSourceItem = Get-Item -LiteralPath $SourceDirectory -Force -ErrorAction Stop
if (($resolvedSourceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Resolved SourceDirectory must not be a reparse point: $SourceDirectory"
}
if ($SourceDirectory.Equals($DestinationDirectory, [System.StringComparison]::OrdinalIgnoreCase) -or
    (Test-IsInside $DestinationDirectory $SourceDirectory) -or
    (Test-IsInside $SourceDirectory $DestinationDirectory)) {
    throw "SourceDirectory and DestinationDirectory must be separate trees."
}

$allItems = @(Get-ChildItem -LiteralPath $SourceDirectory -Recurse -Force)
foreach ($item in $allItems) {
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Reparse points are not allowed in a backend bundle: $($item.FullName)"
    }
}

$sourceFiles = @(
    $allItems |
        Where-Object { -not $_.PSIsContainer -and $_.Name -ne "backend-manifest.tsv" }
)
if ($sourceFiles.Count -eq 0) {
    throw "The backend bundle contains no files."
}

$executables = @($sourceFiles | Where-Object { $_.Name -ieq "bsdtar.exe" })
if ($executables.Count -ne 1) {
    throw "The backend bundle must contain exactly one bsdtar.exe; found $($executables.Count)."
}

$destinationParent = Split-Path -Parent $DestinationDirectory
New-Item -ItemType Directory -Force -Path $destinationParent | Out-Null
$stage = Join-Path $destinationParent (".safearc-backend-stage-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $stage | Out-Null

try {
    foreach ($file in $sourceFiles) {
        $relative = $file.FullName.Substring($SourceDirectory.Length).TrimStart($PathSeparators)
        Assert-SafeRelativeName $relative
        $target = Join-Path $stage $relative
        $targetParent = Split-Path -Parent $target
        New-Item -ItemType Directory -Force -Path $targetParent | Out-Null

        # Copy only the unnamed data stream. This intentionally does not carry Zone.Identifier
        # or any other NTFS alternate stream into the trusted backend directory.
        $input = [System.IO.File]::Open(
            $file.FullName,
            [System.IO.FileMode]::Open,
            [System.IO.FileAccess]::Read,
            [System.IO.FileShare]::Read
        )
        try {
            $output = [System.IO.File]::Open(
                $target,
                [System.IO.FileMode]::CreateNew,
                [System.IO.FileAccess]::Write,
                [System.IO.FileShare]::None
            )
            try {
                $input.CopyTo($output)
                $output.Flush($true)
            }
            finally {
                $output.Dispose()
            }
        }
        finally {
            $input.Dispose()
        }
    }

    $stageFiles = @(
        Get-ChildItem -LiteralPath $stage -Recurse -Force -File |
            Sort-Object { $_.FullName.Substring($stage.Length) }
    )
    $stageExecutable = @($stageFiles | Where-Object { $_.Name -ieq "bsdtar.exe" })
    if ($stageExecutable.Count -ne 1) {
        throw "Internal error: staged backend has no unique bsdtar.exe."
    }

    $executableRelative = $stageExecutable[0].FullName.Substring($stage.Length).TrimStart($PathSeparators)
    $executableRelative = $executableRelative.Replace('\', '/')
    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.Add("SAFEARC-BACKEND-MANIFEST`t1")
    $lines.Add("executable`t$executableRelative")

    foreach ($file in $stageFiles) {
        $relative = $file.FullName.Substring($stage.Length).TrimStart($PathSeparators).Replace('\', '/')
        Assert-SafeRelativeName $relative
        $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        $lines.Add("sha256`t$hash`t$relative")
    }

    $manifestPath = Join-Path $stage "backend-manifest.tsv"
    $content = [string]::Join("`n", $lines) + "`n"
    [System.IO.File]::WriteAllText(
        $manifestPath,
        $content,
        [System.Text.UTF8Encoding]::new($false)
    )

    $backup = $null
    if (Test-Path -LiteralPath $DestinationDirectory) {
        $backup = Join-Path $destinationParent (".safearc-backend-backup-" + [Guid]::NewGuid().ToString("N"))
        Move-Item -LiteralPath $DestinationDirectory -Destination $backup
    }

    try {
        Move-Item -LiteralPath $stage -Destination $DestinationDirectory
        if ($null -ne $backup) {
            Remove-Item -LiteralPath $backup -Recurse -Force
        }
    }
    catch {
        if ($null -ne $backup -and -not (Test-Path -LiteralPath $DestinationDirectory)) {
            Move-Item -LiteralPath $backup -Destination $DestinationDirectory
        }
        throw
    }

    Write-Host "Installed a pinned backend bundle: $DestinationDirectory"
    Write-Host "Files: $($stageFiles.Count)"
    Write-Host "Run: .\target\x86_64-pc-windows-msvc\release\safearc.exe doctor"
}
finally {
    if (Test-Path -LiteralPath $stage) {
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
}
