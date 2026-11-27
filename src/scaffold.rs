//! `ombryt init` and `ombryt new`: a starter site and new posts.

use anyhow::{Result, bail};
use std::path::Path;

const FILES: &[(&str, &str)] = &[
    ("ombryt.toml", include_str!("../starter/ombryt.toml")),
    ("src/_data.yaml", include_str!("../starter/src/_data.yaml")),
    ("src/_includes/base.html", include_str!("../starter/src/_includes/base.html")),
    ("src/_includes/post.html", include_str!("../starter/src/_includes/post.html")),
    ("src/index.jinja", include_str!("../starter/src/index.jinja")),
    ("src/archive.jinja", include_str!("../starter/src/archive.jinja")),
    ("src/404.md", include_str!("../starter/src/404.md")),
    ("src/posts/_data.yaml", include_str!("../starter/src/posts/_data.yaml")),
    ("src/posts/2026-01-01_hello-world.md", include_str!("../starter/src/posts/2026-01-01_hello-world.md")),
    ("src/css/style.css", include_str!("../starter/src/css/style.css")),
    (".gitignore", "_site\n"),
];

const DOCS_FILES: &[(&str, &str)] = &[
    ("ombryt.toml", include_str!("../starter-docs/ombryt.toml")),
    ("src/_data.yaml", include_str!("../starter-docs/src/_data.yaml")),
    ("src/index.md", include_str!("../starter-docs/src/index.md")),
    ("src/guide/index.md", include_str!("../starter-docs/src/guide/index.md")),
    ("src/guide/installation.md", include_str!("../starter-docs/src/guide/installation.md")),
    ("src/guide/usage.md", include_str!("../starter-docs/src/guide/usage.md")),
    (".gitignore", "_site\n"),
];

/// The GitHub Actions workflow for a docs site, printed by `ombryt init --theme docs`.
pub const DOCS_WORKFLOW: &str = include_str!("../starter-docs/.github-docs.yml");

pub fn init(dir: &Path, force: bool, theme: Option<&str>) -> Result<()> {
    let files = match theme {
        None => FILES,
        Some("docs") => DOCS_FILES,
        Some(other) => bail!("no starter for theme {other:?}; there is one for: docs"),
    };
    if dir.join(crate::config::FILE_NAME).exists() && !force {
        bail!("{} already has an ombryt.toml (use --force to overwrite the starter files)", dir.display());
    }
    for (path, text) in files {
        let target = dir.join(path);
        if target.exists() && !force {
            println!("  kept    {path}");
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, text)?;
        println!("  created {path}");
    }
    println!("\nnext: cd {} && ombryt serve", dir.display());
    if theme == Some("docs") {
        println!("\nTo publish on GitHub Pages, add this as .github/workflows/docs.yml:\n\n{DOCS_WORKFLOW}");
    }
    Ok(())
}

pub fn new_post(config: &crate::config::Config, title: &str, section: &str, draft: bool) -> Result<std::path::PathBuf> {
    let today = jiff::ZonedDateTime::now().date();
    let slug = crate::util::slugify(title);
    if slug.is_empty() {
        bail!("the title needs at least one letter or digit");
    }
    let dir = config.src_dir().join(section);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{today}_{slug}.md"));
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    let title_yaml = serde_json::to_string(title)?;
    let draft = if draft { "draft: true\n" } else { "" };
    std::fs::write(
        &path,
        format!("---\ntitle: {title_yaml}\ndescription: \"\"\ntags: []\n{draft}---\n\nWrite here.\n"),
    )?;
    Ok(path)
}
