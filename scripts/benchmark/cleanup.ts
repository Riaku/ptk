#!/usr/bin/env bun
/**
 * Delete the PTK test VM.
 * Usage: bun run scripts/benchmark/cleanup.ts
 */

import { vmDelete } from "./lib/vm";

console.log("Deleting ptk-test VM...");
await vmDelete();
console.log("Done.");
