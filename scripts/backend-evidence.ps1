Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Utf8NoBom([string]$Path, [string]$Content) {
    [System.IO.File]::WriteAllText(
        $Path,
        $Content,
        [System.Text.UTF8Encoding]::new($false)
    )
}

function Write-JsonDocument([string]$Path, [object]$Value) {
    $json = $Value | ConvertTo-Json -Depth 12
    Write-Utf8NoBom $Path ($json + "`n")
}

function Get-LowerHash([string]$Path, [string]$Algorithm) {
    return (Get-FileHash -LiteralPath $Path -Algorithm $Algorithm).Hash.ToLowerInvariant()
}

function Copy-PlainEvidenceFile([string]$Source, [string]$Destination) {
    $parent = Split-Path -Parent $Destination
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    $input = [System.IO.File]::Open(
        $Source,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::Read
    )
    try {
        $output = [System.IO.File]::Open(
            $Destination,
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

function Assert-EvidenceIdentifier([string]$Value, [string]$Description) {
    if ([string]::IsNullOrWhiteSpace($Value) -or
        $Value.Length -gt 256 -or
        $Value -notmatch '^[A-Za-z0-9._+-]+$') {
        throw "Invalid $Description in backend evidence: $Value"
    }
}

function Assert-EvidenceText([string]$Value, [int]$MaximumLength, [string]$Description) {
    if ([string]::IsNullOrWhiteSpace($Value) -or
        $Value.Length -gt $MaximumLength -or
        $Value -match '[\x00-\x1F\x7F]') {
        throw "Invalid $Description in backend evidence."
    }
}

function Sort-EvidenceObjectsOrdinal([object[]]$Values, [string]$Property) {
    $byKey = @{}
    $keys = [System.Collections.Generic.List[string]]::new()
    foreach ($value in $Values) {
        $key = if ($value -is [System.Collections.IDictionary]) {
            [string]$value[$Property]
        }
        else {
            [string]$value.$Property
        }
        if ($byKey.ContainsKey($key)) {
            throw "Duplicate $Property in backend evidence: $key"
        }
        $byKey[$key] = $value
        $keys.Add($key)
    }
    $keyArray = [string[]]$keys.ToArray()
    [Array]::Sort($keyArray, [System.StringComparer]::Ordinal)
    return @($keyArray | ForEach-Object { $byKey[$_] })
}

function Get-SpdxPackageVerificationCode([object[]]$Files) {
    $hashes = @(
        $Files |
            ForEach-Object { Get-LowerHash ([string]$_.fullPath) "SHA1" } |
            Sort-Object
    )
    $joined = [string]::Join("", $hashes)
    $algorithm = [System.Security.Cryptography.SHA1]::Create()
    try {
        $digest = $algorithm.ComputeHash([System.Text.Encoding]::ASCII.GetBytes($joined))
    }
    finally {
        $algorithm.Dispose()
    }
    return [System.BitConverter]::ToString($digest).Replace("-", "").ToLowerInvariant()
}

function Assert-GeneratedEvidenceBounds([string]$EvidenceDirectory) {
    $items = @(Get-ChildItem -LiteralPath $EvidenceDirectory -Recurse -Force)
    if ($items.Count -gt 2048) {
        throw "Generated backend evidence exceeds the 2,048-entry limit."
    }
    $files = @($items | Where-Object { -not $_.PSIsContainer })
    if ($files.Count -gt 1024) {
        throw "Generated backend evidence exceeds the 1,024-file limit."
    }
    [uint64]$totalBytes = 0
    foreach ($file in $files) {
        $totalBytes += [uint64]$file.Length
        if ($totalBytes -gt 32MB) {
            throw "Generated backend evidence exceeds the 32 MiB total limit."
        }
    }
    foreach ($documentName in @(
        "backend-provenance.json",
        "backend.spdx.json",
        "backend-license-inventory.json"
    )) {
        $document = Get-Item -LiteralPath (Join-Path $EvidenceDirectory $documentName) -Force -ErrorAction Stop
        if ($document.PSIsContainer -or $document.Length -gt 4MB) {
            throw "Generated evidence document exceeds the 4 MiB limit: $documentName"
        }
    }
}

function New-IrohaZipBackendEvidence {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$BackendDirectory,

        [Parameter(Mandatory = $true)]
        [object]$SourceMetadata,

        [string]$LicenseSourceDirectory
    )

    $manifestPath = Join-Path $BackendDirectory "backend-manifest.tsv"
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Cannot create backend evidence without backend-manifest.tsv."
    }

    $source = $SourceMetadata.source
    Assert-EvidenceText ([string]$source.kind) 128 "source kind"
    Assert-EvidenceText ([string]$source.repository) 128 "source repository"
    Assert-EvidenceText ([string]$source.verification.status) 32 "verification status"
    Assert-EvidenceText ([string]$source.verification.method) 128 "verification method"
    $supported = [bool]$source.supported
    if ($supported) {
        if ([string]$source.kind -ne "msys2-ucrt64-pacman" -or
            [string]$source.repository -ne "ucrt64" -or
            [string]$source.verification.status -ne "verified" -or
            [string]$source.verification.method -ne "pacman-required-trusted-only") {
            throw "Only verified MSYS2 UCRT64 pacman metadata is a supported backend source."
        }
        Assert-EvidenceText ([string]$source.verification.keyringPackage) 128 "keyring package"
        Assert-EvidenceText ([string]$source.verification.keyringVersion) 256 "keyring version"
        if ([string]$source.verification.keyringPackage -ne "msys2-keyring") {
            throw "Supported MSYS2 evidence must identify the msys2-keyring package."
        }
    }
    else {
        if ([string]$source.kind -ne "unverified-local-bundle" -or
            [string]$source.repository -ne "unverified-local" -or
            [string]$source.verification.status -ne "unverified" -or
            [string]$source.verification.method -ne "explicit-user-accepted-local-bundle" -or
            $null -ne $source.verification.keyringPackage -or
            $null -ne $source.verification.keyringVersion) {
            throw "Unsupported backend metadata does not preserve the explicit warning state."
        }
    }

    $packages = @(Sort-EvidenceObjectsOrdinal @($SourceMetadata.packages) "id")
    if ($packages.Count -eq 0 -or $packages.Count -gt 256) {
        throw "Backend evidence must describe between 1 and 256 packages."
    }
    $packageById = @{}
    foreach ($package in $packages) {
        $packageId = [string]$package.id
        Assert-EvidenceIdentifier $packageId "package id"
        Assert-EvidenceText ([string]$package.name) 512 "package name"
        Assert-EvidenceText ([string]$package.version) 256 "package version"
        Assert-EvidenceText ([string]$package.architecture) 64 "package architecture"
        Assert-EvidenceText ([string]$package.repository) 128 "package repository"
        $licenses = @($package.licenses)
        if ($licenses.Count -eq 0 -or $licenses.Count -gt 64) {
            throw "Package $packageId has no bounded license metadata."
        }
        foreach ($license in $licenses) {
            Assert-EvidenceText ([string]$license) 256 "package license"
        }
        if ($supported) {
            if ([string]$package.name -ne $packageId -or
                -not $packageId.StartsWith("mingw-w64-ucrt-x86_64-", [System.StringComparison]::Ordinal) -or
                [string]$package.repository -ne "ucrt64" -or
                [string]$package.downloadUrl -notmatch '^https://' -or
                [string]$package.archiveSha256 -notmatch '^[0-9a-f]{64}$' -or
                [string]$package.signature -notmatch '^[A-Za-z0-9+/=]{32,32768}$') {
                throw "Verified package metadata is incomplete or unsupported: $packageId"
            }
        }
        elseif ($packageId -ne "unverified-local-bundle" -or
            [string]$package.name -ne "unverified-local-bundle" -or
            [string]$package.version -ne "NOASSERTION" -or
            [string]$package.architecture -ne "windows" -or
            [string]$package.repository -ne "unverified-local" -or
            $null -ne $package.downloadUrl -or
            $null -ne $package.archiveSha256 -or
            $null -ne $package.signature -or
            $licenses.Count -ne 1 -or
            [string]$licenses[0] -ne "NOASSERTION") {
            throw "Unsupported package metadata must retain the canonical unverified identity."
        }
        if ($packageById.ContainsKey($packageId)) {
            throw "Duplicate backend evidence package id: $packageId"
        }
        $packageById[$packageId] = $package
    }
    if (-not $supported -and $packages.Count -ne 1) {
        throw "Unsupported provenance must describe exactly one unverified local package."
    }

    $reservedEvidenceRoot = Join-Path $BackendDirectory ".iroha-zip-evidence"
    $reservedEvidencePrefix = $reservedEvidenceRoot.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    $payloadFiles = @(Sort-EvidenceObjectsOrdinal @(
        Get-ChildItem -LiteralPath $BackendDirectory -Recurse -Force -File |
            Where-Object {
                $_.Name -ne "backend-manifest.tsv" -and
                -not $_.FullName.Equals($reservedEvidenceRoot, [System.StringComparison]::OrdinalIgnoreCase) -and
                -not $_.FullName.StartsWith($reservedEvidencePrefix, [System.StringComparison]::OrdinalIgnoreCase)
            } |
            ForEach-Object {
                $relative = $_.FullName.Substring($BackendDirectory.Length).TrimStart('\', '/')
                [pscustomobject]@{
                    path = $relative.Replace('\', '/')
                    fullPath = $_.FullName
                    sha256 = Get-LowerHash $_.FullName "SHA256"
                }
            }
    ) "path")
    if ($payloadFiles.Count -eq 0 -or $payloadFiles.Count -gt 4096) {
        throw "Backend evidence payload file count is out of bounds."
    }

    $ownerByPath = @{}
    foreach ($mapping in @($SourceMetadata.files)) {
        $path = ([string]$mapping.path).Replace('\', '/')
        $packageId = [string]$mapping.packageId
        if ($ownerByPath.ContainsKey($path) -or -not $packageById.ContainsKey($packageId)) {
            throw "Invalid or duplicate backend ownership mapping: $path"
        }
        $ownerByPath[$path] = $packageId
    }
    if ($ownerByPath.Count -ne $payloadFiles.Count) {
        throw "Backend ownership mapping does not match the payload file count."
    }

    $payloadRecords = @(
        foreach ($file in $payloadFiles) {
            if (-not $ownerByPath.ContainsKey([string]$file.path)) {
                throw "Backend ownership metadata is missing: $($file.path)"
            }
            [ordered]@{
                path = [string]$file.path
                sha256 = [string]$file.sha256
                packageId = [string]$ownerByPath[[string]$file.path]
            }
        }
    )

    $evidence = $reservedEvidenceRoot
    if (Test-Path -LiteralPath $evidence) {
        throw "Backend staging tree unexpectedly contains an evidence directory."
    }
    New-Item -ItemType Directory -Path $evidence | Out-Null

    if (-not [string]::IsNullOrWhiteSpace($LicenseSourceDirectory)) {
        $licenseSourceItem = Get-Item -LiteralPath $LicenseSourceDirectory -Force -ErrorAction Stop
        if (-not $licenseSourceItem.PSIsContainer -or
            ($licenseSourceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "LicenseSourceDirectory must be a regular directory, not a reparse point."
        }
    }

    $licenseRecordsByPackage = @{}
    foreach ($package in $packages) {
        $packageId = [string]$package.id
        $records = @()
        if (-not [string]::IsNullOrWhiteSpace($LicenseSourceDirectory)) {
            $packageLicenseRoot = Join-Path $LicenseSourceDirectory $packageId
            if (Test-Path -LiteralPath $packageLicenseRoot -PathType Container) {
                $rootItem = Get-Item -LiteralPath $packageLicenseRoot -Force
                if (($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                    throw "License source must not be a reparse point: $packageLicenseRoot"
                }
                $licenseItems = @(Get-ChildItem -LiteralPath $packageLicenseRoot -Recurse -Force)
                foreach ($item in $licenseItems) {
                    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                        throw "License evidence must not contain reparse points: $($item.FullName)"
                    }
                }
                foreach ($file in @($licenseItems | Where-Object { -not $_.PSIsContainer } | Sort-Object FullName)) {
                    $relative = $file.FullName.Substring($packageLicenseRoot.Length).TrimStart('\', '/')
                    if ([string]::IsNullOrWhiteSpace($relative) -or
                        $relative.Contains("`t") -or
                        $relative.Contains("`r") -or
                        $relative.Contains("`n")) {
                        throw "Unsafe license evidence path: $relative"
                    }
                    $evidenceRelative = "licenses/$packageId/" + $relative.Replace('\', '/')
                    $destination = Join-Path $evidence $evidenceRelative.Replace('/', '\')
                    Copy-PlainEvidenceFile $file.FullName $destination
                    $records += [ordered]@{
                        path = $evidenceRelative
                        sha256 = Get-LowerHash $destination "SHA256"
                    }
                }
            }
        }
        $licenseRecordsByPackage[$packageId] = @(
            Sort-EvidenceObjectsOrdinal -Values @($records) -Property "path"
        )
    }

    $warning = if ($supported) {
        "The package archives were downloaded and accepted by pacman with Required/TrustedOnly signature policy before their imported bytes were compared."
    }
    else {
        "WARNING: This local bundle is an unsupported source. Its origin and distributor signature were not verified."
    }
    $noticeLines = [System.Collections.Generic.List[string]]::new()
    $noticeLines.Add("# Backend third-party notices")
    $noticeLines.Add("")
    $noticeLines.Add("$warning")
    $noticeLines.Add("")
    $noticeLines.Add("This inventory preserves distributor metadata; it is not legal advice or an independent license conclusion.")
    foreach ($package in $packages) {
        $packageId = [string]$package.id
        $noticeLines.Add("")
        $noticeLines.Add("## $([string]$package.name) $([string]$package.version)")
        $noticeLines.Add("")
        $noticeLines.Add("- Package ID: ``$packageId``")
        $noticeLines.Add("- Repository: ``$([string]$package.repository)``")
        $noticeLines.Add("- Architecture: ``$([string]$package.architecture)``")
        $noticeLines.Add("- Distributor license metadata: ``$([string]::Join(', ', @($package.licenses)))``")
        if ($null -ne $package.downloadUrl -and -not [string]::IsNullOrWhiteSpace([string]$package.downloadUrl)) {
            $noticeLines.Add("- Package URL: $([string]$package.downloadUrl)")
        }
        $licenseFiles = @($licenseRecordsByPackage[$packageId])
        if ($licenseFiles.Count -eq 0) {
            $noticeLines.Add("- Included license files: none supplied by the installed package at its standard license path")
        }
        else {
            $noticeLines.Add("- Included license files:")
            foreach ($licenseFile in $licenseFiles) {
                $noticeLines.Add("  - ``$([string]$licenseFile.path)``")
            }
        }
    }
    $noticesPath = Join-Path $evidence "THIRD-PARTY-NOTICES.md"
    Write-Utf8NoBom $noticesPath ([string]::Join("`n", $noticeLines) + "`n")

    $manifestHash = Get-LowerHash $manifestPath "SHA256"
    $createdAtUtc = [DateTime]::UtcNow.ToString(
        "yyyy-MM-ddTHH:mm:ssZ",
        [System.Globalization.CultureInfo]::InvariantCulture
    )
    $provenancePackages = @(
        foreach ($package in $packages) {
            [ordered]@{
                id = [string]$package.id
                name = [string]$package.name
                version = [string]$package.version
                architecture = [string]$package.architecture
                repository = [string]$package.repository
                downloadUrl = if ($null -eq $package.downloadUrl) { $null } else { [string]$package.downloadUrl }
                archiveSha256 = if ($null -eq $package.archiveSha256) { $null } else { [string]$package.archiveSha256 }
                signature = if ($null -eq $package.signature) { $null } else { [string]$package.signature }
                licenses = @($package.licenses | ForEach-Object { [string]$_ })
            }
        }
    )
    $provenance = [ordered]@{
        schemaVersion = 1
        createdAtUtc = $createdAtUtc
        source = [ordered]@{
            kind = [string]$source.kind
            supported = $supported
            repository = [string]$source.repository
            verification = [ordered]@{
                status = [string]$source.verification.status
                method = [string]$source.verification.method
                keyringPackage = if ($null -eq $source.verification.keyringPackage) { $null } else { [string]$source.verification.keyringPackage }
                keyringVersion = if ($null -eq $source.verification.keyringVersion) { $null } else { [string]$source.verification.keyringVersion }
            }
        }
        manifest = [ordered]@{
            path = "backend-manifest.tsv"
            sha256 = $manifestHash
        }
        packages = @($provenancePackages)
        files = @($payloadRecords)
    }
    $provenancePath = Join-Path $evidence "backend-provenance.json"
    Write-JsonDocument $provenancePath $provenance
    $provenanceHash = Get-LowerHash $provenancePath "SHA256"

    $spdxPackages = @()
    $spdxFiles = @()
    $relationships = @()
    for ($packageIndex = 0; $packageIndex -lt $packages.Count; $packageIndex++) {
        $package = $packages[$packageIndex]
        $packageId = [string]$package.id
        $spdxPackageId = "SPDXRef-Package-$($packageIndex + 1)"
        $ownedFiles = @($payloadFiles | Where-Object { $ownerByPath[[string]$_.path] -eq $packageId })
        $downloadLocation = if ($null -eq $package.downloadUrl -or [string]::IsNullOrWhiteSpace([string]$package.downloadUrl)) {
            "NOASSERTION"
        }
        else {
            [string]$package.downloadUrl
        }
        $spdxPackages += [ordered]@{
            SPDXID = $spdxPackageId
            name = [string]$package.name
            versionInfo = [string]$package.version
            downloadLocation = $downloadLocation
            filesAnalyzed = $true
            packageVerificationCode = [ordered]@{
                packageVerificationCodeValue = Get-SpdxPackageVerificationCode $ownedFiles
            }
            licenseConcluded = "NOASSERTION"
            licenseInfoFromFiles = @("NOASSERTION")
            licenseDeclared = "NOASSERTION"
            licenseComments = "Package-manager/source license metadata: " + [string]::Join(", ", @($package.licenses))
            copyrightText = "NOASSERTION"
        }
        $relationships += [ordered]@{
            spdxElementId = "SPDXRef-DOCUMENT"
            relationshipType = "DESCRIBES"
            relatedSpdxElement = $spdxPackageId
        }
    }
    for ($fileIndex = 0; $fileIndex -lt $payloadFiles.Count; $fileIndex++) {
        $file = $payloadFiles[$fileIndex]
        $spdxFileId = "SPDXRef-File-$($fileIndex + 1)"
        $spdxFiles += [ordered]@{
            SPDXID = $spdxFileId
            fileName = "./$([string]$file.path)"
            checksums = @([ordered]@{
                algorithm = "SHA256"
                checksumValue = [string]$file.sha256
            })
            fileTypes = @("BINARY")
            licenseConcluded = "NOASSERTION"
            licenseInfoInFiles = @("NOASSERTION")
            copyrightText = "NOASSERTION"
        }
        $owner = [string]$ownerByPath[[string]$file.path]
        $ownerIndex = [array]::IndexOf(@($packages | ForEach-Object { [string]$_.id }), $owner)
        if ($ownerIndex -lt 0) {
            throw "Internal error: SPDX owner was not found for $($file.path)"
        }
        $relationships += [ordered]@{
            spdxElementId = "SPDXRef-Package-$($ownerIndex + 1)"
            relationshipType = "CONTAINS"
            relatedSpdxElement = $spdxFileId
        }
    }
    $spdx = [ordered]@{
        spdxVersion = "SPDX-2.3"
        dataLicense = "CC0-1.0"
        SPDXID = "SPDXRef-DOCUMENT"
        name = "iroha-zip backend"
        documentNamespace = "https://github.com/hjosugi/iroha-zip/backend-evidence/${manifestHash}-${provenanceHash}"
        creationInfo = [ordered]@{
            created = $createdAtUtc
            creators = @("Tool: iroha-zip-backend-evidence")
        }
        packages = @($spdxPackages)
        files = @($spdxFiles)
        relationships = @($relationships)
    }
    Write-JsonDocument (Join-Path $evidence "backend.spdx.json") $spdx

    $inventoryPackages = @(
        foreach ($package in $packages) {
            [ordered]@{
                id = [string]$package.id
                name = [string]$package.name
                version = [string]$package.version
                licenses = @($package.licenses | ForEach-Object { [string]$_ })
                licenseFiles = @($licenseRecordsByPackage[[string]$package.id])
            }
        }
    )
    $inventory = [ordered]@{
        schemaVersion = 1
        notice = [ordered]@{
            path = "THIRD-PARTY-NOTICES.md"
            sha256 = Get-LowerHash $noticesPath "SHA256"
        }
        packages = @($inventoryPackages)
        files = @($payloadRecords)
    }
    Write-JsonDocument (Join-Path $evidence "backend-license-inventory.json") $inventory
    Assert-GeneratedEvidenceBounds $evidence
}
