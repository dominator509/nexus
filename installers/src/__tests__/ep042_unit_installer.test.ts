/**
 * EP-042 M4 installer unit proofs (SPEC-016, SPEC-024).
 *
 * Deterministic pure-surface proofs for the installer package: typed
 * failure classification, path guards (traversal/symlink/duplicate/
 * foreign-root), journal state honesty, backup digest binding, and
 * redacted observability. Real filesystem proofs live in
 * tests/release/src/failure/ (ep042_failure_*); this suite keeps the
 * workspace unit battery green for the package itself.
 */

import { describe, expect, it } from "vitest";
import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import {
  INSTALLER_FAILURE_CLASSES,
  InstallerError,
  assertComponentPathWithinRoot,
  assertNoDuplicateStagedPath,
  assertNoSymlinkEscape,
  assertOwnedCleanupTarget,
  buildInstallerEvidence,
  isInstallerError,
  isInstallerFailureClass,
  journalComplete,
  journalRead,
  journalReset,
  looksSecretShaped,
  redactValue,
  verifyBackupDigest,
} from "../index";

function freshRoot(): string {
  const dir = mkdtempSync(join(tmpdir(), "nexus-ep042-m4-unit-"));
  return dir;
}

describe("ep042_unit installer typed failures", () => {
  it("ep042_unit_installer_failure_classes_locked", () => {
    expect(INSTALLER_FAILURE_CLASSES).toContain("MANIFEST_INVALID");
    expect(INSTALLER_FAILURE_CLASSES).toContain("DIGEST_MISMATCH");
    expect(INSTALLER_FAILURE_CLASSES).toContain("BACKUP_FAILED");
    expect(INSTALLER_FAILURE_CLASSES).toContain("STAGING_FAILED");
    expect(INSTALLER_FAILURE_CLASSES).toContain("INSTALL_FAILED");
    expect(INSTALLER_FAILURE_CLASSES).toContain("VALIDATION_FAILED");
    expect(INSTALLER_FAILURE_CLASSES).toContain("ROLLBACK_REQUIRED");
    expect(INSTALLER_FAILURE_CLASSES).toContain("ROLLBACK_FAILED");
    expect(INSTALLER_FAILURE_CLASSES).toContain("TIMEOUT");
    expect(INSTALLER_FAILURE_CLASSES).toContain("UNAVAILABLE");
    expect(INSTALLER_FAILURE_CLASSES).toContain("RESOURCE_EXHAUSTION");
    expect(INSTALLER_FAILURE_CLASSES).toContain("AUTHORIZATION_DENIED");
    expect(INSTALLER_FAILURE_CLASSES).toContain("PATH_ESCAPE");
    expect(INSTALLER_FAILURE_CLASSES).toContain("FOREIGN_RESOURCE");
    expect(INSTALLER_FAILURE_CLASSES).toContain("RECOVERY_FAILED");
    expect(INSTALLER_FAILURE_CLASSES).toContain("SIGNATURE_UNVERIFIED");
    expect(INSTALLER_FAILURE_CLASSES).toContain("COMPATIBILITY_DENIED");
  });

  it("ep042_unit_installer_error_typed_shape", () => {
    const err = new InstallerError("PATH_ESCAPE", "bad path", {
      installId: "i1",
      releaseId: "r1",
      correlationId: "c1",
    });
    expect(err.failureClass).toBe("PATH_ESCAPE");
    const shape = err.toShape();
    expect(shape.failure_class).toBe("PATH_ESCAPE");
    expect(shape.install_id).toBe("i1");
    expect(shape.correlation_id).toBe("c1");
    expect(isInstallerError(err)).toBe(true);
    expect(isInstallerFailureClass("PATH_ESCAPE")).toBe(true);
    expect(isInstallerFailureClass("NOPE")).toBe(false);
  });
});

