//! HTML normalization for fixture diffs.
//!
//! The agent must not weaken this function to make a diff pass. New
//! per-fixture exceptions belong in `meta.toml` as `allowed_diff` regexes,
//! each justified in `AGENT_LOG.md`.

use regex::Regex;

/// Normalize HTML so generated noise (random ids, attribute order, entity
/// spelling, whitespace) does not pollute byte-level diffs against gt's
/// reference output.
pub fn normalize_html(input: &str) -> String {
    let mut s = input.to_string();

    // 1a. Strip id="abc..." on any root <div id="...">.
    s = Regex::new(r#"(<div\b[^>]*?)\s+id="[^"]*""#)
        .unwrap()
        .replace_all(&s, "$1")
        .to_string();

    // 1b. Strip id="abc..." on any root <table id="...">.
    s = Regex::new(r#"(<table\b[^>]*?)\s+id="[^"]*""#)
        .unwrap()
        .replace_all(&s, "$1")
        .to_string();

    // 2. Canonicalize gt's randomized table-class suffix.
    s = Regex::new(r"gt_table_[a-z0-9]+")
        .unwrap()
        .replace_all(&s, "gt_table_X")
        .to_string();

    // 3. Collapse whitespace inside tag attribute regions to a single space.
    //    (Simple approximation: collapse runs of whitespace within `<...>`.)
    s = collapse_tag_whitespace(&s);

    // 4. Strip all whitespace between tags so inputs with and without
    //    pretty-printing converge.
    s = Regex::new(r">[ \t\r\n]+<")
        .unwrap()
        .replace_all(&s, "><")
        .to_string();

    // 5. Sort attributes alphabetically within each tag.
    s = sort_attributes(&s);

    // 6. Canonicalize the most common HTML entities for &, <, >.
    s = s
        .replace("&#x26;", "&amp;")
        .replace("&#38;", "&amp;")
        .replace("&#x3C;", "&lt;")
        .replace("&#60;", "&lt;")
        .replace("&#x3E;", "&gt;")
        .replace("&#62;", "&gt;");

    s.trim().to_string()
}

fn collapse_tag_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    let ws = Regex::new(r"[ \t\r\n]+").unwrap();
    let mut buf = String::new();
    for ch in input.chars() {
        if ch == '<' {
            in_tag = true;
            out.push(ch);
        } else if ch == '>' {
            if in_tag {
                let collapsed = ws.replace_all(buf.trim(), " ").to_string();
                out.push_str(&collapsed);
                buf.clear();
            }
            in_tag = false;
            out.push(ch);
        } else if in_tag {
            buf.push(ch);
        } else {
            out.push(ch);
        }
    }
    out
}

fn sort_attributes(input: &str) -> String {
    let tag_re = Regex::new(r"<([A-Za-z][A-Za-z0-9]*)((?:\s+[^>]*?)?)(/?)>").unwrap();
    let attr_re = Regex::new(r#"([A-Za-z_:][-A-Za-z0-9_:.]*)(?:=("[^"]*"|'[^']*'))?"#).unwrap();
    tag_re
        .replace_all(input, |c: &regex::Captures| {
            let name = &c[1];
            let body = c.get(2).map(|m| m.as_str()).unwrap_or("");
            let slash = &c[3];
            if body.trim().is_empty() {
                return format!("<{}{}>", name, slash);
            }
            let mut attrs: Vec<String> = attr_re
                .captures_iter(body)
                .map(|ac| {
                    let k = ac.get(1).map(|m| m.as_str()).unwrap_or("");
                    let v = ac.get(2).map(|m| m.as_str()).unwrap_or("");
                    if v.is_empty() {
                        k.to_string()
                    } else {
                        format!("{}={}", k, v)
                    }
                })
                .collect();
            attrs.sort();
            format!("<{} {}{}>", name, attrs.join(" "), slash)
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_random_table_id_and_class() {
        let a = r#"<table id="ovz" class="gt_table_abc123">a</table>"#;
        let b = r#"<table  class="gt_table_zzz999"   id="other">a</table>"#;
        assert_eq!(normalize_html(a), normalize_html(b));
    }

    #[test]
    fn attribute_order_does_not_matter() {
        let a = r#"<td class="gt_row" style="x">v</td>"#;
        let b = r#"<td style="x" class="gt_row">v</td>"#;
        assert_eq!(normalize_html(a), normalize_html(b));
    }

    #[test]
    fn between_tag_whitespace_collapses() {
        let a = "<div>\n\n   <p>x</p>\n</div>";
        let b = "<div><p>x</p></div>";
        assert_eq!(normalize_html(a), normalize_html(b));
    }

    #[test]
    fn entities_canonicalize() {
        let a = "<p>1 &#x26; 2</p>";
        let b = "<p>1 &amp; 2</p>";
        assert_eq!(normalize_html(a), normalize_html(b));
    }
}
