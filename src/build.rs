//! A whole build: load, render markdown, expand taxonomies, render templates, post-process, write.

use crate::config::Config;
use crate::content::{self, Kind, Page};
use crate::markdown::Renderer;
use crate::output::{self, Processed};
use crate::render::{Ctx, Entry, Search, Templates, describe};
use crate::util::{json_str, slugify};
use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use serde_json::{Map, Value as Json};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub drafts: bool,
    /// Add the live-reload script (for `ombryt serve`).
    pub live_reload: bool,
    /// Override the site URL (`serve` uses the local address).
    pub site_url: Option<String>,
}

#[derive(Debug, Default)]
pub struct Report {
    pub pages: usize,
    pub assets: usize,
    pub warnings: Vec<String>,
    pub elapsed: Duration,
}

/// Pages with `taxonomy:` in their data become one page for all terms and one page per term.
///
/// ```yaml
/// taxonomy:
///   key: tags            # the field to group by (default: tags)
///   query: post          # which pages count (default: all)
///   exclude: [post]      # values that get no page
///   url: /archive/{term}/
/// ```
///
/// Each page gets `terms` (`[{name, count, url}]`); the per-term pages also get `term` and `query`, a query for
/// the term's pages to give `search.pages()`.
fn expand_taxonomies(pages: Vec<Page>, config: &Config) -> Result<Vec<Page>> {
    if !pages.iter().any(|p| p.data.contains_key("taxonomy")) {
        return Ok(pages);
    }
    let search = Search { entries: Arc::new(pages.iter().map(|p| Entry::new(light(&p.data))).collect()) };
    let mut out = Vec::with_capacity(pages.len());
    for page in pages {
        let Some(tax) = page.data.get("taxonomy").and_then(Json::as_object).cloned() else {
            out.push(page);
            continue;
        };
        let key = tax.get("key").map(json_str).unwrap_or_else(|| "tags".into());
        let query = tax.get("query").map(json_str).unwrap_or_default();
        let exclude: Vec<String> =
            tax.get("exclude").and_then(Json::as_array).map(|a| a.iter().map(json_str).collect()).unwrap_or_default();
        let Some(pattern) = tax.get("url").and_then(Json::as_str).map(str::to_string) else {
            bail!("{}: taxonomy needs a url, such as /archive/{{term}}/", page.src);
        };
        if !pattern.contains("{term}") || !pattern.starts_with('/') {
            bail!("{}: taxonomy url {pattern:?} must start with / and contain {{term}}", page.src);
        }
        let terms: Vec<(String, usize, String)> = search
            .values(&key, &query, &exclude)
            .into_iter()
            .map(|(name, count)| {
                let url = pattern.replace("{term}", &slugify(&name));
                (name, count, url)
            })
            .collect();
        let terms_json: Vec<Json> =
            terms.iter().map(|(n, c, u)| serde_json::json!({"name": n, "count": c, "url": u})).collect();
        for (name, _, url) in &terms {
            let mut p = page.clone();
            let q = if key == "tags" { format!("\"{name}\"") } else { format!("{key}=\"{name}\"") };
            let q = if query.is_empty() { q } else { format!("{query} {q}") };
            p.data.insert("term".into(), Json::String(name.clone()));
            p.data.insert("query".into(), Json::String(q));
            p.data.insert("terms".into(), Json::Array(terms_json.clone()));
            p.data.insert("url".into(), Json::String(url.clone()));
            p.out = out_path(url);
            out.push(p);
        }
        let mut index = page;
        index.data.insert("terms".into(), Json::Array(terms_json));
        index.data.insert("query".into(), Json::String(query.clone()));
        out.push(index);
    }
    let _ = config;
    Ok(out)
}

/// Page data without the heavy fields, for searches made before rendering.
fn light(data: &Map<String, Json>) -> Map<String, Json> {
    let mut d = data.clone();
    d.remove("content");
    d.remove("source");
    d
}

fn out_path(url: &str) -> PathBuf {
    if url.ends_with('/') {
        PathBuf::from(url.trim_start_matches('/')).join("index.html")
    } else {
        PathBuf::from(url.trim_start_matches('/'))
    }
}

