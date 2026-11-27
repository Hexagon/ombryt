---
title: Search
description: A JSON index of the site for client-side search, and how to use it with MiniSearch, Fuse.js or Pagefind.
nav_order: 5
---

A static site has no server to search on, so the search runs in the browser. ombryt writes an index of the pages
for that:

```toml
[search]
index = "/search.json"
query = "post"      # which pages (default: all with a title)
text = true         # include the body text of markdown pages
max_text = 0        # cut each page's text to this many characters (0: no limit)
```

Pages with `search: false` and taxonomy pages are left out.

## The index

```json
{
  "version": 1,
  "generator": "ombryt 0.1.0",
  "documents": [
    {
      "id": 0,
      "url": "/guide/search/",
      "title": "Search",
      "description": "A JSON index of the site…",
      "tags": ["doc"],
      "date": "2026-11-27T00:00:00Z",
      "headings": [{ "text": "The index", "id": "the-index" }],
      "text": "A static site has no server to search on…"
    }
  ]
}
```

`headings` lets a result link straight to the right section (`url#id`).

## Using it

For a few hundred pages, loading the index on the first keystroke and searching it in plain JavaScript is fast
enough. These docs do that: [`docs/src/js/search.js`](https://github.com/Hexagon/ombryt/blob/main/docs/src/js/search.js)
is about 90 lines with no dependencies. Try it with <kbd>/</kbd>.

For fuzzy matching, prefixes and better ranking, give the documents to a library:

```js
import MiniSearch from "https://cdn.jsdelivr.net/npm/minisearch@7/+esm";

const { documents } = await (await fetch("/search.json")).json();
const index = new MiniSearch({
  fields: ["title", "description", "text"],
  storeFields: ["title", "url", "description"],
  searchOptions: { boost: { title: 3, description: 2 }, prefix: true, fuzzy: 0.2 },
});
index.addAll(documents);
const results = index.search("layout");
```

[Fuse.js](https://www.fusejs.io/), [Lunr](https://lunrjs.com/) and [FlexSearch](https://github.com/nextapps-de/flexsearch)
take the same array.

## Large sites

The index holds all of the text, so it grows with the site: about 475 KB (130 KB compressed) for a blog with 66
long posts. Past a few megabytes, set `max_text`, leave `text` off, or use [Pagefind](https://pagefind.app), which
splits its index into small pieces that are loaded as needed. It works on any static site, so it works on
ombryt's output:

```sh
ombryt build
npx pagefind --site _site
```
