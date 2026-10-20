# Contributing to ptk

**Welcome!** We appreciate your interest in contributing to ptk.

## Quick Links

- [Report an Issue](../../issues/new)
- [Open Pull Requests](../../pulls)
- [Start a Discussion](../../discussions)
- [Technical Documentation](docs/contributing/TECHNICAL.md) — Architecture, end-to-end flow, folder map, how to write tests

---

## What is ptk?

**ptk (Rust Token Killer)** is a coding agent proxy that cuts noise from command outputs. It filters and compresses CLI output before it reaches your LLM context, so the agent reads less bash output on common operations.

Every percentage in this repo measures **bash output**, not your bill: those bytes are one contributor to input tokens, and input tokens are only part of a cost that also counts output tokens. See [How PTK Savings Work](docs/guide/resources/savings-explained.md) before quoting any figure.

---

## Ways to Contribute

| Type | Examples |
|------|----------|
| **Report** | File a clear issue with steps to reproduce, expected vs actual behavior |
| **Fix** | Bug fixes, broken filter repairs |
| **Build** | New filters, new command support, new features (for core features, discuss with maintainers before) |
| **Review** | Review open PRs, test changes locally, leave constructive feedback |
| **Document** | Improve docs, clarify |
| **Extend** | Write your own pipeline method as a Rust plugin — no core change needed. See the [Plugins guide](docs/guide/pipelines/plugins.md) (`ptk plugins new/build`) |
---

## Design Philosophy

Four principles guide every PTK design decision. Understanding them helps you write contributions that fit naturally into the project.

### Correctness VS Token Savings

When a user or LLM explicitly requests detailed output via flags (e.g., `git log --comments`, `cargo test -- --nocapture`, `ls -la`), respect that intent. Compressing explicitly-requested detail defeats the purpose — the LLM asked for it because it needs it.

Filters should be flag-aware: default output (no flags) gets aggressively compressed, but verbose/detailed flags should pass through more content. When in doubt, preserve correctness.

> Example: `ptk cargo test` shows failures only. But `ptk cargo test -- --nocapture` preserves all output because the user explicitly asked for it.

### Transparency

The LLM doesn't know PTK is involved for which commands, hooks rewrite commands silently. PTK's output must be a valid, useful subset of the original tool's output, not a different format the LLM wouldn't expect. If an LLM parses `git diff` output, PTK's filtered version must still look like `git diff` output.

Don't invent new output formats. Don't add PTK-specific headers or markers in the default output. The filtered output should be indistinguishable from "a shorter version of the real command."

Enforce it with `guard::never_worse(raw, filtered)` — print and track the value it returns (use `runner::emit_guarded(filtered, hint, raw)` when appending a tee hint). It guarantees PTK never emits more tokens than the raw command, down to emitting nothing when the command produced nothing.

### Never Block

If a filter fails, fall back to raw output. PTK should never prevent a command from executing or producing output. Better to pass through unfiltered than to error out. Same for hooks: exit 0 on all error paths so the agent's command runs unmodified.

Every filter needs a fallback path. Every hook must handle malformed input gracefully. Truncation follows the same rule: capping output at N items is only acceptable if accompanied by a hint that lets the agent recover the hidden data.

### Zero Overhead

<10ms startup. No async runtime. No config file I/O on the critical path. If developers perceive any delay, they'll disable PTK. Speed is the difference between adoption and abandonment.

Use `LazyLock` statics for all regex. No network calls. No disk reads in the hot path. Benchmark before/after with `hyperfine`.

### Extensibility

Always use components already in place to avoid duplication, also use extensible modules when this is possible.
If you want to submit a new core feature, this is an important point to watch.

---

## What Belongs in PTK?

### In Scope

