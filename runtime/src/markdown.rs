//! Bounded Markdown-to-HTML rendering for trusted Zelyra view integration.
//!
//! Raw HTML is emitted as escaped text, and link destinations are restricted
//! to web, mail, or relative URLs before rendering.

use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write};

const MAX_MARKDOWN_SOURCE_BYTES: usize = 1_048_576;
const MAX_MARKDOWN_OUTPUT_BYTES: usize = 4_194_304;

pub fn render_markdown(source: &str) -> Result<String, String> {
    if source.len() > MAX_MARKDOWN_SOURCE_BYTES {
        return Err(format!(
            "Markdown source exceeds the {} byte limit",
            MAX_MARKDOWN_SOURCE_BYTES
        ));
    }

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let events = Parser::new_ext(source, options).collect::<Vec<_>>();
    let events = add_heading_ids(events);
    let events = remove_unsafe_html_and_urls(events);
    let mut html = BoundedOutput::with_capacity(source.len().saturating_mul(2));
    let render_failed = html::write_html_fmt(&mut html, events.into_iter()).is_err();
    if html.overflowed || render_failed {
        return Err(format!(
            "Markdown HTML output exceeds the {} byte limit",
            MAX_MARKDOWN_OUTPUT_BYTES
        ));
    }

    Ok(html.output)
}

/// Build a Markdown table of contents using the same stable heading IDs as
/// [`render_markdown`]. The returned Markdown can be prepended to the source
/// and then rendered by the existing safe Markdown renderer.
pub fn markdown_toc(source: &str, label: &str) -> Result<String, String> {
    if source.len() > MAX_MARKDOWN_SOURCE_BYTES {
        return Err(format!(
            "Markdown source exceeds the {} byte limit",
            MAX_MARKDOWN_SOURCE_BYTES
        ));
    }

    let events = Parser::new_ext(
        source,
        Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_HEADING_ATTRIBUTES,
    )
    .collect::<Vec<_>>();
    let headings = collect_headings(&events);
    if headings.is_empty() {
        return Ok(String::new());
    }

    let mut output = BoundedOutput::with_capacity(source.len().saturating_mul(2));
    output
        .write_str("**")
        .map_err(|_| markdown_output_limit_error("table-of-contents"))?;
    write_escaped_markdown_link_text(&mut output, label)
        .map_err(|_| markdown_output_limit_error("table-of-contents"))?;
    output
        .write_str("**\n\n")
        .map_err(|_| markdown_output_limit_error("table-of-contents"))?;
    for heading in headings {
        output
            .write_str("- [")
            .map_err(|_| markdown_output_limit_error("table-of-contents"))?;
        write_escaped_markdown_link_text(&mut output, heading.text.trim())
            .map_err(|_| markdown_output_limit_error("table-of-contents"))?;
        writeln!(output, "](#{})", heading.id)
            .map_err(|_| markdown_output_limit_error("table-of-contents"))?;
    }
    Ok(output.output)
}

fn markdown_output_limit_error(kind: &str) -> String {
    format!("Markdown {kind} output exceeds the {MAX_MARKDOWN_OUTPUT_BYTES} byte limit")
}

fn write_escaped_markdown_link_text(output: &mut impl Write, value: &str) -> fmt::Result {
    let mut pending_space = false;
    for character in value.chars() {
        if character.is_whitespace() {
            pending_space = true;
            continue;
        }
        if pending_space {
            output.write_char(' ')?;
            pending_space = false;
        }
        if character.is_ascii_punctuation() {
            output.write_char('\\')?;
        }
        output.write_char(character)?;
    }
    Ok(())
}

fn remove_unsafe_html_and_urls(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut output = Vec::with_capacity(events.len());
    let mut unsafe_link_depth = 0usize;
    let mut unsafe_image_depth = 0usize;
    for event in events {
        match &event {
            Event::Start(Tag::Link { dest_url, .. }) if !safe_markdown_url(dest_url, false) => {
                unsafe_link_depth += 1;
            }
            Event::End(TagEnd::Link) if unsafe_link_depth > 0 => {
                unsafe_link_depth -= 1;
            }
            Event::Start(Tag::Image { dest_url, .. }) if !safe_markdown_url(dest_url, true) => {
                unsafe_image_depth += 1;
            }
            Event::End(TagEnd::Image) if unsafe_image_depth > 0 => {
                unsafe_image_depth -= 1;
            }
            Event::Html(raw) | Event::InlineHtml(raw) => {
                // Raw HTML is rendered as literal text so authors can still
                // see it in examples without allowing it to create markup.
                output.push(Event::Text(raw.clone()));
            }
            _ if unsafe_link_depth > 0 => output.push(event),
            _ if unsafe_image_depth > 0 => output.push(event),
            _ => output.push(event),
        }
    }
    output
}

