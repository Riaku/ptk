#!/usr/bin/env bash
# ptk-hook-version: 1
# PTK Cursor Agent hook — rewrites shell commands to use ptk for token savings.
# Works with both Cursor editor and cursor-cli (they share ~/.cursor/hooks.json).
# Cursor preToolUse hook format: receives JSON on stdin, returns JSON on stdout.
# Requires: ptk >= 0.23.0, jq
#
# This is a thin delegating hook: all rewrite logic lives in `ptk rewrite`,
# which is the single source of truth (src/discover/registry.rs).
# To add or change rewrite rules, edit the Rust registry — not this file.

if ! command -v jq &>/dev/null; then
  echo "[ptk] WARNING: jq is not installed. Hook cannot rewrite commands. Install jq: https://jqlang.github.io/jq/download/" >&2
  exit 0
fi

if ! command -v ptk &>/dev/null; then
  echo "[ptk] WARNING: ptk is not installed or not in PATH. Hook cannot rewrite commands. Install: https://github.com/riaku/ptk#installation" >&2
  exit 0
fi

# Version guard: ptk rewrite was added in 0.23.0.
PTK_VERSION=$(ptk --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
if [ -n "$PTK_VERSION" ]; then
  MAJOR=$(echo "$PTK_VERSION" | cut -d. -f1)
  MINOR=$(echo "$PTK_VERSION" | cut -d. -f2)
  if [ "$MAJOR" -eq 0 ] && [ "$MINOR" -lt 23 ]; then
    echo "[ptk] WARNING: ptk $PTK_VERSION is too old (need >= 0.23.0). Upgrade: curl -fsSL https://raw.githubusercontent.com/riaku/ptk/refs/heads/main/install.sh | sh" >&2
    exit 0
  fi
fi

INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

if [ -z "$CMD" ]; then
  echo '{}'
  exit 0
fi

# Delegate all rewrite logic to the Rust binary.
# Exit codes: 0 = allow rewrite, 1 = no rewrite (passthrough),
#             2 = deny, 3 = ask.
# Scrub PTK_REWRITE_HOST: that channel relaxes PTK's approval gate, and this
# hook turns exit 0 into an explicit allow, so an inherited value (a shell rc,
# .envrc, or CI env) must not reach it. Only the delegate that sets it may
# rely on it.
REWRITTEN=$(env -u PTK_REWRITE_HOST ptk rewrite "$CMD" 2>/dev/null)
RC=$?
if [ "$RC" -ne 0 ] && [ "$RC" -ne 3 ]; then
  echo '{}'
  exit 0
fi

# No change — nothing to do.
if [ -z "$REWRITTEN" ] || [ "$CMD" = "$REWRITTEN" ]; then
  echo '{}'
  exit 0
fi

# RC 3 = ask (not enforced by Cursor yet, but future-proof).
PERMISSION="allow"
if [ "$RC" -eq 3 ]; then
  PERMISSION="ask"
fi

jq -n --arg cmd "$REWRITTEN" --arg perm "$PERMISSION" '{
  "continue": true,
  "permission": $perm,
  "updated_input": { "command": $cmd }
}'
