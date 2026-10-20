//! Invariants about configuration files that ship in the repository itself, so a
//! hand-edit of one cannot silently disagree with what `ptk init` writes.
//!
//! The Copilot hook config is the file this catches: `.github/hooks/ptk-rewrite.json`
//! is checked in, read by telemetry (`COPILOT_HOOK_FILE`), and rewritten by
//! `ptk init --copilot`. It was once restored by hand with the right filename and a
//! body that could never fire — `"ptk hook"` with no subcommand is a clap error, and
//! without `"version": 1` Copilot ignores the file entirely.

use std::fs;
use std::path::Path;

use serde_json::Value;

fn checked_in_copilot_hook() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/hooks/ptk-rewrite.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must exist and be readable: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("{} must be valid JSON: {e}", path.display()))
}

#[test]
fn the_checked_in_copilot_hook_carries_the_schema_version() {
    let hook = checked_in_copilot_hook();
    assert_eq!(
        hook.get("version").and_then(Value::as_u64),
        Some(1),
        "Copilot only honors the hook file when it declares \"version\": 1"
    );
}

#[test]
fn the_checked_in_copilot_hook_runs_a_real_subcommand() {
    let hook = checked_in_copilot_hook();
    let command = hook
        .pointer("/hooks/PreToolUse/0/command")
        .and_then(Value::as_str)
        .expect("the hook must register a PreToolUse command");

    assert_eq!(
        command, "ptk hook copilot",
        "bare `ptk hook` is a missing-subcommand error, so the hook would never run"
    );
}

#[test]
fn the_checked_in_copilot_hook_is_wired_to_a_command_hook() {
    let hook = checked_in_copilot_hook();
    let entry = hook
        .pointer("/hooks/PreToolUse/0")
        .expect("the hook must register a PreToolUse entry");

    assert_eq!(
        entry.get("type").and_then(Value::as_str),
        Some("command"),
        "only command hooks are executed"
    );
    assert!(
        entry.get("timeout").and_then(Value::as_u64).is_some(),
        "an unbounded hook can hang the agent's tool call"
    );
}

// ─── Branch model ────────────────────────────────────────────────────────────
//
// The fork's trunk is `main`. Its workflows were inherited from a repository
// whose trunk was `master` with a `develop` staging branch, and every inherited
// branch filter failed the way these things always fail: silently. `branches:
// [develop, master]` matches neither branch here, so the workflow never runs and
// nothing reports it as broken. These tests make the branch names checkable.

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_file(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()))
}

fn repo_files_under(rel_dir: &str, exts: &[&str]) -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, exts: &[&str], out: &mut Vec<(String, String)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|n| n == "target" || n == ".git")
                {
                    continue;
                }
                walk(&path, root, exts, out);
            } else if exts.iter().any(|e| path.to_string_lossy().ends_with(e)) {
                let Ok(body) = fs::read_to_string(&path) else {
                    continue;
                };
                let rel = path
                    .strip_prefix(root)
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                out.push((rel, body));
            }
        }
    }
    let root = repo_root();
    let mut out = Vec::new();
    walk(&root.join(rel_dir), &root, exts, &mut out);
    out.sort();
    out
}

