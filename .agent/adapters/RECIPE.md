# AGENT ADAPTER RECIPE

1. Find the documented standing-instruction file for the new agent platform.
2. Copy the PRIME BLOCK from AGENTS.md byte-for-byte, then add one line naming the platform. Do not add volatile state, node status, preferences, or platform-specific alternate rules.
3. Add the file to the parity check in COMMANDS.md and MANIFEST.md.
4. Run the parity command and `sh scripts/verify.sh`.
