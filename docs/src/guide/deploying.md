---
title: Deploying
description: Publish the output directory to GitHub Pages, any static host, or a server of your own.
nav_order: 6
---

`ombryt build` writes a complete site to `_site/`. There is nothing to run on the server.

## GitHub Pages

```yaml
# .github/workflows/pages.yml
name: pages
on:
  push:
    branches: [main]
permissions:
  contents: read
  pages: write
  id-token: write
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - uses: actions/checkout@v4
      - name: Install ombryt
        run: |
          curl -fsSL https://github.com/Hexagon/ombryt/releases/latest/download/ombryt-x86_64-unknown-linux-musl.tar.gz \
            | tar -xz --strip-components=1 -C /usr/local/bin --wildcards '*/ombryt'
      - run: ombryt build --strict
      - uses: actions/upload-pages-artifact@v3
        with:
          path: _site
      - id: deployment
        uses: actions/deploy-pages@v4
```

Pick "GitHub Actions" as the source under Settings → Pages. For a custom domain, enter it there too, and point a
`CNAME` DNS record for it at `<user>.github.io`. (A `CNAME` file in the site is only read by branch-based Pages.)

## Netlify, Cloudflare Pages and others

Set the build command to download ombryt as above and run `ombryt build`, and the publish directory to `_site`.

## A server of your own

```sh
ombryt build --strict
rsync -a --delete _site/ server:/var/www/example.com/
```

Clean URLs need nothing special: `/posts/hello/` is `posts/hello/index.html`, which every web server serves.
Point the server's 404 page at `/404.html` if you have one.
