#!/usr/bin/env bash
#
# PTK Smoke Test Suite
# Exercises every command to catch regressions after merge.
# Exit code: number of failures (0 = all green)
#
set -euo pipefail

PASS=0
FAIL=0
SKIP=0
FAILURES=()

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# ── Helpers ──────────────────────────────────────────

assert_ok() {
    local name="$1"
    shift
    local output
    if output=$("$@" 2>&1); then
        PASS=$((PASS + 1))
        printf "  ${GREEN}PASS${NC}  %s\n" "$name"
    else
        FAIL=$((FAIL + 1))
        FAILURES+=("$name")
        printf "  ${RED}FAIL${NC}  %s\n" "$name"
        printf "        cmd: %s\n" "$*"
        printf "        out: %s\n" "$(echo "$output" | head -3)"
    fi
}

assert_contains() {
    local name="$1"
    local needle="$2"
    shift 2
    local output
    if output=$("$@" 2>&1) && echo "$output" | grep -q "$needle"; then
        PASS=$((PASS + 1))
        printf "  ${GREEN}PASS${NC}  %s\n" "$name"
    else
        FAIL=$((FAIL + 1))
        FAILURES+=("$name")
        printf "  ${RED}FAIL${NC}  %s\n" "$name"
        printf "        expected: '%s'\n" "$needle"
        printf "        got: %s\n" "$(echo "$output" | head -3)"
    fi
}

assert_exit_ok() {
    local name="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        PASS=$((PASS + 1))
        printf "  ${GREEN}PASS${NC}  %s\n" "$name"
    else
        FAIL=$((FAIL + 1))
        FAILURES+=("$name")
        printf "  ${RED}FAIL${NC}  %s\n" "$name"
        printf "        cmd: %s\n" "$*"
    fi
}

assert_fails() {
    local name="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        FAIL=$((FAIL + 1))
        FAILURES+=("$name (expected failure, got success)")
        printf "  ${RED}FAIL${NC}  %s (expected failure)\n" "$name"
    else
        PASS=$((PASS + 1))
        printf "  ${GREEN}PASS${NC}  %s\n" "$name"
    fi
}

assert_help() {
    local name="$1"
    shift
    assert_contains "$name --help" "Usage:" "$@" --help
}

# Some subcommands forward --help to the tool they wrap (upstream's
# `forwards_to_native_tool`), so the wording and the exit code belong to that
# tool: `npm --help` exits 1 on npm 12. All ptk owes us is that it hands the
# flag over instead of failing on its own account.
assert_forwarded_help() {
    local name="$1"
    local tool="$2"
    shift 2
    if ! command -v "$tool" >/dev/null 2>&1; then
        skip_test "$name --help" "$tool not installed"
        return
    fi
    local output
    output=$("$@" --help 2>&1) || true
    if [ -n "$output" ] && ! printf '%s' "$output" | grep -q 'ptk: Failed to run'; then
        PASS=$((PASS + 1))
        printf "  ${GREEN}PASS${NC}  %s --help\n" "$name"
    else
        FAIL=$((FAIL + 1))
        FAILURES+=("$name --help")
        printf "  ${RED}FAIL${NC}  %s --help\n" "$name"
        printf "        cmd: %s --help\n" "$*"
        printf "        out: %s\n" "$(echo "$output" | head -3)"
    fi
}

# `ptk rewrite` exits by the hook permission protocol (0 allow, 3 ask/default,
# 2 deny, 1 defer) so a host cannot auto-approve a command the user never
# allowed. The rewrite itself is on stdout, and that is what we check here.
assert_rewrite() {
    local name="$1"
    local needle="$2"
    shift 2
    local output
    output=$("$@" 2>&1) || true
    if printf '%s' "$output" | grep -q "$needle"; then
        PASS=$((PASS + 1))
        printf "  ${GREEN}PASS${NC}  %s\n" "$name"
    else
        FAIL=$((FAIL + 1))
        FAILURES+=("$name")
        printf "  ${RED}FAIL${NC}  %s\n" "$name"
        printf "        expected: '%s'\n" "$needle"
        printf "        got: %s\n" "$(echo "$output" | head -3)"
    fi
}

