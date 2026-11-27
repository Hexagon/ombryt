//! Markdown to HTML with pulldown-cmark (CommonMark plus tables, footnotes, task lists, strikethrough, heading
//! attributes and definition lists), heading ids, a table of contents and highlighted code blocks.

use crate::config::Config;
use crate::util::{escape_xml, slugify};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Default)]
pub struct TocEntry {
    pub level: u8,
    pub text: String,
    pub id: String,
    pub children: Vec<TocEntry>,
}

#[derive(Debug, Default)]
pub struct Rendered {
    pub html: String,
    pub toc: Vec<TocEntry>,
}

#[derive(Debug, Clone)]
pub struct Renderer {
    options: Options,
    toc_levels: Vec<u8>,
    anchors: bool,
    highlight: bool,
    prefix: String,
}

impl Renderer {
    pub fn new(config: &Config) -> Renderer {
        let mut options = Options::ENABLE_TABLES
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_HEADING_ATTRIBUTES
            | Options::ENABLE_DEFINITION_LIST
            | Options::ENABLE_GFM;
        if config.markdown.smart_punctuation {
            options |= Options::ENABLE_SMART_PUNCTUATION;
        }
        Renderer {
            options,
            toc_levels: config.markdown.toc_levels.clone(),
            anchors: config.markdown.heading_anchors,
            highlight: config.highlight.enabled,
            prefix: config.highlight.prefix.clone(),
        }
    }

    /// A short piece of markdown (an intro, a description): no ids, no toc.
    pub fn render_inline(&self, text: &str) -> String {
        let mut out = String::new();
        html::push_html(&mut out, Parser::new_ext(text, self.options));
        out
    }

