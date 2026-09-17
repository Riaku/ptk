//! Token-optimized filter for PowerShell `New-Item` (`ni`).
//!
//! Replaces full multi-line table headers with a concise creation confirmation.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

/// Filters raw `New-Item` table output into a compact creation notice.
pub fn filter_new_item_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut directory = String::new();
    let mut created_item: Option<String> = None;

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Directory: ") {
            directory = rest.trim().to_string();
            continue;
        }

        if trimmed.starts_with("Mode ")
            || trimmed.contains("LastWriteTime")
            || trimmed.chars().all(|c| c == '-' || c.is_whitespace())
        {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.iter().all(|p| p.chars().all(|c| c == '-')) {
            continue;
        }

        if parts.len() >= 4 {
            let mode = parts[0];
            let is_dir = mode.starts_with('d');
            let name = parts[parts.len() - 1];

            let path = if !directory.is_empty() {
                format!("{}\\{}", directory, name)
            } else {
                name.to_string()
            };

            if is_dir {
                created_item = Some(format!("+ created directory: {}/", path));
            } else {
                created_item = Some(format!("+ created file: {}", path));
            }
            break;
        }
    }

    if let Some(item) = created_item {
        item.trim_end().to_string()
    } else {
        clean.trim().trim_end().to_string()
    }
}

/// Run `New-Item` via PowerShell and filter output.
pub fn run(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("New-Item", args);
    let cmd_label = format!("New-Item {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "New-Item",
        cmd_label.trim(),
        |raw| filter_new_item_output(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_new_item_output() {
        let sample = r#"
    Directory: C:\source\test

Mode                 LastWriteTime         Length Name
----                 -------------         ------ ----
-a---          2026-09-16  9:53 PM              0 sample.txt
"#;
        let filtered = filter_new_item_output(sample);
        assert_eq!(filtered, "+ created file: C:\\source\\test\\sample.txt");
    }
}
