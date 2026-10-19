# PTK Installation Guide - For AI Coding Assistants

## ⚠️ Name Collision Warning

**Two projects get confused with this one — only the first is a token optimizer:**

1. ✅ **Rust Token Killer** (this project) - LLM token optimizer
   - Repos: `riaku/ptk`
   - Has `ptk gain` command showing the savings dashboard

2. ℹ️ **Rust Type Kit** (`reachingforthejack/rtk`) - DIFFERENT PROJECT
   - Rust codebase query tool and type generator
   - Installs a binary called `rtk`, so it cannot be mistaken for this fork's `ptk`
   - **DO NOT install if you want token optimization**

## Pre-Installation Check (REQUIRED)

**AI assistants should ALWAYS verify if PTK is already installed before attempting installation.**

```bash
# Check if PTK is installed
ptk --version

# CRITICAL: Verify it's the Token Killer (not Type Kit)
ptk gain    # Should show the savings dashboard, NOT "command not found"

# Check installation path
which ptk
```

If `ptk gain` works, you have the **correct** PTK installed. **DO NOT reinstall**. Skip to "Project Initialization".

If `ptk gain` fails but `ptk --version` succeeds, you have the **wrong** PTK (Type Kit). Uninstall and reinstall the correct one (see below).

## Installation (only if PTK not available or wrong PTK installed)

### Step 0: Uninstall Wrong PTK (if needed)

If you accidentally installed Rust Type Kit:

```bash
cargo uninstall ptk
```

### Quick Install (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/riaku/ptk/develop/install.sh | sh
```

After installation, **verify you have the correct ptk**:
```bash
ptk gain  # Must show the savings dashboard (not "command not found")
```

### Quick Install (Windows)

Download `ptk-x86_64-pc-windows-msvc.zip` from the
[releases page](https://github.com/riaku/ptk/releases), extract it, and put
`ptk.exe` on your `PATH` (e.g. `%USERPROFILE%\.local\bin`). This fork has no
winget or Homebrew package.

### Alternative: Manual Installation

```bash
# From this fork's repository (NOT reachingforthejack's rtk!)
cargo install --git https://github.com/riaku/ptk ptk

# ALWAYS VERIFY after installation
ptk gain  # MUST show the savings dashboard, not "command not found"
```

⚠️ **WARNING**: `cargo install ptk` installs nothing — this fork is not published to crates.io and no crate named `ptk` exists. Use the Git URL above and verify with `ptk gain`.

## Apache-2.0 edition (open source only, commercial use allowed)

The same `ptk` binary built without the BUSL-1.1 compression engine: the hook
rewrite system and every command filter, under Apache-2.0 alone. Pick this if
your policy does not allow a source-available licence in the build.

```bash
curl -fsSL https://raw.githubusercontent.com/riaku/ptk/refs/heads/develop/install.sh | PTK_EDITION=oss sh
cargo install --git https://github.com/riaku/ptk ptk --no-default-features
```

Release assets are named `ptk-oss-<target>` (there is no DEB or RPM in this
fork's release). It does not include `ptk pipeline`, PostToolUse
compression, or delegation tasks.

## Project Initialization

### Which mode to choose?

```
  Do you want PTK active across ALL Claude Code projects?
  │
  ├─ YES → ptk init -g              (recommended)
  │         Hook + PTK.md (~10 tokens in context)
  │         Commands auto-rewritten transparently
  │
  ├─ YES, minimal → ptk init -g --hook-only
  │         Hook only, nothing added to CLAUDE.md
  │         Zero tokens in context
  │
  └─ NO, single project → ptk init
            PTK block in ./CLAUDE.md + .ptk/filters.toml template
            No hook, no global effect
```

### Recommended: Global Hook-First Setup

**Best for: All projects, automatic PTK usage**

```bash
ptk init -g
# → Registers the native `ptk hook claude` PreToolUse hook
# → Creates ~/.claude/PTK.md (short awareness file)
# → Adds @PTK.md reference to ~/.claude/CLAUDE.md
# → Prompts: "Patch settings.json? [y/N]"
# → If yes: patches + creates backup (~/.claude/settings.json.bak)