Commands that produce **text output** (typically 100+ tokens) whose bytes can be compressed **20%+** without losing essential information for the LLM. See [Correctness VS Token Savings](#correctness-vs-token-savings).

- Test runners (vitest, pytest, cargo test, go test)
- Linters and type checkers (eslint, ruff, tsc, mypy)
- Build tools (cargo build, dotnet build, make, next build)
- VCS operations (git status/log/diff, gh pr/issue)
- Package managers (pnpm, pip, cargo install, brew)
- File operations (ls, tree, grep, find, cat/head/tail)
- Infrastructure tools with text output (docker, kubectl, terraform)

When implementing a new filter/cmds, be aware of the [Design Philosophy](#design-philosophy) above.

### Out of Scope

- Interactive TUIs (htop, vim, less): not batch-mode compatible
- Binary output (images, compiled artifacts): no text to filter
- Trivial commands: not worth the overhead and may loose important informations
- Commands with no text output: nothing to compress
- Others features not related to a LLM-proxy like PTK

### TOML vs Rust: Which One?

| Use **TOML filter** when | Use **Rust module** when |
|--------------------------|--------------------------|
| Output is plain text with predictable line structure | Output is structured (JSON, NDJSON) |
| Regex line filtering cuts 60%+ of the output bytes | Needs state machine parsing (e.g., pytest phases) |
| No need to inject CLI flags | Needs to inject flags like `--format json` |
| No cross-command routing | Routes to other commands (lint → ruff/mypy) |
| Examples: brew, df, shellcheck, rsync, ping | Examples: vitest, pytest, golangci-lint, gh |

See [`src/filters/README.md`](src/filters/README.md) for TOML filter guidance and [`src/cmds/README.md`](src/cmds/README.md) for Rust module guidance.

### Adding a Filter

For the step-by-step checklist (create filter, register rewrite pattern, register in main.rs, write tests, update docs), see [src/cmds/README.md — Adding a New Command Filter](src/cmds/README.md#adding-a-new-command-filter).

---

## Commit Messages & Changelog

PTK uses [Conventional Commits](https://www.conventionalcommits.org/) to **drive version numbers and release notes automatically**. A push to `main` is the release: `scripts/next-version.sh` reads your commit messages, the release job bumps every `Cargo.toml`, prepends a section to `CHANGELOG.md`, tags `vX.Y.Z`, and publishes the GitHub Release. There is no release pull request to approve.

Never edit `CHANGELOG.md` manually — it is written by the release job from your commit messages.

### Commit format

```
<type>(<scope>): <short description>
```

| Type | Semver Impact | When to Use |
|------|---------------|-------------|
| `feat` | Minor | New features, new filters, new command support |
| `fix` | Patch | Bug fixes, corrections |
| `perf` | Patch | Performance improvements |
| `refactor` | — | Code restructuring (no changelog entry) |
| `docs` | — | Documentation only |
| `chore` | — | Maintenance, CI, deps |
| `feat!` / `fix!` | Major | Breaking changes (add `!` after type) |

**Scope** should match the module or area: `git`, `cargo`, `gh`, `hook`, `tracking`, `cicd`, etc.

### Examples

```
feat(kubectl): add pod log filtering
fix(git): preserve merge commit messages in log filter
perf(cargo): lazy-compile clippy regex patterns
feat!(hook): change rewrite config format
```

These commit messages become CHANGELOG entries and decide the next version, so write them as if they will be read by users:

| Commit | Release effect |
|---|---|
| `fix(scope): …`, `perf(scope): …` | patch bump (`1.2.3` → `1.2.4`) |
| `feat(scope): …` | minor bump (`1.2.3` → `1.3.0`) |
| `feat(scope)!: …` or a `BREAKING CHANGE:` footer | major bump (`1.2.3` → `2.0.0`), or minor below `1.0` |
| `docs:`, `ci:`, `chore:`, `test:`, `build:`, `style:` | no release — most pushes fall here |

---

## Branch Naming Convention

Git branch names cannot include spaces or colons, so we use slash-prefixed names. Pick the prefix that matches your change type and follow it with an optional scope and a short, kebab-case description.

| Prefix | When to Use |
|--------|-------------|
| `fix/` | Bug fixes, corrections, minor adjustments |
| `feat/` | New features, new filters, new command support |
| `chore/` | CI/CD, deps, maintenance, breaking changes |

Combine the prefix with a scope if it adds clarity (e.g. `git`, `kubectl`, `filter`, `tracking`, `config`) and finish with a descriptive slug: `fix/<scope>-<description>` or `feat/<description>`.

Examples:
```
fix/git-log-filter-drops-merge-commits
feat/kubectl-add-pod-list-filter
chore/release-pipeline-cleanup
```

---

## Pull Request Process

### Scope Rules

**Each PR must focus on a single feature, fix, or change.** The diff must stay in-scope with the description written by the author in the PR title and body. Out-of-scope changes (unrelated refactors, drive-by fixes, formatting of untouched files) must go in a separate PR.

**For large features or refactors**, prefer multi-part PRs over one enormous PR. Split the work into logical, reviewable chunks that can each be merged independently. Examples:
- feat(Part 1): Add data model and tests
- feat(Part 2): Add CLI command and integration
- feat(Part 3): Update documentation

**Why**: Small, focused PRs are easier to review, safer to merge, and faster to ship. Large PRs slow down review, hide bugs, and increase merge conflict risk.


### 1. Create Your Branch

```bash
git checkout main
git pull origin main
git checkout -b feat/scope-your-clear-description
```

Opt in once per clone so `git blame` skips the bulk-formatting commits listed in
`.git-blame-ignore-revs` and reports whoever actually wrote each line:

```bash
git config blame.ignoreRevsFile .git-blame-ignore-revs
```

### 2. Make Your Changes

**Respect the existing folder structure.** Place new files where similar files already live. Do not reorganize without prior discussion.

**Keep functions short and focused.** Each function should do one thing. If it needs a comment to explain what it does, it's probably too long -- split it.

**No obvious comments.** Don't comment what the code already says. Comments should explain *why*, never *what* to avoid noise.

**Large command files are expected.** Command modules (`*_cmd.rs`) contain the implementation, tests, and fixture in the same file. A big file is fine when it's self-contained for one command.

### 3. Add Tests

Every change **must** include tests. See [Testing](#testing) below.

### 4. Add Documentation

Documentation updates are required for new filters, new features, and changes that affect already-documented behavior. Bug fixes and refactors typically don't need doc updates. See [Documentation](#documentation) below.

### Contributor License Agreement (CLA)

All contributions require signing our **Contributor License Agreement (CLA)** before being merged.

By signing, you certify that:
- You have authored 100% of the contribution, or have the necessary rights to submit it.
- You grant **PTK AI Labs** a perpetual, worldwide, royalty-free license to use your contribution — including in commercial products such as **ptk Pro**. A contribution to the `ptk` CLI is licensed under the [Apache License 2.0](LICENSE); a contribution to `engine/` or `proxy/` is licensed under [BUSL-1.1](engine/LICENSE), as those crates are (see [`engine/CONTRIBUTING.md`](engine/CONTRIBUTING.md)).
- If your employer has rights over your work, you have obtained their permission.

**This is automatic.** When you open a Pull Request, [CLA Assistant](https://cla-assistant.io) will post a comment asking you to sign. Click the link in that comment to sign with your GitHub account. You only need to sign once.

### 5. Open a Pull Request into `main`

Once your work is ready, open a Pull Request targeting **`main`**. It is the trunk and the
only integration branch — there is no staging branch to promote through.

### 6. Review Process

1. **Maintainer review** -- A maintainer reviews your code for quality and alignment with the project
2. **CI/CD checks** -- Automated tests and linting must pass
3. **Resolution** -- Address any feedback from review or CI failures

#### What CI runs, and what it skips

Every job is its own checkout and compile, so the workflow classifies your diff once and
only pays for the jobs your change can affect. The **changed paths** job decides:

| Flag | Set by |
|------|--------|
| `code` | `src/`, `engine/`, `proxy/`, `tests/`, `benches/`, `build.rs`, `rust-toolchain*`, `.cargo/`, `Cargo.toml`, `Cargo.lock`, `scripts/`, `.github/workflows/` |
| `windows` | `src/cmds/windows/`, `src/core/windows_shell`, `scripts/benchmark.sh`, `.github/workflows/ci.yml` |

`code` gates `test-windows`, `proxy`, `oss edition`, and `benchmark`; `windows` gates
`benchmark (windows)`, which is the single longest job in the workflow and exercises the 14
pwsh and cmd.exe rows that nothing else can move. The `test` matrix — ubuntu and macOS — is
ungated, because macOS is the only leg that exercises the platform socket and path code and
it is not worth making contributors guess when it runs.

A documentation-only pull request therefore skips every gated job — measured at 4.9 minutes
wall clock, 11 checks run and 5 skipped — leaving only the ungated `test` matrix. Note that
`tests/` counts as `code`: editing a test must never be a way to skip running it.

#### Workflow files are linted

A workflow that does not parse fails anonymously: the run is named after the file and
reports "This run likely failed because of a workflow file issue", with no line and no log.
A job-level `if` may only read `github`, `needs`, `inputs` and `vars` — `matrix` is not in
that list, and `if: matrix.os != 'windows-latest'` is rejected at parse time even though
YAML and `cargo test` accept it.

The `workflow lint` job runs [actionlint](https://github.com/rhysd/actionlint) on every
workflow. Run it before pushing:

```bash
actionlint -shellcheck='shellcheck -S error' .github/workflows/*.yml
```

shellcheck is held to error severity because this repository's existing run blocks carry
about a hundred pre-existing SC2086/SC2129 info findings.

### 7. Integration & Release

Once merged, your change is on `main` and is released by that merge. `main` is the
release branch, and there is no release pull request: `cd.yml` computes the version from
your commits, tests the exact commit, writes it into the manifests and `CHANGELOG.md`,
tags it, and publishes the GitHub Release.

```
your branch --> main (review + CI) --> tag vX.Y.Z --> GitHub Release
```

If your commits are docs, CI, or chores only, the push lands and no release is cut — the
release job exits cleanly rather than inventing a version.

---

## Testing

Every change **must** include tests. We follow **TDD (Red-Green-Refactor)**: write a failing test first, implement the minimum to pass, then refactor.

For how to write tests (fixtures, snapshots, token savings verification), see [docs/contributing/TECHNICAL.md — Testing](docs/contributing/TECHNICAL.md#8-testing).

### Test Types

| Type | Where | Run With |
|------|-------|----------|
| **Unit tests** | `#[cfg(test)] mod tests` in each module | `cargo test` |
| **Snapshot tests** | `#[cfg(test)]` create snapshots for filters modules | `cargo test` |
| **Smoke tests** | `scripts/test-all.sh` | `bash scripts/test-all.sh` |
| **Integration tests** | `#[ignore]` tests requiring installed binary | `cargo test --ignored` |

### Pre-Commit Gate (mandatory)

All three must pass before any PR:

```bash
cargo fmt --all --check && cargo clippy --all-targets && cargo test
```

### PR Testing Checklist

- [ ] Unit tests added/updated for changed code
- [ ] Snapshot tests for filters
- [ ] >=20% reduction in bash output verified (measured with PTK's token estimator, not a real tokenizer)
- [ ] Any truncated list has a recovery hint (`force_tee_tail_hint` or `force_tee_hint`) and uses a `CAP_*` from `src/core/truncate.rs`
- [ ] Edge cases covered
- [ ] `cargo fmt --all --check && cargo clippy --all-targets && cargo test` passes
- [ ] Manual test: run `ptk <cmd>` and inspect output

---

## Documentation

Documentation updates are required for new filters, new features, and changes that affect already-documented behavior. Use this table to find which docs to update:

| What you changed | Update these docs |
|------------------|-------------------|
| New Rust filter (`src/cmds/`) | Ecosystem `README.md` (e.g., `src/cmds/git/README.md`), [README.md](README.md) command list |
| New TOML filter (`src/filters/`) | [src/filters/README.md](src/filters/README.md) if naming conventions change, [README.md](README.md) command list |
| New rewrite pattern | `src/discover/rules.rs` — see [Adding a New Command Filter](src/cmds/README.md#adding-a-new-command-filter) |
| Core infrastructure (`src/core/`) | [src/core/README.md](src/core/README.md), [docs/contributing/TECHNICAL.md](docs/contributing/TECHNICAL.md) if flow changes |
| Hook system (`src/hooks/`) | [src/hooks/README.md](src/hooks/README.md), [hooks/README.md](hooks/README.md) for agent-facing docs |
| Architecture or design change | [ARCHITECTURE.md](docs/contributing/ARCHITECTURE.md), [docs/contributing/TECHNICAL.md](docs/contributing/TECHNICAL.md) |

> **Note**: Do NOT edit `CHANGELOG.md` manually — the release job writes it from your commit messages on every push to `main`. See [Commit Messages & Changelog](#commit-messages--changelog).

**Navigation**: [CONTRIBUTING.md](CONTRIBUTING.md) (you are here) → [docs/contributing/TECHNICAL.md](docs/contributing/TECHNICAL.md) (architecture + flow) → each folder's `README.md` (implementation details).

Keep documentation concise and practical -- examples over explanations.

---

## Questions?

- **Bug reports & features**: [Issues](../../issues)
- **Discussions**: [GitHub Discussions](../../discussions)

**For external contributors**: Your PR will undergo automated security review (see [SECURITY.md](SECURITY.md)). 
This protects PTK's shell execution capabilities against injection attacks and supply chain vulnerabilities.

---

**Thank you for contributing to ptk!**
