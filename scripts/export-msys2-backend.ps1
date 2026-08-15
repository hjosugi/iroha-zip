[CmdletBinding()]
param(
    [string]$Msys2Root = "C:\msys64",
    [string]$DestinationDirectory,

    [ValidateSet("UCRT64", "CLANGARM64")]
    [string]$Environment = "UCRT64",

    [ValidateRange(30, 1800)]
    [int]$CommandTimeoutSeconds = 180
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "msys2-command.ps1")

$ProjectRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($DestinationDirectory)) {
    $DestinationDirectory = Join-Path $ProjectRoot "backend\libarchive"
}

$Msys2Root = [System.IO.Path]::GetFullPath($Msys2Root)
$environmentContract = switch ($Environment) {
    "UCRT64" {
        [pscustomobject][ordered]@{
            displayName = "UCRT64"
            directory = "ucrt64"
            packagePrefix = "mingw-w64-ucrt-x86_64-"
            sourceKind = "msys2-ucrt64-pacman"
            assetArchitecture = "x64"
        }
    }
    "CLANGARM64" {
        [pscustomobject][ordered]@{
            displayName = "CLANGARM64"
            directory = "clangarm64"
            packagePrefix = "mingw-w64-clang-aarch64-"
            sourceKind = "msys2-clangarm64-pacman"
            assetArchitecture = "arm64"
        }
    }
}
$environmentDirectory = [string]$environmentContract.directory
$environmentUnix = "/$environmentDirectory"
$environmentBinUnix = "$environmentUnix/bin/"
$packagePrefix = [string]$environmentContract.packagePrefix
$packagePattern = '^' + [regex]::Escape($packagePrefix) + '[A-Za-z0-9@._+-]+$'
$licenseEntryPrefix = "$environmentDirectory/share/licenses/"
$licenseEntryPattern = '^' + [regex]::Escape($licenseEntryPrefix) + `
    '[A-Za-z0-9@._+-]+/.+'
$bash = Join-Path $Msys2Root "usr\bin\bash.exe"
$bsdtar = Join-Path $Msys2Root "$environmentDirectory\bin\bsdtar.exe"
$pacman = Join-Path $Msys2Root "usr\bin\pacman.exe"
if (-not (Test-Path -LiteralPath $bash -PathType Leaf)) {
    throw "MSYS2 bash.exe was not found: $bash"
}
if (-not (Test-Path -LiteralPath $bsdtar -PathType Leaf)) {
    $libarchivePackage = $packagePrefix + "libarchive"
    throw @"
MSYS2 $([string]$environmentContract.displayName) bsdtar.exe was not found: $bsdtar
Install it from an MSYS2 $([string]$environmentContract.displayName) shell:
  pacman -S $libarchivePackage
"@
}
if (-not (Test-Path -LiteralPath $pacman -PathType Leaf)) {
    throw "MSYS2 pacman.exe was not found: $pacman"
}

function Invoke-Msys2([string]$Script, [string[]]$Arguments) {
    return Invoke-IrohaZipMsys2Command `
        -BashPath $bash `
        -Script $Script `
        -Arguments $Arguments `
        -TimeoutSeconds $CommandTimeoutSeconds
}

function Invoke-Msys2Scalar([string]$Script, [string[]]$Arguments) {
    $lines = @(Invoke-Msys2 $Script $Arguments | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($lines.Count -ne 1) {
        $argumentSummary = [string]::Join(", ", @($Arguments | ForEach-Object { "[$_]" }))
        $outputSummary = [string]::Join(" | ", @($lines | ForEach-Object { "[$_]" }))
        if ($outputSummary.Length -gt 1024) {
            $outputSummary = $outputSummary.Substring(0, 1024) + "..."
        }
        throw "Expected one MSYS2 result line; found $($lines.Count). Arguments: $argumentSummary. Output: $outputSummary"
    }
    return [string]$lines[0]
}

function Convert-ToMsysPath([string]$WindowsPath) {
    return Invoke-Msys2Scalar '/usr/bin/cygpath -u -- "$1"' @($WindowsPath)
}

function Export-ArchiveEntry([string]$Archive, [string]$Entry, [string]$Destination) {
    if (Test-Path -LiteralPath $Destination) {
        throw "Archive extraction destination already exists: $Destination"
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null
    $archiveUnix = Convert-ToMsysPath $Archive
    $destinationUnix = Convert-ToMsysPath $Destination
    Invoke-Msys2 `
        'environment="$1"; PATH="$environment/bin:/usr/bin" "$environment/bin/bsdtar.exe" -xOf "$2" -- "$3" > "$4"' `
        @($environmentUnix, $archiveUnix, $Entry, $destinationUnix) | Out-Null
    if (-not (Test-Path -LiteralPath $Destination -PathType Leaf)) {
        throw "Archive entry extraction did not create a regular file: $Entry"
    }
}

function Invoke-Ldd([string[]]$UnixPaths) {
    if ($UnixPaths.Count -eq 0) {
        throw "Invoke-Ldd requires at least one runtime path."
    }
    # Pass every path as data instead of interpolating it into shell syntax.
    return Invoke-Msys2 `
        'environment="$1"; shift; PATH="$environment/bin:/usr/bin" ldd "$@"' `
        (@($environmentUnix) + $UnixPaths)
}

function Convert-EnvironmentPath([string]$UnixPath) {
    if (-not $UnixPath.StartsWith($environmentBinUnix, [System.StringComparison]::Ordinal)) {
        throw "Not a $([string]$environmentContract.displayName) binary path: $UnixPath"
    }
    $name = $UnixPath.Substring($environmentBinUnix.Length).Replace('/', '\')
    return Join-Path (Join-Path $Msys2Root "$environmentDirectory\bin") $name
}

$pending = [System.Collections.Generic.Queue[string]]::new()
$queued = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
$seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
$maximumRuntimeFiles = 256
$lddBatchSize = 64
$pending.Enqueue("${environmentBinUnix}bsdtar.exe")
$null = $queued.Add("${environmentBinUnix}bsdtar.exe")

Write-Host "Resolving $([string]$environmentContract.displayName) runtime dependencies..."
while ($pending.Count -gt 0) {
    $batch = [System.Collections.Generic.List[string]]::new()
    while ($pending.Count -gt 0 -and $batch.Count -lt $lddBatchSize) {
        $current = $pending.Dequeue()
        $null = $queued.Remove($current)
        if (-not $seen.Add($current)) {
            continue
        }
        if ($seen.Count -gt $maximumRuntimeFiles) {
            throw "MSYS2 runtime dependency inventory exceeds the $maximumRuntimeFiles-file limit."
        }
        $batch.Add($current)
    }
    if ($batch.Count -eq 0) {
        continue
    }

    foreach ($lineObject in (Invoke-Ldd $batch.ToArray())) {
        $line = [string]$lineObject
        if ($line -match '=>\s+not found(?:\s|$)') {
            # Windows API-set contract names are virtual loader aliases backed
            # by the OS API-set schema, not redistributable DLL payloads.
            if ($line -match '^\s*(?:api|ext)-ms-win-[A-Za-z0-9._-]+\.dll\s+=>\s+not found\s*$') {
                continue
            }
            $unresolved = $line.Trim()
            if ($unresolved.Length -gt 512) {
                $unresolved = $unresolved.Substring(0, 512) + "..."
            }
            throw "ldd reported an unresolved runtime dependency: $unresolved"
        }
        $dependency = $null
        if ($line -match '=>\s+(/[^\s]+)' -and
            $Matches[1].StartsWith($environmentBinUnix, [System.StringComparison]::Ordinal)) {
            $dependency = [string]$Matches[1]
        }
        elseif ($line -match '^\s*(/[^\s]+)\s+\(' -and
            $Matches[1].StartsWith($environmentBinUnix, [System.StringComparison]::Ordinal)) {
            $dependency = [string]$Matches[1]
        }
        if ($null -ne $dependency -and
            -not $seen.Contains($dependency) -and
            $queued.Add($dependency)) {
            if ($seen.Count + $queued.Count -gt $maximumRuntimeFiles) {
                throw "MSYS2 runtime dependency inventory exceeds the $maximumRuntimeFiles-file limit."
            }
            $pending.Enqueue($dependency)
        }
    }
}

$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("iroha-zip-msys2-" + [Guid]::NewGuid().ToString("N"))
$temporaryBundle = Join-Path $temporaryRoot "bundle"
$packageCache = Join-Path $temporaryRoot "packages"
$packageDatabase = Join-Path $temporaryRoot "database"
$packageExtractRoot = Join-Path $temporaryRoot "extracted"
$licenseRoot = Join-Path $temporaryRoot "licenses"
New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
New-Item -ItemType Directory -Path $temporaryBundle | Out-Null
New-Item -ItemType Directory -Path $packageCache | Out-Null
New-Item -ItemType Directory -Path $packageDatabase | Out-Null
New-Item -ItemType Directory -Path (Join-Path $packageDatabase "local") | Out-Null
New-Item -ItemType Directory -Path $packageExtractRoot | Out-Null
New-Item -ItemType Directory -Path $licenseRoot | Out-Null
try {
    $secureConfig = Join-Path $temporaryRoot "pacman-required-trusted-only.conf"
    $secureConfigUnix = Convert-ToMsysPath $secureConfig
    $packageCacheUnix = Convert-ToMsysPath $packageCache
    $packageDatabaseUnix = Convert-ToMsysPath $packageDatabase
    $secureLogUnix = Convert-ToMsysPath (Join-Path $temporaryRoot "pacman.log")
    $secureConfigText = @"
[options]
Architecture = auto
DBPath = $packageDatabaseUnix
CacheDir = $packageCacheUnix
GPGDir = /etc/pacman.d/gnupg
LogFile = $secureLogUnix
SigLevel = Required TrustedOnly
LocalFileSigLevel = Required TrustedOnly
RemoteFileSigLevel = Required TrustedOnly

[msys]
Include = /etc/pacman.d/mirrorlist.msys

[$environmentDirectory]
Include = /etc/pacman.d/mirrorlist.mingw
"@
    [System.IO.File]::WriteAllText(
        $secureConfig,
        $secureConfigText,
        [System.Text.UTF8Encoding]::new($false)
    )
    Write-Host "Refreshing isolated signed MSYS2 package databases..."
    Invoke-Msys2 `
        'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sy --noconfirm' `
        @($secureConfigUnix) | Out-Null
    $keyringInstalled = Invoke-Msys2Scalar `
        'LANG=C PATH=/usr/bin /usr/bin/pacman -Q -- msys2-keyring' `
        @()
    $keyringParts = $keyringInstalled -split '\s+', 2
    if ($keyringParts.Count -ne 2 -or $keyringParts[0] -ne "msys2-keyring") {
        throw "Cannot identify the MSYS2 keyring used for package verification: $keyringInstalled"
    }

    # Batch package queries. Starting a fresh MSYS2 shell and reopening the
    # package databases for every DLL dominates export time on hosted runners.
    Write-Host "Resolving installed package ownership and signed repository metadata..."
    $runtimePaths = @($seen | Sort-Object)
    $owners = @(
        Invoke-Msys2 `
            'LANG=C PATH=/usr/bin /usr/bin/pacman -Qqo -- "$@"' `
            -Arguments $runtimePaths
    )
    if ($owners.Count -ne $runtimePaths.Count) {
        throw "Cannot map every runtime file to exactly one installed package. Paths=$($runtimePaths.Count) owners=$($owners.Count)"
    }

    $ownerByUnixPath = @{}
    for ($index = 0; $index -lt $runtimePaths.Count; $index++) {
        $unixPath = [string]$runtimePaths[$index]
        $owner = [string]$owners[$index]
        if ($owner -notmatch $packagePattern) {
            throw "Runtime file is not owned by a supported $([string]$environmentContract.displayName) package: $unixPath -> $owner"
        }
        $ownerByUnixPath[$unixPath] = $owner
    }

    $packageNames = @($ownerByUnixPath.Values | Sort-Object -Unique)
    $installedVersions = @{}
    foreach ($installed in @(
        Invoke-Msys2 `
            'LANG=C PATH=/usr/bin /usr/bin/pacman -Q -- "$@"' `
            -Arguments $packageNames
    )) {
        $installedParts = ([string]$installed) -split '\s+', 2
        if ($installedParts.Count -ne 2 -or
            $installedParts[0] -notmatch $packagePattern -or
            $installedVersions.ContainsKey($installedParts[0])) {
            throw "Cannot parse installed package version: $installed"
        }
        $installedVersions[$installedParts[0]] = $installedParts[1]
    }
    if ($installedVersions.Count -ne $packageNames.Count) {
        throw "Cannot identify every installed runtime package version. Expected=$($packageNames.Count) found=$($installedVersions.Count)"
    }

    $printFormat = "%n`t%v`t%r`t%a`t%l`t%h`t%L"
    $repositoryQueryArguments = @($secureConfigUnix, $printFormat) + $packageNames
    $repositoryMetadata = @{}
    foreach ($metadataLine in @(
        Invoke-Msys2 `
            'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sddp --print-format "$2" -- "${@:3}"' `
            -Arguments $repositoryQueryArguments
    )) {
        $parts = ([string]$metadataLine) -split "`t", 7
        if ($parts.Count -ne 7 -or
            $parts[0] -notmatch $packagePattern -or
            $repositoryMetadata.ContainsKey($parts[0])) {
            throw "Cannot parse signed repository package metadata: $metadataLine"
        }
        $repositoryMetadata[$parts[0]] = [pscustomobject][ordered]@{
            version = $parts[1]
            repository = $parts[2]
            architecture = $parts[3]
            downloadUrl = $parts[4]
            archiveSha256 = $parts[5].ToLowerInvariant()
            licenses = @(
                $parts[6].Trim() |
                    Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
            )
        }
    }
    if ($repositoryMetadata.Count -ne $packageNames.Count) {
        throw "Cannot identify every signed repository package. Expected=$($packageNames.Count) found=$($repositoryMetadata.Count)"
    }

    $packageMetadata = @()
    $archiveByPackage = @{}
    $packageIdByName = @{}
    foreach ($packageName in $packageNames) {
        $repositoryPackage = $repositoryMetadata[$packageName]
        $version = [string]$repositoryPackage.version
        $repository = [string]$repositoryPackage.repository
        $architecture = [string]$repositoryPackage.architecture
        $downloadUrl = [string]$repositoryPackage.downloadUrl
        $archiveSha256 = [string]$repositoryPackage.archiveSha256
        $licenses = @($repositoryPackage.licenses)
        if ([string]$installedVersions[$packageName] -ne $version) {
            throw "Installed package is not the current signed repository version: $packageName installed=$($installedVersions[$packageName]) repository=$version. Update MSYS2 first."
        }
        if ($repository -ne $environmentDirectory -or
            $downloadUrl -notmatch '^https://' -or
            $archiveSha256 -notmatch '^[0-9a-f]{64}$') {
            throw "Incomplete or unsupported signed repository metadata for package: $packageName"
        }
        if ($licenses.Count -eq 0) {
            $licenses = @("NOASSERTION")
        }
        $packageIdByName[$packageName] = $packageName
        $packageMetadata += [pscustomobject][ordered]@{
            id = $packageName
            name = $packageName
            version = $version
            architecture = $architecture
            repository = $repository
            downloadUrl = $downloadUrl
            archiveSha256 = $archiveSha256
            signature = $null
            licenses = @($licenses)
        }
    }

    # One download transaction preserves the same Required/TrustedOnly checks
    # while avoiding a separate repository transaction for every package.
    Write-Host "Downloading and verifying $($packageNames.Count) signed package archives..."
    $downloadArguments = @($secureConfigUnix, $packageCacheUnix) + $packageNames
    Invoke-Msys2 `
        'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sddw --noconfirm --cachedir "$2" -- "${@:3}"' `
        -Arguments $downloadArguments | Out-Null
    Invoke-Msys2 `
        'LANG=C PATH=/usr/bin /usr/bin/pacman -Qkk -- "$@"' `
        -Arguments $packageNames | Out-Null

    Write-Host "Extracting verified payload and license evidence..."
    foreach ($package in $packageMetadata) {
        $uri = [System.Uri]::new([string]$package.downloadUrl)
        $archiveName = [System.Uri]::UnescapeDataString([System.IO.Path]::GetFileName($uri.AbsolutePath))
        if ([string]::IsNullOrWhiteSpace($archiveName) -or
            [System.IO.Path]::IsPathRooted($archiveName) -or
            $archiveName -ne [System.IO.Path]::GetFileName($archiveName)) {
            throw "Signed repository metadata contains an unsafe package archive name: $archiveName"
        }
        $archive = Join-Path $packageCache $archiveName
        if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) {
            throw "pacman did not retain the verified package archive: $archive"
        }
        $actualArchiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actualArchiveHash -ne [string]$package.archiveSha256) {
            throw "Downloaded package SHA-256 does not match signed repository metadata: $($package.name)"
        }
        # Required TrustedOnly makes pacman download and validate the detached
        # signature before the download-only transaction succeeds.
        $signaturePath = $archive + ".sig"
        if (-not (Test-Path -LiteralPath $signaturePath -PathType Leaf)) {
            throw "pacman did not retain the required package signature: $signaturePath"
        }
        $signature = [System.Convert]::ToBase64String(
            [System.IO.File]::ReadAllBytes($signaturePath)
        )
        if ($signature.Length -lt 32 -or
            $signature.Length -gt 32768 -or
            $signature -notmatch '^[A-Za-z0-9+/=]+$') {
            throw "Verified package signature is missing or unbounded: $($package.name)"
        }
        $package.signature = $signature
        $archiveByPackage[[string]$package.name] = $archive

        $packageExtract = Join-Path $packageExtractRoot ([string]$package.id)
        New-Item -ItemType Directory -Path $packageExtract | Out-Null
        $archiveUnix = Convert-ToMsysPath $archive
        $licenseEntries = @(
            Invoke-Msys2 `
                'environment="$1"; PATH="$environment/bin:/usr/bin" "$environment/bin/bsdtar.exe" -tf "$2"' `
                @($environmentUnix, $archiveUnix) |
                ForEach-Object { [string]$_ } |
                Where-Object {
                    $_ -match $licenseEntryPattern -and
                    -not $_.EndsWith('/') -and
                    -not $_.Contains('..') -and
                    -not $_.Contains('\')
                } |
                Sort-Object -Unique
        )
        foreach ($entry in $licenseEntries) {
            $licenseRelative = $entry.Substring($licenseEntryPrefix.Length)
            $licenseDestination = Join-Path (Join-Path $licenseRoot ([string]$package.id)) $licenseRelative.Replace('/', '\')
            Export-ArchiveEntry $archive $entry $licenseDestination
        }
    }

    $fileMappings = @()
    foreach ($unixPath in @($seen | Sort-Object)) {
        $source = Convert-EnvironmentPath $unixPath
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "A dependency reported by ldd is missing: $source"
        }
        $packageName = [string]$ownerByUnixPath[$unixPath]
        $archive = [string]$archiveByPackage[$packageName]
        $packageExtract = Join-Path $packageExtractRoot $packageName
        $entry = $unixPath.TrimStart('/')
        $verifiedSource = Join-Path $packageExtract $entry.Replace('/', '\')
        Export-ArchiveEntry $archive $entry $verifiedSource
        $installedHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
        $verifiedHash = (Get-FileHash -LiteralPath $verifiedSource -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($installedHash -ne $verifiedHash) {
            throw "Installed runtime file differs from the signed package archive: $unixPath"
        }
        $fileName = [System.IO.Path]::GetFileName($source)
        $target = Join-Path $temporaryBundle $fileName
        if (Test-Path -LiteralPath $target) {
            throw "Selected runtime paths collide in the flat backend bundle: $fileName"
        }
        Copy-Item -LiteralPath $verifiedSource -Destination $target
        $fileMappings += [ordered]@{
            path = $fileName
            packageId = $packageIdByName[$packageName]
        }
    }

    $verifiedPayloads = @(
        Get-ChildItem -LiteralPath $temporaryBundle -File -Force |
            Sort-Object Name |
            ForEach-Object { $_.FullName }
    )
    if ($verifiedPayloads.Count -ne $seen.Count) {
        throw "Verified backend payload count changed before PE architecture validation."
    }
    & (Join-Path $PSScriptRoot "verify-pe-architecture.ps1") `
        -Files $verifiedPayloads `
        -Architecture ([string]$environmentContract.assetArchitecture)

    $metadata = [ordered]@{
        source = [ordered]@{
            kind = [string]$environmentContract.sourceKind
            supported = $true
            repository = $environmentDirectory
            verification = [ordered]@{
                status = "verified"
                method = "pacman-required-trusted-only"
                keyringPackage = "msys2-keyring"
                keyringVersion = $keyringParts[1]
            }
        }
        packages = @($packageMetadata | Sort-Object id)
        files = @($fileMappings | Sort-Object path)
    }
    $metadataPath = Join-Path $temporaryRoot "backend-source-metadata.json"
    [System.IO.File]::WriteAllText(
        $metadataPath,
        (($metadata | ConvertTo-Json -Depth 10) + "`n"),
        [System.Text.UTF8Encoding]::new($false)
    )

    & (Join-Path $PSScriptRoot "install-backend.ps1") `
        -SourceDirectory $temporaryBundle `
        -DestinationDirectory $DestinationDirectory `
        -EvidenceMetadataPath $metadataPath `
        -LicenseDirectory $licenseRoot
    if ($LASTEXITCODE -ne 0) {
        throw "Backend installation failed after verified MSYS2 export."
    }

    Write-Host "Collected $([string]$environmentContract.displayName) runtime files: $($seen.Count)"
    Write-Host "Verified signed packages: $($packageMetadata.Count)"
    Write-Host "Backend source: MSYS2 $Msys2Root"
}
finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}
