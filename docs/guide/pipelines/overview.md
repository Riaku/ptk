---
title: Pipelines
description: Compose what PTK does with one pipeline.toml — upgrading is a no-op, everything beyond today is opt-in
sidebar:
  order: 1
---

# The PTK pipeline

PTK saves tokens by intercepting what your coding agent reads and sends. In v1.0 what it does is a **pipeline**: a file that says *when* PTK acts, *on what*, and *what it does*. Every built-in behaviour is written in that same grammar, so you can read it (`ptk pipeline show ptk:default`), change one line of it, or replace it.

The file is agent-agnostic. You write `shell`, `read`, `before_tool`; PTK translates to Claude Code, Cursor, Gemini CLI or whatever you set up with `ptk init`. A pipeline written on one machine works on another.

:::note[Full edition only]
The pipeline engine is absent from the Apache-2.0 edition. See [Editions and Licensing](../resources/editions.md).
:::

## What it provides

PTK can act in two places, and a pipeline can add two more kinds of things.

| Where | What PTK can do there | Needs |
|---|---|---|
| **`hook`** — inside your agent, around each tool call | *before* a tool runs: rewrite a shell command through `ptk`, refuse a call, or hand the work to a cheaper model. *After* a tool runs: compress its output before the agent reads it. | `ptk init` (installs the hooks) |
| **`proxy`** — between your agent and the model provider | compress blocks already in the conversation and place the cache breakpoint | the [PTK Proxy](../proxy/overview.md) running |
| **directives** | text PTK puts in front of the agent: how to behave, what commands to use | nothing |
| **tasks** | commands the agent can run that delegate work to a cheaper model — `ptk bulk-read`, `ptk code-write` | a model key you provide |

## Three files you might write

**Nothing.** Upgrade, run `ptk init`, and you have `ptk:default` — PTK exactly as it worked before: the shell-command rewrite, the awareness file, your `filters.toml`, your `config.toml`. No new hooks are registered, nothing changes.

**Turn on more.** Output compression after every tool call, and the proxy's cache layer, are [reference pipelines](./reference-pipelines.md) you opt into — `ptk init` offers the proxy one; `ptk:posthook` you add by hand, as below.

```toml
# ~/.config/ptk/pipeline.toml
[pipeline]
extends = ["ptk:posthook", "ptk:proxy"]
```

**Tune one thing.**

```toml
[pipeline]
extends = "ptk:delegate-reads"

[override.shunt-large-read]
when.file_lines_gt = 500
```

This raises the threshold of the inherited `shunt-large-read` rule from 350 lines; the rest of its `when` is kept.

**Adopt a behaviour and supply what only you can.**

```toml
[pipeline]
extends = "ptk:delegate-reads"

[upstream.flash]
provider = "google"
model = "gemini-2.5-flash"
api_key_env = "GEMINI_API_KEY"

[task.bulk-read]
upstream = "flash"
[task.code-write]
upstream = "flash"
```

## Where to go next

| Page | What it covers |
|---|---|
| [Writing a Pipeline](./writing-pipelines.md) | where files live, the rule tree, actions, routing, overrides |
| [Conditions and Judges](./conditions.md) | every `when` key, the call context, and AI classifiers (judges) |
| [Methods](./methods.md) | the transforms rules chain together, your own chains and filters |
| [Directives](./directives.md) | text PTK puts in front of the agent |
| [Tasks and Upstreams](./tasks.md) | delegating work to a cheaper model |
| [Reference Pipelines](./reference-pipelines.md) | the six built-ins, as literal files |
| [Pipeline Commands](./commands.md) | show, validate, explain, shadow, share, adopt |
| [Errors and Warnings](./errors.md) | every diagnostic code, and the engine's limits |
| [Plugins](./plugins.md) | new methods and task kinds in Rust |

## Where things land on disk

| What | Where |
|---|---|
| user pipeline | `~/.config/ptk/pipeline.toml`, `~/.config/ptk/pipelines/*.toml` |
| project pipeline (trust-gated) | `.ptk/pipeline.toml`, `.ptk/pipelines/*.toml` |
| load problems (hooks never print) | `~/.local/share/ptk/pipeline-status.json` → `ptk pipeline status`, `ptk gain` |
| opt-in decision log | `~/.local/share/ptk/pipeline.log` → `ptk pipeline log tail` |

Paths use the Linux spelling; on macOS the user directory is `~/Library/Application Support/ptk/`, on Windows `%APPDATA%\ptk\`.

Hooks are registered for the events and tools your pipeline uses at the time of `ptk init`. After adding a rule for a new tool or event, run `ptk init -g` again — `ptk pipeline validate` prints `hook needed: …` until you do.

## Version skew: what degrades, what refuses

Pipelines travel — between machines, ptk versions, and (via `ptk pipeline share`) between people. The validator's contract is split by what a rule can do:

- **Transform rules fail open.** A compression rule naming a method or condition this ptk does not know gets a warning (`W014`/`W015`), that rule is skipped, and the rest of the file stays active. A newer pipeline on an older ptk loses an optimization, never the whole file.
- **Control rules fail closed.** A rule with an `action` (deny, route, rewrite, passthrough) must validate completely — an unknown condition or task kind is a hard error for that file. A protection can never *look* installed while being silently inert.

## Upgrading from PTK ≤ 0.51

Nothing to do. `ptk:default` is the PTK you had; there is no migration command.

| You had | In v1.0 |
|---|---|
| `filters.toml` (user or project) | unchanged — still the home of custom command filters, read by the rewrite as before |
| `[hooks] exclude_commands`, `transparent_prefixes` | unchanged, in `config.toml` |
| `[awareness] level` | unchanged; equivalent to `[override.ptk-awareness] level = …` if you prefer it in the pipeline |
| `PreToolUse` hook | unchanged; `PostToolUse` is registered only if you enable `ptk:posthook` |
