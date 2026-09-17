#!/usr/bin/env bash
#
# PTK Docker Test Runner
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
#   scripts/test-docker.sh            # build image + run all tests
#   scripts/test-docker.sh <args>     # extra args forwarded to `cargo test`
#
set -euo pipefail

IMAGE="ptk-build"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Building ${IMAGE} from Dockerfile.build ..."
docker build -f "${REPO_ROOT}/Dockerfile.build" -t "${IMAGE}" "${REPO_ROOT}"

echo "Running cargo test --all as uid 1000 ..."
docker run --rm \
    --user 1000:1000 \
    -e HOME=/tmp/home \
    -e CARGO_HOME=/tmp/cargo \
    -e CARGO_TARGET_DIR=/tmp/target \
    -e GIT_CONFIG_COUNT=1 \
    -e GIT_CONFIG_KEY_0=safe.directory \
    -e GIT_CONFIG_VALUE_0=/workspace \
    -v "${REPO_ROOT}:/workspace:ro" \
    -w /workspace \
    "${IMAGE}" \
    cargo test --all "$@"
