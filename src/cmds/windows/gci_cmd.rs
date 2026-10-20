//! Token-optimized filter for PowerShell `Get-ChildItem` / `gci`.
//!
//! Strips PowerShell table headers, column separation dashes, and wide spacing,
//! producing a dense, token-saving list of directory entries.

use crate::core::runner::{self, RunOptions};
use crate::core::shell::display_args;
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

        if parts.len() >= 2 {
            let mode = parts[0];
            // The name is always the last token; the size, when there is one, is
            // the token right before it. Windows prints
            // `Mode LastWriteTime Length Name` and Linux prints
            // `UnixMode User Group LastWriteTime Size Name`, so counting columns
            // from the front breaks across the two — reading from the back is
            // the same question on both.
            let name = parts[parts.len() - 1];
            let before_name = parts[parts.len() - 2];

            if mode.starts_with('d') {
                current_entries.push(format!("{}/", name));
            } else if let Ok(bytes) = before_name.parse::<u64>() {
                current_entries.push(format!("{} ({})", name, format_size(bytes)));
            } else {
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
        let trimmed_entries: Vec<String> = entries
            .into_iter()
            .map(|e| e.trim_end().to_string())
            .collect();
        if let Some(d) = dir {
            formatted_sections.push(format!(
                "{}:\n  {}",
                d.trim_end(),
                trimmed_entries.join("\n  ")
            ));
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
    // Off Windows the default Get-ChildItem rendering wraps the Name column,
    // so a long filename arrives as two lines and the tail becomes a stray
    // entry. Widening the output keeps each name on its own line while leaving
    // the per-directory `Directory:` headers that the filter groups on.
    // (Format-Table with an explicit column list would fix the layout too, but
    // it drops those headers, so the recursion loses its section structure.)
    let cmd = if cfg!(windows) {
        windows_shell::spawn_powershell_args("Get-ChildItem", args)
    } else {
        windows_shell::spawn_powershell_args_piped("Get-ChildItem", args, "Out-String -Width 512")
    };
    let cmd_label = format!("Get-ChildItem {}", display_args(args));

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
        filter_gci_output,
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
