//! The page query language used by `search.pages()`, feeds and taxonomies. Close to Lume's, so templates port
//! easily:
//!
//! - `rust` — pages tagged `rust`; `"two words"` for tags with spaces; `!draft` for pages *not* tagged so
//! - `part=1`, `author.name=Hexagon` — a field equals a value (for lists: contains it); `key!=value` for not
//! - terms are combined with AND; an empty query matches every page
//!
//! Sorting: `date=desc`, `part`, `title=asc date=desc` (several keys, the first wins).

use crate::util::json_str;
use serde_json::{Map, Value as Json};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq)]
enum Term {
    Tag(String, bool),
    Field(Vec<String>, String, bool),
}

#[derive(Debug, Clone, Default)]
pub struct Query(Vec<Term>);

fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut had_quote = false;
    for c in text.chars() {
        match c {
            '"' | '\'' => {
                quoted = !quoted;
                had_quote = true;
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() || had_quote {
                    out.push(std::mem::take(&mut cur));
                }
                had_quote = false;
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() || had_quote {
        out.push(cur);
    }
    out
}

impl Query {
    pub fn parse(text: &str) -> Query {
        let mut terms = Vec::new();
        for tok in tokens(text) {
            if let Some((key, value)) = tok.split_once("!=") {
                terms.push(Term::Field(path(key), value.to_string(), false));
            } else if let Some((key, value)) = tok.split_once('=') {
                terms.push(Term::Field(path(key), value.to_string(), true));
            } else if let Some(tag) = tok.strip_prefix('!') {
                terms.push(Term::Tag(tag.to_string(), false));
            } else {
                terms.push(Term::Tag(tok, true));
            }
        }
        Query(terms)
    }

    pub fn matches(&self, data: &Map<String, Json>) -> bool {
        self.0.iter().all(|term| match term {
            Term::Tag(tag, want) => {
                let has = data
                    .get("tags")
                    .and_then(Json::as_array)
                    .is_some_and(|tags| tags.iter().any(|t| t.as_str() == Some(tag)));
                has == *want
            }
            Term::Field(key, value, want) => {
                let has = match lookup(data, key) {
                    Some(Json::Array(items)) => items.iter().any(|v| json_str(v) == *value),
                    Some(v) => json_str(v) == *value,
                    None => value.is_empty() || value == "undefined" || value == "null",
                };
                has == *want
            }
        })
    }
}

fn path(key: &str) -> Vec<String> {
    key.split('.').map(str::to_string).collect()
}

pub fn lookup<'a>(data: &'a Map<String, Json>, path: &[String]) -> Option<&'a Json> {
    let (first, rest) = path.split_first()?;
    let mut cur = data.get(first)?;
    for key in rest {
        cur = cur.get(key)?;
    }
    if cur.is_null() { None } else { Some(cur) }
}

#[derive(Debug, Clone)]
pub struct Sort(Vec<(Vec<String>, bool)>);

impl Sort {
    pub fn parse(text: &str) -> Sort {
        let keys = text
            .split_whitespace()
            .map(|t| match t.split_once('=') {
                Some((k, dir)) => (path(k), dir.eq_ignore_ascii_case("desc")),
                None => (path(t), false),
            })
            .collect();
        Sort(keys)
    }

    pub fn compare(&self, a: &Map<String, Json>, b: &Map<String, Json>) -> Ordering {
        for (key, desc) in &self.0 {
            let ord = match (lookup(a, key), lookup(b, key)) {
                (Some(x), Some(y)) => {
                    let o = compare_json(x, y);
                    if *desc { o.reverse() } else { o }
                }
                // Pages without the field go last, whatever the direction.
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            };
            if ord != Ordering::Equal {
                return ord;
            }
        }
        // Ties: by source path, in the direction of the first key, so the order never depends on the file system.
        let desc = self.0.first().is_some_and(|(_, d)| *d);
        let src = |m: &Map<String, Json>| m.get("src").map(json_str).unwrap_or_default();
        let o = src(a).cmp(&src(b));
        if desc { o.reverse() } else { o }
    }
}

fn compare_json(a: &Json, b: &Json) -> Ordering {
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => {
            x.as_f64().unwrap_or(0.0).partial_cmp(&y.as_f64().unwrap_or(0.0)).unwrap_or(Ordering::Equal)
        }
        (Json::Bool(x), Json::Bool(y)) => x.cmp(y),
        _ => json_str(a).to_lowercase().cmp(&json_str(b).to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn page(v: Json) -> Map<String, Json> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn matching() {
        let p = page(json!({"tags": ["post", "rust", "two words"], "part": 1, "author": {"name": "H"}}));
        assert!(Query::parse("").matches(&p));
        assert!(Query::parse("post rust").matches(&p));
        assert!(!Query::parse("post js").matches(&p));
        assert!(Query::parse("\"two words\"").matches(&p));
        assert!(Query::parse("part=1").matches(&p));
        assert!(Query::parse("part!=2 !draft").matches(&p));
        assert!(Query::parse("author.name=H").matches(&p));
        assert!(Query::parse("tags=rust").matches(&p));
        assert!(!Query::parse("!rust").matches(&p));
    }

    #[test]
    fn sorting() {
        let a = page(json!({"date": "2023-01-01T00:00:00Z", "part": 2}));
        let b = page(json!({"date": "2024-01-01T00:00:00Z", "part": 10}));
        let c = page(json!({}));
        assert_eq!(Sort::parse("date=desc").compare(&a, &b), Ordering::Greater);
        assert_eq!(Sort::parse("part").compare(&a, &b), Ordering::Less);
        assert_eq!(Sort::parse("part=desc").compare(&c, &a), Ordering::Greater);
    }
}
