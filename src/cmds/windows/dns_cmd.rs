//! Token-optimized filter for PowerShell `Resolve-DnsName`.
//!
//! Compresses wide table spacing and multi-line Format-List records into
//! a compact, token-saving DNS answer table: `NAME  TYPE  IP/TARGET  (TTL)`.

use crate::core::runner::{self, RunOptions};
use crate::core::shell::display_args;
use crate::core::windows_shell;
use anyhow::Result;

#[derive(Default, Clone)]
struct DnsRecord {
    name: String,
    query_type: String,
    ttl: String,
    target: String,
}

/// True when `output` is a PowerShell error record rather than cmdlet output.
///
/// Checked before parsing because both parsers key off punctuation that appears in
/// error text too: the Format-List path looks for `Key: value` pairs (an error
/// record has `CategoryInfo : ...`), and the table path accepts any line with two
/// or more whitespace-separated fields.
fn output_is_powershell_error(output: &str) -> bool {
    const MARKERS: [&str; 4] = [
        // Also matches Windows PowerShell 5.1's "is not recognized as the name of a cmdlet".
        "is not recognized as",
        // The structured tail that every non-terminating error record carries.
        "CategoryInfo",
        "FullyQualifiedErrorId",
        "CommandNotFoundException",
    ];
    MARKERS.iter().any(|marker| output.contains(marker))
}

/// Filters raw `Resolve-DnsName` output into a concise DNS lookup summary.
pub fn filter_dns_output(raw: &str) -> String {
    let clean = crate::core::utils::strip_ansi(raw);
    let mut entries: Vec<DnsRecord> = Vec::new();

    // A failed spawn -- no NetTCPIP module, a non-Windows host, a typo in the
    // queried name -- prints a PowerShell error record instead of a table. The
    // table parser below splits on whitespace, so such a record was rendered as an
    // answer row ("Resolve-DnsName: The term ..." became NAME="Resolve-DnsName:",
    // TYPE="The"), which made a missing cmdlet look like a successful lookup. Pass
    // the error through untouched so the failure stays visible and diagnosable.
    if output_is_powershell_error(&clean) {
        return clean.trim().trim_end().to_string();
    }

    // Check if output is Format-List style (Name : ... \n QueryType : ...)
    if clean
        .lines()
        .any(|l| l.trim().starts_with("QueryType") && l.contains(':'))
    {
        let mut current = DnsRecord::default();

        for line in clean.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !current.name.is_empty()
                    && (!current.target.is_empty() || !current.query_type.is_empty())
                {
                    entries.push(std::mem::take(&mut current));
                }
                continue;
            }

            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim();
                let val = v.trim();
                match key {
                    "Name" => current.name = val.to_string(),
                    "QueryType" | "Type" => current.query_type = val.to_string(),
                    "TTL" => current.ttl = val.to_string(),
                    "IPAddress" | "IP4Address" | "IP6Address" | "NameHost" | "PrimaryServer"
                        if current.target.is_empty() =>
                    {
                        current.target = val.to_string();
                    }
                    _ => {}
                }
            }
        }

        if !current.name.is_empty()
            && (!current.target.is_empty() || !current.query_type.is_empty())
        {
            entries.push(current);
        }
    } else {
        // Table format
        for line in clean.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("Name ")
                || trimmed.starts_with("Name\t")
                || trimmed.chars().all(|c| c == '-' || c.is_whitespace())
            {
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            // Name Type TTL Section IPAddress/Target
            if parts.len() >= 5 {
                entries.push(DnsRecord {
                    name: parts[0].to_string(),
                    query_type: parts[1].to_string(),
                    ttl: parts[2].to_string(),
                    target: parts[4].to_string(),
                });
            } else if parts.len() >= 2 {
                entries.push(DnsRecord {
                    name: parts[0].to_string(),
                    query_type: parts[1].to_string(),
                    ttl: if parts.len() >= 3 {
                        parts[2].to_string()
                    } else {
                        "-".to_string()
                    },
                    target: if parts.len() >= 4 {
                        parts[3].to_string()
                    } else {
                        "-".to_string()
                    },
                });
            }
        }
    }

    if entries.is_empty() {
        if clean.trim().is_empty() {
            return "(no dns records found)".trim_end().to_string();
        }
        return clean.trim().trim_end().to_string();
    }

    let mut output = Vec::with_capacity(entries.len() + 1);
    let header = format!(
        "{:<30}  {:<6}  {:<6}  {}",
        "NAME", "TYPE", "TTL", "TARGET/IP"
    )
    .trim_end()
    .to_string();
    output.push(header);

    for r in entries {
        output.push(
            format!(
                "{:<30}  {:<6}  {:<6}  {}",
                r.name, r.query_type, r.ttl, r.target
            )
            .trim_end()
            .to_string(),
        );
    }

    output.join("\n").trim_end().to_string()
}

