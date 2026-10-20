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
     │ compute next version      │
     │ from conventional commits │
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

Trigger: push to main (only) | Concurrency: never cancelled

```
     ┌──────────────────┐
     │ push to main      │
     └────────┬─────────┘
              │
     ┌────────▼──────────────────┐
     │ release-please            │
     │ analyze conventional      │
     │ commits                   │
     └────────┬──────────────────┘
              │
         ┌────┴────────────────┐
         │                     │
    no release           release created
         │                     │
         ▼                     ▼
  ┌──────────────┐    ┌───────────────────────┐
  │ create/update│    │ release.yml            │
  │ release PR   │    │ prerelease = false     │
  └──────────────┘    └───────────┬───────────┘
                                  │
                      ┌────────────▼────────────┐
                      │ Build                   │
                      │ 5 platforms             │
                      └────────────┬────────────┘
                                   │
                      ┌────────────▼────────────┐
                      │ GitHub Release           │
                      │ (stable, "Latest" badge) │
                      └────────────┬────────────┘
                                   │
                                   ▼
                              latest tag
```

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
