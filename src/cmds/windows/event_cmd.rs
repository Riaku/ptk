//! Token-optimized filters for PowerShell `Get-EventLog` and `Get-WinEvent`.
//!
//! Compresses event logs into compact timestamped event lines and strips
//! wide table column margins.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

/// Filters raw `Get-EventLog` output.
pub fn filter_eventlog_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut lines = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Table headers and separators
        if trimmed.starts_with("Index ")
            && trimmed.contains("InstanceID")
            || trimmed.chars().all(|c| c == '-' || c.is_whitespace())
        {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.iter().all(|p| p.chars().all(|c| c == '-')) {
            continue;
        }

        // Standard Get-EventLog row:
        // Index Time(Month Day Time) EntryType Source InstanceID Message...
        // 117676 Sep 16 21:50 Information Microsoft-Windows-H... 71 Delete complete...
        if parts.len() >= 7 && (parts[4] == "Information" || parts[4] == "Warning" || parts[4] == "Error") {
            let time_str = format!("{} {} {}", parts[1], parts[2], parts[3]);
            let entry_type = parts[4];
            let source = parts[5];
            let id = parts[6];
            let msg = parts[7..].join(" ");
            lines.push(format!("[{}] {} {} (ID {}): {}", time_str, entry_type, source, id, msg).trim_end().to_string());
        } else {
            // Compress multiple whitespace gaps into 2 spaces
            let mut compressed = String::new();
            let mut prev_space = false;
            for c in trimmed.chars() {
                if c.is_whitespace() {
                    if !prev_space {
                        compressed.push(' ');
                        prev_space = true;
                    }
                } else {
                    compressed.push(c);
                    prev_space = false;
                }
            }
            lines.push(compressed.trim_end().to_string());
        }
    }

    if lines.is_empty() {
        return "(no events found)".trim_end().to_string();
    }

    lines.join("\n").trim_end().to_string()
}

/// Filters raw `Get-WinEvent` output.
pub fn filter_winevent_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut lines = Vec::new();
    let mut current_provider = String::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("ProviderName: ") {
            current_provider = rest.trim().to_string();
            continue;
        }

        if trimmed.starts_with("TimeCreated")
            && trimmed.contains("LevelDisplayName")
            || trimmed.chars().all(|c| c == '-' || c.is_whitespace())
        {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.iter().all(|p| p.chars().all(|c| c == '-')) {
            continue;
        }

        // Format: TimeCreated(Date Time AM/PM) Id LevelDisplayName Message...
        // 2026-09-16 9:50:07 PM 71 Information Delete complete for port...
        if parts.len() >= 6 && (parts[2].eq_ignore_ascii_case("am") || parts[2].eq_ignore_ascii_case("pm")) {
            let time_str = format!("{} {} {}", parts[0], parts[1], parts[2]);
            let id = parts[3];
            let level = parts[4];
            let msg = parts[5..].join(" ");
            let provider_prefix = if !current_provider.is_empty() {
                format!("{} ", current_provider)
            } else {
                String::new()
            };
            lines.push(format!("[{}] {} {}(ID {}): {}", time_str, level, provider_prefix, id, msg).trim_end().to_string());
        } else {
            lines.push(trimmed.trim_end().to_string());
        }
    }

    if lines.is_empty() {
        return "(no events found)".trim_end().to_string();
    }

    lines.join("\n").trim_end().to_string()
}

/// Run `Get-EventLog` via PowerShell and filter output.
pub fn run_eventlog(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Get-EventLog", args);
    let cmd_label = format!("Get-EventLog {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "Get-EventLog",
        cmd_label.trim(),
        |raw| filter_eventlog_output(raw),
        RunOptions::default(),
    )
}

/// Run `Get-WinEvent` via PowerShell and filter output.
pub fn run_winevent(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Get-WinEvent", args);
    let cmd_label = format!("Get-WinEvent {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "Get-WinEvent",
        cmd_label.trim(),
        |raw| filter_winevent_output(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_eventlog_output() {
        let sample = r#"
   Index Time          EntryType   Source                 InstanceID Message
   ----- ----          ---------   ------                 ---------- -------
  117676 Sep 16 21:50  Information Microsoft-Windows-Test         71 Delete complete
"#;
        let filtered = filter_eventlog_output(sample);
        assert!(filtered.contains("[Sep 16 21:50] Information Microsoft-Windows-Test (ID 71): Delete complete"));
    }

    #[test]
    fn test_filter_winevent_output() {
        let sample = r#"
   ProviderName: Microsoft-Windows-VmSwitch

TimeCreated                     Id LevelDisplayName Message
-----------                     -- ---------------- -------
2026-09-16 9:50:07 PM           71 Information      Delete complete
"#;
        let filtered = filter_winevent_output(sample);
        assert!(filtered.contains("[2026-09-16 9:50:07 PM] Information Microsoft-Windows-VmSwitch (ID 71): Delete complete"));
    }
}
