pub const PTK_DATA_DIR: &str = "ptk";
pub const HISTORY_DB: &str = "history.db";
pub const RECALL_DB: &str = "recall.db";
pub const CONFIG_TOML: &str = "config.toml";
pub const FILTERS_TOML: &str = "filters.toml";
pub const TRUSTED_FILTERS_JSON: &str = "trusted_filters.json";
pub const DEFAULT_HISTORY_DAYS: i64 = 90;

/// PTK-only subcommands that should never fall back to raw execution.
/// When adding a new PTK-only subcommand to `Commands`, add its clap name here.
pub const PTK_META_COMMANDS: &[&str] = &[
    "gain",
    "discover",
    "learn",
    "init",
    "config",
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
];
