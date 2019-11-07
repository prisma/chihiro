use console::style;
use hdrhistogram::Histogram;
use metrics_core::{Drain, Key, Observer};

pub struct ConsoleObserver {
    response_times: Histogram<u64>,
    successful: u64,
    error: u64,
}

impl Default for ConsoleObserver {
    fn default() -> Self {
        Self {
            response_times: Histogram::new(3).unwrap(),
            successful: 0,
            error: 0,
        }
    }
}

impl ConsoleObserver {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Observer for ConsoleObserver {
    fn observe_counter(&mut self, key: Key, value: u64) {
        match key.name().as_ref() {
            "success" => self.successful = value,
            "error" => self.error = value,
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

impl Drain<String> for ConsoleObserver {
    fn drain(&mut self) -> String {
        let p50 = (self.response_times.value_at_quantile(0.5) as f64 / 10000.0).round() / 100.0;
        let p95 = (self.response_times.value_at_quantile(0.95) as f64 / 10000.0).round() / 100.0;
        let p99 = (self.response_times.value_at_quantile(0.99) as f64 / 10000.0).round() / 100.0;

        let output = format!(
            "{}: {}, {}: {}, {}: {} ms, {}: {} ms, {}: {} ms",
            style("success").bold().dim(),
            self.successful,
            style("errors").bold().dim(),
            self.error,
            style("p50").bold().dim(),
            p50,
            style("p95").bold().dim(),
            p95,
            style("p99").bold().dim(),
            p99,
        );

        *self = Self::default();

        output
    }
}
