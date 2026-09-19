# PTK — Proxy Token Killer

A high-performance CLI proxy that minimizes LLM token consumption by filtering and compressing command outputs before they reach your agent's context. Single Rust binary, cross-platform (Windows, macOS, Linux), 100+ supported commands, <10ms overhead.

PTK is a fork of [rtk-ai/rtk](https://github.com/rtk-ai/rtk) by [Patrick Szymkowiak](https://github.com/pszymkowiak). It retains all of RTK's cross-platform and POSIX capabilities and adds native support for **Windows PowerShell cmdlets** (`Get-ChildItem`, `Get-Content`, `Select-String`, `Get-Process`, ...) and **Windows CMD built-ins** (`dir`, `type`, `findstr`, `where`), closing long-standing gaps for Windows developers and AI coding agents.

This fork is maintained independently and distributed **only** via [GitHub Releases](https://github.com/Riaku/ptk/releases). Upstream distribution channels belong to the original project.

## Installation

### Pre-built Binaries

Download from [releases](https://github.com/Riaku/ptk/releases):

- Windows: `ptk-x86_64-pc-windows-msvc.zip`
- macOS: `ptk-x86_64-apple-darwin.tar.gz` / `ptk-aarch64-apple-darwin.tar.gz`
- Linux: `ptk-x86_64-unknown-linux-musl.tar.gz` / `ptk-aarch64-unknown-linux-gnu.tar.gz`

> **Windows users**: extract the zip and place `ptk.exe` somewhere on your PATH (e.g. `C:\Users\<you>\.local\bin`). Run it from a terminal — do not double-click the `.exe`.

GitHub Releases is the only supported install method for end users. Developers and contributors can also build from source:

```bash
cargo install --git https://github.com/Riaku/ptk
```

### Verify

```bash
ptk --version
ptk gain        # savings dashboard
```

## Quick Start

```bash
# Install the auto-rewrite hook for your AI tool
ptk init -g                     # Claude Code / Copilot (default)
ptk init -g --gemini            # Gemini CLI
ptk init -g --codex             # Codex (OpenAI)
ptk init -g --agent cursor      # Cursor
ptk init --agent cline          # Cline / Roo Code
ptk init -g --opencode          # OpenCode

# Restart your AI tool, then test
git status                      # automatically rewritten to ptk git status
```

The hook intercepts Bash/shell tool calls and rewrites them to `ptk` equivalents before execution, so the agent gets compact output without needing to call `ptk` explicitly. Built-in agent tools like `Read`/`Grep`/`Glob` bypass the hook — use `ptk read`, `ptk grep`, `ptk find` directly there.

## What It Does

Four strategies applied per command type:

1. **Smart Filtering** — removes noise (banners, whitespace, boilerplate)
2. **Grouping** — aggregates similar items (files by directory, errors by rule)
3. **Truncation** — keeps relevant context, cuts redundancy
4. **Deduplication** — collapses repeated log lines with counts

Examples:

```bash
ptk ls .                        # Compact directory tree
ptk read file.rs -l aggressive  # Signatures only
ptk grep "pattern" .            # Grouped search results
ptk git status                  # Compact, grouped by state
ptk git push                    # -> "ok main"
ptk cargo test                  # Failures only, passes collapsed to a count
ptk pytest                      # Failures only, trimmed tracebacks
ptk docker ps                   # Essential fields only
ptk Get-ChildItem               # Compact Mode/Size/Name list
ptk Get-Process                 # Minimal PID/WS/CPU/Name columns
ptk proxy <command>             # Raw passthrough + tracking
```

When a command fails, the full unfiltered output is saved for recall without re-executing:

```
FAILED: 2/15 tests
[full output: ptk recall 3f9c2a81d4e7]
```

## Savings Disclaimer

PTK cuts **up to 90% of the bash output** your agent reads — that is what it measures, not your bill. Bash output is one contributor to input tokens, and input tokens are only part of the bill. Token counts are estimated as `bytes / 4` (no tokenizer shipped), so percentages are reliable but absolute numbers are approximate.

## Configuration

`~/.config/ptk/config.toml` (Windows: `%APPDATA%\ptk\config.toml`):

```toml
[hooks]
exclude_commands = ["curl", "playwright"]  # skip rewrite for these
```

Telemetry is **disabled by default** and requires explicit opt-in (`ptk telemetry enable`). Manage with `ptk telemetry status|enable|disable|forget`, or block regardless of consent with `export RTK_TELEMETRY_DISABLED=1`.

## Credits

- **[RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk)** — the original project this fork is based on, created by [Patrick Szymkowiak](https://github.com/pszymkowiak) and the RTK core team. All the core filtering engine, hook system, and multi-ecosystem support come from their work.

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details. See [DISCLAIMER.md](DISCLAIMER.md).
