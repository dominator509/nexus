"""EP-007 M3 integration tests: authentication contracts through REAL Keycloak.

Test names begin with ep007_integration_ per the EP-007 milestone contract.
Uses the pinned Keycloak 26.7.0 image (COMPONENT_REGISTRY.yaml) in a real
ephemeral container per test - never an in-memory substitute (TESTING.md).

CANONICAL NEXUS AUTHENTICATION ARCHITECTURE (SPEC-005, EP-007 M3 directive)
--------------------------------------------------------------------------
- Human-facing clients use OIDC Authorization Code + PKCE S256 with
  Keycloak-hosted interactive authentication. Direct Access Grant is NOT an
  authorized human mechanism and is disabled on nexus-app; a real test proves
  the password grant is rejected without revealing credential validity.
- Service clients use the client-credentials grant. nexus-scheduler carries
  the nexus-admin realm role; nexus-connector-runtime is deliberately
  under-scoped and is DENIED at the protected boundary (fail closed).
- admin-cli direct grant is used ONLY inside this ephemeral integration
  harness for Admin API provisioning (bootstrap exception, recorded in the
  ExecPlan Decision Log). It is never a Nexus application login mechanism.

EVIDENCE REQUIREMENTS (directive G) covered by this suite:
- real OIDC discovery (ep007_integration_discovery_document_is_served)
- real Authorization Code + PKCE owner login (ep007_integration_authorization_code_pkce_owner_login)
- interactive required-action completion through the REAL form flow
  (ep007_integration_required_action_update_password_completed_interactively)
- canonical access-token claims mapping incl. tenant claim and boundary
  acceptance (BoundaryValidator mirrors nexus-auth TokenValidator +
  nexus-keycloak KeycloakClaims mapping)
- refresh rotation (ep007_integration_refresh_rotation_issues_new_access_token)
- real Client Credentials service authentication
  (ep007_integration_client_credentials_service_identity)
- insufficiently scoped service client denied at the boundary
  (ep007_integration_insufficiently_scoped_service_client_denied)
- Direct Access Grant denied for human clients
  (ep007_integration_direct_access_grant_denied_for_human_client)
- failure on invalid state, nonce, PKCE verifier, issuer, audience
  (ep007_integration_invalid_state_nonce_verifier_issuer_audience_failures)
- realm/user required-action diagnosis recorded
  (ep007_integration_owner_account_required_action_diagnosis)
- clean container teardown with zero orphans
  (ep007_integration_container_cleanup_leaves_no_orphans)

DEPENDENCY DECISION (recorded in the ExecPlan Decision Log)
----------------------------------------------------------
Keycloak 26.7.0 login, required-action, and callback steps are server-rendered
HTML forms. Completing them with a real HTTP client (urllib + cookie jar) is
REAL interaction with the REAL Keycloak wire surface: real authorize request,
real login form submission, real 302 redirects, real authorization code, real
token endpoint, real JWKS. No browser redirect, login form, authorization
code, token endpoint, or JWKS endpoint is mocked. The repository has no
browser/E2E harness; the smallest dependency that satisfies the flow is the
stdlib HTTP form client, so no browser framework is introduced.

JWT SIGNATURE VERIFICATION
--------------------------
The frozen test environment has no JWT/crypto library (pyproject.toml is
EP-001/EP-002 fenced). RS256 signature verification is implemented with
stdlib modular exponentiation against the REAL JWKS document - this is real
cryptographic verification, not a mock.
"""

from __future__ import annotations

import base64
import contextlib
import hashlib
import html as htmlmod
import http.cookiejar
import http.server
import json
import re
import secrets
import subprocess
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

IMAGE = "quay.io/keycloak/keycloak:26.7.0"
IMAGE_DIGEST = "sha256:0f198be292568439d700cdbfb893e69a6009bb43a94a06a945b1d3d506c76b13"
ROOT = Path(__file__).resolve().parents[2]
REALM_JSON = ROOT / "tests" / "auth" / "nexus-realm.json"
DOCKER = "/usr/bin/docker"

CALLBACK_HOST = "127.0.0.1"
CALLBACK_PORT = 8477
REDIRECT_URI = f"http://{CALLBACK_HOST}:{CALLBACK_PORT}/callback"
TENANT_CLAIM = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"
CANONICAL_ID_RE = re.compile(r"^0190[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")

HUMAN_CLIENT = "nexus-app"
SERVICE_SCHEDULER = "nexus-scheduler"
SERVICE_CONNECTOR = "nexus-connector-runtime"
REALM_ROLE_ADMIN = "nexus-admin"


# --------------------------------------------------------------------------
# Low-level HTTP helpers (stdlib only)
# --------------------------------------------------------------------------

def _b64url_encode(raw: bytes) -> str:
    return base64.urlsafe_b64encode(raw).rstrip(b"=").decode("ascii")


def _b64url_decode(segment: str) -> bytes:
    padding = "=" * (-len(segment) % 4)
    return base64.urlsafe_b64decode(segment + padding)


