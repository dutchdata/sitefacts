//! `sitefacts sync`: the public internet is the source of truth for public code. GitHub lists the repositories,
//! crates.io the packages; the two are matched by repository URL and written to `code.json` beside the facts
//! file, where [`crate::Facts::load`] picks them up. This is the only network code in the crate, and it is
//! behind the `sync` feature.

use crate::facts::{Code, Sync};
use serde_json::Value;

const UA: &str = concat!("sitefacts/", env!("CARGO_PKG_VERSION"), " (+https://github.com/dutchdata/sitefacts)");

pub struct Synced {
    pub code: Vec<Code>,
    /// Things a human should know: repositories without a description, crates whose repository is not listed.
    pub notes: Vec<String>,
}

fn get(url: &str) -> Result<Value, String> {
    let mut req = ureq::get(url).set("User-Agent", UA).set("Accept", "application/json");
    if url.starts_with("https://api.github.com/") {
        if let Ok(t) = std::env::var("GITHUB_TOKEN") { if !t.is_empty() { req = req.set("Authorization", &format!("Bearer {t}")); } }
    }
    let resp = req.call().map_err(|e| match e {
        ureq::Error::Status(403, _) if url.starts_with("https://api.github.com/") => format!("{url}: 403 (GitHub rate limit? set GITHUB_TOKEN)"),
        e => format!("{url}: {e}"),
    })?;
    resp.into_json().map_err(|e| format!("{url}: {e}"))
}

fn s(v: &Value, k: &str) -> Option<String> { v.get(k).and_then(Value::as_str).filter(|x| !x.trim().is_empty()).map(|x| x.trim().to_string()) }

/// One GitHub API repository object → a [`Code`] entry, or `None` when the sync settings leave it out.
pub fn repo(r: &Value, cfg: &Sync, notes: &mut Vec<String>) -> Option<Code> {
    let name = s(r, "name")?;
    let fork = r.get("fork").and_then(Value::as_bool).unwrap_or(false);
    let archived = r.get("archived").and_then(Value::as_bool).unwrap_or(false);
    if cfg.exclude.iter().any(|x| x == &name) || (fork && !cfg.forks) || (archived && !cfg.archived) || r.get("private").and_then(Value::as_bool).unwrap_or(false) { return None; }
    let description = s(r, "description").unwrap_or_default();
    if description.is_empty() { notes.push(format!("{name}: no description on GitHub")); }
    let license = r.get("license").and_then(|l| s(l, "spdx_id")).filter(|l| l != "NOASSERTION");
    Some(Code {
        name, url: s(r, "html_url")?, description, language: s(r, "language"), license, registry: None, version: None, docs: None,
        homepage: s(r, "homepage").filter(|h| h.starts_with("http")), fork,
        updated: s(r, "pushed_at").map(|d| d.chars().take(10).collect()),
    })
}

/// Every public repository of a GitHub user or organization.
pub fn github(user: &str, cfg: &Sync, notes: &mut Vec<String>) -> Result<Vec<Code>, String> {
    let mut out = vec![];
    for page in 1.. {
        let v = get(&format!("https://api.github.com/users/{user}/repos?type=public&per_page=100&sort=full_name&page={page}"))?;
        let Some(arr) = v.as_array() else { return Err(format!("GitHub: unexpected reply for {user}: {}", v.to_string().chars().take(160).collect::<String>())) };
        out.extend(arr.iter().filter_map(|r| repo(r, cfg, notes)));
        if arr.len() < 100 { break; }
    }
    Ok(out)
}

/// A crates.io crate as the listing API returns it.
#[derive(Debug, Clone, PartialEq)]
pub struct Crate { pub name: String, pub version: String, pub repository: Option<String>, pub description: String, pub documentation: Option<String> }

pub fn crate_of(c: &Value) -> Option<Crate> {
    Some(Crate {
        name: s(c, "name")?,
        version: s(c, "max_stable_version").or_else(|| s(c, "max_version")).unwrap_or_default(),
        repository: s(c, "repository"), description: s(c, "description").unwrap_or_default(), documentation: s(c, "documentation"),
    })
}

/// Every crate owned by a crates.io user (by login).
pub fn crates(user: &str) -> Result<Vec<Crate>, String> {
    let u = get(&format!("https://crates.io/api/v1/users/{user}"))?;
    let id = u.pointer("/user/id").and_then(Value::as_u64).ok_or(format!("crates.io: no user {user}"))?;
    let mut out = vec![];
    for page in 1.. {
        let v = get(&format!("https://crates.io/api/v1/crates?user_id={id}&per_page=100&sort=alpha&page={page}"))?;
        let arr = v.get("crates").and_then(Value::as_array).ok_or("crates.io: no crates array")?;
        out.extend(arr.iter().filter_map(crate_of));
        if arr.len() < 100 { break; }
    }
    Ok(out)
}

fn norm(url: &str) -> String { url.trim().trim_end_matches('/').trim_end_matches(".git").to_ascii_lowercase().replacen("http://", "https://", 1) }