fn safe_markdown_url(destination: &str, image: bool) -> bool {
    if destination.chars().any(char::is_control) {
        return false;
    }
    match url::Url::parse(destination) {
        Ok(url) => !image && matches!(url.scheme(), "http" | "https" | "mailto"),
        Err(_) => {
            let Ok(base) = url::Url::parse("https://zelyra.invalid/") else {
                return false;
            };
            let Ok(url) = url::Url::options().base_url(Some(&base)).parse(destination) else {
                return false;
            };
            matches!(url.scheme(), "http" | "https")
                && (!image || url.host_str() == Some("zelyra.invalid"))
        }
    }
}

struct BoundedOutput {
    output: String,
    overflowed: bool,
}

impl BoundedOutput {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            output: String::with_capacity(capacity.min(MAX_MARKDOWN_OUTPUT_BYTES)),
            overflowed: false,
        }
    }
}

impl Write for BoundedOutput {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.output.len().saturating_add(value.len()) > MAX_MARKDOWN_OUTPUT_BYTES {
            self.overflowed = true;
            return Err(fmt::Error);
        }
        self.output.push_str(value);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkdownHeading {
    event_index: usize,
    level: u8,
    id: String,
    text: String,
}

fn collect_headings(events: &[Event<'_>]) -> Vec<MarkdownHeading> {
    let mut headings = Vec::new();
    let mut current: Option<(usize, u8, Option<String>, String)> = None;
    let mut occurrences = HashMap::<String, usize>::new();
    let mut identifiers = HashSet::new();

    for (event_index, event) in events.iter().enumerate() {
        match event {
            Event::Start(Tag::Heading { level, id, .. }) => {
                current = Some((
                    event_index,
                    heading_level(*level),
                    id.as_deref().map(str::to_owned),
                    String::new(),
                ));
            }
            Event::End(TagEnd::Heading(level)) => {
                if let Some((start_index, current_level, explicit_id, text)) = current.take() {
                    if current_level == heading_level(*level) {
                        let base = slugify_heading(explicit_id.as_deref().unwrap_or(&text));
                        let id = unique_heading_id(base, &mut occurrences, &mut identifiers);
                        headings.push(MarkdownHeading {
                            event_index: start_index,
                            level: current_level,
                            id,
                            text,
                        });
                    }
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, _, heading_text)) = current.as_mut() {
                    heading_text.push_str(text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, _, _, heading_text)) = current.as_mut() {
                    heading_text.push(' ');
                }
            }
            _ => {}
        }
    }
    headings
}

fn heading_level(level: pulldown_cmark::HeadingLevel) -> u8 {
    match level {
        pulldown_cmark::HeadingLevel::H1 => 1,
        pulldown_cmark::HeadingLevel::H2 => 2,
        pulldown_cmark::HeadingLevel::H3 => 3,
        pulldown_cmark::HeadingLevel::H4 => 4,
        pulldown_cmark::HeadingLevel::H5 => 5,
        pulldown_cmark::HeadingLevel::H6 => 6,
    }
}

fn unique_heading_id(
    base: String,
    occurrences: &mut HashMap<String, usize>,
    identifiers: &mut HashSet<String>,
) -> String {
    let count = occurrences.entry(base.clone()).or_insert(0);
    let mut slug = if *count == 0 {
        base.clone()
    } else {
        format!("{base}-{}", *count + 1)
    };
    *count += 1;
    while !identifiers.insert(slug.clone()) {
        slug = format!("{base}-{}", *count + 1);
        *count += 1;
    }
    slug
}

fn add_heading_ids(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut rendered = Vec::with_capacity(events.len());
    let headings = collect_headings(&events)
        .into_iter()
        .map(|heading| (heading.event_index, heading.id))
        .collect::<HashMap<_, _>>();
    for (index, event) in events.iter().enumerate() {
        let Event::Start(Tag::Heading {
            level,
            classes,
            attrs,
            ..
        }) = event
        else {
            rendered.push(event.clone());
            continue;
        };
        let Some(slug) = headings.get(&index) else {
            rendered.push(event.clone());
            continue;
        };
        rendered.push(Event::Start(Tag::Heading {
            level: *level,
            id: Some(CowStr::from(slug.clone())),
            classes: classes.clone(),
            attrs: attrs.clone(),
        }));
    }
    rendered
}

fn slugify_heading(heading: &str) -> String {
    let mut slug = String::new();
    let mut separator_pending = false;
    for character in heading.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() || matches!(character, '_' | '-') {
            if separator_pending && !slug.is_empty() && !slug.ends_with('-') {
                slug.push('-');
            }
            separator_pending = false;
            slug.push(character);
        } else {
            separator_pending = true;
        }
    }
    if slug.is_empty() {
        "section".to_owned()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_commonmark_and_gfm_constructs() {
        let rendered = render_markdown(
            "# Heading\n\n~~gone~~ **bold**\n\n| A | B |\n| - | - |\n| 1 | 2 |\n\n- [x] done",
        )
        .unwrap();
        assert!(rendered.contains("<h1 id=\"heading\">Heading</h1>"));
        assert!(rendered.contains("id=\"heading\""));
        assert!(rendered.contains("<del>gone</del>"));
        assert!(rendered.contains("<strong>bold</strong>"));
        assert!(rendered.contains("<table>"));
        assert!(rendered.contains("type=\"checkbox\""));
    }

    #[test]
    fn escapes_raw_html_and_rejects_unsafe_link_schemes() {
        let rendered = render_markdown(
            "<script>alert(1)</script>\n\n<a href=\"javascript:alert(2)\" onclick=\"alert(3)\">bad</a>\n\n[x](javascript:alert%284%29)",
        )
        .unwrap();
        assert!(!rendered.contains("<script"));
        assert!(!rendered.contains("<a href=\"javascript:"));
        assert!(rendered.contains("&lt;a href=\"javascript:"));
        assert!(rendered.contains("bad"));
    }

    #[test]
    fn preserves_safe_links_and_rejects_unsafe_link_and_image_schemes() {
        let rendered = render_markdown(
            "[safe](https://example.test/docs) [relative](../guide) [unsafe](javascript:alert%281%29) ![local](/images/a.png) ![remote](https://tracker.example/pixel) ![alt](data:image/svg+xml;base64,PHN2Zz4=)",
        )
        .unwrap();
        assert!(rendered.contains("href=\"https://example.test/docs\""));
        assert!(rendered.contains("href=\"../guide\""));
        assert!(!rendered.contains("javascript:"));
        assert!(rendered.contains("<img src=\"/images/a.png\""));
        assert!(!rendered.contains("tracker.example"));
        assert!(!rendered.contains("data:image"));
    }

    #[test]
    fn rejects_oversized_markdown_before_parsing() {
        let source = "x".repeat(MAX_MARKDOWN_SOURCE_BYTES + 1);
        assert!(render_markdown(&source)
            .unwrap_err()
            .contains("Markdown source exceeds"));
    }

    #[test]
    fn generates_stable_unique_heading_slugs_and_keeps_explicit_ids() {
        let rendered =
            render_markdown("# Zelyra UI\n\n# Zelyra UI\n\n## Eigene ID {#custom}").unwrap();
        assert!(rendered.contains("id=\"zelyra-ui\""));
        assert!(rendered.contains("id=\"zelyra-ui-2\""));
        assert!(rendered.contains("id=\"custom\""));
    }

    #[test]
    fn table_of_contents_uses_the_same_heading_ids_and_escapes_markdown() {
        let source = "# Guide & Setup\n\n## Duplicate\n\n## Duplicate\n\n### `x](https://evil.example)` {#own-id}\n";
        let rendered = render_markdown(source).unwrap();
        let toc = markdown_toc(source, "Inhaltsverzeichnis\n\n# injected").unwrap();
        let combined = render_markdown(&format!("{toc}\n{source}")).unwrap();

        assert!(rendered.contains("id=\"guide-setup\""));
        assert!(toc.starts_with("**Inhaltsverzeichnis \\# injected**\n\n"));
        assert!(toc.contains(r"- [Guide \& Setup](#guide-setup)"));
        assert!(toc.contains("- [Duplicate](#duplicate)"));
        assert!(toc.contains("- [Duplicate](#duplicate-2)"));
        assert!(toc.contains(r"- [x\]\(https\:\/\/evil\.example\)](#own-id)"));
        assert!(combined.contains("<a href=\"#guide-setup\">Guide &amp; Setup</a>"));
        assert!(!combined.contains("href=\"https://evil.example\""));
        assert!(combined.contains("id=\"own-id\""));
    }

    #[test]
    fn table_of_contents_is_empty_without_headings_and_obeys_source_limit() {
        assert_eq!(markdown_toc("plain text", "Contents").unwrap(), "");
        let source = "#".repeat(MAX_MARKDOWN_SOURCE_BYTES + 1);
        assert!(markdown_toc(&source, "Contents")
            .unwrap_err()
            .contains("Markdown source exceeds"));

        let oversized_label = "x".repeat(MAX_MARKDOWN_OUTPUT_BYTES + 1);
        assert!(markdown_toc("# Heading", &oversized_label)
            .unwrap_err()
            .contains("Markdown table-of-contents output exceeds"));
    }
}
