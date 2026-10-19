---
title: How PTK Savings Work
description: What PTK actually reduces, how bash output savings translate into cost, and why the token counts are estimates
sidebar:
  order: 2
---

# How PTK Savings Work

PTK cuts **up to 90% of the bash output** your agent reads. This page explains what that number measures, what it does not measure, and how it reaches your bill.

## What PTK filters

PTK sits between your agent and the CLI. When the agent runs a shell command, PTK executes it, compresses the output, and returns the compressed version.

```
agent runs a shell command
        |
        v
   PTK filters the output
        |
        v
  agent reads the result
```

The only thing PTK changes is **the bytes a shell command sends back**. Everything PTK reports as "savings" is measured on those bytes.

## The savings chain

```
Cost
├─ Input tokens
│  ├─ Bash output           <- the only part PTK filters
│  ├─ Your prompt
│  ├─ System prompt
│  └─ Conversation history
└─ Output tokens            <- what the model writes
```

Those bytes are **one contributor to input tokens**, alongside your prompt, the system prompt, and conversation history. Input tokens are in turn **only part of the bill**, which also counts output tokens.

So the reduction dilutes at every step: a large cut in bash output produces a smaller cut in input tokens, and a smaller one again in cost. A command showing 90% fewer output bytes does not make your session 90% cheaper.

This is why PTK reports bash output reduction rather than a cost figure. Bash output is the part PTK controls; the rest depends on your prompt, your model, how much the agent writes back, and how much of the conversation is replayed on each call.

## Why the token counts are estimates

`ptk gain` estimates tokens as `bytes / 4` (rounded up). PTK ships **no tokenizer**, so:

- **The percentage is reliable.** The same estimator is applied to the raw output and the filtered output, so the ratio between them holds regardless of the estimator's absolute accuracy.
- **The absolute token counts are approximate.** They will not match your provider's billing. Treat a figure such as `Input tokens: 50,000` as an order of magnitude, not an invoice line.

If you need exact counts, run the raw and filtered output through your model's own tokenizer.

## How to read `ptk gain`

| Column | What it actually is |
|--------|---------------------|
| Input | Estimated tokens from raw command output, `bytes / 4` |
| Output | Estimated tokens after filtering, `bytes / 4` |
| Saved | Input minus Output, in estimated tokens |
| Save% | Reduction in bash output bytes |

`Save%` is the meaningful number. It is a byte ratio, and it is accurate as a ratio.

## What PTK does not reduce

- **Output tokens.** PTK never touches what the model writes.
- **Your prompt, the system prompt, or conversation history.** These are input tokens PTK has no visibility into.
- **Commands with no matching filter.** These pass through untouched and are tracked at 0% savings. See `ptk gain --history`.

## See also

- [What PTK Optimizes](what-ptk-covers.md) — what each supported command filter changes
- [Token Savings Analytics](../analytics/gain.md) — reading the `ptk gain` dashboard
