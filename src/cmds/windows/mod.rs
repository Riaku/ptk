//! Windows CMD and PowerShell command execution and filtering modules.

pub mod dir_cmd;
pub mod dns_cmd;
pub mod event_cmd;
pub mod findstr_cmd;
pub mod format_cmd;
pub mod gci_cmd;
pub mod json_csv_cmd;
pub mod netip_cmd;
pub mod nettcp_cmd;
pub mod new_item_cmd;
pub mod process_cmd;
pub mod service_cmd;
pub mod tnc_cmd;
pub mod type_cmd;

use anyhow::{Context, Result};
use crate::core::windows_shell;
use crate::core::runner::{self, RunOptions};

/// Intelligently routes and filters a raw command string through
/// CMD or PowerShell based on the command syntax.
pub fn run_smart_command(raw: &str, verbose: u8) -> Result<i32> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }

    let tokens = windows_shell::split_command_args(trimmed);
    let first_token = tokens.first().map(|s| s.to_lowercase()).unwrap_or_default();
    let is_ps = windows_shell::is_powershell_cmdlet(&first_token) || trimmed.starts_with('$');

    // If the command contains shell pipelines (|), redirects (>), or chaining (&&, ;),
    // execute it directly in passthrough mode to avoid breaking downstream consumers.
    if windows_shell::has_shell_metachars(trimmed) {
        let cmd = if is_ps {
            windows_shell::spawn_powershell(trimmed)
        } else {
            windows_shell::spawn_cmd(trimmed, &[])
        };
        return runner::run(
            cmd,
            if is_ps { "pwsh" } else { "cmd" },
            trimmed,
            runner::RunMode::Passthrough,
            RunOptions::default(),
        );
    }

    let args: Vec<String> = tokens.into_iter().skip(1).collect();

    if first_token == "dir" {
        return dir_cmd::run(&args, verbose);
    }

    if first_token == "gci" || first_token == "get-childitem" {
        return gci_cmd::run(&args, verbose);
    }

    if first_token == "type" {
        return type_cmd::run_cmd_type(&args, verbose);
    }

    if first_token == "gc" || first_token == "get-content" {
        return type_cmd::run_get_content(&args, verbose);
    }

    if first_token == "findstr" {
        return findstr_cmd::run_findstr(&args, verbose);
    }

    if first_token == "sls" || first_token == "select-string" {
        return findstr_cmd::run_select_string(&args, verbose);
    }

    if first_token == "gps" || first_token == "ps" || first_token == "get-process" {
        return process_cmd::run(&args, verbose);
    }

    if first_token == "gsv" || first_token == "get-service" {
        return service_cmd::run(&args, verbose);
    }

    if first_token == "netip" || first_token == "get-netipaddress" {
        return netip_cmd::run(&args, verbose);
    }

    if first_token == "ni" || first_token == "new-item" {
        return new_item_cmd::run(&args, verbose);
    }

    if first_token == "eventlog" || first_token == "get-eventlog" {
        return event_cmd::run_eventlog(&args, verbose);
    }

    if first_token == "winevent" || first_token == "get-winevent" {
        return event_cmd::run_winevent(&args, verbose);
    }

    if first_token == "nettcp" || first_token == "get-nettcpconnection" {
        return nettcp_cmd::run_nettcp(&args, verbose);
    }

    if first_token == "netstat" {
        return nettcp_cmd::run_netstat(&args, verbose);
    }

    if first_token == "dns" || first_token == "resolve-dnsname" {
        return dns_cmd::run(&args, verbose);
    }

    if first_token == "tnc" || first_token == "test-netconnection" {
        return tnc_cmd::run(&args, verbose);
    }

    if first_token == "convertfrom-json" {
        return json_csv_cmd::run_convertfrom_json(&args, verbose);
    }

    if first_token == "convertto-json" {
        return json_csv_cmd::run_convertto_json(&args, verbose);
    }

    if first_token == "convertfrom-csv" {
        return json_csv_cmd::run_convertfrom_csv(&args, verbose);
    }

    if first_token == "export-csv" || first_token == "epcsv" {
        return json_csv_cmd::run_export_csv(&args, verbose);
    }

    if first_token == "format-table" || first_token == "ft" {
        return format_cmd::run_format_table(&args, verbose);
    }

    if first_token == "format-list" || first_token == "fl" {
        return format_cmd::run_format_list(&args, verbose);
    }

    // If it's another PowerShell cmdlet or starts with $, run in PowerShell as passthrough
    if is_ps {
        let cmd = windows_shell::spawn_powershell(trimmed);
        return runner::run(
            cmd,
            "pwsh",
            trimmed,
            runner::RunMode::Passthrough,
            RunOptions::default(),
        );
    }

    // Default CMD execution
    let mut cmd = windows_shell::spawn_cmd(trimmed, &[]);
    let status = cmd.status().with_context(|| format!("Failed to execute: {}", trimmed))?;
    Ok(crate::core::utils::exit_code_from_status(&status, "run"))
}
