//! Line wrapping with the exact behavior of Python's `textwrap.fill(width, initial_indent, subsequent_indent,
//! break_on_hyphens=False, break_long_words=False)`: greedy on whitespace chunks, long words left whole,
//! whitespace dropped at line ends and at the start of continuation lines.

pub const WIDTH: usize = 78;

fn chunks(text: &str) -> Vec<String> {
    // replace_whitespace: every whitespace char becomes a space (tabs expanded first, as Python does).
    let expanded = text.replace('\t', "        ");
    let mut out: Vec<String> = vec![];
    let mut cur = String::new();
    let mut cur_ws: Option<bool> = None;
    for c in expanded.chars() {
        let ws = c.is_whitespace();
        let c = if ws { ' ' } else { c };
        match cur_ws {
            Some(w) if w == ws => cur.push(c),
            _ => {
                if !cur.is_empty() { out.push(std::mem::take(&mut cur)); }
                cur.push(c);
                cur_ws = Some(ws);
            }
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

fn is_ws(s: &str) -> bool { s.chars().all(|c| c == ' ') }
fn len(s: &str) -> usize { s.chars().count() }

/// `textwrap.fill` with the flags above.
pub fn fill(text: &str, width: usize, first: &str, rest: &str) -> String {
    let mut chunks = chunks(text);
    chunks.reverse(); // stack: pop from the end
    let mut lines: Vec<String> = vec![];
    while !chunks.is_empty() {
        let indent = if lines.is_empty() { first } else { rest };
        let avail = width.saturating_sub(len(indent)).max(1);
        let mut cur: Vec<String> = vec![];
        let mut cur_len = 0;
        // drop_whitespace: leading whitespace on a continuation line is dropped.
        if !lines.is_empty() && chunks.last().is_some_and(|c| is_ws(c)) { chunks.pop(); }
        while let Some(c) = chunks.last() {
            let l = len(c);
            if cur_len + l <= avail { cur_len += l; cur.push(chunks.pop().unwrap()); } else { break; }
        }
        // A word longer than the line, with break_long_words=False: it goes on its own line, whole.
        if let Some(c) = chunks.last() {
            if len(c) > avail && cur_len == 0 { cur.push(chunks.pop().unwrap()); }
        }
        if cur.last().is_some_and(|c| is_ws(c)) { cur.pop(); }
        if cur.is_empty() { continue; }
        lines.push(format!("{indent}{}", cur.concat()));
    }
    lines.join("\n")
}

pub fn wrapped(text: &str, indent: &str) -> String { fill(text, WIDTH, indent, indent) }
pub fn bullet(text: &str) -> String { fill(text, WIDTH, "- ", "  ") }
pub fn quote(text: &str) -> String { fill(text, WIDTH, "> ", "> ") }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_python_textwrap() {
        // Reference outputs produced with Python 3.14 textwrap.fill(..., break_on_hyphens=False, break_long_words=False).
        assert_eq!(fill("the quick brown fox jumps over the lazy dog", 15, "", ""), "the quick brown\nfox jumps over\nthe lazy dog");
        assert_eq!(fill("the quick brown fox", 10, "- ", "  "), "- the\n  quick\n  brown\n  fox");
        assert_eq!(fill("a-very-long-hyphenated-word here", 10, "", ""), "a-very-long-hyphenated-word\nhere");
        assert_eq!(fill("two  spaces kept", 80, "", ""), "two  spaces kept");
        assert_eq!(fill("   leading", 80, "", ""), "   leading");
        assert_eq!(fill("trailing   ", 80, "", ""), "trailing");
        assert_eq!(fill("", 80, "> ", "> "), "");
        assert_eq!(fill("exactly ten", 11, "", ""), "exactly ten");
        assert_eq!(fill("exactly tenx", 11, "", ""), "exactly\ntenx");
    }
}
