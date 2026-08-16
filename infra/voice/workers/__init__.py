"""EP-021 M3 real engine workers (run under the isolated voice venv).

Each worker performs one real inference through the selected open-source
engine and prints a single canonical JSON object on stdout. Workers never
import nexus_voice; the JSON contract is the exchange boundary.
"""
