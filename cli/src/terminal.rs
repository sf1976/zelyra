use std::io::IsTerminal;

#[derive(Clone, Copy)]
enum ColorStyle {
    Brand,
    Heading,
    Command,
}

fn enabled_for(
    zelyra_color: Option<&str>,
    no_color: bool,
    clicolor_force: Option<&str>,
    is_terminal: bool,
) -> bool {
    match zelyra_color.map(str::to_ascii_lowercase).as_deref() {
        Some("always") => return true,
        Some("never") => return false,
        Some("auto") => return is_terminal && !no_color,
        Some(_) => return false,
        None => {}
    }
    if no_color {
        return false;
    }
    if clicolor_force.is_some_and(|value| !value.is_empty() && value != "0") {
        return true;
    }
    is_terminal
}

fn color_enabled(terminal: bool) -> bool {
    enabled_for(
        std::env::var("ZELYRA_COLOR").ok().as_deref(),
        std::env::var_os("NO_COLOR").is_some(),
        std::env::var("CLICOLOR_FORCE").ok().as_deref(),
        terminal,
    )
}

fn paint(text: &str, style: ColorStyle, enabled: bool) -> String {
    if !enabled {
        return text.to_owned();
    }
    let code = match style {
        ColorStyle::Brand => "1;38;5;141",
        ColorStyle::Heading => "1;38;5;45",
        ColorStyle::Command => "38;5;114",
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}

pub(super) fn help(text: &str) -> String {
    let enabled = color_enabled(std::io::stderr().is_terminal());
    text.lines()
        .map(|line| {
            let trimmed = line.trim();
            let style = if trimmed.starts_with("Zelyra ") {
                Some(ColorStyle::Brand)
            } else if matches!(
                trimmed,
                "Usage:"
                    | "Schnelleinstieg"
                    | "Wichtige Befehle"
                    | "Getting started"
                    | "Key commands"
            ) {
                Some(ColorStyle::Heading)
            } else if trimmed.starts_with("zelyra ") || trimmed.starts_with("cd ") {
                Some(ColorStyle::Command)
            } else {
                None
            };
            style.map_or_else(|| line.to_owned(), |style| paint(line, style, enabled))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn success(text: &str) -> String {
    paint(
        text,
        ColorStyle::Command,
        color_enabled(std::io::stdout().is_terminal()),
    )
}

#[cfg(test)]
mod tests {
    use super::{enabled_for, help, paint, ColorStyle};

    #[test]
    fn automatic_color_respects_terminal_and_no_color() {
        assert!(enabled_for(None, false, None, true));
        assert!(!enabled_for(None, false, None, false));
        assert!(!enabled_for(None, true, Some("1"), true));
    }

    #[test]
    fn explicit_color_settings_are_predictable() {
        assert!(enabled_for(Some("always"), true, None, false));
        assert!(!enabled_for(Some("never"), false, Some("1"), true));
        assert!(enabled_for(None, false, Some("1"), false));
        assert!(!enabled_for(Some("invalid"), false, None, true));
    }

    #[test]
    fn plain_output_has_no_escape_codes_and_forced_output_does() {
        assert_eq!(paint("ready", ColorStyle::Brand, false), "ready");
        assert_eq!(
            paint("ready", ColorStyle::Brand, true),
            "\x1b[1;38;5;141mready\x1b[0m"
        );
    }

    #[test]
    fn help_styles_only_recognized_headings_and_commands() {
        let plain = help("Zelyra 0.9.9\n\nUsage:\n  zelyra run main.zyl\nopaque");
        assert!(!plain.contains("\x1b["));
        assert!(plain.contains("zelyra run main.zyl"));
    }
}
