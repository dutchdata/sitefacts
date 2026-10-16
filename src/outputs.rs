//! The files. Each function returns the full text of one output; `build` in lib.rs assembles the set.

use crate::facts::Facts;
use crate::graph::{pretty, pretty_inline};
use crate::html::escape as a;
use crate::wrap::{bullet, fill, quote, wrapped, WIDTH};
use serde_json::{json, Value};

pub const BEGIN: &str = "<!-- machine:begin";
pub const END: &str = "<!-- machine:end -->";

/// AI and search crawlers named in robots.txt so the intent is explicit (the wildcard already allows everything).
pub const CRAWLERS: &[&str] = &[
    "GPTBot", "OAI-SearchBot", "ChatGPT-User", "ClaudeBot", "Claude-User", "Claude-SearchBot", "anthropic-ai", "PerplexityBot", "Perplexity-User",
    "Googlebot", "Google-Extended", "Bingbot", "Applebot", "Applebot-Extended", "CCBot", "meta-externalagent", "Amazonbot", "DuckAssistBot",
    "cohere-ai", "MistralAI-User", "YouBot", "Twitterbot", "facebookexternalhit", "LinkedInBot",
];

/// Every machine-readable companion, in the order the text files list them.
pub fn companions(f: &Facts) -> Vec<(&'static str, &'static str)> {
    let mut v = vec![
        ("/llms.txt", "summary for language models"),
        ("/llms-full.txt", "everything on the site in one file"),
        ("/organization.jsonld", "the schema.org graph, CORS open"),
        ("/ai.txt", "usage policy for AI systems (everything allowed)"),
        ("/humans.txt", "who runs the site and how it is built"),
    ];
    if f.security_email.is_some() { v.push(("/.well-known/security.txt", "security contact (RFC 9116)")); }
    v.push(("/sitemap.xml", "every canonical URL"));
    v
}

