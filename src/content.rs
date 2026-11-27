//! Reads the source tree: pages (markdown and templates), the data cascade (`_data.*` files, then front matter)
//! and the static files that are copied as they are.

use crate::config::Config;
use crate::util::{self, merge, normalize_tags, slugify, split_date_prefix};
use anyhow::{Context, Result, bail};
use serde_json::{Map, Value as Json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Markdown,
    /// A MiniJinja template that is a page itself (`index.jinja`, `feed.xml.jinja`).
    Template,
}

#[derive(Debug, Clone)]
pub struct Page {
    /// Path relative to the source directory, with `/` separators.
    pub src: String,
    pub kind: Kind,
    /// The source without its front matter.
    pub body: String,
    /// Front matter merged over the data cascade, plus the fields ombryt sets (`url`, `date`, …).
    pub data: Map<String, Json>,
    /// Where the page is written, relative to the output directory.
    pub out: PathBuf,
    /// Whether the output is HTML (and so gets a layout's post-processing, metas and link checks).
    pub html: bool,
}

#[derive(Debug, Default)]
pub struct Source {
    pub pages: Vec<Page>,
    /// Static files: (absolute source path, path relative to the output directory).
    pub assets: Vec<(PathBuf, PathBuf)>,
}

/// Splits `---` YAML or `+++` TOML front matter off a source file.
pub fn front_matter(text: &str, file: &str) -> Result<(Map<String, Json>, String)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for (fence, toml) in [("---", false), ("+++", true)] {
        let Some(rest) = text.strip_prefix(fence) else { continue };
        let Some(rest) = rest.strip_prefix('\n').or_else(|| rest.strip_prefix("\r\n")) else { continue };
        // The closing fence is on a line of its own (or the front matter is empty).
        let (head, body) = if let Some(body) = rest.strip_prefix(fence) {
            ("", body)
        } else {
            let needle = format!("\n{fence}");
            let Some(end) = rest.find(&needle) else { bail!("{file}: front matter is not closed with {fence}") };
            (&rest[..end], &rest[end + needle.len()..])
        };
        // The rest of the fence line, then any blank lines.
        let body = body.trim_start_matches([' ', '\t']).trim_start_matches(['\r', '\n']);
        let data = parse_data(head, if toml { "toml" } else { "yaml" }, file)?;
        return Ok((data, body.to_string()));
    }
    Ok((Map::new(), text.to_string()))
}

pub fn parse_data(text: &str, format: &str, file: &str) -> Result<Map<String, Json>> {
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    let value: Json = match format {
        "json" => serde_json::from_str(text).with_context(|| format!("parsing {file}"))?,
        "toml" => toml::from_str(text).with_context(|| format!("parsing {file}"))?,
        _ => serde_yaml_ng::from_str(text).with_context(|| format!("parsing {file}"))?,
    };
    match value {
        Json::Object(map) => Ok(map),
        Json::Null => Ok(Map::new()),
        _ => bail!("{file}: data must be a mapping"),
    }
}

const DATA_FILES: [(&str, &str); 4] =
    [("_data.json", "json"), ("_data.yaml", "yaml"), ("_data.yml", "yaml"), ("_data.toml", "toml")];

fn dir_data(dir: &Path, rel: &str) -> Result<Map<String, Json>> {
    let mut data = Map::new();
    for (name, format) in DATA_FILES {
        let path = dir.join(name);
        if path.exists() {
            let text = std::fs::read_to_string(&path)?;
            let mut d = parse_data(&text, format, &format!("{rel}{name}"))?;
            normalize_tags(&mut d);
            merge(&mut data, &d);
        }
    }
    Ok(data)
}

fn rel_string(path: &Path) -> String {
    path.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

pub fn load(config: &Config, drafts: bool) -> Result<Source> {
    let src = config.src_dir();
    if !src.is_dir() {
        bail!("source directory {} does not exist", src.display());
    }
    let templates = config.templates_dir();
    let mut source = Source::default();
    let mut cascade: HashMap<PathBuf, Map<String, Json>> = HashMap::new();
    let mut root_data = Map::new();
    if let Some(text) = config.theme.as_ref().and_then(|t| t.data()) {
        root_data = parse_data(&text, "yaml", "the theme's _data.yaml")?;
        normalize_tags(&mut root_data);
    }
    merge(&mut root_data, &dir_data(&src, "")?);
    cascade.insert(src.clone(), root_data);

    let walker = walkdir::WalkDir::new(&src).sort_by_file_name().into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        e.depth() == 0 || !(name.starts_with('.') || name.starts_with('_') || e.path() == templates)
    });
    for entry in walker {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(&src).unwrap();
        let rel_str = rel_string(rel);
        if entry.depth() == 0 || config.build.ignore.iter().any(|i| rel_str.starts_with(i.trim_start_matches('/'))) {
            continue;
        }
        let parent = path.parent().unwrap().to_path_buf();
        if entry.file_type().is_dir() {
            let mut data = cascade.get(&parent).cloned().unwrap_or_default();
            merge(&mut data, &dir_data(path, &format!("{rel_str}/"))?);
            cascade.insert(path.to_path_buf(), data);
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let kind = if name.ends_with(".md") {
            Kind::Markdown
        } else if name.ends_with(".jinja") {
            Kind::Template
        } else {
            source.assets.push((path.to_path_buf(), rel.to_path_buf()));
            continue;
        };
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {rel_str}"))?;
        let (mut front, body) = front_matter(&text, &rel_str)?;
        normalize_tags(&mut front);
        let mut data = cascade.get(&parent).cloned().unwrap_or_default();
        merge(&mut data, &front);
        if !drafts && data.get("draft").and_then(Json::as_bool) == Some(true) {
            continue;
        }
        let page = make_page(config, rel, &rel_str, kind, body, data)?;
        source.pages.push(page);
    }

    let mut seen: HashMap<PathBuf, String> = HashMap::new();
    for page in &source.pages {
        if let Some(other) = seen.insert(page.out.clone(), page.src.clone()) {
            bail!("{} and {} are both written to {}", other, page.src, page.out.display());
        }
    }
    Ok(source)
}

