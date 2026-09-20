# Zelyra release workflow

This guide describes the repository's release checks. A manual rehearsal builds
the same Linux and Windows packages as a release but cannot publish a GitHub
release.

## Before release

1. Set the workspace version in `Cargo.toml` and refresh `Cargo.lock`.
2. Finish the bilingual `Unreleased` notes in `CHANGELOG.md`.
3. Run formatting, workspace checks, Clippy, and the complete test suite.
4. Push the release-preparation branch and open a pull request to `main`.

The release tag must exactly match the workspace package version in
`Cargo.toml` (for example, `v0.2.0` for version `0.2.0`). The workflow rejects
a mismatched tag instead of publishing archives whose names and embedded CLI
version disagree.

For changes to release automation, package manifests, or source code, the
release workflow builds the Linux and Windows artifacts on the pull request.
These are validation artifacts only; the publish job does not run for pull
requests.

Release builds use the pinned Rust and Python toolchains and `Cargo.lock`.
Each platform job builds the CLI twice in separate target directories and
requires byte-identical binaries.
The packaging script normalizes archive ordering, ownership, permissions, and
timestamps; its tests require byte-identical archives and SHA-256 sidecars for
repeated packaging. This verifies repeatability in the same runner/toolchain
environment; it is not an independent cross-vendor or cross-toolchain
reproducible-build attestation.

## Manual non-publishing rehearsal

In GitHub, open **Actions → Zelyra Release → Run workflow**, select the branch
to test, and enter the release tag, for example `v0.2.0`. The workflow validates
the tag and builds the platform archives, standalone updater binaries, and
SHA-256 files. It uploads them as workflow artifacts and does not create a tag
or publish a release.

The workflow has read-only repository permission during validation and builds.
Only the tag-triggered publish job receives `contents: write` permission.

## Publish

After review and merge to `main`, verify that all required CI, database, and
release-candidate first-run checks have passed. Then push the matching version
tag, for example:

~~~bash
git switch main
git pull --ff-only
git tag -a v0.2.0 -m "Zelyra 0.2.0"
git push origin v0.2.0
~~~

The tag must use `vMAJOR.MINOR.PATCH`, optionally followed by a prerelease
suffix such as `-alpha.1`. GitHub Actions builds both supported release targets
and publishes the release only after both builds succeed. Do not reuse or move
an existing release tag.

The release workflow currently provides Linux and Windows x86_64 assets. Other
platforms are not implied to be supported by these artifacts.
