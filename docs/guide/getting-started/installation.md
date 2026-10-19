---
title: Installation
description: Install PTK via curl, Homebrew, winget, Cargo, or from source, and verify the correct version
sidebar:
  order: 1
---

# Installation

## Name collision warning

Two unrelated projects share the name `ptk`. Make sure you install the right one:

- **Rust Token Killer** (`rtk-ai/ptk`) — this project, a token-saving CLI proxy
- **Rust Type Kit** (`reachingforthejack/ptk`) — a different tool for generating Rust types

The easiest way to verify you have the correct one: run `ptk gain`. It should display the savings dashboard. If it returns "command not found", you either have the wrong package or PTK is not installed.

## Check before installing

```bash
ptk --version   # should print: ptk x.y.z
ptk gain        # should show the savings dashboard
```

If both commands work, PTK is already installed. Skip to [Project initialization](#project-initialization).

## Quick install (Linux and macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/rtk-ai/ptk/master/install.sh | sh
```

Installs to `~/.local/bin`. Add it to your PATH if it is not there already:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc   # or ~/.zshrc
```

## Homebrew (macOS and Linux)

```bash
brew install rtk-ai/tap/ptk
```

## winget (Windows)

```powershell
winget install rtk-ai.ptk
```

## Cargo

:::caution[Name collision risk]
`cargo install ptk` may install **Rust Type Kit** instead of Rust Token Killer — two unrelated projects share the same crate name. Use the explicit Git URL to guarantee the correct package:
:::

```bash
cargo install --git https://github.com/rtk-ai/ptk --branch master ptk
```

## Pre-built binaries (Windows, Linux, macOS)

Download from [GitHub releases](https://github.com/rtk-ai/ptk/releases):

- macOS: `ptk-x86_64-apple-darwin.tar.gz` / `ptk-aarch64-apple-darwin.tar.gz`
- Linux: `ptk-x86_64-unknown-linux-musl.tar.gz` / `ptk-aarch64-unknown-linux-gnu.tar.gz`
- Windows: `ptk-x86_64-pc-windows-msvc.zip`

**Windows users**: Extract the zip and place `ptk.exe` in a directory on your PATH. Run PTK from Command Prompt, PowerShell, or Windows Terminal — do not double-click the `.exe` (it prints usage and exits immediately). See [Windows](#windows) below — the hook system works natively, no WSL required.

## Apache-2.0 edition

The same `ptk` binary built without the BUSL-1.1 compression engine — the hook rewrite system and every command filter, under Apache-2.0 alone. Pick this if your policy does not allow a source-available licence in the build.

```bash
curl -fsSL https://raw.githubusercontent.com/rtk-ai/ptk/master/install.sh | PTK_EDITION=oss sh
brew install rtk-ai/tap/ptk-oss
cargo install --git https://github.com/rtk-ai/ptk --branch master ptk --no-default-features
```

Release assets are named `ptk-oss-<target>`, with `ptk-oss` Debian and RPM packages alongside. The binary is still called `ptk`, so hooks and scripts are unchanged — but the two editions conflict and cannot be installed together.

See [Editions and Licensing](../resources/editions.md) for what the edition omits and where the licence boundary falls.

## Windows

PTK works fully on native Windows. Since **v0.37.2** the auto-rewrite hook runs as a native binary command (`ptk hook claude`) — no Unix shell, bash, or jq required — so commands are rewritten transparently in Command Prompt, PowerShell, and Windows Terminal, exactly as on Linux and macOS.

### Native Windows (manual install)

Prefer [winget](#winget-windows) if you can — it handles PATH for you.

```powershell
# 1. Download and extract ptk-x86_64-pc-windows-msvc.zip from releases
# 2. Add ptk.exe to your PATH (e.g. C:\Users\<you>\.local\bin)
# 3. Initialize - installs the native binary hook
ptk init -g
```

**Upgrading from an older install?** If you set PTK up before v0.37.2 you may still have the legacy `ptk-rewrite.sh` shell hook, which does need a Unix shell. Re-run `ptk init -g` to migrate to the native binary hook.

**Prerequisites**: some filters shell out to [ripgrep](https://github.com/BurntSushi/ripgrep) (`rg`). Install it and keep it on your PATH to avoid `Binary 'rg' not found on PATH` warnings:

```powershell
winget install BurntSushi.ripgrep.MSVC
```

### WSL

[WSL](https://learn.microsoft.com/en-us/windows/wsl/install) also works and behaves exactly like Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/rtk-ai/ptk/master/install.sh | sh
ptk init -g
```

| Feature | Native Windows | WSL |
|---|---|---|
| Filters (cargo, git, …) | Full | Full |
| Auto-rewrite hook | Yes (native binary) | Yes |
| `ptk init -g` | Hook mode | Hook mode |
| `ptk gain` / analytics | Full | Full |

## Verify installation

```bash
ptk --version   # ptk x.y.z
ptk gain        # savings dashboard
```

If `ptk gain` fails but `ptk --version` succeeds, you installed Rust Type Kit by mistake. Uninstall it first:

```bash
cargo uninstall ptk
```

Then reinstall using one of the methods above.

## Project initialization

For a single project, `ptk init` adds the PTK block to `./CLAUDE.md` and a `.ptk/filters.toml` template, without installing a hook:

```bash
ptk init
```

To install the Claude Code hook (registered in `~/.claude/settings.json`, applies to every project):

```bash
ptk init --global
```

## Uninstall

```bash
ptk init -g --uninstall    # remove hook, PTK.md, and settings.json entry
cargo uninstall ptk         # remove binary (if installed via Cargo)
brew uninstall ptk          # remove binary (if installed via Homebrew)
winget uninstall rtk-ai.ptk # remove binary (if installed via winget)
```
