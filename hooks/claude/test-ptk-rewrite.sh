#!/usr/bin/env bash
# Test suite for ptk-rewrite.sh
# Feeds mock JSON through the hook and verifies the rewritten commands.
#
# Usage: bash ~/.claude/hooks/test-ptk-rewrite.sh

HOOK="${HOOK:-$HOME/.claude/hooks/ptk-rewrite.sh}"
PASS=0
FAIL=0
TOTAL=0

# Colors
GREEN='\033[32m'
RED='\033[31m'
DIM='\033[2m'
RESET='\033[0m'

test_rewrite() {
  local description="$1"
  local input_cmd="$2"
  local expected_cmd="$3"  # empty string = expect no rewrite
  TOTAL=$((TOTAL + 1))

  local input_json
  input_json=$(jq -n --arg cmd "$input_cmd" '{"tool_name":"Bash","tool_input":{"command":$cmd}}')
  local output
  output=$(echo "$input_json" | bash "$HOOK" 2>/dev/null) || true

  if [ -z "$expected_cmd" ]; then
    # Expect no rewrite (hook exits 0 with no output)
    if [ -z "$output" ]; then
      printf "  ${GREEN}PASS${RESET} %s ${DIM}→ (no rewrite)${RESET}\n" "$description"
      PASS=$((PASS + 1))
    else
      local actual
      actual=$(echo "$output" | jq -r '.hookSpecificOutput.updatedInput.command // empty')
      printf "  ${RED}FAIL${RESET} %s\n" "$description"
      printf "       expected: (no rewrite)\n"
      printf "       actual:   %s\n" "$actual"
      FAIL=$((FAIL + 1))
    fi
  else
    local actual
    actual=$(echo "$output" | jq -r '.hookSpecificOutput.updatedInput.command // empty' 2>/dev/null)
    if [ "$actual" = "$expected_cmd" ]; then
      printf "  ${GREEN}PASS${RESET} %s ${DIM}→ %s${RESET}\n" "$description" "$actual"
      PASS=$((PASS + 1))
    else
      printf "  ${RED}FAIL${RESET} %s\n" "$description"
      printf "       expected: %s\n" "$expected_cmd"
      printf "       actual:   %s\n" "$actual"
      FAIL=$((FAIL + 1))
    fi
  fi
}

echo "============================================"
echo "  PTK Rewrite Hook Test Suite"
echo "============================================"
echo ""

# ---- SECTION 1: Existing patterns (regression tests) ----
echo "--- Existing patterns (regression) ---"
test_rewrite "git status" \
  "git status" \
  "ptk git status"

test_rewrite "git log --oneline -10" \
  "git log --oneline -10" \
  "ptk git log --oneline -10"

test_rewrite "git diff HEAD" \
  "git diff HEAD" \
  "ptk git diff HEAD"

test_rewrite "git show abc123" \
  "git show abc123" \
  "ptk git show abc123"

test_rewrite "git add ." \
  "git add ." \
  "ptk git add ."

test_rewrite "gh pr list" \
  "gh pr list" \
  "ptk gh pr list"

test_rewrite "npx playwright test" \
  "npx playwright test" \
  "ptk playwright test"

test_rewrite "ls -la" \
  "ls -la" \
  "ptk ls -la"

test_rewrite "curl -s https://example.com" \
  "curl -s https://example.com" \
  "ptk curl -s https://example.com"

test_rewrite "cat package.json" \
  "cat package.json" \
  "ptk read package.json"

test_rewrite "grep -rn pattern src/" \
  "grep -rn pattern src/" \
  "ptk grep -rn pattern src/"

test_rewrite "rg pattern src/" \
  "rg pattern src/" \
  "ptk grep pattern src/"

test_rewrite "cargo test" \
  "cargo test" \
  "ptk cargo test"

test_rewrite "npx prisma migrate" \
  "npx prisma migrate" \
  "ptk prisma migrate"

test_rewrite "ptk git status" \
  "ptk git status" \
  "ptk git status"

echo ""

# ---- SECTION 2: Env var prefix handling (THE BIG FIX) ----
echo "--- Env var prefix handling (new) ---"
test_rewrite "env + playwright" \
  "TEST_SESSION_ID=2 npx playwright test --config=foo" \
  "TEST_SESSION_ID=2 ptk playwright test --config=foo"

test_rewrite "env + git status" \
  "GIT_PAGER=cat git status" \
  "GIT_PAGER=cat ptk git status"

