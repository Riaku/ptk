//! Windows shell execution engine for PTK.
//!
//! Provides transparent spawning and handling for Windows CMD built-ins
//! (dir, type, findstr, etc.) and PowerShell cmdlets (Get-ChildItem, Get-Content, Select-String, etc.).

use std::process::Command;
use std::sync::OnceLock;

/// List of common Windows CMD built-in commands and utilities.
#[allow(dead_code)]
pub const CMD_BUILTINS: &[&str] = &[
    "dir", "type", "findstr", "where", "tasklist", "taskkill", "set", "copy", "del", "erase",
    "ren", "rename", "move", "ver", "vol", "mkdir", "md", "rmdir", "rd", "cls", "assoc", "ftype",
    "netstat",
];

/// List of common PowerShell cmdlet prefixes and aliases.
pub const POWERSHELL_ALIASES: &[&str] = &[
    "gci", "gc", "sls", "select-string", "get-childitem", "get-content", "test-path",
    "get-process", "gps", "ps", "stop-process", "start-process", "get-service", "gsv",
    "get-item", "get-itemproperty", "set-item", "new-item", "ni", "remove-item",
    "get-netipaddress", "netip", "get-eventlog", "eventlog", "get-winevent", "winevent",
    "get-nettcpconnection", "nettcp", "resolve-dnsname", "dns", "test-netconnection", "tnc",
    "convertfrom-json", "convertto-json", "convertfrom-csv", "export-csv", "epcsv",
    "format-table", "ft", "format-list", "fl",
    "invoke-webrequest", "iwr", "invoke-restmethod", "irm",
];

/// Returns true if the command name is a known Windows CMD built-in.
#[allow(dead_code)]
pub fn is_cmd_builtin(name: &str) -> bool {
    let lower = name.to_lowercase();
    CMD_BUILTINS.contains(&lower.as_str())
}

/// Returns true if the command name is a PowerShell cmdlet or alias.
pub fn is_powershell_cmdlet(name: &str) -> bool {
    let lower = name.to_lowercase();
    if POWERSHELL_ALIASES.contains(&lower.as_str()) {
        return true;
    }
    // Any Verb-Noun cmdlet naming convention (e.g. Get-*, Set-*, Test-*)
    if let Some((verb, noun)) = lower.split_once('-') {
        if !verb.is_empty() && !noun.is_empty() {
            let common_verbs = [
                "get", "set", "new", "remove", "test", "invoke", "start", "stop", "restart",
                "enable", "disable", "add", "clear", "export", "import", "select", "format",
                "out", "write", "measure", "sort", "where",
            ];
            return common_verbs.contains(&verb);
        }
    }
    false
}

/// Resolves the preferred PowerShell executable (`pwsh` for PowerShell 7+, fallback to `powershell`).
pub fn resolve_powershell_bin() -> &'static str {
    static PS_BIN: OnceLock<&'static str> = OnceLock::new();
    *PS_BIN.get_or_init(|| {
        if which::which("pwsh").is_ok() {
            "pwsh"
        } else {
            "powershell"
        }
    })
}

/// Spawns a command inside Windows `cmd.exe`.
///
/// Uses `/D` (disable AutoRun) and `/C` (run command and terminate).
pub fn spawn_cmd(command: &str, args: &[String]) -> Command {
    let mut cmd = Command::new("cmd.exe");
    cmd.arg("/D").arg("/C").arg(command);
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

/// Spawns a command string inside PowerShell (`pwsh` or `powershell.exe`).
///
/// Uses `-NoProfile` and `-NonInteractive` for fast, reproducible execution.
pub fn spawn_powershell(command_str: &str) -> Command {
    let ps_bin = resolve_powershell_bin();
    let mut cmd = Command::new(ps_bin);
    cmd.arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(command_str);
    cmd
}

/// Spawns a PowerShell cmdlet with argument vector.
pub fn spawn_powershell_args(cmdlet: &str, args: &[String]) -> Command {
    let escaped_args: Vec<String> = args
        .iter()
        .map(|a| {
            if a.contains(' ') || a.contains('"') {
                format!("\"{}\"", a.replace('"', "`\""))
            } else {
                a.clone()
            }
        })
        .collect();
    let full_command = format!("{} {}", cmdlet, escaped_args.join(" "));
    spawn_powershell(&full_command)
}

/// Checks if a command string contains shell control metacharacters
/// (pipeline `|`, redirection `>`, `<`, chaining `&&`, `;`) outside of quotes.
pub fn has_shell_metachars(cmd: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = ' ';

    for c in cmd.chars() {
        match c {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote && prev_char != '\\' && prev_char != '`' => {
                in_double_quote = !in_double_quote;
            }
            '|' | '>' | '<' | ';' if !in_single_quote && !in_double_quote => return true,
            '&' if !in_single_quote && !in_double_quote => return true,
            _ => {}
        }
        prev_char = c;
    }
    false
}

/// Splits a command string into arguments, preserving single and double quoted spans.
pub fn split_command_args(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = ' ';

    for c in cmd.chars() {
        match c {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote && prev_char != '\\' && prev_char != '`' => {
                in_double_quote = !in_double_quote;
            }
            c if c.is_whitespace() && !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(c);
            }
        }
        prev_char = c;
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cmd_builtin() {
        assert!(is_cmd_builtin("dir"));
        assert!(is_cmd_builtin("findstr"));
        assert!(is_cmd_builtin("type"));
        assert!(!is_cmd_builtin("grep"));
    }

    #[test]
    fn test_has_shell_metachars() {
        assert!(has_shell_metachars("Get-ChildItem | Measure-Object"));
        assert!(has_shell_metachars("dir > out.txt"));
        assert!(has_shell_metachars("gci; echo done"));
        assert!(has_shell_metachars("gci && echo done"));
        
        // Quoted patterns should NOT be detected as shell metachars
        assert!(!has_shell_metachars("Select-String \"pattern|alternate\" file.txt"));
        assert!(!has_shell_metachars("findstr 'foo>bar' file.txt"));
        assert!(!has_shell_metachars("Get-ChildItem -Path C:\\source\\ptk"));
    }
}

