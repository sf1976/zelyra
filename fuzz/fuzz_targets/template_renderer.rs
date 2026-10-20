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
        min_length: None,
        max: Some(4096),
        min_value: None,
        max_value: None,
        choices: Vec::new(),
        widget: None,
        appearance: None,
                file_upload: None,
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
        before_form_html: None,
        after_form_html: None,
        record_context_fields: Vec::new(),
        form_context_fields: Vec::new(),
        field_content_html: HashMap::new(),
        post_only: true,
        recipient_column: None,
        audit_table: None,
        audit_event: None,
        audit_chain: false,
        layout_html: None,
        tenant_column: None,
        tenant_membership_table: None,
        child_collections: Vec::new(),
    };
    let values = HashMap::from([("comment".to_owned(), text.clone())]);
    let errors = [FieldError {
        field: "comment".into(),
        message: text.clone(),
    }];
    let _ = render_form(&route, &values, &errors, Some(&text));
});
