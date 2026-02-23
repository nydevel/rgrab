use std::collections::HashMap;

use serde::Deserialize;

use crate::span::{Span, SpanEvent, SpanStatus};

// === OTLP ExportTraceServiceRequest ===

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTraceServiceRequest {
    #[serde(default)]
    pub resource_spans: Vec<ResourceSpans>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSpans {
    pub resource: Option<Resource>,
    #[serde(default)]
    pub scope_spans: Vec<ScopeSpans>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeSpans {
    pub scope: Option<InstrumentationScope>,
    #[serde(default)]
    pub spans: Vec<OtlpSpan>,
}

#[derive(Debug, Deserialize)]
pub struct Resource {
    #[serde(default)]
    pub attributes: Vec<KeyValue>,
}

#[derive(Debug, Deserialize)]
pub struct InstrumentationScope {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtlpSpan {
    #[serde(default)]
    pub trace_id: String,
    #[serde(default)]
    pub span_id: String,
    #[serde(default)]
    pub parent_span_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kind: u32,
    #[serde(default)]
    pub start_time_unix_nano: StringOrInt,
    #[serde(default)]
    pub end_time_unix_nano: StringOrInt,
    #[serde(default)]
    pub attributes: Vec<KeyValue>,
    #[serde(default)]
    pub events: Vec<OtlpEvent>,
    pub status: Option<OtlpStatus>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtlpEvent {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub time_unix_nano: StringOrInt,
    #[serde(default)]
    pub attributes: Vec<KeyValue>,
}

#[derive(Debug, Deserialize)]
pub struct OtlpStatus {
    #[serde(default)]
    pub code: u32,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct KeyValue {
    #[serde(default)]
    pub key: String,
    pub value: Option<AnyValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnyValue {
    pub string_value: Option<String>,
    pub int_value: Option<serde_json::Value>,
    pub double_value: Option<f64>,
    pub bool_value: Option<bool>,
}

// OTLP sends nano timestamps as either string or integer
#[derive(Debug, Default)]
pub struct StringOrInt(pub i64);

impl<'de> Deserialize<'de> for StringOrInt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val = serde_json::Value::deserialize(deserializer)?;
        let n = match &val {
            serde_json::Value::String(s) => s.parse::<i64>().unwrap_or(0),
            serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
            _ => 0,
        };
        Ok(StringOrInt(n))
    }
}

// === Span Kind Constants ===

const SPAN_KIND_INTERNAL: u32 = 1;
const SPAN_KIND_SERVER: u32 = 2;
const SPAN_KIND_CLIENT: u32 = 3;
const SPAN_KIND_PRODUCER: u32 = 4;
const SPAN_KIND_CONSUMER: u32 = 5;

// === Conversion to internal Span model ===

fn any_value_to_string(v: &AnyValue) -> String {
    if let Some(s) = &v.string_value {
        return s.clone();
    }
    if let Some(i) = &v.int_value {
        return match i {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => i.to_string(),
        };
    }
    if let Some(d) = v.double_value {
        return d.to_string();
    }
    if let Some(b) = v.bool_value {
        return b.to_string();
    }
    String::new()
}

fn kv_to_hashmap(attrs: &[KeyValue]) -> HashMap<String, String> {
    attrs
        .iter()
        .filter_map(|kv| {
            kv.value
                .as_ref()
                .map(|v| (kv.key.clone(), any_value_to_string(v)))
        })
        .collect()
}

fn find_attribute<'a>(attrs: &'a [KeyValue], key: &str) -> Option<&'a AnyValue> {
    attrs
        .iter()
        .find(|kv| kv.key == key)
        .and_then(|kv| kv.value.as_ref())
}

fn nanos_to_datetime(nanos: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp_nanos(nanos)
}

fn convert_status(status: Option<&OtlpStatus>) -> SpanStatus {
    match status.map(|s| s.code) {
        Some(1) => SpanStatus::Ok,
        Some(2) => SpanStatus::Error,
        _ => SpanStatus::Unset,
    }
}

fn convert_event(event: &OtlpEvent) -> SpanEvent {
    SpanEvent {
        name: event.name.clone(),
        timestamp: nanos_to_datetime(event.time_unix_nano.0),
        attributes: kv_to_hashmap(&event.attributes),
    }
}

pub fn convert_otlp_span(
    otlp_span: &OtlpSpan,
    service_name: &str,
    resource_attrs: &HashMap<String, String>,
) -> Span {
    let attributes = build_span_attributes(otlp_span, resource_attrs);
    let parent = optional_parent(&otlp_span.parent_span_id);

    Span {
        trace_id: otlp_span.trace_id.clone(),
        span_id: otlp_span.span_id.clone(),
        parent_span_id: parent,
        operation_name: otlp_span.name.clone(),
        service_name: service_name.to_string(),
        start_time: nanos_to_datetime(otlp_span.start_time_unix_nano.0),
        end_time: nanos_to_datetime(otlp_span.end_time_unix_nano.0),
        status: convert_status(otlp_span.status.as_ref()),
        attributes,
        events: otlp_span.events.iter().map(convert_event).collect(),
    }
}

fn build_span_attributes(
    otlp_span: &OtlpSpan,
    resource_attrs: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut attributes = kv_to_hashmap(&otlp_span.attributes);
    for (k, v) in resource_attrs {
        attributes.entry(k.clone()).or_insert_with(|| v.clone());
    }
    if otlp_span.kind > 0 {
        attributes
            .entry("span.kind".to_string())
            .or_insert_with(|| span_kind_str(otlp_span.kind).to_string());
    }
    attributes
}

fn span_kind_str(kind: u32) -> &'static str {
    match kind {
        SPAN_KIND_INTERNAL => "internal",
        SPAN_KIND_SERVER => "server",
        SPAN_KIND_CLIENT => "client",
        SPAN_KIND_PRODUCER => "producer",
        SPAN_KIND_CONSUMER => "consumer",
        _ => "unspecified",
    }
}

fn optional_parent(parent_span_id: &str) -> Option<String> {
    if parent_span_id.is_empty() {
        None
    } else {
        Some(parent_span_id.to_string())
    }
}

pub fn extract_spans(req: &ExportTraceServiceRequest) -> Vec<Span> {
    req.resource_spans
        .iter()
        .flat_map(extract_resource_spans)
        .collect()
}

fn extract_resource_spans(rs: &ResourceSpans) -> Vec<Span> {
    let resource_attrs = rs
        .resource
        .as_ref()
        .map(|r| kv_to_hashmap(&r.attributes))
        .unwrap_or_default();

    let service_name = rs
        .resource
        .as_ref()
        .and_then(|r| find_attribute(&r.attributes, "service.name"))
        .map(any_value_to_string)
        .unwrap_or_default();

    rs.scope_spans
        .iter()
        .flat_map(|ss| &ss.spans)
        .map(|s| convert_otlp_span(s, &service_name, &resource_attrs))
        .collect()
}
