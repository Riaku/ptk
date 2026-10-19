---
title: Editions and Licensing
description: What ships in each PTK download, which licence it carries, and how to install the Apache-2.0 edition
sidebar:
  order: 5
---

# Editions and Licensing

PTK ships as **three downloads**. Which one you want depends on what you need and which licence your organisation accepts.

| Download | Binary | Licence | Contents |
|---|---|---|---|
| `ptk-<target>` | `ptk` | Apache-2.0 + BUSL-1.1 | Hook core, every command filter, **and** the compression engine |
| `ptk-oss-<target>` | `ptk` | Apache-2.0 only | Hook core and every command filter. No engine. |
| `ptk-proxy-<target>` | `ptk-proxy` | BUSL-1.1 | The PTK Proxy daemon. Never bundled with either `ptk`. |

The two `ptk` downloads install a binary with the **same name**, so hooks, scripts and agent configuration work unchanged across editions. They cannot be installed side by side.

## Which one should I install?

**Most people want the default `ptk`.** It is the complete tool and free for production use — see [the grant](#licence-terms) below.

**Install the Apache-2.0 edition** if your organisation requires that every binary it runs carry an OSI-approved licence, or if you intend to redistribute PTK as part of a commercial offering.

**Add `ptk-proxy`** only if you want the [PTK Proxy](../proxy/overview.md). It is independent of both `ptk` editions.

## What the Apache-2.0 edition leaves out

It is the same binary minus the compression engine. What that removes:

| Absent | What you lose |
|---|---|
| `ptk pipeline` | Inspecting, validating, explaining and benchmarking pipelines |
| `ptk methods` | Listing registered compression methods |
| `ptk tasks` | Listing and running delegation tasks |
| PostToolUse compression | Compressing the output of an agent's own `Read` and MCP tools |
| Pipeline files | A `pipeline.toml` on disk is inert, exactly as in a build predating the engine |

**What stays is the part you use all day.** The PreToolUse rewrite hook — the mechanism that turns your agent's `git status` into `ptk git status` — is Apache-2.0 and behaves identically. So does every command filter, `ptk init`, `ptk gain`, `ptk discover`, `ptk session`, `ptk recall`, and `ptk llm-proxy`.

The practical difference: bash output is filtered exactly as in the full edition; output your agent reads through its *own* file and MCP tools is not.

## Installing the Apache-2.0 edition

Set `PTK_EDITION=oss` for the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/riaku/ptk/refs/heads/master/install.sh | PTK_EDITION=oss sh
```

Homebrew ships it as a separate formula:

```bash
brew install rtk-ai/tap/ptk-oss
```

Debian and RPM packages are published as `ptk-oss`.

The two editions cannot coexist — both install `/usr/bin/ptk`. Homebrew declares this explicitly (`conflicts_with`), so `brew` refuses the second install with a clear message. The deb and rpm packages carry no declared `Conflicts:`, so `dpkg`/`rpm` stop on the file collision instead. **Uninstall one edition before installing the other.**

From a source checkout:

```bash
cargo build --release --no-default-features
```

`--no-default-features` removes `ptk-engine` from the dependency graph entirely — no BUSL-licensed code is compiled into the binary.

## Licence terms

- **Apache-2.0** covers the `ptk` CLI and its filters.
- **BUSL-1.1** covers the compression engine and the `ptk-proxy` daemon.

The BUSL Additional Use Grant permits **free production use**, including running the daemon. What it does not permit is offering PTK — or a derivative — to third parties as a commercial product or service.

For resale, embedding in a commercial offering, or any use the grant does not cover, contact **contact@rtk-ai.app**.

Licence files travel with every download: the `ptk` archive carries `LICENSE` and `LICENSE-ENGINE`, the Apache-2.0 archive carries `LICENSE` alone, and the daemon archive carries its own `LICENSE`.
