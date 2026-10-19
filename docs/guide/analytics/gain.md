---
title: Token Savings Analytics
description: Measure and analyze the bash output reduction PTK achieves with ptk gain
sidebar:
  order: 1
---

# Token Savings Analytics

`ptk gain` shows how much bash output PTK has removed across all your commands, with daily, weekly, and monthly breakdowns.

What `ptk gain` measures is the reduction in **bash output bytes**, converted to estimated tokens. Bash output is one contributor to input tokens, alongside your prompt, the system prompt and conversation history, and input tokens are in turn only part of the bill, which also counts output tokens. See [How PTK Savings Work](../resources/savings-explained.md) for the full picture.

## Quick reference

```bash
# Default summary
ptk gain

# Temporal breakdowns
ptk gain --daily          # all days since tracking started
ptk gain --weekly         # aggregated by week
ptk gain --monthly        # aggregated by month
ptk gain --all            # all breakdowns at once

# Classic flags
ptk gain --graph          # ASCII graph, last 30 days
ptk gain --history        # last 10 commands
ptk gain --quota          # monthly quota savings estimate (default tier: 20x)
ptk gain --quota -t pro   # use pro tier token budget for estimate
ptk gain --recalls        # recall efficiency per filter (calibration)

# Live dashboard
ptk gain                  # auto-launches the dashboard when the PTK Proxy runs
ptk gain --watch          # force the dashboard
ptk gain --text           # force the static view

# Attribution (where the savings came from)
ptk gain --by-method      # per compression method
ptk gain --by-rule        # per pipeline rule
ptk gain --by-layer       # per token layer (read, tool_result, history, ...)
ptk gain --by-pipeline    # per pipeline
ptk gain --pipeline NAME  # one pipeline, sliced by where its savings land
ptk gain --shadow         # what shadow rules would have saved

# Export
ptk gain --all --format json > savings.json
ptk gain --all --format csv  > savings.csv

# Reset
ptk gain --reset          # zero the savings and recall stats (asks first)
```

## Daily breakdown

```bash
ptk gain --daily
```

**Example output** (illustrative values — yours depend entirely on which commands you run):

```
📅 Daily Breakdown (3 days)
════════════════════════════════════════════════════════════════
Date            Cmds      Input     Output      Saved   Save%
────────────────────────────────────────────────────────────────
2026-01-01       100     500.0K     100.0K     400.0K   80.0%
2026-01-02       100     500.0K     100.0K     400.0K   80.0%
2026-01-03       100     500.0K     100.0K     400.0K   80.0%
────────────────────────────────────────────────────────────────
TOTAL            300       1.5M     300.0K       1.2M   80.0%
```

- **Cmds**: PTK commands executed
- **Input**: Estimated tokens from raw command output (`bytes / 4`)
- **Output**: Estimated tokens after filtering (`bytes / 4`)
- **Saved**: Input - Output, in estimated tokens
- **Save%**: Saved / Input × 100 — a **bash output byte ratio**, not a share of your bill

## Weekly and monthly breakdowns

```bash
ptk gain --weekly
ptk gain --monthly
```

Same columns as daily, aggregated by Sunday-Saturday week or calendar month.

## Export formats

| Format | Flag | Use case |
|--------|------|----------|
| `text` | default | Terminal display |
| `json` | `--format json` | Programmatic analysis, dashboards |
| `csv` | `--format csv` | Excel, Python/R, Google Sheets |

**JSON structure:**
```json
{
  "summary": {
    "total_commands": 300,
    "total_input": 1500000,
    "total_output": 300000,
    "total_saved": 1200000,
    "avg_savings_pct": 80.0
  },
  "daily": [...],
  "weekly": [...],
  "monthly": [...]
}
```

## How token estimation works

`ptk gain` estimates tokens as `bytes / 4`; PTK ships no tokenizer. The same estimator is applied to raw and filtered output, so the percentage is reliable; the absolute token counts are approximate and will not match your provider's billing.

