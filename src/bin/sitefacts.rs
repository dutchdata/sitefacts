use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sitefacts", version, about = "One facts file → every machine-readable file a small website needs")]
struct Cli {
    /// Project root holding facts.json and site/ (default: the current directory)
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    /// Facts file (default: <root>/facts.json)
    #[arg(long, global = true)]
    facts: Option<PathBuf>,
    /// Output directory (default: <root>/site)
    #[arg(long, global = true)]
    site: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Write stale outputs (all of them on the first run)
    Build {
        /// Exit 1 if any output is stale; write nothing
        #[arg(long)]
        check: bool,
    },
    /// Same as `build --check`
    Check,
    /// Print the outputs to stdout instead of writing them (one file, or all with --all)
    Show { name: Option<String>, #[arg(long)] all: bool },
    /// Print an example facts.json to start from
    Init,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli.root.unwrap_or_else(|| PathBuf::from("."));
    let facts = cli.facts.unwrap_or_else(|| root.join("facts.json"));
    let site = cli.site.unwrap_or_else(|| root.join("site"));
    match cli.cmd {
        Cmd::Build { check } => run(&facts, &site, check),
        Cmd::Check => run(&facts, &site, true),
        Cmd::Show { name, all } => {
            let f = sitefacts::Facts::parse(&std::fs::read_to_string(&facts)?).map_err(anyhow::Error::msg)?;
            let doc = std::fs::read_to_string(site.join("index.html"))?;
            let outs = sitefacts::outputs(&f, &doc).map_err(anyhow::Error::msg)?;
            match (name, all) {
                (Some(n), _) => match outs.get(&n) { Some(c) => print!("{c}"), None => bail!("no output named {n}; have: {}", outs.keys().cloned().collect::<Vec<_>>().join(", ")) },
                (None, true) => for (n, c) in &outs { println!("===== {n}\n{c}"); },
                (None, false) => for n in outs.keys() { println!("{n}"); },
            }
            Ok(())
        }
        Cmd::Init => { print!("{}", EXAMPLE); Ok(()) }
    }
}

fn run(facts: &std::path::Path, site: &std::path::Path, check: bool) -> Result<()> {
    let r = sitefacts::run(facts, site, check).map_err(anyhow::Error::msg)?;
    for w in &r.warnings { eprintln!("warning: {w}"); }
    if check {
        if !r.stale.is_empty() { bail!("stale: {} (run sitefacts build)", r.stale.join(", ")); }
        println!("ok: {} files current", r.total);
    } else {
        println!("{} of {} files written{}", r.written.len(), r.total, if r.written.is_empty() { String::new() } else { format!(": {}", r.written.join(", ")) });
    }
    Ok(())
}

const EXAMPLE: &str = r#"{
  "updated": "2026-01-01",
  "url": "https://www.example.com/",
  "legal_name": "EXAMPLE, INC.",
  "brand": "Example",
  "kind": "studio",
  "description": "What the company does, in one or two sentences.",
  "email": "hello@example.com",
  "security_email": "security@example.com",
  "security_txt_expires": "2027-01-01T00:00:00Z",
  "profiles": [{ "name": "Bluesky", "url": "https://bsky.app/profile/example.com", "handle": "@example.com" }],
  "twitter_site": "@example",
  "knows_about": [{ "name": "Software", "wikidata": "Q7397" }],
  "logo": { "path": "/logo.png", "width": 1024, "height": 1024, "caption": "Example logo" },
  "og_image": { "path": "/og-image.png", "width": 1200, "height": 630, "alt": "Example" },
  "icons": [{ "path": "/favicon-32.png", "width": 32, "height": 32, "type": "image/png" }],
  "page_images": [],
  "pages": [],
  "attribution": { "name": "ai", "values": ["openai", "anthropic", "google", "apple", "other"] },
  "humans": { "built_with": "One hand-written HTML page.", "hosted_on": "Static hosting" }
}
"#;