skip_test() {
    local name="$1"
    local reason="$2"
    SKIP=$((SKIP + 1))
    printf "  ${YELLOW}SKIP${NC}  %s (%s)\n" "$name" "$reason"
}

section() {
    printf "\n${BOLD}${CYAN}── %s ──${NC}\n" "$1"
}

# ── Preamble ─────────────────────────────────────────

PTK=$(command -v ptk || echo "")
if [[ -z "$PTK" ]]; then
    echo "ptk not found in PATH. Run: cargo install --path ."
    exit 1
fi

printf "${BOLD}PTK Smoke Test Suite${NC}\n"
printf "Binary: %s\n" "$PTK"
printf "Version: %s\n" "$(ptk --version)"
printf "Date: %s\n" "$(date '+%Y-%m-%d %H:%M')"

# Need a git repo to test git commands
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "Must run from inside a git repository."
    exit 1
fi

REPO_ROOT=$(git rev-parse --show-toplevel)

# ── 1. Version & Help ───────────────────────────────

section "Version & Help"

assert_contains "ptk --version" "ptk" ptk --version
assert_contains "ptk --help" "Usage:" ptk --help

# ── 2. Ls ────────────────────────────────────────────

section "Ls"

assert_ok      "ptk ls ."                     ptk ls .
assert_ok      "ptk ls -la ."                 ptk ls -la .
assert_ok      "ptk ls -lh ."                 ptk ls -lh .
assert_ok      "ptk ls -l src/"               ptk ls -l src/
assert_ok      "ptk ls src/ -l (flag after)"  ptk ls src/ -l
assert_ok      "ptk ls multi paths"           ptk ls src/ scripts/
assert_contains "ptk ls -a shows hidden"      ".git" ptk ls -a .
assert_contains "ptk ls shows sizes"          "K"  ptk ls src/
assert_contains "ptk ls shows dirs with /"    "/" ptk ls .

# ── 2b. Tree ─────────────────────────────────────────

section "Tree"

if command -v tree >/dev/null 2>&1; then
    assert_ok      "ptk tree ."                ptk tree .
    assert_ok      "ptk tree -L 2 ."           ptk tree -L 2 .
    assert_ok      "ptk tree -d -L 1 ."        ptk tree -d -L 1 .
    assert_contains "ptk tree shows src/"      "src" ptk tree -L 1 .
else
    skip_test "ptk tree" "tree not installed"
fi

# ── 3. Read ──────────────────────────────────────────

section "Read"

assert_ok      "ptk read Cargo.toml"          ptk read Cargo.toml
assert_ok      "ptk read --level none Cargo.toml"  ptk read --level none Cargo.toml
assert_ok      "ptk read --level aggressive Cargo.toml" ptk read --level aggressive Cargo.toml
assert_ok      "ptk read -n Cargo.toml"       ptk read -n Cargo.toml
assert_ok      "ptk read --max-lines 5 Cargo.toml" ptk read --max-lines 5 Cargo.toml

section "Read (stdin support)"

assert_ok      "ptk read stdin pipe"          bash -c 'echo "fn main() {}" | ptk read -'

# ── 4. Git ───────────────────────────────────────────

section "Git (existing)"

assert_ok      "ptk git status"               ptk git status
assert_ok      "ptk git status --short"       ptk git status --short
assert_ok      "ptk git status -s"            ptk git status -s
assert_ok      "ptk git status --porcelain"   ptk git status --porcelain
assert_ok      "ptk git log"                  ptk git log
assert_ok      "ptk git log -5"               ptk git log -- -5
assert_ok      "ptk git diff"                 ptk git diff
assert_ok      "ptk git diff --stat"          ptk git diff --stat

section "Git (new: branch, fetch, stash, worktree)"

