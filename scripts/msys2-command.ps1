Set-StrictMode -Version Latest

function Invoke-IrohaZipMsys2Command {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$BashPath,

        [Parameter(Mandatory = $true)]
        [string]$Script,

        [string[]]$Arguments = @(),

        [ValidateRange(1, 1800)]
        [int]$TimeoutSeconds = 180
    )

    if ($Script.Contains([char]0)) {
        throw "MSYS2 command script contains an interior NUL."
    }

    # Windows PowerShell 5.1 cannot preserve an arbitrary shell program passed
    # through its legacy native-command quoting. Put both programs in fresh
    # UTF-8 files and pass only paths, the decimal timeout, and data arguments
    # on the native command line.
    $temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) `
        ("iroha-zip-msys2-command-" + [Guid]::NewGuid().ToString("N"))
    $launcherPath = Join-Path $temporaryRoot "bounded-launcher.sh"
    $commandPath = Join-Path $temporaryRoot "command.sh"
    $boundedLauncher = @'
timeout_seconds="$1"
command_file="$2"
shift 2
exec /usr/bin/timeout --signal=TERM --kill-after=10s "${timeout_seconds}s" \
    /usr/bin/bash --noprofile --norc "$command_file" "$@"
'@
    $output = @()
    $exitCode = $null
    try {
        [System.IO.Directory]::CreateDirectory($temporaryRoot) | Out-Null
        [System.IO.File]::WriteAllText(
            $launcherPath,
            ($boundedLauncher + "`n"),
            [System.Text.UTF8Encoding]::new($false)
        )
        [System.IO.File]::WriteAllText(
            $commandPath,
            ($Script + "`n"),
            [System.Text.UTF8Encoding]::new($false)
        )

        # Native stderr is an ErrorRecord stream in Windows PowerShell 5.1.
        # Keep it capturable even when the caller uses ErrorActionPreference=Stop.
        $previousErrorActionPreference = $ErrorActionPreference
        try {
            $ErrorActionPreference = "Continue"
            $output = @(
                & $BashPath --noprofile --norc $launcherPath `
                    ([string]$TimeoutSeconds) $commandPath @Arguments 2>&1
            )
            $exitCode = $LASTEXITCODE
        }
        finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }
    }
    finally {
        if (Test-Path -LiteralPath $temporaryRoot) {
            Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
        }
    }
    if ($exitCode -eq 124 -or $exitCode -eq 137) {
        throw "MSYS2 command exceeded the ${TimeoutSeconds}-second limit (exit $exitCode).`n$($output -join "`n")"
    }
    if ($exitCode -ne 0) {
        throw "MSYS2 command failed (exit $exitCode).`n$($output -join "`n")"
    }
    return @($output | ForEach-Object { [string]$_ })
}
