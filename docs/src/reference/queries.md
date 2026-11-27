---
title: Queries
description: The query and sort language of search.pages(), feeds, taxonomies and the search index.
nav_order: 5
---

A query is a list of terms separated by spaces. A page must match all of them. An empty query matches every page.

| Term | Matches pages… |
| ---- | -------------- |
| `rust` | tagged `rust` |
| `"two words"` | tagged `two words` |
| `!draft` | not tagged `draft` |
| `part=1` | whose `part` is 1 (for a list field: that contains 1) |
| `part!=1` | whose `part` is not 1 |
| `author.name=Hexagon` | with that value in a nested field |
| `section="Getting started"` | with a value that has spaces |

Values compare as text, so `part=1` matches the number 1 and the string `"1"`.

## Sorting

A sort is a list of fields, each with `=asc` (the default) or `=desc`. Later fields break ties of earlier ones;
pages that tie on all of them are ordered by their source path. Numbers sort as numbers, everything else as text,
without regard to case. Pages without the field come last.

```jinja
search.pages("post", "date=desc")
search.pages("guide-to-rust", "part")
search.pages("doc", "section=desc order")
```
