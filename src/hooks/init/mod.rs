//! Sets up PTK hooks so AI coding agents automatically route commands through PTK.
//!
//! `run` dispatches on the target agent; each agent's install and uninstall live in its own
//! submodule, and the helpers shared between them stay here.
use crate::hooks::constants::{
    CLAUDE_DIR, CLAUDE_HOOK_COMMAND, HOOKS_JSON, HOOKS_SUBDIR, PRE_TOOL_USE_KEY, REWRITE_HOOK_FILE,
    SETTINGS_JSON,
};
use anyhow::{Context, Result};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

use crate::core::utils::{from_json_str, strip_leading_bom};

use super::integrity;
use super::is_claude_hook_command;
use crate::core::config::AwarenessLevel;
#[cfg(test)]
use crate::core::test_isolation;
use crate::core::user_dirs;

mod agents_md;
mod antigravity;
mod claude;
mod codex;
mod copilot;
mod cursor;
mod droid;
mod gemini;
mod hermes;
mod instructions_agents;
mod opencode;
mod pi;
mod trae;
mod vibe;

// `agents_md` and `pi` hold helpers that several submodules share, so they are
// glob-imported and reach siblings through `use super::*`; every other submodule is used only
// from here and is imported by name.
use agents_md::*;
use claude::{
    hook_already_present, hook_gaps, remove_settings_entries, run_claude_md_mode, run_default_mode,
    run_hook_only_mode,
};
use codex::{run_codex_mode, show_codex_config, uninstall_codex};
use cursor::{
    cursor_hook_already_present, install_cursor_hooks, remove_cursor_hooks, resolve_cursor_dir,
};
use gemini::uninstall_gemini;
use instructions_agents::{run_cline_mode, run_windsurf_mode};
use opencode::{
    opencode_plugin_path, remove_opencode_plugin, resolve_opencode_dir, run_opencode_only_mode,
};
use pi::*;

pub use antigravity::{run_antigravity_mode, uninstall_antigravity_mode};
pub use claude::claude_hook_gaps;
pub(crate) use copilot::{COPILOT_HOOK_JSON, copilot_user_dir};
pub use copilot::{run_copilot, run_copilot_global, uninstall_copilot, uninstall_copilot_global};
pub use droid::{run_droid_mode, uninstall_droid};
pub use gemini::run_gemini;
pub use hermes::{run_hermes_mode, uninstall_hermes};
pub use instructions_agents::{run_kilocode_mode, run_kimi_mode};
pub use pi::{run_omp_mode_with_patch_mode, run_pi_mode_with_patch_mode};
pub use trae::{run_trae_mode, uninstall_trae_mode};
pub use vibe::{run_vibe_mode, uninstall_vibe};

// Embedded agent-neutral PTK awareness instructions, one file per `awareness.level`.
pub(super) const PTK_AWARENESS_DEFAULT: &str = include_str!("../../../hooks/ptk-awareness.md");
pub(super) const PTK_AWARENESS_HIGH: &str = include_str!("../../../hooks/ptk-awareness-high.md");
pub(super) const PTK_AWARENESS_FULL: &str = include_str!("../../../hooks/ptk-awareness-full.md");

/// Template written by `ptk init` when no filters.toml exists yet.
const FILTERS_TEMPLATE: &str = r#"# Project-local PTK filters — commit this file with your repo.
# Filters here override user-global and built-in filters.
# Docs: https://github.com/riaku/ptk#custom-filters
schema_version = 1

# Example: suppress build noise from a custom tool
# [filters.my-tool]
# description = "Compact my-tool output"
# match_command = "^my-tool\\s+build"
# strip_ansi = true
# strip_lines_matching = ["^\\s*$", "^Downloading", "^Installing"]
# max_lines = 30
# on_empty = "my-tool: ok"
"#;

/// Template for user-global filters (~/.config/ptk/filters.toml).
const FILTERS_GLOBAL_TEMPLATE: &str = r#"# User-global PTK filters — apply to all your projects.
# Project-local .ptk/filters.toml takes precedence over these.
# Docs: https://github.com/riaku/ptk#custom-filters
schema_version = 1

# Example: suppress noise from a tool you use everywhere
# [filters.my-global-tool]
# description = "Compact my-global-tool output"
# match_command = "^my-global-tool\\b"
# strip_ansi = true
# strip_lines_matching = ["^\\s*$"]
# max_lines = 40
"#;

pub(super) const PTK_MD: &str = "PTK.md";

pub(super) const CLAUDE_MD: &str = "CLAUDE.md";

pub(super) const AGENTS_MD: &str = "AGENTS.md";

pub(super) const PTK_MD_REF: &str = "@PTK.md";

pub(super) const PTK_BLOCK_START: &str = "<!-- ptk-instructions";
const PTK_BLOCK_VERSION: &str = "v2";
pub(super) const PTK_BLOCK_END: &str = "<!-- /ptk-instructions -->";

/// Control flow for settings.json patching
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatchMode {
    Ask,  // Default: prompt user [y/N]
    Auto, // --auto-patch: no prompt
    Skip, // --no-patch: manual instructions
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FilterTrust {
    #[default]
    Ask,
    Trust,
    Skip,
}

/// Result of settings.json patching operation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatchResult {
    Patched,        // Hook was added successfully
    AlreadyPresent, // Hook was already in settings.json
    Declined,       // User declined when prompted
    Skipped,        // --no-patch flag used
    WouldPatch,     // Dry-run: hook would have been added
}

/// Shared context threaded through every init/uninstall function.
///
/// Replaces ad-hoc `verbose: u8, dry_run: bool` parameter pairs to keep
/// signatures compact as more flags are added (mirrors `RunOptions` in
/// `src/core/runner.rs`).
#[derive(Clone, Copy, Default)]
pub struct InitContext {
    pub verbose: u8,
    pub dry_run: bool,
    /// `awareness.level` from config.toml. Agents with a command hook receive the matching
    /// awareness file; agents without a hook always receive `full` (see `print_instructions_agents_awareness_note`).
    pub awareness: AwarenessLevel,
}

pub(super) fn awareness_content(level: AwarenessLevel) -> &'static str {
    match level {
        AwarenessLevel::Default => PTK_AWARENESS_DEFAULT,
        AwarenessLevel::High => PTK_AWARENESS_HIGH,
        AwarenessLevel::Full => PTK_AWARENESS_FULL,
    }
}

/// Awareness content plus the resolved pipeline's awareness-file directives,
/// merged in declaration order. With no directive configured this is exactly
/// `awareness_content` — byte-identical init output. A pipeline
/// `ptk-awareness` directive's `level` overrides the config level.
#[cfg(not(feature = "engine"))]
fn instructions_body(config_level: AwarenessLevel) -> String {
    awareness_content(config_level).to_string()
}

#[cfg(feature = "engine")]
fn instructions_body(config_level: AwarenessLevel) -> String {
    let loaded = crate::core::pipeline_config::load();
    instructions_body_for(config_level, &loaded.resolved)
}

#[cfg(feature = "engine")]
fn instructions_body_for(
    config_level: AwarenessLevel,
    pipeline: &ptk_engine::pipeline::Resolved,
) -> String {
    let mut level = config_level;
    for directive in &pipeline.directives {
        if !directive.def.enabled {
            continue;
        }
        // The built-in `ptk-awareness level = "default"` never overrides the
        // user's `[awareness] level` in config.toml; an explicit user
        // directive or `[override.ptk-awareness]` does.
        let user_set = directive.scope != ptk_engine::pipeline::Scope::Builtin
            || !directive.overridden_by.is_empty();
        if directive.def.use_.as_deref() == Some("ptk-awareness")
            && user_set
            && let Some(l) = directive.def.options.get("level").and_then(|v| v.as_str())
        {
            match l {
                "default" => level = AwarenessLevel::Default,
                "high" => level = AwarenessLevel::High,
                "full" => level = AwarenessLevel::Full,
                _ => {}
            }
        }
    }

    let mut texts = vec![awareness_content(level).to_string()];
    for directive in &pipeline.directives {
        let def = &directive.def;
        if !def.enabled || def.use_.as_deref() == Some("ptk-awareness") {
            continue;
        }
        if let Some(channel) = def.channel.as_deref()
            && channel != "awareness_file"
        {
            continue;
        }
        if let Some(text) = crate::core::pipeline_config::directive_text(directive) {
            texts.push(text);
        } else if def.file.is_some() {
            eprintln!(
                "ptk: directive `{}` names a file that could not be read — skipped",
                directive.name
            );
        }
    }
    // The generated `route-tasks` directive: whenever a task instance is
    // bound to an upstream, the agent learns the exact `ptk <task>` syntax.
    // A task reached only by an `after_tool` route is left out — PTK calls it
    // itself there, so telling the agent how to invoke it is pure context cost.
    let after_tool_only: std::collections::HashSet<&str> = {
        let mut after: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut elsewhere: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for (position, rules) in &pipeline.rules {
            for rule in rules {
                let Some(task) = rule.def.task.as_deref() else {
                    continue;
                };
                if position.event == ptk_engine::block::Event::AfterTool {
                    after.insert(task);
                } else {
                    elsewhere.insert(task);
                }
            }
        }
        after.difference(&elsewhere).copied().collect()
    };
    let bound: Vec<(&str, &str)> = pipeline
        .tasks
        .iter()
        .filter(|(_, (task, _))| task.upstream.is_some())
        .filter(|(name, _)| !after_tool_only.contains(name.as_str()))
        .map(|(name, (task, _))| (name.as_str(), task.kind.as_str()))
        .collect();
    let route = ptk_engine::directive::route_tasks_text(&bound);
    if !route.is_empty() {
        texts.push(route);
    }
    let merged = ptk_engine::directive::merge(&texts);
    if merged.ends_with('\n') {
        merged
    } else {
        format!("{merged}\n")
    }
}

/// Wrap an awareness file in the `<!-- ptk-instructions -->` markers used by
/// `write_ptk_block`, so it can be upserted into a shared file like AGENTS.md.
pub(super) fn ptk_block(body: &str) -> String {
    format!("{PTK_BLOCK_START} {PTK_BLOCK_VERSION} -->\n{body}{PTK_BLOCK_END}\n")
}

/// Shared dry-run footer printed at the end of every init sub-mode.
pub(super) fn print_dry_run_footer() {
    println!("\n[dry-run] Nothing written.");
}

// Legacy full instructions for backward compatibility (--claude-md mode)
pub(super) const PTK_INSTRUCTIONS: &str = r##"<!-- ptk-instructions v2 -->
# PTK (Rust Token Killer) - Token-Optimized Commands

## Golden Rule

**Always prefix commands with `ptk`**. If PTK has a dedicated filter, it uses it. If not, it passes through unchanged. This means PTK is always safe to use.

**Important**: Even in command chains with `&&`, use `ptk`:
```bash
# ❌ Wrong
git add . && git commit -m "msg" && git push

# ✅ Correct
ptk git add . && ptk git commit -m "msg" && ptk git push
```

## PTK Commands by Workflow

### Build & Compile (80-90% savings)
```bash
ptk cargo build         # Cargo build output
ptk cargo check         # Cargo check output
ptk cargo clippy        # Clippy warnings grouped by file (80%)
ptk tsc                 # TypeScript errors grouped by file/code (83%)
ptk lint                # ESLint/Biome violations grouped (84%)
ptk prettier --check    # Files needing format only (70%)
ptk next build          # Next.js build with route metrics (87%)
```

### Test (60-99% savings)
```bash
ptk cargo test          # Cargo test failures only (90%)
ptk go test             # Go test failures only (90%)
ptk jest                # Jest failures only (99.5%)
ptk vitest              # Vitest failures only (99.5%)
ptk playwright test     # Playwright failures only (94%)
ptk pytest              # Python test failures only (90%)
ptk rake test           # Ruby test failures only (90%)
ptk rspec               # RSpec test failures only (60%)
ptk test <cmd> [args...] # Generic test wrapper - failures only
```

### Git (59-80% savings)
```bash
ptk git status          # Compact status
ptk git log             # Compact log (works with all git flags)
ptk git diff            # Compact diff (80%)
ptk git show            # Compact show (80%)
ptk git add             # Ultra-compact confirmations (59%)
ptk git commit          # Ultra-compact confirmations (59%)
ptk git push            # Ultra-compact confirmations
ptk git pull            # Ultra-compact confirmations
ptk git branch          # Compact branch list
ptk git fetch           # Compact fetch
ptk git stash           # Compact stash
ptk git worktree        # Compact worktree
```

Note: Git passthrough works for ALL subcommands, even those not explicitly listed.

### GitHub (26-87% savings)
```bash
ptk gh pr view <num>    # Compact PR view (87%)
ptk gh pr checks        # Compact PR checks (79%)
ptk gh run list         # Compact workflow runs (82%)
ptk gh issue list       # Compact issue list (80%)
ptk gh api              # Compact API responses (26%)
```

### JavaScript/TypeScript Tooling (70-90% savings)
```bash
ptk pnpm list           # Compact dependency tree (70%)
ptk pnpm outdated       # Compact outdated packages (80%)
ptk pnpm install        # Compact install output (90%)
ptk npm run <script>    # Compact npm script output
ptk npx <cmd>           # Compact npx command output
ptk prisma              # Prisma without ASCII art (88%)
ptk uv run <cmd>        # Compact uv project command output
```

