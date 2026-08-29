//! CommonMark parsing helpers shared by the structure index and reference validator.
//!
//! The SpecMan data model leans on the `markdown` crate's `to_mdast` output to
//! deterministically extract inline-link destinations, heading slugs, and heading
//! positions. Keeping those helpers in one place ensures the structure index and
//! the reference validator resolve links and slugs identically.

use std::collections::HashMap;

use markdown::mdast::Node;
use markdown::{ParseOptions, to_mdast};
use unicode_normalization::UnicodeNormalization;

/// A link destination discovered inside a Markdown document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkDestination {
    pub(crate) destination: String,
    /// 1-based line of the opening bracket/angle bracket.
    pub(crate) line: usize,
    /// 1-based column of the opening bracket/angle bracket.
    pub(crate) column: usize,
}

/// A heading discovered inside a Markdown document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HeadingInfo {
    pub(crate) depth: u8,
    /// Plain-text title (formatting stripped, rendered text preserved).
    pub(crate) title: String,
    /// Deterministic SpecMan slug (see [Concept: Markdown Slugs]).
    pub(crate) slug: String,
    /// 1-based line where the heading starts.
    pub(crate) start_line: usize,
    /// 1-based line where the heading ends (last line of the heading itself).
    pub(crate) end_line: usize,
}

/// Parses inline links, reference links, and autolinks from `content`.
///
/// Image destinations are intentionally ignored. Reference links are resolved
/// against link reference definitions found anywhere in the document.
pub(crate) fn parse_links(content: &str) -> Vec<LinkDestination> {
    let Ok(root) = to_mdast(content, &ParseOptions::default()) else {
        return Vec::new();
    };

    let mut definitions: HashMap<String, String> = HashMap::new();
    collect_definitions(&root, &mut definitions);

    let mut links = Vec::new();
    collect_links(&root, &definitions, &mut links);
    links
}