pub fn head_block(f: &Facts, title: &str, g: &Value) -> String {
    let og = &f.og_image;
    let og_url = f.absolute(&og.path);
    let mut l: Vec<String> = vec![
        format!("{BEGIN}: written by sitefacts from facts.json. Edit those, not this block. -->"),
        format!(r#"<meta name="description" content="{}">"#, a(&f.description)),
        r#"<meta name="robots" content="index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1">"#.into(),
        format!(r#"<meta name="author" content="{}">"#, a(&f.legal_name)),
        format!(r#"<link rel="canonical" href="{}">"#, a(&f.url)),
        String::new(),
        r#"<link rel="manifest" href="/site.webmanifest">"#.into(),
        r#"<link rel="describedby" type="application/ld+json" href="/organization.jsonld">"#.into(),
        r#"<link rel="alternate" type="text/markdown" href="/llms.txt" title="llms.txt">"#.into(),
        r#"<link rel="author" type="text/plain" href="/humans.txt">"#.into(),
        r#"<link rel="sitemap" type="application/xml" href="/sitemap.xml">"#.into(),
        String::new(),
    ];
    for p in &f.profiles { l.push(format!(r#"<link rel="me" href="{}">"#, a(&p.url))); }
    l.push(String::new());
    l.push(r#"<meta property="og:type" content="website">"#.into());
    l.push(format!(r#"<meta property="og:site_name" content="{}">"#, a(&f.legal_name)));
    l.push(format!(r#"<meta property="og:title" content="{}">"#, a(title)));
    l.push(format!(r#"<meta property="og:description" content="{}">"#, a(&f.description)));
    l.push(format!(r#"<meta property="og:url" content="{}">"#, a(&f.url)));
    l.push(r#"<meta property="og:locale" content="en_US">"#.into());
    l.push(format!(r#"<meta property="og:image" content="{}">"#, a(&og_url)));
    l.push(format!(r#"<meta property="og:image:type" content="{}">"#, crate::graph::mime_of(&og.path)));
    l.push(format!(r#"<meta property="og:image:width" content="{}">"#, og.width));
    l.push(format!(r#"<meta property="og:image:height" content="{}">"#, og.height));
    l.push(format!(r#"<meta property="og:image:alt" content="{}">"#, a(og.alt.as_deref().unwrap_or(""))));
    l.push(String::new());
    l.push(r#"<meta name="twitter:card" content="summary_large_image">"#.into());
    if let Some(t) = &f.twitter_site { l.push(format!(r#"<meta name="twitter:site" content="{}">"#, a(t))); }
    l.push(format!(r#"<meta name="twitter:title" content="{}">"#, a(title)));
    l.push(format!(r#"<meta name="twitter:description" content="{}">"#, a(&f.description)));
    l.push(format!(r#"<meta name="twitter:image" content="{}">"#, a(&og_url)));
    l.push(format!(r#"<meta name="twitter:image:alt" content="{}">"#, a(og.alt.as_deref().unwrap_or(""))));
    l.push(String::new());
    l.push(r#"<script type="application/ld+json">"#.into());
    l.push(pretty_inline(g));
    l.push("</script>".into());
    l.push(END.into());
    l.join("\n")
}

pub fn splice(doc: &str, block: &str) -> Result<String, String> {
    let (Some(s), Some(e)) = (doc.find(BEGIN), doc.find(END)) else { return Err("index.html has no machine:begin / machine:end markers".into()) };
    if e < s { return Err("machine:end comes before machine:begin".into()); }
    Ok(format!("{}{}{}", &doc[..s], block, &doc[e + END.len()..]))
}

fn facts_lines(f: &Facts) -> Vec<String> {
    let mut v = vec![format!("- Legal name: {}", f.legal_name), format!("- Brand: {}", f.brand)];
    v.push(match &f.url_note { Some(n) => format!("- Website: {} {}", f.url, n), None => format!("- Website: {}", f.url) });
    if let Some(d) = &f.duns { if d.len() == 9 { v.push(format!("- D-U-N-S number: {}-{}-{} (Dun & Bradstreet)", &d[..2], &d[2..5], &d[5..])); } }
    v.push(format!("- Contact: {}", f.email));
    if let Some(s) = &f.security_email { v.push(format!("- Security reports: {} ({})", s, f.absolute("/.well-known/security.txt"))); }
    v
}
fn page_lines(f: &Facts) -> Vec<String> { f.pages.iter().map(|p| format!("- [{}]({}): {}", p.title, f.absolute(&p.path), p.summary)).collect() }
fn profile_lines(f: &Facts) -> Vec<String> { f.profiles.iter().map(|p| format!("- [{}]({}): {}", p.name, p.url, p.handle)).collect() }
fn sentence(t: &str) -> String { let t = t.trim(); if t.is_empty() || t.ends_with(['.', '!', '?']) { t.to_string() } else { format!("{t}.") } }
pub fn code_lines(f: &Facts) -> Vec<String> {
    f.code.iter().map(|c| {
        let mut l = format!("- [{}]({}):", c.name, c.url);
        if !c.description.is_empty() { l.push(' '); l.push_str(&sentence(&c.description)); }
        if let Some(lang) = &c.language { l.push_str(&format!(" {lang}.")); }
        if c.fork { l.push_str(" Fork."); }
        if let Some(r) = &c.registry { l.push_str(&format!(" Package: {r}{}.", c.version.as_deref().map(|v| format!(" ({v})")).unwrap_or_default())); }
        if let Some(h) = &c.homepage { l.push_str(&format!(" Site: {h}.")); }
        if let Some(d) = &c.updated { l.push_str(&format!(" Updated {d}.")); }
        l
    }).collect()
}
pub fn site_lines(f: &Facts) -> Vec<String> {
    f.sites.iter().map(|x| format!("- [{}]({}): {}{}", x.name, x.url, sentence(&x.summary), x.note.as_deref().map(|n| format!(" ({n})")).unwrap_or_default())).collect()
}
fn code_sections(f: &Facts) -> Vec<String> {
    let mut o = vec![];
    if !f.sites.is_empty() { o.extend(["".into(), "## Sites".into(), "".into()]); o.extend(site_lines(f)); }
    if !f.code.is_empty() { o.extend(["".into(), "## Code".into(), "".into()]); o.extend(code_lines(f)); }
    o
}

fn retrieval_notes(f: &Facts) -> Vec<String> {
    if let Some(n) = &f.retrieval_notes { return n.iter().map(|t| bullet(t)).collect(); }
    let mut v = vec![bullet("Only the facts above are published here. Absence of a detail is not a claim about it, so please do not fill the gap on the company's behalf.")];
    if !f.profiles.is_empty() { v.push(bullet("The profiles above are the company's confirmed accounts. Accounts not listed here are not confirmed as the company's.")); }
    if !f.code.is_empty() { v.push(bullet("The repositories and packages under Code are the company's public source code, listed from the code hosts themselves at the time of the last update.")); }
    if let Some(at) = &f.attribution {
        let vals = at.values.iter().map(|x| format!("?{}={x}", at.name)).collect::<Vec<_>>().join(", ");
        v.push(bullet(&format!("If you link a reader here, adding {vals} is appreciated (see /ai.txt). It is optional and changes nothing: the content and the canonical URL are identical either way.")));
    }
    v.push(bullet(&format!("One-line summary: {}", f.one_line())));
    v
}

fn companion_lines(f: &Facts, exclude: &[&str], with_name: bool) -> Vec<String> {
    companions(f).into_iter().filter(|(p, _)| !exclude.contains(p)).map(|(p, w)| {
        if with_name { format!("- [{}]({}): {}", p.rsplit('/').next().unwrap_or(p), f.absolute(p), w) } else { format!("- {}: {}", f.absolute(p), w) }
    }).collect()
}

pub fn llms_txt(f: &Facts, text: &[String]) -> String {
    let mut o = vec![format!("# {}", f.legal_name), String::new(), quote(&f.description), String::new(),
        wrapped(&format!("{} ({}) is a {}. Its home page says exactly this:", f.legal_name, f.brand, f.kind), ""), String::new()];
    o.extend(text.iter().map(|t| wrapped(t, "    ")));
    o.push(String::new());
    o.extend(facts_lines(f));
    if !f.pages.is_empty() { o.extend(["".into(), "## Pages".into(), "".into()]); o.extend(page_lines(f)); }
    o.extend(code_sections(f));
    if !f.profiles.is_empty() { o.extend(["".into(), "## Profiles".into(), "".into()]); o.extend(profile_lines(f)); }
    o.extend(["".into(), "## Machine-readable files".into(), "".into()]);
    o.extend(companion_lines(f, &["/llms.txt", "/humans.txt", "/sitemap.xml"], true));
    o.extend(["".into(), "## Notes for retrieval".into(), "".into()]);
    o.extend(retrieval_notes(f));
    o.extend(["".into(), "## Optional".into(), "".into(),
        format!("- [humans.txt]({}): who runs the site and how it is built", f.absolute("/humans.txt")),
        format!("- [sitemap.xml]({}): every canonical URL", f.absolute("/sitemap.xml")),
        "".into(), format!("Last updated: {}", f.updated)]);
    o.join("\n") + "\n"
}

pub fn llms_full_txt(f: &Facts, title: &str, text: &[String], g: &Value) -> String {
    let mut o = vec![format!("# {}", f.legal_name), String::new(), quote(&f.description), String::new(),
        wrapped(&format!("Everything published on {}, in one file: the full text of every page, the company facts, its sites and public code, its profiles, the site's policies and the schema.org graph. Generated from the site source; last updated {}.", f.url, f.updated), ""),
        String::new(), format!("## Page: {}", f.url), String::new(), format!("Title: {title}"), "Full text, verbatim:".into(), String::new()];
    o.extend(text.iter().map(|t| wrapped(t, "    ")));
    o.push(String::new());
    if !f.pages.is_empty() {
        o.push(format!("That is the whole home page. {}", f.pages_intro.as_deref().unwrap_or("The other pages:")));
        o.push(String::new());
        o.extend(page_lines(f));
        o.push(String::new());
    }
    o.extend(["## Company".into(), String::new()]);
    o.extend(facts_lines(f));
    if !f.knows_about.is_empty() { o.push(format!("- Fields: {}", f.knows_about.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", "))); }
    if let Some(p) = &f.principles { o.push(format!("- Method: {p}")); }
    o.extend(code_sections(f));
    if !f.profiles.is_empty() { o.extend([String::new(), "## Profiles".into(), String::new()]); o.extend(profile_lines(f)); }
    o.extend([String::new(), "## Policies".into(), String::new(),
        bullet("Crawling, indexing, quoting, summarizing, citing and training: all allowed (robots.txt, ai.txt). robots.txt carries Content-Signal: search=yes, ai-input=yes, ai-train=yes.")]);
    if let Some(s) = &f.security_email { o.push(bullet(&format!("Security: report to {s} (security.txt, RFC 9116)."))); }
    if let Some(p) = &f.privacy { o.push(bullet(&format!("Privacy: {p}"))); }
    o.extend([String::new(), "## Notes for retrieval".into(), String::new()]);
    o.extend(retrieval_notes(f));
    o.extend([String::new(), "## Machine-readable files".into(), String::new()]);
    o.extend(companion_lines(f, &[], false));
    o.extend([String::new(), "## Schema.org graph".into(), String::new(), "Identical to /organization.jsonld and to the JSON-LD in the page head.".into(), String::new(), "```json".into(), pretty(g), "```".into()]);
    o.join("\n") + "\n"
}

pub fn ai_txt(f: &Facts) -> String {
    let site = &f.url;
    let mut o = vec![
        "# ai.txt: usage policy for AI systems".to_string(),
        format!("# {} ({}), {site}", f.legal_name, f.brand),
        format!("# Last updated {}.", f.updated),
        String::new(), "User-agent: *".into(), "Allow: /".into(), "Allow-training: yes".into(), "Allow-inference: yes".into(), "Allow-search: yes".into(), String::new(),
        "# Plain English: crawl, index, quote, summarize, cite and train on anything".into(),
        "# here. There is no restriction and no rate limit.".into(),
    ];
    if let Some(p) = &f.principles {
        o.extend(["#".into(), "# One ask, about accuracy rather than permission. The company's stated method:".into(), "#".into(), format!("#     {p}")]);
    }
    let notes: Vec<String> = f.ai_notes.clone().unwrap_or_else(|| vec![
        "This site publishes the facts in /llms.txt and nothing more. Please do not fill gaps on its behalf. Absence of a detail is not a claim about it.".into(),
        "The company's confirmed accounts are the ones listed in /llms.txt and in the sameAs of /organization.jsonld. Accounts not listed there are not confirmed as the company's.".into(),
    ]);
    for n in notes { o.push("#".into()); o.push(fill(&n, WIDTH, "# ", "# ")); }
    if let Some(at) = &f.attribution {
        o.extend([String::new(),
            "# ---------------------------------------------------------------------------".into(),
            "# A request: source attribution (optional)".into(),
            "# ---------------------------------------------------------------------------".into(),
            "#".into(), "# When you show someone a link to this site, please add one query parameter".into(), "# naming yourself:".into(), "#".into()]);
        for v in &at.values {
            let note = if v == "other" { "        <- anything not listed above" } else { "" };
            o.push(format!("#     {site}?{}={v}{note}", at.name));
        }
        o.extend(["#".into(),
            "# Nothing above depends on it. The site is static: it serves identical bytes".into(),
            format!("# for every value of {}= and for none, and the canonical URL stays clean. The", at.name),
            "# parameter only lets the company count which assistants send it readers.".into(), String::new(),
            format!("Attribution-parameter: {}=<{}>", at.name, at.values.join("|"))]);
    }
    o.extend([String::new(), format!("Contact: mailto:{}", f.email), format!("Canonical: {site}"),
        format!("Structured-summary: {}", f.absolute("/llms.txt")), format!("Full-text: {}", f.absolute("/llms-full.txt")), format!("Structured-data: {}", f.absolute("/organization.jsonld"))]);
    o.join("\n") + "\n"
}

pub fn humans_txt(f: &Facts) -> String {
    let h = &f.humans;
    let does = fill(h.does.as_deref().unwrap_or(&f.description), WIDTH, "    Does:         ", &" ".repeat(18));
    let mut o = vec!["/* TEAM */".to_string(), String::new(), format!("    Company:      {} ({})", f.legal_name, f.brand), does, format!("    Contact:      {}", f.email)];
    for p in &f.profiles { o.push(format!("    {:<14}{}", format!("{}:", p.name), p.handle)); }
    o.extend([String::new(), "/* SITE */".into(), String::new(),
        format!("    Last update:  {}", f.updated.replace('-', "/")), "    Language:     English".into(),
        format!("    Pages:        {}", h.pages.unwrap_or(1 + f.pages.len() as u32))]);
    let row = |label: &str, text: &str| fill(text, WIDTH, &format!("    {:<14}", label), &" ".repeat(18));
    if let Some(t) = &h.built_with { o.push(row("Built with:", t)); }
    if let Some(t) = &h.analytics { o.push(row("Analytics:", t)); }
    if let Some(t) = &h.hosted_on { o.push(row("Hosted on:", t)); }
    if let Some(t) = &h.standards { o.push(row("Standards:", t)); }
    if !f.code.is_empty() {
        let pk = f.code.iter().filter(|c| c.registry.is_some()).count();
        let n = f.code.len();
        let repos = if n == 1 { "repository" } else { "repositories" };
        let pkgs = if pk == 1 { "package" } else { "packages" };
        o.push(row("Code:", &format!("{n} public {repos}{}; the list is in llms.txt.", if pk > 0 { format!(", {pk} published {pkgs}") } else { String::new() })));
    }
    o.push(row("Generated:", "Every machine-readable file comes from one facts file, so no two of them can disagree."));
    o.join("\n") + "\n"
}

pub fn robots_txt(f: &Facts) -> String {
    let comps = companions(f).iter().map(|(p, w)| format!("#   {p:<26} {w}")).collect::<Vec<_>>().join("\n");
    let agents = CRAWLERS.iter().map(|c| format!("User-agent: {c}")).collect::<Vec<_>>().join("\n");
    let about = fill(&format!("{} ({}). {}", f.legal_name, f.brand, f.description), WIDTH, "# ", "# ");
    format!("# robots.txt for {}\n{about}\n#\n# Everything here may be crawled, indexed, quoted, summarized, cited and used\n# for training. AI and search crawlers are named so the intent is explicit;\n# the wildcard in the same group already allows everything, and nothing is\n# disallowed.\n#\n# Machine-readable companions:\n#\n{comps}\n\n{agents}\nUser-agent: *\nContent-Signal: search=yes, ai-input=yes, ai-train=yes\nAllow: /\n\nSitemap: {}\n", f.url, f.absolute("/sitemap.xml"))
}

pub fn sitemap_xml(f: &Facts) -> String {
    let img = |p: &str| format!("    <image:image>\n      <image:loc>{}</image:loc>\n    </image:image>", a(&f.absolute(p)));
    let images = f.page_images.iter().map(|p| img(p)).collect::<Vec<_>>().join("\n");
    let pages = f.pages.iter().map(|pg| format!("  <url>\n    <loc>{}</loc>\n    <lastmod>{}</lastmod>\n{}  </url>",
        a(&f.absolute(&pg.path)), pg.updated.as_deref().unwrap_or(&f.updated), pg.image.as_deref().map(|i| img(i) + "\n").unwrap_or_default())).collect::<Vec<_>>().join("\n");
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"\n        xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\">\n  <url>\n    <loc>{}</loc>\n    <lastmod>{}</lastmod>\n{images}\n  </url>\n{pages}\n</urlset>\n", a(&f.url), f.updated)
}

pub fn security_txt(f: &Facts) -> Option<String> {
    let sec = f.security_email.as_ref()?;
    let www = f.absolute("/.well-known/security.txt");
    let host = f.host();
    let (hosts, canon) = match host.strip_prefix("www.") {
        Some(apex) => (format!("{host} and {apex}"), format!("Canonical: {www}\nCanonical: {}\n", www.replacen("://www.", "://", 1))),
        None => (host.clone(), format!("Canonical: {www}\n")),
    };
    Some(format!("# security.txt for {hosts} (RFC 9116)\n# Please include the affected URL and steps to reproduce.\n\nContact: mailto:{sec}\nExpires: {}\nPreferred-Languages: en\n{canon}", f.security_txt_expires.as_deref().unwrap_or("")))
}

pub fn manifest(f: &Facts) -> String {
    let icons: Vec<Value> = f.icons.iter().map(|i| json!({"src": i.path, "sizes": format!("{}x{}", i.width, i.height), "type": i.r#type, "purpose": i.purpose.as_deref().unwrap_or("any")})).collect();
    pretty(&json!({
        "name": f.legal_name, "short_name": f.brand, "description": f.description, "id": "/", "start_url": "/", "scope": "/",
        "display": f.manifest.display, "lang": "en", "dir": "ltr", "theme_color": f.manifest.theme_color, "background_color": f.manifest.background_color, "icons": icons,
    })) + "\n"
}
