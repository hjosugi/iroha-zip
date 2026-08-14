[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Executable,

    [Parameter(Mandatory = $true)]
    [string]$ShellExecutable,

    [Parameter(Mandatory = $true)]
    [string]$BackendDirectory,

    [Parameter(Mandatory = $true)]
    [string]$EvidenceOutput
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-Leaf([string]$Path, [string]$Description) {
    $resolved = Resolve-Path -LiteralPath $Path -ErrorAction Stop
    if (-not (Test-Path -LiteralPath $resolved.Path -PathType Leaf)) {
        throw "$Description is not a regular file: $Path"
    }
    return $resolved.Path
}

function Resolve-Directory([string]$Path, [string]$Description) {
    $resolved = Resolve-Path -LiteralPath $Path -ErrorAction Stop
    if (-not (Test-Path -LiteralPath $resolved.Path -PathType Container)) {
        throw "$Description is not a directory: $Path"
    }
    return $resolved.Path
}

function Invoke-TestProcess {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [string[]]$Arguments = @(),

        [int[]]$ExpectedExitCodes = @(0),

        [int]$TimeoutSeconds = 120,

        [hashtable]$Environment = @{}
    )

    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $FilePath
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    foreach ($argument in $Arguments) {
        $start.ArgumentList.Add($argument)
    }
    foreach ($entry in $Environment.GetEnumerator()) {
        $start.Environment[[string]$entry.Key] = [string]$entry.Value
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $start
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        if (-not $process.Start()) {
            throw "Process did not start: $FilePath"
        }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            $process.Kill($true)
            $process.WaitForExit()
            throw "Process exceeded ${TimeoutSeconds}s: $FilePath $($Arguments -join ' ')"
        }
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        if ($ExpectedExitCodes -notcontains $process.ExitCode) {
            throw "Unexpected exit code $($process.ExitCode) from $FilePath. stderr=$($stderr.Trim()) stdout=$($stdout.Trim())"
        }
        return [pscustomobject][ordered]@{
            exitCode = $process.ExitCode
            elapsedMilliseconds = $stopwatch.ElapsedMilliseconds
            stdout = $stdout
            stderr = $stderr
        }
    }
    finally {
        $stopwatch.Stop()
        $process.Dispose()
    }
}

function Get-StringSha256([string]$Text) {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    return [System.Convert]::ToHexString(
        [System.Security.Cryptography.SHA256]::HashData($bytes)
    ).ToLowerInvariant()
}

