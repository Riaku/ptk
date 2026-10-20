# Copilot Instructions for ptk

**ptk (Rust Token Killer)** is a CLI proxy that filters and compresses command outputs before they reach an LLM context. It wraps common tools (`git`, `cargo`, `grep`, `pnpm`, `go`, etc.) and outputs condensed summaries instead of raw output. Percentages measure bash output, not billed tokens; PTK ships no tokenizer (`src/core/tracking.rs` estimates `bytes / 4`).

## Using ptk in this session

**Always prefix commands with `ptk` when running shell commands** — this reduces token consumption for every operation you perform.

```bash
# Instead of:              Use:
git status                 ptk git status
git log -10                ptk git log -10
cargo test                 ptk cargo test
cargo clippy --all-targets ptk cargo clippy --all-targets
grep -r "pattern" src/     ptk grep -r "pattern" src/
```

**ptk meta-commands** (always use these directly, no prefix needed):
```bash
ptk gain              # Show token savings analytics
ptk gain --history    # Full command history with per-command savings
ptk discover          # Scan session history for missed ptk opportunities
ptk bypass <cmd>       # Run a command raw (no filtering) but still track it
```

**Verify ptk is installed before starting:**
```bash
ptk --version   # Should print: ptk X.Y.Z
ptk gain        # Should show a dashboard (not "command not found")
```

> Name collision: upstream Rust Token Killer (`rtk-ai/rtk`) and the unrelated Rust Type Kit
> (`reachingforthejack/rtk`) both install a binary called `rtk`. This fork installs `ptk`, so
> it is not affected -- a failing `ptk gain` means the `ptk` on PATH is a stale build. Run
> `which -a ptk` and reinstall from `riaku/ptk`.

## Build, Test & Lint

```bash
cargo build                    # Development build
cargo test                     # All tests
cargo test test_name           # Single test
cargo test module::tests::     # Module tests
cargo test -- --nocapture      # With stdout

# Pre-commit gate (must all pass before any PR)
cargo fmt --all --check && cargo clippy --all-targets && cargo test

bash scripts/test-all.sh       # Smoke tests (requires installed binary)
```

PRs target the **`main`** branch — the fork's trunk. There is no `develop` or `master` branch here. All commits require a DCO sign-off (`git commit -s`).

## Architecture

ptk routes CLI commands via a Clap `Commands` enum in `main.rs` to specialized filter modules in `src/cmds/*/`, each executing the underlying command and compressing output. Token savings are tracked in SQLite via `src/core/tracking.rs`.

For full details see [ARCHITECTURE.md](../docs/contributing/ARCHITECTURE.md) and [docs/contributing/TECHNICAL.md](../docs/contributing/TECHNICAL.md). Module responsibilities are documented in each folder's `README.md` and each file's `//!` doc header.

## Key Conventions

- **Error handling**: `anyhow::Result` with `.context("description")?` — no bare `?`, no `unwrap()` in production. Filters must fall back to raw command on error.
- **Regex**: Use `LazyLock<Regex>` for fixed patterns reused across calls; keep runtime-dependent patterns local.
- **Testing**: Unit tests inside modules (`#[cfg(test)] mod tests`). Fixtures in `tests/fixtures/`. Token savings assertions with `count_tokens()`.
- **Exit codes**: Preserve the underlying command's exit code via `std::process::exit(code)`.
- **Performance**: Startup <10ms (no async runtime), binary <5MB stripped.
- **Branch naming**: `fix(scope):`, `feat(scope):`, `chore(scope):` where scope is the affected component.

For the full contribution workflow, design philosophy, and new-filter checklist, see [CONTRIBUTING.md](../CONTRIBUTING.md).

<!-- ptk-instructions v2 -->
# PTK — Token-Optimized CLI

**ptk** is a CLI proxy that filters and compresses command outputs, cutting 60-90% of bash output.

## Rule

Always prefix shell commands with `ptk`:

```bash
# Instead of:              Use:
git status                 ptk git status
git log -10                ptk git log -10
cargo test                 ptk cargo test
docker ps                  ptk docker ps
kubectl get pods           ptk kubectl get pods
```

## Meta commands (use directly)

```bash
ptk gain              # Token savings dashboard
ptk gain --history    # Per-command savings history
ptk discover          # Find missed ptk opportunities
ptk bypass <cmd>      # Run raw (no filtering) but track usage
```
<!-- /ptk-instructions -->