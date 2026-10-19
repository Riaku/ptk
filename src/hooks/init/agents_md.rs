//! Instructions-file block helpers shared by every agent that writes one (PTK block upsert and
//! removal), and the AGENTS.md reference helpers the Codex flow uses.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum PtkBlockUpsert {
    /// No existing block found — appended new block
    Added,
    /// Existing block found with different content — replaced
    Updated,
    /// Existing block found with identical content — no-op
    Unchanged,
    /// Opening marker found without closing marker — not safe to rewrite
    Malformed,
}

/// Insert or replace the PTK instructions block in `content`.
///
/// Returns `(new_content, action)` describing what happened.
/// The caller decides whether to write `new_content` based on `action`.
fn upsert_ptk_block(content: &str, block: &str) -> (String, PtkBlockUpsert) {
    let start_marker = PTK_BLOCK_START;
    let end_marker = PTK_BLOCK_END;

    if let Some(start) = content.find(start_marker) {
        if let Some(relative_end) = content[start..].find(end_marker) {
            let end = start + relative_end;
            let end_pos = end + end_marker.len();
            let current_block = content[start..end_pos].trim();
            let desired_block = block.trim();

            if current_block == desired_block {
                return (content.to_string(), PtkBlockUpsert::Unchanged);
            }

            // Replace stale block with desired block
            let before = content[..start].trim_end();
            let after = content[end_pos..].trim_start();

            let result = match (before.is_empty(), after.is_empty()) {
                (true, true) => desired_block.to_string(),
                (true, false) => format!("{desired_block}\n\n{after}"),
                (false, true) => format!("{before}\n\n{desired_block}"),
                (false, false) => format!("{before}\n\n{desired_block}\n\n{after}"),
            };

            return (result, PtkBlockUpsert::Updated);
        }

        // Opening marker without closing marker — malformed
        return (content.to_string(), PtkBlockUpsert::Malformed);
    }

    // No existing block — append
    let trimmed = content.trim();
    if trimmed.is_empty() {
        (block.to_string(), PtkBlockUpsert::Added)
    } else {
        (
            format!("{trimmed}\n\n{}", block.trim()),
            PtkBlockUpsert::Added,
        )
    }
}

/// Idempotently write an PTK-owned marker block into `path`, preserving user content.
///
/// Reads the file (if any), passes it through [`upsert_ptk_block`], and writes the
/// result back via [`atomic_write`]. Refuses to modify files containing an opening
/// marker without a matching closing marker (bails with a diagnostic and the exact
/// `recovery_cmd` to re-run after manual cleanup).
///
/// Returns the [`PtkBlockUpsert`] action so callers can branch on whether anything
/// was actually changed (e.g., to skip post-install steps on `Unchanged`).
///
/// `label` is shown in user-facing messages (e.g., `"ptk instructions"`,
/// `"Copilot instructions"`).
pub(super) fn write_ptk_block(
    path: &Path,
    block: &str,
    label: &str,
    recovery_cmd: &str,
    ctx: InitContext,
) -> Result<PtkBlockUpsert> {
    let InitContext { dry_run, .. } = ctx;

    let existing = if path.exists() {
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?
    } else {
        String::new()
    };

    let (new_content, action) = upsert_ptk_block(&existing, block);

    match action {
        PtkBlockUpsert::Added => {
            if dry_run {
                println!("[dry-run] would add {} to {}", label, path.display());
            } else {
                atomic_write(path, &new_content)
                    .with_context(|| format!("Failed to write {}", path.display()))?;
                println!("[ok] Added {} to {}", label, path.display());
            }
        }
        PtkBlockUpsert::Updated => {
            if dry_run {
                println!("[dry-run] would update {} in {}", label, path.display());
            } else {
                atomic_write(path, &new_content)
                    .with_context(|| format!("Failed to write {}", path.display()))?;
                println!("[ok] Updated {} in {}", label, path.display());
            }
        }
        PtkBlockUpsert::Unchanged => {
            if !dry_run {
                println!("[ok] {} already up to date in {}", label, path.display());
            }
        }
        PtkBlockUpsert::Malformed => {
            eprintln!(
                "[warn] Found '{}' without closing marker in {}",
                PTK_BLOCK_START,
                path.display()
            );
            if let Some((line_num, _)) = existing
                .lines()
                .enumerate()
                .find(|(_, line)| line.contains(PTK_BLOCK_START))
            {
                eprintln!("    Location: line {}", line_num + 1);
            }
            eprintln!("    Action: Manually remove the incomplete block, then re-run:");
            eprintln!("            {recovery_cmd}");
            anyhow::bail!(
                "Refusing to modify malformed {} at {}",
                label,
                path.display()
            );
        }
    }

    Ok(action)
}

