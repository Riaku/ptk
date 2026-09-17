//! Token-optimized filter for PowerShell `Get-NetTCPConnection` and `netstat`.
//!
//! Compresses wide whitespace, standardizes columns, and caps unbounded
//! connection dumps to save tokens.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

const MAX_DEFAULT_CONNECTIONS: usize = 100;

/// Filters raw `Get-NetTCPConnection` output into a dense connection table.
pub fn filter_nettcp_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut entries = Vec::new();

    // Check if output is Format-List style (LocalAddress : ...)
    if clean.lines().any(|l| l.trim().starts_with("LocalAddress") && l.contains(':')) {
        let mut local_addr = String::new();
        let mut local_port = String::new();
        let mut remote_addr = String::new();
        let mut remote_port = String::new();
        let mut state = String::new();
        let mut pid = String::new();

        for line in clean.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !local_addr.is_empty() {
                    let local = format!("{}:{}", local_addr, local_port);
                    let remote = format!("{}:{}", remote_addr, remote_port);
                    entries.push(
                        format!("{:<28}  {:<28}  {:<12}  {:<6}", local, remote, state, pid)
                            .trim_end()
                            .to_string(),
                    );
                    local_addr.clear();
                    local_port.clear();
                    remote_addr.clear();
                    remote_port.clear();
                    state.clear();
                    pid.clear();
                }
                continue;
            }

            if let Some((k, v)) = trimmed.split_once(':') {
                match k.trim() {
                    "LocalAddress" => local_addr = v.trim().to_string(),
                    "LocalPort" => local_port = v.trim().to_string(),
                    "RemoteAddress" => remote_addr = v.trim().to_string(),
                    "RemotePort" => remote_port = v.trim().to_string(),
                    "State" => state = v.trim().to_string(),
                    "OwningProcess" => pid = v.trim().to_string(),
                    _ => {}
                }
            }
        }

        if !local_addr.is_empty() {
            let local = format!("{}:{}", local_addr, local_port);
            let remote = format!("{}:{}", remote_addr, remote_port);
            entries.push(
                format!("{:<28}  {:<28}  {:<12}  {:<6}", local, remote, state, pid)
                    .trim_end()
                    .to_string(),
            );
        }
    } else {
        // Table format
        for line in clean.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("LocalAddress")
                || trimmed.chars().all(|c| c == '-' || c.is_whitespace())
            {
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            // LocalAddress LocalPort RemoteAddress RemotePort State AppliedSetting ...
            if parts.len() >= 5 {
                let local = format!("{}:{}", parts[0], parts[1]);
                let remote = format!("{}:{}", parts[2], parts[3]);
                let state = parts[4];
                let pid = if parts.len() >= 6 && parts[parts.len() - 1].chars().all(|c| c.is_ascii_digit()) {
                    parts[parts.len() - 1]
                } else {
                    "-"
                };
                entries.push(
                    format!("{:<28}  {:<28}  {:<12}  {:<6}", local, remote, state, pid)
                        .trim_end()
                        .to_string(),
                );
            }
        }
    }

    if entries.is_empty() {
        if clean.trim().is_empty() {
            return "(no connections found)".trim_end().to_string();
        }
        return clean.trim().trim_end().to_string();
    }

    let total = entries.len();
    let take = total.min(MAX_DEFAULT_CONNECTIONS);

    let header = format!("{:<28}  {:<28}  {:<12}  {:<6}", "LOCAL_ADDRESS", "REMOTE_ADDRESS", "STATE", "PID")
        .trim_end()
        .to_string();
    let mut output = Vec::with_capacity(take + 2);
    output.push(header);

    for entry in &entries[..take] {
        output.push(entry.trim_end().to_string());
    }

    if total > MAX_DEFAULT_CONNECTIONS {
        output.push(
            format!(
                "... [ptk: showing {}/{} connections. Filter with 'Get-NetTCPConnection -State Established'] ...",
                take, total
            )
            .trim_end()
            .to_string(),
        );
    }

    output.join("\n").trim_end().to_string()
}

/// Filters raw `netstat` (e.g. `netstat -ano`) output into a dense connection table.
pub fn filter_netstat_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut entries = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("Active Connections")
            || trimmed.starts_with("Proto")
            || trimmed.chars().all(|c| c == '-' || c.is_whitespace())
        {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        // Proto Local_Address Foreign_Address State [PID]
        if parts.len() >= 4 {
            let proto = parts[0];
            let local = parts[1];
            let remote = parts[2];
            let (state, pid) = if parts.len() >= 5 {
                (parts[3], parts[4])
            } else {
                (parts[3], "-")
            };
            entries.push(
                format!("{:<5}  {:<28}  {:<28}  {:<12}  {:<6}", proto, local, remote, state, pid)
                    .trim_end()
                    .to_string(),
            );
        }
    }

    if entries.is_empty() {
        if clean.trim().is_empty() {
            return "(no connections found)".trim_end().to_string();
        }
        return clean.trim().trim_end().to_string();
    }

    let total = entries.len();
    let take = total.min(MAX_DEFAULT_CONNECTIONS);

    let header = format!("{:<5}  {:<28}  {:<28}  {:<12}  {:<6}", "PROTO", "LOCAL_ADDRESS", "REMOTE_ADDRESS", "STATE", "PID")
        .trim_end()
        .to_string();
    let mut output = Vec::with_capacity(take + 2);
    output.push(header);

    for entry in &entries[..take] {
        output.push(entry.trim_end().to_string());
    }

    if total > MAX_DEFAULT_CONNECTIONS {
        output.push(
            format!(
                "... [ptk: showing {}/{} connections] ...",
                take, total
            )
            .trim_end()
            .to_string(),
        );
    }

    output.join("\n").trim_end().to_string()
}

/// Run `Get-NetTCPConnection` via PowerShell and filter output.
pub fn run_nettcp(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Get-NetTCPConnection", args);
    let cmd_label = format!("Get-NetTCPConnection {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "Get-NetTCPConnection",
        cmd_label.trim(),
        |raw| filter_nettcp_output(raw),
        RunOptions::default(),
    )
}

/// Run `netstat` and filter output.
pub fn run_netstat(args: &[String], _verbose: u8) -> Result<i32> {
    let mut cmd = std::process::Command::new("netstat");
    for arg in args {
        cmd.arg(arg);
    }
    let cmd_label = format!("netstat {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "netstat",
        cmd_label.trim(),
        |raw| filter_netstat_output(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_nettcp_output_table() {
        let sample = r#"
LocalAddress                        LocalPort RemoteAddress                       RemotePort State       AppliedSetting
------------                        --------- -------------                       ---------- -----       --------------
127.0.0.1                           54881     0.0.0.0                             0          Listen                    
192.168.1.50                        443       142.250.190.46                      52311      Established
"#;
        let filtered = filter_nettcp_output(sample);
        assert!(filtered.contains("LOCAL_ADDRESS"));
        assert!(filtered.contains("127.0.0.1:54881"));
        assert!(filtered.contains("Established"));
        assert!(!filtered.contains("------------"));
    }

    #[test]
    fn test_filter_netstat_output() {
        let sample = r#"
Active Connections

  Proto  Local Address          Foreign Address        State           PID
  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       2388
  TCP    192.168.1.50:52311     142.250.190.46:443     ESTABLISHED     1234
"#;
        let filtered = filter_netstat_output(sample);
        assert!(filtered.contains("PROTO"));
        assert!(filtered.contains("LISTENING"));
        assert!(filtered.contains("2388"));
        assert!(!filtered.contains("Active Connections"));
    }
}
