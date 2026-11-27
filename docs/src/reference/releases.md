---
title: Releases
description: Where releases are, what is in them, and how to check that a download is genuine.
nav_order: 6
---

Releases are on [GitHub](https://github.com/Hexagon/ombryt/releases) and [crates.io](https://crates.io/crates/ombryt).
What changed in each is in [CHANGELOG.md](https://github.com/Hexagon/ombryt/blob/main/CHANGELOG.md), which is also
the text of the release.

## Files

| File | |
| ---- | - |
| `ombryt-x86_64-unknown-linux-musl.tar.gz` | Linux, x86_64, static |
| `ombryt-aarch64-unknown-linux-musl.tar.gz` | Linux, arm64, static |
| `ombryt-aarch64-apple-darwin.tar.gz` | macOS, Apple silicon |
| `ombryt-x86_64-apple-darwin.tar.gz` | macOS, Intel |
| `ombryt-x86_64-pc-windows-msvc.zip` | Windows, x86_64 |
| `SHA256SUMS` | checksums of the archives |
| `SHA256SUMS.sigstore.json` | a Sigstore signature of `SHA256SUMS` |

Each archive has the binary, the licences, the README and the changelog. The names have no version, so
`https://github.com/Hexagon/ombryt/releases/latest/download/<file>` always gives the newest.

## Checking a download

Every release is built by GitHub Actions from its tag, and signed there with a short-lived Sigstore certificate
for that workflow. No signing key exists to be stolen. There are two ways to check.

With the GitHub CLI, against the build provenance GitHub keeps for each archive:

```sh
gh attestation verify ombryt-x86_64-unknown-linux-musl.tar.gz --repo Hexagon/ombryt
```

With [cosign](https://docs.sigstore.dev/cosign/system_config/installation/), against the signed checksums:

```sh
cosign verify-blob SHA256SUMS \
  --bundle SHA256SUMS.sigstore.json \
  --certificate-identity-regexp '^https://github.com/Hexagon/ombryt/\.github/workflows/release\.yml@refs/tags/v' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
sha256sum -c SHA256SUMS --ignore-missing
```

On macOS, use `shasum -a 256 -c SHA256SUMS --ignore-missing`.

## Versions

ombryt follows [semantic versioning](https://semver.org). Before 1.0, a minor version (0.2 → 0.3) may change
templates, configuration or output; the changelog says what to do. Patch versions do not.
