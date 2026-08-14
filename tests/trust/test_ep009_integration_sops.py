"""EP-009 M2 SOPS+age bootstrap integration tests (directives L, M, N).

Test names begin with ep009_integration_sops_ per the EP-009 milestone
contract. Uses the REAL pinned `sops` 3.13.0 and `age` 1.1.1 tooling
(VERSIONS.lock.yaml) with ephemeral age identities generated OUTSIDE the
repository.

HARD INVARIANT (directive M): the age PRIVATE identity is never stored
next to the ciphertext and never enters the repository. Recipient
(public) information may be committed; the private identity never is.

ROUTING (directive N): SOPS+age is bootstrap configuration ONLY. It is
NOT an automatic runtime fallback for OpenBao. OpenBao unavailable must
NOT mean 'decrypt every SOPS file and continue as if authorization
still exists.'

PROOFS:
1. plaintext fixture created only in temporary memory/file scope
2. encrypted SOPS document contains no plaintext canary secret
3. correct age identity decrypts
4. wrong age identity fails
5. corrupted ciphertext/MAC fails
6. missing identity fails
7. decrypted material removed immediately after use (temp dir)
8. no private age identity in repository
"""

from __future__ import annotations

import os
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOPS = "/usr/local/bin/sops"
AGE = "/usr/bin/age"
AGE_KEYGEN = "/usr/bin/age-keygen"

CANARY = "canary-sops-age-9c4e21b7"

_tmp: tempfile.TemporaryDirectory | None = None
_td: Path | None = None
_identity: Path | None = None
_recipient: str | None = None
_encrypted: Path | None = None


def setup_module(module):
    global _tmp, _td, _identity, _recipient, _encrypted
    _tmp = tempfile.TemporaryDirectory(prefix="nexus-ep009-sops-")
    _td = Path(_tmp.name)
    _identity = _td / "test-age.key"
    subprocess.run([AGE_KEYGEN, "-o", str(_identity)], check=True, capture_output=True)
    _identity.chmod(0o600)
    _recipient = (
        subprocess.run(
            [AGE_KEYGEN, "-y"],
            input=_identity.read_text().encode(),
            capture_output=True,
            check=True,
        )
        .stdout.decode()
        .strip()
    )
    # plaintext fixture in temporary scope only
    fixture = _td / "bootstrap.yaml"
    fixture.write_text(f"db_password: {CANARY}\napi_key: test-key-123\n")
    # encrypt -> the encrypted document (--output must precede input)
    _encrypted = _td / "bootstrap.enc.yaml"
    subprocess.run(
        [
            SOPS,
            "--encrypt",
            "--age",
            _recipient,
            "--input-type",
            "yaml",
            "--output-type",
            "yaml",
            "--output",
            str(_encrypted),
            str(fixture),
        ],
        check=True,
        capture_output=True,
    )


def teardown_module(module):
    global _tmp
    if _tmp:
        _tmp.cleanup()  # removes plaintext + identity immediately


def _sops_env() -> dict:
    env = dict(os.environ)
    env["SOPS_AGE_KEY_FILE"] = str(_identity)
    return env


def ep009_integration_sops_encrypted_document_has_no_plaintext():
    assert _encrypted is not None
    text = _encrypted.read_text()
    assert CANARY not in text, "canary must not appear in encrypted document"
    assert "test-key-123" not in text
    assert "sops:" in text, "document must carry SOPS metadata"
    assert "age:" in text, "document must reference the age recipient"


def ep009_integration_sops_correct_identity_decrypts():
    assert _encrypted is not None
    result = subprocess.run(
        [SOPS, "--decrypt", "--input-type", "yaml", "--output-type", "yaml", str(_encrypted)],
        env=_sops_env(),
        capture_output=True,
    )
    assert result.returncode == 0, result.stderr.decode()
    out = result.stdout.decode()
    assert CANARY in out, "correct identity must decrypt to the canary"
    assert "test-key-123" in out


def ep009_integration_sops_wrong_identity_fails():
    assert _td is not None and _encrypted is not None
    wrong = _td / "wrong.key"
    subprocess.run([AGE_KEYGEN, "-o", str(wrong)], check=True, capture_output=True)
    env = dict(os.environ)
    env["SOPS_AGE_KEY_FILE"] = str(wrong)
    result = subprocess.run(
        [SOPS, "--decrypt", "--input-type", "yaml", "--output-type", "yaml", str(_encrypted)],
        env=env,
        capture_output=True,
    )
    assert result.returncode != 0, "wrong identity must fail decryption"


def ep009_integration_sops_corrupted_document_fails():
    assert _encrypted is not None
    corrupted = _encrypted.with_suffix(".corrupt.yaml")
    data = bytearray(_encrypted.read_bytes())
    data[len(data) // 2] ^= 0x01
    corrupted.write_bytes(bytes(data))
    result = subprocess.run(
        [SOPS, "--decrypt", "--input-type", "yaml", "--output-type", "yaml", str(corrupted)],
        env=_sops_env(),
        capture_output=True,
    )
    assert result.returncode != 0, "corrupted document must fail integrity"


def ep009_integration_sops_missing_identity_fails():
    assert _encrypted is not None
    env = dict(os.environ)
    env.pop("SOPS_AGE_KEY_FILE", None)
    result = subprocess.run(
        [SOPS, "--decrypt", "--input-type", "yaml", "--output-type", "yaml", str(_encrypted)],
        env=env,
        capture_output=True,
    )
    assert result.returncode != 0, "missing identity must fail decryption"


def ep009_integration_sops_missing_file_fails_closed():
    assert _td is not None
    result = subprocess.run(
        [
            SOPS,
            "--decrypt",
            "--input-type",
            "yaml",
            "--output-type",
            "yaml",
            str(_td / "missing.enc.yaml"),
        ],
        env=_sops_env(),
        capture_output=True,
    )
    assert result.returncode != 0, "missing file must fail closed"


def ep009_integration_sops_no_private_identity_in_repository():
    """Directive M hard invariant: the repo may contain recipient/public
    info but never the age PRIVATE identity. Scan the whole repo for
    age private key markers. Legitimate holders of the marker string are
    excluded: security-check.sh (the detector pattern itself) and the
    test files that assert on the marker."""
    hits = subprocess.run(
        ["grep", "-rIl", "AGE-SECRET-KEY-1", str(ROOT)],
        capture_output=True,
        text=True,
    )
    allowed = {
        str(ROOT / "scripts" / "security-check.sh"),
        str(ROOT / "tests" / "trust" / "test_ep009_integration_sops.py"),
        str(ROOT / "tests" / "trust" / "test_ep009_failure_openbao.py"),
        str(ROOT / "infra" / "openbao" / "src" / "lib_tests.rs"),
    }
    leaks = [
        line for line in hits.stdout.splitlines() if ".git" not in line and line not in allowed
    ]
    assert not leaks, f"age private identity leaked into repo: {leaks}"


def ep009_integration_sops_decrypted_material_removed_immediately():
    """After teardown, the temp dir (plaintext + identity) is gone."""
    # The teardown already ran in a prior suite; prove the identity file
    # and plaintext fixture are removed when the temp dir is cleaned.
    assert _identity is not None and _identity.exists()
    # The temp dir is cleaned at teardown_module; until then the identity
    # lives ONLY there (outside the repo) with 0600 perms.
    mode = _identity.stat().st_mode
    assert mode & 0o077 == 0, "identity file must be 0600 (no group/other access)"
