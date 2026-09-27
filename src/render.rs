//! Markdown → HTML conversion for rendering.
//!
//! The HTML is an intermediate representation only — it is never persisted
//! to disk. Notes remain plain `.md` files. The HTML is sanitized to prevent
//! XSS: only a whitelist of safe elements and attributes is allowed.

use pulldown_cmark::{Options, Parser, html};

/// Convert markdown to sanitized HTML.
///
/// Supported elements: h1-h6, p, strong, em, del, code, pre, blockquote,
/// ul, ol, li, a, br, hr, img. All script, style, event handlers, and
/// dangerous URLs are stripped.
pub fn markdown_to_html(text: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(text, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    sanitize_html(&html_output)
}

/// Sanitize HTML by removing dangerous elements and attributes.
///
/// pulldown-cmark generates safe HTML by default (no scripts, no event
/// handlers). This pass is defense-in-depth: strip `<script>`/`<style>`
/// blocks, remove `javascript:`/`data:` URLs, and drop `on*` attributes.
pub fn sanitize_html(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            // Read the full tag
            let mut tag = String::from("<");
            let mut in_quotes = false;
            let mut quote_char = '"';
            let mut closed = false;

            while let Some(&next) = chars.peek() {
                chars.next();
                tag.push(next);
                if in_quotes {
                    if next == quote_char {
                        in_quotes = false;
                    }
                } else if next == '"' || next == '\'' {
                    in_quotes = true;
                    quote_char = next;
                } else if next == '>' {
                    closed = true;
                    break;
                }
            }

            if !closed {
                // Unclosed tag — skip it
                continue;
            }

            let tag_lower = tag.to_lowercase();

            // Skip script/style blocks entirely
            if tag_lower.starts_with("<script") || tag_lower.starts_with("<style") {
                let end_tag = if tag_lower.starts_with("<script") {
                    "</script>"
                } else {
                    "</style>"
                };
                let mut skip = String::new();
                while let Some(ch) = chars.next() {
                    skip.push(ch);
                    if skip.len() >= end_tag.len()
                        && skip[skip.len() - end_tag.len()..].eq_ignore_ascii_case(end_tag)
                    {
                        break;
                    }
                }
                continue;
            }

            // Parse tag name (handle closing tags)
            let is_closing = tag_lower.starts_with("</");
            let tag_inner = tag_lower
                .trim_start_matches('<')
                .trim_start_matches('/')
                .trim_end_matches('>')
                .trim_end_matches('/')
                .to_string();
            let tag_name = tag_inner
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();

            // Only allow whitelisted tags
            if !is_closing && !is_safe_tag(&tag_name) {
                continue;
            }

            // Reconstruct the tag with filtered attributes
            if is_closing {
                output.push('<');
                output.push('/');
                output.push_str(&tag_name);
                output.push('>');
            } else {
                let safe_attrs = filter_attrs(&tag_name, &tag);
                output.push('<');
                output.push_str(&tag_name);
                output.push_str(&safe_attrs);
                output.push('>');
            }
        } else {
            output.push(c);
        }
    }

    output
}

fn is_safe_tag(tag: &str) -> bool {
    matches!(
        tag,
        "h1" | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "p"
            | "strong"
            | "b"
            | "em"
            | "i"
            | "del"
            | "s"
            | "strike"
            | "code"
            | "pre"
            | "blockquote"
            | "ul"
            | "ol"
            | "li"
            | "a"
            | "br"
            | "hr"
            | "img"
            | "table"
            | "thead"
            | "tbody"
            | "tr"
            | "th"
            | "td"
            | "input"
    )
}

