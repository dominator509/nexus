/**
 * EP-035 M3 migration SQL loader.
 *
 * The canonical DDL lives at packages/onboarding/migrations/001_onboarding.sql;
 * this loader reads that file so the executed schema is always the
 * reviewed artifact (no duplicated SQL embedded in code).
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

function resolveMigrationPath(): string {
  const candidates = [
    // Source layout (vitest/tsx run from package root).
    join(process.cwd(), "migrations", "001_onboarding.sql"),
    // Compiled layout (dist/).
    join(
      dirname(fileURLToPath(import.meta.url)),
      "..",
      "migrations",
      "001_onboarding.sql",
    ),
  ];
  for (const candidate of candidates) {
    try {
      readFileSync(candidate, "utf8");
      return candidate;
    } catch {
      // try next
    }
  }
  throw new Error("onboarding migration 001_onboarding.sql not found");
}

export const MIGRATION_SQL: string = readFileSync(
  resolveMigrationPath(),
  "utf8",
);
