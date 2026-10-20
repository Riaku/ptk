//! Routing the CMD twins to their PowerShell equivalents off Windows.
//!
//! Only two of the CMD built-ins are worth routing, and the test is whether
//! PowerShell itself knows the name. `dir` and `type` are real PowerShell
//! aliases (`Get-ChildItem` and `Get-Content`), so a Linux user or agent
//! typing them means something that exists here — and ptk's filtered twins are
//! the better answer than the raw coreutils binary.
//!
//! The other CMD built-ins (`findstr`, `tasklist`, `netstat`, `ipconfig`,
//! `robocopy`, `certutil`, `schtasks`, `reg`) are not PowerShell commands on
//! any platform. They are deliberately not routed: faking them would hide the
//! fact that the Linux path is `ptk sls`, `ptk ps` and friends, and the
//! native "command not found" is a more useful failure than a translation the
//! user never asked for. The rewrite layer drops those rules off Windows for
//! the same reason — see `discover::rules::PtkRule::windows_only`.
//!
//! Switch mapping keeps the CMD spelling working: `/s` recurses, `/b` wants
//! bare names, `/a` selects attributes. The switches that only lay the table
//! for a human reading a console (`/w`, `/p`, `/o`) are dropped quietly
//! because dropping them loses nothing the caller asked for.

/// What a CMD argument list became.
#[derive(Debug, PartialEq, Eq)]
pub struct Translated {
    pub args: Vec<String>,
    /// Switches the translator could not carry over, in the form the user typed.
    pub dropped: Vec<String>,
}

impl Translated {
    fn new() -> Self {
        Self {
            args: Vec::new(),
            dropped: Vec::new(),
        }
    }

    fn push(&mut self, arg: impl Into<String>) {
        self.args.push(arg.into());
    }
}

/// CMD `dir` switches that mean something to `Get-ChildItem`.
const DIR_SEMANTIC_SWITCHES: &[&str] = &["s", "b", "a", "ad", "a-d"];

/// CMD `dir` switches that only change console presentation.
const DIR_PRESENTATION_SWITCHES: &[&str] = &[
    "w", "p", "q", "r", "x", "l", "n", "c", "d", "4", "o", "on", "od", "os", "ow", "oe", "o-n",
    "o-d", "o-s", "o-w", "o-e",
];

