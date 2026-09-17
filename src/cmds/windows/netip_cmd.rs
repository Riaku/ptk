//! Token-optimized filter for PowerShell `Get-NetIPAddress`.
//!
//! Replaces verbose 14-line-per-IP Format-List boilerplate with a dense,
//! LLM-friendly interface-to-IP routing table.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

#[derive(Default)]
struct IpEntry {
    ip: String,
    alias: String,
    family: String,
    prefix: String,
}

/// Filters raw `Get-NetIPAddress` output into a compact routing table.
pub fn filter_netip_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut entries: Vec<IpEntry> = Vec::new();
    let mut current = IpEntry::default();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.ip.is_empty() {
                entries.push(std::mem::take(&mut current));
            }
            continue;
        }

        if let Some((key, val)) = trimmed.split_once(':') {
            let k = key.trim();
            let v = val.trim();
            match k {
                "IPAddress" => {
                    if !current.ip.is_empty() {
                        entries.push(std::mem::take(&mut current));
                    }
                    current.ip = v.to_string();
                }
                "InterfaceAlias" => current.alias = v.to_string(),
                "AddressFamily" => current.family = v.to_string(),
                "PrefixLength" => current.prefix = v.to_string(),
                _ => {}
            }
        }
    }

    if !current.ip.is_empty() {
        entries.push(current);
    }

    if entries.is_empty() {
        if clean.trim().is_empty() {
            return "(no ip addresses found)".trim_end().to_string();
        }
        return clean.trim().trim_end().to_string();
    }

    let header = format!("{:<28}  {:<6}  {}", "INTERFACE", "FAMILY", "IP/PREFIX").trim_end().to_string();
    let mut output = Vec::with_capacity(entries.len() + 1);
    output.push(header);

    for e in entries {
        let ip_with_prefix = if !e.prefix.is_empty() {
            format!("{}/{}", e.ip, e.prefix)
        } else {
            e.ip
        };
        let line = format!("{:<28}  {:<6}  {}", e.alias, e.family, ip_with_prefix);
        output.push(line.trim_end().to_string());
    }

    output.join("\n").trim_end().to_string()
}

/// Run `Get-NetIPAddress` via PowerShell and filter output.
pub fn run(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Get-NetIPAddress", args);
    let cmd_label = format!("Get-NetIPAddress {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "Get-NetIPAddress",
        cmd_label.trim(),
        |raw| filter_netip_output(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_netip_output() {
        let sample = r#"
IPAddress         : 192.168.1.150
InterfaceIndex    : 12
InterfaceAlias    : Wi-Fi
AddressFamily     : IPv4
Type              : Unicast
PrefixLength      : 24
ValidLifetime     : Infinite

IPAddress         : fe80::1
InterfaceIndex    : 1
InterfaceAlias    : Loopback Pseudo-Interface 1
AddressFamily     : IPv6
PrefixLength      : 128
"#;
        let filtered = filter_netip_output(sample);
        assert!(filtered.contains("INTERFACE"));
        assert!(filtered.contains("Wi-Fi"));
        assert!(filtered.contains("192.168.1.150/24"));
        assert!(filtered.contains("Loopback Pseudo-Interface 1"));
        assert!(!filtered.contains("ValidLifetime"));
    }
}