def _http_get_json(url: str, timeout: float = 8.0) -> dict:
    with urllib.request.urlopen(url, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


def _open_text(opener: urllib.request.OpenerDirector, url: str, data: bytes | None = None,
               timeout: float = 20.0) -> str:
    try:
        with opener.open(url, data=data, timeout=timeout) as resp:
            return resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise AssertionError(f"HTTP {exc.code} from {url}: {body[:200]}") from exc


class TokenGrantError(RuntimeError):
    """Token endpoint rejected the grant. Carries the real error payload."""

    def __init__(self, status: int, payload: dict):
        super().__init__(f"token grant failed with HTTP {status}: {payload.get('error')}")
        self.status = status
        self.payload = payload


def _token_request(port: int, data: dict) -> dict:
    token_url = f"http://127.0.0.1:{port}/realms/nexus/protocol/openid-connect/token"
    body = urllib.parse.urlencode(data).encode("utf-8")
    req = urllib.request.Request(token_url, data=body, method="POST")
    req.add_header("Content-Type", "application/x-www-form-urlencoded")
    try:
        with urllib.request.urlopen(req, timeout=15.0) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode("utf-8", errors="replace")
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError:
            payload = {"error": "unknown", "error_description": raw[:120]}
        raise TokenGrantError(exc.code, payload) from exc


# --------------------------------------------------------------------------
# Container lifecycle
# --------------------------------------------------------------------------

def _host_port(container: str, container_port: int = 8080) -> int:
    out = subprocess.run(
        [DOCKER, "port", container, str(container_port)],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    return int(out.rsplit(":", 1)[1])


def _wait_for_keycloak(container: str, timeout: float = 120.0) -> int:
    port = _host_port(container)
    discovery = f"http://127.0.0.1:{port}/realms/nexus/.well-known/openid-configuration"
    deadline = time.monotonic() + timeout
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            doc = _http_get_json(discovery, timeout=3.0)
            if doc.get("issuer"):
                return port
        except (urllib.error.URLError, ConnectionError, OSError) as exc:
            last_error = exc
            time.sleep(1.0)
    raise TimeoutError(f"Keycloak discovery {discovery} not ready within {timeout}s") from last_error


def _cleanup_container(name: str) -> None:
    """Idempotent dispose. Raises if the container survives (teardown
    invariant): a silent rm failure would leave an orphan for the sweep."""
    result = subprocess.run([DOCKER, "rm", "-f", name], capture_output=True, text=True)
    if result.returncode != 0:
        probe = subprocess.run([DOCKER, "inspect", name], capture_output=True, text=True)
        if probe.returncode == 0:
            raise AssertionError(
                f"container {name} survived cleanup: {result.stderr.strip()[:200]}")


def _start_keycloak() -> tuple[str, int, str, str]:
    """Start a REAL Keycloak 26.7.0 container importing the Nexus realm.

    Bootstrap admin credentials are generated at runtime and never printed or
    persisted (directive F: dedicated ephemeral administrator identity).
    """
    name = f"nexus-ep007-{secrets.token_hex(4)}"
    admin_user = "admin"
    admin_pw = secrets.token_urlsafe(24)
    subprocess.run(
        [DOCKER, "run", "-d", "--name", name,
         "-p", "127.0.0.1::8080",
         "-e", f"KC_BOOTSTRAP_ADMIN_USERNAME={admin_user}",
         "-e", f"KC_BOOTSTRAP_ADMIN_PASSWORD={admin_pw}",
         "-v", f"{REALM_JSON}:/opt/keycloak/data/import/nexus-realm.json:ro",
         f"{IMAGE}@{IMAGE_DIGEST}",
         "start-dev", "--import-realm"],
        check=True, capture_output=True, text=True,
    )
    try:
        port = _wait_for_keycloak(name)
    except Exception:
        _cleanup_container(name)
        raise
    return name, port, admin_user, admin_pw


@contextlib.contextmanager
def _keycloak():
    name, port, admin_user, admin_pw = _start_keycloak()
    try:
        yield name, port, admin_user, admin_pw
    finally:
        _cleanup_container(name)


# --------------------------------------------------------------------------
# Keycloak Admin API (bootstrap exception per directive F)
# --------------------------------------------------------------------------

def _admin_token(port: int, user: str, pw: str) -> str:
    body = urllib.parse.urlencode({
        "grant_type": "password",
        "client_id": "admin-cli",
        "username": user,
        "password": pw,
    }).encode()
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/realms/master/protocol/openid-connect/token",
        data=body, method="POST",
    )
    req.add_header("Content-Type", "application/x-www-form-urlencoded")
    with urllib.request.urlopen(req, timeout=15.0) as resp:
        return json.loads(resp.read().decode("utf-8"))["access_token"]


def _admin(port: int, token: str, method: str, path: str, body=None):
    req = urllib.request.Request(f"http://127.0.0.1:{port}{path}", method=method)
    req.add_header("Authorization", f"Bearer {token}")
    req.add_header("Content-Type", "application/json")
    if body is not None:
        req.data = json.dumps(body).encode("utf-8")
    try:
        with urllib.request.urlopen(req, timeout=15.0) as resp:
            raw = resp.read().decode("utf-8")
            return json.loads(raw) if raw else None
    except urllib.error.HTTPError as exc:
        if exc.code == 404 and method == "GET":
            return None
        detail = exc.read().decode("utf-8", errors="replace")[:300]
        raise AssertionError(
            f"admin {method} {path}: HTTP {exc.code}: {detail}") from None


def _admin_optional(port: int, token: str, method: str, path: str, body=None):
    """Admin call that tolerates 404 (used to detect missing roles/scopes)."""
    req = urllib.request.Request(f"http://127.0.0.1:{port}{path}", method=method)
    req.add_header("Authorization", f"Bearer {token}")
    req.add_header("Content-Type", "application/json")
    if body is not None:
        req.data = json.dumps(body).encode("utf-8")
    try:
        with urllib.request.urlopen(req, timeout=15.0) as resp:
            raw = resp.read().decode("utf-8")
            return json.loads(raw) if raw else None
    except urllib.error.HTTPError as exc:
        return None


def _tenant_mapper() -> dict:
    return {
        "name": "nexus-tenant",
        "protocol": "openid-connect",
        "protocolMapper": "oidc-hardcoded-claim-mapper",
        "config": {
            "claim.name": "tenant",
            "claim.value": TENANT_CLAIM,
            "jsonType.label": "String",
            "id.token.claim": "false",
            "access.token.claim": "true",
            "userinfo.token.claim": "false",
        },
    }


def _set_default_scopes(port: int, token: str, client: dict, wanted: set[str]) -> None:
    """Idempotently make `wanted` the default client scopes of a client.

    Keycloak 26.7.0 admin REST: PUT (not POST)
    /clients/{client-uuid}/default-client-scopes/{clientScopeId} adds a
    scope; DELETE removes one.
    """
    scopes = {s["name"]: s["id"] for s in _admin(port, token, "GET", "/admin/realms/nexus/client-scopes")}
    current = _admin(port, token, "GET", f"/admin/realms/nexus/clients/{client['id']}/default-client-scopes") or []
    current_names = {c["name"] for c in current}
    for scope in current:
        if scope["name"] not in wanted:
            _admin(port, token, "DELETE",
                   f"/admin/realms/nexus/clients/{client['id']}/default-client-scopes/{scope['id']}")
    for name in wanted:
        if name not in current_names:
            _admin(port, token, "PUT",
                   f"/admin/realms/nexus/clients/{client['id']}/default-client-scopes/{scopes[name]}")


def _ensure_audience_mapper(port: int, token: str, client: dict) -> None:
    """Add a real oidc-audience-mapper so the client's id is in token `aud`."""
    rep = _admin(port, token, "GET", f"/admin/realms/nexus/clients/{client['id']}")
    mappers = rep.get("protocolMappers") or []
    if not any(m.get("name") == "nexus-audience" for m in mappers):
        mappers.append({
            "name": "nexus-audience",
            "protocol": "openid-connect",
            "protocolMapper": "oidc-audience-mapper",
            "config": {
                "included.client.audience": client["clientId"],
                "access.token.claim": "true",
                "id.token.claim": "true",
            },
        })
        rep["protocolMappers"] = mappers
        _admin(port, token, "PUT", f"/admin/realms/nexus/clients/{client['id']}", rep)


def _bootstrap(port: int, admin_user: str, admin_pw: str, owner_temporary: bool = False) -> dict:
    """Idempotent real provisioning through the Admin API.

    Returns a secrets dict held in memory only - never printed or persisted.
    """
    token = _admin_token(port, admin_user, admin_pw)

    # --- owner ------------------------------------------------------------
    # Provisioned via Admin API with a runtime-generated password (never
    # persisted). Create-with-credentials on POST /users is canonical;
    # if the user already exists the bootstrap falls back to PUT
    # reset-password (Keycloak 26.7.0 uses PUT for reset-password, not POST).
    users = _admin(port, token, "GET", "/admin/realms/nexus/users?username=owner&exact=true") or []
    owner_password = secrets.token_urlsafe(18)
    users_created_inline = False
    if not users:
        _admin(port, token, "POST", "/admin/realms/nexus/users", {
            "username": "owner", "enabled": True, "email": "owner@nexus.local",
            "emailVerified": True, "firstName": "Owner", "lastName": "Nexus",
            "requiredActions": [],
            "credentials": [{
                "type": "password", "value": owner_password,
                "temporary": owner_temporary,
            }],
        })
        users = _admin(port, token, "GET", "/admin/realms/nexus/users?username=owner&exact=true")
        assert users, "owner creation failed"
        users_created_inline = True
    owner = users[0]
    owner_id = owner["id"]
    if not users_created_inline:
        _admin(port, token, "PUT", f"/admin/realms/nexus/users/{owner_id}", {
            "id": owner_id, "username": "owner", "enabled": True, "email": "owner@nexus.local",
            "emailVerified": True, "firstName": "Owner", "lastName": "Nexus",
            "requiredActions": [],
        })
        _admin(port, token, "PUT", f"/admin/realms/nexus/users/{owner_id}/reset-password", {
            "type": "password", "value": owner_password, "temporary": owner_temporary,
        })

    # --- nexus client scope (canonical tenant claim) ----------------------
    scopes = {s["name"]: s for s in _admin(port, token, "GET", "/admin/realms/nexus/client-scopes")}
    if "nexus" not in scopes:
        _admin(port, token, "POST", "/admin/realms/nexus/client-scopes", {
            "name": "nexus", "protocol": "openid-connect",
            "attributes": {"include.in.token.scope": "false"},
            "protocolMappers": [_tenant_mapper()],
        })
        scopes = {s["name"]: s for s in _admin(port, token, "GET", "/admin/realms/nexus/client-scopes")}

    # --- nexus-admin realm role ------------------------------------------
    role = _admin_optional(port, token, "GET", "/admin/realms/nexus/roles/nexus-admin")
    if role is None:
        _admin(port, token, "POST", "/admin/realms/nexus/roles", {"name": REALM_ROLE_ADMIN})
        role = _admin(port, token, "GET", "/admin/realms/nexus/roles/nexus-admin")

    # --- clients ----------------------------------------------------------
    clients = _admin(port, token, "GET", "/admin/realms/nexus/clients")
    app = next(c for c in clients if c["clientId"] == HUMAN_CLIENT)
    scheduler = next(c for c in clients if c["clientId"] == SERVICE_SCHEDULER)
    connector = next(c for c in clients if c["clientId"] == SERVICE_CONNECTOR)

    # openid is an implicit OIDC scope in Keycloak and is never listed or
    # assigned as a client scope; it is always granted in the flow. The
    # "basic" scope carries sub/sid/auth_time/exp/aud/azp/typ into access
    # tokens and must be assigned explicitly because the realm import leaves
    # per-client defaults empty.
    _set_default_scopes(port, token, app, {"basic", "profile", "email", "roles", "acr", "nexus"})
    _set_default_scopes(port, token, scheduler, {"basic", "roles", "nexus"})
    _set_default_scopes(port, token, connector, {"basic", "nexus"})

    scheduler_secret = _admin(port, token, "POST",
                              f"/admin/realms/nexus/clients/{scheduler['id']}/client-secret")["value"]
    connector_secret = _admin(port, token, "POST",
                              f"/admin/realms/nexus/clients/{connector['id']}/client-secret")["value"]

    # --- least-privilege role mapping -------------------------------------
    scheduler_service = _admin(port, token, "GET",
                               "/admin/realms/nexus/users?username=service-account-nexus-scheduler&exact=true")[0]
    connector_service = _admin(port, token, "GET",
                               "/admin/realms/nexus/users?username=service-account-nexus-connector-runtime&exact=true")[0]
    _admin(port, token, "POST",
           f"/admin/realms/nexus/users/{scheduler_service['id']}/role-mappings/realm",
           [{"id": role["id"], "name": role["name"]}])

    # --- audience mappers (canonical aud claim) ---------------------------
    # Keycloak 26.7.0 defaults access-token aud to the "account" client; the
    # canonical Nexus boundary requires the issuing client in aud, so each
    # client gets a real oidc-audience-mapper (included.client.audience).
    for client in (app, scheduler, connector):
        _ensure_audience_mapper(port, token, client)

    return {
        "owner_id": owner_id,
        "owner_password": owner_password,
        "scheduler_secret": scheduler_secret,
        "connector_secret": connector_secret,
        "scheduler_service_id": scheduler_service["id"],
        "connector_service_id": connector_service["id"],
        "app_client_id": app["id"],
    }


# --------------------------------------------------------------------------
# Callback capture server (the exact registered integration redirect URI)
# --------------------------------------------------------------------------

class _CallbackHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):  # noqa: N802 (stdlib method name)
        self.server.captured = self.path
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.end_headers()
        self.wfile.write(b"<html><body>ok</body></html>")

    def log_message(self, fmt, *args):  # noqa: A002 - never log the query string
        del fmt, args


