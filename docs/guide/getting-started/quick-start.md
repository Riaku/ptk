---
title: Quick Start
description: Get PTK running in 5 minutes and see your first token savings
sidebar:
  order: 2
---

# Quick Start

This guide walks you through your first PTK commands after installation.

## Prerequisites

PTK is installed and verified:

```bash
ptk --version   # ptk x.y.z
ptk gain        # shows token savings dashboard
```

If not, see [Installation](./installation.md).

## Step 1: Initialize for your AI assistant

```bash
# For Claude Code (global — applies to all projects)
ptk init --global

# For a single project only (CLAUDE.md instructions, no hook)
cd /your/project && ptk init
```

`ptk init --global` installs the hook that automatically rewrites commands. Restart your AI assistant after this step.

Using something other than Claude Code? Every agent has its own flag:

```bash
ptk init -g --gemini            # Gemini CLI
ptk init -g --codex             # Codex (OpenAI)
ptk init -g --agent cursor      # Cursor
ptk init -g --agent windsurf    # Windsurf
ptk init --agent cline          # Cline / Roo Code
```

See [Supported agents](./supported-agents.md) for the full list, including per-agent uninstall and the project-local vs global distinction.

### Preview without writing: `--dry-run`

To see exactly what `init` would change before it touches anything, add `--dry-run`:

```bash
ptk init --global --dry-run
```

Every would-be file create/update/patch is printed with a `[dry-run] would ...` prefix, then a `[dry-run] Nothing written.` footer. Nothing on disk is modified, no settings.json is patched, and the telemetry consent prompt is skipped. Combine with `-v` to also print the full content PTK would write:

```bash
ptk init --global --dry-run -v
```

`--dry-run` works for every init flavour (`--agent cursor`, `--gemini`, `--codex`, `--copilot`, `--uninstall`, ...). It cannot be combined with `--show`.

## Step 2: Use your tools normally

Once the hook is installed, nothing changes in how you work. Your AI assistant runs commands as usual — the hook intercepts them transparently and rewrites them before execution.

For example, when Claude Code runs `cargo test`, the hook rewrites it to `ptk cargo test` before it executes. The LLM receives filtered output with only the failures — not 500 lines of passing tests. You never see or type `ptk`.

PTK covers all major ecosystems — Git, Cargo/Rust, JavaScript, Python, Go, Ruby, .NET, Docker/Kubernetes, and more. See [What PTK Optimizes](../resources/what-ptk-covers.md) for the full list.

## Step 3: Check your savings

After a few commands, see how much was saved:

```bash
ptk gain
```

Illustrative output:

```
Total commands : 10
Input tokens   : 50,000
Output tokens  : 5,000
Saved          : 45,000  (90.0%)
```

:::caution[The hook only fires on Bash tool calls]
Built-in agent tools like `Read`, `Grep` and `Glob` do not pass through the Bash hook, so they are not auto-rewritten. To get PTK's compact output for those workflows, either use shell commands (`cat`/`head`/`tail`, `rg`/`grep`, `find`) or call `ptk read`, `ptk grep` and `ptk find` directly.

The full edition can additionally compress an agent's own `Read` and MCP tool output through the PostToolUse hook — see [Pipelines](../pipelines/overview.md).
:::

Simple quoted shell wrappers are rewritten too, without changing the shell they name:

```bash
bash -c "head foo && grep -R bar ."
# → bash -c "ptk read foo --head-lines 10 && ptk grep -R bar ."
```

This covers exact `sh -c`, `bash -c`, `zsh -c` and `fish -c` wrappers around a quoted script. A `fish -c` script is read as fish, so fish's own syntax (`(cmd)` substitution, `and`/`or`/`end`) is left alone rather than rewritten under POSIX rules:

```bash
fish -c 'git status; cargo test'      # → fish -c 'ptk git status; ptk cargo test'
fish -c 'git status; and cargo test'  # unchanged
```

Shell expansion inside an outer double quote, extra shell options, redirects to files and nested wrappers pass through unchanged.

## Step 4: Unsupported commands

Commands PTK doesn't recognize run through passthrough — output is unchanged, usage is tracked:

```bash
ptk bypass make install
```

## Next steps

- [What PTK Optimizes](../resources/what-ptk-covers.md) — all supported commands and savings by ecosystem
- [Supported agents](./supported-agents.md) — Claude Code, Cursor, Copilot, and more
- [Configuration](./configuration.md) — customize PTK behavior
- [Token Savings Analytics](../analytics/gain.md) — the full `ptk gain` surface
- [Editions and Licensing](../resources/editions.md) — what each download contains
