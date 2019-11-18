use crate::json_observer::ResponseTime;
use futures::stream::TryStreamExt;
use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use std::io::{Error, ErrorKind};

pub struct MetricsSender {
    endpoint: String,
    database: String,
    client: Client<HttpsConnector>,
    user: String,
    password: String,
}

impl MetricsSender {
    pub fn new(endpoint: &str, database: &str, user: &str, password: &str) -> Self {
        let https = HttpsConnector::new().unwrap();
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
        let payload = serde_json::to_string(metrics).unwrap();

        let mut builder = Request::builder();
        builder.uri(&format!("{}/{}/_doc/", self.endpoint, self.database));
        builder.method("POST");

        let content_length = format!("{}", payload.len());
        builder.header(CONTENT_LENGTH, &content_length);
        builder.header(CONTENT_TYPE, "application/json");

        builder.header(
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
            let body = response.into_body().try_concat().await?;
            let json: serde_json::Value = serde_json::from_slice(&body)?;

            Err(Error::new(
                ErrorKind::Other,
                format!("Failed to send metrics: {}", json),
            )
            .into())
        }
    }
}
