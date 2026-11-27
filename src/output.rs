//! What happens to the rendered pages before they are written: HTML post-processing (inlined assets, meta tags,
//! live reload), and the files made from all pages together: feeds, the sitemap and the link check.

use crate::config::{Config, Feed};
use crate::render::{Entry, Search, format_date};
use crate::util::{escape_xml, json_str, one_line, strip_tags};
use lol_html::html_content::ContentType;
use lol_html::{RewriteStrSettings, element, end_tag, rewrite_str};
use serde_json::{Map, Value as Json};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

pub const LIVE_PATH: &str = "/_ombryt/live";

#[derive(Debug, Default)]
pub struct Processed {
    pub html: String,
    pub links: Vec<String>,
    pub ids: HashSet<String>,
}

/// Reads a site-relative file for inlining: from the output (already copied assets) or the source directory.
fn read_local(config: &Config, href: &str) -> Option<String> {
    let path = href.split(['?', '#']).next()?.trim_start_matches('/');
    if href.contains("://") || href.starts_with("//") || path.contains("..") {
        return None;
    }
    std::fs::read_to_string(config.out_dir().join(path))
        .or_else(|_| std::fs::read_to_string(config.src_dir().join(path)))
        .ok()
}

/// `"=intro"` in `metas` means "the page's `intro`".
fn resolve<'a>(data: &'a Map<String, Json>, value: Option<&'a Json>) -> Option<String> {
    let v = value?;
    let s = json_str(v);
    if let Some(key) = s.strip_prefix('=') {
        return data.get(key).map(json_str).filter(|s| !s.is_empty());
    }
    (!s.is_empty()).then_some(s)
}

fn meta_tags(config: &Config, data: &Map<String, Json>) -> Vec<(String, String, String)> {
    let metas = data.get("metas").and_then(Json::as_object);
    let get = |key: &str| resolve(data, metas.and_then(|m| m.get(key)));
    let title = get("title").or_else(|| data.get("title").map(json_str)).unwrap_or_else(|| config.site.title.clone());
    let description = get("description")
        .or_else(|| data.get("description").map(json_str))
        .or_else(|| data.get("intro").map(json_str))
        .unwrap_or_else(|| config.site.description.clone());
    let description = strip_tags(&crate::markdown::Renderer::new(config).render_inline(&description));
    let site_name = get("site").unwrap_or_else(|| config.site.title.clone());
    let image = get("image").or_else(|| config.metas.image.clone()).map(|i| config.absolute(&i));
    let url = config.absolute(data.get("url").and_then(Json::as_str).unwrap_or("/"));
    let date = data.get("date").and_then(Json::as_str);
    let lang = get("lang").or_else(|| data.get("lang").map(json_str)).unwrap_or_else(|| config.site.language.clone());

    let mut out: Vec<(String, String, String)> = Vec::new();
    let mut push = |attr: &str, key: &str, value: String| {
        if !value.is_empty() {
            out.push((attr.into(), key.into(), value));
        }
    };
    push("name", "description", description.clone());
    push("property", "og:type", if date.is_some() { "article".into() } else { "website".into() });
    push("property", "og:site_name", site_name);
    push("property", "og:locale", lang);
    push("property", "og:title", title.clone());
    push("property", "og:description", description.clone());
    push("property", "og:url", url);
    push("name", "twitter:card", if image.is_some() { "summary_large_image".into() } else { "summary".into() });
    push("name", "twitter:title", title);
    push("name", "twitter:description", description);
    if let Some(site) = &config.metas.twitter {
        push("name", "twitter:site", site.clone());
    }
    if let Some(image) = image {
        push("property", "og:image", image.clone());
        push("name", "twitter:image", image);
    }
    if let Some(date) = date {
        push("property", "article:published_time", date.to_string());
        let updated = data.get("updated").and_then(Json::as_str).unwrap_or(date);
        push("property", "article:modified_time", updated.to_string());
    }
    if config.metas.generator {
        push("name", "generator", format!("ombryt {}", env!("CARGO_PKG_VERSION")));
    }
    out
}

