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
. (Join-Path $PSScriptRoot "..\tests\windows-fixture-tools.ps1")

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class IrohaZipPasswordAutomationNative {
    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetWindowTextW(IntPtr window, string text);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessageW(IntPtr window, uint message, UIntPtr wParam, IntPtr lParam);
}
"@

$ButtonClickMessage = 0x00F5
$PasswordDialogTitle = "Archive password / 書庫のパスワード"
$PasswordEditId = 100
$PasswordConfirmId = 1
$PasswordCancelId = 2

function Wait-Until {
    param(
        [scriptblock]$Condition,
        [int]$TimeoutSeconds = 30,
        [string]$Description = "the requested state"
    )
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        $value = & $Condition
        if ($null -ne $value -and $value -ne $false) { return $value }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Description."
}

function Find-PasswordControl {
    param(
        [System.Windows.Automation.AutomationElement]$Dialog,
        [int]$Id
    )
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::AutomationIdProperty,
        [string]$Id
    )
    return $Dialog.FindFirst(
        [System.Windows.Automation.TreeScope]::Descendants,
        $condition
    )
}

function Wait-ForPasswordDialog {
    param(
        [System.Diagnostics.Process]$Process,
        [int]$TimeoutSeconds = 60
    )
    return Wait-Until -TimeoutSeconds $TimeoutSeconds `
        -Description "the password dialog for process $($Process.Id)" `
        -Condition {
            $Process.Refresh()
            if ($Process.HasExited) {
                throw "Process $($Process.Id) exited before creating the password dialog (exit code $($Process.ExitCode))."
            }
            $processCondition = [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
                $Process.Id
            )
            $nameCondition = [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::NameProperty,
                $PasswordDialogTitle
            )
            $condition = [System.Windows.Automation.AndCondition]::new(
                $processCondition,
                $nameCondition
            )
            [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
                [System.Windows.Automation.TreeScope]::Children,
                $condition
            )
        }
}

function Invoke-PasswordTestProcess {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [string]$Password,

        [switch]$Cancel,

        [int[]]$ExpectedExitCodes = @(0),

        [int]$TimeoutSeconds = 120
    )

    if (-not $Cancel -and [string]::IsNullOrEmpty($Password)) {
        throw "A non-cancelled password UI run requires a password."
    }

    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $FilePath
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    foreach ($argument in $Arguments) {
        $start.ArgumentList.Add($argument)
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $start
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    $started = $false
    try {
        if (-not $process.Start()) {
            throw "Password test process did not start: $FilePath"
        }
        $started = $true
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        $dialog = Wait-ForPasswordDialog -Process $process
        $dialogName = $dialog.Current.Name
        if ($dialogName -cne $PasswordDialogTitle) {
            throw "Password dialog title is not the exact bilingual contract: $dialogName"
        }

        $edit = Find-PasswordControl -Dialog $dialog -Id $PasswordEditId
        if ($null -eq $edit -or
            $edit.Current.ControlType -ne [System.Windows.Automation.ControlType]::Edit -or
            -not $edit.Current.IsPassword -or
            -not $edit.Current.IsEnabled -or
            -not $edit.Current.IsKeyboardFocusable) {
            throw "Password dialog did not expose an enabled, focusable ES_PASSWORD edit control."
        }

        $buttonId = if ($Cancel) { $PasswordCancelId } else { $PasswordConfirmId }
        $button = Find-PasswordControl -Dialog $dialog -Id $buttonId
        if ($null -eq $button -or
            $button.Current.ControlType -ne [System.Windows.Automation.ControlType]::Button -or
            -not $button.Current.IsEnabled -or
            -not $button.Current.IsKeyboardFocusable -or
            [string]::IsNullOrWhiteSpace($button.Current.Name)) {
            throw "Password dialog did not expose an accessible button with ID $buttonId."
        }

        if (-not $Cancel) {
            $edit.SetFocus()
            if (-not [IrohaZipPasswordAutomationNative]::SetWindowTextW(
                [IntPtr]$edit.Current.NativeWindowHandle,
                $Password
            )) {
                throw "Cannot set the public E2E fixture password in the native password control."
            }
        }
        if (-not [IrohaZipPasswordAutomationNative]::PostMessageW(
            [IntPtr]$button.Current.NativeWindowHandle,
            $ButtonClickMessage,
            [UIntPtr]::Zero,
            [IntPtr]::Zero
        )) {
            throw "Cannot activate password dialog button ID $buttonId."
        }

        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            $process.Kill($true)
            $process.WaitForExit()
            throw "Password test process exceeded ${TimeoutSeconds}s: $FilePath $($Arguments -join ' ')"
        }
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        if (-not [string]::IsNullOrEmpty($Password) -and
            ($stdout.Contains($Password) -or $stderr.Contains($Password))) {
            throw "The public E2E fixture password reached process output."
        }
        if ($ExpectedExitCodes -notcontains $process.ExitCode) {
            throw "Unexpected exit code $($process.ExitCode) from password test process. stderr=$($stderr.Trim()) stdout=$($stdout.Trim())"
        }
        return [pscustomobject][ordered]@{
            exitCode = $process.ExitCode
            elapsedMilliseconds = $stopwatch.ElapsedMilliseconds
            stdout = $stdout
            stderr = $stderr
            dialogTitle = $dialogName
            passwordControlProtected = $true
            action = if ($Cancel) { "cancel" } else { "confirm" }
        }
    }
    finally {
        if ($started -and -not $process.HasExited) {
            $process.Kill($true)
            $process.WaitForExit()
        }
        $stopwatch.Stop()
        $process.Dispose()
    }
}

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

