#!/usr/bin/env bash
# Print a CHANGELOG section for <version>, grouped by conventional-commit type.
#
# The release job prepends this to CHANGELOG.md so the file keeps a history now that
# release-please no longer writes one through a pull request. GitHub's generated release
# notes cover the release page; this covers the repository.
set -euo pipefail

VERSION=${1:-}
DATE=${2:-$(date -u +%Y-%m-%d)}
if [ -z "$VERSION" ]; then
  echo "release-notes: usage: release-notes.sh <version> [date]" >&2
  exit 2
fi

BASE_TAG=$(git tag -l 'v[0-9]*' --sort=-version:refname \
  | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -1 || true)

RANGE=()
if [ -n "$BASE_TAG" ]; then
  RANGE=("${BASE_TAG}..HEAD")
fi
SUBJECTS=$(git log "${RANGE[@]}" --format=%s)

# A group is capped: the first automatic release diffs against the last stable tag,
# which for this fork is hundreds of upstream commit subjects. The release page carries
# the full list; this file carries enough to read.
CAP=50

group() {
  local heading=$1 pattern=$2
  local lines total shown
  lines=$(grep -E "$pattern" <<<"$SUBJECTS" | sed 's/^/- /' || true)
  [ -z "$lines" ] && return 0
  total=$(printf '%s\n' "$lines" | wc -l | tr -d ' ')
  if [ "$total" -gt "$CAP" ]; then
    shown=$(printf '%s\n' "$lines" | head -n "$CAP")
    printf '### %s\n\n%s\n\n_…and %d more, listed in the GitHub release notes._\n\n' \
      "$heading" "$shown" "$((total - CAP))"
  else
    printf '### %s\n\n%s\n\n' "$heading" "$lines"
  fi
}

printf '## %s - %s\n\n' "$VERSION" "$DATE"
group "Breaking changes" '^[a-zA-Z][a-zA-Z0-9_-]*(\(.+\))?!:'
group "Features" '^feat(\(.+\))?:'
group "Bug fixes" '^(fix|perf)(\(.+\))?:'
group "Other changes" '^(docs|ci|chore|test|build|style|refactor)(\(.+\))?:'
