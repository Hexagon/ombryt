---
title: Templates and layouts
description: MiniJinja templates, layouts that wrap layouts, and finding other pages with search.
nav_order: 3
---

Templates use [MiniJinja](https://github.com/mitsuhiko/minijinja), a Rust implementation of Jinja2. If you know
Jinja, Nunjucks, Twig or Liquid, it will look familiar. They live in `src/_includes/` (`build.templates` changes
that).

## Layouts

A page names its layout with `layout:`, usually in a `_data.yaml` so that a whole directory gets it. The layout
gets the page's HTML as `content`:

```jinja
<!doctype html>
<html lang="{{ site.language }}">
<head><title>{{ title }} · {{ site.title }}</title></head>
<body>
  <main>{{ content }}</main>
</body>
</html>
```

`content` is already HTML, so it is not escaped. Everything else is, in `.html` templates.

A layout can have front matter with a `layout` of its own. Then its output goes through that one too, so a post
layout can add a sidebar and leave the page head to a base layout:

```jinja
---
layout: base.html
---
<article>
  <h1>{{ title }}</h1>
  {{ content }}
</article>
```

`{% extends %}` and blocks work as well, as do `{% include %}`, `{% import %}` and macros. Template names are
paths relative to `_includes/`.

## What a template can use

| Name | What it is |
| ---- | ---------- |
| any page field | `title`, `date`, `tags`, … and everything else in the page's data |
| `page` | the same data as one object, for when a name is shadowed |
| `content` | the page's HTML (in layouts) |
| `toc` | the page's headings: `[{level, text, id, children}]` |
| `reading` | `{words, minutes}` for markdown pages |
| `site` | `[site]` from `ombryt.toml`: `url`, `title`, `description`, `language`, `author`, `extra` |
| `search` | queries over all pages, see below |

The [filters and functions](/reference/filters/) reference lists everything else, such as `date`, `md` and `url`.

## Finding other pages

`search.pages(query, sort, limit)` returns the pages matching a query. Each one has all its data, including `url`,
`title` and, for markdown pages, `content`:

```jinja
{% for post in search.pages("post", "date=desc", 10) %}
  <h2><a href="{{ post.url }}">{{ post.title }}</a></h2>
  {{ post.intro | md }}
{% endfor %}
```

The query `post` means "tagged post". `part=1` means "with `part` equal to 1", and terms combine:
`"guide-to-rust" part!=1`. The [query language](/reference/queries/) has the details.

Neighbours in a list, for "previous" and "next" links:

```jinja
{% set newer = search.previous(url, "post", "date=desc") %}
{% set older = search.next(url, "post", "date=desc") %}
```

And distinct values of a field, with counts, for tag clouds:

```jinja
{% for tag in search.values("tags", "post", ["post"]) %}
  <a href="/archive/{{ tag.name | slugify }}/">{{ tag.name }}</a> ({{ tag.count }})
{% endfor %}
```

## Template pages

A file such as `src/index.jinja` is a page that is a template itself. It has front matter like any page, can use a
layout, and gets the same variables. The name decides the output: `index.jinja` and `about.html.jinja` make HTML
pages with clean URLs, `llms.txt.jinja` makes `/llms.txt`. Text and JSON outputs are not HTML-escaped (JSON is
escaped as JSON).

```jinja
---
url: /llms.txt
---
# {{ site.title }}
{% for p in search.pages("post") %}
- [{{ p.title }}]({{ p.url | url(true) }})
{% endfor %}
```

## Taxonomy pages

A template page with `taxonomy:` becomes one page for all values of a field and one page per value:

```jinja
---
url: /archive/
taxonomy:
  key: tags              # the field to group by (default: tags)
  query: post            # which pages count (default: all)
  exclude: [post]        # values that get no page
  url: /archive/{term}/  # the URL of each value's page
---
<h1>{% if term %}#{{ term }}{% else %}Archive{% endif %}</h1>
{% for t in terms %}<a href="{{ t.url }}">{{ t.name }}</a> {% endfor %}
{% for post in search.pages(query, "date=desc") %}
  <p><a href="{{ post.url }}">{{ post.title }}</a></p>
{% endfor %}
```

Every page gets `terms` (`[{name, count, url}]`) and `query`, which selects the pages to list: those of the term,
or all of them on the index page. Only the per-value pages have `term`.

## Errors

A template error stops the build. The message names the page, the template, and the line, with the source around
it.
