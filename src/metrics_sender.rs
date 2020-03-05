use crate::json_observer::ResponseTime;
use reqwest::Client;
use std::io::{Error, ErrorKind};

pub struct MetricsSender {
    endpoint: String,
    database: String,
    client: Client,
    user: String,
    password: String,
}

impl MetricsSender {
    pub fn new(endpoint: &str, database: &str, user: &str, password: &str) -> Self {
        Self {
            endpoint: endpoint.into(),
            database: database.into(),
            user: user.into(),
            password: password.into(),
            client: Client::new(),
        }
    }

    pub async fn send(&self, metrics: &ResponseTime) -> crate::Result<()> {
        let response = self
            .client
            .post(&format!("{}/{}/_doc/", self.endpoint, self.database))
            .basic_auth(&self.user, Some(&self.password))
            .json(metrics)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let json: serde_json::Value = response.json().await?;

            Err(Error::new(
                ErrorKind::Other,
                format!("Failed to send metrics: {}", json),
            )
            .into())
        }
    }
}
