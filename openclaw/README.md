# PTK Plugin for OpenClaw

Transparently rewrites shell commands executed via OpenClaw's `exec` tool to their PTK equivalents, cutting up to 90% of the bash output that reaches the LLM context.

This is the OpenClaw equivalent of the Claude Code hooks in `hooks/ptk-rewrite.sh`.

## How it works

The plugin registers a `before_tool_call` hook that intercepts `exec` tool calls. When the agent runs a command like `git status`, the plugin delegates to `ptk rewrite` which returns the optimized command (e.g. `ptk git status`). The compressed output enters the agent's context window, saving tokens.

All rewrite logic lives in PTK itself (`ptk rewrite`). This plugin is a thin delegate -- when new filters are added to PTK, the plugin picks them up automatically with zero changes.

## Installation

### Prerequisites

PTK must be installed and available in `$PATH`. Install the pre-built binary from [GitHub Releases](https://github.com/Riaku/ptk/releases), or via the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/Riaku/ptk/develop/install.sh | sh
```

### Install the plugin

```bash
# Copy the plugin to OpenClaw's extensions directory
mkdir -p ~/.openclaw/extensions/rtk-rewrite
cp openclaw/index.ts openclaw/openclaw.plugin.json ~/.openclaw/extensions/rtk-rewrite/

# Restart the gateway
openclaw gateway restart
```

### Or install via OpenClaw CLI

```bash
openclaw plugins install ./openclaw
```

## Configuration

In `openclaw.json`:

```json5
{
  plugins: {
    entries: {
      "rtk-rewrite": {
        enabled: true,
        config: {
          enabled: true,    // Toggle rewriting on/off
          verbose: false     // Log rewrites to console
        }
      }
    }
  }
}
```

## What gets rewritten

Everything that `ptk rewrite` supports (30+ commands). See the [full command list](https://github.com/Riaku/ptk#what-it-does).

## What's NOT rewritten

Handled by `ptk rewrite` guards:
- Commands already using `PTK`
- Piped commands (`|`, `&&`, `;`)
- Heredocs (`<<`)
- Commands without an PTK filter

## Measured savings

| Command | Output reduction |
|---------|--------------|
| `git log --stat` | 87% |
| `ls -la` | 78% |
| `git status` | 66% |
| `grep` (single file) | 52% |
| `find -name` | 48% |

## License

Apache 2.0 -- same as PTK.
