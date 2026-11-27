---
title: Configuration
description: Every setting in ombryt.toml, with its default.
nav_order: 2
---

`ombryt.toml` is in the project's root. Every setting has a default, so the file can be empty or missing. Unknown
settings are an error, so that typos do not pass unnoticed.

## [site]

| Setting | Default | |
| ------- | ------- | - |
| `url` | `"http://localhost:8000"` | the public URL, for absolute links, feeds, the sitemap and meta tags |
| `title` | `""` | |
| `description` | `""` | |
| `language` | `"en"` | |
| `author` | none | `{ name, url, email }` |
| `extra` | `{}` | anything else, for templates as `site.extra` |

If `url` has a path (`https://example.com/blog`), the `url` filter puts it in front of site paths.

## [build]

| Setting | Default | |
| ------- | ------- | - |
| `src` | `"src"` | source directory |
| `out` | `"_site"` | output directory; emptied before each build, and must be inside the project |
| `templates` | `"_includes"` | template directory, relative to `src` |
| `ignore` | `[]` | paths relative to `src` that are neither pages nor copied |
| `drafts` | `false` | include `draft: true` pages |
| `strict_links` | `false` | fail on broken internal links |
| `words_per_minute` | `275` | for `reading.minutes` |

## [markdown]

| Setting | Default | |
| ------- | ------- | - |
| `smart_punctuation` | `false` | curly quotes, dashes and ellipses |
| `toc_levels` | `[2, 3]` | heading levels in `toc` |
| `heading_anchors` | `false` | add a `#` link to each heading |

## [highlight]

| Setting | Default | |
| ------- | ------- | - |
| `enabled` | `true` | |
| `theme` | `"base16-ocean.dark"` | see `ombryt themes` |
| `light_theme` | none | a second theme, scoped under `light_selector` |
| `light_selector` | `"[data-theme=light]"` | a selector, or an `@media` query |
| `css` | `"/css/highlight.css"` | where the stylesheet is written; `""` for none |
| `prefix` | `"hl-"` | class prefix |

## [[feeds]]

One entry per set of feeds; none by default.

| Setting | Default | |
| ------- | ------- | - |
| `query` | `"post"` | which pages, newest first |
| `limit` | `20` | |
| `rss` | `"/feed.rss"` | |
| `atom` | none | |
| `json` | `"/feed.json"` | JSON Feed 1.1 |
| `content` | `"intro"` | `full`, `intro` or `none` |
| `exclude_tags` | `[]` | |
| `title`, `description` | the site's | |

## [sitemap]

| Setting | Default |
| ------- | ------- |
| `enabled` | `true` |

## [metas]

| Setting | Default | |
| ------- | ------- | - |
| `enabled` | `true` | |
| `image` | none | default share image |
| `twitter` | none | the site's handle, for `twitter:site` |
| `generator` | `true` | add `<meta name="generator">` |

## [search]

| Setting | Default | |
| ------- | ------- | - |
| `index` | `""` | output path of the index, such as `/search.json`; empty for none |
| `query` | `""` | which pages |
| `text` | `true` | include the body text |
| `max_text` | `0` | characters per page, 0 for all |

## A full example

```toml
[site]
url = "https://hexagon.56k.guru"
title = "Hexagon's Blog"
description = "Code, servers and security."
author = { name = "Hexagon", url = "https://hexagon.56k.guru" }

[highlight]
theme = "OneHalfDark"
light_theme = "OneHalfLight"

[[feeds]]
query = "post"
rss = "/feed.rss"
json = "/feed.json"
exclude_tags = ["post", "standalone"]

[search]
index = "/search.json"
query = "post"
```