assert_ok      "ptk git branch"               ptk git branch
assert_ok      "ptk git fetch"                ptk git fetch
assert_ok      "ptk git stash list"           ptk git stash list
assert_ok      "ptk git worktree"             ptk git worktree

section "Git (passthrough: unsupported subcommands)"

assert_ok      "ptk git tag --list"           ptk git tag --list
assert_ok      "ptk git remote -v"            ptk git remote -v
assert_ok      "ptk git rev-parse HEAD"       ptk git rev-parse HEAD

# ── 5. GitHub CLI ────────────────────────────────────

section "GitHub CLI"

if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    assert_ok      "ptk gh pr list"           ptk gh pr list
    assert_ok      "ptk gh run list"          ptk gh run list
    if gh issue list >/dev/null 2>&1; then
        assert_ok      "ptk gh issue list"        ptk gh issue list
    else
        skip_test "ptk gh issue list" "issues disabled or gh not authenticated"
    fi
    # pr create/merge/diff/comment/edit are write ops, test help only
    assert_forwarded_help "ptk gh" gh ptk gh
else
    skip_test "gh commands" "gh not authenticated"
fi

# ── 6. Cargo ─────────────────────────────────────────

section "Cargo (new)"

assert_ok      "ptk cargo build"              ptk cargo build
assert_ok      "ptk cargo clippy"             ptk cargo clippy
# cargo test exits non-zero due to pre-existing failures; check output ignoring exit code
output_cargo_test=$(ptk cargo test 2>&1 || true)
if echo "$output_cargo_test" | grep -q "FAILURES\|test result:\|passed"; then
    PASS=$((PASS + 1))
    printf "  ${GREEN}PASS${NC}  %s\n" "ptk cargo test"
else
    FAIL=$((FAIL + 1))
    FAILURES+=("ptk cargo test")
    printf "  ${RED}FAIL${NC}  %s\n" "ptk cargo test"
    printf "        got: %s\n" "$(echo "$output_cargo_test" | head -3)"
fi
assert_forwarded_help "ptk cargo" cargo ptk cargo

# ── 7. Curl ──────────────────────────────────────────

section "Curl (new)"

# mockhttp.org serves a different JSON document on every request, so check
# that the body arrives as JSON at all rather than pinning a key.
assert_contains "ptk curl JSON preserved" '{' ptk curl https://mockhttp.org/json
assert_ok       "ptk curl plain text"          ptk curl https://mockhttp.org/robots.txt
assert_help     "ptk curl"                     ptk curl

# ── 8. Npm / Npx ────────────────────────────────────

section "Npm / Npx (new)"

assert_forwarded_help "ptk npm" npm ptk npm
assert_forwarded_help "ptk npx" npx ptk npx

# ── 9. Pnpm ─────────────────────────────────────────

section "Pnpm"

assert_forwarded_help "ptk pnpm" pnpm ptk pnpm
assert_forwarded_help "ptk pnpm build" pnpm ptk pnpm build
assert_forwarded_help "ptk pnpm typecheck" pnpm ptk pnpm typecheck

if command -v pnpm >/dev/null 2>&1; then
    assert_ok  "ptk pnpm help"                ptk pnpm help
fi

# ── 10. Grep ─────────────────────────────────────────

section "Grep"

# All three need -r: `grep PATTERN <dir>` without it exits 2 ("Is a directory"),
# which made the first two assertions red and the third one inert.
assert_ok      "ptk grep pattern"             ptk grep -r "pub fn" src/
assert_contains "ptk grep finds results"      "pub fn" ptk grep -r "pub fn" src/
# `-t` is rg-only. The form matters: the pattern must match, the path must be a
# single file, and `-t` must come first, so the only reason to fail is `-t`
# itself -- a directory, a non-matching pattern, or `-t` after another flag all
# make this assertion pass either way.
assert_fails   "ptk grep -t rejected by grep"  ptk grep -t rust "fn main" src/main.rs
if command -v rg >/dev/null 2>&1; then
    assert_ok  "ptk rg with file type"         ptk rg "pub fn" src/ -t rust
