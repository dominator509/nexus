# apps/

Application roots for the Nexus polyglot workspace (ARCHITECTURE.md layout).

Each application is owned and materialized by its own graph node; this
directory is the shared boundary. No application code exists here until its
owning node runs. The workspace manifests at the repository root reference
these paths so the monorepo builds from committed lockfiles even before an
app's owning node executes.

| Path | Application | Stack | Owning node |
| --- | --- | --- | --- |
| `apps/control-plane` | Public and private control API | Rust (Axum) | control-plane node (graph EP-007+) |
| `apps/edge` | Home and site edge runtime | Rust | edge runtime node |
| `apps/cli` | Operator and recovery CLI | Rust | CLI node |
| `apps/web` | React PWA | TypeScript | EP-033 |
| `apps/setup` | Tauri onboarding and deployment | Rust + TS | EP-035 |
| `apps/mobile` | Flutter iOS and Android | Dart | EP-034 |

Ownership rules:

1. An owning node lists its app path in its CHANGE list before creating code.
2. Until an owning node runs, the path stays absent or holds only this
   directory's documentation - no placeholder, demonstration, or sample code.
3. EP-001 owns the workspace manifests and the CI skeleton, not the apps.
