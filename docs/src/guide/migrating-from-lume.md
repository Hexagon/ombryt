---
title: Coming from Lume
description: What changes when a Lume (or Eleventy) site moves to ombryt.
nav_order: 8
---

ombryt took a lot from [Lume](https://lume.land) and [Eleventy](https://www.11ty.dev): the data cascade, layouts
in front matter, `search.pages()`, date prefixes in file names. Most sites move with a handful of mechanical
template changes. This was tested on a 150-page Lume blog: both made the same 179 files (ombryt adds its highlight stylesheet).

## Configuration

`_config.ts` becomes `ombryt.toml`. The plugins most sites use are built in:

| Lume plugin | ombryt |
| ----------- | ------ |
| `code_highlight` | `[highlight]`, at build time, with CSS classes |
| `slugify_urls` | always on |
| `feed` | `[[feeds]]` |
| `metas` | always on, `[metas]` |
| `sitemap` | always on, `[sitemap]` |
| `date` | the `date` filter |
| `reading_info` | `reading.words`, `reading.minutes` |
| `toc` (markdown plugins) | `toc` |
| `inline` | `<link … inline>`, `<script … inline>` |
| `pagefind` | `[search]`, or Pagefind on the output |
| `nunjucks` | MiniJinja |
| `site.copy()` | not needed: files that are not pages are copied |

## Templates

Nunjucks and MiniJinja are both Jinja dialects. What changes:

| Lume / Nunjucks | ombryt |
| --------------- | ------ |
| `page.data.title` | `page.title` (or just `title`) |
| `readingInfo.minutes` | `reading.minutes` |
| `toc.length`, `list.length` | `toc \| length` |
| `item.data.url` in a toc | `item.id` (and `item.text`) |
| `date("yyyy-MM-dd")` (date-fns) | `date("%Y-%m-%d")` (strftime), or `date("DATE")`, `date("ISO")` |
| `search.nextPage(url, q, s)` | `search.next(url, q, s)` (the old name works) |
| `url \| url(true)` | the same |
| `\| dump \| safe` | the same (`dump` is `tojson`) |
| `index.njk` (a page) | `index.jinja` |
| `*.page.js` generators | a template page, or `taxonomy:` for one page per tag |

Layouts keep their names (`post.njk` is fine) and their front matter `layout:` chain.

## Things that differ

- Heading ids are plain slugs: `## What changed: rootcard` gets `what-changed-rootcard`, where Lume made
  `what-changed%3A-rootcard`. Links to old fragments elsewhere will miss the section.
- No anchor link is wrapped around heading text; `markdown.heading_anchors = true` adds a `#` link after it.
- Highlighted code has `hl-` classes and a generated stylesheet, instead of highlight.js classes.
- Dates are formatted with strftime patterns instead of date-fns ones.
- There are no JavaScript plugins. Things that needed code (an `llms.txt`, a tag archive) are template pages.