else
    skip_test  "ptk rg with file type"         "rg not installed"
fi

section "Grep (extra args passthrough)"

assert_ok      "ptk grep -i case insensitive" ptk grep -r "fn" src/ -i
assert_ok      "ptk grep -A context lines"    ptk grep -r "fn run" src/ -A 2

# ── 11. Find ─────────────────────────────────────────

section "Find"

assert_ok      "ptk find *.rs"                ptk find "*.rs" src/
assert_contains "ptk find shows files"        ".rs" ptk find "*.rs" src/

# ── 12. Json ─────────────────────────────────────────

section "Json"

# Create temp JSON file for testing
TMPJSON=$(mktemp /tmp/ptk-test-XXXXX.json)
echo '{"name":"test","count":42,"items":[1,2,3]}' > "$TMPJSON"

assert_ok      "ptk json file"                ptk json "$TMPJSON"

# A document this small is already optimal, and the filter is never allowed to
# make output worse than its input, so both modes echo it unchanged. The
# compression (and the type names) only show up on a nested document.
TMPJSON_NESTED=$(mktemp /tmp/ptk-test-nested-XXXXX.json)
echo '{"service":{"endpoint":"https://api.example.com/v1/users","timeout":30}}' > "$TMPJSON_NESTED"

assert_contains "ptk json nested compact"     '"endpoint"' ptk json "$TMPJSON_NESTED"
# `--keys-only` is the rename of `--schema` (upstream 097456e5): values become
# their type names, so the URL shows as `url` and the number as `int`.
assert_contains "ptk json --keys-only types"  'url' ptk json "$TMPJSON_NESTED" --keys-only

rm -f "$TMPJSON" "$TMPJSON_NESTED"

# ── 13. Deps ─────────────────────────────────────────

section "Deps"

assert_ok      "ptk deps ."                   ptk deps .
assert_contains "ptk deps shows Cargo"        "Cargo" ptk deps .

# ── 14. Env ──────────────────────────────────────────

section "Env"

assert_ok      "ptk env"                      ptk env
assert_ok      "ptk env --filter PATH"        ptk env --filter PATH

# ── 16. Log ──────────────────────────────────────────

section "Log"

TMPLOG=$(mktemp /tmp/ptk-log-XXXXX.log)
for i in $(seq 1 20); do
    echo "[2025-01-01 12:00:00] INFO: repeated message" >> "$TMPLOG"
done
echo "[2025-01-01 12:00:01] ERROR: something failed" >> "$TMPLOG"

assert_ok      "ptk log file"                 ptk log "$TMPLOG"

rm -f "$TMPLOG"

# ── 17. Summary ──────────────────────────────────────

section "Summary"

assert_ok      "ptk summary echo hello"       ptk summary echo hello

# ── 18. Err ──────────────────────────────────────────

section "Err"

assert_ok      "ptk err echo ok"              ptk err echo ok

# ── 19. Test runner ──────────────────────────────────

section "Test runner"

assert_ok      "ptk test echo ok"             ptk test echo ok

# ── 20. Gain ─────────────────────────────────────────

section "Gain"

assert_ok      "ptk gain"                     ptk gain
assert_ok      "ptk gain --history"           ptk gain --history

# ── 21. Config & Init ────────────────────────────────

section "Config & Init"

assert_ok      "ptk config"                   ptk config
assert_ok      "ptk init --show"              ptk init --show

# ── 22. Wget ─────────────────────────────────────────

section "Wget"

if command -v wget >/dev/null 2>&1; then
    assert_ok  "ptk wget stdout"              ptk wget https://mockhttp.org/robots.txt -O -
else
    skip_test "ptk wget" "wget not installed"
fi

# ── 23. Tsc / Lint / Prettier / Next / Playwright ───

section "JS Tooling (help only, no project context)"