### Files & Search (60-75% savings)
```bash
ptk ls <path>           # Tree format, compact (65%)
ptk read <file>         # Code reading with filtering (60%)
ptk grep <pattern>      # Search grouped by file (75%). Format flags (-c, -l, -L, -o, -Z) run raw.
ptk find <pattern>      # Find grouped by directory (70%)
```

### Analysis & Debug (70-90% savings)
```bash
ptk err <cmd> [args...] # Filter errors only; argv runs directly, no shell
ptk log <file>          # Deduplicated logs with counts
ptk json <file>         # JSON structure without values
ptk deps                # Dependency overview
ptk env                 # Environment variables compact
ptk summary <cmd> [args...]  # Smart summary of command output
ptk diff                # Ultra-compact diffs
```

`ptk err`, `ptk test` and `ptk summary` execute the program directly, so shell
syntax (`&&`, `|`, `*`, `$VAR`) is not interpreted. Pass a script explicitly:
`ptk err --shell sh 'npm run build && npm test'`.

### Infrastructure (85% savings)
```bash
ptk docker ps           # Compact container list
ptk docker images       # Compact image list
ptk docker logs <c>     # Deduplicated logs
ptk kubectl get         # Compact resource list
ptk kubectl logs        # Deduplicated pod logs
```

### Network (65-70% savings)
```bash
ptk curl <url>          # Compact HTTP responses (70%)
ptk wget <url>          # Compact download output (65%)
```

### Meta Commands
```bash
ptk gain                # View token savings statistics
ptk gain --history      # View command history with savings
ptk discover            # Analyze Claude Code sessions for missed PTK usage
ptk bypass <cmd>        # Run command without filtering (for debugging)
ptk init                # Add PTK instructions to CLAUDE.md
ptk init --global       # Add PTK to ~/.claude/CLAUDE.md
```

## Token Savings Overview

| Category | Commands | Typical Savings |
|----------|----------|-----------------|
| Tests | vitest, playwright, cargo test | 90-99% |
| Build | next, tsc, lint, prettier | 70-87% |
| Git | status, log, diff, add, commit | 59-80% |
| GitHub | gh pr, gh run, gh issue | 26-87% |
| Package Managers | pnpm, npm, npx | 70-90% |
| Files | ls, read, grep, find | 60-75% |
| Infrastructure | docker, kubectl | 85% |
| Network | curl, wget | 65-70% |

Overall average: **60-90% less bash output** on common development operations.
<!-- /ptk-instructions -->
"##;

/// Main entry point for `ptk init`
#[allow(clippy::too_many_arguments)]
pub fn run(
    global: bool,
    install_claude: bool,
    install_opencode: bool,
    install_cursor: bool,
    install_windsurf: bool,
    install_cline: bool,
    claude_md: bool,
    hook_only: bool,
    codex: bool,
    patch_mode: PatchMode,
    ctx: InitContext,
) -> Result<()> {
    let InitContext { dry_run, .. } = ctx;
    // Validation: Codex mode conflicts
    if codex {
        if install_opencode {
            anyhow::bail!("--codex cannot be combined with --opencode");
        }
        if claude_md {
            anyhow::bail!("--codex cannot be combined with --claude-md");
        }
        if hook_only {
            anyhow::bail!("--codex cannot be combined with --hook-only");
        }
        if matches!(patch_mode, PatchMode::Auto) {
            anyhow::bail!("--codex cannot be combined with --auto-patch");
        }
        if matches!(patch_mode, PatchMode::Skip) {
            anyhow::bail!("--codex cannot be combined with --no-patch");
        }
        run_codex_mode(global, ctx)?;
    } else {
        // Validation: Global-only features
        if install_opencode && !global {
            anyhow::bail!("OpenCode plugin is global-only. Use: ptk init -g --opencode");
        }

        if install_cursor && !global {
            anyhow::bail!("Cursor hooks are global-only. Use: ptk init -g --agent cursor");
        }

        if install_windsurf && !global {
            anyhow::bail!("Windsurf support is global-only. Use: ptk init -g --agent windsurf");
        }

        if install_windsurf {
            run_windsurf_mode(ctx)?;
        } else if install_cline {
            run_cline_mode(ctx)?;
        } else {
            // Mode selection (Claude Code / OpenCode)
            match (install_claude, install_opencode, claude_md, hook_only) {
                (false, true, _, _) => run_opencode_only_mode(ctx)?,
                (true, opencode, true, _) => run_claude_md_mode(global, opencode, ctx)?,
                (true, opencode, false, true) => {
                    run_hook_only_mode(global, patch_mode, opencode, ctx)?
                }
                (true, opencode, false, false) => {
                    run_default_mode(global, patch_mode, opencode, ctx)?
                }
                (false, false, _, _) => {
                    if !install_cursor {
                        anyhow::bail!(
                            "at least one of install_claude or install_opencode must be true"
                        )
                    }
                }
            }

            // Cursor hooks: on their own, or next to the OpenCode plugin with --opencode
            if install_cursor {
                install_cursor_hooks(ctx)?;
            }
        }
    }

    if !dry_run {
        prompt_telemetry_consent()?;
        // Best-effort: unconditionally re-run tracking-DB schema migrations during
        // install/upgrade (bypassing the `user_version` gate `Tracker::new()` uses
        // on its hot path). This both pre-warms the schema so the first PreToolUse
        // hook invocation (or `ptk <cmd>`) after this doesn't pay the one-time
        // migration cost itself, and self-heals a table dropped/corrupted
        // out-of-band (see `tracking::warn_if_missing_table`) — `ptk init` is
        // already the natural "something's wrong, reinstall" move, so no separate
        // repair flag is needed. `CREATE TABLE IF NOT EXISTS`/`ALTER TABLE` are
        // additive, so existing history is left untouched. Never fail `ptk init`
        // over a tracking-DB hiccup — but still tell the user something's wrong,
        // consistent with every other best-effort warning in this function
        // (rust-patterns.md's anti-pattern rule: a silent `Err(_) => {}` leaves
        // the user with zero indication anything went wrong).
        if let Err(e) = crate::core::tracking::ensure_schema_fresh() {
            eprintln!("  [warn] Failed to prepare tracking database: {e}");
        }
    }

    report_pipeline_health();

    if dry_run {
        print_dry_run_footer();
    } else {
        println!();
    }

    if crate::core::tee_file::legacy_tee_migration_pending() {
        println!("{}", crate::core::tee_file::LEGACY_TEE_NOTICE);
        println!();
    } else if crate::core::tee_file::legacy_tee_config_in_use() {
        println!("{}", crate::core::tee_file::LEGACY_TEE_CONFIG_NOTICE);
        println!();
    } else if crate::core::tee_file::legacy_tee_fields_merged_in_use() {
        println!("{}", crate::core::tee_file::LEGACY_TEE_MERGED_NOTICE);
        println!();
    }

    Ok(())
}

/// Idempotent file write: create or update if content differs.
/// When `dry_run` is true, prints the intended action and does not touch the filesystem.
pub(super) fn write_if_changed(
    path: &Path,
    content: &str,
    name: &str,
    ctx: InitContext,
) -> Result<bool> {
    write_if_changed_internal(path, content, name, ctx, false)
}

/// Variant used for protected PTK files. A file that cannot be decoded is
/// still replaceable after the caller's policy allows it (for example,
/// `--auto-patch`), so a read error is treated like differing content instead
/// of preventing recovery.
pub(super) fn write_if_changed_allow_read_error(
    path: &Path,
    content: &str,
    name: &str,
    ctx: InitContext,
) -> Result<bool> {
    write_if_changed_internal(path, content, name, ctx, true)
}

fn write_if_changed_internal(
    path: &Path,
    content: &str,
    name: &str,
    ctx: InitContext,
    allow_read_error: bool,
) -> Result<bool> {
    let InitContext {
        verbose, dry_run, ..
    } = ctx;
    if path.exists() {
        let existing = match fs::read_to_string(path) {
            Ok(existing) => existing,
            Err(_) if allow_read_error => {
                if dry_run {
                    println!("[dry-run] would update {}: {}", name, path.display());
                    if verbose > 0 {
                        println!("[dry-run] content:\n{}", content);
                    }
                } else {
                    atomic_write(path, content)
                        .with_context(|| format!("Failed to write {}: {}", name, path.display()))?;
                    if verbose > 0 {
                        eprintln!("Updated {}: {}", name, path.display());
                    }
                }
                return Ok(true);
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Failed to read {}: {}", name, path.display()));
            }
        };

        if existing == content {
            if verbose > 0 {
                eprintln!("{} already up to date: {}", name, path.display());
            }
            Ok(false)
        } else {
            if dry_run {
                println!("[dry-run] would update {}: {}", name, path.display());
                if verbose > 0 {
                    println!("[dry-run] content:\n{}", content);
                }
            } else {
                atomic_write(path, content)
                    .with_context(|| format!("Failed to write {}: {}", name, path.display()))?;
                if verbose > 0 {
                    eprintln!("Updated {}: {}", name, path.display());
                }
            }
            Ok(true)
        }
    } else {
        if dry_run {
            println!("[dry-run] would create {}: {}", name, path.display());
            if verbose > 0 {
                println!("[dry-run] content:\n{}", content);
            }
        } else {
            ensure_parent_dir(path)?;
            atomic_write(path, content)
                .with_context(|| format!("Failed to write {}: {}", name, path.display()))?;
            if verbose > 0 {
                eprintln!("Created {}: {}", name, path.display());
            }
        }
        Ok(true)
    }
}

pub(super) fn ensure_parent_dir(path: &Path) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory: {}", parent.display()))
}

/// Resolve the final write target: if `path` is a symlink, follow it so
/// the atomic rename lands on the real file and the symlink is preserved.
fn resolve_atomic_target(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Atomic write using tempfile + rename
/// Prevents corruption on crash/interrupt
/// Follows symlinks so the link itself is preserved.
pub(super) fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let target = resolve_atomic_target(path);
    let parent = target.parent().with_context(|| {
        format!(
            "Cannot write to {}: path has no parent directory",
            target.display()
        )
    })?;

    // Create temp file in same directory (ensures same filesystem for atomic rename)
    let mut temp_file = NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temp file in {}", parent.display()))?;

    // Write content
    temp_file
        .write_all(content.as_bytes())
        .with_context(|| format!("Failed to write {} bytes to temp file", content.len()))?;

    // Atomic rename
    temp_file.persist(&target).with_context(|| {
        format!(
            "Failed to atomically replace {} (disk full?)",
            target.display()
        )
    })?;

    Ok(())
}

/// Read a JSON file with path-aware errors. Missing files return `None` and
/// empty files are treated as an empty JSON object.
pub(super) fn read_json_file(path: &Path) -> Result<Option<serde_json::Value>> {
    if !path.exists() {
        return Ok(None);
    }

    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let content = strip_leading_bom(&content);
    if content.trim().is_empty() {
        return Ok(Some(serde_json::json!({})));
    }

    from_json_str(content)
        .map(Some)
        .with_context(|| format!("Failed to parse {} as JSON", path.display()))
}

/// Canonicalize a path even when its final file has not been
/// created yet. This detects agent directories connected by symlinks while
/// retaining a literal-path fallback for genuinely unresolved paths.
pub(super) fn canonicalize_path_for_comparison(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }

    let mut missing_components = Vec::new();
    let mut candidate = path;
    loop {
        if let Ok(mut canonical) = fs::canonicalize(candidate) {
            for component in missing_components.iter().rev() {
                canonical.push(component);
            }
            return canonical;
        }

        let Some(file_name) = candidate.file_name() else {
            return path.to_path_buf();
        };
        missing_components.push(file_name.to_os_string());

        let Some(parent) = candidate.parent() else {
            return path.to_path_buf();
        };
        if parent == candidate {
            return path.to_path_buf();
        }
        candidate = parent;
    }
}