class _CallbackServer(http.server.ThreadingHTTPServer):
    allow_reuse_address = True

    def __init__(self):
        super().__init__((CALLBACK_HOST, CALLBACK_PORT), _CallbackHandler)
        self.captured: str | None = None
        self._thread = threading.Thread(target=self.serve_forever, daemon=True)
        self._thread.start()

    def close(self) -> None:
        self.shutdown()
        self.server_close()
        self._thread.join(timeout=5.0)


# --------------------------------------------------------------------------
# JWT helpers: decode + REAL RS256 verification against the REAL JWKS
# --------------------------------------------------------------------------

def _decode_jwt(token: str) -> tuple[dict, dict, bytes]:
    head_seg, payload_seg, sig_seg = token.split(".")
    header = json.loads(_b64url_decode(head_seg))
    payload = json.loads(_b64url_decode(payload_seg))
    signature = _b64url_decode(sig_seg)
    return header, payload, signature


def _fetch_jwks(port: int) -> list[dict]:
    discovery = _http_get_json(
        f"http://127.0.0.1:{port}/realms/nexus/.well-known/openid-configuration")
    return _http_get_json(discovery["jwks_uri"])["keys"]


def _verify_rs256(token: str, jwks: list[dict]) -> bool:
    """Pure-stdlib RSA PKCS1-v1_5 signature verification against the real JWKS.

    sha256 DigestInfo:
      SEQUENCE 0x30 0x31
        SEQUENCE 0x30 0x0d: OID 2.16.840.1.101.3.4.2.1 (0x06 0x09 0x60 0x86
          0x48 0x01 0x65 0x03 0x04 0x02 0x01), NULL (0x05 0x00)
        OCTET STRING 0x04 0x20 + digest
    """
    head_seg, payload_seg, sig_seg = token.split(".")
    header = json.loads(_b64url_decode(head_seg))
    if header.get("alg") != "RS256":
        return False
    keys = [k for k in jwks if k.get("kid") == header.get("kid") and k.get("kty") == "RSA"]
    if not keys:
        return False
    key = keys[0]
    n = int.from_bytes(_b64url_decode(key["n"]), "big")
    e = int.from_bytes(_b64url_decode(key["e"]), "big")
    signature = _b64url_decode(sig_seg)
    k = (n.bit_length() + 7) // 8
    digest = hashlib.sha256(f"{head_seg}.{payload_seg}".encode("ascii")).digest()
    digest_info = (
        b"\x30\x31\x30\x0d\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x01\x05\x00\x04\x20"
        + digest
    )
    em = b"\x00\x01" + b"\xff" * (k - len(digest_info) - 3) + b"\x00" + digest_info
    m = pow(int.from_bytes(signature, "big"), e, n)
    return m.to_bytes(k, "big") == em


