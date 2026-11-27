//! Builds the starter site (`ombryt init`) and checks the output end to end.

use std::path::Path;
use std::process::Command;

fn ombryt(dir: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_ombryt")).arg("-C").arg(dir).args(args).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "ombryt {args:?} failed:\n{text}");
    text
}

fn read(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join("_site").join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn starter_site_builds_cleanly() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    ombryt(dir, &["init"]);
    let report = ombryt(dir, &["build", "--strict"]);
    assert!(!report.contains("warning"), "{report}");

    let post = read(dir, "posts/hello-world/index.html");
    assert!(post.contains("<h1>Hello, world</h1>"));
    assert!(post.contains("<h2 id=\"code\">Code</h2>"));
    assert!(post.contains("class=\"language-rust\""));
    assert!(post.contains("property=\"og:title\""));
    assert!(post.contains("January 1, 2026"));
    assert!(!post.contains("rel=\"stylesheet\" href=\"/css/style.css\""), "style.css is inlined");

    let archive = read(dir, "archive/meta/index.html");
    assert!(archive.contains("/posts/hello-world/"));
    assert!(read(dir, "404.html").contains("Not found"));
    assert!(read(dir, "feed.rss").contains("<guid isPermaLink=\"true\">https://example.com/posts/hello-world/</guid>"));
    assert!(read(dir, "feed.atom").contains("<entry>"));
    let feed: serde_json::Value = serde_json::from_str(&read(dir, "feed.json")).unwrap();
    assert_eq!(feed["items"][0]["tags"], serde_json::json!(["meta"]));
    let sitemap = read(dir, "sitemap.xml");
    assert!(sitemap.contains("https://example.com/posts/hello-world/") && !sitemap.contains("404"));
    assert!(read(dir, "css/highlight.css").contains(".hl-"));
}

#[test]
fn new_post_and_drafts() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    ombryt(dir, &["init"]);
    let path = ombryt(dir, &["new", "A draft post", "--draft"]);
    assert!(path.trim().ends_with("_a-draft-post.md"), "{path}");
    ombryt(dir, &["build"]);
    assert!(!dir.join("_site/posts/a-draft-post/index.html").exists());
    ombryt(dir, &["build", "--drafts"]);
    assert!(dir.join("_site/posts/a-draft-post/index.html").exists());
}

#[test]
fn broken_links_fail_strict_builds() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    ombryt(dir, &["init"]);
    std::fs::write(dir.join("src/posts/2026-01-02_broken.md"), "---\ntitle: Broken\n---\n[x](/nowhere/)\n").unwrap();
    let out =
        Command::new(env!("CARGO_BIN_EXE_ombryt")).arg("-C").arg(dir).args(["build", "--strict"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("broken link /nowhere/"));
}

#[test]
fn docs_theme_starter_builds_cleanly() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    ombryt(dir, &["init", "--theme", "docs"]);
    let report = ombryt(dir, &["build", "--strict"]);
    assert!(!report.contains("warning"), "{report}");
    let page = read(dir, "guide/installation/index.html");
    assert!(page.contains("aria-current=\"page\">Installation</a>"), "the page is marked in the navigation");
    assert!(page.contains("<a href=\"/guide/usage/\">Usage →</a>"), "the next page follows nav_order");
    assert!(page.contains("<style>"), "the theme's stylesheet is inlined");
    assert!(read(dir, "theme/search.js").contains("search.json"));
    let index: serde_json::Value = serde_json::from_str(&read(dir, "search.json")).unwrap();
    assert_eq!(index["documents"].as_array().unwrap().len(), 4);

    // A project file wins over the theme's.
    std::fs::create_dir_all(dir.join("src/theme")).unwrap();
    std::fs::write(dir.join("src/theme/search.js"), "// mine").unwrap();
    ombryt(dir, &["build"]);
    assert_eq!(read(dir, "theme/search.js"), "// mine");
}

#[test]
fn page_fields_do_not_hide_globals() {
    // `search: false` used to replace the `search` object in that page's templates (issue #3).
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    ombryt(dir, &["init", "--theme", "docs"]);
    let page = "---\ntitle: Page not found\nurl: /404.html\nnav_exclude: true\nsearch: false\n---\n\nNot here.\n";
    std::fs::write(dir.join("src/404.md"), page).unwrap();
    ombryt(dir, &["build", "--strict"]);
    assert!(read(dir, "404.html").contains("Not here."));
    assert!(!read(dir, "search.json").contains("/404.html"), "the page is left out of the index");
}
