# EP-036 OpenTofu modules (SPEC-016)

OpenTofu modules provision supported providers reproducibly through
provider adapters (Contabo, Hetzner, DigitalOcean, AWS, generic SSH).
M2 owns the module root and the AWS provider module. The exact provider
module set is completed by later milestones (M3-M5).

Layout:

- `modules/` - reusable OpenTofu modules, one per provider class.
- `plans/` - composition plans for tested deployment paths.

Provider credentials remain in the local setup process or short-lived
OAuth and are discarded after provisioning unless infrastructure
management is enabled (SPEC-016 requirement 3). No credential value is
ever stored in module state or plan files.

Fully local and existing SSH remain first-class paths (node contract);
cloud modules are optional breadth, never a hidden dependency.
