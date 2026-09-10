## `scripts/security-check.sh` and `scripts/dependency-audit.sh`
**Exit Code**: 1

### Observation
Failed because the `chacha20` crate (v0.10.1) has been yanked.

### Output Snippet
```
Crate:     chacha20
Version:   0.10.1
Warning:   yanked
error: 1 denied warning found!
```
Result: FAILED(Crate chacha20 v0.10.1 is yanked)
