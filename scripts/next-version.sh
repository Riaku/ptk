#!/usr/bin/env bash
# Print the next stable version, derived from the conventional commits since the last
# stable tag. Prints nothing and exits 3 when the range holds nothing releasable, which
# is how the release job knows a docs-only push is not a release.
#
# The rules match what release-please used to do from its manifest, minus the pull
# request: breaking marks bump the major, except below 1.0 where there is no stable
# surface to break and semver says bump the minor. `fix` and `perf` bump the patch;
# `feat` bumps the minor. Everything else (docs, ci, chore, test, build, style, and the
# upstream merge commits) is not a release.
set -euo pipefail

# Stable tags only: `dev-<version>-rc.<n>` are throwaway builds, and measuring from one
# renumbers the stable line.
base_tag() {
  git tag -l 'v[0-9]*' --sort=-version:refname \
    | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' \
    | head -1 || true
}

BASE_TAG=$(base_tag)
if [ -n "$BASE_TAG" ]; then
  BASE_VERSION=${BASE_TAG#v}
  set -- "${BASE_TAG}..HEAD"
else
  # A fork with no release yet has no tag to measure from. Start at the manifest
  # version rather than failing, and measure the whole history.
  BASE_VERSION=$(sed -n 's/^version = "\([^"]*\)".*/\1/p' Cargo.toml | head -1)
  set --
fi

if ! printf '%s' "$BASE_VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "next-version: cannot read a base version (got '${BASE_VERSION}')" >&2
  exit 1
fi

# Positional params rather than an array: macOS ships bash 3.2, where expanding an empty
# array under `set -u` is an unbound variable, and the no-tag branch has no range at all.
SUBJECTS=$(git log "$@" --format=%s)
BODIES=$(git log "$@" --format=%B)

has() { grep -qE "$1" <<<"$2"; }

breaking=0
feat=0
fix=0
has '^[a-zA-Z][a-zA-Z0-9_-]*(\(.+\))?!:' "$SUBJECTS" && breaking=1
has 'BREAKING[ -]CHANGE' "$BODIES" && breaking=1
has '^feat(\(.+\))?:' "$SUBJECTS" && feat=1
has '^(fix|perf)(\(.+\))?:' "$SUBJECTS" && fix=1

IFS='.' read -r MAJOR MINOR PATCH <<<"$BASE_VERSION"

if [ "$breaking" -eq 1 ]; then
  if [ "$MAJOR" -eq 0 ]; then
    MINOR=$((MINOR + 1)); PATCH=0
  else
    MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0
  fi
elif [ "$feat" -eq 1 ]; then
  MINOR=$((MINOR + 1)); PATCH=0
elif [ "$fix" -eq 1 ]; then
  PATCH=$((PATCH + 1))
else
  # Nothing releasable. Not an error: most pushes are documentation or CI.
  exit 3
fi

echo "${MAJOR}.${MINOR}.${PATCH}"
