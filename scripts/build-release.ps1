[CmdletBinding()]
param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$IncludeBackend
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $ProjectRoot
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo was not found. Install the Rust toolchain specified by rust-toolchain.toml."
    }
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        throw "rustup was not found."
    }

    if ($IncludeBackend) {
        $backendManifest = Join-Path $ProjectRoot "backend\libarchive\backend-manifest.tsv"
        if (-not (Test-Path -LiteralPath $backendManifest -PathType Leaf)) {
            throw "-IncludeBackend requires an installed backend. Use the settings screen or an install script first."
        }
    }

    & rustup target add $Target
    if ($LASTEXITCODE -ne 0) { throw "rustup target add failed." }

    if (-not (Test-Path -LiteralPath (Join-Path $ProjectRoot "Cargo.lock"))) {
        & cargo generate-lockfile
        if ($LASTEXITCODE -ne 0) { throw "cargo generate-lockfile failed." }
        Write-Warning "Cargo.lock was generated. Review and commit it before a formal release."
    }

    & cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "cargo fmt check failed." }

    & cargo test --all-targets --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed." }

    & cargo clippy --all-targets --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed." }

    & cargo build --release --target $Target --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed." }

    $version = ((Select-String -LiteralPath "Cargo.toml" -Pattern '^version\s*=\s*"([^\"]+)"').Matches[0].Groups[1].Value)
    $releaseSource = Join-Path $ProjectRoot "target\$Target\release"
    $distRoot = Join-Path $ProjectRoot "dist"
    $appRoot = Join-Path $distRoot "iroha-zip"
    if (Test-Path -LiteralPath $appRoot) {
        Remove-Item -LiteralPath $appRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Path $appRoot | Out-Null

    foreach ($binary in @("iroha-zip.exe", "iroha-zip-shell.exe", "iroha-zip-settings.exe")) {
        $source = Join-Path $releaseSource $binary
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Built binary is missing: $source"
        }
        Copy-Item -LiteralPath $source -Destination (Join-Path $appRoot $binary)
    }

    $releaseBackend = Join-Path $appRoot "backend"
    New-Item -ItemType Directory -Force -Path $releaseBackend | Out-Null
    Copy-Item -LiteralPath (Join-Path $ProjectRoot "backend\README.md") `
        -Destination (Join-Path $releaseBackend "README.md")
    if ($IncludeBackend) {
        Copy-Item -LiteralPath (Join-Path $ProjectRoot "backend\libarchive") `
            -Destination (Join-Path $releaseBackend "libarchive") -Recurse
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $appRoot "scripts") | Out-Null
    foreach ($script in @(
        "install-backend.ps1",
        "export-msys2-backend.ps1",
        "register-associations.ps1",
        "unregister-associations.ps1"
    )) {
        Copy-Item -LiteralPath (Join-Path $ProjectRoot "scripts\$script") `
            -Destination (Join-Path $appRoot "scripts\$script")
    }

    foreach ($file in @(
        "README.md",
        "SECURITY.md",
        "config.example.toml",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "THIRD-PARTY-NOTICES.md",
        "THIRD-PARTY-LICENSES.html"
    )) {
        Copy-Item -LiteralPath (Join-Path $ProjectRoot $file) -Destination (Join-Path $appRoot $file)
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $appRoot "docs") | Out-Null
    foreach ($document in @("BACKEND_MANIFEST.md", "THREAT_MODEL.md")) {
        Copy-Item -LiteralPath (Join-Path $ProjectRoot "docs\$document") `
            -Destination (Join-Path $appRoot "docs\$document")
    }

    $zip = Join-Path $distRoot "iroha-zip-$version-windows-x64.zip"
    if (Test-Path -LiteralPath $zip) {
        Remove-Item -LiteralPath $zip -Force
    }
    Compress-Archive -LiteralPath $appRoot -DestinationPath $zip -CompressionLevel Optimal
    $hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    [System.IO.File]::WriteAllText(
        "$zip.sha256",
        "$hash  $([System.IO.Path]::GetFileName($zip))`n",
        [System.Text.UTF8Encoding]::new($false)
    )

    Write-Host "Release folder: $appRoot"
    Write-Host "Release ZIP:    $zip"
    Write-Host "SHA-256:        $hash"
    if (-not $IncludeBackend) {
        Write-Host "Backend:        not bundled; install it from the settings screen"
    }
}
finally {
    Pop-Location
}
