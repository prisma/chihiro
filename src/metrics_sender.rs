use crate::json_observer::ResponseTime;
use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{client::HttpConnector, Body, Client, Request};
use hyper_tls::HttpsConnector;
use std::io::{Error, ErrorKind};

pub struct MetricsSender {
    endpoint: String,
    database: String,
    client: Client<HttpsConnector<HttpConnector>>,
    user: String,
    password: String,
}

impl MetricsSender {
    pub fn new(endpoint: &str, database: &str, user: &str, password: &str) -> Self {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, Body>(https);

        Self {
            endpoint: endpoint.into(),
            database: database.into(),
            user: user.into(),
            password: password.into(),
            client,
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

        let request = builder.body(Body::from(payload)).unwrap();
        let response = self.client.request(request).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let bytes = hyper::body::to_bytes(response.into_body()).await?;
            let json: serde_json::Value = serde_json::from_slice(&bytes)?;

            Err(Error::new(
                ErrorKind::Other,
                format!("Failed to send metrics: {}", json),
            )
            .into())
        }
    }
}
