/**
 * EP-006 test-zone docker/process state proofs (TESTING.md test zone).
 * Queries REAL docker state; used by teardown and failure tests to
 * prove resources are gone after disposal.
 */

import { runDocker } from "./stack.js";

export function containerExists(name: string): boolean {
  const out = runDocker([
    "ps",
    "-a",
    "--filter",
    `name=${name}`,
    "--format",
    "{{.Names}}",
  ]);
  return out
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .includes(name);
}

export function networkExists(name: string): boolean {
  const out = runDocker([
    "network",
    "ls",
    "--filter",
    `name=${name}`,
    "--format",
    "{{.Name}}",
  ]);
  return out
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .includes(name);
}

export function volumeExists(name: string): boolean {
  const out = runDocker([
    "volume",
    "ls",
    "--filter",
    `name=${name}`,
    "--format",
    "{{.Name}}",
  ]);
  return out
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .includes(name);
}

export function containerPid(name: string): number {
  return Number(
    runDocker(["inspect", "--format", "{{.State.Pid}}", name]).trim(),
  );
}

export function processAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}
