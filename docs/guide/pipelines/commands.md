---
title: Pipeline Commands
description: Inspect, validate, explain, benchmark and share compression pipelines with ptk pipeline
sidebar:
  order: 8
---

# Pipeline Commands

`ptk pipeline` is how you inspect and debug the compression engine: what pipeline is active, what it does to a given input, and why a particular rule fired. See [Pipelines](./overview.md) for the concepts and [Methods](./methods.md) for the transforms themselves.

:::note[Full edition only]
`ptk pipeline`, `ptk methods` and `ptk tasks` are part of the compression engine and are absent from the Apache-2.0 edition. See [Editions and Licensing](../resources/editions.md).
:::

## Quick reference

```bash
ptk pipeline show              # print the resolved pipeline
ptk pipeline validate          # validate every pipeline file, non-zero on error
ptk pipeline test              # parse, resolve and validate every loaded file
ptk pipeline status            # last recorded pipeline load problem, if any
ptk pipeline explain           # what fires for a given agent/tool, and why
ptk pipeline bench <file>      # before/after tokens over a local file
ptk pipeline share             # paste-ready markdown card of a pipeline
ptk pipeline share publish     # publish it: GitHub repo (recommended) or gist
ptk pipeline add <source>      # adopt a shared pipeline: validate, install, report what it needs
ptk pipeline search [query]    # find shared pipelines on GitHub (topic ptk-pipeline)
ptk pipeline promote <name>    # move a shadow install live (--discard to drop it)
ptk pipeline eval <name>       # shadow-evaluate an installed pipeline on your live sessions
ptk pipeline log tail          # the opt-in decision log
```

## Inspecting what is active

```bash
ptk pipeline show
```

Prints the pipeline that actually resolved for this directory. To print a built-in verbatim instead, name it:

```bash
ptk pipeline show ptk:default
ptk pipeline show ptk:posthook
```

## Validating your own pipelines

`validate` exits non-zero on errors, which makes it usable as a pre-commit or CI check:

```bash
ptk pipeline validate
```

`test` is the broader check — it parses, resolves and validates every pipeline file PTK loads, not just the active one. When PTK silently fell back because a file failed to load, `status` is where the reason is recorded:

```bash
ptk pipeline test
ptk pipeline status
```

Validation never writes anything into the agent's session. If a file fails to load while the agent is working, PTK drops **that file only** — your other pipeline files, their deny rules included, stay active — records the problem silently, and shows it the next time you run `ptk gain`, `ptk pipeline status` or `validate`. Every code is listed in [Errors and Warnings](./errors.md).

## Explaining a decision

`explain` answers "why did this get compressed the way it did?" for a hypothetical input:

```bash
ptk pipeline explain --agent claude-code --tool Bash --command "git log -20"
ptk pipeline explain --tool Read --file src/main.rs
ptk pipeline explain --tool Read --file big.json --ranged
```

| Flag | Meaning |
|---|---|
| `--agent` | Agent to explain for (`claude-code`, `cursor`, `cline`, …) |
| `--tool` | Tool name (`Bash`, `Read`, `Grep`, `WebFetch`, …). Defaults to `Bash`, or `Read` with `--file` |
| `--file` | File path, for the `before_tool.read` conditions (file_lines, extension, …) |
| `--ranged` | With `--file`: simulate a ranged read rather than a whole-file read |
| `--format` | Content format (`term`, `json`, `web`, `matches`, `lockfile`) |
| `--command` | Command line, for the `command_*` conditions |
| `--last` | Explain the most recent real decision from the log instead of a hypothetical |

`--file` reads the same facts the hook reads, so a dry run cannot disagree with the live decision. Pipe output in to supply the after-tool size conditions (`bytes`, `lines`, `tokens`); without it a rule gated on output size cannot match:

```bash
ptk pipeline explain --tool Grep < output.txt
```

It reports the position, the rule that fired and why, the methods applied, tokens before and after, the recovery hint — and, for a rule that cannot run on your agent, which capability is missing.

