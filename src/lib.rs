//! sitefacts: one facts file → every machine-readable file a small website needs.
//!
//! ```text
//! sitefacts build            rewrite the outputs under site/ (stale ones only)
//! sitefacts build --check    exit 1 if any output is stale; write nothing
//! ```
//!
//! Inputs: `facts.json` ([`Facts`]) and `site/index.html` (its `<title>` and body text, and the block between
//! `<!-- machine:begin` and `<!-- machine:end -->` that gets rewritten). Outputs, all under `site/`:
//! the head block in index.html, organization.jsonld, llms.txt, llms-full.txt, ai.txt, humans.txt, robots.txt,
//! sitemap.xml, .well-known/security.txt, site.webmanifest + manifest.json, and the IndexNow key file.
//!
//! Why generated: the same facts (profiles, contact, description) appear in up to six files each; hand-kept
//! copies drift, and a retrieval system handed two versions of a fact cannot pick one.

pub mod facts;
pub mod graph;
pub mod html;
pub mod image;
pub mod outputs;
pub mod wrap;

pub use facts::Facts;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The full set of outputs for one site, path (relative to site/) → content.
pub fn outputs(f: &Facts, index_html: &str) -> Result<BTreeMap<String, String>, String> {
    let title = html::page_title(index_html).ok_or("index.html has no <title>")?;
    let text = html::page_text(index_html, &f.nav_class);
    if text.is_empty() { return Err("found no page text in index.html".into()); }
    let g = graph::graph(f, &title, &text);
    let mut out = BTreeMap::new();
    out.insert("index.html".into(), outputs::splice(index_html, &outputs::head_block(f, &title, &g))?);
    out.insert("organization.jsonld".into(), graph::pretty(&g) + "\n");
    out.insert("llms.txt".into(), outputs::llms_txt(f, &text));
    out.insert("llms-full.txt".into(), outputs::llms_full_txt(f, &title, &text, &g));
    out.insert("ai.txt".into(), outputs::ai_txt(f));
    out.insert("humans.txt".into(), outputs::humans_txt(f));
    out.insert("robots.txt".into(), outputs::robots_txt(f));
    out.insert("sitemap.xml".into(), outputs::sitemap_xml(f));
    if let Some(s) = outputs::security_txt(f) { out.insert(".well-known/security.txt".into(), s); }
    let m = outputs::manifest(f);
    out.insert("site.webmanifest".into(), m.clone());
    out.insert("manifest.json".into(), m);
    if let Some(k) = &f.indexnow_key { out.insert(format!("{k}.txt"), format!("{k}\n")); }
    Ok(out)
}

/// Problems with the inputs: missing images, sizes that disagree with the facts, missing pages, dates.
pub fn check_inputs(f: &Facts, site: &Path) -> (Vec<String>, Vec<String>) {
    let mut problems = vec![];
    let mut warnings = vec![];
    let mut imgs: Vec<(&str, u32, u32)> = vec![(&f.logo.path, f.logo.width, f.logo.height), (&f.og_image.path, f.og_image.width, f.og_image.height)];
    imgs.extend(f.icons.iter().map(|i| (i.path.as_str(), i.width, i.height)));
    for (p, w, h) in imgs {
        let file = site.join(p.trim_start_matches('/'));
        if !file.is_file() { problems.push(format!("missing {p}")); continue; }
        match image::size(&file) {
            Ok((gw, gh)) if (gw, gh) != (w, h) => problems.push(format!("{p} is {gw}x{gh}, facts say {w}x{h}")),
            Err(e) => problems.push(e),
            _ => {}
        }
    }
    for p in &f.page_images { if !site.join(p.trim_start_matches('/')).is_file() { problems.push(format!("missing {p}")); } }
    for pg in &f.pages {
        if !site.join(pg.path.trim_matches('/')).join("index.html").is_file() { problems.push(format!("missing page {}", pg.path)); }
        if let Some(i) = &pg.image { if !site.join(i.trim_start_matches('/')).is_file() { problems.push(format!("missing {i}")); } }
    }
    if chrono::NaiveDate::parse_from_str(&f.updated, "%Y-%m-%d").is_err() { problems.push(format!("updated {:?} is not YYYY-MM-DD", f.updated)); }
    if f.security_email.is_some() {
        match f.security_txt_expires.as_deref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()) {
            None => problems.push("security_txt_expires missing or not RFC 3339 (needed with security_email)".into()),
            Some(exp) => {
                let days = (exp.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_days();
                if days < 0 { problems.push(format!("security_txt_expires {} is in the past", f.security_txt_expires.as_deref().unwrap_or(""))); }
                else if days < 60 { warnings.push(format!("security.txt expires in {days} days; move security_txt_expires forward")); }
            }
        }
    }
    (problems, warnings)
}

#[derive(Debug)]
pub struct Run { pub written: Vec<String>, pub stale: Vec<String>, pub total: usize, pub warnings: Vec<String> }

/// Build (or with `check`, only compare) the outputs for a site rooted at `root` (facts.json + site/).
pub fn run(facts_path: &Path, site: &Path, check: bool) -> Result<Run, String> {
    let f = Facts::parse(&std::fs::read_to_string(facts_path).map_err(|e| format!("{}: {e}", facts_path.display()))?)?;
    let (problems, warnings) = check_inputs(&f, site);
    if !problems.is_empty() { return Err(problems.join("; ")); }
    let index = site.join("index.html");
    let doc = std::fs::read_to_string(&index).map_err(|e| format!("{}: {e}", index.display()))?;
    let outs = outputs(&f, &doc)?;
    let mut stale = vec![];
    let mut written = vec![];
    for (name, content) in &outs {
        let p: PathBuf = site.join(name);
        if std::fs::read_to_string(&p).ok().as_deref() == Some(content.as_str()) { continue; }
        stale.push(name.clone());
        if !check {
            if let Some(d) = p.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
            std::fs::write(&p, content).map_err(|e| format!("{}: {e}", p.display()))?;
            written.push(name.clone());
        }
    }
    Ok(Run { written, stale, total: outs.len(), warnings })
}
