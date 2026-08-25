#!/usr/bin/env sh
# EP-042 M3 release transport fetch + verify (SPEC-016, SPEC-024).
#
# REAL fetch: downloads the manifest + component artifacts from the
# configured S3 gateway and verifies every digest against real bytes.
# Exits nonzero on any mismatch or missing object (fail closed).
set -eu
export CI=true
export NO_COLOR=1

cd "$(dirname "$0")/../../.."  # infra/release/scripts -> repo root

release="${1:?usage: release-fetch.sh <release_id> <manifest_out> <components_out> <component_csv>}"
manifest_out="${2:?missing manifest out path}"
components_out="${3:?missing components out dir}"
component_csv="${4:?missing component csv}"

mkdir -p "$(dirname "$manifest_out")"
mkdir -p "$components_out"

node infra/release/src/cli.ts fetch \
  --release "$release" \
  --manifest-out "$manifest_out" \
  --components-out "$components_out" \
  --components "$component_csv"
