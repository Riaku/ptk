# Installation

PTK is distributed as pre-built binaries and via Cargo. This fork does **not** use Homebrew, winget, or any other package manager — install from [GitHub Releases](https://github.com/Riaku/ptk/releases) or from source.

> PTK is a fork of [rtk-ai/rtk](https://github.com/rtk-ai/rtk). Upstream install channels belong to the original project.

## Pre-built Binaries

Download the archive for your platform from [releases](https://github.com/Riaku/ptk/releases):

| Platform | Asset |
|----------|-------|
| Windows (x64) | `ptk-x86_64-pc-windows-msvc.zip` |
| macOS (Intel) | `ptk-x86_64-apple-darwin.tar.gz` |
| macOS (Apple Silicon) | `ptk-aarch64-apple-darwin.tar.gz` |
| Linux (x64, static) | `ptk-x86_64-unknown-linux-musl.tar.gz` |
| Linux (ARM64) | `ptk-aarch64-unknown-linux-gnu.tar.gz` |

Each release includes `checksums.txt`. Verify before installing:

```bash
sha256sum --check --ignore-missing checksums.txt   # Linux
shasum -a 256 --check --ignore-missing checksums.txt  # macOS
```

### Linux / macOS

```bash
tar -xzf ptk-<target>.tar.gz
mkdir -p ~/.local/bin
mv ptk ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"   # add to ~/.bashrc or ~/.zshrc
```

### Windows

1. Extract `ptk-x86_64-pc-windows-msvc.zip`
2. Place `ptk.exe` in a directory on your PATH (e.g. `C:\Users\<you>\.local\bin`)
3. Run from Command Prompt, PowerShell, or Windows Terminal — **do not double-click the `.exe`** (it prints usage and exits)

> Some filters shell out to [ripgrep](https://github.com/BurntSushi/ripgrep) (`rg`). Keep it on your PATH to avoid `Binary 'rg' not found on PATH` warnings.

## Cargo

```bash
cargo install --git https://github.com/Riaku/ptk
```

Or from a clone:

```bash
git clone https://github.com/Riaku/ptk.git
cd ptk
cargo install --path .
```

Requires Rust 1.91+ (edition 2024).

## Verify Installation

```bash
ptk --version   # Should show "ptk <version>"
ptk gain        # Should show the savings dashboard
```

> **Name collision warning**: another project named "PTK" (Rust Type Kit, reachingforthejack/PTK) exists. If `ptk gain` fails, you have the wrong package — reinstall via `cargo install --git` above.

## AI Tool Setup

After installing the binary, set up the auto-rewrite hook for your agent:

```bash
ptk init -g                     # Claude Code / Copilot (default)
ptk init -g --gemini            # Gemini CLI
ptk init -g --codex             # Codex (OpenAI)
ptk init -g --agent cursor      # Cursor
ptk init --agent cline          # Cline / Roo Code
ptk init -g --opencode          # OpenCode
ptk init --show                 # Verify installation
```

Restart your AI tool after install. See the module docs in `hooks/` for per-agent details.

## WSL

WSL behaves exactly like Linux — use the Linux binary or Cargo instructions above inside WSL.

## Uninstall

```bash
ptk init -g --uninstall         # Remove hook, RTK.md, settings entries
cargo uninstall ptk             # Remove binary (if installed via Cargo)
rm ~/.local/bin/ptk             # Remove binary (if installed manually)
```

Data lives in `~/.config/ptk/` (macOS: `~/Library/Application Support/ptk/`, Windows: `%APPDATA%\ptk\`) — delete it to clear all local state, or run `ptk telemetry forget` first if telemetry was enabled.