/// Post-processes one HTML page:
/// - `<link rel="stylesheet" href="/x.css" inline>` and `<script src="/x.js" inline>` become inline elements;
/// - meta tags (description, Open Graph, Twitter) are added to `<head>`, except those the page already has;
/// - with `live`, the live-reload script is added to `<body>`;
/// - links and element ids are collected for the link check.
pub fn process_html(config: &Config, data: &Map<String, Json>, html: &str, live: bool) -> anyhow::Result<Processed> {
    let links = Rc::new(RefCell::new(Vec::<String>::new()));
    let ids = Rc::new(RefCell::new(HashSet::<String>::new()));
    let present = Rc::new(RefCell::new(HashSet::<String>::new()));
    let metas = if config.metas.enabled && data.get("metas") != Some(&Json::Bool(false)) {
        meta_tags(config, data)
    } else {
        vec![]
    };

    let (l1, l2, i1, p1, p2) = (links.clone(), links.clone(), ids.clone(), present.clone(), present.clone());
    let settings = RewriteStrSettings::new()
        .append_element_content_handler(element!("link[rel=stylesheet][inline][href]", |el| {
            if let Some(css) = read_local(config, &el.get_attribute("href").unwrap_or_default()) {
                el.replace(&format!("<style>{css}</style>"), ContentType::Html);
            }
            Ok(())
        }))
        .append_element_content_handler(element!("script[inline][src]", |el| {
            if let Some(js) = read_local(config, &el.get_attribute("src").unwrap_or_default()) {
                el.remove_attribute("src");
                el.remove_attribute("inline");
                el.set_inner_content(&js.replace("</script", "<\\/script"), ContentType::Html);
            }
            Ok(())
        }))
        .append_element_content_handler(element!(
            "a[href], link[href]:not([rel=preconnect]):not([rel=dns-prefetch])",
            move |el| {
                l1.borrow_mut().push(el.get_attribute("href").unwrap_or_default());
                Ok(())
            }
        ))
        .append_element_content_handler(element!("img[src], script[src], source[src], iframe[src]", move |el| {
            l2.borrow_mut().push(el.get_attribute("src").unwrap_or_default());
            Ok(())
        }))
        .append_element_content_handler(element!("[id], a[name]", move |el| {
            let mut ids = i1.borrow_mut();
            for attr in ["id", "name"] {
                if let Some(v) = el.get_attribute(attr) {
                    ids.insert(v);
                }
            }
            Ok(())
        }))
        .append_element_content_handler(element!("meta", move |el| {
            for attr in ["name", "property"] {
                if let Some(v) = el.get_attribute(attr) {
                    p1.borrow_mut().insert(v);
                }
            }
            Ok(())
        }))
        .append_element_content_handler(element!("head", move |el| {
            let metas = metas.clone();
            let present = p2.clone();
            el.on_end_tag(end_tag!(move |end| {
                let present = present.borrow();
                let mut tags = String::new();
                for (attr, key, value) in &metas {
                    if !present.contains(key) {
                        tags += &format!("<meta {attr}=\"{key}\" content=\"{}\">\n", escape_xml(value));
                    }
                }
                end.before(&tags, ContentType::Html);
                Ok(())
            }))?;
            Ok(())
        }))
        .append_element_content_handler(element!("body", move |el| {
            if live {
                el.on_end_tag(end_tag!(|end| {
                    end.before(
                        &format!(
                            "<script>new EventSource(\"{LIVE_PATH}\").onmessage = () => location.reload();</script>"
                        ),
                        ContentType::Html,
                    );
                    Ok(())
                }))?;
            }
            Ok(())
        }));
    let html = rewrite_str(html, settings)?;
    let links = links.borrow().clone();
    let ids = ids.borrow().clone();
    Ok(Processed { html, links, ids })
}

