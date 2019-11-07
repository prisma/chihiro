use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{client::HttpConnector, Body, Client};
use metrics_runtime::Receiver;
use metrics_core::{Drain, Observe};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::{timer::Interval, future::FutureExt};
use crate::{console_observer::ConsoleObserver, query_loader::Query};
use console::style;

pub struct Requester {
    prisma_url: String,
    client: Client<HttpConnector>,
    receiver: Receiver,
}

impl Requester {
    pub fn new(prisma_url: Option<String>) -> crate::Result<Self> {
        let mut builder = Client::builder();
        builder.keep_alive(true);

        let client = builder.build(HttpConnector::new());
        let prisma_url = prisma_url.unwrap_or_else(|| String::from("http://localhost:4466/"));

        let receiver = Receiver::builder().build()?;

        Ok(Self { prisma_url, client, receiver, })
    }

    pub fn clear_metrics(&mut self) -> crate::Result<()> {
        self.receiver = Receiver::builder().build()?;
        Ok(())
    }

    pub async fn run(
        &self,
        query: &Query,
        rate: u64,
        duration: Duration,
        pb: crate::OptionalBar,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut rate_stream = Interval::new_interval(Duration::from_nanos(1_000_000_000 / rate));

        let start = Instant::now();
        let mut tick = Instant::now();
        let mut sent_total = 0;

        while let Some(_) = rate_stream.next().await {
            if Instant::now().duration_since(start) >= duration {
                pb.finish_with_message(&self.metrics());
                break;
            }

            if Instant::now().duration_since(tick) >= Duration::from_secs(1) {
                tick = Instant::now();
                pb.inc(1);
            }

            let current_rate = match Instant::now().duration_since(start).as_nanos() {
                0 => 0,
                nanos => sent_total * 1_000_000_000 / nanos,
            };

            pb.set_message(&format!(
                "{}: {}/{}, {}",
                style("rate").bold().dim(),
                current_rate,
                rate,
                self.metrics()
            ));

            let mut sink = self.receiver.sink();
            let requesting = self.request(query);

            tokio::spawn(async move {
                let start = Instant::now();
                let res = requesting.timeout(Duration::from_secs(10)).await;

                sink.record_timing("response_time", start, Instant::now());

                match res {
                    Ok(Ok(_)) => sink.counter("success").increment(),
                    Ok(Err(_)) | Err(_) => sink.counter("error").increment(),
                }
            });

            sent_total += 1;
        }

        Ok(())
    }

    pub fn request(&self, query: &Query) -> hyper::client::ResponseFuture {
        let json_data = json!({
            "query": query.query(),
            "variables": {}
        });

        let payload = serde_json::to_string(&json_data).unwrap();

        let mut builder = hyper::Request::builder();
        builder.uri(&self.prisma_url);
        builder.method("POST");

        let content_length = format!("{}", payload.len());
        builder.header(CONTENT_LENGTH, &content_length);

        builder.header(CONTENT_TYPE, "application/json");

        let request = builder.body(Body::from(payload)).unwrap();

        self.client.request(request)
    }

    fn metrics(&self) -> String {
        let mut observer = ConsoleObserver::new();
        let cont = self.receiver.controller();
        cont.observe(&mut observer);

        observer.drain()
    }
}
