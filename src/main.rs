mod bar;
mod config;
mod console_observer;
mod json_observer;
mod requester;
mod metrics_sender;
mod bench;
mod kibana;

use structopt::StructOpt;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, StructOpt, Clone)]
pub struct BenchOpt {
    /// The Prisma URL. Default: http://localhost:4466/
    #[structopt(long)]
    prisma_url: Option<String>,
    /// The squery configuration file (toml) to execute.
    #[structopt(long)]
    query_file: String,
    #[structopt(long)]
    validate: bool,
    #[structopt(long)]
    show_progress: bool,
    #[structopt(long)]
    metrics_database: String,
}

#[derive(Debug, StructOpt, Clone)]
pub struct KibanaOpt {
    query_path: PathBuf,
    #[structopt(long)]
    template: PathBuf,
}

#[derive(Debug, StructOpt, Clone)]
/// Prisma Load Tester
pub enum Opt {
    /// Run benchmarks
    Bench(BenchOpt),
    /// Generate Kibana graphs
    Kibana(KibanaOpt),
}

fn main() -> Result<()> {
    match Opt::from_args() {
        Opt::Bench(bench_opts) => bench::run(bench_opts),
        Opt::Kibana(kibana_opts) => kibana::generate(kibana_opts),
    }
}
