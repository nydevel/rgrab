use std::collections::HashMap;

use common::loki::{
    Direction, LabelMatcher, LokiPushRequest, LokiQueryResponse, LokiResultStream, MatchOp,
};

#[test]
fn should_deserialize_loki_push_request() {
    let json = r#"{
        "streams": [{
            "stream": {"service": "web", "level": "info"},
            "values": [
                ["1000000000", "log line 1"],
                ["2000000000", "log line 2"]
            ]
        }]
    }"#;

    let req: LokiPushRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.streams.len(), 1);
    assert_eq!(req.streams[0].stream.get("service").unwrap(), "web");
    assert_eq!(req.streams[0].values.len(), 2);
}

#[test]
fn should_serialize_loki_query_response() {
    let response = LokiQueryResponse {
        status: "success".to_string(),
        data: common::loki::LokiQueryData {
            result_type: "streams".to_string(),
            result: vec![LokiResultStream {
                stream: HashMap::from([("service".to_string(), "web".to_string())]),
                values: vec![["1000".to_string(), "hello".to_string()]],
            }],
            stats: serde_json::json!({}),
        },
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"status\":\"success\""));
    assert!(json.contains("\"resultType\":\"streams\""));
}

#[test]
fn should_default_direction_to_backward() {
    let json = r#""backward""#;
    let dir: Direction = serde_json::from_str(json).unwrap();
    assert!(matches!(dir, Direction::Backward));
}

#[test]
fn should_parse_forward_direction() {
    let json = r#""forward""#;
    let dir: Direction = serde_json::from_str(json).unwrap();
    assert!(matches!(dir, Direction::Forward));
}

#[test]
fn should_match_eq_labels_correctly() {
    let matcher = LabelMatcher {
        name: "service".to_string(),
        op: MatchOp::Eq,
        value: "web".to_string(),
    };

    let matching = HashMap::from([("service".to_string(), "web".to_string())]);
    let non_matching = HashMap::from([("service".to_string(), "api".to_string())]);
    let empty: HashMap<String, String> = HashMap::new();

    assert!(matcher.matches(&matching));
    assert!(!matcher.matches(&non_matching));
    assert!(!matcher.matches(&empty));
}

#[test]
fn should_match_neq_labels_correctly() {
    let matcher = LabelMatcher {
        name: "service".to_string(),
        op: MatchOp::Neq,
        value: "web".to_string(),
    };

    let matching = HashMap::from([("service".to_string(), "api".to_string())]);
    let non_matching = HashMap::from([("service".to_string(), "web".to_string())]);

    assert!(matcher.matches(&matching));
    assert!(!matcher.matches(&non_matching));
}

#[test]
fn should_match_regex_labels_correctly() {
    let matcher = LabelMatcher {
        name: "service".to_string(),
        op: MatchOp::Re,
        value: "web-.*".to_string(),
    };

    let matching = HashMap::from([("service".to_string(), "web-frontend".to_string())]);
    let non_matching = HashMap::from([("service".to_string(), "api".to_string())]);

    assert!(matcher.matches(&matching));
    assert!(!matcher.matches(&non_matching));
}

#[test]
fn should_match_nre_labels_correctly() {
    let matcher = LabelMatcher {
        name: "env".to_string(),
        op: MatchOp::Nre,
        value: "test.*".to_string(),
    };

    let matching = HashMap::from([("env".to_string(), "prod".to_string())]);
    let non_matching = HashMap::from([("env".to_string(), "testing".to_string())]);

    assert!(matcher.matches(&matching));
    assert!(!matcher.matches(&non_matching));
}

#[test]
fn should_deserialize_push_with_metadata() {
    let json = r#"{
        "streams": [{
            "stream": {"service": "web"},
            "values": [
                ["1000000000", "log line", {"trace_id": "abc", "span_id": "def"}]
            ]
        }]
    }"#;

    let req: LokiPushRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.streams[0].values[0].len(), 3);
    let meta = &req.streams[0].values[0][2];
    assert_eq!(meta["trace_id"].as_str().unwrap(), "abc");
}