/// Checks the internal links of every page: that the target exists and, for `#fragment`s into a page made by this
/// build, that it has an element with that id. Returns one message per broken link.
pub fn check_links(
    config: &Config,
    pages: &[(String, Vec<String>)],
    files: &HashSet<String>,
    ids: &HashMap<String, HashSet<String>>,
) -> Vec<String> {
    let mut problems = Vec::new();
    for (page_url, links) in pages {
        let page_file = file_for(page_url, files).unwrap_or_default();
        for link in links {
            let Some(target) = internal(config, page_url, link) else { continue };
            let (path, fragment) = match target.split_once('#') {
                Some((p, f)) => (p.to_string(), Some(f.to_string())),
                None => (target, None),
            };
            let path = path.split('?').next().unwrap_or("").to_string();
            let file = if path.is_empty() { Some(page_file.clone()) } else { file_for(&path, files) };
            let Some(file) = file else {
                problems.push(format!("{page_url}: broken link {link}"));
                continue;
            };
            if let (Some(fragment), Some(known)) = (fragment, ids.get(&file)) {
                let fragment = crate::util::percent_decode(&fragment);
                if !fragment.is_empty() && fragment != "top" && !known.contains(&fragment) {
                    problems.push(format!("{page_url}: no #{fragment} in {link}"));
                }
            }
        }
    }
    problems.sort();
    problems.dedup();
    problems
}

/// The site-absolute path a link points to, or `None` for external links.
fn internal(config: &Config, page_url: &str, link: &str) -> Option<String> {
    let link = link.trim();
    if link.is_empty() {
        return None;
    }
    if let Some(rest) = link.strip_prefix(&config.site.url) {
        return Some(if rest.is_empty() { "/".into() } else { rest.to_string() });
    }
    if link.starts_with("//") || link.contains(':') && !link.starts_with('/') && !link.starts_with('#') {
        return None;
    }
    if link.starts_with('/') || link.starts_with('#') {
        return Some(link.to_string());
    }
    // Relative: against the page's directory.
    let base = match page_url.rfind('/') {
        Some(i) => &page_url[..=i],
        None => "/",
    };
    let mut parts: Vec<&str> = base.split('/').filter(|p| !p.is_empty()).collect();
    let (path, rest) = match link.find(['#', '?']) {
        Some(i) => (&link[..i], &link[i..]),
        None => (link, ""),
    };
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    let trailing = if path.ends_with('/') { "/" } else { "" };
    Some(format!("/{}{trailing}{rest}", parts.join("/")))
}

/// The output file a site path is served from, if there is one.
fn file_for(path: &str, files: &HashSet<String>) -> Option<String> {
    let p = crate::util::percent_decode(path.trim_start_matches('/'));
    let candidates = if p.is_empty() || p.ends_with('/') {
        vec![format!("{p}index.html")]
    } else {
        vec![p.clone(), format!("{p}/index.html"), format!("{p}.html")]
    };
    candidates.into_iter().find(|c| files.contains(c))
}

/// Absolute URLs in `href` and `src`, for content that is read outside the site (feeds).
fn absolutize(config: &Config, html: &str) -> String {
    html.replace("href=\"/", &format!("href=\"{}/", config.site.url))
        .replace("src=\"/", &format!("src=\"{}/", config.site.url))
        .replace(&format!("=\"{}//", config.site.url), "=\"//")
}

struct Item {
    title: String,
    url: String,
    date: Option<String>,
    updated: Option<String>,
    summary: Option<String>,
    content: Option<String>,
    tags: Vec<String>,
}

fn items(config: &Config, feed: &Feed, search: &Search) -> Vec<Item> {
    let md = crate::markdown::Renderer::new(config);
    search
        .find(&feed.query, "date=desc", feed.limit)
        .into_iter()
        .map(|e: &Entry| {
            let d = &e.data;
            let text = |k: &str| d.get(k).map(json_str).filter(|s| !s.is_empty());
            let summary = text("description").or_else(|| text("intro")).map(|s| strip_tags(&md.render_inline(&s)));
            let content = match feed.content.as_str() {
                "full" => text("content"),
                "intro" => text("intro").map(|i| md.render_inline(&i)),
                _ => None,
            };
            let tags = d
                .get("tags")
                .and_then(Json::as_array)
                .map(|t| t.iter().map(json_str).filter(|t| !feed.exclude_tags.contains(t)).collect())
                .unwrap_or_default();
            Item {
                title: one_line(&text("title").unwrap_or_default()),
                url: config.absolute(d.get("url").and_then(Json::as_str).unwrap_or("/")),
                date: text("date"),
                updated: text("updated"),
                summary,
                content: content.map(|c| absolutize(config, &c)),
                tags,
            }
        })
        .collect()
}

