use crate::requester::ServerInfo;
use chrono::Utc;
use hdrhistogram::Histogram;
use metrics_core::{Drain, Key, Observer};
use serde::Serialize;

pub struct JsonObserver {
    response_times: Histogram<u64>,
    server_info: ServerInfo,
    query_name: String,
    successes: u64,
    failures: u64,
    rps: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseTime {
    commit: String,
    connector: String,
    query_name: String,
    p50: u64,
    p95: u64,
    p99: u64,
    rps: u64,
    successes: u64,
    failures: u64,
    time: String,
    version: String,
}

impl JsonObserver {
    pub fn new<S>(server_info: ServerInfo, query_name: S, rps: u64) -> Self
    where
        S: Into<String>,
    {
        Self {
            server_info,
            rps,
            query_name: query_name.into(),
            response_times: Histogram::new(3).unwrap(),
            successes: 0,
            failures: 0,
        }
    }
}

impl Observer for JsonObserver {
    fn observe_counter(&mut self, key: Key, value: u64) {
        match key.name().as_ref() {
            "success" => self.successes = value,
            "error" => self.failures = value,
            _ => (),
        }
    }

    fn observe_gauge(&mut self, _: Key, _: i64) {}

    fn observe_histogram(&mut self, key: Key, values: &[u64]) {
        if key.name().as_ref() == "response_time" {
            for value in values {
                self.response_times.record(*value).unwrap();
            }
        }
    }
}

impl Drain<ResponseTime> for JsonObserver {
    fn drain(&mut self) -> ResponseTime {
        ResponseTime {
            commit: self.server_info.commit.clone(),
            connector: self.server_info.primary_connector.clone(),
            version: self.server_info.version.clone(),
            query_name: self.query_name.clone(),
            p50: self.response_times.value_at_quantile(0.5),
            p95: self.response_times.value_at_quantile(0.95),
            p99: self.response_times.value_at_quantile(0.99),
            rps: self.rps,
            successes: self.successes,
            failures: self.failures,
            time: Utc::now().to_rfc3339(),
        }
    }
}
