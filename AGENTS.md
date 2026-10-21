# AGENTS.md

The key words `MUST`, `MUST NOT`, `SHOULD`, `SHOULD NOT`, and `MAY` in this document are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) and [RFC 8174](https://www.rfc-editor.org/rfc/rfc8174). Only the uppercase forms are normative, and every requirement in this file is stated with one of them; prose without a keyword carries no obligation and exists to explain a rule or to describe behavior.

You MUST read this file before you change code.

## What this project is

**ptk (Rust Token Killer)** is a CLI proxy that reduces LLM token consumption by filtering and compressing command output. It runs the command you hand it and prints a reduced form of what that command wrote.

This repository is the [Riaku](https://github.com/riaku/ptk) fork of [rtk-ai/rtk](https://github.com/rtk-ai/rtk). It keeps the upstream CLI and adds native Windows support: CMD built-ins, PowerShell cmdlets, and console code-page decoding. That code lives in `src/cmds/windows/` (one module per cmdlet) and `src/core/windows_shell.rs` (spawning, argument splitting, cmdlet detection). The Windows entry points are reachable only from the `#[cfg(windows)]` branch of `run_fallback` and from their own subcommands, which is why both module trees carry `#![cfg_attr(not(windows), allow(dead_code))]`.

Two facts keep the numbers honest:

- Every percentage in this repository measures **output bytes**, not a bill.
- ptk ships no tokenizer. `src/core/tracking.rs` estimates tokens as `bytes / 4`, so the ratios are reliable and the absolute token counts are approximate. You MUST report token counts as approximate and percentages as measured output bytes.

## Confirm you have the right binary

Two unrelated tools both install a binary named `rtk`: upstream Rust Token Killer (`rtk-ai/rtk`) and the Rust Type Kit (`reachingforthejack/rtk`). Because `rtk gain` can resolve to the wrong one, this fork installs `ptk` instead.

```bash
ptk --version   # any version; the next line is the real check
ptk gain        # expect token-savings statistics, not "command not found"
```

If `ptk` is missing entirely, you have one of the other two installed. If `ptk` exists but `gain` is unknown, the build is stale.

## Branch and release model

- `main` is the only trunk, and every pull request MUST target `main`. `.github/workflows/pr-target-check.yml` labels and comments on any pull request that targets a different branch.
- `develop` is unused, and the requirement to target `main` covers it.
- Merges are **squash merges**. A pull request lands as one commit on `main`, and that commit's subject is the pull request title.
- A push to `main` starts `.github/workflows/cd.yml`, which tags a release. There is no release branch and no release pull request.

`cd.yml` derives the next version from the conventional-commit subjects since the last tag, using `scripts/next-version.sh`:

| Commit contains | Version bump |
|---|---|
| `BREAKING CHANGE` in the body | Major, or minor while the version is below 1.0 |
| `feat:` | Minor |
| `fix:` or `perf:` | Patch |
| Anything else (`docs:`, `ci:`, `chore:`, `test:`, `build:`, `style:`) | No release |

The pull request title therefore decides the release. Because the merge is a squash, that title becomes the commit subject the script reads, so you MUST write the title as a conventional commit.

After computing the version, `cd.yml` writes it to five manifests (`Cargo.toml`, `engine/Cargo.toml`, `plugin/Cargo.toml`, `text/Cargo.toml`, and `proxy/Cargo.toml`) and to `CHANGELOG.md`, runs the tests, tags `vX.Y.Z`, and calls `release.yml`. To build a pre-release instead, dispatch `cd.yml` by hand from a branch other than `main`; it produces a `dev-*-rc.*` tag from the same script.

`.github/workflows/release.yml` checks out the tag and builds every artifact: the Linux, macOS, and Windows archives, the `ptk-oss` archives, the separately downloaded `ptk-proxy` archives, and `checksums.txt`, which lists the file names `install.sh` downloads. There is no crates.io, Homebrew, winget, DEB, or RPM packaging in this fork, so those jobs and their metadata are absent by design.

## Before you commit

You MUST run all five commands before every commit, and you MUST NOT commit while any of them exits nonzero.

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
cargo check --target x86_64-pc-windows-gnu --workspace
cargo fmt --manifest-path proxy/Cargo.toml --all
```

CI runs the same five in separate jobs and adds checks this gate does not cover, so a green pull request does not excuse skipping the gate.

- `cargo test --workspace` covers the four workspace members: `ptk`, `text`, `engine`, and `plugin`. It does not cover `proxy/`.
- The Windows cross-check type-checks code that is dead on Linux, which is the only way `#[cfg(windows)]` errors surface before CI. A `cfg` split can break either side, which is why the gate has no Windows-only exemption.
- `proxy/` is its own workspace root, so root `fmt`, `clippy`, and `test` all skip it. That is why one of the five commands passes `--manifest-path proxy/Cargo.toml`.

CI runs `cargo clippy` without `-D warnings`, so a clippy warning does not fail the build. Fix warnings anyway, because the ones you leave behind become the next person's noise.

You MUST NOT merge a pull request you opened unless the user explicitly asks you to in this conversation. Open it, get CI green, and hand it over.

CI reports these checks, and a pull request is ready when none of them reports `failure`:

- `changed paths`
- `fmt`
- `clippy`
- `rustdoc`
- `proxy (lint + test)`
- `oss edition (build + purity)`
- `test (ubuntu-latest)`
- `test (macos-latest)`
- `test (windows-latest)`
- `test (windows-latest, x86_64-pc-windows-gnu)`
- `Security Scan`
- `semgrep security scan`
- `workflow lint`
- `test presence`
- `doc review`
- `benchmark`
- `benchmark (windows)`

`test presence` fails when a filter module ships without tests. `oss edition` builds the Apache-2.0 edition and fails if a BUSL crate enters its dependency graph. `changed paths` skips the build, test, and benchmark jobs when the diff cannot affect them, which is what a docs-only pull request looks like, so a job that reports `skipping` is satisfied rather than failed.

Any edit to a Rust file means you MUST rerun the five commands in [Before you commit](#before-you-commit).

## Workspace layout

| Path | Contents |
|---|---|
| `.` | The `ptk` CLI (Apache-2.0) |
| `engine/` | Filtering engine the CLI depends on |
| `text/` | Text-processing crate |
| `plugin/` | Plugin crate |
| `proxy/` | The `ptk-proxy` daemon (BUSL-1.1), its own workspace root |
| `hooks/` | Per-agent hook integrations |
| `src/cmds/` | One module per command family |
| `src/filters/` | TOML filters for commands that need no Rust code |

`proxy/` is deliberately neither a member of the `ptk` workspace nor a dependency of the `ptk` crate. The CLI locates and spawns the `ptk-proxy` binary and reads its SQLite usage database for `ptk gain`. Keeping the daemon separate holds `tokio` and `axum` out of a CLI with a 10 ms startup budget, and it keeps the two licenses in separate artifacts.

Both lockfiles pin `rusqlite 0.34` by hand to match what `icm-store` links, because `libsqlite3-sys` declares `links = "sqlite3"` and two versions of it cannot coexist in one dependency graph. You MUST upgrade both lockfiles together.

Build and test the daemon from its own manifest:

```bash
cargo build --release
cargo build --release --manifest-path proxy/Cargo.toml
cargo test --workspace
cargo test --manifest-path proxy/Cargo.toml
cargo clippy --workspace --all-targets
cargo clippy --manifest-path proxy/Cargo.toml --all-targets
```

## Architecture

`src/lib.rs` defines a Clap `Commands` enum and routes each subcommand to a filter module under `src/cmds/*/`. A module runs the underlying command and compresses its output. `src/core/tracking.rs` records what each call saved in SQLite.

`ptk` passes through any command it has no filter for, so prefixing a command with `ptk` is always safe.

Read these files before working in their area:

- `docs/contributing/ARCHITECTURE.md` for system design, module organization, filtering strategies, and error handling
- `docs/contributing/TECHNICAL.md` for the end-to-end flow, folder map, hook system, and filter pipeline
- `docs/contributing/CODING_PRACTICES.md` for file and function size, comments, imports, and security
- `src/cmds/README.md#adding-a-new-command-filter` for the step-by-step checklist when adding a filter
- `CONTRIBUTING.md` for the contribution workflow, the 20% floor, and the design philosophy

Each folder under `src/cmds/` has a `README.md`, and each module has a `//!` header, so browse the tree to discover what is already implemented.

Supported ecosystems, each confirmed against `ptk --help`:

- `git`, `gh`, `glab`, and Graphite (`gt`)
- `cargo`
- `go` and `golangci-lint`
- `npm`, `pnpm`, and `npx`
- `deno`
- `ruff`, `pytest`, `pip`, and `mypy`
- `rspec`, `rubocop`, and `rake`
- `dotnet`
- `gradlew`, `mvn`, `mvnd`, and `sbt`
- `playwright`, `vitest`, and `jest`
- `docker`, `kubectl`, `aws`, and `psql`
- `php`, including Artisan, PHPUnit, Pest, PHPStan, and Pint

## Coding rules

All six rules below are mandatory, and each states its requirement with a keyword. They override general Rust convention. The full set, with examples and counter-examples, is in `.claude/rules/rust-patterns.md`; you MUST read that file before you write Rust here.

1. You MUST NOT add `async`, and the workspace MUST stay free of `tokio`, `async-std`, and `futures`. The CLI is single-threaded by design, and an async runtime adds 5-10 ms to startup.
2. You MUST NOT call `unwrap()` in production code. Use `.context("description")?` there, and `expect("reason")` in tests.
3. A fixed pattern reused across calls MUST live in a `static LazyLock<Regex>`, and a runtime-dependent pattern MUST stay local. The `unwrap()` inside that initializer is the only place `unwrap()` is allowed, because a bad literal is a programming error caught at first use.
4. Every filter MUST print the child's unfiltered output when filtering fails for any reason. Never block the user.
5. Every filter MUST exit with the child's exit code through `std::process::exit(code)`.
6. Every new command, flag, or argument-handling fix MUST classify arguments through `tokenize` or `tokenize_grammar`. Never scan for `starts_with('-')` and never compare against `arg == "--flag"`; `src/core/README.md` documents the four rules that go with the tokenizer.

Testing strategy and performance targets are in `.claude/rules/cli-testing.md`; you MUST read it before you write tests here. The targets it sets, with what the artifact measures:

| Metric | Target | Measured |
|---|---|---|
| Startup | Under 10 ms | About 2 ms for `ptk --version` |
| Memory | Under 5 MB | Not routinely measured |
| Binary size | Under 5 MB | About 16 MB |

The binary-size row is a goal rather than a passing check. A release build measures about 16 MB, roughly three times the figure in `cli-testing.md`, so do not report the target as though the artifact met it, and do not treat that gap alone as a regression.

Measure startup with `hyperfine 'ptk git log -10' --warmup 3` against `target/release/ptk`. Each filter MUST cut at least 20% of the output bytes, which is the floor `CONTRIBUTING.md` sets for every filter. Test each filter with its own fixtures instead of a synthetic input, and never copy a percentage from another filter or from a doc table, because the asserted threshold differs from filter to filter.

## Running ptk

Prefix a command with `ptk` to get filtered output. Commands ptk does not implement pass through unchanged.

```bash
ptk cargo test          # filtered test output
ptk git log -10         # compact log
ptk bypass git log      # run unfiltered, still record statistics
ptk recall <hash>       # recover output a filter elided, by content hash
ptk gain --history      # recent calls and what they saved
```

`ptk bypass <cmd>` runs one local command without filtering and still records it, so it appears in `ptk gain --history` as 0% reduction. Use it to work around a filter bug or to get complete output. `ptk proxy` is a deprecated alias for it, kept so agent instructions written before the rename keep working; it behaves identically and prints no notice.

`ptk llm-proxy` is a different command. It manages the `ptk-proxy` daemon in `proxy/`, which intercepts an agent's provider API traffic and strips bytes no reader sees, such as ANSI escapes, progress redraws, and duplicate lines.

## This file and ptk init

`ptk init --codex` writes its awareness text to `PTK.md` and adds a single `@PTK.md` reference line to `AGENTS.md`. It does not rewrite the rest of this file. You MUST leave that reference line in place and edit only the guidance around it, because removing the line stops a reader from picking up the injected awareness text.

An inline `<!-- ptk-instructions -->` block is legacy. When `ptk init` finds one, it migrates the block to the `@PTK.md` reference and removes it, so do not add one.

Some `ptk init` variants write `./CLAUDE.md`, or `~/.claude/CLAUDE.md` with `-g`. Those paths are product output for one agent, not repository guidance, so a `CLAUDE.md` reference under `docs/` or in `INSTALL.md` describes what ptk writes for a user's project.

## Working habits

- Before your first edit, you SHOULD run `git status --short --branch` and confirm the repository and branch match what you expect.
- You SHOULD verify with existing tests and fixtures rather than researching an external command's internals, and you MUST ask the user before running more than three exploratory commands beyond the first failing reproduction.
- You SHOULD commit each self-contained step separately, because you can then find and revert a broken change quickly.
- When you are handed a numbered plan, you MUST follow its order and commit after each phase. If a phase cannot be completed, report it to the user as blocked instead of skipping it.
