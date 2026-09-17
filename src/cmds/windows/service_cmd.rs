//! Token-optimized filter for PowerShell `Get-Service` (`gsv`).
//!
//! Strips wide whitespace, removes table delimiters, and formats
//! service statuses into a clean, compact representation.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

/// Filters raw `Get-Service` output into a compact list.
pub fn filter_service_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut entries = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Strip headers and dash lines
        if trimmed.starts_with("Status")
            && trimmed.contains("DisplayName")
            || trimmed.chars().all(|c| c == '-' || c.is_whitespace())
        {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.iter().all(|p| p.chars().all(|c| c == '-')) {
            continue;
        }

        // Typical format: Status Name DisplayName (3+ parts)
        if parts.len() >= 3 && (parts[0] == "Running" || parts[0] == "Stopped" || parts[0] == "Paused") {
            let status = parts[0];
            let name = parts[1];
            let display_name = parts[2..].join(" ");
            entries.push(format!("{:<7}  {}  {}", status, name, display_name).trim_end().to_string());
        } else {
            entries.push(trimmed.trim_end().to_string());
        }
    }

    if entries.is_empty() {
        return "(no services found)".trim_end().to_string();
    }

    let header = format!("{:<7}  {}  {}", "STATUS", "NAME", "DISPLAY_NAME").trim_end().to_string();
    let mut output = Vec::with_capacity(entries.len() + 1);
    output.push(header);
    for entry in entries {
        output.push(entry.trim_end().to_string());
    }

    output.join("\n").trim_end().to_string()
}

/// Checks if arguments require raw passthrough (e.g. `-DependentServices`, `-RequiredServices`).
pub fn is_service_passthrough(args: &[String]) -> bool {
    args.iter().any(|a| {
        let lower = a.to_lowercase();
        lower == "-dependentservices" || lower == "-requiredservices"
    })
}

/// Run `Get-Service` via PowerShell and filter output.
pub fn run(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Get-Service", args);
    let cmd_label = format!("Get-Service {}", args.join(" "));

    if is_service_passthrough(args) {
        return runner::run(
            cmd,
            "Get-Service",
            cmd_label.trim(),
            runner::RunMode::Passthrough,
            RunOptions::default(),
        );
    }

    runner::run_filtered(
        cmd,
        "Get-Service",
        cmd_label.trim(),
        |raw| filter_service_output(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_service_output() {
        let sample = r#"
Status   Name               DisplayName
------   ----               -----------
Stopped  AarSvc_85c06b      Agent Activation Runtime_85c06b
Running  docker             Docker Desktop Service
"#;
        let filtered = filter_service_output(sample);
        assert!(filtered.contains("STATUS"));
        assert!(filtered.contains("Running"));
        assert!(filtered.contains("docker"));
        assert!(filtered.contains("Docker Desktop Service"));
        assert!(!filtered.contains("------"));
    }
}
