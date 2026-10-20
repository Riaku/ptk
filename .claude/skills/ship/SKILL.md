---
description: Build, commit, push & version bump workflow - automates the complete release cycle
allowed-tools: Read Write Edit Bash Grep Glob
---

# Ship Release

Systematic release workflow for PTK: build verification, version bump, changelog update, git tag, and push to trigger CI/CD.

## When to Use

- **Manual invocation**: When ready to release a new version
- **After feature completion**: Before tagging and publishing
- **Before version bump**: To automate the release checklist

## Pre-Release Checklist (Auto-Verified)

Before running `/ship`, verify:

### 1. Quality Checks Pass
```bash
cargo fmt --all --check    # Code formatted
cargo clippy --all-targets # Zero warnings
cargo test --all           # All tests pass
```

### 2. Performance Benchmarks Pass
```bash
hyperfine 'target/release/ptk git status' --warmup 3
# Should show <10ms mean time

/usr/bin/time -l target/release/ptk git status
# Should show <5MB maximum resident set size
```

### 3. Integration Tests Pass
```bash
cargo install --path . --force  # Install locally
cargo test --ignored            # Run integration tests
```

### 4. Git Clean State
```bash
git status  # Should show "nothing to commit, working tree clean"
```

## Release Workflow — automated

Releases are cut by `cd.yml` on every push to `main`. Nothing in this section is done by
hand; it exists so you know what happens and what to do when a release is wrong.

1. `scripts/next-version.sh` reads the conventional commits since the last `vX.Y.Z` tag:
   `fix`/`perf` → patch, `feat` → minor, `!` or a `BREAKING CHANGE:` footer → major (minor
   below 1.0, where there is no stable surface to break). `docs`, `ci`, `chore`, `test`,
   `build`, and `style` cut no release at all — the script exits 3 and the job stops.
2. `cargo test -p ptk --lib` runs on the exact commit about to be published.
3. The version is written into the five manifests by `scripts/set-version.sh`, `Cargo.lock`
   is re-resolved, and `scripts/release-notes.sh` prepends a `CHANGELOG.md` section.
4. `chore(release): vX.Y.Z` is committed, `vX.Y.Z` tagged, and both are pushed.
5. `release.yml` builds the five targets **at that tag** and publishes the GitHub Release
   with generated notes; the `latest` tag moves.

**Never move or delete a published tag.** A wrong release is fixed by the next release, not
by rewriting history. `main` has no branch protection, so the library gate in step 2 is the
only thing standing between a broken merge and a permanent tag.

### If a release failed

| What you see | What it means |
|---|---|
| Job exited cleanly, no tag | Nothing releasable in that push (docs/CI/chore only) |
| Red at "Compile and test the commit being tagged" | `main` is broken; fix it — that push releases the fix |
| Red at "Write the version and the changelog, then tag" | A push raced this job (non-fast-forward); the next push covers both |
| Tag exists but assets are missing | Re-run `release.yml` by hand with `tag` = the existing tag |

### Pre-release from a branch

`workflow_dispatch` on any branch other than `main` builds `dev-<version>-rc.<run>` using the
same `scripts/next-version.sh`, so a branch never claims a version the stable line will use.


## Release Frequency

**Recommended cadence**:
- **PATCH releases**: As needed for critical bugs (24h turnaround)
- **MINOR releases**: Weekly or bi-weekly for new features
- **MAJOR releases**: Quarterly or when breaking changes necessary

## Version History Reference

Check version history:
```bash
git tag -l "v*"  # List all version tags
git log --oneline --tags  # Show commits with tags
```

Example output:
```
v0.17.0 (HEAD -> main, tag: v0.17.0, origin/main)
v0.16.0
v0.15.1
v0.15.0
```

## Common Issues

### Issue: CI/CD Fails After the Tag Was Pushed

**Symptom**: the release build fails and a tag already exists

**Solution**: do not delete or move the tag — anything that already fetched it keeps a stale
copy, and a replacement would collide with the next release's version. Fix the build, then
re-run `release.yml` by hand against the existing tag:

```bash
gh workflow run release.yml -f tag=v0.17.0 -f prerelease=false
```

If the release itself is wrong rather than unbuilt, cut a follow-up: revert on `main` with a
`fix:` commit and let the release job publish the next patch.

### Issue: Version Mismatch

**Symptom**: `ptk --version` disagrees with the release tag

**Solution**: the release job bumps the manifests and tags in one step, so they agree by
construction; a mismatch means the tag was made by hand. Check the tag before blaming the
build:

```bash
git show v0.17.0:Cargo.toml | grep -m1 '^version'
```

### Issue: Changelog Merge Conflict

**Symptom**: CHANGELOG.md has conflicts after rebase

**Solution**: Do not edit CHANGELOG.md manually. It is written by the release job from conventional commit messages when a commit lands on `main`.

## Security Considerations

**Before releasing**:
- [ ] No secrets in code (API keys, tokens)
- [ ] No `.env` files committed
- [ ] Dependencies scanned (`cargo audit`)
- [ ] Shell injection vulnerabilities reviewed
- [ ] Cross-platform shell escaping tested

**Dependency audit**:
```bash
cargo install cargo-audit
cargo audit

# Example output:
# Crate: some-crate
# Version: 0.1.0
# Warning: vulnerability found
# Advisory: CVE-2024-XXXXX
```

If vulnerabilities found:
```bash
# Update vulnerable dependency
cargo update some-crate

# Verify fix
cargo audit

# Re-run quality checks
cargo test --all
```
