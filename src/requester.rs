use crate::{
    bar::OptionalBar,
    config::{Query, QueryConfig},
    console_observer::ConsoleObserver,
    json_observer::{JsonObserver, ResponseTime},
};
use console::style;
use futures::stream::TryStreamExt;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{client::HttpConnector, Body, Client};
use metrics_core::{Drain, Observe};
use metrics_runtime::Receiver;
use serde_json::json;
use serde::Deserialize;
use std::{
    io::{Error, ErrorKind},
    time::{Duration, Instant},
};
use tokio::{future::FutureExt, timer::Interval};

pub struct Requester {
    prisma_url: String,
    client: Client<HttpConnector>,
    receiver: Receiver,
}

#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub commit: String,
    pub version: String,
    pub primary_connector: String,
}

impl Requester {
    pub fn new(prisma_url: Option<String>) -> crate::Result<Self> {
        let mut builder = Client::builder();
        builder.keep_alive(true);
        builder.max_idle_per_host(1);
        builder.keep_alive_timeout(Duration::from_secs(1));

        let client = builder.build(HttpConnector::new());
        let prisma_url = prisma_url.unwrap_or_else(|| String::from("http://localhost:4466/"));

        let receiver = Receiver::builder().build()?;

        Ok(Self {
            prisma_url,
            client,
            receiver,
        })
    }

    pub async fn run(&self, query: &Query, rps: u64, duration: Duration, pb: &crate::OptionalBar) {
        let mut rate_stream = Interval::new_interval(Duration::from_nanos(1_000_000_000 / rps));

        let start = Instant::now();
        let mut tick = Instant::now();
        let mut sent_total = 0;

        while let Some(_) = rate_stream.next().await {
            if Instant::now().duration_since(start) >= duration {
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

            let mut sink = self.receiver.sink();
            let requesting = self.request(query);

            pb.set_message(&format!(
                "{}: {}/{}, {}",
                style("rps").bold().dim(),
                current_rate,
                rps,
                self.console_metrics(),
            ));

            tokio::spawn(async move {
                let start = Instant::now();
                let res = requesting.timeout(Duration::from_millis(2000)).await;

                sink.record_timing("response_time", start, Instant::now());

                match res {
                    Ok(Ok(_)) => sink.counter("success").increment(),
                    Ok(Err(_)) | Err(_) => {
                        sink.counter("error").increment()
                    },
                }
            });

            sent_total += 1;
        }
    }

    pub async fn validate(&self, query_config: &QueryConfig, pb: OptionalBar) -> crate::Result<()> {
        for query in query_config.queries() {
            let res = self.request(&query).await?;
            let body = res.into_body().try_concat().await?;
            let body = String::from_utf8(body.to_vec())?;
            let json: serde_json::Value = serde_json::from_str(&body)?;

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

    pub async fn server_info(&self) -> crate::Result<ServerInfo> {
        let mut builder = hyper::Request::builder();
        builder.uri(dbg!(&format!("{}server_info", self.prisma_url)));
        builder.method("GET");

        let request = builder.body(Body::empty())?;
        let res = self.client.request(request).await?;
        let body = res.into_body().try_concat().await?;

        Ok(serde_json::from_slice(&body)?)
    }

    pub fn console_metrics(&self) -> String {
        let mut observer = ConsoleObserver::new();
        let cont = self.receiver.controller();
        cont.observe(&mut observer);

        observer.drain()
    }

    pub async fn json_metrics(&self, query_name: &str, rps: u64) -> crate::Result<ResponseTime> {
        let server_info = self.server_info().await?;
        let mut observer = JsonObserver::new(server_info, query_name, rps);
        let cont = self.receiver.controller();

        cont.observe(&mut observer);
        Ok(observer.drain())
    }
}
