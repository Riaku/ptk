# Kilo Code Hooks

> Part of [`hooks/`](../README.md) — see also [`src/hooks/`](../../src/hooks/README.md) for installation code

## Specifics

- Prompt-level guidance only (no programmatic hook) -- relies on Kilo Code reading custom instructions
- Installs `../PTK-awareness-full.md` (shared, agent-neutral): the instruction to prefix every shell command with `PTK`, plus meta commands. No hook, so the `full` level is always used regardless of `awareness.level`
- Installed to `.kilocode/rules/PTK-rules.md` (project-local) by `ptk init --agent kilocode`