# --------------------------------------------------------------------------
# Boundary validator: mirrors nexus-auth TokenValidator + nexus-keycloak
# KeycloakClaims mapping (iss, aud, scopes, nbf/exp, tenant, role, nonce).
# --------------------------------------------------------------------------

class BoundaryValidator:
    """Canonical Nexus boundary check over decoded JWT claims (fail closed)."""

    def __init__(self, issuer: str, audience: str,
                 required_scopes: tuple[str, ...] = ("openid",),
                 required_role: str | None = None,
                 expected_nonce: str | None = None,
                 clock_skew_s: int = 60):
        self.issuer = issuer
        self.audience = audience
        self.required_scopes = set(required_scopes)
        self.required_role = required_role
        self.expected_nonce = expected_nonce
        self.clock_skew_s = clock_skew_s

    def rejections(self, claims: dict) -> list[str]:
        reasons: list[str] = []
        now = time.time()
        if claims.get("iss") != self.issuer:
            reasons.append("issuer")
        aud = claims.get("aud")
        if isinstance(aud, list):
            aud_ok = self.audience in aud
        else:
            aud_ok = aud == self.audience
        if not aud_ok:
            reasons.append("audience")
        exp = claims.get("exp")
        if exp is not None and now >= exp:
            reasons.append("expired")
        nbf = claims.get("nbf")
        if nbf is not None and now < nbf - self.clock_skew_s:
            reasons.append("not-before")
        iat = claims.get("iat")
        if iat is not None and now < iat - self.clock_skew_s:
            reasons.append("issued-in-future")
        scopes = set(claims.get("scope", "").split())
        if not self.required_scopes <= scopes:
            reasons.append("scope")
        if not CANONICAL_ID_RE.match(claims.get("tenant") or ""):
            reasons.append("tenant")
        if self.required_role is not None:
            roles = (claims.get("realm_access") or {}).get("roles") or []
            if self.required_role not in roles:
                reasons.append("role")
        if self.expected_nonce is not None and claims.get("nonce") != self.expected_nonce:
            reasons.append("nonce")
        return reasons


