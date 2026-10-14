//! Small, deterministic PDF output for server generated business documents.
//!
//! This renderer intentionally uses PDF's standard Helvetica font and has no
//! browser, network, font download, or external executable dependency. It
//! converts the visible text from rendered HTML to paginated, searchable PDF.

const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_PAGES: usize = 200;
const LINES_PER_PAGE: usize = 51;
const LINE_WIDTH: usize = 92;

pub(super) fn render_html_to_pdf(html: &str) -> Result<Vec<u8>, &'static str> {
    if html.len() > MAX_SOURCE_BYTES {
        return Err("PDF view exceeds the 2 MiB rendering limit");
    }
    let text = html_to_text(html);
    let lines = wrap_text(&text);
    let pages = lines
        .chunks(LINES_PER_PAGE)
        .map(|page| page.to_vec())
        .collect::<Vec<_>>();
    if pages.len() > MAX_PAGES {
        return Err("PDF view exceeds the 200 page rendering limit");
    }
    build_pdf(&pages)
}

fn html_to_text(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    let mut hidden_tag: Option<String> = None;
    while cursor < html.len() {
        let rest = &html[cursor..];
        if rest.starts_with('<') {
            if rest.starts_with("<!--") {
                let Some(relative_end) = rest.find("-->") else {
                    break;
                };
                cursor += relative_end + 3;
                continue;
            }
            let Some(relative_end) = rest.find('>') else {
                break;
            };
            let raw_tag = rest[1..relative_end].trim();
            let closing = raw_tag.starts_with('/');
            let name = raw_tag
                .trim_start_matches('/')
                .split(|character: char| character.is_ascii_whitespace() || character == '/')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if closing && hidden_tag.as_deref() == Some(name.as_str()) {
                hidden_tag = None;
            } else if !closing && matches!(name.as_str(), "script" | "style" | "head") {
                hidden_tag = Some(name.clone());
            }
            if hidden_tag.is_none()
                && (closing
                    || matches!(
                        name.as_str(),
                        "br" | "hr"
                            | "p"
                            | "div"
                            | "section"
                            | "article"
                            | "header"
                            | "footer"
                            | "h1"
                            | "h2"
                            | "h3"
                            | "h4"
                            | "li"
                            | "tr"
                    ))
            {
                output.push('\n');
            } else if hidden_tag.is_none() && matches!(name.as_str(), "td" | "th") {
                output.push_str("  ");
            }
            cursor += relative_end + 1;
            continue;
        }
        if hidden_tag.is_some() {
            let next_tag = rest.find('<').unwrap_or(rest.len());
            cursor += next_tag.max(1);
            continue;
        }
        let character = rest.chars().next().expect("cursor is within the string");
        if character == '&' {
            if let Some(relative_end) = rest.find(';').filter(|end| *end <= 12) {
                if let Some(decoded) = decode_entity(&rest[1..relative_end]) {
                    output.push(decoded);
                    cursor += relative_end + 1;
                    continue;
                }
            }
        }
        if character == '\r' {
            cursor += 1;
            continue;
        }
        output.push(character);
        cursor += character.len_utf8();
    }
    output
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        _ if entity.starts_with("#x") || entity.starts_with("#X") => {
            u32::from_str_radix(&entity[2..], 16)
                .ok()
                .and_then(char::from_u32)
        }
        _ if entity.starts_with('#') => entity[1..].parse().ok().and_then(char::from_u32),
        _ => None,
    }
}

fn wrap_text(text: &str) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = Vec::new();
        for word in paragraph.split_whitespace() {
            let encoded = word.chars().map(win_ansi).collect::<Vec<_>>();
            if encoded.is_empty() {
                continue;
            }
            if !line.is_empty() && line.len() + 1 + encoded.len() > LINE_WIDTH {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(b' ');
            }
            for chunk in encoded.chunks(LINE_WIDTH) {
                if !line.is_empty() && line.len() + chunk.len() > LINE_WIDTH {
                    lines.push(std::mem::take(&mut line));
                }
                line.extend_from_slice(chunk);
                if line.len() >= LINE_WIDTH {
                    lines.push(std::mem::take(&mut line));
                }
            }
        }
        if !line.is_empty() {
            lines.push(line);
        } else {
            lines.push(Vec::new());
        }
    }
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    lines
}