/// Where [`backup_and_atomic_write`] puts the copy it takes before overwriting `path`. Named
/// so a caller that has to vouch for where it writes can vouch for this one too.
pub(super) fn backup_path_for(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

/// What to say on stderr, under `-v`, once [`update_json_file`] has written a file.
pub(super) enum Written {
    /// `Backup: <path>` when a backup was taken, nothing otherwise.
    Backup,
    /// A fixed line.
    Line(String),
}

/// Serialise `root` and write it to `path` atomically, backing the old file up first; under
/// `--dry-run` print `dry_run_message` instead (and the content under `-v` when
/// `content_on_dry_run`). `label` names the file in the serialisation error.
pub(super) fn update_json_file(
    path: &Path,
    root: &serde_json::Value,
    ctx: InitContext,
    label: &str,
    dry_run_message: &str,
    content_on_dry_run: bool,
    written: Written,
) -> Result<()> {
    let serialized = serde_json::to_string_pretty(root)
        .with_context(|| format!("Failed to serialize {label}"))?;
    if ctx.dry_run {
        println!("{dry_run_message}");
        if content_on_dry_run && ctx.verbose > 0 {
            println!("[dry-run] content:\n{serialized}");
        }
        return Ok(());
    }
    let backup = backup_and_atomic_write(path, &serialized)?;
    if ctx.verbose > 0 {
        match written {
            Written::Backup => {
                if let Some(backup) = backup {
                    eprintln!("Backup: {}", backup.display());
                }
            }
            Written::Line(line) => eprintln!("{line}"),
        }
    }
    Ok(())
}

/// Back up an existing JSON file before replacing it atomically.
fn backup_and_atomic_write(path: &Path, content: &str) -> Result<Option<PathBuf>> {
    let backup_path = if path.exists() {
        let backup_path = backup_path_for(path);
        fs::copy(path, &backup_path).with_context(|| {
            format!(
                "Failed to backup {} to {}",
                path.display(),
                backup_path.display()
            )
        })?;
        Some(backup_path)
    } else {
        None
    };

    atomic_write(path, content)
        .with_context(|| format!("Failed to update JSON file: {}", path.display()))?;
    Ok(backup_path)
}

/// Prompt user for confirmation.
/// Prints to stderr (stdout may be piped), reads from stdin, and defaults to
/// No in non-interactive environments.
pub(super) fn prompt_user_confirmation(prompt: &str) -> Result<bool> {
    use std::io::{self, BufRead, IsTerminal};

    eprint!("\n{} [y/N] ", prompt);
    io::stderr().flush().context("Failed to flush prompt")?;

    // If stdin is not a terminal (piped), default to No.
    if !io::stdin().is_terminal() {
        eprintln!("\n(non-interactive mode, defaulting to N)");
        return Ok(false);
    }

    let stdin = io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .context("Failed to read user input")?;

    let response = line.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}
/// Load + validate the pipeline at init time (init runs validate) and
/// refresh the status channel so a fixed file clears its stale warning.
#[cfg(not(feature = "engine"))]
fn report_pipeline_health() {}

#[cfg(feature = "engine")]
fn report_pipeline_health() {
    let loaded = crate::core::pipeline_config::load();
    crate::core::pipeline_config::write_status(&loaded);
    if loaded.diagnostics.is_empty() {
        return;
    }
    println!();
    println!("  Pipeline ({})", loaded.resolved.name);
    for diagnostic in &loaded.diagnostics {
        let tag = match diagnostic.severity() {
            ptk_engine::pipeline::Severity::Error => "[error]",
            ptk_engine::pipeline::Severity::Warning => "[warn]",
        };
        println!("  {tag} {}", diagnostic.render());
    }
    if loaded.fell_back {
        println!("  [warn] pipeline.toml has errors — using the built-in default until fixed");
    }
}

/// Proxy on-ramp: offer to point Claude Code at the PTK proxy for the
/// history/cache layers. Claude Code is the one agent with a confirmed
/// config channel (settings.json `env`); everyone else keeps the manual
/// export line from `ptk llm-proxy start --env`.
/// The base URL the global settings point Claude Code at, if any.
fn proxy_env_target() -> Option<String> {
    let dir = resolve_claude_dir().ok()?;
    let content = fs::read_to_string(dir.join(SETTINGS_JSON)).ok()?;
    let root: serde_json::Value = from_json_str(strip_leading_bom(&content)).ok()?;
    root.pointer("/env/ANTHROPIC_BASE_URL")?
        .as_str()
        .map(str::to_string)
}

#[cfg(feature = "engine")]
fn verify_proxy_setup(base_url: &str, persistence: Option<&str>) {
    for line in proxy_verification_lines(
        base_url,
        crate::llm_proxy::is_reachable(),
        proxy_env_target().as_deref(),
        persistence,
    ) {
        println!("{line}");
    }
}

#[cfg(feature = "engine")]
/// Both halves have to hold: an env var aimed at a dead daemon is not set up,
/// and a running daemon nothing points at records nothing — the state that
/// leaves `ptk gain` empty and looking broken.
///
/// Deliberately local. Verifying a real round trip would mean sending a request
/// to the provider during `init`, and the two failures worth catching are here.
fn proxy_verification_lines(
    base_url: &str,
    reachable: bool,
    target: Option<&str>,
    persistence: Option<&str>,
) -> Vec<String> {
    let aimed_here = target == Some(base_url);
    let mut out = vec![
        format!(
            "  verify: daemon {}",
            if reachable {
                "responding".to_string()
            } else {
                format!("NOT responding on {}", crate::llm_proxy::DEFAULT_BIND)
            }
        ),
        format!(
            "  verify: settings.json {}",
            match target {
                Some(t) if aimed_here => format!("env.ANTHROPIC_BASE_URL = {t}"),
                Some(t) => format!("env.ANTHROPIC_BASE_URL = {t} (not this daemon)"),
                None => "env.ANTHROPIC_BASE_URL missing".to_string(),
            }
        ),
    ];
    if reachable && aimed_here {
        out.push(
            "  proxy: configured — restart Claude Code, then `ptk gain` shows requests".to_string(),
        );
        out.push(match persistence {
            Some(p) => format!("  verify: restart persistence — {p}"),
            None => "  [warn] nothing restarts the daemon after a reboot — rerun `ptk init -g`"
                .to_string(),
        });
        out.push(
            "  Compression: cache-breakpoint alignment — a no-op for clients that set their own, Claude Code included; CCR ships off (`ptk pipeline show ptk:proxy`)"
                .to_string(),
        );
    } else {
        out.push(
            "  proxy: NOT fully configured — `ptk gain` will report no proxy data".to_string(),
        );
    }
    out
}

/// Project scope writes the awareness block and a filters template, and stops.
/// Registering a hook or starting a daemon from a directory you may have just
/// cloned is the trust boundary `ptk trust` exists to hold, so that stays with
/// `-g`. Report the global state rather than leaving the user to discover that
/// `-g` does more — a bare `ptk init` otherwise looks like the whole setup.
fn print_project_scope_footer() {
    for line in project_scope_footer_lines(
        crate::hooks::hook_check::status(),
        proxy_env_target().is_some(),
        crate::llm_proxy::is_reachable(),
    ) {
        println!("{line}");
    }
}

fn project_scope_footer_lines(
    hooks: crate::hooks::hook_check::HookStatus,
    proxy_aimed: bool,
    proxy_reachable: bool,
) -> Vec<String> {
    use crate::hooks::hook_check::HookStatus;
    let hooks = match hooks {
        HookStatus::Ok => "registered globally",
        HookStatus::Outdated => "registered globally (outdated — run `ptk init -g`)",
        HookStatus::Missing => "NOT registered — run `ptk init -g`",
    };
    let proxy = match (proxy_aimed, proxy_reachable) {
        (true, true) => "configured and running",
        (true, false) => "configured, daemon not running — `ptk llm-proxy start`",
        (false, true) => "daemon running but nothing routed to it — `ptk init -g`",
        (false, false) => "not set up — `ptk init -g`",
    };
    vec![
        format!("  hooks:     {hooks}"),
        format!("  proxy:     {proxy}"),
    ]
}

#[cfg(feature = "engine")]
fn offer_proxy_onramp(settings_path: &Path, patch_mode: PatchMode, ctx: InitContext) -> Result<()> {
    use std::io::{BufRead, IsTerminal};
    // Only `PatchMode::Ask` may prompt. `--auto-patch` exists to run without a
    // terminal, and neither it nor `--no-patch` may adopt the proxy on the
    // user's behalf: routing an agent's traffic through a daemon is not a
    // "supported init change", it is a feature the user has to choose.
    if !matches!(patch_mode, PatchMode::Ask) {
        return Ok(());
    }
    if ctx.dry_run || !std::io::stdin().is_terminal() {
        return Ok(());
    }
    if crate::core::user_env::var("PTK_ONRAMP_DISABLED").as_deref() == Some("1") {
        return Ok(());
    }
    let base_url = format!("http://{}", crate::llm_proxy::DEFAULT_BIND);

    let mut root: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(settings_path)?;
        let content = strip_leading_bom(&content);
        if content.trim().is_empty() {
            serde_json::json!({})
        } else {
            from_json_str(content)?
        }
    } else {
        serde_json::json!({})
    };
    let already = root
        .pointer("/env/ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str().map(str::to_string));
    let aimed_here = already.as_deref() == Some(base_url.as_str());
    let autostart_present = session_start_entry_present(&root);
    if aimed_here && autostart_present {
        return Ok(());
    }
    // Routing Claude Code at a daemon that cannot be started breaks every
    // session (#4604): only offer the proxy when its binary is installed, so
    // the SessionStart hook can start it again after a reboot. A daemon that
    // merely answers on the port today may not be startable tomorrow.
    if !crate::llm_proxy::binary_available() {
        eprintln!();
        eprintln!(
            "  PTK proxy not offered: ptk-proxy is not installed (`ptk llm-proxy start` says where to get it)."
        );
        return Ok(());
    }

    if aimed_here {
        // The failure this flow exists to prevent: a base URL that outlives
        // the daemon it points at. The env is set but nothing brings the
        // daemon back after a reboot — offer only the persistence question.
        eprintln!();
        eprintln!(
            "  The PTK proxy is configured in settings.json, but nothing restarts the daemon after a reboot."
        );
    } else {
        eprintln!();
        if let Some(foreign) = already.as_deref() {
            eprintln!(
                "  settings.json already routes Claude Code through {foreign} — another proxy or gateway."
            );
            eprint!("  Replace it with the PTK proxy? Your current routing stops working. [y/N] ");
        } else {
            eprint!(
                "  Set up the PTK proxy (routes Claude Code's API traffic through a local daemon)? [y/N] "
            );
        }
        let mut line = String::new();
        let n = std::io::stdin().lock().read_line(&mut line)?;
        if n == 0 || !matches!(line.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("  Skipped — enable later with `ptk init -g` or `ptk llm-proxy start --env`.");
            return Ok(());
        }
    }

    eprintln!("  How should the daemon come back after a reboot?");
    eprintln!(
        "    1) SessionStart hook — start it automatically when a Claude session begins (recommended)"
    );
    eprintln!("    2) Background service — `ptk llm-proxy service install`");
    eprintln!("    3) Neither — print the export line; settings.json is not touched");
    eprint!("  [1/2/3, default 1] ");
    let mut choice_line = String::new();
    let n = std::io::stdin().lock().read_line(&mut choice_line)?;
    if n == 0 {
        println!("  No answer — nothing changed. Re-run `ptk init -g` to decide later.");
        return Ok(());
    }
    let choice = match choice_line.trim() {
        "" | "1" => 1,
        "2" => 2,
        "3" => 3,
        other => {
            println!("  Unrecognized `{other}` — nothing changed.");
            return Ok(());
        }
    };

    if choice == 3 {
        println!("  settings.json left untouched. Point a session at the daemon with:");
        println!("    export ANTHROPIC_BASE_URL={base_url}");
    } else {
        {
            let obj = root
                .as_object_mut()
                .context("settings.json root is not an object")?;
            let env = obj
                .entry("env")
                .or_insert_with(|| serde_json::json!({}))
                .as_object_mut()
                .context("settings.json `env` is not an object")?;
            env.insert("ANTHROPIC_BASE_URL".into(), serde_json::json!(base_url));
        }
        if choice == 1 && !autostart_present {
            insert_session_start_entry(&mut root)?;
        }
        let serialized = serde_json::to_string_pretty(&root)?;
        let backup = backup_and_atomic_write(settings_path, &serialized)?;
        println!("  settings.json: env.ANTHROPIC_BASE_URL = {base_url}");
        if let Some(path) = backup {
            println!("  Backup: {}", path.display());
        }
        if choice == 1 {
            println!(
                "  settings.json: SessionStart hook registered — the proxy starts with each session"
            );
        }
    }

    match ensure_pipeline_extends("ptk:proxy", "") {
        PipelineChange::Created(path) => {
            println!("  {path}: created with extends = \"ptk:proxy\"");
        }
        PipelineChange::AlreadyActive => {}
        PipelineChange::ManualNeeded => {
            println!(
                "  Add `extends = \"ptk:proxy\"` to [pipeline] in your pipeline.toml — \
                 without it the daemon forwards unchanged."
            );
        }
    }

    match crate::llm_proxy::ensure_running(crate::llm_proxy::DEFAULT_BIND) {
        Ok(_) => println!("  ptk-proxy: running"),
        Err(e) => {
            println!("  [warn] ptk-proxy not started: {e}");
            println!("  Start it manually: ptk llm-proxy start");
        }
    }
    if choice == 2 {
        let exe = std::env::current_exe().context("resolve the ptk binary path")?;
        match std::process::Command::new(exe)
            .args(["llm-proxy", "service", "install"])
            .status()
        {
            Ok(s) if s.success() => println!("  service: installed"),
            Ok(s) => println!("  [warn] service install exited with {s}"),
            Err(e) => println!("  [warn] service install failed: {e}"),
        }
    }
    match choice {
        1 => verify_proxy_setup(&base_url, Some("SessionStart autostart")),
        2 => verify_proxy_setup(&base_url, Some("background service")),
        _ => println!("  Keep it across logins: ptk llm-proxy service install"),
    }
    Ok(())
}

#[cfg(feature = "engine")]
/// True when any SessionStart entry already carries the ptk claude hook.
fn session_start_entry_present(root: &serde_json::Value) -> bool {
    root.pointer(&format!(
        "/hooks/{}",
        crate::hooks::constants::SESSION_START_KEY
    ))
    .and_then(|v| v.as_array())
    .is_some_and(|arr| {
        arr.iter().any(|entry| {
            entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .is_some_and(|hs| {
                    hs.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .is_some_and(is_claude_hook_command)
                    })
                })
        })
    })
}

