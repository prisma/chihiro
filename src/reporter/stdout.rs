use super::Reporter;
use crate::response_summary::{ConnectorType, ResponseSummary};
use async_trait::async_trait;
use console::{pad_str, style, Alignment};

pub struct StdoutReporter;

#[async_trait]
impl Reporter for StdoutReporter {
    async fn report(&self, url: &str, connector: ConnectorType) -> crate::Result<()> {
        let summary = ResponseSummary::aggregate(url, connector).await?;
        let (previous_id, next_id) = summary.commits();
        let padding = summary.longest_query();

        println!(
            "Comparing commit {} (old) to {} (new)",
            &previous_id, &next_id,
        );

        println!();

        for (query, p50, p95, p99) in summary.differences() {
            print!("{} :: ", pad_str(query, padding, Alignment::Left, None));

            if p50 <= 0.0 {
                print!("p50: {:>10} ", style(format!("{:.2}%", p50)).green().bold())
            } else {
                print!("p50: {:>10} ", style(format!("{:.2}%", p50)).red().bold())
            }

            if p95 <= 0.0 {
                print!("p95: {:>10} ", style(format!("{:.2}%", p95)).green().bold())
            } else {
                print!("p95: {:>10} ", style(format!("{:.2}%", p95)).red().bold())
            }

            if p99 <= 0.0 {
                print!("p99: {:>10} ", style(format!("{:.2}%", p99)).green().bold())
            } else {
                print!("p99: {:>10} ", style(format!("{:.2}%", p99)).red().bold())
            }

            println!()
        }

        Ok(())
    }
}
