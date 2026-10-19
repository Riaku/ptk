# PTK - Complete Feature Reference

> **ptk (Rust Token Killer)** -- High-performance CLI proxy that cuts up to 90% of the bash output your agent reads.

Single Rust binary, zero external dependencies, < 10ms overhead per command.

## About bash output reduction

The savings PTK reports (for example in `ptk gain`) measure **bash output bytes removed**: the output a shell command returns before the agent reads it. It is not equivalent to a bill reduction of the same size.

Bash output is only one source of input tokens, alongside your prompt, the system prompt and the conversation history. Input tokens are themselves only part of the bill, which also counts output tokens. The reduction dilutes at each step.

The token counts PTK displays are estimated as `bytes / 4`: PTK ships no tokenizer, so **the percentages are reliable but absolute token counts are approximate**.

Full explanation: [How PTK Savings Work](../guide/resources/savings-explained.md)

---

## Table of contents

1. [Overview](#overview)
2. [Global flags](#global-flags)
3. [File commands](#file-commands)
4. [Git commands](#git-commands)
5. [GitHub CLI commands](#github-cli-commands)
6. [Test commands](#test-commands)
7. [Build and lint commands](#build-and-lint-commands)
8. [Formatting commands](#formatting-commands)
9. [Package managers](#package-managers)
10. [Containers and orchestration](#containers-and-orchestration)
11. [Data and network](#data-and-network)
12. [Cloud and databases](#cloud-and-databases)
13. [Stacked PRs (Graphite)](#stacked-prs-graphite)
14. [Analytics and tracking](#analytics-and-tracking)
15. [Hook system](#hook-system)
16. [Configuration](#configuration)
17. [Recall system (output recovery)](#recall-system)
18. [Telemetry](#telemetry)

---

## Overview

ptk acts as a proxy between an LLM (Claude Code, Gemini CLI, etc.) and system commands. Four filtering strategies are applied depending on the command type:

| Strategy | Description | Example |
|----------|-------------|---------|
| **Smart filtering** | Removes noise (comments, whitespace, boilerplate) | `ls -la` -> compact tree |
| **Grouping** | Aggregates by directory, error type, rule | Tests grouped by file |
| **Truncation** | Keeps relevant context, removes redundancy | Condensed diff |
| **Deduplication** | Merges repeated log lines with counters | `error x42` |

### Fallback mechanism

If ptk does not recognize a subcommand, it runs the raw command (passthrough) and records the event in the tracking database. This makes ptk **always safe** to use -- no command is ever blocked.

---

## Global flags

These flags apply to **all** subcommands:

| Flag | Short | Description |
|------|-------|-------------|
| `--verbose` | `-v` | Increase verbosity (-v, -vv, -vvv). Shows filtering details. |
| `--skip-env` | -- | Sets `SKIP_ENV_VALIDATION=1` for child processes (Next.js, tsc, lint, prisma). |

**Examples:**

```bash
ptk -v git status          # Compact status + filtering details on stderr
ptk -vvv cargo test        # Maximum verbosity (debug)
ptk --skip-env next build  # Disable Next.js env validation
```

---

## File commands

### `ptk ls` -- Directory listing

**Purpose:** Replaces `ls` and `tree` with token-optimized output.

**Syntax:**
```bash
ptk ls [args...]
```

All native `ls` flags are supported (`-l`, `-a`, `-h`, `-R`, etc.).

**Before / After:**
```
# ls -la (45 lines, ~800 tokens)           # ptk ls (12 lines, ~150 tokens)
drwxr-xr-x  15 user staff 480 ...          my-project/
-rw-r--r--   1 user staff 1234 ...          +-- src/ (8 files)
-rw-r--r--   1 user staff 567 ...           |   +-- main.rs
...40 more lines...                         +-- Cargo.toml
                                            +-- README.md
```

---

### `ptk tree` -- Directory tree

**Purpose:** Proxy to native `tree` with filtered output.

**Syntax:**
```bash
ptk tree [args...]
```

Supports all native `tree` flags (`-L`, `-d`, `-a`, etc.).

---

### `ptk read` -- File reading

**Purpose:** Replaces `cat`, `head`, `tail` with smart content filtering.

**Syntax:**
```bash
ptk read <file> [options]
ptk read - [options]          # Read from stdin
```

**Options:**

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--level` | `-l` | `minimal` | Filtering level: `none`, `minimal`, `aggressive` |
| `--max-lines` | `-m` | unlimited | Structural preview capped at N lines (signatures and imports, not the first N) |
| `--head-lines` | | unlimited | Keep only the first N lines, byte-exact |
| `--tail-lines` | | unlimited | Keep only the last N lines, byte-exact |
| `--line-numbers` | `-n` | no | Show line numbers |

**Filtering levels:**

| Level | Description |
|-------|-------------|
| `none` | No filtering, raw output |
| `minimal` | Removes comments and excessive blank lines |
| `aggressive` | Signatures only (removes function bodies) |

**Before / After (aggressive mode):**
```
# cat main.rs (~200 lines)                  # ptk read main.rs -l aggressive (~50 lines)
fn main() -> Result<()> {                   fn main() -> Result<()> { ... }
    let config = Config::load()?;           fn process_data(input: &str) -> Vec<u8> { ... }
    let data = process_data(&input);        struct Config { ... }
    for item in data {                      impl Config { fn load() -> Result<Self> { ... } }
        println!("{}", item);
    }
    Ok(())
}
...
```

**Languages supported for filtering:** Rust, Python, JavaScript, TypeScript, Go, C, C++, Java, Ruby, Shell.

---

### `ptk smart` -- Heuristic summary

**Purpose:** Generates a 2-line technical summary of a source file.

**Syntax:**
```bash
ptk smart <file> [--model heuristic] [--force-download]
```

**Example:**
```
$ ptk smart src/tracking.rs
SQLite-based token tracking system for command executions.
Records input/output tokens, savings %, execution times with 90-day retention.
```

---

### `ptk find` -- File search

**Purpose:** Replaces `find` and `fd` with compact output grouped by directory.

**Syntax:**
```bash
ptk find [args...]
```

Supports both PTK syntax and native `find` syntax (`-name`, `-type`, etc.).

**Before / After:**
```
# find . -name "*.rs" (30 lines)            # ptk find "*.rs" . (8 lines)
./src/main.rs                                src/ (12 .rs)
./src/git.rs                                   main.rs, git.rs, config.rs
./src/config.rs                                tracking.rs, filter.rs, utils.rs
./src/tracking.rs                              ...6 more
./src/filter.rs                              tests/ (3 .rs)
./src/utils.rs                                 test_git.rs, test_ls.rs, test_filter.rs
...24 more lines...
```

---

### `ptk grep` -- Content search

**Purpose:** Replaces `grep` and `rg` with truncated output grouped by file.

**Syntax:**
```bash
ptk grep <pattern> [path] [options]
```

**Options:**

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--max-len` |  | 80 | Maximum line length |
| `--max` |  | 200 | Maximum number of results |
| `--context-only` |  | no | Show only the match context |

None of these options has a short form: `-l`, `-m` and `-c` are reserved for the native flags with the same letter (`--files-with-matches`, `--max-count`, `--count`) and are passed to the engine.

Extra arguments are passed to the engine actually invoked -- `grep` for `ptk grep`, `rg` for `ptk rg`; PTK never substitutes one for the other. A ripgrep-specific flag (`--glob`, `-t`) is therefore only valid under `ptk rg`. Flags that change the output format (`-c`, `-l`, `-L`, `-o`, `-Z`) go straight to the engine without PTK filtering.

There is no `--file-type` option: `ptk rg -t rust` filters by type, while `ptk grep -t rust` surfaces grep's error (`invalid option -- 't'`).

**Before / After:**
```
# grep -rn "fn run" (20 lines)              # ptk grep -rn "fn run" (10 lines)
src/git.rs:45:pub fn run(...)                src/git.rs
src/git.rs:120:fn run_status(...)              45: pub fn run(...)
src/ls.rs:12:pub fn run(...)                   120: fn run_status(...)
src/ls.rs:25:fn run_tree(...)                src/ls.rs
...                                            12: pub fn run(...)
                                               25: fn run_tree(...)
```

---

### `ptk ast-grep` -- Structural (AST) search

**Purpose:** Replaces `ast-grep` with capped output grouped by file.

**Syntax:**
```bash
ptk ast-grep run -p '<pattern>' [path] [options]
```

Groups matches by file, capped at 5 per file and 50 in total; the overflow is replaced by a count note ("N more match line(s) in X" / "N more match line(s) in M more file(s) not shown"), followed by a `[full output: ...]` hint pointing to the complete output, as long as recall is enabled (`[retriever] mode`) and the compaction saves enough to pay for the hint. Otherwise the count note is followed by a fallback (`--json`, a narrower pattern, or `ptk proxy ast-grep`), and the full output is not archived. When the compacted output would end up costing more than the raw one (count notes that cost more than the lines they replace, or a hint or fallback longer than the margin saved), the raw output is printed whole, with no note and no fallback. ast-grep prints one line per source line of a match, and a structural match can span several lines, so the count is in lines, not matches. On a real search in this repository, ~85% reduction.

Only `run` is filtered: either named explicitly, or implied when no positional argument before `--` names another subcommand. `scan`, `test`, `new`, `lsp`, `outline`, `completions` and `help` pass through unchanged (the ast-grep 0.45.3 subcommands other than `run`): their output does not have this shape, and `lsp` talks over stdin. So do `run --stdin` and `run -i`: the first reads the source on the pipe the capture closes, the second opens a full-screen session (one question per match) that it writes to stdout while reading answers from /dev/tty, so the capture would swallow the whole session, question included, while ast-grep waits for an answer.

`--json` is not filtered -- an explicit request for structured output passes through unchanged, without compression.

---

### `ptk diff` -- Condensed diff

**Purpose:** Ultra-condensed diff between two files (changed lines only).

**Syntax:**
```bash
ptk diff <file1> <file2>
ptk diff -                    # Condense a unified diff read on stdin
```

Pour comparer deux fichiers : code de sortie **0** si identiques, **1** si differents,
**2** si un fichier ne peut pas etre lu. Les fichiers non UTF-8 sont compares octet
par octet ; seuls leurs noms sont affiches lorsqu'ils different.

---

### `ptk wc` -- Compact count

**Purpose:** Replaces `wc` with compact output (removes paths and padding).

**Syntax:**
```bash
ptk wc [args...]
```

Supports all native `wc` flags (`-l`, `-w`, `-c`, etc.).

---

## Git commands

### Overview

All git subcommands are supported. Unrecognized commands are passed directly to git (passthrough).

**Global git options:**

| Option | Description |
|--------|-------------|
| `-C <path>` | Change directory before running |
| `-c <key=value>` | Override a git config value |
| `--git-dir <path>` | Path to the .git directory |
| `--work-tree <path>` | Path to the working tree |
| `--no-pager` | Disable the pager |
| `--no-optional-locks` | Skip optional locks |
| `--bare` | Treat as a bare repository |
| `--literal-pathspecs` | Literal pathspecs |

---

### `ptk git status` -- Compact status

```bash
ptk git status [args...]    # Supports all git status flags
```

**Before / After:**
```
# git status (~20 lines, ~400 tokens)       # ptk git status (~5 lines, ~80 tokens)
On branch main                               main | 3M 1? 1A
Your branch is up to date with               M src/main.rs
  'origin/main'.                              M src/git.rs
                                              M tests/test_git.rs
Changes not staged for commit:                ? new_file.txt
  (use "git add <file>..." to update)        A staged_file.rs
  modified:   src/main.rs
  modified:   src/git.rs
  ...
```

---

### `ptk git log` -- Compact history

```bash
ptk git log [args...]    # Supports --oneline, --graph, --all, -n, etc.
```

**Before / After:**
```
# git log (50+ lines)                       # ptk git log -n 5 (5 lines)
commit abc123def... (HEAD -> main)           abc123 Fix token counting bug
Author: User <user@email.com>               def456 Add vitest support
Date:   Mon Jan 15 10:30:00 2024            789abc Refactor filter engine
                                             012def Update README
    Fix token counting bug                   345ghi Initial commit
...
```

---

### `ptk git diff` -- Compact diff

```bash
ptk git diff [args...]    # Supports --stat, --cached, --staged, etc.
```

**Before / After:**

Raw `git diff`:
```
diff --git a/git.rs b/git.rs
index 50f7a19..225f918 100644
--- a/git.rs
+++ b/git.rs
@@ -1,3 +1,3 @@
 fn helper(branch: &str) -> String {
-    format!("ko {}", branch)
+    format!("ok {}", branch)
 }
diff --git a/main.rs b/main.rs
index 765936a..7abfb4f 100644
--- a/main.rs
+++ b/main.rs
@@ -1,5 +1,5 @@
 fn main() {
-    let x = 42;
-    // old code
+    let config = Config::load()?;
+    config.validate()?;
     println!("start");
 }
```

`ptk git diff`:
```
git.rs  | 2 +-
 main.rs | 4 ++--
 2 files changed, 3 insertions(+), 3 deletions(-)

Changes:

git.rs
@@ -1,3 +1,3 @@
 fn helper(branch: &str) -> String {
-    format!("ko {}", branch)
+    format!("ok {}", branch)
 }
  +1 -1

main.rs
@@ -1,5 +1,5 @@
 fn main() {
-    let x = 42;
-    // old code
+    let config = Config::load()?;
+    config.validate()?;
     println!("start");
 }
  +2 -2
```

On a diff this small, ptk saves little. Compression comes from larger diffs, where per-file headers and per-hunk caps matter; truncated output ends with a truncation note and the hint `[full diff: ptk git diff --no-compact]`.

Hunk lines and `@@` headers are printed at column 0, in git's unified form, so `ptk git diff | grep "^-"` works. ptk's own annotations are indented by two spaces so those same greps do not count them: the per-file total (`  +2 -2`) and the truncation note (`  ... (30 deletions, 20 additions truncated)`).

Limitations:
- Beyond 100 changed lines per hunk, `grep -c "^-"` is capped at what is displayed; the truncation note gives the remainder per sign, but it is indented and therefore invisible to the same grep.
- On a combined diff (`diff --cc`, which git produces for each conflicted path during a merge, rebase, cherry-pick or stash pop), the marker can sit in the second column: ` +ours` is counted in the `+N -M` total but stays invisible to `grep "^+"`.
- The output is not an applicable patch: ptk removes the `diff --git` / `---` / `+++` headers. Use `ptk bypass git diff` for a faithful patch, or `ptk gh pr diff --patch`, which passes through unfiltered.

---

### `ptk git show` -- Compact show

```bash
ptk git show [args...]
```

Shows the commit summary + stat + compact diff.

> **Warning (redirecting to a file).** For a large blob
> (`ptk git show HEAD:large-file`), the output is windowed: only a preview
> is shown, followed by a hint
> `[see remaining: ptk proxy git show 'HEAD:...' | tail -n +N]`.
> A hand-written `ptk git show HEAD:x > file` can therefore silently
> truncate the content (the exit code stays 0). To capture the full
> file, follow the recovery hint, or use `ptk proxy git show`. A bare
> `git show` is not enough when the PTK hook is active: it is rewritten to
> `ptk git show`, which windows the output again.

---

### `ptk git add` -- Compact add

```bash
ptk git add [args...]    # Supports -A, -p, --all, etc.
```

**Output:** `ok` (a single word)

---

### `ptk git commit` -- Compact commit

```bash
ptk git commit -m "message" [args...]    # Supports -a, --amend, --allow-empty, etc.
```

**Output:** `ok abc1234` (confirmation + short hash)

---

### `ptk git push` -- Compact push

```bash
ptk git push [args...]    # Supports -u, remote, branch, etc.
```

**Before / After:**
```
# git push (15 lines, ~200 tokens)          # ptk git push (1 line, ~10 tokens)
Enumerating objects: 5, done.                ok main
Counting objects: 100% (5/5), done.
Delta compression using up to 8 threads
...
```

---

### `ptk git pull` -- Compact pull

```bash
ptk git pull [args...]
```

**Output:** `ok 3 files +10 -2`

---

### `ptk git branch` -- Compact branches

```bash
ptk git branch [args...]    # Supports -d, -D, -m, etc.
```

Shows the current branch, local branches and remote branches compactly.

---

### `ptk git fetch` -- Compact fetch

```bash
ptk git fetch [args...]
```

**Output:** `ok fetched (N new refs)`

---

### `ptk git stash` -- Compact stash

```bash
ptk git stash [list|show|pop|apply|drop|push] [args...]
```

---

### `ptk git worktree` -- Compact worktree

```bash
ptk git worktree [add|remove|prune|list] [args...]
```

---

### Git passthrough

Any git subcommand not listed above is run directly:

```bash
ptk git rebase main        # Runs git rebase main
ptk git cherry-pick abc    # Runs git cherry-pick abc
ptk git tag v1.0.0         # Runs git tag v1.0.0
```

---

## GitHub CLI commands

### `ptk gh` -- Compact GitHub CLI

**Purpose:** Replaces `gh` with optimized output.

**Syntax:**
```bash
ptk gh <subcommand> [args...]
```

**Supported subcommands:**

| Command | Description |
|---------|-------------|
| `ptk gh pr list` | Compact PR list |
| `ptk gh pr view <num>` | PR details + checks |
| `ptk gh pr checks` | CI check status |
| `ptk gh issue list` | Compact issue list |
| `ptk gh run list` | Workflow run status |
| `ptk gh api <endpoint>` | Compact API response |

**Before / After:**
```
# gh pr list (~30 lines)                    # ptk gh pr list (~10 lines)
Showing 10 of 15 pull requests in org/repo   #42 feat: add vitest (open, 2d)
                                              #41 fix: git diff crash (open, 3d)
#42  feat: add vitest support                 #40 chore: update deps (merged, 5d)
  user opened about 2 days ago                #39 docs: add guide (merged, 1w)
  ... labels: enhancement
...
```

---

## Test commands

### `ptk test` -- Generic test wrapper

**Purpose:** Runs any test command and shows only the failures.

**Syntax:**
```bash
ptk test <command...>
ptk test --shell fish '<fish command>'
```

By default the command and its arguments are run directly, with no shell
expansion. `--shell` takes a whole command as a single argument when a shell's
syntax is needed.

**Example:**
```bash
ptk test cargo test
ptk test npm test
ptk test bun test
ptk test pytest
```

**Before / After:**
```
# cargo test (200+ lines on failure)        # ptk test cargo test (~20 lines)
running 15 tests                             FAILED: 2/15 tests
test utils::test_parse ... ok                  test_edge_case: assertion failed
test utils::test_format ... ok                 test_overflow: panic at utils.rs:18
test utils::test_edge_case ... FAILED
...150 lines of backtrace...
```

---

### `ptk err` -- Errors/warnings only

**Purpose:** Runs a command and shows only errors and warnings.

**Syntax:**
```bash
ptk err <command...>
ptk err --shell fish '<fish command>'
```

Without `--shell`, argument boundaries are preserved, and globs, variables and
operators are not interpreted by a shell.

**Example:**
```bash
ptk err npm run build
ptk err cargo build
```

---

### `ptk cargo test` -- Rust tests

```bash
ptk cargo test [args...]
```

Shows only failures. Supports all `cargo test` arguments.

---

### `ptk cargo nextest` -- Rust tests (nextest)

```bash
ptk cargo nextest [run|list|--lib] [args...]
```

Filters `cargo nextest` output to show only failures.

---

### `ptk jest` / `ptk vitest` -- Jest/Vitest tests

```bash
ptk jest [args...]
ptk vitest [args...]
```

---

### `ptk playwright test` -- Playwright E2E tests

```bash
ptk playwright [args...]
```

---

### `ptk pytest` -- Python tests

```bash
ptk pytest [args...]
```

---

### `ptk go test` -- Go tests

```bash
ptk go test [args...]
```

Uses Go's NDJSON streaming output for precise filtering.

---

## Build and lint commands

### `ptk cargo build` -- Rust build

```bash
ptk cargo build [args...]
```

Removes "Compiling..." lines, keeping only errors and the final result.

---

### `ptk cargo check` -- Rust check

```bash
ptk cargo check [args...]
```

Removes "Checking..." lines, keeping only errors.

---

### `ptk cargo clippy` -- Rust Clippy

```bash
ptk cargo clippy [args...]
```

Groups warnings by lint rule.

---

### `ptk cargo install` -- Rust install

```bash
ptk cargo install [args...]
```

Removes dependency compilation, keeping only the install result and errors.

---

### `ptk tsc` -- TypeScript compiler

```bash
ptk tsc [args...]
```

Groups TypeScript errors by file and error code.

**Before / After:**
```
# tsc --noEmit (50 lines)                   # ptk tsc (15 lines)
src/api.ts(12,5): error TS2345: ...          src/api.ts (3 errors)
src/api.ts(15,10): error TS2345: ...           TS2345: Argument type mismatch (x2)
src/api.ts(20,3): error TS7006: ...            TS7006: Parameter implicitly has 'any'
src/utils.ts(5,1): error TS2304: ...         src/utils.ts (1 error)
...                                            TS2304: Cannot find name 'foo'
```

---

### `ptk lint` -- ESLint / Biome

```bash
ptk lint [args...]
ptk lint biome [args...]
```

Groups violations by rule and by file. Auto-detects the linter.

---

### `ptk prettier` -- Format checking

```bash
ptk prettier [args...]    # e.g. ptk prettier --check .
```

Shows only the files that need formatting.

---

### `ptk format` -- Universal formatter

```bash
ptk format [args...]
```

Auto-detects the project formatter (prettier, black, ruff format) and applies a compact filter.

---

### `ptk next build` -- Next.js build

```bash
ptk next [args...]
```

Compact output with route metrics.

---

### `ptk ruff` -- Python linter/formatter

```bash
ptk ruff check [args...]
ptk ruff format --check [args...]
```

Compressed JSON output.

---

### `ptk mypy` -- Python type checker

```bash
ptk mypy [args...]
```

Groups type errors by file.

---

### `ptk golangci-lint` -- Go linter

```bash
ptk golangci-lint run [args...]
```

Compressed JSON output.

---

## Formatting commands

### `ptk prettier` -- Prettier

```bash
ptk prettier --check .
ptk prettier --write src/
```

---

### `ptk format` -- Universal detector

```bash
ptk format [args...]
```

Automatically detects prettier, black, ruff format and rustfmt. Applies a unified compact filter.

---

## Package managers

### `ptk pnpm` -- pnpm

| Command | Description |
|---------|-------------|
| `ptk pnpm list [-d N]` | Compact dependency tree |
| `ptk pnpm outdated` | Outdated packages: `pkg: old -> new` |
| `ptk pnpm install` | Filters progress bars |
| `ptk pnpm build` | Delegates to the Next.js filter |
| `ptk pnpm typecheck` | Delegates to the tsc filter |

Unrecognized subcommands are passed directly to pnpm (passthrough).

---

### `ptk npm` -- npm

```bash
ptk npm [args...]    # e.g. ptk npm run build
```

Filters npm boilerplate (progress bars, headers, etc.).

---

### `ptk npx` -- npx with smart routing

```bash
ptk npx [args...]
```

Routes to specialized filters:
- `ptk npx tsc` -> tsc filter
- `ptk npx eslint` -> lint filter
- `ptk npx prisma` -> prisma filter
- Others -> filtered passthrough

---

### `ptk pip` -- pip / uv

```bash
ptk pip list              # Package list (auto-detects uv)
ptk pip outdated          # Outdated packages
ptk pip install <pkg>     # Install
```

Auto-detects `uv` when available and uses it instead of `pip`.

---

### `ptk deps` -- Dependency summary

**Purpose:** Compact summary of project dependencies.

```bash
ptk deps [path]    # Default: current directory
```

Auto-detects `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`, `Gemfile`, etc.

---

### `ptk prisma` -- Prisma ORM

| Command | Description |
|---------|-------------|
| `ptk prisma generate` | Client generation (removes ASCII art) |
| `ptk prisma migrate dev [--name N]` | Create and apply a migration |
| `ptk prisma migrate status` | Migration status |
| `ptk prisma migrate deploy` | Deploy to production |
| `ptk prisma db-push` | Schema push |

---

## Containers and orchestration

### `ptk docker` -- Docker

| Command | Description |
|---------|-------------|
| `ptk docker ps` | Compact container list |
| `ptk docker images` | Compact image list |
| `ptk docker logs <container>` | Deduplicated logs |
| `ptk docker compose ps` | Compact Compose services |
| `ptk docker compose logs [service]` | Deduplicated Compose logs |
| `ptk docker compose build [service]` | Build summary |

Unrecognized subcommands are passed through directly (passthrough).

**Before / After:**
```
# docker ps (long lines, ~30 tokens/line)         # ptk docker ps (~10 tokens/line)
CONTAINER ID   IMAGE          COMMAND     ...      web  nginx:1.25 Up 2d (healthy)
abc123def456   nginx:1.25     "/dock..."  ...      db   postgres:16 Up 2d (healthy)
789012345678   postgres:16    "docker..."           redis redis:7 Up 1d
```

---

### `ptk kubectl` -- Kubernetes

| Command | Description | Options |
|---------|-------------|---------|
| `ptk kubectl pods [-n ns] [-A]` | Compact pod list | Namespace or all |
| `ptk kubectl services [-n ns] [-A]` | Compact service list | Namespace or all |
| `ptk kubectl logs <pod> [-c container]` | Deduplicated logs | Specific container |

Unrecognized subcommands are passed through directly (passthrough).

---

## Data and network

### `ptk json` -- JSON structure

**Purpose:** Shows the structure of a JSON file without the values.

```bash
ptk json <file> [--depth N]    # Default: depth 5
ptk json -                     # From stdin
```

**Before / After:**
```
# cat package.json (50 lines)               # ptk json package.json (10 lines)
{                                            {
  "name": "my-app",                            name: string
  "version": "1.0.0",                         version: string
  "dependencies": {                            dependencies: { 15 keys }
    "react": "^18.2.0",                        devDependencies: { 8 keys }
    "next": "^14.0.0",                         scripts: { 6 keys }
    ...15 dependencies...                    }
  },
  ...
}
```

---

### `ptk env` -- Environment variables

```bash
ptk env                    # All variables (sensitive ones masked)
ptk env -f AWS             # Filter by name
ptk env --show-all         # Include sensitive values
```

Sensitive variables (tokens, secrets, passwords) are masked by default: `AWS_SECRET_ACCESS_KEY=***`.

---

### `ptk log` -- Deduplicated logs

**Purpose:** Filters and deduplicates log output.

```bash
ptk log <file>     # From a file
ptk log            # From stdin (pipe)
```

Repeated lines are merged: `[ERROR] Connection refused (x42)`.

---

### `ptk curl` -- HTTP with truncation

```bash
ptk curl [args...]
```

Truncates long responses and saves the full output for recovery.

---

### `ptk wget` -- Compact download

```bash
ptk wget <url> [args...]
ptk wget -O - <url>           # Output to stdout
```

Removes progress bars and noise.

---

### `ptk summary` -- Heuristic summary

**Purpose:** Runs a command and generates a heuristic summary of its output.

```bash
ptk summary <command...>
ptk summary --shell fish '<fish command>'
```

Useful for long-running commands whose output has no dedicated filter.
The command is run directly by default; `--shell` explicitly turns on
interpretation of a whole command by the chosen shell.

---

### `ptk bypass` -- Passthrough with tracking

**Purpose:** Runs a command **without filtering** but records usage for tracking.

```bash
ptk bypass <command...>
```

Useful for debugging: compare raw output with filtered output.

---

## Cloud and databases

### `ptk aws` -- AWS CLI

```bash
ptk aws <service> [args...]
```

Forces JSON output and compresses the result. Supports all AWS services (sts, s3, ec2, ecs, rds, cloudformation, etc.).

---

### `ptk psql` -- PostgreSQL

```bash
ptk psql [args...]
```

Removes table borders and compresses the output.

---

## Stacked PRs (Graphite)

### `ptk gt` -- Graphite

| Command | Description |
|---------|-------------|
| `ptk gt log` | Compact stack log |
| `ptk gt submit` | Compact submit |
| `ptk gt sync` | Compact sync |
| `ptk gt restack` | Compact restack |
| `ptk gt create` | Compact create |
| `ptk gt branch` | Compact branch info |

Unrecognized subcommands are passed through directly or detected as git passthrough.

---

## Analytics and tracking

### Tracking system

PTK records every command execution in a SQLite database:

- **Location:** `~/.local/share/ptk/tracking.db` (Linux), `~/Library/Application Support/ptk/tracking.db` (macOS)
- **Retention:** 90 days, automatic
- **Metrics:** input/output tokens, bash output reduction percentage, execution time, project

---

### `ptk gain` -- Bash output reduction statistics

```bash
ptk gain                        # Global summary
ptk gain -p                     # Filter by current project
ptk gain --graph                # ASCII graph (last 30 days)
ptk gain --history              # Recent command history
ptk gain --daily                # Day-by-day breakdown
ptk gain --weekly               # Weekly breakdown
ptk gain --monthly              # Monthly breakdown
ptk gain --all                  # All breakdowns
ptk gain --quota -t pro         # Estimate of bash output saved against the monthly quota
ptk gain --failures             # Parsing failure log (commands that fell back)
ptk gain --format json          # JSON export (for dashboards)
ptk gain --format csv           # CSV export
```

**Options:**

| Option | Short | Description |
|--------|-------|-------------|
| `--project` | `-p` | Filter by current directory |
| `--graph` | `-g` | ASCII graph of the last 30 days |
| `--history` | `-H` | Recent command history |
| `--quota` | `-q` | Estimate of bash output saved against the monthly quota |
| `--tier` | `-t` | Subscription tier: `pro`, `5x`, `20x` (default: `20x`) |
| `--daily` | `-d` | Daily breakdown |
| `--weekly` | `-w` | Weekly breakdown |
| `--monthly` | `-m` | Monthly breakdown |
| `--all` | `-a` | All breakdowns |
| `--format` | `-f` | Output format: `text`, `json`, `csv` |
| `--failures` | `-F` | Show commands that fell back |

**Example output (illustrative values):**
```
$ ptk gain
PTK Token Savings Summary
  Total commands:     1,000
  Total input:        2,000,000 tokens
  Total output:       400,000 tokens
  Total saved:        1,600,000 tokens (80%)
  Avg per command:    1,600 tokens saved

Top commands:
  git status    300x  -80%
  cargo test    150x  -90%
  git diff      100x  -75%
```

---

### `ptk discover` -- Missed opportunities

**Purpose:** Analyzes Claude Code history to find commands that ptk could have optimized.

```bash
ptk discover                          # Current project, last 30 days
ptk discover --all --since 7          # All projects, last 7 days
ptk discover -p /path/to/project      # Filter by project
ptk discover --limit 20               # Max commands per section
ptk discover --format json            # JSON export
```

**Options:**

| Option | Short | Description |
|--------|-------|-------------|
| `--project` | `-p` | Filter by project path |
| `--limit` | `-l` | Max commands per section (default: 15) |
| `--all` | `-a` | Scan all projects |
| `--since` | `-s` | Last N days (default: 30) |
| `--format` | `-f` | Format: `text`, `json` |

---

### `ptk learn` -- Learn from errors

**Purpose:** Analyzes Claude Code CLI error history to detect recurring corrections.

```bash
ptk learn                             # Current project
ptk learn --all --since 7             # All projects
ptk learn --write-rules               # Generate .claude/rules/cli-corrections.md
ptk learn --min-confidence 0.8        # Confidence threshold (default: 0.6)
ptk learn --min-occurrences 3         # Minimum occurrences (default: 1)
ptk learn --format json               # JSON export
```

---

### `ptk cc-economics` -- Claude Code economics

**Purpose:** Compares Claude Code spending (via ccusage) with the bash output saved by PTK.

```bash
ptk cc-economics                      # Summary
ptk cc-economics --daily              # Daily breakdown
ptk cc-economics --weekly             # Weekly breakdown
ptk cc-economics --monthly            # Monthly breakdown
ptk cc-economics --all                # All breakdowns
ptk cc-economics --format json        # JSON export
```

---

### `ptk hook-audit` -- Hook metrics

**Prerequisite:** Requires `PTK_HOOK_AUDIT=1` in the environment.

```bash
ptk hook-audit                        # Last 7 days (default)
ptk hook-audit --since 30             # Last 30 days
ptk hook-audit --since 0              # Full history
```

---

## Hook system

### How it works

The PTK hook intercepts Bash commands in Claude Code **before they run** and automatically rewrites them to their PTK equivalent.

**Flow:**
```
Claude Code "git status"
    |
    v
settings.json -> PreToolUse hook
    |
    v
ptk-rewrite.sh (bash)
    |
    v
ptk rewrite "git status"  ->  "ptk git status"
    |
    v
Claude Code runs "ptk git status"
    |
    v
Filtered output returned to Claude
```

**Key points:**
- Claude never sees the rewrite -- it simply receives optimized output
- The hook is a lightweight delegator that calls `ptk rewrite`
- Commands already prefixed with `ptk` pass through unchanged
- Heredocs (`<<`) are not modified
- Unrecognized commands pass through unchanged

### Installation

```bash
ptk init -g                     # Recommended install (hook + PTK.md)
ptk init -g --auto-patch        # Non-interactive (CI/CD)
ptk init -g --hook-only         # Hook only, without PTK.md
ptk init --show                 # Check the installation
ptk init -g --uninstall         # Uninstall
```

### Installed files

| File | Description |
|------|-------------|
| `~/.claude/hooks/ptk-rewrite.sh` | Hook script (delegates to `ptk rewrite`) |
| `~/.claude/PTK.md` | Minimal instructions for the LLM |
| `~/.claude/settings.json` | PreToolUse hook registration |

### `ptk rewrite` -- Command rewriting

Internal command used by the hooks. The exit code carries the permission decision, taken from Claude Code's permission rules applied to every command in a chain:

- `0`: prints the rewritten command; allow rules cover every command, so the hook may allow it without asking
- `1`: no output, no rewrite (no PTK equivalent, nothing to rewrite, a command excluded in config, a construct PTK does not rewrite such as `$(...)` or a file redirect, or a deny rule matched inside a shell wrapper's script); the command runs unchanged
- `2`: a deny matched (checked before anything else). With no output, a deny rule matched and the hook defers to the agent's own deny. With output, a ptk pipeline rule denied or routed the command, and the hook blocks it with that message
- `3`: prints the rewritten command; an ask rule matched, or allow rules do not cover every command, including when there are no rules at all; the agent asks for confirmation

An integration therefore reads stdout on `0` and `3` (and on `2`, for the block message), and keeps the confirmation prompt on `3`. With `PTK_REWRITE_HOST=openclaw`, an integration that handles approval itself gets `0` in place of every `3` that does not come from an ask rule; an integration that allows without asking on `0` therefore runs `ptk rewrite` without that variable.

```bash
ptk rewrite "git status"           # -> "ptk git status" (exit 3 with no rules)
ptk rewrite "echo hello"           # -> (exit 1, no rewrite)
ptk rewrite "ptk git status"       # -> "ptk git status" (exit 3 with no rules, unchanged)
```

### `ptk verify` -- Integrity check

Checks the integrity of the installed hook with a SHA-256 check.

```bash
ptk verify
```

### Automatically rewritten commands

| Raw command | Rewritten to |
|-------------|--------------|
| `git status/diff/log/add/commit/push/pull` | `ptk git ...` |
| `gh pr/issue/run` | `ptk gh ...` |
| `cargo test/build/clippy/check` | `ptk cargo ...` |
| `cat/head/tail <file>` | `ptk read <file>` |
| `rg/grep <pattern>` | `ptk grep <pattern>` |
| `ast-grep run -p <pattern>` | `ptk ast-grep run -p <pattern>` |
| `ls` | `ptk ls` |
| `tree` | `ptk tree` |
| `wc` | `ptk wc` |
| `jest` | `ptk jest` |
| `vitest` | `ptk vitest` |
| `tsc` | `ptk tsc` |
| `eslint/biome` | `ptk lint` |
| `prettier` | `ptk prettier` |
| `playwright` | `ptk playwright` |
| `prisma` | `ptk prisma` |
| `ruff check/format` | `ptk ruff ...` |
| `pytest` | `ptk pytest` |
| `mypy` | `ptk mypy` |
| `pip list/install` | `ptk pip ...` |
| `go test/build/vet` | `ptk go ...` |
| `golangci-lint` | `ptk golangci-lint` |
| `docker ps/images/logs` | `ptk docker ...` |
| `kubectl get/logs` | `ptk kubectl ...` |
| `curl` | `ptk curl` |
| `pnpm list/outdated` | `ptk pnpm ...` |

### Excluding commands

To prevent some commands from being rewritten, add them to `config.toml`:

```toml
[hooks]
exclude_commands = ["curl", "playwright"]
```

The pattern is anchored at the start of the command: `"curl"` excludes `curl https://...` but not `curl-config`.
Before comparing, PTK strips the wrapper, interpreter or path, so `"playwright"` covers
`playwright test` as well as `npx playwright test` or `pnpm exec playwright test`, and `"pytest"` also covers
`python3 -m pytest tests/`. Arguments are kept: `"^ls$"` excludes bare `ls` without
also matching `ls -la`.

---

## Configuration

### Configuration file

**Location:** `~/.config/ptk/config.toml` (Linux) or `~/Library/Application Support/ptk/config.toml` (macOS)

**Commands:**
```bash
ptk config                # Show the current configuration
ptk config --create       # Create the file with default values
```

### Full structure

```toml
[tracking]
enabled = true              # Enable/disable tracking
history_days = 90           # Retention in days (automatic cleanup)
database_path = "/custom/path/tracking.db"  # Custom path (optional)

[display]
colors = true               # Colored output
emoji = true                # Use emojis
max_width = 120             # Maximum output width

[filters]
ignore_dirs = [".git", "node_modules", "target", "__pycache__", ".venv", "vendor"]
ignore_files = ["*.lock", "*.min.js", "*.min.css"]

[retriever]
mode = "sqlite"             # sqlite (default) | tee (legacy files) | disabled
max_entries = 200           # Max entries in the recall database
retention_days = 30         # Entry retention
# database_path = "/custom/recall.db"  # Custom path (optional)
# A legacy [tee] section is still recognized: mapped to mode = "tee",
# or "disabled" if enabled = false

[telemetry]
enabled = false             # Anonymous telemetry (1 ping/day, requires consent)
# consent_given = true      # Set automatically by `ptk init` or `ptk telemetry enable`
# consent_date = "..."      # Consent date (RFC 3339)

[hooks]
exclude_commands = []       # Commands excluded from automatic rewriting
```

### Environment variables

| Variable | Description |
|----------|-------------|
| `PTK_RECALL=0` | Disable recall saving |
| `PTK_TEE_DIR` | Override the tee directory ("tee" mode) |
| `PTK_TELEMETRY_DISABLED=1` | Disable telemetry |
| `PTK_HOOK_AUDIT=1` | Enable hook auditing |
| `SKIP_ENV_VALIDATION=1` | Disable env validation (Next.js, etc.) |

---

## Recall system

### Raw output recovery

When a command fails (or a filter truncates a list), PTK saves the complete raw output in a local SQLite database, addressed by content hash. This lets the LLM retrieve the output without re-running the command.

**How it works:**
1. The command fails (exit code != 0) or the output is truncated
2. PTK stores the raw output (gzip) in `~/.local/share/ptk/recall.db`
3. A hint with the hash is shown: `[full output: ptk recall <hash>]` or `[+N hidden: ptk recall <hash>]`
4. `ptk recall <hash>` returns the missing part (delta); `--full` returns the complete output

**Commands:**
```bash
ptk recall <hash>            # Part not shown (delta)
ptk recall <hash> --full     # Complete output
ptk recall <hash> --grep RE  # Filter by regex
ptk recall --list            # List entries
ptk gain --recalls           # Retrieval rate per filter
ptk config recall <mode>     # Change mode without editing the config (sqlite|tee|disabled)
```

`ptk gain --recalls` keeps data strictly separated by mode: an exact rate in sqlite mode
(reads go through `ptk recall`), an approximation (`≥`) in tee mode (only bash reads
are detectable through the hook, not the Read tool). A high rate indicates a filter
that hides output the agent comes back for.

**Configuration:**

| Setting | Default | Description |
|---------|---------|-------------|
| `retriever.mode` | `"sqlite"` | `"sqlite"`, `"tee"` (legacy files), `"disabled"` |
| `retriever.max_entries` | `200` | FIFO eviction beyond this |
| `retriever.retention_days` | `30` | Old entries are purged |
| `retriever.max_entry_bytes` | `10 MB` | Truncated beyond this (at the last complete line) |
| Minimum size | 500 bytes | Failures that are too short are not saved |

The `"tee"` mode keeps the legacy behavior (`.log` files in `~/.local/share/ptk/tee/`, rotation via `tee_max_files`). A legacy `[tee]` section in the config is mapped automatically (see above).

Change the mode with `ptk config recall <sqlite|tee|disabled>` (a targeted edit of
config.toml that preserves comments and migrates legacy `[tee]` fields); the equivalent
field for manual editing is `[retriever] mode`.

---

## Telemetry

PTK can send one anonymous ping per day (23-hour interval) for usage statistics. Telemetry is opt-in and **disabled by default**; it requires explicit consent (GDPR Art. 6, 7). `ptk init` asks for consent with a `[y/N]` prompt.

**Data sent:** device hash (SHA-256 of a random salt), version, OS, architecture, command count over 24h, top commands, bash output reduction percentage, recall counters per filter family (elisions/retrievals -- names taken from a closed allowlist, never a hash, path, argument or content).

**Data controller:** `PTK AI Labs`, contact@rtk-ai.app

**Managing telemetry:**
```bash
ptk telemetry status     # Show consent status
ptk telemetry enable     # Give consent (interactive prompt)
ptk telemetry disable    # Withdraw consent
ptk telemetry forget     # Withdraw + delete local data + request server-side erasure
```

**Disable via environment variable:**
```bash
export PTK_TELEMETRY_DISABLED=1
```

No personal data, no command content and no file path is transmitted. Server retention: 12 months max. Details: [docs/TELEMETRY.md](../TELEMETRY.md)

---

## Commands by category

| Category | Commands |
|----------|----------|
| **Files** | ls, tree, read, find, grep, ast-grep, diff |
| **Git** | status, log, diff, show, add, commit, push, pull |
| **GitHub** | pr, issue, run, api |
| **Tests** | cargo test, vitest, playwright, pytest, go test |
| **Build/Lint** | cargo build, tsc, eslint, prettier, next, ruff, clippy |
| **Packages** | pnpm, npm, pip, deps, prisma |
| **Containers** | docker, kubectl |
| **Data** | json, env, log, curl, wget |
| **Analytics** | gain, discover, learn, cc-economics |

---

## Coverage

PTK covers the 9 categories above, with automatic passthrough for unrecognized subcommands, so it is always safe to use as a prefix.
