//! Token-optimized filters for PowerShell formatting cmdlets:
//! - `Format-Table` (`ft`)
//! - `Format-List` (`fl`)

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

/// Filters `Format-Table` output by removing dashed lines and compressing multi-column whitespace.
pub fn filter_format_table(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut lines = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Drop dashes
        if trimmed.chars().all(|c| c == '-' || c.is_whitespace()) {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        lines.push(parts.join("  ").trim_end().to_string());
    }

    if lines.is_empty() {
        if clean.trim().is_empty() {
            return String::new();
        }
        return clean.trim().trim_end().to_string();
    }

    lines.join("\n").trim_end().to_string()
}

/// Filters `Format-List` output by compressing `Key : Value` spacing and collapsing excess blank lines.
pub fn filter_format_list(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut lines = Vec::new();
    let mut last_was_empty = false;

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !last_was_empty && !lines.is_empty() {
                lines.push(String::new());
                last_was_empty = true;
            }
            continue;
        }
        last_was_empty = false;

        if let Some((k, v)) = trimmed.split_once(':') {
            lines.push(format!("{}: {}", k.trim(), v.trim()).trim_end().to_string());
        } else {
            lines.push(trimmed.trim_end().to_string());
        }
    }

    // Trim trailing empty line if any
    while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        lines.pop();
    }

    if lines.is_empty() {
        if clean.trim().is_empty() {
            return String::new();
        }
        return clean.trim().trim_end().to_string();
    }

    lines.join("\n").trim_end().to_string()
}

/// Run `Format-Table` via PowerShell and filter output.
pub fn run_format_table(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Format-Table", args);
    let cmd_label = format!("Format-Table {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "Format-Table",
        cmd_label.trim(),
        |raw| filter_format_table(raw),
        RunOptions::default(),
    )
}

/// Run `Format-List` via PowerShell and filter output.
pub fn run_format_list(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Format-List", args);
    let cmd_label = format!("Format-List {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "Format-List",
        cmd_label.trim(),
        |raw| filter_format_list(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_format_table() {
        let sample = r#"
Name                           Id PriorityClass
----                           -- -------------
cmd                         12345        Normal
powershell                  67890        Normal
"#;
        let filtered = filter_format_table(sample);
        assert_eq!(filtered, "Name  Id  PriorityClass\ncmd  12345  Normal\npowershell  67890  Normal");
    }

    #[test]
    fn test_filter_format_list() {
        let sample = r#"
Id            : 12345
ProcessName   : cmd


Id            : 67890
ProcessName   : powershell
"#;
        let filtered = filter_format_list(sample);
        assert_eq!(filtered, "Id: 12345\nProcessName: cmd\n\nId: 67890\nProcessName: powershell");
    }
}
