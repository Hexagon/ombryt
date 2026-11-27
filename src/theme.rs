//! Themes: templates, assets and defaults that a site gets without having them in its own source.
//!
//! A theme is either built in (embedded in the binary, such as `docs`) or a directory. Either way it has:
//!
//! - `theme.toml`: default settings, under the project's `ombryt.toml`;
//! - `_data.yaml`: default data for every page, under the project's own `src/_data.*`;
//! - `_includes/`: templates, used when the project has none of the same name;
//! - `assets/`: files copied to the output, unless the project has a file at the same path.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Builtin {
    pub name: &'static str,
    pub description: &'static str,
    config: &'static str,
    data: &'static str,
    templates: &'static [(&'static str, &'static str)],
    assets: &'static [(&'static str, &'static [u8])],
}

macro_rules! theme_file {
    ($theme:literal, $path:literal) => {
        include_str!(concat!("../themes/", $theme, "/", $path))
    };
}
macro_rules! theme_asset {
    ($theme:literal, $path:literal) => {
        ($path, include_bytes!(concat!("../themes/", $theme, "/assets/", $path)) as &[u8])
    };
}

pub const BUILTIN: &[Builtin] = &[Builtin {
    name: "docs",
    description: "Documentation with a nested sidebar (nav_order, parent), search, and light and dark modes",
    config: theme_file!("docs", "theme.toml"),
    data: theme_file!("docs", "_data.yaml"),
    templates: &[
        ("base.html", theme_file!("docs", "_includes/base.html")),
        ("page.html", theme_file!("docs", "_includes/page.html")),
    ],
    assets: &[
        theme_asset!("docs", "theme/docs.css"),
        theme_asset!("docs", "theme/search.js"),
        theme_asset!("docs", "favicon.svg"),
    ],
}];

#[derive(Debug, Clone)]
pub enum Theme {
    Builtin(&'static Builtin),
    Dir(PathBuf),
}

impl Theme {
    /// A built-in theme by name, or else a directory relative to the project root.
    pub fn resolve(name: &str, root: &Path) -> Result<Theme> {
        if let Some(b) = BUILTIN.iter().find(|b| b.name == name) {
            return Ok(Theme::Builtin(b));
        }
        let dir = root.join(name);
        if dir.is_dir() {
            return Ok(Theme::Dir(dir));
        }
        let names: Vec<&str> = BUILTIN.iter().map(|b| b.name).collect();
        bail!("no theme {name:?}: not a built-in one ({}) and no such directory", names.join(", "))
    }

    fn read(&self, rel: &str) -> Option<String> {
        match self {
            Theme::Builtin(_) => None,
            Theme::Dir(dir) => std::fs::read_to_string(dir.join(rel)).ok(),
        }
    }

    pub fn config(&self) -> Option<String> {
        match self {
            Theme::Builtin(b) => Some(b.config.to_string()),
            Theme::Dir(_) => self.read("theme.toml"),
        }
    }

    pub fn data(&self) -> Option<String> {
        match self {
            Theme::Builtin(b) => Some(b.data.to_string()),
            Theme::Dir(_) => self.read("_data.yaml"),
        }
    }

    pub fn template(&self, name: &str) -> Option<String> {
        match self {
            Theme::Builtin(b) => b.templates.iter().find(|(n, _)| *n == name).map(|(_, t)| t.to_string()),
            Theme::Dir(_) if name.contains("..") => None,
            Theme::Dir(_) => self.read(&format!("_includes/{name}")),
        }
    }

    /// (path relative to the output, contents).
    pub fn assets(&self) -> Result<Vec<(String, Vec<u8>)>> {
        match self {
            Theme::Builtin(b) => Ok(b.assets.iter().map(|(p, c)| (p.to_string(), c.to_vec())).collect()),
            Theme::Dir(dir) => {
                let base = dir.join("assets");
                let mut out = Vec::new();
                if base.is_dir() {
                    for entry in walkdir::WalkDir::new(&base).sort_by_file_name() {
                        let entry = entry?;
                        if entry.file_type().is_file() {
                            let rel = entry.path().strip_prefix(&base).unwrap().to_string_lossy().replace('\\', "/");
                            let bytes = std::fs::read(entry.path()).with_context(|| format!("reading {rel}"))?;
                            out.push((rel, bytes));
                        }
                    }
                }
                Ok(out)
            }
        }
    }

    /// The directory to watch for changes, for a theme on disk.
    pub fn dir(&self) -> Option<&Path> {
        match self {
            Theme::Builtin(_) => None,
            Theme::Dir(d) => Some(d),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_docs_theme() {
        let t = Theme::resolve("docs", Path::new(".")).unwrap();
        assert!(t.template("page.html").unwrap().contains("nav_order"));
        assert!(t.template("nope.html").is_none());
        assert!(t.assets().unwrap().iter().any(|(p, _)| p == "theme/search.js"));
        let config: toml::Table = toml::from_str(&t.config().unwrap()).unwrap();
        assert!(config.contains_key("search"));
        assert!(Theme::resolve("no-such-theme", Path::new(".")).is_err());
    }
}