test_rewrite "env + git log" \
  "GIT_PAGER=cat git log --oneline -10" \
  "GIT_PAGER=cat ptk git log --oneline -10"

test_rewrite "multi env + vitest" \
  "NODE_ENV=test CI=1 npx vitest" \
  "NODE_ENV=test CI=1 ptk vitest"

test_rewrite "env + ls" \
  "LANG=C ls -la" \
  "LANG=C ptk ls -la"

test_rewrite "env + npm run" \
  "NODE_ENV=test npm run test:e2e" \
  "NODE_ENV=test ptk npm run test:e2e"

test_rewrite "env + docker compose (unsupported subcommand, NOT rewritten)" \
  "COMPOSE_PROJECT_NAME=test docker compose up -d" \
  ""

test_rewrite "env + docker compose logs (supported, rewritten)" \
  "COMPOSE_PROJECT_NAME=test docker compose logs web" \
  "COMPOSE_PROJECT_NAME=test ptk docker compose logs web"

echo ""

# ---- SECTION 3: New patterns ----
echo "--- New patterns ---"
test_rewrite "npm run test:e2e" \
  "npm run test:e2e" \
  "ptk npm run test:e2e"

test_rewrite "npm run build" \
  "npm run build" \
  "ptk npm run build"

test_rewrite "npm jest run" \
  "npm jest run" \
  "ptk jest"

test_rewrite "docker compose up -d (NOT rewritten — unsupported by ptk)" \
  "docker compose up -d" \
  ""

test_rewrite "docker compose logs postgrest" \
  "docker compose logs postgrest" \
  "ptk docker compose logs postgrest"

test_rewrite "docker compose ps" \
  "docker compose ps" \
  "ptk docker compose ps"

test_rewrite "docker compose build" \
  "docker compose build" \
  "ptk docker compose build"

test_rewrite "docker compose down (NOT rewritten — unsupported by ptk)" \
  "docker compose down" \
  ""

test_rewrite "docker compose -f file.yml up (NOT rewritten — flag before subcommand)" \
  "docker compose -f docker-compose.preview.yml --project-name myapp up -d --build" \
  ""

test_rewrite "docker run --rm postgres" \
  "docker run --rm postgres" \
  "ptk docker run --rm postgres"

test_rewrite "docker exec -it db psql" \
  "docker exec -it db psql" \
  "ptk docker exec -it db psql"

test_rewrite "find . -name '*.ts'" \
  "find . -name '*.ts'" \
  "ptk find . -name '*.ts'"

test_rewrite "tree src/" \
  "tree src/" \
  "ptk tree src/"

test_rewrite "wget https://example.com/file" \
  "wget https://example.com/file" \
  "ptk wget https://example.com/file"

test_rewrite "gh api repos/owner/repo" \
  "gh api repos/owner/repo" \
  "ptk gh api repos/owner/repo"

test_rewrite "gh release list" \
  "gh release list" \
  "ptk gh release list"

test_rewrite "kubectl describe pod foo" \
  "kubectl describe pod foo" \
  "ptk kubectl describe pod foo"

test_rewrite "kubectl apply -f deploy.yaml" \
  "kubectl apply -f deploy.yaml" \
  "ptk kubectl apply -f deploy.yaml"

echo ""

# ---- SECTION 3b: PTK_DISABLED and redirect fixes (#345, #346) ----
echo "--- PTK_DISABLED (#345) ---"
test_rewrite "PTK_DISABLED=1 git status (no rewrite)" \
  "PTK_DISABLED=1 git status" \
  ""

test_rewrite "PTK_DISABLED=1 cargo test (no rewrite)" \
  "PTK_DISABLED=1 cargo test" \
  ""

test_rewrite "FOO=1 PTK_DISABLED=1 git status (no rewrite)" \
  "FOO=1 PTK_DISABLED=1 git status" \
  ""

echo ""
echo "--- Redirect operators (#346) ---"
test_rewrite "cargo test 2>&1 | head" \
  "cargo test 2>&1 | head" \
  "ptk cargo test 2>&1 | head"

test_rewrite "cargo test 2>&1" \
  "cargo test 2>&1" \
  "ptk cargo test 2>&1"

test_rewrite "cargo test &>/dev/null" \
  "cargo test &>/dev/null" \
  "ptk cargo test &>/dev/null"