fn make_page(
    config: &Config,
    rel: &Path,
    rel_str: &str,
    kind: Kind,
    body: String,
    mut data: Map<String, Json>,
) -> Result<Page> {
    let file = rel.file_name().unwrap().to_string_lossy();
    let stem = match kind {
        Kind::Markdown => file.trim_end_matches(".md").to_string(),
        Kind::Template => file.trim_end_matches(".jinja").to_string(),
    };
    // `feed.xml.jinja` writes `feed.xml`; `about.jinja` and `about.html.jinja` write `about/index.html`.
    let (stem, ext) = match stem.rsplit_once('.') {
        Some((s, e)) if kind == Kind::Template && e != "html" => (s.to_string(), Some(e.to_string())),
        Some((s, "html")) => (s.to_string(), None),
        _ => (stem, None),
    };
    let (file_date, stem) = split_date_prefix(&stem);

    // Dates: front matter first, then the file name. Stored as RFC 3339 UTC.
    for key in ["date", "updated"] {
        if let Some(value) = data.get(key) {
            let text = util::json_str(value);
            let ts = util::parse_date(&text).with_context(|| format!("{rel_str}: can not read {key} {text:?}"))?;
            data.insert(key.into(), Json::String(util::format_date(ts)));
        }
    }
    if !data.contains_key("date")
        && let Some(d) = file_date
    {
        data.insert("date".into(), Json::String(util::format_date(util::date_from_civil(d))));
    }

    let dirs: Vec<String> = rel
        .parent()
        .map(|p| p.components().map(|c| slugify(&c.as_os_str().to_string_lossy())).collect())
        .unwrap_or_default();
    let url = match data.get("url").and_then(Json::as_str) {
        Some(u) if u.starts_with('/') => u.to_string(),
        Some(u) => bail!("{rel_str}: url {u:?} must start with /"),
        None => {
            let mut parts = dirs.clone();
            let slug = slugify(stem);
            match &ext {
                Some(ext) => parts.push(format!("{stem}.{ext}")),
                None if slug == "index" => {}
                None => parts.push(slug),
            }
            let joined = parts.join("/");
            match ext {
                Some(_) => format!("/{joined}"),
                None if joined.is_empty() => "/".to_string(),
                None => format!("/{joined}/"),
            }
        }
    };
    let out = if url.ends_with('/') {
        PathBuf::from(url.trim_start_matches('/')).join("index.html")
    } else {
        PathBuf::from(url.trim_start_matches('/'))
    };
    let html = out.extension().is_some_and(|e| e == "html" || e == "htm");
    data.insert("url".into(), Json::String(url));
    data.insert("src".into(), Json::String(rel_str.to_string()));
    data.insert("slug".into(), Json::String(slugify(stem)));
    if kind == Kind::Markdown {
        let words = crate::markdown::word_count(&body);
        let minutes = words.div_ceil(config.build.words_per_minute.max(1));
        data.insert("reading".into(), serde_json::json!({"words": words, "minutes": minutes.max(1)}));
        data.insert("source".into(), Json::String(body.trim().to_string()));
    }
    Ok(Page { src: rel_str.to_string(), kind, body, data, out, html })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_front_matter() {
        let (data, body) = front_matter("---\ntitle: Hi\ntags: [a]\n---\n\nBody\n", "x.md").unwrap();
        assert_eq!(data["title"], "Hi");
        assert_eq!(body, "Body\n");
    }

    #[test]
    fn toml_front_matter() {
        let (data, body) = front_matter("+++\ntitle = \"Hi\"\n+++\nBody", "x.md").unwrap();
        assert_eq!(data["title"], "Hi");
        assert_eq!(body, "Body");
    }

    #[test]
    fn no_front_matter() {
        let (data, body) = front_matter("# Just text\n---\n", "x.md").unwrap();
        assert!(data.is_empty());
        assert_eq!(body, "# Just text\n---\n");
    }

    #[test]
    fn unclosed() {
        assert!(front_matter("---\ntitle: x\n", "x.md").is_err());
    }
}
