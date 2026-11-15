//! Just enough XML for the game's settings files: the start tags in document
//! order with their attributes, comments left out. No text content, no
//! entities.

/// A start tag (or an end tag, whose `tag` starts with `/`).
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub tag: String,
    pub attributes: Vec<(String, String)>,
}

impl Element {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Every tag of `text` in order, comments, the XML declaration and other
/// `<?` / `<!` constructs left out.
pub fn elements(text: &str) -> Vec<Element> {
    let text = strip_comments(text);
    let mut found = Vec::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        if rest.starts_with(['?', '!']) {
            continue;
        }
        let end = rest.find('>').unwrap_or(rest.len());
        let inside = &rest[..end];
        rest = &rest[end..];
        let name_end = inside
            .find(|c: char| c.is_whitespace() || c == '/' && !inside.starts_with('/'))
            .unwrap_or(inside.len());
        let tag = inside[..name_end].trim().to_owned();
        if tag.is_empty() {
            continue;
        }
        found.push(Element {
            tag,
            attributes: attributes(&inside[name_end..]),
        });
    }
    found
}

/// The text without `<!-- ... -->` comments (an unclosed one runs to the
/// end).
pub fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// `key = "value"` pairs of an element's tag text; keys are kept as written.
pub fn attributes(tag: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = tag;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        let Some(eq) = rest.find('=') else { break };
        let key = rest[..eq].trim().to_owned();
        let after = rest[eq + 1..].trim_start();
        let Some(quoted) = after.strip_prefix('"') else {
            break;
        };
        let Some(close) = quoted.find('"') else { break };
        found.push((key, quoted[..close].to_owned()));
        rest = &quoted[close + 1..];
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: FMT-4.2
    #[test]
    fn tags_and_attributes_in_order() {
        let found = elements(
            r#"<?xml version="1.0" ?>
<A x = "1"><!-- <B gone="1"/> --><B y="two words"
   z="3"/></A>"#,
        );
        let tags: Vec<&str> = found.iter().map(|e| e.tag.as_str()).collect();
        assert_eq!(tags, ["A", "B", "/A"]);
        assert_eq!(found[0].get("x"), Some("1"));
        assert_eq!(found[1].get("y"), Some("two words"));
        assert_eq!(found[1].get("z"), Some("3"));
        assert_eq!(found[1].get("gone"), None);
    }
}
