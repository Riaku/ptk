# Claude Code Hooks

> Part of [`hooks/`](../README.md) — see also [`src/hooks/`](../../src/hooks/README.md) for installation code

## Specifics

- Shell-based `PreToolUse` hook -- requires `jq` for JSON parsing
- Returns `updatedInput` JSON for transparent command rewrite (agent doesn't know PTK is involved)
- Exits silently (exit 0) on any failure: jq missing, PTK missing, PTK too old (< 0.23.0), no match
- Version guard checks `ptk --version` against minimum 0.23.0
- `../PTK-awareness.md` (or `-high` / `-full`, per `awareness.level` in config.toml) is the instructions file embedded into CLAUDE.md by `ptk init`

## Testing

```bash
# Run the full test suite (60+ assertions)
bash hooks/test-ptk-rewrite.sh

# Test against a specific hook path
HOOK=/path/to/ptk-rewrite.sh bash hooks/test-ptk-rewrite.sh

# Enable audit logging during testing
PTK_HOOK_AUDIT=1 PTK_AUDIT_DIR=/tmp bash hooks/test-ptk-rewrite.sh
```