/// Recognizes a CMD switch token against a known set.
///
/// The known set is what makes this safe off Windows: a bare "starts with a
/// slash" rule reads `/etc` and `/tmp` as switches, which turns every absolute
/// path into a dropped argument. Anything not in `known` is a positional, not
/// a switch.
fn switch(arg: &str, known: &[&'static str]) -> Option<&'static str> {
    let rest = arg.strip_prefix('/').or_else(|| arg.strip_prefix('-'))?;
    let lower = rest.to_lowercase();
    known.iter().find(|&&k| k == lower).copied()
}

/// CMD `dir` -> `Get-ChildItem`.
pub fn dir_to_get_child_item(args: &[String]) -> Translated {
    let mut out = Translated::new();
    let mut force = false;
    let mut directory = false;
    let mut file = false;

    for arg in args {
        let sw = switch(
            arg,
            &[DIR_SEMANTIC_SWITCHES, DIR_PRESENTATION_SWITCHES].concat(),
        );

        match sw {
            Some("s") => out.push("-Recurse"),
            Some("b") => out.push("-Name"),
            Some("a") => force = true,
            // /AD selects directories; /A-D excludes them, i.e. plain files.
            Some("ad") => {
                directory = true;
                force = true;
            }
            Some("a-d") => {
                file = true;
                force = true;
            }
            // Presentation-only: wide columns, paging, sort order, short names.
            Some(p) if DIR_PRESENTATION_SWITCHES.contains(&p) => {}
            // Not a switch we know: a path, a PowerShell flag, or a pattern.
            _ => out.push(arg.clone()),
        }
    }

    if directory {
        out.push("-Directory");
    }
    if file {
        out.push("-File");
    }
    if force {
        out.push("-Force");
    }

    out
}

/// CMD `type` -> `Get-Content`.
///
/// CMD concatenates every file given, and so does `Get-Content` — but only when
/// the paths arrive as one array. Two bare paths bound positionally fail with
/// "A positional parameter cannot be found that accepts argument", so multiple
/// files go through `-Path` comma-joined.
pub fn type_to_get_content(args: &[String]) -> Translated {
    let mut out = Translated::new();
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();

    for arg in args.iter().filter(|a| a.starts_with('-')) {
        out.push(arg.clone());
    }

    match files.len() {
        0 => {}
        1 => out.push(files[0].clone()),
        _ => {
            out.push("-Path");
            out.push(
                files
                    .iter()
                    .map(|f| f.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
    }

    out
}

/// Reports the CMD switches that could not be carried over to PowerShell.
///
/// Silence here would be the worse failure: a dropped switch can invert the
/// meaning of a search, so the user would find out from a wrong result rather
/// than an error. This goes to stderr so it never pollutes a piped stdout.
pub fn warn_dropped(builtin: &str, dropped: &[String]) {
    if dropped.is_empty() {
        return;
    }
    eprintln!(
        "ptk: no PowerShell equivalent for {} in `{}` — run it through pwsh if you need it",
        dropped.join(", "),
        builtin
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn dir_switches_map_to_get_child_item() {
        let t = dir_to_get_child_item(&s(&["/s", "/tmp"]));
        assert_eq!(t.args, s(&["-Recurse", "/tmp"]));
        assert!(t.dropped.is_empty());

        let t = dir_to_get_child_item(&s(&["/b"]));
        assert_eq!(t.args, s(&["-Name"]));
    }

    #[test]
    fn an_absolute_path_is_never_a_switch() {
        // The bug this guards: `/etc` reads as a switch under a bare
        // "starts with a slash" rule, so the path gets dropped and the command
        // silently lists the wrong directory.
        for path in ["/etc", "/tmp", "/opt/data", "/var/log"] {
            let t = dir_to_get_child_item(&s(&[path]));
            assert_eq!(t.args, s(&[path]), "{} must survive as a path", path);
            assert!(t.dropped.is_empty());
        }
    }

    #[test]
    fn dir_attribute_switches_become_powershell_filters() {
        let t = dir_to_get_child_item(&s(&["/ad"]));
        assert!(t.args.contains(&"-Directory".to_string()));
        assert!(t.args.contains(&"-Force".to_string()));

        let t = dir_to_get_child_item(&s(&["/a-d"]));
        assert!(t.args.contains(&"-File".to_string()));

        let t = dir_to_get_child_item(&s(&["/a"]));
        assert_eq!(t.args, s(&["-Force"]));
    }

    #[test]
    fn dir_presentation_switches_are_dropped_without_complaint() {
        let t = dir_to_get_child_item(&s(&["/w", "/p", "/on", "/tmp"]));
        assert_eq!(t.args, s(&["/tmp"]));
        assert!(
            t.dropped.is_empty(),
            "presentation switches lose nothing: {:?}",
            t.dropped
        );
    }

    #[test]
    fn powershell_flags_pass_through_untouched() {
        let t = dir_to_get_child_item(&s(&["-Recurse", "-Force", "/etc"]));
        assert_eq!(t.args, s(&["-Recurse", "-Force", "/etc"]));
    }

    #[test]
    fn type_arguments_carry_straight_over() {
        let t = type_to_get_content(&s(&["/etc/hostname"]));
        assert_eq!(t.args, s(&["/etc/hostname"]));
        assert!(t.dropped.is_empty());
    }

    #[test]
    fn type_with_several_files_uses_a_path_array() {
        // Two bare paths bind positionally in PowerShell and fail outright, so
        // more than one file has to arrive under -Path.
        let t = type_to_get_content(&s(&["/etc/hostname", "/etc/hosts"]));
        assert_eq!(t.args, s(&["-Path", "/etc/hostname,/etc/hosts"]));
    }
}