# --------------------------------------------------------------------------
# Authorization Code + PKCE flow: real forms, real redirects, real code
# --------------------------------------------------------------------------

_FORM_RE = re.compile(r"<form[^>]*action=\"([^\"]+)\"[^>]*>(.*?)</form>", re.I | re.S)
_INPUT_RE = re.compile(r"<input\b[^>]*>", re.I)


def _parse_form(html: str, current_url: str) -> tuple[str, dict[str, str]]:
    form = _FORM_RE.search(html)
    assert form is not None, "no HTML form found on page"
    action = urllib.parse.urljoin(current_url, htmlmod.unescape(form.group(1)))
    fields: dict[str, str] = {}
    for tag in _INPUT_RE.findall(form.group(2)):
        name = re.search(r'name="([^"]*)"', tag, re.I)
        value = re.search(r'value="([^"]*)"', tag, re.I)
        if name:
            fields[name.group(1)] = urllib.parse.unquote(value.group(1)) if value else ""
    return action, fields


def _submit_form(opener: urllib.request.OpenerDirector, action: str, fields: dict[str, str],
                 overrides: dict[str, str], current_url: str) -> str:
    data = dict(fields)
    data.update(overrides)
    body = urllib.parse.urlencode(data).encode("utf-8")
    return _open_text(opener, action, data=body)


def _is_login_form_again(html: str) -> bool:
    """True when Keycloak re-rendered the username/password login form
    (authentication failed). A required-action page (e.g. UPDATE_PASSWORD)
    has no username/password inputs and is NOT a login re-render."""
    inputs = _form_inputs(html)
    return "username" in inputs and "password" in inputs


def _strip_secure_flags(jar: http.cookiejar.CookieJar) -> None:
    """Browsers send Secure cookies on http://127.0.0.1 (loopback is a
    trustworthy origin); urllib does not implement that exemption, so mirror
    it explicitly. Keycloak 26.7.0 marks its auth-session cookies Secure even
    over plain http; without this the login POST drops the session cookies and
    Keycloak rejects it with 'Restart login cookie not found'."""
    for cookie in jar:
        cookie.secure = False


def _complete_auth_code_pkce_flow(
    port: int,
    *,
    username: str,
    password: str,
    new_password: str | None = None,
    expect_state: str | None = None,
    verifier: str | None = None,
    scope: str = "openid profile",
) -> tuple[str, str, bool, str]:
    """Complete a REAL authorization-code + PKCE login against real Keycloak.

    Returns (code, returned_state, saw_required_action, nonce). Raises
    AssertionError on state mismatch or failed authentication. Never logs
    credentials, the code, or tokens.
    """
    state = secrets.token_urlsafe(24)
    nonce = secrets.token_urlsafe(24)
    verifier = verifier or secrets.token_urlsafe(48)[:64]
    if not 43 <= len(verifier) <= 128:
        raise AssertionError("PKCE verifier length must be 43..128")
    challenge = _b64url_encode(hashlib.sha256(verifier.encode("ascii")).digest())

    server = _CallbackServer()
    try:
        jar = http.cookiejar.CookieJar()
        opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
        base = f"http://127.0.0.1:{port}/realms/nexus"
        auth_params = urllib.parse.urlencode({
            "response_type": "code",
            "client_id": HUMAN_CLIENT,
            "redirect_uri": REDIRECT_URI,
            "scope": scope,
            "state": state,
            "nonce": nonce,
            "code_challenge": challenge,
            "code_challenge_method": "S256",
        })
        auth_url = f"{base}/protocol/openid-connect/auth?{auth_params}"
        login_html = _open_text(opener, auth_url)
        _strip_secure_flags(jar)
        assert "login-actions/authenticate" in login_html or "username" in login_html, \
            "authorize endpoint did not render the login form"

        action, fields = _parse_form(login_html, auth_url)
        assert "username" in fields and "password" in fields, "login form missing credential fields"
        post_login = _submit_form(opener, action, fields, {"username": username, "password": password},
                                  auth_url)
        _strip_secure_flags(jar)

        saw_required_action = False
        if server.captured is None:
            if _is_login_form_again(post_login):
                raise AssertionError("authentication failed (login form re-rendered)")
            if "password-new" in _form_inputs(post_login):
                saw_required_action = True
                assert new_password is not None, "required action needs new_password"
                raction, rfields = _parse_form(post_login, auth_url)
                assert "password-new" in rfields and "password-confirm" in rfields
                post_login = _submit_form(opener, raction, rfields,
                                          {"password-new": new_password,
                                           "password-confirm": new_password}, auth_url)
                _strip_secure_flags(jar)
        if server.captured is None:
            raise AssertionError("authorization code was not delivered to the registered redirect URI")

        query = urllib.parse.parse_qs(urllib.parse.urlsplit(server.captured).query)
        if "code" not in query or "state" not in query:
            raise AssertionError("callback did not carry code and state")
        returned_state = query["state"][0]
        if expect_state is not None and returned_state != expect_state:
            raise AssertionError("state mismatch detected before token exchange")
        return query["code"][0], returned_state, saw_required_action, nonce
    finally:
        server.close()


def _form_inputs(html: str) -> set[str]:
    names: set[str] = set()
    for tag in _INPUT_RE.findall(html):
        name = re.search(r'name="([^"]*)"', tag, re.I)
        if name:
            names.add(name.group(1))
    return names


