use tokio::timer::Interval;
use std::time::{Instant, Duration};
use hyper::{Client, client::HttpConnector, Body};
use metrics::{timing, counter};
use serde_json::json;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};

pub struct Requester {
    rate_stream: Interval,
    duration_to_run: Option<Duration>,
    prisma_url: String,
    client: Client<HttpConnector>,
}

impl From<crate::Opt> for Requester {
    fn from(opts: crate::Opt) -> Self {
        let rate_stream = Interval::new_interval(
            Duration::from_nanos(1_000_000_000 / opts.rate)
        );

        let duration_to_run = opts.duration.map(Duration::from_secs);
        let prisma_url = opts.prisma_url.unwrap_or_else(|| String::from("http://localhost:4466/"));

        let mut builder = Client::builder();
        builder.keep_alive(true);

        let client = builder.build(HttpConnector::new());

        Self {
            rate_stream,
            duration_to_run,
            prisma_url,
            client,
        }
    }
}

impl Requester {
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();

        let json_data = json!({
            "query": "query {\n  findManyArtist(where: { Name_ends_with: \"is\"}) {\n    id\n    Name\n  }\n}\n",
            "variables": {}
        });

        let payload = serde_json::to_string(&json_data)?;
        let content_length = format!("{}", payload.len());

        while let Some(_) = self.rate_stream.next().await {
            match self.duration_to_run {
                Some(d) if Instant::now().duration_since(start) >= d => {
                    break;
                },
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
