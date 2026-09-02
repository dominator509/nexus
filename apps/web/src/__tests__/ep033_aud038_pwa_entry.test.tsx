import { describe, expect, it } from "vitest";
import { readFileSync, existsSync } from "node:fs";
import { execSync } from "node:child_process";
import { resolve } from "node:path";
import { renderToString } from "react-dom/server";
import {
  AuthenticatedSession,
  BoundContext,
  BusinessContext,
  DashboardShell,
  PresentedCapability,
} from "@nexus/web";
import { DashboardShellView, CapabilityButton } from "@nexus/ui";

/**
 * AUD-038 hostile suite: the web package must be an ACTUAL React PWA
 * entry, not just a contract layer. Proofs:
 *
 * 1. index.html exists with a real #root mount and the PWA manifest.
 * 2. src/main.tsx exists and imports the REAL @nexus/ui components
 *    (never a mock) and ReactDOM createRoot.
 * 3. A production Vite build emits dist/index.html + a JS bundle
 *    (the PWA is buildable, not a fixture).
 * 4. The same real components render through react-dom/server with
 *    contract state (the PWA's mount path is real production code).
 */
describe("ep033_aud038_pwa_entry", () => {
  const root = resolve(__dirname, "../..");

  it("index.html is a real PWA entry with root mount + manifest", () => {
    const html = readFileSync(resolve(root, "index.html"), "utf8");
    expect(html).toContain('<div id="root"></div>');
    expect(html).toContain('type="module"');
    expect(html).toContain("src/main.tsx");
    expect(html).toContain("manifest.webmanifest");
  });

  it("manifest.webmanifest declares the installable PWA", () => {
    const manifest = readFileSync(
      resolve(root, "public/manifest.webmanifest"),
      "utf8",
    );
    expect(manifest).toContain('"display": "standalone"');
    expect(manifest).toContain('"start_url": "/"');
  });

  it("main.tsx mounts REAL @nexus/ui components over contract state", () => {
    const source = readFileSync(resolve(root, "src/main.tsx"), "utf8");
    // Production components, never mocks.
    expect(source).toContain('from "@nexus/ui"');
    expect(source).toContain("DashboardShellView");
    expect(source).toContain("CapabilityButton");
    expect(source).toContain("ApprovalCardView");
    expect(source).toContain("StatusBadge");
    // Real ReactDOM client mount.
    expect(source).toContain('from "react-dom/client"');
    expect(source).toContain("createRoot");
    expect(source).toContain('document.getElementById("root")');
  });

  it("production build output exists (dist/index.html + JS bundle)", () => {
    // The unit suite must not depend on a pre-existing build artifact
    // from a previous local run: a fresh checkout has no dist/. The
    // REAL Vite build is the proof, so run it here (production build
    // is deterministic and fast for this surface). The build is a real
    // subprocess; under parallel ship-gate load it can exceed vitest's
    // 5000ms default, so this proof owns an explicit 30s budget. The
    // assertion is unchanged - dist/index.html must still exist and
    // reference the emitted JS bundle.
    execSync("pnpm pwa:build", {
      cwd: root,
      stdio: "pipe",
    });
    expect(existsSync(resolve(root, "dist/index.html"))).toBe(true);
    const assets = resolve(root, "dist/assets");
    const js = readFileSync(resolve(root, "dist/index.html"), "utf8");
    expect(js).toContain("/assets/");
    expect(js).toContain(".js");
    expect(existsSync(assets)).toBe(true);
  }, 30_000);

  it("the PWA components render the same production markup server-side", () => {
    const { context, shell, capability } = pwaFixtures();
    const html = renderToString(
      <div>
        <DashboardShellView
          shell={shell}
          context={context}
          connectivity="CONNECTED"
        />
        <CapabilityButton capability={capability} label="home.lights.query" />
      </div>,
    );
    expect(html).toContain('data-route="approvals"');
    expect(html).toContain('data-capability="home.lights.query"');
  });
});

function pwaFixtures() {
  function uuid(n: number): string {
    return `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
  }
  const session = AuthenticatedSession.fromWire({
    session_id: uuid(1),
    principal_id: uuid(2),
    tenant_id: uuid(3),
    device_id: uuid(4),
    grant_flow: "AUTHORIZATION_CODE",
    strength: "MULTI_FACTOR",
    created_at_unix_s: 1_700_000_000,
    expires_at_unix_s: 1_800_000_000,
    revoked: false,
    correlation: uuid(5),
  });
  const context = BoundContext.bind(
    session,
    BusinessContext.fromWire({
      tenant_id: uuid(3),
      principal_id: uuid(2),
      scope: "BUSINESS",
      business_id: uuid(10),
      correlation: uuid(5),
    }),
  );
  const shell = DashboardShell.create(
    "approvals",
    "approvals",
    "CONNECTED",
    context,
  );
  const capability = PresentedCapability.fromWire({
    capability_id: "home.lights.query",
    class: "QUERY",
    availability: "AVAILABLE",
    visible: true,
    authorized: true,
    required_approval: "NONE",
  });
  return { context, shell, capability };
}
