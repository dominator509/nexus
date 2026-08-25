/**
 * EP-042 M4 Node ESM resolve loader for native TS type-stripping.
 *
 * The workspace canonical sources (@nexus/setup update core) use
 * extensionless relative imports (bundler resolution). Node's native
 * TypeScript type-stripping requires explicit .ts extensions. This
 * loader appends the .ts extension when the extensionless specifier
 * does not resolve, so the installer CLI can execute the REAL canonical
 * update-core code directly under `node --experimental-transform-types`
 * without a bundler and without duplicating canonical logic.
 *
 * Resolution-only: it never rewrites file contents.
 */

import { access } from "node:fs/promises";
import { fileURLToPath, pathToFileURL } from "node:url";

async function exists(url) {
  try {
    await access(fileURLToPath(url));
    return true;
  } catch {
    return false;
  }
}

export async function resolve(specifier, context, nextResolve) {
  if (typeof specifier !== "string") return nextResolve(specifier, context);
  const isRelative = specifier.startsWith("./") || specifier.startsWith("../");
  const isAbsolute = specifier.startsWith("/");
  const isCwdRelative = specifier.includes("/") && !isAbsolute && !isRelative;

  // A CWD-relative entry path (e.g. `installers/src/cli.ts`) must be
  // resolved against the current working directory, not as a package.
  if (isCwdRelative) {
    const cwdBase = pathToFileURL(`${process.cwd()}/`);
    const asFile = new URL(specifier, cwdBase);
    if (await exists(asFile)) {
      return nextResolve(asFile.href, context);
    }
    const asTs = new URL(`${asFile.pathname}.ts`, asFile);
    if (await exists(asTs)) {
      return nextResolve(asTs.href, context);
    }
  }

  if (!isRelative && !isAbsolute) {
    return nextResolve(specifier, context);
  }

  // First try the plain specifier (already works for .ts files).
  try {
    return await nextResolve(specifier, context);
  } catch {
    // Fall through to extension resolution.
  }

  const base = context.parentURL
    ? new URL(specifier, context.parentURL)
    : new URL(specifier, pathToFileURL(process.cwd() + "/"));
  const candidates = [
    new URL(`${base.pathname}.ts`, base),
    new URL(`${base.pathname}.tsx`, base),
    new URL(`${base.pathname}/index.ts`, base),
  ];
  for (const candidate of candidates) {
    if (await exists(candidate)) {
      return nextResolve(candidate.href, context);
    }
  }
  // Return the original error if nothing resolves.
  return nextResolve(specifier, context);
}
