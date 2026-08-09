[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SourceDirectory,

    [string]$DestinationDirectory,

    [string]$EvidenceMetadataPath,

    [string]$LicenseDirectory,

    [switch]$AllowUnsupportedSource
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "backend-evidence.ps1")
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

$sourceEvidenceRoot = Join-Path $SourceDirectory ".iroha-zip-evidence"
if (Test-Path -LiteralPath $sourceEvidenceRoot) {
    $sourceEvidenceItem = Get-Item -LiteralPath $sourceEvidenceRoot -Force
    if (-not $sourceEvidenceItem.PSIsContainer) {
        throw "The reserved .iroha-zip-evidence source entry must be a directory."
    }
}
$sourceEvidencePrefix = $sourceEvidenceRoot.TrimEnd($PathSeparators) + [System.IO.Path]::DirectorySeparatorChar
$sourceFiles = @(
    $allItems |
        Where-Object {
            -not $_.PSIsContainer -and
            $_.Name -ne "backend-manifest.tsv" -and
            -not $_.FullName.Equals($sourceEvidenceRoot, [System.StringComparison]::OrdinalIgnoreCase) -and
            -not $_.FullName.StartsWith($sourceEvidencePrefix, [System.StringComparison]::OrdinalIgnoreCase)
        }
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
$stage = Join-Path $destinationParent (".iroha-zip-backend-stage-" + [Guid]::NewGuid().ToString("N"))
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
    $lines.Add("IROHA-ZIP-BACKEND-MANIFEST`t1")
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

    if ([string]::IsNullOrWhiteSpace($EvidenceMetadataPath)) {
        if (-not $AllowUnsupportedSource) {
            Write-Warning "The selected local bundle is an unsupported source. Its origin and distributor signature cannot be verified."
            throw "Re-run with -AllowUnsupportedSource only after reviewing and accepting this provenance warning."
        }
        Write-Warning "UNSUPPORTED BACKEND SOURCE: origin and distributor signature were not verified."
        $localPackageId = "unverified-local-bundle"
        $fileMappings = @(
            foreach ($file in $stageFiles) {
                $relative = $file.FullName.Substring($stage.Length).TrimStart($PathSeparators).Replace('\', '/')
                [ordered]@{
                    path = $relative
                    packageId = $localPackageId
                }
            }
        )
        $sourceMetadata = [pscustomobject][ordered]@{
            source = [pscustomobject][ordered]@{
                kind = "unverified-local-bundle"
                supported = $false
                repository = "unverified-local"
                verification = [pscustomobject][ordered]@{
                    status = "unverified"
                    method = "explicit-user-accepted-local-bundle"
                    keyringPackage = $null
                    keyringVersion = $null
                }
            }
            packages = @([pscustomobject][ordered]@{
                id = $localPackageId
                name = $localPackageId
                version = "NOASSERTION"
                architecture = "windows"
                repository = "unverified-local"
                downloadUrl = $null
                archiveSha256 = $null
                signature = $null
                licenses = @("NOASSERTION")
            })
            files = @($fileMappings)
        }
    }
    else {
        $metadataItem = Get-Item -LiteralPath $EvidenceMetadataPath -Force -ErrorAction Stop
        if ($metadataItem.PSIsContainer -or
            ($metadataItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0 -or
            $metadataItem.Length -gt 4MB) {
            throw "Evidence metadata must be a regular JSON file no larger than 4 MiB."
        }
        $metadataText = [System.IO.File]::ReadAllText($metadataItem.FullName)
        $sourceMetadata = $metadataText | ConvertFrom-Json
    }

    New-IrohaZipBackendEvidence `
        -BackendDirectory $stage `
        -SourceMetadata $sourceMetadata `
        -LicenseSourceDirectory $LicenseDirectory

    $validatorCandidates = @(
        (Join-Path $ProjectRoot "iroha-zip.exe"),
        (Join-Path $ProjectRoot "target\x86_64-pc-windows-msvc\release\iroha-zip.exe"),
        (Join-Path $ProjectRoot "target\release\iroha-zip.exe")
    )
    $validator = $validatorCandidates |
        Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
        Select-Object -First 1
    if ($null -ne $validator) {
        & $validator verify-backend-evidence $stage
        if ($LASTEXITCODE -ne 0) {
            throw "Generated backend evidence failed independent validation; the existing backend was preserved."
        }
    }
    else {
        Write-Warning "iroha-zip.exe was not available for independent evidence validation. The settings screen and private release build validate it before use."
    }

    $backup = $null
    if (Test-Path -LiteralPath $DestinationDirectory) {
        $backup = Join-Path $destinationParent (".iroha-zip-backend-backup-" + [Guid]::NewGuid().ToString("N"))
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
    if ($sourceMetadata.source.supported) {
        Write-Host "Provenance: verified supported source ($($sourceMetadata.source.kind))"
    }
    else {
        Write-Warning "Provenance: unsupported source; see .iroha-zip-evidence for the explicit warning and inventory."
    }
    Write-Host "Run: .\target\x86_64-pc-windows-msvc\release\iroha-zip.exe doctor"
}
finally {
    if (Test-Path -LiteralPath $stage) {
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
}