/// A branch filter naming a branch that does not exist is not an error to GitHub:
/// the workflow simply never triggers. `main` is the only trunk branch.
#[test]
fn no_workflow_gates_on_a_branch_that_does_not_exist() {
    let workflows = repo_files_under(".github/workflows", &[".yml", ".yaml"]);
    assert!(
        !workflows.is_empty(),
        "there must be workflow files to check"
    );

    let mut offenders = Vec::new();
    for (rel, body) in &workflows {
        for (n, line) in body.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with('#') {
                continue;
            }
            let names_a_stale_branch = code.contains("master") || code.contains("develop");
            let is_a_filter = code.contains("branches:")
                || code.contains("target-branch")
                || code.contains("refs/heads/")
                || code.contains("base.ref")
                || code.contains("head_ref")
                || code.contains("base_ref");
            // `branches:` may wrap onto indented list items on the next lines.
            let is_list_item = code.starts_with("- ") && line.starts_with("    ");
            if names_a_stale_branch && (is_a_filter || is_list_item) {
                offenders.push(format!("{rel}:{}: {}", n + 1, code));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these filters name a branch that does not exist, so the workflow never runs:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn pull_requests_are_gated_to_main() {
    let check = repo_file(".github/workflows/pr-target-check.yml");
    assert!(
        check.contains("base.ref != 'main'"),
        "pr-target-check must compare the base branch against `main`"
    );

    let template = repo_file(".github/PULL_REQUEST_TEMPLATE.md");
    assert!(
        template.contains("`main`") && !template.contains("`develop`"),
        "the PR template must tell contributors to target `main`"
    );

    let copilot = repo_file(".github/copilot-instructions.md");
    assert!(
        copilot.contains("**`main`**"),
        "the Copilot instructions must name `main` as the PR target"
    );
}

#[test]
fn dependency_updates_land_on_the_trunk() {
    let dependabot = repo_file(".github/dependabot.yml");
    let targets: Vec<&str> = dependabot
        .lines()
        .filter(|l| l.trim_start().starts_with("target-branch:"))
        .collect();
    assert!(!targets.is_empty(), "dependabot must pin an update branch");
    for line in targets {
        assert!(
            line.contains("\"main\""),
            "dependabot must target the trunk, found: {line}"
        );
    }
}

/// Every documented way to install the fork fetches a script or a branch of this
/// repository by name. A stale branch name there is a 404 for a new user.
#[test]
fn install_instructions_fetch_from_the_trunk() {
    let mut checked = 0;
    let mut offenders = Vec::new();

    let mut files: Vec<(String, String)> = vec![
        ("INSTALL.md".into(), repo_file("INSTALL.md")),
        ("README.md".into(), repo_file("README.md")),
        ("install.sh".into(), repo_file("install.sh")),
    ];
    files.extend(repo_files_under("docs", &[".md"]));
    files.extend(
        repo_files_under(".", &[".md"])
            .into_iter()
            .filter(|(rel, _)| rel.starts_with("README") || rel.starts_with("hooks/"))
            .collect::<Vec<_>>(),
    );
    files.extend(repo_files_under("scripts", &[".sh"]));
    files.extend(repo_files_under("openclaw", &[".md"]));
    files.extend(repo_files_under("hooks", &[".sh"]));

    for (rel, body) in files {
        for (n, line) in body.lines().enumerate() {
            let stale = line.contains("riaku/ptk/develop")
                || line.contains("riaku/ptk/refs/heads/develop")
                || line.contains("riaku/ptk/blob/develop")
                || line.contains("--branch develop")
                || line.contains("origin/develop");
            if stale {
                offenders.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
            if line.contains("riaku/ptk") {
                checked += 1;
            }
        }
    }

    assert!(checked > 0, "the scan must actually find fork URLs");
    assert!(
        offenders.is_empty(),
        "these instructions point at a branch that is not the trunk:\n{}",
        offenders.join("\n")
    );
}

/// A stale diff base is worse than a stale trigger: `git diff origin/master...HEAD`
/// fails, `|| true` swallows it, and the job happily scans an empty file list. The
/// Security Scan job ran that way on every PR in this fork.
#[test]
fn no_workflow_diffs_against_a_ref_that_does_not_exist() {
    let workflows = repo_files_under(".github/workflows", &[".yml", ".yaml"]);
    let mut offenders = Vec::new();
    for (rel, body) in &workflows {
        for (n, line) in body.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with('#') {
                continue;
            }
            for stale in [
                "origin/master",
                "origin/develop",
                "refs/remotes/origin/master",
            ] {
                if code.contains(stale) {
                    offenders.push(format!("{rel}:{}: {}", n + 1, code));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these diffs are measured against a branch this repository does not have, so\n\
         they resolve to nothing (and any `|| true` hides it):\n{}",
        offenders.join("\n")
    );
}
