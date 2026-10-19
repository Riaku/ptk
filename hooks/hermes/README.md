# PTK Plugin for Hermes

Rewrites Hermes `terminal` tool commands to PTK equivalents before execution, so Hermes receives compact command output without changing your workflow.

## Installation

```bash
ptk init --agent hermes
```

The installer writes the plugin to `~/.hermes/plugins/ptk-rewrite/` and enables it through `plugins.enabled` in the Hermes config.

## How it works

Hermes loads plugins from Python, so the plugin entrypoint is Python. The Python code is only a thin Hermes adapter. It reads the Hermes terminal tool payload, calls `ptk rewrite` for the actual command decision, then mutates the terminal tool `command` before Hermes executes it.

All rewrite rules stay in Rust inside `ptk rewrite`. When PTK adds or changes command rewrite behavior, the Hermes plugin picks up that behavior by delegating to the PTK binary.

## Fail-open behavior

The plugin does not block command execution. If anything goes wrong, Hermes runs the original command unchanged.

If ptk is not available in PATH when Hermes loads the plugin, the plugin prints a warning and skips hook registration.

- `ptk` is missing from `PATH`
- `ptk rewrite` exits with an error
- Hermes sends a non-terminal tool call
- The tool payload has no string `command`
- The plugin raises an unexpected exception

## Limitations

- Only Hermes `terminal` tool calls are rewritten.
- Commands skipped by `ptk rewrite` stay unchanged, including commands already prefixed with `ptk`, compound shell commands, heredocs, and commands without an PTK filter.
- Shell hooks are not used for Hermes command rewriting. The integration depends on Hermes loading Python plugins and passing a mutable terminal tool payload.
