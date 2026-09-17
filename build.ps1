<#
.SYNOPSIS
    Builds and tests PTK (Proxy Token Killer) using Docker cross-compilation for Windows.

.DESCRIPTION
    Cross-compiles PTK to x86_64-pc-windows-gnu inside a Docker container.
    Caches the Cargo crates registry and git dependencies in persistent Docker volumes
    for fast, incremental builds and tests.

.PARAMETER Release
    Build optimized release binary (default is fast debug build).

.PARAMETER Check
    Run fast syntax and type checking only (no binary emitted).

.PARAMETER Test
    Run unit tests inside the Docker container using cached volumes.

.PARAMETER TestFilter
    Optional name filter for tests (e.g. -Test -TestFilter gci_cmd).

.PARAMETER Clean
    Clean the target build directory.

.EXAMPLE
    .\build.ps1
    Fast debug build (~3 seconds on subsequent runs).

.EXAMPLE
    .\build.ps1 -Release
    Build optimized, stripped release binary.

.EXAMPLE
    .\build.ps1 -Check
    Fast type check.

.EXAMPLE
    .\build.ps1 -Test
    Run unit tests.

.EXAMPLE
    .\build.ps1 -Test -TestFilter gci_cmd
    Run specific unit tests matching a filter.
#>

param(
    [switch]$Release,
    [switch]$Check,
    [switch]$Test,
    [string]$TestFilter,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"

$workspace = $PSScriptRoot
if (-not $workspace) {
    $workspace = Get-Location
}

if ($Clean) {
    Write-Host "[ptk] Cleaning target directory..." -ForegroundColor Yellow
    docker run --rm -v "${workspace}:/workspace" ptk-builder cargo clean
    Write-Host "[ptk] Clean complete." -ForegroundColor Green
    return
}

# Determine cargo command
if ($Test) {
    $filterArg = if ($TestFilter) { " $TestFilter" } else { "" }
    $cargoCmd = "RUST_MIN_STACK=8388608 cargo test --bin ptk$filterArg"
    $mode = "Test"
} elseif ($Check) {
    $cargoCmd = "cargo check --target x86_64-pc-windows-gnu -j 2"
    $mode = "Check"
} elseif ($Release) {
    $cargoCmd = "cargo build --release --target x86_64-pc-windows-gnu -j 2"
    $mode = "Release"
} else {
    $cargoCmd = "cargo build --target x86_64-pc-windows-gnu -j 2"
    $mode = "Debug (Fast)"
}

Write-Host "[ptk] Executing PTK [$mode] with cached Docker volumes..." -ForegroundColor Cyan

docker run --rm `
    -v "${workspace}:/workspace" `
    -v ptk-cargo-registry:/usr/local/cargo/registry `
    -v ptk-cargo-git:/usr/local/cargo/git `
    ptk-builder sh -c "$cargoCmd"

if ($LASTEXITCODE -eq 0 -and -not $Check -and -not $Test) {
    $binSubdir = if ($Release) { "release" } else { "debug" }
    $binPath = Join-Path $workspace "target\x86_64-pc-windows-gnu\$binSubdir\ptk.exe"

    if (Test-Path $binPath) {
        Write-Host "[ptk] Build succeeded!" -ForegroundColor Green
        Write-Host "[ptk] Binary: $binPath" -ForegroundColor Gray
        $ver = & $binPath --version
        Write-Host "[ptk] Verified: $ver" -ForegroundColor Green
    }
}
