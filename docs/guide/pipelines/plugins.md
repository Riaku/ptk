---
title: Plugins
description: Write your own pipeline method in Rust and build it into ptk — the Method trait, the ptk-plugin SDK, and the ptk plugins workflow
sidebar:
  order: 10
---

# Plugins — your own methods and task kinds

A **method** is the unit of work in a pipeline (`ansi`, `dedup`, `cap`, `toon`,
…). A **plugin** is a package *you* write that provides one or more new methods.
The built-ins cover common output; a plugin is for a transform only you need —
masking a proprietary secret format, collapsing a house log style, reshaping a
bespoke tool's output.

Decisions are different from transforms. If you want an AI/LLM judgment ("is this
worth compressing?", "which model should serve this?"), that is a
[`[classifier.*]`](./conditions.md#judges--conditions-on-meaning) in your `pipeline.toml`, not a
plugin. Plugins are for *transforming* a block of text.

## The workflow

```bash
ptk plugins new shout      # scaffold a method crate in ~/.config/ptk/plugins/
# edit shout/src/lib.rs — fill in apply()
ptk plugins build          # compile a custom ptk with your method linked in
ptk plugins list           # show your plugin crates and what this binary carries
```

Then name the method in a rule, exactly like a built-in:

```toml
[[hook.after_tool.shell]]
name = "loud-errors"
use  = ["shout"]
```

`ptk plugins build` generates a small host crate that depends on `ptk` and your
plugin crates, registers each method, and calls `ptk::entry()`; it compiles with
`cargo` and installs the result (a binary still named `ptk`) to `~/.cargo/bin`.
Nothing about the engine, the hooks, or installation changes — a plugin is
purely additive.

## What you write

A method implements one trait from the `ptk-plugin` SDK. The scaffold gives you
an identity transform to fill in:

```rust
use ptk_plugin::{ptk_plugin, Block, Method, MethodCtx, MethodInfo, Outcome};

struct Shout;

impl Method for Shout {
    fn name(&self) -> &'static str {
        "shout"
    }

    fn info(&self) -> MethodInfo {
        MethodInfo::LOSSLESS_TRANSFORM
    }

    fn apply(&self, block: &Block, _ctx: &MethodCtx) -> Outcome {
        Outcome::Continue(block.text.to_uppercase())
    }
}

ptk_plugin!(Shout);
```

- `apply` receives the block's `text` (plus its subject, format, and provenance)
  and returns an `Outcome`: `Continue(String)` to pass the transformed text down
  the chain, `ShortCircuit(String)` to stop the chain, or `Skip` to leave the
  block untouched.
- `info()` is the contract the engine type-checks at load: `lossy`, `recovery`
  (how a reader gets the original back, e.g. `Recall` or `None`), `cost`, and what the method consumes/produces. Use
  `MethodInfo::LOSSLESS_TRANSFORM` as the base and override only what differs —
  a lossy method must declare its recovery so the original can be restored:
  by default from the recall store (`ptk recall <hash>`), or from files under
  the tee directory if you set `[retriever] mode = "tee"`.
- `ptk_plugin!(…)` lists what the crate exports so the build can register it.
  A crate may export several methods: `ptk_plugin!(Shout, Whisper);`. To export
  task kinds too (or instead), use the long form:
  `ptk_plugin!(methods: [Shout], kinds: [Checklist]);` — `ptk plugins build`
  registers both.

`ptk pipeline validate` checks a plugin method like any other — a bad contract
(a terminal method not last, a `Slow` method at PreToolUse, a format it cannot
consume) fails at build, with the method name.

## Your own task kinds

Methods transform content; **task kinds** define what a delegated worker does.
The built-ins are `summarize` (answer a question about files), `generate`
(write one file from a spec), and `digest` (condense command output). A plugin
can register a new kind the same way it registers a method:

```rust
use ptk_plugin::{ptk_plugin, CorpusFile, TaskIntent, TaskKind, TaskOutcome, WorkerRequest};

struct Checklist;

impl TaskKind for Checklist {
    fn name(&self) -> &'static str { "checklist" }
    fn usage(&self) -> &'static str { "ptk <task> --question \"...\" <files>" }
    fn default_instructions(&self) -> &'static str { "Answer as a checklist." }
    fn assemble(&self, intent: &TaskIntent, corpus: &[CorpusFile],
                instructions: Option<&str>, prompt: Option<&str>)
        -> Result<WorkerRequest, String> { /* build the worker prompt */ }
    fn handle(&self, output: &str) -> TaskOutcome {
        TaskOutcome::Print(output.trim().to_string())
    }
}

ptk_plugin!(methods: [], kinds: [Checklist]);
```

The `ptk_plugin!` line is what `ptk plugins build` wires into the binary. Once registered,
`[task.review] kind = "checklist"` validates, `ptk review`
exists as a CLI command, and rules can `action = "route"` to it. Names must be
unique — built-in kinds cannot be shadowed. A pipeline that declares a task
with a kind your build does not have fails validation (E014).

## Upgrading ptk

A plugin is compiled into your custom `ptk` binary, so installing a new ptk
release replaces that binary and your plugins with it. After every upgrade:

1. In each plugin's `Cargo.toml`, set the `ptk-plugin` dependency's `tag` to
   the new version (`ptk --version` → `vX.Y.Z`). `ptk plugins new` writes the
   tag once, at scaffold time.
2. Run `ptk plugins build` again.

Until you rebuild, a pipeline rule that uses a plugin method fails open: it gets
W014 and is skipped while the rest of the file stays active. A control rule (one
with `action =`) that depends on it is an error (E007) and its file is not
loaded. `ptk pipeline validate` shows which rules are affected.

## Requirements and notes

- A Rust toolchain (`cargo`) on PATH, and network access on the first build —
  the scaffolded `Cargo.toml` depends on `ptk-plugin` by git, pinned to the tag
  matching your installed `ptk`, so your method always links the exact engine
  your `ptk` runs.
- A plugin adds methods and task kinds, not conditions. Every field of a
  tool's input is already a condition via the `input.*` prefix
  (`when."input.subagent_type" = "Explore"`), and AI-driven conditions are
  classifiers.
- A project-local `.ptk/plugins/` is trust-gated like every other project file;
  review with `ptk trust` before it is built.
