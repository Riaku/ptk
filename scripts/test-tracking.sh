#!/usr/bin/env bash
# Test tracking end-to-end: run commands, verify they appear in ptk gain --history
set -euo pipefail

# Workaround for macOS bash pipe handling in strict mode
set +e  # Allow errors in pipe chains to continue

PASS=0; FAIL=0; FAILURES=()
RED='\033[0;31m'; GREEN='\033[0;32m'; NC='\033[0m'

check() {
    local name="$1" needle="$2"
    shift 2
    local output
    if output=$("$@" 2>&1) && echo "$output" | grep -q "$needle"; then
        PASS=$((PASS+1)); printf "  ${GREEN}PASS${NC}  %s\n" "$name"
    else
        FAIL=$((FAIL+1)); FAILURES+=("$name")
        printf "  ${RED}FAIL${NC}  %s\n" "$name"
        printf "        expected: '%s'\n" "$needle"
        printf "        got: %s\n" "$(echo "$output" | head -3)"
    fi
}

echo "═══ PTK Tracking Validation ═══"
echo ""

# 1. Commandes avec filtrage réel — doivent apparaitre dans history
echo "── Optimized commands (token savings) ──"
ptk ls . >/dev/null 2>&1
check "ptk ls tracked" "ptk ls" ptk gain --history

ptk git status >/dev/null 2>&1
check "ptk git status tracked" "ptk git status" ptk gain --history

ptk git log -5 >/dev/null 2>&1
check "ptk git log tracked" "ptk git log" ptk gain --history

# Git passthrough (timing-only)
echo ""
echo "── Passthrough commands (timing-only) ──"
ptk git tag --list >/dev/null 2>&1
check "git passthrough tracked" "git tag --list" ptk gain --history

# gh commands (if authenticated)
echo ""
echo "── GitHub CLI tracking ──"
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    ptk gh pr list >/dev/null 2>&1 || true
    check "ptk gh pr list tracked" "ptk gh pr" ptk gain --history

    ptk gh run list >/dev/null 2>&1 || true
    check "ptk gh run list tracked" "ptk gh run" ptk gain --history
else
    echo "  SKIP  gh (not authenticated)"
fi

# Stdin commands
echo ""
echo "── Stdin commands ──"
echo -e "line1\nline2\nline1\nERROR: bad\nline1" | ptk log >/dev/null 2>&1
check "ptk log stdin tracked" "ptk log" ptk gain --history

# Summary — verify passthrough doesn't dilute
echo ""
echo "── Summary integrity ──"
output=$(ptk gain 2>&1)
if echo "$output" | grep -q "Tokens saved"; then
    PASS=$((PASS+1)); printf "  ${GREEN}PASS${NC}  ptk gain summary works\n"
else
    FAIL=$((FAIL+1)); printf "  ${RED}FAIL${NC}  ptk gain summary\n"
fi

echo ""
echo "═══ Results: ${PASS} passed, ${FAIL} failed ═══"
if [ ${#FAILURES[@]} -gt 0 ]; then
    echo "Failures: ${FAILURES[*]}"
fi
exit $FAIL