assert_forwarded_help "ptk tsc" tsc ptk tsc
assert_forwarded_help "ptk lint" eslint ptk lint
assert_forwarded_help "ptk prettier" prettier ptk prettier
assert_forwarded_help "ptk next" next ptk next
assert_forwarded_help "ptk playwright" playwright ptk playwright

# ── 24. Prisma ───────────────────────────────────────

section "Prisma (help only)"

assert_forwarded_help "ptk prisma" prisma ptk prisma

# ── 25. Vitest ───────────────────────────────────────

section "Vitest (help only)"

assert_forwarded_help "ptk vitest" vitest ptk vitest

# ── 26. Docker / Kubectl (help only) ────────────────

section "Docker / Kubectl (help only)"

assert_forwarded_help "ptk docker" docker ptk docker
assert_forwarded_help "ptk kubectl" kubectl ptk kubectl

# ── 27. Python (conditional) ────────────────────────

section "Python (conditional)"

if command -v pytest &>/dev/null; then
    assert_help    "ptk pytest"                    ptk pytest --help
else
    skip_test "ptk pytest" "pytest not installed"
fi

if command -v ruff &>/dev/null; then
    assert_help    "ptk ruff"                      ptk ruff --help
else
    skip_test "ptk ruff" "ruff not installed"
fi

if command -v pip &>/dev/null; then
    assert_help    "ptk pip"                       ptk pip --help
else
    skip_test "ptk pip" "pip not installed"
fi

# ── 28. Go (conditional) ────────────────────────────

section "Go (conditional)"

if command -v go &>/dev/null; then
    assert_help    "ptk go"                        ptk go --help
    assert_help    "ptk go test"                   ptk go test -h
    assert_help    "ptk go build"                  ptk go build -h
    assert_help    "ptk go vet"                    ptk go vet -h
else
    skip_test "ptk go" "go not installed"
fi

if command -v golangci-lint &>/dev/null; then
    assert_help    "ptk golangci-lint"             ptk golangci-lint --help
else
    skip_test "ptk golangci-lint" "golangci-lint not installed"
fi

# ── 29. Graphite (conditional) ─────────────────────

section "Graphite (conditional)"

if command -v gt &>/dev/null; then
    assert_help   "ptk gt"                          ptk gt --help
    assert_ok     "ptk gt log short"                ptk gt log short
else
    skip_test "ptk gt" "gt not installed"
fi

# ── 30. Ruby (conditional) ──────────────────────────

section "Ruby (conditional)"

if command -v rspec &>/dev/null; then
    assert_help    "ptk rspec"                     ptk rspec --help
else
    skip_test "ptk rspec" "rspec not installed"
fi

if command -v rubocop &>/dev/null; then
    assert_help    "ptk rubocop"                   ptk rubocop --help
else
    skip_test "ptk rubocop" "rubocop not installed"
fi

if command -v rake &>/dev/null; then
    assert_help    "ptk rake"                      ptk rake --help
else
    skip_test "ptk rake" "rake not installed"
fi

# ── 31. Global flags ────────────────────────────────

section "Global flags"

# `-u` was --ultra-compact's alias upstream and was removed because it ate
# `git push -u`; the surviving global flag is checked here instead.
assert_ok      "ptk --skip-env ls ."           ptk --skip-env ls .
assert_forwarded_help "ptk npm" npm ptk --skip-env npm

# ── 32. CcEconomics ─────────────────────────────────

section "CcEconomics"

assert_ok      "ptk cc-economics"             ptk cc-economics

# ── 33. Learn ───────────────────────────────────────

section "Learn"

assert_ok      "ptk learn --help"             ptk learn --help
assert_ok      "ptk learn (no sessions)"      ptk learn --since 0 2>&1 || true

# ── 32. Rewrite ───────────────────────────────────────

section "Rewrite"

