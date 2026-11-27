---
title: Getting started
description: Install ombryt, make a site and publish it.
nav_order: 1
---

## Install

Download the archive for your system from the
[latest release](https://github.com/Hexagon/ombryt/releases/latest), unpack it and put `ombryt` somewhere on your
`PATH`. There are builds for Linux (x86_64 and aarch64, static), macOS (Apple silicon and Intel) and Windows
(x86_64). [Releases](/reference/releases/) explains how to check that a download is genuine.

With a Rust toolchain you can also install from crates.io:

```sh
cargo install ombryt --locked
```

## Make a site

```sh
ombryt init my-site
cd my-site
ombryt serve
```

`init` writes a small starter site. `serve` builds it, serves it at `http://127.0.0.1:8000/`, and rebuilds and
reloads the browser when a file changes.

The starter looks like this:

```text
my-site/
├── ombryt.toml              site settings
└── src/
    ├── _data.yaml           data for every page (here: the default layout)
    ├── _includes/           templates; never copied to the output
    │   ├── base.html
    │   └── post.html
    ├── index.jinja          the front page, a template
    ├── archive.jinja        one page per tag
    ├── 404.md
    ├── css/style.css        copied as it is
    └── posts/
        ├── _data.yaml       layout and tag for everything in posts/
        └── 2026-01-01_hello-world.md
```

## Write a post

```sh
ombryt new "What I learned this week"
```

This makes `src/posts/<today>_what-i-learned-this-week.md` with some front matter. The date in the file name is
the post's date, and the URL is `/posts/what-i-learned-this-week/`. Add `--draft` to keep it out of the site until
you remove `draft: true`; `ombryt serve --drafts` shows drafts.

## Publish

```sh
ombryt build --strict
```

The site is written to `_site/`. Copy that directory to any web server or static host. `--strict` makes the build
fail on broken internal links, which is a good default in CI. [Deploying](/guide/deploying/) has examples.
