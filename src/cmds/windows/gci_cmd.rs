//! Token-optimized filter for PowerShell `Get-ChildItem` / `gci`.
//!
//! Strips PowerShell table headers, column separation dashes, and wide spacing,
//! producing a dense, token-saving list of directory entries.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

/// Filter raw PowerShell `Get-ChildItem` table output into a compact list.
pub fn filter_gci_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut sections: Vec<(Option<String>, Vec<String>)> = Vec::new();
    let mut current_dir: Option<String> = None;
    let mut current_entries: Vec<String> = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Header: "Directory: C:\path"
        if let Some(rest) = trimmed.strip_prefix("Directory: ") {
            if !current_entries.is_empty() || current_dir.is_some() {
                sections.push((current_dir.take(), std::mem::take(&mut current_entries)));
            }
            current_dir = Some(rest.trim().to_string());
            continue;
        }

        // Table headers and separators
        if trimmed.starts_with("Mode ")
            || trimmed.contains("LastWriteTime")
            || trimmed.chars().all(|c| c == '-' || c.is_whitespace())
        {
            continue;
        }

        // Typical PowerShell Format-Table line:
        // d----          2026-04-26  1:23 PM                node_modules
        // -a---          2026-09-16  8:49 PM           2512 Cargo.toml
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.iter().all(|p| p.chars().all(|c| c == '-')) {
            continue;
        }

        if parts.len() >= 4 {
            let mode = parts[0];
            let is_dir = mode.starts_with('d');

            if is_dir {
                // Directory: last element is the name
                let name = parts[parts.len() - 1];
                current_entries.push(format!("{}/", name));
            } else if parts.len() >= 5 {
                // File: parts are [Mode, Date, Time, AM/PM, Length, ...Name]
                // or without AM/PM: [Mode, Date, Time, Length, ...Name]
                let (len_idx, name_idx) = if parts[3].eq_ignore_ascii_case("am")
                    || parts[3].eq_ignore_ascii_case("pm")
                {
                    (4, 5)
                } else {
                    (3, 4)
                };

                if len_idx < parts.len() && name_idx <= parts.len() {
                    let len_str = parts[len_idx];
                    let name = parts[name_idx..].join(" ");
                    if let Ok(bytes) = len_str.parse::<u64>() {
                        current_entries.push(format!("{} ({})", name, format_size(bytes)));
                    } else {
                        current_entries.push(format!("{} ({})", name, len_str));
                    }
                } else {
                    let name = parts[parts.len() - 1];
                    current_entries.push(name.to_string());
                }
            } else {
                let name = parts[parts.len() - 1];
                current_entries.push(name.to_string());
            }
        } else {
            current_entries.push(trimmed.to_string());
        }
    }

    if !current_entries.is_empty() || current_dir.is_some() {
        sections.push((current_dir, current_entries));
    }

    if sections.is_empty() || sections.iter().all(|(_, entries)| entries.is_empty()) {
        if clean.trim().is_empty() {
            return "(empty directory)".trim_end().to_string();
        }
        // Fallback: if output wasn't a recognized table, return clean output unchanged
        return clean.trim().trim_end().to_string();
    }

    let mut formatted_sections = Vec::new();
    for (dir, entries) in sections {
        if entries.is_empty() {
            continue;
        }
        let trimmed_entries: Vec<String> = entries.into_iter().map(|e| e.trim_end().to_string()).collect();
        if let Some(d) = dir {
            formatted_sections.push(format!("{}:\n  {}", d.trim_end(), trimmed_entries.join("\n  ")));
        } else {
            formatted_sections.push(trimmed_entries.join("\n  "));
        }
    }

    formatted_sections.join("\n\n").trim_end().to_string()
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{}KB", bytes / KB)
    } else {
        format!("{}B", bytes)
    }
}

/// Checks if arguments require passthrough (e.g. `-Name` / `-n` which returns bare strings).
pub fn is_gci_passthrough(args: &[String]) -> bool {
    args.iter().any(|a| {
        let lower = a.to_lowercase();
        lower == "-name" || lower == "-n"
    })
}

/// Run `Get-ChildItem` via PowerShell and filter output.
pub fn run(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Get-ChildItem", args);
    let cmd_label = format!("Get-ChildItem {}", args.join(" "));

    if is_gci_passthrough(args) {
        return runner::run(
            cmd,
            "Get-ChildItem",
            cmd_label.trim(),
            runner::RunMode::Passthrough,
            RunOptions::default(),
        );
    }

    runner::run_filtered(
        cmd,
        "Get-ChildItem",
        cmd_label.trim(),
        |raw| filter_gci_output(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_gci_output() {
        let sample = r#"
    Directory: C:\test\project

Mode                 LastWriteTime         Length Name
----                 -------------         ------ ----
d----          2026-09-16  8:49 PM                src
-a---          2026-09-16  8:49 PM           2512 Cargo.toml
-a---          2026-09-16  8:49 PM          26527 README.md
"#;
        let filtered = filter_gci_output(sample);
        assert!(filtered.contains("C:\\test\\project:"));
        assert!(filtered.contains("src/"));
        assert!(filtered.contains("Cargo.toml (2KB)"));
        assert!(filtered.contains("README.md (25KB)"));
        assert!(!filtered.contains("Mode"));
        assert!(!filtered.contains("LastWriteTime"));
    }
}
