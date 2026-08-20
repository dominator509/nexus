/**
 * EP-033 M5 accessibility package unit tests: the SSR harness renders
 * the REAL production components into a standards-shaped document, and
 * the LF-005 journey composes the REAL contracts across devices.
 */

import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { renderOwnedSurfacesHtml } from "../harness.js";
import { runLf005Journey } from "../lf005.js";

describe("ep033_a11y_harness", () => {
  it("renders the owned production surfaces into a complete document", () => {
    const html = renderOwnedSurfacesHtml();
    expect(html).toContain("<!doctype html>");
    expect(html).toContain('<html lang="en">');
    expect(html).toContain("<main");
    expect(html).toContain("Dashboard shell");
    expect(html).toContain("Quarantine host");
    expect(html).toContain("Chat message composer");
  });

  it("renders the FOUR_EYES requirement verbatim (never collapsed)", () => {
    const html = renderOwnedSurfacesHtml();
    expect(html).toContain("two distinct principals");
    expect(html).toContain('data-four-eyes="true"');
  });

  it("renders a disabled capability button for visible-but-unauthorized", () => {
    const html = renderOwnedSurfacesHtml();
    expect(html).toContain('aria-disabled="true"');
    expect(html).toContain('disabled=""');
  });

  it("renders stale status non-color (text label present)", () => {
    const html = renderOwnedSurfacesHtml();
    expect(html).toContain("OFFLINE");
    expect(html).toContain("(stale)");
  });

  it("includes a skip link and main landmark (keyboard contract)", () => {
    const html = renderOwnedSurfacesHtml();
    expect(html).toContain('href="#main"');
    expect(html).toContain("main");
  });
});

describe("ep033_lf005_continuity", () => {
  it("binds the voice-started objective in the web dashboard via correlation", () => {
    const evidence = runLf005Journey("test-run-1");
    expect(evidence.journey.voice_start.transcript_origin).toBe("AGENT");
    expect(evidence.journey.web_dashboard_continue.objective_bound).toBe(true);
    expect(evidence.journey.web_dashboard_continue.rendered_surface).toBe("objectives");
  });

  it("satisfies mobile FOUR_EYES approval with two distinct principals", () => {
    const evidence = runLf005Journey("test-run-2");
    expect(evidence.journey.mobile_approval.approval_class).toBe("FOUR_EYES");
    expect(evidence.journey.mobile_approval.distinct_approvers).toBe(2);
    expect(evidence.journey.mobile_approval.satisfied).toBe(true);
    expect(evidence.journey.mobile_approval.state).toBe("APPROVED");
  });

  it("delivers the final artifact in the same task graph", () => {
    const evidence = runLf005Journey("test-run-3");
    expect(evidence.journey.final_artifact_same_graph.objective_ids_consistent).toBe(true);
    expect(evidence.journey.final_artifact_same_graph.correlation_consistent).toBe(true);
    expect(evidence.journey.final_artifact_same_graph.artifact_task_done).toBe(true);
  });

  it("preserves the UI authority distinctions", () => {
    const evidence = runLf005Journey("test-run-4");
    expect(evidence.authority_distinctions.displayed_not_authorized).toBe(true);
    expect(evidence.authority_distinctions.approved_not_executed_until_dispatched).toBe(true);
    expect(evidence.authority_distinctions.executed_only_after_dispatch).toBe(true);
  });

  it("binds a current-run identity to every journey", () => {
    const evidence = runLf005Journey("run-abc-123");
    expect(evidence.run_id).toBe("run-abc-123");
    expect(evidence.node).toBe("EP-033");
    expect(evidence.milestone).toBe("M5");
  });
});

describe("ep033_a11y_dependency_direction", () => {
  it("imports only @nexus packages, react, and the scan toolchain", () => {
    const files = [
      "src/harness.tsx",
      "src/scan.ts",
      "src/lf005.ts",
    ];
    for (const file of files) {
      const source = readFileSync(join(process.cwd(), "src", file.replace(/^src\//, "")), "utf8");
      const lines = source.split("\n").filter((line: string) => line.includes("from \"") || line.includes("from '"));
      for (const line of lines) {
        const match = line.match(/from ["']([^"']+)["']/);
        if (!match || !match[1]) continue;
        const spec = match[1];
        const allowed =
          spec.startsWith(".") ||
          spec === "react" ||
          spec === "react-dom" ||
          spec === "react-dom/server" ||
          spec === "playwright-core" ||
          spec === "axe-core" ||
          spec.startsWith("@nexus/") ||
          spec.startsWith("node:");
        expect(allowed, `${file}: import "${spec}" violates dependency direction`).toBe(true);
      }
    }
  });
});
