use common::otlp::{
    AnyValue, ExportTraceServiceRequest, KeyValue, OtlpSpan, OtlpStatus, Resource, ResourceSpans,
    ScopeSpans, StringOrInt, convert_otlp_span, extract_spans,
};
use common::span::SpanStatus;

fn make_key_value(key: &str, string_value: &str) -> KeyValue {
    KeyValue {
        key: key.to_string(),
        value: Some(AnyValue {
            string_value: Some(string_value.to_string()),
            int_value: None,
            double_value: None,
            bool_value: None,
        }),
    }
}

fn make_test_otlp_span() -> OtlpSpan {
    OtlpSpan {
        trace_id: "trace1".to_string(),
        span_id: "span1".to_string(),
        parent_span_id: String::new(),
        name: "GET /api".to_string(),
        kind: 2,
        start_time_unix_nano: StringOrInt(1_000_000_000),
        end_time_unix_nano: StringOrInt(2_000_000_000),
        attributes: vec![make_key_value("http.method", "GET")],
        events: vec![],
        status: Some(OtlpStatus {
            code: 1,
            message: String::new(),
        }),
    }
}

#[test]
fn should_convert_otlp_span_to_internal() {
    let otlp = make_test_otlp_span();
    let resource_attrs = std::collections::HashMap::new();
    let span = convert_otlp_span(&otlp, "web-service", &resource_attrs);

    assert_eq!(span.trace_id, "trace1");
    assert_eq!(span.span_id, "span1");
    assert_eq!(span.operation_name, "GET /api");
    assert_eq!(span.service_name, "web-service");
    assert_eq!(span.status, SpanStatus::Ok);
    assert!(span.parent_span_id.is_none());
    assert_eq!(span.attributes.get("http.method").unwrap(), "GET");
    assert_eq!(span.attributes.get("span.kind").unwrap(), "server");
}

#[test]
fn should_set_parent_span_id_when_present() {
    let mut otlp = make_test_otlp_span();
    otlp.parent_span_id = "parent1".to_string();
    let span = convert_otlp_span(&otlp, "svc", &std::collections::HashMap::new());

    assert_eq!(span.parent_span_id.as_deref(), Some("parent1"));
}

#[test]
fn should_convert_error_status() {
    let mut otlp = make_test_otlp_span();
    otlp.status = Some(OtlpStatus {
        code: 2,
        message: "failed".to_string(),
    });
    let span = convert_otlp_span(&otlp, "svc", &std::collections::HashMap::new());

    assert_eq!(span.status, SpanStatus::Error);
}

#[test]
fn should_convert_unset_status_when_none() {
    let mut otlp = make_test_otlp_span();
    otlp.status = None;
    let span = convert_otlp_span(&otlp, "svc", &std::collections::HashMap::new());

    assert_eq!(span.status, SpanStatus::Unset);
}

#[test]
fn should_merge_resource_attributes() {
    let otlp = make_test_otlp_span();
    let resource_attrs = std::collections::HashMap::from([
        ("deployment".to_string(), "prod".to_string()),
        ("http.method".to_string(), "POST".to_string()),
    ]);
    let span = convert_otlp_span(&otlp, "svc", &resource_attrs);

    assert_eq!(span.attributes.get("deployment").unwrap(), "prod");
    // Span attributes take precedence over resource attributes
    assert_eq!(span.attributes.get("http.method").unwrap(), "GET");
}

#[test]
fn should_extract_spans_from_request() {
    let req = ExportTraceServiceRequest {
        resource_spans: vec![ResourceSpans {
            resource: Some(Resource {
                attributes: vec![make_key_value("service.name", "my-service")],
            }),
            scope_spans: vec![ScopeSpans {
                scope: None,
                spans: vec![make_test_otlp_span()],
            }],
        }],
    };

    let spans = extract_spans(&req);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].service_name, "my-service");
    assert_eq!(spans[0].trace_id, "trace1");
}

#[test]
fn should_extract_multiple_spans_from_multiple_scopes() {
    let mut span2 = make_test_otlp_span();
    span2.span_id = "span2".to_string();
    span2.name = "POST /api".to_string();

    let req = ExportTraceServiceRequest {
        resource_spans: vec![ResourceSpans {
            resource: Some(Resource {
                attributes: vec![make_key_value("service.name", "svc")],
            }),
            scope_spans: vec![
                ScopeSpans {
                    scope: None,
                    spans: vec![make_test_otlp_span()],
                },
                ScopeSpans {
                    scope: None,
                    spans: vec![span2],
                },
            ],
        }],
    };

    let spans = extract_spans(&req);
    assert_eq!(spans.len(), 2);
}

#[test]
fn should_handle_empty_resource_spans() {
    let req = ExportTraceServiceRequest {
        resource_spans: vec![],
    };
    let spans = extract_spans(&req);
    assert!(spans.is_empty());
}

#[test]
fn should_deserialize_string_or_int_from_string() {
    let json = r#""1234567890""#;
    let val: StringOrInt = serde_json::from_str(json).unwrap();
    assert_eq!(val.0, 1_234_567_890);
}

#[test]
fn should_deserialize_string_or_int_from_number() {
    let json = "1234567890";
    let val: StringOrInt = serde_json::from_str(json).unwrap();
    assert_eq!(val.0, 1_234_567_890);
}

#[test]
fn should_convert_span_kind_values() {
    let kinds = [
        (1, "internal"),
        (2, "server"),
        (3, "client"),
        (4, "producer"),
        (5, "consumer"),
    ];

    for (kind_code, expected_str) in kinds {
        let mut otlp = make_test_otlp_span();
        otlp.kind = kind_code;
        otlp.attributes.clear();
        let span = convert_otlp_span(&otlp, "svc", &std::collections::HashMap::new());
        assert_eq!(
            span.attributes.get("span.kind").unwrap(),
            expected_str,
            "kind {kind_code} should map to {expected_str}"
        );
    }
}