# Automated alternatives:
ptk init -g --auto-patch    # Patch without prompting
ptk init -g --no-patch      # Print manual instructions instead

# Verify installation
ptk init --show  # Check the hook is registered
```

**Context cost**: the hook adds a short `PTK.md` to your context instead of a full command reference, and rewrites commands transparently at no per-command context cost.

**What is settings.json?**
Claude Code's hook registry. PTK adds a PreToolUse hook that rewrites commands transparently. Without this, Claude won't invoke the hook automatically.

```
  Claude Code          settings.json       ptk hook claude       PTK binary
       │                    │                     │                    │
       │  "git status"      │                     │                    │
       │ ──────────────────►│                     │                    │
       │                    │  PreToolUse trigger  │                    │
       │                    │ ───────────────────►│                    │
       │                    │                     │  rewrite command   │
       │                    │                     │  → ptk git status  │
       │                    │◄────────────────────│                    │
       │                    │  updated command     │                    │
       │                    │                                          │
       │  execute: ptk git status                                      │
       │ ─────────────────────────────────────────────────────────────►│
       │                                                               │  filter
       │  "3 modified, 1 untracked ✓"                                  │
       │◄──────────────────────────────────────────────────────────────│
```

**Backup Safety**:
PTK backs up existing settings.json before changes. Restore if needed:
```bash
cp ~/.claude/settings.json.bak ~/.claude/settings.json
```

### Alternative: Local Project Setup

**Best for: Single project without hook**

```bash
cd /path/to/your/project
ptk init  # Adds the PTK block to ./CLAUDE.md and a .ptk/filters.toml template
```

**Context cost**: instructions loaded only for this project

### Upgrading from Previous Version

#### From old 137-line CLAUDE.md injection (pre-0.22)

```bash
ptk init -g  # Automatically migrates to hook-first mode
# → Removes old 137-line block
# → Installs hook + PTK.md
# → Adds @PTK.md reference
```

#### From an `ptk-rewrite.sh` shell hook (pre-0.37.2)

Since v0.37.2 the hook is a native binary command (`ptk hook claude`) and needs no Unix shell or jq. Older installs used an `ptk-rewrite.sh` script in `~/.claude/hooks/`.

```bash
# Migrate to the native binary hook
ptk init --global

# Verify the new hook is active
ptk init --show
# Should show: [ok] Hook: ptk hook claude (native binary command)
```

## Common User Flows

### First-Time User (Recommended)
```bash
# 1. Install PTK
cargo install --git https://github.com/riaku/ptk ptk
ptk gain  # Verify (must show the savings dashboard)

# 2. Setup with prompts
ptk init -g
# → Answer 'y' when prompted to patch settings.json
# → Creates backup automatically

# 3. Restart Claude Code
# 4. Test: git status (should use ptk)
```

### CI/CD or Automation
```bash
# Non-interactive setup (no prompts)
ptk init -g --auto-patch

# Verify in scripts
ptk init --show | grep "Hook:"
```

### Conservative User (Manual Control)
```bash
# Get manual instructions without patching
ptk init -g --no-patch

# Review printed JSON snippet
# Manually edit ~/.claude/settings.json
# Restart Claude Code
```

### Temporary Trial
```bash
# Install hook
ptk init -g --auto-patch

# Later: remove everything
ptk init -g --uninstall

# Restore backup if needed
cp ~/.claude/settings.json.bak ~/.claude/settings.json
```

## Installation Verification

```bash
# Basic test
ptk ls .

# Test with git
ptk git status

# Test with pnpm
ptk pnpm list

# Test with Vitest
ptk vitest
```

## Uninstalling

### Complete Removal (Global Installations Only)

```bash
# Complete removal (global installations only)
ptk init -g --uninstall

# What gets removed:
#   - Hook: the `ptk hook claude` entry (and any legacy ~/.claude/hooks/ptk-rewrite.sh)
#   - Context: ~/.claude/PTK.md
#   - Reference: @PTK.md line from ~/.claude/CLAUDE.md
#   - Registration: PTK hook entry from settings.json

