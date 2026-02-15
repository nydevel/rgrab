use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LokiPushRequest {
    pub streams: Vec<LokiStream>,
}

#[derive(Debug, Deserialize)]
pub struct LokiStream {
    pub stream: HashMap<String, String>,
    pub values: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
pub struct LokiQueryResponse {
    pub status: String,
    pub data: LokiQueryData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LokiQueryData {
    pub result_type: String,
    pub result: Vec<LokiResultStream>,
    pub stats: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct LokiResultStream {
    pub stream: HashMap<String, String>,
    pub values: Vec<[String; 2]>,
}

#[derive(Debug, Serialize)]
pub struct LokiLabelsResponse {
    pub status: String,
    pub data: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Forward,
    #[default]
    Backward,
}

#[derive(Debug, Clone)]
pub struct LabelMatcher {
    pub name: String,
    pub op: MatchOp,
    pub value: String,
}

#[derive(Debug, Clone, Copy)]
pub enum MatchOp {
    Eq,
    Neq,
    Re,
    Nre,
}

impl LabelMatcher {
    pub fn matches(&self, labels: &HashMap<String, String>) -> bool {
        let actual = labels.get(&self.name).map(String::as_str).unwrap_or("");
        match self.op {
            MatchOp::Eq => actual == self.value,
            MatchOp::Neq => actual != self.value,
            MatchOp::Re => regex::Regex::new(&self.value)
                .map(|re| re.is_match(actual))
                .unwrap_or(false),
            MatchOp::Nre => regex::Regex::new(&self.value)
                .map(|re| !re.is_match(actual))
                .unwrap_or(true),
        }
    }
}
