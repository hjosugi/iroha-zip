Set-StrictMode -Version Latest

function Expand-PinnedUuFixture {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourcePath,

        [Parameter(Mandatory = $true)]
        [string]$ExpectedFileName,

        [Parameter(Mandatory = $true)]
        [ValidatePattern('^[0-9a-f]{64}$')]
        [string]$ExpectedEncodedSha256,

        [Parameter(Mandatory = $true)]
        [ValidateRange(1, 4194304)]
        [int]$ExpectedDecodedBytes,

        [Parameter(Mandatory = $true)]
        [ValidatePattern('^[0-9a-f]{64}$')]
        [string]$ExpectedDecodedSha256,

        [Parameter(Mandatory = $true)]
        [string]$DestinationPath
    )

    $resolvedSource = (Resolve-Path -LiteralPath $SourcePath -ErrorAction Stop).Path
    $source = Get-Item -LiteralPath $resolvedSource -Force -ErrorAction Stop
    if ($source.PSIsContainer -or
        ($source.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Pinned UU fixture is not an ordinary file: $SourcePath"
    }
    if ($source.Length -gt 65536) {
        throw "Pinned UU fixture exceeds the 64 KiB encoded limit: $SourcePath"
    }

    $encodedSha256 = (Get-FileHash -LiteralPath $resolvedSource -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($encodedSha256 -cne $ExpectedEncodedSha256) {
        throw "Pinned UU fixture hash mismatch: $SourcePath"
    }

    $lines = [System.IO.File]::ReadAllLines($resolvedSource, [System.Text.Encoding]::ASCII)
    if ($lines.Count -lt 4 -or $lines[0] -cne "begin 644 $ExpectedFileName" -or
        $lines[$lines.Count - 1] -cne "end") {
        throw "Pinned UU fixture has an unexpected envelope: $SourcePath"
    }

    $decoded = [System.Collections.Generic.List[byte]]::new($ExpectedDecodedBytes)
    $sawTerminator = $false
    for ($lineIndex = 1; $lineIndex -lt $lines.Count - 1; $lineIndex++) {
        $line = $lines[$lineIndex]
        if ($line -ceq '`') {
            if ($sawTerminator -or $lineIndex -ne $lines.Count - 2) {
                throw "Pinned UU fixture has a misplaced terminator: $SourcePath"
            }
            $sawTerminator = $true
            continue
        }
        if ($sawTerminator -or $line.Length -lt 2) {
            throw "Pinned UU fixture has data outside its bounded body: $SourcePath"
        }

        $decodedLineBytes = (([int][char]$line[0] - 32) -band 0x3f)
        if ($decodedLineBytes -lt 1 -or $decodedLineBytes -gt 45) {
            throw "Pinned UU fixture has an invalid line length: $SourcePath"
        }
        $encodedCharacters = 4 * [int][System.Math]::Ceiling($decodedLineBytes / 3.0)
        if ($line.Length -ne 1 + $encodedCharacters) {
            throw "Pinned UU fixture has an invalid encoded line width: $SourcePath"
        }

        $remaining = $decodedLineBytes
        for ($characterIndex = 1; $characterIndex -lt $line.Length; $characterIndex += 4) {
            $values = [int[]]::new(4)
            for ($offset = 0; $offset -lt 4; $offset++) {
                $code = [int][char]$line[$characterIndex + $offset]
                if ($code -lt 32 -or $code -gt 96) {
                    throw "Pinned UU fixture contains a non-UU character: $SourcePath"
                }
                $values[$offset] = ($code - 32) -band 0x3f
            }

            $block = [byte[]]@(
                (($values[0] -shl 2) -bor ($values[1] -shr 4)) -band 0xff
                (($values[1] -shl 4) -bor ($values[2] -shr 2)) -band 0xff
                (($values[2] -shl 6) -bor $values[3]) -band 0xff
            )
            $take = [System.Math]::Min(3, $remaining)
            for ($byteIndex = 0; $byteIndex -lt $take; $byteIndex++) {
                $decoded.Add($block[$byteIndex])
            }
            $remaining -= $take
        }
        if ($remaining -ne 0 -or $decoded.Count -gt 4194304) {
            throw "Pinned UU fixture exceeded its declared bounded line: $SourcePath"
        }
    }
    if (-not $sawTerminator -or $decoded.Count -ne $ExpectedDecodedBytes) {
        throw "Pinned UU fixture decoded length mismatch: $SourcePath"
    }

    $decodedBytes = $decoded.ToArray()
    $decodedSha256 = [System.Convert]::ToHexString(
        [System.Security.Cryptography.SHA256]::HashData($decodedBytes)
    ).ToLowerInvariant()
    if ($decodedSha256 -cne $ExpectedDecodedSha256) {
        throw "Pinned UU fixture decoded hash mismatch: $SourcePath"
    }

    $destination = [System.IO.Path]::GetFullPath($DestinationPath)
    [System.IO.Directory]::CreateDirectory((Split-Path -Parent $destination)) | Out-Null
    $stream = [System.IO.FileStream]::new(
        $destination,
        [System.IO.FileMode]::CreateNew,
        [System.IO.FileAccess]::Write,
        [System.IO.FileShare]::None
    )
    try {
        $stream.Write($decodedBytes, 0, $decodedBytes.Length)
        $stream.Flush($true)
    }
    finally {
        $stream.Dispose()
    }

    return [pscustomobject][ordered]@{
        path = $destination
        encodedBytes = $source.Length
        encodedSha256 = $encodedSha256
        decodedBytes = $decodedBytes.Length
        decodedSha256 = $decodedSha256
    }
}