describe("ep042_unit installer path guards", () => {
  it("ep042_unit_path_traversal_rejected", () => {
    const root = freshRoot();
    try {
      expect(() =>
        assertComponentPathWithinRoot(root, "../escape", "c1"),
      ).toThrow(InstallerError);
      expect(() => assertComponentPathWithinRoot(root, "/abs", "c1")).toThrow(
        InstallerError,
      );
      expect(() =>
        assertComponentPathWithinRoot(root, "a/../../b", "c1"),
      ).toThrow(InstallerError);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("ep042_unit_path_traversal_allowed_inside", () => {
    const root = freshRoot();
    try {
      const target = assertComponentPathWithinRoot(root, "bin/core", "c1");
      expect(target.startsWith(resolve(root))).toBe(true);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("ep042_unit_symlink_escape_rejected", () => {
    const root = freshRoot();
    try {
      const outside = join(root, "outside");
      mkdirSync(outside, { recursive: true });
      mkdirSync(join(root, "stage"), { recursive: true });
      symlinkSync(outside, join(root, "stage", "models"), "dir");
      expect(() =>
        assertNoSymlinkEscape(
          join(root, "stage"),
          join(root, "stage", "models", "x"),
          "c1",
        ),
      ).toThrow(InstallerError);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("ep042_unit_duplicate_staged_path_rejected", () => {
    const root = freshRoot();
    try {
      const seen = new Map<string, string>();
      seen.set("c1", "/stage/bin/core");
      expect(() =>
        assertNoDuplicateStagedPath(seen, "c2", "/stage/bin/core"),
      ).toThrow(InstallerError);
      expect(() =>
        assertNoDuplicateStagedPath(seen, "c2", "/stage/models/m"),
      ).not.toThrow();
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("ep042_unit_foreign_root_cleanup_rejected", () => {
    const root = freshRoot();
    const foreign = join(tmpdir(), "nexus-ep042-m4-foreign-unit");
    try {
      mkdirSync(foreign, { recursive: true });
      expect(() => assertOwnedCleanupTarget(root, foreign)).toThrow(
        InstallerError,
      );
    } finally {
      rmSync(foreign, { recursive: true, force: true });
      rmSync(root, { recursive: true, force: true });
    }
  });
});

describe("ep042_unit installer journal honesty", () => {
  it("ep042_unit_journal_missing_not_complete", () => {
    const root = freshRoot();
    try {
      const cfg = {
        journalPath: join(root, "installer.journal.jsonl"),
        runId: "run-1",
        gitCommit: "sha",
        installId: "i1",
        releaseId: "r1",
      };
      const record = journalRead(cfg);
      expect(record.entries.length).toBe(0);
      expect(record.lastState).toBeUndefined();
      expect(journalComplete(record)).toBe(false);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("ep042_unit_journal_only_installed_completes", () => {
    const root = freshRoot();
    try {
      const cfg = {
        journalPath: join(root, "installer.journal.jsonl"),
        runId: "run-1",
        gitCommit: "sha",
        installId: "i1",
        releaseId: "r1",
      };
      journalReset(cfg);
      writeFileSync(
        cfg.journalPath,
        '{"ts":"t","run_id":"run-1","git_commit":"sha","install_id":"i1","release_id":"r1","state":"FAILED","detail":"x"}\n',
        "utf8",
      );
      const record = journalRead(cfg);
      expect(record.lastState).toBe("FAILED");
      expect(journalComplete(record)).toBe(false);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});

describe("ep042_unit installer observability redaction", () => {
  it("ep042_unit_redact_secret_shapes", () => {
    const akia = `AKIA${"0".repeat(12)}CDEF`;
    const sk = `sk-${"a".repeat(12)}`;
    expect(redactValue(sk)).toBe("[REDACTED]");
    expect(redactValue(akia)).toBe("[REDACTED]");
    expect(redactValue("plain-value")).toBe("plain-value");
    expect(looksSecretShaped(sk)).toBe(true);
    expect(looksSecretShaped("plain")).toBe(false);
  });

  it("ep042_unit_evidence_redaction_canary", () => {
    const canary = `ghp_${"a".repeat(30)}`;
    const evidence = buildInstallerEvidence({
      run_id: "run-1",
      git_commit: "sha",
      install_id: "i1",
      release_id: "r1",
      manifest_digest: "sha256:" + "b".repeat(64),
      component_identities: ["c1"],
      component_digests: ["sha256:" + "c".repeat(64)],
      compatibility_state: "COMPATIBLE",
      backup_state: "COMPLETED",
      staging_state: "VALIDATED",
      install_state: "INSTALLED",
      rollback_state: "NONE",
      recovery_state: "NONE",
      cleanup_state: "NONE",
      redaction_canary: canary,
      created_at: "2026-08-25T00:00:00Z",
    });
    expect(evidence.redaction_applied).toBe(true);
    expect(JSON.stringify(evidence)).not.toContain(canary);
  });
});

describe("ep042_unit installer backup digest", () => {
  it("ep042_unit_backup_digest_binding", async () => {
    const root = freshRoot();
    try {
      const backupRoot = join(root, "backup");
      mkdirSync(backupRoot, { recursive: true });
      writeFileSync(join(backupRoot, "file"), "bytes");
      const declared = `sha256:${"a".repeat(64)}`;
      const verdict = await verifyBackupDigest(backupRoot, declared);
      expect(verdict).toBe("MISMATCH");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});
