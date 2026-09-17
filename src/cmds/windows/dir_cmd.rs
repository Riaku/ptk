//! Token-optimized filter for Windows CMD `dir` command.
//!
//! Strips Windows volume serial headers, directory metadata headers,
//! and footer byte counts, formatting output into a dense, LLM-friendly list.

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

/// Filter raw `dir` output into a token-optimized representation.
pub fn filter_dir_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut sections: Vec<(Option<String>, Vec<String>)> = Vec::new();
    let mut current_dir: Option<String> = None;
    let mut current_entries: Vec<String> = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Strip volume headers
        if trimmed.starts_with("Volume in drive") || trimmed.starts_with("Volume Serial Number") {
            continue;
        }

        // Track "Directory of ..." header
        if let Some(rest) = trimmed.strip_prefix("Directory of ") {
            if !current_entries.is_empty() || current_dir.is_some() {
                sections.push((current_dir.take(), std::mem::take(&mut current_entries)));
            }
            current_dir = Some(rest.trim().to_string());
            continue;
        }

        // Strip summary footer lines (e.g. "3 File(s) 79,514 bytes", "3 Dir(s) 120,456 bytes free")
        if trimmed.ends_with("bytes free") || trimmed.contains("File(s)") {
            continue;
        }

        // Standard DIR line format:
        // MM/DD/YYYY  HH:MM AM/PM    <DIR>          folder_name
        // MM/DD/YYYY  HH:MM AM/PM            12,345 file_name
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 3 {
            // Check if parts[0] is a date (contains / or -)
            let is_date = parts[0].contains('/') || parts[0].contains('-');
            if is_date && parts.len() >= 4 {
                let (type_idx, name_idx) = if parts[2].eq_ignore_ascii_case("am")
                    || parts[2].eq_ignore_ascii_case("pm")
                {
                    (3, 4)
                } else {
                    (2, 3)
                };

                if name_idx < parts.len() {
                    let type_or_size = parts[type_idx];
                    let name = parts[name_idx..].join(" ");

                    // Skip current/parent directory pointers
                    if name == "." || name == ".." {
                        continue;
                    }

                    if type_or_size.eq_ignore_ascii_case("<DIR>") {
                        current_entries.push(format!("{}/", name));
                    } else if let Ok(bytes) = type_or_size.replace(',', "").parse::<u64>() {
                        current_entries.push(format!("{} ({})", name, format_size(bytes)));
                    } else {
                        current_entries.push(name);
                    }
                    continue;
                }
            }
        }
        current_entries.push(trimmed.to_string());
    }

    if !current_entries.is_empty() || current_dir.is_some() {
        sections.push((current_dir, current_entries));
    }

    if sections.is_empty() || sections.iter().all(|(_, entries)| entries.is_empty()) {
        return "(empty directory)".trim_end().to_string();
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

/// Run `dir` with args via `cmd.exe` and filter its output.
pub fn run(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_cmd("dir", args);
    let cmd_label = format!("dir {}", args.join(" "));
    
    runner::run_filtered(
        cmd,
        "dir",
        cmd_label.trim(),
        |raw| filter_dir_output(raw),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_dir_output() {
        let sample = r#"
 Volume in drive C has no label.
 Volume Serial Number is 1234-5678

 Directory of C:\test\project

09/16/2026  08:49 PM    <DIR>          .
09/16/2026  08:49 PM    <DIR>          ..
09/16/2026  08:49 PM    <DIR>          src
09/16/2026  08:49 PM             2,512 Cargo.toml
09/16/2026  08:49 PM            26,527 README.md
               2 File(s)         29,039 bytes
               3 Dir(s)  120,456,789,012 bytes free
"#;
        let filtered = filter_dir_output(sample);
        assert!(filtered.contains("C:\\test\\project:"));
        assert!(filtered.contains("src/"));
        assert!(filtered.contains("Cargo.toml (2KB)"));
        assert!(filtered.contains("README.md (25KB)"));
        assert!(!filtered.contains("Volume in drive"));
        assert!(!filtered.contains("bytes free"));
    }
}
