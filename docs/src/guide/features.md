---
title: Feeds, SEO and highlighting
description: RSS, Atom and JSON Feed, the sitemap, meta tags, inlined assets, syntax highlighting and the link check.
nav_order: 4
---

## Feeds

Each `[[feeds]]` entry in `ombryt.toml` makes up to three feeds of the same pages:

```toml
[[feeds]]
query = "post"          # which pages
limit = 20
rss = "/feed.rss"
atom = "/feed.atom"
json = "/feed.json"     # JSON Feed 1.1
content = "intro"       # item body: "full" (the page), "intro" (its intro field) or "none"
exclude_tags = ["post"] # tags not listed as categories
```

The summary of an item is the page's `description`, or else its `intro`. Links in the body are made absolute.

## Sitemap

`sitemap.xml` lists every HTML page, with `lastmod` from `updated` or `date`, and `priority` if the page has one.
`sitemap: false` leaves a page out; `[sitemap] enabled = false` turns it off. A `sitemap.xml` of your own in
`src/` wins.

## Meta tags

Every HTML page gets a description, Open Graph and Twitter tags in its `<head>`, unless the template has written
them already. They come from the page:

| Tag | From |
| --- | ---- |
| `description`, `og:description` | `metas.description`, `description`, `intro`, or the site description |
| `og:title` | `metas.title` or `title` |
| `og:image`, `twitter:image` | `metas.image` or `[metas] image` in `ombryt.toml` |
| `og:type` | `article` for pages with a date, otherwise `website` |
| `article:published_time` | `date` and `updated` |

A value of `"=field"` in `metas` means "the page's `field`", so `metas: { description: "=intro" }` in a
`_data.yaml` works for a whole directory. `metas: false` turns them off for a page, `[metas] enabled = false` for
the site.

## Inlined assets

`<link rel="stylesheet" href="/css/style.css" inline>` is replaced by a `<style>` with the file's contents, and
`<script src="/js/app.js" inline></script>` by an inline script. The file is read from the output (or the source
directory). It saves a request for small, critical files.

## Syntax highlighting

Fenced code blocks with a language are highlighted when the site is built, with the grammars and themes of
[bat](https://github.com/sharkdp/bat). The HTML has CSS classes, not colours, and ombryt writes the stylesheet:

```toml
[highlight]
theme = "OneHalfDark"               # ombryt themes lists them
light_theme = "OneHalfLight"        # optional
light_selector = "[data-theme=light]" # or "@media (prefers-color-scheme: light)"
css = "/css/highlight.css"
prefix = "hl-"
```

Link the stylesheet from your layout. Common short names work (`sh`, `js`, `ts`, `rs`, `yml`, …); an unknown
language gives a plain, escaped block.

## Link check

Every build checks the links in every HTML page: `href` and `src` values that point into the site, including
absolute links to `site.url`, must lead to a file of the build, and a `#fragment` into a page must name an element
id there. Problems are printed as warnings. `ombryt build --strict` or `build.strict_links = true` makes them
errors.

## Live reload

`ombryt serve` adds a small script to each page that reloads it after a rebuild. Nothing is added by
`ombryt build`. The dev server uses `http://<host>:<port>` as the site URL, so absolute links stay local.
