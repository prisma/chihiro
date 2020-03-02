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
use response_summary::ConnectorType;
use server::Server;
use std::path::PathBuf;
use structopt::StructOpt;
use error::Error;

type Result<T> = std::result::Result<T, error::Error>;

#[derive(Debug, StructOpt, Clone)]
pub struct BenchOpt {
    /// The Prisma URL
    #[structopt(long, default_value = "http://localhost:4466/")]
    endpoint_url: String,
    /// The query configuration file (toml) to execute
    #[structopt(long)]
    query_file: String,
    /// Validate queries before benchmarking
    #[structopt(long)]
    validate: bool,
    /// Show fancy progress metrics (disable for CI)
    #[structopt(long)]
    show_progress: bool,
    /// Which Elastic Search database to write
    #[structopt(long)]
    metrics_database: String,
    /// The GraphQL endpoint type. (prisma|hasura)
    #[structopt(long)]
    endpoint_type: Option<requester::EndpointType>,
    /// Path to the local SQLite database
    #[structopt(long, default_value = "file:metrics.db")]
    sqlite_path: String,
    /// Username to the ElasticSearch database
    #[structopt(long, env = "ELASTIC_USER")]
    elastic_user: String,
    /// Password to the ElasticSearch database
    #[structopt(long, env = "ELASTIC_PW")]
    elastic_password: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct KibanaOpt {
    /// Path to the query file
    query_path: PathBuf,
    /// Path to the template file
    #[structopt(long)]
    template: PathBuf,
}

#[derive(Debug, StructOpt, Clone)]
pub struct SetupOpt {
    /// The server hostname
    host: String,
    /// The location of the private key to connect to the server
    #[structopt(long)]
    private_key: PathBuf,
    /// Username to the server
    #[structopt(long)]
    user: String,
    /// Passphrase for the private key
    #[structopt(long, env = "SSH_PASSPHRASE")]
    passphrase: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub struct StdoutReportOpt {
    /// Path to the local SQLite database
    #[structopt(long, default_value = "file:metrics.db")]
    sqlite_path: String,
    /// The connector to get the reports from (postgres|mysql)
    #[structopt(long)]
    connector: ConnectorType,
}

#[derive(Debug, StructOpt, Clone)]
pub struct SlackReportOpt {
    /// The webhook URI for sending the report
    #[structopt(long, env = "SLACK_WEBHOOK_URL")]
    webhook_url: String,
    /// Path to the local SQLite database
    #[structopt(long, default_value = "file:metrics.db")]
    sqlite_path: String,
    /// The connector to get the reports from (postgres|mysql)
    #[structopt(long)]
    connector: ConnectorType,
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
            let result = StdoutReporter.from_sqlite(&report_opts.sqlite_path, report_opts.connector).await;

            if let Err(e @ Error::NotEnoughMeasurements(..)) = result {
                println!("{}", e);
                Ok(())
            } else {
                result
            }
        }
        Opt::SlackReport(report_opts) => {
            let result = SlackReporter::new(&report_opts.webhook_url)
                .from_sqlite(&report_opts.sqlite_path, report_opts.connector)
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
