// EP-007 M5 combined live-fire ceremony (register + authenticate).
//
// Drives REAL Chrome via Playwright + CDP with a standards-compliant
// VIRTUAL WebAuthn authenticator. The passkey credential is created by
// the REAL registration flow in THIS browser context, so the same virtual
// authenticator can then satisfy the REAL authentication assertion
// (signCount 1 -> 2). Nothing is mocked: every HTTP exchange is the real
// Keycloak 26.7.0 wire surface.
//
// Phases:
//   1. register: owner password login -> webauthn-register-passwordless
//      required action -> navigator.credentials.create() -> resident
//      credential in the virtual authenticator -> callback code (exchanged,
//      then end-session to invalidate the registration session).
//   2. authenticate: fresh authorize request -> passkey-only login via the
//      conditional-UI authenticator -> navigator.credentials.get() against
//      the SAME virtual authenticator -> callback code -> token exchange.
//
// Prints one JSON object to stdout:
//   {"ok": true, "register": {...}, "authenticate": {...}}
// or {"ok": false, "reason": "..."}
//
// Usage:
//   node ep007_combined_ceremony.mjs <register-authorize-url> <auth-authorize-url>
//       <username> <password> <callback-uri> <token-endpoint>
//       <reg-code-verifier> <auth-code-verifier> <profile-dir> <out-file>
//
// Tokens are written to <out-file> (0600) and never printed.

import { chromium } from "/usr/lib/node_modules/@playwright/mcp/node_modules/playwright/index.mjs";
import { writeFileSync, chmodSync, rmSync } from "node:fs";
import { createHash, randomBytes } from "node:crypto";

const [
  regAuthorizeUrl,
  authAuthorizeUrl,
  username,
  password,
  callbackUri,
  tokenEndpoint,
  regVerifier,
  authVerifier,
  profileDir,
  outFile,
] = process.argv.slice(2);

function fail(reason) {
  console.log(JSON.stringify({ ok: false, reason }));
  process.exit(1);
}

function hashOf(raw) {
  if (!raw) return null;
  return createHash("sha256")
    .update(Buffer.from(raw))
    .digest("hex")
    .slice(0, 16);
}

async function exchange(code, verifier) {
  const body = new URLSearchParams({
    grant_type: "authorization_code",
    client_id: "nexus-app",
    code,
    redirect_uri: callbackUri,
    code_verifier: verifier,
  });
  const resp = await fetch(tokenEndpoint, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: body.toString(),
  });
  if (!resp.ok) {
    const text = await resp.text();
    fail(`token exchange failed: ${resp.status} ${text.slice(0, 200)}`);
  }
  return resp.json();
}

async function waitForCallback(timeoutMs) {
  await page.waitForURL((u) => u.toString().startsWith(callbackUri), {
    timeout: timeoutMs,
  });
  const code = new URL(page.url()).searchParams.get("code");
  if (!code) fail(`no code in callback; url=${page.url()}`);
  return code;
}

const browser = await chromium.launch({
  executablePath: "/usr/bin/google-chrome",
  headless: true,
  args: [
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--ignore-certificate-errors",
    // Keycloak binds IPv4 (0.0.0.0:8080); Chrome resolves `localhost`
    // to ::1 first, which refused the connection and produced a
    // chrome-error page. Force the IPv4 loopback. The realm passwordless
    // policy RP ID stays `localhost` (a registrable domain), so WebAuthn
    // still passes.
    "--host-resolver-rules=MAP localhost 127.0.0.1",
  ],
});

let page;
const credentialEvents = [];
const pageErrors = [];
const consoleErrors = [];

