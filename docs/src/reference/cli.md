---
title: Command line
description: Every ombryt command and option.
nav_order: 1
---

```text
ombryt [-C <dir>] <command>
```

`-C <dir>` runs in another project directory (the one with `ombryt.toml`). Without a config file, the
[defaults](/reference/configuration/) are used.

## build

```sh
ombryt build [--drafts] [--strict] [--url <url>]
```

Builds the site into the output directory, which is emptied first. Prints the number of pages and files, the time
it took, and link warnings.

| Option | |
| ------ | - |
| `--drafts` | include pages with `draft: true` |
| `--strict` | fail on broken internal links |
| `--url <url>` | build for another URL than `site.url`, such as a preview |

The exit code is 0 on success and 1 on any error.

## serve

```sh
ombryt serve [--port 8000] [--host 127.0.0.1] [--drafts] [--open]
```

Builds, serves the output, watches the source directory and `ombryt.toml`, rebuilds on changes and reloads open
pages. A failed rebuild prints the error and keeps the last good build. `--host 0.0.0.0` serves on the local
network; `--open` opens a browser.

## init

```sh
ombryt init [dir] [--force]
```

Writes a starter site into `dir` (default: the current directory). It stops if there is an `ombryt.toml` already;
`--force` overwrites the starter's files and leaves others alone.

## new

```sh
ombryt new "Title" [--section posts] [--draft]
```

Makes `src/<section>/<today>_<slug>.md` with front matter, and prints its path.

## themes

```sh
ombryt themes
```

Lists the syntax highlighting themes, for `[highlight] theme` and `light_theme`.