#[cfg(feature = "engine")]
/// SessionStart entries carry no matcher: the hook fires for every session.
fn insert_session_start_entry(root: &mut serde_json::Value) -> Result<()> {
    let obj = root
        .as_object_mut()
        .context("settings.json root is not an object")?;
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("settings.json `hooks` is not an object")?;
    let arr = hooks
        .entry(crate::hooks::constants::SESSION_START_KEY)
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .context("settings.json `SessionStart` is not an array")?;
    arr.push(serde_json::json!({
        "hooks": [{ "type": "command", "command": "ptk hook claude" }]
    }));
    Ok(())
}

#[cfg(feature = "engine")]
enum PipelineChange {
    Created(String),
    AlreadyActive,
    ManualNeeded,
}

/// Make the user pipeline extend a built-in: writes a fresh
/// `~/.config/ptk/pipeline.toml` when none exists; never edits an existing
/// file (the user is told what line to add instead).
#[cfg(feature = "engine")]
fn ensure_pipeline_extends(builtin: &str, extra_body: &str) -> PipelineChange {
    let Some(path) = crate::core::pipeline_config::user_pipeline_path() else {
        return PipelineChange::ManualNeeded;
    };
    let loaded = crate::core::pipeline_config::load();
    if loaded.resolved.extends.iter().any(|e| e == builtin) {
        return PipelineChange::AlreadyActive;
    }
    if crate::core::pipeline_config::any_user_pipeline_files() {
        return PipelineChange::ManualNeeded;
    }
    let content =
        format!("schema_version = 1\n\n[pipeline]\nextends = \"{builtin}\"\n{extra_body}");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::write(&path, content) {
        Ok(()) => PipelineChange::Created(path.display().to_string()),
        Err(_) => PipelineChange::ManualNeeded,
    }
}

#[cfg(feature = "engine")]
/// Delegation on-ramp: offer delegated reads. Writes the
/// adoption block only into a fresh pipeline.toml; an existing file gets the
/// block printed for the user to merge by hand.
fn offer_delegation_onramp(patch_mode: PatchMode, ctx: InitContext) -> Result<()> {
    use std::io::{BufRead, IsTerminal};
    if !matches!(patch_mode, PatchMode::Ask) {
        return Ok(());
    }
    if ctx.dry_run || !std::io::stdin().is_terminal() {
        return Ok(());
    }
    if crate::core::user_env::var("PTK_ONRAMP_DISABLED").as_deref() == Some("1") {
        return Ok(());
    }
    let loaded = crate::core::pipeline_config::load();
    if loaded
        .resolved
        .tasks
        .values()
        .any(|(task, _)| task.upstream.is_some())
    {
        return Ok(());
    }

    eprintln!();
    eprint!("  Delegate oversize reads to a cheaper model (needs a model API key)? [y/N] ");
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    if !matches!(line.trim().to_lowercase().as_str(), "y" | "yes") {
        println!(
            "  Skipped — adopt later with `extends = \"ptk:delegate-reads\"` (see `ptk pipeline show ptk:delegate-reads`)."
        );
        return Ok(());
    }

    const ADOPTION_BODY: &str = "\n[upstream.flash]\nprovider    = \"google\"\nmodel       = \"gemini-2.5-flash\"\napi_key_env = \"GEMINI_API_KEY\"\n\n[task.bulk-read]\nupstream = \"flash\"\n\n[task.code-write]\nupstream = \"flash\"\n";
    match ensure_pipeline_extends("ptk:delegate-reads", ADOPTION_BODY) {
        PipelineChange::Created(path) => {
            println!("  {path}: created with the ptk:delegate-reads adoption block");
            crate::core::pipeline_cmd::print_auth_guidance(
                &crate::core::pipeline_config::load().resolved,
            );
            println!(
                "  Set GEMINI_API_KEY (or edit the [upstream.flash] block), then re-run `ptk init -g`."
            );
        }
        PipelineChange::AlreadyActive => {}
        PipelineChange::ManualNeeded => {
            println!("  Merge this into your pipeline.toml:");
            println!("    [pipeline]");
            println!("    extends = \"ptk:delegate-reads\"");
            for body_line in ADOPTION_BODY.lines().filter(|l| !l.is_empty()) {
                println!("    {body_line}");
            }
        }
    }
    Ok(())
}

/// Prompt user for consent to patch settings.json.
pub(super) fn prompt_user_consent(settings_path: &Path) -> Result<bool> {
    prompt_user_confirmation(&format!("Patch existing {}?", settings_path.display()))
}

pub fn save_telemetry_consent(accepted: bool) -> Result<()> {
    let mut config = crate::core::config::Config::load().unwrap_or_default();
    config.telemetry.consent_given = Some(accepted);
    config.telemetry.enabled = accepted;
    config.telemetry.consent_date = Some(chrono::Utc::now().to_rfc3339());
    config
        .save()
        .context("Failed to save telemetry consent to config.toml")
}

fn prompt_telemetry_consent() -> Result<()> {
    use std::io::{self, BufRead, IsTerminal};

    let config = crate::core::config::Config::load().unwrap_or_default();
    match config.telemetry.consent_given {
        Some(true) => return Ok(()),
        Some(false) => return Ok(()),
        None => {}
    }

    // Explicit opt-out must short-circuit before the TTY heuristic: some
    // non-interactive environments (devcontainer `postCreateCommand`, certain
    // CI agents) hand ptk a pseudo-TTY, so `is_terminal()` returns true even
    // though no human is available to answer — the prompt then hangs forever.
    // Setting `PTK_TELEMETRY_DISABLED=1` is the documented workaround, so the
    // init prompt has to honour it too, not only `telemetry::maybe_ping`.
    if crate::core::telemetry_cmd::telemetry_disabled_by_env() {
        return Ok(());
    }

    if !io::stdin().is_terminal() {
        return Ok(());
    }

    eprintln!();
    eprintln!("--- Telemetry ---");
    eprintln!("PTK collects anonymous usage metrics once per day to improve filters.");
    eprintln!();
    eprintln!("  What:    command names (not arguments), token savings, OS, version");
    eprintln!("  Why:     prioritize filter development for the most-used commands");
    eprintln!("  Who:     PTK AI Labs, contact@rtk-ai.app");
    eprintln!("  Rights:  disable anytime with `ptk telemetry disable`,");
    eprintln!("           request erasure with `ptk telemetry forget`");
    eprintln!("  Details: https://github.com/riaku/ptk/blob/main/docs/TELEMETRY.md");
    eprintln!();
    eprint!("Enable anonymous telemetry? [y/N] ");

    let stdin = io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .context("Failed to read user input")?;

    let accepted = {
        let response = line.trim().to_lowercase();
        response == "y" || response == "yes"
    };

    save_telemetry_consent(accepted)?;

    if accepted {
        eprintln!("  Telemetry enabled. Disable anytime: ptk telemetry disable");
    } else {
        eprintln!("  Telemetry disabled.");
    }

    Ok(())
}

pub(super) fn print_manual_instructions(hook_command: &str, include_opencode: bool) {
    let settings_path = resolve_claude_dir()
        .unwrap_or_else(|_| PathBuf::from(format!("~/{}", CLAUDE_DIR)))
        .join(SETTINGS_JSON);
    println!("\n  MANUAL STEP: Add this to {}:", settings_path.display());
    println!("  {{");
    println!("    \"hooks\": {{ \"PreToolUse\": [{{");
    println!("      \"matcher\": \"Bash\",");
    println!("      \"hooks\": [{{ \"type\": \"command\",");
    println!("        \"command\": \"{}\"", hook_command);
    println!("      }}]");
    println!("    }}]}}");
    println!("  }}");
    if include_opencode {
        println!("\n  Then restart Claude Code and OpenCode. Test with: git status\n");
    } else {
        println!("\n  Then restart Claude Code. Test with: git status\n");
    }
}

/// Full uninstall for Claude, Gemini, Codex, Cursor, Pi, or OMP artifacts.
#[allow(dead_code)] // Kept as the default-policy API for in-crate callers and tests.
pub fn uninstall(
    global: bool,
    gemini: bool,
    codex: bool,
    cursor: bool,
    pi: bool,
    omp: bool,
    ctx: InitContext,
) -> Result<()> {
    uninstall_with_patch_mode(global, gemini, codex, cursor, pi, omp, PatchMode::Ask, ctx)
}

/// Full uninstall with an explicit confirmation policy for managed
/// Pi-compatible extensions.
#[allow(clippy::too_many_arguments)]
pub fn uninstall_with_patch_mode(
    global: bool,
    gemini: bool,
    codex: bool,
    cursor: bool,
    pi: bool,
    omp: bool,
    patch_mode: PatchMode,
    ctx: InitContext,
) -> Result<()> {
    let InitContext {
        verbose, dry_run, ..
    } = ctx;
    if codex {
        uninstall_codex(global, ctx)?;
        if dry_run {
            print_dry_run_footer();
        }
        return Ok(());
    }

    if cursor {
        if !global {
            anyhow::bail!("Cursor uninstall only works with --global flag");
        }
        let cursor_removed = remove_cursor_hooks(ctx).context("Failed to remove Cursor hooks")?;
        if !cursor_removed.is_empty() {
            let header = if dry_run {
                "[dry-run] would uninstall PTK (Cursor):"
            } else {
                "PTK uninstalled (Cursor):"
            };
            println!("{}", header);
            for item in &cursor_removed {
                println!("  - {}", item);
            }
            if !dry_run {
                println!("\nRestart Cursor to apply changes.");
            }
        } else {
            println!("PTK Cursor support was not installed (nothing to remove)");
        }
        if dry_run {
            print_dry_run_footer();
        }
        return Ok(());
    }

    if pi {
        uninstall_pi_with_patch_mode(global, patch_mode, ctx)?;
        return Ok(());
    }

    if omp {
        uninstall_omp_with_patch_mode(global, patch_mode, ctx)?;
        return Ok(());
    }

    if !global {
        anyhow::bail!(
            "Uninstall only works with --global flag. For local projects, manually remove PTK from CLAUDE.md"
        );
    }

    let claude_dir = resolve_claude_dir()?;
    let mut removed = Vec::new();

    // Also uninstall Gemini artifacts if --gemini or always (clean everything)
    if gemini {
        let gemini_removed = uninstall_gemini(ctx)?;
        removed.extend(gemini_removed);
        if !removed.is_empty() {
            let header = if dry_run {
                "[dry-run] would uninstall PTK (Gemini):"
            } else {
                "PTK uninstalled (Gemini):"
            };
            println!("{}", header);
            for item in &removed {
                println!("  - {}", item);
            }
            if !dry_run {
                println!("\nRestart Gemini CLI to apply changes.");
            }
        } else {
            println!("PTK Gemini support was not installed (nothing to remove)");
        }
        if dry_run {
            print_dry_run_footer();
        }
        return Ok(());
    }

    // 1. Remove legacy hook file (if exists from old installation)
    let hook_path = claude_dir.join(HOOKS_SUBDIR).join(REWRITE_HOOK_FILE);
    if hook_path.exists() {
        if dry_run {
            println!(
                "[dry-run] would remove hook script: {}",
                hook_path.display()
            );
        } else {
            // nosemgrep: filesystem-deletion -- expected in hooks/init uninstall-path cleanup and tests.
            fs::remove_file(&hook_path)
                .with_context(|| format!("Failed to remove hook: {}", hook_path.display()))?;
        }
        removed.push(format!("Hook script: {}", hook_path.display()));
    }

    // 1b. Remove integrity hash file
    if dry_run {
        // integrity::remove_hash would delete the sidecar file; just report intent.
        if integrity::hash_path_for(&hook_path).exists() {
            println!("[dry-run] would remove integrity hash sidecar");
            removed.push("Integrity hash: removed".to_string());
        }
    } else if integrity::remove_hash(&hook_path)? {
        removed.push("Integrity hash: removed".to_string());
    }

    // 2. Remove PTK.md
    let ptk_md_path = claude_dir.join(PTK_MD);
    if ptk_md_path.exists() {
        if dry_run {
            println!("[dry-run] would remove PTK.md: {}", ptk_md_path.display());
        } else {
            // nosemgrep: filesystem-deletion -- expected in hooks/init uninstall-path cleanup and tests.
            fs::remove_file(&ptk_md_path)
                .with_context(|| format!("Failed to remove PTK.md: {}", ptk_md_path.display()))?;
        }
        removed.push(format!("PTK.md: {}", ptk_md_path.display()));
    }

    // 3. Remove @PTK.md reference from CLAUDE.md
    let claude_md_path = claude_dir.join(CLAUDE_MD);
    if claude_md_path.exists() {
        let content = fs::read_to_string(&claude_md_path)
            .with_context(|| format!("Failed to read CLAUDE.md: {}", claude_md_path.display()))?;

        let mut claude_md_changed = false;
        let mut working_content = content.clone();

        if has_ptk_reference(&working_content, &[PTK_MD_REF]) {
            let new_content = working_content
                .lines()
                .filter(|line| line.trim() != PTK_MD_REF)
                .collect::<Vec<_>>()
                .join("\n");

            working_content = clean_double_blanks(&new_content);
            claude_md_changed = true;
            removed.push("CLAUDE.md: removed @PTK.md reference".to_string());
        }

        if working_content.contains(PTK_BLOCK_START) {
            let (cleaned, did_remove) = remove_ptk_block(&working_content);
            if did_remove {
                working_content = cleaned;
                claude_md_changed = true;
                removed.push("CLAUDE.md: removed ptk-instructions block".to_string());
            }
        }

        if claude_md_changed {
            let trimmed = working_content.trim();
            if trimmed.is_empty() {
                if dry_run {
                    println!(
                        "[dry-run] would remove CLAUDE.md (empty after cleanup): {}",
                        claude_md_path.display()
                    );
                } else {
                    // nosemgrep: filesystem-deletion
                    fs::remove_file(&claude_md_path).with_context(|| {
                        format!(
                            "Failed to remove empty CLAUDE.md: {}",
                            claude_md_path.display()
                        )
                    })?;
                }
                removed.retain(|r| !r.starts_with("CLAUDE.md:"));
                removed.push("CLAUDE.md: removed (was empty after cleanup)".to_string());
            } else if dry_run {
                println!(
                    "[dry-run] would update CLAUDE.md: {}",
                    claude_md_path.display()
                );
                if verbose > 0 {
                    println!("[dry-run] content:\n{}", working_content);
                }
            } else {
                fs::write(&claude_md_path, &working_content).with_context(|| {
                    format!("Failed to write CLAUDE.md: {}", claude_md_path.display())
                })?;
            }
        }
    }

    // 4. Remove hook entry from settings.json
    let settings_removed = remove_settings_entries(ctx)?;
    if settings_removed.hook {
        removed.push("settings.json: removed PTK hook entry".to_string());
    }
    if settings_removed.proxy_base_url {
        removed.push("settings.json: removed env.ANTHROPIC_BASE_URL (PTK proxy)".to_string());
    }

    // 5. Remove OpenCode plugin
    let opencode_removed = remove_opencode_plugin(ctx)?;
    for path in opencode_removed {
        removed.push(format!("OpenCode plugin: {}", path.display()));
    }

    // 6. Remove Cursor hooks
    let cursor_removed = remove_cursor_hooks(ctx)?;
    removed.extend(cursor_removed);

    // Report results
    if removed.is_empty() {
        println!("PTK was not installed (nothing to remove)");
        println!("  Checked: {}", hook_path.display());
        println!("  Checked: {}", claude_dir.join(PTK_MD).display());
        println!("  Checked: {}", claude_md_path.display());
        println!("  Checked: {}", claude_dir.join(SETTINGS_JSON).display());
    } else {
        let header = if dry_run {
            "[dry-run] would uninstall PTK:"
        } else {
            "PTK uninstalled:"
        };
        println!("{}", header);
        for item in removed {
            println!("  - {}", item);
        }
        if !dry_run {
            println!("\nRestart Claude Code, OpenCode, and Cursor (if used) to apply changes.");
        }
    }

    if dry_run {
        print_dry_run_footer();
    }

    Ok(())
}

