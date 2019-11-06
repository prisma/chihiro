mod query_loader;
mod requester;
mod console_observer;

use query_loader::QueryConfig;
use requester::Requester;
use structopt::StructOpt;
use indicatif::{ProgressBar, ProgressStyle};
use console::style;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, StructOpt, Clone)]
/// Prisma Load Tester
pub struct Opt {
    /// The Prisma URL. Default: http://localhost:4466/
    #[structopt(long)]
    prisma_url: Option<String>,
    /// The query configuration file (toml) to execute.
    #[structopt(long)]
    query_file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opt::from_args();

    let query_config = QueryConfig::new(opts.query_file)?;
    let mut requester = Requester::new(opts.prisma_url)?;

    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.cyan} [{elapsed_precise:.bold.dim}] {wide_msg}");

    let tests = query_config.test_count();

    for (i, (query, rate)) in query_config.queries().enumerate() {
        println!(
            "[{}] {}",
            style(&format!("{}/{}", i+1, tests)).bold().dim(),
            query.name(),
        );

        let pb = ProgressBar::new(query_config.duration().as_secs());

        pb.set_style(spinner_style.clone());

        requester.run(query.query(), rate, query_config.duration(), pb).await?;
        requester.clear_metrics()?;
    }

    Ok(())
}
