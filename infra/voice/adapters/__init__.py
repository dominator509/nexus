"""EP-021 M3 provider adapters (project interpreter, stdlib-only).

Each adapter wraps a real engine worker (subprocess into the isolated
voice venv) and maps the canonical JSON back onto the nexus_voice
contract types. The adapters contain no inference code of their own;
every result originates from a real engine run.
"""
