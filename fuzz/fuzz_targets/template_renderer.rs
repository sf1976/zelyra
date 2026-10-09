#![no_main]

use libfuzzer_sys::fuzz_target;
use std::collections::HashMap;
use zelyra_ast::{CrudFormViewDef, FormDef, FormField, Span, Type};
use zelyra_forms::FieldError;
use zelyra_web::{render_form, CsrfProtection, FormRoute};

fuzz_target!(|input: &[u8]| {
    if input.len() > 8 * 1024 {
        return;
    }
    let text = String::from_utf8_lossy(input).into_owned();
    let field = FormField {
        name: "comment".into(),
        ty: Some(Type::String),
        label: Some(text.clone()),
        placeholder: Some(text.clone()),
        required: false,
        max: Some(4096),
        widget: None,
        readonly: false,
        span: Span::default(),
    };
    let route = FormRoute {
        path: text.clone(),
        action: text.clone(),
        form: FormDef {
            name: "FuzzForm".into(),
            table: None,
            fields: vec![field],
            actions: Vec::new(),
            span: Span::default(),
        },
        table: None,
        schema: None,
        requires_auth: false,
        permissions: Vec::new(),
        csrf: CsrfProtection::new(text.clone()),
        form_view: CrudFormViewDef::default(),
        post_only: true,
        audit_table: None,
        audit_event: None,
        audit_chain: false,
        layout_html: None,
    };
    let values = HashMap::from([("comment".to_owned(), text.clone())]);
    let errors = [FieldError {
        field: "comment".into(),
        message: text.clone(),
    }];
    let _ = render_form(&route, &values, &errors, Some(&text));
});