# Note: the bash hook rewrites only the first command segment (sed-based);
# full compound rewriting (both sides of &) is handled by `ptk rewrite` (Rust).
# The critical behavior tested here: `&` after `cargo test` is NOT mistaken for
# a redirect — the hook still rewrites cargo test, no crash.
test_rewrite "cargo test & git status (bash hook rewrites first segment only)" \
  "cargo test & git status" \
  "ptk cargo test & git status"

echo ""

# ---- SECTION 4: Vitest edge case (fixed double "run" bug) ----
echo "--- Vitest run dedup ---"
test_rewrite "vitest (no args)" \
  "vitest" \
  "ptk vitest"

test_rewrite "vitest run (no run)" \
  "vitest run" \
  "ptk vitest"

test_rewrite "vitest --reporter" \
  "vitest --reporter=verbose" \
  "ptk vitest --reporter=verbose"

test_rewrite "npx vitest" \
  "npx vitest" \
  "ptk vitest"

test_rewrite "pnpm vitest --coverage" \
  "pnpm vitest --coverage" \
  "ptk vitest --coverage"

echo ""

# ---- SECTION 5: Should NOT rewrite ----
echo "--- Should NOT rewrite ---"
test_rewrite "heredoc" \
  "cat <<'EOF'
hello
EOF" \
  ""

test_rewrite "echo (no pattern)" \
  "echo hello world" \
  ""

test_rewrite "cd (no pattern)" \
  "cd /tmp" \
  ""

test_rewrite "mkdir (no pattern)" \
  "mkdir -p foo/bar" \
  ""

test_rewrite "python3 (no pattern)" \
  "python3 script.py" \
  ""

test_rewrite "node (no pattern)" \
  "node -e 'console.log(1)'" \
  ""

echo ""

# ---- SECTION 6: Audit logging ----
echo "--- Audit logging (PTK_HOOK_AUDIT=1) ---"

AUDIT_TMPDIR=$(mktemp -d)
trap "rm -rf $AUDIT_TMPDIR" EXIT

test_audit_log() {
  local description="$1"
  local input_cmd="$2"
  local expected_action="$3"
  TOTAL=$((TOTAL + 1))

  # Clean log
  rm -f "$AUDIT_TMPDIR/hook-audit.log"

  local input_json
  input_json=$(jq -n --arg cmd "$input_cmd" '{"tool_name":"Bash","tool_input":{"command":$cmd}}')
  echo "$input_json" | PTK_HOOK_AUDIT=1 PTK_AUDIT_DIR="$AUDIT_TMPDIR" bash "$HOOK" 2>/dev/null || true

  if [ ! -f "$AUDIT_TMPDIR/hook-audit.log" ]; then
    printf "  ${RED}FAIL${RESET} %s (no log file created)\n" "$description"
    FAIL=$((FAIL + 1))
    return
  fi

  local log_line
  log_line=$(head -1 "$AUDIT_TMPDIR/hook-audit.log")
  local actual_action
  actual_action=$(echo "$log_line" | cut -d'|' -f2 | tr -d ' ')

  if [ "$actual_action" = "$expected_action" ]; then
    printf "  ${GREEN}PASS${RESET} %s ${DIM}→ %s${RESET}\n" "$description" "$actual_action"
    PASS=$((PASS + 1))
  else
    printf "  ${RED}FAIL${RESET} %s\n" "$description"
    printf "       expected action: %s\n" "$expected_action"
    printf "       actual action:   %s\n" "$actual_action"
    printf "       log line:        %s\n" "$log_line"
    FAIL=$((FAIL + 1))
  fi
}

test_audit_log "audit: rewrite git status" \
  "git status" \
  "rewrite"

test_audit_log "audit: skip already_ptk" \
  "ptk git status" \
  "skip:already_ptk"

test_audit_log "audit: skip heredoc" \
  "cat <<'EOF'
hello
EOF" \
  "skip:heredoc"

test_audit_log "audit: skip no_match" \
  "echo hello world" \
  "skip:no_match"

test_audit_log "audit: rewrite cargo test" \
  "cargo test" \
  "rewrite"