/// Attach each crate to the repository it names; crates whose repository is not in the list get their own entry.
pub fn merge(mut repos: Vec<Code>, crates: Vec<Crate>, notes: &mut Vec<String>) -> Vec<Code> {
    for c in crates {
        let page = format!("https://crates.io/crates/{}", c.name);
        let docs = Some(c.documentation.clone().unwrap_or_else(|| format!("https://docs.rs/{}", c.name)));
        match c.repository.as_deref().and_then(|r| repos.iter_mut().find(|x| norm(&x.url) == norm(r))) {
            Some(r) => {
                r.registry = Some(page);
                r.version = Some(c.version);
                r.docs = docs;
                if r.description.is_empty() { r.description = c.description; }
            }
            None => {
                notes.push(format!("crate {} is not matched to a listed repository ({})", c.name, c.repository.as_deref().unwrap_or("no repository field")));
                repos.push(Code {
                    name: c.name.clone(), url: c.repository.clone().unwrap_or_else(|| page.clone()), description: c.description, language: Some("Rust".into()),
                    license: None, registry: Some(page), version: Some(c.version), docs, homepage: None, fork: false, updated: None,
                });
            }
        }
    }
    repos.sort_by_key(|c| c.name.to_ascii_lowercase());
    repos
}

/// Fetch everything the sync settings name.
pub fn run(cfg: &Sync) -> Result<Synced, String> {
    let mut notes = vec![];
    let repos = match &cfg.github { Some(u) => github(u, cfg, &mut notes)?, None => vec![] };
    let crates = match &cfg.crates { Some(u) => crates(u)?, None => vec![] };
    if repos.is_empty() && crates.is_empty() { return Err("sync: nothing configured (facts.json needs \"sync\": {\"github\": \"<user>\", \"crates\": \"<user>\"})".into()); }
    Ok(Synced { code: merge(repos, crates, &mut notes), notes })
}

/// The `code.json` text: pretty, stable key order, trailing newline.
pub fn to_json(code: &[Code]) -> String { serde_json::to_string_pretty(code).unwrap_or_default() + "\n" }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn repos_and_crates_merge_by_repository_url() {
        let cfg = Sync { github: None, crates: None, exclude: vec!["skipme".into()], forks: false, archived: false };
        let mut notes = vec![];
        let repos: Vec<Code> = [
            json!({"name": "tool", "html_url": "https://github.com/ex/tool", "description": "A tool.", "language": "Rust", "fork": false, "pushed_at": "2026-10-15T01:02:03Z", "license": {"spdx_id": "MIT"}, "homepage": ""}),
            json!({"name": "nodesc", "html_url": "https://github.com/ex/nodesc", "description": null, "language": null, "fork": false, "pushed_at": "2026-01-01T00:00:00Z", "license": {"spdx_id": "NOASSERTION"}}),
            json!({"name": "forked", "html_url": "https://github.com/ex/forked", "description": "x", "fork": true}),
            json!({"name": "skipme", "html_url": "https://github.com/ex/skipme", "description": "x", "fork": false}),
            json!({"name": "old", "html_url": "https://github.com/ex/old", "description": "x", "fork": false, "archived": true}),
        ].iter().filter_map(|r| repo(r, &cfg, &mut notes)).collect();
        assert_eq!(repos.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["tool", "nodesc"]);
        assert_eq!(repos[0].license.as_deref(), Some("MIT"));
        assert_eq!(repos[0].updated.as_deref(), Some("2026-10-15"));
        assert!(repos[1].license.is_none() && repos[0].homepage.is_none());
        assert_eq!(notes, ["nodesc: no description on GitHub"]);

        let crates = vec![
            crate_of(&json!({"name": "tool", "max_stable_version": "0.2.0", "max_version": "0.3.0-rc1", "repository": "https://github.com/ex/tool/", "description": "A tool.", "documentation": null})).unwrap(),
            crate_of(&json!({"name": "loose", "max_version": "1.0.0", "repository": "https://github.com/other/loose", "description": "Loose crate.", "documentation": "https://docs.example"})).unwrap(),
        ];
        let merged = merge(repos, crates, &mut notes);
        assert_eq!(merged.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["loose", "nodesc", "tool"]);
        let tool = &merged[2];
        assert_eq!((tool.registry.as_deref(), tool.version.as_deref(), tool.docs.as_deref()), (Some("https://crates.io/crates/tool"), Some("0.2.0"), Some("https://docs.rs/tool")));
        assert_eq!((merged[0].url.as_str(), merged[0].docs.as_deref(), merged[0].language.as_deref()), ("https://github.com/other/loose", Some("https://docs.example"), Some("Rust")));
        assert!(notes[1].starts_with("crate loose is not matched"));
        let j = to_json(&merged);
        assert!(j.contains("\"registry\": \"https://crates.io/crates/tool\"") && !j.contains("\"fork\"") && !j.contains("\"license\": null"), "{j}");
        let back: Vec<Code> = serde_json::from_str(&j).unwrap();
        assert_eq!(back, merged);
    }
}
