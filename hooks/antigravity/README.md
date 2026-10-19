# Google Antigravity Hooks

> Part of [`hooks/`](../README.md)

## Specifics

- Native programmatic `PreToolUse` lifecycle hook (`ptk hook antigravity`) providing transparent command rewriting via `overwrite.CommandLine`
- Supported across all Antigravity surfaces: Antigravity CLI (`agy`), Antigravity IDE, and Antigravity 2.0
- Installs the awareness file selected by `awareness.level` as `rules/AGENTS.md` inside the plugin (plain markdown, applied whenever the plugin is active)
- Uses Antigravity's modular Plugin architecture:
  - Local workspace: `.agents/plugins/ptk/`
  - Global scope: `~/.gemini/config/plugins/ptk/`
- Full lifecycle support:
  - Setup: `ptk init --agent antigravity` (or `ptk init -g --agent antigravity`)
  - Preview: `ptk init --agent antigravity --dry-run`
  - Uninstallation: `ptk init --agent antigravity --uninstall` (or `ptk init -g --agent antigravity --uninstall`)

## Permission Evaluation Note

Antigravity evaluates its tool permissions *after* lifecycle hooks rewrite commands. If you maintain strict command allowlists, ensure permitted commands account for `ptk` (e.g. `command(ptk git status)` or `command(ptk *)`).