# Test log format (4 pipe-separated fields)
rm -f "$AUDIT_TMPDIR/hook-audit.log"
input_json=$(jq -n --arg cmd "git status" '{"tool_name":"Bash","tool_input":{"command":$cmd}}')
echo "$input_json" | PTK_HOOK_AUDIT=1 PTK_AUDIT_DIR="$AUDIT_TMPDIR" bash "$HOOK" 2>/dev/null || true
TOTAL=$((TOTAL + 1))
log_line=$(cat "$AUDIT_TMPDIR/hook-audit.log" 2>/dev/null || echo "")
field_count=$(echo "$log_line" | awk -F' \\| ' '{print NF}')
if [ "$field_count" = "4" ]; then
  printf "  ${GREEN}PASS${RESET} audit: log format has 4 fields ${DIM}→ %s${RESET}\n" "$log_line"
  PASS=$((PASS + 1))
else
  printf "  ${RED}FAIL${RESET} audit: log format (expected 4 fields, got %s)\n" "$field_count"
  printf "       log line: %s\n" "$log_line"
  FAIL=$((FAIL + 1))
fi

# Test no log when PTK_HOOK_AUDIT is unset
rm -f "$AUDIT_TMPDIR/hook-audit.log"
input_json=$(jq -n --arg cmd "git status" '{"tool_name":"Bash","tool_input":{"command":$cmd}}')
echo "$input_json" | PTK_AUDIT_DIR="$AUDIT_TMPDIR" bash "$HOOK" 2>/dev/null || true
TOTAL=$((TOTAL + 1))
if [ ! -f "$AUDIT_TMPDIR/hook-audit.log" ]; then
  printf "  ${GREEN}PASS${RESET} audit: no log when PTK_HOOK_AUDIT unset\n"
  PASS=$((PASS + 1))
else
  printf "  ${RED}FAIL${RESET} audit: log created when PTK_HOOK_AUDIT unset\n"
  FAIL=$((FAIL + 1))
fi

echo ""

# ---- SECTION: inherited PTK_REWRITE_HOST must not relax this hook ----
# This hook turns exit 0 into an explicit allow, and PTK_REWRITE_HOST is the
# channel that turns PTK's default ask into exit 0. An inherited value (a shell
# rc, .envrc, or CI env) must therefore be scrubbed before the hook calls
# `ptk rewrite`.
if command -v ptk &>/dev/null; then
  echo "--- Inherited host name (PTK_REWRITE_HOST) ---"
  HOST_TMP=$(mktemp -d)
  mkdir -p "$HOST_TMP/home/.config" "$HOST_TMP/claude" "$HOST_TMP/cache" "$HOST_TMP/project/.claude"
  printf '{"permissions":{}}' >"$HOST_TMP/claude/settings.json"
  host_input=$(jq -n --arg cmd "git status" '{"tool_name":"Bash","tool_input":{"command":$cmd}}')
  host_output=$(
    cd "$HOST_TMP/project" &&
      echo "$host_input" |
      HOME="$HOST_TMP/home" \
        CLAUDE_CONFIG_DIR="$HOST_TMP/claude" \
        XDG_CONFIG_HOME="$HOST_TMP/home/.config" \
        XDG_CACHE_HOME="$HOST_TMP/cache" \
        PTK_REWRITE_HOST=openclaw \
        bash "$HOOK" 2>/dev/null
  ) || true
  TOTAL=$((TOTAL + 1))
  host_decision=$(echo "$host_output" | jq -r '.hookSpecificOutput.permissionDecision // empty' 2>/dev/null)
  host_cmd=$(echo "$host_output" | jq -r '.hookSpecificOutput.updatedInput.command // empty' 2>/dev/null)
  if [ "$host_cmd" = "ptk git status" ] && [ "$host_decision" != "allow" ]; then
    printf "  ${GREEN}PASS${RESET} host: inherited PTK_REWRITE_HOST does not auto-allow\n"
    PASS=$((PASS + 1))
  else
    printf "  ${RED}FAIL${RESET} host: inherited PTK_REWRITE_HOST auto-allowed a rewrite\n"
    printf "       expected: ptk git status with no permissionDecision\n"
    printf "       actual:   %s (decision: %s)\n" "$host_cmd" "${host_decision:-none}"
    FAIL=$((FAIL + 1))
  fi
  rm -rf "$HOST_TMP"
  echo ""
fi

# ---- SUMMARY ----
echo "============================================"
if [ $FAIL -eq 0 ]; then
  printf "  ${GREEN}ALL $TOTAL TESTS PASSED${RESET}\n"
else
  printf "  ${RED}$FAIL FAILED${RESET} / $TOTAL total ($PASS passed)\n"
fi
echo "============================================"

exit $FAIL