`--last` is usually the fastest route when something surprising just happened — it needs the decision log enabled.

## Shadow mode

```toml
[[hook.after_tool.shell]]
name   = "gentle-term"
when   = { is_ptk_command = false }
use    = ["ansi", "dedup"]
shadow = true
```

The rule runs, its decision and would-be savings are logged, nothing is emitted. `ptk gain --shadow` shows what it would have saved; turn `shadow` off when the numbers convince you. Transforms only, and hook rules only: a `shadow` proxy rule is skipped (W018). To try a whole shared pipeline the same way, see [shadow installs](#trying-a-pipeline-silently-shadow-installs).

## Measuring a pipeline

`bench` runs the resolved chain over a local file and reports before/after tokens:

```bash
ptk pipeline bench my-output.txt
```

Use it to check whether a pipeline change helps on your own data.

## The decision log

Off by default. When enabled it records **decisions and commands, never content**:

```bash
ptk pipeline log on
ptk pipeline log tail                    # most recent 20 entries
ptk pipeline log tail --limit 100
ptk pipeline log show --rule my-rule     # filter by rule name
ptk pipeline log show --limit 200
ptk pipeline log off
```

Each entry records timestamp, session, position, tool category, the command as the agent issued it, the rule that fired, action or methods, tokens before and after, shadow flag and recovery hint — never a tool's output, a file's contents or a directive's text. It lives at `~/.local/share/ptk/pipeline.log` (JSON lines).

```text
12:04:11  hook.after_tool.shell   cargo test        term-default    term        3,812 → 640   recall 1f3a9c
12:04:15  hook.before_tool.read   src/Service.java  shunt-large-read route → bulk-read
12:04:31  hook.after_tool.shell   ptk bulk-read …   —               skipped: is_ptk_command
```

Enable it while debugging a pipeline, then turn it off — it is the input `ptk pipeline explain --last` reads.

## Recovery reporting

PTK counts how often the agent asks for an original back. Reads of the recall store are recorded per tool and reported by `ptk gain --recalls` and the recovery-reads table in `ptk gain --by-method`.

It is reporting only: PTK never disables a rule on its own.

`ptk gain` also breaks savings down `--by-rule`, `--by-method`, `--by-layer` and `--shadow`; directives appear net of their standing cost and tasks net of worker cost. See [Token Savings Analytics](../analytics/gain.md).

## Sharing a pipeline

`share` renders a pipeline's measured impact and the pipeline file itself as one artifact. On a terminal it draws a clean, screenshot-able card; piped or redirected — or with `--md` — it emits the paste-ready **markdown** card instead (so `ptk pipeline share > card.md` keeps working), and the two renderings never mix on one stream.

```bash
ptk pipeline share              # the active pipeline (terminal card)
ptk pipeline share ptk:default  # a named one
ptk pipeline share --md         # the markdown card, even on a terminal
```

The card has two panels. **What it does** is derived from the resolved rules, never hand-written. **Measured impact** reports percentages per lever — command filtering and agent hooks as the share of a tool's output bytes removed, delegated reads as the absolute context a worker kept out of your window, and prompt-cache as the share of input served from cache. Figures are estimates (≈ bytes/4): ratios are reliable, absolutes approximate. Percentages travel between setups; token counts and costs do not, which is why the card leads with percentages.

### Publishing

`ptk pipeline share publish` is the interactive publisher. It offers two channels and walks you through validate → secret scan → preview of what leaves your machine → one confirmation:

- **GitHub repository** (recommended) — one public repo per pipeline, in **your** account: `pipeline.toml` plus a README card. The repo description carries the one-line install command, the repo gets the `ptk-pipeline` topic so `ptk pipeline search` finds it, and stars on [github.com/topics/ptk-pipeline](https://github.com/topics/ptk-pipeline) are the leaderboard.
- **GitHub repository (private)** — same repo, visible only to accounts you grant access. For a team or your own machines: anyone with access installs it with the same `ptk pipeline add github:you/repo` line (their `gh` login provides the access). No topic, no search listing — and private is one visibility toggle away from public, so the secret scan and the warnings apply unchanged.
- **GitHub gist** — a single public link with no discovery. Also available non-interactively as `ptk pipeline share --gist`.

Other hosts: publishing targets GitHub, but **installing** does not — `ptk pipeline add <url>` accepts a raw file URL from anywhere (GitLab raw links included), so nothing stops a pipeline living wherever your team keeps code.

```bash
ptk pipeline share publish       # interactive: choose repo or gist, confirm, publish
ptk pipeline share --gist        # non-interactive gist; prints: ptk pipeline add <url>
```

Everything goes through your own `gh` CLI — PTK never sees or stores a token. If `gh` is missing or logged out, the publisher prints the exact manual steps instead. Publishing the same pipeline again updates the same repo, so its link and stars survive. If the pipeline was itself installed from someone else's repo, the publisher offers to **fork** that repo instead of creating a fresh one, keeping the attribution chain visible on GitHub.

What leaves your machine: the pipeline file and the aggregate percentages from your share card. No commands, no paths, no history — and every number is labeled **self-reported** on the published card.

:::caution[Publishing is public, permanent, and your responsibility]
A repo or gist is public, permanent, and indexable. **You** are responsible for its contents — PTK does not review or endorse shared pipelines. Your pipeline file is published **verbatim**, including any `[upstream.*]` blocks (`base_url`, `api_key_env`). Before publishing, a pre-flight scan checks for API-key shapes, tokens, private-key blocks, absolute home paths, and email addresses; a finding blocks publication unless you explicitly type `publish anyway` (or pass `--force`). The scan is a safety net, not a guarantee: read the card yourself before sharing it.
:::

## Adopting a shared pipeline

`add` is the other half of `share`: one command validates a shared pipeline, installs it, and tells you what it needs.

```bash
ptk pipeline add github:owner/repo              # a published repo (see `share publish`)
ptk pipeline add github:owner/repo@<ref>        # a specific commit, tag, or branch
ptk pipeline add card.md                        # a saved share card (the TOML is auto-extracted)
ptk pipeline add - < card.md                    # or pasted on stdin
ptk pipeline add https://example.com/p          # or any URL
ptk pipeline add https://gist.github.com/u/ID   # or a gist link from `share --gist`
ptk pipeline add card.md --name flash           # name it (default: the pipeline's own name, or the file stem)
ptk pipeline add github:owner/repo --shadow     # try it silently instead of installing (see below)
```

A `github:` source (a plain `https://github.com/owner/repo` link works too) fetches the repo's `pipeline.toml` **pinned to a resolved commit** — the commit is recorded locally, so nothing the author pushes later changes what you installed. A `gist.github.com` page link is resolved to its raw content automatically.

It validates in memory first — a pipeline that does not parse or type-check installs **nothing**. On success it writes a named pipeline to `~/.config/ptk/pipelines/<name>.toml` (merged with your others, active immediately), then prints any missing auth as the exact `export` to run, and a `ptk llm-proxy start` line only if the pipeline actually has proxy rules.

On a terminal, after showing the pipeline's rules, `add` offers three choices: **install now**, **try silently first** (a shadow install — see below), or **quit**. Re-adding a name that is already installed never replaces it silently: `add` shows a line diff of the change (and the commit movement, for pinned sources), repeats that **you are responsible for what you pull and use**, and asks. Non-interactively (pipes, scripts, CI) nothing installs without an explicit opt-in: pass `--force` to accept the disclosure, or `--shadow` to try the pipeline without applying anything. The same rule applies to publishing: `share --gist` in a pipe needs `--force`, and the interactive publisher asks for an explicit "I understand" after its warning.

### Trying a pipeline silently (shadow installs)

`--shadow` (or answering `try silently` interactively) installs into a shadow slot: the pipeline is evaluated and **measured** on your live sessions but applied to nothing — no rewrites, no filtering, no denies, no tasks. After a few sessions, compare and decide:

```bash
ptk pipeline eval <name>          # what it would have done, vs your active pipeline, same traffic
ptk pipeline promote <name>       # go live (shows the diff against any installed version, confirms)
ptk pipeline promote <name> --discard   # drop it; nothing was ever applied
```

```text
installed: ~/.config/ptk/pipelines/flash.toml
auth needed: export GEMINI_API_KEY=...   (upstream `flash` — task `bulk-read`)
active now, merged with your other pipelines — see `ptk pipeline show`
```

The shared file names the *environment variable* for each key (`api_key_env`), never the key itself — you supply your own.

:::caution[You are responsible for what you install]
A pipeline directs your agent: it can rewrite the commands it runs, filter and replace what it reads, inject instructions, and send content to any upstream endpoint it declares. `add` prints exactly what the pipeline does — its rules, directives in full, and every upstream with the env var it would send — **before** anything runs. Read that summary, especially for a pipeline fetched from a URL.
:::

:::danger[Pipelines that need plugins run third-party native code]
Some pipelines reference **plugin** methods. A plugin is not sandboxed TOML — it is third-party native Rust that ptk compiles and runs on your machine with your privileges. If a shared pipeline needs a plugin you do not have, `add` names the missing plugin and refuses to install: to use it you must obtain the plugin's source from its author, add it to your build, and rebuild ptk. If the pipeline references a plugin you already have, `add` lists it in the disclosure. Either way, install a plugin only if you trust its author and have read its source — you alone are responsible for any native code you build and run.
:::

## Discovering shared pipelines

```bash
ptk pipeline search              # shared pipelines, most-starred first
ptk pipeline search rust         # filtered by a free-text query
```

Discovery is GitHub's own index of the `ptk-pipeline` topic. Each result shows the repo, its stars, and its description (which carries the author's headline number and the install line). The leaderboard is simply [github.com/topics/ptk-pipeline](https://github.com/topics/ptk-pipeline) sorted by stars. Descriptions and numbers are **self-reported by their authors** — ptk verifies nothing; `ptk pipeline add` shows every rule before anything installs, and `--shadow` lets you measure a pipeline on your own traffic before it touches anything.

## Evaluating a pipeline on your own usage

`eval` answers "would this pipeline do better on *my* workload?" without adopting it and without storing any conversation content. While armed, every tool output your sessions produce is also run through the candidate's transforms **in memory**; only the before/after sizes are recorded, the content is discarded, and your agent's output is untouched.

```bash
ptk pipeline eval squeeze-term --start   # arm the candidate (must be installed — see `add`)
# ...work normally for a few sessions...
ptk pipeline eval squeeze-term           # compare: candidate vs your active pipeline, same traffic
ptk pipeline eval squeeze-term --stop    # disarm
```

```text
Pipeline eval — `squeeze-term` · last 30 days · vs raw, same trajectory
  candidate      5.1K → 3.2K tokens  (37% removed · 214 outputs)   est.
  active now     5.1K → 4.0K tokens  (22% removed · 214 outputs)   est.
  ~ tokens est. ≈ bytes/4; both measured on the same live outputs
  ~ classifier/route effects excluded from the candidate
```

Both lines are measured on the **same** outputs, so the comparison is apples-to-apples. Limits, stated on the report itself: token counts are bytes÷4 estimates; a candidate's classifier- or route-driven rules are excluded (they would need live LLM calls), so eval covers its deterministic transforms.

The continuous counterpart lives in `ptk gain`: a per-mechanism efficiency section (filter+compress, LLM-spend, recalls, directives, proxy bill actual-vs-without) with a confidence tag on every line — `measured` where real provider usage exists, `est.` where it is a size estimate.

## Related commands

```bash
ptk methods list    # every registered method with its contract
ptk tasks list      # every configured task with its kind and upstream
```
