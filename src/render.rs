//! Templates: a MiniJinja environment with ombryt's filters, the `search` object and layout chaining.
//!
//! Templates live in `_includes/`. A template may start with front matter naming its own `layout`, so layouts can
//! wrap each other (as in Lume or Eleventy); `{% extends %}` works too.

use crate::config::Config;
use crate::markdown::Renderer;
use crate::query::{Query, Sort};
use crate::util;
use minijinja::value::{Enumerator, Kwargs, Object, Serde, from_args};
use minijinja::{AutoEscape, Environment, Error, ErrorKind, State, UndefinedBehavior, Value};
use serde_json::{Map, Value as Json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A page as templates and queries see it.
#[derive(Debug)]
pub struct Entry {
    pub data: Map<String, Json>,
    pub value: Value,
}

impl Entry {
    pub fn new(data: Map<String, Json>) -> Entry {
        let value = Value::from(Serde(&data));
        Entry { data, value }
    }
}

/// `search` in templates: queries over every page's data (and, for markdown pages, `content`).
#[derive(Debug)]
pub struct Search {
    pub entries: Arc<Vec<Entry>>,
}

impl Search {
    pub fn find(&self, query: &str, sort: &str, limit: usize) -> Vec<&Entry> {
        let q = Query::parse(query);
        let mut found: Vec<&Entry> = self.entries.iter().filter(|e| q.matches(&e.data)).collect();
        let sort = Sort::parse(if sort.is_empty() { "date=desc" } else { sort });
        found.sort_by(|a, b| sort.compare(&a.data, &b.data));
        if limit > 0 {
            found.truncate(limit);
        }
        found
    }

    /// Distinct values of `key` (a list field like `tags`, or a plain one) over the pages matching `query`, in the
    /// order they first appear (newest pages first), with how many pages have each.
    pub fn values(&self, key: &str, query: &str, exclude: &[String]) -> Vec<(String, usize)> {
        let mut out: Vec<(String, usize)> = Vec::new();
        let path: Vec<String> = key.split('.').map(str::to_string).collect();
        for e in self.find(query, "date=desc", 0) {
            let vals: Vec<String> = match crate::query::lookup(&e.data, &path) {
                Some(Json::Array(items)) => items.iter().map(util::json_str).collect(),
                Some(v) => vec![util::json_str(v)],
                None => vec![],
            };
            for v in vals {
                if v.is_empty() || exclude.contains(&v) {
                    continue;
                }
                match out.iter_mut().find(|(name, _)| *name == v) {
                    Some((_, n)) => *n += 1,
                    None => out.push((v, 1)),
                }
            }
        }
        out
    }

    fn neighbour(&self, url: &str, query: &str, sort: &str, step: isize) -> Value {
        let list = self.find(query, sort, 0);
        let Some(i) = list.iter().position(|e| e.data.get("url").and_then(Json::as_str) == Some(url)) else {
            return Value::UNDEFINED;
        };
        let j = i as isize + step;
        if j < 0 {
            return Value::UNDEFINED;
        }
        list.get(j as usize).map(|e| e.value.clone()).unwrap_or(Value::UNDEFINED)
    }
}

impl Object for Search {
    fn call_method(self: &Arc<Self>, _: &mut State, method: &str, args: &[Value]) -> Result<Value, Error> {
        let arg = |v: Option<Value>| v.filter(|v| !v.is_undefined() && !v.is_none()).map(|v| v.to_string());
        match method {
            // search.pages(query="", sort="date=desc", limit=0)
            "pages" => {
                let (q, s, l): (Option<Value>, Option<Value>, Option<usize>) = from_args(args)?;
                let found = self.find(&arg(q).unwrap_or_default(), &arg(s).unwrap_or_default(), l.unwrap_or(0));
                Ok(Value::from_iter(found.into_iter().map(|e| e.value.clone())))
            }
            // search.page(query, sort="date=desc"): the first match.
            "page" => {
                let (q, s): (Option<Value>, Option<Value>) = from_args(args)?;
                let found = self.find(&arg(q).unwrap_or_default(), &arg(s).unwrap_or_default(), 1);
                Ok(found.first().map(|e| e.value.clone()).unwrap_or(Value::UNDEFINED))
            }
            // search.next(url, query, sort) / search.previous(url, query, sort): neighbours in that order.
            // Lume's names (nextPage, previousPage) are accepted too.
            "next" | "nextPage" | "previous" | "previousPage" => {
                let (u, q, s): (String, Option<Value>, Option<Value>) = from_args(args)?;
                let step = if method.starts_with("next") { 1 } else { -1 };
                Ok(self.neighbour(&u, &arg(q).unwrap_or_default(), &arg(s).unwrap_or_default(), step))
            }
            // search.values(key="tags", query="", exclude=[]): [{name, count}]
            "values" | "tags" => {
                let (a, b, exclude): (Option<Value>, Option<Value>, Option<Vec<String>>) = from_args(args)?;
                let (key, query) = if method == "tags" {
                    ("tags".to_string(), arg(a).unwrap_or_default())
                } else {
                    (arg(a).unwrap_or_else(|| "tags".into()), arg(b.clone()).unwrap_or_default())
                };
                let exclude = if method == "tags" {
                    b.and_then(|b| b.try_iter().ok().map(|i| i.map(|v| v.to_string()).collect())).unwrap_or_default()
                } else {
                    exclude.unwrap_or_default()
                };
                let vals = self.values(&key, &query, &exclude);
                Ok(Value::from_iter(vals.into_iter().map(|(name, count)| {
                    Value::from_iter([("name", Value::from(name)), ("count", Value::from(count))])
                })))
            }
            _ => Err(Error::new(ErrorKind::UnknownMethod, format!("search has no method {method}"))),
        }
    }
}

/// A page's template context: a few variables ombryt sets, then the page's own data.
#[derive(Debug)]
pub struct Ctx {
    pub extra: Vec<(&'static str, Value)>,
    pub page: Value,
}

impl Object for Ctx {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let key = key.as_str()?;
        if let Some((_, v)) = self.extra.iter().find(|(k, _)| *k == key) {
            return Some(v.clone());
        }
        match key {
            "page" => return Some(self.page.clone()),
            // Globals win over page fields of the same name (`search: false` keeps a page out of the index).
            "site" | "search" => return None,
            _ => {}
        }
        self.page.get_attr(key).ok().filter(|v| !v.is_undefined())
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::NonEnumerable
    }
}

pub struct Templates {
    pub env: Environment<'static>,
    /// Front matter of templates, read when they are loaded, for layout chaining.
    layouts: Arc<Mutex<HashMap<String, Option<String>>>>,
}

fn auto_escape(name: &str) -> AutoEscape {
    let name = name.trim_end_matches(".jinja");
    match name.rsplit_once('.').map(|(_, e)| e) {
        Some("html" | "htm" | "njk" | "xml" | "svg") | None => AutoEscape::Html,
        Some("json") => AutoEscape::Json,
        _ => AutoEscape::None,
    }
}

fn error(msg: impl Into<std::borrow::Cow<'static, str>>) -> Error {
    Error::new(ErrorKind::InvalidOperation, msg)
}

/// Date formats: `DATE` (2023-09-21), `HUMAN_DATE` (September 21, 2023), `DATETIME`, `ISO`/`RFC3339`, `RFC2822`,
/// `YEAR`, or any strftime pattern (`%d %b %Y`).
pub fn format_date(value: &str, format: &str) -> Option<String> {
    let ts = util::parse_date(value)?;
    let pattern = match format {
        "DATE" | "" => "%Y-%m-%d",
        "HUMAN_DATE" => "%B %-d, %Y",
        "DATETIME" => "%Y-%m-%d %H:%M",
        "ISO" | "RFC3339" => "%Y-%m-%dT%H:%M:%SZ",
        "RFC2822" => "%a, %d %b %Y %H:%M:%S +0000",
        "YEAR" => "%Y",
        other => other,
    };
    let zoned = ts.to_zoned_datetime(jiff::tz::TimeZone::UTC);
    jiff::fmt::strtime::format(pattern, &zoned).ok()
}

impl Templates {
    pub fn new(config: &Config, markdown: Renderer, search: Arc<Search>) -> Templates {
        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Chainable);
        env.set_auto_escape_callback(auto_escape);
        env.set_debug(true);
        // Escape only what HTML needs (MiniJinja's default also escapes `/`), and print `none` as nothing.
        env.set_formatter(|out, state, value| {
            if value.is_none() || value.is_undefined() {
                return Ok(());
            }
            if matches!(state.auto_escape(), AutoEscape::Html) && !value.is_safe() {
                out.write_str(&util::escape_xml(&value.to_string())).map_err(Error::from)?;
                return Ok(());
            }
            minijinja::escape_formatter(out, state, value)
        });

        let dir: PathBuf = config.templates_dir();
        let theme = config.theme.clone();
        let layouts: Arc<Mutex<HashMap<String, Option<String>>>> = Default::default();
        let seen = layouts.clone();
        env.set_loader(move |name| {
            if name.contains("..") {
                return Ok(None);
            }
            let path = dir.join(name);
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(_) => match theme.as_ref().and_then(|t| t.template(name)) {
                    Some(text) => text,
                    None => return Ok(None),
                },
            };
            let (data, body) = crate::content::front_matter(&text, name)
                .map_err(|e| Error::new(ErrorKind::SyntaxError, format!("{e:#}")))?;
            let layout = data.get("layout").and_then(Json::as_str).map(str::to_string);
            seen.lock().unwrap().insert(name.to_string(), layout);
            Ok(Some(body))
        });

        let md = markdown.clone();
        env.add_filter("md", move |v: Value| -> Value {
            if v.is_undefined() || v.is_none() {
                return Value::from_safe_string(String::new());
            }
            Value::from_safe_string(md.render_inline(&v.to_string()))
        });
        let md = markdown;
        env.add_filter("md_inline", move |v: Value| -> Value {
            if v.is_undefined() || v.is_none() {
                return Value::from_safe_string(String::new());
            }
            let html = md.render_inline(&v.to_string());
            let html = html.trim();
            let html = html.strip_prefix("<p>").and_then(|h| h.strip_suffix("</p>")).unwrap_or(html);
            Value::from_safe_string(html.to_string())
        });
        env.add_filter("date", |v: Value, format: Option<String>| -> Result<String, Error> {
            if v.is_undefined() || v.is_none() {
                return Ok(String::new());
            }
            let text = v.to_string();
            let f = format.unwrap_or_default();
            format_date(&text, &f).ok_or_else(|| error(format!("date: can not format {text:?} as {f:?}")))
        });
        let site_url = config.site.url.clone();
        let base = site_url.split_once("://").and_then(|(_, r)| r.find('/').map(|i| r[i..].to_string()));
        let base = base.unwrap_or_default().trim_end_matches('/').to_string();
        env.add_filter("url", move |v: Value, absolute: Option<bool>| -> String {
            let url = v.to_string();
            if url.contains("://") || url.starts_with("//") {
                return url;
            }
            let path = if url.starts_with('/') { format!("{base}{url}") } else { url };
            if absolute.unwrap_or(false) {
                let origin = match site_url.split_once("://") {
                    Some((scheme, rest)) => format!("{scheme}://{}", rest.split('/').next().unwrap_or("")),
                    None => site_url.clone(),
                };
                format!("{origin}{path}")
            } else {
                path
            }
        });
        env.add_filter("dump", |v: Value| -> Result<Value, Error> {
            let json = serde_json::to_string(&v).map_err(|e| error(e.to_string()))?;
            let json = json.replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026");
            Ok(Value::from_safe_string(json))
        });
        env.add_filter("slugify", |v: String| util::slugify(&v));
        env.add_filter("plaintext", |v: Value| util::strip_tags(&v.to_string()));
        env.add_filter("xml_escape", |v: Value| util::escape_xml(&v.to_string()));
        env.add_filter("one_line", |v: Value| util::one_line(&v.to_string()));
        env.add_filter("truncate_words", |v: Value, n: usize, kwargs: Kwargs| -> Result<String, Error> {
            let end: Option<String> = kwargs.get("end")?;
            kwargs.assert_all_used()?;
            let text = v.to_string();
            let words: Vec<&str> = text.split_whitespace().collect();
            if words.len() <= n {
                return Ok(words.join(" "));
            }
            Ok(format!("{}{}", words[..n].join(" "), end.unwrap_or_else(|| "…".into())))
        });
        env.add_function("now", || util::format_date(jiff::Timestamp::now()));

        let site = serde_json::json!({
            "url": config.site.url,
            "title": config.site.title,
            "description": config.site.description,
            "language": config.site.language,
            "author": config.site.author,
            "extra": config.site.extra,
            "generator": format!("ombryt {}", env!("CARGO_PKG_VERSION")),
        });
        env.add_global("site", Value::from(Serde(site)));
        env.add_global("search", Value::from_dyn_object(search));
        Templates { env, layouts }
    }

    pub fn layout_of(&self, name: &str) -> Option<String> {
        self.layouts.lock().unwrap().get(name).cloned().flatten()
    }

    /// Renders `content` (already HTML or text) through `layout` and the layouts it names in turn.
    pub fn apply_layouts(&self, mut layout: Option<String>, mut content: String, ctx: &Ctx) -> Result<String, Error> {
        let mut depth = 0;
        while let Some(name) = layout {
            depth += 1;
            if depth > 16 {
                return Err(error(format!("layouts nest more than 16 deep (at {name}); is there a loop?")));
            }
            let tpl = self.env.get_template(&name)?;
            let mut extra = ctx.extra.clone();
            extra.retain(|(k, _)| *k != "content");
            extra.push(("content", Value::from_safe_string(content)));
            content = tpl.render(Value::from_object(Ctx { extra, page: ctx.page.clone() }))?;
            layout = self.layout_of(&name);
        }
        Ok(content)
    }
}

/// An error with the template's location and source excerpt, as MiniJinja's debug info gives it.
pub fn describe(err: &Error) -> String {
    let mut out = format!("{err:#}");
    let mut source = std::error::Error::source(err);
    while let Some(e) = source {
        out += &format!("\ncaused by: {e:#}");
        source = e.source();
    }
    out
}
