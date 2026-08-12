# COMPUTE FABRIC

Every node registers hardware, operating system, trust level, locality, latency, availability, power policy, and supported runtimes. Workload manifests declare minimum CPU, RAM, accelerator, storage, network, privacy, device affinity, maximum latency, and failover class.

The scheduler prefers local deterministic execution, then a trusted nearby node, then the user-owned VPS, then an external provider. Camera streams remain on the home edge. Interactive speech prefers the nearest suitable node. Heavy training may run on a rented GPU. Control-plane state remains durable in the configured canonical services.

Placement changes are proposed, benchmarked, canaried, and reversible. A worker cannot migrate itself to a less trusted node or send a private workload to cloud without policy authorization.