/// Patch AGENTS.md: add @PTK.md (or absolute path), migrate old inline block if present
pub(super) fn patch_agents_md(path: &Path, ptk_md_ref: &str, ctx: InitContext) -> Result<bool> {
    let InitContext {
        verbose, dry_run, ..
    } = ctx;
    let mut content = if path.exists() {
        fs::read_to_string(path)
            .with_context(|| format!("Failed to read AGENTS.md: {}", path.display()))?
    } else {
        String::new()
    };

    let mut migrated = false;
    if content.contains(PTK_BLOCK_START) {
        let (new_content, did_migrate) = remove_ptk_block(&content);
        if did_migrate {
            content = new_content;
            migrated = true;
            if verbose > 0 {
                eprintln!("Migrated: removed old PTK block from AGENTS.md");
            }
        }
    }

    // ISSUE #892: Check for both relative and absolute @PTK.md references
    if content.contains(PTK_MD_REF) || content.contains(ptk_md_ref) {
        if verbose > 0 {
            eprintln!("{} reference already present in AGENTS.md", ptk_md_ref);
        }
        // ISSUE #892: Migrate old relative @PTK.md to absolute path if needed
        if ptk_md_ref != PTK_MD_REF && content.contains(PTK_MD_REF) && !content.contains(ptk_md_ref)
        {
            content = content.replace(PTK_MD_REF, ptk_md_ref);
            if dry_run {
                println!(
                    "[dry-run] would migrate {} to {} in {}",
                    PTK_MD_REF,
                    ptk_md_ref,
                    path.display()
                );
            } else {
                atomic_write(path, &content)
                    .with_context(|| format!("Failed to write AGENTS.md: {}", path.display()))?;
                if verbose > 0 {
                    eprintln!("Migrated {} to {}", PTK_MD_REF, ptk_md_ref);
                }
            }
            return Ok(true);
        }
        if migrated {
            if dry_run {
                println!(
                    "[dry-run] would write migrated AGENTS.md: {}",
                    path.display()
                );
            } else {
                atomic_write(path, &content)
                    .with_context(|| format!("Failed to write AGENTS.md: {}", path.display()))?;
            }
        }
        return Ok(false);
    }

    let new_content = if content.is_empty() {
        format!("{}\n", ptk_md_ref)
    } else {
        format!("{}\n\n{}\n", content.trim(), ptk_md_ref)
    };

    if dry_run {
        println!(
            "[dry-run] would add {} reference to AGENTS.md: {}",
            ptk_md_ref,
            path.display()
        );
        if verbose > 0 {
            println!("[dry-run] content:\n{}", new_content);
        }
    } else {
        atomic_write(path, &new_content)
            .with_context(|| format!("Failed to write AGENTS.md: {}", path.display()))?;
        if verbose > 0 {
            eprintln!("Added {} reference to AGENTS.md", ptk_md_ref);
        }
    }

    Ok(true)
}

pub(super) fn has_ptk_reference(content: &str, refs: &[&str]) -> bool {
    content
        .lines()
        .map(str::trim)
        .any(|line| refs.contains(&line))
}

pub(super) fn remove_ptk_reference_from_agents(
    path: &Path,
    refs: &[&str],
    ctx: InitContext,
) -> Result<bool> {
    let InitContext {
        verbose, dry_run, ..
    } = ctx;
    if !path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read AGENTS.md: {}", path.display()))?;
    if !has_ptk_reference(&content, refs) {
        return Ok(false);
    }

    let new_content = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !refs.contains(&trimmed)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let cleaned = clean_double_blanks(&new_content);

    if dry_run {
        println!(
            "[dry-run] would remove PTK.md reference from AGENTS.md: {}",
            path.display()
        );
        if verbose > 0 {
            println!("[dry-run] content:\n{}", cleaned);
        }
        return Ok(true);
    }

    atomic_write(path, &cleaned)
        .with_context(|| format!("Failed to write AGENTS.md: {}", path.display()))?;

    if verbose > 0 {
        eprintln!(
            "Removed PTK.md reference from AGENTS.md: {}",
            path.display()
        );
    }

    Ok(true)
}

