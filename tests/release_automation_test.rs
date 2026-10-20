//! Guards for the automatic release path: a push to `main` tags and publishes a
//! release with no release pull request in between.
//!
//! The version arithmetic and the changelog grouping run as shell scripts in CI, so
//! they are exercised here against throwaway git repositories rather than by matching
//! strings in YAML — a regex typo in `next-version.sh` should fail a test, not produce
//! a surprise major release.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn repo_file(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// Where the scripts live. Invoked by absolute path with the throwaway repo as the
/// working directory, which is how CI calls them.
fn script(name: &str) -> PathBuf {
    repo_root().join("scripts").join(name)
}

/// `bash` is what CI uses on all three platforms (Git Bash on Windows). If it is
/// genuinely absent, skip rather than fail: the shell logic cannot be exercised.
fn shell_available() -> bool {
    Command::new("bash")
        .arg("-c")
        .arg("exit 0")
        .status()
        .is_ok_and(|s| s.success())
}

macro_rules! require_shell {
    () => {
        if !shell_available() {
            eprintln!("skipping: no bash on this host");
            return;
        }
    };
}

struct TempRepo {
    dir: PathBuf,
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl TempRepo {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("ptk-release-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp repo");
        let repo = Self { dir };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo.git(&["config", "user.name", "Test"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo.git(&["commit", "-q", "--allow-empty", "-m", "chore: init"]);
        repo
    }

    fn git(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("run git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn commit(&self, message: &str) {
        self.git(&["commit", "-q", "--allow-empty", "-m", message]);
    }

    fn tag(&self, name: &str) {
        self.git(&["tag", "-f", name]);
    }

    fn write(&self, rel: &str, body: &str) {
        let path = self.dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, body).expect("write");
    }

    fn run(&self, script_name: &str, args: &[&str]) -> Output {
        Command::new("bash")
            .arg(script(script_name))
            .args(args)
            .current_dir(&self.dir)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("run script")
    }
}

/// The next version the release job would tag, or panics with stderr on a real error.
fn next_version(repo: &TempRepo) -> String {
    let out = repo.run("next-version.sh", &[]);
    assert!(
        out.status.success(),
        "next-version.sh failed ({}): {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

// ── Version arithmetic ──────────────────────────────────────────────────────

#[test]
fn a_fix_commit_bumps_the_patch_version() {
    require_shell!();
    let repo = TempRepo::new("fix-patch");
    repo.tag("v1.2.3");
    repo.commit("fix(cli): stop eating argv");
    assert_eq!(next_version(&repo), "1.2.4");
}

#[test]
fn a_performance_commit_bumps_the_patch_version() {
    require_shell!();
    let repo = TempRepo::new("perf-patch");
    repo.tag("v1.2.3");
    repo.commit("perf(ci): stop paying 19 minutes for a docs change");
    assert_eq!(next_version(&repo), "1.2.4");
}

#[test]
fn a_feature_commit_bumps_the_minor_version() {
    require_shell!();
    let repo = TempRepo::new("feat-minor");
    repo.tag("v1.2.3");
    repo.commit("feat(engine): a new filter");
    assert_eq!(next_version(&repo), "1.3.0");
}

#[test]
fn a_bang_marks_a_breaking_change() {
    require_shell!();
    let repo = TempRepo::new("bang-major");
    repo.tag("v1.2.3");
    repo.commit("feat(cli)!: rename proxy to bypass");
    assert_eq!(next_version(&repo), "2.0.0");
}

#[test]
fn a_breaking_change_footer_is_honoured_too() {
    require_shell!();
    let repo = TempRepo::new("footer-major");
    repo.tag("v1.2.3");
    repo.commit("feat(cli): rename proxy to bypass\n\nBREAKING CHANGE: rtk proxy is rtk bypass");
    assert_eq!(next_version(&repo), "2.0.0");
}

#[test]
fn a_breaking_change_before_one_zero_bumps_the_minor_version() {
    require_shell!();
    let repo = TempRepo::new("pre-one-zero");
    repo.tag("v0.49.0");
    repo.commit("feat(cli)!: rename proxy to bypass");
    assert_eq!(
        next_version(&repo),
        "0.50.0",
        "0.x has no stable surface to break: semver says bump the minor, so a fork \
         sitting on 0.49.0 must not leap to 1.0.0 on its first automatic release"
    );
}

#[test]
fn documentation_and_ci_changes_do_not_cut_a_release() {
    require_shell!();
    let repo = TempRepo::new("no-release");
    repo.tag("v1.2.3");
    repo.commit("docs: explain the release path");
    repo.commit("ci: cache the build");
    repo.commit("chore: tidy");
    repo.commit("Merge pull request #42 from Riaku/some-branch");
    let out = repo.run("next-version.sh", &[]);
    assert_eq!(
        out.status.code(),
        Some(3),
        "a docs-only push must signal 'nothing to release' with exit 3, got {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "");
}

#[test]
fn pre_release_tags_are_never_the_base() {
    require_shell!();
    let repo = TempRepo::new("rc-base");
    repo.tag("v1.2.3");
    repo.tag("dev-1.3.0-rc.7");
    repo.commit("fix: a real fix");
    assert_eq!(
        next_version(&repo),
        "1.2.4",
        "dev-*-rc.* tags are throwaway builds; measuring from them renumbers the stable line"
    );
}

#[test]
fn a_repository_with_no_tags_bumps_its_manifest_version() {
    require_shell!();
    let repo = TempRepo::new("no-tags");
    repo.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\".\"]\n\n[package]\nname = \"ptk\"\nversion = \"0.49.0\"\n",
    );
    repo.commit("fix: first ever fix");
    assert_eq!(next_version(&repo), "0.49.1");
}

#[test]
fn several_in_one_range_take_the_widest_bump() {
    require_shell!();
    let repo = TempRepo::new("widest");
    repo.tag("v1.2.3");
    repo.commit("fix: one");
    repo.commit("feat: two");
    repo.commit("fix: three");
    assert_eq!(next_version(&repo), "1.3.0");
}

// ── Manifest bumping ────────────────────────────────────────────────────────

#[test]
fn every_manifest_carries_the_new_version() {
    require_shell!();
    let repo = TempRepo::new("set-version");
    for rel in [
        "Cargo.toml",
        "engine/Cargo.toml",
        "plugin/Cargo.toml",
        "text/Cargo.toml",
        "proxy/Cargo.toml",
    ] {
        repo.write(rel, &repo_file(rel));
    }
    let out = repo.run("set-version.sh", &["9.9.9"]);
    assert!(
        out.status.success(),
        "set-version.sh failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    for rel in [
        "Cargo.toml",
        "engine/Cargo.toml",
        "plugin/Cargo.toml",
        "text/Cargo.toml",
        "proxy/Cargo.toml",
    ] {
        let text = std::fs::read_to_string(repo.dir.join(rel)).expect("read manifest");
        assert!(
            text.contains("version = \"9.9.9\""),
            "{rel} did not get the new version"
        );
    }
}

#[test]
fn dependency_version_requirements_are_left_alone() {
    require_shell!();
    let repo = TempRepo::new("set-version-deps");
    for rel in [
        "Cargo.toml",
        "engine/Cargo.toml",
        "plugin/Cargo.toml",
        "text/Cargo.toml",
        "proxy/Cargo.toml",
    ] {
        repo.write(rel, &repo_file(rel));
    }
    let out = repo.run("set-version.sh", &["9.9.9"]);
    assert!(
        out.status.success(),
        "set-version.sh failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let engine = std::fs::read_to_string(repo.dir.join("engine/Cargo.toml")).expect("read");
    assert!(
        engine.contains(r#"toon-format = { version = "0.5.0""#),
        "a dependency's version requirement was rewritten by the release bump"
    );
}

#[test]
fn a_malformed_version_is_refused() {
    require_shell!();
    let repo = TempRepo::new("set-version-bad");
    repo.write(
        "Cargo.toml",
        "[package]\nname = \"ptk\"\nversion = \"0.49.0\"\n",
    );
    let out = repo.run("set-version.sh", &["v1.2"]);
    assert_eq!(out.status.code(), Some(2), "a bad version must exit 2");
}

// ── Changelog section ───────────────────────────────────────────────────────

#[test]
fn the_changelog_section_groups_changes_by_type() {
    require_shell!();
    let repo = TempRepo::new("changelog");
    repo.tag("v1.2.3");
    repo.commit("feat(cli): add bypass alias");
    repo.commit("fix(core): stop truncating");
    repo.commit("docs: note the release path");
    let out = repo.run("release-notes.sh", &["1.3.0", "2026-10-20"]);
    assert!(
        out.status.success(),
        "release-notes.sh failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let notes = String::from_utf8_lossy(&out.stdout);
    assert!(
        notes.contains("## 1.3.0 - 2026-10-20"),
        "missing header:\n{notes}"
    );
    assert!(
        notes.contains("### Features"),
        "missing features group:\n{notes}"
    );
    assert!(
        notes.contains("- feat(cli): add bypass alias"),
        "missing feature:\n{notes}"
    );
    assert!(
        notes.contains("### Bug fixes"),
        "missing bug-fix group:\n{notes}"
    );
    assert!(
        notes.contains("- fix(core): stop truncating"),
        "missing fix:\n{notes}"
    );
    let features = notes.split("### Bug fixes").next().unwrap_or("");
    assert!(
        !features.contains("docs: note the release path"),
        "a docs commit leaked into the features group:\n{notes}"
    );
}

// ── Wiring ──────────────────────────────────────────────────────────────────

#[test]
fn cd_tags_main_without_a_release_pull_request() {
    let cd = repo_file(".github/workflows/cd.yml");
    assert!(
        !cd.contains("release-please"),
        "cd.yml still runs release-please: the release must not wait for a pull request"
    );
    assert!(
        cd.contains("scripts/next-version.sh"),
        "cd.yml must derive the version from the commits with scripts/next-version.sh"
    );
    assert!(
        cd.contains("scripts/set-version.sh"),
        "cd.yml must write the new version into the manifests"
    );
    assert!(
        cd.contains("chore(release)"),
        "cd.yml needs a guard against re-releasing its own version-bump commit"
    );
    assert!(cd.contains("git tag"), "cd.yml must create the tag itself");
}

#[test]
fn the_commit_being_tagged_is_compiled_and_tested_first() {
    let cd = repo_file(".github/workflows/cd.yml");
    let test_at = cd
        .find("cargo test")
        .expect("cd.yml must validate the commit it is about to tag");
    let tag_at = cd.find("git tag").expect("cd.yml must create the tag");
    assert!(
        test_at < tag_at,
        "the fast test gate must run before the tag is created, or a broken main gets published"
    );
}

#[test]
fn the_longest_group_is_capped_in_the_changelog() {
    require_shell!();
    let repo = TempRepo::new("changelog-cap");
    repo.tag("v1.2.3");
    for n in 0..60 {
        repo.commit(&format!("fix(core): repair thing {n}"));
    }
    let out = repo.run("release-notes.sh", &["1.2.4", "2026-10-20"]);
    assert!(out.status.success(), "release-notes.sh failed");
    let notes = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        notes.matches("- fix(core): repair thing").count(),
        50,
        "the group must stop at the cap"
    );
    assert!(
        notes.contains("and 10 more"),
        "the truncated tail must be accounted for, not silently dropped:\n{notes}"
    );
}

#[test]
fn the_release_please_flow_is_gone_and_not_merely_unused() {
    for rel in [
        "release-please-config.json",
        ".release-please-manifest.json",
    ] {
        assert!(
            !repo_root().join(rel).exists(),
            "{rel} is still checked in: with no release pull request there is nothing to \
             configure, and leaving it invites someone to re-enable it"
        );
    }
}

#[test]
fn the_build_packages_the_tag_it_names() {
    let rel = repo_file(".github/workflows/release.yml");
    assert!(
        rel.contains("ref: ${{ inputs.tag }}"),
        "the build must check out the tag it packages: a workflow_call run otherwise \
         builds the caller's ref, so a merge landing mid-build ships new code under an \
         old version number"
    );
}

#[test]
fn the_release_page_carries_generated_notes() {
    let rel = repo_file(".github/workflows/release.yml");
    assert!(
        rel.contains("generate_release_notes: true"),
        "the GitHub release must carry generated notes now that no changelog PR narrates it"
    );
}
