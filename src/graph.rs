//! The schema.org graph: organization, website, home page, og image, then one WebSite per listed site and one
//! SoftwareSourceCode per listed repository. Written inline in the page head and as
//! /organization.jsonld, identical. Key order is fixed so the output is stable.

use crate::facts::Facts;
use serde_json::{json, Map, Value};

pub fn graph(f: &Facts, title: &str, text: &[String]) -> Value {
    let url = f.url.as_str();
    let (org, website, page) = (format!("{url}#organization"), format!("{url}#website"), format!("{url}#webpage"));
    let mut o = Map::new();
    o.insert("@type".into(), json!(f.org_type));
    o.insert("@id".into(), json!(org));
    o.insert("name".into(), json!(f.legal_name));
    o.insert("legalName".into(), json!(f.legal_name));
    o.insert("alternateName".into(), json!(f.brand));
    o.insert("url".into(), json!(url));
    o.insert("description".into(), json!(f.description));
    o.insert("logo".into(), json!({
        "@type": "ImageObject", "@id": format!("{url}#logo"),
        "url": f.absolute(&f.logo.path), "contentUrl": f.absolute(&f.logo.path),
        "width": f.logo.width, "height": f.logo.height, "caption": f.logo.caption.clone().unwrap_or_default(),
    }));
    o.insert("image".into(), json!({"@id": format!("{url}#og-image")}));
    o.insert("email".into(), json!(f.email));
    if let Some(sec) = &f.security_email {
        o.insert("contactPoint".into(), json!({
            "@type": "ContactPoint", "contactType": "security", "email": sec,
            "url": f.absolute("/.well-known/security.txt"), "availableLanguage": "en",
        }));
    }
    if let Some(d) = &f.duns {
        o.insert("duns".into(), json!(d));
        o.insert("iso6523Code".into(), json!(format!("0060:{d}")));
        o.insert("identifier".into(), json!({"@type": "PropertyValue", "propertyID": "DUNS", "value": d}));
    }
    if !f.knows_about.is_empty() {
        o.insert("knowsAbout".into(), Value::Array(f.knows_about.iter().map(|t| {
            let mut n = Map::new();
            n.insert("@type".into(), json!("Thing"));
            n.insert("name".into(), json!(t.name));
            if let Some(q) = &t.wikidata { n.insert("sameAs".into(), json!(format!("https://www.wikidata.org/wiki/{q}"))); }
            Value::Object(n)
        }).collect()));
    }
    if let Some(p) = &f.principles {
        o.insert("publishingPrinciples".into(), json!({"@type": "CreativeWork", "name": "Method", "text": p}));
    }
    if !f.profiles.is_empty() {
        o.insert("sameAs".into(), json!(f.profiles.iter().map(|p| p.url.clone()).collect::<Vec<_>>()));
    }
    let og = &f.og_image;
    let mut g = json!({
        "@context": "https://schema.org",
        "@graph": [
            Value::Object(o),
            {
                "@type": "WebSite", "@id": website, "url": url, "name": f.legal_name,
                "alternateName": [f.brand, f.host().trim_start_matches("www.")],
                "description": f.description, "inLanguage": "en", "publisher": {"@id": org},
            },
            {
                "@type": ["WebPage", "AboutPage"], "@id": page, "url": url, "name": title,
                "description": f.description, "text": text.join(" "), "inLanguage": "en",
                "isPartOf": {"@id": website}, "about": {"@id": org}, "mainEntity": {"@id": org},
                "primaryImageOfPage": {"@id": format!("{url}#og-image")}, "dateModified": f.updated,
            },
            {
                "@type": "ImageObject", "@id": format!("{url}#og-image"),
                "url": f.absolute(&og.path), "contentUrl": f.absolute(&og.path),
                "width": og.width, "height": og.height, "caption": og.alt.clone().unwrap_or_default(),
                "encodingFormat": mime_of(&og.path),
            }
        ]
    });
    let nodes = g["@graph"].as_array_mut().expect("graph array");
    for st in &f.sites {
        let mut n = Map::new();
        n.insert("@type".into(), json!("WebSite"));
        n.insert("@id".into(), json!(st.url));
        n.insert("url".into(), json!(st.url));
        n.insert("name".into(), json!(st.name));
        n.insert("description".into(), json!(st.summary));
        n.insert("publisher".into(), json!({"@id": org}));
        nodes.push(Value::Object(n));
    }
    for c in &f.code {
        let mut n = Map::new();
        n.insert("@type".into(), json!("SoftwareSourceCode"));
        n.insert("@id".into(), json!(c.url));
        n.insert("name".into(), json!(c.name));
        n.insert("url".into(), json!(c.url));
        n.insert("codeRepository".into(), json!(c.url));
        if !c.description.is_empty() { n.insert("description".into(), json!(c.description)); }
        if let Some(l) = &c.language { n.insert("programmingLanguage".into(), json!(l)); }
        if let Some(l) = &c.license { n.insert("license".into(), json!(format!("https://spdx.org/licenses/{l}"))); }
        if let Some(v) = &c.version { n.insert("version".into(), json!(v)); }
        let same: Vec<&String> = c.registry.iter().chain(c.docs.iter()).chain(c.homepage.iter()).collect();
        if !same.is_empty() { n.insert("sameAs".into(), json!(same)); }
        if let Some(d) = &c.updated { n.insert("dateModified".into(), json!(d)); }
        n.insert(if c.fork { "maintainer" } else { "author" }.into(), json!({"@id": org}));
        nodes.push(Value::Object(n));
    }
    g
}

pub fn mime_of(path: &str) -> &'static str {
    match path.rsplit('.').next().map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("png") => "image/png", Some("webp") => "image/webp", Some("jpg") | Some("jpeg") => "image/jpeg", Some("svg") => "image/svg+xml", Some("gif") => "image/gif", _ => "application/octet-stream",
    }
}

/// `json.dumps(indent=2, ensure_ascii=False)`.
pub fn pretty(v: &Value) -> String { serde_json::to_string_pretty(v).unwrap_or_default() }

/// For the inline `<script>`: `</` can never end the element early if it is never written.
pub fn pretty_inline(v: &Value) -> String { pretty(v).replace("</", "<\\/") }