/// Empties the output directory, refusing anything that looks like it is not one.
fn clean(config: &Config) -> Result<()> {
    let out = config.out_dir();
    let root = config.root.canonicalize().unwrap_or_else(|_| config.root.clone());
    if out.exists() {
        let canon = out.canonicalize()?;
        let src = config.src_dir().canonicalize().unwrap_or_default();
        if canon == root || canon == src || src.starts_with(&canon) || !canon.starts_with(&root) {
            bail!("refusing to empty {}: it is not an output directory inside the project", out.display());
        }
        for entry in std::fs::read_dir(&out)? {
            let path = entry?.path();
            if path.is_dir() { std::fs::remove_dir_all(&path)? } else { std::fs::remove_file(&path)? }
        }
    } else {
        std::fs::create_dir_all(&out)?;
    }
    Ok(())
}

fn render_page(tpl: &Templates, entry: &Entry, page: &Page) -> Result<String> {
    let ctx = Ctx { extra: Vec::new(), page: entry.value.clone() };
    let layout = page.data.get("layout").and_then(Json::as_str).map(str::to_string);
    let result = match page.kind {
        Kind::Markdown => {
            let content = page.data.get("content").and_then(Json::as_str).unwrap_or("").to_string();
            tpl.apply_layouts(layout, content, &ctx)
        }
        Kind::Template => tpl
            .env
            .get_template(&format!("page:{}", page.src))
            .and_then(|t| t.render(Value::from_object(Ctx { extra: Vec::new(), page: entry.value.clone() })))
            .and_then(|content| tpl.apply_layouts(layout, content, &ctx)),
    };
    result.map_err(|e| anyhow::anyhow!("{}: {}", page.src, describe(&e)))
}

use minijinja::Value;

