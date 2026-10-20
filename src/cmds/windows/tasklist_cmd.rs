//! Token-optimized filter for Windows CMD `tasklist`.
//!
//! Keeps the image name, PID and memory usage for the default table. Other
//! formats (`/v`, `/svc`, `/fo CSV`) are passed through unchanged.

use crate::core::runner::{self, RunOptions};
use crate::core::shell::display_args;
use crate::core::windows_shell;
use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

static HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*Image Name\s+PID\s+Session Name\s+Session#\s+Mem Usage\s*$")
        .expect("valid tasklist header regex")
});

static ROW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(.+?)\s+(\d+)\s+\S+\s+\d+\s+([\d,]+\s+K)\s*$")
        .expect("valid tasklist row regex")
});

/// Filters raw `tasklist` output into image, PID and memory columns.
pub fn filter_tasklist_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let lines: Vec<&str> = clean.lines().collect();
    let Some(header_idx) = lines.iter().position(|line| HEADER.is_match(line)) else {
        return clean.trim_end().to_string();
    };

    let mut output = vec!["Image Name PID Mem Usage".to_string()];
    for line in &lines[header_idx + 1..] {
        if line.trim().is_empty() || line.chars().all(|c| c == '=' || c.is_whitespace()) {
            continue;
        }
        if let Some(caps) = ROW.captures(line) {
            output.push(format!(
                "{} {} {}",
                caps[1].trim(),
                &caps[2],
                caps[3].trim()
            ));
        } else {
            output.push(line.trim_end().to_string());
        }
    }

    if output.len() == 1 {
        return "tasklist: no processes".to_string();
    }
    output.join("\n")
}

/// Run `tasklist` via `cmd.exe` and filter its default table.
pub fn run(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_cmd("tasklist", args);
    let cmd_label = format!("tasklist {}", display_args(args));

    runner::run_filtered(
        cmd,
        "tasklist",
        cmd_label.trim(),
        filter_tasklist_output,
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_default_table() {
        let sample = r#"
Image Name                     PID Session Name        Session#    Mem Usage
========================= ======== ================ =========== ============
System Idle Process              0 Services                   0          8 K
System                           4 Services                   0     14,192 K
smss.exe                      1240 Services                   0      1,472 K
"#;
        let filtered = filter_tasklist_output(sample);
        assert!(filtered.starts_with("Image Name PID Mem Usage"));
        assert!(filtered.contains("System Idle Process 0 8 K"));
        assert!(filtered.contains("System 4 14,192 K"));
        assert!(filtered.contains("smss.exe 1240 1,472 K"));
        assert!(!filtered.contains("Session Name"));
    }

    #[test]
    fn test_filter_non_default_passthrough() {
        let sample = r#"
Image Name                     PID Session Name        Session#    Mem Usage Status          User Name
========================= ======== ================ =========== ============== ===============
chrome.exe                    1234 Console                    1    100,000 K Running         DEFIANT\ReRet
"#;
        assert_eq!(filter_tasklist_output(sample), sample.trim_end());
    }

    #[test]
    fn test_filter_empty() {
        assert_eq!(filter_tasklist_output(""), "");
    }

    #[test]
    fn test_filter_header_only() {
        assert_eq!(
            filter_tasklist_output(
                "Image Name                     PID Session Name        Session#    Mem Usage\n"
            ),
            "tasklist: no processes"
        );
    }
}
