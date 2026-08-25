#!/usr/bin/env sh
# EP-042 M3 release transport readiness probe (SPEC-016, SPEC-024).
#
# REAL probe: healthz on the S3 gateway AND a probe PUT -> GET -> digest
# verify -> DELETE through the transport client. Never "endpoint
# configured -> healthy". Exits nonzero when the probe is not verified.
set -eu
export CI=true
export NO_COLOR=1

cd "$(dirname "$0")/../../.."  # infra/release/scripts -> repo root
node infra/release/src/cli.ts probe
