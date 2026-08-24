"""EP-041 pytest fixtures.

Makes the owned Python package importable by inserting the repo python/
root on sys.path (same pattern as tests/connectors/conftest.py). No
network, no mocks, no provider SDKs.
"""

from __future__ import annotations

import sys
from pathlib import Path

PYTHON_ROOT = Path(__file__).resolve().parents[2] / "python"

if str(PYTHON_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))
