---
title: Discover and Session
description: Find missed savings opportunities with ptk discover, and track PTK adoption with ptk session
sidebar:
  order: 2
---

# Discover and Session

## ptk discover — find missed savings

`ptk discover` analyzes your Claude Code command history to identify commands that ran without PTK filtering, and estimates how much bash output PTK would have removed from them.

```bash
ptk discover                    # analyze current project history
ptk discover --all              # all projects
ptk discover --all --since 7    # last 7 days, all projects
```

**Example output** (illustrative values):

```
Missed savings analysis (last 7 days)
────────────────────────────────────
Command              Count   Est. lost
cargo test              12     ~48,000 tokens
git log                  8     ~12,000 tokens
pnpm list                3      ~6,000 tokens
────────────────────────────────────
Total missed:           23     ~66,000 tokens

Run `ptk init --global` to capture these automatically.
```

The `~N tokens` figures are **estimated bash output bytes divided by 4**, not tokens billed by your provider. PTK ships no real tokenizer, and bash output is only one contributor to input tokens. Read them as an order of magnitude of the output volume PTK could have compressed. See [How PTK Savings Work](../resources/savings-explained.md).

If commands appear in the missed list after installing PTK, it usually means the hook isn't active for that agent. See [Troubleshooting](../resources/troubleshooting.md#ai-assistant-not-using-ptk) — "AI assistant not using PTK".

## ptk session — adoption tracking

`ptk session` shows PTK adoption across recent Claude Code sessions: how many shell commands ran through PTK vs. raw.

```bash
ptk session
```

**Example output** (illustrative values):

```
Recent sessions (last 10)
─────────────────────────────────────────────────────
Session                         Total   PTK   Coverage
2026-01-03 10:00  (40 cmds)       40    38      95.0%
2026-01-02 10:00  (40 cmds)       40    40     100.0%
2026-01-01 10:00  (40 cmds)       40    36      90.0%
─────────────────────────────────────────────────────
Average coverage: 95.0%
```

Low coverage on a session usually means PTK was disabled (`PTK_DISABLED=1`) or the hook wasn't active for a specific subagent.
