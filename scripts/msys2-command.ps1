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

    # Keep the caller's script and arguments out of interpolated shell syntax.
    # The outer shell receives them as positional arguments and starts a fresh
    # inner shell behind coreutils timeout.
    $boundedLauncher = @'
timeout_seconds="$1"
command_script="$2"
shift 2
exec /usr/bin/timeout --signal=TERM --kill-after=10s "${timeout_seconds}s" \
    /usr/bin/bash --noprofile --norc -lc "$command_script" iroha-zip-command "$@"
'@
    $output = @(
        & $BashPath --noprofile --norc -lc $boundedLauncher iroha-zip-timeout `
            ([string]$TimeoutSeconds) $Script @Arguments 2>&1
    )
    $exitCode = $LASTEXITCODE
    if ($exitCode -eq 124 -or $exitCode -eq 137) {
        throw "MSYS2 command exceeded the ${TimeoutSeconds}-second limit (exit $exitCode).`n$($output -join "`n")"
    }
    if ($exitCode -ne 0) {
        throw "MSYS2 command failed (exit $exitCode).`n$($output -join "`n")"
    }
    return @($output | ForEach-Object { [string]$_ })
}
