# Pi Hooks

> Part of [`hooks/`](../README.md)

## Design Intent

PTK's Pi extension is a **rewrite-only token optimizer**. It mutates bash commands to their
`ptk`-prefixed equivalents, so filtered output reaches the context.

**Permission gating is intentionally out of scope.** PTK does not block, confirm, or audit
commands — that concern belongs to a dedicated permission extension (e.g. one that gates
`rm -rf`, `sudo`, etc.). This separation keeps PTK's hook fast, predictable, and composable
with other Pi extensions.

## Specifics

- TypeScript extension using Pi's `ExtensionAPI` (not a shell hook, no `zx` dependency)
- Subscribes to `tool_call` event, narrows to `bash` tool
- Calls `ptk rewrite` via `pi.exec`; mutates `event.input.command` in-place if rewrite differs
- All error paths return `undefined` (pass through); PTK never blocks execution on its own
- If a ptk pipeline rule blocks the command, the extension returns `{ block: true, reason }` with the rule's message
- Version guard at load time: checks `ptk >= 0.23.0`; warns and registers no-op if too old or missing
- Install updates only current or known historical stock content. For modified or unrelated content it asks before overwriting; `--auto-patch` approves and `--no-patch` leaves the file unchanged. A declined protected update, including `--no-patch`, exits nonzero; in `--dry-run` mode it reports the prompt without changing files or creating directories. LF and CRLF line endings are treated as the same stock content.
- Installed to `.pi/extensions/ptk.ts` by `ptk init --agent pi` (project-local) or `~/.pi/agent/extensions/ptk.ts` by `ptk init --agent pi --global`

The same extension is shared with Oh My Pi (OMP) through OMP's `legacy-pi-compat` layer. OMP installs it at `.omp/extensions/ptk.ts` (project-local) or `~/.omp/agent/extensions/ptk.ts` (global). The global OMP path follows `PI_CODING_AGENT_DIR`, which OMP also honors. When the Pi and OMP paths in either scope resolve to the same file, PTK records Pi/OMP ownership in an adjacent hidden `.ptk-agents` state file. A valid sidecar is authoritative; missing or unreadable state is treated as uncertain, warned about, and not used to claim sole ownership. A definitively shared project or global file prompts before removal (`--auto-patch` approves it); an uncertain legacy or corrupt-state uninstall warns and proceeds, while `--no-patch` still leaves definitively shared files in place with a nonzero exit. PTK currently targets OMP's default profile and `.omp` project directory, not named profiles or custom `PI_CONFIG_DIR` locations.

## Uninstall

```bash
# Remove project-local install (run from the project root)
ptk init --uninstall --agent pi
# → removes .pi/extensions/ptk.ts

# Remove global install
ptk init --uninstall --agent pi --global
# → removes ~/.pi/agent/extensions/ptk.ts
```

Uninstalling an absent extension is a no-op. Current and known historical stock files are removed, while modified PTK content is preserved and causes a normal uninstall to fail with a manual-removal message; `--dry-run` previews that refusal. Unreadable content is left alone but causes a normal uninstall to fail nonzero; `--dry-run` reports it and succeeds. Unrelated content is left alone. When Pi and OMP paths in either scope resolve to the same file, a valid sidecar recording both agents makes uninstall ask before removing it; `--auto-patch` approves and `--no-patch` leaves it in place with a nonzero exit. Missing or unreadable ownership state produces a warning and proceeds without definitive shared-file protection. Only the extension file and PTK's adjacent ownership state are managed by install/uninstall.