/// Derives the deterministic SpecMan heading slug from a plain-text title.
///
/// Returns `None` when the resulting slug is empty (treated as invalid).
pub(crate) fn slugify(title: &str) -> Option<String> {
    let nfkd: String = title.nfkd().collect();
    let lower = nfkd.to_lowercase();

    let mut result = String::new();
    let mut pending_space = false;

    for c in lower.chars() {
        if c.is_whitespace() {
            pending_space = true;
        } else if c.is_alphabetic() || c.is_numeric() || c == '-' {
            if pending_space {
                push_separator(&mut result);
                pending_space = false;
            }
            if c == '-' {
                push_separator(&mut result);
            } else {
                result.push(c);
            }
        }
        // Any other character is dropped during filtering.
    }

    // Trim any trailing hyphen left behind by a dangling separator.
    while result.ends_with('-') {
        result.pop();
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Extracts headings (with disambiguated slugs) in document order.
pub(crate) fn extract_headings(content: &str) -> Vec<HeadingInfo> {
    let Ok(root) = to_mdast(content, &ParseOptions::default()) else {
        return Vec::new();
    };

    let mut headings = Vec::new();
    collect_headings(&root, &mut headings);
    headings.sort_by_key(|heading| heading.start_line);

    let mut seen: HashMap<String, usize> = HashMap::new();
    for heading in &mut headings {
        let Some(base) = slugify(&heading.title) else {
            heading.slug = String::new();
            continue;
        };

        let count = seen.entry(base.clone()).or_insert(0);
        let slug = if *count == 0 {
            base.clone()
        } else {
            format!("{base}-{count}")
        };
        *count += 1;
        heading.slug = slug;
    }

    headings
}

/// Appends a single hyphen separator, collapsing contiguous hyphens and
/// suppressing leading/trailing separators.
fn push_separator(result: &mut String) {
    if !result.is_empty() && !result.ends_with('-') {
        result.push('-');
    }
}

/// Returns the child nodes for parent node variants, or an empty slice for leaves.
fn children(node: &Node) -> &[Node] {
    match node {
        Node::Root(node) => &node.children,
        Node::Blockquote(node) => &node.children,
        Node::FootnoteDefinition(node) => &node.children,
        Node::List(node) => &node.children,
        Node::ListItem(node) => &node.children,
        Node::Paragraph(node) => &node.children,
        Node::Heading(node) => &node.children,
        Node::Emphasis(node) => &node.children,
        Node::Strong(node) => &node.children,
        Node::Delete(node) => &node.children,
        Node::Link(node) => &node.children,
        Node::LinkReference(node) => &node.children,
        Node::Table(node) => &node.children,
        Node::TableRow(node) => &node.children,
        Node::TableCell(node) => &node.children,
        _ => &[],
    }
}

/// Collects link reference definitions keyed by their normalized identifier.
fn collect_definitions(node: &Node, map: &mut HashMap<String, String>) {
    if let Node::Definition(definition) = node {
        map.insert(definition.identifier.clone(), definition.url.clone());
    }
    for child in children(node) {
        collect_definitions(child, map);
    }
}

/// Collects inline links, reference links, and autolinks in document order.
fn collect_links(
    node: &Node,
    definitions: &HashMap<String, String>,
    out: &mut Vec<LinkDestination>,
) {
    if let Node::Link(link) = node {
        let (line, column) = link
            .position
            .as_ref()
            .map(|position| (position.start.line, position.start.column))
            .unwrap_or((0, 0));
        out.push(LinkDestination {
            destination: link.url.clone(),
            line,
            column,
        });
    } else if let Node::LinkReference(reference) = node {
        let destination = definitions
            .get(&reference.identifier)
            .cloned()
            .unwrap_or_else(|| reference.identifier.clone());
        let (line, column) = reference
            .position
            .as_ref()
            .map(|position| (position.start.line, position.start.column))
            .unwrap_or((0, 0));
        out.push(LinkDestination {
            destination,
            line,
            column,
        });
    }

    for child in children(node) {
        collect_links(child, definitions, out);
    }
}

/// Collects headings in document order.
fn collect_headings(node: &Node, out: &mut Vec<HeadingInfo>) {
    if let Node::Heading(heading) = node {
        let mut title = String::new();
        for child in &heading.children {
            plain_text(child, &mut title);
        }

        let (start_line, end_line) = heading
            .position
            .as_ref()
            .map(|position| (position.start.line, position.end.line))
            .unwrap_or((0, 0));

        out.push(HeadingInfo {
            depth: heading.depth,
            title,
            slug: String::new(),
            start_line,
            end_line,
        });
    }

    for child in children(node) {
        collect_headings(child, out);
    }
}

/// Appends the plain-text rendering of an inline node, stripping formatting while
/// preserving the rendered text.
fn plain_text(node: &Node, out: &mut String) {
    match node {
        Node::Text(node) => out.push_str(&node.value),
        Node::InlineCode(node) => out.push_str(&node.value),
        Node::InlineMath(node) => out.push_str(&node.value),
        Node::Break(_) => out.push(' '),
        Node::Image(node) => out.push_str(&node.alt),
        Node::ImageReference(node) => out.push_str(&node.alt),
        Node::Link(_) | Node::LinkReference(_) => {
            for child in children(node) {
                plain_text(child, out);
            }
        }
        Node::Emphasis(_) | Node::Strong(_) | Node::Delete(_) => {
            for child in children(node) {
                plain_text(child, out);
            }
        }
        // Inline HTML, footnotes, and other synthetic nodes do not contribute
        // rendered text to a heading slug.
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_concept_headings() {
        assert_eq!(
            slugify("Concept: Markdown Slugs").as_deref(),
            Some("concept-markdown-slugs")
        );
        assert_eq!(
            slugify("Concept: SpecMan Workspace").as_deref(),
            Some("concept-specman-workspace")
        );
    }

    #[test]
    fn slug_strips_inline_formatting() {
        let headings = extract_headings("# **Bold** and `code`\n\n# [A link](#a-link)\n");
        let slugs: Vec<&str> = headings.iter().map(|h| h.slug.as_str()).collect();
        assert_eq!(slugs, vec!["bold-and-code", "a-link"]);
    }

    #[test]
    fn slug_normalizes_whitespace_and_hyphens() {
        assert_eq!(slugify("  Foo   Bar ").as_deref(), Some("foo-bar"));
        assert_eq!(slugify("Foo - Bar").as_deref(), Some("foo-bar"));
        assert_eq!(slugify("Foo--Bar").as_deref(), Some("foo-bar"));
        assert_eq!(slugify("- Leading").as_deref(), Some("leading"));
    }

    #[test]
    fn slug_rejects_empty_result() {
        assert_eq!(slugify("!!!"), None);
        assert_eq!(slugify("   "), None);
    }

    #[test]
    fn parses_inline_reference_and_autolinks() {
        let content = "\
# Intro

See [the guide](guide.md) and [the concept][c] plus <https://example.com>.

[c]: ./concept.md#concept-x
";
        let links = parse_links(content);
        let destinations: Vec<&str> = links.iter().map(|l| l.destination.as_str()).collect();
        assert_eq!(
            destinations,
            vec!["guide.md", "./concept.md#concept-x", "https://example.com"]
        );
    }

    #[test]
    fn ignores_image_destinations() {
        let links = parse_links("![alt](image.png) and [real](doc.md)");
        let destinations: Vec<&str> = links.iter().map(|l| l.destination.as_str()).collect();
        assert_eq!(destinations, vec!["doc.md"]);
    }

    #[test]
    fn extracts_disambiguated_heading_slugs() {
        let content = "\
# Overview

## Overview

### Overview
";
        let headings = extract_headings(content);
        let slugs: Vec<&str> = headings.iter().map(|h| h.slug.as_str()).collect();
        assert_eq!(slugs, vec!["overview", "overview-1", "overview-2"]);
    }
}
