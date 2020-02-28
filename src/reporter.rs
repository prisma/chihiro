use super::response_summary::ResponseSummary;
use async_trait::async_trait;
use console::{pad_str, style, Alignment};

#[async_trait]
pub trait Reporter {
    async fn from_sqlite(&self, path: &str) -> crate::Result<()>;
}

pub struct StdoutReporter;

#[async_trait]
impl Reporter for StdoutReporter {
    async fn from_sqlite(&self, path: &str) -> crate::Result<()> {
        let summary = ResponseSummary::find_from_sqlite(path).await?;
        let (previous_id, next_id) = summary.commits();

        println!(
            "Comparing commit {} (old) to {} (new)",
            &previous_id[0..15],
            &next_id[0..15]
        );

        println!();

        for (query, p50, p95, p99) in summary.differences() {
            print!(
                "{} :: ",
                pad_str(query, summary.longest_query(), Alignment::Left, None)
            );

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
