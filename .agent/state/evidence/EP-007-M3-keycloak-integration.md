# EP-007 M3 evidence - real Keycloak 26.7.0 authentication integration

Node: EP-007 (authentication: Keycloak, OIDC, passkeys, sessions, device
enrollment, step-up). Milestone M3: real dependency and transport integration.
Date: 2026-08-13.

## Pinned component

- Image: `quay.io/keycloak/keycloak:26.7.0`
- Digest: `sha256:0f198be292568439d700cdbfb893e69a6009bb43a94a06a945b1d3d506c76b13`
- Realm fixture: `tests/auth/nexus-realm.json` (realm `nexus`)
- Suite: `tests/auth/test_ep007_integration_keycloak.py`
- Gate: `sh scripts/nodes/EP-007.sh M3` -> `EP-007 M3: ok`

## Canonical authentication architecture (SPEC-005, EP-007 M3 directive)

- HUMAN clients: OIDC Authorization Code + PKCE S256, Keycloak-hosted
  interactive authentication. Direct Access Grant is NOT authorized for human
  login and is disabled on `nexus-app`; the real suite proves the password
  grant is rejected without revealing credential validity.
- SERVICE clients: client-credentials grant. `nexus-scheduler` holds the
  `nexus-admin` realm role; `nexus-connector-runtime` is under-scoped and is
  DENIED at the protected boundary (fail closed).
- admin-cli direct grant: used ONLY inside this ephemeral integration harness
  for Admin API provisioning (dedicated ephemeral bootstrap administrator;
  never a Nexus application login mechanism).

## Realm/user required-action diagnosis (directive A) - non-secret evidence

Observed against a live container via the real Admin API:

- Owner user: enabled=true, email=owner@nexus.local, emailVerified=true,
  firstName=Owner, lastName=Nexus, requiredActions=[] (empty), attributes={}.
- Owner password credential: type=password, NOT temporary.
- User profile config: username/email/firstName/lastName are required
  attributes for role `user` (view/edit by admin+user).
- Realm required actions: VERIFY_PROFILE enabled, defaultAction=false;
  VERIFY_EMAIL enabled, defaultAction=false; UPDATE_PASSWORD enabled,
  defaultAction=false; CONFIGURE_TOTP enabled, defaultAction=false;
  CONFIGURE_RECOVERY_AUTHN_CODES enabled, defaultAction=false;
  webauthn-register enabled, defaultAction=false; TERMS_AND_CONDITIONS
  disabled; UPDATE_EMAIL disabled.
- Direct-grant flow exists (alias "direct grant").

CONCLUSION: the earlier `invalid_grant` / `event=resolve_required_actions`
failure had no explicit required action on the user. The required action was
injected into the AUTHENTICATION SESSION dynamically because (a) the imported
owner had an incomplete user profile (missing firstName/lastName, which the
profile config requires) and/or (b) the password credential was flagged
temporary (UPDATE_PASSWORD). Both conditions are corrected in the canonical
bootstrap: profile fields are set and credentials are provisioned through the
Admin API. The interactive flow completes any legitimate required action
through the REAL form (proven by the UPDATE_PASSWORD test: a temporary
credential presents the Update Password page and the flow completes it).

## Real wire-surface evidence (directive G)

The suite proves against the pinned container:

- real OIDC discovery document (issuer, authorization/token endpoints,
  jwks_uri, grant types, S256 code challenge method, RS256 id token alg);
- real Authorization Code + PKCE owner login through the real login form,
  real 302 redirects, real authorization code delivered to the registered
  integration redirect URI, real token exchange with the original verifier;
- interactive required-action completion (UPDATE_PASSWORD) through the real
  form when the credential is temporary; the new password then authenticates
  and the old temporary password no longer does;
- canonical access-token claims mapping: iss, aud (includes the issuing
  client via a real oidc-audience-mapper), azp, sub, preferred_username,
  exp/iat, scope, acr (authentication context), tenant claim
  (0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01), typ=Bearer; id-token nonce binds
  the client nonce; RS256 signatures verified against the REAL JWKS
  (pure-stdlib modular-exponentiation verification; the frozen test env has
  no JWT/crypto library);
- refresh rotation: every refresh-token use issues a NEW access token;
  Keycloak 26.7.0 tolerates reuse of the previous refresh token by default
  (rotation with reuse tolerance; revocation is a client-policy concern for
  later nodes - recorded in the Decision Log);
- client credentials for service identities: token subject is the service
  account (not the owner), azp = client id, least-privilege roles;
- insufficiently scoped service client (nexus-connector-runtime, no
  nexus-admin role) is DENIED by the boundary validator (fail closed);
- Direct Access Grant against nexus-app is denied (HTTP 400
  unauthorized_client, "Client not allowed for direct access grants.") with an
  IDENTICAL response for the correct and a bogus password - the denial never
  reveals credential validity;
- failure paths: state mismatch rejected before token exchange; wrong PKCE
  verifier rejected by the real token endpoint; wrong nonce, wrong issuer,
  wrong audience rejected by the boundary validator;
- authorize endpoint rejects missing OAuth2 parameters;
- zero orphan containers after the suite (idempotent dispose + sweep).

## Keycloak 26.7.0 behaviors discovered (recorded in Decision Log)

1. Realm import file must be mode 0644 (container runs non-root).
2. Keycloak 26 requires KC_BOOTSTRAP_ADMIN_USERNAME / KC_BOOTSTRAP_ADMIN_PASSWORD
   for bootstrap admin in start-dev.
3. Realm JSON `clientScopes` section suppresses ALL built-in client scopes
   (profile, email, roles, acr, basic, ...); the custom `nexus` tenant-mapper
   scope is therefore created at bootstrap via the Admin API instead.
4. `openid` is an implicit OIDC scope, never listed/assigned as a client scope.
5. The `basic` client scope carries sub/sid/aud/azp/typ/auth_time into access
   tokens and must be assigned explicitly when per-client defaults are empty.
6. Access-token `aud` defaults to the `account` client; the canonical Nexus
   boundary requires the issuing client in aud, so each client gets a real
   oidc-audience-mapper (included.client.audience).
7. Admin REST verb changes in 26.7.0: users/{id}/reset-password is PUT (not
   POST); clients/{id}/default-client-scopes/{scopeId} is PUT (not POST).
8. Keycloak marks auth-session cookies Secure even over plain http; browsers
   send them on http://127.0.0.1 (loopback trust). urllib lacks the
   exemption, so the test client mirrors it explicitly (never a mock - the
   real cookies, real redirects, and real token endpoint are still used).
9. Direct Access Grant on a human client is denied at the CLIENT level before
   credential checks: identical unauthorized_client error for correct and
   incorrect passwords.

## Secrets discipline

Owner password, admin password, client secrets, and tokens are generated at
runtime, held in memory, and never printed or persisted. Pytest runs with
--tb=native so failure tracebacks never dump frame locals (pytest 9 dumps the
raising frame's locals by default; the M3/M4/M5 gate arms enforce native
tracebacks). The realm fixture contains NO credentials.
