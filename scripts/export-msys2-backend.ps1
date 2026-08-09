[CmdletBinding()]
param(
    [string]$Msys2Root = "C:\msys64",
    [string]$DestinationDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($DestinationDirectory)) {
    $DestinationDirectory = Join-Path $ProjectRoot "backend\libarchive"
}

$Msys2Root = [System.IO.Path]::GetFullPath($Msys2Root)
$bash = Join-Path $Msys2Root "usr\bin\bash.exe"
$bsdtar = Join-Path $Msys2Root "ucrt64\bin\bsdtar.exe"
$pacman = Join-Path $Msys2Root "usr\bin\pacman.exe"
if (-not (Test-Path -LiteralPath $bash -PathType Leaf)) {
    throw "MSYS2 bash.exe was not found: $bash"
}
if (-not (Test-Path -LiteralPath $bsdtar -PathType Leaf)) {
    throw @"
MSYS2 UCRT64 bsdtar.exe was not found: $bsdtar
Install it from an MSYS2 UCRT64 shell:
  pacman -S mingw-w64-ucrt-x86_64-libarchive
"@
}
if (-not (Test-Path -LiteralPath $pacman -PathType Leaf)) {
    throw "MSYS2 pacman.exe was not found: $pacman"
}

function Invoke-Msys2([string]$Script, [string[]]$Arguments) {
    $output = @(& $bash --noprofile --norc -lc $Script iroha-zip @Arguments 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw "MSYS2 command failed (exit $LASTEXITCODE).`n$($output -join "`n")"
    }
    return @($output | ForEach-Object { [string]$_ })
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
        'PATH=/ucrt64/bin:/usr/bin /ucrt64/bin/bsdtar.exe -xOf "$1" -- "$2" > "$3"' `
        @($archiveUnix, $Entry, $destinationUnix) | Out-Null
    if (-not (Test-Path -LiteralPath $Destination -PathType Leaf)) {
        throw "Archive entry extraction did not create a regular file: $Entry"
    }
}

function Invoke-Ldd([string]$UnixPath) {
    # Pass the path as bash $1 instead of interpolating it into shell syntax.
    $output = @(
        & $bash --noprofile --norc -lc 'PATH=/ucrt64/bin:/usr/bin ldd "$1"' `
            iroha-zip-ldd $UnixPath 2>&1
    )
    if ($LASTEXITCODE -ne 0) {
        throw "ldd failed for $UnixPath`n$($output -join "`n")"
    }
    return $output
}

function Convert-UcrtPath([string]$UnixPath) {
    if (-not $UnixPath.StartsWith("/ucrt64/bin/", [System.StringComparison]::Ordinal)) {
        throw "Not a UCRT64 binary path: $UnixPath"
    }
    $name = $UnixPath.Substring("/ucrt64/bin/".Length).Replace('/', '\')
    return Join-Path (Join-Path $Msys2Root "ucrt64\bin") $name
}

$pending = [System.Collections.Generic.Queue[string]]::new()
$seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
$pending.Enqueue("/ucrt64/bin/bsdtar.exe")

while ($pending.Count -gt 0) {
    $current = $pending.Dequeue()
    if (-not $seen.Add($current)) {
        continue
    }

    foreach ($lineObject in (Invoke-Ldd $current)) {
        $line = [string]$lineObject
        $dependency = $null
        if ($line -match '=>\s+(/ucrt64/bin/[^\s]+)') {
            $dependency = $Matches[1]
        }
        elseif ($line -match '^\s*(/ucrt64/bin/[^\s]+)\s+\(') {
            $dependency = $Matches[1]
        }
        if ($null -ne $dependency -and -not $seen.Contains($dependency)) {
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

[ucrt64]
Include = /etc/pacman.d/mirrorlist.mingw
"@
    [System.IO.File]::WriteAllText(
        $secureConfig,
        $secureConfigText,
        [System.Text.UTF8Encoding]::new($false)
    )
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

    $ownerByUnixPath = @{}
    foreach ($unixPath in @($seen | Sort-Object)) {
        $owner = Invoke-Msys2Scalar `
            'LANG=C PATH=/usr/bin /usr/bin/pacman -Qqo -- "$1"' `
            @($unixPath)
        if ($owner -notmatch '^mingw-w64-ucrt-x86_64-[A-Za-z0-9@._+-]+$') {
            throw "Runtime file is not owned by a supported UCRT64 package: $unixPath -> $owner"
        }
        $ownerByUnixPath[$unixPath] = $owner
    }

    $packageNames = @($ownerByUnixPath.Values | Sort-Object -Unique)
    $packageMetadata = @()
    $archiveByPackage = @{}
    $packageIdByName = @{}
    foreach ($packageName in $packageNames) {
        $installed = Invoke-Msys2Scalar `
            'LANG=C PATH=/usr/bin /usr/bin/pacman -Q -- "$1"' `
            @($packageName)
        $installedParts = $installed -split '\s+', 2
        if ($installedParts.Count -ne 2 -or $installedParts[0] -ne $packageName) {
            throw "Cannot parse installed package version: $installed"
        }
        $version = Invoke-Msys2Scalar `
            'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sp --print-format "$2" -- "$3"' `
            @($secureConfigUnix, "%v", $packageName)
        $repository = Invoke-Msys2Scalar `
            'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sp --print-format "$2" -- "$3"' `
            @($secureConfigUnix, "%r", $packageName)
        $architecture = Invoke-Msys2Scalar `
            'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sp --print-format "$2" -- "$3"' `
            @($secureConfigUnix, "%a", $packageName)
        $downloadUrl = Invoke-Msys2Scalar `
            'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sp --print-format "$2" -- "$3"' `
            @($secureConfigUnix, "%l", $packageName)
        $archiveSha256 = (Invoke-Msys2Scalar `
            'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sp --print-format "$2" -- "$3"' `
            @($secureConfigUnix, "%h", $packageName)).ToLowerInvariant()
        $signature = Invoke-Msys2Scalar `
            'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sp --print-format "$2" -- "$3"' `
            @($secureConfigUnix, "%g", $packageName)
        $licenses = @(
            Invoke-Msys2 `
                'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sp --print-format "$2" -- "$3"' `
                @($secureConfigUnix, "%L", $packageName) |
                ForEach-Object { ([string]$_).Trim() } |
                Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
        )
        if ($installedParts[1] -ne $version) {
            throw "Installed package is not the current signed repository version: $packageName installed=$($installedParts[1]) repository=$version. Update MSYS2 first."
        }
        if ($repository -ne "ucrt64" -or
            $downloadUrl -notmatch '^https://' -or
            $archiveSha256 -notmatch '^[0-9a-f]{64}$' -or
            $signature.Length -lt 32 -or
            $signature -notmatch '^[A-Za-z0-9+/=]+$') {
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
            signature = $signature
            licenses = @($licenses)
        }
    }

    foreach ($package in $packageMetadata) {
        Invoke-Msys2 `
            'LANG=C PATH=/usr/bin /usr/bin/pacman --config "$1" -Sddw --noconfirm --cachedir "$2" -- "$3"' `
            @($secureConfigUnix, $packageCacheUnix, [string]$package.name) | Out-Null
        Invoke-Msys2 `
            'LANG=C PATH=/usr/bin /usr/bin/pacman -Qkk -- "$1"' `
            @([string]$package.name) | Out-Null

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
        $archiveByPackage[[string]$package.name] = $archive

        $packageExtract = Join-Path $packageExtractRoot ([string]$package.id)
        New-Item -ItemType Directory -Path $packageExtract | Out-Null
        $licenseEntries = @(
            & $bsdtar -tf $archive 2>&1 |
                ForEach-Object { [string]$_ } |
                Where-Object {
                    $_ -match '^ucrt64/share/licenses/[A-Za-z0-9@._+-]+/.+' -and
                    -not $_.EndsWith('/') -and
                    -not $_.Contains('..') -and
                    -not $_.Contains('\')
                } |
                Sort-Object -Unique
        )
        if ($LASTEXITCODE -ne 0) {
            throw "Cannot list verified package archive: $archive"
        }
        foreach ($entry in $licenseEntries) {
            $licenseRelative = $entry.Substring("ucrt64/share/licenses/".Length)
            $licenseDestination = Join-Path (Join-Path $licenseRoot ([string]$package.id)) $licenseRelative.Replace('/', '\')
            Export-ArchiveEntry $archive $entry $licenseDestination
        }
    }

    $fileMappings = @()
    foreach ($unixPath in @($seen | Sort-Object)) {
        $source = Convert-UcrtPath $unixPath
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

    $metadata = [ordered]@{
        source = [ordered]@{
            kind = "msys2-ucrt64-pacman"
            supported = $true
            repository = "ucrt64"
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

    Write-Host "Collected UCRT64 runtime files: $($seen.Count)"
    Write-Host "Verified signed packages: $($packageMetadata.Count)"
    Write-Host "Backend source: MSYS2 $Msys2Root"
}
finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}
