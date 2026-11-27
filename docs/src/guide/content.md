---
title: Content and data
description: Pages, front matter, the data cascade, URLs, dates and static files.
nav_order: 2
---

Everything ombryt builds comes from the source directory (`src/` by default).

## What becomes what

| File | Becomes |
| ---- | ------- |
| `*.md` | A page: markdown rendered to HTML, then put through its layout |
| `*.jinja` | A page that is a template itself, such as `index.jinja` or `feed.xml.jinja` |
| `_data.yaml`, `_data.json`, `_data.toml` | Data for every page in that directory and below |
| Anything else | Copied to the output as it is |

Files and directories whose names start with `_` or `.` are skipped, as is the template directory (`_includes/`).
`build.ignore` in `ombryt.toml` skips more paths.

## Front matter

A page can start with YAML between `---` lines, or TOML between `+++` lines:

```markdown
---
title: Ownership and borrowing
description: The one big idea that makes Rust different.
tags: [rust, guide]
part: 4
---

The body, in markdown.
```

Every field is available to templates, both by name (`{{ title }}`) and on `page` (`{{ page.title }}`). Some fields
mean something to ombryt; [Page data](/reference/page-data/) lists them.

## The data cascade

Data is merged from the outside in: `src/_data.yaml`, then the `_data.*` of each directory on the way down, then the
page's front matter. Objects merge key by key, `tags` lists are joined, and everything else is replaced by the
innermost value.

```text
src/_data.yaml              layout: base.html
src/posts/_data.yaml        layout: post.html, tags: [post]
src/posts/rust/_data.yaml   tags: [rust], series: { title: The guide to Rust }
src/posts/rust/2026-11-09_introduction.md
                            title: Introduction, part: 1
```

The introduction ends up with `layout: post.html`, `tags: [post, rust]`, the series and its own title and part.

## URLs

A page's URL comes from its path. Directory and file names are made into slugs (lower case, `_` and spaces become
`-`), a leading `YYYY-MM-DD_` or `YYYY-MM-DD-` is taken off, and pages get clean URLs:

| Source | URL | File |
| ------ | --- | ---- |
| `src/about.md` | `/about/` | `about/index.html` |
| `src/posts/2023-09-06_Getting_Started.md` | `/posts/getting-started/` | `posts/getting-started/index.html` |
| `src/posts/ssh/2023-09-21_index.md` | `/posts/ssh/` | `posts/ssh/index.html` |
| `src/index.jinja` | `/` | `index.html` |
| `src/llms.txt.jinja` | `/llms.txt` | `llms.txt` |

`url: /somewhere/` in the front matter sets the URL directly; `url: /404.html` writes that file. Two pages with the
same URL stop the build.

## Dates

`date` and `updated` can be given in the front matter as `2023-09-21`, `2023-09-21 10:00:00`, or RFC 3339
(`2023-09-21T10:00:00+02:00`). Without `date`, a date at the start of the file name is used. Dates are stored as
UTC (`2023-09-21T00:00:00Z`), so they sort as text, and the [`date` filter](/reference/filters/) formats them.

## Drafts

Pages with `draft: true` are left out of the build, the feeds, the sitemap and every `search`. `--drafts` (on
`build` and `serve`) or `build.drafts = true` brings them back.

## Markdown

ombryt uses [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark): CommonMark with tables, footnotes,
strikethrough, task lists, definition lists and GitHub-style alerts. Headings get ids from their text, and
`## Title {#custom-id .class}` sets them. Fenced code blocks with a language are
[highlighted](/guide/features/#syntax-highlighting) when the site is built. HTML in markdown is passed through.
