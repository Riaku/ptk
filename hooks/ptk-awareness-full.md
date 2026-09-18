# PTK

Prefix every shell command with `ptk`: `ptk git status`, `ptk cargo test`,
`ptk npm run build`, `ptk ls src/`. Keep the prefix inside chains:
`ptk git add . && ptk git commit -m "msg"`. Commands PTK has no filter for
run as-is, so the prefix is always safe.

# Command output

Command output here is condensed to save tokens, keeping every signal and
dropping costly noise. Treat it as the complete result: run commands
normally, and batch related commands into one call to avoid extra turns.
Truncated results state their recovery path in their own output. Re-run a
command as `ptk proxy <cmd>` only when its result is unusable: empty when
output was clearly expected, contradicting its exit code, or garbled.

## About PTK

PTK (Rust Token Killer) is a CLI proxy that filters command output to save
tokens; behavior and exit code are unchanged.

- `ptk gain` / `ptk gain --history` — token savings, overall and per command.
- `ptk proxy <cmd>` — run a command unfiltered, still tracked.
- `PTK_DISABLED=1 <cmd>` — skip PTK for one command.
- `ptk discover` — find past commands PTK could have condensed.