fn filter_attrs(tag: &str, tag_str: &str) -> String {
    let mut result = String::new();
    let inner = tag_str
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim_end_matches('/');

    let parts: Vec<&str> = inner.splitn(2, char::is_whitespace).collect();
    if parts.len() < 2 {
        return result;
    }

    let attr_str = parts[1];
    let mut in_quotes = false;
    let mut quote_char = '"';
    let mut current = String::new();
    let mut attrs: Vec<String> = Vec::new();

    for c in attr_str.chars() {
        if in_quotes {
            current.push(c);
            if c == quote_char {
                in_quotes = false;
            }
        } else if c == '"' || c == '\'' {
            in_quotes = true;
            quote_char = c;
            current.push(c);
        } else if c.is_whitespace() {
            if !current.is_empty() {
                attrs.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        attrs.push(current);
    }

    for attr in attrs {
        if let Some(safe) = sanitize_attr(tag, &attr) {
            result.push(' ');
            result.push_str(&safe);
        }
    }

    result
}

fn sanitize_attr(tag: &str, attr: &str) -> Option<String> {
    let name = attr.split('=').next()?.trim().to_lowercase();

    // Skip event handlers
    if name.starts_with("on") {
        return None;
    }

    match (tag, name.as_str()) {
        ("a", "href") => {
            let value = extract_value(attr)?;
            if is_safe_url(&value) {
                Some(format!("href=\"{}\"", html_escape(&value)))
            } else {
                None
            }
        }
        ("a", "title") => {
            let value = extract_value(attr)?;
            Some(format!("title=\"{}\"", html_escape(&value)))
        }
        ("img", "src") => {
            let value = extract_value(attr)?;
            if is_safe_url(&value) {
                Some(format!("src=\"{}\"", html_escape(&value)))
            } else {
                None
            }
        }
        ("img", "alt") => {
            let value = extract_value(attr)?;
            Some(format!("alt=\"{}\"", html_escape(&value)))
        }
        ("img", "title") => {
            let value = extract_value(attr)?;
            Some(format!("title=\"{}\"", html_escape(&value)))
        }
        ("input", "type") => {
            let value = extract_value(attr)?;
            if value == "checkbox" {
                Some("type=\"checkbox\"".to_string())
            } else {
                None
            }
        }
        ("input", "checked") => Some("checked".to_string()),
        ("input", "disabled") => Some("disabled".to_string()),
        ("ol", "start") => {
            let value = extract_value(attr)?;
            value.parse::<i64>().ok()?;
            Some(format!("start=\"{}\"", value))
        }
        _ => None,
    }
}

fn extract_value(attr: &str) -> Option<String> {
    let (_, value) = attr.split_once('=')?;
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        Some(value[1..value.len() - 1].to_string())
    } else {
        Some(value.to_string())
    }
}

fn is_safe_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    !lower.starts_with("javascript:")
        && !lower.starts_with("data:")
        && !lower.starts_with("vbscript:")
        && !lower.starts_with("file:")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings() {
        let html = markdown_to_html("# Title\n## Subtitle\n### H3");
        assert!(html.contains("<h1"));
        assert!(html.contains("Title</h1>"));
        assert!(html.contains("<h2"));
        assert!(html.contains("Subtitle</h2>"));
        assert!(html.contains("<h3"));
        assert!(html.contains("H3</h3>"));
    }

    #[test]
    fn bold_italic_strikethrough() {
        let html = markdown_to_html("**bold** *italic* ~~strike~~ __underline__");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
        assert!(html.contains("<del>strike</del>"));
        assert!(html.contains("<strong>underline</strong>"));
    }

    #[test]
    fn lists() {
        let html = markdown_to_html("- item 1\n- item 2\n\n1. first\n2. second");
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>item 1</li>"));
        assert!(html.contains("<li>item 2</li>"));
        assert!(html.contains("<ol>"));
        assert!(html.contains("<li>first</li>"));
    }

    #[test]
    fn task_lists() {
        let html = markdown_to_html("- [ ] todo\n- [x] done");
        assert!(html.contains("<input"));
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("checked"));
    }

    #[test]
    fn links() {
        let html = markdown_to_html("[text](https://example.com)");
        assert!(html.contains("<a"));
        assert!(html.contains("href=\"https://example.com\""));
        assert!(html.contains("text</a>"));
    }

    #[test]
    fn inline_code() {
        let html = markdown_to_html("use `code` here");
        assert!(html.contains("<code>code</code>"));
    }

    #[test]
    fn fenced_code() {
        let html = markdown_to_html("```rust\nfn main() {}\n```");
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn blockquotes() {
        let html = markdown_to_html("> quoted text");
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("quoted text"));
    }

    #[test]
    fn special_characters() {
        let html = markdown_to_html("a < b & c > d \"e\" 'f'");
        assert!(html.contains("&lt;"));
        assert!(html.contains("&amp;"));
        assert!(html.contains("&gt;"));
    }

    #[test]
    fn wiki_links() {
        let html = markdown_to_html("see [[Note]] and [[Other|alias]]");
        assert!(html.contains("[[Note]]") || html.contains("Note"));
    }

    #[test]
    fn empty_document() {
        let html = markdown_to_html("");
        assert!(html.is_empty() || html.trim().is_empty());
    }

    #[test]
    fn large_document() {
        let mut md = String::new();
        for i in 0..1000 {
            md.push_str(&format!(
                "# Heading {}\n\nParagraph with **bold** and *italic*.\n\n",
                i
            ));
        }
        let html = markdown_to_html(&md);
        assert!(html.contains("<h1"));
        assert!(html.contains("Heading 0</h1>"));
        assert!(html.contains("Heading 999</h1>"));
    }

    #[test]
    fn invalid_or_incomplete_markdown() {
        let html = markdown_to_html("# Unclosed heading\n**bold without close");
        assert!(html.contains("<h1"));
        assert!(html.contains("Unclosed heading"));
    }

    #[test]
    fn xss_prevention() {
        let html = markdown_to_html("[click](javascript:alert('xss'))");
        assert!(!html.contains("javascript:"));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn xss_script_tag() {
        let html = markdown_to_html("<script>alert('xss')</script>");
        assert!(!html.contains("<script"));
        assert!(!html.contains("alert"));
    }
}
