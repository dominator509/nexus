/**
 * EP-033 M5 real-browser accessibility scan (machine-observed).
 *
 * Runs the REAL axe-core WCAG 2.2 A/AA rule set against the REAL
 * server-rendered markup of the production @nexus/ui components in
 * REAL headless Chrome (system Chrome, no emulation). The scan writes
 * current-run machine-readable evidence; the gate rejects zero-match
 * or stale evidence.
 */

import { describe, expect, it } from "vitest";
import { chromium } from "playwright-core";
import axe from "axe-core";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { renderOwnedSurfacesHtml } from "../harness.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
// __tests__ -> src -> web -> accessibility -> tests -> repo root
const EVIDENCE_DIR = join(
  __dirname,
  "..",
  "..",
  "..",
  "..",
  "..",
  ".agent",
  "state",
  "evidence",
);

interface AxeViolation {
  id: string;
  impact: string;
  description: string;
  nodes: Array<{ target: Array<string>; html: string; failureSummary: string }>;
}

interface AxeResult {
  violations: Array<AxeViolation>;
  passes: Array<unknown>;
  incomplete: Array<unknown>;
}

const CHROME = process.env.CHROME_BIN ?? "/usr/bin/google-chrome";

describe("ep033_browser_scan", () => {
  it(
    "scans the rendered owned surfaces with axe-core WCAG 2.2 A/AA in real Chrome and writes current-run evidence",
    { timeout: 180_000 },
    async () => {
      const runId = `ep033-m5-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
      const html = renderOwnedSurfacesHtml();
      const browser = await chromium.launch({
        executablePath: CHROME,
        headless: true,
        args: ["--no-sandbox", "--disable-dev-shm-usage"],
      });
      try {
        const page = await browser.newPage({
          viewport: { width: 1280, height: 900 },
        });
        await page.setContent(html, { waitUntil: "load" });

        // Inject the real axe-core engine source into the real page.
        await page.evaluate((source: string) => {
          const script = document.createElement("script");
          script.textContent = source;
          document.head.appendChild(script);
        }, axe.source);

        // Run the WCAG 2.2 A/AA tag set.
        const result = (await page.evaluate(async () => {
          const globalAxe = (
            window as unknown as {
              axe: {
                run: (context: unknown, opts: unknown) => Promise<unknown>;
              };
            }
          ).axe;
          return globalAxe.run(document, {
            runOnly: {
              type: "tag",
              values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"],
            },
          });
        })) as AxeResult;

        const keyboard = await page.evaluate(() => ({
          focusable_elements: document.querySelectorAll(
            'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
          ).length,
          skip_link_present: document.querySelector('a[href="#main"]') !== null,
          main_landmark_present: document.querySelector("main#main") !== null,
        }));

        const evidence = {
          node: "EP-033",
          milestone: "M5",
          proof: "LF-005 accessibility-scan",
          run_id: runId,
          standard: "WCAG 2.2 A/AA (axe-core rule set)",
          scope:
            "Owned surfaces: DashboardShellView, ApprovalCardView, StatusBadge, CapabilityButton, ChatComposer rendered by react-dom/server",
          tool: {
            engine: "axe-core",
            version: axe.version,
            browser: browser.version(),
          },
          results: {
            violations: result.violations.map((v) => ({
              id: v.id,
              impact: v.impact,
              description: v.description,
              nodes: v.nodes.map((n) => ({
                target: n.target.join(" "),
                html: n.html.slice(0, 200),
                summary: n.failureSummary ?? "",
              })),
            })),
            passes: result.passes.length,
            incomplete: result.incomplete.length,
          },
          keyboard,
          timestamp_unix_s: Math.floor(Date.now() / 1000),
        };

        mkdirSync(EVIDENCE_DIR, { recursive: true });
        const evidencePath = join(EVIDENCE_DIR, "LF-005-ep033-m5.json");
        writeFileSync(
          evidencePath,
          JSON.stringify(evidence, null, 2) + "\n",
          "utf8",
        );

        // Machine-observed result: the scan RAN in a real browser and
        // produced a current-run evidence file. WCAG 2.2 A/AA violations
        // on owned surfaces would fail the gate here.
        expect(result.violations.length).toBe(0);
        expect(result.passes.length).toBeGreaterThan(0);
        expect(keyboard.skip_link_present).toBe(true);
        expect(keyboard.main_landmark_present).toBe(true);
        expect(keyboard.focusable_elements).toBeGreaterThan(0);
        console.log(
          `[ep033_browser_scan] run=${runId} violations=${result.violations.length} passes=${result.passes.length} evidence=${evidencePath}`,
        );
      } finally {
        await browser.close();
      }
    },
  );
});
