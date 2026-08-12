#!/usr/bin/env python3
import os, sys
required = [os.getenv("R2_ACCESS_KEY_ID") or os.getenv("AWS_ACCESS_KEY_ID"), os.getenv("R2_SECRET_ACCESS_KEY") or os.getenv("AWS_SECRET_ACCESS_KEY"), os.getenv("R2_BUCKET") or os.getenv("AWS_S3_BUCKET")]
if not all(required):
    raise SystemExit(1)
# The owning storage node replaces this presence-only bootstrap probe with a signed read-only HeadBucket probe after endpoint selection is recorded.
print("s3 bootstrap credentials present")
