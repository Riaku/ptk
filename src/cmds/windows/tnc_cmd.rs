//! Token-optimized filter for PowerShell `Test-NetConnection` (`tnc`).
//!
//! Compresses verbose multi-line connection tests into a concise, 1-2 line verdict:
//! `[TCP OK] <Host>:<Port> (via <Interface>, IP <IP>)`
//! or `[PING OK] <Host> (RTT: <ms>)`
//! or `[FAIL] <Host>:<Port> -> TcpTestFailed`.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

/// Filters raw `Test-NetConnection` output into a concise verdict.
pub fn filter_tnc_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut computer_name = String::new();
    let mut remote_addr = String::new();
    let mut remote_port = String::new();
    let mut interface = String::new();
    let mut ping_succeeded = String::new();
    let mut ping_rtt = String::new();
    let mut tcp_succeeded = String::new();
    let mut warning_line = String::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("WARNING:") {
            warning_line = trimmed.to_string();
            continue;
        }

        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "ComputerName" => computer_name = val.to_string(),
                "RemoteAddress" => remote_addr = val.to_string(),
                "RemotePort" => remote_port = val.to_string(),
                "InterfaceAlias" => interface = val.to_string(),
                "PingSucceeded" => ping_succeeded = val.to_string(),
                "PingReplyDetails (RTT)" | "RoundTripTime" => ping_rtt = val.to_string(),
                "TcpTestSucceeded" => tcp_succeeded = val.to_string(),
                _ => {}
            }
        }
    }

    let host = if !computer_name.is_empty() {
        computer_name
    } else {
        remote_addr.clone()
    };

    // If TCP test was conducted
    if !remote_port.is_empty() {
        let is_ok = tcp_succeeded.eq_ignore_ascii_case("true");
        if is_ok {
            let mut details = Vec::new();
            if !remote_addr.is_empty() && remote_addr != host {
                details.push(format!("IP {}", remote_addr));
            }
            if !interface.is_empty() {
                details.push(format!("via {}", interface));
            }
            let detail_str = if details.is_empty() {
                String::new()
            } else {
                format!(" ({})", details.join(", "))
            };
            return format!("[TCP OK] {}:{}{}", host, remote_port, detail_str)
                .trim_end()
                .to_string();
        } else {
            let mut msg = format!("[TCP FAIL] {}:{}", host, remote_port);
            if !warning_line.is_empty() {
                msg.push_str(&format!(" -> {}", warning_line));
            }
            return msg.trim_end().to_string();
        }
    }

    // Ping test
    if !ping_succeeded.is_empty() {
        let is_ok = ping_succeeded.eq_ignore_ascii_case("true");
        if is_ok {
            let rtt = if !ping_rtt.is_empty() {
                format!(" (RTT: {})", ping_rtt)
            } else {
                String::new()
            };
            let ip_str = if !remote_addr.is_empty() && remote_addr != host {
                format!(" [{}]", remote_addr)
            } else {
                String::new()
            };
            return format!("[PING OK] {}{}{}", host, ip_str, rtt)
                .trim_end()
                .to_string();
        } else {
            return format!("[PING FAIL] {}", host).trim_end().to_string();
        }
    }

    // Fallback if parsing didn't match standard keys
    if clean.trim().is_empty() {
        "(no network test results)".trim_end().to_string()
    } else {
        clean.lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n").trim_end().to_string()
    }
}

/// Run `Test-NetConnection` via PowerShell and filter output.
pub fn run(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Test-NetConnection", args);
    let cmd_label = format!("Test-NetConnection {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "Test-NetConnection",
        cmd_label.trim(),
        |raw| filter_tnc_output(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_tnc_tcp_success() {
        let sample = r#"
ComputerName     : google.com
RemoteAddress    : 172.253.155.138
RemotePort       : 443
InterfaceAlias   : ProtonVPN
SourceAddress    : 10.2.0.2
TcpTestSucceeded : True
"#;
        let filtered = filter_tnc_output(sample);
        assert_eq!(
            filtered,
            "[TCP OK] google.com:443 (IP 172.253.155.138, via ProtonVPN)"
        );
    }

    #[test]
    fn test_filter_tnc_ping_success() {
        let sample = r#"
ComputerName           : google.com
RemoteAddress          : 172.253.155.138
InterfaceAlias         : ProtonVPN
SourceAddress          : 10.2.0.2
PingSucceeded          : True
PingReplyDetails (RTT) : 19 ms
"#;
        let filtered = filter_tnc_output(sample);
        assert_eq!(
            filtered,
            "[PING OK] google.com [172.253.155.138] (RTT: 19 ms)"
        );
    }

    #[test]
    fn test_filter_tnc_tcp_fail() {
        let sample = r#"
WARNING: TCP connect to (127.0.0.1 : 80) failed
ComputerName     : 127.0.0.1
RemoteAddress    : 127.0.0.1
RemotePort       : 80
TcpTestSucceeded : False
"#;
        let filtered = filter_tnc_output(sample);
        assert!(filtered.contains("[TCP FAIL] 127.0.0.1:80"));
        assert!(filtered.contains("WARNING: TCP connect to (127.0.0.1 : 80) failed"));
    }
}