try {
  const context = await browser.newContext({
    acceptDownloads: false,
    ignoreHTTPSErrors: true,
  });
  page = await context.newPage();

  page.on("pageerror", (e) => pageErrors.push(e.message));
  page.on("console", (m) => {
    if (m.type() === "error") consoleErrors.push(m.text());
  });

  const cdp = await context.newCDPSession(page);
  await cdp.send("WebAuthn.enable", { enableUI: false });
  await cdp.send("WebAuthn.addVirtualAuthenticator", {
    options: {
      protocol: "ctap2",
      transport: "internal",
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  cdp.on("WebAuthn.credentialAdded", (p) => {
    credentialEvents.push({ ...p, type: "created" });
  });
  cdp.on("WebAuthn.credentialAsserted", (p) => {
    credentialEvents.push({ ...p, type: "asserted" });
  });

  // Phase 1: REGISTER
  await page.goto(regAuthorizeUrl, {
    waitUntil: "domcontentloaded",
    timeout: 30000,
  });
  await page.waitForLoadState("domcontentloaded", { timeout: 30000 });
  await page.fill("#username", username);
  await page.fill("#password", password);
  await page.click("#kc-login");
  await page.waitForLoadState("domcontentloaded", { timeout: 30000 });

  await page
    .waitForURL(/login-actions\/required-action/, { timeout: 30000 })
    .catch(() => {});
  const regHtml = await page.content();
  if (
    !regHtml.includes("navigator.credentials") &&
    !regHtml.includes("Register")
  ) {
    fail(`registration page not shown; url=${page.url()}`);
  }
  const clicked = await page
    .locator(
      "#registerWebAuthn, #register-webauthn, button[name='register'], input[name='register']",
    )
    .first()
    .click({ timeout: 10000 })
    .then(() => true)
    .catch(() => false);
  if (!clicked) fail("register button not found");

  const regCode = await waitForCallback(45000);
  const regTokens = await exchange(regCode, regVerifier);

  // End the registration session (logout) so phase 2 starts unauthenticated.
  const logoutUrl =
    `${tokenEndpoint.replace(/\/token$/, "/logout")}?` +
    new URLSearchParams({
      client_id: "nexus-app",
      post_logout_redirect_uri: callbackUri,
      id_token_hint: regTokens.id_token,
    });
  await page
    .goto(logoutUrl, { waitUntil: "domcontentloaded", timeout: 30000 })
    .catch(() => {});

  // Phase 2: AUTHENTICATE
  await page.goto(authAuthorizeUrl, {
    waitUntil: "domcontentloaded",
    timeout: 30000,
  });
  await page.waitForLoadState("domcontentloaded", { timeout: 30000 });

  await page
    .waitForURL((u) => u.toString().startsWith(callbackUri), { timeout: 45000 })
    .catch(async () => {
      // Conditional UI may need the username step first.
      try {
        await page.fill("#username", username);
        await page.click("#kc-login");
        await page.waitForLoadState("domcontentloaded", { timeout: 30000 });
      } catch {}
    });

  const authCode = await waitForCallback(45000);
  const authTokens = await exchange(authCode, authVerifier);

  const created = credentialEvents.find((e) => e.type === "created");
  const asserted = credentialEvents.find((e) => e.type === "asserted");
  const createdCred = created?.credential;
  const assertedCred = asserted?.credential;
  const createdHash = hashOf(createdCred?.credentialId);
  const assertedHash = hashOf(assertedCred?.credentialId);

  const safe = {
    ok: true,
    register: {
      codeLength: regCode.length,
      credentialAdded: credentialEvents.filter((e) => e.type === "created")
        .length,
      credentialIdHash: createdHash,
      resident: createdCred?.isResidentCredential ?? null,
      rpId: createdCred?.rpId ?? null,
      signCount: createdCred?.signCount ?? null,
    },
    authenticate: {
      codeLength: authCode.length,
      credentialAsserted: credentialEvents.filter((e) => e.type === "asserted")
        .length,
      credentialIdHash: assertedHash,
      signCount: assertedCred?.signCount ?? null,
      sameCredential:
        createdHash !== null &&
        assertedHash !== null &&
        createdHash === assertedHash,
    },
    pageErrors,
    consoleErrors,
  };

  const payload = {
    access_token: authTokens.access_token,
    refresh_token: authTokens.refresh_token,
    id_token: authTokens.id_token,
    expires_in: authTokens.expires_in,
    scope: authTokens.scope,
    register_tokens: {
      access_token: regTokens.access_token,
      refresh_token: regTokens.refresh_token,
      id_token: regTokens.id_token,
    },
    correlation: randomBytes(16).toString("hex"),
  };
  writeFileSync(outFile, JSON.stringify(payload, null, 1), { mode: 0o600 });
  chmodSync(outFile, 0o600);
  console.log(JSON.stringify(safe));
} catch (err) {
  try {
    fail(
      `driver error: ${err.message}; url=${page ? await page.url() : "none"}`,
    );
  } catch {
    fail(`driver error: ${err.message}`);
  }
} finally {
  await browser.close().catch(() => {});
  try {
    rmSync(profileDir, { recursive: true, force: true });
  } catch {}
}