/// Clean up consecutive blank lines (collapse 3+ to 2)
/// Used when removing @PTK.md line from CLAUDE.md
pub(super) fn clean_double_blanks(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.trim().is_empty() {
            // Count consecutive blank lines
            let mut blank_count = 0;
            while i < lines.len() && lines[i].trim().is_empty() {
                blank_count += 1;
                i += 1;
            }

            // Keep at most 2 blank lines
            let keep = blank_count.min(2);
            result.extend(std::iter::repeat_n("", keep));
        } else {
            result.push(line);
            i += 1;
        }
    }

    result.join("\n")
}

/// Only command hooks belong to PTK; prompt/agent entries are user-owned.
pub(super) fn is_command_hook(hook: &serde_json::Value, matches: impl Fn(&str) -> bool) -> bool {
    hook.get("type").is_none_or(|kind| kind == "command")
        && hook
            .get("command")
            .and_then(serde_json::Value::as_str)
            .is_some_and(matches)
}

/// Codex/Cursor match tool names with regular expressions.
pub(super) fn group_covers_tool(group: &serde_json::Value, tool: &str) -> bool {
    match group.get("matcher") {
        None | Some(serde_json::Value::Null) => true,
        Some(matcher) => matcher.as_str().is_some_and(|pattern| {
            pattern.is_empty()
                || pattern == "*"
                || regex::Regex::new(pattern).is_ok_and(|regex| regex.is_match(tool))
        }),
    }
}

#[derive(Clone, Copy)]
pub(super) enum HookEntries {
    Grouped,
    Flat,
}

/// Share traversal, while callers retain their host's matcher and ownership rules.
pub(super) fn hook_present(
    root: &serde_json::Value,
    event: &str,
    layout: HookEntries,
    covers: impl Fn(&serde_json::Value) -> bool,
    owns: impl Fn(&serde_json::Value) -> bool,
) -> bool {
    root.get("hooks")
        .and_then(|hooks| hooks.get(event))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                covers(entry)
                    && match layout {
                        HookEntries::Grouped => entry
                            .get("hooks")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|hooks| hooks.iter().any(&owns)),
                        HookEntries::Flat => owns(entry),
                    }
            })
        })
}

pub(super) fn append_hook_entry(
    root: &mut serde_json::Value,
    event: &str,
    entry: serde_json::Value,
) -> Result<()> {
    let hooks = root
        .as_object_mut()
        .context("hook config root is not an object")?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("hooks value is not an object")?;
    hooks
        .entry(event)
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .with_context(|| format!("{event} value is not an array"))?
        .push(entry);
    Ok(())
}

/// Remove only owned entries, pruning a group only when this removal empties it.
pub(super) fn remove_hook_entries(
    root: &mut serde_json::Value,
    event: &str,
    layout: HookEntries,
    owns: impl Fn(&serde_json::Value) -> bool,
) -> bool {
    let Some(entries) = root
        .get_mut("hooks")
        .and_then(|hooks| hooks.get_mut(event))
        .and_then(serde_json::Value::as_array_mut)
    else {
        return false;
    };
    let mut removed = false;
    entries.retain_mut(|entry| match layout {
        HookEntries::Flat => {
            let matched = owns(entry);
            removed |= matched;
            !matched
        }
        HookEntries::Grouped => {
            let Some(hooks) = entry
                .get_mut("hooks")
                .and_then(serde_json::Value::as_array_mut)
            else {
                return true;
            };
            let before = hooks.len();
            hooks.retain(|hook| !owns(hook));
            if hooks.len() == before {
                return true;
            }
            removed = true;
            !hooks.is_empty()
        }
    });
    removed
}

/// Deep-merge PTK hook entry into settings.json under the given event key.
/// Creates the hooks structure if missing, preserves existing hooks.
pub(super) fn insert_hook_entry(
    root: &mut serde_json::Value,
    event_key: &str,
    matcher: &str,
    hook_command: &str,
) -> Result<()> {
    let root_obj = match root.as_object_mut() {
        Some(obj) => obj,
        None => {
            *root = serde_json::json!({});
            root.as_object_mut().expect("just-created json object")
        }
    };

    let hooks = root_obj
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("hooks value is not an object")?;

    let event_array = hooks
        .entry(event_key)
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .with_context(|| format!("{event_key} value is not an array"))?;

    event_array.push(serde_json::json!({
        "matcher": matcher,
        "hooks": [{
            "type": "command",
            "command": hook_command
        }]
    }));
    Ok(())
}

/// Generate .ptk/filters.toml template in the current directory if not present.
pub(super) fn generate_project_filters_template(ctx: InitContext) -> Result<()> {
    let InitContext {
        verbose, dry_run, ..
    } = ctx;
    let ptk_dir = user_dirs::in_working_dir(".ptk");
    let path = ptk_dir.join("filters.toml");

    if path.exists() {
        if verbose > 0 {
            eprintln!(".ptk/filters.toml already exists, skipping template");
        }
        return Ok(());
    }

    if dry_run {
        println!(
            "[dry-run] would create .ptk/filters.toml template: {}",
            path.display()
        );
        return Ok(());
    }

    fs::create_dir_all(&ptk_dir)
        .with_context(|| format!("Failed to create directory: {}", ptk_dir.display()))?;
    fs::write(&path, FILTERS_TEMPLATE)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    println!(
        "  filters:   {} (template, edit to add project filters)",
        path.display()
    );
    Ok(())
}

/// Generate ~/.config/ptk/filters.toml template if not present.
pub(super) fn generate_global_filters_template(ctx: InitContext) -> Result<()> {
    let InitContext {
        verbose, dry_run, ..
    } = ctx;
    let ptk_dir = user_dirs::config().unwrap_or_else(|| {
        std::path::PathBuf::from(".config").join(crate::core::constants::PTK_DATA_DIR)
    });
    let path = ptk_dir.join(crate::core::constants::FILTERS_TOML);

    if path.exists() {
        if verbose > 0 {
            eprintln!("{} already exists, skipping template", path.display());
        }
        return Ok(());
    }

    if dry_run {
        println!(
            "[dry-run] would create global filters template: {}",
            path.display()
        );
        return Ok(());
    }

    fs::create_dir_all(&ptk_dir)
        .with_context(|| format!("Failed to create directory: {}", ptk_dir.display()))?;
    fs::write(&path, FILTERS_GLOBAL_TEMPLATE)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    println!(
        "  filters:   {} (template, edit to add user-global filters)",
        path.display()
    );
    Ok(())
}

pub fn finalize_filter_trust(global: bool, dry_run: bool, trust: FilterTrust) -> Result<()> {
    let paths = crate::hooks::trust::gated_filter_paths();
    let path = match if global { paths.get(1) } else { paths.first() } {
        Some(p) => p,
        None => return Ok(()),
    };
    if !path.exists() {
        return Ok(());
    }

    let status = crate::hooks::trust::check_trust(path)
        .unwrap_or(crate::hooks::trust::TrustStatus::Untrusted);
    if matches!(
        status,
        crate::hooks::trust::TrustStatus::Trusted | crate::hooks::trust::TrustStatus::EnvOverride
    ) {
        return Ok(());
    }

    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(_) => return Ok(()),
    };
    let content = String::from_utf8_lossy(&bytes);
    let filters = crate::core::toml_filter::active_filter_summaries(&content);
    if filters.is_empty() {
        return Ok(());
    }

    if dry_run {
        println!(
            "[dry-run] {} untrusted custom filter(s) in {}",
            filters.len(),
            path.display()
        );
        return Ok(());
    }

    let scope = if global { "global" } else { "project" };
    crate::hooks::trust::print_filter_notice(path, scope, &filters);

    let enable = match trust {
        FilterTrust::Trust => true,
        FilterTrust::Skip => false,
        FilterTrust::Ask => crate::hooks::trust::confirm_enable_at_tty()?,
    };

    if enable {
        let hash = crate::hooks::integrity::compute_hash_bytes(&bytes);
        crate::hooks::trust::trust_filter_with_hash(path, &hash)?;
        eprintln!("Enabled. Revoke with `ptk untrust`.");
    } else {
        eprintln!("\x1b[33m  Not enabled — run `ptk trust` to review and enable.\x1b[0m");
    }
    Ok(())
}

/// An agent's config directory: `override_dir` when set and non-empty, else `home/subdir`.
/// `error` names the agent and its variable when neither is available.
pub(super) fn resolve_config_dir(
    override_dir: Option<OsString>,
    home_dir: Option<PathBuf>,
    subdir: &str,
    error: &'static str,
) -> Result<PathBuf> {
    if let Some(dir) = override_dir.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    home_dir.map(|home| home.join(subdir)).context(error)
}

pub(super) fn resolve_home_subdir(subdir: &str) -> Result<PathBuf> {
    user_dirs::home()
        .map(|h| h.join(subdir))
        .context(if cfg!(windows) {
            "Cannot determine home directory. Is %USERPROFILE% set?"
        } else {
            "Cannot determine home directory. Is $HOME set?"
        })
}

pub fn resolve_claude_dir() -> Result<PathBuf> {
    resolve_claude_dir_from(
        user_dirs::env_path("CLAUDE_CONFIG_DIR").map(PathBuf::from),
        user_dirs::home(),
    )
}

pub(super) fn resolve_claude_dir_from(
    claude_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
) -> Result<PathBuf> {
    resolve_config_dir(
        claude_dir.map(PathBuf::into_os_string),
        home_dir,
        CLAUDE_DIR,
        "Cannot determine Claude config directory. Set $CLAUDE_CONFIG_DIR or $HOME.",
    )
}

/// Show current ptk configuration
pub fn show_config(codex: bool, omp: bool) -> Result<()> {
    if omp {
        return show_omp_config();
    }
    if codex {
        return show_codex_config();
    }

    show_claude_config()
}

