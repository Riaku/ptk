# PTK

Prefix every shell command with `PTK`: `PTK git status`, `PTK cargo test`,
`PTK npm run build`, `PTK ls src/`. Keep the prefix inside chains:
`PTK git add . && PTK git commit -m "msg"`. Commands PTK has no filter for
run as-is, so the prefix is always safe.

# Command output

Command output here is condensed to save tokens, keeping every signal and
dropping costly noise. Treat it as the complete result: run commands
normally, and batch related commands into one call to avoid extra turns.
Truncated results state their recovery path in their own output. Re-run a
command as `PTK proxy <cmd>` only when its result is unusable: empty when
output was clearly expected, contradicting its exit code, or garbled.

## About PTK

PTK (Rust Token Killer) is a CLI proxy that filters command output to save
tokens; behavior and exit code are unchanged.

- `PTK gain` / `PTK gain --history` — token savings, overall and per command.
- `PTK proxy <cmd>` — run a command unfiltered, still tracked.
- `RTK_DISABLED=1 <cmd>` — skip PTK for one command.
- `PTK discover` — find past commands PTK could have condensed.
