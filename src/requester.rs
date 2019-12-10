use crate::{
    bar::OptionalBar,
    config::{Query, QueryConfig},
    console_observer::ConsoleObserver,
    json_observer::{JsonObserver, ResponseTime},
};
use console::style;
use futures::stream::StreamExt;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{client::HttpConnector, Body, Client};
use metrics_core::{Drain, Observe};
use metrics_runtime::{Controller, Receiver};
use serde::Deserialize;
use serde_json::json;
use std::{
    io::{Error, ErrorKind},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
    collections::HashSet,
};
use tokio::{time::{interval, timeout}, task::JoinHandle};

pub struct Requester {
    prisma_url: String,
    receiver: Receiver,
    client: Client<HttpConnector>,
}

#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub commit: String,
    pub version: String,
    pub primary_connector: String,
}

enum ResponseType {
    Ok,
    Error(String)
}

impl Requester {
    pub fn new(prisma_url: Option<String>) -> crate::Result<Self> {
        let mut builder = Client::builder();
        builder.keep_alive(true);

        let client = builder.build(HttpConnector::new());
        let prisma_url = prisma_url.unwrap_or_else(|| String::from("http://localhost:4466/"));

        let receiver = Receiver::builder().build()?;

        Ok(Self {
            prisma_url,
            client,
            receiver,
        })
    }

    fn reconnect_client(&mut self) {
        let mut builder = Client::builder();
        builder.keep_alive(true);
        self.client = builder.build(HttpConnector::new());
    }

    pub async fn run(&mut self, query: &Query, rps: u64, duration: Duration, pb: &OptionalBar) {
        self.reconnect_client();
        let mut rate_stream = interval(Duration::from_nanos(1_000_000_000 / rps));

        let start = Instant::now();
        let mut tick = Instant::now();
        let mut sent_total = 0;
        let in_flight = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::with_capacity((duration.as_secs() * rps) as usize);
        while Instant::now().duration_since(start) < duration {
            rate_stream.tick().await;

            if Instant::now().duration_since(tick) >= Duration::from_secs(1) {
                tick = Instant::now();
                pb.inc(1);
            }

            let current_rate = match Instant::now().duration_since(start).as_nanos() {
                0 => 0,
                nanos => sent_total * 1_000_000_000 / nanos + 1,
            };

            let cont = self.receiver.controller();
            let mut sink = self.receiver.sink();

            let pb = pb.clone();

            let in_flight = in_flight.clone();
            in_flight.fetch_add(1, Ordering::SeqCst);

            let requesting = timeout(Duration::from_secs(10), self.request(query));
            let jh: JoinHandle<ResponseType> = tokio::spawn(async move {
                let start = Instant::now();
                let res = requesting.await;

                sink.record_timing("response_time", start, Instant::now());

                let metrics = Self::drain_metrics(cont);

                pb.set_message(&format!(
                    "{}: {}/{}, {}",
                    style("rps").bold().dim(),
                    current_rate,
                    rps,
                    metrics,
                ));

                in_flight.fetch_sub(1, Ordering::SeqCst);

                match res {
                    Ok(Ok(_)) => {
                        sink.counter("success").increment();
                        ResponseType::Ok
                    },
                    Ok(Err(e)) => {
                        sink.counter("error").increment();
                        ResponseType::Error(format!("{}", e))
                    }
                    Err(e) => {
                        sink.counter("error").increment();
                        ResponseType::Error(format!("{}", e))
                    }
                }
            });

            handles.push(jh);

            sent_total += 1;
        }

        let mut seen_errors = HashSet::new();

        for handle in handles {
            if let Ok(ResponseType::Error(s)) = handle.await {
                seen_errors.insert(s);
            }
        }

        if !seen_errors.is_empty() {
            println!("Errors:");
            for error in seen_errors.into_iter() {
                println!("{}", error);
            }
        }
    }

    pub async fn validate(&self, query_config: &QueryConfig, pb: OptionalBar) -> crate::Result<()> {
        for query in query_config.queries() {
            let res = self.request(&query).await?;
            let content_length: usize = res
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|s| s.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            let mut body: Vec<u8> = Vec::with_capacity(content_length);
            let mut chunks = res.into_body();

            while let Some(chunk) = chunks.next().await {
                body.extend_from_slice(&chunk?);
            }

            let json: serde_json::Value = serde_json::from_slice(body.as_slice())?;

            if json["errors"] != serde_json::Value::Null {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Query {} returned an error: {}", query.name(), json),
                )
                .into());
            }

            pb.inc(1);
        }

        pb.finish_with_message("All queries validated");

        Ok(())
    }

    pub async fn server_info(&self) -> crate::Result<ServerInfo> {
        let builder = hyper::Request::builder()
            .uri(&format!("{}server_info", self.prisma_url))
            .method("GET");

        let request = builder.body(Body::empty())?;
        let res = self.client.request(request).await?;

        let content_length: usize = res
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|s| s.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let mut body: Vec<u8> = Vec::with_capacity(content_length);
        let mut chunks = res.into_body();

        while let Some(chunk) = chunks.next().await {
            body.extend_from_slice(&chunk?);
        }

        Ok(serde_json::from_slice(&body)?)
    }

    pub async fn json_metrics(&self, query_name: &str, rps: u64) -> crate::Result<ResponseTime> {
        let server_info = self.server_info().await?;
        let mut observer = JsonObserver::new(server_info, query_name, rps);
        let cont = self.receiver.controller();

        cont.observe(&mut observer);
        Ok(observer.drain())
    }

    pub fn request(&self, query: &Query) -> hyper::client::ResponseFuture {
        let json_data = json!({
            "query": query.query(),
            "variables": {}
        });

        let payload = serde_json::to_string(&json_data).unwrap();
        let content_length = format!("{}", payload.len());

        let builder = hyper::Request::builder()
            .uri(&self.prisma_url)
            .method("POST")
            .header(CONTENT_LENGTH, &content_length)
            .header(CONTENT_TYPE, "application/json");

        let request = builder.body(Body::from(payload)).unwrap();

        self.client.request(request)
    }

    pub fn console_metrics(&self) -> String {
        Self::drain_metrics(self.receiver.controller())
    }

    fn drain_metrics(cont: Controller) -> String {
        let mut observer = ConsoleObserver::new();
        cont.observe(&mut observer);
        observer.drain()
    }
}