/// Run `Resolve-DnsName` via PowerShell and filter output.
pub fn run(args: &[String], _verbose: u8) -> Result<i32> {
    let cmd = windows_shell::spawn_powershell_args("Resolve-DnsName", args);
    let cmd_label = format!("Resolve-DnsName {}", display_args(args));

    runner::run_filtered(
        cmd,
        "Resolve-DnsName",
        cmd_label.trim(),
        filter_dns_output,
        RunOptions::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_dns_output_table() {
        let sample = r#"
Name                                           Type   TTL   Section    IPAddress
----                                           ----   ---   -------    ---------
google.com                                     AAAA   30    Answer     2607:f8b0:4001:c76::8a
google.com                                     A      30    Answer     142.250.190.46
"#;
        let filtered = filter_dns_output(sample);
        assert!(filtered.contains("NAME"));
        assert!(filtered.contains("google.com"));
        assert!(filtered.contains("AAAA"));
        assert!(filtered.contains("2607:f8b0:4001:c76::8a"));
        assert!(!filtered.contains("----"));
    }

    #[test]
    fn test_filter_dns_output_list() {
        let sample = r#"
Name       : localhost
QueryType  : AAAA
TTL        : 1200
Section    : Question
IP6Address : ::1

Name       : localhost
QueryType  : A
TTL        : 1200
Section    : Question
IP4Address : 127.0.0.1
"#;
        let filtered = filter_dns_output(sample);
        assert!(filtered.contains("localhost"));
        assert!(filtered.contains("127.0.0.1"));
        assert!(filtered.contains("::1"));
    }

    #[test]
    fn test_first_address_for_a_name_wins() {
        // Resolve-DnsName can report several address keys for one name; the
        // filter keeps the first and drops the rest, so the name is listed once.
        let sample = r#"
Name       : multi.example
QueryType  : A
TTL        : 60
Section    : Answer
IP4Address : 10.0.0.1
IPAddress  : 10.0.0.2
"#;
        let filtered = filter_dns_output(sample);
        assert!(
            filtered.contains("10.0.0.1"),
            "the first address is kept: {filtered}"
        );
        assert!(
            !filtered.contains("10.0.0.2"),
            "a second address for the same name is dropped: {filtered}"
        );
    }

    #[test]
    fn test_powershell_error_is_not_rendered_as_a_dns_record() {
        // On a host without the NetTCPIP module -- or without Windows at all -- the
        // spawn fails and PowerShell prints its error on stdout. The table parser
        // splits any line with >= 5 whitespace fields into a record, so this error
        // used to surface as an answer row: NAME="Resolve-DnsName:", TYPE="The".
        let sample = "Resolve-DnsName: The term 'Resolve-DnsName' is not recognized as a name of a cmdlet, function, script file, or executable program.\n";
        let filtered = filter_dns_output(sample);

        assert!(
            !filtered.contains("NAME") && !filtered.contains("TARGET/IP"),
            "an error must not be formatted as a results table: {filtered}"
        );
        assert!(
            filtered.contains("is not recognized"),
            "the error text must survive so the user can act on it: {filtered}"
        );
    }

    #[test]
    fn test_full_powershell_error_dump_passes_through() {
        let sample = r#"Resolve-DnsName: The term 'Resolve-DnsName' is not recognized as a name of a cmdlet, function, script file, or executable program.
Check the spelling of the name, or if a path was included, verify that the path is correct and try again.
At line:1 char:1
+ Resolve-DnsName localhost
+ ~~~~~~~~~~~~~~~
    + CategoryInfo          : ObjectNotFound: (Resolve-DnsName:String) [], CommandNotFoundException
    + FullyQualifiedErrorId : CommandNotFoundException
"#;
        let filtered = filter_dns_output(sample);

        assert!(
            !filtered.contains("NAME") && !filtered.contains("TARGET/IP"),
            "an error record must not become a DNS answer: {filtered}"
        );
        assert!(
            filtered.contains("CommandNotFoundException"),
            "the failure reason must survive: {filtered}"
        );
    }
}
