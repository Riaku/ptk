// PTK Pi extension — rewrites bash commands to use ptk for token savings.
// Shared with Oh My Pi (OMP) — OMP loads this same file via its legacy-pi-compat layer.
// Requires: ptk >= 0.23.0 in PATH.
//
// This is a thin delegating extension: all rewrite logic lives in `ptk rewrite`,
// which is the single source of truth (src/discover/registry.rs).
// To add or change rewrite rules, edit the Rust registry — not this file.
//
// Exit code contract for `ptk rewrite`:
//   0 + stdout  Rewrite found → mutate command
//   1           No PTK equivalent → pass through unchanged
//   3 + stdout  Rewrite (advisory) → mutate command

import type {
  BashToolCallEvent,
  ExtensionAPI,
  ToolCallEvent,
} from "@earendil-works/pi-coding-agent"

const REWRITE_TIMEOUT_MS = 2_000
const MIN_SUPPORTED_PTK_MINOR = 23

// Local reimplementation of the package's `isToolCallEventType("bash", event)` type
// guard. That helper is a value export, so importing it pulls in the whole
// `@earendil-works/pi-coding-agent` barrel at extension load — profiled at ~250ms
// warmed, vs ~10ms for a type-only import. `BashToolCallEvent`/`ToolCallEvent`
// below are type-only imports and are erased at compile time, so they carry none
// of that cost. See #2753.
function isBashToolCallEvent(event: ToolCallEvent): event is BashToolCallEvent {
  return event.toolName === "bash"
}

// Parse "X.Y.Z" semver, return [major, minor, patch] or null.
function parseSemver(raw: string): [number, number, number] | null {
  const m = raw.trim().match(/(\d+)\.(\d+)\.(\d+)/)
  if (!m) return null
  return [parseInt(m[1], 10), parseInt(m[2], 10), parseInt(m[3], 10)]
}

// Calls `ptk rewrite`; returns the rewritten command or null (pass through).
async function rewriteCommand(
  pi: ExtensionAPI,
  cmd: string,
  signal?: AbortSignal
): Promise<string | null> {
  const result = await pi.exec("ptk", ["rewrite", cmd], {
    timeout: REWRITE_TIMEOUT_MS,
    signal,
  })
  if (result.killed) return null
  if (result.code !== 0 && result.code !== 3) return null
  return result.stdout.trim() || null
}

type StatusContext = {
  ui?: {
    setStatus?: (key: string, text: string) => void
  }
}

// Register before the async version probe so a host cannot miss the handler
// while the probe is in flight. If session_start happens first, retain its
// context and apply the status as soon as the probe reports a failure.
// pi.notify is intentionally not used — OMP wipes it on the initial render.
function registerPtkUnavailableNotice(pi: ExtensionAPI) {
  let reason: string | undefined
  let sessionContext: StatusContext | undefined

  const applyStatus = () => {
    if (!reason || !sessionContext) return
    try {
      sessionContext.ui?.setStatus?.("ptk", `PTK disabled: ${reason}`)
    } catch {
      // Status reporting must never affect the extension's fail-open behavior.
    }
  }

  try {
    pi.on("session_start", (_event: unknown, ctx: unknown) => {
      sessionContext = ctx as StatusContext
      applyStatus()
    })
  } catch {
    // Runtimes without a session_start event: nothing to report.
    return (_reason: string) => {}
  }

  return (nextReason: string) => {
    reason = nextReason
    applyStatus()
  }
}

export default async function (pi: ExtensionAPI) {
  const reportPtkUnavailable = registerPtkUnavailableNotice(pi)

  // Probe ptk version at load time; disables extension if missing or too old.
  const ver = await pi.exec("ptk", ["--version"], { timeout: REWRITE_TIMEOUT_MS })
  if (ver.code !== 0) {
    reportPtkUnavailable("ptk binary not found in PATH")
    console.warn("[ptk] ptk binary not found in PATH — extension disabled")
    return
  }

  // Warn and bail if ptk predates 0.23.0 (when `ptk rewrite` was introduced).
  const parsed = parseSemver(ver.stdout.replace(/^ptk\s+/, ""))
  if (parsed) {
    const [major, minor] = parsed
    if (major === 0 && minor < MIN_SUPPORTED_PTK_MINOR) {
      reportPtkUnavailable(`ptk ${parsed.join(".")} is too old (need >= 0.23.0)`)
      console.warn(`[ptk] ptk ${ver.stdout.trim()} is too old (need >= 0.23.0) — extension disabled`)
      return
    }
  }

  pi.on("tool_call", async (event, ctx) => {
    try {
      if (!isBashToolCallEvent(event)) return

      const cmd = event.input.command
      if (typeof cmd !== "string" || cmd.trim() === "") return

      if (cmd.startsWith("ptk ")) return
      if (process.env.PTK_DISABLED === "1") return

      // Delegate to PTK.
      const rewritten = await rewriteCommand(pi, cmd, ctx.signal)
      if (rewritten && rewritten !== cmd) {
        event.input.command = rewritten
      }
    } catch (err) {
      // Fail open: never block execution on an unexpected error.
      console.warn("[ptk] unexpected error in tool_call handler; passing through command", err)
      return
    }
  })
}
