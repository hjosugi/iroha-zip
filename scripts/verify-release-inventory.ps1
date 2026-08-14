[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$ReleaseJson,

    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string[]]$AssetFiles,

    [Parameter(Mandatory = $true)]
    [ValidateSet("Draft", "PublishedImmutable")]
    [string]$ExpectedState
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$release = Get-Content -LiteralPath $ReleaseJson -Raw -Encoding utf8 | ConvertFrom-Json
$assets = @($release.assets)

switch ($ExpectedState) {
    "Draft" {
        if (-not $release.isDraft -or $release.isPrerelease -or $release.isImmutable) {
            throw "Expected a mutable, non-prerelease draft release."
        }
    }
    "PublishedImmutable" {
        if ($release.isDraft -or $release.isPrerelease -or -not $release.isImmutable) {
            throw "Expected a published, non-prerelease immutable release."
        }
    }
}

$expected = [Collections.Generic.Dictionary[string, object]]::new(
    [StringComparer]::Ordinal
)
foreach ($assetFile in $AssetFiles) {
    $item = Get-Item -LiteralPath $assetFile
    if (-not $item.PSIsContainer -and $item.LinkType) {
        throw "Release asset must not be a link: $assetFile"
    }
    if ($item.PSIsContainer) {
        throw "Release asset must be a regular file: $assetFile"
    }

    $name = $item.Name
    if ($expected.ContainsKey($name)) {
        throw "Duplicate local release asset name: $name"
    }
    $digest = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $expected.Add($name, [pscustomobject]@{
        Size = [int64]$item.Length
        Digest = "sha256:$digest"
    })
}

if ($assets.Count -ne $expected.Count) {
    throw "Expected $($expected.Count) release assets; found $($assets.Count)."
}

$seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach ($asset in $assets) {
    $name = [string]$asset.name
    if (-not $expected.ContainsKey($name)) {
        throw "Unexpected release asset: $name"
    }
    if (-not $seen.Add($name)) {
        throw "Duplicate remote release asset: $name"
    }
    if ([string]$asset.state -cne "uploaded") {
        throw "Release asset is not uploaded: $name ($($asset.state))"
    }

    $local = $expected[$name]
    if ([int64]$asset.size -ne $local.Size) {
        throw "Release asset size mismatch: $name"
    }
    if ([string]$asset.digest -cne $local.Digest) {
        throw "Release asset digest mismatch: $name"
    }
}

Write-Host "Verified $($assets.Count) $ExpectedState release assets by exact name, size, and SHA-256."
