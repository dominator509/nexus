# ENVIRONMENT

## Tool locks

The authoritative set is VERSIONS.lock.yaml. EP-000 records exact binary hashes and installation method. Minimum host for development is Linux, macOS, or Windows with Git, Docker, Rust, Node, pnpm, Python, uv, jq, curl, OpenSSL, age, SOPS, OpenTofu, and platform-specific Flutter and Tauri prerequisites.

| Tool | Locked version | Purpose |
| --- | --- | --- |
| Rust | 1.97.1 | Control plane, edge, connectors, Tauri |
| Python | 3.14.6 | Voice and Microbrain services |
| uv | 0.12.0 | Python dependency and environment lock |
| Node | 24.18.0 LTS | Temporal workers and web tools |
| pnpm | 11.17.0 | TypeScript workspace |
| Flutter | 3.44.7 | iOS and Android |
| Tauri | 2.11.2 | Setup and desktop |
| Docker Compose | 5.1.4 | Local and reference deployment |
| OpenTofu | 1.12.1 | Cloud provisioning |

## Environment variables

PREFLIGHT.md and `.env.example` are the exhaustive variable inventory. Runtime services do not read arbitrary undeclared variables. Configuration is parsed into typed structures at startup; missing required values produce one table of errors and exit with code 78.

## Local setup

1. Materialize the blueprint.
2. Install exact tool versions from VERSIONS.lock.yaml using the methods EP-000 verifies.
3. Copy `.env.example` to `.env`, create age material, and fill required values.
4. Run `sh scripts/preflight.sh`.
5. Launch the graph prompt.

## Environment parity

Development, test, staging, and production use the same container images and schemas. Differences are configuration, scale, credentials, domains, release profile, and provider certification. No environment-specific application behavior or demo mode is allowed.

## Development profiles

- `blueprint`: validates the pack before source exists.
- `core`: PostgreSQL, NATS, Temporal, Keycloak, OpenFGA, OPA, OpenBao, Caddy, Nexus Core.
- `home`: core plus Home Assistant, voice, Frigate, go2rtc.
- `communications`: core plus Asterisk, mail, and fax.
- `sentinel`: core plus lab firewall and DNS telemetry.
- `full-lab`: all real test dependencies and hardware-lab bindings.

## Troubleshooting

Use only COMMANDS.md. Read the failing script output, relevant runbook, ledger tail, and owning ExecPlan. Do not use floating package installers, edit generated contracts, or bypass preflight.