function Compare-ExpectedTree([object[]]$ExpectedInventory, [string]$ActualRoot) {
    $expected = @($ExpectedInventory | Sort-Object -Property path, kind)
    $actual = @(Get-TreeInventory $ActualRoot)
    $expectedJson = ConvertTo-Json -InputObject $expected -Depth 5 -Compress
    $actualJson = ConvertTo-Json -InputObject $actual -Depth 5 -Compress
    if ($expectedJson -cne $actualJson) {
        throw "Extracted fixture tree differs from its pinned inventory.`nExpected: $expectedJson`nActual: $actualJson"
    }
    $files = @($expected | Where-Object { $_.kind -eq "file" })
    return [pscustomobject][ordered]@{
        entryCount = $expected.Count
        fileCount = $files.Count
        totalBytes = ($files | Measure-Object -Property bytes -Sum).Sum
        manifestSha256 = Get-StringSha256 $expectedJson
    }
}

function Write-TestConfig(
    [string]$Path,
    [string]$Backend,
    [bool]$OpenAfterDoubleClick,
    [string]$Isolation = "appcontainer",
    [uint64]$MaxSingleFileBytes = 67108864
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
max_single_file_bytes = $MaxSingleFileBytes
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
    if ($Evidence.schemaVersion -ne 4 -or
        $Evidence.requestedMode -cne $expectedMode -or
        -not $Evidence.token.isAppContainer -or
        [bool]$Evidence.token.isLessPrivilegedAppContainer -ne $ExpectLpac -or
        $Evidence.token.capabilityCount -ne 0 -or
        -not $Evidence.network.denied -or
        -not $Evidence.timeout.rejected -or
        -not $Evidence.memory.rejected -or
        -not $Evidence.crash.terminatedWithoutSuccess -or
        $Evidence.crash.exitCode -eq 0 -or
        -not $Evidence.loaderFailure.createProcessRejected -or
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
    if ($cleanupRecords.Count -ne 7 -or
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
    schemaVersion = 5
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
        supported = $null
        failureClass = $null
        failClosed = $null
        isolation = $null
        doctor = $null
    }
    formats = @()
    encryptedArchives = @()
    encryptedArchiveFailures = $null
    readFixtures = @()
    pinnedFixtureDecoderSelfTest = $null
    rawStreamNegative = $null
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
    ) -ExpectedExitCodes @(0, 2)
    if ($lpacIsolationRun.exitCode -eq 0) {
        $lpacIsolation = $lpacIsolationRun.stdout | ConvertFrom-Json -Depth 20
        Assert-IsolationEvidence $lpacIsolation $true $report.binaries.irohaZipSha256
        $report.lpac.supported = $true
        $report.lpac.failClosed = $true
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
            exitCode = $lpacDoctorRun.exitCode
            elapsedMilliseconds = $lpacDoctorRun.elapsedMilliseconds
            outputSha256 = Get-StringSha256 $lpacDoctorRun.stdout
        }
    }
    else {
        $unsupportedPattern = 'GetTokenInformation\(TokenIsLessPrivilegedAppContainer\) failed: The parameter is incorrect\.'
        if (-not [string]::IsNullOrWhiteSpace($lpacIsolationRun.stdout) -or
            $lpacIsolationRun.stderr -notmatch $unsupportedPattern) {
            throw "LPAC isolation failed for an unrecognized reason: $($lpacIsolationRun.stderr.Trim())"
        }

        $lpacDoctorRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
            "--config", $lpacConfigPath, "doctor"
        ) -ExpectedExitCodes @(2)
        if ($lpacDoctorRun.stderr -notmatch $unsupportedPattern -or
            $lpacDoctorRun.stdout -match 'backend execution succeeded') {
            throw "LPAC doctor did not fail closed at the token verification boundary."
        }
        $report.lpac.supported = $false
        $report.lpac.failureClass = "token-query-invalid-parameter"
        $report.lpac.failClosed = $true
        $report.lpac.isolation = [ordered]@{
            exitCode = $lpacIsolationRun.exitCode
            elapsedMilliseconds = $lpacIsolationRun.elapsedMilliseconds
            stdoutEmpty = $true
            stderrSha256 = Get-StringSha256 $lpacIsolationRun.stderr
        }
        $report.lpac.doctor = [ordered]@{
            exitCode = $lpacDoctorRun.exitCode
            elapsedMilliseconds = $lpacDoctorRun.elapsedMilliseconds
            backendExecutionReported = $false
            stdoutSha256 = Get-StringSha256 $lpacDoctorRun.stdout
            stderrSha256 = Get-StringSha256 $lpacDoctorRun.stderr
        }
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
    $publicFixturePassword = "公開テスト-日本語-password-42"
    $encryptedFormats = @(
        [pscustomobject]@{ name = "zipcrypt"; option = "zip:encryption=zipcrypt" },
        [pscustomobject]@{ name = "aes128"; option = "zip:encryption=aes128" },
        [pscustomobject]@{ name = "aes256"; option = "zip:encryption=aes256" }
    )
    $encryptedArchives = @{}
    foreach ($fixture in $encryptedFormats) {
        $generatedArchive = Join-Path $filterArchivesRoot `
            ("encrypted-$($fixture.name).zip")
        $generateRun = Invoke-TestProcess -FilePath $backendExecutable -Arguments @(
            "-c", "--format=zip", "--options", $fixture.option,
            "--passphrase", $publicFixturePassword,
            "--no-xattrs", "--no-acls", "--no-fflags",
            "-f", $generatedArchive, "-C", $filterSourceRoot,
            "fixture.txt", "nested", "empty"
        )
        $archive = Join-Path $archivesRoot ("暗号化-$($fixture.name).zip")
        Copy-Item -LiteralPath $generatedArchive -Destination $archive
        $destination = Join-Path $extractedRoot ("encrypted-$($fixture.name)")
        $previewRun = Invoke-PasswordTestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "preview", $archive, "--prompt-password"
        ) -Password $publicFixturePassword
        if ($previewRun.stdout -notmatch '(?m)^file\s+\d+\s+fixture\.txt$') {
            throw "Encrypted $($fixture.name) preview did not expose the controlled fixture tree."
        }
        $extractRun = Invoke-PasswordTestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "extract", $archive,
            "--output", $destination, "--prompt-password"
        ) -Password $publicFixturePassword
        $tree = Compare-Tree $filterSourceRoot $destination
        $report.encryptedArchives += [pscustomobject][ordered]@{
            format = "zip"
            encryption = $fixture.name
            archiveBytes = (Get-Item -LiteralPath $archive).Length
            archiveSha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
            controlledSourceManifestSha256 = $filterSourceTree.manifestSha256
            treeManifestSha256 = $tree.manifestSha256
            controlledFixtureGenerationMilliseconds = $generateRun.elapsedMilliseconds
            previewMilliseconds = $previewRun.elapsedMilliseconds
            extractMilliseconds = $extractRun.elapsedMilliseconds
            nativeBilingualDialog = ($previewRun.dialogTitle -ceq $PasswordDialogTitle -and
                $extractRun.dialogTitle -ceq $PasswordDialogTitle)
            passwordControlProtected = ($previewRun.passwordControlProtected -and
                $extractRun.passwordControlProtected)
            passwordAbsentFromOutput = $true
            oneUsePrompt = $true
            explicitCleanupRequired = $true
        }
        $encryptedArchives[$fixture.name] = $archive
    }

    $wrongPasswordDestination = Join-Path $extractedRoot "encrypted-wrong-must-not-exist"
    $wrongPasswordRun = Invoke-PasswordTestProcess -FilePath $executablePath -Arguments @(
        "--config", $configPath, "extract", $encryptedArchives["aes256"],
        "--output", $wrongPasswordDestination, "--prompt-password"
    ) -Password "公開テスト-日本語-wrong-42" -ExpectedExitCodes @(2)
    if (Test-Path -LiteralPath $wrongPasswordDestination) {
        throw "Wrong encrypted-archive password published a destination tree."
    }

    $cancelDestination = Join-Path $extractedRoot "encrypted-cancel-must-not-exist"
    $cancelRun = Invoke-PasswordTestProcess -FilePath $executablePath -Arguments @(
        "--config", $configPath, "extract", $encryptedArchives["aes256"],
        "--output", $cancelDestination, "--prompt-password"
    ) -Cancel
    if ($cancelRun.exitCode -ne 0 -or
        -not [string]::IsNullOrWhiteSpace($cancelRun.stdout) -or
        -not [string]::IsNullOrWhiteSpace($cancelRun.stderr) -or
        (Test-Path -LiteralPath $cancelDestination)) {
        throw "Cancelling the password dialog was not a clean no-publication result."
    }
    $report.encryptedArchiveFailures = [ordered]@{
        wrongPasswordRejected = $true
        wrongPasswordExitCode = $wrongPasswordRun.exitCode
        wrongPasswordDestinationAbsent = $true
        wrongPasswordStderrSha256 = Get-StringSha256 $wrongPasswordRun.stderr
        cancelExitCode = $cancelRun.exitCode
        cancelOutputEmpty = $true
        cancelDestinationAbsent = $true
        nativeBilingualDialog = ($wrongPasswordRun.dialogTitle -ceq $PasswordDialogTitle -and
            $cancelRun.dialogTitle -ceq $PasswordDialogTitle)
        passwordControlsProtected = ($wrongPasswordRun.passwordControlProtected -and
            $cancelRun.passwordControlProtected)
        explicitCleanupRequired = $true
    }

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

    $rawSourceRoot = Join-Path $backendFixtureRoot "raw-source"
    [System.IO.Directory]::CreateDirectory($rawSourceRoot) | Out-Null
    $rawSource = Join-Path $rawSourceRoot "raw-fixture.txt"
    [System.IO.File]::WriteAllText(
        $rawSource,
        "controlled raw compressed stream fixture`n",
        [System.Text.UTF8Encoding]::new($false)
    )
    $rawSourceBytes = (Get-Item -LiteralPath $rawSource).Length
    $rawSourceSha256 = (Get-FileHash -LiteralPath $rawSource -Algorithm SHA256).Hash.ToLowerInvariant()
    $rawReadFixtures = @(
        [pscustomobject]@{ name = "raw-gzip"; filter = "-z"; extension = "gz" },
        [pscustomobject]@{ name = "raw-bzip2"; filter = "-j"; extension = "bz2" },
        [pscustomobject]@{ name = "raw-xz"; filter = "-J"; extension = "xz" },
        [pscustomobject]@{ name = "raw-zstd"; filter = "--zstd"; extension = "zst" },
        [pscustomobject]@{ name = "raw-compress"; filter = "-Z"; extension = "Z" }
    )
    $rawCreatedArchives = @{}
    foreach ($fixture in $rawReadFixtures) {
        $generatedArchive = Join-Path $filterArchivesRoot `
            ("$($fixture.name).txt.$($fixture.extension)")
        $generateRun = Invoke-TestProcess -FilePath $backendExecutable -Arguments @(
            "-c", "--format=raw", $fixture.filter,
            "--no-xattrs", "--no-acls", "--no-fflags",
            "-f", $generatedArchive, "-C", $rawSourceRoot, "raw-fixture.txt"
        )
        $generatedArchive = Resolve-Leaf $generatedArchive `
            "controlled $($fixture.name) fixture"
        $archiveFileName = "読取-$($fixture.name).txt.$($fixture.extension)"
        $archive = Join-Path $archivesRoot $archiveFileName
        Copy-Item -LiteralPath $generatedArchive -Destination $archive
        $expectedOutputName = "読取-$($fixture.name).txt"
        $expectedInventory = @(
            [pscustomobject][ordered]@{
                path = $expectedOutputName
                kind = "file"
                bytes = $rawSourceBytes
                sha256 = $rawSourceSha256
            }
        )
        $destination = Join-Path $extractedRoot $fixture.name
        $previewRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "preview", $archive
        )
        $extractRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "extract", $archive, "--output", $destination
        )
        $tree = Compare-ExpectedTree $expectedInventory $destination
        $report.readFixtures += [pscustomobject][ordered]@{
            format = $fixture.name
            extension = $fixture.extension
            archiveBytes = (Get-Item -LiteralPath $archive).Length
            archiveSha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
            controlledSourceBytes = $rawSourceBytes
            controlledSourceSha256 = $rawSourceSha256
            outputName = $expectedOutputName
            treeManifestSha256 = $tree.manifestSha256
            controlledFixtureGenerationMilliseconds = $generateRun.elapsedMilliseconds
            previewMilliseconds = $previewRun.elapsedMilliseconds
            extractMilliseconds = $extractRun.elapsedMilliseconds
            previewOutputSha256 = Get-StringSha256 $previewRun.stdout
            expectedFilterChecked = $true
            explicitCleanupRequired = $true
        }
        $rawCreatedArchives[$fixture.name] = $archive
    }

    $mismatchedRawArchive = Join-Path $archivesRoot "raw-filter-mismatch.txt.xz"
    Copy-Item -LiteralPath $rawCreatedArchives["raw-gzip"] -Destination $mismatchedRawArchive
    $mismatchedRawDestination = Join-Path $extractedRoot "raw-filter-mismatch-must-not-exist"
    $mismatchedRawRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "--config", $configPath, "extract", $mismatchedRawArchive,
        "--output", $mismatchedRawDestination
    ) -ExpectedExitCodes @(2)
    if ($mismatchedRawRun.stderr -notmatch 'raw-stream filter mismatch: expected xz' -or
        (Test-Path -LiteralPath $mismatchedRawDestination)) {
        throw "A raw stream whose bytes disagree with its extension did not fail closed."
    }

    $rawLimitConfigPath = Join-Path $testRoot "設定-raw-limit.toml"
    Write-TestConfig $rawLimitConfigPath $backendPath $false "appcontainer" 32
    $limitedRawDestination = Join-Path $extractedRoot "raw-limit-must-not-exist"
    $limitedRawRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "--config", $rawLimitConfigPath, "extract", $rawCreatedArchives["raw-gzip"],
        "--output", $limitedRawDestination
    ) -ExpectedExitCodes @(2)
    if ($limitedRawRun.stderr -notmatch 'raw-stream output exceeds 32 bytes' -or
        (Test-Path -LiteralPath $limitedRawDestination)) {
        throw "A raw stream exceeding the configured single-file limit did not fail closed."
    }

    $corruptRawArchive = Join-Path $archivesRoot "raw-corrupt.txt.gz"
    $corruptRawBytes = [System.IO.File]::ReadAllBytes($rawCreatedArchives["raw-gzip"])
    if ($corruptRawBytes.Length -le 10) {
        throw "The controlled gzip stream is too short for the corruption regression."
    }
    $corruptRawBytes[10] = $corruptRawBytes[10] -bxor 0x01
    [System.IO.File]::WriteAllBytes($corruptRawArchive, $corruptRawBytes)
    $corruptRawDestination = Join-Path $extractedRoot "raw-corrupt-must-not-exist"
    $corruptRawRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "--config", $configPath, "extract", $corruptRawArchive,
        "--output", $corruptRawDestination
    ) -ExpectedExitCodes @(2)
    if ($corruptRawRun.stderr -notmatch 'could not open the raw stream' -or
        (Test-Path -LiteralPath $corruptRawDestination)) {
        throw "A gzip stream with an invalid compressed payload did not fail closed."
    }
    $report.rawStreamNegative = [ordered]@{
        filterMismatchRejected = $true
        expectedFilter = "xz"
        actualFilter = "gzip"
        filterMismatchExitCode = $mismatchedRawRun.exitCode
        filterMismatchStderrSha256 = Get-StringSha256 $mismatchedRawRun.stderr
        byteLimitRejected = $true
        byteLimit = 32
        byteLimitExitCode = $limitedRawRun.exitCode
        byteLimitStderrSha256 = Get-StringSha256 $limitedRawRun.stderr
        compressedPayloadCorruptionRejected = $true
        compressedPayloadCorruptionExitCode = $corruptRawRun.exitCode
        compressedPayloadCorruptionStderrSha256 = Get-StringSha256 $corruptRawRun.stderr
        allDestinationsAbsent = $true
        explicitCleanupRequired = $true
    }

    $windowsRoot = [Environment]::GetFolderPath([Environment+SpecialFolder]::Windows)
    if ([string]::IsNullOrWhiteSpace($windowsRoot)) {
        throw "Windows did not report its system root for the controlled CAB fixture."
    }
    $makeCabPath = Resolve-Leaf (Join-Path $windowsRoot "System32\makecab.exe") `
        "Windows makecab"
    $makeCabSignature = Get-AuthenticodeSignature -FilePath $makeCabPath
    if ($makeCabSignature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
        $null -eq $makeCabSignature.SignerCertificate -or
        $makeCabSignature.SignerCertificate.Subject -notmatch `
            '(^|,\s*)O=Microsoft Corporation(,|$)') {
        throw "The controlled CAB generator is not validly Microsoft-signed: $makeCabPath"
    }
    $cabSourceRoot = Join-Path $backendFixtureRoot "cab-source"
    [System.IO.Directory]::CreateDirectory($cabSourceRoot) | Out-Null
    $cabSource = Join-Path $cabSourceRoot "cab-fixture.txt"
    [System.IO.File]::WriteAllText(
        $cabSource,
        "controlled Microsoft Cabinet LZX fixture`n",
        [System.Text.UTF8Encoding]::new($false)
    )
    $cabSourceTree = Compare-Tree $cabSourceRoot $cabSourceRoot
    $generatedCab = Join-Path $filterArchivesRoot "controlled-lzx.cab"
    $cabGenerateRun = Invoke-TestProcess -FilePath $makeCabPath -Arguments @(
        "/V1", "/D", "CompressionType=LZX", "/D", "CompressionMemory=21",
        $cabSource, $generatedCab
    )
    $generatedCab = Resolve-Leaf $generatedCab "controlled CAB fixture"
    $cabArchive = Join-Path $archivesRoot "読取-cab.cab"
    Copy-Item -LiteralPath $generatedCab -Destination $cabArchive
    $cabDestination = Join-Path $extractedRoot "cab"
    $cabPreviewRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "--config", $configPath, "preview", $cabArchive
    )
    $cabExtractRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
        "--config", $configPath, "extract", $cabArchive, "--output", $cabDestination
    )
    $cabTree = Compare-Tree $cabSourceRoot $cabDestination
    $report.readFixtures += [pscustomobject][ordered]@{
        format = "cab-lzx"
        extension = "cab"
        archiveBytes = (Get-Item -LiteralPath $cabArchive).Length
        archiveSha256 = (Get-FileHash -LiteralPath $cabArchive -Algorithm SHA256).Hash.ToLowerInvariant()
        controlledSourceManifestSha256 = $cabSourceTree.manifestSha256
        treeManifestSha256 = $cabTree.manifestSha256
        controlledFixtureGenerationMilliseconds = $cabGenerateRun.elapsedMilliseconds
        previewMilliseconds = $cabPreviewRun.elapsedMilliseconds
        extractMilliseconds = $cabExtractRun.elapsedMilliseconds
        generator = [ordered]@{
            fileName = "makecab.exe"
            fileSha256 = (Get-FileHash -LiteralPath $makeCabPath -Algorithm SHA256).Hash.ToLowerInvariant()
            authenticodeStatus = $makeCabSignature.Status.ToString()
            signerSubject = $makeCabSignature.SignerCertificate.Subject
            compression = "LZX:21"
        }
        explicitCleanupRequired = $true
    }

    $libarchiveFixtureRoot = Resolve-Directory (
        Join-Path $PSScriptRoot "..\tests\fixtures\libarchive-v3.8.9"
    ) "pinned libarchive fixture directory"
    $decoderSelfTestSource = Join-Path $libarchiveFixtureRoot `
        "test_read_format_rar5_stored.rar.uu"
    $decoderSelfTestCases = @(
        [pscustomobject]@{
            name = "encoded-hash-drift"
            encodedSha256 = "0000000000000000000000000000000000000000000000000000000000000000"
            decodedBytes = 109
            decodedSha256 = "35d75e315d164d2e329afc28f7d844f013271b4fcffd4ddd78efcdd114a383a7"
            expectedMessage = "Pinned UU fixture hash mismatch:"
        }
        [pscustomobject]@{
            name = "decoded-length-drift"
            encodedSha256 = "ec73ba623a8e8eee4909dcdf45f0526ff9adc1d856d054ce742e6c1ba1fb5fa8"
            decodedBytes = 108
            decodedSha256 = "35d75e315d164d2e329afc28f7d844f013271b4fcffd4ddd78efcdd114a383a7"
            expectedMessage = "Pinned UU fixture decoded length mismatch:"
        }
        [pscustomobject]@{
            name = "decoded-hash-drift"
            encodedSha256 = "ec73ba623a8e8eee4909dcdf45f0526ff9adc1d856d054ce742e6c1ba1fb5fa8"
            decodedBytes = 109
            decodedSha256 = "0000000000000000000000000000000000000000000000000000000000000000"
            expectedMessage = "Pinned UU fixture decoded hash mismatch:"
        }
    )
    foreach ($selfTest in $decoderSelfTestCases) {
        $rejectedDestination = Join-Path $filterArchivesRoot `
            ("decoder-must-not-publish-$($selfTest.name).rar")
        $rejected = $false
        try {
            Expand-PinnedUuFixture `
                -SourcePath $decoderSelfTestSource `
                -ExpectedFileName "test_read_format_rar5_stored.rar" `
                -ExpectedEncodedSha256 $selfTest.encodedSha256 `
                -ExpectedDecodedBytes $selfTest.decodedBytes `
                -ExpectedDecodedSha256 $selfTest.decodedSha256 `
                -DestinationPath $rejectedDestination | Out-Null
        }
        catch {
            if (-not $_.Exception.Message.StartsWith(
                $selfTest.expectedMessage,
                [System.StringComparison]::Ordinal
            )) {
                throw "Pinned UU decoder self-test returned an unexpected error: $($_.Exception.Message)"
            }
            $rejected = $true
        }
        if (-not $rejected -or (Test-Path -LiteralPath $rejectedDestination)) {
            throw "Pinned UU decoder self-test published a rejected fixture: $($selfTest.name)"
        }
    }
    $report.pinnedFixtureDecoderSelfTest = [ordered]@{
        passed = $true
        rejectionCases = @($decoderSelfTestCases | ForEach-Object { $_.name })
    }
    $pinnedReadFixtures = @(
        [pscustomobject]@{
            format = "rar"
            extension = "rar"
            uuFile = "test_read_format_rar_windows.rar.uu"
            decodedFile = "test_read_format_rar_windows.rar"
            encodedSha256 = "d934dc7895212d468a2d44111e77d95536d79c3c9eae56690667d483ae9419d7"
            decodedBytes = 814
            decodedSha256 = "8d689455e9ecd92c19426604e2360b5ef8eb023890fe46aabbe2864260b70fc9"
            expectedInventory = @(
                [pscustomobject][ordered]@{
                    path = "test.txt"; kind = "file"; bytes = 16
                    sha256 = "2d45c5f87d1b6cef59a1d67a0ddeea9c75a7df81e5b64d30ecff39199b411bd9"
                }
                [pscustomobject][ordered]@{
                    path = "testdir"; kind = "directory"; bytes = 0; sha256 = $null
                }
                [pscustomobject][ordered]@{
                    path = "testdir/test.txt"; kind = "file"; bytes = 16
                    sha256 = "2d45c5f87d1b6cef59a1d67a0ddeea9c75a7df81e5b64d30ecff39199b411bd9"
                }
                [pscustomobject][ordered]@{
                    path = "testemptydir"; kind = "directory"; bytes = 0; sha256 = $null
                }
                [pscustomobject][ordered]@{
                    path = "testshortcut.lnk"; kind = "file"; bytes = 441
                    sha256 = "08b633f146f22534956b11bbc92e85f3f975e2820ecb892f958db5ae7bd7cf1f"
                }
            )
        }
        [pscustomobject]@{
            format = "rar5"
            extension = "rar"
            uuFile = "test_read_format_rar5_stored.rar.uu"
            decodedFile = "test_read_format_rar5_stored.rar"
            encodedSha256 = "ec73ba623a8e8eee4909dcdf45f0526ff9adc1d856d054ce742e6c1ba1fb5fa8"
            decodedBytes = 109
            decodedSha256 = "35d75e315d164d2e329afc28f7d844f013271b4fcffd4ddd78efcdd114a383a7"
            expectedInventory = @(
                [pscustomobject][ordered]@{
                    path = "helloworld.txt"; kind = "file"; bytes = 29
                    sha256 = "fef9ad8cf601b43f76c6320075f62267c6e5c0a526d750a70b80c919a4a0aad8"
                }
            )
        }
        [pscustomobject]@{
            format = "lha-level3"
            extension = "lzh"
            uuFile = "test_read_format_lha_header3.lzh.uu"
            decodedFile = "test_read_format_lha_header3.lzh"
            encodedSha256 = "4bcbe7e493bca4d79eb21d1c2dc8031190b1b41fb487fc61e71c757d3232b33f"
            decodedBytes = 548
            decodedSha256 = "d36f9beaf7d1aa482315e810c8cfca327975ffd31a05082004102327310e419d"
            expectedInventory = @(
                [pscustomobject][ordered]@{
                    path = "dir"; kind = "directory"; bytes = 0; sha256 = $null
                }
                [pscustomobject][ordered]@{
                    path = "dir2"; kind = "directory"; bytes = 0; sha256 = $null
                }
                [pscustomobject][ordered]@{
                    path = "file1"; kind = "file"; bytes = 60
                    sha256 = "d0c504f06bbd64d183524eb35e5482ee5d966d456b905a24147165b2904d301b"
                }
                [pscustomobject][ordered]@{
                    path = "file2"; kind = "file"; bytes = 78
                    sha256 = "60f47caf717b06cf21b3bbb7775e49269a1b5cd6b94bba62da29a2ecb048ccf2"
                }
            )
        }
        [pscustomobject]@{
            format = "zipx-bzip2"
            extension = "zipx"
            uuFile = "test_read_format_zip_bzip2.zipx.uu"
            decodedFile = "test_read_format_zip_bzip2.zipx"
            encodedSha256 = "7baa771d86ac20a4d1ed079be94088c1628d8a513843981f353bda27ba36d359"
            decodedBytes = 708
            decodedSha256 = "373ec637744c762bb6c69c2c4f6cc2d9dad85ed5d4662b0ffb9373077dbf01a5"
            expectedInventory = @(
                [pscustomobject][ordered]@{
                    path = "vimrc"; kind = "file"; bytes = 912
                    sha256 = "b16e85e457397ab2043a7ee0a3c84307c6b4eac157fd0b721694761f25b3ed5b"
                }
            )
        }
    )
    foreach ($fixture in $pinnedReadFixtures) {
        $sourceUu = Join-Path $libarchiveFixtureRoot $fixture.uuFile
        $generatedArchive = Join-Path $filterArchivesRoot $fixture.decodedFile
        $decodeTimer = [System.Diagnostics.Stopwatch]::StartNew()
        try {
            $decoded = Expand-PinnedUuFixture `
                -SourcePath $sourceUu `
                -ExpectedFileName $fixture.decodedFile `
                -ExpectedEncodedSha256 $fixture.encodedSha256 `
                -ExpectedDecodedBytes $fixture.decodedBytes `
                -ExpectedDecodedSha256 $fixture.decodedSha256 `
                -DestinationPath $generatedArchive
        }
        finally {
            $decodeTimer.Stop()
        }

        $archive = Join-Path $archivesRoot ("読取-$($fixture.format).$($fixture.extension)")
        Copy-Item -LiteralPath $decoded.path -Destination $archive
        $archiveSha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($archiveSha256 -cne $fixture.decodedSha256 -or
            (Get-Item -LiteralPath $archive).Length -ne $fixture.decodedBytes) {
            throw "Copied pinned archive differs from the verified decoded fixture: $($fixture.format)"
        }

        $destination = Join-Path $extractedRoot $fixture.format
        $previewRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "preview", $archive
        )
        $extractRun = Invoke-TestProcess -FilePath $executablePath -Arguments @(
            "--config", $configPath, "extract", $archive, "--output", $destination
        )
        $tree = Compare-ExpectedTree $fixture.expectedInventory $destination
        $report.readFixtures += [pscustomobject][ordered]@{
            format = $fixture.format
            extension = $fixture.extension
            archiveBytes = $fixture.decodedBytes
            archiveSha256 = $archiveSha256
            treeManifestSha256 = $tree.manifestSha256
            fixtureDecodeMilliseconds = $decodeTimer.ElapsedMilliseconds
            previewMilliseconds = $previewRun.elapsedMilliseconds
            extractMilliseconds = $extractRun.elapsedMilliseconds
            previewOutputSha256 = Get-StringSha256 $previewRun.stdout
            pinnedSource = [ordered]@{
                project = "libarchive"
                version = "3.8.9"
                tagObject = "f1f785cc218bb05876c54680f10d3d4e54575ea2"
                commit = "27cbc7827172698143e440801fc0ba39ccb4f1f5"
                license = "BSD-2-Clause"
                uuFile = $fixture.uuFile
                uuBytes = $decoded.encodedBytes
                uuSha256 = $decoded.encodedSha256
            }
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

    $rawShellArchive = Join-Path $shellRoot "シェルraw.txt.gz"
    Copy-Item -LiteralPath $rawCreatedArchives["raw-gzip"] -Destination $rawShellArchive
    $rawShellRun = Invoke-TestProcess -FilePath $shellExecutablePath `
        -Arguments @($rawShellArchive) `
        -Environment @{ LOCALAPPDATA = $shellLocalAppData }
    $rawShellDestination = Join-Path $shellRoot "シェルraw.txt"
    $rawShellExpected = @(
        [pscustomobject][ordered]@{
            path = "シェルraw.txt"
            kind = "file"
            bytes = $rawSourceBytes
            sha256 = $rawSourceSha256
        }
    )
    $rawShellTree = Compare-ExpectedTree $rawShellExpected $rawShellDestination
    $report.shell = [ordered]@{
        elapsedMilliseconds = $shellRun.elapsedMilliseconds
        destinationCreated = $true
        treeManifestSha256 = $shellTree.manifestSha256
        defaultConfigPathUsed = $true
        rawStream = [ordered]@{
            elapsedMilliseconds = $rawShellRun.elapsedMilliseconds
            destinationCreated = $true
            treeManifestSha256 = $rawShellTree.manifestSha256
            outputNameDerivedFromOuterArchive = $true
            internalReaderDispatchedWithoutUi = $true
        }
        explicitCleanupRequired = $true
    }

    $expectedReadFormats = @(
        "tar-bz2",
        "tar-xz",
        "tar-zstd",
        "tar-compress",
        "raw-gzip",
        "raw-bzip2",
        "raw-xz",
        "raw-zstd",
        "raw-compress",
        "cab-lzx",
        "rar",
        "rar5",
        "lha-level3",
        "zipx-bzip2"
    )
    $actualReadFormats = @($report.readFixtures | ForEach-Object { $_.format })
    if ((ConvertTo-Json -InputObject $actualReadFormats -Compress) -cne
        (ConvertTo-Json -InputObject $expectedReadFormats -Compress)) {
        throw "Windows E2E did not execute the exact 14-format additional-read matrix."
    }
    $expectedEncryptedFormats = @("zipcrypt", "aes128", "aes256")
    $actualEncryptedFormats = @($report.encryptedArchives | ForEach-Object { $_.encryption })
    if ((ConvertTo-Json -InputObject $actualEncryptedFormats -Compress) -cne
        (ConvertTo-Json -InputObject $expectedEncryptedFormats -Compress)) {
        throw "Windows E2E did not execute the exact three-format encrypted ZIP matrix."
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