# Restart Claude Code after uninstall
```

**For Local Projects**: Manually remove PTK block from `./CLAUDE.md`

### Binary Removal

```bash
# If installed via cargo
cargo uninstall ptk

# If installed from a release archive, delete the binary
rm "$(command -v ptk)"      # macOS / Linux
# Windows: delete ptk.exe from wherever you put it
```

### Restore from Backup (if needed)

```bash
cp ~/.claude/settings.json.bak ~/.claude/settings.json
```

## Essential Commands

### Files
```bash
ptk ls .              # Compact tree view
ptk read file.rs      # Optimized reading
ptk grep "pattern" .  # Grouped search results
```

### Git
```bash
ptk git status        # Compact status
ptk git log -n 10     # Condensed logs
ptk git diff          # Optimized diff
ptk git add .         # → "ok ✓"
ptk git commit -m "msg"  # → "ok ✓ abc1234"
ptk git push          # → "ok ✓ main"
```

### Pnpm
```bash
ptk pnpm list     # Compact dependency tree
ptk pnpm outdated # Available updates
ptk pnpm install  # Silent installation
```

### Tests
```bash
ptk cargo test      # Filtered Cargo test output
ptk go test         # Filtered Go tests (NDJSON)
ptk jest            # Filtered Jest output
ptk vitest          # Filtered Vitest output
ptk playwright test # Filtered Playwright output
ptk pytest          # Filtered Python tests
ptk rake test       # Filtered Ruby tests
ptk rspec           # Filtered RSpec tests
ptk test <cmd> [args...]  # Generic test wrapper - failures only
```

### Statistics
```bash
ptk gain              # Savings dashboard
ptk gain --graph      # With ASCII graph
ptk gain --history    # With command history
```

## What PTK Filters

PTK compresses the output of a shell command before your agent reads it. What that looks like in practice:

| Operation | What PTK does to the output |
|-----------|-----------------------------|
| `vitest` / `jest` | Failures only; passing suites collapse to a count |
| `git status` | Compact stat format, grouped by state |
| `pnpm list` | Compact dependency tree |
| `pnpm outdated` | Package, current and target version only |
| `cargo test` | Failures only, with the assertion and location |

PTK reduces **bash output bytes**. That is the part PTK controls — it is not the same as reducing your bill by the same amount, because bash output is only one contributor to input tokens, and input tokens are only part of a bill that also counts output tokens.

See [How PTK Savings Work](docs/guide/resources/savings-explained.md) for the full explanation, including why the token counts PTK reports are estimates.

## Troubleshooting

### PTK command not found after installation
```bash
# Check PATH
echo $PATH | grep -o '[^:]*\.cargo[^:]*'

# Add to PATH if needed (~/.bashrc or ~/.zshrc)
export PATH="$HOME/.cargo/bin:$PATH"

# Reload shell
source ~/.bashrc  # or source ~/.zshrc
```

### PTK command not available (e.g., vitest)
```bash
# Update to the latest release, then check the version
cargo install --git https://github.com/riaku/ptk ptk --force
ptk --version
```

### Compilation error
```bash
# Update Rust
rustup update stable

# Clean and recompile
cargo clean
cargo build --release
cargo install --path . --force
```

## Support and Contributing

- **Repository**: https://github.com/riaku/ptk
- **Troubleshooting**: See [Troubleshooting](docs/guide/resources/troubleshooting.md) for common issues
- **GitHub issues**: https://github.com/riaku/ptk/issues
- **Pull Requests**: https://github.com/riaku/ptk/pulls

⚠️ **If you installed the wrong ptk (Type Kit)**, see [Troubleshooting](docs/guide/resources/troubleshooting.md#ptk-gain-says-not-a-ptk-command)

## AI Assistant Checklist

Before each session:

- [ ] Verify PTK is installed: `ptk --version`
- [ ] If not installed → follow "Installation" above
- [ ] If project not initialized → `ptk init`
- [ ] Use `ptk` for ALL git/pnpm/test/vitest commands
- [ ] Check savings: `ptk gain`

**Golden Rule**: AI coding assistants should ALWAYS use `ptk` as a proxy for shell commands that generate verbose output (git, pnpm, npm, cargo test, vitest, docker, kubectl).
