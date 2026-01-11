use clap::Parser;
use scraper::{Html, Node};
use similar::{ChangeTag, TextDiff};
use std::fs;

#[derive(Parser)]
#[command(name = "html-diff")]
#[command(
    about = "Semantic HTML diff - compares DOM structure, ignoring whitespace and entity encoding"
)]
struct Args {
    /// First HTML file
    file1: String,
    /// Second HTML file
    file2: String,
    /// Show context lines around differences
    #[arg(short = 'C', long, default_value = "3")]
    context: usize,
    /// Ignore specific CSS selectors (can be repeated)
    #[arg(short, long)]
    ignore: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let html1 = fs::read_to_string(&args.file1).expect("Failed to read file1");
    let html2 = fs::read_to_string(&args.file2).expect("Failed to read file2");

    let canon1 = canonicalize(&html1, &args.ignore);
    let canon2 = canonicalize(&html2, &args.ignore);

    let diff = TextDiff::from_lines(&canon1, &canon2);
    let mut has_diff = false;

    for (idx, group) in diff.grouped_ops(args.context).iter().enumerate() {
        if idx > 0 {
            println!("{}", "─".repeat(60));
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let (tag, color) = match change.tag() {
                    ChangeTag::Delete => {
                        has_diff = true;
                        ("-", "\x1b[31m")
                    }
                    ChangeTag::Insert => {
                        has_diff = true;
                        ("+", "\x1b[32m")
                    }
                    ChangeTag::Equal => (" ", ""),
                };
                let reset = if color.is_empty() { "" } else { "\x1b[0m" };
                print!("{}{}{} {}", color, tag, reset, change.value());
            }
        }
    }

    if !has_diff {
        println!("No differences found.");
    }

    std::process::exit(if has_diff { 1 } else { 0 });
}

/// Canonicalize HTML to a normalized form for comparison
fn canonicalize(html: &str, ignore_selectors: &[String]) -> String {
    let doc = Html::parse_document(html);
    let mut lines = Vec::new();

    canonicalize_node(doc.root_element(), 0, &mut lines, ignore_selectors, &doc);

    lines.join("\n")
}

fn canonicalize_node(
    node: scraper::ElementRef,
    depth: usize,
    lines: &mut Vec<String>,
    ignore_selectors: &[String],
    doc: &Html,
) {
    let indent = "  ".repeat(depth);
    let tag = node.value().name();

    // Check if this element matches any ignore selector
    for selector_str in ignore_selectors {
        if let Ok(selector) = scraper::Selector::parse(selector_str) {
            if doc.select(&selector).any(|el| el == node) {
                return;
            }
        }
    }

    // Build opening tag with sorted attributes
    let mut attrs: Vec<_> = node.value().attrs().collect();
    attrs.sort_by(|a, b| a.0.cmp(b.0));

    let attr_str = attrs
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", k, normalize_attr_value(v)))
        .collect::<Vec<_>>()
        .join(" ");

    if attr_str.is_empty() {
        lines.push(format!("{}<{}>", indent, tag));
    } else {
        lines.push(format!("{}<{} {}>", indent, tag, attr_str));
    }

    // Process children
    for child in node.children() {
        match child.value() {
            Node::Element(_) => {
                if let Some(el) = scraper::ElementRef::wrap(child) {
                    canonicalize_node(el, depth + 1, lines, ignore_selectors, doc);
                }
            }
            Node::Text(text) => {
                let normalized = normalize_text(text);
                if !normalized.is_empty() {
                    lines.push(format!("{}  {}", indent, normalized));
                }
            }
            _ => {}
        }
    }

    // Closing tag (skip for void elements)
    let void_elements = [
        "br", "hr", "img", "input", "link", "meta", "area", "base", "col", "embed", "param",
        "source", "track", "wbr",
    ];
    if !void_elements.contains(&tag) {
        lines.push(format!("{}</{}>", indent, tag));
    }
}

/// Normalize text content - decode entities, collapse whitespace
fn normalize_text(text: &str) -> String {
    let decoded = html_decode(text);
    // Collapse whitespace
    let normalized: String = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized
}

/// Normalize attribute value - decode entities
fn normalize_attr_value(value: &str) -> String {
    html_decode(value)
}

