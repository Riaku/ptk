use super::report::PtkStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineSafety {
    None,
    ProducerOnly,
    #[allow(dead_code)]
    FinalOnly,
    Both,
}

impl PipelineSafety {
    pub fn producer_safe(self) -> bool {
        matches!(self, Self::ProducerOnly | Self::Both)
    }

    pub fn final_safe(self) -> bool {
        matches!(self, Self::FinalOnly | Self::Both)
    }
}

pub struct PtkRule {
    pub pattern: &'static str,
    pub ptk_cmd: &'static str,
    /// Pipeline stage positions this command may be rewritten in (#3171).
    pub pipeline_safety: PipelineSafety,
    pub rewrite_prefixes: &'static [&'static str],
    pub category: &'static str,
    pub savings_pct: f64,
    pub subcmd_savings: &'static [(&'static str, f64)],
    pub subcmd_status: &'static [(&'static str, PtkStatus)],
    /// The target is a Windows-only binary with no PowerShell Core equivalent on
    /// Linux, so the rule must not fire there. Rewriting `tasklist` to
    /// `ptk tasklist` on Linux sends the agent into the exec fallback, which
    /// dies with `No such file or directory` — a worse outcome than the native
    /// `command not found` the shell would have given. Leaving the command alone
    /// lets the real error surface.
    pub windows_only: bool,
}

impl PtkRule {
    // `Default::default()` isn't a const fn on stable, so `RULES` (a const array) can't call
    // it via `..Default::default()` — hence this associated const as the const-context
    // workaround. Once `const_trait_impl` stabilizes, drop this and derive/impl
    // `const Default` instead, then switch call sites back to `..PtkRule::default()`.
    pub const DEFAULT: PtkRule = PtkRule {
        pattern: "",
        ptk_cmd: "",
        pipeline_safety: PipelineSafety::None,
        rewrite_prefixes: &[],
        category: "",
        savings_pct: 60.0,
        subcmd_savings: &[],
        subcmd_status: &[],
        windows_only: false,
    };
}

