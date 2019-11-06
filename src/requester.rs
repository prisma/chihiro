use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{client::HttpConnector, Body, Client};
use metrics::{counter, timing};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::timer::Interval;

pub struct Requester {
    prisma_url: String,
    client: Client<HttpConnector>,
}

impl Requester {
    pub fn new(prisma_url: Option<String>) -> Self {
        let mut builder = Client::builder();
        builder.keep_alive(true);

        let client = builder.build(HttpConnector::new());
        let prisma_url = prisma_url.unwrap_or_else(|| String::from("http://localhost:4466/"));

        Self { prisma_url, client }
    }

    pub async fn run(
        &self,
        query: &str,
        rate: u64,
        duration: Option<Duration>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();

        let json_data = json!({
            "query": query,
            "variables": {}
        });

        let payload = serde_json::to_string(&json_data)?;
        let content_length = format!("{}", payload.len());

        let mut rate_stream = Interval::new_interval(Duration::from_nanos(1_000_000_000 / rate));

        while let Some(_) = rate_stream.next().await {
            match duration {
                Some(d) if Instant::now().duration_since(start) >= d => {
                    break;
                }
                _ => {
                    let mut builder = hyper::Request::builder();
                    builder.uri(&self.prisma_url);
                    builder.method("POST");

                    builder.header(CONTENT_LENGTH, &content_length);
                    builder.header(CONTENT_TYPE, "application/json");

                    let request = builder.body(Body::from(payload.clone()))?;
                    let requesting = self.client.request(request);

                    tokio::spawn(async move {
                        let start = Instant::now();
                        let res = requesting.await;

                        timing!("response_time", start, Instant::now());

                        match res {
                            Ok(_) => counter!("successful", 1),
                            Err(_) => counter!("error", 1),
                        }
                    });
                }
            }
        }

        Ok(())
    }
}