/// Decode HTML entities to their actual characters
fn html_decode(s: &str) -> String {
    let mut result = s.to_string();

    // Named entities
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&quot;", "\"");
    result = result.replace("&apos;", "'");
    result = result.replace("&nbsp;", " ");

    // Numeric entities (decimal)
    let re_decimal = regex::Regex::new(r"&#(\d+);").unwrap();
    result = re_decimal
        .replace_all(&result, |caps: &regex::Captures| {
            let num: u32 = caps[1].parse().unwrap_or(0);
            char::from_u32(num)
                .map(|c| c.to_string())
                .unwrap_or_default()
        })
        .to_string();

    // Numeric entities (hex)
    let re_hex = regex::Regex::new(r"&#[xX]([0-9a-fA-F]+);").unwrap();
    result = re_hex
        .replace_all(&result, |caps: &regex::Captures| {
            let num = u32::from_str_radix(&caps[1], 16).unwrap_or(0);
            char::from_u32(num)
                .map(|c| c.to_string())
                .unwrap_or_default()
        })
        .to_string();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────
    // html_decode tests
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn decode_named_entities() {
        assert_eq!(html_decode("&amp;"), "&");
        assert_eq!(html_decode("&lt;"), "<");
        assert_eq!(html_decode("&gt;"), ">");
        assert_eq!(html_decode("&quot;"), "\"");
        assert_eq!(html_decode("&apos;"), "'");
        assert_eq!(html_decode("&nbsp;"), " ");
    }

    #[test]
    fn decode_decimal_entities() {
        assert_eq!(html_decode("&#39;"), "'");
        assert_eq!(html_decode("&#039;"), "'");
        assert_eq!(html_decode("&#60;"), "<");
        assert_eq!(html_decode("&#62;"), ">");
        assert_eq!(html_decode("&#34;"), "\"");
    }

    #[test]
    fn decode_hex_entities() {
        assert_eq!(html_decode("&#x27;"), "'");
        assert_eq!(html_decode("&#X27;"), "'");
        assert_eq!(html_decode("&#x3c;"), "<");
        assert_eq!(html_decode("&#x3C;"), "<");
        assert_eq!(html_decode("&#x3E;"), ">");
    }

    #[test]
    fn decode_mixed_entities() {
        assert_eq!(
            html_decode("It&#039;s &amp; it&#x27;s the same"),
            "It's & it's the same"
        );
    }

    #[test]
    fn decode_unicode_entities() {
        assert_eq!(html_decode("&#8212;"), "—"); // em dash
        assert_eq!(html_decode("&#x2014;"), "—");
        assert_eq!(html_decode("&#169;"), "©"); // copyright
        assert_eq!(html_decode("&#x00A9;"), "©");
    }

    #[test]
    fn decode_preserves_plain_text() {
        assert_eq!(html_decode("hello world"), "hello world");
        assert_eq!(html_decode("no entities here"), "no entities here");
    }

    // ─────────────────────────────────────────────────────────────
    // normalize_text tests
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn normalize_collapses_spaces() {
        assert_eq!(normalize_text("hello   world"), "hello world");
        assert_eq!(normalize_text("  leading"), "leading");
        assert_eq!(normalize_text("trailing  "), "trailing");
        assert_eq!(normalize_text("  both  "), "both");
    }

    #[test]
    fn normalize_collapses_newlines() {
        assert_eq!(normalize_text("hello\nworld"), "hello world");
        assert_eq!(normalize_text("hello\n\n\nworld"), "hello world");
        assert_eq!(normalize_text("hello\r\nworld"), "hello world");
    }

    #[test]
    fn normalize_collapses_tabs() {
        assert_eq!(normalize_text("hello\tworld"), "hello world");
        assert_eq!(normalize_text("hello\t\t\tworld"), "hello world");
    }

    #[test]
    fn normalize_mixed_whitespace() {
        assert_eq!(normalize_text("  hello \n\t world  "), "hello world");
    }

    #[test]
    fn normalize_decodes_and_collapses() {
        assert_eq!(normalize_text("hello&nbsp;&nbsp;world"), "hello world");
        assert_eq!(normalize_text("it&#039;s  fine"), "it's fine");
    }

    #[test]
    fn normalize_empty_becomes_empty() {
        assert_eq!(normalize_text(""), "");
        assert_eq!(normalize_text("   "), "");
        assert_eq!(normalize_text("\n\t\r"), "");
    }

    // ─────────────────────────────────────────────────────────────
    // canonicalize tests
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn canon_identical_html() {
        let html1 = "<html><body><p>hello</p></body></html>";
        let html2 = "<html><body><p>hello</p></body></html>";
        assert_eq!(canonicalize(html1, &[]), canonicalize(html2, &[]));
    }

    #[test]
    fn canon_different_whitespace() {
        let html1 = "<html><body><p>hello</p></body></html>";
        let html2 = "<html>\n  <body>\n    <p>hello</p>\n  </body>\n</html>";
        assert_eq!(canonicalize(html1, &[]), canonicalize(html2, &[]));
    }

    #[test]
    fn canon_different_entity_encoding() {
        let html1 = "<html><body><p>it&#039;s</p></body></html>";
        let html2 = "<html><body><p>it&#x27;s</p></body></html>";
        assert_eq!(canonicalize(html1, &[]), canonicalize(html2, &[]));
    }

    #[test]
    fn canon_text_whitespace_normalized() {
        let html1 = "<html><body><p>  hello   world  </p></body></html>";
        let html2 = "<html><body><p>hello world</p></body></html>";
        assert_eq!(canonicalize(html1, &[]), canonicalize(html2, &[]));
    }

    #[test]
    fn canon_attribute_order_normalized() {
        let html1 = r#"<html><body><div id="x" class="y"></div></body></html>"#;
        let html2 = r#"<html><body><div class="y" id="x"></div></body></html>"#;
        assert_eq!(canonicalize(html1, &[]), canonicalize(html2, &[]));
    }

    #[test]
    fn canon_detects_different_content() {
        let html1 = "<html><body><p>hello</p></body></html>";
        let html2 = "<html><body><p>world</p></body></html>";
        assert_ne!(canonicalize(html1, &[]), canonicalize(html2, &[]));
    }

    #[test]
    fn canon_detects_different_structure() {
        let html1 = "<html><body><p>hello</p></body></html>";
        let html2 = "<html><body><div>hello</div></body></html>";
        assert_ne!(canonicalize(html1, &[]), canonicalize(html2, &[]));
    }

    #[test]
    fn canon_detects_different_attributes() {
        let html1 = r#"<html><body><div id="a"></div></body></html>"#;
        let html2 = r#"<html><body><div id="b"></div></body></html>"#;
        assert_ne!(canonicalize(html1, &[]), canonicalize(html2, &[]));
    }

    #[test]
    fn canon_detects_missing_element() {
        let html1 = "<html><body><p>one</p><p>two</p></body></html>";
        let html2 = "<html><body><p>one</p></body></html>";
        assert_ne!(canonicalize(html1, &[]), canonicalize(html2, &[]));
    }

    // ─────────────────────────────────────────────────────────────
    // ignore selector tests
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn ignore_by_id() {
        let html1 = r#"<html><body><div id="skip">ignored</div><p>keep</p></body></html>"#;
        let html2 = r#"<html><body><div id="skip">different</div><p>keep</p></body></html>"#;
        let ignore = vec!["#skip".to_string()];
        assert_eq!(canonicalize(html1, &ignore), canonicalize(html2, &ignore));
    }

    #[test]
    fn ignore_by_class() {
        let html1 = r#"<html><body><div class="dynamic">123</div><p>static</p></body></html>"#;
        let html2 = r#"<html><body><div class="dynamic">456</div><p>static</p></body></html>"#;
        let ignore = vec![".dynamic".to_string()];
        assert_eq!(canonicalize(html1, &ignore), canonicalize(html2, &ignore));
    }

    #[test]
    fn ignore_multiple_selectors() {
        let html1 =
            r#"<html><body><div id="a">1</div><div class="b">2</div><p>x</p></body></html>"#;
        let html2 =
            r#"<html><body><div id="a">9</div><div class="b">8</div><p>x</p></body></html>"#;
        let ignore = vec!["#a".to_string(), ".b".to_string()];
        assert_eq!(canonicalize(html1, &ignore), canonicalize(html2, &ignore));
    }

    // ─────────────────────────────────────────────────────────────
    // void element tests
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn void_elements_no_close_tag() {
        let canon = canonicalize("<html><body><br><hr><img src=\"x\"></body></html>", &[]);
        assert!(!canon.contains("</br>"));
        assert!(!canon.contains("</hr>"));
        assert!(!canon.contains("</img>"));
    }

    #[test]
    fn void_elements_self_closing_equivalent() {
        let html1 = "<html><body><br></body></html>";
        let html2 = "<html><body><br/></body></html>";
        assert_eq!(canonicalize(html1, &[]), canonicalize(html2, &[]));
    }
}
