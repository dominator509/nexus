/**
 * EP-043 RX-010 M1: release tag truth (AUD-076).
 *
 * collectReleaseTag() must never be satisfied by a branch pointer or a
 * detached commit. Only an actual refs/tags/* ref counts as a release
 * tag; anything else fails closed to "" so the readiness gate can only
 * pass when the repository is really at a release tag.
 */

import { describe, expect, it } from "vitest";
import { mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { collectReleaseTag } from "@nexus/release-evidence";

function repoWithHead(headContent: string): string {
  const base = join(
    tmpdir(),
    `nexus-rx010-releasetag-${Date.now()}-${Math.floor(Math.random() * 1e6)}`,
  );
  mkdirSync(join(base, ".git"), { recursive: true });
  writeFileSync(join(base, ".git", "HEAD"), headContent);
  return base;
}

describe("EP-043 M1 release tag truth (AUD-076)", () => {
  it("rx010_release_tag_accepts_tag_ref", () => {
    const root = repoWithHead("ref: refs/tags/green-v2/RX-010/abc123\n");
    expect(collectReleaseTag(root)).toBe("green-v2/RX-010/abc123");
  });

  it("rx010_release_tag_rejects_branch_pointer", () => {
    // The exact AUD-076 failure: being on master used to count as a tag.
    const root = repoWithHead("ref: refs/heads/master\n");
    expect(collectReleaseTag(root)).toBe("");
  });

  it("rx010_release_tag_rejects_detached_commit", () => {
    const root = repoWithHead("8fe13189e9f837cdeac6c4ff9175b399aa9f12c3\n");
    expect(collectReleaseTag(root)).toBe("");
  });

  it("rx010_release_tag_fails_closed_on_missing_git", () => {
    const root = join(
      tmpdir(),
      `nexus-rx010-nogit-${Date.now()}-${Math.floor(Math.random() * 1e6)}`,
    );
    expect(collectReleaseTag(root)).toBe("");
  });
});
