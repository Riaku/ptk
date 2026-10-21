---
title: Editions and Licensing
description: What ships in each PTK download, which licence it carries, and how to build the engine edition
sidebar:
  order: 5
---

# Editions and Licensing

PTK carries two licences in two separate binaries, and GitHub Releases ships only the Apache-2.0 one. The edition that links both — the CLI **plus** the compression engine — exists as a source build.

| Download | Binary | Licence | Contents |
|---|---|---|---|
| `ptk-oss-<target>` | `ptk` | Apache-2.0 only | Hook core and every command filter. No engine. |
| `ptk-proxy-<target>` | `ptk-proxy` | BUSL-1.1 | The PTK Proxy daemon. Never bundled with `ptk`. |
| *source build* | `ptk` | Apache-2.0 + BUSL-1.1 | Hook core, every command filter **and** the compression engine |

A locally built engine edition and the prebuilt `ptk` install a binary with the **same name**, so hooks, scripts and agent configuration work unchanged. They cannot be installed side by side.

Prebuilt binaries exist for Apple silicon macOS (`ptk-oss-aarch64-apple-darwin.tar.gz`), Linux x86_64 (`ptk-oss-x86_64-unknown-linux-musl.tar.gz`), Linux arm64 (`ptk-oss-aarch64-unknown-linux-gnu.tar.gz`) and Windows x64 (`ptk-oss-x86_64-pc-windows-msvc.zip`). Intel Macs and Windows on ARM have no release asset — build from source there.

## Which one should I install?

**The prebuilt `ptk` (Apache-2.0) is the download for almost everyone.** It is the complete hook-and-filter tool: your agent's command output is rewritten and compressed exactly as before.

**Build the engine edition from source** only for the features in the table below:

```bash
cargo install --git https://github.com/riaku/ptk --branch main ptk
```

**Add `ptk-proxy`** only if you want the [PTK Proxy](../proxy/overview.md). It is independent of `ptk` and installs nothing into the `ptk` binary.

## What the prebuilt binary leaves out

It is the same binary minus the compression engine. What that removes:

| Absent | What you lose |
|---|---|
| `ptk pipeline` | Inspecting, validating, explaining and benchmarking pipelines |
| `ptk methods` | Listing registered compression methods |
| `ptk tasks` | Listing and running delegation tasks |
| PostToolUse compression | Compressing the output of an agent's own `Read` and MCP tools |
| Pipeline files | A `pipeline.toml` on disk is inert, exactly as in a build predating the engine |

**What stays is the part you use all day.** The PreToolUse rewrite hook — the mechanism that turns your agent's `git status` into `ptk git status` — is Apache-2.0 and behaves identically. So does every command filter, `ptk init`, `ptk gain`, `ptk discover`, `ptk session`, `ptk recall`, and `ptk llm-proxy`.

The practical difference: bash output is filtered identically; output your agent reads through its *own* file and MCP tools is not compressed.

## Licence terms

- **Apache-2.0** covers the `ptk` CLI and its filters.
- **BUSL-1.1** covers the compression engine and the `ptk-proxy` daemon.

The BUSL Additional Use Grant permits **free production use**, including running the daemon. What it does not permit is offering PTK — or a derivative — to third parties as a commercial product or service.

For resale, embedding in a commercial offering, or any use the grant does not cover, contact **contact@rtk-ai.app**.

Licence files travel with every download: the `ptk-oss` archive carries `LICENSE` alone, the `ptk-proxy` archive carries its own `LICENSE` plus `LICENSE-ENGINE` for the engine crate it links, and a source-built engine edition carries both.
