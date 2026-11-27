//! Small helpers: slugs, dates, data merging, escaping.

use jiff::{Timestamp, civil, tz::TimeZone};
use serde_json::{Map, Value as Json};

/// A URL-safe slug: lower case ASCII letters, digits and single dashes. Non-ASCII letters are kept, lower cased,
/// as browsers handle them fine and stripping them would turn non-English titles into nothing.
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut dash = false;
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(c);
        } else if c != '\'' && c != '’' {
            dash = true;
        }
    }
    out
}

/// Splits a leading `YYYY-MM-DD` (followed by `_` or `-`) off a file stem.
pub fn split_date_prefix(stem: &str) -> (Option<civil::Date>, &str) {
    if stem.len() > 11
        && stem.is_char_boundary(10)
        && matches!(&stem[10..11], "_" | "-")
        && let Ok(date) = stem[..10].parse::<civil::Date>()
    {
        return (Some(date), &stem[11..]);
    }
    (None, stem)
}

/// Parses the date forms found in front matter: RFC 3339, a civil date-time (taken as UTC), or a plain date.
pub fn parse_date(text: &str) -> Option<Timestamp> {
    let text = text.trim();
    if let Ok(ts) = text.parse::<Timestamp>() {
        return Some(ts);
    }
    if let Ok(dt) = text.parse::<civil::DateTime>() {
        return dt.to_zoned_datetime(TimeZone::UTC).ok().map(|z| z.timestamp());
    }
    if let Ok(d) = text.parse::<civil::Date>() {
        return d.to_zoned_datetime(TimeZone::UTC).ok().map(|z| z.timestamp());
    }
    // `2023-09-21 10:00:00 +02:00` and similar.
    if let Ok(z) = jiff::fmt::strtime::parse("%Y-%m-%d %H:%M:%S %:z", text).and_then(|t| t.to_timestamp()) {
        return Some(z);
    }
    None
}

/// The canonical form dates take in page data: RFC 3339 in UTC, second precision.
pub fn format_date(ts: Timestamp) -> String {
    ts.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn date_from_civil(d: civil::Date) -> Timestamp {
    d.to_zoned_datetime(TimeZone::UTC).map(|z| z.timestamp()).unwrap_or(Timestamp::UNIX_EPOCH)
}

/// Merges `over` into `base`: objects merge key by key, `tags` arrays are concatenated (without duplicates),
/// everything else is replaced.
pub fn merge(base: &mut Map<String, Json>, over: &Map<String, Json>) {
    for (key, value) in over {
        match (base.get_mut(key), value) {
            (Some(Json::Object(b)), Json::Object(o)) => merge(b, o),
            (Some(Json::Array(b)), Json::Array(o)) if key == "tags" => {
                for v in o {
                    if !b.contains(v) {
                        b.push(v.clone());
                    }
                }
            }
            _ => {
                base.insert(key.clone(), value.clone());
            }
        }
    }
}

/// `tags: rust` → `tags: [rust]`, so templates and queries can always treat tags as a list.
pub fn normalize_tags(data: &mut Map<String, Json>) {
    if let Some(Json::String(s)) = data.get("tags") {
        let list = s.split(',').map(|t| Json::String(t.trim().to_string())).filter(|t| t != "").collect();
        data.insert("tags".into(), Json::Array(list));
    }
}

pub fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Text content of an HTML fragment, whitespace collapsed. Block-level tags separate words, inline ones do not.
/// Good enough for summaries, word counts and the search index.
pub fn strip_tags(html: &str) -> String {
    const INLINE: &[&str] = &[
        "a", "abbr", "b", "bdi", "cite", "code", "del", "em", "i", "ins", "kbd", "mark", "q", "s", "samp", "small",
        "span", "strong", "sub", "sup", "time", "u", "var",
    ];
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            rest = &rest[start..];
            break;
        };
        let tag = &rest[start + 1..start + end];
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        if !INLINE.contains(&name.as_str()) {
            out.push(' ');
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    let out = out
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    one_line(&out)
}

/// Decodes `%xx` escapes in a URL path; invalid ones are kept as they are.
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok());
        if bytes[i] == b'%'
            && let Some(b) = hex.and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The string form of a data value, as compared in queries.
pub fn json_str(v: &Json) -> String {
    match v {
        Json::String(s) => s.clone(),
        Json::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("running_deno_application_at_boot"), "running-deno-application-at-boot");
        assert_eq!(slugify("  --Trim-- "), "trim");
        assert_eq!(slugify("Don't panic"), "dont-panic");
        assert_eq!(slugify("Ägg på väg"), "ägg-på-väg");
    }

    #[test]
    fn date_prefix() {
        let (d, rest) = split_date_prefix("2023-08-28_scheduling-tasks");
        assert_eq!(d.unwrap().to_string(), "2023-08-28");
        assert_eq!(rest, "scheduling-tasks");
        assert_eq!(split_date_prefix("2023-08-28").1, "2023-08-28");
        assert_eq!(split_date_prefix("about").1, "about");
    }

    #[test]
    fn dates() {
        assert_eq!(format_date(parse_date("2023-09-21").unwrap()), "2023-09-21T00:00:00Z");
        assert_eq!(format_date(parse_date("2023-09-21T10:00:00Z").unwrap()), "2023-09-21T10:00:00Z");
        assert_eq!(format_date(parse_date("2023-09-21T10:00:00+02:00").unwrap()), "2023-09-21T08:00:00Z");
        assert_eq!(format_date(parse_date("2023-09-21 10:00:00").unwrap()), "2023-09-21T10:00:00Z");
        assert!(parse_date("yesterday").is_none());
    }

    #[test]
    fn merging() {
        let mut base = serde_json::json!({"tags": ["post"], "a": {"x": 1, "y": 2}, "b": 1});
        let over = serde_json::json!({"tags": ["rust", "post"], "a": {"y": 3}, "b": 2});
        merge(base.as_object_mut().unwrap(), over.as_object().unwrap());
        assert_eq!(base, serde_json::json!({"tags": ["post", "rust"], "a": {"x": 1, "y": 3}, "b": 2}));
    }

    #[test]
    fn decoding() {
        assert_eq!(percent_decode("a%20b%2"), "a b%2");
        assert_eq!(percent_decode("%C3%A4"), "ä");
    }

    #[test]
    fn tags() {
        assert_eq!(strip_tags("<p>a &amp; <b>b</b>, c</p><p>d</p>"), "a & b, c d");
    }
}
