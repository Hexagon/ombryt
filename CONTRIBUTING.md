# Contributing

## Where things are

- `src/`: the generator. `build.rs` runs a build from start to end; `content.rs` reads the source tree,
  `markdown.rs` and `highlight.rs` make HTML, `render.rs` has the templates and `search`, `output.rs` post-processes
  pages and writes feeds, the sitemap, the search index and the link check. `serve.rs` is the dev server.
- `themes/`: built-in themes, embedded in the binary (listed in `src/theme.rs`).
- `starter/`, `starter-docs/`: what `ombryt init` writes, also embedded.
- `docs/`: the documentation, on the docs theme, published to https://ombryt.56k.guru by `pages.yml`.
- `tests/`: builds of whole sites through the binary.

## Checks

Run these before a change is done; CI runs them on every pull request:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- -C docs build --strict
```

## Rules

- Anything a user can see (a setting, a template variable, a filter, a command) is documented in `docs/` in the
  same change, and listed under "Unreleased" in `CHANGELOG.md`.
- Output must not depend on the machine: sort what comes from the file system, and keep dates in UTC.
- New dependencies need a reason; the binary should stay small and build without C toolchains.
- Write plainly: short sentences, no marketing.

## Releases

1. Rename "Unreleased" in `CHANGELOG.md` to `## <version> (<date>)` and set `version` in `Cargo.toml`.
2. Commit, tag `v<version>` and push the tag.

`.github/workflows/release.yml` checks both, builds the archives for Linux, macOS and Windows, signs them
(Sigstore, keyless) with build provenance, makes the GitHub release with the changelog section, publishes the crate
to crates.io and moves the `v<major>` tag.

The first crates.io publish needs a `CARGO_REGISTRY_TOKEN` secret (in the `crates-io` environment). After it, set
up trusted publishing for this repository and `release.yml` on crates.io, and delete the secret.
