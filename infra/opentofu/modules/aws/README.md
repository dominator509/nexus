# EP-036 AWS OpenTofu module (SPEC-016)

Provisions a Nexus compute node on AWS through the official AWS
provider. M2 owns the deterministic module contract: `tofu validate`
and `tofu plan` must be reproducible. No real cloud account is touched
by M2; provider certification is owned by later milestones.

The module requires an opaque credential reference; the actual secret
lives in the local setup process / short-lived OAuth (SPEC-016
requirement 3) and is never written to disk by this module.

## Inputs

- `region` - AWS region slug (validated by nexus-provider-aws shape
  rules, e.g. `us-east-1`).
- `instance_type` - EC2 instance type.
- `node_name` - compute node name (fabric registry identity).
- `ami_id` - base AMI for the Nexus bootstrap image.
- `ssh_key_name` - existing EC2 key pair name (bootstrap identity).

## Outputs

- `instance_id` - the created resource identity (exact-target readback
  anchor for the compute fabric).
- `public_ip` - reachability address for later verification.
