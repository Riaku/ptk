//! Token-optimized filter for PowerShell `Get-Process` (`gps`, `ps`).
//!
//! Compresses wide whitespace, strips table separators, and formats
//! process lists into a dense, token-saving representation.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

const MAX_DEFAULT_PROCESSES: usize = 80;

/// Filters raw `Get-Process` output into a compact list.
pub fn filter_process_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut entries = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Strip headers and dash separators
        if trimmed.starts_with("NPM(K)")
            || trimmed.contains("ProcessName")
            || trimmed.chars().all(|c| c == '-' || c.is_whitespace())
        {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.iter().all(|p| p.chars().all(|c| c == '-')) {
            continue;
        }

        // Standard pwsh format:
        // NPM(K) PM(M) WS(M) CPU(s) Id SI ProcessName (7+ parts)
        // Or if CPU is empty: NPM(K) PM(M) WS(M) Id SI ProcessName (6+ parts)
        if parts.len() >= 6 {
            let (pid, mem, cpu, name) = if parts.len() >= 7 {
                let pid = parts[parts.len() - 3];
                let name = parts[parts.len() - 1];
                let cpu = parts[parts.len() - 4];
                let mem = parts[parts.len() - 5];
                (pid, mem, cpu, name)
            } else {
                let pid = parts[parts.len() - 2];
                let name = parts[parts.len() - 1];
                let mem = parts[parts.len() - 3];
                (pid, mem, "-", name)
            };

            entries.push(format!("{:>7}  {:>7}  {:>7}  {}", pid, mem, cpu, name).trim_end().to_string());
        } else {
            entries.push(trimmed.trim_end().to_string());
        }
    }

    if entries.is_empty() {
        return "(no processes found)".trim_end().to_string();
    }

    let total = entries.len();
    let take = total.min(MAX_DEFAULT_PROCESSES);

    let header = format!("{:>7}  {:>7}  {:>7}  {}", "PID", "MEM(M)", "CPU(s)", "NAME").trim_end().to_string();
    let mut output = Vec::with_capacity(take + 2);
    output.push(header);

    for entry in &entries[..take] {
        output.push(entry.trim_end().to_string());
    }

    if total > MAX_DEFAULT_PROCESSES {
        output.push(
            format!(
                "... [ptk: showing {}/{} processes. Filter with 'gps <name>'] ...",
                take, total
            )
            .trim_end()
            .to_string(),
        );
    }

    output.join("\n").trim_end().to_string()
}

/// Checks if arguments require raw passthrough (e.g. `-FileVersionInfo`, `-Module`).
pub fn is_process_passthrough(args: &[String]) -> bool {
    args.iter().any(|a| {
        let lower = a.to_lowercase();
        lower == "-fileversioninfo" || lower == "-module"
    })
}

/// Run `Get-Process` via PowerShell and filter output.
pub fn run(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Get-Process", args);
    let cmd_label = format!("Get-Process {}", args.join(" "));

    if is_process_passthrough(args) {
        return runner::run(
            cmd,
            "Get-Process",
            cmd_label.trim(),
            runner::RunMode::Passthrough,
            RunOptions::default(),
        );
    }

    runner::run_filtered(
        cmd,
        "Get-Process",
        cmd_label.trim(),
        |raw| filter_process_output(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_process_output() {
        let sample = r#"
 NPM(K)    PM(M)      WS(M)     CPU(s)      Id  SI ProcessName
 ------    -----      -----     ------      --  -- -----------
     69    87.34      52.56     389.33   26820   1 1Password
     17     7.68       6.84       5.27   29316   1 1Password-BrowserSupport
"#;
        let filtered = filter_process_output(sample);
        assert!(filtered.contains("PID"));
        assert!(filtered.contains("26820"));
        assert!(filtered.contains("1Password"));
        assert!(!filtered.contains("NPM(K)"));
        assert!(!filtered.contains("------"));
    }
}
