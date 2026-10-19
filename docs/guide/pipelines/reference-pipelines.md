---
title: Reference Pipelines
description: The six built-in pipelines as literal files — ptk:default, ptk:posthook, ptk:proxy, ptk:delegate-reads, ptk:verbatim-diffs, ptk:terse
sidebar:
  order: 7
---

# Reference Pipelines

Printable with `ptk pipeline show <name>`. Adopt with `extends`; tune with [`[override]`](./writing-pipelines.md#overrides).

## `ptk:default` — PTK exactly as it works today

What every user has after upgrading, with no action. One hook event (`PreToolUse`), the awareness file, your `filters.toml` and `config.toml`. Nothing else is registered or changed.

It also carries one `proxy.request` rule, `normalize`, which does nothing unless you are running the daemon. It strips ANSI codes, progress bars and repeated lines — bytes no reader ever sees — and each of its methods is a pure function of the block's own content, so the request prefix stays byte-identical across turns and the upstream prompt cache keeps hitting.

```toml
schema_version = 1
[pipeline]
name = "ptk:default"

[[hook.before_tool.shell]]
name   = "ptk-rewrite"            # `ls` → `ptk ls` for every command PTK has a filter for; others untouched
action = "rewrite"

[[proxy.request.tool_result]]
name = "normalize"                # inert without a running daemon
use  = ["ansi", "progress", "dedup"]

[[directive]]
use = "ptk-awareness"
level = "default"     # today's [awareness] level in config.toml still applies

[guards]
never_worse = true
min_size = 512
```

How the rewrite flows: your agent calls `Bash "ls -la"` → the hook evaluates `hook.before_tool.shell` → `ptk-rewrite` matches → the call becomes `ptk ls -la` → inside `ptk`, the `ls` filter (built-in, or yours from `filters.toml`) applies. Excluding a command is a `passthrough` rule ahead of `ptk-rewrite`, or `[hooks] exclude_commands` in `config.toml` as before.

**Hooks follow the pipeline.** `ptk init` registers only the events your active pipeline uses, for the tools its rules name. With `ptk:default` that is `PreToolUse` on `Bash`, as today. Enable `ptk:posthook` and `PostToolUse` is registered too. Registration happens at `ptk init`: after adding a rule at a new event or for a new tool (a `hook.before_tool.read` rule, any `hook.after_tool` rule), run `ptk init -g` again. Until you do, `ptk pipeline validate`, `ptk gain` and `ptk init --show` print `hook needed: …`.

## `ptk:posthook` — compress every tool's output

Opt in. Adds `PostToolUse`: after a tool runs, its output is compressed before the agent reads it. Only representation changes; nothing is dropped except by `cap`, which keeps the full output for `ptk recall`.

```toml
[pipeline]
name = "ptk:posthook"
extends = "ptk:default"

[[hook.after_tool.shell]]
name = "term-default"             # floor for output that did not come through ptk
when = { is_ptk_command = false }
use  = ["term"]

[[hook.after_tool.read]]
name = "lockfiles"
when = { basename_in = ["package-lock.json", "Cargo.lock", "yarn.lock", "pnpm-lock.yaml"] }
use  = ["lockfile"]

[[hook.after_tool.read]]
name = "json-reads"
enabled = false
when = { format = "json" }
use  = ["json"]

[[hook.after_tool.read]]
name = "strip-line-numbers"
enabled = false
use  = ["strip-line-numbers"]

[[hook.after_tool.search]]
name = "grep-group"
when = { format = "matches" }     # grep output; glob file lists are not matches and pass untouched
use  = ["matches"]

[[hook.after_tool.web]]
name = "web-default"
use  = ["web"]
```

## `ptk:proxy` — cache alignment in the proxy

Opt in; needs the PTK proxy running (`ptk init` offers to install it). Normalisation
(`ansi`, `progress`, `dedup`) is inherited from `ptk:default`, so what this pipeline adds on top
is the cache breakpoint.

Both are **Anthropic-dialect only**, and `cache-align` does nothing for a client
that already sets its own `cache_control` breakpoints — Claude Code does. An
OpenAI-dialect client is measured but not compressed ([Provider dialects](../proxy/configuration.md#provider-dialects)).

```toml
[pipeline]
name = "ptk:proxy"
extends = "ptk:default"

[[proxy.request.cache]]
name = "align"
use  = ["cache-align"]
```

## `ptk:delegate-reads` — route oversize reads to a cheaper model

```toml
[pipeline]
name = "ptk:delegate-reads"
extends = "ptk:default"

[task.bulk-read]
kind = "summarize"    # no upstream: you supply the model
[task.code-write]
kind = "generate"

[[hook.before_tool.read]]
name   = "shunt-large-read"
when   = { file_lines_gt = 350, whole_file = true }
action = "route"
task   = "bulk-read"
```

Adopt it as in [the third example on the overview](./overview.md#three-files-you-might-write). Tune: `[override.shunt-large-read] when.file_lines_gt = 500`; `[task.bulk-read] temperature = 0.2 instructions = "…"`. `code-write` has no rule — denying a write wastes tokens already generated — the agent learns it from the generated `route-tasks` directive. A `cat big.java` is covered too: it is rewritten to `ptk read big.java`, which evaluates the same rule.

**Routing a command, and routing output** ([how routing works](./writing-pipelines.md#routing-before-a-tool)). The same action covers the other two
anchors. Neither is shipped in a built-in — write them yourself:

```toml
[pipeline]
name = "delegate-commands"
extends = "ptk:posthook"

[upstream.flash]
provider     = "google"
model        = "gemini-2.5-flash"
api_key_env  = "GEMINI_API_KEY"
timeout_secs = 8

[task.check-tests]
kind     = "digest"
upstream = "flash"

[[hook.before_tool.shell]]           # the command never runs in your context
name   = "delegate-tests"
when   = { command_matches = "^(npm|pnpm) test\\b" }
action = "route"
task   = "check-tests"

[[hook.after_tool.web]]              # the fetch already happened; digest it
name   = "digest-pages"
when   = { tokens_gt = 4000 }
action = "route"
task   = "check-tests"
```

The first denies the call and tells the agent to run
`ptk check-tests --question "…" -- npm test`; the exit code still comes from
`npm test`. The second sends the fetched page to the worker and returns its
bullets with a `ptk recall` hint for the original.

Prefer a transform chain where one is enough. `use = ["term"]`
costs no time and no tokens and keeps the whole output recoverable; an
`after_tool` route spends a provider round trip — seconds, on the hook path —
to throw detail away. It earns its keep on output that is both very large and
genuinely disposable, and `timeout_secs` is what bounds the damage when the
worker is slow.

## `ptk:verbatim-diffs` — no line numbers, diffs untouched

```toml
[pipeline]
name = "ptk:verbatim-diffs"
extends = ["ptk:posthook", "ptk:proxy"]

[override.strip-line-numbers]     # for agents whose edit tools do not address lines by number
enabled = true

[[hook.before_tool.shell]]        # diffs must reach the agent byte-for-byte
name   = "keep-diffs"
when   = { command_matches = "^git (diff|show)\\b" }
action = "passthrough"
```

Selective compression with a recovery path and grep grouping come from `ptk:posthook` (`term`, `matches`). It also inherits `ptk:proxy`, which adds the cache breakpoint (see [`ptk:proxy`](#ptkproxy--cache-alignment-in-the-proxy)).

## `ptk:terse` — terse prose

```toml
[pipeline]
name = "ptk:terse"
extends = "ptk:default"

[[directive]]
use = "terse-output"
```

A directive only: it changes what the agent *writes*, never what it reads, so it
composes with anything. For terse prose *and* output compression, name both —
`extends = ["ptk:terse", "ptk:posthook"]`. A directive is also the one thing you
can adopt without a pipeline at all: a file containing nothing but
`[[directive]] use = "terse-output"` is valid and leaves `ptk:default` intact.