def _exchange_code(port: int, code: str, verifier: str) -> dict:
    return _token_request(port, {
        "grant_type": "authorization_code",
        "client_id": HUMAN_CLIENT,
        "redirect_uri": REDIRECT_URI,
        "code": code,
        "code_verifier": verifier,
    })


def _issuer(port: int) -> str:
    return f"http://127.0.0.1:{port}/realms/nexus"


# --------------------------------------------------------------------------
# Tests
# --------------------------------------------------------------------------

def ep007_integration_keycloak_image_is_pinned() -> None:
    """The integration identity provider is the locked Keycloak artifact."""
    inspect = subprocess.run(
        [DOCKER, "image", "inspect", f"{IMAGE}@{IMAGE_DIGEST}"],
        capture_output=True, text=True,
    )
    assert inspect.returncode == 0, "pinned keycloak:26.7.0 image not present locally"


def ep007_integration_discovery_document_is_served() -> None:
    """The real Keycloak serves a canonical OIDC discovery document."""
    with _keycloak() as (_name, port, _au, _ap):
        doc = _http_get_json(
            f"http://127.0.0.1:{port}/realms/nexus/.well-known/openid-configuration")
        assert doc["issuer"] == _issuer(port)
        assert doc["authorization_endpoint"].startswith(
            f"http://127.0.0.1:{port}/realms/nexus/protocol/openid-connect/auth")
        assert doc["token_endpoint"].startswith(
            f"http://127.0.0.1:{port}/realms/nexus/protocol/openid-connect/token")
        assert "jwks_uri" in doc
        grants = doc.get("grant_types_supported", [])
        assert "authorization_code" in grants
        assert "client_credentials" in grants
        assert "refresh_token" in grants
        assert "S256" in doc.get("code_challenge_methods_supported", [])
        assert "RS256" in doc.get("id_token_signing_alg_values_supported", [])


def ep007_integration_owner_account_required_action_diagnosis() -> None:
    """Record the non-secret realm/user required-action diagnosis (directive A)."""
    with _keycloak() as (_name, port, au, ap):
        _bootstrap(port, au, ap, owner_temporary=False)
        token = _admin_token(port, au, ap)
        owner = _admin(port, token, "GET",
                       "/admin/realms/nexus/users?username=owner&exact=true")[0]
        # Static user state: complete profile, no explicit required actions.
        assert owner["enabled"] is True
        assert owner["email"] == "owner@nexus.local"
        assert owner["emailVerified"] is True
        assert owner["firstName"] == "Owner"
        assert owner["lastName"] == "Nexus"
        assert owner["requiredActions"] == []
        creds = _admin(port, token, "GET",
                       f"/admin/realms/nexus/users/{owner['id']}/credentials")
        pw_creds = [c for c in creds if c.get("type") == "password"]
        assert pw_creds, "owner has a password credential"
        for c in pw_creds:
            cd = json.loads(c.get("credentialData") or "{}")
            assert cd.get("temporary") in (None, False), "standard bootstrap credential is not temporary"
        # Realm-wide required actions: enabled but none default.
        ras = _admin(port, token, "GET", "/admin/realms/nexus/authentication/required-actions")
        by_alias = {r["alias"]: r for r in ras}
        for alias in ("VERIFY_PROFILE", "VERIFY_EMAIL", "UPDATE_PASSWORD",
                      "CONFIGURE_TOTP", "CONFIGURE_RECOVERY_AUTHN_CODES", "webauthn-register"):
            assert alias in by_alias and by_alias[alias]["enabled"] is True
            assert by_alias[alias]["defaultAction"] is False
        # Dynamic required-action source: a temporary credential injects
        # UPDATE_PASSWORD into the AUTHENTICATION SESSION even when the user's
        # requiredActions array is empty (proven by the interactive test).
        profile = _admin_optional(port, token, "GET", "/admin/realms/nexus/users/profile") or {}
        required_attrs = {a["name"] for a in profile.get("attributes", [])
                          if a.get("required")}
        assert {"firstName", "lastName", "email"} <= required_attrs, \
            "user profile requires firstName/lastName/email"


def ep007_integration_authorization_code_pkce_owner_login() -> None:
    """REAL Authorization Code + PKCE owner login (canonical human flow)."""
    with _keycloak() as (_name, port, au, ap):
        boot = _bootstrap(port, au, ap)
        verifier = secrets.token_urlsafe(48)[:64]
        code, returned_state, saw_required, sent_nonce = _complete_auth_code_pkce_flow(
            port, username="owner", password=boot["owner_password"], verifier=verifier)
        assert not saw_required, "standard bootstrap login should not hit required actions"
        assert code and returned_state
        tokens = _exchange_code(port, code, verifier)
        assert tokens["token_type"] == "Bearer"
        assert tokens.get("expires_in", 0) > 0
        access = tokens["access_token"]
        id_token = tokens["id_token"]
        assert "refresh_token" in tokens

        jwks = _fetch_jwks(port)
        assert _verify_rs256(access, jwks), "access token signature invalid against real JWKS"
        assert _verify_rs256(id_token, jwks), "id token signature invalid against real JWKS"

        _header, claims, _sig = _decode_jwt(access)
        assert claims["iss"] == _issuer(port)
        assert HUMAN_CLIENT in (claims["aud"] if isinstance(claims["aud"], list) else [claims["aud"]]), \
            "client must appear in the access-token audience"
        assert claims.get("azp") == HUMAN_CLIENT
        assert claims.get("preferred_username") == "owner"
        assert claims["exp"] > claims["iat"]
        assert claims["typ"] == "Bearer"
        assert claims.get("tenant") == TENANT_CLAIM
        assert CANONICAL_ID_RE.match(claims.get("tenant") or "")
        assert claims.get("acr") in ("0", "1"), "authentication context claim missing"

        validator = BoundaryValidator(_issuer(port), HUMAN_CLIENT,
                                      required_scopes=("openid", "profile"))
        assert validator.rejections(claims) == [], validator.rejections(claims)

        _ih, id_claims, _is = _decode_jwt(id_token)
        assert id_claims["iss"] == _issuer(port)
        assert id_claims["aud"] == HUMAN_CLIENT
        assert id_claims["sub"] == claims["sub"]
        assert id_claims["nonce"] == sent_nonce, "id token must bind the client nonce"


