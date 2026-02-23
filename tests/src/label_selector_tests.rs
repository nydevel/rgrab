use common::label_selector::parse_label_selector;
use common::loki::MatchOp;

#[test]
fn should_parse_empty_selector() {
    let result = parse_label_selector("{}").unwrap();
    assert!(result.is_empty());
}

#[test]
fn should_parse_single_eq_matcher() {
    let result = parse_label_selector(r#"{service="web"}"#).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "service");
    assert!(matches!(result[0].op, MatchOp::Eq));
    assert_eq!(result[0].value, "web");
}

#[test]
fn should_parse_multiple_matchers() {
    let result = parse_label_selector(r#"{service="web", env="prod"}"#).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "service");
    assert_eq!(result[1].name, "env");
}

#[test]
fn should_parse_neq_operator() {
    let result = parse_label_selector(r#"{level!="debug"}"#).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].op, MatchOp::Neq));
}

#[test]
fn should_parse_regex_operator() {
    let result = parse_label_selector(r#"{service=~"web.*"}"#).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].op, MatchOp::Re));
    assert_eq!(result[0].value, "web.*");
}

#[test]
fn should_parse_nre_operator() {
    let result = parse_label_selector(r#"{service!~"test.*"}"#).unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0].op, MatchOp::Nre));
}

#[test]
fn should_reject_missing_braces() {
    assert!(parse_label_selector("service=\"web\"").is_err());
}

#[test]
fn should_parse_whitespace_around_matchers() {
    let result = parse_label_selector(r#"{ service = "web" }"#).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "service");
    assert_eq!(result[0].value, "web");
}

#[test]
fn should_parse_escaped_quotes_in_value() {
    let result = parse_label_selector(r#"{msg="hello \"world\""}"#).unwrap();
    assert_eq!(result[0].value, "hello \"world\"");
}

#[test]
fn should_match_eq_label() {
    let matchers = parse_label_selector(r#"{service="web"}"#).unwrap();
    let mut labels = std::collections::HashMap::new();
    labels.insert("service".to_string(), "web".to_string());
    assert!(matchers[0].matches(&labels));
}

#[test]
fn should_not_match_eq_label_wrong_value() {
    let matchers = parse_label_selector(r#"{service="api"}"#).unwrap();
    let mut labels = std::collections::HashMap::new();
    labels.insert("service".to_string(), "web".to_string());
    assert!(!matchers[0].matches(&labels));
}

#[test]
fn should_match_neq_label() {
    let matchers = parse_label_selector(r#"{service!="api"}"#).unwrap();
    let mut labels = std::collections::HashMap::new();
    labels.insert("service".to_string(), "web".to_string());
    assert!(matchers[0].matches(&labels));
}

#[test]
fn should_match_regex_label() {
    let matchers = parse_label_selector(r#"{service=~"web.*"}"#).unwrap();
    let mut labels = std::collections::HashMap::new();
    labels.insert("service".to_string(), "web-frontend".to_string());
    assert!(matchers[0].matches(&labels));
}

#[test]
fn should_match_missing_label_as_empty() {
    let matchers = parse_label_selector(r#"{env=""}"#).unwrap();
    let labels = std::collections::HashMap::new();
    assert!(matchers[0].matches(&labels));
}
