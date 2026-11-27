# Changelog

What changed in each release, and what users have to do about it. The newest first. The details of every change
are in the git history.

Before a release is tagged, "Unreleased" becomes `## <version> (<date>)`: the release workflow refuses a tag without
its section, and puts the section in the release notes.

## Unreleased

## 0.1.1 (2026-11-27)

- A page field named `site` or `search` (such as `search: false`) no longer hides the template global of that name;
  it is still on `page`. Before, `search: false` broke the docs theme.
- Docs theme: the search results close when a result is picked, and stay closed when you come back with the
  browser's back button.

## 0.1.0 (2026-11-27)

The first release.

- `ombryt build`, `serve` (with live reload), `init`, `new` and `themes`.
- Markdown with pulldown-cmark: tables, footnotes, task lists, alerts, heading ids and attributes, a table of
  contents, and reading time.
- MiniJinja templates, layouts with their own front matter, the `search` object (`pages`, `page`, `next`,
  `previous`, `values`, `tags`) and filters for markdown, dates and URLs.
- The data cascade (`_data.yaml`, `_data.json`, `_data.toml`), clean URLs from file names, dates from file names,
  drafts, and taxonomy pages.
- Syntax highlighting at build time with bat's grammars and themes, and a generated stylesheet with a light and a
  dark theme.
- RSS, Atom and JSON Feed; a sitemap; Open Graph and Twitter meta tags; inlined CSS and JS.
- A JSON search index for client-side search.
- A link check of every internal link and fragment, as warnings or (`--strict`) errors.
- Themes, and a built-in `docs` theme with nested navigation, search, light and dark modes, and a layout for
  phones.
- A GitHub Action (`Hexagon/ombryt@v0`) to install ombryt, and a reusable workflow to publish docs to GitHub Pages.
