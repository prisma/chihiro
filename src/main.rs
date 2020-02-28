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

use bench::Bench;
use reporter::Reporter;
use reporter::StdoutReporter;
use server::Server;
use std::path::PathBuf;
use structopt::StructOpt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
pub struct ReportOpt {
    #[structopt(long, default_value = "file:metrics.db")]
    database_path: String,
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
    Report(ReportOpt),
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    match Opt::from_args() {
        Opt::Bench(bench_opts) => Bench::new(bench_opts).await?.run().await,
        Opt::Kibana(kibana_opts) => kibana::generate(kibana_opts),
        Opt::Setup(setup_opts) => Server::new(setup_opts)?.setup(),
        Opt::Report(report_opts) => StdoutReporter.from_sqlite(&report_opts.database_path).await,
    }
}
