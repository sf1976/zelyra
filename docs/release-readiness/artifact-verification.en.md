# Verifying Zelyra release artifacts

For a published release, download the archive and its `.sha256` sidecar from
the same GitHub release. Verify the checksum and the GitHub release
attestation before extracting the archive:

```sh
sha256sum --check zelyra-v0.4.0-x86_64-unknown-linux-gnu.tar.gz.sha256
gh attestation verify ./zelyra-v0.4.0-x86_64-unknown-linux-gnu.tar.gz --repo sf1976/zelyra
```

For Windows, use the corresponding `x86_64-pc-windows-msvc.zip` archive and
sidecar. `Get-FileHash -Algorithm SHA256 <archive>` can be compared with the
hex digest in its `.sha256` file; `gh attestation verify` works for either
archive format.

The release workflow attaches a GitHub Artifact Attestation with build
provenance and the SPDX SBOM to each tagged platform archive. The verification
command checks the archive's signed build provenance against the attestation
stored for the repository.
The SBOM is also included as a separate release asset and can be inspected
directly. This workflow configuration still requires a successful rehearsal
on a published candidate before the 0.4.0 gate is complete. A matching
checksum by itself does not establish who built an artifact.