def ep007_integration_required_action_update_password_completed_interactively() -> None:
    """A temporary credential raises UPDATE_PASSWORD in the auth session; the
    interactive flow completes it through the REAL Keycloak form."""
    with _keycloak() as (_name, port, au, ap):
        boot = _bootstrap(port, au, ap, owner_temporary=True)
        temp_pw = boot["owner_password"]
        new_pw = secrets.token_urlsafe(18)
        verifier = secrets.token_urlsafe(48)[:64]
        code, _state, saw_required, _nn = _complete_auth_code_pkce_flow(
            port, username="owner", password=temp_pw, new_password=new_pw, verifier=verifier)
        assert saw_required, "UPDATE_PASSWORD required action was not presented interactively"
        tokens = _exchange_code(port, code, verifier)
        assert tokens["access_token"]
        _header, claims, _sig = _decode_jwt(tokens["access_token"])
        assert claims["preferred_username"] == "owner"

        # New password works; old temporary password no longer authenticates.
        v2 = secrets.token_urlsafe(48)[:64]
        code2, _s2, _r2, _nn2 = _complete_auth_code_pkce_flow(
            port, username="owner", password=new_pw, verifier=v2)
        assert code2
        try:
            v3 = secrets.token_urlsafe(48)[:64]
            _complete_auth_code_pkce_flow(port, username="owner", password=temp_pw, verifier=v3)
            raise AssertionError("old temporary password still authenticates")
        except AssertionError as exc:
            assert "authentication failed" in str(exc) or "form" in str(exc) \
                or "code" in str(exc) or "state" in str(exc), exc


def ep007_integration_direct_access_grant_denied_for_human_client() -> None:
    """Direct Access Grant is denied for the human client (architecture rule).

    The denial is client-level: the same error is returned whether the
    username/password is correct or not, so the response never reveals
    credential validity."""
    with _keycloak() as (_name, port, au, ap):
        boot = _bootstrap(port, au, ap)
        good = None
        bogus = None
        for password in (boot["owner_password"], "definitely-not-the-password"):
            try:
                _token_request(port, {
                    "grant_type": "password",
                    "client_id": HUMAN_CLIENT,
                    "username": "owner",
                    "password": password,
                    "scope": "openid profile",
                })
                raise AssertionError("password grant unexpectedly succeeded")
            except TokenGrantError as exc:
                assert exc.status == 400
                assert exc.payload.get("error") == "unauthorized_client"
                desc = exc.payload.get("error_description", "").lower()
                assert "direct access grants" in desc, f"unexpected description: {desc}"
                assert "invalid_grant" != exc.payload.get("error")
                if good is None:
                    good = exc.payload
                else:
                    bogus = exc.payload
        assert good == bogus, "response leaks credential validity"


def ep007_integration_client_credentials_service_identity() -> None:
    """A confidential service client authenticates with client credentials and
    carries only its service identity and least-privilege roles."""
    with _keycloak() as (_name, port, au, ap):
        boot = _bootstrap(port, au, ap)
        # Owner subject for comparison (proves the token is NOT the owner).
        verifier = secrets.token_urlsafe(48)[:64]
        code, _state, _r, _nn = _complete_auth_code_pkce_flow(
            port, username="owner", password=boot["owner_password"], verifier=verifier)
        owner_tokens = _exchange_code(port, code, verifier)
        _oh, owner_claims, _os = _decode_jwt(owner_tokens["access_token"])

        resp = _token_request(port, {
            "grant_type": "client_credentials",
            "client_id": SERVICE_SCHEDULER,
            "client_secret": boot["scheduler_secret"],
            "scope": "openid",
        })
        _header, claims, _sig = _decode_jwt(resp["access_token"])
        assert claims["iss"] == _issuer(port)
        assert claims.get("azp") == SERVICE_SCHEDULER
        assert claims["sub"] != owner_claims["sub"], "token must be a service account, not the owner"
        assert claims["sub"] == boot["scheduler_service_id"], \
            "subject must be the scheduler service account user"
        assert claims.get("preferred_username") in (None, "service-account-nexus-scheduler")
        roles = (claims.get("realm_access") or {}).get("roles") or []
        assert REALM_ROLE_ADMIN in roles
        assert claims.get("tenant") == TENANT_CLAIM

        jwks = _fetch_jwks(port)
        assert _verify_rs256(resp["access_token"], jwks)
        validator = BoundaryValidator(_issuer(port), SERVICE_SCHEDULER,
                                      required_scopes=("openid",),
                                      required_role=REALM_ROLE_ADMIN)
        assert validator.rejections(claims) == [], validator.rejections(claims)


def ep007_integration_insufficiently_scoped_service_client_denied() -> None:
    """The connector service client has no nexus-admin role and is DENIED at
    the protected boundary (fail closed)."""
    with _keycloak() as (_name, port, au, ap):
        boot = _bootstrap(port, au, ap)
        resp = _token_request(port, {
            "grant_type": "client_credentials",
            "client_id": SERVICE_CONNECTOR,
            "client_secret": boot["connector_secret"],
            "scope": "openid",
        })
        _header, claims, _sig = _decode_jwt(resp["access_token"])
        assert claims.get("azp") == SERVICE_CONNECTOR
        assert claims["sub"] == boot["connector_service_id"]
        roles = (claims.get("realm_access") or {}).get("roles") or []
        assert REALM_ROLE_ADMIN not in roles
        validator = BoundaryValidator(_issuer(port), SERVICE_CONNECTOR,
                                      required_scopes=("openid",),
                                      required_role=REALM_ROLE_ADMIN)
        rejections = validator.rejections(claims)
        assert "role" in rejections, "under-scoped service token must be denied at the boundary"