```
Input Tokens  = estimate_tokens(raw_command_output)
Output Tokens = estimate_tokens(ptk_filtered_output)
Saved Tokens  = Input - Output
Savings %     = (Saved / Input) × 100
```

## Database

Savings data is stored locally in SQLite:

- **Location**: `~/.local/share/ptk/history.db` on Linux, `~/Library/Application Support/ptk/history.db` on macOS (override with `PTK_DB_PATH`) — paths below use the Linux location
- **Retention**: 90 days (automatic cleanup)
- **Scope**: Global across all projects and Claude sessions

```bash
# Inspect raw data
sqlite3 ~/.local/share/ptk/history.db \
  "SELECT timestamp, ptk_cmd, saved_tokens FROM commands
   ORDER BY timestamp DESC LIMIT 10"

# Backup
cp ~/.local/share/ptk/history.db ~/backups/ptk-history-$(date +%Y%m%d).db

# Reset
rm ~/.local/share/ptk/history.db    # recreated on next command
```

Prefer `--reset` over deleting the file: it zeroes both the token savings and the
[recall stats](#recall-efficiency), which live separately, and leaves the database intact.

```bash
ptk gain --reset          # asks for confirmation
ptk gain --reset --yes    # skip the prompt (scripts)
```

## Analysis workflows

```bash
# Weekly progress: generate a CSV report every Monday
ptk gain --weekly --format csv > reports/week-$(date +%Y-%W).csv

# Monthly budget review
ptk gain --monthly --format json | jq '.monthly[] |
  {month, saved_tokens, quota_pct: (.saved_tokens / 6000000 * 100)}'

# Cron: daily JSON snapshot for a dashboard
0 0 * * * ptk gain --all --format json > /var/www/dashboard/ptk-stats.json
```

**Python/pandas:**
```python
import pandas as pd
import subprocess

result = subprocess.run(['ptk', 'gain', '--all', '--format', 'csv'],
                       capture_output=True, text=True)
lines = result.stdout.split('\n')
daily_start = lines.index('# Daily Data') + 2
daily_end = lines.index('', daily_start)
daily_df = pd.read_csv(pd.StringIO('\n'.join(lines[daily_start:daily_end])))
daily_df['date'] = pd.to_datetime(daily_df['date'])
daily_df.plot(x='date', y='savings_pct', kind='line')
```

**GitHub Actions (weekly stats):**
```yaml
on:
  schedule:
    - cron: '0 0 * * 1'
jobs:
  stats:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo install --git https://github.com/riaku/ptk --branch master ptk
      - run: ptk gain --weekly --format json > stats/week-$(date +%Y-%W).json
      - run: git add stats/ && git commit -m "Weekly ptk stats" && git push
```

## Quota estimate

`--quota` expresses the estimated tokens saved as a fraction of a monthly subscription budget. Like every other figure in `ptk gain`, it is derived from the `bytes / 4` estimate of bash output, so treat it as an order of magnitude rather than a billing forecast.

```bash
ptk gain --quota          # uses 20x tier by default
ptk gain --quota -t pro   # Claude Pro plan budget
ptk gain --quota -t 5x    # 5× usage plan budget
ptk gain --quota -t 20x   # 20× usage plan budget
```

The tiers (`pro`, `5x`, `20x`) correspond to Anthropic Claude API subscription levels, each with a different monthly token allocation. PTK uses those allocations as a denominator to express your savings as a percentage of your budget.

:::tip[Find missed savings]
`ptk gain` shows what PTK saved. To find commands that ran *without* PTK and calculate what you lost, see [ptk discover](./discover.md).
:::

## Live dashboard

`--watch` replaces the static report with a full-screen live dashboard covering **both** savings layers at once: the agent hooks trimming command output, and the [PTK Proxy](../proxy/overview.md) compressing billed input tokens.

```bash
ptk gain --watch
```

### Auto-launch

A bare `ptk gain` launches the dashboard **automatically** once the PTK Proxy is running. All four conditions must hold:

1. You passed no flag asking for a specific static view — no `--daily`, `--weekly`, `--monthly`, `--all`, `--history`, `--graph`, `--quota`, and `--format` left at `text`.
2. You did not pass `--text`.
3. Standard output is an interactive terminal — piping or redirecting opts out, so scripts and cron jobs are unaffected.
4. The PTK Proxy is running and reachable.

Without the proxy running, `ptk gain` behaves exactly as it always has.

`--text` is the escape hatch — it forces the static view with the proxy up:

```bash
ptk gain --text
```

`--watch` and `--text` are mutually exclusive. Asking for `--watch` without a terminal falls back to the text view with a warning rather than failing.

## Attribution

The default report tells you *how much* was saved. The attribution flags tell you *where it came from*, which helps you tune a pipeline. Each covers both the hook and proxy sites; proxy figures appear only if you run the [PTK Proxy](../proxy/overview.md).

```bash
ptk gain --by-method      # which compression method did the work
ptk gain --by-rule        # which pipeline rule fired
ptk gain --by-layer       # which token layer (read, tool_result, history, ...)
ptk gain --by-pipeline    # which pipeline
```

These combine — passing several renders several sections in one report.

### One pipeline in detail

`--pipeline <NAME>` slices a single pipeline by where its savings actually land: which tool outputs the hooks trimmed, and which methods compressed billed input.

```bash
ptk gain --pipeline my-pipeline
```

### Shadow rules

Shadow rules are recorded but never emitted — they let you measure a rule's effect before trusting it with real output. `--shadow` reports what they *would* have saved:

```bash
ptk gain --shadow
```

A shadow rule showing strong savings is a candidate to promote; one showing nothing is a rule you can delete.

:::note[Full edition only]
Rule, pipeline and shadow attribution describe the compression engine, which the Apache-2.0 edition does not include. See [Editions and Licensing](../resources/editions.md).
:::

## Recall efficiency

`--recalls` measures how often your AI assistant goes back for output a filter elided — the signal that a filter's cap is too aggressive for your workflow. Every time a filter stores elided output (an *elision*) and every time the assistant retrieves it (a *recall*), PTK counts it per filter:

```bash
ptk gain --recalls
```

Illustrative output:

```
Recall efficiency  (current mode: sqlite)

SQLITE (exact — reads go through ptk recall)
FILTER                   ELISIONS RECALLED   RATE
vitest                         50       20    40%
docker-images                 100        2     2%

TEE (approximate — bash-observed reads only)
FILTER                   ELISIONS RECALLED   RATE
cargo_test                     40        4   ≥10%
```

How to read it:

- **RATE** is the share of elided outputs the assistant went back for. Re-reading the same entry counts once — the rate measures *entries consulted*, not read commands.
- A **high rate** means that filter regularly hides output the assistant needs: every recall is an extra API round-trip you paid for. Consider raising that filter's cap.
- A **low rate** means the filter's cap is well calibrated — the elided output was noise.

The two sections are never merged because the data quality differs:

- **SQLITE (exact)**: reads go through `ptk recall <hash>`, the only access path, so the count is exact.
- **TEE (approximate)**: in legacy [tee mode](../getting-started/configuration.md#recall-system), reads are shell commands (`tail`, `cat`, `grep`, …) observed by the rewrite hook before execution. Editor or assistant file-tool reads are invisible, denied commands are not counted, and the per-file dedup window is finite — so the `≥` rate is approximate, not exact. A high rate is still a reliable signal that the cap is too aggressive.

Stats survive entry eviction and retention cleanup: calibration data is kept even after the underlying outputs are purged.

## Troubleshooting

**No data showing:**
```bash
ls -lh ~/.local/share/ptk/history.db
sqlite3 ~/.local/share/ptk/history.db "SELECT COUNT(*) FROM commands"
git status    # run any tracked command to generate data
```

**Incorrect statistics:** Token estimation is a heuristic. For precise counts, use `tiktoken`:
```bash
pip install tiktoken
git status > output.txt
python -c "
import tiktoken
enc = tiktoken.get_encoding('cl100k_base')
print(len(enc.encode(open('output.txt').read())), 'actual tokens')
"
```
