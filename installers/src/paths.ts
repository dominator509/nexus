/**
 * EP-042 M4 installer path safety (SPEC-016; fence L abuse cases).
 *
 * Real filesystem path guards so an installer cannot be abused through
 * component paths or cleanup targets:
 *
 * - PATH TRAVERSAL -> PATH_ESCAPE (component path escaping the root)
 * - SYMLINK ESCAPE -> PATH_ESCAPE (staged symlink pointing outside)
 * - DUPLICATE COMPONENT OVERWRITE -> PATH_ESCAPE (collision)
 * - FOREIGN-ROOT CLEANUP REQUEST -> FOREIGN_RESOURCE
 *
 * Guards run against the real filesystem (lstat) where symlink escape
 * is checked, so a staged symlink is detected from real inode data.
 */

import { lstatSync, realpathSync } from "node:fs";
import { basename, dirname, isAbsolute, resolve, sep } from "node:path";
import { InstallerError } from "./errors";

/**
 * Reject a component-relative path that escapes the install root:
 * absolute paths, parent traversal, and empty names are denied.
 */
export function assertComponentPathWithinRoot(
  root: string,
  componentPath: string,
  componentId: string,
): string {
  if (componentPath === "") {
    throw new InstallerError("PATH_ESCAPE", "empty component path", {
      componentId,
    });
  }
  if (isAbsolute(componentPath)) {
    throw new InstallerError("PATH_ESCAPE", "absolute component path", {
      componentId,
    });
  }
  const parts = componentPath.split("/");
  for (const part of parts) {
    if (part === "..") {
      throw new InstallerError(
        "PATH_ESCAPE",
        "parent traversal in component path",
        {
          componentId,
        },
      );
    }
    if (part === "." || part.trim() === "") {
      throw new InstallerError("PATH_ESCAPE", "empty path segment", {
        componentId,
      });
    }
    if (part.includes("\\") || part.includes("\0")) {
      throw new InstallerError("PATH_ESCAPE", "invalid path separator", {
        componentId,
      });
    }
  }
  const resolved = resolve(root, componentPath);
  if (resolved !== root && !resolved.startsWith(`${root}${sep}`)) {
    throw new InstallerError(
      "PATH_ESCAPE",
      "component path escapes install root",
      {
        componentId,
      },
    );
  }
  return resolved;
}

/**
 * Reject a symlink escape: if the path (or its deepest existing
 * ancestor) resolves to a real location outside the root, deny. A
 * staged symlink pointing outside the install root is an abuse vector.
 */
export function assertNoSymlinkEscape(
  root: string,
  candidatePath: string,
  componentId: string,
): void {
  // Resolve the deepest existing ancestor (the file itself may not
  // exist yet at staging time, but its parent directory must not be a
  // symlink that escapes the root).
  let probe = candidatePath;
  const seen = new Set<string>();
  while (!seen.has(probe)) {
    seen.add(probe);
    let real: string;
    try {
      real = realpathSync(probe);
    } catch {
      const parent = dirname(probe);
      if (parent === probe) return;
      probe = parent;
      continue;
    }
    const rootReal = realpathSync(root);
    if (real !== rootReal && !real.startsWith(`${rootReal}${sep}`)) {
      throw new InstallerError("PATH_ESCAPE", "symlink escapes install root", {
        componentId,
      });
    }
    return;
  }
}

/**
 * Reject a duplicate component overwrite: two components mapping to the
 * same staged path would silently overwrite each other.
 */
export function assertNoDuplicateStagedPath(
  stagedPaths: ReadonlyMap<string, string>,
  componentId: string,
  targetPath: string,
): void {
  for (const [otherId, otherPath] of stagedPaths) {
    if (otherId !== componentId && otherPath === targetPath) {
      throw new InstallerError(
        "PATH_ESCAPE",
        `component ${componentId} collides with ${otherId} at ${targetPath}`,
        { componentId },
      );
    }
  }
}

/**
 * Reject a foreign-root cleanup request: a cleanup/recovery target must
 * be inside the owned install root. Anything else is a foreign resource
 * and is denied.
 */
export function assertOwnedCleanupTarget(
  ownedRoot: string,
  requestedTarget: string,
): void {
  const resolved = resolve(requestedTarget);
  const rootReal = realpathSync(ownedRoot);
  let targetReal: string;
  try {
    targetReal = realpathSync(resolved);
  } catch {
    throw new InstallerError(
      "FOREIGN_RESOURCE",
      `cleanup target does not exist: ${basename(resolved)}`,
    );
  }
  if (targetReal !== rootReal && !targetReal.startsWith(`${rootReal}${sep}`)) {
    throw new InstallerError(
      "FOREIGN_RESOURCE",
      `cleanup target outside owned root: ${basename(resolved)}`,
    );
  }
}

/**
 * Reject an install root that is a symlink escape or not a directory
 * (deny before any mutation).
 */
export function assertInstallRootUsable(root: string): void {
  try {
    const st = lstatSync(root);
    if (!st.isDirectory()) {
      throw new InstallerError(
        "INSTALL_FAILED",
        "install root is not a directory",
      );
    }
    if (st.isSymbolicLink()) {
      throw new InstallerError(
        "PATH_ESCAPE",
        "install root must not be a symlink",
      );
    }
  } catch (error) {
    if (error instanceof InstallerError) throw error;
    // Missing root is acceptable for a first install; the installer
    // creates it. Any other lstat failure is a real filesystem denial.
    const code = (error as NodeJS.ErrnoException).code;
    if (code !== "ENOENT") {
      throw new InstallerError(
        "INSTALL_FAILED",
        `install root unusable: ${code ?? "unknown"}`,
      );
    }
  }
}