fn show_claude_config() -> Result<()> {
    let claude_dir = resolve_claude_dir()?;
    let hook_path = claude_dir.join(HOOKS_SUBDIR).join(REWRITE_HOOK_FILE);
    let ptk_md_path = claude_dir.join(PTK_MD);
    let global_claude_md = claude_dir.join(CLAUDE_MD);
    let local_claude_md = user_dirs::in_working_dir(CLAUDE_MD);

    println!("ptk Configuration:\n");

    // Check hook: prefer binary command detection, fall back to script file
    let settings_path = claude_dir.join(SETTINGS_JSON);
    let binary_hook_registered = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path).unwrap_or_default();
        if let Ok(root) = from_json_str::<serde_json::Value>(&content) {
            hook_already_present(&root, PRE_TOOL_USE_KEY, CLAUDE_HOOK_COMMAND)
        } else {
            false
        }
    } else {
        false
    };

    if binary_hook_registered {
        println!("[ok] Hook: {} (native binary command)", CLAUDE_HOOK_COMMAND);
    } else if hook_path.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&hook_path)?;
            let perms = metadata.permissions();
            let is_executable = perms.mode() & 0o111 != 0;

            let hook_content = fs::read_to_string(&hook_path)?;
            let has_guards =
                hook_content.contains("command -v ptk") && hook_content.contains("command -v jq");
            let is_thin_delegator = hook_content.contains("ptk rewrite");
            let hook_version = super::hook_check::parse_hook_version(&hook_content);

            if !is_executable {
                println!(
                    "[warn] Hook: {} (NOT executable - run: chmod +x)",
                    hook_path.display()
                );
            } else if !is_thin_delegator {
                println!(
                    "[warn] Hook: {} (outdated — run `ptk init -g` to upgrade to native binary)",
                    hook_path.display()
                );
            } else if is_executable && has_guards {
                println!(
                    "[warn] Hook: {} (legacy script v{} — run `ptk init -g` to upgrade)",
                    hook_path.display(),
                    hook_version
                );
            } else {
                println!(
                    "[warn] Hook: {} (no guards - outdated)",
                    hook_path.display()
                );
            }
        }

        #[cfg(not(unix))]
        {
            println!(
                "[warn] Hook: {} (legacy script — run `ptk init -g` to upgrade)",
                hook_path.display()
            );
        }
    } else {
        println!("[--] Hook: not found");
    }

    // Check PTK.md
    if ptk_md_path.exists() {
        println!("[ok] PTK.md: {} (slim mode)", ptk_md_path.display());
    } else {
        println!("[--] PTK.md: not found");
    }

    // Check hook integrity (only relevant for legacy script hooks)
    if hook_path.exists() && !binary_hook_registered {
        match integrity::verify_hook_at(&hook_path) {
            Ok(integrity::IntegrityStatus::Verified) => {
                println!("[ok] Integrity: hook hash verified");
            }
            Ok(integrity::IntegrityStatus::Tampered { .. }) => {
                println!("[FAIL] Integrity: hook modified outside ptk init (run: ptk verify)");
            }
            Ok(integrity::IntegrityStatus::NoBaseline) => {
                println!("[warn] Integrity: no baseline hash (run: ptk init -g to establish)");
            }
            Ok(integrity::IntegrityStatus::NotInstalled)
            | Ok(integrity::IntegrityStatus::OrphanedHash) => {
                // Don't show integrity line if hook isn't installed
            }
            Err(_) => {
                println!("[warn] Integrity: check failed");
            }
        }
    }

    // Check global CLAUDE.md
    if global_claude_md.exists() {
        let content = fs::read_to_string(&global_claude_md)?;
        if content.contains(PTK_MD_REF) {
            println!("[ok] Global (~/.claude/CLAUDE.md): @PTK.md reference");
        } else if content.contains(PTK_BLOCK_START) {
            println!(
                "[warn] Global (~/.claude/CLAUDE.md): old PTK block (run: ptk init -g to migrate)"
            );
        } else {
            println!("[--] Global (~/.claude/CLAUDE.md): exists but ptk not configured");
        }
    } else {
        println!("[--] Global (~/.claude/CLAUDE.md): not found");
    }

    // Check local CLAUDE.md
    if local_claude_md.exists() {
        let content = fs::read_to_string(&local_claude_md)?;
        if content.contains("ptk") {
            println!("[ok] Local (./CLAUDE.md): ptk enabled");
        } else {
            println!("[--] Local (./CLAUDE.md): exists but ptk not configured");
        }
    } else {
        println!("[--] Local (./CLAUDE.md): not found");
    }

    // Check settings.json (detailed status)
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let content = strip_leading_bom(&content);
        if !content.trim().is_empty() {
            if let Ok(root) = from_json_str::<serde_json::Value>(content) {
                let required = claude::required_claude_events();
                let missing: Vec<&str> = required
                    .iter()
                    .filter(|(event, _)| !hook_already_present(&root, event, CLAUDE_HOOK_COMMAND))
                    .map(|(event, _)| *event)
                    .collect();
                if required.is_empty() {
                    println!("[--] settings.json: the active pipeline registers no hook events");
                } else if missing.is_empty() {
                    let gaps = hook_gaps(&root, &required);
                    if gaps.is_empty() {
                        let events: Vec<&str> = required.iter().map(|(event, _)| *event).collect();
                        println!(
                            "[ok] settings.json: PTK hooks configured ({})",
                            events.join(" + ")
                        );
                    } else {
                        println!("[warn] settings.json: {}", gaps.join("; "));
                        println!("    Run: ptk init -g --auto-patch");
                    }
                } else {
                    println!("[warn] settings.json: {} hook missing", missing.join(", "));
                    println!("    Run: ptk init -g --auto-patch");
                }
            } else {
                println!("[warn] settings.json: exists but invalid JSON");
            }
        } else {
            println!("[--] settings.json: empty");
        }
    } else {
        println!("[--] settings.json: not found");
    }

    // Check OpenCode plugin
    match resolve_opencode_dir() {
        Ok(opencode_dir) => {
            let plugin = opencode_plugin_path(&opencode_dir);
            if plugin.exists() {
                println!("[ok] OpenCode: plugin installed ({})", plugin.display());
            } else {
                println!("[--] OpenCode: plugin not found");
            }
        }
        _ => println!("[--] OpenCode: config dir not found"),
    }

    // Check Cursor hooks
    match resolve_cursor_dir() {
        Ok(cursor_dir) => {
            let cursor_hook = cursor_dir.join(HOOKS_SUBDIR).join(REWRITE_HOOK_FILE);
            let cursor_hooks_json = cursor_dir.join(HOOKS_JSON);

            // Check for binary command in hooks.json first
            let cursor_binary_registered = if cursor_hooks_json.exists() {
                let content = fs::read_to_string(&cursor_hooks_json).unwrap_or_default();
                if let Ok(root) = from_json_str::<serde_json::Value>(&content) {
                    cursor_hook_already_present(&root)
                } else {
                    false
                }
            } else {
                false
            };

            if cursor_binary_registered {
                println!("[ok] Cursor hook: registered in hooks.json");
            } else if cursor_hook.exists() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let meta = fs::metadata(&cursor_hook)?;
                    let is_executable = meta.permissions().mode() & 0o111 != 0;

                    if !is_executable {
                        println!(
                            "[warn] Cursor hook: {} (legacy script, NOT executable)",
                            cursor_hook.display()
                        );
                    } else {
                        println!(
                            "[warn] Cursor hook: {} (legacy script — run `ptk init -g --agent cursor` to upgrade)",
                            cursor_hook.display()
                        );
                    }
                }

                #[cfg(not(unix))]
                {
                    println!(
                        "[warn] Cursor hook: {} (legacy script — run `ptk init -g --agent cursor` to upgrade)",
                        cursor_hook.display()
                    );
                }
            } else {
                println!("[--] Cursor hook: not found");
            }
        }
        _ => println!("[--] Cursor: home dir not found"),
    }

    println!("\nUsage:");
    println!("  ptk init              # Full injection into local CLAUDE.md");
    println!("  ptk init -g           # Hook + PTK.md + @PTK.md + settings.json (recommended)");
    println!("  ptk init -g --auto-patch    # Same as above but no prompt");
    println!("  ptk init -g --no-patch      # Skip settings.json (manual setup)");
    println!("  ptk init -g --uninstall     # Remove all PTK artifacts");
    println!("  ptk init -g --claude-md     # Legacy: full injection into ~/.claude/CLAUDE.md");
    println!("  ptk init -g --hook-only     # Hook only, no PTK.md");
    println!("  ptk init --codex            # Configure local AGENTS.md + PTK.md + hooks.json");
    println!("  ptk init -g --codex         # Configure global AGENTS.md + PTK.md + hooks.json");
    println!("  ptk init -g --opencode      # Claude setup + OpenCode plugin");
    println!("  ptk init -g --agent cursor  # Install Cursor Agent hooks");

    Ok(())
}

#[cfg(test)]
use tempfile::TempDir;
#[cfg(test)]
pub(super) fn with_claude_dir_override<F: FnOnce(&Path)>(tmp: &TempDir, f: F) {
    let claude_dir = tmp.path().join(CLAUDE_DIR);
    fs::create_dir_all(&claude_dir).unwrap();

    test_isolation::with_agent_dir(tmp.path(), "CLAUDE_CONFIG_DIR", &claude_dir, || {
        f(&claude_dir)
    });
}

#[cfg(test)]
pub(super) fn with_missing_claude_dir_override<F: FnOnce(&Path)>(tmp: &TempDir, f: F) {
    let claude_dir = tmp.path().join(CLAUDE_DIR);
    assert!(
        !claude_dir.exists(),
        "test precondition: Claude config dir must be missing"
    );

    test_isolation::with_agent_dir(tmp.path(), "CLAUDE_CONFIG_DIR", &claude_dir, || {
        f(&claude_dir)
    });
}

#[cfg(test)]
mod tests {
    use crate::hooks::constants::POST_TOOL_USE_KEY;

