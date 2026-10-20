---
title: Troubleshooting
description: Common PTK issues and how to fix them
sidebar:
  order: 3
---

# Troubleshooting

## `ptk gain` says "not a ptk command"

**Symptom:**
```bash
$ ptk gain
ptk: 'gain' is not a ptk command. See 'ptk --help'.
```

**Cause:** the `ptk` on your PATH is a stale build of this fork. It is not a name collision --
the unrelated **Rust Type Kit** (`reachingforthejack/rtk`) installs a binary called `rtk`, not
`ptk`.

**Fix:**
```bash
cargo uninstall ptk
curl -fsSL https://raw.githubusercontent.com/riaku/ptk/main/install.sh | sh
ptk gain    # should now show token savings stats
```

## How to tell which ptk you have

| If `ptk gain`... | You have |
|------------------|----------|
| Shows token savings dashboard | Rust Token Killer ✅ |
| Returns "not a ptk command" | A stale build of this fork ❌ |

## AI assistant not using PTK

**Symptom:** Claude Code (or another agent) runs `cargo test` instead of `ptk cargo test`.

**Checklist:**

1. Verify PTK is installed:
   ```bash
   ptk --version
   ptk gain
   ```

2. Initialize the hook:
   ```bash
   ptk init --global    # Claude Code
   ptk init --global --agent cursor    # Cursor
   ptk init --global --opencode  # OpenCode
   ```

3. Restart your AI assistant.

4. Verify hook status:
   ```bash
   ptk init --show
   ```

5. Check `settings.json` has the hook registered (Claude Code):
   ```bash
   cat ~/.claude/settings.json | grep ptk
   ```

## PTK not found after `cargo install`

**Symptom:**
```bash
$ ptk --version
zsh: command not found: ptk
```

**Cause:** `~/.cargo/bin` is not in your PATH.

**Fix:**

For bash (`~/.bashrc`) or zsh (`~/.zshrc`):
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

For fish (`~/.config/fish/config.fish`):
```fish
set -gx PATH $HOME/.cargo/bin $PATH
```

Then reload:
```bash
source ~/.zshrc    # or ~/.bashrc
ptk --version
```

## PTK on Windows

### Double-clicking ptk.exe does nothing

**Symptom:** You double-click `ptk.exe`, a terminal flashes and closes instantly.

**Cause:** PTK is a command-line tool. With no arguments, it prints usage and exits. The console window opens and closes before you can read anything.

**Fix:** Open a terminal first, then run PTK from there:
- Press `Win+R`, type `cmd`, press Enter
- Or open PowerShell or Windows Terminal
- Then run: `ptk --version`

### Hook not working (no auto-rewrite)

**Symptom:** On native Windows, commands are not auto-rewritten. An older `ptk init -g` printed "Falling back to --claude-md mode".

**Cause:** Before v0.37.2, `ptk init -g` registered no hook on native Windows: it fell back to injecting the full PTK instructions into `~/.claude/CLAUDE.md`. A setup made then still has no hook.

**Fix:** Upgrade to v0.37.2 or later, where `ptk init -g` registers the auto-rewrite hook on Windows as a native binary command (`ptk hook claude`). No Unix shell, bash, or jq is required. Re-run `ptk init -g`: it replaces the CLAUDE.md block with an `@PTK.md` reference and, once you confirm, adds `ptk hook claude` to `settings.json`. A legacy `~/.claude/hooks/ptk-rewrite.sh` hook, if one exists, is deleted along with its `.ptk-hook.sha256` and its `settings.json` entry.

Answer `y` when it asks to patch `settings.json`. Outside a terminal it cannot ask and defaults to `N`, so use `--auto-patch` there:

```powershell
ptk init -g                # answer y at the settings.json prompt
ptk init -g --auto-patch   # or: patch settings.json without asking
```

The default `N` leaves no hook registered: any legacy `ptk-rewrite.sh` entry is removed and `ptk hook claude` is not added. The `PTK hook registered (global).` banner prints either way; the line that confirms the patch is `settings.json: hook added` (or `settings.json: hook already present` when an earlier run added it). Restart Claude Code, then confirm:

```powershell
ptk init --show
```

It should report `[ok] Hook: ptk hook claude (native binary command)`.

[WSL](https://learn.microsoft.com/en-us/windows/wsl/install) also works and behaves like Linux if you prefer it.

### Node.js tools not found

**Symptom:**
```
ptk vitest --run
Error: program not found
```

**Cause:** On Windows, Node.js tools are installed as `.CMD`/`.BAT` wrappers. Older PTK versions couldn't find them.

**Fix:** Update to PTK v0.23.1+:
```bash
cargo install --git https://github.com/riaku/ptk --branch main ptk
ptk --version    # should be 0.23.1+
```

## Compilation error during installation

```bash
rustup update stable
rustup default stable
cargo clean
cargo build --release
cargo install --path . --force
```

Minimum required Rust version: 1.91 (edition 2024 needs Cargo 1.85 or newer).

## OpenCode not using PTK

```bash
ptk init --global --opencode
# restart OpenCode
ptk init --show    # should show "OpenCode: plugin installed"
```

The plugin needs OpenCode 1.18.29 or newer. With an older version, `ptk init`
installs nothing and prints the version it found, when it can read
`opencode --version`. Upgrade OpenCode rather than looking for a rewrite
setting.

If the plugin loads but nothing changes, check that `ptk` is on the `PATH`
OpenCode itself sees. The plugin only resolves it from `PATH`, because
OpenCode's shell tool does not source `.profile`, `.bashrc` or `.bash_profile`
and a binary found elsewhere cannot be spawned as a bare `ptk`. Set
`PTK_DISABLED=1` to confirm which way round the behaviour goes.

## `cargo install ptk` fails or installs nothing

This fork is not published to crates.io and no crate named `ptk` exists, so `cargo install ptk` errors out. (The Rust Type Kit is crate `rtk`, not `ptk`.)

Always use the explicit URL, pinned to the release branch:

```bash
cargo install --git https://github.com/riaku/ptk --branch main ptk
```

## Does PTK break Claude's prompt cache?

No. PTK filters command output once, at execution time. The filtered result is written into the
conversation history and never changes afterwards, and prompt caching matches on a stable prefix
— PTK does not rewrite anything the cache has already seen.

Smaller tool results also make caching cheaper: cache writes bill at 1.25x and cache reads at
0.1x the input rate, so fewer tokens in means less to write once and less to re-read every turn.

To see your own cache write and read volumes next to PTK's savings:

```bash
ptk cc-economics
```

`ptk gain` reports token savings only; the cache breakdown lives in `ptk cc-economics`.

## Run the diagnostic script

From the PTK repository root:

```bash
bash scripts/check-installation.sh
```

Checks:
- PTK installed and in PATH
- Correct version (Token Killer, not Type Kit)
- Available features
- Claude Code integration
- Hook status

## Still stuck?

Open an issue: https://github.com/riaku/ptk/issues
