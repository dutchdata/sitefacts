//! End to end: an example facts file and a tiny site → every output, then `check` is clean, then a facts edit
//! makes exactly the right files stale.
use std::fs;
use std::path::Path;

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut d = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
    d.extend_from_slice(&w.to_be_bytes());
    d.extend_from_slice(&h.to_be_bytes());
    d.extend_from_slice(&[8, 6, 0, 0, 0]);
    d
}

fn facts_json() -> String {
    r#"{
  "updated": "2026-10-15", "url": "https://www.example.com/", "legal_name": "EXAMPLE, INC.", "brand": "Example",
  "kind": "studio", "org_type": "Organization", "description": "Makes small tools. Exact, checked, plain.",
  "principles": "Specific, checkable claims.", "email": "hello@example.com",
  "security_email": "security@example.com", "security_txt_expires": "2099-01-01T00:00:00Z",
  "profiles": [{"name": "Bluesky", "url": "https://bsky.app/profile/example.com", "handle": "@example.com"}],
  "twitter_site": "@example", "knows_about": [{"name": "Software", "wikidata": "Q7397"}, {"name": "Tools"}],
  "logo": {"path": "/logo.png", "width": 64, "height": 64, "caption": "Example logo"},
  "og_image": {"path": "/og.png", "width": 1200, "height": 630, "alt": "Example"},
  "icons": [{"path": "/logo.png", "width": 64, "height": 64, "type": "image/png"}],
  "page_images": ["/logo.png"], "indexnow_key": "abc123",
  "pages": [{"path": "/tool/", "title": "Tool", "summary": "A tool.", "image": "/og.png"}],
  "attribution": {"name": "ai", "values": ["openai", "other"]},
  "humans": {"built_with": "One page.", "hosted_on": "A bucket."},
  "_doc": "ignored"
}"#.to_string()
}

const INDEX: &str = "<!doctype html>\n<html><head>\n<title>Example &amp; Co</title>\n<style>a > b {}</style>\n<!-- machine:begin -->\nold\n<!-- machine:end -->\n<script>if (1 < 2) {}</script>\n</head>\n<body>\n<h1>Example</h1>\n<p>We make small tools.</p>\n<p class=\"apps\"><a href=\"/tool/\">Tool →</a></p>\n<ul><li>One</li></ul>\n</body></html>\n";

fn setup(root: &Path) {
    let site = root.join("site");
    fs::create_dir_all(site.join("tool")).unwrap();
    fs::write(root.join("facts.json"), facts_json()).unwrap();
    fs::write(site.join("index.html"), INDEX).unwrap();
    fs::write(site.join("logo.png"), png(64, 64)).unwrap();
    fs::write(site.join("og.png"), png(1200, 630)).unwrap();
    fs::write(site.join("tool/index.html"), "<html><title>Tool</title><body><p>x</p></body></html>").unwrap();
}

