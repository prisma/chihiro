mod query_loader;
mod requester;

use metrics_core::{Builder, Drain, Observe};
use metrics_runtime::{observers::PrometheusBuilder, Receiver};
use query_loader::QueryConfig;
use requester::Requester;
use structopt::StructOpt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, StructOpt, Clone)]
/// Prisma Load Tester
pub struct Opt {
    /// Request timeout in seconds.
    #[structopt(long)]
    timeout: Option<u64>,
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

    let receiver = Receiver::builder().build()?;
    let cont = receiver.controller();
    receiver.install();

    let query_config = QueryConfig::new(opts.query_file)?;
    let requester = Requester::new(opts.prisma_url);

    for (query, rate) in query_config.queries() {
        println!("{} (rate: {})", query.name(), rate);
        requester
            .run(query.query(), rate, query_config.duration())
            .await?;

        let mut observer = PrometheusBuilder::new().build();
        cont.observe(&mut observer);

        println!("{}", observer.drain());
    }

    Ok(())
}
