# RELEASE

- [ ] Open RELEASE.md and determine the release type.
- [ ] Run `sh scripts/release-build.sh`.
- [ ] Verify image and package signatures and SBOMs.
- [ ] Run staging deploy and smoke only when staging credentials exist.
- [ ] Do not run production deployment; print the exact manual command.
