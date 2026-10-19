pub const PTK_DATA_DIR: &str = "ptk";
pub const HISTORY_DB: &str = "history.db";
pub const RECALL_DB: &str = "recall.db";
pub const CONFIG_TOML: &str = "config.toml";
pub const FILTERS_TOML: &str = "filters.toml";
pub const PIPELINE_TOML: &str = "pipeline.toml";
pub const TRUSTED_FILTERS_JSON: &str = "trusted_filters.json";
pub const DEFAULT_HISTORY_DAYS: i64 = 90;

/// PTK-only subcommands that should never fall back to raw execution.
/// When adding a new PTK-only subcommand to `Commands`, add its clap name here.
///
/// Keep the Windows CMD/PowerShell names (`dir`, `gci`, `ps`, …) out of this
/// list: `run_fallback` intercepts them through `windows_shell` *after* this
/// check, so listing one here would exit before their raw execution runs.
pub const PTK_META_COMMANDS: &[&str] = &[
    "gain",
    "discover",
    "learn",
    "init",
    "config",
    "bypass",
    "proxy",
    "recall",
    "run",
    "hook",
    "hook-audit",
    "pipe",
    "cc-economics",
    "verify",
    "trust",
    "untrust",
    "session",
    "rewrite",
    "telemetry",
    "smart",
    "deps",
    "json",
    "llm-proxy",
    "pipeline",
    "methods",
    "tasks",
    "plugins",
    "test",
    "format",
    // `err` and `summary` name no real binary, and since they grew `--shell`
    // they have a flag to get wrong. Falling through would try to exec a
    // program called `err` instead of reporting the flag error (#4125 review).
    "err",
    "summary",
];