/// The feeds a `[[feeds]]` entry asks for: (output path, contents).
pub fn feeds(config: &Config, feed: &Feed, search: &Search) -> Vec<(String, String)> {
    let items = items(config, feed, search);
    let title = feed.title.clone().unwrap_or_else(|| config.site.title.clone());
    let description = feed.description.clone().unwrap_or_else(|| config.site.description.clone());
    let site = &config.site.url;
    let newest = items
        .iter()
        .filter_map(|i| i.updated.clone().or_else(|| i.date.clone()))
        .max()
        .unwrap_or_else(|| crate::util::format_date(jiff::Timestamp::UNIX_EPOCH));
    let generator = format!("ombryt {}", env!("CARGO_PKG_VERSION"));
    let mut out = Vec::new();

    if let Some(path) = &feed.rss {
        let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        x += "<rss version=\"2.0\" xmlns:content=\"http://purl.org/rss/1.0/modules/content/\" xmlns:atom=\"http://www.w3.org/2005/Atom\">\n<channel>\n";
        x += &format!("<title>{}</title>\n<link>{}/</link>\n", escape_xml(&title), escape_xml(site));
        x += &format!("<description>{}</description>\n", escape_xml(&description));
        x += &format!("<language>{}</language>\n", escape_xml(&config.site.language));
        x += &format!("<lastBuildDate>{}</lastBuildDate>\n", format_date(&newest, "RFC2822").unwrap_or_default());
        x += &format!(
            "<atom:link href=\"{}\" rel=\"self\" type=\"application/rss+xml\"/>\n<generator>{generator}</generator>\n",
            escape_xml(&config.absolute(path))
        );
        for i in &items {
            x += "<item>\n";
            x += &format!("<title>{}</title>\n<link>{}</link>\n", escape_xml(&i.title), escape_xml(&i.url));
            x += &format!("<guid isPermaLink=\"true\">{}</guid>\n", escape_xml(&i.url));
            if let Some(d) = i.date.as_deref().and_then(|d| format_date(d, "RFC2822")) {
                x += &format!("<pubDate>{d}</pubDate>\n");
            }
            if let Some(s) = &i.summary {
                x += &format!("<description>{}</description>\n", escape_xml(s));
            }
            if let Some(c) = &i.content {
                x += &format!("<content:encoded>{}</content:encoded>\n", escape_xml(c));
            }
            for t in &i.tags {
                x += &format!("<category>{}</category>\n", escape_xml(t));
            }
            x += "</item>\n";
        }
        x += "</channel>\n</rss>\n";
        out.push((path.clone(), x));
    }

    if let Some(path) = &feed.atom {
        let mut x =
            String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
        x += &format!("<title>{}</title>\n<subtitle>{}</subtitle>\n", escape_xml(&title), escape_xml(&description));
        x += &format!("<id>{}/</id>\n<updated>{newest}</updated>\n", escape_xml(site));
        x += &format!("<link href=\"{}/\"/>\n", escape_xml(site));
        x += &format!("<link rel=\"self\" href=\"{}\"/>\n", escape_xml(&config.absolute(path)));
        x += &format!("<generator>{generator}</generator>\n");
        if let Some(a) = &config.site.author {
            x += &format!("<author><name>{}</name></author>\n", escape_xml(&a.name));
        }
        for i in &items {
            let date = i.date.clone().unwrap_or_else(|| newest.clone());
            x += "<entry>\n";
            x += &format!("<title>{}</title>\n<id>{}</id>\n", escape_xml(&i.title), escape_xml(&i.url));
            x += &format!("<link href=\"{}\"/>\n", escape_xml(&i.url));
            x +=
                &format!("<published>{date}</published>\n<updated>{}</updated>\n", i.updated.as_ref().unwrap_or(&date));
            if let Some(s) = &i.summary {
                x += &format!("<summary>{}</summary>\n", escape_xml(s));
            }
            if let Some(c) = &i.content {
                x += &format!("<content type=\"html\">{}</content>\n", escape_xml(c));
            }
            for t in &i.tags {
                x += &format!("<category term=\"{}\"/>\n", escape_xml(t));
            }
            x += "</entry>\n";
        }
        x += "</feed>\n";
        out.push((path.clone(), x));
    }

    if let Some(path) = &feed.json {
        let mut feed_json = serde_json::json!({
            "version": "https://jsonfeed.org/version/1.1",
            "title": title,
            "home_page_url": format!("{site}/"),
            "feed_url": config.absolute(path),
            "description": description,
            "language": config.site.language,
        });
        if let Some(a) = &config.site.author {
            feed_json["authors"] = serde_json::json!([{"name": a.name, "url": a.url}]);
        }
        let list: Vec<Json> = items
            .iter()
            .map(|i| {
                let mut item = serde_json::json!({"id": i.url, "url": i.url, "title": i.title});
                if let Some(s) = &i.summary {
                    item["summary"] = s.clone().into();
                }
                if let Some(c) = &i.content {
                    item["content_html"] = c.clone().into();
                }
                if let Some(d) = &i.date {
                    item["date_published"] = d.clone().into();
                }
                if let Some(d) = &i.updated {
                    item["date_modified"] = d.clone().into();
                }
                if !i.tags.is_empty() {
                    item["tags"] = i.tags.clone().into();
                }
                item
            })
            .collect();
        feed_json["items"] = list.into();
        out.push((path.clone(), serde_json::to_string_pretty(&feed_json).unwrap_or_default() + "\n"));
    }
    out
}

