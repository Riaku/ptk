//! Invariants about configuration files that ship in the repository itself, so a
//! hand-edit of one cannot silently disagree with what `ptk init` writes.
//!
//! The Copilot hook config is the file this catches: `.github/hooks/ptk-rewrite.json`
//! is checked in, read by telemetry (`COPILOT_HOOK_FILE`), and rewritten by
//! `ptk init --copilot`. It was once restored by hand with the right filename and a
//! body that could never fire — `"ptk hook"` with no subcommand is a clap error, and
//! without `"version": 1` Copilot ignores the file entirely.

use std::fs;
use std::path::Path;

use serde_json::Value;

fn checked_in_copilot_hook() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/hooks/ptk-rewrite.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must exist and be readable: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("{} must be valid JSON: {e}", path.display()))
}

#[test]
fn the_checked_in_copilot_hook_carries_the_schema_version() {
    let hook = checked_in_copilot_hook();
    assert_eq!(
        hook.get("version").and_then(Value::as_u64),
        Some(1),
        "Copilot only honors the hook file when it declares \"version\": 1"
    );
}

#[test]
fn the_checked_in_copilot_hook_runs_a_real_subcommand() {
    let hook = checked_in_copilot_hook();
    let command = hook
        .pointer("/hooks/PreToolUse/0/command")
        .and_then(Value::as_str)
        .expect("the hook must register a PreToolUse command");

    assert_eq!(
        command, "ptk hook copilot",
        "bare `ptk hook` is a missing-subcommand error, so the hook would never run"
    );
}

#[test]
fn the_checked_in_copilot_hook_is_wired_to_a_command_hook() {
    let hook = checked_in_copilot_hook();
    let entry = hook
        .pointer("/hooks/PreToolUse/0")
        .expect("the hook must register a PreToolUse entry");

    assert_eq!(
        entry.get("type").and_then(Value::as_str),
        Some("command"),
        "only command hooks are executed"
    );
    assert!(
        entry.get("timeout").and_then(Value::as_u64).is_some(),
        "an unbounded hook can hang the agent's tool call"
    );
}
