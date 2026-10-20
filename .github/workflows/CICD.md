# CI/CD Flows

## PR Quality Gates (ci.yml)

Trigger: pull_request to main

```
                          ┌──────────────────┐
                          │    PR opened      │
                          └────────┬─────────┘
                                   │
                          ┌────────▼─────────┐
                          │    fmt --all     │
                          └────────┬─────────┘
                                   │
                       ┌───────────▼──────────┐
                       │ clippy --all-targets │
                       └───┬───┬───┬───┬───┬──┘
                           │   │   │   │   │
           ┌───────────────┘   │   │   │   └────────────────┐
           │       ┌───────────┘   │   └───────────┐        │
           ▼       ▼              ▼               ▼        ▼
     ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌─────────┐ ┌──────────┐
     │ test     │ │ security │ │ semgrep   │ │benchmark│ │ doc      │
     │ ubuntu   │ │ cargo    │ │ AST-aware │ │ >=80%   │ │ review   │
     │ windows  │ │ audit    │ │ diff-only │ │ savings │ │ ai agent │
     │ macos    │ │ patterns │ │           │ │         │ │          │
     └────┬─────┘ └────┬─────┘ └─────┬─────┘ └────┬────┘ └────┬─────┘
          │            │             │             │            │
          └────────────┴─────────┬───┴─────────────┴────────────┘
                                 │
                      ┌──────────▼─────────┐
                      │  All must pass     │
                      │  to merge          │
                      └────────────────────┘

     + DCO check (independent, main PRs only)
     + Dependabot (weekly: Cargo deps + GitHub Actions, targeting main)
     + benchmark (windows): pwsh + cmd.exe filter rows on windows-latest
```

## Feature branches — pre-release (cd.yml)

Trigger: workflow_dispatch from any branch except main | Concurrency: cancel-in-progress

```
     ┌──────────────────┐
     │ dispatch (branch) │
     │ OR dispatch       │
     └────────┬─────────┘
              │
     ┌────────▼──────────────────┐
     │ pre-release                │
     │ next-version.sh (shared    │
     │ with the stable path)      │
     │ tag = dev-{next}-rc.{run} │
     └────────┬──────────────────┘
              │
     ┌────────▼──────────────────┐
     │ release.yml               │
     │ prerelease = true         │
     └────────┬──────────────────┘
              │
      ┌────────▼──────────────────┐
      │ Build                     │
      │ 5 platforms               │
      └────────┬──────────────────┘
               │
      ┌────────▼──────────────────┐
      │ GitHub Release            │
      │ (pre-release badge)       │
      └──────────────────────────┘
```

## Merge to main — stable release (cd.yml)

Trigger: push to main only (never a pull request, never a dispatch) | Concurrency: never cancelled

No release pull request: the tag is created on the commit that landed.

```
     ┌──────────────────────────┐
     │ push to main (merge only) │
     └────────────┬─────────────┘
                  │
     ┌────────────▼─────────────────┐
     │ scripts/next-version.sh      │
     │ conventional commits since   │
     │ the last stable vX.Y.Z       │
     └────────────┬─────────────────┘
                  │
        ┌─────────┴──────────────────────┐
        │                                │
  nothing releasable             feat / fix / perf / !
  (docs, ci, chore, test)                │
        │                     ┌──────────▼─────────────────┐
        ▼                     │ cargo test -p ptk --lib     │
  ┌───────────────┐           │ on the exact commit         │
  │ stop, no tag  │           └──────────┬─────────────────┘
  └───────────────┘                      │
                              ┌──────────▼─────────────────┐
                              │ set-version.sh: 5 manifests │
                              │ prepend CHANGELOG.md        │
                              │ commit chore(release): vX.Y.Z│
                              │ tag vX.Y.Z, push both       │
                              └──────────┬─────────────────┘
                                         │
                              ┌──────────▼─────────────────┐
                              │ release.yml                │
                              │ prerelease = false         │
                              │ (checks out the tag)       │
                              └──────────┬─────────────────┘
                                         │
                              ┌──────────▼─────────────────┐
                              │ GitHub Release             │
                              │ 5 targets + generated notes│
                              │ (stable, "Latest" badge)   │
                              └──────────┬─────────────────┘
                                         │
                                         ▼
                                    latest tag
```

The push-triggered run cannot loop: the bump commit it pushes carries
`chore(release):`, and the job's `if` skips that message.

## Manual release (release.yml)

Trigger: workflow_dispatch

```
     ┌────────────────────────┐
     │ workflow_dispatch       │
     │ inputs: tag, prerelease │
     └───────────┬────────────┘
                 │
      ┌───────────▼────────────┐
      │ Full build pipeline     │
      │ 5 platforms             │
      └───────────┬────────────┘
                  │
           ┌──────┴──────┐
           │             │
    prerelease=false  prerelease=true
           │             │
           ▼             ▼
      latest tag     pre-release
                   badge only
```