pub fn build(config: &Config, opts: &Options) -> Result<Report> {
    let start = Instant::now();
    let mut config = config.clone();
    if let Some(url) = &opts.site_url {
        config.site.url = url.trim_end_matches('/').to_string();
    }
    let config = &config;
    let source = content::load(config, opts.drafts || config.build.drafts)?;
    let md = Renderer::new(config);

    // 1. Markdown bodies, in parallel. Their HTML and toc become page data, so listings and feeds can use them.
    let mut pages = source.pages;
    pages.par_iter_mut().filter(|p| p.kind == Kind::Markdown).for_each(|p| {
        let r = md.render(&p.body);
        p.data.insert("content".into(), Json::String(r.html));
        p.data.insert("toc".into(), serde_json::to_value(&r.toc).unwrap_or(Json::Null));
    });

    // 2. Taxonomy pages.
    let pages = expand_taxonomies(pages, config)?;

    // 3. Templates, in parallel; every page sees every other through `search`.
    let entries: Vec<Entry> = pages.par_iter().map(|p| Entry::new(p.data.clone())).collect();
    let search = Arc::new(Search { entries: Arc::new(entries) });
    let mut tpl = Templates::new(config, md, search.clone());
    let mut added = HashSet::new();
    for p in pages.iter().filter(|p| p.kind == Kind::Template) {
        if added.insert(p.src.clone()) {
            tpl.env
                .add_template_owned(format!("page:{}", p.src), p.body.clone())
                .map_err(|e| anyhow::anyhow!("{}: {}", p.src, describe(&e)))?;
        }
    }
    let rendered: Vec<String> =
        pages.par_iter().zip(search.entries.par_iter()).map(|(p, e)| render_page(&tpl, e, p)).collect::<Result<_>>()?;

    // 4. Static files, then post-processing (which may inline them), then writing.
    clean(config)?;
    let out = config.out_dir();
    source.assets.par_iter().try_for_each(|(from, to)| -> Result<()> {
        let dest = out.join(to);
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::copy(from, &dest).with_context(|| format!("copying {}", from.display()))?;
        Ok(())
    })?;
    let mut files: HashSet<String> =
        source.assets.iter().map(|(_, to)| to.to_string_lossy().replace('\\', "/")).collect();
    // Theme assets, where the project has no file of its own.
    if let Some(theme) = &config.theme {
        let pages_out: HashSet<String> = pages.iter().map(|p| p.out.to_string_lossy().replace('\\', "/")).collect();
        for (rel, bytes) in theme.assets()? {
            if !files.contains(&rel) && !pages_out.contains(&rel) {
                output::write(&out, &rel, &bytes)?;
                files.insert(rel);
            }
        }
    }

    let hl = &config.highlight;
    if hl.enabled && !hl.css.is_empty() {
        let rel = hl.css.trim_start_matches('/').to_string();
        if files.contains(&rel) {
            bail!("{} is both a source file and the generated highlight stylesheet; set highlight.css", hl.css);
        }
        let css = crate::highlight::css(&hl.prefix, &hl.theme, hl.light_theme.as_deref(), &hl.light_selector)?;
        output::write(&out, &rel, css.as_bytes())?;
        files.insert(rel);
    }

    let processed: Vec<Option<Processed>> = pages
        .par_iter()
        .zip(rendered.par_iter())
        .map(|(p, html)| -> Result<Option<Processed>> {
            if !p.html {
                return Ok(None);
            }
            output::process_html(config, &p.data, html, opts.live_reload)
                .map(Some)
                .with_context(|| format!("post-processing {}", p.src))
        })
        .collect::<Result<_>>()?;
    pages.par_iter().zip(rendered.par_iter()).zip(processed.par_iter()).try_for_each(|((p, html), proc)| {
        let body = proc.as_ref().map(|x| x.html.as_str()).unwrap_or(html);
        output::write(&out, &p.out.to_string_lossy(), body.as_bytes())
    })?;
    for p in &pages {
        files.insert(p.out.to_string_lossy().replace('\\', "/"));
    }

    // 5. Site-wide files.
    for feed in &config.feeds {
        for (path, text) in output::feeds(config, feed, &search) {
            output::write(&out, &path, text.as_bytes())?;
            files.insert(path.trim_start_matches('/').to_string());
        }
    }
    if !config.search.index.is_empty() {
        let urls: HashSet<String> = pages
            .iter()
            .filter(|p| p.html)
            .filter_map(|p| p.data.get("url").and_then(Json::as_str))
            .map(String::from)
            .collect();
        let path = config.search.index.trim_start_matches('/').to_string();
        output::write(&out, &path, output::search_index(config, &search, &urls).as_bytes())?;
        files.insert(path);
    }
    if config.sitemap.enabled && !files.contains("sitemap.xml") {
        let html: Vec<&Map<String, Json>> = pages.iter().filter(|p| p.html).map(|p| &p.data).collect();
        output::write(&out, "sitemap.xml", output::sitemap(config, &html).as_bytes())?;
        files.insert("sitemap.xml".into());
    }

    // 6. Links.
    let mut ids: HashMap<String, HashSet<String>> = HashMap::new();
    let mut links: Vec<(String, Vec<String>)> = Vec::new();
    for (p, proc) in pages.iter().zip(processed) {
        if let Some(proc) = proc {
            let url = p.data.get("url").and_then(Json::as_str).unwrap_or("/").to_string();
            ids.insert(p.out.to_string_lossy().replace('\\', "/"), proc.ids);
            links.push((url, proc.links));
        }
    }
    let warnings = output::check_links(config, &links, &files, &ids);
    if config.build.strict_links && !warnings.is_empty() {
        bail!("{} broken links:\n  {}", warnings.len(), warnings.join("\n  "));
    }

    Ok(Report { pages: pages.len(), assets: source.assets.len(), warnings, elapsed: start.elapsed() })
}

/// The directories and files a build reads, for the watcher.
pub fn watched(config: &Config) -> Vec<PathBuf> {
    let mut paths = vec![config.src_dir()];
    let file = config.root.join(crate::config::FILE_NAME);
    if file.exists() {
        paths.push(file);
    }
    if let Some(dir) = config.theme.as_ref().and_then(|t| t.dir()) {
        paths.push(dir.to_path_buf());
    }
    paths
}
