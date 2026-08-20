/**
 * EP-033 M5 accessibility scan driver (real browser, machine-observed).
 *
 * Loads the REAL rendered owned-surfaces document (react-dom/server
 * output of the production @nexus/ui components) in REAL headless
 * Chrome and runs the REAL axe-core WCAG 2.2 AA rule set against the
 * real DOM. Results are written as current-run machine-readable
 * evidence (LF-005 / EP-033 M5) with the run identity embedded.
 *
 * No jsdom, no emulation, no component doubles: the DOM under test is
 * the actual server-rendered markup of the production components.
 */

import { chromium } from "playwright-core";
import axe from "axe-core";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { renderOwnedSurfacesHtml } from "./harness.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const EVIDENCE_DIR = join(
  __dirname,
  "..",
  "..",
  "..",
  "..",
  ".agent",
  "state",
  "evidence",
);

export interface AccessibilityEvidence {
  node: string;
  milestone: string;
  proof: string;
  run_id: string;
  standard: string;
  scope: string;
  tool: {
    engine: string;
    version: string;
    browser: string;
  };
  results: {
    violations: Array<{
      id: string;
      impact: string;
      description: string;
      nodes: Array<{ target: string; html: string; summary: string }>;
    }>;
    passes: number;
    incomplete: number;
  };
  keyboard: {
    focusable_elements: number;
    skip_link_present: boolean;
    main_landmark_present: boolean;
  };
  timestamp_unix_s: number;
}

export function runId(): string {
  return `ep033-m5-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

export async function runAccessibilityScan(
  chromePath: string,
  run: string,
  html: string,
): Promise<AccessibilityEvidence> {
  const browser = await chromium.launch({
    executablePath: chromePath,
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

    // Run the WCAG 2.2 A/AA tag set (axe-core tag vocabulary).
    const axeResult = await page.evaluate(async () => {
      const globalAxe = (
        window as unknown as {
          axe: { run: (context: unknown, opts: unknown) => Promise<unknown> };
        }
      ).axe;
      return globalAxe.run(document, {
        runOnly: {
          type: "tag",
          values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"],
        },
      });
    });

    const typed = axeResult as {
      violations: Array<{
        id: string;
        impact: string;
        description: string;
        nodes: Array<{
          target: Array<string>;
          html: string;
          failureSummary: string;
        }>;
      }>;
      passes: Array<unknown>;
      incomplete: Array<unknown>;
    };

    const evidence: AccessibilityEvidence = {
      node: "EP-033",
      milestone: "M5",
      proof: "LF-005 accessibility-scan",
      run_id: run,
      standard: "WCAG 2.2 A/AA (axe-core rule set)",
      scope:
        "Owned surfaces: DashboardShellView, ApprovalCardView, StatusBadge, CapabilityButton, ChatComposer rendered by react-dom/server",
      tool: {
        engine: "axe-core",
        version: axe.version ?? "unknown",
        browser: browser.version(),
      },
      results: {
        violations: typed.violations.map((v) => ({
          id: v.id,
          impact: v.impact,
          description: v.description,
          nodes: v.nodes.map((n) => ({
            target: n.target.join(" "),
            html: n.html.slice(0, 200),
            summary: n.failureSummary ?? "",
          })),
        })),
        passes: typed.passes.length,
        incomplete: typed.incomplete.length,
      },
      keyboard: await page.evaluate(() => {
        const focusable = document.querySelectorAll(
          'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
        ).length;
        return {
          focusable_elements: focusable,
          skip_link_present: document.querySelector('a[href="#main"]') !== null,
          main_landmark_present: document.querySelector("main#main") !== null,
        };
      }),
      timestamp_unix_s: Math.floor(Date.now() / 1000),
    };
    return evidence;
  } finally {
    await browser.close();
  }
}

export function writeEvidence(evidence: AccessibilityEvidence): string {
  mkdirSync(EVIDENCE_DIR, { recursive: true });
  const path = join(EVIDENCE_DIR, "LF-005-ep033-m5.json");
  writeFileSync(path, JSON.stringify(evidence, null, 2) + "\n", "utf8");
  return path;
}

export async function main(): Promise<number> {
  const chromePath = process.env.CHROME_BIN ?? "/usr/bin/google-chrome";
  const run = runId();
  const html = renderOwnedSurfacesHtml();
  const evidence = await runAccessibilityScan(chromePath, run, html);
  const path = writeEvidence(evidence);
  console.log(
    `LF-005 accessibility scan: ${evidence.results.violations.length} violations, ${evidence.results.passes} passes`,
  );
  console.log(`evidence: ${path}`);
  if (evidence.results.violations.length > 0) {
    for (const v of evidence.results.violations) {
      console.log(
        `  violation: ${v.id} (${v.impact}) - ${v.nodes.length} nodes`,
      );
    }
    return 2;
  }
  return 0;
}

// Executed directly: `node dist/scan.js`
if (import.meta.url === `file://${process.argv[1]}`) {
  main().then((code) => process.exit(code));
}