/// The client-side search index: `{"version": 1, "documents": [{id, url, title, description, tags, date, headings,
/// text}]}`. Small sites can load it straight into MiniSearch or Fuse.js in the browser.
pub fn search_index(config: &Config, search: &Search, html_urls: &HashSet<String>) -> String {
    let opts = &config.search;
    let md = crate::markdown::Renderer::new(config);
    let mut docs = Vec::new();
    for e in search.find(&opts.query, "date=desc", 0) {
        let d = &e.data;
        let url = d.get("url").and_then(Json::as_str).unwrap_or("");
        if d.get("search") == Some(&Json::Bool(false)) || d.contains_key("taxonomy") || !html_urls.contains(url) {
            continue;
        }
        let Some(title) = d.get("title").map(json_str).filter(|t| !t.is_empty()) else { continue };
        let description = d
            .get("description")
            .or_else(|| d.get("intro"))
            .map(|v| strip_tags(&md.render_inline(&json_str(v))))
            .unwrap_or_default();
        let mut headings = Vec::new();
        fn walk(items: &[Json], out: &mut Vec<Json>) {
            for item in items {
                if let Some(t) = item.get("text") {
                    out.push(serde_json::json!({"text": t, "id": item.get("id")}));
                }
                if let Some(Json::Array(c)) = item.get("children") {
                    walk(c, out);
                }
            }
        }
        if let Some(Json::Array(toc)) = d.get("toc") {
            walk(toc, &mut headings);
        }
        let mut doc = serde_json::json!({
            "id": docs.len(),
            "url": url,
            "title": one_line(&title),
            "description": description,
        });
        if let Some(Json::Array(tags)) = d.get("tags") {
            doc["tags"] = Json::Array(tags.clone());
        }
        if let Some(date) = d.get("date") {
            doc["date"] = date.clone();
        }
        if !headings.is_empty() {
            doc["headings"] = Json::Array(headings);
        }
        if opts.text
            && let Some(html) = d.get("content").and_then(Json::as_str)
        {
            let mut text = strip_tags(&without_anchors(html));
            if opts.max_text > 0 && text.chars().count() > opts.max_text {
                text = text.chars().take(opts.max_text).collect();
            }
            doc["text"] = Json::String(text);
        }
        docs.push(doc);
    }
    let index = serde_json::json!({
        "version": 1,
        "generator": format!("ombryt {}", env!("CARGO_PKG_VERSION")),
        "documents": docs,
    });
    serde_json::to_string(&index).unwrap_or_default()
}

