//! Token-optimized filters for JSON and CSV cmdlets:
//! - `ConvertFrom-Json`
//! - `ConvertTo-Json`
//! - `ConvertFrom-Csv`
//! - `Export-Csv` (`epcsv`)

use crate::core::runner::{self, RunOptions};
use crate::core::windows_shell;
use anyhow::Result;

/// Filters `ConvertFrom-Json` output into a compact representation.
pub fn filter_convertfrom_json(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut lines = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // If table separator line (all dashes/whitespace), strip it
        if trimmed.chars().all(|c| c == '-' || c.is_whitespace()) {
            continue;
        }

        // If key-value pair (e.g. `a : 1`), compress space
        if let Some((k, v)) = trimmed.split_once(':') {
            lines.push(format!("{}: {}", k.trim(), v.trim()).trim_end().to_string());
        } else {
            // Normalize multi-column table spacing
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            lines.push(parts.join("  ").trim_end().to_string());
        }
    }

    if lines.is_empty() {
        if clean.trim().is_empty() {
            return String::new();
        }
        return clean.trim().trim_end().to_string();
    }

    lines.join("\n").trim_end().to_string()
}

/// Minifies JSON output from `ConvertTo-Json` without breaking valid JSON.
pub fn filter_convertto_json(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let trimmed = clean.trim();

    // Try parsing as serde_json::Value to produce safely minified JSON
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Ok(minified) = serde_json::to_string(&val) {
            return minified.trim_end().to_string();
        }
    }

    // Fallback: strip line-by-line whitespace
    clean
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

/// Filters `ConvertFrom-Csv` output into a compact representation.
pub fn filter_convertfrom_csv(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut lines = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Strip dashed table lines
        if trimmed.chars().all(|c| c == '-' || c.is_whitespace()) {
            continue;
        }

        // Check if list format `key : value`
        if let Some((k, v)) = trimmed.split_once(':') {
            lines.push(format!("{}: {}", k.trim(), v.trim()).trim_end().to_string());
        } else {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            lines.push(parts.join("  ").trim_end().to_string());
        }
    }

    if lines.is_empty() {
        if clean.trim().is_empty() {
            return String::new();
        }
        return clean.trim().trim_end().to_string();
    }

    lines.join("\n").trim_end().to_string()
}

/// Filters `Export-Csv` execution confirmation.
pub fn filter_export_csv_output(raw: &str, target_file: Option<&str>) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    if let Some(target) = target_file {
        format!("+ Exported CSV -> {}", target).trim_end().to_string()
    } else if clean.trim().is_empty() {
        "+ Exported CSV successfully".trim_end().to_string()
    } else {
        clean.trim().trim_end().to_string()
    }
}

/// Run `ConvertFrom-Json` via PowerShell and filter output.
pub fn run_convertfrom_json(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("ConvertFrom-Json", args);
    let cmd_label = format!("ConvertFrom-Json {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "ConvertFrom-Json",
        cmd_label.trim(),
        |raw| filter_convertfrom_json(raw),
        RunOptions::default(),
    )
}

/// Run `ConvertTo-Json` via PowerShell and minify output.
pub fn run_convertto_json(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("ConvertTo-Json", args);
    let cmd_label = format!("ConvertTo-Json {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "ConvertTo-Json",
        cmd_label.trim(),
        |raw| filter_convertto_json(raw),
        RunOptions::default(),
    )
}

/// Run `ConvertFrom-Csv` via PowerShell and filter output.
pub fn run_convertfrom_csv(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("ConvertFrom-Csv", args);
    let cmd_label = format!("ConvertFrom-Csv {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "ConvertFrom-Csv",
        cmd_label.trim(),
        |raw| filter_convertfrom_csv(raw),
        RunOptions::default(),
    )
}

/// Run `Export-Csv` via PowerShell and produce concise notification.
pub fn run_export_csv(args: &[String], _verbose: u8) -> Result<i32> {
    let mut target_file = None;
    for i in 0..args.len() {
        let arg = &args[i];
        if (arg.eq_ignore_ascii_case("-Path") || arg.eq_ignore_ascii_case("-LiteralPath")) && i + 1 < args.len() {
            target_file = Some(args[i + 1].as_str());
            break;
        } else if !arg.starts_with('-') && target_file.is_none() {
            target_file = Some(arg.as_str());
        }
    }

    let cmd = windows_shell::spawn_powershell_args("Export-Csv", args);
    let cmd_label = format!("Export-Csv {}", args.join(" "));

    runner::run_filtered(
        cmd,
        "Export-Csv",
        cmd_label.trim(),
        move |raw| filter_export_csv_output(raw, target_file),
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_convertfrom_json() {
        let sample = r#"
a b
- -
1 2
"#;
        let filtered = filter_convertfrom_json(sample);
        assert_eq!(filtered, "a  b\n1  2");
    }

    #[test]
    fn test_filter_convertto_json() {
        let sample = r#"
{
  "Id": 4,
  "ProcessName": "System"
}
"#;
        let filtered = filter_convertto_json(sample);
        assert_eq!(filtered, r#"{"Id":4,"ProcessName":"System"}"#);
    }

    #[test]
    fn test_filter_convertfrom_csv() {
        let sample = r#"
id   : 1
name : alice

id   : 2
name : bob
"#;
        let filtered = filter_convertfrom_csv(sample);
        assert!(filtered.contains("id: 1"));
        assert!(filtered.contains("name: alice"));
    }
}
