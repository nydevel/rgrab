use anyhow::Result;
use common::log::LogEntry;
use common::span::Span;

pub struct ApiClient {
    base_url: String,
    client: reqwest::Client,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .connect_timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    pub async fn fetch_logs(&self, limit: usize) -> Result<Vec<LogEntry>> {
        let url = format!("{}/api/logs?limit={limit}", self.base_url);
        let resp = self.client.get(&url).send().await?.json().await?;
        Ok(resp)
    }

    pub async fn fetch_traces(&self, limit: usize) -> Result<Vec<Span>> {
        let url = format!("{}/api/traces?limit={limit}", self.base_url);
        let resp = self.client.get(&url).send().await?.json().await?;
        Ok(resp)
    }

    pub async fn fetch_trace(&self, trace_id: &str) -> Result<Vec<Span>> {
        let url = format!("{}/api/traces?trace_id={trace_id}", self.base_url);
        let resp = self.client.get(&url).send().await?.json().await?;
        Ok(resp)
    }

    pub async fn fetch_labels(&self) -> Result<Vec<String>> {
        let url = format!("{}/rgrab/api/v1/labels", self.base_url);
        let resp: LabelsResponse = self.client.get(&url).send().await?.json().await?;
        Ok(resp.data)
    }

    pub async fn fetch_label_values(&self, name: &str) -> Result<Vec<String>> {
        let url = format!("{}/rgrab/api/v1/label/{name}/values", self.base_url);
        let resp: LabelsResponse = self.client.get(&url).send().await?.json().await?;
        Ok(resp.data)
    }
}

#[derive(serde::Deserialize)]
struct LabelsResponse {
    data: Vec<String>,
}
