#!/usr/bin/env bash
# Write <version> into the [package] section of every manifest in the release.
#
# The versions are hand-maintained rather than inherited from
# [workspace.package], so a release touches five files; doing it here means the tagged
# tree and the version its binaries report cannot disagree. Only the first `version =`
# inside [package] is rewritten — dependency requirements such as
# `toon-format = { version = "0.5.0" }` live in their own tables and must not move.
set -euo pipefail

VERSION=${1:-}
if ! printf '%s' "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "set-version: expected a bare semver version, got '${VERSION}'" >&2
  exit 2
fi

MANIFESTS=(
  Cargo.toml
  engine/Cargo.toml
  plugin/Cargo.toml
  text/Cargo.toml
  proxy/Cargo.toml
)

for manifest in "${MANIFESTS[@]}"; do
  if [ ! -f "$manifest" ]; then
    echo "set-version: $manifest is missing" >&2
    exit 1
  fi
  tmp=$(mktemp)
  awk -v version="$VERSION" '
    /^\[/ { section = $0 }
    section == "[package]" && !done && /^version = "/ {
      print "version = \"" version "\""
      done = 1
      next
    }
    { print }
  ' "$manifest" >"$tmp"
  mv "$tmp" "$manifest"
  if ! grep -q "version = \"$VERSION\"" "$manifest"; then
    echo "set-version: $manifest still does not say $VERSION" >&2
    exit 1
  fi
done