#[test]
fn builds_everything_then_check_is_clean() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    setup(root);
    let (facts, site) = (root.join("facts.json"), root.join("site"));
    let r = sitefacts::run(&facts, &site, false).unwrap();
    assert_eq!(r.total, 12);
    assert_eq!(r.written.len(), 12);
    for f in ["index.html", "organization.jsonld", "llms.txt", "llms-full.txt", "ai.txt", "humans.txt", "robots.txt", "sitemap.xml", ".well-known/security.txt", "site.webmanifest", "manifest.json", "abc123.txt"] {
        assert!(site.join(f).is_file(), "{f} missing");
    }
    let r2 = sitefacts::run(&facts, &site, true).unwrap();
    assert!(r2.stale.is_empty(), "check after build: {:?}", r2.stale);

    let index = fs::read_to_string(site.join("index.html")).unwrap();
    assert!(index.contains("<!-- machine:begin: written by sitefacts"), "marker rewritten");
    assert!(!index.contains("\nold\n"), "old block replaced");
    assert!(index.contains(r#"<meta property="og:title" content="Example &amp; Co">"#));
    assert!(index.contains(r#"<link rel="me" href="https://bsky.app/profile/example.com">"#));
    assert!(index.ends_with("</body></html>\n"), "rest of the page untouched");

    let llms = fs::read_to_string(site.join("llms.txt")).unwrap();
    assert!(llms.contains("    Example\n    We make small tools.\n    One\n"), "page text, nav skipped:\n{llms}");
    assert!(!llms.contains("Tool →"));
    assert!(llms.contains("- [Tool](https://www.example.com/tool/): A tool."));
    assert!(llms.contains("?ai=openai, ?ai=other"));
    assert!(llms.contains("EXAMPLE, INC. (Example) is a studio."));

    let graph: serde_json::Value = serde_json::from_str(&fs::read_to_string(site.join("organization.jsonld")).unwrap()).unwrap();
    let org = &graph["@graph"][0];
    assert_eq!(org["@type"], "Organization");
    assert_eq!(org["knowsAbout"][0]["sameAs"], "https://www.wikidata.org/wiki/Q7397");
    assert!(org.get("duns").is_none(), "no duns → no duns fields");
    assert_eq!(graph["@graph"][2]["text"], "Example We make small tools. One");
    let inline_start = index.find(r#"<script type="application/ld+json">"#).unwrap();
    assert!(index[inline_start..].contains("\"@type\": \"Organization\""));

    let sitemap = fs::read_to_string(site.join("sitemap.xml")).unwrap();
    assert!(sitemap.contains("<loc>https://www.example.com/tool/</loc>") && sitemap.contains("<image:loc>https://www.example.com/og.png</image:loc>"));
    let robots = fs::read_to_string(site.join("robots.txt")).unwrap();
    assert!(robots.contains("User-agent: GPTBot") && robots.contains("Sitemap: https://www.example.com/sitemap.xml"));
    let sec = fs::read_to_string(site.join(".well-known/security.txt")).unwrap();
    assert!(sec.contains("Canonical: https://www.example.com/.well-known/security.txt\nCanonical: https://example.com/.well-known/security.txt"));
    let humans = fs::read_to_string(site.join("humans.txt")).unwrap();
    assert!(humans.contains("    Pages:        2") && humans.contains("    Bluesky:      @example.com") && humans.contains("Hosted on:    A bucket."));
    let ai = fs::read_to_string(site.join("ai.txt")).unwrap();
    assert!(ai.contains("#     https://www.example.com/?ai=other        <- anything not listed above") && ai.contains("Attribution-parameter: ai=<openai|other>"));

    // A facts edit makes the right files stale, and only those.
    let edited = facts_json().replace("\"handle\": \"@example.com\"", "\"handle\": \"@ex\"");
    fs::write(&facts, edited).unwrap();
    let r3 = sitefacts::run(&facts, &site, true).unwrap();
    assert_eq!(r3.stale, ["humans.txt", "llms-full.txt", "llms.txt"]);
}

#[test]
fn input_problems_are_reported_not_panicked() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    setup(root);
    fs::write(root.join("site/og.png"), png(100, 100)).unwrap();
    let err = sitefacts::run(&root.join("facts.json"), &root.join("site"), true).unwrap_err();
    assert!(err.contains("/og.png is 100x100, facts say 1200x630"), "{err}");
    fs::remove_file(root.join("site/logo.png")).unwrap();
    let err = sitefacts::run(&root.join("facts.json"), &root.join("site"), true).unwrap_err();
    assert!(err.contains("missing /logo.png"), "{err}");
    fs::write(root.join("site/index.html"), "<html><body></body></html>").unwrap();
    fs::write(root.join("site/logo.png"), png(64, 64)).unwrap();
    fs::write(root.join("site/og.png"), png(1200, 630)).unwrap();
    let err = sitefacts::run(&root.join("facts.json"), &root.join("site"), true).unwrap_err();
    assert!(err.contains("no <title>"), "{err}");
}
