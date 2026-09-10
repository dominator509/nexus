## `scripts/smoke-test.sh`
**Exit Code**: 1

### Observation
Failed due to missing `NEXUS_BASE_DOMAIN` parameter.

### Output Snippet
```
scripts/smoke/runtime.sh: 9: NEXUS_BASE_DOMAIN: parameter not set
```
Result: NOT_RUNNABLE_ENV(NEXUS_BASE_DOMAIN parameter not set)
