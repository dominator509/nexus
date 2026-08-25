#!/usr/bin/env sh
# EP-042 M3 release transport publish (SPEC-016, SPEC-024).
#
# REAL publish: reads a canonical release manifest + component artifact
# files, verifies digest binding against real bytes, and uploads the
# manifest + components to the configured S3 gateway. Exits nonzero on
# any mismatch or transport failure (fail closed).
set -eu
export CI=true
export NO_COLOR=1

cd "$(dirname "$0")/../../.."  # infra/release/scripts -> repo root

release="${1:?usage: release-publish.sh <release_id> <manifest> <components_dir>}"
manifest="${2:?missing manifest path}"
components_dir="${3:?missing components dir}"

node infra/release/src/cli.ts publish \
  --release "$release" \
  --manifest "$manifest" \
  --components "$components_dir"
