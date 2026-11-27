---
title: Documentation sites
description: The built-in docs theme, and a reusable GitHub workflow that publishes a project's docs to GitHub Pages.
nav_order: 7
---

ombryt has a built-in theme for documentation. A project needs a folder of markdown and four lines of
configuration; the theme brings the layout, the navigation, search, highlighting and light and dark modes. These
pages are made with it.

## Start

```sh
ombryt init docs --theme docs
cd docs
ombryt serve
```

That makes:

```text
docs/
├── ombryt.toml          [build] theme = "docs"
└── src/
    ├── _data.yaml       links in the header and the navigation
    ├── index.md
    └── guide/
        ├── index.md
        ├── installation.md
        └── usage.md
```

## Navigation

Every page with a `title` is in the sidebar. The front matter decides where:

```markdown
---
title: Installation
parent: Guide       # the title of the page this one is under; leave out for the top level
nav_order: 1        # order among its siblings (then by title)
---
```

Pages nest up to three levels (a page, its children, their children). The previous and next links at the end of
each page follow the same order. Other fields the theme reads:

| Field | |
| ----- | - |
| `nav_title` | a shorter title for the sidebar and the pager |
| `nav_exclude: true` | leave the page out of the navigation |
| `hide_title: true` | no `<h1>` from the title (when the page has its own) |
| `hide_toc: true` | no "On this page" list |
| `grand_parent` | shown in the breadcrumb, for third-level pages |

`parent: Guide` in `src/guide/_data.yaml` saves writing it in every page of a directory. The directory's own
`index.md` then needs `parent: ""`.

## Links and customizing

`src/_data.yaml` has the links:

```yaml
top_links:            # in the header
  - title: GitHub
    url: https://github.com/you/project
nav_links:            # below the navigation
  - title: Changelog
    url: https://github.com/you/project/blob/main/CHANGELOG.md
```

A file in the project wins over the theme's file of the same name:

| To change | Add |
| --------- | --- |
| the stylesheet | `src/theme/docs.css` (start from [the theme's](https://github.com/Hexagon/ombryt/tree/main/themes/docs/assets/theme/docs.css)) |
| the icon | `src/favicon.svg` |
| tags in `<head>` (analytics, fonts) | `src/_includes/head.html` |
| the footer | `src/_includes/footer.html` (`footer_hide_generator: true` drops the "Built with" line) |
| the page layout | `src/_includes/page.html` or `base.html` |
| highlighting, search | the same settings in `ombryt.toml` |

A theme can also be a directory: `theme = "my-theme"` reads `my-theme/theme.toml`, `_data.yaml`, `_includes/` and
`assets/`, with the same rules.

## Publishing with GitHub Actions

ombryt has a [reusable workflow](https://docs.github.com/en/actions/sharing-automations/reusing-workflows) that
installs ombryt, builds the docs and deploys them to GitHub Pages. In a project, add
`.github/workflows/docs.yml`:

```yaml
name: docs
on:
  push:
    branches: [main]
  workflow_dispatch:
jobs:
  docs:
    uses: Hexagon/ombryt/.github/workflows/docs.yml@v0
    permissions:
      contents: read
      pages: write
      id-token: write
    with:
      path: docs
```

Then pick "GitHub Actions" as the source under Settings → Pages. A custom domain is set there as well, with a
`CNAME` DNS record pointing at `<user>.github.io`.

| Input | Default | |
| ----- | ------- | - |
| `path` | `docs` | the directory with `ombryt.toml` |
| `version` | `latest` | the ombryt version, such as `0.1.0` |
| `strict` | `true` | fail on broken links |
| `deploy` | `true` | `false` builds (and checks) without deploying, for pull requests |
| `url` | the site's | override `site.url`, e.g. for a preview |

With `deploy: false` the workflow is a check that the docs build, which suits pull requests:

```yaml
on: pull_request
jobs:
  docs:
    uses: Hexagon/ombryt/.github/workflows/docs.yml@v0
    with:
      deploy: false
```

## Only installing ombryt

For workflows of your own, the `Hexagon/ombryt` action installs a release, checks it against its checksums and
its build provenance, and puts it on the `PATH`:

```yaml
steps:
  - uses: actions/checkout@v4
  - uses: Hexagon/ombryt@v0
    with:
      version: latest
  - run: ombryt -C docs build --strict
```

It works on Linux, macOS and Windows runners.

## Coming from Lumocs

[Lumocs](https://github.com/Hexagon/lumocs) projects move with few changes: `title`, `nav_order` and `parent` mean
the same, and `top_links` and `nav_links` in `_data.json` work as they are (icons are ignored). Replace
`_config.ts` and `deno.json` with an `ombryt.toml`, and the Deno steps in the workflow with the reusable one.