/// Strip the inline PTK block from an instructions file's content, returning the cleaned
/// text and whether a block was removed.
pub(super) fn remove_ptk_block(content: &str) -> (String, bool) {
    if let Some(start) = content.find(PTK_BLOCK_START)
        && let Some(relative_end) = content[start..].find(PTK_BLOCK_END)
    {
        let end_pos = start + relative_end + PTK_BLOCK_END.len();
        let before = content[..start].trim_end();
        let after = content[end_pos..].trim_start();

        let result = if after.is_empty() {
            format!("{}\n", before)
        } else {
            format!("{}\n\n{}", before, after)
        };

        (result, true) // migrated
    } else if content.contains(PTK_BLOCK_START) {
        eprintln!(
            "[warn] Warning: Found '{}' without closing marker.",
            PTK_BLOCK_START
        );
        eprintln!("    This can happen if CLAUDE.md was manually edited.");

        if let Some((line_num, _)) = content
            .lines()
            .enumerate()
            .find(|(_, line)| line.contains(PTK_BLOCK_START))
        {
            eprintln!("    Location: line {}", line_num + 1);
        }

        eprintln!("    Action: Manually remove the incomplete block, then re-run:");
        eprintln!("            ptk init -g");
        (content.to_string(), false)
    } else {
        (content.to_string(), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_remove_ptk_block_ignores_end_marker_before_start() {
        let before =
            format!("# My rules\n\nThe closing marker is `{PTK_BLOCK_END}`.\nKeep these notes.");
        let input = format!("{before}\n\n{PTK_INSTRUCTIONS}\n\nMore user content.");

        let (cleaned, removed) = remove_ptk_block(&input);

        assert!(removed);
        assert_eq!(cleaned, format!("{before}\n\nMore user content."));
        assert_eq!(remove_ptk_block(&cleaned), (cleaned.clone(), false));
    }

    #[test]
    fn test_remove_ptk_block_preserves_unmatched_markers() {
        for input in [
            "# User rules\n".to_string(),
            format!("# User rules\n{PTK_BLOCK_END}\nKeep this.\n"),
            format!("# User rules\n{PTK_BLOCK_START} v2 -->\nKeep this.\n"),
            format!(
                "# User rules\n{PTK_BLOCK_END}\nKeep this.\n{PTK_BLOCK_START} v2 -->\nUnfinished.\n"
            ),
        ] {
            assert_eq!(remove_ptk_block(&input), (input.clone(), false));
        }
    }

    #[test]
    fn test_remove_ptk_block_stops_at_first_end_marker() {
        let after = format!("The closing marker is `{PTK_BLOCK_END}`.\nKeep these notes.");
        let input = format!("# My rules\n\n{PTK_INSTRUCTIONS}\n\n{after}\n");

        let (cleaned, removed) = remove_ptk_block(&input);

        assert!(removed);
        assert_eq!(cleaned, format!("# My rules\n\n{after}\n"));
    }

    #[test]
    fn test_upsert_ptk_block_appends_when_missing() {
        let input = "# Team instructions";
        let (content, action) = upsert_ptk_block(input, PTK_INSTRUCTIONS);
        assert_eq!(action, PtkBlockUpsert::Added);
        assert!(content.contains("# Team instructions"));
        assert!(content.contains(PTK_BLOCK_START));
    }

    #[test]
    fn test_upsert_ptk_block_updates_stale_block() {
        let input = format!(
            "# Team instructions\n\n{} v1 -->\nOLD PTK CONTENT\n{}\n\nMore notes\n",
            PTK_BLOCK_START, PTK_BLOCK_END
        );

        let (content, action) = upsert_ptk_block(&input, PTK_INSTRUCTIONS);
        assert_eq!(action, PtkBlockUpsert::Updated);
        assert!(!content.contains("OLD PTK CONTENT"));
        assert!(content.contains("ptk cargo test")); // from current PTK_INSTRUCTIONS
        assert!(content.contains("# Team instructions"));
        assert!(content.contains("More notes"));
    }

    #[test]
    fn test_upsert_ptk_block_noop_when_already_current() {
        let input = format!(
            "# Team instructions\n\n{}\n\nMore notes\n",
            PTK_INSTRUCTIONS
        );
        let (content, action) = upsert_ptk_block(&input, PTK_INSTRUCTIONS);
        assert_eq!(action, PtkBlockUpsert::Unchanged);
        assert_eq!(content, input);
    }

    #[test]
    fn test_upsert_ptk_block_detects_malformed_block() {
        let input = format!("{} v2 -->\npartial", PTK_BLOCK_START);
        let (content, action) = upsert_ptk_block(&input, PTK_INSTRUCTIONS);
        assert_eq!(action, PtkBlockUpsert::Malformed);
        assert_eq!(content, input);
    }

    #[test]
    fn test_patch_agents_md_adds_reference_once() {
        let temp = TempDir::new().unwrap();
        let agents_md = temp.path().join("AGENTS.md");

        fs::write(&agents_md, "# Team rules\n").unwrap();
        let first_added = patch_agents_md(&agents_md, PTK_MD_REF, InitContext::default()).unwrap();
        let second_added = patch_agents_md(&agents_md, PTK_MD_REF, InitContext::default()).unwrap();

        assert!(first_added);
        assert!(!second_added);

        let content = fs::read_to_string(&agents_md).unwrap();
        assert_eq!(content.matches("@PTK.md").count(), 1);
    }

    #[test]
    fn test_patch_agents_md_creates_missing_file() {
        let temp = TempDir::new().unwrap();
        let agents_md = temp.path().join("AGENTS.md");

        let added = patch_agents_md(&agents_md, PTK_MD_REF, InitContext::default()).unwrap();

        assert!(added);
        let content = fs::read_to_string(&agents_md).unwrap();
        assert_eq!(content, "@PTK.md\n");
    }

    #[test]
    fn test_patch_agents_md_migrates_inline_block() {
        let temp = TempDir::new().unwrap();
        let agents_md = temp.path().join("AGENTS.md");
        fs::write(
            &agents_md,
            format!(
                "# Team rules\n\n{} v2 -->\nold\n{}\n",
                PTK_BLOCK_START, PTK_BLOCK_END
            ),
        )
        .unwrap();

        let added = patch_agents_md(&agents_md, PTK_MD_REF, InitContext::default()).unwrap();

        assert!(added);
        let content = fs::read_to_string(&agents_md).unwrap();
        assert!(!content.contains("old"));
        assert_eq!(content.matches("@PTK.md").count(), 1);
    }
}
