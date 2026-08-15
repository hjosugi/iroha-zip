[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^v[0-9]+\.[0-9]+\.[0-9]+$')]
    [string]$Tag,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')]
    [string]$Repository,

    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string[]]$AssetFiles,

    [ValidateRange(1, 60)]
    [int]$MaxAttempts = 24,

    [ValidateRange(1, 10000)]
    [int]$RetryDelayMilliseconds = 5000,

    [ValidateNotNullOrEmpty()]
    [string]$GitHubCli = "gh",

    [scriptblock]$CommandInvoker
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Invoke-GitHubCli {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$InvocationArguments
    )

    if ($null -ne $CommandInvoker) {
        $mockResult = & $CommandInvoker $InvocationArguments
        if ($null -eq $mockResult -or
            $null -eq $mockResult.PSObject.Properties["ExitCode"] -or
            $null -eq $mockResult.PSObject.Properties["Output"]) {
            throw "The test command invoker returned an invalid result."
        }
        return [pscustomobject]@{
            ExitCode = [int]$mockResult.ExitCode
            Output = @($mockResult.Output)
        }
    }

    $output = @(& $GitHubCli @InvocationArguments 2>&1)
    return [pscustomobject]@{
        ExitCode = [int]$LASTEXITCODE
        Output = $output
    }
}

function Format-FailureOutput {
    param(
        [object[]]$Output
    )

    $message = (($Output | ForEach-Object { [string]$_ }) -join [Environment]::NewLine).Trim()
    if ([string]::IsNullOrWhiteSpace($message)) {
        return "GitHub CLI returned no diagnostic output."
    }
    return $message
}

if ($AssetFiles.Count -ne 11) {
    throw "Expected exactly 11 release assets; found $($AssetFiles.Count)."
}

$seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$assets = @()
foreach ($assetFile in $AssetFiles) {
    $item = Get-Item -LiteralPath $assetFile
    if ($item.PSIsContainer -or $item.LinkType) {
        throw "Release asset must be a regular non-link file: $assetFile"
    }
    if (-not $seen.Add($item.Name)) {
        throw "Duplicate local release asset name: $($item.Name)"
    }
    $assets += $item
}
$assets = @($assets | Sort-Object Name)

$lastFailure = "GitHub CLI verification was not attempted."
foreach ($attempt in 1..$MaxAttempts) {
    $releaseResult = Invoke-GitHubCli -InvocationArguments @(
        "release", "verify", $Tag, "--repo", $Repository
    )
    if ($releaseResult.ExitCode -ne 0) {
        $lastFailure = Format-FailureOutput -Output $releaseResult.Output
    }
    else {
        $allAssetsVerified = $true
        foreach ($asset in $assets) {
            $assetResult = Invoke-GitHubCli -InvocationArguments @(
                "release", "verify-asset", $Tag, $asset.FullName, "--repo", $Repository
            )
            if ($assetResult.ExitCode -ne 0) {
                $allAssetsVerified = $false
                $lastFailure = "asset verification failed for $($asset.Name): " +
                    (Format-FailureOutput -Output $assetResult.Output)
                break
            }
        }
        if ($allAssetsVerified) {
            Write-Host "Verified the GitHub release attestation and all 11 downloaded assets for $Tag."
            return
        }
    }

    if ($attempt -lt $MaxAttempts) {
        Write-Host "Release attestation is not fully available (attempt $attempt/$MaxAttempts); retrying."
        Start-Sleep -Milliseconds $RetryDelayMilliseconds
    }
}

$attemptLabel = if ($MaxAttempts -eq 1) { "attempt" } else { "attempts" }
throw "GitHub release attestation verification failed after $MaxAttempts $attemptLabel for ${Tag}: $lastFailure"
