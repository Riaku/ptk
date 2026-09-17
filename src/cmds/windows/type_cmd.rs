//! Token-optimized file reader for Windows CMD `type` and PowerShell `Get-Content`.
//!
//! Applies line-count caps, collapses consecutive empty lines, and truncates
//! oversized files with helpful recovery hints so agents don't blow context.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

const MAX_DEFAULT_LINES: usize = 500;

/// Filters file content to reduce token bloat.
pub fn filter_content(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();

    if total == 0 {
        return String::new();
    }

    let mut result = Vec::new();
    let mut consecutive_blank = 0;

    let take = total.min(max_lines);
    for line in &lines[..take] {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            consecutive_blank += 1;
            if consecutive_blank <= 1 {
                result.push("");
            }
        } else {
            consecutive_blank = 0;
            result.push(trimmed);
        }
    }

    let mut output = result.join("\n");
    if total > max_lines {
        output.push_str(&format!(
            "\n... [ptk: truncated {} remaining lines (showing {}/{})] ...",
            total - max_lines,
            max_lines,
            total
        ));
    }
    output
}

/// Run `type` via CMD and filter output.
pub fn run_cmd_type(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_cmd("type", args);
    let cmd_label = format!("type {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "type",
        cmd_label.trim(),
        |raw| filter_content(raw, MAX_DEFAULT_LINES),
        RunOptions::default(),
    )
}

/// Checks if arguments require raw passthrough (e.g. `-Wait` live tailing, `-AsByteStream` binary reading).
pub fn is_get_content_passthrough(args: &[String]) -> bool {
    args.iter().any(|a| {
        let lower = a.to_lowercase();
        lower == "-wait" || lower == "-asbytestream" || lower == "-stream"
    })
}

/// Run `Get-Content` via PowerShell and filter output.
pub fn run_get_content(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Get-Content", args);
    let cmd_label = format!("Get-Content {}", args.join(" "));

    if is_get_content_passthrough(args) {
        return runner::run(
            cmd,
            "Get-Content",
            cmd_label.trim(),
            runner::RunMode::Passthrough,
            RunOptions::default(),
        );
    }

    runner::run_filtered(
        cmd,
        "Get-Content",
        cmd_label.trim(),
        |raw| filter_content(raw, MAX_DEFAULT_LINES),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_content_collapses_empty_lines() {
        let text = "Line 1\n\n\n\nLine 2\n";
        let filtered = filter_content(text, 100);
        assert_eq!(filtered, "Line 1\n\nLine 2");
    }

    #[test]
    fn test_filter_content_truncates_large_files() {
        let lines: Vec<String> = (1..=20).map(|i| format!("Line {}", i)).collect();
        let text = lines.join("\n");
        let filtered = filter_content(&text, 5);
        assert!(filtered.contains("Line 5"));
        assert!(!filtered.contains("Line 6"));
        assert!(filtered.contains("[ptk: truncated 15 remaining lines (showing 5/20)]"));
    }
}