function Get-TreeInventory([string]$Root) {
    $rootPath = (Resolve-Path -LiteralPath $Root).Path
    $records = @(
        Get-ChildItem -LiteralPath $rootPath -Force -Recurse |
            ForEach-Object {
                $relative = [System.IO.Path]::GetRelativePath($rootPath, $_.FullName).Replace('\', '/')
                $relative = $relative.Replace([char]92, [char]47)
                if ($_.PSIsContainer) {
                    [pscustomobject][ordered]@{
                        path = $relative
                        kind = "directory"
                        bytes = 0
                        sha256 = $null
                    }
                }
                else {
                    [pscustomobject][ordered]@{
                        path = $relative
                        kind = "file"
                        bytes = $_.Length
                        sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
                    }
                }
            } |
            Sort-Object -Property path, kind
    )
    return $records
}

function Compare-Tree([string]$Expected, [string]$Actual) {
    $expectedInventory = @(Get-TreeInventory $Expected)
    $actualInventory = @(Get-TreeInventory $Actual)
    $expectedJson = ConvertTo-Json -InputObject $expectedInventory -Depth 5 -Compress
    $actualJson = ConvertTo-Json -InputObject $actualInventory -Depth 5 -Compress
    if ($expectedJson -cne $actualJson) {
        throw "Extracted tree differs from the source tree.`nExpected: $expectedJson`nActual: $actualJson"
    }
    $files = @($expectedInventory | Where-Object { $_.kind -eq "file" })
    return [pscustomobject][ordered]@{
        entryCount = $expectedInventory.Count
        fileCount = $files.Count
        totalBytes = ($files | Measure-Object -Property bytes -Sum).Sum
        manifestSha256 = Get-StringSha256 $expectedJson
    }
}

function Write-TestConfig(
    [string]$Path,
    [string]$Backend,
    [bool]$OpenAfterDoubleClick,
    [string]$Isolation = "appcontainer"
) {
    if ($Backend.Contains("'")) {
        throw "The E2E backend path cannot contain a single quote: $Backend"
    }
    if ($Isolation -notin @("appcontainer", "lpac")) {
        throw "Unsupported E2E isolation mode: $Isolation"
    }
    $literalBackend = $Backend.Replace("'", "''")
    $openValue = if ($OpenAfterDoubleClick) { "true" } else { "false" }
    $text = @"
[backend]
directory = '$literalBackend'

[sandbox]
timeout_seconds = 30
memory_limit_mib = 768
isolation = "$Isolation"

[limits]
max_archive_bytes = 134217728
max_files = 1000
max_directories = 500
max_total_bytes = 268435456
max_single_file_bytes = 67108864
max_depth = 64
max_path_bytes = 4096

[behavior]
preserve_mark_of_the_web = true
attachment_handoff = "disabled"
open_after_double_click = $openValue
default_filename_encoding = "auto"
"@
    [System.IO.Directory]::CreateDirectory((Split-Path -Parent $Path)) | Out-Null
    [System.IO.File]::WriteAllText($Path, $text, [System.Text.UTF8Encoding]::new($false))
}

function Assert-IsolationEvidence(
    [object]$Evidence,
    [bool]$ExpectLpac,
    [string]$ExpectedProbeSha256
) {
    $expectedMode = if ($ExpectLpac) { "lpac" } else { "appcontainer" }
    if ($Evidence.schemaVersion -ne 3 -or
        $Evidence.requestedMode -cne $expectedMode -or
        -not $Evidence.token.isAppContainer -or
        [bool]$Evidence.token.isLessPrivilegedAppContainer -ne $ExpectLpac -or
        $Evidence.token.capabilityCount -ne 0 -or
        -not $Evidence.network.denied -or
        -not $Evidence.timeout.rejected -or
        -not $Evidence.memory.rejected -or
        [string]::IsNullOrWhiteSpace($Evidence.processTemp.tempEnvironment) -or
        [string]::IsNullOrWhiteSpace($Evidence.processTemp.tmpEnvironment) -or
        [string]::IsNullOrWhiteSpace($Evidence.processTemp.resolvedPath) -or
        -not $Evidence.processTemp.rngSucceeded -or
        -not $Evidence.processTemp.deleteOnCloseSucceeded -or
        -not $Evidence.stagingWriteSeal.aclApplied) {
        throw "Isolation evidence did not satisfy the zero-capability $expectedMode contract."
    }

    $expectedReadablePaths = @(
        "root-file",
        "nested-file",
        "parent-directory",
        "root-directory",
        "nested-directory",
        "current-directory"
    )
    $expectedDeniedOperations = @(
        "overwrite-existing-file",
        "append-existing-file",
        "create-root-file",
        "create-parent-file",
        "create-root-directory",
        "overwrite-nested-file",
        "create-nested-file",
        "rename-file",
        "delete-file",
        "change-file-attributes",
        "open-dacl-for-write",
        "open-owner-for-write"
    )
    $actualReadableJson = ConvertTo-Json `
        -InputObject @($Evidence.stagingWriteSeal.readablePaths) -Compress
    $expectedReadableJson = ConvertTo-Json -InputObject $expectedReadablePaths -Compress
    $actualDeniedJson = ConvertTo-Json `
        -InputObject @($Evidence.stagingWriteSeal.deniedOperations) -Compress
    $expectedDeniedJson = ConvertTo-Json -InputObject $expectedDeniedOperations -Compress
    if ($actualReadableJson -cne $expectedReadableJson -or
        $actualDeniedJson -cne $expectedDeniedJson) {
        throw "Staging-source ACL evidence did not match the exact $expectedMode read/write contract."
    }

    $cleanupRecords = @($Evidence.cleanup)
    if ($cleanupRecords.Count -ne 5 -or
        @($cleanupRecords | Where-Object {
            -not $_.profileDeleteSucceeded -or -not $_.temporaryRootRemoved
        }).Count -ne 0) {
        throw "One or more $expectedMode isolation probes left a profile or temporary root behind."
    }
    if ($Evidence.probeSha256 -cne $ExpectedProbeSha256) {
        throw "$expectedMode isolation probe bytes differ from the tested iroha-zip executable."
    }
}

$builtExecutablePath = Resolve-Leaf $Executable "iroha-zip executable"
$builtShellExecutablePath = Resolve-Leaf $ShellExecutable "iroha-zip shell executable"
$backendPath = Resolve-Directory $BackendDirectory "verified backend"
$evidencePath = [System.IO.Path]::GetFullPath($EvidenceOutput)
$evidenceParent = Split-Path -Parent $evidencePath
[System.IO.Directory]::CreateDirectory($evidenceParent) | Out-Null

$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) `
    ("iroha-zip-e2e-日本語-" + [Guid]::NewGuid().ToString("N"))
[System.IO.Directory]::CreateDirectory($testRoot) | Out-Null
$backendFixtureParent = if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
    [System.IO.Path]::GetTempPath()
}
else {
    $env:RUNNER_TEMP
}
$backendFixtureRoot = Join-Path $backendFixtureParent `
    ("iroha-zip-e2e-filters-" + [Guid]::NewGuid().ToString("N"))
