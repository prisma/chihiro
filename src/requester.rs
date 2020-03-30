use crate::{
    bar::OptionalBar,
    config::{Query, QueryConfig, SingleQuery},
    console_observer::ConsoleObserver,
    error::Error,
    json_observer::{JsonObserver, ResponseTime},
};
use console::style;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{client::HttpConnector, Body, Client};
use metrics_core::{Drain, Observe};
use metrics_runtime::{Controller, Receiver};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashSet,
    str::FromStr,
    time::{Duration, Instant},
};
use tokio::{
    task::JoinHandle,
    time::{interval, timeout},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndpointType {
    Prisma,
    Hasura,
    Photon,
}

impl Default for EndpointType {
    fn default() -> Self {
        Self::Prisma
    }
}

impl FromStr for EndpointType {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "hasura" => Ok(Self::Hasura),
            "prisma" => Ok(Self::Prisma),
            "photon" => Ok(Self::Photon),
            typ => Err(Error::InvalidEndpointType(typ.into())),
        }
    }
}

pub struct Requester {
    endpoint_type: EndpointType,
    endpoint_url: String,
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
    Error(String),
}

impl Requester {
    pub fn new(endpoint_type: Option<EndpointType>, endpoint_url: String) -> crate::Result<Self> {
        let builder = Client::builder();
        let client = builder.build(HttpConnector::new());
        let receiver = Receiver::builder().build()?;
        let endpoint_type = endpoint_type.unwrap_or(EndpointType::Prisma);

        Ok(Self {
            endpoint_type,
            endpoint_url,
            client,
            receiver,
        })
    }

    pub async fn run(&mut self, query: &Query, rps: u64, duration: Duration, pb: &OptionalBar) {
        let mut rate_stream = interval(Duration::from_nanos(1_000_000_000 / rps));

        let start = Instant::now();
        let mut tick = Instant::now();
        let mut sent_total = 0;

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

            let requesting = match query {
                Query::Single(single_query) => {
                    timeout(Duration::from_secs(10), self.request(single_query))
                }
                Query::Batch { query, batch } => {
                    timeout(Duration::from_secs(10), self.batch(query, *batch))
                }
            };

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

                match res {
                    Ok(Ok(res)) => {
                        if res.status().is_success() {
                            sink.counter("success").increment();
                            ResponseType::Ok
                        } else {
                            sink.counter("error").increment();
                            ResponseType::Error(format!("{}", res.status().as_str()))
                        }
                    }
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
            let res = match query {
                Query::Single(single_query) => self.request(single_query).await?,
                Query::Batch { query, batch } => self.batch(query, *batch).await?,
            };

            pb.inc(1);

            if res.status().is_success() {
                let bytes = hyper::body::to_bytes(res.into_body()).await?;
                let json: serde_json::Value = serde_json::from_slice(bytes.as_ref())?;

                if json["errors"] != serde_json::Value::Null {
                    return Err(Error::InvalidQuery {
                        query: query.name().into(),
                        error: json,
                    });
                }
            } else {
                return Err(Error::InvalidQuery {
                    query: query.name().into(),
                    error: serde_json::Value::String(res.status().as_str().into()),
                });
            }
        }

        pb.finish_with_message("All queries validated");

        Ok(())
    }

    pub async fn server_info(&self) -> crate::Result<ServerInfo> {
        match self.endpoint_type {
            EndpointType::Prisma | EndpointType::Photon => {
                let builder = hyper::Request::builder()
                    .uri(&format!("{}server_info", self.endpoint_url))
                    .method("GET");

                let request = builder.body(Body::empty())?;
                let res = self.client.request(request).await?;
                let bytes = hyper::body::to_bytes(res.into_body()).await?;

                Ok(serde_json::from_slice(&bytes)?)
            }
            EndpointType::Hasura => Ok(ServerInfo {
                commit: String::from("hasura-1.0.0"),
                version: String::from("1.0.0"),
                primary_connector: String::from("postgres"),
            }),
        }
    }

    pub async fn json_metrics(&self, query_name: &str, rps: u64) -> crate::Result<ResponseTime> {
        let server_info = self.server_info().await?;
        let mut observer = JsonObserver::new(server_info, query_name, rps);
        let cont = self.receiver.controller();

        cont.observe(&mut observer);
        Ok(observer.drain())
    }

    pub fn request(&self, query: &SingleQuery) -> hyper::client::ResponseFuture {
        let json_data = json!({
            "query": query.query().trim(),
            "variables": {}
        });

        let payload = serde_json::to_string(&json_data).unwrap();
        let content_length = format!("{}", payload.len());

        let builder = hyper::Request::builder()
            .uri(&self.endpoint_url)
            .method("POST")
            .header(CONTENT_LENGTH, &content_length)
            .header(CONTENT_TYPE, "application/json");

        let request = builder.body(Body::from(payload)).unwrap();

        self.client.request(request)
    }

    pub fn batch(&self, query: &SingleQuery, batch: u64) -> hyper::client::ResponseFuture {
        let queries: Vec<serde_json::Value> = (0..batch)
            .map(|_| {
                json!({
                    "query": query.query().trim(),
                    "variables": {},
                })
            })
            .collect();

        let json_data = json!({
            "batch": queries,
        });

        let payload = serde_json::to_string(&json_data).unwrap();
        let content_length = format!("{}", payload.len());

        let builder = hyper::Request::builder()
            .uri(&self.endpoint_url)
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
