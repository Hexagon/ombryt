---
title: Hello, world
description: The first post, and what ombryt does with markdown.
tags: [meta]
---

This post is in `src/posts/`, so it gets `post.html` as its layout and the tag
`post` from `src/posts/_data.yaml`. Its date comes from the file name.

## Code

```rust
fn main() {
    println!("Hello from ombryt");
}
```

## Tables and lists

| Feature | Where |
| ------- | ----- |
| Feeds   | `[[feeds]]` in ombryt.toml |
| Archive | `src/archive.jinja` |

- [x] Markdown
- [x] Syntax highlighting at build time
- [ ] Your first post: `ombryt new "My first post"`