fn win_ansi(character: char) -> u8 {
    match character as u32 {
        value @ 0x20..=0x7e => value as u8,
        value @ 0xa0..=0xff => value as u8,
        0x20ac => 0x80,
        0x201a => 0x82,
        0x0192 => 0x83,
        0x201e => 0x84,
        0x2026 => 0x85,
        0x2020 => 0x86,
        0x2021 => 0x87,
        0x02c6 => 0x88,
        0x2030 => 0x89,
        0x0160 => 0x8a,
        0x2039 => 0x8b,
        0x0152 => 0x8c,
        0x017d => 0x8e,
        0x2018 => 0x91,
        0x2019 => 0x92,
        0x201c => 0x93,
        0x201d => 0x94,
        0x2022 => 0x95,
        0x2013 => 0x96,
        0x2014 => 0x97,
        0x02dc => 0x98,
        0x2122 => 0x99,
        0x0161 => 0x9a,
        0x203a => 0x9b,
        0x0153 => 0x9c,
        0x017e => 0x9e,
        0x0178 => 0x9f,
        _ => b'?',
    }
}

fn build_pdf(pages: &[Vec<Vec<u8>>]) -> Result<Vec<u8>, &'static str> {
    let mut objects = Vec::with_capacity(3 + pages.len() * 2);
    let page_ids = (0..pages.len())
        .map(|index| 4 + index * 2)
        .collect::<Vec<_>>();
    let kids = page_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", pages.len()).into_bytes());
    objects.push(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    for (index, page) in pages.iter().enumerate() {
        let page_id = page_ids[index];
        let content_id = page_id + 1;
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>"
            )
            .into_bytes(),
        );
        let mut content = b"BT\n/F1 10 Tf\n48 794 Td\n14 TL\n".to_vec();
        for line in page {
            content.push(b'(');
            for byte in line {
                match byte {
                    b'(' | b')' | b'\\' => {
                        content.push(b'\\');
                        content.push(*byte);
                    }
                    0..=31 | 127 => content.push(b' '),
                    _ => content.push(*byte),
                }
            }
            content.extend_from_slice(b") Tj\nT*\n");
        }
        content.extend_from_slice(b"ET");
        let stream = format!("<< /Length {} >>\nstream\n", content.len(),);
        let mut stream_object = stream.into_bytes();
        stream_object.extend_from_slice(&content);
        stream_object.extend_from_slice(b"\nendstream");
        objects.push(stream_object);
    }
    if objects.len() > u16::MAX as usize {
        return Err("PDF has too many objects");
    }
    let mut output = b"%PDF-1.4\n%Zelyra\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0usize);
    for (index, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        output.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        output.extend_from_slice(object);
        output.extend_from_slice(b"\nendobj\n");
    }
    let xref_offset = output.len();
    output.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    output.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    output.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_searchable_paginated_pdf_without_external_tools() {
        let html = "<html><head><style>p { color:red }</style></head><body><!-- internal secret --><script>do_not_include()</script><h1>Rechnung</h1><p>Grüße &amp; danke</p></body></html>";
        let pdf = render_html_to_pdf(html).unwrap();
        let output = String::from_utf8_lossy(&pdf);
        assert!(output.starts_with("%PDF-1.4"));
        assert!(output.contains("Rechnung"));
        assert!(!output.contains("internal secret"));
        assert!(!output.contains("do_not_include"));
        assert!(pdf
            .windows(b"Gr\xfc\xdfe & danke".len())
            .any(|window| window == b"Gr\xfc\xdfe & danke"));
        assert!(output.ends_with("%%EOF\n"));
    }

    #[test]
    fn rejects_pdf_sources_above_the_fixed_limit() {
        let html = "x".repeat(MAX_SOURCE_BYTES + 1);
        assert!(render_html_to_pdf(&html).unwrap_err().contains("2 MiB"));
    }
}