    fn settings_with(entries: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "hooks": entries })
    }

    fn ptk_entry(matcher: &str) -> serde_json::Value {
        serde_json::json!([{ "matcher": matcher, "hooks": [{ "type": "command", "command": "ptk hook claude" }] }])
    }

    #[test]
    fn hook_gaps_names_tools_the_installed_matcher_misses() {
        let root = settings_with(serde_json::json!({ "PreToolUse": ptk_entry("Bash") }));
        let required = vec![(PRE_TOOL_USE_KEY, "Bash|Read".to_string())];
        assert_eq!(
            hook_gaps(&root, &required),
            vec!["PreToolUse does not cover Read"]
        );
    }

    #[test]
    fn hook_gaps_names_a_missing_event() {
        let root = settings_with(serde_json::json!({ "PreToolUse": ptk_entry("Bash") }));
        let required = vec![
            (PRE_TOOL_USE_KEY, "Bash".to_string()),
            (POST_TOOL_USE_KEY, "Bash".to_string()),
        ];
        assert_eq!(
            hook_gaps(&root, &required),
            vec!["PostToolUse hook missing"]
        );
    }

    #[test]
    fn hook_gaps_is_silent_when_covered_or_ptk_is_not_installed() {
        let wildcard = settings_with(serde_json::json!({ "PreToolUse": ptk_entry("*") }));
        let required = vec![(PRE_TOOL_USE_KEY, "Bash|Read".to_string())];
        assert!(hook_gaps(&wildcard, &required).is_empty());
        let exact = settings_with(serde_json::json!({ "PreToolUse": ptk_entry("Read|Bash") }));
        assert!(hook_gaps(&exact, &required).is_empty());
        let foreign = settings_with(serde_json::json!({
            "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "other-tool" }] }]
        }));
        assert!(
            hook_gaps(&foreign, &required).is_empty(),
            "no ptk hook: not our business"
        );
    }

    use super::claude::{remove_hook_from_json, remove_legacy_hook_entries_from_json};
    use super::codex::{codex_hook_already_present, remove_codex_hook_from_json};
    use super::cursor::remove_legacy_cursor_hook_entries_from_json;
    use super::trae::{
        insert_trae_hook_entry, remove_trae_hook_from_json, trae_hook_already_present,
    };
    use super::*;
    use crate::hooks::constants::{
        CLAUDE_PRE_MATCHER, CODEX_HOOK_COMMAND, CURSOR_HOOK_COMMAND, TRAE_HOOK_COMMAND,
        TRAE_RUN_COMMAND_MATCHER,
    };
    use tempfile::TempDir;

    #[test]
    fn test_awareness_content_maps_each_level_to_its_file() {
        assert_eq!(
            awareness_content(AwarenessLevel::Default),
            PTK_AWARENESS_DEFAULT
        );
        assert_eq!(awareness_content(AwarenessLevel::High), PTK_AWARENESS_HIGH);
        assert_eq!(awareness_content(AwarenessLevel::Full), PTK_AWARENESS_FULL);
    }

    #[cfg(feature = "engine")]
    #[test]
    fn test_instructions_body_without_directives_is_byte_identical() {
        let (pipeline, diagnostics) = ptk_engine::pipeline::resolve(&[]);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        for level in [
            AwarenessLevel::Default,
            AwarenessLevel::High,
            AwarenessLevel::Full,
        ] {
            assert_eq!(
                instructions_body_for(level, &pipeline),
                awareness_content(level),
                "no directives configured -> init output must not change"
            );
        }
    }

    #[cfg(feature = "engine")]
    #[test]
    fn test_instructions_body_appends_directives_and_honors_level() {
        let parsed = ptk_engine::pipeline::parse(
            r#"
schema_version = 1
[[directive]]
use = "terse-output"
channel = "awareness_file"

[[directive]]
name = "delegate-tools"
text = "For files over 350 lines use bulk-read."

[override.ptk-awareness]
level = "high"
"#,
            "u.toml",
        );
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let (pipeline, diagnostics) =
            ptk_engine::pipeline::resolve(&[ptk_engine::pipeline::SourceFile {
                scope: ptk_engine::pipeline::Scope::User,
                label: "u.toml".to_string(),
                file: parsed.file.expect("parses"),
            }]);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let body = instructions_body_for(AwarenessLevel::Default, &pipeline);
        assert!(
            body.starts_with(PTK_AWARENESS_HIGH.trim_end()),
            "[override.ptk-awareness] level=high overrides the config level"
        );
        // Anchored on the directive's opening line: this assertion is about
        // declaration ORDER, so it wants the position where terse-output
        // starts, not an arbitrary sentence inside it.
        let terse = body
            .find("Keep prose short")
            .expect("terse-output text present");
        let custom = body
            .find("bulk-read")
            .expect("custom directive text present");
        assert!(terse < custom, "declaration order preserved");
        assert!(body.ends_with('\n'));
    }

    #[test]
    fn test_awareness_levels_nest() {
        let contract = PTK_AWARENESS_DEFAULT.trim_end();
        assert!(
            PTK_AWARENESS_HIGH.starts_with(contract),
            "high must start with the default output contract verbatim"
        );
        assert!(
            PTK_AWARENESS_FULL.contains(contract),
            "full must contain the default output contract verbatim"
        );
    }

    #[test]
    fn test_awareness_activation_rule_only_in_full() {
        const ACTIVATION: &str = "Prefix every shell command with `ptk`";
        assert!(!PTK_AWARENESS_DEFAULT.contains(ACTIVATION));
        assert!(!PTK_AWARENESS_HIGH.contains(ACTIVATION));
        assert!(PTK_AWARENESS_FULL.contains(ACTIVATION));
    }

    #[test]
    fn test_awareness_meta_commands_present_above_default() {
        for (name, content) in [
            ("default", PTK_AWARENESS_DEFAULT),
            ("high", PTK_AWARENESS_HIGH),
            ("full", PTK_AWARENESS_FULL),
        ] {
            assert!(
                content.contains("ptk bypass <cmd>"),
                "{name} must mention ptk bypass"
            );
        }
        for (name, content) in [("high", PTK_AWARENESS_HIGH), ("full", PTK_AWARENESS_FULL)] {
            assert!(
                content.contains("PTK_DISABLED=1"),
                "{name} must mention PTK_DISABLED"
            );
            assert!(content.contains("ptk gain"), "{name} must mention ptk gain");
        }
        assert!(!PTK_AWARENESS_DEFAULT.contains("ptk gain"));
    }

    #[test]
    fn test_awareness_files_stay_within_line_budget() {
        assert!(PTK_AWARENESS_DEFAULT.lines().count() <= 10);
        assert!(PTK_AWARENESS_HIGH.lines().count() <= 21);
        assert!(PTK_AWARENESS_FULL.lines().count() <= 25);
    }

    #[test]
    fn test_ptk_block_wraps_body_in_markers() {
        let block = ptk_block(PTK_AWARENESS_FULL);
        assert!(block.starts_with(&format!("{PTK_BLOCK_START} v2 -->\n")));
        assert!(block.ends_with(&format!("{PTK_BLOCK_END}\n")));
        assert!(block.contains(PTK_AWARENESS_FULL));
    }

    #[test]
    fn test_local_init_block_follows_awareness_level_not_legacy() {
        for level in [
            AwarenessLevel::Default,
            AwarenessLevel::High,
            AwarenessLevel::Full,
        ] {
            let block = ptk_block(awareness_content(level));
            assert!(block.starts_with(PTK_BLOCK_START));
            assert!(block.contains(awareness_content(level).trim_end()));
        }
        // Plain local init must not ship the legacy Golden Rule; --claude-md keeps it.
        assert!(!ptk_block(awareness_content(AwarenessLevel::Default)).contains("Golden Rule"));
        assert!(!ptk_block(awareness_content(AwarenessLevel::High)).contains("Golden Rule"));
        assert!(PTK_INSTRUCTIONS.contains("Golden Rule"));
    }

    #[test]
    fn test_init_mentions_all_top_level_commands() {
        for cmd in [
            "ptk cargo",
            "ptk gh",
            "ptk vitest",
            "ptk tsc",
            "ptk lint",
            "ptk prettier",
            "ptk next",
            "ptk playwright",
            "ptk prisma",
            "ptk pnpm",
            "ptk npm",
            "ptk uv",
            "ptk curl",
            "ptk git",
            "ptk docker",
            "ptk kubectl",
        ] {
            assert!(
                PTK_INSTRUCTIONS.contains(cmd),
                "Missing {cmd} in PTK_INSTRUCTIONS"
            );
        }
    }

    #[test]
    fn test_init_has_version_marker() {
        assert!(
            PTK_INSTRUCTIONS.contains(PTK_BLOCK_START),
            "PTK_INSTRUCTIONS must start with PTK_BLOCK_START marker"
        );
        assert!(
            PTK_INSTRUCTIONS.contains(PTK_BLOCK_END),
            "PTK_INSTRUCTIONS must end with PTK_BLOCK_END marker"
        );
    }

    #[test]
    fn test_migration_removes_old_block() {
        let input = format!(
            "# My Config\n\n{} v2 -->\nOLD PTK STUFF\n{}\n\nMore content",
            PTK_BLOCK_START, PTK_BLOCK_END
        );

        let (result, migrated) = remove_ptk_block(&input);
        assert!(migrated);
        assert!(!result.contains("OLD PTK STUFF"));
        assert!(result.contains("# My Config"));
        assert!(result.contains("More content"));
    }

    #[test]
    fn test_migration_warns_on_missing_end_marker() {
        let input = format!("{} v2 -->\nOLD STUFF\nNo end marker", PTK_BLOCK_START);
        let (result, migrated) = remove_ptk_block(&input);
        assert!(!migrated);
        assert_eq!(result, input);
    }

    #[test]
    fn test_init_is_idempotent() {
        let temp = TempDir::new().unwrap();
        let claude_md = temp.path().join("CLAUDE.md");

        fs::write(&claude_md, "# My stuff\n\n@PTK.md\n").unwrap();

        let content = fs::read_to_string(&claude_md).unwrap();
        let count = content.matches("@PTK.md").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_write_if_changed_dry_run_does_not_create_file() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("ptk-test.md");

        let changed = write_if_changed(
            &target,
            "some content",
            "test file",
            InitContext {
                dry_run: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(
            changed,
            "dry-run should report would-change for missing file"
        );
        assert!(
            !target.exists(),
            "dry-run must not create file: {}",
            target.display()
        );
    }

    #[test]
    fn test_write_if_changed_dry_run_does_not_modify_existing_file() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("ptk-test.md");
        fs::write(&target, "original").unwrap();

        let changed = write_if_changed(
            &target,
            "new content",
            "test file",
            InitContext {
                dry_run: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(changed, "dry-run should report would-change");
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            "original",
            "dry-run must not modify file contents"
        );
    }

    #[test]
    fn test_local_init_unchanged() {
        // Local init should use claude-md mode
        let temp = TempDir::new().unwrap();
        let claude_md = temp.path().join("CLAUDE.md");

        fs::write(&claude_md, PTK_INSTRUCTIONS).unwrap();
        let content = fs::read_to_string(&claude_md).unwrap();

        assert!(content.contains(PTK_BLOCK_START));
    }

    // Tests for hook_already_present()
    // Tests for insert_hook_entry()
    #[test]
    fn test_insert_hook_entry_empty_root() {
        let mut json_content = serde_json::json!({});
        let hook_command = "/Users/test/.claude/hooks/ptk-rewrite.sh";

        insert_hook_entry(
            &mut json_content,
            PRE_TOOL_USE_KEY,
            CLAUDE_PRE_MATCHER,
            hook_command,
        )
        .unwrap();

        // Should create full structure
        assert!(json_content.get("hooks").is_some());
        assert!(
            json_content
                .get("hooks")
                .unwrap()
                .get("PreToolUse")
                .is_some()
        );

        let pre_tool_use = json_content["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre_tool_use.len(), 1);

        let command = pre_tool_use[0]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(command, hook_command);
    }

    #[test]
    fn test_insert_hook_entry_preserves_existing() {
        let mut json_content = serde_json::json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": [{
                        "type": "command",
                        "command": "/some/other/hook.sh"
                    }]
                }]
            }
        });

        let hook_command = "/Users/test/.claude/hooks/ptk-rewrite.sh";
        insert_hook_entry(
            &mut json_content,
            PRE_TOOL_USE_KEY,
            CLAUDE_PRE_MATCHER,
            hook_command,
        )
        .unwrap();

        let pre_tool_use = json_content["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre_tool_use.len(), 2); // Should have both hooks

        // Check first hook is preserved
        let first_command = pre_tool_use[0]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(first_command, "/some/other/hook.sh");

        // Check second hook is PTK
        let second_command = pre_tool_use[1]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(second_command, hook_command);
    }

    #[test]
    fn test_insert_hook_preserves_other_keys() {
        let mut json_content = serde_json::json!({
            "env": {"PATH": "/custom/path"},
            "permissions": {"allowAll": true},
            "model": "claude-sonnet-4"
        });

        let hook_command = "/Users/test/.claude/hooks/ptk-rewrite.sh";
        insert_hook_entry(
            &mut json_content,
            PRE_TOOL_USE_KEY,
            CLAUDE_PRE_MATCHER,
            hook_command,
        )
        .unwrap();

        // Should preserve all other keys
        assert_eq!(json_content["env"]["PATH"], "/custom/path");
        assert_eq!(json_content["permissions"]["allowAll"], true);
        assert_eq!(json_content["model"], "claude-sonnet-4");

        // And add hooks
        assert!(json_content.get("hooks").is_some());
    }

    // Tests for atomic_write()
    #[test]
    fn test_atomic_write() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.json");

        let content = r#"{"key": "value"}"#;
        atomic_write(&file_path, content).unwrap();

        assert!(file_path.exists());
        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, content);
    }

    #[cfg(unix)]
    #[test]
    fn test_atomic_write_preserves_symlink() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let target_path = temp.path().join("real-settings.json");
        let link_path = temp.path().join("settings.json");

        fs::write(&target_path, "{}").expect("seed target file");
        symlink(&target_path, &link_path).expect("create symlink");

        atomic_write(&link_path, "{\"hooks\":{}}").unwrap();

        let meta = fs::symlink_metadata(&link_path).unwrap();
        assert!(meta.file_type().is_symlink(), "symlink must survive");
        let written = fs::read_to_string(&target_path).unwrap();
        assert_eq!(written, "{\"hooks\":{}}");
    }

    #[cfg(unix)]
    #[test]
    fn test_atomic_write_preserves_relative_symlink() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("real");
        fs::create_dir(&subdir).unwrap();
        let target_path = subdir.join("settings.json");
        let link_path = temp.path().join("settings.json");

        fs::write(&target_path, "{}").expect("seed target file");
        symlink(Path::new("real/settings.json"), &link_path).expect("create relative symlink");

        atomic_write(&link_path, "{\"patched\":true}").unwrap();

        let meta = fs::symlink_metadata(&link_path).unwrap();
        assert!(meta.file_type().is_symlink(), "symlink must survive");
        let written = fs::read_to_string(&target_path).unwrap();
        assert_eq!(written, "{\"patched\":true}");
    }

    // Test for preserve_order round-trip
    #[test]
    fn test_preserve_order_round_trip() {
        let original = r#"{"env": {"PATH": "/usr/bin"}, "permissions": {"allowAll": true}, "model": "claude-sonnet-4"}"#;
        let parsed: serde_json::Value = serde_json::from_str(original).unwrap();
        let serialized = serde_json::to_string(&parsed).unwrap();

        // Keys should appear in same order
        let _original_keys: Vec<&str> = original.split("\"").filter(|s| s.contains(":")).collect();
        let _serialized_keys: Vec<&str> =
            serialized.split("\"").filter(|s| s.contains(":")).collect();

        // Just check that keys exist (preserve_order doesn't guarantee exact order in nested objects)
        assert!(serialized.contains("\"env\""));
        assert!(serialized.contains("\"permissions\""));
        assert!(serialized.contains("\"model\""));
    }

    // Tests for clean_double_blanks()
    #[test]
    fn test_clean_double_blanks() {
        // Input: line1, 2 blank lines, line2, 1 blank line, line3, 3 blank lines, line4
        // Expected: line1, 2 blank lines (kept), line2, 1 blank line, line3, 2 blank lines (max), line4
        let input = "line1\n\n\nline2\n\nline3\n\n\n\nline4";
        // That's: line1 \n \n \n line2 \n \n line3 \n \n \n \n line4
        // Which is: line1, blank, blank, line2, blank, line3, blank, blank, blank, line4
        // So 2 blanks after line1 (keep both), 1 blank after line2 (keep), 3 blanks after line3 (keep 2)
        let expected = "line1\n\n\nline2\n\nline3\n\n\nline4";
        assert_eq!(clean_double_blanks(input), expected);
    }

    #[test]
    fn test_clean_double_blanks_preserves_single() {
        let input = "line1\n\nline2\n\nline3";
        assert_eq!(clean_double_blanks(input), input); // No change
    }

    // Tests for remove_hook_from_settings()
    // Legacy migration tests

    // The next three tests exercise every host's hook registration side by side, which is why
    // the per-host helpers they call are pub(super) rather than private.
    #[test]
    fn test_legacy_migration_preserves_mixed_groups_and_prompt_hooks() {
        let legacy = "/home/user/hooks/ptk-rewrite.sh";
        let mut root = serde_json::json!({"hooks": {"PreToolUse": [
            {"hooks": [
                {"command": legacy},
                {"type": "prompt", "command": legacy},
                {"command": "echo user"},
                {"command": CLAUDE_HOOK_COMMAND}
            ]},
            {"hooks": []}
        ]}});
        assert!(remove_legacy_hook_entries_from_json(&mut root));
        assert!(!remove_legacy_hook_entries_from_json(&mut root));
        assert_eq!(
            root["hooks"]["PreToolUse"],
            serde_json::json!([
                {"hooks": [
                    {"type": "prompt", "command": legacy},
                    {"command": "echo user"},
                    {"command": CLAUDE_HOOK_COMMAND}
                ]},
                {"hooks": []}
            ])
        );
        let mut cursor = serde_json::json!({"hooks": {"preToolUse": [
            {"command": legacy},
            {"type": "prompt", "command": legacy},
            {"command": CURSOR_HOOK_COMMAND}
        ]}});
        assert!(remove_legacy_cursor_hook_entries_from_json(&mut cursor));
        assert!(!remove_legacy_cursor_hook_entries_from_json(&mut cursor));
        assert_eq!(
            cursor["hooks"]["preToolUse"],
            serde_json::json!([
                {"type": "prompt", "command": legacy},
                {"command": CURSOR_HOOK_COMMAND}
            ])
        );
    }

    #[test]
    fn test_hook_presence_respects_host_matcher_forms() {
        for matcher in [
            None,
            Some(""),
            Some("*"),
            Some("Bash"),
            Some("Read|Bash"),
            Some("^Ba"),
        ] {
            let mut root = serde_json::json!({"hooks": {"PreToolUse": [
                {"hooks": [{"command": CODEX_HOOK_COMMAND}]}
            ]}});
            if let Some(matcher) = matcher {
                root["hooks"]["PreToolUse"][0]["matcher"] = serde_json::json!(matcher);
            }
            assert!(codex_hook_already_present(&root), "{matcher:?}");
            root["hooks"]["PreToolUse"][0]["hooks"][0]["command"] =
                serde_json::json!(CLAUDE_HOOK_COMMAND);
            assert!(
                hook_already_present(&root, PRE_TOOL_USE_KEY, CLAUDE_HOOK_COMMAND),
                "{matcher:?}"
            );
        }
        for matcher in [
            serde_json::Value::Null,
            serde_json::json!("Read, Bash"),
            serde_json::json!("Ba"),
            serde_json::json!("["),
        ] {
            let mut root = serde_json::json!({"hooks": {"PreToolUse": [
                {"matcher": matcher, "hooks": [{"command": CLAUDE_HOOK_COMMAND}]}
            ]}});
            assert!(hook_already_present(
                &root,
                PRE_TOOL_USE_KEY,
                CLAUDE_HOOK_COMMAND
            ));
            assert!(claude::refresh_hook_matcher(
                &mut root,
                PRE_TOOL_USE_KEY,
                CLAUDE_PRE_MATCHER
            ));
            assert_eq!(
                root["hooks"]["PreToolUse"][0]["matcher"],
                serde_json::json!(CLAUDE_PRE_MATCHER)
            );
        }
        for matcher in [
            None,
            Some(""),
            Some("*"),
            Some("Shell"),
            Some("Read|Shell"),
            Some("^Sh"),
        ] {
            let mut root =
                serde_json::json!({"hooks": {"preToolUse": [{"command": CURSOR_HOOK_COMMAND}]}});
            if let Some(matcher) = matcher {
                root["hooks"]["preToolUse"][0]["matcher"] = serde_json::json!(matcher);
            }
            assert!(cursor_hook_already_present(&root), "{matcher:?}");
        }
    }

    #[test]
    fn test_grouped_registration_preserves_user_hooks_and_checks_matcher() {
        for (command, tool) in [
            (CLAUDE_HOOK_COMMAND, "Bash"),
            (CODEX_HOOK_COMMAND, "Bash"),
            (TRAE_HOOK_COMMAND, TRAE_RUN_COMMAND_MATCHER),
        ] {
            let present = |root: &serde_json::Value| match command {
                CLAUDE_HOOK_COMMAND => hook_already_present(root, PRE_TOOL_USE_KEY, command),
                CODEX_HOOK_COMMAND => codex_hook_already_present(root),
                _ => trae_hook_already_present(root),
            };
            let mut root = serde_json::json!({"hooks": {"PreToolUse": [
                {"matcher": "Read", "hooks": [{"type": "command", "command": command, "timeout": 99}]},
                {"matcher": tool, "hooks": [{"type": "prompt", "command": command}]},
                {"matcher": tool, "hooks": []}
            ], "Stop": [{"command": "echo stop"}]}, "user": true});
            assert_eq!(
                present(&root),
                command == CLAUDE_HOOK_COMMAND,
                "inactive and prompt hooks do not count; claude presence is matcher-blind: {command}"
            );
            match command {
                CLAUDE_HOOK_COMMAND | CODEX_HOOK_COMMAND => {
                    insert_hook_entry(&mut root, PRE_TOOL_USE_KEY, CLAUDE_PRE_MATCHER, command)
                        .unwrap()
                }
                _ => insert_trae_hook_entry(&mut root).unwrap(),
            }
            assert!(present(&root));
            let groups = root["hooks"]["PreToolUse"].as_array_mut().unwrap();
            groups.last_mut().unwrap()["hooks"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({"type": "command", "command": "echo user"}));
            let remove = |root: &mut serde_json::Value| match command {
                CLAUDE_HOOK_COMMAND => remove_hook_from_json(root),
                CODEX_HOOK_COMMAND => remove_codex_hook_from_json(root),
                _ => remove_trae_hook_from_json(root),
            };
            assert!(remove(&mut root));
            assert!(!remove(&mut root));
            assert!(!present(&root));
            assert_eq!(
                root["hooks"]["PreToolUse"],
                serde_json::json!([
                    {"matcher": tool, "hooks": [{"type": "prompt", "command": command}]},
                    {"matcher": tool, "hooks": []},
                    {"matcher": tool, "hooks": [{"type": "command", "command": "echo user"}]}
                ])
            );
            assert_eq!(
                root["hooks"]["Stop"],
                serde_json::json!([{"command": "echo stop"}])
            );
            assert_eq!(root["user"], true);
        }
    }

    #[test]
    fn test_uninstall_removes_ptk_instructions_block() {
        let temp = TempDir::new().unwrap();
        let claude_md = temp.path().join("CLAUDE.md");

        fs::write(&claude_md, PTK_INSTRUCTIONS).unwrap();
        assert!(claude_md.exists());

        let content = fs::read_to_string(&claude_md).unwrap();
        assert!(content.contains(PTK_BLOCK_START));

        let (cleaned, did_remove) = remove_ptk_block(&content);
        assert!(did_remove);
        assert!(!cleaned.contains(PTK_BLOCK_START));
        assert!(!cleaned.contains("ptk cargo test"));
    }

    #[test]
    fn test_uninstall_preserves_non_ptk_content() {
        let content = format!(
            "# My Project\n\nSome custom instructions.\n\n{}\n\n## Other Notes\n\nKeep this.",
            PTK_INSTRUCTIONS
        );

        let (cleaned, did_remove) = remove_ptk_block(&content);

        assert!(did_remove);
        assert!(cleaned.contains("# My Project"));
        assert!(cleaned.contains("Some custom instructions."));
        assert!(cleaned.contains("## Other Notes"));
        assert!(cleaned.contains("Keep this."));
        assert!(!cleaned.contains(PTK_BLOCK_START));
    }

    #[test]
    fn test_uninstall_handles_both_artifacts() {
        let temp = TempDir::new().expect("create isolated home");
        with_claude_dir_override(&temp, |claude_dir| {
            let claude_md = claude_dir.join(CLAUDE_MD);
            let prose = "@PTK.md is where I keep my own token rules; do not delete.\n\
                         @PTK.md.backup is a different file.\n\
                         Read @PTK.md for details.";
            let content = format!(
                "# Config\n\n{prose}\n\n@PTK.md\n  @PTK.md \t\n\n{PTK_INSTRUCTIONS}\n\nMore stuff"
            );
            fs::write(&claude_md, content).expect("write instructions");

            for _ in 0..2 {
                uninstall(
                    true,
                    false,
                    false,
                    false,
                    false,
                    false,
                    InitContext::default(),
                )
                .expect("uninstall PTK");
                let cleaned = fs::read_to_string(&claude_md).expect("read retained instructions");
                assert_eq!(cleaned, format!("# Config\n\n{prose}\n\nMore stuff"));
            }
        });
    }

    #[test]
    fn test_uninstall_preserves_prose_without_exact_ptk_reference() {
        let temp = TempDir::new().expect("create isolated home");
        with_claude_dir_override(&temp, |claude_dir| {
            let claude_md = claude_dir.join(CLAUDE_MD);
            let content = "# My rules\r\n\r\n\r\n\
                           @PTK.md is where I keep my own rules.\r\n\
                           Read @PTK.md for details.\r\n";
            fs::write(&claude_md, content).expect("write instructions");

            uninstall(
                true,
                false,
                false,
                false,
                false,
                false,
                InitContext::default(),
            )
            .expect("uninstall PTK");

            assert_eq!(
                fs::read_to_string(&claude_md).expect("read retained instructions"),
                content
            );
        });
    }

    #[test]
    fn test_uninstall_dry_run_preserves_instruction_artifacts() {
        let temp = TempDir::new().expect("create isolated home");
        with_claude_dir_override(&temp, |claude_dir| {
            let claude_md = claude_dir.join(CLAUDE_MD);
            let content = format!("# My rules\n\n@PTK.md\n\n{PTK_INSTRUCTIONS}\n");
            fs::write(&claude_md, &content).expect("write instructions");

            uninstall(
                true,
                false,
                false,
                false,
                false,
                false,
                InitContext {
                    dry_run: true,
                    ..InitContext::default()
                },
            )
            .expect("preview uninstall");

            assert_eq!(
                fs::read_to_string(&claude_md).expect("read retained instructions"),
                content
            );
        });
    }

    #[test]
    fn test_uninstall_integration_preserves_user_content() {
        let user_content = "# My Project Rules\n\nAlways use snake_case.";
        let installed = format!("{}\n\n{}", user_content, PTK_INSTRUCTIONS);

        let (cleaned, did_remove) = remove_ptk_block(&installed);
        assert!(did_remove);
        assert!(!cleaned.trim().is_empty(), "user content should remain");
        assert!(
            cleaned.contains("My Project Rules"),
            "user content must be preserved"
        );
        assert!(
            cleaned.contains("snake_case"),
            "user content must be preserved"
        );
        assert!(
            !cleaned.contains(PTK_BLOCK_START),
            "PTK block must be fully removed"
        );
        assert!(
            !cleaned.contains(PTK_BLOCK_END),
            "PTK end marker must be removed"
        );
    }
    #[test]
    fn test_read_json_file_handles_missing_and_empty_files() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("missing.json");
        let empty = temp.path().join("empty.json");
        fs::write(&empty, "  \n").unwrap();

        assert!(read_json_file(&missing).unwrap().is_none());
        assert_eq!(read_json_file(&empty).unwrap(), Some(serde_json::json!({})));
    }

    #[cfg(feature = "engine")]
    #[test]
    fn test_read_json_file_errors_include_the_path() {
        let temp = TempDir::new().unwrap();
        let invalid = temp.path().join("invalid.json");
        fs::write(&invalid, "{").unwrap();

        let parse_error = read_json_file(&invalid).unwrap_err();
        assert!(format!("{parse_error:#}").contains(&invalid.display().to_string()));

        let read_error = read_json_file(temp.path()).unwrap_err();
        assert!(format!("{read_error:#}").contains(&temp.path().display().to_string()));
    }

    #[test]
    fn test_backup_and_atomic_write_preserves_previous_content() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("hooks.json");
        fs::write(&path, "old").unwrap();

        let backup = backup_and_atomic_write(&path, "new").unwrap().unwrap();

        assert_eq!(backup, path.with_extension("json.bak"));
        assert_eq!(fs::read_to_string(path).unwrap(), "new");
        assert_eq!(fs::read_to_string(backup).unwrap(), "old");
    }

    #[test]
    fn test_backup_and_atomic_write_failure_preserves_original() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("hooks.json");
        fs::write(&path, "old").unwrap();
        fs::create_dir(path.with_extension("json.bak")).unwrap();

        let err = backup_and_atomic_write(&path, "new").unwrap_err();

        assert!(format!("{err:#}").contains("backup"));
        assert_eq!(fs::read_to_string(path).unwrap(), "old");
    }
}