/// Removes the `#` links that `heading_anchors` adds, so they do not end up in the text.
fn without_anchors(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(i) = rest.find("<a class=\"anchor\"") {
        out.push_str(&rest[..i]);
        match rest[i..].find("</a>") {
            Some(j) => rest = &rest[i + j + 4..],
            None => {
                rest = &rest[i..];
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `sitemap.xml` for every HTML page, with `lastmod` from `updated`/`date` and `priority` if the page has one.
pub fn sitemap(config: &Config, pages: &[&Map<String, Json>]) -> String {
    let mut x = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    let mut sorted: Vec<&&Map<String, Json>> = pages.iter().collect();
    sorted.sort_by_key(|d| d.get("url").and_then(Json::as_str).unwrap_or("").to_string());
    for d in sorted {
        if d.get("sitemap") == Some(&Json::Bool(false)) {
            continue;
        }
        let url = config.absolute(d.get("url").and_then(Json::as_str).unwrap_or("/"));
        x += &format!("<url><loc>{}</loc>", escape_xml(&url));
        if let Some(date) = d.get("updated").or_else(|| d.get("date")).and_then(Json::as_str) {
            x += &format!("<lastmod>{}</lastmod>", &date[..date.len().min(10)]);
        }
        if let Some(p) = d.get("priority") {
            x += &format!("<priority>{}</priority>", json_str(p));
        }
        x += "</url>\n";
    }
    x += "</urlset>\n";
    x
}

pub fn write(out: &Path, rel: &str, contents: &[u8]) -> anyhow::Result<()> {
    let path = out.join(rel.trim_start_matches('/'));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        let mut c = Config::default();
        c.site.url = "https://example.com".into();
        c
    }

    #[test]
    fn internal_links() {
        let c = config();
        assert_eq!(internal(&c, "/a/b/", "../c/").as_deref(), Some("/a/c/"));
        assert_eq!(internal(&c, "/a/b/", "x.png").as_deref(), Some("/a/b/x.png"));
        assert_eq!(internal(&c, "/a/", "https://example.com/z/").as_deref(), Some("/z/"));
        assert_eq!(internal(&c, "/a/", "https://other.org/"), None);
        assert_eq!(internal(&c, "/a/", "mailto:x@y"), None);
        assert_eq!(internal(&c, "/a/", "#top").as_deref(), Some("#top"));
    }

    #[test]
    fn link_check() {
        let c = config();
        let files: HashSet<String> = ["index.html", "a/index.html", "img/x.png"].map(String::from).into();
        let ids: HashMap<String, HashSet<String>> = [("a/index.html".to_string(), ["intro".to_string()].into())].into();
        let pages = vec![(
            "/".to_string(),
            vec!["/a/".into(), "/a/#intro".into(), "/a/#nope".into(), "/img/x.png".into(), "/missing/".into()],
        )];
        let problems = check_links(&c, &pages, &files, &ids);
        assert_eq!(problems, vec!["/: broken link /missing/", "/: no #nope in /a/#nope"]);
    }

    #[test]
    fn inlines_and_adds_metas() {
        let c = config();
        let data: Map<String, Json> = serde_json::from_value(
            serde_json::json!({"url": "/p/", "title": "T", "intro": "Some *intro*", "metas": {"image": "/i.png"}}),
        )
        .unwrap();
        let html = "<html><head><meta name=\"description\" content=\"mine\"></head><body><a href=\"/x\">x</a><h2 id=\"h\">h</h2></body></html>";
        let p = process_html(&c, &data, html, true).unwrap();
        assert!(p.html.contains("og:title"), "{}", p.html);
        assert!(p.html.contains("content=\"https://example.com/i.png\""), "{}", p.html);
        assert_eq!(p.html.matches("name=\"description\"").count(), 1, "{}", p.html);
        assert!(p.html.contains("EventSource"));
        assert_eq!(p.links, vec!["/x"]);
        assert!(p.ids.contains("h"));
    }
}
