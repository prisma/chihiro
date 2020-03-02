mod bar;
mod bench;
mod config;
mod console_observer;
mod json_observer;
mod kibana;
mod metrics_sender;
mod metrics_storage;
mod reporter;
mod requester;
mod response_summary;
mod server;
mod error;

use bench::Bench;
use reporter::{Reporter, SlackReporter, StdoutReporter};
use server::Server;
use std::path::PathBuf;
use structopt::StructOpt;
use error::Error;

type Result<T> = std::result::Result<T, error::Error>;

#[derive(Debug, StructOpt, Clone)]
pub struct BenchOpt {
    /// The Prisma URL. Default: http://localhost:4466/
    #[structopt(long, default_value = "http://localhost:4466/")]
    endpoint_url: String,
    /// The query configuration file (toml) to execute.
    #[structopt(long)]
    query_file: String,
    /// Validate queries before benchmarking.
    #[structopt(long)]
    validate: bool,
    /// Show fancy progress metrics (disable for CI).
    #[structopt(long)]
    show_progress: bool,
    /// Which Elastic Search database to write.
    #[structopt(long)]
    metrics_database: String,
    /// The GraphQL endpoint type. (prisma|hasura)
    #[structopt(long)]
    endpoint_type: Option<requester::EndpointType>,
    #[structopt(long, default_value = "file:metrics.db")]
    sqlite_path: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct KibanaOpt {
    query_path: PathBuf,
    #[structopt(long)]
    template: PathBuf,
}

#[derive(Debug, StructOpt, Clone)]
pub struct SetupOpt {
    host: String,
    #[structopt(long)]
    private_key: PathBuf,
    #[structopt(long)]
    user: String,
    #[structopt(long, env = "SSH_PASSPHRASE")]
    passphrase: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct StdoutReportOpt {
    #[structopt(long, default_value = "file:metrics.db")]
    sqlite_path: String,
    #[structopt(long)]
    connector: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct SlackReportOpt {
    #[structopt(long)]
    webhook_url: String,
    #[structopt(long, default_value = "file:metrics.db")]
    sqlite_path: String,
    #[structopt(long)]
    connector: String,
}

#[derive(Debug, StructOpt, Clone)]
/// Prisma Load Tester
pub enum Opt {
    /// Run benchmarks
    Bench(BenchOpt),
    /// Generate Kibana graphs
    Kibana(KibanaOpt),
    /// Set up remote app server
    Setup(SetupOpt),
    /// Print last report statistics to the console
    StdoutReport(StdoutReportOpt),
    /// Send last report statistics to Slack
    SlackReport(SlackReportOpt),
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    match Opt::from_args() {
        Opt::Bench(bench_opts) => Bench::new(bench_opts).await?.run().await,
        Opt::Kibana(kibana_opts) => kibana::generate(kibana_opts),
        Opt::Setup(setup_opts) => Server::new(setup_opts)?.setup(),
        Opt::StdoutReport(report_opts) => {
            let result = StdoutReporter.from_sqlite(&report_opts.sqlite_path, &report_opts.connector).await;

            if let Err(e @ Error::NotEnoughMeasurements(..)) = result {
                println!("{}", e);
                Ok(())
            } else {
                result
            }
        }
        Opt::SlackReport(report_opts) => {
            let result = SlackReporter::new(&report_opts.webhook_url)
                .from_sqlite(&report_opts.sqlite_path, &report_opts.connector)
                .await;

            if let Err(e @ Error::NotEnoughMeasurements(..)) = result {
                println!("{}", e);
                Ok(())
            } else {
                result
            }
        }
    }
}
