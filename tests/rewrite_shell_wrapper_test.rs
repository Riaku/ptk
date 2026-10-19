use std::process::Output;

mod common;

fn rewrite(command: &str) -> Output {
    let home = tempfile::tempdir().expect("create isolated home");
    common::ptk_command()
        .args(["rewrite", command])
        .current_dir(home.path())
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env("PTK_TELEMETRY_DISABLED", "1")
        .output()
        .expect("run ptk rewrite")
}

#[test]
fn bash_command_string_rewrites_inner_commands() {
    let output = rewrite(r#"bash -c "head foo && grep -R bar .""#);

    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        r#"bash -c "ptk read foo --head-lines 10 && ptk grep -R bar .""#
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn zsh_command_string_rewrites_inner_commands() {
    let output = rewrite("zsh -c 'git status; cargo test'");

    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "zsh -c 'ptk git status; ptk cargo test'"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn fish_command_string_rewrites_portable_inner_commands() {
    let output = rewrite("fish -c 'git status; cargo test'");

    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "fish -c 'ptk git status; ptk cargo test'"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn fish_specific_inner_script_passes_through() {
    let output = rewrite("fish -c 'git status; and cargo test'");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

/// `&|` and `&!` are single operators in zsh — background and disown. The
/// shared lexer reads each as two tokens, so a rewrite re-emits them spaced:
/// `cmd & | …` is a parse error there, and `cmd & ! …` negates the next
/// command, turning a failing script into a successful one. Both pass through.
#[test]
fn zsh_specific_inner_script_passes_through() {
    for command in [
        "zsh -c 'cargo build &| tail -3'",
        "zsh -c 'ls &! ls /nonexistent'",
        "zsh -c 'ls&!ls /nonexistent'",
    ] {
        let output = rewrite(command);

        assert_eq!(output.status.code(), Some(1), "{command}");
        assert!(output.stdout.is_empty(), "{command}");
        assert!(output.stderr.is_empty(), "{command}");
    }
}

#[test]
fn unsafe_inner_script_passes_through_without_output() {
    let output = rewrite("bash -c 'git status $(whoami)'");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}