impl Default for PtkRule {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A rule declaring subcommands also needs its tool in
/// `core::tracking::SUBCOMMAND_ROUTERS`, or its telemetry label stops at the tool name.
pub const RULES: &[PtkRule] = &[
    PtkRule {
        pattern: r"^(?:git|yadm)\s+(?:-[Cc]\s+\S+\s+)*(status|log|diff|show|add|commit|checkout|push|pull|branch|fetch|stash|worktree)(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk git",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["git", "yadm"],
        category: "Git",
        savings_pct: 70.0,
        subcmd_savings: &[
            ("diff", 80.0),
            ("show", 80.0),
            ("add", 59.0),
            ("commit", 59.0),
        ],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^gh\s+(pr|issue|run|repo|api|release)(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk gh",
        rewrite_prefixes: &["gh"],
        category: "GitHub",
        savings_pct: 82.0,
        subcmd_savings: &[("pr", 87.0), ("run", 82.0), ("issue", 80.0)],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^glab\s+(mr|issue|ci|pipeline|api|release)(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk glab",
        rewrite_prefixes: &["glab"],
        category: "GitLab",
        savings_pct: 82.0,
        subcmd_savings: &[("mr", 87.0), ("ci", 82.0), ("issue", 80.0)],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^cargo\s+(build|test|clippy|check|fmt|install)(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk cargo",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["cargo"],
        category: "Cargo",
        savings_pct: 80.0,
        subcmd_savings: &[("test", 90.0), ("check", 80.0)],
        subcmd_status: &[("fmt", PtkStatus::Passthrough)],
        windows_only: false,
    },
    PtkRule {
        pattern: r"^pnpm\s+(exec|i|install|list|ls|outdated|run|run-script)",
        ptk_cmd: "ptk pnpm",
        rewrite_prefixes: &["pnpm"],
        category: "PackageManager",
        savings_pct: 80.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^npm\s+(exec|run|run-script|rum|urn|x)(\s|$)",
        ptk_cmd: "ptk npm",
        rewrite_prefixes: &["npm"],
        category: "PackageManager",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^npx\s+",
        ptk_cmd: "ptk npx",
        rewrite_prefixes: &["npx"],
        category: "PackageManager",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(cat|head|tail)\s+",
        ptk_cmd: "ptk read",
        rewrite_prefixes: &["cat", "head", "tail"],
        category: "Files",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^grep\s+",
        ptk_cmd: "ptk grep",
        pipeline_safety: PipelineSafety::Both,
        rewrite_prefixes: &["grep"],
        category: "Files",
        savings_pct: 75.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^rg\s+",
        ptk_cmd: "ptk rg",
        pipeline_safety: PipelineSafety::Both,
        rewrite_prefixes: &["rg"],
        category: "Files",
        savings_pct: 75.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^ast-grep\s+",
        ptk_cmd: "ptk ast-grep",
        // Unlike grep/rg, `ptk ast-grep`'s run() captures with stdin null on the
        // path it filters, so it must not be rewritten as a pipeline's final
        // stage — that would silently drop the pipe input. `run --stdin` and the
        // other subcommands do reach the child's stdin, but only when invoked
        // directly: this rule keeps the hook from producing that form at all.
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["ast-grep"],
        category: "Files",
        savings_pct: 85.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^ls(\s|$)",
        ptk_cmd: "ptk ls",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["ls"],
        category: "Files",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^find\s+",
        ptk_cmd: "ptk find",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["find"],
        category: "Files",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^((p?np(m|x)|p?npm\s+(exec|run|run-script)|npm\s+(rum|urn|x)|pnpm\s+dlx)\s+)?tsc(\s|$)",
        ptk_cmd: "ptk tsc",
        rewrite_prefixes: &[
            "npm exec tsc",
            "npm rum tsc",
            "npm run tsc",
            "npm run-script tsc",
            "npm tsc",
            "npm urn tsc",
            "npm x tsc",
            "npx tsc",
            "pnpm dlx tsc",
            "pnpm exec tsc",
            "pnpm run tsc",
            "pnpm run-script tsc",
            "pnpm tsc",
            "pnpx tsc",
            "tsc",
        ],
        category: "Build",
        savings_pct: 83.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^((p?np(m|x)|p?npm\s+(exec|run|run-script)|npm\s+(rum|urn|x)|pnpm\s+dlx)\s+)?(biome|eslint|lint)(\s|$)",
        ptk_cmd: "ptk lint",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &[
            "biome",
            "eslint",
            "lint",
            "npm biome",
            "npm eslint",
            "npm exec biome",
            "npm exec eslint",
            "npm lint",
            "npm rum biome",
            "npm rum eslint",
            "npm rum lint",
            "npm run biome",
            "npm run eslint",
            "npm run lint",
            "npm run-script biome",
            "npm run-script eslint",
            "npm run-script lint",
            "npm urn biome",
            "npm urn eslint",
            "npm urn lint",
            "npm x biome",
            "npm x eslint",
            "npx biome",
            "npx eslint",
            "npx lint",
            "pnpm biome",
            "pnpm dlx biome",
            "pnpm dlx eslint",
            "pnpm eslint",
            "pnpm exec biome",
            "pnpm exec eslint",
            "pnpm lint",
            "pnpm run biome",
            "pnpm run eslint",
            "pnpm run lint",
            "pnpm run-script biome",
            "pnpm run-script eslint",
            "pnpm run-script lint",
            "pnpx biome",
            "pnpx eslint",
            "pnpx lint",
        ],
        category: "Build",
        savings_pct: 84.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^((p?np(m|x)|p?npm\s+(exec|run|run-script)|npm\s+(rum|urn|x)|pnpm\s+dlx)\s+)?prettier(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk prettier",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &[
            "npm exec prettier",
            "npm prettier",
            "npm rum prettier",
            "npm run prettier",
            "npm run-script prettier",
            "npm urn prettier",
            "npm x prettier",
            "npx prettier",
            "pnpm dlx prettier",
            "pnpm exec prettier",
            "pnpm prettier",
            "pnpm run prettier",
            "pnpm run-script prettier",
            "pnpx prettier",
            "prettier",
        ],
        category: "Build",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^((p?np(m|x)|p?npm\s+(exec|run|run-script)|npm\s+(rum|urn|x)|pnpm\s+dlx)\s+)?next\s+build(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk next",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &[
            "next build",
            "npm exec next build",
            "npm next build",
            "npm rum next build",
            "npm run next build",
            "npm run-script next build",
            "npm urn next build",
            "npm x next build",
            "npx next build",
            "pnpm dlx next build",
            "pnpm exec next build",
            "pnpm next build",
            "pnpm run next build",
            "pnpm run-script next build",
            "pnpx next build",
        ],
        category: "Build",
        savings_pct: 87.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^((p?np(m|x)|p?npm\s+(exec|run|run-script)|npm\s+(rum|urn|x)|pnpm\s+dlx)\s+)?jest(\s+run)?(\s|$)",
        ptk_cmd: "ptk jest",
        rewrite_prefixes: &[
            "jest run",
            "jest",
            "npm exec jest run",
            "npm exec jest",
            "npm jest run",
            "npm jest",
            "npm rum jest run",
            "npm rum jest",
            "npm run jest run",
            "npm run jest",
            "npm run-script jest run",
            "npm run-script jest",
            "npm urn jest run",
            "npm urn jest",
            "npm x jest run",
            "npm x jest",
            "npx jest run",
            "npx jest",
            "pnpm dlx jest run",
            "pnpm dlx jest",
            "pnpm exec jest run",
            "pnpm exec jest",
            "pnpm jest run",
            "pnpm jest",
            "pnpm run jest run",
            "pnpm run jest",
            "pnpm run-script jest run",
            "pnpm run-script jest",
            "pnpx jest run",
            "pnpx jest",
        ],
        category: "Tests",
        savings_pct: 99.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^((p?np(m|x)|p?npm\s+(exec|run|run-script)|npm\s+(rum|urn|x)|pnpm\s+dlx)\s+)?vitest(\s+run)?(\s|$)",
        ptk_cmd: "ptk vitest",
        rewrite_prefixes: &[
            "npm exec vitest run",
            "npm exec vitest",
            "npm rum vitest run",
            "npm rum vitest",
            "npm run vitest run",
            "npm run vitest",
            "npm run-script vitest run",
            "npm run-script vitest",
            "npm urn vitest run",
            "npm urn vitest",
            "npm vitest run",
            "npm vitest",
            "npm x vitest run",
            "npm x vitest",
            "npx vitest run",
            "npx vitest",
            "pnpm dlx vitest run",
            "pnpm dlx vitest",
            "pnpm exec vitest run",
            "pnpm exec vitest",
            "pnpm run vitest run",
            "pnpm run vitest",
            "pnpm run-script vitest run",
            "pnpm run-script vitest",
            "pnpm vitest run",
            "pnpm vitest",
            "pnpx vitest run",
            "pnpx vitest",
            "vitest run",
            "vitest",
        ],
        category: "Tests",
        savings_pct: 99.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^ctest(?:\s|$)",
        ptk_cmd: "ptk ctest",
        rewrite_prefixes: &["ctest"],
        category: "Tests",
        savings_pct: 80.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^((p?np(m|x)|p?npm\s+(exec|run|run-script)|npm\s+(rum|urn|x)|pnpm\s+dlx)\s+)?playwright(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk playwright",
        rewrite_prefixes: &[
            "npm exec playwright",
            "npm playwright",
            "npm rum playwright",
            "npm run playwright",
            "npm run-script playwright",
            "npm urn playwright",
            "npm x playwright",
            "npx playwright",
            "playwright",
            "pnpm dlx playwright",
            "pnpm exec playwright",
            "pnpm playwright",
            "pnpm run playwright",
            "pnpm run-script playwright",
            "pnpx playwright",
        ],
        category: "Tests",
        savings_pct: 94.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^((p?np(m|x)|p?npm\s+(exec|run|run-script)|npm\s+(rum|urn|x)|pnpm\s+dlx)\s+)?prisma(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk prisma",
        rewrite_prefixes: &[
            "npm exec prisma",
            "npm prisma",
            "npm rum prisma",
            "npm run prisma",
            "npm run-script prisma",
            "npm urn prisma",
            "npm x prisma",
            "npx prisma",
            "pnpm dlx prisma",
            "pnpm exec prisma",
            "pnpm prisma",
            "pnpm run prisma",
            "pnpm run-script prisma",
            "pnpx prisma",
            "prisma",
        ],
        category: "Build",
        savings_pct: 88.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^docker\s+(ps|images|logs|run|exec|build|compose\s+(ps|logs|build))(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk docker",
        rewrite_prefixes: &["docker"],
        category: "Infra",
        savings_pct: 85.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^kubectl\s+(get|logs|describe|apply)(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk kubectl",
        rewrite_prefixes: &["kubectl"],
        category: "Infra",
        savings_pct: 85.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^oc\s+(get|logs|describe|apply|status|adm)(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk oc",
        rewrite_prefixes: &["oc"],
        category: "Infra",
        savings_pct: 85.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^tree(\s|$)",
        ptk_cmd: "ptk tree",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["tree"],
        category: "Files",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^diff\s+",
        ptk_cmd: "ptk diff",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["diff"],
        category: "Files",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^curl\s+",
        ptk_cmd: "ptk curl",
        rewrite_prefixes: &["curl"],
        category: "Network",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^wget\s+",
        ptk_cmd: "ptk wget",
        rewrite_prefixes: &["wget"],
        category: "Network",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(python3?\s+-m\s+)?mypy(\s|$)",
        ptk_cmd: "ptk mypy",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["python3 -m mypy", "python -m mypy", "mypy"],
        category: "Build",
        savings_pct: 80.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^ruff\s+(check|format)(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk ruff",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["ruff"],
        category: "Python",
        savings_pct: 80.0,
        subcmd_savings: &[("check", 80.0), ("format", 75.0)],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^sqlfluff\s+lint(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk sqlfluff",
        rewrite_prefixes: &["sqlfluff"],
        category: "Python",
        savings_pct: 75.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(python[0-9.]*\s+-m\s+)?pytest(\s|$)",
        ptk_cmd: "ptk pytest",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["python3 -m pytest", "python -m pytest", "pytest"],
        category: "Python",
        savings_pct: 90.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(pip3?|uv\s+pip)\s+(list|outdated|install|show)(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk pip",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["pip3", "pip", "uv pip"],
        category: "Python",
        savings_pct: 75.0,
        subcmd_savings: &[("list", 75.0), ("outdated", 80.0)],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^uv\s+run(?:\s|$)",
        ptk_cmd: "ptk uv",
        rewrite_prefixes: &["uv"],
        category: "Python",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^go\s+(test|build|vet)(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk go",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["go"],
        category: "Go",
        savings_pct: 85.0,
        subcmd_savings: &[("test", 90.0), ("build", 80.0), ("vet", 75.0)],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:golangci-lint|golangci)\s+(run)(?:\s|$)",
        ptk_cmd: "ptk golangci-lint run",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["golangci-lint run", "golangci run"],
        category: "Go",
        savings_pct: 85.0,
        ..PtkRule::DEFAULT
    },
    // Scala/SBT
    PtkRule {
        pattern: r#"^sbt\s+["']?(testOnly|testQuick|test|compile|run|clean|assembly|package)(?:[\s"']|$)"#,
        ptk_cmd: "ptk sbt",
        rewrite_prefixes: &["sbt"],
        category: "Build",
        savings_pct: 80.0,
        subcmd_savings: &[("test", 90.0), ("testOnly", 90.0), ("compile", 75.0)],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^bundle\s+(install|update)\b",
        ptk_cmd: "ptk bundle",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["bundle"],
        category: "Ruby",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:bundle\s+exec\s+)?(?:bin/)?(?:rake|rails)\s+test(?:[\s:]|$|[;|&()<>])",
        ptk_cmd: "ptk rake",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &[
            "bundle exec rails",
            "bundle exec rake",
            "bin/rails",
            "rails",
            "rake",
        ],
        category: "Ruby",
        savings_pct: 85.0,
        subcmd_savings: &[("test", 90.0)],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:bundle\s+exec\s+)?rspec(?:\s|$)",
        ptk_cmd: "ptk rspec",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["bundle exec rspec", "bin/rspec", "rspec"],
        category: "Tests",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:bundle\s+exec\s+)?rubocop(?:\s|$)",
        ptk_cmd: "ptk rubocop",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["bundle exec rubocop", "rubocop"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    // PHP tooling
    PtkRule {
        pattern: r"^php\s+artisan(?:\s|$)",
        ptk_cmd: "ptk php",
        rewrite_prefixes: &["php"],
        category: "Build",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^php\s+-l(?:\s|$)",
        ptk_cmd: "ptk php",
        rewrite_prefixes: &["php"],
        category: "Build",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^php\s+run-tests\.php(?:\s|$)",
        ptk_cmd: "ptk phpt",
        rewrite_prefixes: &["php run-tests.php"],
        category: "Tests",
        savings_pct: 99.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:php\s+)?(?:\./)?(?:(?:vendor/)?bin/)?phpunit(?:\s|$)",
        ptk_cmd: "ptk phpunit",
        pipeline_safety: PipelineSafety::ProducerOnly,
        // rewrite_segment_inner normalizes the php wrapper, `./`, vendor/bin and
        // composer bin-dir before matching, so only the residual forms remain:
        // a plain `bin/` (not a Composer dir, so it survives normalization) and
        // the bare tool name.
        rewrite_prefixes: &["bin/phpunit", "phpunit"],
        category: "Tests",
        savings_pct: 75.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:php\s+)?(?:\./)?(?:(?:vendor/)?bin/)?phpstan\s+analy[sz]e\b",
        ptk_cmd: "ptk phpstan",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["bin/phpstan", "phpstan"],
        category: "Build",
        savings_pct: 65.0,
        subcmd_savings: &[("analyse", 65.0), ("analyze", 65.0)],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:\./)?(?:vendor/bin/)?pest(?:\s|$)",
        ptk_cmd: "ptk pest",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["pest"],
        category: "Tests",
        savings_pct: 80.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:\./)?(?:vendor/bin/)?paratest(?:\s|$)",
        ptk_cmd: "ptk paratest",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["paratest"],
        category: "Tests",
        savings_pct: 80.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:\./)?(?:vendor/bin/)?ecs(?:\s|$)",
        ptk_cmd: "ptk ecs",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["ecs"],
        category: "Build",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:\./)?(?:vendor/bin/)?pint(?:\s|$)",
        ptk_cmd: "ptk pint",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["pint"],
        category: "Build",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^aws\s+",
        ptk_cmd: "ptk aws",
        rewrite_prefixes: &["aws"],
        category: "Infra",
        savings_pct: 80.0,
        subcmd_savings: &[
            ("sts", 80.0),
            ("s3", 60.0),
            ("ec2", 85.0),
            ("ecs", 90.0),
            ("rds", 80.0),
            ("cloudformation", 90.0),
            ("logs", 88.0),
            ("lambda", 90.0),
            ("iam", 85.0),
            ("dynamodb", 70.0),
            ("s3api", 75.0),
            ("eks", 87.0),
            ("sqs", 78.0),
            ("secretsmanager", 75.0),
        ],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^psql(\s|$)",
        ptk_cmd: "ptk psql",
        rewrite_prefixes: &["psql"],
        category: "Infra",
        savings_pct: 75.0,
        ..PtkRule::DEFAULT
    },
    // Bun/Deno
    PtkRule {
        pattern: r"^bun\s+(install|add|remove|test|build|run|pm\s+ls|pm|x)\b",
        ptk_cmd: "ptk bun",
        rewrite_prefixes: &["bun"],
        category: "PackageManager",
        savings_pct: 75.0,
        subcmd_savings: &[("test", 90.0), ("install", 70.0), ("pm ls", 70.0)],
        // Audited against what each subcommand actually does. "pm ls" is
        // filtered and every other "bun pm" is passthrough, which is why the
        // pattern captures the two-word form separately. "build" writes its
        // bundle to stdout unless an output flag is present, so its common form
        // runs unfiltered and it cannot claim the headline number.
        subcmd_status: &[
            ("run", PtkStatus::Passthrough),
            ("pm", PtkStatus::Passthrough),
            ("build", PtkStatus::Passthrough),
        ],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^bunx\s+",
        ptk_cmd: "ptk bunx",
        rewrite_prefixes: &["bunx"],
        category: "PackageManager",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^deno\s+(test|lint|check|run|task|compile|install)\b",
        ptk_cmd: "ptk deno",
        rewrite_prefixes: &["deno"],
        category: "Build",
        savings_pct: 75.0,
        // Measured against real deno 2.9.6 output: the lint and check filters
        // remove ANSI, download and blank lines and keep every diagnostic, so
        // they save bytes rather than content.
        subcmd_savings: &[("test", 90.0), ("lint", 40.0), ("check", 50.0)],
        // Audited alongside the bun rule above: test, lint and check are
        // filtered, the rest run unchanged.
        subcmd_status: &[
            ("run", PtkStatus::Passthrough),
            ("task", PtkStatus::Passthrough),
            ("install", PtkStatus::Passthrough),
            ("compile", PtkStatus::Passthrough),
        ],
        ..PtkRule::DEFAULT
    },
    // TOML-filtered commands
    PtkRule {
        pattern: r"^ansible-playbook\b",
        ptk_cmd: "ptk ansible-playbook",
        rewrite_prefixes: &["ansible-playbook"],
        category: "Infra",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^brew\s+(install|upgrade)\b",
        ptk_cmd: "ptk brew",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["brew"],
        category: "PackageManager",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^choco(?:latey)?\s+(list|outdated|search|find|info|install|upgrade|uninstall|pin)\b",
        ptk_cmd: "ptk choco",
        rewrite_prefixes: &["choco", "chocolatey"],
        category: "PackageManager",
        savings_pct: 60.0,
        windows_only: true,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^scoop\s+(list|status|search|info|install|update|upgrade|uninstall)\b",
        ptk_cmd: "ptk scoop",
        rewrite_prefixes: &["scoop"],
        category: "PackageManager",
        savings_pct: 60.0,
        windows_only: true,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^winget\s+(list|ls|show|upgrade|search|find|install|uninstall|download)\b",
        ptk_cmd: "ptk winget",
        rewrite_prefixes: &["winget"],
        category: "PackageManager",
        savings_pct: 65.0,
        windows_only: true,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^ipconfig(?:\s+/+all)?\s*$",
        ptk_cmd: "ptk ipconfig",
        rewrite_prefixes: &["ipconfig"],
        category: "Network",
        savings_pct: 70.0,
        windows_only: true,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^systeminfo\b",
        ptk_cmd: "ptk systeminfo",
        rewrite_prefixes: &["systeminfo"],
        category: "System",
        savings_pct: 85.0,
        windows_only: true,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^tasklist\b",
        ptk_cmd: "ptk tasklist",
        rewrite_prefixes: &["tasklist"],
        category: "System",
        savings_pct: 65.0,
        windows_only: true,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^robocopy\b",
        ptk_cmd: "ptk robocopy",
        rewrite_prefixes: &["robocopy"],
        category: "Files",
        savings_pct: 75.0,
        windows_only: true,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^certutil\s+-(?:store|dump|hashfile)\b",
        ptk_cmd: "ptk certutil",
        rewrite_prefixes: &["certutil"],
        category: "System",
        savings_pct: 60.0,
        windows_only: true,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^ping\b",
        ptk_cmd: "ptk ping",
        rewrite_prefixes: &["ping"],
        category: "Network",
        savings_pct: 60.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^schtasks\s+/+query\b",
        ptk_cmd: "ptk schtasks",
        rewrite_prefixes: &["schtasks"],
        category: "System",
        savings_pct: 70.0,
        windows_only: true,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^reg\s+query\b",
        ptk_cmd: "ptk reg",
        rewrite_prefixes: &["reg"],
        category: "System",
        savings_pct: 75.0,
        windows_only: true,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^composer\s+(install|update|require)\b",
        ptk_cmd: "ptk composer",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["composer"],
        category: "PackageManager",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^df(\s|$)",
        ptk_cmd: "ptk df",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["df"],
        category: "System",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^dotnet\s+build\b",
        ptk_cmd: "ptk dotnet",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["dotnet"],
        category: "Build",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^du\b",
        ptk_cmd: "ptk du",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["du"],
        category: "System",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^fail2ban-client\b",
        ptk_cmd: "ptk fail2ban-client",
        rewrite_prefixes: &["fail2ban-client"],
        category: "Infra",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^gcloud\b",
        ptk_cmd: "ptk gcloud",
        rewrite_prefixes: &["gcloud"],
        category: "Infra",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:\./gradlew|gradlew\.bat|gradlew|gradle)(?:\s+(test|build|clean|assemble\w*|install\w*|check|lint\w*|dependencies))?(\s|$)",
        ptk_cmd: "ptk gradlew",
        rewrite_prefixes: &["./gradlew", "gradlew.bat", "gradlew", "gradle"],
        category: "Build",
        savings_pct: 75.0,
        subcmd_savings: &[("test", 90.0), ("build", 80.0), ("check", 80.0)],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^hadolint\b",
        ptk_cmd: "ptk hadolint",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["hadolint"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^helm\b",
        ptk_cmd: "ptk helm",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["helm"],
        category: "Infra",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^iptables\b",
        ptk_cmd: "ptk iptables",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["iptables"],
        category: "Infra",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^make\b",
        ptk_cmd: "ptk make",
        rewrite_prefixes: &["make"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^markdownlint\b",
        ptk_cmd: "ptk markdownlint",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["markdownlint"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^mix\s+(compile|format)(\s|$)",
        ptk_cmd: "ptk mix",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["mix"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^(?:\./mvnw|mvnw\.cmd|mvnw|mvn)\b(?:\s+\S+)*?\s+(compile|test|integration-test|package|install|verify|deploy)\b",
        ptk_cmd: "ptk mvn",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["./mvnw", "mvnw.cmd", "mvnw", "mvn"],
        category: "Build",
        savings_pct: 82.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        // `mvnd` is a separate binary, not a `mvn` wrapper — it must keep its
        // own ptk_cmd so the daemon is what actually runs. mvnd ships
        // `mvnd.cmd` on Windows; listed explicitly (longer prefix first) on
        // both the pattern and rewrite_prefixes, mirroring the mvn rule's
        // `mvnw.cmd` handling — `^mvnd\b` alone matches the `.` boundary in
        // `mvnd.cmd` but can't then reach `\s+(compile|...)`, so it silently
        // fails to classify the command at all.
        pattern: r"^(?:mvnd\.cmd|mvnd)\b(?:\s+\S+)*?\s+(compile|test|integration-test|package|install|verify|deploy)\b",
        ptk_cmd: "ptk mvnd",
        rewrite_prefixes: &["mvnd.cmd", "mvnd"],
        category: "Build",
        savings_pct: 82.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^ping\b",
        ptk_cmd: "ptk ping",
        rewrite_prefixes: &["ping"],
        category: "Network",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^pio\s+run(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk pio",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["pio"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^poetry\s+(install|lock|update)\b",
        ptk_cmd: "ptk poetry",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["poetry"],
        category: "Python",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^pre-commit\b",
        ptk_cmd: "ptk pre-commit",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["pre-commit"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^ps(\s|$)",
        ptk_cmd: "ptk ps",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["ps"],
        category: "System",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^pulumi\s+(preview|up|destroy|refresh|stack)(\s|$)",
        ptk_cmd: "ptk pulumi",
        rewrite_prefixes: &["pulumi"],
        category: "Infra",
        savings_pct: 45.0,
        subcmd_savings: &[
            ("up", 66.0),
            ("destroy", 72.0),
            ("refresh", 35.0),
            ("preview", 25.0),
            ("stack", 29.0),
        ],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^quarto\s+render(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk quarto",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["quarto"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^rsync\b",
        ptk_cmd: "ptk rsync",
        rewrite_prefixes: &["rsync"],
        category: "Network",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^shellcheck\b",
        ptk_cmd: "ptk shellcheck",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["shellcheck"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^shopify\s+theme\s+(push|pull)(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk shopify",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["shopify"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^sops\b",
        ptk_cmd: "ptk sops",
        rewrite_prefixes: &["sops"],
        category: "Infra",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^swift\s+(build|test)\b",
        ptk_cmd: "ptk swift",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["swift"],
        category: "Build",
        savings_pct: 65.0,
        subcmd_savings: &[("test", 90.0)],
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^systemctl\s+status\b",
        ptk_cmd: "ptk systemctl",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["systemctl"],
        category: "System",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^terraform\s+plan(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk terraform",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["terraform"],
        category: "Infra",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^tofu\s+(fmt|init|plan|validate)(\s|$)",
        ptk_cmd: "ptk tofu",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["tofu"],
        category: "Infra",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^trunk\s+build(?:\s|$|[;|&()<>])",
        ptk_cmd: "ptk trunk",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["trunk"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^uv\s+(sync|pip\s+install)\b",
        ptk_cmd: "ptk uv",
        rewrite_prefixes: &["uv"],
        category: "Python",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^yamllint\b",
        ptk_cmd: "ptk yamllint",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["yamllint"],
        category: "Build",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^wc(\s|$)",
        ptk_cmd: "ptk wc",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["wc"],
        category: "Files",
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^gt\s+",
        ptk_cmd: "ptk gt",
        rewrite_prefixes: &["gt"],
        category: "Git",
        savings_pct: 70.0,
        ..PtkRule::DEFAULT
    },
    PtkRule {
        pattern: r"^liquibase(?:\s|$)",
        ptk_cmd: "ptk liquibase",
        pipeline_safety: PipelineSafety::ProducerOnly,
        rewrite_prefixes: &["liquibase"],
        category: "Infra",
        savings_pct: 65.0,
        ..PtkRule::DEFAULT
    },
];

pub const IGNORED_PREFIXES: &[&str] = &[
    "cd ",
    "cd\t",
    "echo ",
    "printf ",
    "export ",
    "source ",
    "mkdir ",
    "rm ",
    "mv ",
    "cp ",
    "chmod ",
    "chown ",
    "touch ",
    "which ",
    "type ",
    "test ",
    "true",
    "false",
    "sleep ",
    "wait",
    "kill ",
    "set ",
    "unset ",
    "sort ",
    "uniq ",
    "tr ",
    "cut ",
    "awk ",
    "sed ",
    "python3 -c",
    "python -c",
    "node -e",
    "ruby -e",
    "ptk ",
    "pwd",
    "bash ",
    "sh ",
    "then\n",
    "then ",
    "else\n",
    "else ",
    "do\n",
    "do ",
    "for ",
    "while ",
    "if ",
    "case ",
];

pub const IGNORED_EXACT: &[&str] = &[
    "cd", "echo", "true", "false", "wait", "pwd", "bash", "sh", "fi", "done",
];
