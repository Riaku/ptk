# PTK — Copilot Integration (VS Code Copilot Chat + Copilot CLI)

**Usage**: Token-optimized CLI proxy (cuts up to 90% of bash output)

## What's automatic

The `.github/copilot-instructions.md` file is loaded at session start by both Copilot CLI and VS Code Copilot Chat.
It instructs Copilot to prefix commands with `PTK` automatically.

The `.github/hooks/rtk-rewrite.json` hook adds a `PreToolUse` safety net via `PTK hook` —
a cross-platform Rust binary that intercepts raw bash tool calls and rewrites them.
No shell scripts, no `jq` dependency, works on Windows natively.

## Meta commands (always use directly)

```bash
PTK gain              # Token savings dashboard for this session
PTK gain --history    # Per-command history with savings %
PTK discover          # Scan session history for missed PTK opportunities
PTK proxy <cmd>       # Run raw (no filtering) but still track it
```

## Installation verification

```bash
PTK --version   # Should print: PTK X.Y.Z
PTK gain        # Should show a dashboard (not "command not found")
which PTK       # Verify correct binary path
```

> ⚠️ **Name collision**: If `PTK gain` fails, you may have `reachingforthejack/PTK`
> (Rust Type Kit) installed instead. Check `which PTK` and reinstall from Riaku/ptk.

## How the hook works

`PTK hook` reads `PreToolUse` JSON from stdin, detects the agent format, and responds appropriately:

**VS Code Copilot Chat** (supports `updatedInput` — transparent rewrite, no denial):
1. Agent runs `git status` → `PTK hook` intercepts via `PreToolUse`
2. `PTK hook` detects VS Code format (`tool_name`/`tool_input` keys)
3. Returns `hookSpecificOutput.updatedInput.command = "PTK git status"`
4. Agent runs the rewritten command silently — no denial, no retry

**GitHub Copilot CLI** (deny-with-suggestion — CLI ignores `updatedInput` today, see [issue #2013](https://github.com/github/copilot-cli/issues/2013)):
1. Agent runs `git status` → `PTK hook` intercepts via `PreToolUse`
2. `PTK hook` detects Copilot CLI format (`toolName`/`toolArgs` keys)
3. Returns `permissionDecision: deny` with reason: `"Token savings: use 'PTK git status' instead"`
4. Copilot reads the reason and re-runs `PTK git status`

When Copilot CLI adds `updatedInput` support, only `PTK hook` needs updating — no config changes.

## Integration comparison

| Tool                  | Mechanism                               | Hook output              | File                               |
|-----------------------|-----------------------------------------|--------------------------|------------------------------------|
| Claude Code           | `PreToolUse` hook with `updatedInput`   | Transparent rewrite      | `hooks/rtk-rewrite.sh`             |
| VS Code Copilot Chat  | `PreToolUse` hook with `updatedInput`   | Transparent rewrite      | `.github/hooks/rtk-rewrite.json`   |
| GitHub Copilot CLI    | `PreToolUse` deny-with-suggestion       | Denial + retry           | `.github/hooks/rtk-rewrite.json`   |
| OpenCode              | Plugin `tool.execute.before`            | Transparent rewrite      | `hooks/opencode-PTK.ts`            |
| (any)                 | Custom instructions                     | Prompt-level guidance    | `.github/copilot-instructions.md`  |
