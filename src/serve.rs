//! `ombryt serve`: build, serve the output, rebuild on every change and reload open pages.

use crate::build::{self, Options};
use crate::config::Config;
use crate::output::LIVE_PATH;
use anyhow::Result;
use notify::RecursiveMode;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Clients = Arc<Mutex<Vec<Sender<()>>>>;

fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "xml" => "application/xml",
        "rss" => "application/rss+xml",
        "atom" => "application/atom+xml",
        "txt" | "md" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" | "cur" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

enum Found {
    File(PathBuf),
    Redirect(String),
    Missing,
}

fn resolve(root: &Path, url: &str) -> Found {
    let path = crate::util::percent_decode(url.split(['?', '#']).next().unwrap_or("/"));
    if path.split('/').any(|s| s == "..") {
        return Found::Missing;
    }
    let file = root.join(path.trim_start_matches('/'));
    if file.is_file() {
        return Found::File(file);
    }
    if file.is_dir() {
        if !path.ends_with('/') {
            return Found::Redirect(format!("{path}/"));
        }
        let index = file.join("index.html");
        if index.is_file() {
            return Found::File(index);
        }
    }
    let html = PathBuf::from(format!("{}.html", file.display()));
    if html.is_file() {
        return Found::File(html);
    }
    Found::Missing
}

fn header(k: &str, v: &str) -> tiny_http::Header {
    tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap()
}

fn handle(req: tiny_http::Request, root: &Path, clients: &Clients) {
    let url = req.url().to_string();
    if url == LIVE_PATH {
        // Server-sent events, written straight to the socket so that every event is flushed at once.
        let (tx, rx) = channel();
        clients.lock().unwrap().push(tx);
        let mut w = req.into_writer();
        let head =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n\r\n: connected\n\n";
        if w.write_all(head.as_bytes()).and_then(|_| w.flush()).is_err() {
            return;
        }
        loop {
            let event: &[u8] = match rx.recv_timeout(Duration::from_secs(20)) {
                Ok(()) => b"data: reload\n\n",
                Err(RecvTimeoutError::Timeout) => b": keep-alive\n\n",
                Err(RecvTimeoutError::Disconnected) => return,
            };
            if w.write_all(event).and_then(|_| w.flush()).is_err() {
                return;
            }
        }
    }
    let result = match resolve(root, &url) {
        Found::File(path) => match std::fs::File::open(&path) {
            Ok(file) => req.respond(
                tiny_http::Response::from_file(file)
                    .with_header(header("Content-Type", mime(&path)))
                    .with_header(header("Cache-Control", "no-store")),
            ),
            Err(_) => req.respond(tiny_http::Response::empty(500)),
        },
        Found::Redirect(to) => req.respond(tiny_http::Response::empty(301).with_header(header("Location", &to))),
        Found::Missing => {
            let page = root.join("404.html");
            match std::fs::read(&page) {
                Ok(body) => req.respond(
                    tiny_http::Response::from_data(body)
                        .with_status_code(404)
                        .with_header(header("Content-Type", "text/html; charset=utf-8")),
                ),
                Err(_) => req.respond(tiny_http::Response::from_string("404 not found").with_status_code(404)),
            }
        }
    };
    let _ = result;
}

fn rebuild(root: &Path, opts: &Options) -> Option<Config> {
    let config = match Config::load(root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e:#}");
            return None;
        }
    };
    match build::build(&config, opts) {
        Ok(report) => {
            crate::print_report(&report);
            Some(config)
        }
        Err(e) => {
            eprintln!("error: {e:#}");
            None
        }
    }
}

pub fn serve(root: &Path, host: &str, port: u16, drafts: bool, open: bool) -> Result<()> {
    let address = format!("{host}:{port}");
    let local = format!("http://{}:{port}", if host == "0.0.0.0" { "localhost" } else { host });
    let opts = Options { drafts, live_reload: true, site_url: Some(local.clone()) };
    let config = Config::load(root)?;
    rebuild(root, &opts);

    let server = tiny_http::Server::http(&address).map_err(|e| anyhow::anyhow!("can not listen on {address}: {e}"))?;
    let clients: Clients = Default::default();

    // Watch, debounce, rebuild, reload.
    let (tx, rx) = channel::<()>();
    let out_dir = config.out_dir();
    let mut debouncer = notify_debouncer_full::new_debouncer(Duration::from_millis(120), None, move |res| {
        if let Ok(events) = res {
            let events: Vec<notify_debouncer_full::DebouncedEvent> = events;
            if events.iter().any(|e| e.paths.iter().any(|p| !p.starts_with(&out_dir))) {
                let _ = tx.send(());
            }
        }
    })?;
    for path in build::watched(&config) {
        let mode = if path.is_dir() { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };
        debouncer.watch(&path, mode)?;
    }
    {
        let clients = clients.clone();
        let root = root.to_path_buf();
        std::thread::spawn(move || {
            while rx.recv().is_ok() {
                while rx.try_recv().is_ok() {}
                if rebuild(&root, &opts).is_some() {
                    clients.lock().unwrap().retain(|c| c.send(()).is_ok());
                }
            }
        });
    }

    println!("serving {} at {local}/ (ctrl-c to stop)", config.out_dir().display());
    if open {
        let cmd = if cfg!(target_os = "macos") {
            "open"
        } else if cfg!(windows) {
            "explorer"
        } else {
            "xdg-open"
        };
        let _ = std::process::Command::new(cmd).arg(format!("{local}/")).spawn();
    }
    let root_dir = config.out_dir();
    for req in server.incoming_requests() {
        let clients = clients.clone();
        let root_dir = root_dir.clone();
        std::thread::spawn(move || handle(req, &root_dir, &clients));
    }
    drop(debouncer);
    Ok(())
}
