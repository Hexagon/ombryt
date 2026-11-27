//! `ombryt.toml`: the site's configuration. Every field has a default, so an empty file is a valid config.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "ombryt.toml";

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub site: Site,
    pub build: Build,
    pub markdown: Markdown,
    pub highlight: Highlight,
    pub feeds: Vec<Feed>,
    pub sitemap: Sitemap,
    pub metas: Metas,
    pub search: SearchIndex,
    /// Directory the config was loaded from; paths in the config are relative to it.
    #[serde(skip)]
    pub root: PathBuf,
    /// The theme named by `build.theme`, if any.
    #[serde(skip)]
    pub theme: Option<crate::theme::Theme>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Site {
    /// The public URL, e.g. `https://example.com`. Used for absolute links, feeds and the sitemap.
    pub url: String,
    pub title: String,
    pub description: String,
    pub language: String,
    pub author: Option<Author>,
    /// Anything else, available to templates as `site.extra`.
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Build {
    /// Source directory.
    pub src: PathBuf,
    /// Output directory. Emptied before every build.
    pub out: PathBuf,
    /// Template directory, relative to `src`. Never copied to the output.
    pub templates: String,
    /// Glob-free path prefixes (relative to `src`) that are neither pages nor copied.
    pub ignore: Vec<String>,
    /// Pages with `draft: true` are left out unless this is set (or `--drafts` is given).
    pub drafts: bool,
    /// Fail the build on broken internal links instead of warning.
    pub strict_links: bool,
    /// Words per minute for `reading.minutes`.
    pub words_per_minute: usize,
    /// A built-in theme (`docs`) or a theme directory, relative to the project.
    pub theme: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Markdown {
    pub smart_punctuation: bool,
    /// Heading levels that get an entry in `toc`.
    pub toc_levels: Vec<u8>,
    /// Add a `<a class="anchor">` link to headings with an id.
    pub heading_anchors: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Highlight {
    pub enabled: bool,
    /// CSS class prefix for highlighted tokens.
    pub prefix: String,
    /// Where the generated stylesheet goes in the output, or empty for none.
    pub css: String,
    /// Theme for the default (dark) stylesheet. See `ombryt themes`.
    pub theme: String,
    /// Optional second theme, scoped under `light_selector`.
    pub light_theme: Option<String>,
    pub light_selector: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Feed {
    /// Query selecting the pages, same syntax as `search.pages()`.
    pub query: String,
    pub limit: usize,
    pub rss: Option<String>,
    pub atom: Option<String>,
    pub json: Option<String>,
    /// What goes in the item body: `full` (the rendered page body), `intro` (the `intro` field, as markdown) or `none`.
    pub content: String,
    /// Tags left out of the items' categories.
    pub exclude_tags: Vec<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sitemap {
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Metas {
    /// Inject description, Open Graph and Twitter meta tags into every HTML page's `<head>`.
    pub enabled: bool,
    /// Default share image (absolute or site-relative URL).
    pub image: Option<String>,
    pub twitter: Option<String>,
    pub generator: bool,
}

/// A JSON index of the pages for client-side search (MiniSearch, Fuse.js, Lunr, FlexSearch, …).
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchIndex {
    /// Output path, e.g. `/search.json`. Empty (the default) for no index.
    pub index: String,
    /// Which pages, as a query. Pages with `search: false` and taxonomy pages are always left out.
    pub query: String,
    /// Include the body text of markdown pages.
    pub text: bool,
    /// Cut each page's text to this many characters (0: no limit).
    pub max_text: usize,
}

impl Default for SearchIndex {
    fn default() -> Self {
        SearchIndex { index: String::new(), query: String::new(), text: true, max_text: 0 }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            site: Site::default(),
            build: Build::default(),
            markdown: Markdown::default(),
            highlight: Highlight::default(),
            feeds: Vec::new(),
            sitemap: Sitemap::default(),
            metas: Metas::default(),
            search: SearchIndex::default(),
            root: PathBuf::from("."),
            theme: None,
        }
    }
}

impl Default for Site {
    fn default() -> Self {
        Site {
            url: "http://localhost:8000".into(),
            title: String::new(),
            description: String::new(),
            language: "en".into(),
            author: None,
            extra: serde_json::Value::Object(Default::default()),
        }
    }
}

impl Default for Build {
    fn default() -> Self {
        Build {
            src: "src".into(),
            out: "_site".into(),
            templates: "_includes".into(),
            ignore: Vec::new(),
            drafts: false,
            strict_links: false,
            words_per_minute: 275,
            theme: None,
        }
    }
}

impl Default for Markdown {
    fn default() -> Self {
        Markdown { smart_punctuation: false, toc_levels: vec![2, 3], heading_anchors: false }
    }
}

impl Default for Highlight {
    fn default() -> Self {
        Highlight {
            enabled: true,
            prefix: "hl-".into(),
            css: "/css/highlight.css".into(),
            theme: "base16-ocean.dark".into(),
            light_theme: None,
            light_selector: "[data-theme=light]".into(),
        }
    }
}

impl Default for Feed {
    fn default() -> Self {
        Feed {
            query: "post".into(),
            limit: 20,
            rss: Some("/feed.rss".into()),
            atom: None,
            json: Some("/feed.json".into()),
            content: "intro".into(),
            exclude_tags: Vec::new(),
            title: None,
            description: None,
        }
    }
}

impl Default for Sitemap {
    fn default() -> Self {
        Sitemap { enabled: true }
    }
}

impl Default for Metas {
    fn default() -> Self {
        Metas { enabled: true, image: None, twitter: None, generator: true }
    }
}

fn merge_toml(base: &mut toml::Table, over: toml::Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => merge_toml(b, o),
            (_, v) => {
                base.insert(key, v);
            }
        }
    }
}

impl Config {
    /// Loads `ombryt.toml` from `dir`, or the defaults if there is none.
    pub fn load(dir: &Path) -> Result<Config> {
        let path = dir.join(FILE_NAME);
        let mut table: toml::Table = if path.exists() {
            let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?
        } else {
            toml::Table::new()
        };
        // A theme's settings go under the project's.
        let theme_name = table.get("build").and_then(|b| b.get("theme")).and_then(|t| t.as_str()).map(str::to_string);
        let theme = match theme_name {
            Some(name) => Some(crate::theme::Theme::resolve(&name, dir)?),
            None => None,
        };
        if let Some(text) = theme.as_ref().and_then(|t| t.config()) {
            let mut base: toml::Table = toml::from_str(&text).context("parsing the theme's theme.toml")?;
            merge_toml(&mut base, table);
            table = base;
        }
        let mut config: Config =
            toml::Value::Table(table).try_into().with_context(|| format!("reading {}", path.display()))?;
        config.root = dir.to_path_buf();
        config.theme = theme;
        config.site.url = config.site.url.trim_end_matches('/').to_string();
        Ok(config)
    }

    pub fn src_dir(&self) -> PathBuf {
        self.root.join(&self.build.src)
    }

    pub fn out_dir(&self) -> PathBuf {
        self.root.join(&self.build.out)
    }

    pub fn templates_dir(&self) -> PathBuf {
        self.src_dir().join(&self.build.templates)
    }

    /// `/path` → `https://site/path`. Leaves absolute URLs alone.
    pub fn absolute(&self, url: &str) -> String {
        if url.contains("://") { url.to_string() } else { format!("{}/{}", self.site.url, url.trim_start_matches('/')) }
    }
}
