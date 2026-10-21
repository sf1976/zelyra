//! Bounded Markdown-to-HTML rendering for trusted Zelyra view integration.
//!
//! Raw HTML is emitted as escaped text, and link destinations are restricted
//! to web, mail, or relative URLs before rendering.

use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write};

const MAX_MARKDOWN_SOURCE_BYTES: usize = 1_048_576;
const MAX_MARKDOWN_HTML_BYTES: usize = 4_194_304;

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
    let mut html = BoundedHtml::with_capacity(source.len().saturating_mul(2));
    let render_failed = html::write_html_fmt(&mut html, events.into_iter()).is_err();
    if html.overflowed || render_failed {
        return Err(format!(
            "Markdown HTML output exceeds the {} byte limit",
            MAX_MARKDOWN_HTML_BYTES
        ));
    }

    Ok(html.output)
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

struct BoundedHtml {
    output: String,
    overflowed: bool,
}

impl BoundedHtml {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            output: String::with_capacity(capacity.min(MAX_MARKDOWN_HTML_BYTES)),
            overflowed: false,
        }
    }
}

impl Write for BoundedHtml {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.output.len().saturating_add(value.len()) > MAX_MARKDOWN_HTML_BYTES {
            self.overflowed = true;
            return Err(fmt::Error);
        }
        self.output.push_str(value);
        Ok(())
    }
}

fn add_heading_ids(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut rendered = Vec::with_capacity(events.len());
    let mut identifiers = HashSet::new();
    let mut occurrences = HashMap::<String, usize>::new();
    for (index, event) in events.iter().enumerate() {
        let Event::Start(Tag::Heading {
            level,
            id,
            classes,
            attrs,
        }) = event
        else {
            rendered.push(event.clone());
            continue;
        };

        let mut heading_text = String::new();
        for child in events.iter().skip(index + 1) {
            match child {
                Event::End(TagEnd::Heading(end_level)) if end_level == level => break,
                Event::Text(text) | Event::Code(text) => heading_text.push_str(text),
                Event::SoftBreak | Event::HardBreak => heading_text.push(' '),
                _ => {}
            }
        }
        let base = slugify_heading(id.as_deref().unwrap_or(&heading_text));
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
        rendered.push(Event::Start(Tag::Heading {
            level: *level,
            id: Some(CowStr::from(slug)),
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
}
