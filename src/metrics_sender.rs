use crate::json_observer::ResponseTime;
use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use isahc::HttpClient;
use http::Request;
use std::io::{Error, ErrorKind};

pub struct MetricsSender {
    endpoint: String,
    database: String,
    client: HttpClient,
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
            client: HttpClient::new().unwrap(),
        }
    }

    pub async fn send(&self, metrics: &ResponseTime) -> crate::Result<()> {
        let payload = serde_json::to_string(metrics)?;
        let content_length = format!("{}", payload.len());


        let builder = Request::builder()
            .uri(&format!("{}/{}/_doc/", self.endpoint, self.database))
            .method("POST")
            .header(CONTENT_LENGTH, &content_length)
            .header(CONTENT_TYPE, "application/json")
            .header(
                AUTHORIZATION,
                &format!(
                    "Basic {}",
                    base64::encode(&format!("{}:{}", self.user, self.password))
                ),
            );

        let request = builder.body(payload).unwrap();
        let response = self.client.send_async(request).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let json: serde_json::Value = response.into_body().json().unwrap();

            Err(Error::new(
                ErrorKind::Other,
                format!("Failed to send metrics: {}", json),
            )
            .into())
        }
    }
}
