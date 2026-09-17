# PTK Docker Test Runner (PowerShell)
#
# Runs the full Rust test suite (unit + integration) inside the build container
# so results match CI regardless of the host toolchain.
#
# Why non-root: some tests (e.g. unwritable_hooks_dir_never_breaks_the_hook) rely
# on Unix DAC permissions that root bypasses. GitHub CI runs as a non-root user,
# so we run as uid 1000 here too. HOME/CARGO_HOME/CARGO_TARGET_DIR point at
# container-local paths to avoid polluting the mounted workspace, and git is told
# the bind-mounted repo is a safe.directory (root-owned mount vs uid 1000).
#
# Usage:
#   ./scripts/test-docker.ps1                 # build image + run all tests
#   ./scripts/test-docker.ps1 -- --nocapture  # extra args forwarded to cargo test

param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$TestArgs
)

$ErrorActionPreference = 'Stop'
$Image = 'ptk-build'
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

Write-Host "Building ${Image} from Dockerfile.build ..."
docker build -f (Join-Path $RepoRoot 'Dockerfile.build') -t $Image $RepoRoot
if ($LASTEXITCODE -ne 0) { throw "docker build failed ($LASTEXITCODE)" }

Write-Host 'Running cargo test --all as uid 1000 ...'
docker run --rm `
    --user 1000:1000 `
    -e HOME=/tmp/home `
    -e CARGO_HOME=/tmp/cargo `
    -e CARGO_TARGET_DIR=/tmp/target `
    -e GIT_CONFIG_COUNT=1 `
    -e GIT_CONFIG_KEY_0=safe.directory `
    -e GIT_CONFIG_VALUE_0=/workspace `
    -v "${RepoRoot}:/workspace:ro" `
    -w /workspace `
    $Image `
    cargo test --all @TestArgs

exit $LASTEXITCODE
