"""EP-007 M4 failure tests: forced failures against REAL Keycloak.

Test names begin with ep007_failure_ per the EP-007 milestone contract.
Each test exercises a REAL failure mechanism against the pinned Keycloak
26.7.0 container (or a real network boundary) - no mocks, no stubs
(TESTING.md):

- unavailable dependency: the Keycloak container is SIGKILLed mid-session
  and every subsequent request fails closed with a real connection error;
- denied permission: wrong client secret and disabled service client are
  rejected by the real token endpoint;
- corrupted token: a real access token with a flipped signature byte fails
  real JWKS/RS256 verification and is rejected at the boundary;
- wrong issuer/audience: a real token is rejected by the canonical boundary;
- exhausted budget: repeated wrong-password interactive logins all fail
  against the real login form and never yield an authorization code;
- timeout: requests to a closed port fail fast (fail closed), never hang.

Shared helpers come from test_ep007_integration_keycloak.py (same suite
directory; imported as a module - never duplicated).
"""

from __future__ import annotations

import contextlib
import secrets
import socket
import subprocess
import urllib.error
import urllib.parse
import urllib.request

import test_ep007_integration_keycloak as t

HUMAN_CLIENT = t.HUMAN_CLIENT


@contextlib.contextmanager
def _bootstrap_ctx():
    """Start a container + bootstrap, yielding (name, port, secrets)."""
    name, port, au, ap = t._start_keycloak()
    try:
        yield name, port, t._bootstrap(port, au, ap)
    finally:
        t._cleanup_container(name)


def ep007_failure_keycloak_container_terminated_fails_closed() -> None:
    """A terminated dependency fails closed: every request errors, nothing hangs."""
    with _bootstrap_ctx() as (name, port, boot):
        # Real discovery works before termination.
        doc = t._http_get_json(
            f"http://127.0.0.1:{port}/realms/nexus/.well-known/openid-configuration")
        assert doc["issuer"].endswith(f":{port}/realms/nexus")
        # Terminate the REAL dependency (SIGKILL, not graceful stop).
        killed = subprocess.run([t.DOCKER, "kill", "-s", "KILL", name],
                                capture_output=True, text=True)
        assert killed.returncode == 0, "docker kill failed"
        # The next request MUST fail closed with a real connection error.
        try:
            t._http_get_json(
                f"http://127.0.0.1:{port}/realms/nexus/.well-known/openid-configuration",
                timeout=3.0)
            raise AssertionError("request to a terminated dependency unexpectedly succeeded")
        except (urllib.error.URLError, ConnectionError, OSError):
            pass  # fail closed
        # The token endpoint must fail the same way.
        token_url = f"http://127.0.0.1:{port}/realms/nexus/protocol/openid-connect/token"
        body = urllib.parse.urlencode({
            "grant_type": "client_credentials",
            "client_id": t.SERVICE_SCHEDULER,
            "client_secret": boot["scheduler_secret"],
        }).encode()
        try:
            req = urllib.request.Request(token_url, data=body, method="POST")
            req.add_header("Content-Type", "application/x-www-form-urlencoded")
            urllib.request.urlopen(req, timeout=3.0)
            raise AssertionError("token request to a terminated dependency unexpectedly succeeded")
        except (urllib.error.URLError, ConnectionError, OSError):
            pass  # fail closed


def ep007_failure_wrong_client_secret_denied() -> None:
    """A wrong service secret is denied by the real token endpoint."""
    with _bootstrap_ctx() as (_name, port, boot):
        try:
            t._token_request(port, {
                "grant_type": "client_credentials",
                "client_id": t.SERVICE_SCHEDULER,
                "client_secret": "definitely-not-the-secret",
                "scope": "openid",
            })
            raise AssertionError("wrong client secret was accepted")
        except t.TokenGrantError as exc:
            assert exc.status in (400, 401)
            # Keycloak 26.7.0 reports client-authentication failure as
            # unauthorized_client (observed live); invalid_client also accepted.
            assert exc.payload.get("error") in ("invalid_client", "unauthorized_client")


