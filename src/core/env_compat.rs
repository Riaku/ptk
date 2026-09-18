//! Environment variable compatibility layer.
//!
//! PTK renamed all `RTK_*` environment variables to `PTK_*`. To avoid breaking
//! existing setups, every read prefers the new `PTK_` name and falls back to the
//! legacy `RTK_` name when the new one is unset.

use std::env::VarError;

/// Read `PTK_<suffix>`, falling back to the legacy `RTK_<suffix>` when unset.
///
/// `suffix` is the part after the prefix, e.g. `"DB_PATH"` resolves
/// `PTK_DB_PATH` first, then `RTK_DB_PATH`.
pub fn env_compat(suffix: &str) -> Result<String, VarError> {
    match std::env::var(format!("PTK_{suffix}")) {
        Ok(v) => Ok(v),
        Err(VarError::NotPresent) => std::env::var(format!("RTK_{suffix}")),
        Err(e) => Err(e),
    }
}

/// Check whether `PTK_<suffix>` (or legacy `RTK_<suffix>`) is present at all.
pub fn env_compat_present(suffix: &str) -> bool {
    env_compat(suffix).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_ptk_over_rtk() {
        temp_env::with_vars(
            [
                ("PTK_COMPAT_TEST", Some("new")),
                ("RTK_COMPAT_TEST", Some("old")),
            ],
            || {
                assert_eq!(env_compat("COMPAT_TEST").unwrap(), "new");
            },
        );
    }

    #[test]
    fn falls_back_to_rtk_when_ptk_unset() {
        temp_env::with_vars(
            [
                ("PTK_COMPAT_TEST", None::<&str>),
                ("RTK_COMPAT_TEST", Some("old")),
            ],
            || {
                assert_eq!(env_compat("COMPAT_TEST").unwrap(), "old");
            },
        );
    }

    #[test]
    fn missing_both_returns_not_present() {
        temp_env::with_vars(
            [
                ("PTK_COMPAT_TEST", None::<&str>),
                ("RTK_COMPAT_TEST", None::<&str>),
            ],
            || {
                assert!(matches!(
                    env_compat("COMPAT_TEST"),
                    Err(VarError::NotPresent)
                ));
                assert!(!env_compat_present("COMPAT_TEST"));
            },
        );
    }
}