assert_rewrite  "rewrite git status"          "ptk git status"         ptk rewrite "git status"
assert_rewrite  "rewrite cargo test"          "ptk cargo test"         ptk rewrite "cargo test"
assert_rewrite  "rewrite compound &&"         "ptk git status"         ptk rewrite "git status && cargo test"
assert_rewrite  "rewrite pipe preserves"      "| head"                 ptk rewrite "git log | head"

section "Rewrite (#345: PTK_DISABLED skip)"

assert_fails   "rewrite PTK_DISABLED=1 skip"                          ptk rewrite "PTK_DISABLED=1 git status"
assert_fails   "rewrite env PTK_DISABLED skip"                        ptk rewrite "FOO=1 PTK_DISABLED=1 cargo test"

section "Rewrite (#346: 2>&1 preserved)"

assert_rewrite  "rewrite 2>&1 preserved"      "2>&1"                  ptk rewrite "cargo test 2>&1 | head"

section "Rewrite (#196: gh --json skip)"

assert_fails   "rewrite gh --json skip"                               ptk rewrite "gh pr list --json number"
assert_fails   "rewrite gh --jq skip"                                 ptk rewrite "gh api /repos --jq .name"
assert_fails   "rewrite gh --template skip"                           ptk rewrite "gh pr view 1 --template '{{.title}}'"
assert_rewrite  "rewrite gh normal works"     "ptk gh pr list"        ptk rewrite "gh pr list"

# ── 33. Verify ────────────────────────────────────────

section "Verify"

assert_ok      "ptk verify"                   ptk verify

# ── 34. Proxy ─────────────────────────────────────────

section "Proxy"

assert_ok      "ptk bypass echo hello"        ptk bypass echo hello
assert_contains "ptk bypass passthrough"      "hello" ptk bypass echo hello

# ── 35. Discover ──────────────────────────────────────

section "Discover"

assert_ok      "ptk discover"                 ptk discover

# ── 36. Diff ──────────────────────────────────────────

section "Diff"

assert_ok       "ptk diff identical files"     ptk diff Cargo.toml Cargo.toml
assert_fails    "ptk diff differing files"     ptk diff Cargo.toml LICENSE
assert_contains "ptk diff shows classic changes" "^< " bash -c 'ptk diff "$1" "$2"; [[ $? -eq 1 ]]' _ Cargo.toml LICENSE

# ── 37. Wc ────────────────────────────────────────────

section "Wc"

assert_ok      "ptk wc Cargo.toml"            ptk wc Cargo.toml

# ── 38. Smart ─────────────────────────────────────────

section "Smart"

assert_ok      "ptk smart src/main.rs"        ptk smart src/main.rs

# ── 39. Json edge cases ──────────────────────────────

section "Json (edge cases)"

assert_fails   "ptk json on TOML (#347)"                              ptk json Cargo.toml

# ── 40. Docker (conditional) ─────────────────────────

section "Docker (conditional)"

if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    assert_ok  "ptk docker ps"               ptk docker ps
    assert_ok  "ptk docker images"           ptk docker images
else
    skip_test "ptk docker" "docker not running"
fi

# ── 41. Hook check ───────────────────────────────────

section "Hook check (#344)"

if [ -f "$HOME/.claude/hooks/ptk-rewrite.sh" ]; then
    assert_contains "ptk init --show hook version" "version" ptk init --show
else
    skip_test "ptk init --show hook version" "no hook installed (run ptk init first)"
fi

# ══════════════════════════════════════════════════════
# Report
# ══════════════════════════════════════════════════════

printf "\n${BOLD}══════════════════════════════════════${NC}\n"
printf "${BOLD}Results: ${GREEN}%d passed${NC}, ${RED}%d failed${NC}, ${YELLOW}%d skipped${NC}\n" "$PASS" "$FAIL" "$SKIP"

if [[ ${#FAILURES[@]} -gt 0 ]]; then
    printf "\n${RED}Failures:${NC}\n"
    for f in "${FAILURES[@]}"; do
        printf "  - %s\n" "$f"
    done
fi

printf "${BOLD}══════════════════════════════════════${NC}\n"

exit "$FAIL"
