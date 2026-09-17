//! Token-optimized filter for Windows `findstr` and PowerShell `Select-String`.
//!
//! Groups matches by filename instead of repeating the full file path
//! on every single matching line, saving significant token overhead.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;
use std::collections::BTreeMap;

/// Groups `file:line:content` or `file:content` matches by file.
pub fn filter_search_output(raw: &str) -> String {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut non_matching = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // findstr format: path\to\file.ext:line:content OR path\to\file.ext:content
        // Note: On Windows paths can start with drive letter e.g. C:\...
        let split_pos = if trimmed.len() > 2 && trimmed.as_bytes()[1] == b':' {
            // Skip drive letter colon
            trimmed[2..].find(':').map(|idx| idx + 2)
        } else {
            trimmed.find(':')
        };

        if let Some(pos) = split_pos {
            let file = trimmed[..pos].trim();
            let match_content = trimmed[pos + 1..].trim();
            grouped
                .entry(file.to_string())
                .or_default()
                .push(match_content.to_string());
        } else {
            non_matching.push(trimmed.to_string());
        }
    }

    if grouped.is_empty() && non_matching.is_empty() {
        return "(no matches found)".trim_end().to_string();
    }

    let mut output = Vec::new();
    for (file, matches) in grouped {
        output.push(format!("{}:", file).trim_end().to_string());
        for m in matches {
            output.push(format!("  {}", m).trim_end().to_string());
        }
    }

    for line in non_matching {
        output.push(line.trim_end().to_string());
    }

    output.join("\n").trim_end().to_string()
}

/// Run `findstr` via CMD and filter output.
pub fn run_findstr(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_cmd("findstr", args);
    let cmd_label = format!("findstr {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "findstr",
        cmd_label.trim(),
        |raw| filter_search_output(raw),
        RunOptions::default(),
    )
}

/// Checks if arguments require raw passthrough (e.g. `-Quiet` boolean check, `-Raw` string output).
pub fn is_select_string_passthrough(args: &[String]) -> bool {
    args.iter().any(|a| {
        let lower = a.to_lowercase();
        lower == "-quiet" || lower == "-q" || lower == "-raw"
    })
}

/// Run `Select-String` via PowerShell and filter output.
pub fn run_select_string(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Select-String", args);
    let cmd_label = format!("Select-String {}", args.join(" "));

    if is_select_string_passthrough(args) {
        return runner::run(
            cmd,
            "Select-String",
            cmd_label.trim(),
            runner::RunMode::Passthrough,
            RunOptions::default(),
        );
    }

    runner::run_filtered(
        cmd,
        "Select-String",
        cmd_label.trim(),
        |raw| filter_search_output(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_search_output_grouping() {
        let sample = "src\\main.rs:10:fn main() {\nsrc\\main.rs:25:println!(\"hi\");\nsrc\\lib.rs:5:pub fn test() {}\n";
        let filtered = filter_search_output(sample);
        assert!(filtered.contains("src\\main.rs:"));
        assert!(filtered.contains("  10:fn main() {"));
        assert!(filtered.contains("  25:println!(\"hi\");"));
        assert!(filtered.contains("src\\lib.rs:"));
        assert!(filtered.contains("  5:pub fn test() {}"));
    }
}
