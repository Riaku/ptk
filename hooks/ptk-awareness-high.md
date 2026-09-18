# Command output

Command output here is condensed to save tokens, keeping every signal and
dropping costly noise. Treat it as the complete result: run commands
normally, and batch related commands into one call to avoid extra turns.
Truncated results state their recovery path in their own output. Re-run a
command as `ptk proxy <cmd>` only when its result is unusable: empty when
output was clearly expected, contradicting its exit code, or garbled.

## About PTK

The condensing is done by PTK (Rust Token Killer), a CLI proxy. A hook
rewrites each shell command to `ptk <cmd>` before it runs; behavior and
exit code are unchanged, only the output is filtered. Commands PTK has no
filter for run as-is.

- `ptk gain` / `ptk gain --history` — token savings, overall and per command.
- `ptk proxy <cmd>` — run a command unfiltered, still tracked.
- `PTK_DISABLED=1 <cmd>` — skip the hook for one command.
- `ptk discover` — find past commands PTK could have condensed.
