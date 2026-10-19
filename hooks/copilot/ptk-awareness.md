# PTK — Copilot Integration (VS Code Copilot Chat + Copilot CLI)

**Usage**: Token-optimized CLI proxy

## What's automatic

The `.github/copilot-instructions.md` file is loaded at session start by both Copilot CLI and VS Code Copilot Chat.
It instructs Copilot to prefix commands with `ptk` automatically.

The `.github/hooks/ptk-rewrite.json` hook adds a `PreToolUse` safety net via `ptk hook` —
a cross-platform Rust binary that intercepts raw bash tool calls and rewrites them.
No shell scripts, no `jq` dependency, works on Windows natively.

## Meta commands (always use directly)

```bash
ptk gain              # Token savings dashboard for this session
ptk gain --history    # Per-command history with savings %
ptk discover          # Scan session history for missed ptk opportunities
ptk bypass <cmd>       # Run raw (no filtering) but still track it
```

## Installation verification

```bash
ptk --version   # Should print: ptk X.Y.Z
ptk gain        # Should show a dashboard (not "command not found")
which ptk       # Verify correct binary path
```

## How the hook works

`ptk hook` reads `PreToolUse` JSON from stdin, detects the agent format, and responds appropriately:

**VS Code Copilot Chat** (supports `updatedInput` — transparent rewrite, no denial):
1. Agent runs `git status` → `ptk hook` intercepts via `PreToolUse`
2. `ptk hook` detects VS Code format (`tool_name`/`tool_input` keys)
3. Returns `hookSpecificOutput.updatedInput.command = "ptk git status"`
4. Agent runs the rewritten command silently — no denial, no retry

**GitHub Copilot CLI** (deny-with-suggestion — CLI ignores `updatedInput` today, see [issue #2013](https://github.com/github/copilot-cli/issues/2013)):
1. Agent runs `git status` → `ptk hook` intercepts via `PreToolUse`
2. `ptk hook` detects Copilot CLI format (`toolName`/`toolArgs` keys)
3. Returns `permissionDecision: deny` with reason: `"Token savings: use 'ptk git status' instead"`
4. Copilot reads the reason and re-runs `ptk git status`