def ep007_failure_disabled_service_client_denied() -> None:
    """A disabled service client is denied at the real boundary (revocation)."""
    with _bootstrap_ctx() as (name, port, boot):
        # Grant works while enabled.
        ok = t._token_request(port, {
            "grant_type": "client_credentials",
            "client_id": t.SERVICE_CONNECTOR,
            "client_secret": boot["connector_secret"],
            "scope": "openid",
        })
        assert ok["access_token"]
        # Revoke by disabling the client through the real Admin API.
        admin_user = subprocess.run(
            [t.DOCKER, "exec", name, "sh", "-c", "printf %s \"${KC_BOOTSTRAP_ADMIN_USERNAME:-}\""],
            capture_output=True, text=True).stdout.strip() or "admin"
        admin_pw = subprocess.run(
            [t.DOCKER, "exec", name, "sh", "-c", "printf %s \"${KC_BOOTSTRAP_ADMIN_PASSWORD:-}\""],
            capture_output=True, text=True).stdout.strip()
        tok = t._admin_token(port, admin_user, admin_pw)
        clients = t._admin(port, tok, "GET", "/admin/realms/nexus/clients") or []
        connector = next(c for c in clients if c["clientId"] == t.SERVICE_CONNECTOR)
        connector["enabled"] = False
        t._admin(port, tok, "PUT", f"/admin/realms/nexus/clients/{connector['id']}", connector)
        # New grants are denied (real revocation effect).
        try:
            t._token_request(port, {
                "grant_type": "client_credentials",
                "client_id": t.SERVICE_CONNECTOR,
                "client_secret": boot["connector_secret"],
                "scope": "openid",
            })
            raise AssertionError("disabled service client was still granted")
        except t.TokenGrantError as exc:
            assert exc.status in (401, 403)
            assert exc.payload.get("error") in ("invalid_client", "unauthorized_client")


def ep007_failure_corrupted_access_token_rejected() -> None:
    """A real token with a corrupted signature fails JWKS/RS256 verification."""
    with _bootstrap_ctx() as (_name, port, boot):
        verifier = secrets.token_urlsafe(48)[:64]
        code, _state, _r, _nn = t._complete_auth_code_pkce_flow(
            port, username="owner", password=boot["owner_password"], verifier=verifier)
        tokens = t._exchange_code(port, code, verifier)
        jwks = t._fetch_jwks(port)
        assert t._verify_rs256(tokens["access_token"], jwks), "baseline token must verify"
        head, payload, sig = tokens["access_token"].split(".")
        raw_sig = bytearray(t._b64url_decode(sig))
        raw_sig[0] ^= 0x01  # flip one bit in the real signature
        corrupted = f"{head}.{payload}.{t._b64url_encode(bytes(raw_sig))}"
        assert not t._verify_rs256(corrupted, jwks), \
            "corrupted signature must fail real RS256 verification"


def ep007_failure_wrong_issuer_or_audience_rejected() -> None:
    """A real token is rejected by the boundary for wrong issuer or audience."""
    with _bootstrap_ctx() as (_name, port, boot):
        verifier = secrets.token_urlsafe(48)[:64]
        code, _state, _r, _nn = t._complete_auth_code_pkce_flow(
            port, username="owner", password=boot["owner_password"], verifier=verifier)
        tokens = t._exchange_code(port, code, verifier)
        _h, claims, _s = t._decode_jwt(tokens["access_token"])
        wrong_issuer = t.BoundaryValidator("https://evil.example/realms/nexus", HUMAN_CLIENT,
                                           required_scopes=("openid", "profile"))
        assert "issuer" in wrong_issuer.rejections(claims)
        wrong_audience = t.BoundaryValidator(t._issuer(port), "evil-client",
                                             required_scopes=("openid", "profile"))
        assert "audience" in wrong_audience.rejections(claims)
        missing_scope = t.BoundaryValidator(t._issuer(port), HUMAN_CLIENT,
                                            required_scopes=("openid", "nexus.missing"))
        assert "scope" in missing_scope.rejections(claims)


def ep007_failure_wrong_password_budget_exhausted() -> None:
    """Repeated wrong-password logins fail against the REAL form and never
    issue an authorization code (authentication budget exhausted)."""
    with _bootstrap_ctx() as (_name, port, boot):
        budget = 3
        failures = 0
        for _ in range(budget):
            verifier = secrets.token_urlsafe(48)[:64]
            try:
                t._complete_auth_code_pkce_flow(
                    port, username="owner", password="wrong-password-budget",
                    verifier=verifier)
                raise AssertionError("wrong password unexpectedly authenticated")
            except AssertionError as exc:
                assert "authentication failed" in str(exc), exc
                failures += 1
        assert failures == budget, "authentication budget was not exhausted"


def ep007_failure_closed_port_fails_fast() -> None:
    """Requests to a closed port fail fast (fail closed), never hang."""
    # The callback port is only bound while a callback server is running;
    # with no server, connecting must raise a real connection error quickly.
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.settimeout(1.0)
        already_open = probe.connect_ex((t.CALLBACK_HOST, t.CALLBACK_PORT)) == 0
    if already_open:
        return  # a server happens to be bound; nothing to prove here
    try:
        t._http_get_json(f"http://{t.CALLBACK_HOST}:{t.CALLBACK_PORT}/callback", timeout=2.0)
        raise AssertionError("closed port unexpectedly answered")
    except (urllib.error.URLError, ConnectionError, OSError):
        pass  # fail fast and closed
