---
title: Filters and functions
description: The template filters and functions ombryt adds to MiniJinja's.
nav_order: 4
---

All of [MiniJinja's built-in filters](https://docs.rs/minijinja/latest/minijinja/filters/index.html) are there
(`length`, `join`, `default`, `replace`, `urlencode`, `tojson`, `sort`, …). ombryt adds these.

## Filters

| Filter | |
| ------ | - |
| `md` | markdown to HTML: `{{ intro \| md }}` |
| `md_inline` | the same, without the surrounding `<p>` |
| `date(format)` | formats a date, see below |
| `url(absolute=false)` | a site path with the site's base path; `url(true)` adds the origin |
| `slugify` | `"Hello, World"` → `hello-world` |
| `plaintext` | the text of HTML, tags removed |
| `one_line` | whitespace collapsed to single spaces |
| `truncate_words(n, end="…")` | the first `n` words |
| `xml_escape` | for hand-written XML |
| `dump` | the same as `tojson`, for templates from Nunjucks |

## Date formats

`date` takes a name or a [strftime pattern](https://docs.rs/jiff/latest/jiff/fmt/strtime/index.html#conversion-specifications):

| Format | Gives |
| ------ | ----- |
| `DATE` (the default) | `2023-09-21` |
| `HUMAN_DATE` | `September 21, 2023` |
| `DATETIME` | `2023-09-21 10:00` |
| `ISO`, `RFC3339` | `2023-09-21T10:00:00Z` |
| `RFC2822` | `Thu, 21 Sep 2023 10:00:00 +0000` |
| `YEAR` | `2023` |
| `"%d %b %Y"` | `21 Sep 2023` |

Dates are in UTC.

## Functions and globals

| Name | |
| ---- | - |
| `now()` | the time of the build, in RFC 3339 |
| `site` | `[site]` from `ombryt.toml`, plus `site.generator` |
| `search.pages(query, sort, limit)` | pages matching `query`, sorted, at most `limit` (0: all) |
| `search.page(query, sort)` | the first of them |
| `search.next(url, query, sort)` | the page after `url` in that list |
| `search.previous(url, query, sort)` | the page before it |
| `search.values(key, query, exclude)` | distinct values of `key`, with counts: `[{name, count}]` |
| `search.tags(query, exclude)` | the same for `tags` |

`sort` defaults to `date=desc`. See [Queries](/reference/queries/).
