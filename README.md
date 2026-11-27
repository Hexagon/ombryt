# ombryt

A fast static site generator in a single binary. Markdown and Jinja templates in, a complete site out: feeds, a
sitemap, meta tags, a search index, syntax highlighting and checked links, with nothing else to install.

```sh
ombryt init my-site && cd my-site
ombryt serve
```

**Documentation: [ombryt.56k.guru](https://ombryt.56k.guru)** (built by ombryt, from [docs/](docs/)).

The name is Swedish: *ombrytning* is the step in printing where text is laid out into pages.

## What it does

- **Markdown** (CommonMark with tables, footnotes, task lists, alerts), heading ids and a table of contents.
- **Templates** with [MiniJinja](https://github.com/mitsuhiko/minijinja): layouts that wrap layouts, includes,
  macros, and `search.pages()` to list other pages.
- **A data cascade**: `_data.yaml` per directory, merged with front matter.
- **Clean URLs** from file names, with dates taken from `2026-11-27_title.md`.
- **Syntax highlighting** at build time, 200+ languages, light and dark themes, as CSS classes.
- **Feeds** (RSS, Atom, JSON Feed), **a sitemap**, **Open Graph and Twitter tags**.
- **A JSON search index** for client-side search.
- **Taxonomy pages** (one page per tag), **drafts**, **inlined CSS and JS**.
- **A link check** of every internal link and `#fragment`, on every build.
- **A dev server** that rebuilds and reloads the browser in milliseconds.
- **A docs theme**: a documentation site from a folder of markdown, and a reusable GitHub workflow that publishes it.

A 150-page blog builds in about 0.6 seconds.

## Install

Download an archive from the [releases](https://github.com/Hexagon/ombryt/releases) (Linux, macOS, Windows), or:

```sh
cargo install ombryt --locked
```

In GitHub Actions:

```yaml
- uses: Hexagon/ombryt@v0
- run: ombryt build --strict
```

Release archives are signed with Sigstore and have build provenance attestations;
[Releases](https://ombryt.56k.guru/reference/releases/) explains how to check them.

## Building from source

```sh
cargo build --release
./target/release/ombryt -C docs serve   # the documentation, with live reload
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the checks and how releases are made.

## License

MIT or Apache-2.0, at your option.
