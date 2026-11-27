//! ombryt: a single-binary static site generator. See README.md.

mod build;
mod config;
mod content;
mod highlight;
mod markdown;
mod output;
mod query;
mod render;
mod scaffold;
mod serve;
mod theme;
mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "ombryt", version, about = "A fast, single-binary static site generator")]
struct Cli {
    /// The project directory (where ombryt.toml is).
    #[arg(short = 'C', long, global = true, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build the site into the output directory.
    Build {
        /// Include pages with `draft: true`.
        #[arg(long)]
        drafts: bool,
        /// Fail on broken internal links.
        #[arg(long)]
        strict: bool,
        /// Build for another URL than `site.url`, such as a preview.
        #[arg(long)]
        url: Option<String>,
    },
    /// Build, serve on localhost, rebuild on changes and reload the browser.
    Serve {
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long)]
        drafts: bool,
        /// Open the site in a browser.
        #[arg(long)]
        open: bool,
    },
    /// Create a starter site.
    Init {
        #[arg(default_value = ".")]
        dir: PathBuf,
        #[arg(long)]
        force: bool,
        /// Start from a theme's starter instead of a blog (`docs`).
        #[arg(long)]
        theme: Option<String>,
    },
    /// Create a new dated post.
    New {
        title: String,
        /// Directory under src for the post.
        #[arg(long, default_value = "posts")]
        section: String,
        #[arg(long)]
        draft: bool,
    },
    /// List the built-in site themes and the syntax highlighting themes.
    Themes,
}

pub fn print_report(report: &build::Report) {
    for w in &report.warnings {
        eprintln!("warning: {w}");
    }
    println!(
        "built {} pages and {} files in {} ms{}",
        report.pages,
        report.assets,
        report.elapsed.as_millis(),
        if report.warnings.is_empty() { String::new() } else { format!(", {} warnings", report.warnings.len()) }
    );
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { drafts, strict, url } => {
            let mut config = config::Config::load(&cli.root)?;
            config.build.strict_links |= strict;
            let report = build::build(&config, &build::Options { drafts, site_url: url, ..Default::default() })?;
            print_report(&report);
        }
        Command::Serve { port, host, drafts, open } => serve::serve(&cli.root, &host, port, drafts, open)?,
        Command::Init { dir, force, theme } => scaffold::init(&cli.root.join(dir), force, theme.as_deref())?,
        Command::New { title, section, draft } => {
            let config = config::Config::load(&cli.root)?;
            let path = scaffold::new_post(&config, &title, &section, draft)?;
            println!("{}", path.display());
        }
        Command::Themes => {
            println!("Site themes ([build] theme):");
            for t in theme::BUILTIN {
                println!("  {:<12} {}", t.name, t.description);
            }
            println!("\nHighlight themes ([highlight] theme, light_theme):");
            for name in highlight::theme_names() {
                println!("  {name}");
            }
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
