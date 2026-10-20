//! The environment variables ptk reads: ptk's own switches (`PTK_TEE=0`,
//! `PTK_NO_TOML=1`, …), its path overrides (`PTK_DB_PATH`, …), and the agent,
//! CI and tool variables it consults (`CLAUDE_CONFIG_DIR`, `CI`,
//! `COMPOSER_BIN_DIR`, …). Paths are read through `user_dirs::env_path`, which
//! resolves here.
//!
//! A test build does not read them from the process environment, which is the
//! developer's shell: an exported `PTK_NO_TOML=1` would switch the TOML engine
//! off under every test that expects it on, and an exported `CLAUDE_CONFIG_DIR`
//! would point a test at their real settings. A test sets the ones it needs
//! with `with_vars` or `with_path`, which hold them for the calling thread
//! only, so tests running in parallel never see each other's.
//!
//! Reading one anywhere else bypasses that, so
//! `test_isolation::only_user_dirs_resolves_user_locations` refuses it.

#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::collections::HashMap;
use std::ffi::OsString;

/// The variable's value, `None` when it is unset or not valid UTF-8.
///
/// In a test build, only what the calling thread set.
pub fn var(name: &str) -> Option<String> {
    var_os(name)?.into_string().ok()
}

/// The variable's value as the platform holds it, `None` when it is unset.
///
/// In a test build, only what the calling thread set.
///
/// A `PTK_*` name that is unset falls back to its pre-rename ancestor: the
/// fork renamed every `RTK_*` variable, and a configuration written for
/// upstream rtk (`RTK_DB_PATH`, `RTK_TEE=0`, …) must keep working. The new
/// name always wins, so a machine that sets both is not ambiguous.
pub fn var_os(name: &str) -> Option<OsString> {
    if let Some(value) = read_var_os(name) {
        return Some(value);
    }
    legacy_name(name).and_then(|legacy| read_var_os(&legacy))
}

/// One variable, read under its own name only — never its `RTK_*` ancestor.
///
/// The [`var_os`] fallback is right for a switch: `RTK_TEE=0` means exactly
/// what `PTK_TEE=0` means, so a setup written for rtk keeps its intent. It is
/// wrong for a path that names a store whose schema the fork changed. Pointing
/// ptk at rtk's live `history.db` makes every insert fail on the renamed
/// `ptk_cmd` column (rtk's table has `rtk_cmd`), and the two binaries end up
/// fighting over one file: whichever wrote last leaves the other unable to
/// record anything. A caller that opens a database reads it exactly, so the
/// two stay in their own files and a migrated user migrates on purpose rather
/// than by accident.
pub fn var_exact(name: &str) -> Option<String> {
    read_var_os(name)?.into_string().ok()
}

/// The current name of a variable that existed as `RTK_<suffix>` before the
/// rename, as that ancestor was spelled. `None` for anything else: only the
/// fork's own variables have an ancestor, and `CI` must not go looking for
/// `RCI`.
fn legacy_name(name: &str) -> Option<String> {
    name.strip_prefix("PTK_")
        .map(|suffix| format!("RTK_{suffix}"))
}

/// One variable, exactly as named — no legacy fallback.
fn read_var_os(name: &str) -> Option<OsString> {
    #[cfg(not(test))]
    {
        std::env::var_os(name)
    }
    #[cfg(test)]
    {
        SET.with(|set| set.borrow().get(name).cloned())
    }
}

#[cfg(test)]
thread_local! {
    static SET: RefCell<HashMap<String, OsString>> = RefCell::new(HashMap::new());
}

/// Run `f` with each variable set, or unset for `None`, as [`var`] and
/// [`var_os`] see it on this thread. The previous values come back when `f`
/// returns or panics.
#[cfg(test)]
pub fn with_vars<R>(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> R) -> R {
    set_while(
        vars.iter()
            .map(|&(name, value)| (name, value.map(OsString::from)))
            .collect(),
        f,
    )
}

/// [`with_vars`] for one variable holding a path.
#[cfg(test)]
pub fn with_path<P: AsRef<std::ffi::OsStr>, R>(
    name: &str,
    value: Option<P>,
    f: impl FnOnce() -> R,
) -> R {
    set_while(vec![(name, value.map(|v| v.as_ref().to_owned()))], f)
}

#[cfg(test)]
fn set_while<R>(vars: Vec<(&str, Option<OsString>)>, f: impl FnOnce() -> R) -> R {
    struct Restore(Vec<(String, Option<OsString>)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            SET.with(|set| {
                let mut set = set.borrow_mut();
                for (name, previous) in self.0.drain(..).rev() {
                    match previous {
                        Some(value) => set.insert(name, value),
                        None => set.remove(&name),
                    };
                }
            });
        }
    }

    let _restore = SET.with(|set| {
        let mut set = set.borrow_mut();
        Restore(
            vars.into_iter()
                .map(|(name, value)| {
                    let previous = match value {
                        Some(value) => set.insert(name.to_string(), value),
                        None => set.remove(name),
                    };
                    (name.to_string(), previous)
                })
                .collect(),
        )
    });
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_process_environment_is_not_read() {
        // Any variable the process has: a test build that fell back to the
        // process environment would see it.
        let Some(name) = std::env::vars_os().find_map(|(name, _)| name.into_string().ok()) else {
            return;
        };
        assert_eq!(var_os(&name), None, "{name} leaked through");
    }

    #[test]
    fn values_nest_and_come_back() {
        with_vars(&[("PTK_X", Some("1"))], || {
            assert_eq!(var("PTK_X").as_deref(), Some("1"));
            with_vars(&[("PTK_X", None), ("PTK_Y", Some("2"))], || {
                assert_eq!(var("PTK_X"), None);
                assert_eq!(var("PTK_Y").as_deref(), Some("2"));
            });
            assert_eq!(var("PTK_X").as_deref(), Some("1"));
            assert_eq!(var("PTK_Y"), None);
        });
        assert_eq!(var("PTK_X"), None);
    }

    #[test]
    fn a_legacy_rtk_name_is_still_read() {
        // PTK renamed every RTK_* variable; a setup written for rtk must keep
        // working, so the old name is read when the new one is unset.
        with_vars(&[("RTK_TEE", Some("0"))], || {
            assert_eq!(var("PTK_TEE").as_deref(), Some("0"));
            assert_eq!(
                var_os("PTK_TEE").as_deref(),
                Some(std::ffi::OsStr::new("0"))
            );
        });
    }

    #[test]
    fn the_ptk_name_wins_over_the_legacy_one() {
        with_vars(&[("PTK_TEE", Some("1")), ("RTK_TEE", Some("0"))], || {
            assert_eq!(var("PTK_TEE").as_deref(), Some("1"));
        });
    }

    #[test]
    fn a_name_without_the_ptk_prefix_has_no_legacy_alias() {
        // Only PTK_* has an RTK_* ancestor: `CI`, `CLAUDE_CONFIG_DIR` and the
        // rest must not go looking for a variable that never existed.
        with_vars(&[("RCI", Some("1")), ("CI", None)], || {
            assert_eq!(var("CI"), None);
        });
    }

    #[test]
    fn another_thread_does_not_see_them() {
        with_path("PTK_X", Some("/somewhere"), || {
            assert!(var_os("PTK_X").is_some());
            let seen = std::thread::spawn(|| var_os("PTK_X"))
                .join()
                .expect("thread");
            assert_eq!(seen, None);
        });
    }
}
