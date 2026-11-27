---
title: Page data
description: The fields ombryt reads from front matter, and the ones it sets.
nav_order: 3
---

## Fields ombryt reads

| Field | |
| ----- | - |
| `title` | used by feeds, the sitemap's neighbours, meta tags and search |
| `layout` | the template to render the page with |
| `url` | the page's URL instead of the one from its path; must start with `/` |
| `date`, `updated` | see [Dates](/guide/content/#dates) |
| `tags` | a list (or a comma-separated string); queries match on it |
| `draft` | `true` leaves the page out |
| `description`, `intro` | summaries for feeds, meta tags and search |
| `metas` | meta tag values, or `false`; see [Meta tags](/guide/features/#meta-tags) |
| `priority` | written to the sitemap |
| `sitemap` | `false` leaves the page out of the sitemap |
| `search` | `false` leaves the page out of the search index |
| `taxonomy` | makes a template page into [taxonomy pages](/guide/templates/#taxonomy-pages) |

## Fields ombryt sets

| Field | |
| ----- | - |
| `url` | the page's URL, such as `/posts/hello/` |
| `src` | its path in the source directory |
| `slug` | the last part of its URL |
| `date`, `updated` | in RFC 3339, UTC |
| `content` | the HTML of a markdown page |
| `source` | the markdown of a markdown page, without front matter |
| `toc` | `[{level, text, id, children}]` |
| `reading` | `{words, minutes}` |
| `term`, `terms`, `query` | on taxonomy pages |

Everything else is yours: `series`, `part`, `header`, `enable_toc`… are all just data for your templates.
