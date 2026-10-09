use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
};
use zelyra_ast::Type;

use super::json_escape;

pub(super) fn format_openapi(program: &zelyra_ast::Program) -> String {
    let paths = program
        .apis
        .iter()
        .map(|api| {
            let method = api.method.to_ascii_lowercase();
            let path_parameters = api
                .input
                .iter()
                .filter(|field| api.path.contains(&format!("{{{}}}", field.name)))
                .map(|field| {
                    format!(
                        "{{\"name\":\"{}\",\"in\":\"path\",\"required\":true,\"schema\":{}}}",
                        json_escape(&field.name),
                        openapi_schema(&field.ty)
                    )
                })
                .collect::<Vec<_>>();
            let query_parameters = if matches!(api.method.as_str(), "GET" | "DELETE") {
                api.input
                    .iter()
                    .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
                    .map(|field| {
                        format!(
                            "{{\"name\":\"{}\",\"in\":\"query\",\"required\":{},\"schema\":{}}}",
                            json_escape(&field.name),
                            !matches!(field.ty, Type::Option(_)),
                            openapi_schema(&field.ty)
                        )
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let request_body = if matches!(api.method.as_str(), "GET" | "DELETE") {
                String::new()
            } else {
                let properties = api
                    .input
                    .iter()
                    .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
                    .map(|field| {
                        format!(
                            "\"{}\":{}",
                            json_escape(&field.name),
                            openapi_schema(&field.ty)
                        )
                    })
                    .collect::<Vec<_>>();
                let required = api
                    .input
                    .iter()
                    .filter(|field| {
                        !api.path.contains(&format!("{{{}}}", field.name))
                            && !matches!(field.ty, Type::Option(_))
                    })
                    .map(|field| format!("\"{}\"", json_escape(&field.name)))
                    .collect::<Vec<_>>();
                if properties.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\"requestBody\":{{\"required\":true,\"content\":{{\"application/json\":{{\"schema\":{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}}}}}}},",
                        properties.join(","),
                        required.join(",")
                    )
                }
            };
            let mut parameters = path_parameters;
            parameters.extend(query_parameters);
            let security = if api.requires_auth || !api.permissions.is_empty() {
                format!(
                    ",\"x-zelyra-requires-auth\":{},\"x-zelyra-permissions\":[{}]",
                    api.requires_auth,
                    api.permissions
                        .iter()
                        .map(|permission| format!("\"{}\"", json_escape(permission)))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                String::new()
            };
            let metadata = format!(
                "{}{}{}",
                api.version
                    .as_ref()
                    .map(|version| format!(
                        ",\"x-zelyra-api-version\":\"{}\"",
                        json_escape(version)
                    ))
                    .unwrap_or_default(),
                if api.deprecated { ",\"deprecated\":true" } else { "" },
                api.rate_limit
                    .map(|limit| format!(
                        ",\"x-zelyra-rate-limit\":{{\"requests\":{},\"window_seconds\":{}}}",
                        limit.requests, limit.window_seconds
                    ))
                    .unwrap_or_default()
            );
            let responses = std::iter::once(format!(
                "\"200\":{{\"description\":\"Successful response\",\"content\":{{\"application/json\":{{\"schema\":{}}}}}}}",
                openapi_schema(&api.output)
            ))
            .chain(api.errors.iter().map(|error| {
                let response = if let Some(payload) = &error.payload {
                    format!(
                        "{{\"description\":\"{}\",\"content\":{{\"application/json\":{{\"schema\":{}}}}}}}",
                        json_escape(&error.name),
                        openapi_error_schema(payload)
                    )
                } else {
                    format!("{{\"description\":\"{}\"}}", json_escape(&error.name))
                };
                format!("\"{}\":{}", error.status, response)
            }))
            .collect::<Vec<_>>()
            .join(",");
            let operation_id = format!(
                "{}_{}",
                method,
                api.path
                    .trim_matches('/')
                    .replace(['{', '}'], "")
                    .replace('/', "_")
            );
            format!(
                "\"{}\":{{\"{}\":{{\"operationId\":\"{}\",\"parameters\":[{}],{}\"responses\":{{{}}}{}{}}}}}",
                json_escape(&api.path),
                method,
                json_escape(&operation_id),
                parameters.join(","),
                request_body,
                responses,
                metadata,
                security
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let components = format_openapi_components(program);
    let compiler_version = env!("CARGO_PKG_VERSION");
    format!(
        "{{\"openapi\":\"3.0.3\",\"info\":{{\"title\":\"Zelyra API\",\"version\":\"{compiler_version}\"}},\"paths\":{{{paths}}},\"components\":{{\"schemas\":{{{components}}}}}}}"
    )
}

fn format_openapi_components(program: &zelyra_ast::Program) -> String {
    let mut components = Vec::new();
    for definition in &program.types {
        components.push(format!(
            "\"{}\":{}",
            json_escape(&definition.name),
            openapi_schema(&definition.target)
        ));
    }
    for record in &program.records {
        let properties = record
            .fields
            .iter()
            .map(|field| {
                format!(
                    "\"{}\":{}",
                    json_escape(&field.name),
                    openapi_schema(&field.ty)
                )
            })
            .collect::<Vec<_>>();
        let required = record
            .fields
            .iter()
            .filter(|field| !matches!(field.ty, Type::Option(_)))
            .map(|field| format!("\"{}\"", json_escape(&field.name)))
            .collect::<Vec<_>>();
        components.push(format!(
            "\"{}\":{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}",
            json_escape(&record.name),
            properties.join(","),
            required.join(",")
        ));
    }
    for table in &program.tables {
        let properties = table
            .columns
            .iter()
            .map(|column| {
                format!(
                    "\"{}\":{}",
                    json_escape(&column.name),
                    openapi_schema(&column.ty)
                )
            })
            .collect::<Vec<_>>();
        let required = table
            .columns
            .iter()
            .filter(|column| column.required && !matches!(column.ty, Type::Option(_)))
            .map(|column| format!("\"{}\"", json_escape(&column.name)))
            .collect::<Vec<_>>();
        let schema = format!(
            "{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}]}}",
            properties.join(","),
            required.join(",")
        );
        components.push(format!(
            "\"{}\":{}",
            json_escape(&table.name),
            schema.clone()
        ));
        if let Some(singular) = super::singular_type_name(&table.name) {
            components.push(format!("\"{}\":{}", json_escape(&singular), schema));
        }
    }
    components.join(",")
}

pub(super) fn format_typescript_client(program: &zelyra_ast::Program) -> String {
    let mut output = String::new();
    output.push_str("// Generated by Zelyra. Do not edit by hand.\n\n");
    output.push_str("export type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };\n\n");

    let mut error_codes = program
        .apis
        .iter()
        .flat_map(|api| api.errors.iter().map(|error| error.name.clone()))
        .collect::<Vec<_>>();
    error_codes.sort();
    error_codes.dedup();
    if error_codes.is_empty() {
        output.push_str("export type ZelyraApiErrorCode = string;\n\n");
    } else {
        let codes = error_codes
            .iter()
            .map(|code| format!("\"{}\"", json_escape(code)))
            .collect::<Vec<_>>()
            .join(" | ");
        writeln!(
            output,
            "export type ZelyraApiErrorCode = {} | (string & {{}});\n",
            codes
        )
        .expect("writing to a String cannot fail");
    }

    let mut emitted = HashSet::new();
    for definition in &program.types {
        if emitted.insert(definition.name.clone()) {
            writeln!(
                output,
                "export type {} = {};",
                definition.name,
                typescript_type(&definition.target)
            )
            .expect("writing to a String cannot fail");
        }
    }
    for record in &program.records {
        if emitted.insert(record.name.clone()) {
            writeln!(output, "export interface {} {{", record.name)
                .expect("writing to a String cannot fail");
            for field in &record.fields {
                let optional = matches!(field.ty, Type::Option(_));
                writeln!(
                    output,
                    "  {}{}: {};",
                    field.name,
                    if optional { "?" } else { "" },
                    typescript_type(&field.ty)
                )
                .expect("writing to a String cannot fail");
            }
            output.push_str("}\n\n");
        }
    }
    for table in &program.tables {
        let names = std::iter::once(table.name.clone()).chain(
            super::singular_type_name(&table.name)
                .into_iter()
                .filter(|name| name != &table.name),
        );
        for name in names {
            if emitted.insert(name.clone()) {
                writeln!(output, "export interface {} {{", name)
                    .expect("writing to a String cannot fail");
                for column in &table.columns {
                    let optional = !column.required || matches!(column.ty, Type::Option(_));
                    writeln!(
                        output,
                        "  {}{}: {};",
                        column.name,
                        if optional { "?" } else { "" },
                        typescript_type(&column.ty)
                    )
                    .expect("writing to a String cannot fail");
                }
                output.push_str("}\n\n");
            }
        }
    }

    let payload_types = program
        .apis
        .iter()
        .flat_map(|api| {
            api.errors.iter().filter_map(|error| {
                error
                    .payload
                    .as_ref()
                    .map(|payload| (error.name.clone(), typescript_type(payload)))
            })
        })
        .collect::<HashMap<_, _>>();
    if payload_types.is_empty() {
        output.push_str("export type ZelyraApiErrorPayload = JsonValue;\n\n");
    } else {
        output.push_str("export interface ZelyraApiErrorPayloads {\n");
        let mut payload_types = payload_types.into_iter().collect::<Vec<_>>();
        payload_types.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, ty) in payload_types {
            writeln!(output, "  \"{name}\": {ty};").expect("writing to a String cannot fail");
        }
        output.push_str("}\n\nexport type ZelyraApiErrorPayload = ZelyraApiErrorPayloads[keyof ZelyraApiErrorPayloads];\n\n");
    }

    output.push_str(
        "export interface ZelyraClientOptions {\n  baseUrl: string;\n  fetch?: typeof fetch;\n  token?: string;\n}\n\n",
    );
    output.push_str(
        "export class ZelyraApiError extends Error {\n  constructor(\n    public readonly status: number,\n    public readonly code: ZelyraApiErrorCode | undefined,\n    public readonly body: string,\n    message: string,\n  ) {\n    super(message);\n  }\n\n  static async fromResponse(response: Response): Promise<ZelyraApiError> {\n    const body = await response.text();\n    let code: ZelyraApiErrorCode | undefined;\n    let message = `Zelyra API request failed (${response.status})`;\n    try {\n      const payload = JSON.parse(body) as { error?: { code?: unknown; message?: unknown } };\n      if (payload.error && typeof payload.error === \"object\") {\n        if (typeof payload.error.code === \"string\") code = payload.error.code;\n        if (typeof payload.error.message === \"string\") message = payload.error.message;\n      }\n    } catch {\n      // Keep the original response body when the server did not return JSON.
    }\n    return new ZelyraApiError(response.status, code, body, message);\n  }\n}\n\n",
    );
    output.push_str(
        "export class ZelyraClient {\n  private readonly baseUrl: string;\n  private readonly fetchImpl: typeof fetch;\n  private readonly token?: string;\n\n  constructor(options: ZelyraClientOptions) {\n    this.baseUrl = options.baseUrl.replace(/\\/$/, \"\");\n    this.fetchImpl = options.fetch ?? fetch;\n    this.token = options.token;\n  }\n\n",
    );

    for api in &program.apis {
        format_typescript_operation(&mut output, api);
    }
    output = output
        .replace(
            "export class ZelyraApiError extends Error {",
            "export class ZelyraApiError<Details = ZelyraApiErrorPayload> extends Error {",
        )
        .replace(
            "public readonly code: ZelyraApiErrorCode | undefined,\n    public readonly body: string,",
            "public readonly code: ZelyraApiErrorCode | undefined,\n    public readonly details: Details | undefined,\n    public readonly body: string,",
        )
        .replace(
            "static async fromResponse(response: Response): Promise<ZelyraApiError> {",
            "static async fromResponse<Details = ZelyraApiErrorPayload>(response: Response): Promise<ZelyraApiError<Details>> {",
        )
        .replace(
            "let code: ZelyraApiErrorCode | undefined;\n    let message",
            "let code: ZelyraApiErrorCode | undefined;\n    let details: Details | undefined;\n    let message",
        )
        .replace(
            "{ error?: { code?: unknown; message?: unknown } }",
            "{ error?: { code?: unknown; message?: unknown; details?: unknown } }",
        )
        .replace(
            "if (typeof payload.error.message === \"string\") message = payload.error.message;",
            "if (typeof payload.error.message === \"string\") message = payload.error.message;\n        details = payload.error.details as Details | undefined;",
        )
        .replace(
            "new ZelyraApiError(response.status, code, body, message)",
            "new ZelyraApiError(response.status, code, details, body, message)",
        );
    output.push_str("}\n");
    output
}

fn format_typescript_operation(output: &mut String, api: &zelyra_ast::ApiDef) {
    let method = api.method.to_ascii_uppercase();
    let operation = typescript_operation_name(api);
    let query_fields = if matches!(method.as_str(), "GET" | "DELETE") {
        api.input
            .iter()
            .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let body_fields = if matches!(method.as_str(), "GET" | "DELETE") {
        Vec::new()
    } else {
        api.input
            .iter()
            .filter(|field| !api.path.contains(&format!("{{{}}}", field.name)))
            .collect::<Vec<_>>()
    };
    let has_params = !api.input.is_empty();
    write!(output, "  async {}(", operation).expect("writing to a String cannot fail");
    if has_params {
        write!(output, "params: {{ ").expect("writing to a String cannot fail");
        for (index, field) in api.input.iter().enumerate() {
            if index > 0 {
                output.push(' ');
            }
            let optional = matches!(field.ty, Type::Option(_));
            write!(
                output,
                "{}{}: {};",
                field.name,
                if optional { "?" } else { "" },
                typescript_type(&field.ty)
            )
            .expect("writing to a String cannot fail");
        }
        output.push_str(" }");
    }
    writeln!(output, "): Promise<{}> {{", typescript_type(&api.output))
        .expect("writing to a String cannot fail");
    let path = typescript_path_template(&api.path);
    writeln!(output, "    let url = this.baseUrl + `{}`;", path)
        .expect("writing to a String cannot fail");
    if !query_fields.is_empty() {
        output.push_str("    const query = new URLSearchParams();\n");
        for field in query_fields {
            writeln!(
                output,
                "    if (params.{} !== undefined && params.{} !== null) query.set(\"{}\", String(params.{}));",
                field.name, field.name, field.name, field.name
            )
            .expect("writing to a String cannot fail");
        }
        output.push_str("    const queryString = query.toString();\n    if (queryString) url += `?${queryString}`;\n");
    }
    output.push_str("    const headers: Record<string, string> = {};\n    if (this.token) headers.Authorization = `Bearer ${this.token}`;\n");
    if !body_fields.is_empty() {
        output.push_str("    headers[\"Content-Type\"] = \"application/json\";\n");
    }
    writeln!(
        output,
        "    const response = await this.fetchImpl(url, {{ method: \"{}\", headers{} }});",
        method,
        if body_fields.is_empty() {
            String::new()
        } else {
            format!(
                ", body: JSON.stringify({{{}}})",
                body_fields
                    .iter()
                    .map(|field| format!("{}: params.{}", field.name, field.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    )
    .expect("writing to a String cannot fail");
    output.push_str("    if (!response.ok) throw await ZelyraApiError.fromResponse(response);\n");
    output.push_str("    return await response.json() as ");
    output.push_str(&typescript_type(&api.output));
    output.push_str(";\n  }\n\n");
}

fn typescript_type(ty: &Type) -> String {
    match ty {
        Type::Int | Type::UInt | Type::Float | Type::Decimal => "number".into(),
        Type::Bool => "boolean".into(),
        Type::String
        | Type::Char
        | Type::Bytes
        | Type::Timestamp
        | Type::Date
        | Type::Time
        | Type::Duration => "string".into(),
        Type::Unit => "void".into(),
        Type::Option(inner) => format!("{} | null", typescript_type(inner)),
        Type::Result(ok, _) => typescript_type(ok),
        Type::Array(inner) => format!("Array<{}>", typescript_type(inner)),
        Type::Map(_, value) => format!("Record<string, {}>", typescript_type(value)),
        Type::HttpResult(inner) => format!("HttpResult<{}>", typescript_type(inner)),
        Type::Named(name) => match name.as_str() {
            "Id" => "number".into(),
            "Email" | "Url" | "Uuid" | "Money" => "string".into(),
            "Unit" => "void".into(),
            _ => name.clone(),
        },
        Type::Unknown => "unknown".into(),
    }
}

fn typescript_operation_name(api: &zelyra_ast::ApiDef) -> String {
    let mut name = format!(
        "{}_{}",
        api.method.to_ascii_lowercase(),
        api.path.trim_matches('/').replace(['{', '}'], "")
    );
    name = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    if name.ends_with('_') {
        name.pop();
    }
    if name == api.method.to_ascii_lowercase() {
        name.push_str("_root");
    }
    name
}

fn typescript_path_template(path: &str) -> String {
    let mut result = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        let (literal, after_start) = rest.split_at(start);
        result.push_str(&escape_typescript_template(literal));
        let Some(end) = after_start.find('}') else {
            result.push_str(&escape_typescript_template(after_start));
            return result;
        };
        let name = &after_start[1..end];
        result.push_str("${encodeURIComponent(String(params.");
        result.push_str(name);
        result.push_str("))}");
        rest = &after_start[end + 1..];
    }
    result.push_str(&escape_typescript_template(rest));
    result
}

fn escape_typescript_template(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

fn openapi_schema(ty: &Type) -> String {
    match ty {
        Type::Int | Type::UInt => "{\"type\":\"integer\"}".into(),
        Type::Float | Type::Decimal => "{\"type\":\"number\"}".into(),
        Type::Bool => "{\"type\":\"boolean\"}".into(),
        Type::Array(inner) => format!("{{\"type\":\"array\",\"items\":{}}}", openapi_schema(inner)),
        Type::Map(_, value) => format!(
            "{{\"type\":\"object\",\"additionalProperties\":{}}}",
            openapi_schema(value)
        ),
        Type::Option(inner) => openapi_schema(inner),
        Type::Result(ok, _) => openapi_schema(ok),
        Type::HttpResult(_) => "{\"type\":\"object\"}".into(),
        Type::Named(name) => match name.as_str() {
            "Id" => "{\"type\":\"integer\",\"format\":\"int64\"}".into(),
            "Email" => "{\"type\":\"string\",\"format\":\"email\"}".into(),
            "Url" => "{\"type\":\"string\",\"format\":\"uri\"}".into(),
            "Uuid" => "{\"type\":\"string\",\"format\":\"uuid\"}".into(),
            _ => format!(
                "{{\"$ref\":\"#/components/schemas/{}\"}}",
                json_escape(name)
            ),
        },
        _ => "{\"type\":\"string\"}".into(),
    }
}

fn openapi_error_schema(payload: &Type) -> String {
    let details = serde_json::from_str(&openapi_schema(payload))
        .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
    serde_json::json!({
        "type": "object",
        "required": ["error"],
        "properties": {
            "error": {
                "type": "object",
                "required": ["code", "message", "details"],
                "properties": {
                    "code": {"type": "string"},
                    "message": {"type": "string"},
                    "details": details,
                }
            }
        }
    })
    .to_string()
}