    pub fn render(&self, text: &str) -> Rendered {
        let mut out = String::with_capacity(text.len() * 3 / 2);
        let mut flat: Vec<TocEntry> = Vec::new();
        let mut used: HashMap<String, usize> = HashMap::new();
        let mut events = Parser::new_ext(text, self.options);
        let mut pending: Vec<Event> = Vec::new();

        while let Some(event) = events.next() {
            match event {
                Event::Start(Tag::Heading { level, id, classes, attrs }) => {
                    let mut inner = Vec::new();
                    for e in events.by_ref() {
                        if matches!(e, Event::End(TagEnd::Heading(_))) {
                            break;
                        }
                        inner.push(e);
                    }
                    let mut inner_html = String::new();
                    html::push_html(&mut inner_html, inner.iter().cloned());
                    let text = crate::util::strip_tags(&inner_html);
                    let base = id.map(|i| i.to_string()).unwrap_or_else(|| slugify(&text));
                    let id = unique(&mut used, if base.is_empty() { "section".into() } else { base });
                    let n = level_num(level);
                    if self.toc_levels.contains(&n) {
                        flat.push(TocEntry { level: n, text: text.clone(), id: id.clone(), children: vec![] });
                    }
                    flush(&mut out, &mut pending);
                    out += &format!("<h{n} id=\"{}\"", escape_xml(&id));
                    if !classes.is_empty() {
                        let c: Vec<&str> = classes.iter().map(|c| c.as_ref()).collect();
                        out += &format!(" class=\"{}\"", escape_xml(&c.join(" ")));
                    }
                    for (k, v) in attrs {
                        out += &format!(" {}=\"{}\"", escape_xml(&k), escape_xml(v.as_deref().unwrap_or("")));
                    }
                    out += ">";
                    out += &inner_html;
                    if self.anchors {
                        out += &format!(
                            " <a class=\"anchor\" href=\"#{}\" aria-label=\"Link to this section\">#</a>",
                            escape_xml(&id)
                        );
                    }
                    out += &format!("</h{n}>\n");
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    let mut code = String::new();
                    for e in events.by_ref() {
                        match e {
                            Event::Text(t) => code += &t,
                            Event::End(TagEnd::CodeBlock) => break,
                            _ => {}
                        }
                    }
                    let lang = match &kind {
                        CodeBlockKind::Fenced(info) => info.split([' ', ',', '{']).next().unwrap_or("").to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    flush(&mut out, &mut pending);
                    out += &self.code_block(&code, &lang);
                }
                other => pending.push(other),
            }
        }
        flush(&mut out, &mut pending);
        Rendered { html: out, toc: nest(flat) }
    }

    fn code_block(&self, code: &str, lang: &str) -> String {
        if self.highlight
            && !lang.is_empty()
            && let Some(html) = crate::highlight::highlight(code, lang, &self.prefix)
        {
            return html;
        }
        let class = if lang.is_empty() { String::new() } else { format!(" class=\"language-{}\"", escape_xml(lang)) };
        format!("<pre><code{class}>{}</code></pre>\n", escape_xml(code))
    }
}

fn flush(out: &mut String, pending: &mut Vec<Event>) {
    if !pending.is_empty() {
        html::push_html(out, pending.drain(..));
    }
}

fn unique(used: &mut HashMap<String, usize>, base: String) -> String {
    let n = used.entry(base.clone()).or_insert(0);
    *n += 1;
    if *n == 1 { base } else { format!("{base}-{}", *n - 1) }
}

fn level_num(level: HeadingLevel) -> u8 {
    level as u8
}

/// Turns a flat heading list into a tree: each entry holds the deeper ones that follow it.
fn nest(flat: Vec<TocEntry>) -> Vec<TocEntry> {
    fn take(items: &mut std::iter::Peekable<std::vec::IntoIter<TocEntry>>, level: u8) -> Vec<TocEntry> {
        let mut out: Vec<TocEntry> = Vec::new();
        while let Some(next) = items.peek() {
            if next.level < level {
                break;
            }
            let mut entry = items.next().unwrap();
            let child_level = items.peek().map(|c| c.level).filter(|l| *l > entry.level);
            if let Some(level) = child_level {
                entry.children = take(items, level);
            }
            out.push(entry);
        }
        out
    }
    let min = flat.iter().map(|e| e.level).min().unwrap_or(1);
    take(&mut flat.into_iter().peekable(), min)
}

/// Words in markdown, for reading time: everything outside code blocks.
pub fn word_count(text: &str) -> usize {
    let mut n = 0;
    let mut in_code = false;
    for e in Parser::new(text) {
        match e {
            Event::Start(Tag::CodeBlock(_)) => in_code = true,
            Event::End(TagEnd::CodeBlock) => in_code = false,
            Event::Text(t) | Event::Code(t) if !in_code => n += t.split_whitespace().count(),
            _ => {}
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer() -> Renderer {
        Renderer::new(&Config::default())
    }

    #[test]
    fn headings_get_ids_and_toc() {
        let r = renderer().render("# Title\n\n## One\n\n### One A\n\n## One\n\n## Two {#custom}\n");
        assert!(r.html.contains("<h2 id=\"one\">One</h2>"), "{}", r.html);
        assert!(r.html.contains("<h2 id=\"one-1\">One</h2>"), "{}", r.html);
        assert!(r.html.contains("<h2 id=\"custom\">Two</h2>"), "{}", r.html);
        assert_eq!(r.toc.len(), 3);
        assert_eq!(r.toc[0].children[0].id, "one-a");
    }

    #[test]
    fn code_is_highlighted_or_escaped() {
        let r = renderer().render("```rust\nlet x = 1;\n```\n\n```\n<b>\n```\n");
        assert!(r.html.contains("language-rust"), "{}", r.html);
        assert!(r.html.contains("&lt;b&gt;"), "{}", r.html);
    }

    #[test]
    fn gfm() {
        let html = renderer().render("| a |\n|---|\n| b |\n\n- [x] done\n\n~~gone~~").html;
        assert!(html.contains("<table>") && html.contains("checkbox") && html.contains("<del>"), "{html}");
    }

    #[test]
    fn words() {
        assert_eq!(word_count("one two\n\n```\nnot these\n```\n\nthree `four`"), 4);
    }
}
