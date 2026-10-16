//! Just enough HTML: escape attributes, unescape titles, and collect the readable text of a page's body
//! (headings, paragraphs, list items; scripts, styles, buttons and elements marked as navigation skipped).

/// `html.escape(s, quote=True)`: `& < > " '`.
pub fn escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#x27;"),
            _ => o.push(c),
        }
    }
    o
}

/// Named and numeric entities most pages use; unknown ones are left as written.
pub fn unescape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        o.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest.find(';').filter(|&e| e <= 12);
        match end {
            Some(e) => {
                let ent = &rest[1..e];
                let rep = match ent {
                    "amp" => Some("&".to_string()), "lt" => Some("<".into()), "gt" => Some(">".into()), "quot" => Some("\"".into()),
                    "apos" => Some("'".into()), "nbsp" => Some("\u{a0}".into()), "mdash" => Some("\u{2014}".into()), "ndash" => Some("\u{2013}".into()),
                    "hellip" => Some("\u{2026}".into()), "middot" => Some("\u{b7}".into()), "copy" => Some("\u{a9}".into()), "rsquo" => Some("\u{2019}".into()),
                    "lsquo" => Some("\u{2018}".into()), "rdquo" => Some("\u{201d}".into()), "ldquo" => Some("\u{201c}".into()),
                    _ if ent.starts_with("#x") || ent.starts_with("#X") => u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32).map(String::from),
                    _ if ent.starts_with('#') => ent[1..].parse::<u32>().ok().and_then(char::from_u32).map(String::from),
                    _ => None,
                };
                match rep {
                    Some(r) => { o.push_str(&r); rest = &rest[e + 1..]; }
                    None => { o.push('&'); rest = &rest[1..]; }
                }
            }
            None => { o.push('&'); rest = &rest[1..]; }
        }
    }
    o.push_str(rest);
    o
}

/// The `<title>` text, unescaped.
pub fn page_title(doc: &str) -> Option<String> {
    let s = doc.find("<title>")? + 7;
    let e = doc[s..].find("</title>")? + s;
    Some(unescape(doc[s..e].trim()))
}

const BLOCKS: &[&str] = &["p", "h1", "h2", "h3", "h4", "h5", "h6", "li"];
const SKIP: &[&str] = &["script", "style", "button", "template", "noscript"];

/// Readable text blocks of `<body>`, whitespace collapsed. Block elements carrying `skip_class` (default
/// `apps`, the site's navigation line) are skipped. Mirrors the Python `HTMLParser` collector the files were born with.
pub fn page_text(doc: &str, skip_class: &str) -> Vec<String> {
    let mut blocks = vec![];
    let mut in_body = false;
    let mut skip = 0usize;
    let mut nav_tag: Option<String> = None;
    let mut block: Option<String> = None;
    let mut i = 0;
    let b = doc.as_bytes();
    while i < b.len() {
        if b[i] == b'<' {
            if doc[i..].starts_with("<!--") {
                i = doc[i..].find("-->").map(|e| i + e + 3).unwrap_or(b.len());
                continue;
            }
            let Some(e) = doc[i..].find('>') else { break };
            let raw = &doc[i + 1..i + e];
            i += e + 1;
            if raw.starts_with('!') || raw.starts_with('?') { continue; }
            let closing = raw.starts_with('/');
            let inner = raw.trim_start_matches('/');
            let name_end = inner.find(|c: char| c.is_whitespace() || c == '/').unwrap_or(inner.len());
            let name = inner[..name_end].to_ascii_lowercase();
            let attrs = &inner[name_end..];
            if !closing {
                if name == "body" { in_body = true; }
                else if name == "script" || name == "style" {
                    // Raw text elements: their content is not markup, so jump to the matching close tag.
                    let lower = doc[i..].to_ascii_lowercase();
                    let close = format!("</{name}");
                    match lower.find(&close) {
                        Some(e) => { i += e; i += doc[i..].find('>').map(|x| x + 1).unwrap_or(0); }
                        None => break,
                    }
                }
                else if SKIP.contains(&name.as_str()) { skip += 1; }
                else if in_body && skip == 0 && BLOCKS.contains(&name.as_str()) {
                    if has_class(attrs, skip_class) { skip += 1; nav_tag = Some(name.clone()); continue; }
                    block = Some(String::new());
                }
            } else {
                if nav_tag.as_deref() == Some(name.as_str()) { nav_tag = None; skip = skip.saturating_sub(1); continue; }
                if SKIP.contains(&name.as_str()) { skip = skip.saturating_sub(1); }
                else if BLOCKS.contains(&name.as_str()) {
                    if let Some(t) = block.take() {
                        let text = t.split_whitespace().collect::<Vec<_>>().join(" ");
                        if !text.is_empty() { blocks.push(text); }
                    }
                }
            }
        } else {
            let e = doc[i..].find('<').map(|e| i + e).unwrap_or(b.len());
            if let Some(t) = block.as_mut() { if skip == 0 { t.push_str(&unescape(&doc[i..e])); } }
            i = e;
        }
    }
    blocks
}

fn has_class(attrs: &str, class: &str) -> bool {
    let a = attrs.to_ascii_lowercase();
    let Some(p) = a.find("class=") else { return false };
    let v = &attrs[p + 6..];
    let v = v.trim_start_matches(['"', '\'']);
    let end = v.find(['"', '\'']).unwrap_or(v.len());
    v[..end].split_whitespace().any(|c| c == class)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_blocks() {
        let doc = r#"<html><head><title>T &amp; U</title><style>a > b {} p{}</style><script>if (a < b) {}</script></head><body><h1>Hello  <b>world</b></h1>
        <p class="apps"><a href="/apps/">Apps</a></p><p>One &mdash; two</p><script>var x="<p>no</p>"</script><ul><li>a</li><li></li></ul><button>Theme</button></body></html>"#;
        assert_eq!(page_title(doc).as_deref(), Some("T & U"));
        assert_eq!(page_text(doc, "apps"), ["Hello world", "One \u{2014} two", "a"]);
        assert_eq!(escape("a<b>&\"c'"), "a&lt;b&gt;&amp;&quot;c&#x27;");
        assert_eq!(unescape("&#x27;&#65;&unknown;&amp"), "'A&unknown;&amp");
    }
}
