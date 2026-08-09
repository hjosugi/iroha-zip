[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string[]]$Files,

    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string]$ExpectedPublisher,

    [string]$EvidencePath,

    [string]$ExpectedEvidencePath,

    [switch]$RequireTimestamp
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ExpectedBinaryNames = @(
    "iroha-zip.exe",
    "iroha-zip-settings.exe",
    "iroha-zip-shell.exe"
)
$CodeSigningOid = "1.3.6.1.5.5.7.3.3"

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw "Authenticode verification requires Windows WinVerifyTrust APIs."
}
if (-not (Get-Command Get-AuthenticodeSignature -ErrorAction SilentlyContinue)) {
    throw "Get-AuthenticodeSignature is unavailable."
}
if ([string]::IsNullOrWhiteSpace($ExpectedPublisher)) {
    throw "ExpectedPublisher must be the exact non-empty signer certificate subject."
}
if ($Files.Count -ne $ExpectedBinaryNames.Count) {
    throw "Expected exactly three release executables; found $($Files.Count)."
}

$resolvedFiles = @($Files | ForEach-Object {
    $item = Get-Item -LiteralPath $_ -ErrorAction Stop
    if (-not $item.PSIsContainer -and $item.Extension -ceq ".exe") {
        $item
    }
    else {
        throw "Release signature input is not an .exe file: $_"
    }
})

$actualNames = @($resolvedFiles | ForEach-Object { $_.Name } | Sort-Object)
$expectedNames = @($ExpectedBinaryNames | Sort-Object)
if ([string]::Join("`n", $actualNames) -cne [string]::Join("`n", $expectedNames)) {
    throw "Release signature inputs must be exactly: $([string]::Join(', ', $ExpectedBinaryNames))."
}
if (@($resolvedFiles.FullName | Sort-Object -Unique).Count -ne $resolvedFiles.Count) {
    throw "Release signature inputs contain duplicate paths."
}

$evidenceFiles = @()
foreach ($file in ($resolvedFiles | Sort-Object Name)) {
    $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "Authenticode validation failed for $($file.Name): $($signature.Status) ($($signature.StatusMessage))"
    }
    if ([string]$signature.SignatureType -cne "Authenticode") {
        throw "An embedded Authenticode signature is required for $($file.Name)."
    }

    $signer = $signature.SignerCertificate
    if ($null -eq $signer) {
        throw "Authenticode signer certificate is missing for $($file.Name)."
    }
    if (-not [string]::Equals(
        $signer.Subject,
        $ExpectedPublisher,
        [System.StringComparison]::Ordinal
    )) {
        throw "Unexpected publisher for $($file.Name): '$($signer.Subject)'."
    }

    $ekuOids = @()
    foreach ($extension in $signer.Extensions) {
        if ($extension.Oid.Value -eq "2.5.29.37") {
            if ($extension -isnot [System.Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]) {
                throw "Cannot decode the signer EKU extension for $($file.Name)."
            }
            foreach ($usage in $extension.EnhancedKeyUsages) {
                $ekuOids += $usage.Value
            }
        }
    }
    if ($ekuOids -notcontains $CodeSigningOid) {
        throw "Signer certificate lacks the Code Signing EKU for $($file.Name)."
    }

    $timestamp = $signature.TimeStamperCertificate
    if ($RequireTimestamp -and $null -eq $timestamp) {
        throw "RFC3161 timestamp certificate is missing for $($file.Name)."
    }

    $timestampEvidence = $null
    if ($null -ne $timestamp) {
        $timestampEvidence = [ordered]@{
            subject = $timestamp.Subject
            issuer = $timestamp.Issuer
            serial_number = $timestamp.SerialNumber
            thumbprint_sha1 = $timestamp.Thumbprint.ToLowerInvariant()
            not_before_utc = $timestamp.NotBefore.ToUniversalTime().ToString("O")
            not_after_utc = $timestamp.NotAfter.ToUniversalTime().ToString("O")
        }
    }

    $evidenceFiles += [ordered]@{
        name = $file.Name
        length = $file.Length
        sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        signature_type = [string]$signature.SignatureType
        status = [string]$signature.Status
        signer = [ordered]@{
            subject = $signer.Subject
            issuer = $signer.Issuer
            serial_number = $signer.SerialNumber
            thumbprint_sha1 = $signer.Thumbprint.ToLowerInvariant()
            not_before_utc = $signer.NotBefore.ToUniversalTime().ToString("O")
            not_after_utc = $signer.NotAfter.ToUniversalTime().ToString("O")
            enhanced_key_usage_oids = @($ekuOids | Sort-Object -Unique)
        }
        timestamp = $timestampEvidence
    }
}

$signerThumbprints = @($evidenceFiles | ForEach-Object {
    $_.signer.thumbprint_sha1
} | Sort-Object -Unique)
if ($signerThumbprints.Count -ne 1) {
    throw "All three executables must use the same release signing certificate."
}

$evidence = [ordered]@{
    schema_version = 1
    verification_api = "PowerShell Get-AuthenticodeSignature / Windows WinVerifyTrust"
    expected_publisher = $ExpectedPublisher
    timestamp_required = [bool]$RequireTimestamp
    files = $evidenceFiles
}
$json = $evidence | ConvertTo-Json -Depth 8
$serializedEvidence = "$json`n"

if (-not [string]::IsNullOrWhiteSpace($EvidencePath)) {
    $absoluteEvidencePath = [System.IO.Path]::GetFullPath($EvidencePath)
    $parent = Split-Path -Parent $absoluteEvidencePath
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        throw "Evidence output directory does not exist: $parent"
    }
    [System.IO.File]::WriteAllText(
        $absoluteEvidencePath,
        $serializedEvidence,
        [System.Text.UTF8Encoding]::new($false)
    )
    Write-Host "Signature evidence: $absoluteEvidencePath"
}

if (-not [string]::IsNullOrWhiteSpace($ExpectedEvidencePath)) {
    $expectedEvidence = Get-Item -LiteralPath $ExpectedEvidencePath -ErrorAction Stop
    if ($expectedEvidence.PSIsContainer -or $expectedEvidence.Length -gt 1MB) {
        throw "Expected signature evidence must be a file no larger than 1 MiB."
    }
    $expectedBytes = [System.IO.File]::ReadAllText($expectedEvidence.FullName)
    if ($expectedBytes -cne $serializedEvidence) {
        throw "Signature evidence does not match the verified executable inventory."
    }
    Write-Host "Matched signature evidence: $($expectedEvidence.FullName)"
}

Write-Host "Verified Authenticode publisher and timestamp on all three release executables."