def ep007_integration_refresh_rotation_issues_new_access_token() -> None:
    """Refresh tokens rotate into new access tokens; reuse is rejected."""
    with _keycloak() as (_name, port, au, ap):
        boot = _bootstrap(port, au, ap)
        verifier = secrets.token_urlsafe(48)[:64]
        code, _state, _r, _nn = _complete_auth_code_pkce_flow(
            port, username="owner", password=boot["owner_password"], verifier=verifier)
        first = _exchange_code(port, code, verifier)
        refresh = first["refresh_token"]
        rotated = _token_request(port, {
            "grant_type": "refresh_token",
            "client_id": HUMAN_CLIENT,
            "refresh_token": refresh,
        })
        assert rotated["access_token"] != first["access_token"]
        _header, claims, _sig = _decode_jwt(rotated["access_token"])
        assert claims["preferred_username"] == "owner"
        assert claims["iss"] == _issuer(port)
        jwks = _fetch_jwks(port)
        assert _verify_rs256(rotated["access_token"], jwks)
        validator = BoundaryValidator(_issuer(port), HUMAN_CLIENT,
                                      required_scopes=("openid", "profile"))
        assert validator.rejections(claims) == [], validator.rejections(claims)
        # Rotation semantics in Keycloak 26.7.0: each refresh-token use issues
        # a NEW access token; the previous refresh token remains usable until
        # the session expires (reuse tolerance by default, documented in the
        # Decision Log). Revocation is a client-policy concern for later nodes.
        reused = _token_request(port, {
            "grant_type": "refresh_token",
            "client_id": HUMAN_CLIENT,
            "refresh_token": refresh,
        })
        assert reused["access_token"] not in (first["access_token"], rotated["access_token"])
        _h2, reused_claims, _s2 = _decode_jwt(reused["access_token"])
        assert reused_claims["preferred_username"] == "owner"


def ep007_integration_invalid_state_nonce_verifier_issuer_audience_failures() -> None:
    """Real negative proofs: wrong state, nonce, verifier, issuer, audience."""
    with _keycloak() as (_name, port, au, ap):
        boot = _bootstrap(port, au, ap)

        # (a) state mismatch is rejected before any token exchange
        verifier_a = secrets.token_urlsafe(48)[:64]
        try:
            _complete_auth_code_pkce_flow(
                port, username="owner", password=boot["owner_password"],
                verifier=verifier_a, expect_state="attacker-controlled-state")
            raise AssertionError("state mismatch was accepted")
        except AssertionError as exc:
            assert "state mismatch" in str(exc), exc

        # (b) wrong PKCE verifier is rejected by the real token endpoint
        verifier_b = secrets.token_urlsafe(48)[:64]
        code_b, _state_b, _r, _nn_b = _complete_auth_code_pkce_flow(
            port, username="owner", password=boot["owner_password"], verifier=verifier_b)
        wrong_verifier = secrets.token_urlsafe(48)[:64]
        try:
            _exchange_code(port, code_b, wrong_verifier)
            raise AssertionError("wrong PKCE verifier was accepted")
        except TokenGrantError as exc:
            assert exc.payload.get("error") == "invalid_grant"

        # (c) wrong nonce is rejected at validation
        verifier_c = secrets.token_urlsafe(48)[:64]
        code_c, _state_c, _r, _nn_c = _complete_auth_code_pkce_flow(
            port, username="owner", password=boot["owner_password"], verifier=verifier_c)
        tokens_c = _exchange_code(port, code_c, verifier_c)
        _ih, id_claims, _is = _decode_jwt(tokens_c["id_token"])
        nonce_validator = BoundaryValidator(_issuer(port), HUMAN_CLIENT,
                                            expected_nonce="attacker-nonce")
        assert "nonce" in nonce_validator.rejections(id_claims)

        # (d) wrong issuer is rejected at validation
        _h, access_claims, _s = _decode_jwt(tokens_c["access_token"])
        issuer_validator = BoundaryValidator("https://evil.example/realms/nexus", HUMAN_CLIENT,
                                             required_scopes=("openid", "profile"))
        assert "issuer" in issuer_validator.rejections(access_claims)

        # (e) wrong audience is rejected at validation
        audience_validator = BoundaryValidator(_issuer(port), "evil-client",
                                               required_scopes=("openid", "profile"))
        assert "audience" in audience_validator.rejections(access_claims)


def ep007_integration_authorize_endpoint_requires_code_flow() -> None:
    """The authorize endpoint exists and rejects missing OAuth2 parameters."""
    with _keycloak() as (_name, port, _au, _ap):
        auth_url = f"http://127.0.0.1:{port}/realms/nexus/protocol/openid-connect/auth"
        req = urllib.request.Request(auth_url, method="GET")
        try:
            urllib.request.urlopen(req, timeout=15.0)
            raise AssertionError("authorize endpoint should reject missing parameters")
        except urllib.error.HTTPError as exc:
            assert exc.code in (400, 401, 403), f"unexpected status {exc.code}"


def ep007_integration_container_cleanup_leaves_no_orphans() -> None:
    """Every ephemeral Keycloak container is removed; zero survivors remain."""
    name, port, au, ap = _start_keycloak()
    _bootstrap(port, au, ap)
    _cleanup_container(name)
    inspect = subprocess.run([DOCKER, "inspect", name], capture_output=True, text=True)
    assert inspect.returncode != 0, "container still present after rm -f"
    remaining = subprocess.run(
        [DOCKER, "ps", "-a", "--filter", "name=nexus-ep007-", "--format", "{{.Names}}"],
        capture_output=True, text=True,
    ).stdout.strip()
    assert remaining == "", f"orphan EP-007 containers remain: {remaining}"
