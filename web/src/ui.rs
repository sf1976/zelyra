use super::*;

pub(super) const CRUD_LAYOUT_CONTENT_MARKER: &str = "\u{0}ZELYRA_CRUD_CONTENT\u{0}";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DefaultNavigationLink {
    pub(super) path: String,
    pub(super) label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DefaultUiContext {
    pub(super) current_path: String,
    pub(super) current_label: String,
    pub(super) navigation: Vec<DefaultNavigationLink>,
}

pub(super) fn apply_generated_layout(
    mut response: Response,
    layout_html: Option<&str>,
) -> Response {
    let Some(layout_html) = layout_html else {
        return response;
    };
    if response.location.is_some() || !response.content_type.starts_with("text/html") {
        return response;
    }
    let content = generated_layout_fragment(&response.body);
    response.body = layout_html.replace(CRUD_LAYOUT_CONTENT_MARKER, &content);
    response
}

fn generated_layout_fragment(content: &str) -> String {
    let content = content.trim_start();
    let Some(open_end) = content
        .strip_prefix("<main")
        .and_then(|rest| rest.find('>').map(|index| index + "<main".len() + 1))
    else {
        return content.to_owned();
    };
    let Some(close_start) = content.rfind("</main>").filter(|index| *index >= open_end) else {
        return content.to_owned();
    };
    let attributes = &content["<main".len()..open_end - 1];
    let generated_class = if attributes.contains("class=") {
        ""
    } else {
        " class=\"z-generated-page\""
    };
    format!(
        "<div{generated_class}{attributes}>{}</div>",
        &content[open_end..close_start]
    )
}

pub(super) fn render_default_application_shell(
    content: &str,
    context: &DefaultUiContext,
    language: UiLanguage,
) -> String {
    let document_start = content.trim_start();
    if document_start
        .get(..14)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<!doctype html"))
        || document_start
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<html"))
    {
        return content.to_owned();
    }

    let localized_current_label = localize_user_text(language, &context.current_label);
    let current_label = html_escape_preserving_locale_references(&localized_current_label);
    let page_title =
        html_escape_preserving_locale_references(&format!("{localized_current_label} | Zelyra"));
    let home_path = context
        .navigation
        .first()
        .map(|link| link.path.as_str())
        .unwrap_or(if context.current_path == "/login" {
            "/"
        } else {
            &context.current_path
        });
    let mut navigation = String::new();
    for (index, link) in context.navigation.iter().enumerate() {
        let active = navigation_link_is_active(&link.path, &context.current_path);
        let current = if active { " aria-current=\"page\"" } else { "" };
        let label =
            html_escape_preserving_locale_references(&localize_user_text(language, &link.label));
        navigation.push_str(&format!(
            "<a href=\"{}\" aria-label=\"{label}\"{current}><span class=\"zelyra-nav-icon\" aria-hidden=\"true\">{:02}</span><span class=\"zelyra-nav-label\">{label}</span></a>",
            html_escape(&link.path),
            index + 1,
        ));
    }

    let main_content = if content.trim_start().starts_with("<main") {
        content.to_owned()
    } else {
        format!("<main>{content}</main>")
    };
    format!(
        "<!doctype html><html data-z-ui data-zelyra-language><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{page_title}</title></head><body><a class=\"zelyra-skip-link\" href=\"#zelyra-content\">{}</a><div class=\"z-app-shell zelyra-app\"><header class=\"z-app-shell__header zelyra-topbar\"><div class=\"zelyra-breadcrumb\"><span>{}</span><span aria-hidden=\"true\">/</span><strong>{current_label}</strong></div><span class=\"zelyra-environment\">{}</span><div class=\"z-toolbar zelyra-ui-toolbar\"><button class=\"z-button z-mobile-nav__toggle\" type=\"button\" data-z-mobile-nav-toggle aria-controls=\"zelyra-default-navigation\" aria-expanded=\"false\" hidden>{}</button><button class=\"z-button\" type=\"button\" data-z-theme-toggle data-z-theme-toggle-label hidden aria-label=\"Theme wechseln\">Theme: light</button><button class=\"z-button\" type=\"button\" data-z-density-toggle hidden>Density</button></div></header><aside class=\"z-app-shell__sidebar zelyra-sidebar\"><a class=\"zelyra-brand\" href=\"{}\"><img class=\"zelyra-brand-logo\" src=\"{}\" alt=\"Zelyra\" /></a><p class=\"zelyra-sidebar-caption\">{}</p><nav class=\"z-app-shell__navigation zelyra-nav\" id=\"zelyra-default-navigation\" aria-label=\"{}\">{navigation}</nav><div class=\"zelyra-sidebar-footer\"><span class=\"zelyra-status-dot\" aria-hidden=\"true\"></span><span>{}</span></div></aside><section class=\"z-app-shell__main zelyra-generated-main\" id=\"zelyra-content\"><div class=\"zelyra-page-content\">{main_content}</div></section><footer class=\"z-app-shell__footer zelyra-footer\"><span>{}</span><span>Zelyra</span></footer></div></body></html>",
        tr(language, "shell.skip_to_content"),
        tr(language, "shell.workspace_label"),
        tr(language, "shell.environment_label"),
        tr(language, "shell.navigation_label"),
        html_escape(home_path),
        ZELYRA_UI_LOGO_PATH,
        tr(language, "shell.navigation_caption"),
        tr(language, "shell.navigation_label"),
        tr(language, "shell.powered_by"),
        tr(language, "shell.powered_by"),
    )
}

pub(super) fn navigation_link_is_active(link_path: &str, current_path: &str) -> bool {
    if link_path == "/" {
        return current_path == "/";
    }
    current_path == link_path
        || current_path
            .strip_prefix(link_path.trim_end_matches('/'))
            .is_some_and(|suffix| suffix.starts_with('/'))
}

pub(super) fn crud_route_matches_path(crud: &CrudRoute, path: &str) -> bool {
    let base = crud.path.trim_end_matches('/');
    let base = if base.is_empty() { "/" } else { base };
    let patterns = [
        crud.path.clone(),
        format!("{base}/new"),
        format!("{base}/{{id}}"),
        format!("{base}/{{id}}/edit"),
        format!("{base}/{{id}}/delete"),
        format!("{base}/{{id}}/restore"),
    ];
    patterns
        .iter()
        .any(|pattern| match_path(pattern, path).is_some())
        || crud
            .actions
            .iter()
            .any(|action| match_path(&action.form.path, path).is_some())
}

pub(super) fn add_navigation_link(
    navigation: &mut Vec<DefaultNavigationLink>,
    path: impl Into<String>,
    label: impl Into<String>,
) {
    let path = path.into();
    if !navigation.iter().any(|link| link.path == path) {
        navigation.push(DefaultNavigationLink {
            path,
            label: label.into(),
        });
    }
}

pub(super) fn tr(_language: UiLanguage, key: &str) -> String {
    i18n::reference(key)
}

#[cfg(test)]
pub(super) fn localize_html(source: &str, language: UiLanguage) -> String {
    localize_html_with_catalog(source, language, &ProjectUiCatalogs::default())
}

pub(super) fn localize_html_with_catalog(
    source: &str,
    language: UiLanguage,
    project_catalogs: &ProjectUiCatalogs,
) -> String {
    let language_code = match language {
        UiLanguage::English => "en",
        UiLanguage::German => "de",
    };
    let translated = localize_framework_markup_with_catalog(source, language, project_catalogs);
    let mut html = translated.replace("data-zelyra-language", &format!("lang=\"{language_code}\""));
    let marker = "data-zelyra-i18n=\"";
    let mut search_from = 0;
    while let Some(relative_start) = html[search_from..].find(marker) {
        let marker_start = search_from + relative_start;
        let key_start = marker_start + marker.len();
        let Some(key_end_relative) = html[key_start..].find('"') else {
            break;
        };
        let key_end = key_start + key_end_relative;
        let key = html[key_start..key_end].to_owned();
        let Some(open_tag_start) = html[..marker_start].rfind('<') else {
            break;
        };
        let Some(tag_name_end_relative) = html[open_tag_start + 1..]
            .find(|character: char| character.is_whitespace() || character == '>')
        else {
            break;
        };
        let tag_name_end = open_tag_start + 1 + tag_name_end_relative;
        let tag_name = html[open_tag_start + 1..tag_name_end].to_owned();
        let Some(open_tag_end_relative) = html[tag_name_end..].find('>') else {
            break;
        };
        let content_start = tag_name_end + open_tag_end_relative + 1;
        let closing_tag = format!("</{tag_name}>");
        let Some(content_end_relative) = html[content_start..].find(&closing_tag) else {
            break;
        };
        let content_end = content_start + content_end_relative;
        let translated = html_escape(&project_catalogs.text(language, &key));
        html.replace_range(content_start..content_end, &translated);
        search_from = content_start + translated.len() + closing_tag.len();
    }
    resolve_locale_references(&html, language, project_catalogs)
}

pub(super) fn localize_framework_markup_with_catalog(
    source: &str,
    language: UiLanguage,
    project_catalogs: &ProjectUiCatalogs,
) -> String {
    let mut html = source.to_owned();
    for tag in [
        "h1", "h2", "h3", "p", "button", "label", "legend", "th", "option",
    ] {
        let opening = format!("<{tag}");
        let closing = format!("</{tag}>");
        let mut cursor = 0;
        while let Some(relative_start) = html[cursor..].find(&opening) {
            let start = cursor + relative_start;
            let after_name = start + opening.len();
            if html
                .as_bytes()
                .get(after_name)
                .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
            {
                cursor = after_name;
                continue;
            }
            let Some(open_end_relative) = html[after_name..].find('>') else {
                break;
            };
            let content_start = after_name + open_end_relative + 1;
            let Some(content_end_relative) = html[content_start..].find(&closing) else {
                break;
            };
            let content_end = content_start + content_end_relative;
            if !html[content_start..content_end].contains('<') {
                if let Some(copy) = framework_text_with_catalog(
                    language,
                    &html[content_start..content_end],
                    project_catalogs,
                ) {
                    let translated = html_escape(&copy);
                    html.replace_range(content_start..content_end, &translated);
                    cursor = content_start + translated.len() + closing.len();
                    continue;
                }
            }
            cursor = content_end + closing.len();
        }
    }
    html
}

pub(super) fn resolve_locale_references(
    source: &str,
    language: UiLanguage,
    project_catalogs: &ProjectUiCatalogs,
) -> String {
    let mut output = String::with_capacity(source.len());
    let mut remaining = source;
    while let Some(start) = remaining.find(LOCALE_REFERENCE_START) {
        output.push_str(&remaining[..start]);
        let reference_start = start + LOCALE_REFERENCE_START.len();
        let Some(end_relative) = remaining[reference_start..].find(LOCALE_REFERENCE_END) else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let end = reference_start + end_relative;
        let token_end = end + LOCALE_REFERENCE_END.len_utf8();
        let reference = &remaining[reference_start..end];
        if let Some(translated) = locale_reference_text(reference, language, project_catalogs, 0) {
            output.push_str(&html_escape(&translated));
        } else {
            output.push_str(&remaining[start..token_end]);
        }
        remaining = &remaining[token_end..];
    }
    output.push_str(remaining);
    output
}

pub(super) fn locale_reference_text(
    reference: &str,
    language: UiLanguage,
    project_catalogs: &ProjectUiCatalogs,
    depth: usize,
) -> Option<String> {
    if depth > 8 {
        return None;
    }
    let (key, encoded_field) = reference
        .split_once(LOCALE_REFERENCE_PARAMETER)
        .map_or((reference, None), |(key, value)| (key, Some(value)));
    if !i18n::valid_catalog_key(key) {
        return None;
    }
    let mut translated = project_catalogs.text(language, key).into_owned();
    if let Some(encoded_parameter) = encoded_field {
        let (parameter, encoded_value) = encoded_parameter.split_once('=')?;
        if !i18n::valid_catalog_key(parameter) {
            return None;
        }
        let mut value = decode_locale_parameter(encoded_value)?;
        if parameter == "field" {
            if let Some(nested) = value
                .strip_prefix(LOCALE_REFERENCE_START)
                .and_then(|value| value.strip_suffix(LOCALE_REFERENCE_END))
            {
                value = locale_reference_text(nested, language, project_catalogs, depth + 1)?;
            }
        }
        translated = translated.replace(&format!("{{{parameter}}}"), &value);
    }
    Some(translated)
}

pub(super) fn decode_locale_parameter(value: &str) -> Option<String> {
    if value.len() & 1 != 0 {
        return None;
    }
    let bytes = value
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let pair: &[u8; 2] = pair.try_into().ok()?;
            let digits = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(digits, 16).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

pub(super) fn localize_user_text(_language: UiLanguage, source: &str) -> String {
    source.strip_prefix("@i18n:").map_or_else(
        || {
            source
                .replace(LOCALE_REFERENCE_START, "&#xe000;zelyra-locale:")
                .replace(LOCALE_REFERENCE_PARAMETER, "&#xe002;")
                .replace(LOCALE_REFERENCE_END, "&#xe001;")
        },
        i18n::reference,
    )
}

pub(super) fn inject_design_system(
    source: &str,
    has_replacement_stylesheet: bool,
    has_project_theme: bool,
) -> String {
    let has_ui_markup = source.contains("class=\"zelyra-app\"")
        || source.contains("class=\"z-")
        || source.contains("class='z-")
        || source.contains("data-z-ui");
    if !has_ui_markup {
        return source.to_owned();
    }
    let mut marked_source = source.to_owned();
    if !marked_source.contains("data-z-ui") {
        if let Some(html_start) = marked_source.find("<html") {
            if let Some(tag_end_relative) = marked_source[html_start..].find('>') {
                let tag_end = html_start + tag_end_relative;
                marked_source.insert_str(tag_end, " data-z-ui");
            }
        }
    }
    let selected_stylesheet_missing = if has_replacement_stylesheet {
        !marked_source.contains(&format!("href=\"{PROJECT_UI_CSS_PATH}\""))
    } else {
        !marked_source.contains(&format!("href=\"{ZELYRA_UI_CSS_PATH}\""))
    };
    let javascript_missing = !marked_source.contains(&format!("src=\"{ZELYRA_UI_JS_PATH}\""));
    let project_theme_missing =
        has_project_theme && !marked_source.contains(&format!("href=\"{PROJECT_THEME_CSS_PATH}\""));
    if !selected_stylesheet_missing && !javascript_missing && !project_theme_missing {
        return marked_source;
    }
    let mut assets = String::new();
    if selected_stylesheet_missing {
        let path = if has_replacement_stylesheet {
            PROJECT_UI_CSS_PATH
        } else {
            ZELYRA_UI_CSS_PATH
        };
        assets.push_str(&format!(
            "<link rel=\"stylesheet\" href=\"{path}\" data-z-ui-stylesheet>"
        ));
    }
    if javascript_missing {
        assets.push_str(&format!(
            "<script src=\"{ZELYRA_UI_JS_PATH}\" defer data-z-ui-script></script>"
        ));
    }
    if project_theme_missing {
        assets.push_str(&format!("<link rel=\"stylesheet\" href=\"{PROJECT_THEME_CSS_PATH}\" data-zelyra-theme=\"project\">"));
    }
    if let Some(head_end) = marked_source.rfind("</head>") {
        let mut html = String::with_capacity(marked_source.len() + assets.len());
        html.push_str(&marked_source[..head_end]);
        html.push_str(&assets);
        html.push_str(&marked_source[head_end..]);
        html
    } else {
        marked_source
    }
}

pub(super) fn append_learning_assistant(
    html: &str,
    path: &str,
    language: UiLanguage,
    generated_crud: bool,
) -> String {
    let machine_page = path.starts_with("/machines");
    let section = if machine_page {
        "machine"
    } else if generated_crud {
        "crud"
    } else {
        "view"
    };
    let title = tr(language, &format!("learning.{section}.title"));
    let introduction = tr(language, &format!("learning.{section}.intro"));
    let heading = tr(language, &format!("learning.{section}.heading"));
    let example = tr(language, &format!("learning.{section}.example"));
    let guidance = tr(language, &format!("learning.{section}.guidance"));
    let title = html_escape_preserving_locale_references(&title);
    let introduction = html_escape_preserving_locale_references(&introduction);
    let heading = html_escape_preserving_locale_references(&heading);
    let example = html_escape_preserving_locale_references(&example);
    let guidance = html_escape_preserving_locale_references(&guidance);
    let button_label = html_escape_preserving_locale_references(&tr(language, "learning.button"));
    let style = r#"<style>
.zelyra-learning-assistant{position:fixed;right:24px;bottom:24px;z-index:2147483000;font:500 14px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif;color:#17243b}
.zelyra-learning-assistant summary{display:flex;align-items:center;gap:9px;list-style:none;cursor:pointer;padding:10px 16px 10px 10px;border:1px solid #dce3f2;border-radius:999px;background:linear-gradient(135deg,#fff 5%,#f2f5ff 100%);box-shadow:0 10px 34px #17243b2b;color:#27375a;font-weight:700}
.zelyra-learning-assistant summary::-webkit-details-marker{display:none}
.zelyra-learning-assistant summary:focus-visible{outline:3px solid #927cff;outline-offset:3px}
.zelyra-learning-assistant-icon{display:grid;place-items:center;width:28px;height:28px;border-radius:50%;background:linear-gradient(145deg,#8067ff,#32c7c1);color:white;font-weight:800}
.zelyra-learning-assistant[open] summary{border-color:#a89bff}
.zelyra-learning-panel{position:absolute;right:0;bottom:56px;width:min(390px,calc(100vw - 32px));padding:22px;border:1px solid #e1e6f0;border-radius:18px;background:#fff;box-shadow:0 22px 70px #17243b30}
.zelyra-learning-panel h2{margin:0 0 6px;font-size:19px;letter-spacing:-.02em}
.zelyra-learning-panel h3{margin:18px 0 8px;font-size:15px}
.zelyra-learning-panel p{margin:8px 0;color:#526078}
.zelyra-learning-panel pre{overflow:auto;margin:10px 0;padding:13px 15px;border-radius:10px;background:#111b31;color:#e4e9ff;font:13px/1.6 ui-monospace,SFMono-Regular,Consolas,monospace}
.zelyra-learning-panel code{color:#5140bb;font:600 12px ui-monospace,SFMono-Regular,Consolas,monospace}
@media(max-width:560px){.zelyra-learning-assistant{right:14px;bottom:14px}.zelyra-learning-panel{right:-2px;max-height:70vh;overflow:auto}}
@media print{.zelyra-learning-assistant{display:none}}
</style>"#;
    let widget = format!(
        "{style}<details class=\"zelyra-learning-assistant\"><summary aria-label=\"{button_label}\"><span class=\"zelyra-learning-assistant-icon\" aria-hidden=\"true\">i</span><span>{button_label}</span></summary><section class=\"zelyra-learning-panel\" role=\"region\" aria-label=\"{title}\"><h2>{title}</h2><p>{introduction}</p><h3>{heading}</h3><pre><code>{example}</code></pre><p>{guidance}</p></section></details>"
    );
    if let Some(body_end) = html.rfind("</body>") {
        let mut rendered = String::with_capacity(html.len() + widget.len());
        rendered.push_str(&html[..body_end]);
        rendered.push_str(&widget);
        rendered.push_str(&html[body_end..]);
        rendered
    } else {
        format!("{html}{widget}")
    }
}