[System.IO.Directory]::CreateDirectory($backendFixtureRoot) | Out-Null
$runtimeRoot = Join-Path $testRoot "runtime"
[System.IO.Directory]::CreateDirectory($runtimeRoot) | Out-Null
$executablePath = Join-Path $runtimeRoot "iroha-zip.exe"
$shellExecutablePath = Join-Path $runtimeRoot "iroha-zip-shell.exe"
Copy-Item -LiteralPath $builtExecutablePath -Destination $executablePath
Copy-Item -LiteralPath $builtShellExecutablePath -Destination $shellExecutablePath
$executablePath = Resolve-Leaf $executablePath "independent iroha-zip executable"
$shellExecutablePath = Resolve-Leaf $shellExecutablePath "independent iroha-zip shell executable"
$builtExecutableHash = (Get-FileHash -LiteralPath $builtExecutablePath -Algorithm SHA256).Hash
$executableHash = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash
$builtShellHash = (Get-FileHash -LiteralPath $builtShellExecutablePath -Algorithm SHA256).Hash
$shellHash = (Get-FileHash -LiteralPath $shellExecutablePath -Algorithm SHA256).Hash
if ($builtExecutableHash -cne $executableHash -or $builtShellHash -cne $shellHash) {
    throw "Independent E2E executable copies do not match the Cargo build artifacts."
}
$failure = $null
$report = [ordered]@{
    schemaVersion = 2
    status = "running"
    generatedAtUtc = [DateTime]::UtcNow.ToString("o")
    runner = [ordered]@{
        osDescription = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
        osArchitecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
        processArchitecture = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString()
        imageOs = $env:ImageOS
        imageVersion = $env:ImageVersion
        runnerName = $env:RUNNER_NAME
    }
    binaries = [ordered]@{
        irohaZipSha256 = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash.ToLowerInvariant()
        shellSha256 = (Get-FileHash -LiteralPath $shellExecutablePath -Algorithm SHA256).Hash.ToLowerInvariant()
        backendManifestSha256 = (Get-FileHash -LiteralPath (Join-Path $backendPath "backend-manifest.tsv") -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    sourceTree = $null
    isolation = $null
    doctor = $null
    lpac = [ordered]@{
        isolation = $null
        doctor = $null
    }
    formats = @()
    readFixtures = @()
    invalidArchive = $null
    shell = $null
    cleanup = [ordered]@{
        temporaryRootRemoved = $false
    }
    failure = $null
}

try {
    $configPath = Join-Path $testRoot "設定.toml"
    Write-TestConfig $configPath $backendPath $false
    $lpacConfigPath = Join-Path $testRoot "設定-lpac.toml"
    Write-TestConfig $lpacConfigPath $backendPath $false "lpac"

    $sourceRoot = Join-Path $testRoot "入力-日本語"
    [System.IO.Directory]::CreateDirectory($sourceRoot) | Out-Null
    [System.IO.File]::WriteAllText(
        (Join-Path $sourceRoot "こんにちは.txt"),
        "いろは ZIP Windows E2E`n",
        [System.Text.UTF8Encoding]::new($false)
    )
    $binary = [byte[]]::new(8192)
    for ($index = 0; $index -lt $binary.Length; $index++) {
        $binary[$index] = [byte](($index * 17 + 31) % 256)
    }
    $nested = Join-Path $sourceRoot "nested"
    [System.IO.Directory]::CreateDirectory($nested) | Out-Null
    [System.IO.File]::WriteAllBytes((Join-Path $nested "deterministic.bin"), $binary)
    [System.IO.Directory]::CreateDirectory((Join-Path $sourceRoot "空のフォルダー")) | Out-Null
    $longRoot = $sourceRoot
    foreach ($segment in 1..7) {
        $longRoot = Join-Path $longRoot ("長い階層-$segment-" + ("x" * 36))
    }
    [System.IO.Directory]::CreateDirectory($longRoot) | Out-Null
    [System.IO.File]::WriteAllText(
        (Join-Path $longRoot "終端-日本語.txt"),
        "long-path-content`n",
        [System.Text.UTF8Encoding]::new($false)
    )
    $sourceInventory = @(Get-TreeInventory $sourceRoot)
    $longestRelativePath = ($sourceInventory.path | ForEach-Object { $_.Length } | Measure-Object -Maximum).Maximum
    if ($longestRelativePath -le 260) {
        throw "Long-path fixture is not longer than 260 characters: $longestRelativePath"
    }
    $sourceTree = Compare-Tree $sourceRoot $sourceRoot
    $report.sourceTree = [ordered]@{
        entryCount = $sourceTree.entryCount
        fileCount = $sourceTree.fileCount
        totalBytes = $sourceTree.totalBytes
        manifestSha256 = $sourceTree.manifestSha256
        longestRelativePathCharacters = $longestRelativePath
        includesJapaneseNames = $true
        includesEmptyDirectory = $true
    }

    $backendValidation = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "verify-backend-evidence", $backendPath, "--require-supported"
    )

    $isolationRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "--config", $configPath, "isolation-report"
    )
    $isolation = $isolationRun.stdout | ConvertFrom-Json -Depth 20
    Assert-IsolationEvidence $isolation $false $report.binaries.irohaZipSha256
    $report.isolation = $isolation

    $doctorRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "--config", $configPath, "doctor"
    )
    if ($doctorRun.stdout -notmatch 'requested=AppContainer' -or
        $doctorRun.stdout -notmatch 'AppContainer=true' -or
        $doctorRun.stdout -notmatch 'LPAC=false' -or
        $doctorRun.stdout -notmatch 'capabilities=0') {
        throw "Doctor output did not expose measured AppContainer token evidence."
    }
    $report.doctor = [ordered]@{
        elapsedMilliseconds = $doctorRun.elapsedMilliseconds
        outputSha256 = Get-StringSha256 $doctorRun.stdout
        backendEvidenceValidationMilliseconds = $backendValidation.elapsedMilliseconds
    }

    $lpacIsolationRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "--config", $lpacConfigPath, "isolation-report"
    )
    $lpacIsolation = $lpacIsolationRun.stdout | ConvertFrom-Json -Depth 20
    Assert-IsolationEvidence $lpacIsolation $true $report.binaries.irohaZipSha256
    $report.lpac.isolation = $lpacIsolation

    $lpacDoctorRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "--config", $lpacConfigPath, "doctor"
    )
    if ($lpacDoctorRun.stdout -notmatch 'requested=LPAC' -or
        $lpacDoctorRun.stdout -notmatch 'AppContainer=true' -or
        $lpacDoctorRun.stdout -notmatch 'LPAC=true' -or
        $lpacDoctorRun.stdout -notmatch 'capabilities=0') {
        throw "Doctor output did not expose measured zero-capability LPAC token evidence."
    }
    $report.lpac.doctor = [ordered]@{
        elapsedMilliseconds = $lpacDoctorRun.elapsedMilliseconds
        outputSha256 = Get-StringSha256 $lpacDoctorRun.stdout
    }

    $archivesRoot = Join-Path $testRoot "archives"
    $extractedRoot = Join-Path $testRoot "extracted"
    [System.IO.Directory]::CreateDirectory($archivesRoot) | Out-Null
    [System.IO.Directory]::CreateDirectory($extractedRoot) | Out-Null
    $formats = @(
        [pscustomobject]@{ cli = "zip"; extension = "zip" },
        [pscustomobject]@{ cli = "seven-zip"; extension = "7z" },
        [pscustomobject]@{ cli = "tar"; extension = "tar" },
        [pscustomobject]@{ cli = "tar-gz"; extension = "tar.gz" }
    )
    $createdArchives = @{}
    foreach ($format in $formats) {
        $archive = Join-Path $archivesRoot ("検証-$($format.cli).$($format.extension)")
        $destination = Join-Path $extractedRoot $format.cli
        $createRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "create", $format.cli, $sourceRoot, $archive
        )
        $previewRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "preview", $archive
        )
        $extractRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "extract", $archive, "--output", $destination
        )
        $tree = Compare-Tree $sourceRoot $destination
        $archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
        $report.formats += [pscustomobject][ordered]@{
            format = $format.cli
            extension = $format.extension
            archiveBytes = (Get-Item -LiteralPath $archive).Length
            archiveSha256 = $archiveHash
            treeManifestSha256 = $tree.manifestSha256
            createMilliseconds = $createRun.elapsedMilliseconds
            previewMilliseconds = $previewRun.elapsedMilliseconds
            extractMilliseconds = $extractRun.elapsedMilliseconds
            previewOutputSha256 = Get-StringSha256 $previewRun.stdout
            explicitCleanupRequired = $true
        }
        $createdArchives[$format.cli] = $archive
    }

    $backendExecutable = Resolve-Leaf (Join-Path $backendPath "bsdtar.exe") "verified bsdtar"
    $filterSourceRoot = Join-Path $backendFixtureRoot "source"
    $filterArchivesRoot = Join-Path $backendFixtureRoot "archives"
    [System.IO.Directory]::CreateDirectory((Join-Path $filterSourceRoot "nested")) | Out-Null
    [System.IO.Directory]::CreateDirectory((Join-Path $filterSourceRoot "empty")) | Out-Null
    [System.IO.Directory]::CreateDirectory($filterArchivesRoot) | Out-Null
    [System.IO.File]::WriteAllText(
        (Join-Path $filterSourceRoot "fixture.txt"),
        "controlled read-filter fixture`n",
        [System.Text.UTF8Encoding]::new($false)
    )
    [System.IO.File]::WriteAllBytes(
        (Join-Path $filterSourceRoot "nested\bytes.bin"),
        [byte[]](0, 1, 2, 127, 128, 254, 255)
    )
    $filterSourceTree = Compare-Tree $filterSourceRoot $filterSourceRoot
    $readFixtures = @(
        [pscustomobject]@{ name = "tar-bz2"; filter = "-j"; extension = "tar.bz2" },
        [pscustomobject]@{ name = "tar-xz"; filter = "-J"; extension = "tar.xz" },
        [pscustomobject]@{ name = "tar-zstd"; filter = "--zstd"; extension = "tar.zst" },
        [pscustomobject]@{ name = "tar-compress"; filter = "-Z"; extension = "tar.Z" }
    )
    foreach ($fixture in $readFixtures) {
        $archive = Join-Path $archivesRoot ("読取-$($fixture.name).$($fixture.extension)")
        $generatedArchive = Join-Path $filterArchivesRoot ("$($fixture.name).$($fixture.extension)")
        $destination = Join-Path $extractedRoot $fixture.name
        $generateRun = Invoke-TestProcess -FilePath $backendExecutable -Arguments @(
            "-c", "--format=pax", $fixture.filter,
            "--no-xattrs", "--no-acls", "--no-fflags",
            "-f", $generatedArchive, "-C", $filterSourceRoot,
            "fixture.txt", "nested", "empty"
        )
        Copy-Item -LiteralPath $generatedArchive -Destination $archive
        $previewRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "preview", $archive
        )
        $extractRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "extract", $archive, "--output", $destination
        )
        $tree = Compare-Tree $filterSourceRoot $destination
        $report.readFixtures += [pscustomobject][ordered]@{
            format = $fixture.name
            extension = $fixture.extension
            archiveBytes = (Get-Item -LiteralPath $archive).Length
            archiveSha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
            controlledSourceManifestSha256 = $filterSourceTree.manifestSha256
            treeManifestSha256 = $tree.manifestSha256
            controlledFixtureGenerationMilliseconds = $generateRun.elapsedMilliseconds
            previewMilliseconds = $previewRun.elapsedMilliseconds
            extractMilliseconds = $extractRun.elapsedMilliseconds
            explicitCleanupRequired = $true
        }
    }

    $invalidArchive = Join-Path $archivesRoot "壊れた.zip"
    [System.IO.File]::WriteAllText(
        $invalidArchive,
        "this is deliberately not an archive",
        [System.Text.UTF8Encoding]::new($false)
    )
    $invalidDestination = Join-Path $extractedRoot "invalid-must-not-exist"
    $invalidRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "--config", $configPath, "extract", $invalidArchive, "--output", $invalidDestination
    ) -ExpectedExitCodes @(2)
    if (Test-Path -LiteralPath $invalidDestination) {
        throw "Invalid archive failure published a destination tree."
    }
    $report.invalidArchive = [ordered]@{
        rejected = $true
        exitCode = $invalidRun.exitCode
        destinationAbsent = $true
        explicitCleanupRequired = $true
        stderrSha256 = Get-StringSha256 $invalidRun.stderr
    }

    $shellRoot = Join-Path $testRoot "shell-日本語"
    $shellLocalAppData = Join-Path $shellRoot "local-app-data"
    [System.IO.Directory]::CreateDirectory($shellRoot) | Out-Null
    $shellArchive = Join-Path $shellRoot "シェル検証.zip"
    Copy-Item -LiteralPath $createdArchives["zip"] -Destination $shellArchive
    $shellConfig = Join-Path $shellLocalAppData "iroha-zip\config.toml"
    Write-TestConfig $shellConfig $backendPath $false
    $shellRun = Invoke-TestProcess -FilePath $shellExecutablePath `
        -Arguments @($shellArchive) `
        -Environment @{ LOCALAPPDATA = $shellLocalAppData }
    $shellDestination = Join-Path $shellRoot "シェル検証"
    $shellTree = Compare-Tree $sourceRoot $shellDestination
    $report.shell = [ordered]@{
        elapsedMilliseconds = $shellRun.elapsedMilliseconds
        destinationCreated = $true
        treeManifestSha256 = $shellTree.manifestSha256
        defaultConfigPathUsed = $true
        explicitCleanupRequired = $true
    }

    $report.status = "passed"
}
catch {
    $failure = $_
    $report.status = "failed"
    $report.failure = $_.Exception.Message
}
finally {
    try {
        foreach ($ownedRoot in @($testRoot, $backendFixtureRoot)) {
            if (Test-Path -LiteralPath $ownedRoot) {
                Remove-Item -LiteralPath $ownedRoot -Recurse -Force
            }
        }
        $report.cleanup.temporaryRootRemoved = (
            -not (Test-Path -LiteralPath $testRoot) -and
            -not (Test-Path -LiteralPath $backendFixtureRoot)
        )
    }
    catch {
        $report.cleanup.temporaryRootRemoved = $false
        if ($null -eq $failure) {
            $failure = $_
            $report.status = "failed"
            $report.failure = $_.Exception.Message
        }
        else {
            $report.failure = "$($report.failure); cleanup failed: $($_.Exception.Message)"
        }
    }
    $json = ConvertTo-Json -InputObject $report -Depth 30
    [System.IO.File]::WriteAllText(
        $evidencePath,
        "$json`n",
        [System.Text.UTF8Encoding]::new($false)
    )
}

if ($null -ne $failure) {
    throw "Windows E2E failed; machine-readable evidence: $evidencePath`n$($report.failure)"
}
Write-Host "Windows E2E passed: $evidencePath"
